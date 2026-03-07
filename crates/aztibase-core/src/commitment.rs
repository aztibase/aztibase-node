use crate::crypto::Hash;

/// A state proof that can be verified against a commitment root.
/// Supports binary Merkle proofs, Verkle proofs, and light client proofs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StateProof {
    Merkle(MerkleProof),
    Verkle(VerkleProof),
    LightClient(LightClientProof),
}

/// Binary Merkle proof: sibling hashes along the path from leaf to root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MerkleProof {
    pub leaf_hash: Hash,
    pub siblings: Vec<(Hash, Side)>,
}

/// Which side the sibling sits on relative to the path node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Verkle proof placeholder: path commitments from leaf to root.
/// Real IPA/KZG opening proofs will replace this in a future sprint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerkleProof {
    pub leaf_hash: Hash,
    pub path_commitments: Vec<Hash>,
}

/// Light client proof: carries the state root, committed height, and a
/// serialized finality certificate so that a light client can verify
/// canonical state without replaying execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LightClientProof {
    pub state_root: Hash,
    pub committed_height: u64,
    /// BLS aggregate signature bitmap + aggregate sig, serialized.
    /// Kept as opaque bytes so aztibase-core doesn't depend on consensus types.
    pub finality_certificate: Vec<u8>,
    /// The inner state proof (Merkle or Verkle) for a specific leaf.
    pub inner_proof: Box<StateProof>,
}

/// Trait for pluggable state commitment schemes.
/// Both Merkle and Verkle backends implement this trait, allowing
/// the execution layer to swap between them.
pub trait StateCommitment {
    fn commit(&self, leaves: &[[u8; 32]]) -> Hash;
    fn prove(&self, leaves: &[[u8; 32]], index: usize) -> Option<StateProof>;
    fn verify(&self, root: &Hash, leaf: &[u8; 32], proof: &StateProof) -> bool;
}
