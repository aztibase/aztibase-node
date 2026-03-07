use aztibase_core::{Keypair, PublicKey, address_from_pubkey};

use crate::routing::{RoutingError, TxKind, route_tx};

type Address = [u8; 32];

const ENVELOPE_MAGIC: u8 = 0xAA;
const MAX_ENVELOPE_SIZE: usize = 1_048_576;
const MIN_ENVELOPE_SIZE: usize = 1 + 4 + 32 + 64; // magic + payload_len + pubkey + sig

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedTx {
    pub payload: Vec<u8>,
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
}

#[derive(Debug, PartialEq, Eq)]
pub enum TxError {
    TooShort,
    InvalidMagic(u8),
    LengthMismatch { expected: usize, actual: usize },
    Oversized(usize),
    InvalidSignature,
    SenderMismatch { envelope: Address, payload: Address },
    RoutingFailed(RoutingError),
}

impl std::fmt::Display for TxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxError::TooShort => write!(f, "envelope too short"),
            TxError::InvalidMagic(b) => write!(f, "invalid envelope magic: 0x{b:02x}"),
            TxError::LengthMismatch { expected, actual } => {
                write!(f, "envelope length: expected {expected}, got {actual}")
            }
            TxError::Oversized(len) => write!(f, "envelope too large: {len} bytes"),
            TxError::InvalidSignature => write!(f, "signature verification failed"),
            TxError::SenderMismatch { .. } => {
                write!(f, "sender address does not match signer")
            }
            TxError::RoutingFailed(e) => write!(f, "routing: {e}"),
        }
    }
}

impl std::error::Error for TxError {}

impl SignedTx {
    /// Sign a TxKind-encoded payload with an Ed25519 keypair.
    pub fn new(payload: Vec<u8>, keypair: &Keypair) -> Self {
        let sig_bytes = keypair.sign(&payload);
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&sig_bytes);
        Self {
            payload,
            public_key: *keypair.public_key().as_bytes(),
            signature,
        }
    }

    /// Verify the Ed25519 signature against the embedded public key.
    pub fn verify(&self) -> bool {
        let pk = match PublicKey::from_bytes(&self.public_key) {
            Some(pk) => pk,
            None => return false,
        };
        pk.verify(&self.payload, &self.signature)
    }

    /// Derive the sender address (BLAKE3 hash of the public key).
    pub fn sender_address(&self) -> Address {
        address_from_pubkey(&self.public_key)
    }

    /// Serialize to wire format: `[0xAA][payload_len:u32 LE][payload][pubkey:32][sig:64]`.
    pub fn encode(&self) -> Vec<u8> {
        let payload_len = self.payload.len() as u32;
        let total = MIN_ENVELOPE_SIZE + self.payload.len();
        let mut buf = Vec::with_capacity(total);
        buf.push(ENVELOPE_MAGIC);
        buf.extend_from_slice(&payload_len.to_le_bytes());
        buf.extend_from_slice(&self.payload);
        buf.extend_from_slice(&self.public_key);
        buf.extend_from_slice(&self.signature);
        buf
    }

    /// Deserialize from wire format. Validates structure only, not signature.
    pub fn decode(data: &[u8]) -> Result<Self, TxError> {
        if data.len() > MAX_ENVELOPE_SIZE {
            return Err(TxError::Oversized(data.len()));
        }
        if data.len() < MIN_ENVELOPE_SIZE {
            return Err(TxError::TooShort);
        }
        if data[0] != ENVELOPE_MAGIC {
            return Err(TxError::InvalidMagic(data[0]));
        }
        let payload_len = u32::from_le_bytes([data[1], data[2], data[3], data[4]]) as usize;
        let expected = MIN_ENVELOPE_SIZE + payload_len;
        if data.len() != expected {
            return Err(TxError::LengthMismatch {
                expected,
                actual: data.len(),
            });
        }
        let payload = data[5..5 + payload_len].to_vec();
        let pk_start = 5 + payload_len;
        let mut public_key = [0u8; 32];
        public_key.copy_from_slice(&data[pk_start..pk_start + 32]);
        let sig_start = pk_start + 32;
        let mut signature = [0u8; 64];
        signature.copy_from_slice(&data[sig_start..sig_start + 64]);
        Ok(Self {
            payload,
            public_key,
            signature,
        })
    }
}

