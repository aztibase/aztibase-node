pub mod behaviour;
pub mod discovery;
pub mod gossip;
pub mod transport;

pub use transport::{Libp2pTransport, NetworkEvent, TransportConfig};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossipsub_config_builds() {
        let config = gossip::gossipsub_config();
        assert!(config.is_ok());
    }

    #[test]
    fn dendrite_topics_count() {
        let topics = gossip::dendrite_topics();
        assert_eq!(topics.len(), 6);
    }

    #[test]
    fn dendrite_topics_contain_expected_names() {
        let topics = gossip::dendrite_topics();
        let names: Vec<String> = topics.iter().map(|t| t.to_string()).collect();
        assert!(names.iter().any(|n| n.contains("blocks")));
        assert!(names.iter().any(|n| n.contains("transactions")));
        assert!(names.iter().any(|n| n.contains("consensus")));
    }

    #[test]
    fn kademlia_behaviour_creates() {
        let peer_id = libp2p::PeerId::random();
        let _behaviour = discovery::kademlia_behaviour(peer_id);
    }

    #[tokio::test]
    async fn transport_creates() {
        let config = TransportConfig::default();
        let transport = Libp2pTransport::new(config);
        assert!(transport.is_ok());
    }

    #[tokio::test]
    async fn transport_has_peer_id() {
        let transport = Libp2pTransport::new(TransportConfig::default()).unwrap();
        let peer_id = transport.local_peer_id();
        assert_ne!(peer_id.to_string(), "");
    }

    #[tokio::test]
    async fn transport_subscribes_all_topics() {
        let transport = Libp2pTransport::new(TransportConfig::default()).unwrap();
        assert_eq!(transport.subscribed_topics().len(), 6);
    }

    #[tokio::test]
    async fn transport_listen_tcp() {
        let mut transport = Libp2pTransport::new(TransportConfig::default()).unwrap();
        let addr: libp2p::Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
        let result = transport.listen_on(addr);
        assert!(result.is_ok());

        let event = transport.next_event().await;
        assert!(matches!(event, NetworkEvent::Listening(_)));
    }
}
