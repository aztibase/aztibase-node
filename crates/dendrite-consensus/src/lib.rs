pub mod dag;
pub mod pouw;
pub mod validator;

pub use dag::{DagBlock, DagError};
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
}