/// Decode a signed envelope, verify the signature, route the inner payload,
/// and confirm that the derived sender address matches the transaction's sender field.
pub fn verify_and_route(raw: &[u8]) -> Result<TxKind, TxError> {
    let signed = SignedTx::decode(raw)?;
    if !signed.verify() {
        return Err(TxError::InvalidSignature);
    }
    let sender = signed.sender_address();
    let tx = route_tx(&signed.payload).map_err(TxError::RoutingFailed)?;
    if *tx.sender() != sender {
        return Err(TxError::SenderMismatch {
            envelope: sender,
            payload: *tx.sender(),
        });
    }
    Ok(tx)
}

/// Verify and route a batch of signed envelopes, collecting successes and errors.
pub fn verify_and_route_batch(raw_txs: &[Vec<u8>]) -> (Vec<TxKind>, Vec<(usize, TxError)>) {
    let mut routed = Vec::with_capacity(raw_txs.len());
    let mut errors = Vec::new();
    for (i, raw) in raw_txs.iter().enumerate() {
        match verify_and_route(raw) {
            Ok(tx) => routed.push(tx),
            Err(e) => errors.push((i, e)),
        }
    }
    (routed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::Keypair;

    fn make_transfer(kp: &Keypair) -> (TxKind, Vec<u8>) {
        let sender = address_from_pubkey(kp.public_key().as_bytes());
        let tx = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 0,
        };
        let payload = tx.encode();
        (tx, payload)
    }

    #[test]
    fn sign_verify_roundtrip() {
        let kp = Keypair::generate();
        let (_, payload) = make_transfer(&kp);
        let signed = SignedTx::new(payload, &kp);
        assert!(signed.verify());
    }

    #[test]
    fn tampered_payload_rejected() {
        let kp = Keypair::generate();
        let (_, payload) = make_transfer(&kp);
        let mut signed = SignedTx::new(payload, &kp);
        signed.payload[0] ^= 0xFF;
        assert!(!signed.verify());
    }

    #[test]
    fn sender_address_deterministic() {
        let kp = Keypair::generate();
        let expected = address_from_pubkey(kp.public_key().as_bytes());
        let signed = SignedTx::new(vec![1, 2, 3], &kp);
        assert_eq!(signed.sender_address(), expected);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let kp = Keypair::generate();
        let (_, payload) = make_transfer(&kp);
        let signed = SignedTx::new(payload, &kp);
        let encoded = signed.encode();
        let decoded = SignedTx::decode(&encoded).unwrap();
        assert_eq!(decoded, signed);
    }

    #[test]
    fn oversized_rejected() {
        let data = vec![0u8; MAX_ENVELOPE_SIZE + 1];
        assert_eq!(
            SignedTx::decode(&data),
            Err(TxError::Oversized(MAX_ENVELOPE_SIZE + 1))
        );
    }

    #[test]
    fn truncated_rejected() {
        let kp = Keypair::generate();
        let signed = SignedTx::new(vec![1, 2, 3], &kp);
        let mut encoded = signed.encode();
        encoded.truncate(50);
        assert!(matches!(SignedTx::decode(&encoded), Err(TxError::TooShort)));
    }

    #[test]
    fn wrong_magic_rejected() {
        let mut data = vec![0u8; MIN_ENVELOPE_SIZE];
        data[0] = 0x01;
        assert_eq!(SignedTx::decode(&data), Err(TxError::InvalidMagic(0x01)));
    }

    #[test]
    fn verify_and_route_valid_transfer() {
        let kp = Keypair::generate();
        let (expected_tx, payload) = make_transfer(&kp);
        let signed = SignedTx::new(payload, &kp);
        let routed = verify_and_route(&signed.encode()).unwrap();
        assert_eq!(routed, expected_tx);
    }

    #[test]
    fn verify_and_route_bad_signature() {
        let kp = Keypair::generate();
        let (_, payload) = make_transfer(&kp);
        let mut signed = SignedTx::new(payload, &kp);
        signed.signature[0] ^= 0xFF;
        assert_eq!(
            verify_and_route(&signed.encode()),
            Err(TxError::InvalidSignature)
        );
    }

    #[test]
    fn verify_and_route_sender_mismatch() {
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();
        let sender1 = address_from_pubkey(kp1.public_key().as_bytes());
        let tx = TxKind::Transfer {
            from: sender1,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 0,
        };
        let signed = SignedTx::new(tx.encode(), &kp2);
        assert!(matches!(
            verify_and_route(&signed.encode()),
            Err(TxError::SenderMismatch { .. })
        ));
    }

    #[test]
    fn verify_and_route_batch_mixed() {
        let kp = Keypair::generate();
        let (_, payload) = make_transfer(&kp);
        let good = SignedTx::new(payload, &kp).encode();
        let bad = vec![0xFF];
        let (routed, errors) = verify_and_route_batch(&[good, bad]);
        assert_eq!(routed.len(), 1);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 1);
    }
}
