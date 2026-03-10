use std::num::NonZeroUsize;
use std::time::Duration;

use libp2p::kad::store::MemoryStore;
use libp2p::{PeerId, StreamProtocol, kad, mdns};

pub fn kademlia_protocol(genesis_hex: Option<&str>) -> String {
    match genesis_hex {
        Some(hex) => format!("/aztibase/kad/1.0.0/{hex}"),
        None => "/aztibase/kad/1.0.0".to_string(),
    }
}

pub fn kademlia_config() -> kad::Config {
    kademlia_config_scoped(None)
}

pub fn kademlia_config_scoped(genesis_hex: Option<&str>) -> kad::Config {
    let protocol = kademlia_protocol(genesis_hex);
    let mut config =
        kad::Config::new(StreamProtocol::try_from_owned(protocol).expect("valid protocol string"));
    config.set_query_timeout(Duration::from_secs(60));
    if let Some(factor) = NonZeroUsize::new(20) {
        config.set_replication_factor(factor);
    }
    config.disjoint_query_paths(true);
    config
}

pub fn kademlia_behaviour(peer_id: PeerId) -> kad::Behaviour<MemoryStore> {
    kademlia_behaviour_scoped(peer_id, None)
}

pub fn kademlia_behaviour_scoped(
    peer_id: PeerId,
    genesis_hex: Option<&str>,
) -> kad::Behaviour<MemoryStore> {
    let store = MemoryStore::new(peer_id);
    let config = kademlia_config_scoped(genesis_hex);
    let mut behaviour = kad::Behaviour::with_config(peer_id, store, config);
    behaviour.set_mode(Some(kad::Mode::Server));
    behaviour
}

pub fn mdns_behaviour(peer_id: PeerId) -> std::io::Result<mdns::tokio::Behaviour> {
    mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kademlia_protocol_with_genesis() {
        let proto = kademlia_protocol(Some("deadbeef"));
        assert_eq!(proto, "/aztibase/kad/1.0.0/deadbeef");
    }

    #[test]
    fn kademlia_protocol_without_genesis() {
        let proto = kademlia_protocol(None);
        assert_eq!(proto, "/aztibase/kad/1.0.0");
    }

    #[test]
    fn different_genesis_different_protocol() {
        let a = kademlia_protocol(Some("aaaaaaaa"));
        let b = kademlia_protocol(Some("bbbbbbbb"));
        assert_ne!(a, b);
    }

    #[test]
    fn kademlia_config_scoped_does_not_panic() {
        let _ = kademlia_config_scoped(Some("cafe0123"));
        let _ = kademlia_config_scoped(None);
    }
}
