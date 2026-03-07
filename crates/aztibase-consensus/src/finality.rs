use std::collections::HashSet;

use aztibase_core::{
    BlockHash, BlsKeypair, BlsPublicKey, BlsSignature, aggregate_signatures, hash, verify_aggregate,
};
use serde::{Deserialize, Serialize};

use crate::ValidatorSet;

/// Cryptographic proof that a supermajority of validators agreed on a
/// committed batch and its resulting state root. Light clients can verify
/// finality without replaying the DAG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FinalityCertificate {
    pub batch_hash: BlockHash,
    pub state_root: [u8; 32],
    pub aggregate_signature: BlsSignature,
    /// Bitmap indicating which validators (by sorted index) signed.
    pub signer_bitmap: Vec<bool>,
}

/// Message that validators sign: BLAKE3(domain || batch_hash || state_root).
/// Domain separator prevents cross-protocol signature replay.
fn finality_message(batch_hash: &BlockHash, state_root: &[u8; 32]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(84);
    buf.extend_from_slice(b"AZTIBASE_FINALITY_V1");
    buf.extend_from_slice(batch_hash);
    buf.extend_from_slice(state_root);
    hash(&buf)
}

/// Collect a validator's signature for a committed batch.
pub fn sign_finality(
    keypair: &BlsKeypair,
    batch_hash: &BlockHash,
    state_root: &[u8; 32],
) -> BlsSignature {
    let msg = finality_message(batch_hash, state_root);
    keypair.sign(&msg)
}

fn has_unique_bls_keys(keys: &[BlsPublicKey]) -> bool {
    let set: HashSet<&[u8; 48]> = keys.iter().map(|k| k.as_bytes()).collect();
    set.len() == keys.len()
}

/// Build a finality certificate from individual validator signatures.
///
/// `signers` maps validator BLS public keys to their signatures.
/// `validator_bls_keys` is the full ordered list of validator BLS public keys
/// (same order used for the signer bitmap).
///
/// Returns `None` if fewer than `quorum` signatures are provided or
/// aggregation fails.
pub fn build_certificate(
    batch_hash: BlockHash,
    state_root: [u8; 32],
    validator_bls_keys: &[BlsPublicKey],
    signers: &[(BlsPublicKey, BlsSignature)],
    validator_set: &ValidatorSet,
) -> Option<FinalityCertificate> {
    let quorum = validator_set.quorum_count();

    if signers.len() < quorum {
        return None;
    }

    if !has_unique_bls_keys(validator_bls_keys) {
        return None;
    }

    let mut bitmap = vec![false; validator_bls_keys.len()];
    let mut sigs = Vec::with_capacity(signers.len());

    for (pk, sig) in signers {
        if let Some(idx) = validator_bls_keys.iter().position(|k| k == pk) {
            if bitmap[idx] {
                continue;
            }
            bitmap[idx] = true;
            sigs.push(sig.clone());
        }
    }

    if sigs.len() < quorum {
        return None;
    }

    let agg = aggregate_signatures(&sigs)?;

    Some(FinalityCertificate {
        batch_hash,
        state_root,
        aggregate_signature: agg,
        signer_bitmap: bitmap,
    })
}

/// Verify a finality certificate against a known validator set.
///
/// Checks:
/// 1. Signer bitmap has enough signers (>= quorum)
/// 2. Aggregated BLS signature is valid for the signing validators
pub fn verify_certificate(
    cert: &FinalityCertificate,
    validator_bls_keys: &[BlsPublicKey],
    validator_set: &ValidatorSet,
) -> bool {
    if cert.signer_bitmap.len() != validator_bls_keys.len() {
        return false;
    }

    if !has_unique_bls_keys(validator_bls_keys) {
        return false;
    }

    let signer_keys: Vec<BlsPublicKey> = cert
        .signer_bitmap
        .iter()
        .zip(validator_bls_keys.iter())
        .filter(|(signed, _)| **signed)
        .map(|(_, pk)| pk.clone())
        .collect();

    let quorum = validator_set.quorum_count();
    if signer_keys.len() < quorum {
        return false;
    }

    let msg = finality_message(&cert.batch_hash, &cert.state_root);
    verify_aggregate(&signer_keys, &msg, &cert.aggregate_signature)
}

/// Build a finality certificate pulling BLS public keys from the ValidatorSet.
/// This is the preferred API when BLS keys are stored in genesis.
pub fn build_certificate_from_set(
    batch_hash: BlockHash,
    state_root: [u8; 32],
    signers: &[(BlsPublicKey, BlsSignature)],
    validator_set: &ValidatorSet,
) -> Option<FinalityCertificate> {
    let ordered = validator_set.bls_keys_ordered();
    let bls_keys: Vec<BlsPublicKey> = ordered.into_iter().map(|(_, k)| k).collect();
    build_certificate(batch_hash, state_root, &bls_keys, signers, validator_set)
}

