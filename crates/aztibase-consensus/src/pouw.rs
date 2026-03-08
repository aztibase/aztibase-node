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
    pub assigned_validator: Option<[u8; 32]>,
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
            assigned_validator: None,
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

/// A validator's commitment to provide AI compute for specific models.
/// Validators must register commitments before they can earn PoUW rewards.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeCommitment {
    pub validator_id: ValidatorId,
    pub supported_models: Vec<String>,
    pub committed_stake: u64,
    pub registered_round: u64,
    pub active: bool,
}

impl ComputeCommitment {
    pub fn new(
        validator_id: ValidatorId,
        supported_models: Vec<String>,
        committed_stake: u64,
        current_round: u64,
    ) -> Self {
        Self {
            validator_id,
            supported_models,
            committed_stake,
            registered_round: current_round,
            active: true,
        }
    }

    pub fn supports_model(&self, model_id: &str) -> bool {
        self.active && self.supported_models.iter().any(|m| m == model_id)
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Tracks compute commitments across the validator set.
#[derive(Clone, Debug, Default)]
pub struct ComputeCommitmentStore {
    commitments: std::collections::HashMap<ValidatorId, ComputeCommitment>,
}

impl ComputeCommitmentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, commitment: ComputeCommitment) -> Option<ComputeCommitment> {
        self.commitments.insert(commitment.validator_id, commitment)
    }

    pub fn get(&self, validator: &ValidatorId) -> Option<&ComputeCommitment> {
        self.commitments.get(validator)
    }

    pub fn deregister(&mut self, validator: &ValidatorId) -> Option<ComputeCommitment> {
        if let Some(c) = self.commitments.get_mut(validator) {
            if !c.active {
                return None;
            }
            c.deactivate();
            Some(c.clone())
        } else {
            None
        }
    }

    pub fn validators_for_model(&self, model_id: &str) -> Vec<ValidatorId> {
        self.commitments
            .values()
            .filter(|c| c.supports_model(model_id))
            .map(|c| c.validator_id)
            .collect()
    }

    pub fn active_count(&self) -> usize {
        self.commitments.values().filter(|c| c.active).count()
    }

    pub fn total_committed_stake(&self) -> u64 {
        self.commitments
            .values()
            .filter(|c| c.active)
            .map(|c| c.committed_stake)
            .sum()
    }
}

/// A record of a single validated attestation for scoring purposes.
#[derive(Clone, Debug)]
pub struct AttestationRecord {
    pub task_id: Hash,
    pub validator_id: ValidatorId,
    pub result_hash: Hash,
    pub compute_units: u64,
    pub latency_ms: u64,
    pub round: u64,
    pub accepted: bool,
}

/// Sliding window history of a validator's attestation performance.
#[derive(Clone, Debug, Default)]
pub struct ValidatorWorkHistory {
    records: Vec<AttestationRecord>,
}

impl ValidatorWorkHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, rec: AttestationRecord) {
        self.records.push(rec);
    }

    pub fn records_in_window(&self, current_round: u64, window: u64) -> Vec<&AttestationRecord> {
        let cutoff = current_round.saturating_sub(window);
        self.records.iter().filter(|r| r.round >= cutoff).collect()
    }

    pub fn accuracy_in_window(&self, current_round: u64, window: u64) -> f32 {
        let window_recs = self.records_in_window(current_round, window);
        if window_recs.is_empty() {
            return 0.0;
        }
        let accepted = window_recs.iter().filter(|r| r.accepted).count();
        accepted as f32 / window_recs.len() as f32
    }

    pub fn avg_latency_in_window(&self, current_round: u64, window: u64) -> f32 {
        let window_recs = self.records_in_window(current_round, window);
        if window_recs.is_empty() {
            return f32::MAX;
        }
        let total: u64 = window_recs.iter().map(|r| r.latency_ms).sum();
        total as f32 / window_recs.len() as f32
    }

    pub fn availability_in_window(&self, current_round: u64, window: u64) -> u64 {
        self.records_in_window(current_round, window).len() as u64
    }

    pub fn total_records(&self) -> usize {
        self.records.len()
    }

    pub fn prune_before(&mut self, cutoff_round: u64) {
        self.records.retain(|r| r.round >= cutoff_round);
    }
}

/// Multi-metric PoUW scoring: 0.4 accuracy + 0.3 latency + 0.3 availability.
/// Replaces StubPoUWScore with real performance tracking.
pub struct SlidingWindowPoUWScore {
    histories: std::collections::HashMap<ValidatorId, ValidatorWorkHistory>,
    max_latency_ms: f32,
    max_availability: f32,
}

