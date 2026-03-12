use aztibase_core::{BlockHash, Hash, Keypair, PublicKey, ValidatorId, hash};
use serde::{Deserialize, Serialize};

/// A block in the Aztibase DAG.
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
    /// Ed25519 signature over the block hash, produced by the author.
    /// Empty for genesis blocks.
    pub signature: Vec<u8>,
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
        signing_key: Option<&Keypair>,
    ) -> Result<Self, DagError> {
        if round > 0 && parents.is_empty() {
            return Err(DagError::NoParents);
        }

        let hash = Self::hash_fields(round, &author, &parents, &payload, timestamp);
        let signature = signing_key.map(|kp| kp.sign(&hash)).unwrap_or_default();

        Ok(Self {
            hash,
            round,
            author,
            parents,
            payload,
            timestamp,
            signature,
        })
    }

    /// Create the genesis block (round 0, no parents, no signature).
    pub fn genesis(author: ValidatorId, timestamp: u64) -> Self {
        let parents = Vec::new();
        let payload = Vec::new();
        let hash = Self::hash_fields(0, &author, &parents, &payload, timestamp);
        Self {
            hash,
            round: 0,
            author,
            parents,
            payload,
            timestamp,
            signature: Vec::new(),
        }
    }

    /// Verify that this block's signature is valid for the given public key.
    /// Genesis blocks (empty signature) pass without verification.
    pub fn verify_signature(&self, pubkey: &PublicKey) -> bool {
        if self.is_genesis() {
            return true;
        }
        if self.signature.is_empty() {
            return false;
        }
        pubkey.verify(&self.hash, &self.signature)
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

    /// Recompute the hash from this block's fields for verification.
    pub fn compute_hash(&self) -> Hash {
        Self::hash_fields(
            self.round,
            &self.author,
            &self.parents,
            &self.payload,
            self.timestamp,
        )
    }

    /// Check if this block is a genesis block.
    pub fn is_genesis(&self) -> bool {
        self.round == 0 && self.parents.is_empty()
    }

    fn hash_fields(
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

    #[error("Invalid or missing Ed25519 signature on DAG block")]
    InvalidSignature,
}
