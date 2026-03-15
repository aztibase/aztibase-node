use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, DefaultBodyLimit, State, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{
    Json, Router,
    routing::{get, post},
};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, broadcast, mpsc};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::{debug, info};

use aztibase_consensus::ComputeCommitmentStore;
use aztibase_core::{Keypair, address_from_pubkey};
use aztibase_execution::AccountState;
use aztibase_execution::model_registry::{MODEL_REGISTRY_ADDRESS, ModelMetadata, ModelRegistry};
use aztibase_execution::{
    AgentPolicyStore, BridgeEscrow, BridgeWithdrawProofs, ChainParams, EmissionTracker,
    GovernanceStore, L2AnchorStore, L2Registry, SignedTx, StakingStore, TxKind,
};
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
const INTERNAL_ERROR: i32 = -32603;

// ── Subscription Event ──────────────────────────────────────────────

#[derive(Clone, Debug, Serialize)]
pub struct SubscriptionEvent {
    pub topic: String,
    pub data: serde_json::Value,
}

// ── Shared State ────────────────────────────────────────────────────

const TESTNET_CHAIN_ID: u64 = 0xA27B;
const FAUCET_DRIP_AMOUNT: u128 = 1_000_000;
const FAUCET_COOLDOWN_SECS: u64 = 60;
const NODE_VERSION: &str = env!("CARGO_PKG_VERSION");

fn faucet_keypair() -> Keypair {
    let seed = aztibase_core::hash(b"AZTIBASE_TESTNET_FAUCET");
    Keypair::from_secret_bytes(&seed)
}

pub struct RpcState {
    pub accounts: Arc<RwLock<AccountState>>,
    pub tx_sender: mpsc::Sender<Vec<u8>>,
    pub batch_count: Arc<AtomicU64>,
    pub receipt_store: Option<Arc<StateStore>>,
    pub base_fee: Arc<AtomicU64>,
    pub node_metrics: Option<crate::metrics::NodeMetrics>,
    pub event_bus: Arc<EventBus>,
    ws_connection_count: Arc<AtomicU64>,
    ip_tracker: IpConnectionTracker,
    pub pending_task_count: Arc<AtomicU64>,
    pub compute_commitments: Option<Arc<RwLock<ComputeCommitmentStore>>>,
    pub governance: Option<Arc<RwLock<GovernanceStore>>>,
    pub chain_params: Option<Arc<RwLock<ChainParams>>>,
    pub emission_tracker: Option<Arc<RwLock<EmissionTracker>>>,
    pub staking_store: Option<Arc<RwLock<StakingStore>>>,
    pub agent_policy_store: Option<Arc<RwLock<AgentPolicyStore>>>,
    pub l2_registry: Option<Arc<RwLock<L2Registry>>>,
    pub l2_anchor_store: Option<Arc<RwLock<L2AnchorStore>>>,
    pub bridge_escrow: Option<Arc<RwLock<BridgeEscrow>>>,
    pub bridge_withdraw_proofs: Option<Arc<RwLock<BridgeWithdrawProofs>>>,
    pub chain_id: u64,
    pub genesis_hash: Option<[u8; 32]>,
    faucet_tracker: Arc<std::sync::Mutex<HashMap<[u8; 32], std::time::Instant>>>,
    faucet_nonce: Arc<AtomicU64>,
    pub faucet_enabled: bool,
    pub rate_limiter: Arc<RpcRateLimiter>,
    pub permissive_cors: bool,
    pub cors_allowed_origins: Vec<String>,
    pub max_body_bytes: usize,
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
            ip_tracker: self.ip_tracker.clone(),
            pending_task_count: Arc::clone(&self.pending_task_count),
            compute_commitments: self.compute_commitments.clone(),
            governance: self.governance.clone(),
            chain_params: self.chain_params.clone(),
            emission_tracker: self.emission_tracker.clone(),
            staking_store: self.staking_store.clone(),
            agent_policy_store: self.agent_policy_store.clone(),
            l2_registry: self.l2_registry.clone(),
            l2_anchor_store: self.l2_anchor_store.clone(),
            bridge_escrow: self.bridge_escrow.clone(),
            bridge_withdraw_proofs: self.bridge_withdraw_proofs.clone(),
            chain_id: self.chain_id,
            genesis_hash: self.genesis_hash,
            faucet_tracker: Arc::clone(&self.faucet_tracker),
            faucet_nonce: Arc::clone(&self.faucet_nonce),
            faucet_enabled: self.faucet_enabled,
            rate_limiter: Arc::clone(&self.rate_limiter),
            permissive_cors: self.permissive_cors,
            cors_allowed_origins: self.cors_allowed_origins.clone(),
            max_body_bytes: self.max_body_bytes,
        }
    }
}

const MAX_WS_CONNECTIONS: u64 = 256;
const MAX_WS_PER_IP: u64 = 8;
const MAX_SUBSCRIPTIONS_PER_CLIENT: usize = 16;
const MAX_WS_FRAME_SIZE: usize = 1_048_576;

/// Per-IP WebSocket connection counter.
#[derive(Clone, Default)]
pub struct IpConnectionTracker {
    counts: Arc<std::sync::Mutex<HashMap<IpAddr, u64>>>,
}

impl IpConnectionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_acquire(&self, ip: IpAddr) -> bool {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        let count = counts.entry(ip).or_insert(0);
        if *count >= MAX_WS_PER_IP {
            return false;
        }
        *count += 1;
        true
    }

    pub fn release(&self, ip: IpAddr) {
        let mut counts = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(count) = counts.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                counts.remove(&ip);
            }
        }
    }
}

// ── Per-IP Rate Limiter (token bucket) ──────────────────────────────

const DEFAULT_RATE_LIMIT: u32 = 100;
const BUCKET_REFILL_INTERVAL_MS: u64 = 1000;
const MAX_RATE_LIMIT_BUCKETS: usize = 10_000;
const BUCKET_EVICTION_SECS: u64 = 300;

struct IpBucket {
    tokens: u32,
    last_refill: Instant,
}

pub struct RpcRateLimiter {
    buckets: std::sync::Mutex<HashMap<IpAddr, IpBucket>>,
    limit_per_second: u32,
}

impl RpcRateLimiter {
    pub fn new(limit_per_second: u32) -> Self {
        Self {
            buckets: std::sync::Mutex::new(HashMap::new()),
            limit_per_second: if limit_per_second == 0 {
                DEFAULT_RATE_LIMIT
            } else {
                limit_per_second
            },
        }
    }

    pub fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();

        if buckets.len() > MAX_RATE_LIMIT_BUCKETS {
            let eviction_threshold = std::time::Duration::from_secs(BUCKET_EVICTION_SECS);
            buckets.retain(|_, b| b.last_refill.elapsed() < eviction_threshold);
        }

