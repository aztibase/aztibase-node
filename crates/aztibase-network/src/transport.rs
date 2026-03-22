use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport, ResponseChannel};
use libp2p::swarm::SwarmEvent;
use libp2p::{
    Multiaddr, PeerId, Swarm, autonat, connection_limits, dcutr, gossipsub, identify, kad, mdns,
};
use tracing::{debug, info, warn};

use crate::behaviour::{AztibaseBehaviour, AztibaseBehaviourEvent};
use crate::block_sync::{BLOCK_SYNC_PROTOCOL, BlockSyncCodec, BlockSyncRequest, BlockSyncResponse};
use crate::connection_filter::ConnectionFilter;
use crate::light_sync::{LIGHT_SYNC_PROTOCOL, LightSyncCodec, LightSyncRequest, LightSyncResponse};
use crate::peer_store::PeerStore;
use crate::reputation::{OffenseSeverity, PeerReputationStore};
use crate::{discovery, gossip};

pub const MAX_ESTABLISHED_CONNECTIONS: u32 = 50;
pub const AUTONAT_PROBE_INTERVAL_SECS: u64 = 30;

/// Protocol version for peer compatibility checks.
/// Peers with different major versions are disconnected.
pub const PROTOCOL_VERSION: u32 = 1;

/// Agent string prefix used in libp2p identify.
pub const AGENT_PREFIX: &str = "aztibase";

/// Build the agent version string: `aztibase/<major>`.
pub fn agent_version() -> String {
    format!("{}/{}", AGENT_PREFIX, PROTOCOL_VERSION)
}

/// Parse a protocol version from a peer's agent string.
/// Returns None if the agent string doesn't match the expected format.
pub fn parse_agent_version(agent: &str) -> Option<u32> {
    let stripped = agent
        .strip_prefix(AGENT_PREFIX)
        .and_then(|s| s.strip_prefix('/'))?;
    stripped.split('.').next()?.parse().ok()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatStatus {
    Unknown,
    Public,
    Private,
}

impl std::fmt::Display for NatStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, "unknown"),
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
        }
    }
}

pub const DEFAULT_WEBRTC_PORT: u16 = 9000;

pub struct TransportConfig {
    pub idle_timeout_secs: u64,
    pub reputation_store: Option<Arc<PeerReputationStore>>,
    pub peer_store: Option<Arc<PeerStore>>,
    pub enable_autonat: bool,
    pub relay_servers: Vec<Multiaddr>,
    pub autonat_probe_interval_secs: u64,
    pub enable_webrtc: bool,
    pub webrtc_listen_port: u16,
    pub genesis_hash: Option<[u8; 32]>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 300,
            reputation_store: None,
            peer_store: None,
            enable_autonat: true,
            relay_servers: Vec::new(),
            autonat_probe_interval_secs: AUTONAT_PROBE_INTERVAL_SECS,
            enable_webrtc: false,
            webrtc_listen_port: DEFAULT_WEBRTC_PORT,
            genesis_hash: None,
        }
    }
}

#[derive(Debug)]
pub enum NetworkEvent {
    Message {
        source: PeerId,
        topic: String,
        data: Vec<u8>,
    },
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    Listening(Multiaddr),
    LightSyncRequest {
        peer: PeerId,
        request: LightSyncRequest,
        channel: ResponseChannel<LightSyncResponse>,
    },
    LightSyncResponse {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: LightSyncResponse,
    },
    LightSyncOutboundFailure {
        peer: PeerId,
        request_id: OutboundRequestId,
        error: request_response::OutboundFailure,
    },
    BlockSyncRequest {
        peer: PeerId,
        request: BlockSyncRequest,
        channel: ResponseChannel<BlockSyncResponse>,
    },
    BlockSyncResponse {
        peer: PeerId,
        request_id: OutboundRequestId,
        response: BlockSyncResponse,
    },
    BlockSyncOutboundFailure {
        peer: PeerId,
        request_id: OutboundRequestId,
        error: request_response::OutboundFailure,
    },
}

#[derive(Debug, Clone, Default)]
pub struct NatTraversalStats {
    pub dcutr_attempts: u64,
    pub dcutr_successes: u64,
    pub dcutr_failures: u64,
}

