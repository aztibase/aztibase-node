pub mod behaviour;
pub mod connection_filter;
pub mod discovery;
pub mod gossip;
pub mod light_sync;
pub mod peer_store;
pub mod reputation;
pub mod transport;
#[cfg(feature = "webrtc")]
pub mod webrtc;

pub use connection_filter::{ConnectionFilter, FilterReason};
pub use gossip::{
    MessageAcceptance, TOPIC_CONSENSUS, TOPIC_STATE_SYNC, TOPIC_TRANSACTIONS, chain_scoped_topics,
    genesis_hex_prefix, validate_gossip_message,
};
pub use libp2p::{Multiaddr, PeerId};
pub use light_sync::{
    LIGHT_SYNC_PROTOCOL, LightSyncCodec, LightSyncMessage, LightSyncProtocol, LightSyncRequest,
    LightSyncResponse, MAX_HEADERS_PER_REQUEST, MultiPeerValidator, SyncFinalityCert, SyncHeader,
    build_header_request, build_header_response, build_proof_request, build_proof_response,
    decode_light_sync, decode_request, decode_response, encode_light_sync, encode_request,
    encode_response, verify_header_chain,
};
pub use peer_store::{PeerStore, StoredPeer};
pub use reputation::{OffenseSeverity, PeerReputation, PeerReputationStore};
pub use transport::{
    DEFAULT_WEBRTC_PORT, Libp2pTransport, NatStatus, NatTraversalStats, NetworkEvent,
    TransportConfig,
};
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

    #[test]
    fn topic_weights_differentiated() {
        let params = gossip::peer_score_params();
        let consensus_hash = libp2p::gossipsub::IdentTopic::new(gossip::TOPIC_CONSENSUS).hash();
        let tx_hash = libp2p::gossipsub::IdentTopic::new(gossip::TOPIC_TRANSACTIONS).hash();
        let state_hash = libp2p::gossipsub::IdentTopic::new(gossip::TOPIC_STATE_SYNC).hash();

        let consensus_weight = params.topics[&consensus_hash].topic_weight;
        let tx_weight = params.topics[&tx_hash].topic_weight;
        let state_weight = params.topics[&state_hash].topic_weight;

        assert!(consensus_weight > tx_weight);
        assert!(tx_weight > state_weight);
    }

    #[test]
    fn invalid_message_penalty_is_severe() {
        let params = gossip::peer_score_params();
        let consensus_hash = libp2p::gossipsub::IdentTopic::new(gossip::TOPIC_CONSENSUS).hash();
        let tp = &params.topics[&consensus_hash];
        assert!(tp.invalid_message_deliveries_weight <= -50.0);
        assert!(tp.invalid_message_deliveries_decay <= 0.1);
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

    #[tokio::test]
    async fn transport_initializes_with_autonat() {
        let config = TransportConfig {
            enable_autonat: true,
            ..TransportConfig::default()
        };
        let transport = Libp2pTransport::new(config).unwrap();
        assert_eq!(transport.nat_status(), NatStatus::Unknown);
    }

    #[test]
    fn nat_status_display() {
        assert_eq!(NatStatus::Unknown.to_string(), "unknown");
        assert_eq!(NatStatus::Public.to_string(), "public");
        assert_eq!(NatStatus::Private.to_string(), "private");
    }

    #[tokio::test]
    async fn transport_accepts_relay_servers() {
        let relay: libp2p::Multiaddr =
            "/ip4/127.0.0.1/tcp/4001/p2p/12D3KooWDpJ7As7BWAwRMfu1VU2WCqNjvq387JEYKDBj4kx6nXTN"
                .parse()
                .unwrap();
        let config = TransportConfig {
            relay_servers: vec![relay],
            ..TransportConfig::default()
        };
        let transport = Libp2pTransport::new(config);
        assert!(transport.is_ok());
    }

    #[test]
    fn transport_config_defaults() {
        let config = TransportConfig::default();
        assert!(config.enable_autonat);
        assert!(config.relay_servers.is_empty());
        assert!(config.peer_store.is_none());
        assert_eq!(
            config.autonat_probe_interval_secs,
            transport::AUTONAT_PROBE_INTERVAL_SECS
        );
        assert!(!config.enable_webrtc);
        assert_eq!(config.webrtc_listen_port, DEFAULT_WEBRTC_PORT);
    }

    #[tokio::test]
    async fn transport_webrtc_config_propagates() {
        let config = TransportConfig {
            enable_webrtc: true,
            webrtc_listen_port: 8500,
            ..TransportConfig::default()
        };
        assert!(config.enable_webrtc);
        assert_eq!(config.webrtc_listen_port, 8500);
        let transport = Libp2pTransport::new(config).unwrap();
        assert_ne!(transport.local_peer_id().to_string(), "");
    }

    #[tokio::test]
    async fn transport_has_dcutr_behaviour() {
        let transport = Libp2pTransport::new(TransportConfig::default()).unwrap();
        let stats = transport.nat_traversal_stats();
        assert_eq!(stats.dcutr_attempts, 0);
        assert_eq!(stats.dcutr_successes, 0);
        assert_eq!(stats.dcutr_failures, 0);
    }

    #[test]
    fn nat_traversal_stats_default() {
        let stats = NatTraversalStats::default();
        assert_eq!(stats.dcutr_attempts, 0);
        assert_eq!(stats.dcutr_successes, 0);
        assert_eq!(stats.dcutr_failures, 0);
    }

    #[test]
    fn nat_traversal_stats_clone() {
        let mut stats = NatTraversalStats::default();
        stats.dcutr_attempts = 5;
        stats.dcutr_successes = 3;
        stats.dcutr_failures = 2;
        let cloned = stats.clone();
        assert_eq!(cloned.dcutr_attempts, 5);
        assert_eq!(cloned.dcutr_successes, 3);
        assert_eq!(cloned.dcutr_failures, 2);
    }

    #[test]
    fn validate_rejects_empty_message() {
        let result = gossip::validate_gossip_message(gossip::TOPIC_BLOCKS, &[]);
        assert_eq!(result, gossip::MessageAcceptance::Reject);
    }

    #[test]
    fn validate_rejects_undersized_block() {
        let small = vec![0u8; 10];
        let result = gossip::validate_gossip_message(gossip::TOPIC_BLOCKS, &small);
        assert_eq!(result, gossip::MessageAcceptance::Reject);
    }

    #[test]
    fn validate_accepts_valid_message() {
        let data = vec![0u8; 128];
        let result = gossip::validate_gossip_message(gossip::TOPIC_BLOCKS, &data);
        assert_eq!(result, gossip::MessageAcceptance::Accept);
    }
}
