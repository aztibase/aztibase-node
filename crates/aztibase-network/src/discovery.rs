use std::num::NonZeroUsize;
use std::time::Duration;

use libp2p::kad::store::MemoryStore;
use libp2p::{PeerId, StreamProtocol, kad, mdns};

pub fn kademlia_config() -> kad::Config {
    let mut config = kad::Config::new(StreamProtocol::new("/aztibase/kad/1.0.0"));
    config.set_query_timeout(Duration::from_secs(60));
    if let Some(factor) = NonZeroUsize::new(20) {
        config.set_replication_factor(factor);
    }
    config.disjoint_query_paths(true);
    config
}

pub fn kademlia_behaviour(peer_id: PeerId) -> kad::Behaviour<MemoryStore> {
    let store = MemoryStore::new(peer_id);
    let config = kademlia_config();
    let mut behaviour = kad::Behaviour::with_config(peer_id, store, config);
    behaviour.set_mode(Some(kad::Mode::Server));
    behaviour
}

pub fn mdns_behaviour(peer_id: PeerId) -> std::io::Result<mdns::tokio::Behaviour> {
    mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
}
