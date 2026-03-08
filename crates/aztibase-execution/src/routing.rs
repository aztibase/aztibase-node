use serde::{Deserialize, Serialize};

type Address = [u8; 32];

const PREFIX_TRANSFER: u8 = 0x01;
const PREFIX_DEPLOY: u8 = 0x02;
const PREFIX_CALL: u8 = 0x03;
const PREFIX_EVM_DEPLOY: u8 = 0x04;
const PREFIX_EVM_CALL: u8 = 0x05;
const PREFIX_AI_INFER: u8 = 0x06;
const PREFIX_CREATE_AGENT: u8 = 0x07;
const PREFIX_REGISTER_MODEL: u8 = 0x08;
const PREFIX_POST_TASK: u8 = 0x09;
const PREFIX_SUBMIT_ATTESTATION: u8 = 0x0A;
const PREFIX_COMMIT_COMPUTE: u8 = 0x0B;
const PREFIX_DEREGISTER_COMPUTE: u8 = 0x0C;
const PREFIX_DEREGISTER_MODEL: u8 = 0x0D;

/// Maximum encoded transaction size (1 MB). Rejects oversized payloads before
/// deserialization to prevent memory-bomb attacks via oversized payloads.
const MAX_TX_SIZE: u64 = 1_048_576;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    Transfer {
        from: Address,
        to: Address,
        value: u64,
        nonce: u64,
        gas_price: u64,
    },
    ContractDeploy {
        deployer: Address,
        code: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    },
    ContractCall {
        caller: Address,
        contract: Address,
        func_name: String,
        args_data: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    },
    EvmDeploy {
        deployer: Address,
        code: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
        gas_price: u64,
    },
    EvmCall {
        caller: Address,
        contract: Address,
        calldata: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
        value: u64,
        gas_price: u64,
    },
    AiInfer {
        requester: Address,
        model_id: String,
        input: Vec<u8>,
        nonce: u64,
        max_compute_units: u64,
        gas_price: u64,
    },
    CreateAgent {
        creator: Address,
        model_id: String,
        nonce: u64,
        gas_price: u64,
    },
    RegisterModel {
        owner: Address,
        model_id: String,
        fingerprint: [u8; 32],
        compute_cost: u64,
        min_stake: u64,
        nonce: u64,
        gas_price: u64,
    },
    PostTask {
        requester: Address,
        model_id: String,
        input_hash: [u8; 32],
        reward: u64,
        deadline_round: u64,
        nonce: u64,
        gas_price: u64,
    },
    SubmitAttestation {
        validator: Address,
        task_id: [u8; 32],
        result_hash: [u8; 32],
        compute_units: u64,
        signature: Vec<u8>,
        nonce: u64,
        gas_price: u64,
    },
    CommitCompute {
        validator: Address,
        supported_models: Vec<String>,
        committed_stake: u64,
        bls_pubkey: Vec<u8>,
        bls_pop: Vec<u8>,
        nonce: u64,
        gas_price: u64,
    },
    DeregisterCompute {
        validator: Address,
        nonce: u64,
        gas_price: u64,
    },
    DeregisterModel {
        owner: Address,
        model_id: String,
        nonce: u64,
        gas_price: u64,
    },
}

impl TxKind {
    /// Serialize to wire format: [prefix_byte][postcard payload].
    pub fn encode(&self) -> Vec<u8> {
        let prefix = match self {
            TxKind::Transfer { .. } => PREFIX_TRANSFER,
            TxKind::ContractDeploy { .. } => PREFIX_DEPLOY,
            TxKind::ContractCall { .. } => PREFIX_CALL,
            TxKind::EvmDeploy { .. } => PREFIX_EVM_DEPLOY,
            TxKind::EvmCall { .. } => PREFIX_EVM_CALL,
            TxKind::AiInfer { .. } => PREFIX_AI_INFER,
            TxKind::CreateAgent { .. } => PREFIX_CREATE_AGENT,
            TxKind::RegisterModel { .. } => PREFIX_REGISTER_MODEL,
            TxKind::PostTask { .. } => PREFIX_POST_TASK,
            TxKind::SubmitAttestation { .. } => PREFIX_SUBMIT_ATTESTATION,
            TxKind::CommitCompute { .. } => PREFIX_COMMIT_COMPUTE,
            TxKind::DeregisterCompute { .. } => PREFIX_DEREGISTER_COMPUTE,
            TxKind::DeregisterModel { .. } => PREFIX_DEREGISTER_MODEL,
        };
        let payload = postcard::to_allocvec(self).expect("TxKind serialization cannot fail");
        let mut buf = Vec::with_capacity(1 + payload.len());
        buf.push(prefix);
        buf.extend_from_slice(&payload);
        buf
    }

