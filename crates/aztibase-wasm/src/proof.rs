use serde::{Deserialize, Serialize};

use crate::hash;

#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub leaf_hash: [u8; 32],
    pub siblings: Vec<([u8; 32], Side)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug)]
pub struct VerkleProof {
    pub leaf_hash: [u8; 32],
    pub stem: Vec<u8>,
    pub value_hash: [u8; 32],
    pub levels: Vec<VerkleProofLevel>,
}

#[derive(Clone, Debug)]
pub struct VerkleProofLevel {
    pub child_index: u8,
    pub child_commitments: Vec<[u8; 32]>,
}

pub fn verify_merkle_proof(root: &[u8; 32], leaf: &[u8; 32], proof: &MerkleProof) -> bool {
    if proof.leaf_hash != *leaf {
        return false;
    }

    let mut current = *leaf;
    for (sibling, side) in &proof.siblings {
        let mut combined = [0u8; 64];
        match side {
            Side::Right => {
                combined[..32].copy_from_slice(&current);
                combined[32..].copy_from_slice(sibling);
            }
            Side::Left => {
                combined[..32].copy_from_slice(sibling);
                combined[32..].copy_from_slice(&current);
            }
        }
        current = hash(&combined);
    }
    current == *root
}

const VERKLE_INNER_DOMAIN: &[u8] = b"AZTB_VERKLE_INNER\0";

