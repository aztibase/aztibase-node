use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{DefaultBodyLimit, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
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

#[derive(Serialize, Deserialize, Clone)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone)]
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

// ── Subscription Event ──────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct SubscriptionEvent {
    pub topic: String,
    pub data: serde_json::Value,
}

// ── Shared State ────────────────────────────────────────────────────

pub struct RpcState {
    pub accounts: Arc<RwLock<AccountState>>,
    pub tx_sender: mpsc::Sender<Vec<u8>>,
    pub batch_count: Arc<AtomicU64>,
    pub receipt_store: Option<Arc<StateStore>>,
    pub base_fee: Arc<AtomicU64>,
    pub node_metrics: Option<Arc<RwLock<serde_json::Value>>>,
    pub event_bus: Arc<EventBus>,
    ws_connection_count: Arc<AtomicU64>,
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
            event_bus: Arc::clone(&self.event_bus),
            ws_connection_count: Arc::clone(&self.ws_connection_count),
        }
    }
}

const MAX_WS_CONNECTIONS: u64 = 256;
const MAX_SUBSCRIPTIONS_PER_CLIENT: usize = 16;
const MAX_WS_FRAME_SIZE: usize = 1_048_576;

// ── Event Bus ───────────────────────────────────────────────────────

pub struct EventBus {
    new_heads: broadcast::Sender<serde_json::Value>,
    finality: broadcast::Sender<serde_json::Value>,
}

impl EventBus {
    pub fn new() -> Self {
        let (new_heads, _) = broadcast::channel(256);
        let (finality, _) = broadcast::channel(256);
        Self {
            new_heads,
            finality,
        }
    }

    pub fn publish_new_head(&self, header: serde_json::Value) {
        let _ = self.new_heads.send(header);
    }

    pub fn publish_finality(&self, cert: serde_json::Value) {
        let _ = self.finality.send(cert);
    }

    fn subscribe(&self, topic: &str) -> Option<broadcast::Receiver<serde_json::Value>> {
        match topic {
            "newHeads" => Some(self.new_heads.subscribe()),
            "finality" => Some(self.finality.subscribe()),
            _ => None,
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
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
        batch_count: Arc<AtomicU64>,
        receipt_store: Option<Arc<StateStore>>,
        base_fee: Arc<AtomicU64>,
    ) -> Self {
        Self {
            state: RpcState {
                accounts,
                tx_sender,
                batch_count,
                receipt_store,
                base_fee,
                node_metrics: None,
                event_bus: Arc::new(EventBus::new()),
                ws_connection_count: Arc::new(AtomicU64::new(0)),
            },
        }
    }

    pub fn with_metrics(mut self, metrics: Arc<RwLock<serde_json::Value>>) -> Self {
        self.state.node_metrics = Some(metrics);
        self
    }

    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.state.event_bus = bus;
        self
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.state.event_bus)
    }

    pub fn router(&self) -> Router {
        let mut router = Router::new()
            .route("/", post(handle_rpc))
            .route("/ws", get(handle_ws_upgrade));
        if self.state.node_metrics.is_some() {
            router = router.route("/metrics", get(handle_metrics));
        }
        router
            .layer(DefaultBodyLimit::max(MAX_WS_FRAME_SIZE))
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

// ── HTTP Request Handler ────────────────────────────────────────────

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

    if request.method == "aztb_subscribe" || request.method == "aztb_unsubscribe" {
        return (
            StatusCode::OK,
            Json(JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Subscriptions are only available over WebSocket".into(),
            )),
        );
    }

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

// ── WebSocket Handler ───────────────────────────────────────────────

async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<RpcState>,
) -> impl IntoResponse {
    let current = state.ws_connection_count.load(Ordering::Relaxed);
    if current >= MAX_WS_CONNECTIONS {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Too many WebSocket connections",
        )
            .into_response();
    }
    ws.max_message_size(MAX_WS_FRAME_SIZE)
        .on_upgrade(move |socket| handle_ws_connection(socket, state))
        .into_response()
}

