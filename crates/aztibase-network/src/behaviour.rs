use libp2p::connection_limits;
use libp2p::kad::store::MemoryStore;
use libp2p::request_response;
use libp2p::swarm::NetworkBehaviour;
use libp2p::{autonat, dcutr, gossipsub, identify, kad, mdns, relay};

use crate::block_sync::BlockSyncCodec;
use crate::light_sync::LightSyncCodec;

#[derive(NetworkBehaviour)]
pub struct AztibaseBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub mdns: mdns::tokio::Behaviour,
    pub connection_limits: connection_limits::Behaviour,
    pub light_sync: request_response::Behaviour<LightSyncCodec>,
    pub block_sync: request_response::Behaviour<BlockSyncCodec>,
    pub autonat: autonat::Behaviour,
    pub relay_client: relay::client::Behaviour,
    pub dcutr: dcutr::Behaviour,
    pub identify: identify::Behaviour,
}