        let bucket = buckets.entry(ip).or_insert(IpBucket {
            tokens: self.limit_per_second,
            last_refill: now,
        });
        let elapsed_ms = bucket.last_refill.elapsed().as_millis() as u64;
        if elapsed_ms >= BUCKET_REFILL_INTERVAL_MS {
            bucket.tokens = self.limit_per_second;
            bucket.last_refill = now;
        }
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    pub fn limit_per_second(&self) -> u32 {
        self.limit_per_second
    }
}

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
                ip_tracker: IpConnectionTracker::new(),
                pending_task_count: Arc::new(AtomicU64::new(0)),
                compute_commitments: None,
                governance: None,
                chain_params: None,
                emission_tracker: None,
                staking_store: None,
                agent_policy_store: None,
                l2_registry: None,
                l2_anchor_store: None,
                bridge_escrow: None,
                bridge_withdraw_proofs: None,
                chain_id: TESTNET_CHAIN_ID,
                genesis_hash: None,
                faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
                faucet_nonce: Arc::new(AtomicU64::new(0)),
                faucet_enabled: true,
                rate_limiter: Arc::new(RpcRateLimiter::new(DEFAULT_RATE_LIMIT)),
                permissive_cors: false,
                cors_allowed_origins: Vec::new(),
                max_body_bytes: MAX_WS_FRAME_SIZE,
            },
        }
    }

    pub fn with_faucet_enabled(mut self, enabled: bool) -> Self {
        self.state.faucet_enabled = enabled;
        self
    }

    pub fn with_rate_limit(mut self, limit_per_second: u32) -> Self {
        self.state.rate_limiter = Arc::new(RpcRateLimiter::new(limit_per_second));
        self
    }

    pub fn with_cors_config(mut self, permissive: bool, allowed_origins: Vec<String>) -> Self {
        self.state.permissive_cors = permissive;
        self.state.cors_allowed_origins = allowed_origins;
        self
    }

    pub fn with_max_body_bytes(mut self, max_bytes: usize) -> Self {
        self.state.max_body_bytes = max_bytes;
        self
    }

    pub fn with_chain_id(mut self, chain_id: u64) -> Self {
        self.state.chain_id = chain_id;
        self
    }

    pub fn with_genesis_hash(mut self, hash: [u8; 32]) -> Self {
        self.state.genesis_hash = Some(hash);
        self
    }

    pub fn with_metrics(mut self, metrics: crate::metrics::NodeMetrics) -> Self {
        self.state.node_metrics = Some(metrics);
        self
    }

    pub fn with_event_bus(mut self, bus: Arc<EventBus>) -> Self {
        self.state.event_bus = bus;
        self
    }

    pub fn with_pending_task_count(mut self, count: Arc<AtomicU64>) -> Self {
        self.state.pending_task_count = count;
        self
    }

    pub fn with_compute_commitments(mut self, store: Arc<RwLock<ComputeCommitmentStore>>) -> Self {
        self.state.compute_commitments = Some(store);
        self
    }

    pub fn with_governance(mut self, store: Arc<RwLock<GovernanceStore>>) -> Self {
        self.state.governance = Some(store);
        self
    }

    pub fn with_chain_params(mut self, params: Arc<RwLock<ChainParams>>) -> Self {
        self.state.chain_params = Some(params);
        self
    }

    pub fn with_emission_tracker(mut self, tracker: Arc<RwLock<EmissionTracker>>) -> Self {
        self.state.emission_tracker = Some(tracker);
        self
    }

    pub fn with_staking_store(mut self, store: Arc<RwLock<StakingStore>>) -> Self {
        self.state.staking_store = Some(store);
        self
    }

    pub fn with_agent_policy_store(mut self, store: Arc<RwLock<AgentPolicyStore>>) -> Self {
        self.state.agent_policy_store = Some(store);
        self
    }

    pub fn with_l2_registry(mut self, store: Arc<RwLock<L2Registry>>) -> Self {
        self.state.l2_registry = Some(store);
        self
    }

    pub fn with_l2_anchor_store(mut self, store: Arc<RwLock<L2AnchorStore>>) -> Self {
        self.state.l2_anchor_store = Some(store);
        self
    }

    pub fn with_bridge_escrow(mut self, store: Arc<RwLock<BridgeEscrow>>) -> Self {
        self.state.bridge_escrow = Some(store);
        self
    }

    pub fn with_bridge_withdraw_proofs(mut self, store: Arc<RwLock<BridgeWithdrawProofs>>) -> Self {
        self.state.bridge_withdraw_proofs = Some(store);
        self
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.state.event_bus)
    }

    pub fn router(&self) -> Router {
        let mut router = Router::new()
            .route("/", post(handle_rpc))
            .route("/ws", get(handle_ws_upgrade))
            .route("/health", get(handle_health));
        if self.state.node_metrics.is_some() {
            router = router
                .route("/metrics", get(handle_metrics_prometheus))
                .route("/metrics/json", get(handle_metrics_json));
        }

        let cors = if self.state.permissive_cors {
            // testnet only — production should use explicit origins
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
        } else if self.state.cors_allowed_origins.is_empty() {
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(Vec::<axum::http::HeaderValue>::new()))
                .allow_methods(Any)
                .allow_headers(Any)
        } else {
            let origins: Vec<axum::http::HeaderValue> = self
                .state
                .cors_allowed_origins
                .iter()
                .filter_map(|o| o.parse().ok())
                .collect();
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods(Any)
                .allow_headers(Any)
        };

        router
            .layer(cors)
            .layer(DefaultBodyLimit::max(self.state.max_body_bytes))
            .with_state(self.state.clone())
    }

    pub async fn serve(self, addr: SocketAddr) -> anyhow::Result<()> {
        let router = self.router();
        info!(%addr, "RPC server listening");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;
        Ok(())
    }
}

// ── HTTP Request Handler ────────────────────────────────────────────

async fn handle_rpc(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<RpcState>,
    body: String,
) -> impl IntoResponse {
    if !state.rate_limiter.check(addr.ip()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(JsonRpcResponse::error(
                serde_json::Value::Null,
                -32000,
                "rate limited".into(),
            )),
        );
    }

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
        "aztb_getModelInfo" => handle_get_model_info(state, req).await,
        "aztb_listModels" => handle_list_models(state, req).await,
        "aztb_getTaskStatus" => handle_get_task_status(state, req).await,
        "aztb_pendingTaskCount" => handle_pending_task_count(state, req).await,
        "aztb_getComputeCommitment" => handle_get_compute_commitment(state, req).await,
        "aztb_listComputeProviders" => handle_list_compute_providers(state, req).await,
        "aztb_faucetDrip" => handle_faucet_drip(state, req).await,
        "aztb_nodeInfo" => handle_node_info(state, req).await,
        "aztb_chainId" => handle_chain_id(state, req).await,
        "aztb_genesisHash" => handle_genesis_hash(state, req).await,
        "aztb_getBlockByNumber" => handle_get_block_by_number(state, req).await,
        "aztb_getBlockByHash" => handle_get_block_by_hash(state, req).await,
        "aztb_getTransactionByHash" => handle_get_transaction_by_hash(state, req).await,
        "aztb_getBatchRoot" => handle_get_batch_root(state, req).await,
        "aztb_getBlockRange" => handle_get_block_range(state, req).await,
        "aztb_getTransactionsByBatch" => handle_get_transactions_by_batch(state, req).await,
        "aztb_getReceiptsByBatch" => handle_get_receipts_by_batch(state, req).await,
        "aztb_getProposal" => handle_get_proposal(state, req).await,
        "aztb_listProposals" => handle_list_proposals(state, req).await,
        "aztb_getChainParam" => handle_get_chain_param(state, req).await,
        "aztb_listChainParams" => handle_list_chain_params(state, req).await,
        "aztb_getEmissionInfo" => handle_get_emission_info(state, req).await,
        "aztb_getEpochRewards" => handle_get_epoch_rewards(state, req).await,
        "aztb_getVestingStatus" => handle_get_vesting_status(state, req).await,
        "aztb_getValidatorStake" => handle_get_validator_stake(state, req).await,
        "aztb_getDelegation" => handle_get_delegation(state, req).await,
        "aztb_getActiveValidators" => handle_get_active_validators(state, req).await,
        "aztb_getUnbondingStatus" => handle_get_unbonding_status(state, req).await,
        "aztb_getAgentPolicy" => handle_get_agent_policy(state, req).await,
        "aztb_getCheckpoint" => handle_get_checkpoint(state, req).await,
        "aztb_latestCheckpoint" => handle_latest_checkpoint(state, req).await,
        "aztb_getBlockTransactionCount" => handle_get_block_tx_count(state, req).await,
        "aztb_sendRawTransaction" => handle_send_transaction(state, req).await,
        "aztb_getL2State" => handle_get_l2_state(state, req).await,
        "aztb_listL2s" => handle_list_l2s(state, req).await,
        "aztb_getBridgeBalance" => handle_get_bridge_balance(state, req).await,
        "aztb_getBridgeProofStatus" => handle_get_bridge_proof_status(state, req).await,
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
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
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

    let client_ip = addr.ip();
    if !state.ip_tracker.try_acquire(client_ip) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many WebSocket connections from this IP",
        )
            .into_response();
    }

    ws.max_message_size(MAX_WS_FRAME_SIZE)
        .on_upgrade(move |socket| handle_ws_connection(socket, state, client_ip))
        .into_response()
}

async fn handle_ws_connection(socket: WebSocket, state: RpcState, client_ip: IpAddr) {
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
    state.ip_tracker.release(client_ip);
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

async fn handle_get_block_tx_count(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let num = match parse_u64_param(&req.params, 0) {
        Ok(n) => n,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };
    match aztibase_execution::get_batch_by_number(store, num) {
        Ok(Some(anchor_hash)) => {
            let count = aztibase_execution::get_batch_txs(store, &anchor_hash)
                .ok()
                .flatten()
                .map(|txs| txs.len())
                .unwrap_or(0);
            JsonRpcResponse::success(req.id.clone(), serde_json::json!(count))
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
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

// ── Model Registry & Task Endpoints ─────────────────────────────────

fn registry_storage(accounts: &AccountState) -> std::collections::BTreeMap<Vec<u8>, Vec<u8>> {
    accounts.storage(&MODEL_REGISTRY_ADDRESS)
}

async fn handle_get_model_info(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let model_id = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing model_id parameter".into(),
            );
        }
    };
    let accounts = state.accounts.read().await;
    let storage = registry_storage(&accounts);
    match ModelRegistry::get(&storage, model_id) {
        Some(meta) => JsonRpcResponse::success(req.id.clone(), model_to_json(&meta)),
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_list_models(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let accounts = state.accounts.read().await;
    let storage = registry_storage(&accounts);
    let active: Vec<serde_json::Value> = ModelRegistry::list_active(&storage)
        .into_iter()
        .map(|m| model_to_json(&m))
        .collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(active))
}

async fn handle_get_task_status(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hex_str = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing task_id parameter".into(),
            );
        }
    };
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let task_id_bytes = match hex::decode(hex_str) {
        Ok(b) if b.len() == 32 => b,
        _ => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "task_id must be 32-byte hex".into(),
            );
        }
    };

    let accounts = state.accounts.read().await;
    let storage = registry_storage(&accounts);
    let mut key = b"task:".to_vec();
    key.extend_from_slice(&task_id_bytes);

    match storage.get(&key) {
        Some(data) => {
            if let Ok(task) = postcard::from_bytes::<aztibase_consensus::InferenceTask>(data) {
                let mut result = serde_json::json!({
                    "taskId": format!("0x{}", hex::encode(task.task_id)),
                    "modelId": task.model_id,
                    "inputHash": format!("0x{}", hex::encode(task.input_hash)),
                    "requester": format!("0x{}", hex::encode(task.requester)),
                    "reward": format!("0x{:x}", task.reward),
                    "deadlineRound": task.deadline_round,
                    "status": "pending",
                });
                if let Some(v) = task.assigned_validator {
                    result["assignedValidator"] =
                        serde_json::json!(format!("0x{}", hex::encode(v)));
                }
                JsonRpcResponse::success(req.id.clone(), result)
            } else {
                JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null)
            }
        }
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_pending_task_count(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let count = state.pending_task_count.load(Ordering::Relaxed);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(count))
}