pub fn verify_verkle_proof(root: &[u8; 32], proof: &VerkleProof) -> bool {
    let leaf_commit = {
        let mut buf = Vec::with_capacity(18 + proof.stem.len() + 32);
        buf.extend_from_slice(b"AZTB_VERKLE_LEAF\0");
        buf.extend_from_slice(&proof.stem);
        buf.extend_from_slice(&proof.value_hash);
        hash(&buf)
    };

    if proof.levels.is_empty() {
        return leaf_commit == *root;
    }

    let mut expected_child = leaf_commit;
    for level in proof.levels.iter().rev() {
        if level.child_commitments.len() != 256 {
            return false;
        }
        if level.child_commitments[level.child_index as usize] != expected_child {
            return false;
        }
        let mut buf = Vec::with_capacity(VERKLE_INNER_DOMAIN.len() + 256 * 32);
        buf.extend_from_slice(VERKLE_INNER_DOMAIN);
        for c in &level.child_commitments {
            buf.extend_from_slice(c);
        }
        expected_child = hash(&buf);
    }
    expected_child == *root
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsMerkleProof {
    pub leaf_hash: String,
    pub siblings: Vec<JsSibling>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsSibling {
    pub hash: String,
    pub side: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsVerkleProof {
    pub leaf_hash: String,
    pub stem: String,
    pub value_hash: String,
    pub levels: Vec<JsVerkleLevel>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsVerkleLevel {
    pub child_index: u8,
    pub child_commitments: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsLightClientProof {
    pub state_root: String,
    pub committed_height: u64,
    pub finality_certificate: Vec<u8>,
    pub inner_proof_type: String,
    pub inner_proof: serde_json::Value,
}

impl JsMerkleProof {
    pub fn into_merkle_proof(self) -> MerkleProof {
        let leaf_hash = crate::parse_hash(&self.leaf_hash).unwrap_or([0u8; 32]);
        let siblings = self
            .siblings
            .iter()
            .map(|s| {
                let h = crate::parse_hash(&s.hash).unwrap_or([0u8; 32]);
                let side = if s.side == "left" {
                    Side::Left
                } else {
                    Side::Right
                };
                (h, side)
            })
            .collect();
        MerkleProof {
            leaf_hash,
            siblings,
        }
    }
}

impl JsVerkleProof {
    pub fn into_verkle_proof(self) -> VerkleProof {
        let leaf_hash = crate::parse_hash(&self.leaf_hash).unwrap_or([0u8; 32]);
        let stem = crate::parse_hash(&self.stem)
            .map(|h| h.to_vec())
            .unwrap_or_else(|| vec![0u8; 32]);
        let value_hash = crate::parse_hash(&self.value_hash).unwrap_or([0u8; 32]);
        let levels = self
            .levels
            .into_iter()
            .map(|l| VerkleProofLevel {
                child_index: l.child_index,
                child_commitments: l
                    .child_commitments
                    .iter()
                    .map(|s| crate::parse_hash(s).unwrap_or([0u8; 32]))
                    .collect(),
            })
            .collect();
        VerkleProof {
            leaf_hash,
            stem,
            value_hash,
            levels,
        }
    }
}

pub fn verify_light_client_proof(lcp: &JsLightClientProof, leaf: &[u8; 32]) -> bool {
    if lcp.finality_certificate.is_empty() {
        return false;
    }

    let state_root = match crate::parse_hash(&lcp.state_root) {
        Some(r) => r,
        None => return false,
    };

    match lcp.inner_proof_type.as_str() {
        "merkle" => {
            let inner: JsMerkleProof = match serde_json::from_value(lcp.inner_proof.clone()) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let mp = inner.into_merkle_proof();
            verify_merkle_proof(&state_root, leaf, &mp)
        }
        "verkle" => {
            let inner: JsVerkleProof = match serde_json::from_value(lcp.inner_proof.clone()) {
                Ok(p) => p,
                Err(_) => return false,
            };
            let vp = inner.into_verkle_proof();
            verify_verkle_proof(&state_root, &vp)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_merkle_tree(leaves: &[[u8; 32]]) -> [u8; 32] {
        if leaves.is_empty() {
            return [0u8; 32];
        }
        if leaves.len() == 1 {
            return leaves[0];
        }
        let mut layer: Vec<[u8; 32]> = leaves.to_vec();
        while layer.len() > 1 {
            let mut next = Vec::with_capacity(layer.len().div_ceil(2));
            for pair in layer.chunks(2) {
                if pair.len() == 2 {
                    let mut combined = [0u8; 64];
                    combined[..32].copy_from_slice(&pair[0]);
                    combined[32..].copy_from_slice(&pair[1]);
                    next.push(hash(&combined));
                } else {
                    next.push(pair[0]);
                }
            }
            layer = next;
        }
        layer[0]
    }

    fn build_merkle_proof(leaves: &[[u8; 32]], index: usize) -> MerkleProof {
        let mut siblings = Vec::new();
        let mut layer: Vec<[u8; 32]> = leaves.to_vec();
        let mut pos = index;

        while layer.len() > 1 {
            let sibling_pos = if pos % 2 == 0 { pos + 1 } else { pos - 1 };
            if sibling_pos < layer.len() {
                let side = if pos % 2 == 0 {
                    Side::Right
                } else {
                    Side::Left
                };
                siblings.push((layer[sibling_pos], side));
            }

            let mut next = Vec::with_capacity(layer.len().div_ceil(2));
            for pair in layer.chunks(2) {
                if pair.len() == 2 {
                    let mut combined = [0u8; 64];
                    combined[..32].copy_from_slice(&pair[0]);
                    combined[32..].copy_from_slice(&pair[1]);
                    next.push(hash(&combined));
                } else {
                    next.push(pair[0]);
                }
            }
            layer = next;
            pos /= 2;
        }

        MerkleProof {
            leaf_hash: leaves[index],
            siblings,
        }
    }

    #[test]
    fn merkle_proof_valid() {
        let leaves: Vec<[u8; 32]> = (0..8u8).map(|i| hash(&[i])).collect();
        let root = build_merkle_tree(&leaves);

        for i in 0..leaves.len() {
            let proof = build_merkle_proof(&leaves, i);
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof),
                "failed for leaf {i}"
            );
        }
    }

    #[test]
    fn merkle_proof_rejects_wrong_leaf() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let root = build_merkle_tree(&leaves);
        let proof = build_merkle_proof(&leaves, 0);
        let fake = hash(b"fake");
        assert!(!verify_merkle_proof(&root, &fake, &proof));
    }

    #[test]
    fn merkle_proof_rejects_wrong_root() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let proof = build_merkle_proof(&leaves, 0);
        let fake_root = hash(b"fake_root");
        assert!(!verify_merkle_proof(&fake_root, &leaves[0], &proof));
    }

    #[test]
    fn verkle_proof_rejects_wrong_root() {
        let root = hash(b"root");
        let vp = VerkleProof {
            leaf_hash: hash(b"leaf"),
            stem: vec![0u8; 32],
            value_hash: [0u8; 32],
            levels: vec![],
        };
        assert!(!verify_verkle_proof(&root, &vp));
    }

    #[test]
    fn verkle_proof_rejects_wrong_width() {
        let root = hash(b"root");
        let vp = VerkleProof {
            leaf_hash: hash(b"leaf"),
            stem: vec![0u8; 32],
            value_hash: [0u8; 32],
            levels: vec![VerkleProofLevel {
                child_index: 0,
                child_commitments: vec![[0u8; 32]; 10],
            }],
        };
        assert!(!verify_verkle_proof(&root, &vp));
    }

    #[test]
    fn light_client_proof_merkle() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let root = build_merkle_tree(&leaves);
        let proof = build_merkle_proof(&leaves, 1);

        let js_proof = JsLightClientProof {
            state_root: crate::hex_encode(&root),
            committed_height: 42,
            finality_certificate: vec![0xDE, 0xAD],
            inner_proof_type: "merkle".into(),
            inner_proof: serde_json::to_value(JsMerkleProof {
                leaf_hash: crate::hex_encode(&proof.leaf_hash),
                siblings: proof
                    .siblings
                    .iter()
                    .map(|(h, s)| JsSibling {
                        hash: crate::hex_encode(h),
                        side: match s {
                            Side::Left => "left".into(),
                            Side::Right => "right".into(),
                        },
                    })
                    .collect(),
            })
            .unwrap(),
        };

        assert!(verify_light_client_proof(&js_proof, &leaves[1]));
    }

    #[test]
    fn light_client_proof_rejects_empty_cert() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let root = build_merkle_tree(&leaves);
        let proof = build_merkle_proof(&leaves, 0);

        let js_proof = JsLightClientProof {
            state_root: crate::hex_encode(&root),
            committed_height: 1,
            finality_certificate: vec![],
            inner_proof_type: "merkle".into(),
            inner_proof: serde_json::to_value(JsMerkleProof {
                leaf_hash: crate::hex_encode(&proof.leaf_hash),
                siblings: proof
                    .siblings
                    .iter()
                    .map(|(h, s)| JsSibling {
                        hash: crate::hex_encode(h),
                        side: match s {
                            Side::Left => "left".into(),
                            Side::Right => "right".into(),
                        },
                    })
                    .collect(),
            })
            .unwrap(),
        };

        assert!(!verify_light_client_proof(&js_proof, &leaves[0]));
    }

    #[test]
    fn light_client_proof_rejects_nested() {
        let js_proof = JsLightClientProof {
            state_root: crate::hex_encode(&[0u8; 32]),
            committed_height: 1,
            finality_certificate: vec![1],
            inner_proof_type: "lightclient".into(),
            inner_proof: serde_json::Value::Null,
        };
        assert!(!verify_light_client_proof(&js_proof, &[0u8; 32]));
    }

    #[test]
    fn js_merkle_proof_json_roundtrip() {
        let js = JsMerkleProof {
            leaf_hash: "aa".repeat(32),
            siblings: vec![JsSibling {
                hash: "bb".repeat(32),
                side: "left".into(),
            }],
        };
        let json = serde_json::to_string(&js).unwrap();
        let parsed: JsMerkleProof = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.leaf_hash, js.leaf_hash);
        assert_eq!(parsed.siblings.len(), 1);
    }
}
