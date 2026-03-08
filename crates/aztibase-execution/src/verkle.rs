use aztibase_core::commitment::{StateCommitment, StateProof, VerkleProof, VerkleProofLevel};
use aztibase_core::hash;

/// Width of Verkle tree inner nodes.
/// Matches key byte width: each byte of the 32-byte key addresses one level.
const NODE_WIDTH: usize = 256;

const VERKLE_INNER_DOMAIN: &[u8] = b"AZTB_VERKLE_INNER\0";
const VERKLE_LEAF_DOMAIN: &[u8] = b"AZTB_VERKLE_LEAF\0";

#[derive(Clone, Debug, Default)]
enum VerkleNode {
    Inner {
        children: Vec<Option<Box<VerkleNode>>>,
        commitment: [u8; 32],
    },
    Leaf {
        stem: Vec<u8>,
        value_hash: [u8; 32],
    },
    #[default]
    Empty,
}

impl VerkleNode {
    fn new_inner() -> Self {
        Self::Inner {
            children: (0..NODE_WIDTH).map(|_| None).collect(),
            commitment: [0u8; 32],
        }
    }

    fn commitment(&self) -> [u8; 32] {
        match self {
            VerkleNode::Inner { commitment, .. } => *commitment,
            VerkleNode::Leaf { stem, value_hash } => leaf_commitment(stem, value_hash),
            VerkleNode::Empty => [0u8; 32],
        }
    }
}

fn leaf_commitment(stem: &[u8], value_hash: &[u8; 32]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(VERKLE_LEAF_DOMAIN.len() + stem.len() + 32);
    buf.extend_from_slice(VERKLE_LEAF_DOMAIN);
    buf.extend_from_slice(stem);
    buf.extend_from_slice(value_hash);
    hash(&buf)
}

/// Compute an inner node commitment from 256 child commitments.
/// Uses domain-separated BLAKE3: H(VERKLE_INNER_DOMAIN || c0 || c1 || ... || c255).
///
/// In a full IPA Verkle tree this would be a Pedersen vector commitment
/// over the Banderwagon curve; the BLAKE3 version is a binding (but not
/// hiding) placeholder that preserves the same verification structure.
fn inner_commitment(child_commitments: &[[u8; 32]; 256]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(VERKLE_INNER_DOMAIN.len() + 256 * 32);
    buf.extend_from_slice(VERKLE_INNER_DOMAIN);
    for c in child_commitments {
        buf.extend_from_slice(c);
    }
    hash(&buf)
}

/// Verkle tree using domain-separated BLAKE3 hash commitments as a
/// binding placeholder for IPA polynomial commitments. The tree structure
/// (width-256 inner nodes, stem-based key navigation) matches the
/// production design; only the commitment math is simplified.
pub struct VerkleTree {
    root: VerkleNode,
    leaf_count: usize,
}

impl VerkleTree {
    pub fn new() -> Self {
        Self {
            root: VerkleNode::Empty,
            leaf_count: 0,
        }
    }

    pub fn insert(&mut self, key: &[u8; 32], value_hash: [u8; 32]) {
        let stem: Vec<u8> = key.to_vec();
        self.root = Self::insert_node(std::mem::take(&mut self.root), &stem, value_hash, 0);
        self.leaf_count += 1;
        self.recompute_commitments();
    }

    fn insert_node(
        node: VerkleNode,
        stem: &[u8],
        value_hash: [u8; 32],
        depth: usize,
    ) -> VerkleNode {
        if depth >= stem.len() {
            return VerkleNode::Leaf {
                stem: stem.to_vec(),
                value_hash,
            };
        }

        match node {
            VerkleNode::Empty => VerkleNode::Leaf {
                stem: stem.to_vec(),
                value_hash,
            },
            VerkleNode::Leaf {
                stem: existing_stem,
                value_hash: existing_value,
            } => {
                if existing_stem == stem {
                    return VerkleNode::Leaf {
                        stem: stem.to_vec(),
                        value_hash,
                    };
                }
                let mut inner = VerkleNode::new_inner();
                inner = Self::insert_node(inner, &existing_stem, existing_value, depth);
                Self::insert_node(inner, stem, value_hash, depth)
            }
            VerkleNode::Inner {
                mut children,
                commitment: _,
            } => {
                let idx = stem[depth] as usize;
                let child = children[idx].take().map_or(VerkleNode::Empty, |b| *b);
                let new_child = Self::insert_node(child, stem, value_hash, depth + 1);
                children[idx] = Some(Box::new(new_child));
                VerkleNode::Inner {
                    children,
                    commitment: [0u8; 32],
                }
            }
        }
    }

    fn recompute_commitments(&mut self) {
        Self::compute_commitment(&mut self.root);
    }

