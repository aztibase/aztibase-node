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

    let max_future = 10;
    if block.round > local_round + max_future {
        return Err(WireError::FutureRound {
            vertex: block.round,
            local: local_round,
        });
    }

    Ok(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validators() -> ValidatorSet {
        let mut vs = ValidatorSet::new();
        vs.add([1u8; 32], 100);
        vs.add([2u8; 32], 100);
        vs.add([3u8; 32], 100);
        vs
    }

    #[test]
    fn roundtrip_encode_decode() {
        let vs = test_validators();
        let block = DagBlock::genesis([1u8; 32], 1000);
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
        let vs = test_validators();
        let parent = DagBlock::genesis([1u8; 32], 1000);
        let block = DagBlock::new(1, [2u8; 32], vec![parent.hash], vec![1, 2, 3, 4], 2000).unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let decoded = decode_vertex(&encoded, &vs, 1).unwrap();
        assert_eq!(decoded.hash, block.hash);
        assert_eq!(decoded.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn rejects_too_short() {
        let vs = test_validators();
        let result = decode_vertex(&[1], &vs, 0);
        assert!(matches!(result, Err(WireError::TooShort(1))));
    }

    #[test]
    fn rejects_wrong_version() {
        let vs = test_validators();
        let block = DagBlock::genesis([1u8; 32], 1000);
        let mut encoded = encode_vertex(&block).unwrap();
        encoded[0] = 255;
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(result, Err(WireError::UnsupportedVersion(255))));
    }

    #[test]
    fn rejects_tampered_hash() {
        let vs = test_validators();
        let mut block = DagBlock::genesis([1u8; 32], 1000);
        block.hash = [0xFF; 32];
        let mut buf = vec![WIRE_VERSION];
        buf.extend_from_slice(&postcard::to_allocvec(&block).unwrap());
        let result = decode_vertex(&buf, &vs, 0);
        assert!(matches!(result, Err(WireError::HashMismatch)));
    }

    #[test]
    fn rejects_unknown_validator() {
        let vs = test_validators();
        let block = DagBlock::genesis([99u8; 32], 1000);
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(result, Err(WireError::UnknownValidator(99, ..))));
    }

    #[test]
    fn rejects_future_round() {
        let vs = test_validators();
        let parent_hash = aztibase_core::hash(b"fake_parent");
        let block = DagBlock::new(50, [1u8; 32], vec![parent_hash], vec![], 3000).unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(matches!(
            result,
            Err(WireError::FutureRound {
                vertex: 50,
                local: 0
            })
        ));
    }

    #[test]
    fn accepts_near_future_round() {
        let vs = test_validators();
        let parent_hash = aztibase_core::hash(b"fake_parent");
        let block = DagBlock::new(8, [1u8; 32], vec![parent_hash], vec![], 3000).unwrap();
        let encoded = encode_vertex(&block).unwrap();
        let result = decode_vertex(&encoded, &vs, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_oversized_message() {
        let vs = test_validators();
        let mut buf = vec![WIRE_VERSION];
        buf.extend_from_slice(&vec![0u8; MAX_VERTEX_SIZE + 1]);
        let result = decode_vertex(&buf, &vs, 0);
        assert!(matches!(result, Err(WireError::TooLarge { .. })));
    }
}
