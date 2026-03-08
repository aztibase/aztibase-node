pub mod behaviour;
pub mod discovery;
pub mod gossip;
pub mod light_sync;
pub mod transport;
#[cfg(feature = "webrtc")]
pub mod webrtc;

pub use gossip::{TOPIC_CONSENSUS, TOPIC_STATE_SYNC, TOPIC_TRANSACTIONS};
pub use libp2p::{Multiaddr, PeerId};
pub use light_sync::{
    LIGHT_SYNC_PROTOCOL, LightSyncCodec, LightSyncMessage, LightSyncProtocol, LightSyncRequest,
    LightSyncResponse, MAX_HEADERS_PER_REQUEST, MultiPeerValidator, SyncFinalityCert, SyncHeader,
    build_header_request, build_header_response, build_proof_request, build_proof_response,
    decode_light_sync, decode_request, decode_response, encode_light_sync, encode_request,
    encode_response, verify_header_chain,
};
pub use transport::{Libp2pTransport, NetworkEvent, TransportConfig};
#[cfg(feature = "webrtc")]
pub use webrtc::{WebRtcConfig, WebRtcTransport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossipsub_config_builds() {
        let config = gossip::gossipsub_config();
        assert!(config.is_ok());
    }

    #[test]
    fn gossipsub_config_values() {
        let config = gossip::gossipsub_config().unwrap();
        assert_eq!(
            config.duplicate_cache_time(),
            std::time::Duration::from_secs(gossip::DUPLICATE_CACHE_SECS)
        );
        assert_eq!(config.max_transmit_size(), gossip::MAX_TRANSMIT_SIZE);
        assert_eq!(
            config.max_messages_per_rpc(),
            Some(gossip::MAX_MESSAGES_PER_RPC)
        );
        assert_eq!(
            config.heartbeat_interval(),
            std::time::Duration::from_millis(gossip::HEARTBEAT_MS)
        );
    }

    #[test]
    fn aztibase_topics_count() {
        let topics = gossip::aztibase_topics();
        assert_eq!(topics.len(), 6);
    }

    #[test]
    fn aztibase_topics_contain_expected_names() {
        let topics = gossip::aztibase_topics();
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

    #[test]
    fn peer_score_params_valid() {
        let params = gossip::peer_score_params();
        assert!(params.validate().is_ok());
        assert_eq!(params.topics.len(), gossip::ALL_TOPICS.len());
    }

    #[test]
    fn peer_score_thresholds_valid() {
        let thresholds = gossip::peer_score_thresholds();
        assert!(thresholds.validate().is_ok());
        assert!(thresholds.gossip_threshold < 0.0);
        assert!(thresholds.publish_threshold <= thresholds.gossip_threshold);
        assert!(thresholds.graylist_threshold <= thresholds.publish_threshold);
    }

    #[tokio::test]
    async fn transport_creates_with_signed_messages() {
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