    fn compute_commitment(node: &mut VerkleNode) -> [u8; 32] {
        match node {
            VerkleNode::Empty => [0u8; 32],
            VerkleNode::Leaf { stem, value_hash } => leaf_commitment(stem, value_hash),
            VerkleNode::Inner {
                children,
                commitment,
            } => {
                let mut child_commits = [[0u8; 32]; 256];
                for (i, child) in children.iter_mut().enumerate() {
                    child_commits[i] = match child {
                        Some(c) => Self::compute_commitment(c),
                        None => [0u8; 32],
                    };
                }
                *commitment = inner_commitment(&child_commits);
                *commitment
            }
        }
    }

    pub fn root_hash(&self) -> [u8; 32] {
        self.root.commitment()
    }

    pub fn len(&self) -> usize {
        self.leaf_count
    }

    pub fn is_empty(&self) -> bool {
        self.leaf_count == 0
    }

    /// Generate a verifiable opening proof for a key.
    /// Each level of the proof carries all 256 sibling commitments so the
    /// verifier can independently recompute inner-node commitments.
    /// Generate a verifiable opening proof for a key.
    pub fn prove(&self, key: &[u8; 32], value_hash: &[u8; 32]) -> Option<VerkleProof> {
        let stem: Vec<u8> = key.to_vec();
        let mut levels = Vec::new();
        Self::collect_proof(&self.root, &stem, 0, &mut levels)?;
        Some(VerkleProof {
            leaf_hash: hash(key),
            stem: stem.clone(),
            value_hash: *value_hash,
            levels,
        })
    }

    fn collect_proof(
        node: &VerkleNode,
        stem: &[u8],
        depth: usize,
        levels: &mut Vec<VerkleProofLevel>,
    ) -> Option<()> {
        match node {
            VerkleNode::Empty => None,
            VerkleNode::Leaf {
                stem: existing_stem,
                ..
            } => {
                if existing_stem == stem {
                    Some(())
                } else {
                    None
                }
            }
            VerkleNode::Inner { children, .. } => {
                if depth >= stem.len() {
                    return None;
                }
                let idx = stem[depth] as usize;

                let child_commitments: Vec<[u8; 32]> = children
                    .iter()
                    .map(|c| c.as_ref().map_or([0u8; 32], |n| n.commitment()))
                    .collect();

                levels.push(VerkleProofLevel {
                    child_index: idx as u8,
                    child_commitments,
                });

                let child = children[idx].as_ref()?;
                Self::collect_proof(child, stem, depth + 1, levels)
            }
        }
    }

    /// Verify an opening proof against a root hash.
    ///
    /// Recomputes inner-node commitments bottom-up and checks the
    /// reconstructed root matches `expected_root`. For leaf-as-root
    /// trees (no inner nodes), verifies the leaf commitment directly.
    pub fn verify_proof(expected_root: &[u8; 32], proof: &VerkleProof) -> bool {
        let leaf_commit = leaf_commitment(&proof.stem, &proof.value_hash);

        if proof.levels.is_empty() {
            return leaf_commit == *expected_root;
        }

        let mut expected_child = leaf_commit;

        for level in proof.levels.iter().rev() {
            if level.child_commitments.len() != NODE_WIDTH {
                return false;
            }
            if level.child_commitments[level.child_index as usize] != expected_child {
                return false;
            }
            let arr: [_; 256] = level.child_commitments.as_slice().try_into().unwrap();
            expected_child = inner_commitment(&arr);
        }

        expected_child == *expected_root
    }
}

impl Default for VerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Verkle commitment backend implementing the StateCommitment trait.
pub struct VerkleCommitment;