/// Verify a finality certificate pulling BLS public keys from the ValidatorSet.
pub fn verify_certificate_from_set(
    cert: &FinalityCertificate,
    validator_set: &ValidatorSet,
) -> bool {
    let ordered = validator_set.bls_keys_ordered();
    let bls_keys: Vec<BlsPublicKey> = ordered.into_iter().map(|(_, k)| k).collect();
    verify_certificate(cert, &bls_keys, validator_set)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_validators(n: usize) -> (ValidatorSet, Vec<BlsKeypair>, Vec<BlsPublicKey>) {
        let mut vs = ValidatorSet::new();
        let mut keypairs = Vec::new();
        let mut bls_keys = Vec::new();

        for i in 0..n {
            let addr = [i as u8 + 1; 32];
            vs.add(addr, 100);
            let kp = BlsKeypair::generate();
            bls_keys.push(kp.public_key().clone());
            keypairs.push(kp);
        }

        (vs, keypairs, bls_keys)
    }

    #[test]
    fn sign_and_build_certificate() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch1");
        let state_root = hash(b"state1");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        assert_eq!(cert.batch_hash, batch_hash);
        assert_eq!(cert.state_root, state_root);
        assert_eq!(cert.signer_bitmap.iter().filter(|s| **s).count(), 3);
    }

    #[test]
    fn verify_valid_certificate() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch2");
        let state_root = hash(b"state2");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        assert!(verify_certificate(&cert, &bls_keys, &vs));
    }

    #[test]
    fn reject_insufficient_signers() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch3");
        let state_root = hash(b"state3");

        // Only 2 signers — quorum is 3
        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..2]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs);
        assert!(cert.is_none());
    }

    #[test]
    fn reject_wrong_state_root() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch4");
        let state_root = hash(b"state4");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let mut cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        // Tamper with state root
        cert.state_root = hash(b"tampered");
        assert!(!verify_certificate(&cert, &bls_keys, &vs));
    }

    #[test]
    fn reject_wrong_batch_hash() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch5");
        let state_root = hash(b"state5");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let mut cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        cert.batch_hash = hash(b"tampered_batch");
        assert!(!verify_certificate(&cert, &bls_keys, &vs));
    }

    #[test]
    fn reject_manipulated_bitmap() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch6");
        let state_root = hash(b"state6");

        // Only 2 sign
        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..2]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        // Build with a 2-validator set (quorum=2), so cert is valid for it
        let (small_vs, _, _) = setup_validators(2);
        let cert =
            build_certificate(batch_hash, state_root, &bls_keys, &signers, &small_vs).unwrap();

        // But verification uses the real 4-validator set (quorum=3)
        assert!(!verify_certificate(&cert, &bls_keys, &vs));
    }

    #[test]
    fn reject_bitmap_length_mismatch() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch7");
        let state_root = hash(b"state7");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let mut cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        // Tamper: add extra entry to bitmap
        cert.signer_bitmap.push(true);
        assert!(!verify_certificate(&cert, &bls_keys, &vs));
    }

    #[test]
    fn reject_duplicate_bls_keys() {
        let (vs, keypairs, mut bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch_dup");
        let state_root = hash(b"state_dup");

        // Duplicate the first key into the second slot
        bls_keys[1] = bls_keys[0].clone();

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs);
        assert!(cert.is_none());
    }

    #[test]
    fn finality_cert_with_genesis_bls() {
        let mut vs = ValidatorSet::new();
        let mut keypairs = Vec::new();

        for i in 0..4u8 {
            let kp = BlsKeypair::generate();
            vs.add_with_bls([i + 1; 32], 100, Some(kp.public_key().clone()));
            keypairs.push(kp);
        }

        let batch_hash = hash(b"genesis_bls_batch");
        let state_root = hash(b"genesis_bls_state");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate_from_set(batch_hash, state_root, &signers, &vs).unwrap();
        assert!(verify_certificate_from_set(&cert, &vs));
    }

    #[test]
    fn full_validator_set_signs() {
        let (vs, keypairs, bls_keys) = setup_validators(4);
        let batch_hash = hash(b"batch8");
        let state_root = hash(b"state8");

        let signers: Vec<(BlsPublicKey, BlsSignature)> = keypairs
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        assert!(verify_certificate(&cert, &bls_keys, &vs));
        assert!(cert.signer_bitmap.iter().all(|s| *s));
    }
}