pub struct Libp2pTransport {
    swarm: Swarm<AztibaseBehaviour>,
    topics: HashMap<String, gossipsub::IdentTopic>,
    local_peer_id: PeerId,
    reputation: Option<Arc<PeerReputationStore>>,
    peer_store: Option<Arc<PeerStore>>,
    conn_filter: ConnectionFilter,
    peer_ips: HashMap<PeerId, IpAddr>,
    nat_status: NatStatus,
    relay_servers: Vec<Multiaddr>,
    kad_bootstrapped: bool,
    nat_traversal_stats: NatTraversalStats,
    genesis_hex: Option<String>,
}

impl Libp2pTransport {
    pub fn new(config: TransportConfig) -> Result<Self> {
        let probe_interval = Duration::from_secs(config.autonat_probe_interval_secs);
        let relay_servers = config.relay_servers.clone();
        let genesis_hex = config.genesis_hash.as_ref().map(gossip::genesis_hex_prefix);

        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .context("Failed to configure TCP transport")?
            .with_quic()
            .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)
            .context("Failed to configure relay client")?
            .with_behaviour(|key, relay_client| {
                let peer_id = key.public().to_peer_id();

                let gs_config = gossip::gossipsub_config()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                let mut gs = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gs_config,
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let score_params = gossip::peer_score_params_scoped(genesis_hex.as_deref());
                gs.with_peer_score(score_params, gossip::peer_score_thresholds())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let kademlia =
                    discovery::kademlia_behaviour_scoped(peer_id, genesis_hex.as_deref());
                let mdns = discovery::mdns_behaviour(peer_id)?;

                let conn_limits = connection_limits::ConnectionLimits::default()
                    .with_max_established(Some(MAX_ESTABLISHED_CONNECTIONS))
                    .with_max_established_per_peer(Some(2));

                let light_sync = request_response::Behaviour::with_codec(
                    LightSyncCodec,
                    [(LIGHT_SYNC_PROTOCOL, ProtocolSupport::Full)],
                    request_response::Config::default(),
                );

                let block_sync = request_response::Behaviour::with_codec(
                    BlockSyncCodec,
                    [(BLOCK_SYNC_PROTOCOL, ProtocolSupport::Full)],
                    request_response::Config::default(),
                );

                let autonat_config = autonat::Config {
                    retry_interval: probe_interval,
                    ..Default::default()
                };
                let autonat = autonat::Behaviour::new(peer_id, autonat_config);

                let identify_config =
                    identify::Config::new(format!("/aztibase/{}", PROTOCOL_VERSION), key.public())
                        .with_agent_version(agent_version());
                let identify = identify::Behaviour::new(identify_config);

                Ok(AztibaseBehaviour {
                    gossipsub: gs,
                    kademlia,
                    mdns,
                    connection_limits: connection_limits::Behaviour::new(conn_limits),
                    light_sync,
                    block_sync,
                    autonat,
                    relay_client,
                    dcutr: dcutr::Behaviour::new(peer_id),
                    identify,
                })
            })
            .context("Failed to configure behaviour")?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(config.idle_timeout_secs))
            })
            .build();

        let local_peer_id = *swarm.local_peer_id();

        let mut transport = Self {
            swarm,
            topics: HashMap::new(),
            local_peer_id,
            reputation: config.reputation_store,
            peer_store: config.peer_store,
            conn_filter: ConnectionFilter::new(),
            peer_ips: HashMap::new(),
            nat_status: NatStatus::Unknown,
            relay_servers,
            kad_bootstrapped: false,
            nat_traversal_stats: NatTraversalStats::default(),
            genesis_hex,
        };

        transport.subscribe_all()?;

        Ok(transport)
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    pub fn nat_status(&self) -> NatStatus {
        self.nat_status
    }

    pub fn subscribed_topics(&self) -> Vec<&str> {
        self.topics.keys().map(|s| s.as_str()).collect()
    }

    fn subscribe_all(&mut self) -> Result<()> {
        for topic in gossip::aztibase_topics_scoped(self.genesis_hex.as_deref()) {
            let topic_str = topic.to_string();
            self.swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .context("Failed to subscribe to topic")?;
            self.topics.insert(topic_str, topic);
        }
        Ok(())
    }

    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm.listen_on(addr).context("Failed to listen")?;
        Ok(())
    }

    fn listen_on_relay_servers(&mut self) {
        for relay_addr in self.relay_servers.clone() {
            let circuit_addr = relay_addr
                .clone()
                .with(libp2p::multiaddr::Protocol::P2pCircuit);
            match self.swarm.listen_on(circuit_addr.clone()) {
                Ok(_) => {
                    info!(%relay_addr, "Listening via relay circuit");
                }
                Err(e) => {
                    warn!(%relay_addr, error = %e, "Failed to listen on relay circuit");
                }
            }
        }
    }

    pub fn publish(&mut self, topic_name: &str, data: Vec<u8>) -> Result<()> {
        let topic = self.topics.get(topic_name).context("Unknown topic")?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), data)
            .context("Failed to publish")?;
        Ok(())
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm.dial(addr).context("Failed to dial")?;
        Ok(())
    }

    pub fn add_peer(&mut self, peer_id: PeerId, addr: Multiaddr) {
        self.swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, addr);
    }

    pub async fn next_event(&mut self) -> NetworkEvent {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    },
                )) => {
                    if let Some(ref rep_store) = self.reputation {
                        let peer_bytes = propagation_source.to_bytes();
                        if rep_store.is_banned(&peer_bytes).unwrap_or(false) {
                            let _ = self.swarm.disconnect_peer_id(propagation_source);
                            continue;
                        }
                    }
                    let topic_str = message.topic.to_string();
                    match gossip::validate_gossip_message(&topic_str, &message.data) {
                        gossip::MessageAcceptance::Accept => {
                            return NetworkEvent::Message {
                                source: propagation_source,
                                topic: topic_str,
                                data: message.data,
                            };
                        }
                        gossip::MessageAcceptance::Reject => {
                            warn!(
                                %propagation_source,
                                topic = %topic_str,
                                len = message.data.len(),
                                "Rejected invalid gossip message"
                            );
                            self.record_peer_offense(&propagation_source, OffenseSeverity::Medium);
                        }
                        gossip::MessageAcceptance::Ignore => {}
                    }
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Mdns(mdns::Event::Discovered(
                    peers,
                ))) => {
                    for (peer_id, addr) in peers {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, addr);
                    }
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Mdns(mdns::Event::Expired(
                    peers,
                ))) => {
                    for (peer_id, _) in peers {
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .remove_explicit_peer(&peer_id);
                    }
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Autonat(event)) => match event {
                    autonat::Event::StatusChanged { old, new } => {
                        let new_status = match new {
                            autonat::NatStatus::Public(_) => NatStatus::Public,
                            autonat::NatStatus::Private => NatStatus::Private,
                            autonat::NatStatus::Unknown => NatStatus::Unknown,
                        };
                        if new_status != self.nat_status {
                            info!(
                                old = %format!("{old:?}"),
                                new = %new_status,
                                "NAT status changed"
                            );
                            self.nat_status = new_status;

                            if new_status == NatStatus::Private && !self.relay_servers.is_empty() {
                                self.listen_on_relay_servers();
                            }
                        }
                    }
                    autonat::Event::InboundProbe(_) | autonat::Event::OutboundProbe(_) => {}
                },
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::RelayClient(_)) => {}
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Dcutr(event)) => {
                    self.nat_traversal_stats.dcutr_attempts += 1;
                    let remote = event.remote_peer_id;
                    match event.result {
                        Ok(connection_id) => {
                            self.nat_traversal_stats.dcutr_successes += 1;
                            info!(%remote, ?connection_id, "DCUtR direct connection upgrade succeeded");
                        }
                        Err(error) => {
                            self.nat_traversal_stats.dcutr_failures += 1;
                            warn!(%remote, %error, "DCUtR direct connection upgrade failed");
                        }
                    }
                }
                SwarmEvent::IncomingConnectionError { error, .. } => {
                    warn!("Incoming connection denied: {error}");
                }
                SwarmEvent::OutgoingConnectionError { error, peer_id, .. } => {
                    warn!("Outgoing connection denied: {error}");
                    if let (Some(rep_store), Some(pid)) = (&self.reputation, peer_id) {
                        let _ = rep_store.record_offense(&pid.to_bytes(), OffenseSeverity::Low);
                    }
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    return NetworkEvent::Listening(address);
                }
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    if let Some(ref rep_store) = self.reputation {
                        let peer_bytes = peer_id.to_bytes();
                        if rep_store.is_banned(&peer_bytes).unwrap_or(false) {
                            warn!(%peer_id, "Rejecting banned peer");
                            let _ = self.swarm.disconnect_peer_id(peer_id);
                            continue;
                        }
                        let _ = rep_store.record_seen(&peer_bytes);
                        let _ = rep_store.evict_excess();
                    }

                    let remote_addr = endpoint.get_remote_address().clone();
                    if let Some(ip) = ip_from_multiaddr(&remote_addr) {
                        if let Err(reason) = self.conn_filter.try_accept(ip) {
                            warn!(%peer_id, %ip, %reason, "Connection filtered");
                            let _ = self.swarm.disconnect_peer_id(peer_id);
                            continue;
                        }
                        self.peer_ips.insert(peer_id, ip);
                    }

                    if let Some(ref ps) = self.peer_store {
                        let _ = ps.insert(&peer_id.to_bytes(), &[remote_addr.to_string()]);
                    }

                    if !self.kad_bootstrapped
                        && self.swarm.behaviour_mut().kademlia.bootstrap().is_ok()
                    {
                        debug!("Kademlia bootstrap triggered");
                        self.kad_bootstrapped = true;
                    }

                    return NetworkEvent::PeerConnected(peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    if let Some(ip) = self.peer_ips.remove(&peer_id) {
                        self.conn_filter.release(ip);
                    }
                    if let Some(ref ps) = self.peer_store {
                        let _ = ps.update_last_seen(&peer_id.to_bytes());
                    }
                    return NetworkEvent::PeerDisconnected(peer_id);
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::LightSync(
                    request_response::Event::Message { peer, message, .. },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        return NetworkEvent::LightSyncRequest {
                            peer,
                            request,
                            channel,
                        };
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        return NetworkEvent::LightSyncResponse {
                            peer,
                            request_id,
                            response,
                        };
                    }
                },
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::LightSync(
                    request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                        ..
                    },
                )) => {
                    return NetworkEvent::LightSyncOutboundFailure {
                        peer,
                        request_id,
                        error,
                    };
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::LightSync(
                    request_response::Event::ResponseSent { .. },
                )) => {}
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::BlockSync(
                    request_response::Event::Message { peer, message, .. },
                )) => match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        return NetworkEvent::BlockSyncRequest {
                            peer,
                            request,
                            channel,
                        };
                    }
                    request_response::Message::Response {
                        request_id,
                        response,
                    } => {
                        return NetworkEvent::BlockSyncResponse {
                            peer,
                            request_id,
                            response,
                        };
                    }
                },
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::BlockSync(
                    request_response::Event::OutboundFailure {
                        peer,
                        request_id,
                        error,
                        ..
                    },
                )) => {
                    return NetworkEvent::BlockSyncOutboundFailure {
                        peer,
                        request_id,
                        error,
                    };
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::BlockSync(
                    request_response::Event::ResponseSent { .. },
                )) => {}
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Kademlia(
                    kad::Event::RoutingUpdated {
                        peer, addresses, ..
                    },
                )) => {
                    if let Some(ref ps) = self.peer_store {
                        let addr_strs: Vec<String> =
                            addresses.iter().map(|a| a.to_string()).collect();
                        let _ = ps.insert(&peer.to_bytes(), &addr_strs);
                    }
                    debug!(%peer, "Kademlia routing table updated");
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Kademlia(
                    kad::Event::OutboundQueryProgressed {
                        result:
                            kad::QueryResult::Bootstrap(Ok(kad::BootstrapOk { num_remaining, .. })),
                        ..
                    },
                )) => {
                    if num_remaining == 0 {
                        debug!("Kademlia bootstrap complete");
                    }
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Kademlia(_)) => {}
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Identify(
                    identify::Event::Received { peer_id, info, .. },
                )) => {
                    debug!(
                        %peer_id,
                        agent = %info.agent_version,
                        "Identify received"
                    );
                    if let Some(remote_version) = parse_agent_version(&info.agent_version) {
                        if remote_version != PROTOCOL_VERSION {
                            warn!(
                                %peer_id,
                                local = PROTOCOL_VERSION,
                                remote = remote_version,
                                "Protocol version mismatch — disconnecting peer"
                            );
                            let _ = self.swarm.disconnect_peer_id(peer_id);
                        }
                    } else {
                        debug!(
                            %peer_id,
                            agent = %info.agent_version,
                            "Non-Aztibase peer — allowing connection"
                        );
                    }
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Identify(_)) => {}
                _ => {}
            }
        }
    }

    pub fn record_peer_offense(&self, peer_id: &PeerId, severity: OffenseSeverity) {
        if let Some(ref rep_store) = self.reputation {
            let peer_bytes = peer_id.to_bytes();
            match rep_store.record_offense(&peer_bytes, severity) {
                Ok(rep) if rep.banned_until.is_some() => {
                    info!(%peer_id, score = rep.score, "Peer banned after offense");
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(%peer_id, "Failed to record offense: {e}");
                }
            }
        }
    }

    pub fn reputation_store(&self) -> Option<&Arc<PeerReputationStore>> {
        self.reputation.as_ref()
    }

    pub fn peer_store(&self) -> Option<&Arc<PeerStore>> {
        self.peer_store.as_ref()
    }

    pub fn nat_traversal_stats(&self) -> &NatTraversalStats {
        &self.nat_traversal_stats
    }

    pub fn load_cached_peers(&mut self, limit: usize) -> usize {
        let (ps, rep) = match (&self.peer_store, &self.reputation) {
            (Some(ps), rep) => (ps, rep),
            _ => return 0,
        };

        let recent = match ps.list_recent(limit) {
            Ok(peers) => peers,
            Err(e) => {
                warn!("Failed to load cached peers: {e}");
                return 0;
            }
        };

        let mut dialed = 0;
        for (peer_bytes, stored) in &recent {
            if let Some(rep_store) = rep
                && rep_store.is_banned(peer_bytes).unwrap_or(false)
            {
                continue;
            }
            for addr_str in &stored.addrs {
                if let Ok(addr) = addr_str.parse::<Multiaddr>()
                    && self.swarm.dial(addr).is_ok()
                {
                    dialed += 1;
                    break;
                }
            }
        }
        if dialed > 0 {
            info!(count = dialed, "Dialed cached peers from store");
        }
        dialed
    }

    pub fn send_light_sync_request(
        &mut self,
        peer: &PeerId,
        request: LightSyncRequest,
    ) -> OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .light_sync
            .send_request(peer, request)
    }

    pub fn send_light_sync_response(
        &mut self,
        channel: ResponseChannel<LightSyncResponse>,
        response: LightSyncResponse,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .light_sync
            .send_response(channel, response)
            .map_err(|_| anyhow::anyhow!("failed to send light sync response"))
    }

    pub fn send_block_sync_request(
        &mut self,
        peer: &PeerId,
        request: BlockSyncRequest,
    ) -> OutboundRequestId {
        self.swarm
            .behaviour_mut()
            .block_sync
            .send_request(peer, request)
    }

    pub fn send_block_sync_response(
        &mut self,
        channel: ResponseChannel<BlockSyncResponse>,
        response: BlockSyncResponse,
    ) -> Result<()> {
        self.swarm
            .behaviour_mut()
            .block_sync
            .send_response(channel, response)
            .map_err(|_| anyhow::anyhow!("failed to send block sync response"))
    }
}

fn ip_from_multiaddr(addr: &Multiaddr) -> Option<IpAddr> {
    for proto in addr.iter() {
        match proto {
            libp2p::multiaddr::Protocol::Ip4(ip) => return Some(IpAddr::V4(ip)),
            libp2p::multiaddr::Protocol::Ip6(ip) => return Some(IpAddr::V6(ip)),
            _ => {}
        }
    }
    None
}