impl StateCommitment for VerkleCommitment {
    fn commit(&self, leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }
        let mut tree = VerkleTree::new();
        for (i, leaf) in leaves.iter().enumerate() {
            let key = hash(&(i as u64).to_le_bytes());
            tree.insert(&key, *leaf);
        }
        tree.root_hash()
    }

    fn prove(&self, leaves: &[[u8; 32]], index: usize) -> Option<StateProof> {
        if index >= leaves.len() || leaves.is_empty() {
            return None;
        }
        let mut tree = VerkleTree::new();
        for (i, leaf) in leaves.iter().enumerate() {
            let key = hash(&(i as u64).to_le_bytes());
            tree.insert(&key, *leaf);
        }
        let key = hash(&(index as u64).to_le_bytes());
        tree.prove(&key, &leaves[index]).map(StateProof::Verkle)
    }

    fn verify(&self, root: &[u8; 32], _leaf: &[u8; 32], proof: &StateProof) -> bool {
        let StateProof::Verkle(vp) = proof else {
            return false;
        };
        VerkleTree::verify_proof(root, vp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verkle_insert_and_commit() {
        let mut tree = VerkleTree::new();
        let key1 = hash(b"account_1");
        let key2 = hash(b"account_2");
        tree.insert(&key1, hash(b"value_1"));
        tree.insert(&key2, hash(b"value_2"));

        assert_eq!(tree.len(), 2);
        assert_ne!(tree.root_hash(), [0u8; 32]);
    }

    #[test]
    fn verkle_deterministic() {
        let mut t1 = VerkleTree::new();
        let mut t2 = VerkleTree::new();

        let k1 = hash(b"a");
        let k2 = hash(b"b");
        let v1 = hash(b"v1");
        let v2 = hash(b"v2");

        t1.insert(&k1, v1);
        t1.insert(&k2, v2);
        t2.insert(&k1, v1);
        t2.insert(&k2, v2);

        assert_eq!(t1.root_hash(), t2.root_hash());
    }

    #[test]
    fn verkle_proof_roundtrip() {
        let mut tree = VerkleTree::new();
        let keys: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let values: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i + 100])).collect();
        for (key, val) in keys.iter().zip(values.iter()) {
            tree.insert(key, *val);
        }

        let root = tree.root_hash();
        for (i, (key, val)) in keys.iter().zip(values.iter()).enumerate() {
            let proof = tree.prove(key, val).expect("proof should exist");
            assert!(
                VerkleTree::verify_proof(&root, &proof),
                "proof must verify for key index {i}"
            );
        }
    }

    #[test]
    fn verkle_proof_rejects_wrong_root() {
        let mut tree = VerkleTree::new();
        let key = hash(b"account");
        let val = hash(b"value");
        tree.insert(&key, val);
        let root = tree.root_hash();
        let proof = tree.prove(&key, &val).unwrap();

        let fake_root = hash(b"tampered");
        assert!(!VerkleTree::verify_proof(&fake_root, &proof));
        assert!(VerkleTree::verify_proof(&root, &proof));
    }

    #[test]
    fn verkle_proof_rejects_wrong_value() {
        let mut tree = VerkleTree::new();
        let key = hash(b"account");
        let val = hash(b"value");
        tree.insert(&key, val);
        let root = tree.root_hash();
        let proof = tree.prove(&key, &val).unwrap();

        let mut tampered_proof = proof.clone();
        tampered_proof.value_hash = hash(b"wrong_value");
        assert!(!VerkleTree::verify_proof(&root, &tampered_proof));
    }

    #[test]
    fn verkle_proof_rejects_tampered_siblings() {
        let mut tree = VerkleTree::new();
        let k1 = hash(b"account_a");
        let k2 = hash(b"account_b");
        let v1 = hash(b"value_a");
        let v2 = hash(b"value_b");
        tree.insert(&k1, v1);
        tree.insert(&k2, v2);
        let root = tree.root_hash();
        let mut proof = tree.prove(&k1, &v1).unwrap();

        if let Some(level) = proof.levels.first_mut() {
            let tamper_idx = if level.child_index == 0 { 1 } else { 0 } as usize;
            level.child_commitments[tamper_idx] = hash(b"tampered");
        }
        assert!(!VerkleTree::verify_proof(&root, &proof));
    }

    #[test]
    fn verkle_proof_rejects_nonexistent_key() {
        let mut tree = VerkleTree::new();
        let key = hash(b"exists");
        let val = hash(b"val");
        tree.insert(&key, val);

        let missing = hash(b"missing");
        assert!(tree.prove(&missing, &val).is_none());
    }

    #[test]
    fn verkle_commitment_trait() {
        let vc = VerkleCommitment;
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let root = vc.commit(&leaves);
        assert_ne!(root, [0u8; 32]);

        let proof = vc.prove(&leaves, 0).unwrap();
        assert!(vc.verify(&root, &leaves[0], &proof));
    }

    #[test]
    fn verkle_commitment_trait_all_leaves() {
        let vc = VerkleCommitment;
        let leaves: Vec<[u8; 32]> = (0..8u8).map(|i| hash(&[i])).collect();
        let root = vc.commit(&leaves);

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = vc.prove(&leaves, i).unwrap();
            assert!(vc.verify(&root, leaf, &proof), "leaf {i} must verify");
        }
    }

    #[test]
    fn verkle_domain_separation() {
        let mut tree = VerkleTree::new();
        let key = hash(b"key");
        let val = hash(b"val");
        tree.insert(&key, val);

        let root = tree.root_hash();
        assert_ne!(root, hash(b"key"));
        assert_ne!(root, val);
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn verkle_single_leaf_proof() {
        let mut tree = VerkleTree::new();
        let key = hash(b"only_key");
        let val = hash(b"only_val");
        tree.insert(&key, val);
        let root = tree.root_hash();

        let proof = tree.prove(&key, &val).unwrap();
        assert!(proof.levels.is_empty(), "leaf-as-root has no inner levels");
        assert!(VerkleTree::verify_proof(&root, &proof));
    }

    #[test]
    fn verkle_empty_proof_rejects() {
        let proof = VerkleProof {
            leaf_hash: hash(b"x"),
            stem: vec![0u8; 32],
            value_hash: [0u8; 32],
            levels: vec![],
        };
        let fake_root = hash(b"not_leaf");
        assert!(!VerkleTree::verify_proof(&fake_root, &proof));
    }
}
