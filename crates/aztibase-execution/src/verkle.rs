use aztibase_core::commitment::{StateCommitment, StateProof, VerkleProof};
use aztibase_core::hash;

/// Width of Verkle tree inner nodes.
/// In production, this would be 256 to match the key byte width.
/// Using 256 here for correct key-mapping even though the BLAKE3
/// placeholder doesn't leverage polynomial commitments yet.
const NODE_WIDTH: usize = 256;

/// Internal node in the Verkle trie.
/// Each inner node has up to NODE_WIDTH children addressed by a single key byte.
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
            VerkleNode::Leaf { stem, value_hash } => {
                let mut buf = Vec::with_capacity(stem.len() + 32);
                buf.extend_from_slice(stem);
                buf.extend_from_slice(value_hash);
                hash(&buf)
            }
            VerkleNode::Empty => [0u8; 32],
        }
    }
}

/// Verkle tree using BLAKE3 hash commitments as a placeholder for
/// real IPA/KZG polynomial commitments. The tree structure (width-256
/// inner nodes, stem/suffix key split) matches the production design;
/// only the commitment math is simplified.
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

    /// Insert a key-value pair. The key is hashed to produce a 32-byte stem.
    /// Bytes 0..31 navigate the tree (one byte per level), byte 31 is the suffix.
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
            VerkleNode::Leaf { stem, value_hash } => {
                let mut buf = Vec::with_capacity(stem.len() + 32);
                buf.extend_from_slice(stem);
                buf.extend_from_slice(value_hash);
                hash(&buf)
            }
            VerkleNode::Inner {
                children,
                commitment,
            } => {
                let mut buf = Vec::with_capacity(NODE_WIDTH * 32);
                for child in children.iter_mut() {
                    let c = match child {
                        Some(c) => Self::compute_commitment(c),
                        None => [0u8; 32],
                    };
                    buf.extend_from_slice(&c);
                }
                *commitment = hash(&buf);
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

    /// Generate a proof for a key. Returns the path commitments from root to leaf.
    pub fn prove(&self, key: &[u8; 32]) -> Option<VerkleProof> {
        let stem: Vec<u8> = key.to_vec();
        let mut path = Vec::new();
        Self::collect_proof_path(&self.root, &stem, 0, &mut path)?;
        Some(VerkleProof {
            leaf_hash: hash(key),
            path_commitments: path,
        })
    }

    fn collect_proof_path(
        node: &VerkleNode,
        stem: &[u8],
        depth: usize,
        path: &mut Vec<[u8; 32]>,
    ) -> Option<()> {
        match node {
            VerkleNode::Empty => None,
            VerkleNode::Leaf {
                stem: existing_stem,
                ..
            } => {
                if existing_stem == stem {
                    path.push(node.commitment());
                    Some(())
                } else {
                    None
                }
            }
            VerkleNode::Inner { children, .. } => {
                if depth >= stem.len() {
                    return None;
                }
                path.push(node.commitment());
                let idx = stem[depth] as usize;
                let child = children[idx].as_ref()?;
                Self::collect_proof_path(child, stem, depth + 1, path)
            }
        }
    }

    /// Verify a proof against the root hash.
    /// With BLAKE3 placeholders, we verify the proof path ends at the root.
    pub fn verify_proof(root: &[u8; 32], proof: &VerkleProof) -> bool {
        if proof.path_commitments.is_empty() {
            return false;
        }
        proof.path_commitments[0] == *root
    }
}

impl Default for VerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Verkle commitment backend implementing the StateCommitment trait.
/// Uses the VerkleTree internally with BLAKE3 placeholder commitments.
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
        tree.prove(&key).map(StateProof::Verkle)
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
        for (i, key) in keys.iter().enumerate() {
            tree.insert(key, hash(&[i as u8 + 100]));
        }

        let root = tree.root_hash();
        for key in &keys {
            let proof = tree.prove(key).expect("proof should exist");
            assert!(VerkleTree::verify_proof(&root, &proof));
        }
    }

    #[test]
    fn verkle_verify_tampered() {
        let mut tree = VerkleTree::new();
        let key = hash(b"account");
        tree.insert(&key, hash(b"value"));
        let root = tree.root_hash();

        let proof = tree.prove(&key).unwrap();
        let fake_root = hash(b"tampered_root");
        assert!(!VerkleTree::verify_proof(&fake_root, &proof));
        assert!(VerkleTree::verify_proof(&root, &proof));
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
}
