use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, info};

use aztibase_execution::AccountState;
use aztibase_storage::StateStore;

// ── JSON-RPC 2.0 Types ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: Some(result),
            error: None,
            id,
        }
    }

    fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            result: None,
            error: Some(JsonRpcError { code, message }),
            id,
        }
    }
}

// JSON-RPC error codes
const PARSE_ERROR: i32 = -32700;
const INVALID_REQUEST: i32 = -32600;
const METHOD_NOT_FOUND: i32 = -32601;
const INVALID_PARAMS: i32 = -32602;

// ── Shared State ────────────────────────────────────────────────────

/// Shared state accessible by RPC handlers. The execution pipeline updates
/// the AccountState via the write lock; RPC handlers read via the read lock.
pub struct RpcState {
    pub accounts: Arc<RwLock<AccountState>>,
    pub tx_sender: mpsc::Sender<Vec<u8>>,
    pub batch_count: Arc<std::sync::atomic::AtomicU64>,
    pub receipt_store: Option<Arc<StateStore>>,
    pub base_fee: Arc<std::sync::atomic::AtomicU64>,
    pub node_metrics: Option<Arc<RwLock<serde_json::Value>>>,
}

impl Clone for RpcState {
    fn clone(&self) -> Self {
        Self {
            accounts: Arc::clone(&self.accounts),
            tx_sender: self.tx_sender.clone(),
            batch_count: Arc::clone(&self.batch_count),
            receipt_store: self.receipt_store.clone(),
            base_fee: Arc::clone(&self.base_fee),
            node_metrics: self.node_metrics.clone(),
        }
    }
}

// ── RPC Server ──────────────────────────────────────────────────────

pub struct RpcServer {
    state: RpcState,
}

impl RpcServer {
    pub fn new(
        accounts: Arc<RwLock<AccountState>>,
        tx_sender: mpsc::Sender<Vec<u8>>,
        batch_count: Arc<std::sync::atomic::AtomicU64>,
        receipt_store: Option<Arc<StateStore>>,
        base_fee: Arc<std::sync::atomic::AtomicU64>,
    ) -> Self {
        Self {
            state: RpcState {
                accounts,
                tx_sender,
                batch_count,
                receipt_store,
                base_fee,
                node_metrics: None,
            },
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<RwLock<serde_json::Value>>) -> Self {
        self.state.node_metrics = Some(metrics);
        self
    }

    pub fn router(&self) -> Router {
        let mut router = Router::new().route("/", post(handle_rpc));
        if self.state.node_metrics.is_some() {
            router = router.route("/metrics", get(handle_metrics));
        }
        router
            .layer(DefaultBodyLimit::max(1_048_576))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, addr: SocketAddr) -> anyhow::Result<()> {
        let router = self.router();
        info!(%addr, "RPC server listening");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).await?;
        Ok(())
    }
}

// ── Request Handler ─────────────────────────────────────────────────

async fn handle_rpc(State(state): State<RpcState>, body: String) -> impl IntoResponse {
    let request: JsonRpcRequest = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::OK,
                Json(JsonRpcResponse::error(
                    serde_json::Value::Null,
                    PARSE_ERROR,
                    format!("Parse error: {e}"),
                )),
            );
        }
    };

    if request.jsonrpc != "2.0" {
        return (
            StatusCode::OK,
            Json(JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Invalid JSON-RPC version".into(),
            )),
        );
    }

    debug!(method = %request.method, "RPC request");

    let response = dispatch(&state, &request).await;
    (StatusCode::OK, Json(response))
}

async fn dispatch(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "aztb_getBalance" => handle_get_balance(state, req).await,
        "aztb_getNonce" => handle_get_nonce(state, req).await,
        "aztb_getCode" => handle_get_code(state, req).await,
        "aztb_sendTransaction" => handle_send_transaction(state, req).await,
        "aztb_blockNumber" => handle_block_number(state, req).await,
        "aztb_getStateRoot" => handle_get_state_root(state, req).await,
        "aztb_getTransactionReceipt" => handle_get_transaction_receipt(state, req).await,
        "aztb_getAccountType" => handle_get_account_type(state, req).await,
        "aztb_gasPrice" => handle_gas_price(state, req).await,
        "aztb_estimateGas" => handle_estimate_gas(state, req).await,
        _ => JsonRpcResponse::error(
            req.id.clone(),
            METHOD_NOT_FOUND,
            format!("Method not found: {}", req.method),
        ),
    }
}

// ── Method Handlers ─────────────────────────────────────────────────

