use aztibase_core::PublicKey;

use crate::dag::DagBlock;
use crate::validator::ValidatorSet;

const WIRE_VERSION: u8 = 1;
const MAX_VERTEX_SIZE: usize = 512 * 1024; // 512 KiB

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("message too short ({0} bytes)")]
    TooShort(usize),

    #[error("unsupported wire version {0}")]
    UnsupportedVersion(u8),

    #[error("message exceeds size limit ({size} > {limit})")]
    TooLarge { size: usize, limit: usize },

    #[error("postcard decode failed: {0}")]
    Decode(String),

    #[error("hash mismatch")]
    HashMismatch,

    #[error("unknown validator {0:02x}{1:02x}{2:02x}{3:02x}")]
    UnknownValidator(u8, u8, u8, u8),

    #[error("round {vertex} is too far ahead of local round {local}")]
    FutureRound { vertex: u64, local: u64 },

    #[error("round gap detected: vertex at {vertex}, local at {local}")]
    RoundGap { vertex: u64, local: u64 },

    #[error("invalid Ed25519 signature on vertex")]
    InvalidSignature,
}

/// Encode a DAG vertex for gossip transmission.
/// Wire format: [version: u8][postcard payload]
pub fn encode_vertex(block: &DagBlock) -> Result<Vec<u8>, WireError> {
    let body = postcard::to_allocvec(block).map_err(|e| WireError::Decode(e.to_string()))?;
    let mut buf = Vec::with_capacity(1 + body.len());
    buf.push(WIRE_VERSION);
    buf.extend_from_slice(&body);
    Ok(buf)
}

