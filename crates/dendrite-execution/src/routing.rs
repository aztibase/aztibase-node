use bincode::Options;
use serde::{Deserialize, Serialize};

type Address = [u8; 32];

const PREFIX_TRANSFER: u8 = 0x01;
const PREFIX_DEPLOY: u8 = 0x02;
const PREFIX_CALL: u8 = 0x03;

/// Maximum encoded transaction size (1 MB). Rejects oversized payloads before
/// deserialization to prevent memory-bomb attacks via bincode length prefixes.
const MAX_TX_SIZE: u64 = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    Transfer {
        from: Address,
        to: Address,
        value: u64,
        nonce: u64,
    },
    ContractDeploy {
        deployer: Address,
        code: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
    },
    ContractCall {
        caller: Address,
        contract: Address,
        func_name: String,
        args_data: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
    },
}

impl TxKind {
    /// Serialize to wire format: [prefix_byte][bincode payload].
    pub fn encode(&self) -> Vec<u8> {
        let prefix = match self {
            TxKind::Transfer { .. } => PREFIX_TRANSFER,
            TxKind::ContractDeploy { .. } => PREFIX_DEPLOY,
            TxKind::ContractCall { .. } => PREFIX_CALL,
        };
        let payload = bincode::serialize(self).expect("TxKind serialization cannot fail");
        let mut buf = Vec::with_capacity(1 + payload.len());
        buf.push(prefix);
        buf.extend_from_slice(&payload);
        buf
    }

    fn expected_prefix(&self) -> u8 {
        match self {
            TxKind::Transfer { .. } => PREFIX_TRANSFER,
            TxKind::ContractDeploy { .. } => PREFIX_DEPLOY,
            TxKind::ContractCall { .. } => PREFIX_CALL,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RoutingError {
    EmptyPayload,
    UnknownPrefix(u8),
    DecodeFailed(String),
    PrefixMismatch { declared: u8, actual: u8 },
    OversizedPayload(usize),
    InvalidFuncName(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::EmptyPayload => write!(f, "empty transaction payload"),
            RoutingError::UnknownPrefix(p) => write!(f, "unknown tx prefix: 0x{p:02x}"),
            RoutingError::DecodeFailed(e) => write!(f, "tx decode failed: {e}"),
            RoutingError::PrefixMismatch { declared, actual } => {
                write!(
                    f,
                    "prefix mismatch: declared 0x{declared:02x}, actual 0x{actual:02x}"
                )
            }
            RoutingError::OversizedPayload(len) => {
                write!(f, "payload too large: {len} bytes (max {MAX_TX_SIZE})")
            }
            RoutingError::InvalidFuncName(name) => {
                write!(f, "invalid function name: {name}")
            }
        }
    }
}

impl std::error::Error for RoutingError {}

fn bincode_options() -> impl Options {
    bincode::DefaultOptions::new()
        .with_limit(MAX_TX_SIZE)
        .with_fixint_encoding()
}

fn is_valid_func_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Decode raw payload bytes into a typed transaction.
/// Wire format: [prefix_byte][bincode-encoded TxKind].
pub fn route_tx(raw: &[u8]) -> Result<TxKind, RoutingError> {
    if raw.len() as u64 > MAX_TX_SIZE {
        return Err(RoutingError::OversizedPayload(raw.len()));
    }
    let (&prefix, body) = raw.split_first().ok_or(RoutingError::EmptyPayload)?;
    match prefix {
        PREFIX_TRANSFER | PREFIX_DEPLOY | PREFIX_CALL => {}
        other => return Err(RoutingError::UnknownPrefix(other)),
    }
    let decoded: TxKind = bincode_options()
        .deserialize(body)
        .map_err(|e| RoutingError::DecodeFailed(e.to_string()))?;
    let actual = decoded.expected_prefix();
    if prefix != actual {
        return Err(RoutingError::PrefixMismatch {
            declared: prefix,
            actual,
        });
    }
    if let TxKind::ContractCall { ref func_name, .. } = decoded
        && !is_valid_func_name(func_name)
    {
        return Err(RoutingError::InvalidFuncName(func_name.clone()));
    }
    Ok(decoded)
}

/// Classify and decode a batch of raw payloads, collecting successes and errors.
pub fn route_batch(raw_txs: &[Vec<u8>]) -> (Vec<TxKind>, Vec<(usize, RoutingError)>) {
    let mut routed = Vec::with_capacity(raw_txs.len());
    let mut errors = Vec::new();
    for (i, raw) in raw_txs.iter().enumerate() {
        match route_tx(raw) {
            Ok(tx) => routed.push(tx),
            Err(e) => errors.push((i, e)),
        }
    }
    (routed, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_roundtrip() {
        let tx = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 500,
            nonce: 3,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_TRANSFER);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn deploy_roundtrip() {
        let tx = TxKind::ContractDeploy {
            deployer: [3u8; 32],
            code: vec![0x00, 0x61, 0x73, 0x6d],
            nonce: 0,
            gas_limit: 1_000_000,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_DEPLOY);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn call_roundtrip() {
        let tx = TxKind::ContractCall {
            caller: [4u8; 32],
            contract: [5u8; 32],
            func_name: "transfer".into(),
            args_data: vec![1, 2, 3],
            nonce: 7,
            gas_limit: 500_000,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_CALL);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn empty_payload_error() {
        assert_eq!(route_tx(&[]), Err(RoutingError::EmptyPayload));
    }

    #[test]
    fn unknown_prefix_error() {
        assert_eq!(
            route_tx(&[0xFF, 0x00]),
            Err(RoutingError::UnknownPrefix(0xFF))
        );
    }

    #[test]
    fn corrupted_body_error() {
        let bad = vec![PREFIX_TRANSFER, 0xFF, 0xFF];
        assert!(matches!(route_tx(&bad), Err(RoutingError::DecodeFailed(_))));
    }

    #[test]
    fn route_batch_mixed() {
        let good = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 100,
            nonce: 0,
        };
        let bad = vec![0xFE];
        let (routed, errors) = route_batch(&[good.encode(), bad]);
        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0], good);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].0, 1);
    }