fn parse_address(params: &serde_json::Value) -> Result<[u8; 32], String> {
    let hex_str = params
        .get(0)
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing address parameter".to_string())?;
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))?;
    let address: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "address must be 32 bytes".to_string())?;
    Ok(address)
}

async fn handle_get_balance(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let address = match parse_address(&req.params) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let accounts = state.accounts.read().await;
    let balance = accounts.balance(&address);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(format!("0x{balance:x}")))
}

async fn handle_get_nonce(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let address = match parse_address(&req.params) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let accounts = state.accounts.read().await;
    let nonce = accounts.nonce(&address);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(nonce))
}

async fn handle_get_code(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let address = match parse_address(&req.params) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let accounts = state.accounts.read().await;
    match accounts.code(&address) {
        Some(code) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!(format!("0x{}", hex::encode(code))),
        ),
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_send_transaction(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hex_str = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing transaction hex string".into(),
            );
        }
    };
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let raw = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(e) => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                format!("invalid hex: {e}"),
            );
        }
    };

    if let Err(e) = aztibase_execution::verify_and_route(&raw) {
        return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, format!("invalid tx: {e}"));
    }

    let tx_hash = aztibase_core::hash(&raw);

    match state.tx_sender.try_send(raw) {
        Ok(()) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!(format!("0x{}", hex::encode(tx_hash))),
        ),
        Err(mpsc::error::TrySendError::Full(_)) => {
            JsonRpcResponse::error(req.id.clone(), -32000, "mempool full".into())
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            JsonRpcResponse::error(req.id.clone(), -32000, "node shutting down".into())
        }
    }
}

async fn handle_block_number(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let count = state.batch_count.load(std::sync::atomic::Ordering::Relaxed);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(format!("0x{count:x}")))
}

async fn handle_get_state_root(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let accounts = state.accounts.read().await;
    let root = accounts.state_root();
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!(format!("0x{}", hex::encode(root))),
    )
}

async fn handle_get_account_type(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let address = match parse_address(&req.params) {
        Ok(a) => a,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let accounts = state.accounts.read().await;
    let account_type = accounts.account_type(&address);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(account_type.as_str()))
}

async fn handle_get_transaction_receipt(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hex_str = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing tx hash parameter".into(),
            );
        }
    };
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let hash_bytes = match hex::decode(hex_str) {
        Ok(b) => b,
        Err(e) => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                format!("invalid hex: {e}"),
            );
        }
    };
    let tx_hash: [u8; 32] = match hash_bytes.try_into() {
        Ok(h) => h,
        Err(_) => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "tx hash must be 32 bytes".into(),
            );
        }
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "receipt store not available".into(),
            );
        }
    };

    match aztibase_execution::get_receipt(store, &tx_hash) {
        Ok(Some(receipt)) => {
            let mut result = serde_json::json!({
                "txHash": format!("0x{}", hex::encode(receipt.tx_hash)),
                "success": receipt.success,
                "gasUsed": format!("0x{:x}", receipt.gas_used),
            });
            if let Some(addr) = receipt.contract_address {
                result["contractAddress"] = serde_json::json!(format!("0x{}", hex::encode(addr)));
            }
            if let Some(err) = &receipt.error {
                result["error"] = serde_json::json!(err);
            }
            if let Some(ih) = &receipt.inference_hash {
                result["inferenceHash"] = serde_json::json!(format!("0x{}", hex::encode(ih)));
            }
            JsonRpcResponse::success(req.id.clone(), result)
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_gas_price(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let base_fee = state.base_fee.load(std::sync::atomic::Ordering::Relaxed);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(format!("0x{base_fee:x}")))
}

async fn handle_estimate_gas(_state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let prefix = req
        .params
        .get(0)
        .and_then(|v| v.as_str())
        .and_then(|s| {
            let s = s.strip_prefix("0x").unwrap_or(s);
            u8::from_str_radix(s, 16).ok()
        })
        .unwrap_or(0x01);
    let estimate = aztibase_execution::BaseFeeCalculator::estimate_gas(prefix);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(format!("0x{estimate:x}")))
}

// ── Metrics Endpoint ────────────────────────────────────────────────

