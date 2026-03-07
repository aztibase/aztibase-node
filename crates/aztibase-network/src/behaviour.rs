use libp2p::connection_limits;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{gossipsub, kad, mdns};

#[derive(NetworkBehaviour)]
pub struct AztibaseBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub connection_limits: connection_limits::Behaviour,
}