async fn handle_get_compute_commitment(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.compute_commitments {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "Compute commitments not available".into(),
            );
        }
    };

    let validator_hex = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(h) => h,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "Expected validator hex string as first parameter".into(),
            );
        }
    };

    let hex_str = validator_hex.strip_prefix("0x").unwrap_or(validator_hex);
    let validator_bytes = match hex::decode(hex_str) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "Invalid validator hex (expected 32-byte hex)".into(),
            );
        }
    };

    let guard = store.read().await;
    match guard.get(&validator_bytes) {
        Some(c) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!({
                "validatorId": format!("0x{}", hex::encode(c.validator_id)),
                "supportedModels": c.supported_models,
                "committedStake": c.committed_stake,
                "registeredRound": c.registered_round,
                "active": c.active,
            }),
        ),
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_list_compute_providers(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.compute_commitments {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "Compute commitments not available".into(),
            );
        }
    };

    let model_id = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(m) => m,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "Expected model_id string as first parameter".into(),
            );
        }
    };

    let guard = store.read().await;
    let validators: Vec<String> = guard
        .validators_for_model(model_id)
        .into_iter()
        .map(|v| format!("0x{}", hex::encode(v)))
        .collect();

    JsonRpcResponse::success(req.id.clone(), serde_json::json!(validators))
}

fn model_to_json(meta: &ModelMetadata) -> serde_json::Value {
    serde_json::json!({
        "modelId": meta.model_id,
        "owner": format!("0x{}", hex::encode(meta.owner)),
        "fingerprint": format!("0x{}", hex::encode(meta.fingerprint)),
        "computeCost": meta.compute_cost,
        "minStake": meta.min_stake,
        "registeredRound": meta.registered_round,
        "active": meta.active,
    })
}

// ── Faucet + Node Info Endpoints ────────────────────────────────────

/// Testnet-only faucet: submits a FaucetDrip consensus transaction.
/// The drip goes through mempool → gossip → consensus → execution,
/// ensuring all nodes credit the same balance (cross-node consistent).
async fn handle_faucet_drip(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    if !state.faucet_enabled {
        return JsonRpcResponse::error(
            req.id.clone(),
            -32000,
            "faucet is disabled on this network".into(),
        );
    }

    let addr_hex = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s.strip_prefix("0x").unwrap_or(s),
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing address parameter".into(),
            );
        }
    };

    let addr_bytes = match hex::decode(addr_hex) {
        Ok(b) if b.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&b);
            arr
        }
        _ => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "invalid address: expected 32-byte hex".into(),
            );
        }
    };

    {
        let mut tracker = state
            .faucet_tracker
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        if tracker.len() > MAX_RATE_LIMIT_BUCKETS {
            let eviction_threshold = std::time::Duration::from_secs(BUCKET_EVICTION_SECS);
            tracker.retain(|_, t| t.elapsed() < eviction_threshold);
        }

        if let Some(last) = tracker.get(&addr_bytes)
            && last.elapsed().as_secs() < FAUCET_COOLDOWN_SECS
        {
            let remaining = FAUCET_COOLDOWN_SECS - last.elapsed().as_secs();
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                format!("rate limited: retry in {remaining}s"),
            );
        }
        tracker.insert(addr_bytes, std::time::Instant::now());
    }

    let kp = faucet_keypair();
    let faucet_addr = address_from_pubkey(kp.public_key().as_bytes());
    let nonce = state.faucet_nonce.fetch_add(1, Ordering::Relaxed);

    let tx = TxKind::FaucetDrip {
        validator: faucet_addr,
        recipient: addr_bytes,
        amount: FAUCET_DRIP_AMOUNT,
        nonce,
        gas_price: 0,
    };
    let payload = tx.encode();
    let signed = SignedTx::new(payload, &kp);
    let envelope = signed.encode();
    let tx_hash = aztibase_core::hash(&envelope);

    match state.tx_sender.try_send(envelope) {
        Ok(()) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!({
                "address": format!("0x{}", hex::encode(addr_bytes)),
                "amount": FAUCET_DRIP_AMOUNT,
                "tx_hash": format!("0x{}", hex::encode(tx_hash)),
            }),
        ),
        Err(mpsc::error::TrySendError::Full(_)) => JsonRpcResponse::error(
            req.id.clone(),
            -32000,
            "mempool full, try again later".into(),
        ),
        Err(mpsc::error::TrySendError::Closed(_)) => {
            JsonRpcResponse::error(req.id.clone(), -32000, "node shutting down".into())
        }
    }
}

async fn handle_node_info(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let block_height = state.batch_count.load(Ordering::Relaxed);

    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "version": NODE_VERSION,
            "chainId": format!("0x{:x}", state.chain_id),
            "blockHeight": block_height,
            "protocolVersion": "aztb/1",
        }),
    )
}

async fn handle_chain_id(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!(format!("0x{:x}", state.chain_id)),
    )
}

async fn handle_genesis_hash(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    match state.genesis_hash {
        Some(hash) => {
            let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
            JsonRpcResponse::success(req.id.clone(), serde_json::json!(format!("0x{hex}")))
        }
        None => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, "No genesis loaded".into()),
    }
}

// ── Historical Query Endpoints ─────────────────────────────────────

const MAX_BLOCK_RANGE: usize = 100;