    pub fn nonce(&self) -> u64 {
        match self {
            TxKind::Transfer { nonce, .. }
            | TxKind::ContractDeploy { nonce, .. }
            | TxKind::ContractCall { nonce, .. }
            | TxKind::EvmDeploy { nonce, .. }
            | TxKind::EvmCall { nonce, .. }
            | TxKind::AiInfer { nonce, .. }
            | TxKind::CreateAgent { nonce, .. }
            | TxKind::RegisterModel { nonce, .. }
            | TxKind::PostTask { nonce, .. }
            | TxKind::SubmitAttestation { nonce, .. }
            | TxKind::CommitCompute { nonce, .. }
            | TxKind::DeregisterCompute { nonce, .. }
            | TxKind::DeregisterModel { nonce, .. } => *nonce,
        }
    }

    pub fn gas_price(&self) -> u64 {
        match self {
            TxKind::Transfer { gas_price, .. }
            | TxKind::ContractDeploy { gas_price, .. }
            | TxKind::ContractCall { gas_price, .. }
            | TxKind::EvmDeploy { gas_price, .. }
            | TxKind::EvmCall { gas_price, .. }
            | TxKind::AiInfer { gas_price, .. }
            | TxKind::CreateAgent { gas_price, .. }
            | TxKind::RegisterModel { gas_price, .. }
            | TxKind::PostTask { gas_price, .. }
            | TxKind::SubmitAttestation { gas_price, .. }
            | TxKind::CommitCompute { gas_price, .. }
            | TxKind::DeregisterCompute { gas_price, .. }
            | TxKind::DeregisterModel { gas_price, .. } => *gas_price,
        }
    }

    pub fn gas_limit(&self) -> u64 {
        match self {
            TxKind::Transfer { .. } => 21_000,
            TxKind::ContractDeploy { gas_limit, .. }
            | TxKind::ContractCall { gas_limit, .. }
            | TxKind::EvmDeploy { gas_limit, .. }
            | TxKind::EvmCall { gas_limit, .. } => *gas_limit,
            TxKind::AiInfer {
                max_compute_units, ..
            } => *max_compute_units,
            TxKind::CreateAgent { .. } => 53_000,
            TxKind::RegisterModel { .. } => 100_000,
            TxKind::PostTask { .. } => 42_000,
            TxKind::SubmitAttestation { .. } => 50_000,
            TxKind::CommitCompute { .. } => 75_000,
            TxKind::DeregisterCompute { .. } => 50_000,
            TxKind::DeregisterModel { .. } => 60_000,
        }
    }

    pub fn sender(&self) -> &Address {
        match self {
            TxKind::Transfer { from, .. } => from,
            TxKind::ContractDeploy { deployer, .. } => deployer,
            TxKind::ContractCall { caller, .. } => caller,
            TxKind::EvmDeploy { deployer, .. } => deployer,
            TxKind::EvmCall { caller, .. } => caller,
            TxKind::AiInfer { requester, .. } => requester,
            TxKind::CreateAgent { creator, .. } => creator,
            TxKind::RegisterModel { owner, .. } => owner,
            TxKind::PostTask { requester, .. } => requester,
            TxKind::SubmitAttestation { validator, .. } => validator,
            TxKind::CommitCompute { validator, .. } => validator,
            TxKind::DeregisterCompute { validator, .. } => validator,
            TxKind::DeregisterModel { owner, .. } => owner,
        }
    }

