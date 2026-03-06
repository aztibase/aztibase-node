pub mod commit;
pub mod dag;
pub mod dag_store;
pub mod engine;
pub mod ordering;
pub mod pouw;
pub mod validator;

pub use commit::{CommitConfig, CommitRule, LeaderStatus};
pub use dag::{DagBlock, DagError};
pub use dag_store::{DagStore, DagStoreError, DagStoreResult};
pub use engine::{ConsensusConfig, ConsensusEngine, ConsensusInput, ConsensusOutput, RoundState};
pub use ordering::{CommittedBatch, extract_committed_batch};
pub use validator::{ValidatorInfo, ValidatorSet};

#[cfg(test)]
mod tests {
    use super::*;
    use dendrite_core::hash;

    // ── Task 5: DagBlock tests ─────────────────────────────────────

    #[test]
    fn test_genesis_block() {
        let author = [1u8; 32];
        let block = DagBlock::genesis(author, 1000);
        assert!(block.is_genesis());
        assert_eq!(block.round, 0);
        assert!(block.parents.is_empty());
        assert_eq!(block.author, author);
    }

    #[test]
    fn test_block_with_parents() {
        let parent_hash = hash(b"parent");
        let author = [2u8; 32];
        let block = DagBlock::new(1, author, vec![parent_hash], vec![1, 2, 3], 2000).unwrap();
        assert!(!block.is_genesis());
        assert_eq!(block.round, 1);
        assert_eq!(block.parents.len(), 1);
        assert_eq!(block.parents[0], parent_hash);
        assert_eq!(block.payload, vec![1, 2, 3]);
    }

    #[test]
    fn test_block_multiple_parents() {
        let p1 = hash(b"parent_a");
        let p2 = hash(b"parent_b");
        let p3 = hash(b"parent_c");
        let block = DagBlock::new(2, [3u8; 32], vec![p1, p2, p3], vec![], 3000).unwrap();
        assert_eq!(block.parents.len(), 3);
        assert_eq!(block.round, 2);
    }