async fn handle_get_block_by_number(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let num = match parse_u64_param(&req.params, 0) {
        Ok(n) => n,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_batch_by_number(store, num) {
        Ok(Some(anchor_hash)) => {
            let state_root = aztibase_execution::get_batch_root(store, &anchor_hash)
                .ok()
                .flatten();
            let tx_hashes = aztibase_execution::get_batch_txs(store, &anchor_hash)
                .ok()
                .flatten()
                .unwrap_or_default();
            let txs: Vec<String> = tx_hashes
                .iter()
                .map(|h| format!("0x{}", hex::encode(h)))
                .collect();
            let mut result = serde_json::json!({
                "number": format!("0x{num:x}"),
                "hash": format!("0x{}", hex::encode(anchor_hash)),
                "transactions": txs,
            });
            if let Some(sr) = state_root {
                result["stateRoot"] = serde_json::json!(format!("0x{}", hex::encode(sr)));
            }
            JsonRpcResponse::success(req.id.clone(), result)
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_block_by_hash(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_batch_root(store, &hash) {
        Ok(Some(state_root)) => {
            let tx_hashes = aztibase_execution::get_batch_txs(store, &hash)
                .ok()
                .flatten()
                .unwrap_or_default();
            let txs: Vec<String> = tx_hashes
                .iter()
                .map(|h| format!("0x{}", hex::encode(h)))
                .collect();
            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "hash": format!("0x{}", hex::encode(hash)),
                    "stateRoot": format!("0x{}", hex::encode(state_root)),
                    "transactions": txs,
                }),
            )
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_transaction_by_hash(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_transaction(store, &hash) {
        Ok(Some(data)) => {
            let mut result = serde_json::json!({
                "hash": format!("0x{}", hex::encode(hash)),
                "raw": format!("0x{}", hex::encode(&data)),
            });
            if let Ok(tx) = postcard::from_bytes::<TxKind>(&data) {
                let hex_addr = |a: &[u8; 32]| format!("0x{}", hex::encode(a));
                let val128 = |v: u128| -> serde_json::Value {
                    serde_json::Value::Number(serde_json::Number::from(v as u64))
                };
                match &tx {
                    TxKind::Transfer {
                        from,
                        to,
                        value,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "Transfer".into();
                        result["from"] = hex_addr(from).into();
                        result["to"] = hex_addr(to).into();
                        result["value"] = val128(*value);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::Stake {
                        staker,
                        amount,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "Stake".into();
                        result["from"] = hex_addr(staker).into();
                        result["to"] = hex_addr(staker).into();
                        result["value"] = val128(*amount);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::Unstake {
                        staker,
                        amount,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "Unstake".into();
                        result["from"] = hex_addr(staker).into();
                        result["to"] = hex_addr(staker).into();
                        result["value"] = val128(*amount);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::Delegate {
                        delegator,
                        validator_id,
                        amount,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "Delegate".into();
                        result["from"] = hex_addr(delegator).into();
                        result["to"] = hex_addr(validator_id).into();
                        result["value"] = val128(*amount);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::Undelegate {
                        delegator,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "Undelegate".into();
                        result["from"] = hex_addr(delegator).into();
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::FaucetDrip {
                        validator,
                        recipient,
                        amount,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "FaucetDrip".into();
                        result["from"] = hex_addr(validator).into();
                        result["to"] = hex_addr(recipient).into();
                        result["value"] = val128(*amount);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::ContractDeploy {
                        deployer,
                        nonce,
                        gas_limit,
                        gas_price,
                        ..
                    } => {
                        result["type"] = "ContractDeploy".into();
                        result["from"] = hex_addr(deployer).into();
                        result["nonce"] = (*nonce).into();
                        result["gasLimit"] = (*gas_limit).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::ContractCall {
                        caller,
                        contract,
                        func_name,
                        nonce,
                        gas_limit,
                        gas_price,
                        ..
                    } => {
                        result["type"] = "ContractCall".into();
                        result["from"] = hex_addr(caller).into();
                        result["to"] = hex_addr(contract).into();
                        result["func"] = func_name.clone().into();
                        result["nonce"] = (*nonce).into();
                        result["gasLimit"] = (*gas_limit).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::EvmDeploy {
                        deployer,
                        nonce,
                        gas_limit,
                        gas_price,
                        ..
                    } => {
                        result["type"] = "EvmDeploy".into();
                        result["from"] = hex_addr(deployer).into();
                        result["nonce"] = (*nonce).into();
                        result["gasLimit"] = (*gas_limit).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::EvmCall {
                        caller,
                        contract,
                        nonce,
                        gas_limit,
                        value,
                        gas_price,
                        ..
                    } => {
                        result["type"] = "EvmCall".into();
                        result["from"] = hex_addr(caller).into();
                        result["to"] = hex_addr(contract).into();
                        result["value"] = val128(*value);
                        result["nonce"] = (*nonce).into();
                        result["gasLimit"] = (*gas_limit).into();
                        result["gasPrice"] = (*gas_price).into();
                    }
                    TxKind::BridgeDeposit {
                        depositor,
                        l2_chain_id,
                        l2_recipient,
                        amount,
                        nonce,
                        gas_price,
                    } => {
                        result["type"] = "BridgeDeposit".into();
                        result["from"] = hex_addr(depositor).into();
                        result["to"] = hex_addr(l2_recipient).into();
                        result["value"] = val128(*amount);
                        result["nonce"] = (*nonce).into();
                        result["gasPrice"] = (*gas_price).into();
                        result["l2ChainId"] = hex_addr(l2_chain_id).into();
                    }
                    _ => {
                        result["type"] = format!("{:?}", std::mem::discriminant(&tx)).into();
                    }
                }
            }
            if let Ok(Some(r)) = aztibase_execution::get_receipt(store, &hash) {
                result["receipt"] = serde_json::json!({
                    "success": r.success,
                    "gasUsed": format!("0x{:x}", r.gas_used),
                });
                if let Some(err) = &r.error {
                    result["receipt"]["error"] = serde_json::json!(err);
                }
            }
            JsonRpcResponse::success(req.id.clone(), result)
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_batch_root(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_batch_root(store, &hash) {
        Ok(Some(root)) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!(format!("0x{}", hex::encode(root))),
        ),
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_block_range(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let from = match parse_u64_param(&req.params, 0) {
        Ok(n) => n,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let to = match parse_u64_param(&req.params, 1) {
        Ok(n) => n,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    if to < from {
        return JsonRpcResponse::error(
            req.id.clone(),
            INVALID_PARAMS,
            "'to' must be >= 'from'".into(),
        );
    }

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_batch_range(store, from, to, MAX_BLOCK_RANGE) {
        Ok(entries) => {
            let blocks: Vec<serde_json::Value> = entries
                .iter()
                .map(|(num, hash)| {
                    serde_json::json!({
                        "number": format!("0x{num:x}"),
                        "hash": format!("0x{}", hex::encode(hash)),
                    })
                })
                .collect();
            JsonRpcResponse::success(req.id.clone(), serde_json::json!(blocks))
        }
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_transactions_by_batch(
    state: &RpcState,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    match aztibase_execution::get_batch_txs(store, &hash) {
        Ok(Some(tx_hashes)) => {
            let txs: Vec<String> = tx_hashes
                .iter()
                .map(|h| format!("0x{}", hex::encode(h)))
                .collect();
            JsonRpcResponse::success(req.id.clone(), serde_json::json!(txs))
        }
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}")),
    }
}

async fn handle_get_receipts_by_batch(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(req.id.clone(), -32000, "store not available".into());
        }
    };

    let tx_hashes = match aztibase_execution::get_batch_txs(store, &hash) {
        Ok(Some(h)) => h,
        Ok(None) => {
            return JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null);
        }
        Err(e) => {
            return JsonRpcResponse::error(req.id.clone(), -32000, format!("storage error: {e}"));
        }
    };

    let mut receipts = Vec::with_capacity(tx_hashes.len());
    for tx_hash in &tx_hashes {
        match aztibase_execution::get_receipt(store, tx_hash) {
            Ok(Some(r)) => {
                let mut entry = serde_json::json!({
                    "txHash": format!("0x{}", hex::encode(r.tx_hash)),
                    "success": r.success,
                    "gasUsed": format!("0x{:x}", r.gas_used),
                });
                if let Some(err) = &r.error {
                    entry["error"] = serde_json::json!(err);
                }
                receipts.push(entry);
            }
            _ => {
                receipts.push(serde_json::json!({
                    "txHash": format!("0x{}", hex::encode(tx_hash)),
                    "error": "receipt not found",
                }));
            }
        }
    }

    JsonRpcResponse::success(req.id.clone(), serde_json::json!(receipts))
}

async fn handle_get_proposal(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let proposal_id = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let gov = match &state.governance {
        Some(g) => g,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "governance not available".into(),
            );
        }
    };

    let gov_guard = gov.read().await;
    match gov_guard.get(&proposal_id) {
        Some(proposal) => {
            let tally = gov_guard.tally(&proposal_id);
            let mut entry = serde_json::json!({
                "id": format!("0x{}", hex::encode(proposal.id)),
                "proposer": format!("0x{}", hex::encode(proposal.proposer)),
                "description": proposal.description,
                "paramKey": proposal.param_key,
                "paramValue": proposal.param_value,
                "startRound": proposal.start_round,
                "endRound": proposal.end_round,
                "status": format!("{:?}", proposal.status),
            });
            if let Some(t) = tally {
                entry["approveWeight"] = serde_json::json!(format!("0x{:x}", t.approve_weight));
                entry["rejectWeight"] = serde_json::json!(format!("0x{:x}", t.reject_weight));
                entry["voterCount"] = serde_json::json!(t.voter_count);
            }
            JsonRpcResponse::success(req.id.clone(), entry)
        }
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_list_proposals(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let gov = match &state.governance {
        Some(g) => g,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "governance not available".into(),
            );
        }
    };

    let status_filter = req
        .params
        .get(0)
        .and_then(|v| v.as_str())
        .and_then(|s| match s {
            "Active" => Some(aztibase_execution::ProposalStatus::Active),
            "Passed" => Some(aztibase_execution::ProposalStatus::Passed),
            "Rejected" => Some(aztibase_execution::ProposalStatus::Rejected),
            "Executed" => Some(aztibase_execution::ProposalStatus::Executed),
            _ => None,
        });

    let gov_guard = gov.read().await;
    let proposals = gov_guard.list(status_filter.as_ref());

    let result: Vec<serde_json::Value> = proposals
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": format!("0x{}", hex::encode(p.id)),
                "proposer": format!("0x{}", hex::encode(p.proposer)),
                "description": p.description,
                "paramKey": p.param_key,
                "paramValue": p.param_value,
                "startRound": p.start_round,
                "endRound": p.end_round,
                "status": format!("{:?}", p.status),
            })
        })
        .collect();

    JsonRpcResponse::success(req.id.clone(), serde_json::json!(result))
}

async fn handle_get_chain_param(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let params_store = match &state.chain_params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "chain params not available".into(),
            );
        }
    };

    let key = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(k) => k,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing param key".into(),
            );
        }
    };

    let cp = params_store.read().await;
    match cp.get(key) {
        Some(val) => {
            let def = aztibase_execution::param_def(key);
            let entry = serde_json::json!({
                "key": key,
                "value": val.to_string(),
                "type": format!("{:?}", def.map(|d| d.param_type)),
                "description": def.map(|d| d.description).unwrap_or(""),
            });
            JsonRpcResponse::success(req.id.clone(), entry)
        }
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_list_chain_params(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let params_store = match &state.chain_params {
        Some(p) => p,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "chain params not available".into(),
            );
        }
    };

    let cp = params_store.read().await;
    let result: Vec<serde_json::Value> = cp
        .list()
        .iter()
        .map(|(key, val)| {
            let def = aztibase_execution::param_def(key);
            serde_json::json!({
                "key": key,
                "value": val.to_string(),
                "type": format!("{:?}", def.map(|d| d.param_type)),
                "description": def.map(|d| d.description).unwrap_or(""),
            })
        })
        .collect();

    JsonRpcResponse::success(req.id.clone(), serde_json::json!(result))
}

async fn handle_get_emission_info(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let tracker = match &state.emission_tracker {
        Some(t) => t,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "emission tracker not available".into(),
            );
        }
    };

    let et = tracker.read().await;
    let current_round = state.batch_count.load(Ordering::Relaxed);
    let rounds_in_epoch = if et.epoch_length > 0 {
        current_round % et.epoch_length
    } else {
        0
    };
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "current_epoch": et.current_epoch,
            "total_emitted": et.total_emitted.to_string(),
            "total_supply_in_existence": et.total_supply_in_existence().to_string(),
            "remaining_emission": et.remaining_emission().to_string(),
            "hard_cap": aztibase_execution::tokenomics::TOTAL_SUPPLY.to_string(),
            "genesis_mint": aztibase_execution::tokenomics::GENESIS_MINT.to_string(),
            "epoch_length": et.epoch_length,
            "rounds_in_current_epoch": rounds_in_epoch,
            "rounds_until_next_epoch": et.epoch_length.saturating_sub(rounds_in_epoch),
            "treasury_balance": et.treasury_balance.to_string(),
            "insurance_balance": et.insurance_balance.to_string(),
        }),
    )
}

