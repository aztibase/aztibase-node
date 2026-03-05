pub mod dag;
pub mod pouw;
pub mod validator;

pub use dag::DagConsensus;

#[cfg(test)]
mod tests {
    use dendrite_core::*;

    #[test]
    fn test_block_references_parents() {
        let genesis_hash = hash(b"genesis");
        let header = BlockHeader {
            version: 1,
            slot: 1,
            round: 1,
            author: [1u8; 32],
            parents: vec![genesis_hash],
            state_root: VerkleRoot::default(),
            transactions_root: [0u8; 32],
            receipts_root: [0u8; 32],
            ai_commitment_root: [0u8; 32],
            timestamp: 1709654400000,
            signature: vec![],
        };
        assert_eq!(header.parents.len(), 1);
        assert_eq!(header.parents[0], genesis_hash);
    }

    #[test]
    fn test_dag_block_multiple_parents() {
        let parent_a = hash(b"block_a");
        let parent_b = hash(b"block_b");
        let parent_c = hash(b"block_c");
        let header = BlockHeader {
            version: 1,
            slot: 2,
            round: 2,
            author: [2u8; 32],
            parents: vec![parent_a, parent_b, parent_c],
            state_root: VerkleRoot::default(),
            transactions_root: [0u8; 32],
            receipts_root: [0u8; 32],
            ai_commitment_root: [0u8; 32],
            timestamp: 1709654400001,
            signature: vec![],
        };
        assert_eq!(header.parents.len(), 3);
        assert_eq!(header.round, 2);
    }
}
