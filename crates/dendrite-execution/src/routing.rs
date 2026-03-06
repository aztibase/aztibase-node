use serde::{Deserialize, Serialize};

type Address = [u8; 32];

const PREFIX_TRANSFER: u8 = 0x01;
const PREFIX_DEPLOY: u8 = 0x02;
const PREFIX_CALL: u8 = 0x03;

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
}

#[derive(Debug, PartialEq, Eq)]
pub enum RoutingError {
    EmptyPayload,
    UnknownPrefix(u8),
    DecodeFailed(String),
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::EmptyPayload => write!(f, "empty transaction payload"),
            RoutingError::UnknownPrefix(p) => write!(f, "unknown tx prefix: 0x{p:02x}"),
            RoutingError::DecodeFailed(e) => write!(f, "tx decode failed: {e}"),
        }
    }
}

impl std::error::Error for RoutingError {}

/// Decode raw payload bytes into a typed transaction.
/// Wire format: [prefix_byte][bincode-encoded TxKind].
pub fn route_tx(raw: &[u8]) -> Result<TxKind, RoutingError> {
    let (&prefix, body) = raw.split_first().ok_or(RoutingError::EmptyPayload)?;
    match prefix {
        PREFIX_TRANSFER | PREFIX_DEPLOY | PREFIX_CALL => {}
        other => return Err(RoutingError::UnknownPrefix(other)),
    }
    bincode::deserialize(body).map_err(|e| RoutingError::DecodeFailed(e.to_string()))
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
}