impl SlidingWindowPoUWScore {
    pub fn new(max_latency_ms: f32, expected_tasks_per_window: f32) -> Self {
        Self {
            histories: std::collections::HashMap::new(),
            max_latency_ms,
            max_availability: expected_tasks_per_window,
        }
    }

    pub fn record_attestation(&mut self, rec: AttestationRecord) {
        self.histories
            .entry(rec.validator_id)
            .or_default()
            .record(rec);
    }

    pub fn history(&self, validator: &ValidatorId) -> Option<&ValidatorWorkHistory> {
        self.histories.get(validator)
    }

    pub fn prune_all(&mut self, cutoff_round: u64) {
        for h in self.histories.values_mut() {
            h.prune_before(cutoff_round);
        }
    }
}

impl PoUWScore for SlidingWindowPoUWScore {
    fn useful_work_score(&self, validator: &ValidatorId, window: u64) -> f32 {
        let history = match self.histories.get(validator) {
            Some(h) => h,
            None => return 0.0,
        };

        let current_round = history.records.last().map(|r| r.round).unwrap_or(0);

        let accuracy = history.accuracy_in_window(current_round, window);

        let avg_latency = history.avg_latency_in_window(current_round, window);
        let latency_score = if avg_latency >= self.max_latency_ms {
            0.0
        } else {
            1.0 - (avg_latency / self.max_latency_ms)
        };

        let availability = history.availability_in_window(current_round, window) as f32;
        let availability_score = (availability / self.max_availability).min(1.0);

        let raw = 0.4 * accuracy + 0.3 * latency_score + 0.3 * availability_score;
        raw.clamp(0.0, 1.0)
    }
}

/// Aggregates attestations for a single task and determines consensus.
/// Requires at least `quorum` matching result hashes to accept.
#[derive(Clone, Debug)]
pub struct AttestationAggregator {
    quorum: usize,
}

impl AttestationAggregator {
    pub fn new(quorum: usize) -> Self {
        Self {
            quorum: quorum.max(2),
        }
    }