    fn expected_prefix(&self) -> u8 {
        match self {
            TxKind::Transfer { .. } => PREFIX_TRANSFER,
            TxKind::ContractDeploy { .. } => PREFIX_DEPLOY,
            TxKind::ContractCall { .. } => PREFIX_CALL,
            TxKind::EvmDeploy { .. } => PREFIX_EVM_DEPLOY,
            TxKind::EvmCall { .. } => PREFIX_EVM_CALL,
            TxKind::AiInfer { .. } => PREFIX_AI_INFER,
            TxKind::CreateAgent { .. } => PREFIX_CREATE_AGENT,
            TxKind::RegisterModel { .. } => PREFIX_REGISTER_MODEL,
            TxKind::PostTask { .. } => PREFIX_POST_TASK,
            TxKind::SubmitAttestation { .. } => PREFIX_SUBMIT_ATTESTATION,
            TxKind::CommitCompute { .. } => PREFIX_COMMIT_COMPUTE,
            TxKind::DeregisterCompute { .. } => PREFIX_DEREGISTER_COMPUTE,
            TxKind::DeregisterModel { .. } => PREFIX_DEREGISTER_MODEL,
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
/// Wire format: [prefix_byte][postcard-encoded TxKind].
pub fn route_tx(raw: &[u8]) -> Result<TxKind, RoutingError> {
    if raw.len() as u64 > MAX_TX_SIZE {
        return Err(RoutingError::OversizedPayload(raw.len()));
    }
    let (&prefix, body) = raw.split_first().ok_or(RoutingError::EmptyPayload)?;
    match prefix {
        PREFIX_TRANSFER
        | PREFIX_DEPLOY
        | PREFIX_CALL
        | PREFIX_EVM_DEPLOY
        | PREFIX_EVM_CALL
        | PREFIX_AI_INFER
        | PREFIX_CREATE_AGENT
        | PREFIX_REGISTER_MODEL
        | PREFIX_POST_TASK
        | PREFIX_SUBMIT_ATTESTATION
        | PREFIX_COMMIT_COMPUTE
        | PREFIX_DEREGISTER_COMPUTE
        | PREFIX_DEREGISTER_MODEL => {}
        other => return Err(RoutingError::UnknownPrefix(other)),
    }
    let (decoded, remaining): (TxKind, &[u8]) =
        postcard::take_from_bytes(body).map_err(|e| RoutingError::DecodeFailed(e.to_string()))?;
    if !remaining.is_empty() {
        return Err(RoutingError::DecodeFailed(format!(
            "trailing bytes: {}",
            remaining.len()
        )));
    }
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
    if let TxKind::AiInfer { ref model_id, .. } = decoded
        && !is_valid_func_name(model_id)
    {
        return Err(RoutingError::InvalidFuncName(model_id.clone()));
    }
    if let TxKind::CreateAgent { ref model_id, .. } = decoded
        && !is_valid_func_name(model_id)
    {
        return Err(RoutingError::InvalidFuncName(model_id.clone()));
    }
    if let TxKind::RegisterModel { ref model_id, .. } = decoded
        && !is_valid_func_name(model_id)
    {
        return Err(RoutingError::InvalidFuncName(model_id.clone()));
    }
    if let TxKind::PostTask { ref model_id, .. } = decoded
        && !is_valid_func_name(model_id)
    {
        return Err(RoutingError::InvalidFuncName(model_id.clone()));
    }
    if let TxKind::DeregisterModel { ref model_id, .. } = decoded
        && !is_valid_func_name(model_id)
    {
        return Err(RoutingError::InvalidFuncName(model_id.clone()));
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
        };
        let mut encoded = tx.encode();
        encoded.push(0xFF);
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::DecodeFailed(_))
        ));
    }

    #[test]
    fn evm_deploy_roundtrip() {
        let tx = TxKind::EvmDeploy {
            deployer: [6u8; 32],
            code: vec![0x60, 0x00, 0x60, 0x00, 0xf3],
            nonce: 0,
            gas_limit: 1_000_000,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_EVM_DEPLOY);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn evm_call_roundtrip() {
        let tx = TxKind::EvmCall {
            caller: [7u8; 32],
            contract: [8u8; 32],
            calldata: vec![0xa9, 0x05, 0x9c, 0xbb],
            nonce: 1,
            gas_limit: 500_000,
            value: 0,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_EVM_CALL);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn prefix_only_no_body() {
        assert!(matches!(
            route_tx(&[PREFIX_TRANSFER]),
            Err(RoutingError::DecodeFailed(_))
        ));
    }

    #[test]
    fn ai_infer_roundtrip() {
        let tx = TxKind::AiInfer {
            requester: [9u8; 32],
            model_id: "sentiment_v1".into(),
            input: vec![0.5f32, 0.3, 0.8]
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect(),
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_AI_INFER);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn create_agent_roundtrip() {
        let tx = TxKind::CreateAgent {
            creator: [10u8; 32],
            model_id: "sentiment_v1".into(),
            nonce: 0,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_CREATE_AGENT);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded, tx);
    }

    #[test]
    fn create_agent_empty_model_rejected() {
        let tx = TxKind::CreateAgent {
            creator: [10u8; 32],
            model_id: "".into(),
            nonce: 0,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::InvalidFuncName(_))
        ));
    }

    #[test]
    fn ai_infer_empty_model_id_rejected() {
        let tx = TxKind::AiInfer {
            requester: [9u8; 32],
            model_id: "".into(),
            input: vec![1, 2, 3],
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 0,
        };
        let encoded = tx.encode();
        assert!(matches!(
            route_tx(&encoded),
            Err(RoutingError::InvalidFuncName(_))
        ));
    }

    #[test]
    fn register_model_roundtrip() {
        let tx = TxKind::RegisterModel {
            owner: [0xA0; 32],
            model_id: "llama_7b".into(),
            fingerprint: [0xBB; 32],
            compute_cost: 500,
            min_stake: 1000,
            nonce: 3,
            gas_price: 2,
        };
        let encoded = tx.encode();
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 3);
        assert_eq!(decoded.gas_price(), 2);
        assert_eq!(*decoded.sender(), [0xA0; 32]);
    }

    #[test]
    fn post_task_roundtrip() {
        let tx = TxKind::PostTask {
            requester: [0xC0; 32],
            model_id: "llama_7b".into(),
            input_hash: [0xDD; 32],
            reward: 100,
            deadline_round: 50,
            nonce: 7,
            gas_price: 1,
        };
        let encoded = tx.encode();
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 7);
        assert_eq!(decoded.gas_price(), 1);
        assert_eq!(*decoded.sender(), [0xC0; 32]);
    }

    #[test]
    fn submit_attestation_roundtrip() {
        let tx = TxKind::SubmitAttestation {
            validator: [0xE0; 32],
            task_id: [0x11; 32],
            result_hash: [0x22; 32],
            compute_units: 42,
            signature: vec![0xAA; 64],
            nonce: 10,
            gas_price: 3,
        };
        let encoded = tx.encode();
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 10);
        assert_eq!(decoded.gas_price(), 3);
        assert_eq!(*decoded.sender(), [0xE0; 32]);
    }

    #[test]
    fn deregister_compute_roundtrip() {
        let tx = TxKind::DeregisterCompute {
            validator: [0xD0; 32],
            nonce: 5,
            gas_price: 1,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_DEREGISTER_COMPUTE);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 5);
        assert_eq!(decoded.gas_price(), 1);
        assert_eq!(*decoded.sender(), [0xD0; 32]);
    }

    #[test]
    fn deregister_model_roundtrip() {
        let tx = TxKind::DeregisterModel {
            owner: [0xE0; 32],
            model_id: "llama_7b".into(),
            nonce: 8,
            gas_price: 2,
        };
        let encoded = tx.encode();
        assert_eq!(encoded[0], PREFIX_DEREGISTER_MODEL);
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 8);
        assert_eq!(decoded.gas_price(), 2);
        assert_eq!(*decoded.sender(), [0xE0; 32]);
    }

    #[test]
    fn commit_compute_roundtrip() {
        let tx = TxKind::CommitCompute {
            validator: [0xF0; 32],
            supported_models: vec!["llama-7b".into(), "gpt-neo".into()],
            committed_stake: 5000,
            bls_pubkey: vec![0xAB; 48],
            bls_pop: vec![0xCD; 96],
            nonce: 15,
            gas_price: 4,
        };
        let encoded = tx.encode();
        let decoded = route_tx(&encoded).unwrap();
        assert_eq!(decoded.nonce(), 15);
        assert_eq!(decoded.gas_price(), 4);
        assert_eq!(*decoded.sender(), [0xF0; 32]);
    }
}