async fn handle_metrics(State(state): State<RpcState>) -> impl IntoResponse {
    match &state.node_metrics {
        Some(metrics) => {
            let data = metrics.read().await;
            (StatusCode::OK, Json(data.clone()))
        }
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "metrics not enabled"})),
        ),
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> (RpcState, mpsc::Receiver<Vec<u8>>) {
        let (tx, rx) = mpsc::channel(64);
        let state = RpcState {
            accounts: Arc::new(RwLock::new(AccountState::new())),
            tx_sender: tx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            receipt_store: None,
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            node_metrics: None,
        };
        (state, rx)
    }

    fn test_state_with_accounts() -> (RpcState, mpsc::Receiver<Vec<u8>>) {
        let mut accounts = AccountState::new();
        let addr = [0x01u8; 32];
        accounts.set_balance(&addr, 1000);
        accounts.get_mut(&addr).nonce = 5;
        accounts.set_code(&addr, vec![0x00, 0x61, 0x73, 0x6d]);

        let (tx, rx) = mpsc::channel(64);
        let state = RpcState {
            accounts: Arc::new(RwLock::new(accounts)),
            tx_sender: tx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(42)),
            receipt_store: None,
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            node_metrics: None,
        };
        (state, rx)
    }

    async fn rpc_call(state: &RpcState, body: &str) -> serde_json::Value {
        let router = Router::new()
            .route("/", post(handle_rpc))
            .with_state(state.clone());

        let request = Request::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn malformed_json_returns_parse_error() {
        let (state, _rx) = test_state();
        let resp = rpc_call(&state, "not json").await;
        assert_eq!(resp["error"]["code"], PARSE_ERROR);
    }

    #[tokio::test]
    async fn invalid_version_returns_error() {
        let (state, _rx) = test_state();
        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"1.0","method":"aztb_blockNumber","id":1}"#,
        )
        .await;
        assert_eq!(resp["error"]["code"], INVALID_REQUEST);
    }

    #[tokio::test]
    async fn unknown_method_returns_error() {
        let (state, _rx) = test_state();
        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"unknown_method","id":1}"#,
        )
        .await;
        assert_eq!(resp["error"]["code"], METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn get_balance_known_address() {
        let (state, _rx) = test_state_with_accounts();
        let addr_hex = hex::encode([0x01u8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getBalance","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], "0x3e8");
    }

    #[tokio::test]
    async fn get_balance_unknown_address() {
        let (state, _rx) = test_state();
        let addr_hex = hex::encode([0xFFu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getBalance","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], "0x0");
    }

    #[tokio::test]
    async fn get_balance_invalid_address() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_getBalance","params":["0xDEAD"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn get_nonce_known_address() {
        let (state, _rx) = test_state_with_accounts();
        let addr_hex = hex::encode([0x01u8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getNonce","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], 5);
    }

    #[tokio::test]
    async fn get_nonce_unknown_address() {
        let (state, _rx) = test_state();
        let addr_hex = hex::encode([0xAAu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getNonce","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], 0);
    }

    #[tokio::test]
    async fn get_code_contract_address() {
        let (state, _rx) = test_state_with_accounts();
        let addr_hex = hex::encode([0x01u8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getCode","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], "0x0061736d");
    }

    #[tokio::test]
    async fn get_code_eoa_returns_null() {
        let (state, _rx) = test_state();
        let addr_hex = hex::encode([0xBBu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getCode","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["result"].is_null());
    }

    #[tokio::test]
    async fn send_transaction_valid() {
        let (state, _rx) = test_state();
        let kp = aztibase_core::Keypair::generate();
        let sender = aztibase_core::address_from_pubkey(kp.public_key().as_bytes());
        let tx = aztibase_execution::TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 0,
        };
        let signed = aztibase_execution::SignedTx::new(tx.encode(), &kp);
        let tx_hex = hex::encode(signed.encode());
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_sendTransaction","params":["0x{tx_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {}", resp);
        assert!(resp["result"].as_str().unwrap().starts_with("0x"));
    }

    #[tokio::test]
    async fn send_transaction_malformed() {
        let (state, _rx) = test_state();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_sendTransaction","params":["0xDEADBEEF"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn block_number_returns_count() {
        let (state, _rx) = test_state_with_accounts();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_blockNumber","id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["result"], "0x2a");
    }

    #[tokio::test]
    async fn get_state_root_empty() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_getStateRoot","id":1}"#;
        let resp = rpc_call(&state, body).await;
        let hex_str = resp["result"].as_str().unwrap();
        assert!(hex_str.starts_with("0x"));
        assert_eq!(hex_str.len(), 66);
    }

    #[tokio::test]
    async fn get_state_root_with_data() {
        let (state, _rx) = test_state_with_accounts();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_getStateRoot","id":1}"#;
        let resp = rpc_call(&state, body).await;
        let hex_str = resp["result"].as_str().unwrap();
        assert!(hex_str.starts_with("0x"));
        assert_ne!(hex_str, &format!("0x{}", hex::encode([0u8; 32])));
    }

    // ── Receipt tests ────────────────────────────────────────────────

    use std::sync::atomic::{AtomicU32, Ordering};

    static RECEIPT_TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn receipt_test_db_path() -> std::path::PathBuf {
        let id = RECEIPT_TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_rpc_receipt_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn test_state_with_receipts() -> (RpcState, mpsc::Receiver<Vec<u8>>, std::path::PathBuf) {
        let path = receipt_test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let store = Arc::new(store);

        let tx_hash = aztibase_core::hash(b"test-receipt-tx");
        let receipt = aztibase_execution::ExecutionReceipt {
            tx_hash,
            success: true,
            gas_used: 21_000,
            contract_address: None,
            error: None,
            inference_hash: None,
            anomaly_score: 0.0,
        };
        aztibase_execution::store_receipts(&store, &[receipt]).unwrap();

        let (tx, rx) = mpsc::channel(64);
        let state = RpcState {
            accounts: Arc::new(RwLock::new(AccountState::new())),
            tx_sender: tx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            receipt_store: Some(store),
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            node_metrics: None,
        };
        (state, rx, path)
    }

    #[tokio::test]
    async fn get_receipt_found() {
        let (state, _rx, path) = test_state_with_receipts();
        let tx_hash = aztibase_core::hash(b"test-receipt-tx");
        let tx_hex = hex::encode(tx_hash);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getTransactionReceipt","params":["0x{tx_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(resp["result"]["success"], true);
        assert_eq!(resp["result"]["gasUsed"], "0x5208");
        assert!(resp["result"]["txHash"].as_str().unwrap().starts_with("0x"));
        cleanup(&path);
    }

    #[tokio::test]
    async fn get_receipt_not_found() {
        let (state, _rx, path) = test_state_with_receipts();
        let missing = hex::encode([0xFFu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getTransactionReceipt","params":["0x{missing}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none());
        assert!(resp["result"].is_null());
        cleanup(&path);
    }

    #[tokio::test]
    async fn get_receipt_no_store_returns_error() {
        let (state, _rx) = test_state();
        let hash_hex = hex::encode([0xAAu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getTransactionReceipt","params":["0x{hash_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["error"]["code"], -32000);
    }

    #[tokio::test]
    async fn get_receipt_invalid_hash() {
        let (state, _rx, path) = test_state_with_receipts();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_getTransactionReceipt","params":["0xDEAD"],"id":1}"#;
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["error"]["code"], INVALID_PARAMS);
        cleanup(&path);
    }

    #[tokio::test]
    async fn get_account_type_eoa() {
        let (state, _rx) = test_state();
        let addr_hex = hex::encode([0x01u8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getAccountType","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], "EOA");
    }

    #[tokio::test]
    async fn get_account_type_contract() {
        let (state, _rx) = test_state_with_accounts();
        let addr_hex = hex::encode([0x01u8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getAccountType","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert_eq!(resp["result"], "Contract");
    }

    #[tokio::test]
    async fn gas_price_returns_base_fee() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_gasPrice","params":[],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["result"], "0x1");
    }

    #[tokio::test]
    async fn estimate_gas_transfer() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_estimateGas","params":["0x01"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["result"], "0x5208"); // 21000
    }

    #[tokio::test]
    async fn estimate_gas_create_agent() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_estimateGas","params":["0x07"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["result"], "0xcf08"); // 53000
    }

    // ── Metrics endpoint tests ──────────────────────────────────────

    async fn metrics_call(state: &RpcState) -> (StatusCode, serde_json::Value) {
        let router = Router::new()
            .route("/metrics", get(handle_metrics))
            .with_state(state.clone());

        let request = Request::builder()
            .method("GET")
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, json)
    }

    #[tokio::test]
    async fn metrics_endpoint_returns_data() {
        let (mut state, _rx) = test_state();
        let metrics = Arc::new(RwLock::new(serde_json::json!({
            "consensus": {
                "vertices_proposed": 10,
                "commits": 3
            },
            "execution": {
                "batch_count": 5,
                "base_fee": 1
            }
        })));
        state.node_metrics = Some(metrics);

        let (status, json) = metrics_call(&state).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["consensus"]["vertices_proposed"], 10);
        assert_eq!(json["execution"]["batch_count"], 5);
    }

    #[tokio::test]
    async fn metrics_endpoint_disabled_returns_503() {
        let (state, _rx) = test_state();
        let (status, json) = metrics_call(&state).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(json["error"].as_str().unwrap().contains("not enabled"));
    }
}
