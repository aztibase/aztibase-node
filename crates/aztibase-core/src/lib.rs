pub mod bls;
pub mod crypto;
pub mod error;
pub mod types;

pub use bls::*;
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
        let serialized = bincode::serialize(&header).expect("serialize");
        let deserialized: BlockHeader = bincode::deserialize(&serialized).expect("deserialize");
        assert_eq!(deserialized.version, 1);
        assert_eq!(deserialized.slot, 0);
        assert_eq!(deserialized.timestamp, 1709654400000);
    }
}