async fn handle_get_epoch_rewards(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let tracker = match &state.emission_tracker {
        Some(t) => t,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "emission tracker not available".into(),
            );
        }
    };
    let count = req.params.get(0).and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let et = tracker.read().await;
    let events: Vec<serde_json::Value> = et
        .recent_rewards(count.min(50))
        .iter()
        .map(|e| {
            serde_json::json!({
                "epoch": e.epoch,
                "round": e.round,
                "total_emission": e.total_emission.to_string(),
                "validator_pool": e.validator_pool.to_string(),
                "credits": e.credits.iter().map(|(addr, amt)| {
                    serde_json::json!({
                        "validator": format!("0x{}", hex::encode(addr)),
                        "amount": amt.to_string(),
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(events))
}

async fn handle_get_vesting_status(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let category_str = match req.params.get(0).and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                "missing allocation category parameter".into(),
            );
        }
    };

    let current_round = state.batch_count.load(Ordering::Relaxed);
    let allocs = aztibase_execution::genesis_allocations(0);

    let entry = allocs
        .iter()
        .find(|a| a.category.to_string() == category_str);
    match entry {
        Some(alloc) => {
            let vested = alloc.vesting.vested_at(current_round);
            let locked = alloc.vesting.locked_at(current_round);
            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "category": category_str,
                    "total": alloc.amount.to_string(),
                    "vested": vested.to_string(),
                    "locked": locked.to_string(),
                    "cliff_end_round": alloc.vesting.cliff_end_round(),
                    "end_round": alloc.vesting.end_round(),
                    "description": alloc.description,
                }),
            )
        }
        None => {
            let valid: Vec<String> = aztibase_execution::AllocationCategory::ALL
                .iter()
                .map(|c| c.to_string())
                .collect();
            JsonRpcResponse::error(
                req.id.clone(),
                INVALID_PARAMS,
                format!(
                    "unknown category '{category_str}', valid: {}",
                    valid.join(", ")
                ),
            )
        }
    }
}

async fn handle_get_validator_stake(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.staking_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "staking store not available".into(),
            );
        }
    };

    let validator_id = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let ss = store.read().await;
    match ss.get_validator(&validator_id) {
        Some(v) => {
            let slash_records: Vec<_> = ss
                .validator_slash_history(&validator_id)
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "offense": format!("{:?}", r.offense_type),
                        "slash_bps": r.slash_bps,
                        "round": r.round,
                        "amount_slashed": r.amount_slashed,
                    })
                })
                .collect();
            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "validator_id": hex::encode(v.validator_id),
                    "self_stake": v.self_stake,
                    "total_delegated": v.total_delegated,
                    "effective_stake": v.effective_stake(),
                    "active": v.active,
                    "registered_round": v.registered_round,
                    "slash_history": slash_records,
                }),
            )
        }
        None => JsonRpcResponse::error(req.id.clone(), -32000, "validator not found".into()),
    }
}

async fn handle_get_delegation(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.staking_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "staking store not available".into(),
            );
        }
    };

    let address = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let ss = store.read().await;
    match ss.get_delegation(&address) {
        Some(d) => JsonRpcResponse::success(
            req.id.clone(),
            serde_json::json!({
                "validator_id": hex::encode(d.validator_id),
                "amount": d.amount,
                "round_delegated": d.round_delegated,
            }),
        ),
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_get_active_validators(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.staking_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "staking store not available".into(),
            );
        }
    };

    let ss = store.read().await;
    let validators: Vec<_> = ss
        .active_validators()
        .iter()
        .map(|v| {
            serde_json::json!({
                "validator_id": hex::encode(v.validator_id),
                "self_stake": v.self_stake,
                "total_delegated": v.total_delegated,
                "effective_stake": v.effective_stake(),
                "registered_round": v.registered_round,
            })
        })
        .collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(validators))
}

async fn handle_get_unbonding_status(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.staking_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "staking store not available".into(),
            );
        }
    };

    let address = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let ss = store.read().await;
    let entries: Vec<_> = ss
        .pending_unbonding(&address)
        .iter()
        .map(|e| {
            serde_json::json!({
                "amount": e.amount,
                "available_round": e.available_round,
            })
        })
        .collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(entries))
}

async fn handle_get_agent_policy(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.agent_policy_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "agent policy store not available".into(),
            );
        }
    };

    let agent_addr = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let aps = store.read().await;
    match aps.get_policy(&agent_addr) {
        Some(policy) => {
            let kinds: Vec<String> = policy
                .allowed_tx_kinds
                .iter()
                .map(|k| format!("0x{k:02x}"))
                .collect();
            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "owner": format!("0x{}", hex::encode(policy.owner)),
                    "per_tx_limit": policy.per_tx_limit.to_string(),
                    "per_epoch_limit": policy.per_epoch_limit.to_string(),
                    "allowed_tx_kinds": kinds,
                    "expiry_epoch": policy.expiry_epoch,
                }),
            )
        }
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_get_checkpoint(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "storage not available".into(),
            );
        }
    };

    let batch_index = match parse_u64_param(&req.params, 0) {
        Ok(n) => n,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    match aztibase_execution::get_checkpoint_raw(store, batch_index) {
        Ok(Some(bytes)) => match postcard::from_bytes::<aztibase_consensus::Checkpoint>(&bytes) {
            Ok(cp) => checkpoint_to_json(req, &cp),
            Err(_) => JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "corrupt checkpoint data".into(),
            ),
        },
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, format!("storage: {e}")),
    }
}

async fn handle_latest_checkpoint(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let store = match &state.receipt_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "storage not available".into(),
            );
        }
    };

    match aztibase_execution::latest_checkpoint_raw(store) {
        Ok(Some(bytes)) => match postcard::from_bytes::<aztibase_consensus::Checkpoint>(&bytes) {
            Ok(cp) => checkpoint_to_json(req, &cp),
            Err(_) => JsonRpcResponse::error(
                req.id.clone(),
                INTERNAL_ERROR,
                "corrupt checkpoint data".into(),
            ),
        },
        Ok(None) => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
        Err(e) => JsonRpcResponse::error(req.id.clone(), INTERNAL_ERROR, format!("storage: {e}")),
    }
}

fn checkpoint_to_json(
    req: &JsonRpcRequest,
    cp: &aztibase_consensus::Checkpoint,
) -> JsonRpcResponse {
    let root_hex: String = cp.state_root.iter().map(|b| format!("{b:02x}")).collect();
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({
            "batch_index": cp.batch_index,
            "state_root": format!("0x{root_hex}"),
            "has_finality_cert": cp.finality_cert.is_some(),
            "timestamp": cp.timestamp,
        }),
    )
}

// ── L2 Bridge RPC Handlers ──────────────────────────────────────────

async fn handle_get_l2_state(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let registry = match &state.l2_registry {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "l2 registry not available".into(),
            );
        }
    };
    let anchor_store = match &state.l2_anchor_store {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "l2 anchor store not available".into(),
            );
        }
    };

    let l2_chain_id = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let reg = registry.read().await;
    if reg.get(&l2_chain_id).is_none() {
        return JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null);
    }
    drop(reg);

    let store = anchor_store.read().await;
    match store.latest(&l2_chain_id) {
        Some(anchor) => {
            let current_batch = state.batch_count.load(Ordering::Relaxed);
            let finalized = current_batch.saturating_sub(anchor.l1_batch_index)
                >= aztibase_execution::BRIDGE_FINALITY_BATCHES;
            JsonRpcResponse::success(
                req.id.clone(),
                serde_json::json!({
                    "state_root": format!("0x{}", hex::encode(anchor.state_root)),
                    "block_range": [anchor.l2_block_start, anchor.l2_block_end],
                    "sequencer": format!("0x{}", hex::encode(anchor.sequencer)),
                    "batch_index": anchor.l1_batch_index,
                    "finalized": finalized,
                }),
            )
        }
        None => JsonRpcResponse::success(req.id.clone(), serde_json::Value::Null),
    }
}

async fn handle_list_l2s(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let registry = match &state.l2_registry {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "l2 registry not available".into(),
            );
        }
    };

    let reg = registry.read().await;
    let chains: Vec<serde_json::Value> = reg
        .list()
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "l2_chain_id": format!("0x{}", hex::encode(r.l2_chain_id)),
                "name": r.name,
                "sequencer_set": r.sequencer_set.iter()
                    .map(|s| format!("0x{}", hex::encode(s)))
                    .collect::<Vec<_>>(),
                "bridge_address": format!("0x{}", hex::encode(r.bridge_address)),
            })
        })
        .collect();
    JsonRpcResponse::success(req.id.clone(), serde_json::json!(chains))
}

async fn handle_get_bridge_balance(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let escrow = match &state.bridge_escrow {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "bridge escrow not available".into(),
            );
        }
    };

    let l2_chain_id = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };
    let account = match parse_hash_param(&req.params, 1) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = escrow.read().await;
    let locked = store.balance(&l2_chain_id, &account);
    JsonRpcResponse::success(
        req.id.clone(),
        serde_json::json!({ "locked": locked.to_string() }),
    )
}

async fn handle_get_bridge_proof_status(state: &RpcState, req: &JsonRpcRequest) -> JsonRpcResponse {
    let proofs = match &state.bridge_withdraw_proofs {
        Some(s) => s,
        None => {
            return JsonRpcResponse::error(
                req.id.clone(),
                -32000,
                "bridge withdraw proofs not available".into(),
            );
        }
    };

    let proof_hash = match parse_hash_param(&req.params, 0) {
        Ok(h) => h,
        Err(e) => return JsonRpcResponse::error(req.id.clone(), INVALID_PARAMS, e),
    };

    let store = proofs.read().await;
    let used = store.is_used(&proof_hash);
    JsonRpcResponse::success(req.id.clone(), serde_json::json!({ "used": used }))
}

fn parse_u64_param(params: &serde_json::Value, index: usize) -> Result<u64, String> {
    let val = params
        .get(index)
        .ok_or_else(|| format!("missing parameter at index {index}"))?;
    if let Some(n) = val.as_u64() {
        return Ok(n);
    }
    if let Some(s) = val.as_str() {
        let s = s.strip_prefix("0x").unwrap_or(s);
        return u64::from_str_radix(s, 16).map_err(|e| format!("invalid number: {e}"));
    }
    Err(format!(
        "parameter at index {index} must be a number or hex string"
    ))
}