/// Decode and validate a gossip vertex.
///
/// Checks: size limits, version byte, postcard decode, hash integrity,
/// known validator, and round proximity.
pub fn decode_vertex(
    data: &[u8],
    validators: &ValidatorSet,
    local_round: u64,
) -> Result<DagBlock, WireError> {
    if data.len() < 2 {
        return Err(WireError::TooShort(data.len()));
    }

    if data.len() > MAX_VERTEX_SIZE {
        return Err(WireError::TooLarge {
            size: data.len(),
            limit: MAX_VERTEX_SIZE,
        });
    }

    let version = data[0];
    if version != WIRE_VERSION {
        return Err(WireError::UnsupportedVersion(version));
    }

    let block: DagBlock =
        postcard::from_bytes(&data[1..]).map_err(|e| WireError::Decode(e.to_string()))?;

    let expected = block.compute_hash();
    if expected != block.hash {
        return Err(WireError::HashMismatch);
    }

    if !validators.contains(&block.author) {
        let a = block.author;
        return Err(WireError::UnknownValidator(a[0], a[1], a[2], a[3]));
    }

    if !block.is_genesis() {
        let pubkey_bytes = validators
            .ed25519_key(&block.author)
            .or_else(|| {
                // Fallback: if no stored Ed25519 key, try interpreting author as raw pubkey
                PublicKey::from_bytes(&block.author).map(|_| block.author)
            })
            .ok_or(WireError::InvalidSignature)?;
        let pubkey = PublicKey::from_bytes(&pubkey_bytes).ok_or(WireError::InvalidSignature)?;
        if !block.verify_signature(&pubkey) {
            return Err(WireError::InvalidSignature);
        }
    }

    let catchup_threshold = 100;
    if block.round > local_round + catchup_threshold {
        return Err(WireError::RoundGap {
            vertex: block.round,
            local: local_round,
        });
    }

    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::Keypair;

    fn test_keypairs() -> (Keypair, Keypair, Keypair) {
        let kp1 = Keypair::from_secret_bytes(&[1u8; 32]);
        let kp2 = Keypair::from_secret_bytes(&[2u8; 32]);
        let kp3 = Keypair::from_secret_bytes(&[3u8; 32]);
        (kp1, kp2, kp3)
    }

    fn test_validators_from_keypairs(kps: &(Keypair, Keypair, Keypair)) -> ValidatorSet {
        let mut vs = ValidatorSet::new();
        let pk0 = *kps.0.public_key().as_bytes();
        let pk1 = *kps.1.public_key().as_bytes();
        let pk2 = *kps.2.public_key().as_bytes();
        vs.add(pk0, 100);
        vs.add(pk1, 100);
        vs.add(pk2, 100);
        vs.set_ed25519_key(&pk0, pk0);
        vs.set_ed25519_key(&pk1, pk1);
        vs.set_ed25519_key(&pk2, pk2);
        vs
    }

    #[test]
    fn roundtrip_encode_decode() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author = *kps.0.public_key().as_bytes();
        let block = DagBlock::genesis(author, 1000);
        let encoded = encode_vertex(&block).unwrap();

        assert_eq!(encoded[0], WIRE_VERSION);

        let decoded = decode_vertex(&encoded, &vs, 0).unwrap();
        assert_eq!(decoded.hash, block.hash);
        assert_eq!(decoded.round, block.round);
        assert_eq!(decoded.author, block.author);
        assert_eq!(decoded.payload, block.payload);
    }

    #[test]
    fn roundtrip_with_payload() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author1 = *kps.0.public_key().as_bytes();
        let author2 = *kps.1.public_key().as_bytes();
        let parent = DagBlock::genesis(author1, 1000);
        let block = DagBlock::new(
            1,
            author2,
            vec![parent.hash],
            vec![1, 2, 3, 4],
            2000,
            Some(&kps.1),
        )
        .unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let decoded = decode_vertex(&encoded, &vs, 1).unwrap();
        assert_eq!(decoded.hash, block.hash);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn rejects_too_short() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let result = decode_vertex(&[1], &vs, 0);
        assert!(matches!(result, Err(WireError::TooShort(1))));
    }

    #[test]
    fn rejects_wrong_version() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author = *kps.0.public_key().as_bytes();
        let block = DagBlock::genesis(author, 1000);
        let mut encoded = encode_vertex(&block).unwrap();
        encoded[0] = 255;
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(result, Err(WireError::UnsupportedVersion(255))));
    }

    #[test]
    fn rejects_tampered_hash() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author = *kps.0.public_key().as_bytes();
        let mut block = DagBlock::genesis(author, 1000);
        block.hash = [0xFF; 32];
        let mut buf = vec![WIRE_VERSION];
        buf.extend_from_slice(&postcard::to_allocvec(&block).unwrap());
        let result = decode_vertex(&buf, &vs, 0);
        assert!(matches!(result, Err(WireError::HashMismatch)));
    }

    #[test]
    fn rejects_unknown_validator() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let block = DagBlock::genesis([99u8; 32], 1000);
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(result, Err(WireError::UnknownValidator(..))));
    }

    #[test]
    fn detects_round_gap() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author = *kps.0.public_key().as_bytes();
        let parent_hash = aztibase_core::hash(b"fake_parent");
        let block =
            DagBlock::new(150, author, vec![parent_hash], vec![], 3000, Some(&kps.0)).unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(
            result,
            Err(WireError::RoundGap {
                vertex: 150,
                local: 0
            })
        ));
    }

    #[test]
    fn accepts_near_future_round() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let author = *kps.0.public_key().as_bytes();
        let parent_hash = aztibase_core::hash(b"fake_parent");
        let block =
            DagBlock::new(90, author, vec![parent_hash], vec![], 3000, Some(&kps.0)).unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_oversized_message() {
        let kps = test_keypairs();
        let vs = test_validators_from_keypairs(&kps);
        let _ = vs;
        let mut vs2 = ValidatorSet::new();
        vs2.add([0u8; 32], 100);
        let mut buf = vec![WIRE_VERSION];
        buf.extend_from_slice(&vec![0u8; MAX_VERTEX_SIZE + 1]);
        let result = decode_vertex(&buf, &vs2, 0);
        assert!(matches!(result, Err(WireError::TooLarge { .. })));
    }
}