async fn handle_ws_connection(socket: WebSocket, state: RpcState) {
    state.ws_connection_count.fetch_add(1, Ordering::Relaxed);
    let (mut ws_tx, mut ws_rx) = socket.split();

    let (response_tx, mut response_rx) = mpsc::channel::<String>(64);

    let mut sub_handles: HashMap<String, tokio::task::JoinHandle<()>> = HashMap::new();
    let mut sub_counter: u64 = 0;

    let writer = tokio::spawn(async move {
        while let Some(msg) = response_rx.recv().await {
            if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        let text = match msg {
            Message::Text(t) => t.to_string(),
            Message::Close(_) => break,
            _ => continue,
        };

        if text.len() > MAX_WS_FRAME_SIZE {
            let err = JsonRpcResponse::error(
                serde_json::Value::Null,
                PARSE_ERROR,
                "Message too large".into(),
            );
            let _ = response_tx.send(serde_json::to_string(&err).unwrap()).await;
            continue;
        }

        // Try JSON-RPC first
        let request: JsonRpcRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(_) => {
                // Try light sync message format
                if let Ok(light_msg) = serde_json::from_str::<serde_json::Value>(&text)
                    && let Some(msg_type) = light_msg.get("type").and_then(|v| v.as_str())
                {
                    let resp = handle_light_sync_ws(&state, msg_type, &light_msg).await;
                    let _ = response_tx
                        .send(serde_json::to_string(&resp).unwrap())
                        .await;
                    continue;
                }
                let err = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    PARSE_ERROR,
                    "Parse error".into(),
                );
                let _ = response_tx.send(serde_json::to_string(&err).unwrap()).await;
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            let err = JsonRpcResponse::error(
                request.id,
                INVALID_REQUEST,
                "Invalid JSON-RPC version".into(),
            );
            let _ = response_tx.send(serde_json::to_string(&err).unwrap()).await;
            continue;
        }

        match request.method.as_str() {
            "aztb_subscribe" => {
                let topic = request.params.get(0).and_then(|v| v.as_str()).unwrap_or("");

                if sub_handles.len() >= MAX_SUBSCRIPTIONS_PER_CLIENT {
                    let err =
                        JsonRpcResponse::error(request.id, -32000, "Too many subscriptions".into());
                    let _ = response_tx.send(serde_json::to_string(&err).unwrap()).await;
                    continue;
                }

                if let Some(mut rx) = state.event_bus.subscribe(topic) {
                    sub_counter += 1;
                    let sub_id = format!("0x{sub_counter:x}");

                    let sub_id_clone = sub_id.clone();
                    let resp_tx = response_tx.clone();
                    let handle = tokio::spawn(async move {
                        while let Ok(data) = rx.recv().await {
                            let notification = serde_json::json!({
                                "jsonrpc": "2.0",
                                "method": "aztb_subscription",
                                "params": {
                                    "subscription": sub_id_clone,
                                    "result": data,
                                }
                            });
                            if resp_tx
                                .send(serde_json::to_string(&notification).unwrap())
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                    });
                    sub_handles.insert(sub_id.clone(), handle);

                    let resp =
                        JsonRpcResponse::success(request.id, serde_json::Value::String(sub_id));
                    let _ = response_tx
                        .send(serde_json::to_string(&resp).unwrap())
                        .await;
                } else {
                    let err = JsonRpcResponse::error(
                        request.id,
                        INVALID_PARAMS,
                        format!("Unknown subscription topic: {topic}"),
                    );
                    let _ = response_tx.send(serde_json::to_string(&err).unwrap()).await;
                }
            }
            "aztb_unsubscribe" => {
                let sub_id = request
                    .params
                    .get(0)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let removed = if let Some(handle) = sub_handles.remove(&sub_id) {
                    handle.abort();
                    true
                } else {
                    false
                };
                let resp = JsonRpcResponse::success(request.id, serde_json::Value::Bool(removed));
                let _ = response_tx
                    .send(serde_json::to_string(&resp).unwrap())
                    .await;
            }
            _ => {
                let resp = dispatch(&state, &request).await;
                let _ = response_tx
                    .send(serde_json::to_string(&resp).unwrap())
                    .await;
            }
        }
    }

    for (_, h) in sub_handles {
        h.abort();
    }
    drop(response_tx);
    let _ = writer.await;
    state.ws_connection_count.fetch_sub(1, Ordering::Relaxed);
}

// ── Light Sync over WebSocket ───────────────────────────────────────

async fn handle_light_sync_ws(
    state: &RpcState,
    msg_type: &str,
    msg: &serde_json::Value,
) -> serde_json::Value {
    match msg_type {
        "RequestHeaders" => {
            let _from_round = msg.get("from_round").and_then(|v| v.as_u64()).unwrap_or(0);
            let _count = msg.get("count").and_then(|v| v.as_u64()).unwrap_or(100);
            let batch_count = state.batch_count.load(Ordering::Relaxed);
            serde_json::json!({
                "type": "ResponseHeaders",
                "headers": [],
                "finality_cert": null,
                "target_round": batch_count,
            })
        }
        "RequestBalance" => {
            let address_hex = msg.get("address").and_then(|v| v.as_str()).unwrap_or("");
            let addr_hex = address_hex.strip_prefix("0x").unwrap_or(address_hex);
            let balance = if let Ok(bytes) = hex::decode(addr_hex) {
                if let Ok(addr) = <[u8; 32]>::try_from(bytes.as_slice()) {
                    let accounts = state.accounts.read().await;
                    accounts.balance(&addr)
                } else {
                    0
                }
            } else {
                0
            };
            serde_json::json!({
                "type": "BalanceResponse",
                "address": address_hex,
                "balance": format!("0x{balance:x}"),
            })
        }
        "RequestProof" => {
            serde_json::json!({
                "type": "ResponseProof",
                "proof": null,
            })
        }
        _ => {
            serde_json::json!({
                "type": "Error",
                "message": format!("Unknown message type: {msg_type}"),
            })
        }
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
    let count = state.batch_count.load(Ordering::Relaxed);
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
    let base_fee = state.base_fee.load(Ordering::Relaxed);
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
            batch_count: Arc::new(AtomicU64::new(0)),
            receipt_store: None,
            base_fee: Arc::new(AtomicU64::new(1)),
            node_metrics: None,
            event_bus: Arc::new(EventBus::new()),
            ws_connection_count: Arc::new(AtomicU64::new(0)),
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
            batch_count: Arc::new(AtomicU64::new(42)),
            receipt_store: None,
            base_fee: Arc::new(AtomicU64::new(1)),
            node_metrics: None,
            event_bus: Arc::new(EventBus::new()),
            ws_connection_count: Arc::new(AtomicU64::new(0)),
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

    use std::sync::atomic::AtomicU32;

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
            batch_count: Arc::new(AtomicU64::new(0)),
            receipt_store: Some(store),
            base_fee: Arc::new(AtomicU64::new(1)),
            node_metrics: None,
            event_bus: Arc::new(EventBus::new()),
            ws_connection_count: Arc::new(AtomicU64::new(0)),
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

    // ── WebSocket tests ─────────────────────────────────────────────

    async fn start_test_server(state: RpcState) -> SocketAddr {
        let router = Router::new()
            .route("/", post(handle_rpc))
            .route("/ws", get(handle_ws_upgrade))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        addr
    }

    async fn ws_connect(
        addr: SocketAddr,
    ) -> (
        futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            tokio_tungstenite::tungstenite::Message,
        >,
        futures::stream::SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    ) {
        let url = format!("ws://{addr}/ws");
        let (stream, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
        stream.split()
    }

    #[tokio::test]
    async fn ws_upgrade_and_rpc_call() {
        let (state, _rx) = test_state_with_accounts();
        let addr = start_test_server(state).await;
        let (mut tx, mut rx) = ws_connect(addr).await;

        let req = r#"{"jsonrpc":"2.0","method":"aztb_blockNumber","id":1}"#;
        tx.send(tokio_tungstenite::tungstenite::Message::Text(req.into()))
            .await
            .unwrap();

        let msg = rx.next().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(resp["result"], "0x2a");
    }

    #[tokio::test]
    async fn ws_light_sync_request() {
        let (state, _rx) = test_state();
        state.batch_count.store(99, Ordering::Relaxed);
        let addr = start_test_server(state).await;
        let (mut tx, mut rx) = ws_connect(addr).await;

        let req = r#"{"type":"RequestHeaders","version":1,"from_round":1,"count":10}"#;
        tx.send(tokio_tungstenite::tungstenite::Message::Text(req.into()))
            .await
            .unwrap();

        let msg = rx.next().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(resp["type"], "ResponseHeaders");
        assert_eq!(resp["target_round"], 99);
    }

    #[tokio::test]
    async fn ws_subscribe_and_receive_event() {
        let (state, _rx) = test_state();
        let bus = Arc::clone(&state.event_bus);
        let addr = start_test_server(state).await;
        let (mut tx, mut rx) = ws_connect(addr).await;

        let sub_req = r#"{"jsonrpc":"2.0","method":"aztb_subscribe","params":["newHeads"],"id":1}"#;
        tx.send(tokio_tungstenite::tungstenite::Message::Text(
            sub_req.into(),
        ))
        .await
        .unwrap();

        let msg = rx.next().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        let sub_id = resp["result"].as_str().unwrap().to_string();
        assert!(sub_id.starts_with("0x"));

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        bus.publish_new_head(serde_json::json!({"round": 42, "state_root": "0xabc"}));

        let notification = tokio::time::timeout(std::time::Duration::from_secs(2), rx.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let notif: serde_json::Value =
            serde_json::from_str(notification.to_text().unwrap()).unwrap();
        assert_eq!(notif["method"], "aztb_subscription");
        assert_eq!(notif["params"]["subscription"], sub_id);
        assert_eq!(notif["params"]["result"]["round"], 42);
    }

    #[tokio::test]
    async fn ws_unsubscribe_stops_delivery() {
        let (state, _rx) = test_state();
        let bus = Arc::clone(&state.event_bus);
        let addr = start_test_server(state).await;
        let (mut tx, mut rx) = ws_connect(addr).await;

        let sub_req = r#"{"jsonrpc":"2.0","method":"aztb_subscribe","params":["finality"],"id":1}"#;
        tx.send(tokio_tungstenite::tungstenite::Message::Text(
            sub_req.into(),
        ))
        .await
        .unwrap();

        let msg = rx.next().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        let sub_id = resp["result"].as_str().unwrap().to_string();

        let unsub_req = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_unsubscribe","params":["{sub_id}"],"id":2}}"#
        );
        tx.send(tokio_tungstenite::tungstenite::Message::Text(
            unsub_req.into(),
        ))
        .await
        .unwrap();

        let msg = rx.next().await.unwrap().unwrap();
        let resp: serde_json::Value = serde_json::from_str(msg.to_text().unwrap()).unwrap();
        assert_eq!(resp["result"], true);

        bus.publish_finality(serde_json::json!({"round": 100}));

        let timeout_result =
            tokio::time::timeout(std::time::Duration::from_millis(200), rx.next()).await;
        assert!(
            timeout_result.is_err(),
            "Should not receive event after unsubscribe"
        );
    }

    #[tokio::test]
    async fn http_subscribe_returns_error() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_subscribe","params":["newHeads"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["error"]["code"], INVALID_REQUEST);
        assert!(
            resp["error"]["message"]
                .as_str()
                .unwrap()
                .contains("WebSocket")
        );
    }

    #[tokio::test]
    async fn ws_connection_limit_enforced() {
        let (state, _rx) = test_state();
        state
            .ws_connection_count
            .store(MAX_WS_CONNECTIONS, Ordering::Relaxed);
        let addr = start_test_server(state).await;

        let url = format!("ws://{addr}/ws");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(
            result.is_err() || {
                let (_, response) = result.unwrap();
                response.status() == axum::http::StatusCode::SERVICE_UNAVAILABLE
            }
        );
    }
}
