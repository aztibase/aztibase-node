use aztibase_core::{Hash, ValidatorId, hash};
use serde::{Deserialize, Serialize};

/// A task requesting AI inference work from the network.
/// Submitted by users, distributed to validators with PoUW capability.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferenceTask {
    pub task_id: Hash,
    pub model_id: String,
    pub input_hash: Hash,
    pub requester: [u8; 32],
    pub reward: u64,
    pub deadline_round: u64,
}

impl InferenceTask {
    pub fn new(
        model_id: String,
        input_hash: Hash,
        requester: [u8; 32],
        reward: u64,
        deadline_round: u64,
    ) -> Self {
        let task_id = compute_task_id(&model_id, &input_hash, &requester);
        Self {
            task_id,
            model_id,
            input_hash,
            requester,
            reward,
            deadline_round,
        }
    }

    pub fn is_expired(&self, current_round: u64) -> bool {
        current_round > self.deadline_round
    }
}

/// A validator's attestation that they completed an inference task.
/// Contains the result hash and a signature for verification.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferenceAttestation {
    pub task_id: Hash,
    pub result_hash: Hash,
    pub compute_units: u64,
    pub validator_id: ValidatorId,
    pub signature: Vec<u8>,
}

impl InferenceAttestation {
    pub fn new(
        task_id: Hash,
        result_hash: Hash,
        compute_units: u64,
        validator_id: ValidatorId,
        signature: Vec<u8>,
    ) -> Self {
        Self {
            task_id,
            result_hash,
            compute_units,
            validator_id,
            signature,
        }
    }

    pub fn attestation_hash(&self) -> Hash {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.task_id);
        buf.extend_from_slice(&self.result_hash);
        buf.extend_from_slice(&self.compute_units.to_le_bytes());
        buf.extend_from_slice(&self.validator_id);
        hash(&buf)
    }
}

/// Trait for scoring a validator's useful work contribution.
/// Returns a value in `[0.0, 1.0]` representing the validator's
/// PoUW reputation over a sliding window.
pub trait PoUWScore: Send + Sync {
    fn useful_work_score(&self, validator: &ValidatorId, window: u64) -> f32;
}

/// Stub implementation that returns 0.0 for all validators.
/// Used until the full PoUW subsystem is operational in M6.
pub struct StubPoUWScore;

impl PoUWScore for StubPoUWScore {
    fn useful_work_score(&self, _validator: &ValidatorId, _window: u64) -> f32 {
        0.0
    }
}

fn compute_task_id(model_id: &str, input_hash: &Hash, requester: &[u8; 32]) -> Hash {
    let mut buf = Vec::new();
    buf.extend_from_slice(model_id.as_bytes());
    buf.extend_from_slice(input_hash);
    buf.extend_from_slice(requester);
    hash(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_task_creation() {
        let input_hash = hash(b"test-input");
        let requester = [1u8; 32];
        let task = InferenceTask::new("sentiment_v1".into(), input_hash, requester, 1000, 100);
        assert_eq!(task.model_id, "sentiment_v1");
        assert_eq!(task.input_hash, input_hash);
        assert_eq!(task.requester, requester);
        assert_eq!(task.reward, 1000);
        assert_eq!(task.deadline_round, 100);
        assert_ne!(task.task_id, [0u8; 32]);
    }

    #[test]
    fn task_expiry() {
        let task = InferenceTask::new("model".into(), hash(b"input"), [1u8; 32], 500, 50);
        assert!(!task.is_expired(50));
        assert!(task.is_expired(51));
        assert!(!task.is_expired(0));
    }

    #[test]
    fn attestation_hash_deterministic() {
        let att = InferenceAttestation::new(
            hash(b"task"),
            hash(b"result"),
            100,
            [2u8; 32],
            vec![0xAA; 64],
        );
        let h1 = att.attestation_hash();
        let h2 = att.attestation_hash();
        assert_eq!(h1, h2);
        assert_ne!(h1, [0u8; 32]);
    }

    #[test]
    fn task_serialization_roundtrip() {
        let task = InferenceTask::new("classifier_v2".into(), hash(b"data"), [3u8; 32], 2000, 200);
        let encoded = bincode::serialize(&task).unwrap();
        let decoded: InferenceTask = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, task);
    }

    #[test]
    fn attestation_serialization_roundtrip() {
        let att = InferenceAttestation::new(
            hash(b"task-123"),
            hash(b"result-456"),
            500,
            [4u8; 32],
            vec![0xBB; 64],
        );
        let encoded = bincode::serialize(&att).unwrap();
        let decoded: InferenceAttestation = bincode::deserialize(&encoded).unwrap();
        assert_eq!(decoded, att);
    }

    #[test]
    fn stub_pouw_score_returns_zero() {
        let scorer = StubPoUWScore;
        let v = [5u8; 32];
        assert_eq!(scorer.useful_work_score(&v, 100), 0.0);
        assert_eq!(scorer.useful_work_score(&v, 0), 0.0);
    }

    #[test]
    fn task_id_deterministic() {
        let t1 = InferenceTask::new("m".into(), hash(b"i"), [1u8; 32], 100, 10);
        let t2 = InferenceTask::new("m".into(), hash(b"i"), [1u8; 32], 100, 10);
        assert_eq!(t1.task_id, t2.task_id);
    }

    #[test]
    fn different_inputs_different_task_id() {
        let t1 = InferenceTask::new("m".into(), hash(b"a"), [1u8; 32], 100, 10);
        let t2 = InferenceTask::new("m".into(), hash(b"b"), [1u8; 32], 100, 10);
        assert_ne!(t1.task_id, t2.task_id);
    }
}
