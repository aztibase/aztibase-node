use dendrite_core::{BlockHash, Hash, ValidatorId, hash};
use serde::{Deserialize, Serialize};

/// A block in the Dendrite DAG.
///
/// Unlike a traditional linear blockchain where each block has one parent,
/// DAG blocks reference multiple parents. This enables parallel block
/// production by multiple validators in the same round.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DagBlock {
    /// Hash of this block (computed from header fields).
    pub hash: BlockHash,
    /// Round number. Monotonically increasing per validator.
    pub round: u64,
    /// The validator who authored this block.
    pub author: ValidatorId,
    /// Hashes of parent blocks. Must be non-empty (except genesis).
    pub parents: Vec<BlockHash>,
    /// Serialized payload (transactions, attestations, etc.).
    pub payload: Vec<u8>,
    /// Unix timestamp in milliseconds.
    pub timestamp: u64,
}

impl DagBlock {
    /// Create a new DAG block and compute its hash.
    ///
    /// # Errors
    /// Returns `DagError::NoParents` if `parents` is empty and `round > 0`.
    /// Returns `DagError::RoundNotMonotonic` if any parent has `round >= self.round`.
    pub fn new(
        round: u64,
        author: ValidatorId,
        parents: Vec<BlockHash>,
        payload: Vec<u8>,
        timestamp: u64,
    ) -> Result<Self, DagError> {
        // Genesis block (round 0) is the only block allowed to have no parents.
        if round > 0 && parents.is_empty() {
            return Err(DagError::NoParents);
        }

        let hash = Self::compute_hash(round, &author, &parents, &payload, timestamp);

        Ok(Self {
            hash,
            round,
            author,
            parents,
            payload,
            timestamp,
        })
    }

    /// Create the genesis block (round 0, no parents).
    pub fn genesis(author: ValidatorId, timestamp: u64) -> Self {
        let parents = Vec::new();
        let payload = Vec::new();
        let hash = Self::compute_hash(0, &author, &parents, &payload, timestamp);
        Self {
            hash,
            round: 0,
            author,
            parents,
            payload,
            timestamp,
        }
    }

    /// Validate that all parent rounds are strictly less than this block's round.
    /// `parent_rounds` maps parent hash -> parent round.
    pub fn validate_parent_rounds(
        &self,
        parent_rounds: &[(BlockHash, u64)],
    ) -> Result<(), DagError> {
        for (parent_hash, parent_round) in parent_rounds {
            if *parent_round >= self.round {
                return Err(DagError::RoundNotMonotonic {
                    block_round: self.round,
                    parent_hash: *parent_hash,
                    parent_round: *parent_round,
                });
            }
        }
        Ok(())
    }

    /// Check if this block is a genesis block.
    pub fn is_genesis(&self) -> bool {
        self.round == 0 && self.parents.is_empty()
    }

    fn compute_hash(
        round: u64,
        author: &ValidatorId,
        parents: &[BlockHash],
        payload: &[u8],
        timestamp: u64,
    ) -> Hash {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&round.to_le_bytes());
        preimage.extend_from_slice(author);
        for parent in parents {
            preimage.extend_from_slice(parent);
        }
        preimage.extend_from_slice(payload);
        preimage.extend_from_slice(&timestamp.to_le_bytes());
        hash(&preimage)
    }
}

/// Errors from DAG block operations.
#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("Non-genesis block must have at least one parent")]
    NoParents,

    #[error(
        "Round not monotonic: block round {block_round}, parent {parent_hash:?} has round {parent_round}"
    )]
    RoundNotMonotonic {
        block_round: u64,
        parent_hash: BlockHash,
        parent_round: u64,
    },
}
