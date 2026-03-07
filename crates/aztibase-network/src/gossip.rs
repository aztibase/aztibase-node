use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use libp2p::gossipsub;

pub const TOPIC_BLOCKS: &str = "/aztibase/blocks/1.0.0";
pub const TOPIC_TRANSACTIONS: &str = "/aztibase/transactions/1.0.0";
pub const TOPIC_CONSENSUS: &str = "/aztibase/consensus/1.0.0";
pub const TOPIC_STATE_SYNC: &str = "/aztibase/state-sync/1.0.0";
pub const TOPIC_AI_PROOFS: &str = "/aztibase/ai-proofs/1.0.0";
pub const TOPIC_VALIDATOR_ANNOUNCE: &str = "/aztibase/validator-announce/1.0.0";

pub const ALL_TOPICS: &[&str] = &[
    TOPIC_BLOCKS,
    TOPIC_TRANSACTIONS,
    TOPIC_CONSENSUS,
    TOPIC_STATE_SYNC,
    TOPIC_AI_PROOFS,
    TOPIC_VALIDATOR_ANNOUNCE,
];

pub const MAX_TRANSMIT_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
pub const MAX_MESSAGES_PER_RPC: usize = 100;
pub const HEARTBEAT_MS: u64 = 500;
pub const DUPLICATE_CACHE_SECS: u64 = 120; // 2 minutes

pub fn aztibase_topics() -> Vec<gossipsub::IdentTopic> {
    ALL_TOPICS
        .iter()
        .map(|t| gossipsub::IdentTopic::new(*t))
        .collect()
}

pub fn gossipsub_config() -> Result<gossipsub::Config, String> {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut hasher = DefaultHasher::new();
        message.data.hash(&mut hasher);
        gossipsub::MessageId::from(hasher.finish().to_string())
    };

    gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_millis(HEARTBEAT_MS))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .mesh_n(8)
        .mesh_n_low(6)
        .mesh_n_high(12)
        .message_id_fn(message_id_fn)
        .duplicate_cache_time(Duration::from_secs(DUPLICATE_CACHE_SECS))
        .max_transmit_size(MAX_TRANSMIT_SIZE)
        .max_messages_per_rpc(Some(MAX_MESSAGES_PER_RPC))
        .build()
        .map_err(|e| format!("{e}"))
}

pub fn peer_score_params() -> gossipsub::PeerScoreParams {
    let topic_params = gossipsub::TopicScoreParams {
        topic_weight: 1.0,
        time_in_mesh_weight: 0.5,
        time_in_mesh_quantum: Duration::from_secs(1),
        time_in_mesh_cap: 3600.0,
        first_message_deliveries_weight: 1.0,
        first_message_deliveries_decay: 0.5,
        first_message_deliveries_cap: 2000.0,
        mesh_message_deliveries_weight: -1.0,
        mesh_message_deliveries_decay: 0.5,
        mesh_message_deliveries_cap: 100.0,
        mesh_message_deliveries_threshold: 20.0,
        mesh_message_deliveries_window: Duration::from_millis(10),
        mesh_message_deliveries_activation: Duration::from_secs(5),
        mesh_failure_penalty_weight: -1.0,
        mesh_failure_penalty_decay: 0.5,
        invalid_message_deliveries_weight: -10.0,
        invalid_message_deliveries_decay: 0.3,
    };

    let mut params = gossipsub::PeerScoreParams::default();
    for topic_str in ALL_TOPICS {
        let topic = gossipsub::IdentTopic::new(*topic_str);
        params.topics.insert(topic.hash(), topic_params.clone());
    }
    params.behaviour_penalty_weight = -10.0;
    params.behaviour_penalty_threshold = 1.0;
    params.behaviour_penalty_decay = 0.9;
    params
}

pub fn peer_score_thresholds() -> gossipsub::PeerScoreThresholds {
    gossipsub::PeerScoreThresholds {
        gossip_threshold: -10.0,
        publish_threshold: -50.0,
        graylist_threshold: -80.0,
        accept_px_threshold: 10.0,
        opportunistic_graft_threshold: 20.0,
    }
}
