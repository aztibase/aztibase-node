use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::StreamExt;
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId, Swarm, gossipsub, mdns};

use crate::behaviour::{AztibaseBehaviour, AztibaseBehaviourEvent};
use crate::{discovery, gossip};

pub struct TransportConfig {
    pub idle_timeout_secs: u64,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 60,
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
}

pub struct Libp2pTransport {
    swarm: Swarm<AztibaseBehaviour>,
    topics: HashMap<String, gossipsub::IdentTopic>,
    local_peer_id: PeerId,
}

impl Libp2pTransport {
    pub fn new(config: TransportConfig) -> Result<Self> {
        let swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .context("Failed to configure TCP transport")?
            .with_quic()
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();

                let gs_config = gossip::gossipsub_config()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                let gs = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gs_config,
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

                let kademlia = discovery::kademlia_behaviour(peer_id);
                let mdns = discovery::mdns_behaviour(peer_id)?;

                Ok(AztibaseBehaviour {
                    gossipsub: gs,
                    kademlia,
                    mdns,
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
        };

        transport.subscribe_all()?;

        Ok(transport)
    }

    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
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
                SwarmEvent::NewListenAddr { address, .. } => {
                    return NetworkEvent::Listening(address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    return NetworkEvent::PeerConnected(peer_id);
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    return NetworkEvent::PeerDisconnected(peer_id);
                }
                _ => {}
            }
        }
    }
}