    #[test]
    fn test_non_genesis_requires_parents() {
        let result = DagBlock::new(1, [4u8; 32], vec![], vec![], 4000);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DagError::NoParents));
    }

    #[test]
    fn test_round_monotonicity_valid() {
        let parent_hash = hash(b"parent");
        let block = DagBlock::new(5, [5u8; 32], vec![parent_hash], vec![], 5000).unwrap();
        let result = block.validate_parent_rounds(&[(parent_hash, 4)]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_round_monotonicity_invalid() {
        let parent_hash = hash(b"parent");
        let block = DagBlock::new(5, [6u8; 32], vec![parent_hash], vec![], 6000).unwrap();
        let result = block.validate_parent_rounds(&[(parent_hash, 5)]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DagError::RoundNotMonotonic { .. }
        ));
    }

    #[test]
    fn test_block_hash_deterministic() {
        let author = [7u8; 32];
        let parent = hash(b"p");
        let b1 = DagBlock::new(1, author, vec![parent], vec![42], 7000).unwrap();
        let b2 = DagBlock::new(1, author, vec![parent], vec![42], 7000).unwrap();
        assert_eq!(b1.hash, b2.hash);
    }

    #[test]
    fn test_block_hash_differs_on_different_input() {
        let author = [8u8; 32];
        let parent = hash(b"p");
        let b1 = DagBlock::new(1, author, vec![parent], vec![1], 8000).unwrap();
        let b2 = DagBlock::new(1, author, vec![parent], vec![2], 8000).unwrap();
        assert_ne!(b1.hash, b2.hash);
    }

    // ── Task 6: ValidatorSet tests ─────────────────────────────────

    #[test]
    fn test_validator_add_and_get() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        assert!(vs.add(v1, 100).is_none());
        assert_eq!(vs.get(&v1), Some(100));
        assert_eq!(vs.len(), 1);
        assert_eq!(vs.total_stake(), 100);
    }

    #[test]
    fn test_validator_update_stake() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        vs.add(v1, 100);
        let old = vs.add(v1, 200);
        assert_eq!(old, Some(100));
        assert_eq!(vs.get(&v1), Some(200));
        assert_eq!(vs.total_stake(), 200);
        assert_eq!(vs.len(), 1);
    }

    #[test]
    fn test_validator_remove() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        vs.add(v1, 100);
        let removed = vs.remove(&v1);
        assert_eq!(removed, Some(100));
        assert!(!vs.contains(&v1));
        assert_eq!(vs.total_stake(), 0);
        assert!(vs.is_empty());
    }

    #[test]
    fn test_validator_remove_nonexistent() {
        let mut vs = ValidatorSet::new();
        assert_eq!(vs.remove(&[99u8; 32]), None);
    }

    #[test]
    fn test_supermajority() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];
        vs.add(v1, 100);
        vs.add(v2, 100);
        vs.add(v3, 100);
        // Total: 300. Supermajority needs >200 (strict >2/3).
        assert!(vs.has_supermajority(&[v1, v2, v3])); // 300 > 200
        assert!(!vs.has_supermajority(&[v1, v2])); // 200 is NOT > 200
        assert!(!vs.has_supermajority(&[v1])); // 100 < 200
    }

    #[test]
    fn test_leader_selection_deterministic() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        vs.add(v1, 100);
        vs.add(v2, 100);

        let leader_r0 = vs.leader_for_round(0);
        let leader_r0_again = vs.leader_for_round(0);
        assert_eq!(leader_r0, leader_r0_again);
    }

    #[test]
    fn test_leader_selection_varies_by_round() {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        vs.add(v1, 100);
        vs.add(v2, 100);

        let mut saw_v1 = false;
        let mut saw_v2 = false;
        for round in 0..200 {
            let leader = vs.leader_for_round(round).unwrap();
            if leader == v1 {
                saw_v1 = true;
            }
            if leader == v2 {
                saw_v2 = true;
            }
        }
        assert!(
            saw_v1 && saw_v2,
            "Both validators should be selected as leader"
        );
    }

    #[test]
    fn test_leader_empty_set() {
        let vs = ValidatorSet::new();
        assert_eq!(vs.leader_for_round(0), None);
    }

    #[test]
    fn test_quorum_count() {
        let mut vs = ValidatorSet::new();
        assert_eq!(vs.quorum_count(), 0);

        vs.add([1u8; 32], 100);
        assert_eq!(vs.quorum_count(), 1); // n=1, f=0, quorum=1

        vs.add([2u8; 32], 100);
        assert_eq!(vs.quorum_count(), 2); // n=2, f=0, quorum=2

        vs.add([3u8; 32], 100);
        assert_eq!(vs.quorum_count(), 3); // n=3, f=0, quorum=3

        vs.add([4u8; 32], 100);
        assert_eq!(vs.quorum_count(), 3); // n=4, f=1, quorum=3
    }

    #[test]
    fn test_block_hash_verification() {
        let block = DagBlock::genesis([1u8; 32], 1000);
        assert_eq!(block.compute_hash(), block.hash);
    }

    // ── Task 7: DagStore tests ──────────────────────────────────────

    use dendrite_storage::StateStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_consensus_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn make_dag_store() -> (DagStore, std::path::PathBuf) {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let dag = DagStore::new(store).unwrap();
        (dag, path)
    }

    #[test]
    fn test_dag_store_insert_genesis() {
        let (mut dag, path) = make_dag_store();
        let genesis = DagBlock::genesis([1u8; 32], 1000);
        let gh = genesis.hash;
        dag.insert(genesis).unwrap();

        assert!(dag.contains(&gh));
        assert_eq!(dag.len(), 1);
        assert_eq!(dag.round_of(&gh).unwrap(), 0);
        assert_eq!(dag.blocks_at_round(0).len(), 1);

        let retrieved = dag.get(&gh).unwrap();
        assert_eq!(retrieved.hash, gh);
        assert!(retrieved.is_genesis());

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_insert_with_parents() {
        let (mut dag, path) = make_dag_store();
        let g1 = DagBlock::genesis([1u8; 32], 1000);
        let g2 = DagBlock::genesis([2u8; 32], 1000);
        let g1h = g1.hash;
        let g2h = g2.hash;
        dag.insert(g1).unwrap();
        dag.insert(g2).unwrap();

        let child = DagBlock::new(1, [1u8; 32], vec![g1h, g2h], vec![], 2000).unwrap();
        let ch = child.hash;
        dag.insert(child).unwrap();

        assert_eq!(dag.len(), 3);
        assert_eq!(dag.parents(&ch).unwrap(), &[g1h, g2h]);
        assert!(dag.children(&g1h).unwrap().contains(&ch));
        assert!(dag.children(&g2h).unwrap().contains(&ch));

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_rejects_missing_parent() {
        let (mut dag, path) = make_dag_store();
        let fake_parent = hash(b"nonexistent");
        let block = DagBlock::new(1, [1u8; 32], vec![fake_parent], vec![], 2000).unwrap();
        let result = dag.insert(block);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DagStoreError::MissingParent { .. }
        ));

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_rejects_duplicate() {
        let (mut dag, path) = make_dag_store();
        let genesis = DagBlock::genesis([1u8; 32], 1000);
        let g = genesis.clone();
        dag.insert(genesis).unwrap();
        let result = dag.insert(g);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            DagStoreError::DuplicateBlock(_)
        ));

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_ancestor_check() {
        let (mut dag, path) = make_dag_store();

        let g = DagBlock::genesis([1u8; 32], 1000);
        let gh = g.hash;
        dag.insert(g).unwrap();

        let b1 = DagBlock::new(1, [2u8; 32], vec![gh], vec![], 2000).unwrap();
        let b1h = b1.hash;
        dag.insert(b1).unwrap();

        let b2 = DagBlock::new(2, [3u8; 32], vec![b1h], vec![], 3000).unwrap();
        let b2h = b2.hash;
        dag.insert(b2).unwrap();

        assert!(dag.is_ancestor(&gh, &b2h));
        assert!(dag.is_ancestor(&b1h, &b2h));
        assert!(dag.is_ancestor(&b2h, &b2h));
        assert!(!dag.is_ancestor(&b2h, &gh));

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_causal_order() {
        let (mut dag, path) = make_dag_store();

        let g1 = DagBlock::genesis([1u8; 32], 1000);
        let g2 = DagBlock::genesis([2u8; 32], 1000);
        let g1h = g1.hash;
        let g2h = g2.hash;
        dag.insert(g1).unwrap();
        dag.insert(g2).unwrap();

        let b1 = DagBlock::new(1, [1u8; 32], vec![g1h, g2h], vec![], 2000).unwrap();
        let b1h = b1.hash;
        dag.insert(b1).unwrap();

        let order = dag.causal_order(&[b1h]).unwrap();
        assert_eq!(order.len(), 3);
        // Genesis blocks must come before the child.
        let b1_pos = order.iter().position(|h| *h == b1h).unwrap();
        let g1_pos = order.iter().position(|h| *h == g1h).unwrap();
        let g2_pos = order.iter().position(|h| *h == g2h).unwrap();
        assert!(g1_pos < b1_pos);
        assert!(g2_pos < b1_pos);

        cleanup(&path);
    }

    #[test]
    fn test_dag_store_highest_round() {
        let (mut dag, path) = make_dag_store();
        assert_eq!(dag.highest_round(), None);

        let g = DagBlock::genesis([1u8; 32], 1000);
        let gh = g.hash;
        dag.insert(g).unwrap();
        assert_eq!(dag.highest_round(), Some(0));

        let b = DagBlock::new(1, [2u8; 32], vec![gh], vec![], 2000).unwrap();
        dag.insert(b).unwrap();
        assert_eq!(dag.highest_round(), Some(1));

        cleanup(&path);
    }

    // ── Task 8: CommitRule tests ────────────────────────────────────

    fn build_3validator_set() -> (ValidatorSet, [u8; 32], [u8; 32], [u8; 32]) {
        let mut vs = ValidatorSet::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];
        vs.add(v1, 100);
        vs.add(v2, 100);
        vs.add(v3, 100);
        (vs, v1, v2, v3)
    }

    #[test]
    fn test_direct_commit_with_supermajority() {
        let (mut dag, path) = make_dag_store();
        let (vs, v1, v2, v3) = build_3validator_set();
        let config = CommitConfig {
            wave_length: 2,
            vrf_seed: None,
        };

        // Wave 0: leader round = 0, voting round = 1.
        // Elect leader for round 0 -- use whoever the ValidatorSet picks.
        let leader = vs.leader_for_round(0).unwrap();
        let leader_block = DagBlock::genesis(leader, 1000);
        let lh = leader_block.hash;
        dag.insert(leader_block).unwrap();

        // All other validators also produce genesis blocks.
        let others: Vec<[u8; 32]> = [v1, v2, v3].into_iter().filter(|v| *v != leader).collect();
        for v in &others {
            dag.insert(DagBlock::genesis(*v, 1000)).unwrap();
        }

        // Voting round (round 1): all 3 validators reference the leader block.
        let voting_blocks: Vec<DagBlock> = [v1, v2, v3]
            .iter()
            .map(|v| DagBlock::new(1, *v, vec![lh], vec![], 2000).unwrap())
            .collect();
        for vb in voting_blocks {
            dag.insert(vb).unwrap();
        }

        let rule = CommitRule::new(&dag, &vs, config);
        let status = rule.try_direct_commit(0);
        assert_eq!(status, LeaderStatus::Commit(lh));

        cleanup(&path);
    }

    #[test]
    fn test_direct_skip_no_leader_block() {
        let (dag, path) = make_dag_store();
        let (vs, _, _, _) = build_3validator_set();
        let config = CommitConfig {
            wave_length: 2,
            vrf_seed: None,
        };

        // No blocks at all -- leader block missing.
        let rule = CommitRule::new(&dag, &vs, config);
        let status = rule.try_direct_commit(0);
        assert!(matches!(status, LeaderStatus::Skip(0)));

        cleanup(&path);
    }

    #[test]
    fn test_indirect_commit_via_anchor() {
        let (mut dag, path) = make_dag_store();
        let (vs, v1, v2, v3) = build_3validator_set();
        let config = CommitConfig {
            wave_length: 2,
            vrf_seed: None,
        };

        // Wave 0: leader at round 0.
        let leader_w0 = vs.leader_for_round(0).unwrap();
        let lb0 = DagBlock::genesis(leader_w0, 1000);
        let lh0 = lb0.hash;
        dag.insert(lb0).unwrap();

        let others: Vec<[u8; 32]> = [v1, v2, v3]
            .into_iter()
            .filter(|v| *v != leader_w0)
            .collect();
        for v in &others {
            dag.insert(DagBlock::genesis(*v, 1000)).unwrap();
        }

        // Round 1: only 1 validator votes for wave-0 leader (not enough for direct).
        let vb1 = DagBlock::new(1, v1, vec![lh0], vec![], 2000).unwrap();
        let vb1h = vb1.hash;
        dag.insert(vb1).unwrap();

        // Wave 1: leader at round 2.
        let leader_w1 = vs.leader_for_round(2).unwrap();
        let lb1 = DagBlock::new(2, leader_w1, vec![vb1h], vec![], 3000).unwrap();
        let lh1 = lb1.hash;
        dag.insert(lb1).unwrap();

        // The wave-0 leader is in the causal history of wave-1 leader.
        assert!(dag.is_ancestor(&lh0, &lh1));

        let rule = CommitRule::new(&dag, &vs, config);
        let status = rule.try_indirect_commit(0, &lh1);
        assert_eq!(status, LeaderStatus::Commit(lh0));

        cleanup(&path);
    }

    #[test]
    fn test_vrf_leader_deterministic() {
        let mut vs = ValidatorSet::new();
        vs.add([1u8; 32], 100);
        vs.add([2u8; 32], 100);
        vs.add([3u8; 32], 100);

        let seed = [42u8; 32];
        let l1 = vs.vrf_leader_for_round(0, &seed);
        let l2 = vs.vrf_leader_for_round(0, &seed);
        assert_eq!(l1, l2);
    }

    #[test]
    fn test_vrf_leader_varies_by_seed() {
        let mut vs = ValidatorSet::new();
        vs.add([1u8; 32], 100);
        vs.add([2u8; 32], 100);
        vs.add([3u8; 32], 100);

        let mut seen = std::collections::HashSet::new();
        for i in 0..100u8 {
            let seed = hash(&[i]);
            let leader = vs.vrf_leader_for_round(0, &seed).unwrap();
            seen.insert(leader);
        }
        assert!(
            seen.len() > 1,
            "VRF should produce varied leaders across seeds"
        );
    }

    #[test]
    fn test_vrf_leader_varies_by_round() {
        let mut vs = ValidatorSet::new();
        vs.add([1u8; 32], 100);
        vs.add([2u8; 32], 100);
        vs.add([3u8; 32], 100);

        let seed = [99u8; 32];
        let mut seen = std::collections::HashSet::new();
        for round in 0..100 {
            let leader = vs.vrf_leader_for_round(round, &seed).unwrap();
            seen.insert(leader);
        }
        assert!(
            seen.len() > 1,
            "VRF should produce varied leaders across rounds"
        );
    }

    #[test]
    fn test_vrf_leader_empty_set() {
        let vs = ValidatorSet::new();
        assert_eq!(vs.vrf_leader_for_round(0, &[0u8; 32]), None);
    }

    #[test]
    fn test_commit_rule_uses_vrf_when_seeded() {
        let (mut dag, path) = make_dag_store();
        let (vs, v1, v2, v3) = build_3validator_set();
        let seed = [77u8; 32];
        let config = CommitConfig {
            wave_length: 2,
            vrf_seed: Some(seed),
        };

        let leader = vs.vrf_leader_for_round(0, &seed).unwrap();
        let leader_block = DagBlock::genesis(leader, 1000);
        let lh = leader_block.hash;
        dag.insert(leader_block).unwrap();

        let others: Vec<[u8; 32]> = [v1, v2, v3].into_iter().filter(|v| *v != leader).collect();
        for v in &others {
            dag.insert(DagBlock::genesis(*v, 1000)).unwrap();
        }

        let voting_blocks: Vec<DagBlock> = [v1, v2, v3]
            .iter()
            .map(|v| DagBlock::new(1, *v, vec![lh], vec![], 2000).unwrap())
            .collect();
        for vb in voting_blocks {
            dag.insert(vb).unwrap();
        }

        let rule = CommitRule::new(&dag, &vs, config);
        let status = rule.try_direct_commit(0);
        assert_eq!(status, LeaderStatus::Commit(lh));

        cleanup(&path);
    }
}