fn parse_hash_param(params: &serde_json::Value, index: usize) -> Result<[u8; 32], String> {
    let hex_str = params
        .get(index)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing hash parameter at index {index}"))?;
    let hex_str = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {e}"))?;
    bytes
        .try_into()
        .map_err(|_| "hash must be 32 bytes".to_string())
}

// ── Health Endpoint ────────────────────────────────────────────────

async fn handle_health(State(state): State<RpcState>) -> impl IntoResponse {
    let block_height = state.batch_count.load(Ordering::Relaxed);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "blockHeight": block_height,
            "chainId": format!("0x{:x}", state.chain_id),
        })),
    )
}

// ── Metrics Endpoints ───────────────────────────────────────────────

async fn handle_metrics_prometheus(State(state): State<RpcState>) -> impl IntoResponse {
    match &state.node_metrics {
        Some(metrics) => (
            StatusCode::OK,
            [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
            metrics.encode_prometheus(),
        )
            .into_response(),
        None => (StatusCode::SERVICE_UNAVAILABLE, "metrics not enabled").into_response(),
    }
}

async fn handle_metrics_json(State(state): State<RpcState>) -> impl IntoResponse {
    match &state.node_metrics {
        Some(metrics) => (StatusCode::OK, Json(metrics.encode_json())),
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
            ip_tracker: IpConnectionTracker::new(),
            pending_task_count: Arc::new(AtomicU64::new(0)),
            compute_commitments: None,
            governance: None,
            chain_params: None,
            emission_tracker: None,
            staking_store: None,
            agent_policy_store: None,
            l2_registry: None,
            l2_anchor_store: None,
            bridge_escrow: None,
            bridge_withdraw_proofs: None,
            chain_id: TESTNET_CHAIN_ID,
            genesis_hash: None,
            faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
            faucet_nonce: Arc::new(AtomicU64::new(0)),
            faucet_enabled: true,
            rate_limiter: Arc::new(RpcRateLimiter::new(DEFAULT_RATE_LIMIT)),
            permissive_cors: true,
            cors_allowed_origins: Vec::new(),
            max_body_bytes: MAX_WS_FRAME_SIZE,
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
            ip_tracker: IpConnectionTracker::new(),
            pending_task_count: Arc::new(AtomicU64::new(0)),
            compute_commitments: None,
            governance: None,
            chain_params: None,
            emission_tracker: None,
            staking_store: None,
            agent_policy_store: None,
            l2_registry: None,
            l2_anchor_store: None,
            bridge_escrow: None,
            bridge_withdraw_proofs: None,
            chain_id: TESTNET_CHAIN_ID,
            genesis_hash: None,
            faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
            faucet_nonce: Arc::new(AtomicU64::new(0)),
            faucet_enabled: true,
            rate_limiter: Arc::new(RpcRateLimiter::new(DEFAULT_RATE_LIMIT)),
            permissive_cors: true,
            cors_allowed_origins: Vec::new(),
            max_body_bytes: MAX_WS_FRAME_SIZE,
        };
        (state, rx)
    }

    async fn rpc_call(state: &RpcState, body: &str) -> serde_json::Value {
        let router = Router::new()
            .route("/", post(handle_rpc))
            .with_state(state.clone());

        let mut request = Request::builder()
            .method("POST")
            .uri("/")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        request
            .extensions_mut()
            .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));

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
            gas_price: 1,
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
            ip_tracker: IpConnectionTracker::new(),
            pending_task_count: Arc::new(AtomicU64::new(0)),
            compute_commitments: None,
            governance: None,
            chain_params: None,
            emission_tracker: None,
            staking_store: None,
            agent_policy_store: None,
            l2_registry: None,
            l2_anchor_store: None,
            bridge_escrow: None,
            bridge_withdraw_proofs: None,
            chain_id: TESTNET_CHAIN_ID,
            genesis_hash: None,
            faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
            faucet_nonce: Arc::new(AtomicU64::new(0)),
            faucet_enabled: true,
            rate_limiter: Arc::new(RpcRateLimiter::new(DEFAULT_RATE_LIMIT)),
            permissive_cors: true,
            cors_allowed_origins: Vec::new(),
            max_body_bytes: MAX_WS_FRAME_SIZE,
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

    async fn metrics_prometheus_call(state: &RpcState) -> (StatusCode, String) {
        let router = Router::new()
            .route("/metrics", get(handle_metrics_prometheus))
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
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    async fn metrics_json_call(state: &RpcState) -> (StatusCode, serde_json::Value) {
        let router = Router::new()
            .route("/metrics/json", get(handle_metrics_json))
            .with_state(state.clone());

        let request = Request::builder()
            .method("GET")
            .uri("/metrics/json")
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
    async fn metrics_prometheus_endpoint_returns_text() {
        let (mut state, _rx) = test_state();
        let metrics = crate::metrics::NodeMetrics::new();
        metrics.update_consensus(10, 20, 3, 2, 0, 500);
        metrics.update_execution(5, 1);
        state.node_metrics = Some(metrics);

        let (status, text) = metrics_prometheus_call(&state).await;
        assert_eq!(status, StatusCode::OK);
        assert!(text.contains("aztibase_consensus_vertices_proposed"));
        assert!(text.contains("aztibase_consensus_commits"));
        assert!(text.contains("aztibase_execution_block_height"));
    }

    #[tokio::test]
    async fn metrics_json_endpoint_returns_data() {
        let (mut state, _rx) = test_state();
        let metrics = crate::metrics::NodeMetrics::new();
        metrics.update_consensus(10, 20, 3, 2, 0, 500);
        metrics.update_execution(5, 1);
        state.node_metrics = Some(metrics);

        let (status, json) = metrics_json_call(&state).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["consensus"]["vertices_proposed"], 10);
        assert_eq!(json["execution"]["block_height"], 5);
    }

    #[tokio::test]
    async fn metrics_endpoint_disabled_returns_503() {
        let (state, _rx) = test_state();
        let (status, text) = metrics_prometheus_call(&state).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(text.contains("not enabled"));
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
            axum::serve(
                listener,
                router.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
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

    // ── Model Registry & Task RPC tests ─────────────────────────────

    fn test_state_with_model() -> (RpcState, mpsc::Receiver<Vec<u8>>) {
        let mut accounts = AccountState::new();
        let owner = [1u8; 32];
        let fp = aztibase_core::hash(b"model-weights");
        let mut storage = std::collections::BTreeMap::new();
        ModelRegistry::register(
            &mut storage,
            "sentiment_v1".into(),
            owner,
            fp,
            1000,
            500,
            10,
        )
        .unwrap();
        for (k, v) in &storage {
            accounts.set_storage(&MODEL_REGISTRY_ADDRESS, k.clone(), v.clone());
        }

        let (tx, rx) = mpsc::channel(64);
        let state = RpcState {
            accounts: Arc::new(RwLock::new(accounts)),
            tx_sender: tx,
            batch_count: Arc::new(AtomicU64::new(0)),
            receipt_store: None,
            base_fee: Arc::new(AtomicU64::new(1)),
            node_metrics: None,
            event_bus: Arc::new(EventBus::new()),
            ws_connection_count: Arc::new(AtomicU64::new(0)),
            ip_tracker: IpConnectionTracker::new(),
            pending_task_count: Arc::new(AtomicU64::new(0)),
            compute_commitments: None,
            governance: None,
            chain_params: None,
            emission_tracker: None,
            staking_store: None,
            agent_policy_store: None,
            l2_registry: None,
            l2_anchor_store: None,
            bridge_escrow: None,
            bridge_withdraw_proofs: None,
            chain_id: TESTNET_CHAIN_ID,
            genesis_hash: None,
            faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
            faucet_nonce: Arc::new(AtomicU64::new(0)),
            faucet_enabled: true,
            rate_limiter: Arc::new(RpcRateLimiter::new(DEFAULT_RATE_LIMIT)),
            permissive_cors: true,
            cors_allowed_origins: Vec::new(),
            max_body_bytes: MAX_WS_FRAME_SIZE,
        };
        (state, rx)
    }

    #[tokio::test]
    async fn get_model_info_found() {
        let (state, _rx) = test_state_with_model();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_getModelInfo","params":["sentiment_v1"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(resp["result"]["modelId"], "sentiment_v1");
        assert_eq!(resp["result"]["computeCost"], 1000);
        assert_eq!(resp["result"]["active"], true);
    }

    #[tokio::test]
    async fn get_model_info_not_found() {
        let (state, _rx) = test_state();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_getModelInfo","params":["nonexistent"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(resp["result"].is_null());
    }

    #[tokio::test]
    async fn list_models_returns_active() {
        let (state, _rx) = test_state_with_model();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_listModels","params":[],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        let models = resp["result"].as_array().unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0]["modelId"], "sentiment_v1");
    }

    #[tokio::test]
    async fn list_models_empty() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_listModels","params":[],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        let models = resp["result"].as_array().unwrap();
        assert!(models.is_empty());
    }

    #[tokio::test]
    async fn get_task_status_not_found() {
        let (state, _rx) = test_state();
        let fake_id = hex::encode([0xABu8; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getTaskStatus","params":["0x{fake_id}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["result"].is_null());
    }

    #[tokio::test]
    async fn get_task_status_found() {
        let mut accounts = AccountState::new();
        let task = aztibase_consensus::InferenceTask::new(
            "model_a".into(),
            aztibase_core::hash(b"input"),
            [1u8; 32],
            500,
            100,
        );
        let task_id = task.task_id;
        let mut key = b"task:".to_vec();
        key.extend_from_slice(&task_id);
        let data = postcard::to_allocvec(&task).unwrap();
        accounts.set_storage(&MODEL_REGISTRY_ADDRESS, key, data);

        let (tx, _rx) = mpsc::channel(64);
        let state = RpcState {
            accounts: Arc::new(RwLock::new(accounts)),
            tx_sender: tx,
            batch_count: Arc::new(AtomicU64::new(0)),
            receipt_store: None,
            base_fee: Arc::new(AtomicU64::new(1)),
            node_metrics: None,
            event_bus: Arc::new(EventBus::new()),
            ws_connection_count: Arc::new(AtomicU64::new(0)),
            ip_tracker: IpConnectionTracker::new(),
            pending_task_count: Arc::new(AtomicU64::new(0)),
            compute_commitments: None,
            governance: None,
            chain_params: None,
            emission_tracker: None,
            staking_store: None,
            agent_policy_store: None,
            l2_registry: None,
            l2_anchor_store: None,
            bridge_escrow: None,
            bridge_withdraw_proofs: None,
            chain_id: TESTNET_CHAIN_ID,
            genesis_hash: None,
            faucet_tracker: Arc::new(std::sync::Mutex::new(HashMap::new())),
            faucet_nonce: Arc::new(AtomicU64::new(0)),
            faucet_enabled: true,
            rate_limiter: Arc::new(RpcRateLimiter::new(100)),
            permissive_cors: true,
            cors_allowed_origins: vec![],
            max_body_bytes: 2 * 1024 * 1024,
        };

        let task_hex = hex::encode(task_id);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getTaskStatus","params":["0x{task_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(resp["result"]["modelId"], "model_a");
        assert_eq!(resp["result"]["status"], "pending");
        assert_eq!(resp["result"]["deadlineRound"], 100);
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

    #[tokio::test]
    async fn pending_task_count_returns_count() {
        let (state, _rx) = test_state();
        state.pending_task_count.store(42, Ordering::Relaxed);
        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_pendingTaskCount","params":[],"id":1}"#,
        )
        .await;
        assert_eq!(resp["result"], 42);
    }

    #[tokio::test]
    async fn get_compute_commitment_found() {
        let (state, _rx) = test_state();
        let validator = [0xAA; 32];
        let store = Arc::new(RwLock::new(ComputeCommitmentStore::new()));
        {
            let mut guard = store.write().await;
            guard.register(aztibase_consensus::ComputeCommitment::new(
                validator,
                vec!["llama-7b".into()],
                5000,
                vec![],
                1,
            ));
        }
        let mut state = state;
        state.compute_commitments = Some(store);

        let hex_id = hex::encode(validator);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getComputeCommitment","params":["0x{hex_id}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(resp["result"]["committedStake"], 5000);
        assert_eq!(resp["result"]["active"], true);
        assert_eq!(resp["result"]["supportedModels"][0], "llama-7b");
    }

    #[tokio::test]
    async fn get_compute_commitment_not_found() {
        let (state, _rx) = test_state();
        let store = Arc::new(RwLock::new(ComputeCommitmentStore::new()));
        let mut state = state;
        state.compute_commitments = Some(store);

        let hex_id = hex::encode([0xBB; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getComputeCommitment","params":["0x{hex_id}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["result"].is_null());
    }

    #[test]
    fn ip_tracker_limits_connections() {
        let tracker = IpConnectionTracker::new();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        for _ in 0..MAX_WS_PER_IP {
            assert!(tracker.try_acquire(ip));
        }
        assert!(!tracker.try_acquire(ip), "should reject beyond limit");

        tracker.release(ip);
        assert!(tracker.try_acquire(ip), "should allow after release");

        let other_ip: IpAddr = "192.168.1.1".parse().unwrap();
        assert!(
            tracker.try_acquire(other_ip),
            "different IP should be independent"
        );
    }

    #[tokio::test]
    async fn list_compute_providers_returns_validators() {
        let (state, _rx) = test_state();
        let store = Arc::new(RwLock::new(ComputeCommitmentStore::new()));
        {
            let mut guard = store.write().await;
            guard.register(aztibase_consensus::ComputeCommitment::new(
                [0x01; 32],
                vec!["llama-7b".into()],
                1000,
                vec![],
                1,
            ));
            guard.register(aztibase_consensus::ComputeCommitment::new(
                [0x02; 32],
                vec!["gpt-neo".into()],
                2000,
                vec![],
                1,
            ));
        }
        let mut state = state;
        state.compute_commitments = Some(store);

        let body = r#"{"jsonrpc":"2.0","method":"aztb_listComputeProviders","params":["llama-7b"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        let providers = resp["result"].as_array().unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(
            providers[0].as_str().unwrap(),
            format!("0x{}", hex::encode([0x01; 32]))
        );
    }

    // ── Faucet tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn faucet_drip_credits_account() {
        let (state, mut rx) = test_state();
        let addr = [0xAA; 32];
        let addr_hex = hex::encode(addr);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x{addr_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(
            resp["result"]["amount"].as_u64().unwrap(),
            FAUCET_DRIP_AMOUNT as u64
        );
        assert!(
            resp["result"]["tx_hash"]
                .as_str()
                .unwrap()
                .starts_with("0x")
        );

        let raw = rx.try_recv().expect("faucet tx should be in channel");
        let signed = aztibase_execution::SignedTx::decode(&raw).expect("valid envelope");
        assert!(signed.verify(), "faucet signature must be valid");
        let tx = aztibase_execution::routing::route_tx(&signed.payload).expect("valid routing");
        assert!(matches!(tx, aztibase_execution::TxKind::FaucetDrip { .. }));
    }

    #[tokio::test]
    async fn faucet_drip_rate_limited() {
        let (state, _rx) = test_state();
        let addr_hex = hex::encode([0xBB; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x{addr_hex}"],"id":1}}"#
        );

        let resp1 = rpc_call(&state, &body).await;
        assert!(resp1.get("error").is_none());

        let resp2 = rpc_call(&state, &body).await;
        assert!(
            resp2["error"]["message"]
                .as_str()
                .unwrap()
                .contains("rate limited")
        );
    }

    #[tokio::test]
    async fn faucet_drip_invalid_address() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0xDEAD"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert_eq!(resp["error"]["code"], INVALID_PARAMS);
    }

    // ── Node Info tests ──────────────────────────────────────────────

    #[tokio::test]
    async fn node_info_returns_metadata() {
        let (state, _rx) = test_state();
        let body = r#"{"jsonrpc":"2.0","method":"aztb_nodeInfo","id":1}"#;
        let resp = rpc_call(&state, &body).await;
        assert!(resp.get("error").is_none(), "unexpected error: {resp}");
        assert_eq!(
            resp["result"]["chainId"],
            format!("0x{:x}", TESTNET_CHAIN_ID)
        );
        assert_eq!(resp["result"]["protocolVersion"], "aztb/1");
        assert!(resp["result"]["version"].as_str().is_some());
    }

    // ── Health endpoint tests ────────────────────────────────────────

    #[tokio::test]
    async fn health_endpoint_returns_ok() {
        let (state, _rx) = test_state();
        let router = Router::new()
            .route("/health", get(handle_health))
            .with_state(state.clone());

        let request = Request::builder()
            .method("GET")
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(json["blockHeight"].as_u64().is_some());
    }

    #[tokio::test]
    async fn get_chain_param_returns_value() {
        let (mut state, _rx) = test_state();
        state.chain_params = Some(Arc::new(RwLock::new(ChainParams::defaults())));

        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_getChainParam","params":["base_fee_floor"],"id":1}"#,
        )
        .await;
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["key"], "base_fee_floor");
        assert_eq!(resp["result"]["value"], "1");
    }

    #[tokio::test]
    async fn list_chain_params_returns_all() {
        let (mut state, _rx) = test_state();
        state.chain_params = Some(Arc::new(RwLock::new(ChainParams::defaults())));

        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_listChainParams","params":[],"id":1}"#,
        )
        .await;
        assert!(resp["error"].is_null());
        let result = resp["result"].as_array().unwrap();
        assert!(result.len() >= 9);
        let keys: Vec<&str> = result.iter().filter_map(|e| e["key"].as_str()).collect();
        assert!(keys.contains(&"base_fee_floor"));
        assert!(keys.contains(&"max_stored_txs"));
    }

    #[tokio::test]
    async fn get_emission_info_returns_data() {
        let (mut state, _rx) = test_state();
        state.emission_tracker = Some(Arc::new(RwLock::new(
            aztibase_execution::EmissionTracker::new(1000),
        )));

        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_getEmissionInfo","params":[],"id":1}"#,
        )
        .await;
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["hard_cap"], "1000000000");
        assert_eq!(resp["result"]["genesis_mint"], "400000000");
        assert_eq!(resp["result"]["total_emitted"], "0");
        assert_eq!(resp["result"]["current_epoch"], 0);
    }

    #[tokio::test]
    async fn get_vesting_status_returns_allocation() {
        let (state, _rx) = test_state();

        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_getVestingStatus","params":["core_team"],"id":1}"#,
        )
        .await;
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["category"], "core_team");
        assert_eq!(resp["result"]["total"], "60000000");
        assert_eq!(resp["result"]["vested"], "0");
        assert_eq!(resp["result"]["locked"], "60000000");
    }

    #[tokio::test]
    async fn get_vesting_status_invalid_category() {
        let (state, _rx) = test_state();

        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_getVestingStatus","params":["nonexistent"],"id":1}"#,
        )
        .await;
        assert!(!resp["error"].is_null());
        let msg = resp["error"]["message"].as_str().unwrap();
        assert!(msg.contains("unknown category"));
    }

    fn test_state_with_staking() -> (RpcState, mpsc::Receiver<Vec<u8>>) {
        let (mut state, rx) = test_state();
        let mut store = StakingStore::new();
        let vid = [0xAAu8; 32];
        store
            .register_validator(vid, 100_000, 1, u128::MAX, 1)
            .expect("register");
        store
            .delegate([0xBBu8; 32], vid, 50_000, u128::MAX, 2)
            .expect("delegate");
        store
            .begin_unstake(vid, 10_000, 1, 100, 4_536_000)
            .expect("unstake");
        state.staking_store = Some(Arc::new(RwLock::new(store)));
        (state, rx)
    }

    #[tokio::test]
    async fn get_validator_stake_rpc() {
        let (state, _rx) = test_state_with_staking();
        let vid_hex = "aa".repeat(32);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getValidatorStake","params":["{}"],"id":1}}"#,
            vid_hex
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null(), "unexpected error: {:?}", resp);
        assert_eq!(resp["result"]["self_stake"], 90_000);
        assert_eq!(resp["result"]["total_delegated"], 50_000);
        assert_eq!(resp["result"]["effective_stake"], 140_000);
        assert!(resp["result"]["active"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn get_delegation_rpc() {
        let (state, _rx) = test_state_with_staking();
        let addr_hex = "bb".repeat(32);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getDelegation","params":["{}"],"id":1}}"#,
            addr_hex
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null(), "unexpected error: {:?}", resp);
        assert_eq!(resp["result"]["amount"], 50_000);
        assert_eq!(resp["result"]["round_delegated"], 2);
    }

    #[tokio::test]
    async fn get_active_validators_rpc() {
        let (state, _rx) = test_state_with_staking();
        let resp = rpc_call(
            &state,
            r#"{"jsonrpc":"2.0","method":"aztb_getActiveValidators","params":[],"id":1}"#,
        )
        .await;
        assert!(resp["error"].is_null(), "unexpected error: {:?}", resp);
        let list = resp["result"].as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["effective_stake"], 140_000);
    }

    #[tokio::test]
    async fn get_unbonding_status_rpc() {
        let (state, _rx) = test_state_with_staking();
        let vid_hex = "aa".repeat(32);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getUnbondingStatus","params":["{}"],"id":1}}"#,
            vid_hex
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null(), "unexpected error: {:?}", resp);
        let entries = resp["result"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["amount"], 10_000);
    }

    #[tokio::test]
    async fn get_agent_policy_rpc() {
        let (mut state, _rx) = test_state();
        let store = Arc::new(RwLock::new(AgentPolicyStore::new()));
        let agent = [0xCC; 32];
        let owner = [0xDD; 32];
        {
            let mut s = store.write().await;
            s.set_policy(
                agent,
                aztibase_execution::AgentPolicy {
                    owner,
                    per_tx_limit: 1_000,
                    per_epoch_limit: 5_000,
                    allowed_tx_kinds: vec![0x01, 0x06],
                    expiry_epoch: 200,
                },
            );
        }
        state.agent_policy_store = Some(store);
        let agent_hex = hex::encode(agent);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getAgentPolicy","params":["{}"],"id":1}}"#,
            agent_hex
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null(), "unexpected error: {:?}", resp);
        assert_eq!(resp["result"]["per_tx_limit"], "1000");
        assert_eq!(resp["result"]["per_epoch_limit"], "5000");
        assert_eq!(resp["result"]["expiry_epoch"], 200);
        let kinds = resp["result"]["allowed_tx_kinds"].as_array().unwrap();
        assert_eq!(kinds.len(), 2);
    }

    #[tokio::test]
    async fn get_agent_policy_not_found() {
        let (mut state, _rx) = test_state();
        state.agent_policy_store = Some(Arc::new(RwLock::new(AgentPolicyStore::new())));
        let agent_hex = hex::encode([0xFF; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getAgentPolicy","params":["{}"],"id":1}}"#,
            agent_hex
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null());
        assert!(resp["result"].is_null());
    }

    #[tokio::test]
    async fn get_block_tx_count_no_store() {
        let (state, _rx) = test_state();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_getBlockTransactionCount","params":[0],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(resp["error"].is_object());
    }

    #[tokio::test]
    async fn send_raw_transaction_alias() {
        let (state, _rx) = test_state();
        let body =
            r#"{"jsonrpc":"2.0","method":"aztb_sendRawTransaction","params":["aabb"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(
            !resp["error"]["message"]
                .as_str()
                .unwrap_or("")
                .contains("not found")
        );
    }

    #[tokio::test]
    async fn cors_preflight_returns_headers() {
        let (state, _rx) = test_state();
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        let router = Router::new()
            .route("/", post(handle_rpc))
            .layer(cors)
            .with_state(state);

        let request = Request::builder()
            .method("OPTIONS")
            .uri("/")
            .header("origin", "http://localhost:3000")
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        let headers = response.headers();
        assert!(
            headers.contains_key("access-control-allow-origin"),
            "CORS allow-origin header missing"
        );
    }

    // ── L2 Bridge RPC tests ─────────────────────────────────────────

    #[tokio::test]
    async fn get_l2_state_empty() {
        let (mut state, _rx) = test_state();
        state.l2_registry = Some(Arc::new(RwLock::new(L2Registry::new())));
        state.l2_anchor_store = Some(Arc::new(RwLock::new(L2AnchorStore::new())));
        let chain_hex = hex::encode([0xAA; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getL2State","params":["0x{chain_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null());
        assert!(resp["result"].is_null());
    }

    #[tokio::test]
    async fn list_l2s_returns_registered() {
        let (mut state, _rx) = test_state();
        let registry = Arc::new(RwLock::new(L2Registry::new()));
        {
            let mut reg = registry.write().await;
            reg.register(aztibase_execution::L2Registration {
                owner: [1u8; 32],
                l2_chain_id: [0xBB; 32],
                name: "TestRollup".into(),
                sequencer_set: vec![[2u8; 32]],
                bridge_address: [3u8; 32],
            })
            .unwrap();
        }
        state.l2_registry = Some(registry);
        let body = r#"{"jsonrpc":"2.0","method":"aztb_listL2s","params":[],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(resp["error"].is_null());
        let list = resp["result"].as_array().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], "TestRollup");
    }

    #[tokio::test]
    async fn get_bridge_balance_returns_locked() {
        let (mut state, _rx) = test_state();
        let escrow = Arc::new(RwLock::new(BridgeEscrow::new()));
        {
            let mut e = escrow.write().await;
            e.lock(&[0xCC; 32], &[0xDD; 32], 5000);
        }
        state.bridge_escrow = Some(escrow);
        let chain_hex = hex::encode([0xCC; 32]);
        let acct_hex = hex::encode([0xDD; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getBridgeBalance","params":["0x{chain_hex}","0x{acct_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["locked"], "5000");
    }

    #[tokio::test]
    async fn get_bridge_proof_status_unused() {
        let (mut state, _rx) = test_state();
        state.bridge_withdraw_proofs = Some(Arc::new(RwLock::new(BridgeWithdrawProofs::new())));
        let proof_hex = hex::encode([0xEE; 32]);
        let body = format!(
            r#"{{"jsonrpc":"2.0","method":"aztb_getBridgeProofStatus","params":["0x{proof_hex}"],"id":1}}"#
        );
        let resp = rpc_call(&state, &body).await;
        assert!(resp["error"].is_null());
        assert_eq!(resp["result"]["used"], false);
    }

    #[test]
    fn rate_limiter_allows_within_budget() {
        let limiter = RpcRateLimiter::new(5);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        for _ in 0..5 {
            assert!(limiter.check(ip));
        }
        assert!(!limiter.check(ip), "6th request should be rejected");
    }

    #[test]
    fn rate_limiter_isolates_ips() {
        let limiter = RpcRateLimiter::new(2);
        let a: IpAddr = "10.0.0.1".parse().unwrap();
        let b: IpAddr = "10.0.0.2".parse().unwrap();
        assert!(limiter.check(a));
        assert!(limiter.check(a));
        assert!(!limiter.check(a));
        assert!(limiter.check(b), "different IP should have own budget");
    }

    #[test]
    fn rate_limiter_zero_defaults_to_100() {
        let limiter = RpcRateLimiter::new(0);
        assert_eq!(limiter.limit_per_second(), DEFAULT_RATE_LIMIT);
    }

    #[tokio::test]
    async fn faucet_disabled_on_mainnet_profile() {
        let (mut state, _rx) = test_state();
        state.faucet_enabled = false;
        let body = r#"{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x0000000000000000000000000000000000000000000000000000000000000001"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        assert!(resp["error"].is_object(), "faucet should be disabled");
    }

    #[tokio::test]
    async fn faucet_enabled_on_testnet_profile() {
        let (mut state, _rx) = test_state();
        state.faucet_enabled = true;
        let body = r#"{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x0000000000000000000000000000000000000000000000000000000000000001"],"id":1}"#;
        let resp = rpc_call(&state, body).await;
        let has_disabled_error = resp["error"]["message"]
            .as_str()
            .map(|s| s.contains("disabled"))
            .unwrap_or(false);
        assert!(
            !has_disabled_error,
            "faucet should not be disabled on testnet"
        );
    }
}
