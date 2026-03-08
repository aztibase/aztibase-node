use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::request_response::{self, OutboundRequestId, ProtocolSupport, ResponseChannel};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm, autonat, connection_limits, gossipsub, mdns};
use tracing::{info, warn};

use crate::behaviour::{AztibaseBehaviour, AztibaseBehaviourEvent};
use crate::connection_filter::ConnectionFilter;
use crate::light_sync::{LIGHT_SYNC_PROTOCOL, LightSyncCodec, LightSyncRequest, LightSyncResponse};
use crate::reputation::{OffenseSeverity, PeerReputationStore};
use crate::{discovery, gossip};

pub const MAX_ESTABLISHED_CONNECTIONS: u32 = 50;
pub const AUTONAT_PROBE_INTERVAL_SECS: u64 = 30;

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

pub struct TransportConfig {
    pub idle_timeout_secs: u64,
    pub reputation_store: Option<Arc<PeerReputationStore>>,
    pub enable_autonat: bool,
    pub relay_servers: Vec<Multiaddr>,
    pub autonat_probe_interval_secs: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 60,
            reputation_store: None,
            enable_autonat: true,
            relay_servers: Vec::new(),
            autonat_probe_interval_secs: AUTONAT_PROBE_INTERVAL_SECS,
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
}

pub struct Libp2pTransport {
    swarm: Swarm<AztibaseBehaviour>,
    topics: HashMap<String, gossipsub::IdentTopic>,
    local_peer_id: PeerId,
    reputation: Option<Arc<PeerReputationStore>>,
    conn_filter: ConnectionFilter,
    peer_ips: HashMap<PeerId, IpAddr>,
    nat_status: NatStatus,
    relay_servers: Vec<Multiaddr>,
}

impl Libp2pTransport {
    pub fn new(config: TransportConfig) -> Result<Self> {
        let probe_interval = Duration::from_secs(config.autonat_probe_interval_secs);
        let relay_servers = config.relay_servers.clone();

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

                gs.with_peer_score(gossip::peer_score_params(), gossip::peer_score_thresholds())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let kademlia = discovery::kademlia_behaviour(peer_id);
                let mdns = discovery::mdns_behaviour(peer_id)?;

                let conn_limits = connection_limits::ConnectionLimits::default()
                    .with_max_established(Some(MAX_ESTABLISHED_CONNECTIONS))
                    .with_max_established_per_peer(Some(2));

                let light_sync = request_response::Behaviour::with_codec(
                    LightSyncCodec,
                    [(LIGHT_SYNC_PROTOCOL, ProtocolSupport::Full)],
                    request_response::Config::default(),
                );

                let autonat_config = autonat::Config {
                    retry_interval: probe_interval,
                    ..Default::default()
                };
                let autonat = autonat::Behaviour::new(peer_id, autonat_config);

                Ok(AztibaseBehaviour {
                    gossipsub: gs,
                    kademlia,
                    mdns,
                    connection_limits: connection_limits::Behaviour::new(conn_limits),
                    light_sync,
                    autonat,
                    relay_client,
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
            conn_filter: ConnectionFilter::new(),
            peer_ips: HashMap::new(),
            nat_status: NatStatus::Unknown,
            relay_servers,
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
        for topic in gossip::aztibase_topics() {
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
            .gossipsub
            .add_explicit_peer(&peer_id);
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
                    return NetworkEvent::Message {
                        source: propagation_source,
                        topic: message.topic.to_string(),
                        data: message.data,
                    };
                }
                SwarmEvent::Behaviour(AztibaseBehaviourEvent::Mdns(mdns::Event::Discovered(
                    peers,
                ))) => {
                    for (peer_id, addr) in peers {
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .add_explicit_peer(&peer_id);
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
                    }

                    if let Some(ip) = ip_from_multiaddr(endpoint.get_remote_address()) {
                        if let Err(reason) = self.conn_filter.try_accept(ip) {
                            warn!(%peer_id, %ip, %reason, "Connection filtered");
                            let _ = self.swarm.disconnect_peer_id(peer_id);
                            continue;
                        }
                        self.peer_ips.insert(peer_id, ip);
                    }

                    return NetworkEvent::PeerConnected(peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    if let Some(ip) = self.peer_ips.remove(&peer_id) {
                        self.conn_filter.release(ip);
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
