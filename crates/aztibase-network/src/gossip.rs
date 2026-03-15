use std::time::Duration;

use libp2p::gossipsub;

pub const TOPIC_BLOCKS: &str = "/aztibase/blocks/1.0.0";
pub const TOPIC_TRANSACTIONS: &str = "/aztibase/transactions/1.0.0";
pub const TOPIC_CONSENSUS: &str = "/aztibase/consensus/1.0.0";
pub const TOPIC_STATE_SYNC: &str = "/aztibase/state-sync/1.0.0";
pub const TOPIC_AI_PROOFS: &str = "/aztibase/ai-proofs/1.0.0";
pub const TOPIC_VALIDATOR_ANNOUNCE: &str = "/aztibase/validator-announce/1.0.0";
pub const TOPIC_CHECKPOINT_ANNOUNCE: &str = "/aztibase/checkpoint-announce/1.0.0";
pub const TOPIC_COMMITTED_BATCHES: &str = "/aztibase/committed-batches/1.0.0";

pub const ALL_TOPICS: &[&str] = &[
    TOPIC_BLOCKS,
    TOPIC_TRANSACTIONS,
    TOPIC_CONSENSUS,
    TOPIC_STATE_SYNC,
    TOPIC_AI_PROOFS,
    TOPIC_VALIDATOR_ANNOUNCE,
    TOPIC_CHECKPOINT_ANNOUNCE,
    TOPIC_COMMITTED_BATCHES,
];

const TOPIC_BASE_NAMES: &[&str] = &[
    "blocks",
    "transactions",
    "consensus",
    "state-sync",
    "ai-proofs",
    "validator-announce",
    "checkpoint-announce",
    "committed-batches",
];

pub const MAX_TRANSMIT_SIZE: usize = 2 * 1024 * 1024; // 2 MiB
pub const MAX_MESSAGES_PER_RPC: usize = 100;
pub const HEARTBEAT_MS: u64 = 500;
pub const DUPLICATE_CACHE_SECS: u64 = 120; // 2 minutes

pub fn genesis_hex_prefix(genesis_hash: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        genesis_hash[0], genesis_hash[1], genesis_hash[2], genesis_hash[3]
    )
}

pub fn chain_scoped_topics(genesis_hex: Option<&str>) -> Vec<String> {
    TOPIC_BASE_NAMES
        .iter()
        .map(|name| match genesis_hex {
            Some(hex) => format!("/aztibase/{name}/1.0.0/{hex}"),
            None => format!("/aztibase/{name}/1.0.0"),
        })
        .collect()
}

pub fn aztibase_topics() -> Vec<gossipsub::IdentTopic> {
    ALL_TOPICS
        .iter()
        .map(|t| gossipsub::IdentTopic::new(*t))
        .collect()
}

pub fn aztibase_topics_scoped(genesis_hex: Option<&str>) -> Vec<gossipsub::IdentTopic> {
    chain_scoped_topics(genesis_hex)
        .into_iter()
        .map(gossipsub::IdentTopic::new)
        .collect()
}

pub fn gossipsub_config() -> Result<gossipsub::Config, String> {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut preimage = Vec::with_capacity(64 + message.data.len());
        if let Some(ref peer) = message.source {
            preimage.extend_from_slice(peer.to_bytes().as_slice());
        }
        preimage.extend_from_slice(message.topic.as_str().as_bytes());
        preimage.extend_from_slice(&message.data);
        let hash = aztibase_core::hash(&preimage);
        gossipsub::MessageId::from(hash.to_vec())
    };

    gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_millis(HEARTBEAT_MS))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .mesh_n(3)
        .mesh_n_low(2)
        .mesh_n_high(12)
        .mesh_outbound_min(1)
        .message_id_fn(message_id_fn)
        .duplicate_cache_time(Duration::from_secs(DUPLICATE_CACHE_SECS))
        .max_transmit_size(MAX_TRANSMIT_SIZE)
        .max_messages_per_rpc(Some(MAX_MESSAGES_PER_RPC))
        .build()
        .map_err(|e| format!("{e}"))
}

fn topic_weight(topic: &str) -> f64 {
    match topic {
        TOPIC_CONSENSUS => 2.0,
        TOPIC_BLOCKS => 1.5,
        TOPIC_VALIDATOR_ANNOUNCE => 1.5,
        TOPIC_TRANSACTIONS => 1.0,
        TOPIC_STATE_SYNC => 0.5,
        TOPIC_AI_PROOFS => 0.5,
        TOPIC_CHECKPOINT_ANNOUNCE => 1.5,
        TOPIC_COMMITTED_BATCHES => 1.5,
        _ => 1.0,
    }
}

