use aztibase_core::Hash;
use aztibase_core::commitment::{LightClientProof, StateCommitment, StateProof};

use crate::state::MerkleCommitment;
use crate::verkle::VerkleCommitment;

/// Build a light client proof wrapping an inner state proof with
/// the committed height and serialized finality certificate.
pub fn build_light_client_proof(
    state_root: Hash,
    committed_height: u64,
    finality_certificate: Vec<u8>,
    inner_proof: StateProof,
) -> StateProof {
    StateProof::LightClient(LightClientProof {
        state_root,
        committed_height,
        finality_certificate,
        inner_proof: Box::new(inner_proof),
    })
}

/// Verify a light client proof:
/// 1. Check the inner proof (Merkle or Verkle) against the claimed state root.
/// 2. The caller must separately verify the finality certificate against
///    the validator set (this function validates structure only).
///
/// Returns `true` if the inner state proof is valid for the given leaf
/// against the proof's state root.
pub fn verify_light_client_proof(proof: &LightClientProof, leaf: &[u8; 32]) -> bool {
    if proof.finality_certificate.is_empty() {
        return false;
    }

    match proof.inner_proof.as_ref() {
        StateProof::Merkle(_) => {
            let commitment = MerkleCommitment;
            commitment.verify(&proof.state_root, leaf, &proof.inner_proof)
        }
        StateProof::Verkle(_) => {
            let commitment = VerkleCommitment;
            commitment.verify(&proof.state_root, leaf, &proof.inner_proof)
        }
        StateProof::LightClient(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::commitment::StateCommitment;

    #[test]
    fn light_client_proof_roundtrip() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| [i; 32]).collect();
        let commitment = MerkleCommitment;
        let root = commitment.commit(&leaves);
        let inner = commitment.prove(&leaves, 1).unwrap();

        let lc_proof = build_light_client_proof(root, 42, vec![0xDE, 0xAD], inner);

        match &lc_proof {
            StateProof::LightClient(lcp) => {
                assert_eq!(lcp.state_root, root);
                assert_eq!(lcp.committed_height, 42);
                assert!(verify_light_client_proof(lcp, &leaves[1]));
            }
            _ => panic!("expected LightClient proof"),
        }
    }

    #[test]
    fn light_client_proof_rejects_empty_cert() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| [i; 32]).collect();
        let commitment = MerkleCommitment;
        let root = commitment.commit(&leaves);
        let inner = commitment.prove(&leaves, 1).unwrap();

        let lcp = LightClientProof {
            state_root: root,
            committed_height: 1,
            finality_certificate: vec![],
            inner_proof: Box::new(inner),
        };
        assert!(!verify_light_client_proof(&lcp, &leaves[1]));
    }

    #[test]
    fn light_client_proof_rejects_wrong_leaf() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| [i; 32]).collect();
        let commitment = MerkleCommitment;
        let root = commitment.commit(&leaves);
        let inner = commitment.prove(&leaves, 1).unwrap();

        let lcp = LightClientProof {
            state_root: root,
            committed_height: 1,
            finality_certificate: vec![0xFF],
            inner_proof: Box::new(inner),
        };
        let wrong_leaf = [99u8; 32];
        assert!(!verify_light_client_proof(&lcp, &wrong_leaf));
    }

    #[test]
    fn light_client_proof_rejects_nested() {
        let inner_lcp = StateProof::LightClient(LightClientProof {
            state_root: [0u8; 32],
            committed_height: 0,
            finality_certificate: vec![1],
            inner_proof: Box::new(StateProof::Merkle(aztibase_core::commitment::MerkleProof {
                leaf_hash: [0u8; 32],
                siblings: vec![],
            })),
        });

        let lcp = LightClientProof {
            state_root: [0u8; 32],
            committed_height: 1,
            finality_certificate: vec![1],
            inner_proof: Box::new(inner_lcp),
        };
        assert!(!verify_light_client_proof(&lcp, &[0u8; 32]));
    }
}
