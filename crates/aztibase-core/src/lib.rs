pub mod bls;
pub mod commitment;
pub mod crypto;
pub mod error;
pub mod types;

pub use bls::*;
pub use commitment::*;
pub use crypto::*;
pub use error::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_deterministic() {
        let data = b"aztibase genesis block";
        let h1 = hash(data);
        let h2 = hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let h1 = hash(b"block_a");
        let h2 = hash(b"block_b");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_keypair_sign_verify() {
        let kp = Keypair::generate();
        let msg = b"test transaction";
        let sig = kp.sign(msg);
        assert!(kp.public_key().verify(msg, &sig));
    }

    #[test]
    fn test_signature_rejects_wrong_message() {
        let kp = Keypair::generate();
        let sig = kp.sign(b"correct message");
        assert!(!kp.public_key().verify(b"wrong message", &sig));
    }

    #[test]
    fn test_signature_rejects_wrong_key() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        let sig = kp1.sign(b"message");
        assert!(!kp2.public_key().verify(b"message", &sig));
    }

    #[test]
    fn test_crypto_provider_hash() {
        let provider = DefaultCryptoProvider;
        let h1 = provider.hash(b"test");
        let h2 = hash(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_crypto_provider_sign_verify() {
        let provider = DefaultCryptoProvider;
        let kp = provider.generate_keypair();
        let msg = b"provider test";
        let sig = provider.sign(&kp, msg);
        assert!(provider.verify(&kp.public_key(), msg, &sig));
        assert!(!provider.verify(&kp.public_key(), b"wrong", &sig));
    }

    #[test]
    fn test_crypto_provider_name() {
        let provider = DefaultCryptoProvider;
        assert_eq!(provider.algorithm_name(), "Ed25519+BLAKE3");
    }

    // ── Crypto test vectors (known-answer pinning tests) ──────────

    fn hex_to_32(s: &str) -> [u8; 32] {
        let bytes = hex::decode(s).unwrap();
        bytes.try_into().unwrap()
    }

    #[test]
    fn blake3_known_answer_empty() {
        let expected =
            hex_to_32("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");
        assert_eq!(hash(b""), expected);
    }

    #[test]
    fn blake3_known_answer_aztibase() {
        let expected =
            hex_to_32("e1c9d1b1e6030436cf4e7d4ac84597ffbcc195cecdee40de45977c2f25bb9e03");
        assert_eq!(hash(b"aztibase"), expected);
    }

    #[test]
    fn blake3_tx_domain_prefix() {
        let expected =
            hex_to_32("63b195e0c809f44eef84705e6051f53c561bf2ed8f21d98eae58e51aed102be6");
        assert_eq!(hash(b"AZTB_TX_V1"), expected);
    }

    #[test]
    fn ed25519_known_answer_from_fixed_secret() {
        let secret = [0x01u8; 32];
        let kp = Keypair::from_secret_bytes(&secret);
        let expected_pk =
            hex_to_32("8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c");
        assert_eq!(*kp.public_key().as_bytes(), expected_pk);

        let sig = kp.sign(b"test");
        assert_eq!(sig.len(), 64);
        assert!(kp.public_key().verify(b"test", &sig));

        let expected_sig = hex::decode(
            "ef12237050764bc02516598f45bc4f8839bd0e85d23f36a202867bba58cc68d8\
             f3786a1fbc290c5247ed725923713dcb7210acf3b4959a098e18c22206512a07",
        )
        .unwrap();
        assert_eq!(sig, expected_sig);
    }

    #[test]
    fn ed25519_signature_stability_across_reconstructions() {
        let secret = [0x42u8; 32];
        let kp1 = Keypair::from_secret_bytes(&secret);
        let kp2 = Keypair::from_secret_bytes(&secret);
        let sig1 = kp1.sign(b"deterministic");
        let sig2 = kp2.sign(b"deterministic");
        assert_eq!(sig1, sig2, "same secret must produce identical sigs");
    }

    #[test]
    fn bls_known_answer_from_fixed_secret() {
        let secret = [0x01u8; 32];
        let kp = BlsKeypair::from_secret_bytes(&secret).unwrap();

        let expected_pk = hex::decode(
            "aa1a1c26055a329817a5759d877a2795f9499b97d6056edde0eea39512f24e8b\
             c874b4471f0501127abb1ea0d9f68ac1",
        )
        .unwrap();
        assert_eq!(kp.public_key().as_bytes().as_slice(), &expected_pk);

        let sig = kp.sign(b"test");
        let expected_sig = hex::decode(
            "8eff502864b76aef982c8c21f73df927fc01091bf215265f3b5108ebaf5126b6\
             684f0ffee3371946ff7d04bc81f4bf6c19018e2b13b2d7275c650403e0bb1458\
             62aa41490e62a1aaaa6161c2951a1f0f1aed1c0a3f5097f73cbd59f91cafe01d",
        )
        .unwrap();
        assert_eq!(sig.as_bytes().as_slice(), &expected_sig);

        assert!(kp.public_key().verify(b"test", &sig));
    }

    #[test]
    fn bls_pop_known_answer_from_fixed_secret() {
        let secret = [0x01u8; 32];
        let kp = BlsKeypair::from_secret_bytes(&secret).unwrap();

        let expected_pop = hex::decode(
            "82ddc4aa85aa01a3db921c397443f205cb087584dbcd07c4c4f559082c00947f\
             8dffb89e7cba4a1ccd02168c76087aad0d5a96075c454770f429218cbe65e418\
             5c606b74f435538d127c33317c2a1b6241011bcb27512b193f34dff9353ee801",
        )
        .unwrap();
        assert_eq!(
            kp.proof_of_possession().as_bytes().as_slice(),
            &expected_pop
        );

        assert!(verify_proof_of_possession(
            kp.public_key(),
            kp.proof_of_possession()
        ));
    }

    #[test]
    fn bls_aggregate_known_answer() {
        let sk1 = [0x01u8; 32];
        let sk2 = [0x02u8; 32];
        let kp1 = BlsKeypair::from_secret_bytes(&sk1).unwrap();
        let kp2 = BlsKeypair::from_secret_bytes(&sk2).unwrap();

        let msg = b"aggregate_test";
        let sig1 = kp1.sign(msg);
        let sig2 = kp2.sign(msg);
        let agg = aggregate_signatures(&[sig1.clone(), sig2.clone()]).unwrap();

        let pks = vec![kp1.public_key().clone(), kp2.public_key().clone()];
        assert!(verify_aggregate(&pks, msg, &agg));
        assert!(!verify_aggregate(&pks, b"wrong", &agg));

        // Stability: re-aggregate produces same bytes
        let agg2 = aggregate_signatures(&[sig1, sig2]).unwrap();
        assert_eq!(agg.as_bytes(), agg2.as_bytes());
    }

    #[test]
    fn test_block_header_serialization() {
        let header = BlockHeader {
            version: 1,
            slot: 0,
            round: 0,
            author: [0u8; 32],
            parents: vec![],
            state_root: VerkleRoot::default(),
            transactions_root: [0u8; 32],
            receipts_root: [0u8; 32],
            ai_commitment_root: [0u8; 32],
            timestamp: 1709654400000,
            signature: vec![],
        };
        let serialized = postcard::to_allocvec(&header).expect("serialize");
        let deserialized: BlockHeader = postcard::from_bytes(&serialized).expect("deserialize");
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.slot, 0);
        assert_eq!(deserialized.timestamp, 1709654400000);
    }
}