fn base_topic_params() -> gossipsub::TopicScoreParams {
    gossipsub::TopicScoreParams {
        topic_weight: 1.0,
        time_in_mesh_weight: 0.5,
        time_in_mesh_quantum: Duration::from_secs(1),
        time_in_mesh_cap: 3600.0,
        first_message_deliveries_weight: 1.0,
        first_message_deliveries_decay: 0.5,
        first_message_deliveries_cap: 2000.0,
        mesh_message_deliveries_weight: -1.0,
        mesh_message_deliveries_decay: 0.5,
        mesh_message_deliveries_cap: 500.0,
        mesh_message_deliveries_threshold: 50.0,
        mesh_message_deliveries_window: Duration::from_millis(10),
        mesh_message_deliveries_activation: Duration::from_secs(5),
        mesh_failure_penalty_weight: -1.0,
        mesh_failure_penalty_decay: 0.5,
        invalid_message_deliveries_weight: -50.0,
        invalid_message_deliveries_decay: 0.1,
    }
}

pub fn peer_score_params() -> gossipsub::PeerScoreParams {
    peer_score_params_scoped(None)
}

pub fn peer_score_params_scoped(genesis_hex: Option<&str>) -> gossipsub::PeerScoreParams {
    let mut params = gossipsub::PeerScoreParams::default();
    let topics = chain_scoped_topics(genesis_hex);
    for topic_str in &topics {
        let mut tp = base_topic_params();
        tp.topic_weight = topic_weight(topic_str);
        let topic = gossipsub::IdentTopic::new(topic_str);
        params.topics.insert(topic.hash(), tp);
    }
    params.behaviour_penalty_weight = -10.0;
    params.behaviour_penalty_threshold = 1.0;
    params.behaviour_penalty_decay = 0.9;
    params
}

const MIN_BLOCK_SIZE: usize = 64;
const MIN_TX_SIZE: usize = 32;
const MIN_CONSENSUS_SIZE: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageAcceptance {
    Accept,
    Reject,
    Ignore,
}

pub fn validate_gossip_message(topic: &str, data: &[u8]) -> MessageAcceptance {
    if data.is_empty() {
        return MessageAcceptance::Reject;
    }

    if data.len() > MAX_TRANSMIT_SIZE {
        return MessageAcceptance::Reject;
    }

    match topic {
        _ if topic.contains("blocks") && data.len() < MIN_BLOCK_SIZE => MessageAcceptance::Reject,
        _ if topic.contains("transactions") && data.len() < MIN_TX_SIZE => {
            MessageAcceptance::Reject
        }
        _ if topic.contains("consensus") && data.len() < MIN_CONSENSUS_SIZE => {
            MessageAcceptance::Reject
        }
        _ => MessageAcceptance::Accept,
    }
}

pub fn peer_score_thresholds() -> gossipsub::PeerScoreThresholds {
    gossipsub::PeerScoreThresholds {
        gossip_threshold: -10.0,
        publish_threshold: -30.0,
        graylist_threshold: -60.0,
        accept_px_threshold: 10.0,
        opportunistic_graft_threshold: 20.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_hex_prefix_format() {
        let mut h = [0u8; 32];
        h[0] = 0xAB;
        h[1] = 0xCD;
        h[2] = 0x12;
        h[3] = 0x34;
        assert_eq!(genesis_hex_prefix(&h), "abcd1234");
    }

    #[test]
    fn chain_scoped_topics_with_genesis() {
        let topics = chain_scoped_topics(Some("deadbeef"));
        assert_eq!(topics.len(), 8);
        for t in &topics {
            assert!(t.ends_with("/deadbeef"), "missing genesis suffix: {t}");
            assert!(t.starts_with("/aztibase/"));
        }
        assert_eq!(topics[0], "/aztibase/blocks/1.0.0/deadbeef");
        assert_eq!(topics[2], "/aztibase/consensus/1.0.0/deadbeef");
    }

    #[test]
    fn chain_scoped_topics_without_genesis() {
        let topics = chain_scoped_topics(None);
        assert_eq!(topics.len(), 8);
        assert_eq!(topics[0], "/aztibase/blocks/1.0.0");
        assert_eq!(topics[1], "/aztibase/transactions/1.0.0");
        for t in &topics {
            assert!(!t.ends_with('/'), "trailing slash: {t}");
        }
    }

    #[test]
    fn different_genesis_produces_different_topics() {
        let a = chain_scoped_topics(Some("aaaaaaaa"));
        let b = chain_scoped_topics(Some("bbbbbbbb"));
        for (ta, tb) in a.iter().zip(b.iter()) {
            assert_ne!(ta, tb);
        }
    }

    #[test]
    fn scoped_topics_returns_ident_topics() {
        let topics = aztibase_topics_scoped(Some("cafe0123"));
        assert_eq!(topics.len(), 8);
        let first = topics[0].to_string();
        assert!(first.contains("cafe0123"));
    }

    #[test]
    fn default_topics_unchanged() {
        let default = aztibase_topics();
        assert_eq!(default.len(), 8);
        let first = default[0].to_string();
        assert_eq!(first, TOPIC_BLOCKS);
    }
}
