use std::collections::BTreeMap;

use aztibase_consensus::{
    AttestationAggregator, InferenceAttestation, InferenceTask, PoUWScore, SlidingWindowPoUWScore,
};
use aztibase_core::Hash;

const MAX_PENDING_TASKS: usize = 1024;

/// Pool of pending inference tasks awaiting validator execution.
pub struct TaskPool {
    tasks: BTreeMap<Hash, InferenceTask>,
    by_model: BTreeMap<String, Vec<Hash>>,
}

impl TaskPool {
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            by_model: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, task: InferenceTask) -> bool {
        if self.tasks.len() >= MAX_PENDING_TASKS {
            return false;
        }
        if self.tasks.contains_key(&task.task_id) {
            return false;
        }
        let task_id = task.task_id;
        let model_id = task.model_id.clone();
        self.tasks.insert(task_id, task);
        self.by_model.entry(model_id).or_default().push(task_id);
        true
    }

    pub fn get(&self, task_id: &Hash) -> Option<&InferenceTask> {
        self.tasks.get(task_id)
    }

    pub fn remove(&mut self, task_id: &Hash) -> Option<InferenceTask> {
        if let Some(task) = self.tasks.remove(task_id) {
            if let Some(ids) = self.by_model.get_mut(&task.model_id) {
                ids.retain(|id| id != task_id);
                if ids.is_empty() {
                    self.by_model.remove(&task.model_id);
                }
            }
            Some(task)
        } else {
            None
        }
    }

    pub fn tasks_for_model(&self, model_id: &str) -> Vec<&InferenceTask> {
        self.by_model
            .get(model_id)
            .map(|ids| ids.iter().filter_map(|id| self.tasks.get(id)).collect())
            .unwrap_or_default()
    }

    /// Remove all expired tasks, returning them for refund processing.
    pub fn drain_expired(&mut self, current_round: u64) -> Vec<InferenceTask> {
        let expired_ids: Vec<Hash> = self
            .tasks
            .values()
            .filter(|t| t.is_expired(current_round))
            .map(|t| t.task_id)
            .collect();
        expired_ids
            .into_iter()
            .filter_map(|id| self.remove(&id))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}

/// Selects the best validator for a given task based on PoUW scores
/// and compute commitments.
pub struct TaskAssigner;