    /// Aggregate attestations for a single task.
    /// Returns `Some((result_hash, matching_attestations))` if quorum is met,
    /// `None` if no result hash has enough agreement.
    pub fn aggregate<'a>(
        &self,
        attestations: &'a [InferenceAttestation],
    ) -> Option<(Hash, Vec<&'a InferenceAttestation>)> {
        if attestations.len() < self.quorum {
            return None;
        }

        let mut groups: std::collections::HashMap<Hash, Vec<&InferenceAttestation>> =
            std::collections::HashMap::new();
        let mut seen_validators: std::collections::HashSet<ValidatorId> =
            std::collections::HashSet::new();
        for att in attestations {
            if seen_validators.insert(att.validator_id) {
                groups.entry(att.result_hash).or_default().push(att);
            }
        }

        groups
            .into_iter()
            .filter(|(_, group)| group.len() >= self.quorum)
            .max_by_key(|(_, group)| group.len())
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
    fn compute_commitment_creation() {
        let v = [1u8; 32];
        let c = ComputeCommitment::new(v, vec!["model_a".into(), "model_b".into()], 500, 10);
        assert!(c.active);
        assert!(c.supports_model("model_a"));
        assert!(c.supports_model("model_b"));
        assert!(!c.supports_model("model_c"));
        assert_eq!(c.committed_stake, 500);
    }

    #[test]
    fn compute_commitment_deactivate() {
        let mut c = ComputeCommitment::new([1u8; 32], vec!["m".into()], 100, 0);
        assert!(c.supports_model("m"));
        c.deactivate();
        assert!(!c.supports_model("m"));
        assert!(!c.active);
    }

    #[test]
    fn commitment_store_register_and_query() {
        let mut store = ComputeCommitmentStore::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];

        store.register(ComputeCommitment::new(v1, vec!["m1".into()], 100, 0));
        store.register(ComputeCommitment::new(
            v2,
            vec!["m1".into(), "m2".into()],
            200,
            0,
        ));

        assert_eq!(store.active_count(), 2);
        assert_eq!(store.total_committed_stake(), 300);
        assert_eq!(store.validators_for_model("m1").len(), 2);
        assert_eq!(store.validators_for_model("m2").len(), 1);
        assert_eq!(store.validators_for_model("m3").len(), 0);
    }

    #[test]
    fn sliding_window_score_with_good_validator() {
        let mut scorer = SlidingWindowPoUWScore::new(1000.0, 10.0);
        let v = [1u8; 32];

        for i in 0..10 {
            scorer.record_attestation(AttestationRecord {
                task_id: hash(&[i]),
                validator_id: v,
                result_hash: hash(b"correct"),
                compute_units: 100,
                latency_ms: 200,
                round: i as u64,
                accepted: true,
            });
        }

        let score = scorer.useful_work_score(&v, 20);
        assert!(score > 0.8, "Good validator should score high: {score}");
    }

    #[test]
    fn sliding_window_score_with_bad_validator() {
        let mut scorer = SlidingWindowPoUWScore::new(1000.0, 10.0);
        let v = [2u8; 32];

        for i in 0..10 {
            scorer.record_attestation(AttestationRecord {
                task_id: hash(&[i]),
                validator_id: v,
                result_hash: hash(&[i]),
                compute_units: 100,
                latency_ms: 900,
                round: i as u64,
                accepted: i % 5 == 0,
            });
        }

        let score = scorer.useful_work_score(&v, 20);
        assert!(score < 0.5, "Bad validator should score low: {score}");
    }

    #[test]
    fn sliding_window_unknown_validator_zero() {
        let scorer = SlidingWindowPoUWScore::new(1000.0, 10.0);
        assert_eq!(scorer.useful_work_score(&[99u8; 32], 100), 0.0);
    }

    #[test]
    fn attestation_aggregator_quorum() {
        let agg = AttestationAggregator::new(2);

        let correct_hash = hash(b"result_A");
        let a1 = InferenceAttestation::new(hash(b"task"), correct_hash, 100, [1u8; 32], vec![]);
        let a2 = InferenceAttestation::new(hash(b"task"), correct_hash, 100, [2u8; 32], vec![]);
        let a3 = InferenceAttestation::new(hash(b"task"), hash(b"wrong"), 100, [3u8; 32], vec![]);

        let atts = [a1, a2, a3];
        let result = agg.aggregate(&atts).unwrap();
        assert_eq!(result.0, correct_hash);
        assert_eq!(result.1.len(), 2);
    }

    #[test]
    fn attestation_aggregator_no_quorum() {
        let agg = AttestationAggregator::new(2);
        let a1 = InferenceAttestation::new(hash(b"t"), hash(b"r1"), 100, [1u8; 32], vec![]);
        let a2 = InferenceAttestation::new(hash(b"t"), hash(b"r2"), 100, [2u8; 32], vec![]);

        let atts = [a1, a2];
        assert!(agg.aggregate(&atts).is_none());
    }

    #[test]
    fn attestation_aggregator_insufficient_count() {
        let agg = AttestationAggregator::new(3);
        let a1 = InferenceAttestation::new(hash(b"t"), hash(b"r"), 100, [1u8; 32], vec![]);
        let atts = [a1];
        assert!(agg.aggregate(&atts).is_none());
    }

    #[test]
    fn work_history_pruning() {
        let mut history = ValidatorWorkHistory::new();
        for i in 0..20 {
            history.record(AttestationRecord {
                task_id: hash(&[i]),
                validator_id: [1u8; 32],
                result_hash: hash(b"r"),
                compute_units: 10,
                latency_ms: 50,
                round: i as u64,
                accepted: true,
            });
        }
        assert_eq!(history.total_records(), 20);
        history.prune_before(10);
        assert_eq!(history.total_records(), 10);
    }

    #[test]
    fn commitment_store_deregister() {
        let mut store = ComputeCommitmentStore::new();
        let v1 = [1u8; 32];
        store.register(ComputeCommitment::new(v1, vec!["m1".into()], 100, 0));

        let removed = store.deregister(&v1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().committed_stake, 100);
        assert_eq!(store.active_count(), 0);
        assert_eq!(store.validators_for_model("m1").len(), 0);
        assert!(store.deregister(&[2u8; 32]).is_none());
    }

    #[test]
    fn commitment_store_double_deregister_returns_none() {
        let mut store = ComputeCommitmentStore::new();
        let v1 = [1u8; 32];
        store.register(ComputeCommitment::new(v1, vec!["m1".into()], 200, 0));

        assert!(store.deregister(&v1).is_some());
        assert!(store.deregister(&v1).is_none());
    }

    #[test]
    fn different_inputs_different_task_id() {
        let t1 = InferenceTask::new("m".into(), hash(b"a"), [1u8; 32], 100, 10);
        let t2 = InferenceTask::new("m".into(), hash(b"b"), [1u8; 32], 100, 10);
        assert_ne!(t1.task_id, t2.task_id);
    }

    #[test]
    fn attestation_aggregator_rejects_duplicate_validator() {
        let agg = AttestationAggregator::new(2);
        let rh = hash(b"result");
        let same_validator = [1u8; 32];
        let a1 = InferenceAttestation::new(hash(b"t"), rh, 100, same_validator, vec![]);
        let a2 = InferenceAttestation::new(hash(b"t"), rh, 100, same_validator, vec![]);
        let atts = [a1, a2];
        assert!(
            agg.aggregate(&atts).is_none(),
            "duplicate validator should not count toward quorum"
        );
    }
}
