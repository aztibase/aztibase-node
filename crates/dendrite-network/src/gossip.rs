use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

use libp2p::gossipsub;

pub const TOPIC_BLOCKS: &str = "/dendrite/blocks/1.0.0";
pub const TOPIC_TRANSACTIONS: &str = "/dendrite/transactions/1.0.0";
pub const TOPIC_CONSENSUS: &str = "/dendrite/consensus/1.0.0";
pub const TOPIC_STATE_SYNC: &str = "/dendrite/state-sync/1.0.0";
pub const TOPIC_AI_PROOFS: &str = "/dendrite/ai-proofs/1.0.0";
pub const TOPIC_VALIDATOR_ANNOUNCE: &str = "/dendrite/validator-announce/1.0.0";

pub const ALL_TOPICS: &[&str] = &[
    TOPIC_BLOCKS,
    TOPIC_TRANSACTIONS,
    TOPIC_CONSENSUS,
    TOPIC_STATE_SYNC,
    TOPIC_AI_PROOFS,
    TOPIC_VALIDATOR_ANNOUNCE,
];

pub fn dendrite_topics() -> Vec<gossipsub::IdentTopic> {
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
        .heartbeat_interval(Duration::from_secs(1))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .mesh_n(8)
        .mesh_n_low(6)
        .mesh_n_high(12)
        .message_id_fn(message_id_fn)
        .build()
        .map_err(|e| format!("{e}"))
}