impl TaskAssigner {
    pub fn select_validator(
        candidates: &[[u8; 32]],
        scorer: &SlidingWindowPoUWScore,
        window: u64,
    ) -> Option<[u8; 32]> {
        if candidates.is_empty() {
            return None;
        }

        candidates
            .iter()
            .max_by(|a, b| {
                let sa = scorer.useful_work_score(a, window);
                let sb = scorer.useful_work_score(b, window);
                sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }
}

/// Settles a completed task: verifies attestation quorum and computes
/// the reward split among attesting validators.
pub struct TaskSettlement;

impl TaskSettlement {
    pub fn settle(
        task: &InferenceTask,
        attestations: &[InferenceAttestation],
        aggregator: &AttestationAggregator,
    ) -> SettlementResult {
        match aggregator.aggregate(attestations) {
            Some((result_hash, matching)) => {
                let count = matching.len() as u128;
                let per_validator = task.reward / count;
                let remainder = task.reward % count;

                let payouts: Vec<([u8; 32], u128)> = matching
                    .iter()
                    .enumerate()
                    .map(|(i, att)| {
                        let bonus = if (i as u128) < remainder { 1 } else { 0 };
                        (att.validator_id, per_validator + bonus)
                    })
                    .collect();

                SettlementResult::Settled {
                    task_id: task.task_id,
                    result_hash,
                    payouts,
                }
            }
            None => SettlementResult::NoQuorum {
                task_id: task.task_id,
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum SettlementResult {
    Settled {
        task_id: Hash,
        result_hash: Hash,
        payouts: Vec<([u8; 32], u128)>,
    },
    NoQuorum {
        task_id: Hash,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::hash;

    fn make_task(model: &str, round: u64) -> InferenceTask {
        InferenceTask::new(model.into(), hash(b"input"), [1u8; 32], 1000, round)
    }

    #[test]
    fn task_pool_insert_and_query() {
        let mut pool = TaskPool::new();
        let task = make_task("m1", 100);
        let tid = task.task_id;

        assert!(pool.insert(task));
        assert_eq!(pool.len(), 1);
        assert!(pool.get(&tid).is_some());
        assert_eq!(pool.tasks_for_model("m1").len(), 1);
        assert_eq!(pool.tasks_for_model("m2").len(), 0);
    }

    #[test]
    fn task_pool_rejects_duplicate() {
        let mut pool = TaskPool::new();
        let task = make_task("m1", 100);
        let task2 = task.clone();
        assert!(pool.insert(task));
        assert!(!pool.insert(task2));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn task_pool_remove() {
        let mut pool = TaskPool::new();
        let task = make_task("m1", 100);
        let tid = task.task_id;
        pool.insert(task);

        let removed = pool.remove(&tid).unwrap();
        assert_eq!(removed.task_id, tid);
        assert_eq!(pool.len(), 0);
        assert_eq!(pool.tasks_for_model("m1").len(), 0);
    }

    #[test]
    fn task_pool_drains_expired() {
        let mut pool = TaskPool::new();
        pool.insert(make_task("m1", 10));
        pool.insert(InferenceTask::new(
            "m2".into(),
            hash(b"other"),
            [2u8; 32],
            500,
            50,
        ));

        let drained = pool.drain_expired(20);
        assert_eq!(drained.len(), 1);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn task_pool_capacity_limit() {
        let mut pool = TaskPool::new();
        for i in 0..MAX_PENDING_TASKS {
            let task = InferenceTask::new(
                format!("m{i}"),
                hash(&(i as u64).to_le_bytes()),
                [1u8; 32],
                100,
                1000,
            );
            assert!(pool.insert(task));
        }
        let overflow = make_task("overflow", 1000);
        assert!(!pool.insert(overflow));
    }

    #[test]
    fn task_assigner_picks_highest_scorer() {
        let mut scorer = SlidingWindowPoUWScore::new(1000.0, 10.0);
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];

        for i in 0..5 {
            scorer.record_attestation(aztibase_consensus::AttestationRecord {
                task_id: hash(&[i]),
                validator_id: v1,
                result_hash: hash(b"r"),
                compute_units: 100,
                latency_ms: 100,
                round: i as u64,
                accepted: true,
            });
        }

        for i in 0..5 {
            scorer.record_attestation(aztibase_consensus::AttestationRecord {
                task_id: hash(&[i + 10]),
                validator_id: v2,
                result_hash: hash(b"r"),
                compute_units: 100,
                latency_ms: 800,
                round: i as u64,
                accepted: i % 3 == 0,
            });
        }

        let best = TaskAssigner::select_validator(&[v1, v2], &scorer, 20).unwrap();
        assert_eq!(best, v1);
    }

    #[test]
    fn settlement_distributes_reward() {
        let task = make_task("m1", 100);
        let rh = hash(b"result");
        let a1 = InferenceAttestation::new(task.task_id, rh, 50, [1u8; 32], vec![]);
        let a2 = InferenceAttestation::new(task.task_id, rh, 50, [2u8; 32], vec![]);
        let agg = AttestationAggregator::new(2);

        let atts = [a1, a2];
        let result = TaskSettlement::settle(&task, &atts, &agg);

        match result {
            SettlementResult::Settled { payouts, .. } => {
                assert_eq!(payouts.len(), 2);
                let total: u128 = payouts.iter().map(|(_, v)| v).sum();
                assert_eq!(total, task.reward);
            }
            _ => panic!("Expected Settled"),
        }
    }

    #[test]
    fn settlement_no_quorum_returns_none() {
        let task = make_task("m1", 100);
        let a1 = InferenceAttestation::new(task.task_id, hash(b"r1"), 50, [1u8; 32], vec![]);
        let a2 = InferenceAttestation::new(task.task_id, hash(b"r2"), 50, [2u8; 32], vec![]);
        let agg = AttestationAggregator::new(2);

        let atts = [a1, a2];
        let result = TaskSettlement::settle(&task, &atts, &agg);

        match result {
            SettlementResult::NoQuorum { task_id } => assert_eq!(task_id, task.task_id),
            _ => panic!("Expected NoQuorum"),
        }
    }

    #[test]
    fn settlement_odd_reward_distributes_remainder() {
        let mut task = make_task("m1", 100);
        task.reward = 1001;
        let rh = hash(b"result");
        let a1 = InferenceAttestation::new(task.task_id, rh, 50, [1u8; 32], vec![]);
        let a2 = InferenceAttestation::new(task.task_id, rh, 50, [2u8; 32], vec![]);
        let agg = AttestationAggregator::new(2);

        let atts = [a1, a2];
        let result = TaskSettlement::settle(&task, &atts, &agg);

        match result {
            SettlementResult::Settled { payouts, .. } => {
                let total: u128 = payouts.iter().map(|(_, v)| v).sum();
                assert_eq!(total, 1001);
                assert_eq!(payouts[0].1, 501);
                assert_eq!(payouts[1].1, 500);
            }
            _ => panic!("Expected Settled"),
        }
    }

    #[test]
    fn drain_expired_returns_tasks() {
        let mut pool = TaskPool::new();
        let t1 = make_task("m1", 10);
        let t2 = InferenceTask::new("m2".into(), hash(b"other"), [2u8; 32], 500, 50);
        let t1_id = t1.task_id;
        pool.insert(t1);
        pool.insert(t2);

        let drained = pool.drain_expired(20);
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0].task_id, t1_id);
        assert_eq!(pool.len(), 1);
    }
}
