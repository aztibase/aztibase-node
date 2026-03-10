use serde::{Deserialize, Serialize};

/// Minimum quorum signatures required for a valid DHT record.
/// For a 3f+1 validator set, this should be 2f+1.
pub const MIN_QUORUM_SIGNATURES: usize = 2;

/// Maximum age (in rounds) before a DHT record is considered stale.
pub const MAX_RECORD_AGE_ROUNDS: u64 = 10_000;

/// Types of records stored in the Aztibase DHT.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DhtRecordKind {
    ValidatorSet,
    RelayProvider,
    ChainTip,
}

/// A single validator's signature over a DHT record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhtRecordSignature {
    pub validator_id: [u8; 32],
    pub signature: Vec<u8>,
}

/// A quorum-signed DHT record with freshness metadata (S8-1).
///
/// All Genesis-specific DHT records (validator sets, relay providers,
/// chain tips) must be signed by a quorum of validators. Single-signer
/// records are rejected at validation time, preventing DHT poisoning.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedDhtRecord {
    pub kind: DhtRecordKind,
    pub round: u64,
    pub data: Vec<u8>,
    pub signatures: Vec<DhtRecordSignature>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DhtValidationError {
    InsufficientSignatures {
        got: usize,
        need: usize,
    },
    StaleRecord {
        record_round: u64,
        current_round: u64,
    },
    EmptyData,
    InvalidSignature {
        validator: [u8; 32],
    },
}

impl std::fmt::Display for DhtValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsufficientSignatures { got, need } => {
                write!(f, "insufficient DHT record signatures: {got} < {need}")
            }
            Self::StaleRecord {
                record_round,
                current_round,
            } => write!(
                f,
                "stale DHT record: round {record_round}, current {current_round}"
            ),
            Self::EmptyData => write!(f, "DHT record has empty data"),
            Self::InvalidSignature { validator } => {
                write!(
                    f,
                    "invalid signature from validator {:02x?}",
                    &validator[..4]
                )
            }
        }
    }
}

impl std::error::Error for DhtValidationError {}

/// Validate a signed DHT record for quorum and freshness.
///
/// Checks:
/// 1. Non-empty data
/// 2. At least `min_quorum` unique validator signatures
/// 3. Record is not stale (round >= current_round - MAX_RECORD_AGE_ROUNDS)
/// 4. Signature verification via the provided callback
pub fn validate_dht_record<F>(
    record: &SignedDhtRecord,
    current_round: u64,
    min_quorum: usize,
    verify_sig: F,
) -> Result<(), DhtValidationError>
where
    F: Fn(&[u8; 32], &[u8], &[u8]) -> bool,
{
    if record.data.is_empty() {
        return Err(DhtValidationError::EmptyData);
    }

    let effective_quorum = min_quorum.max(MIN_QUORUM_SIGNATURES);
    if record.signatures.len() < effective_quorum {
        return Err(DhtValidationError::InsufficientSignatures {
            got: record.signatures.len(),
            need: effective_quorum,
        });
    }

    if current_round > MAX_RECORD_AGE_ROUNDS && record.round < current_round - MAX_RECORD_AGE_ROUNDS
    {
        return Err(DhtValidationError::StaleRecord {
            record_round: record.round,
            current_round,
        });
    }

    let signing_payload = dht_signing_payload(record);
    for sig in &record.signatures {
        if !verify_sig(&sig.validator_id, &signing_payload, &sig.signature) {
            return Err(DhtValidationError::InvalidSignature {
                validator: sig.validator_id,
            });
        }
    }

    Ok(())
}

/// Construct the canonical signing payload for a DHT record.
/// Validators sign: kind || round (8 bytes LE) || data.
fn dht_signing_payload(record: &SignedDhtRecord) -> Vec<u8> {
    let kind_byte = match record.kind {
        DhtRecordKind::ValidatorSet => 0x01,
        DhtRecordKind::RelayProvider => 0x02,
        DhtRecordKind::ChainTip => 0x03,
    };
    let mut payload = Vec::with_capacity(1 + 8 + record.data.len());
    payload.push(kind_byte);
    payload.extend_from_slice(&record.round.to_le_bytes());
    payload.extend_from_slice(&record.data);
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(round: u64, sig_count: usize) -> SignedDhtRecord {
        let sigs: Vec<DhtRecordSignature> = (0..sig_count)
            .map(|i| DhtRecordSignature {
                validator_id: [i as u8; 32],
                signature: vec![0xAA; 64],
            })
            .collect();
        SignedDhtRecord {
            kind: DhtRecordKind::ValidatorSet,
            round,
            data: vec![1, 2, 3, 4],
            signatures: sigs,
        }
    }

    fn always_valid(_: &[u8; 32], _: &[u8], _: &[u8]) -> bool {
        true
    }

    fn always_invalid(_: &[u8; 32], _: &[u8], _: &[u8]) -> bool {
        false
    }

    #[test]
    fn valid_record_passes() {
        let record = make_record(100, 3);
        assert!(validate_dht_record(&record, 200, 2, always_valid).is_ok());
    }

    #[test]
    fn insufficient_signatures_rejected() {
        let record = make_record(100, 1);
        let err = validate_dht_record(&record, 200, 3, always_valid).unwrap_err();
        assert_eq!(
            err,
            DhtValidationError::InsufficientSignatures { got: 1, need: 3 }
        );
    }

    #[test]
    fn stale_record_rejected() {
        let record = make_record(100, 3);
        let err = validate_dht_record(&record, 100 + MAX_RECORD_AGE_ROUNDS + 1, 2, always_valid)
            .unwrap_err();
        assert!(matches!(err, DhtValidationError::StaleRecord { .. }));
    }

    #[test]
    fn empty_data_rejected() {
        let mut record = make_record(100, 3);
        record.data = vec![];
        let err = validate_dht_record(&record, 200, 2, always_valid).unwrap_err();
        assert_eq!(err, DhtValidationError::EmptyData);
    }

    #[test]
    fn invalid_signature_rejected() {
        let record = make_record(100, 3);
        let err = validate_dht_record(&record, 200, 2, always_invalid).unwrap_err();
        assert!(matches!(err, DhtValidationError::InvalidSignature { .. }));
    }

    #[test]
    fn fresh_record_near_boundary_accepted() {
        let record = make_record(100, 3);
        assert!(validate_dht_record(&record, 100 + MAX_RECORD_AGE_ROUNDS, 2, always_valid).is_ok());
    }

    #[test]
    fn signing_payload_deterministic() {
        let r1 = make_record(42, 1);
        let r2 = make_record(42, 1);
        assert_eq!(dht_signing_payload(&r1), dht_signing_payload(&r2));
    }

    #[test]
    fn different_kind_different_payload() {
        let mut r1 = make_record(42, 1);
        let mut r2 = make_record(42, 1);
        r1.kind = DhtRecordKind::ValidatorSet;
        r2.kind = DhtRecordKind::ChainTip;
        assert_ne!(dht_signing_payload(&r1), dht_signing_payload(&r2));
    }
}