    #[test]
    fn prefix_mismatch_rejected() {
        let tx = TxKind::ContractDeploy {
            deployer: [3u8; 32],
            code: vec![0x00],
            nonce: 0,
            gas_limit: 100,
        };
        let mut encoded = tx.encode();
        encoded[0] = PREFIX_TRANSFER;
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::PrefixMismatch { .. })
        ));
    }

    #[test]
    fn valid_func_name_accepted() {
        let tx = TxKind::ContractCall {
            caller: [4u8; 32],
            contract: [5u8; 32],
            func_name: "transfer_tokens".into(),
            args_data: vec![],
            nonce: 0,
            gas_limit: 100_000,
        };
        let encoded = tx.encode();
        assert!(route_tx(&encoded).is_ok());
    }

    #[test]
    fn empty_func_name_rejected() {
        let tx = TxKind::ContractCall {
            caller: [4u8; 32],
            contract: [5u8; 32],
            func_name: "".into(),
            args_data: vec![],
            nonce: 0,
            gas_limit: 100_000,
        };
        let encoded = tx.encode();
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::InvalidFuncName(_))
        ));
    }

    #[test]
    fn special_char_func_name_rejected() {
        let tx = TxKind::ContractCall {
            caller: [4u8; 32],
            contract: [5u8; 32],
            func_name: "drop;--".into(),
            args_data: vec![],
            nonce: 0,
            gas_limit: 100_000,
        };
        let encoded = tx.encode();
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::InvalidFuncName(_))
        ));
    }

    #[test]
    fn too_long_func_name_rejected() {
        let tx = TxKind::ContractCall {
            caller: [4u8; 32],
            contract: [5u8; 32],
            func_name: "a".repeat(129),
            args_data: vec![],
            nonce: 0,
            gas_limit: 100_000,
        };
        let encoded = tx.encode();
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::InvalidFuncName(_))
        ));
    }

    #[test]
    fn trailing_bytes_rejected() {
        let tx = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 500,
            nonce: 3,
        };
        let mut encoded = tx.encode();
        encoded.push(0xFF);
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::DecodeFailed(_))
        ));
    }

    #[test]
    fn prefix_only_no_body() {
        assert!(matches!(
            route_tx(&[PREFIX_TRANSFER]),
            Err(RoutingError::DecodeFailed(_))
        ));
    }
}
