use serde::{Deserialize, Serialize};

use crate::finality::FinalityCertificate;

pub const CHECKPOINT_INTERVAL: u64 = 1000;

/// A weak subjectivity checkpoint: a finalized state at a specific batch.
/// Nodes syncing from scratch or recovering after downtime can verify they
/// are on the correct chain by checking against a trusted checkpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub batch_index: u64,
    pub state_root: [u8; 32],
    pub finality_cert: Option<FinalityCertificate>,
    pub timestamp: u64,
}

impl Checkpoint {
    pub fn new(
        batch_index: u64,
        state_root: [u8; 32],
        finality_cert: Option<FinalityCertificate>,
        timestamp: u64,
    ) -> Self {
        Self {
            batch_index,
            state_root,
            finality_cert,
            timestamp,
        }
    }

    pub fn matches(&self, batch_index: u64, state_root: &[u8; 32]) -> bool {
        self.batch_index == batch_index && self.state_root == *state_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finality::SignerBitmap;
    use aztibase_core::BlsKeypair;

    fn dummy_cert() -> FinalityCertificate {
        let kp = BlsKeypair::from_secret_bytes(&[0x01u8; 32]).unwrap();
        FinalityCertificate {
            batch_hash: [0xAA; 32],
            state_root: [0xBB; 32],
            aggregate_signature: kp.sign(b"test"),
            signer_bitmap: SignerBitmap::new(3),
        }
    }

    #[test]
    fn checkpoint_creation() {
        let cp = Checkpoint::new(1000, [0xBB; 32], Some(dummy_cert()), 1709654400000);
        assert_eq!(cp.batch_index, 1000);
        assert_eq!(cp.state_root, [0xBB; 32]);
        assert_eq!(cp.timestamp, 1709654400000);
        assert!(cp.finality_cert.is_some());
    }

    #[test]
    fn checkpoint_creation_without_cert() {
        let cp = Checkpoint::new(1000, [0xBB; 32], None, 1709654400000);
        assert_eq!(cp.batch_index, 1000);
        assert!(cp.finality_cert.is_none());
    }

    #[test]
    fn checkpoint_matches() {
        let cp = Checkpoint::new(1000, [0xBB; 32], Some(dummy_cert()), 0);
        assert!(cp.matches(1000, &[0xBB; 32]));
        assert!(!cp.matches(999, &[0xBB; 32]));
        assert!(!cp.matches(1000, &[0xCC; 32]));
    }

    #[test]
    fn checkpoint_serialization_roundtrip() {
        let cp = Checkpoint::new(2000, [0xCC; 32], Some(dummy_cert()), 1234567890);
        let bytes = postcard::to_allocvec(&cp).unwrap();
        let cp2: Checkpoint = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(cp2.batch_index, 2000);
        assert_eq!(cp2.state_root, [0xCC; 32]);
        assert_eq!(cp2.timestamp, 1234567890);
        assert!(cp2.finality_cert.is_some());
    }

    #[test]
    fn checkpoint_serialization_without_cert() {
        let cp = Checkpoint::new(3000, [0xDD; 32], None, 9999);
        let bytes = postcard::to_allocvec(&cp).unwrap();
        let cp2: Checkpoint = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(cp2.batch_index, 3000);
        assert_eq!(cp2.state_root, [0xDD; 32]);
        assert!(cp2.finality_cert.is_none());
    }
}
