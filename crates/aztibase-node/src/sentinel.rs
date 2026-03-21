use std::collections::VecDeque;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, mpsc};
use tract_onnx::prelude::*;

/// How healthy the chain appears at a given point in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    Normal,
    Warning,
    Critical,
}

impl HealthLevel {
    fn from_score(score: f32) -> Self {
        if score > 0.7 {
            Self::Critical
        } else if score > 0.3 {
            Self::Warning
        } else {
            Self::Normal
        }
    }
}

/// A single chain-health snapshot produced by the Sentinel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainHealth {
    pub score: f32,
    pub level: HealthLevel,
    pub features: Vec<f32>,
    pub feature_names: Vec<String>,
    pub batch_height: u64,
    pub timestamp_ms: u64,
}

/// Raw inputs the Sentinel reads from shared node state on each tick.
#[derive(Clone, Debug, Default)]
pub struct SentinelInput {
    pub batch_height: u64,
    pub commit_latency_us: u64,
    pub txs_processed: u64,
    pub base_fee: u64,
    pub equivocations: u64,
    pub active_validators: u64,
    pub total_gas_used: u64,
    pub gas_limit: u64,
    pub empty_batches: u64,
    pub total_batches_window: u64,
    pub ms_since_last_finality: u64,
    pub peer_count: u64,
    pub mempool_size: u64,
}

const NUM_FEATURES: usize = 15;

const FEATURE_NAMES: [&str; NUM_FEATURES] = [
    "block_height_delta",
    "commit_latency_ms",
    "commit_latency_stddev",
    "tps",
    "tps_acceleration",
    "base_fee",
    "base_fee_delta",
    "equivocation_count",
    "validator_set_size",
    "total_gas_used",
    "block_fullness",
    "empty_block_ratio",
    "time_since_finality_ms",
    "peer_count",
    "mempool_size",
];

fn feature_name_strings() -> Vec<String> {
    FEATURE_NAMES.iter().map(|s| (*s).to_string()).collect()
}

const HISTORY_CAPACITY: usize = 1000;

/// Normalization parameters exported from the training script.
#[derive(Deserialize)]
struct NormParams {
    mins: Vec<f32>,
    ranges: Vec<f32>,
    threshold: f32,
}

/// Tier 2 ONNX autoencoder scorer.
/// Normalizes input features, runs inference, computes reconstruction error.
struct OnnxScorer {
    model: TypedModel,
    params: NormParams,
}

impl OnnxScorer {
    fn load(model_dir: &Path) -> Option<Self> {
        let onnx_path = model_dir.join("sentinel_v1.onnx");
        let params_path = model_dir.join("sentinel_v1_params.json");

        tracing::info!(
            onnx = %onnx_path.display(),
            params = %params_path.display(),
            onnx_exists = onnx_path.exists(),
            params_exists = params_path.exists(),
            "Sentinel ONNX: checking model files"
        );

        if !onnx_path.exists() || !params_path.exists() {
            return None;
        }

        let params_json = match std::fs::read_to_string(&params_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to read sentinel params");
                return None;
            }
        };
        let params: NormParams = match serde_json::from_str(&params_json) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse sentinel params");
                return None;
            }
        };

        if params.mins.len() != NUM_FEATURES || params.ranges.len() != NUM_FEATURES {
            tracing::warn!("Sentinel ONNX params dimension mismatch");
            return None;
        }

        let inference_model = match tract_onnx::onnx().model_for_path(&onnx_path) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load ONNX model");
                return None;
            }
        };

        let typed = match inference_model
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), [1, NUM_FEATURES as i64]),
            )
            .and_then(|m| m.into_optimized())
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to optimize ONNX model");
                return None;
            }
        };

        tracing::info!(
            threshold = params.threshold,
            "Sentinel Tier 2 ONNX model loaded"
        );
        Some(Self {
            model: typed,
            params,
        })
    }

    fn score(&self, features: &[f32]) -> f32 {
        let normalized: Vec<f32> = features
            .iter()
            .zip(self.params.mins.iter().zip(self.params.ranges.iter()))
            .map(|(f, (min, range))| {
                if *range == 0.0 {
                    0.5
                } else {
                    (f - min) / range
                }
            })
            .collect();

        let input =
            match tract_ndarray::Array2::from_shape_vec((1, NUM_FEATURES), normalized.clone()) {
                Ok(a) => a,
                Err(_) => return 0.0,
            };

        let plan = match self.model.clone().into_runnable() {
            Ok(p) => p,
            Err(_) => return 0.0,
        };

        let output = match plan.run(tvec!(TValue::from_const(input.into_arc_tensor()))) {
            Ok(r) => r,
            Err(_) => return 0.0,
        };

        let view = match output[0].to_array_view::<f32>() {
            Ok(v) => v,
            Err(_) => return 0.0,
        };
        let reconstructed: Vec<f32> = view.iter().copied().collect();
        if reconstructed.len() < NUM_FEATURES {
            return 0.0;
        }

        let mse: f32 = normalized
            .iter()
            .zip(reconstructed.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            / NUM_FEATURES as f32;

        // Map reconstruction error to [0, 1] score using threshold
        // Below threshold = healthy (0..0.3), above = warning/critical
        let ratio = mse / self.params.threshold;
        (ratio * 0.3).clamp(0.0, 1.0)
    }
}

/// Scores chain health from a 15-feature vector.
/// Uses ONNX autoencoder (Tier 2) if model is available, falls back to heuristic (Tier 1).
pub struct ChainHealthScorer {
    prev_batch_height: u64,
    prev_tps: f32,
    prev_base_fee: f32,
    latency_samples: VecDeque<f32>,
    onnx: Option<OnnxScorer>,
}

impl Default for ChainHealthScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainHealthScorer {
    pub fn new() -> Self {
        Self {
            prev_batch_height: 0,
            prev_tps: 0.0,
            prev_base_fee: 0.0,
            latency_samples: VecDeque::with_capacity(64),
            onnx: None,
        }
    }

    pub fn with_model_dir(mut self, model_dir: &Path) -> Self {
        self.onnx = OnnxScorer::load(model_dir);
        self
    }

    pub fn is_tier2(&self) -> bool {
        self.onnx.is_some()
    }

    /// Extract 15 features from raw input and return a health snapshot.
    pub fn score(&mut self, input: &SentinelInput) -> ChainHealth {
        let height_delta = input.batch_height.saturating_sub(self.prev_batch_height) as f32;
        let commit_latency_ms = input.commit_latency_us as f32 / 1000.0;

        self.latency_samples.push_back(commit_latency_ms);
        if self.latency_samples.len() > 64 {
            self.latency_samples.pop_front();
        }
        let latency_stddev = stddev(&self.latency_samples);

        let tps = if input.total_batches_window > 0 {
            input.txs_processed as f32 / input.total_batches_window.max(1) as f32
        } else {
            0.0
        };
        let tps_accel = tps - self.prev_tps;

        let base_fee = input.base_fee as f32;
        let base_fee_delta = base_fee - self.prev_base_fee;

        let equivocations = input.equivocations as f32;
        let validator_count = input.active_validators as f32;
        let total_gas = input.total_gas_used as f32;
        let block_fullness = if input.gas_limit > 0 {
            total_gas / input.gas_limit as f32
        } else {
            0.0
        };
        let empty_ratio = if input.total_batches_window > 0 {
            input.empty_batches as f32 / input.total_batches_window as f32
        } else {
            0.0
        };
        let time_since_finality = input.ms_since_last_finality as f32;
        let peer_count = input.peer_count as f32;
        let mempool_size = input.mempool_size as f32;

        self.prev_batch_height = input.batch_height;
        self.prev_tps = tps;
        self.prev_base_fee = base_fee;

        let features = vec![
            height_delta,
            commit_latency_ms,
            latency_stddev,
            tps,
            tps_accel,
            base_fee,
            base_fee_delta,
            equivocations,
            validator_count,
            total_gas,
            block_fullness,
            empty_ratio,
            time_since_finality,
            peer_count,
            mempool_size,
        ];

        let score = if let Some(ref onnx) = self.onnx {
            onnx.score(&features)
        } else {
            self.heuristic_score(&features)
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        ChainHealth {
            score,
            level: HealthLevel::from_score(score),
            features,
            feature_names: feature_name_strings(),
            batch_height: input.batch_height,
            timestamp_ms: now,
        }
    }

    /// Deterministic heuristic scoring over the 15-feature vector.
    /// Each sub-score contributes a weighted penalty; total is clamped to [0, 1].
    fn heuristic_score(&self, f: &[f32]) -> f32 {
        let mut score: f32 = 0.0;

        // f[0] = height_delta: 0 means chain stalled
        if f[0] < 1.0 {
            score += 0.25;
        }

        // f[1] = commit_latency_ms: above 2000ms is concerning
        if f[1] > 2000.0 {
            score += 0.10;
        }
        if f[1] > 5000.0 {
            score += 0.10;
        }

        // f[2] = latency_stddev: high jitter signals instability
        if f[2] > 1000.0 {
            score += 0.05;
        }

        // f[7] = equivocations: any is bad
        if f[7] > 0.0 {
            score += 0.15;
        }

        // f[8] = validator_count: below 3 is precarious
        if f[8] < 3.0 && f[8] > 0.0 {
            score += 0.10;
        }
        if f[8] < 1.0 {
            score += 0.15;
        }

        // f[11] = empty_block_ratio: above 80% means low activity
        if f[11] > 0.8 {
            score += 0.05;
        }

        // f[12] = time_since_finality_ms: above 10s is concerning
        if f[12] > 10_000.0 {
            score += 0.10;
        }
        if f[12] > 30_000.0 {
            score += 0.10;
        }

        // f[13] = peer_count: 0 peers is isolated
        if f[13] < 1.0 {
            score += 0.10;
        }

        score.clamp(0.0, 1.0)
    }
}

/// Appends feature vectors to a CSV file for Tier 2 training data collection.
struct FeatureExporter {
    path: PathBuf,
    header_written: bool,
}

impl FeatureExporter {
    fn new(path: PathBuf) -> Self {
        let header_written = path.exists();
        Self {
            path,
            header_written,
        }
    }

    fn append(&mut self, health: &ChainHealth) {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path);

        let mut file = match file {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(path = %self.path.display(), error = %e, "Failed to open sentinel CSV");
                return;
            }
        };

        if !self.header_written {
            let header = format!(
                "timestamp_ms,batch_height,score,{}\n",
                FEATURE_NAMES.join(",")
            );
            if file.write_all(header.as_bytes()).is_err() {
                return;
            }
            self.header_written = true;
        }

        let features_csv: String = health
            .features
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let row = format!(
            "{},{},{},{}\n",
            health.timestamp_ms, health.batch_height, health.score, features_csv
        );
        let _ = file.write_all(row.as_bytes());
    }
}

/// Shared Sentinel state accessible from RPC and WebSocket handlers.
pub struct SentinelState {
    history: RwLock<VecDeque<ChainHealth>>,
    latest: RwLock<Option<ChainHealth>>,
    actions: RwLock<VecDeque<SentinelAction>>,
    enabled: bool,
    rpc_latest: Option<Arc<RwLock<Option<serde_json::Value>>>>,
    rpc_history: Option<Arc<RwLock<Vec<serde_json::Value>>>>,
    rpc_actions: Option<Arc<RwLock<Vec<serde_json::Value>>>>,
    event_bus: Option<Arc<dyn HealthPublisher + Send + Sync>>,
    exporter: std::sync::Mutex<Option<FeatureExporter>>,
}

/// Trait for publishing health events (implemented by EventBus in aztibase-rpc).
pub trait HealthPublisher {
    fn publish_chain_health(&self, health: serde_json::Value);
}

impl SentinelState {
    pub fn new(enabled: bool) -> Self {
        Self {
            history: RwLock::new(VecDeque::with_capacity(HISTORY_CAPACITY)),
            latest: RwLock::new(None),
            actions: RwLock::new(VecDeque::with_capacity(ACTION_LOG_CAPACITY)),
            enabled,
            rpc_latest: None,
            rpc_history: None,
            rpc_actions: None,
            event_bus: None,
            exporter: std::sync::Mutex::new(None),
        }
    }

    pub fn with_rpc(
        mut self,
        latest: Arc<RwLock<Option<serde_json::Value>>>,
        history: Arc<RwLock<Vec<serde_json::Value>>>,
    ) -> Self {
        self.rpc_latest = Some(latest);
        self.rpc_history = Some(history);
        self
    }

    pub fn with_rpc_actions(mut self, actions: Arc<RwLock<Vec<serde_json::Value>>>) -> Self {
        self.rpc_actions = Some(actions);
        self
    }

    pub fn with_event_bus(mut self, bus: Arc<dyn HealthPublisher + Send + Sync>) -> Self {
        self.event_bus = Some(bus);
        self
    }

    pub fn with_export(self, data_dir: PathBuf) -> Self {
        let csv_path = data_dir.join("sentinel_features.csv");
        tracing::info!(path = %csv_path.display(), "Sentinel CSV export enabled");
        *self.exporter.lock().unwrap() = Some(FeatureExporter::new(csv_path));
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn push(&self, health: ChainHealth) {
        let json = serde_json::to_value(&health).unwrap_or_default();

        // Update internal state
        let mut hist = self.history.write().await;
        if hist.len() >= HISTORY_CAPACITY {
            hist.pop_front();
        }
        let mut latest = self.latest.write().await;
        *latest = Some(health.clone());
        hist.push_back(health.clone());

        // Push to RPC shared state
        if let Some(ref rpc_latest) = self.rpc_latest {
            *rpc_latest.write().await = Some(json.clone());
        }
        if let Some(ref rpc_history) = self.rpc_history {
            let mut rpc_hist = rpc_history.write().await;
            rpc_hist.push(json.clone());
            if rpc_hist.len() > HISTORY_CAPACITY {
                rpc_hist.remove(0);
            }
        }

        // Publish to WebSocket subscribers
        if let Some(ref bus) = self.event_bus {
            bus.publish_chain_health(json);
        }

        // Append to CSV for Tier 2 training data
        if let Ok(mut guard) = self.exporter.lock()
            && let Some(ref mut exporter) = *guard
        {
            exporter.append(&health);
        }
    }

    pub async fn latest(&self) -> Option<ChainHealth> {
        self.latest.read().await.clone()
    }

    pub async fn history(&self, limit: usize) -> Vec<ChainHealth> {
        let hist = self.history.read().await;
        let take = limit.min(hist.len()).min(100);
        hist.iter().rev().take(take).cloned().collect()
    }

    pub async fn push_action(&self, action: SentinelAction) {
        let json = serde_json::to_value(&action).unwrap_or_default();

        let mut actions = self.actions.write().await;
        if actions.len() >= ACTION_LOG_CAPACITY {
            actions.pop_front();
        }
        actions.push_back(action);

        if let Some(ref rpc_actions) = self.rpc_actions {
            let mut rpc_acts = rpc_actions.write().await;
            rpc_acts.push(json.clone());
            if rpc_acts.len() > ACTION_LOG_CAPACITY {
                rpc_acts.remove(0);
            }
        }

        // Publish action events on the chainHealth WebSocket topic
        if let Some(ref bus) = self.event_bus {
            bus.publish_chain_health(json);
        }
    }

    pub async fn recent_actions(&self, limit: usize) -> Vec<SentinelAction> {
        let actions = self.actions.read().await;
        let take = limit.min(actions.len()).min(ACTION_LOG_CAPACITY);
        actions.iter().rev().take(take).cloned().collect()
    }
}

// ──────────────────────────────────────────────────────────────
// Tier 3: Autonomous Action Engine
// ──────────────────────────────────────────────────────────────

/// Confidence tier derived from the health score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfidenceTier {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl ConfidenceTier {
    pub fn from_score(score: f32) -> Self {
        if score >= 0.95 {
            Self::Critical
        } else if score >= 0.8 {
            Self::High
        } else if score >= 0.5 {
            Self::Medium
        } else if score >= 0.3 {
            Self::Low
        } else {
            Self::None
        }
    }
}

/// An action the Sentinel decided to take (or recommends taking).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SentinelAction {
    pub tier: ConfidenceTier,
    pub kind: ActionKind,
    pub score: f32,
    pub batch_height: u64,
    pub timestamp_ms: u64,
    pub dry_run: bool,
}

/// The category of autonomous action.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionKind {
    /// Log warning, no on-chain effect.
    Alert { message: String },
    /// Adjust base fee parameter via EmergencyAction::ForceParam.
    AdjustBaseFee {
        param: String,
        old_value: u64,
        new_value: u64,
    },
    /// Submit a governance proposal for human vote.
    ProposeGovernance {
        title: String,
        param_key: String,
        param_value: String,
    },
    /// Emergency chain pause via EmergencyAction::Pause.
    EmergencyPause,
    /// Per-validator performance warning (observer-only).
    ValidatorWarning {
        validator_id: [u8; 32],
        message: String,
    },
    /// Cross-epoch pattern detected (observer-only).
    PatternAlert { pattern: String, details: String },
}

const ACTION_LOG_CAPACITY: usize = 100;
const SUSTAINED_TICKS_LOW: usize = 2;
const SUSTAINED_TICKS_MEDIUM: usize = 3;
const SUSTAINED_TICKS_HIGH: usize = 3;
const SUSTAINED_TICKS_CRITICAL: usize = 2;
const ALERT_COOLDOWN_BATCHES: u64 = 100;
const ONCHAIN_COOLDOWN_BATCHES: u64 = 1000;
const BASE_FEE_ADJUST_PERCENT: u64 = 5;

/// Evaluates health scores and decides whether to take autonomous action.
///
/// Safety guardrails:
/// - Sustained anomaly: N consecutive ticks above threshold before acting
/// - Cooldown: minimum batches between successive actions
/// - Dry-run: default mode — logs what would happen without executing
/// - Graceful degrade: nodes without emergency key only emit alerts
pub struct ActionEngine {
    dry_run: bool,
    auto_pause_enabled: bool,
    has_emergency_key: bool,
    current_epoch: Arc<AtomicU64>,
    sunset_epoch: u64,
    /// Ring buffer of consecutive scores for sustained anomaly detection.
    sustained_window: VecDeque<f32>,
    sustained_window_cap: usize,
    last_alert_batch: Option<u64>,
    last_onchain_batch: Option<u64>,
    action_log: VecDeque<SentinelAction>,
    /// Channel to send signed transaction bytes to the node main loop for mempool injection.
    tx_sender: Option<mpsc::Sender<Vec<u8>>>,
}

impl ActionEngine {
    pub fn new(dry_run: bool, auto_pause_enabled: bool) -> Self {
        Self {
            dry_run,
            auto_pause_enabled,
            has_emergency_key: false,
            current_epoch: Arc::new(AtomicU64::new(0)),
            sunset_epoch: aztibase_execution::EMERGENCY_KEY_SUNSET_EPOCH,
            sustained_window: VecDeque::with_capacity(8),
            sustained_window_cap: 8,
            last_alert_batch: None,
            last_onchain_batch: None,
            action_log: VecDeque::with_capacity(ACTION_LOG_CAPACITY),
            tx_sender: None,
        }
    }

    pub fn with_emergency_key(mut self, has_key: bool) -> Self {
        self.has_emergency_key = has_key;
        self
    }

    pub fn with_epoch(mut self, epoch: Arc<AtomicU64>) -> Self {
        self.current_epoch = epoch;
        self
    }

    pub fn with_tx_sender(mut self, sender: mpsc::Sender<Vec<u8>>) -> Self {
        self.tx_sender = Some(sender);
        self
    }

    /// Evaluate a health snapshot and return an action if warranted.
    pub fn evaluate(&mut self, health: &ChainHealth, base_fee: u64) -> Option<SentinelAction> {
        self.sustained_window.push_back(health.score);
        if self.sustained_window.len() > self.sustained_window_cap {
            self.sustained_window.pop_front();
        }

        let tier = ConfidenceTier::from_score(health.score);
        if tier == ConfidenceTier::None {
            return None;
        }

        let sustained_count = self.sustained_count_at_tier(tier);
        let required = match tier {
            ConfidenceTier::Critical => SUSTAINED_TICKS_CRITICAL,
            ConfidenceTier::High => SUSTAINED_TICKS_HIGH,
            ConfidenceTier::Medium => SUSTAINED_TICKS_MEDIUM,
            ConfidenceTier::Low => SUSTAINED_TICKS_LOW,
            ConfidenceTier::None => unreachable!(),
        };
        if sustained_count < required {
            return None;
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let action = match tier {
            ConfidenceTier::Critical if self.has_emergency_key && self.auto_pause_enabled => {
                if !self.cooldown_ok(health.batch_height, true) {
                    return None;
                }
                let epoch = self.current_epoch.load(Ordering::Relaxed);
                if epoch >= self.sunset_epoch {
                    tracing::warn!(
                        epoch,
                        sunset = self.sunset_epoch,
                        "Sentinel CRITICAL but emergency key expired — recommending manual intervention"
                    );
                    return Some(self.record_action(SentinelAction {
                        tier,
                        kind: ActionKind::Alert {
                            message: format!(
                                "CRITICAL anomaly (score={:.3}) but emergency key expired at epoch {epoch}",
                                health.score
                            ),
                        },
                        score: health.score,
                        batch_height: health.batch_height,
                        timestamp_ms: now_ms,
                        dry_run: self.dry_run,
                    }));
                }
                self.last_onchain_batch = Some(health.batch_height);
                SentinelAction {
                    tier,
                    kind: ActionKind::EmergencyPause,
                    score: health.score,
                    batch_height: health.batch_height,
                    timestamp_ms: now_ms,
                    dry_run: self.dry_run,
                }
            }
            ConfidenceTier::High => {
                if !self.cooldown_ok(health.batch_height, true) {
                    return None;
                }
                self.last_onchain_batch = Some(health.batch_height);
                SentinelAction {
                    tier,
                    kind: ActionKind::ProposeGovernance {
                        title: format!(
                            "Sentinel: chain health degraded (score={:.2})",
                            health.score
                        ),
                        param_key: "base_fee_floor".into(),
                        param_value: adjusted_fee(base_fee, true).to_string(),
                    },
                    score: health.score,
                    batch_height: health.batch_height,
                    timestamp_ms: now_ms,
                    dry_run: self.dry_run,
                }
            }
            ConfidenceTier::Medium if self.has_emergency_key => {
                if !self.cooldown_ok(health.batch_height, true) {
                    return None;
                }
                let new_fee = adjusted_fee(base_fee, true);
                self.last_onchain_batch = Some(health.batch_height);
                SentinelAction {
                    tier,
                    kind: ActionKind::AdjustBaseFee {
                        param: "base_fee_floor".into(),
                        old_value: base_fee,
                        new_value: new_fee,
                    },
                    score: health.score,
                    batch_height: health.batch_height,
                    timestamp_ms: now_ms,
                    dry_run: self.dry_run,
                }
            }
            ConfidenceTier::Low | ConfidenceTier::Medium => {
                if !self.cooldown_ok(health.batch_height, false) {
                    return None;
                }
                self.last_alert_batch = Some(health.batch_height);
                SentinelAction {
                    tier,
                    kind: ActionKind::Alert {
                        message: format!(
                            "Chain health {tier:?} (score={:.3}, batch={})",
                            health.score, health.batch_height
                        ),
                    },
                    score: health.score,
                    batch_height: health.batch_height,
                    timestamp_ms: now_ms,
                    dry_run: self.dry_run,
                }
            }
            _ => {
                if !self.cooldown_ok(health.batch_height, false) {
                    return None;
                }
                self.last_alert_batch = Some(health.batch_height);
                SentinelAction {
                    tier,
                    kind: ActionKind::Alert {
                        message: format!(
                            "Chain health {tier:?} (score={:.3}), no key for on-chain action",
                            health.score
                        ),
                    },
                    score: health.score,
                    batch_height: health.batch_height,
                    timestamp_ms: now_ms,
                    dry_run: self.dry_run,
                }
            }
        };

        Some(self.record_action(action))
    }

    /// Send signed transaction bytes to the node for mempool injection.
    pub async fn submit_tx(&self, tx_bytes: Vec<u8>) -> bool {
        if self.dry_run {
            tracing::info!(
                len = tx_bytes.len(),
                "Sentinel dry-run: would submit tx to mempool"
            );
            return false;
        }
        if let Some(ref sender) = self.tx_sender {
            sender.send(tx_bytes).await.is_ok()
        } else {
            tracing::warn!("Sentinel has no tx_sender — cannot submit on-chain action");
            false
        }
    }

    pub fn action_log(&self) -> &VecDeque<SentinelAction> {
        &self.action_log
    }

    pub fn recent_actions(&self, limit: usize) -> Vec<SentinelAction> {
        let take = limit.min(self.action_log.len()).min(ACTION_LOG_CAPACITY);
        self.action_log.iter().rev().take(take).cloned().collect()
    }

    fn cooldown_ok(&self, batch_height: u64, is_onchain: bool) -> bool {
        if is_onchain {
            match self.last_onchain_batch {
                None => true,
                Some(last) => batch_height >= last + ONCHAIN_COOLDOWN_BATCHES,
            }
        } else {
            match self.last_alert_batch {
                None => true,
                Some(last) => batch_height >= last + ALERT_COOLDOWN_BATCHES,
            }
        }
    }

    fn sustained_count_at_tier(&self, tier: ConfidenceTier) -> usize {
        let threshold = match tier {
            ConfidenceTier::Critical => 0.95,
            ConfidenceTier::High => 0.8,
            ConfidenceTier::Medium => 0.5,
            ConfidenceTier::Low => 0.3,
            ConfidenceTier::None => return 0,
        };
        self.sustained_window
            .iter()
            .rev()
            .take_while(|&&s| s >= threshold)
            .count()
    }

    fn record_action(&mut self, action: SentinelAction) -> SentinelAction {
        if self.action_log.len() >= ACTION_LOG_CAPACITY {
            self.action_log.pop_front();
        }
        self.action_log.push_back(action.clone());
        action
    }
}

fn adjusted_fee(current: u64, increase: bool) -> u64 {
    let delta = current.max(1) * BASE_FEE_ADJUST_PERCENT / 100;
    if increase {
        current.saturating_add(delta.max(1))
    } else {
        current.saturating_sub(delta).max(1)
    }
}

/// Shared atomic handles the Sentinel reads from the main event loop.
pub struct SentinelHandles {
    pub batch_count: Arc<AtomicU64>,
    pub base_fee: Arc<AtomicU64>,
    pub commit_latency_us: Arc<AtomicU64>,
    pub equivocations: Arc<AtomicU64>,
    pub txs_processed: Arc<AtomicU64>,
    pub peer_count: Arc<AtomicU64>,
    pub mempool_size: Arc<AtomicU64>,
    pub active_validators: Arc<AtomicU64>,
}

/// Configuration for the Sentinel Tier 3 action engine.
pub struct SentinelConfig {
    pub dry_run: bool,
    pub auto_pause: bool,
    pub has_emergency_key: bool,
    pub current_epoch: Arc<AtomicU64>,
    pub tx_sender: Option<mpsc::Sender<Vec<u8>>>,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            dry_run: true,
            auto_pause: false,
            has_emergency_key: false,
            current_epoch: Arc::new(AtomicU64::new(0)),
            tx_sender: None,
        }
    }
}

/// Runs the Sentinel tick loop.
///
/// Reads shared node state every `interval_batches` batches, scores health,
/// and pushes to `SentinelState` for RPC/WebSocket consumption.
/// Tier 3 action engine evaluates each score and may trigger autonomous actions.
pub async fn run_sentinel(
    state: Arc<SentinelState>,
    handles: SentinelHandles,
    interval_batches: u64,
    model_dir: Option<PathBuf>,
    action_config: SentinelConfig,
    node_metrics: Option<aztibase_rpc::NodeMetrics>,
) {
    let mut scorer = if let Some(ref dir) = model_dir {
        let s = ChainHealthScorer::new().with_model_dir(dir);
        if s.is_tier2() {
            tracing::info!("Sentinel running in Tier 2 (ONNX) mode");
        } else {
            tracing::info!("Sentinel running in Tier 1 (heuristic) mode — no ONNX model found");
        }
        s
    } else {
        tracing::info!("Sentinel running in Tier 1 (heuristic) mode");
        ChainHealthScorer::new()
    };

    let mut action_engine = ActionEngine::new(action_config.dry_run, action_config.auto_pause)
        .with_emergency_key(action_config.has_emergency_key)
        .with_epoch(action_config.current_epoch);
    if let Some(sender) = action_config.tx_sender {
        action_engine = action_engine.with_tx_sender(sender);
    }
    tracing::info!(
        dry_run = action_engine.dry_run,
        auto_pause = action_engine.auto_pause_enabled,
        has_emergency_key = action_engine.has_emergency_key,
        "Sentinel Tier 3 action engine initialized"
    );

    let mut last_scored_batch: u64 = 0;
    let mut last_seen_batch: u64 = 0;
    let mut last_progress_time = std::time::Instant::now();
    let mut stall_reported = false;
    const STALL_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(120);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let current_batch = handles.batch_count.load(Ordering::Relaxed);

        // Track progress against the last *seen* batch, not last scored
        if current_batch > last_seen_batch {
            last_seen_batch = current_batch;
            last_progress_time = std::time::Instant::now();
            stall_reported = false;
        }

        // Detect chain stall — no new batches for 30+ seconds.
        // Force CRITICAL regardless of model output: a halted chain is always critical.
        // Re-publish every 10s so the RPC score stays current during extended stalls.
        let stall_elapsed = last_progress_time.elapsed();
        let stalled = stall_elapsed > STALL_THRESHOLD && current_batch > 0;
        if stalled {
            let stall_secs = stall_elapsed.as_secs();
            if !stall_reported || stall_secs % 10 < 3 {
                stall_reported = true;
                let stall_health = ChainHealth {
                    score: 1.0,
                    level: HealthLevel::Critical,
                    features: vec![0.0; FEATURE_NAMES.len()],
                    feature_names: FEATURE_NAMES.iter().map(|s| s.to_string()).collect(),
                    batch_height: current_batch,
                    timestamp_ms: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                tracing::warn!(
                    stall_secs,
                    batch = current_batch,
                    peer_count = handles.peer_count.load(Ordering::Relaxed),
                    "Chain stall detected — no new batches, forcing CRITICAL"
                );
                // Tier 3: evaluate stall for autonomous action
                let base_fee = handles.base_fee.load(Ordering::Relaxed);
                if let Some(action) = action_engine.evaluate(&stall_health, base_fee) {
                    log_action(&action);
                    if let Some(ref m) = node_metrics {
                        m.inc_sentinel_actions();
                    }
                    state.push_action(action).await;
                }
                if let Some(ref m) = node_metrics {
                    m.set_sentinel_health(
                        (stall_health.score * 1000.0) as i64,
                        health_level_int(stall_health.level),
                    );
                }
                state.push(stall_health).await;
            }
            continue;
        }

        if current_batch < last_scored_batch + interval_batches {
            continue;
        }

        let base_fee = handles.base_fee.load(Ordering::Relaxed);
        let input = SentinelInput {
            batch_height: current_batch,
            commit_latency_us: handles.commit_latency_us.load(Ordering::Relaxed),
            txs_processed: handles.txs_processed.load(Ordering::Relaxed),
            base_fee,
            equivocations: handles.equivocations.load(Ordering::Relaxed),
            active_validators: handles.active_validators.load(Ordering::Relaxed),
            total_gas_used: 0,
            gas_limit: 0,
            empty_batches: 0,
            total_batches_window: current_batch.saturating_sub(last_scored_batch),
            ms_since_last_finality: 0,
            peer_count: handles.peer_count.load(Ordering::Relaxed),
            mempool_size: handles.mempool_size.load(Ordering::Relaxed),
        };

        let health = scorer.score(&input);

        tracing::debug!(
            score = health.score,
            level = ?health.level,
            batch = health.batch_height,
            "Sentinel tick"
        );

        if health.level != HealthLevel::Normal {
            tracing::warn!(
                score = health.score,
                level = ?health.level,
                batch = health.batch_height,
                "Chain health degraded"
            );
        }

        // Tier 3: evaluate health for autonomous action
        if let Some(action) = action_engine.evaluate(&health, base_fee) {
            log_action(&action);
            if let Some(ref m) = node_metrics {
                m.inc_sentinel_actions();
            }
            state.push_action(action).await;
        }

        if let Some(ref m) = node_metrics {
            m.set_sentinel_health(
                (health.score * 1000.0) as i64,
                health_level_int(health.level),
            );
        }

        state.push(health).await;
        last_scored_batch = current_batch;
    }
}

fn health_level_int(level: HealthLevel) -> i64 {
    match level {
        HealthLevel::Normal => 0,
        HealthLevel::Warning => 1,
        HealthLevel::Critical => 2,
    }
}

fn log_action(action: &SentinelAction) {
    let dry_tag = if action.dry_run { " [DRY-RUN]" } else { "" };
    match &action.kind {
        ActionKind::Alert { message } => {
            tracing::warn!(
                tier = ?action.tier,
                score = action.score,
                batch = action.batch_height,
                "{message}{dry_tag}"
            );
        }
        ActionKind::AdjustBaseFee {
            param,
            old_value,
            new_value,
        } => {
            tracing::warn!(
                tier = ?action.tier,
                score = action.score,
                %param, old_value, new_value,
                "Sentinel action: adjust base fee{dry_tag}"
            );
        }
        ActionKind::ProposeGovernance {
            title,
            param_key,
            param_value,
        } => {
            tracing::warn!(
                tier = ?action.tier,
                score = action.score,
                %title, %param_key, %param_value,
                "Sentinel action: governance proposal{dry_tag}"
            );
        }
        ActionKind::EmergencyPause => {
            tracing::error!(
                tier = ?action.tier,
                score = action.score,
                batch = action.batch_height,
                "Sentinel action: EMERGENCY PAUSE{dry_tag}"
            );
        }
        ActionKind::ValidatorWarning {
            validator_id,
            message,
        } => {
            tracing::warn!(
                tier = ?action.tier,
                validator = format!("{:02x}{:02x}{:02x}{:02x}", validator_id[0], validator_id[1], validator_id[2], validator_id[3]),
                "{message}{dry_tag}"
            );
        }
        ActionKind::PatternAlert { pattern, details } => {
            tracing::warn!(
                tier = ?action.tier,
                %pattern, %details,
                "Sentinel pattern alert{dry_tag}"
            );
        }
    }
}

fn stddev(samples: &VecDeque<f32>) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let n = samples.len() as f32;
    let mean = samples.iter().sum::<f32>() / n;
    let variance = samples.iter().map(|s| (s - mean).powi(2)).sum::<f32>() / n;
    variance.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_chain_scores_low() {
        let mut scorer = ChainHealthScorer::new();
        let input = SentinelInput {
            batch_height: 100,
            commit_latency_us: 400_000,
            txs_processed: 500,
            base_fee: 1,
            equivocations: 0,
            active_validators: 5,
            total_gas_used: 100_000,
            gas_limit: 1_000_000,
            empty_batches: 10,
            total_batches_window: 100,
            ms_since_last_finality: 1600,
            peer_count: 4,
            mempool_size: 50,
        };
        let health = scorer.score(&input);
        assert_eq!(health.level, HealthLevel::Normal);
        assert!(health.score < 0.3, "healthy chain score: {}", health.score);
    }

    #[test]
    fn stalled_chain_scores_high() {
        let mut scorer = ChainHealthScorer::new();
        // First tick to set prev_batch_height
        scorer.score(&SentinelInput {
            batch_height: 100,
            active_validators: 3,
            peer_count: 2,
            total_batches_window: 100,
            ..Default::default()
        });
        // Second tick with no progress
        let health = scorer.score(&SentinelInput {
            batch_height: 100,
            commit_latency_us: 10_000_000,
            equivocations: 2,
            active_validators: 1,
            ms_since_last_finality: 60_000,
            peer_count: 0,
            total_batches_window: 0,
            ..Default::default()
        });
        assert_eq!(health.level, HealthLevel::Critical);
        assert!(health.score > 0.7, "stalled chain score: {}", health.score);
    }

    #[test]
    fn equivocation_raises_score() {
        let mut scorer = ChainHealthScorer::new();
        let baseline = scorer.score(&SentinelInput {
            batch_height: 50,
            active_validators: 5,
            peer_count: 4,
            total_batches_window: 50,
            ..Default::default()
        });
        let with_equiv = scorer.score(&SentinelInput {
            batch_height: 100,
            equivocations: 1,
            active_validators: 5,
            peer_count: 4,
            total_batches_window: 50,
            ..Default::default()
        });
        assert!(
            with_equiv.score > baseline.score,
            "equivocation should raise score: {} vs {}",
            with_equiv.score,
            baseline.score
        );
    }

    #[test]
    fn health_level_thresholds() {
        assert_eq!(HealthLevel::from_score(0.0), HealthLevel::Normal);
        assert_eq!(HealthLevel::from_score(0.29), HealthLevel::Normal);
        assert_eq!(HealthLevel::from_score(0.31), HealthLevel::Warning);
        assert_eq!(HealthLevel::from_score(0.69), HealthLevel::Warning);
        assert_eq!(HealthLevel::from_score(0.71), HealthLevel::Critical);
        assert_eq!(HealthLevel::from_score(1.0), HealthLevel::Critical);
    }

    #[test]
    fn score_clamped_to_unit() {
        let mut scorer = ChainHealthScorer::new();
        let health = scorer.score(&SentinelInput {
            batch_height: 0,
            commit_latency_us: 100_000_000,
            equivocations: 100,
            active_validators: 0,
            ms_since_last_finality: 1_000_000,
            peer_count: 0,
            total_batches_window: 0,
            ..Default::default()
        });
        assert!(health.score >= 0.0);
        assert!(health.score <= 1.0);
    }

    #[test]
    fn feature_names_match_feature_count() {
        let mut scorer = ChainHealthScorer::new();
        let health = scorer.score(&SentinelInput {
            batch_height: 10,
            active_validators: 3,
            peer_count: 2,
            total_batches_window: 10,
            ..Default::default()
        });
        assert_eq!(health.features.len(), 15);
        assert_eq!(health.feature_names.len(), 15);
    }

    #[test]
    fn history_bounded() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = SentinelState::new(true);
            for i in 0..1100 {
                state
                    .push(ChainHealth {
                        score: 0.1,
                        level: HealthLevel::Normal,
                        features: vec![0.0; 15],
                        feature_names: feature_name_strings(),
                        batch_height: i,
                        timestamp_ms: 0,
                    })
                    .await;
            }
            let hist = state.history(2000).await;
            assert!(hist.len() <= 100, "history query capped at 100");
            let full = state.history.read().await;
            assert_eq!(full.len(), HISTORY_CAPACITY);
        });
    }

    #[test]
    fn csv_export_writes_header_and_rows() {
        let dir = std::env::temp_dir().join(format!("sentinel_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("sentinel_features.csv");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = SentinelState::new(true).with_export(dir.clone());
            for i in 0..3 {
                state
                    .push(ChainHealth {
                        score: 0.1 + i as f32 * 0.1,
                        level: HealthLevel::Normal,
                        features: vec![i as f32; 15],
                        feature_names: feature_name_strings(),
                        batch_height: i * 50,
                        timestamp_ms: 1000 + i,
                    })
                    .await;
            }
        });

        let content = std::fs::read_to_string(&csv_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 4, "1 header + 3 data rows");
        assert!(lines[0].starts_with("timestamp_ms,batch_height,score,"));
        assert!(lines[0].contains("block_height_delta"));
        assert!(lines[1].starts_with("1000,0,"));

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn csv_export_appends_without_duplicate_header() {
        let dir = std::env::temp_dir().join(format!("sentinel_append_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv_path = dir.join("sentinel_features.csv");

        // Pre-create the CSV with a header (simulates node restart)
        std::fs::write(
            &csv_path,
            "timestamp_ms,batch_height,score,f1\n1,2,0.1,0.0\n",
        )
        .unwrap();

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = SentinelState::new(true).with_export(dir.clone());
            state
                .push(ChainHealth {
                    score: 0.2,
                    level: HealthLevel::Normal,
                    features: vec![1.0; 15],
                    feature_names: feature_name_strings(),
                    batch_height: 100,
                    timestamp_ms: 5000,
                })
                .await;
        });

        let content = std::fs::read_to_string(&csv_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        // Should have original header + original row + new row (no duplicate header)
        assert_eq!(lines.len(), 3, "original header + 1 old row + 1 new row");
        assert!(lines[2].starts_with("5000,100,"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn latency_stddev_computed() {
        let mut scorer = ChainHealthScorer::new();
        // Push several ticks with varying latency
        for lat_us in [100_000, 200_000, 300_000, 400_000, 500_000] {
            scorer.score(&SentinelInput {
                batch_height: 100,
                commit_latency_us: lat_us,
                active_validators: 3,
                peer_count: 2,
                total_batches_window: 1,
                ..Default::default()
            });
        }
        let health = scorer.score(&SentinelInput {
            batch_height: 200,
            commit_latency_us: 600_000,
            active_validators: 3,
            peer_count: 2,
            total_batches_window: 100,
            ..Default::default()
        });
        // features[2] is latency_stddev — should be nonzero after varying inputs
        assert!(health.features[2] > 0.0, "stddev should be nonzero");
    }

    // ─── Tier 3 Action Engine tests ──────────────────────────

    fn make_health(score: f32, batch: u64) -> ChainHealth {
        ChainHealth {
            score,
            level: HealthLevel::from_score(score),
            features: vec![0.0; 15],
            feature_names: feature_name_strings(),
            batch_height: batch,
            timestamp_ms: 0,
        }
    }

    #[test]
    fn confidence_tier_from_score() {
        assert_eq!(ConfidenceTier::from_score(0.0), ConfidenceTier::None);
        assert_eq!(ConfidenceTier::from_score(0.29), ConfidenceTier::None);
        assert_eq!(ConfidenceTier::from_score(0.3), ConfidenceTier::Low);
        assert_eq!(ConfidenceTier::from_score(0.49), ConfidenceTier::Low);
        assert_eq!(ConfidenceTier::from_score(0.5), ConfidenceTier::Medium);
        assert_eq!(ConfidenceTier::from_score(0.79), ConfidenceTier::Medium);
        assert_eq!(ConfidenceTier::from_score(0.8), ConfidenceTier::High);
        assert_eq!(ConfidenceTier::from_score(0.94), ConfidenceTier::High);
        assert_eq!(ConfidenceTier::from_score(0.95), ConfidenceTier::Critical);
        assert_eq!(ConfidenceTier::from_score(1.0), ConfidenceTier::Critical);
    }

    #[test]
    fn low_score_requires_sustained_ticks() {
        let mut engine = ActionEngine::new(true, false);
        // Single tick at LOW should not trigger (needs 2 sustained)
        let action = engine.evaluate(&make_health(0.35, 1000), 1);
        assert!(action.is_none(), "single LOW tick should not trigger");

        // Second sustained tick triggers
        let action = engine.evaluate(&make_health(0.35, 1100), 1);
        assert!(action.is_some(), "second sustained LOW tick should trigger");
        let a = action.unwrap();
        assert_eq!(a.tier, ConfidenceTier::Low);
        assert!(matches!(a.kind, ActionKind::Alert { .. }));
    }

    #[test]
    fn medium_without_key_falls_back_to_alert() {
        let mut engine = ActionEngine::new(true, false);
        // No emergency key — MEDIUM should produce Alert, not AdjustBaseFee
        for i in 0..SUSTAINED_TICKS_MEDIUM {
            let _ = engine.evaluate(&make_health(0.6, 1000 + (i as u64 * 200)), 1);
        }
        let action = engine.evaluate(&make_health(0.6, 2000), 1);
        // The last evaluate should have triggered after sustained count met
        // Check the action log
        let recent = engine.recent_actions(5);
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert!(matches!(last.kind, ActionKind::Alert { .. }));
    }

    #[test]
    fn medium_with_key_adjusts_base_fee() {
        let mut engine = ActionEngine::new(true, false).with_emergency_key(true);
        for i in 0..SUSTAINED_TICKS_MEDIUM {
            engine.evaluate(&make_health(0.6, i as u64 * 100), 100);
        }
        let recent = engine.recent_actions(5);
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert_eq!(last.tier, ConfidenceTier::Medium);
        assert!(matches!(last.kind, ActionKind::AdjustBaseFee { .. }));
        if let ActionKind::AdjustBaseFee {
            new_value,
            old_value,
            ..
        } = &last.kind
        {
            assert_eq!(*old_value, 100);
            assert_eq!(*new_value, 105); // +5%
        }
    }

    #[test]
    fn high_score_proposes_governance() {
        let mut engine = ActionEngine::new(true, false);
        for i in 0..SUSTAINED_TICKS_HIGH {
            engine.evaluate(&make_health(0.85, i as u64 * 100), 50);
        }
        let recent = engine.recent_actions(5);
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert_eq!(last.tier, ConfidenceTier::High);
        assert!(matches!(last.kind, ActionKind::ProposeGovernance { .. }));
    }

    #[test]
    fn critical_with_key_and_auto_pause() {
        let mut engine = ActionEngine::new(true, true).with_emergency_key(true);
        for i in 0..SUSTAINED_TICKS_CRITICAL {
            engine.evaluate(&make_health(0.98, i as u64 * 100), 1);
        }
        let recent = engine.recent_actions(5);
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert_eq!(last.tier, ConfidenceTier::Critical);
        assert!(matches!(last.kind, ActionKind::EmergencyPause));
        assert!(last.dry_run);
    }

    #[test]
    fn critical_without_auto_pause_falls_to_alert() {
        let mut engine = ActionEngine::new(true, false).with_emergency_key(true);
        for i in 0..SUSTAINED_TICKS_CRITICAL {
            engine.evaluate(&make_health(0.98, i as u64 * 100), 1);
        }
        let recent = engine.recent_actions(5);
        // Without auto_pause, CRITICAL should not trigger EmergencyPause
        // It falls through to the High branch instead
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert!(!matches!(last.kind, ActionKind::EmergencyPause));
    }

    #[test]
    fn onchain_cooldown_prevents_rapid_fire() {
        let mut engine = ActionEngine::new(true, false).with_emergency_key(true);
        // First MEDIUM action at batch 0
        for i in 0..SUSTAINED_TICKS_MEDIUM {
            engine.evaluate(&make_health(0.6, i as u64 * 100), 100);
        }
        let first_count = engine.action_log().len();
        assert!(first_count > 0);

        // Second attempt too soon (within 1000-batch cooldown)
        for i in 0..SUSTAINED_TICKS_MEDIUM {
            engine.evaluate(&make_health(0.6, 500 + i as u64 * 100), 100);
        }
        assert_eq!(
            engine.action_log().len(),
            first_count,
            "cooldown should block second action"
        );

        // After cooldown passes
        for i in 0..SUSTAINED_TICKS_MEDIUM {
            engine.evaluate(&make_health(0.6, 1500 + i as u64 * 100), 100);
        }
        assert!(
            engine.action_log().len() > first_count,
            "action should fire after cooldown"
        );
    }

    #[test]
    fn normal_score_resets_sustained_window() {
        let mut engine = ActionEngine::new(true, false);
        // Build up LOW sustained
        engine.evaluate(&make_health(0.35, 1000), 1);
        // Interrupt with normal
        engine.evaluate(&make_health(0.1, 1100), 1);
        // LOW again — should need full sustained count again
        let action = engine.evaluate(&make_health(0.35, 1200), 1);
        assert!(
            action.is_none(),
            "sustained window should have been broken by normal score"
        );
    }

    #[test]
    fn expired_emergency_key_degrades_to_alert() {
        let epoch = Arc::new(AtomicU64::new(100_000)); // past sunset
        let mut engine = ActionEngine::new(true, true)
            .with_emergency_key(true)
            .with_epoch(epoch);
        for i in 0..SUSTAINED_TICKS_CRITICAL {
            engine.evaluate(&make_health(0.98, i as u64 * 100), 1);
        }
        let recent = engine.recent_actions(5);
        assert!(!recent.is_empty());
        let last = &recent[0];
        assert!(matches!(last.kind, ActionKind::Alert { .. }));
        if let ActionKind::Alert { message } = &last.kind {
            assert!(message.contains("expired"));
        }
    }

    #[test]
    fn adjusted_fee_calculation() {
        assert_eq!(adjusted_fee(100, true), 105);
        assert_eq!(adjusted_fee(100, false), 95);
        assert_eq!(adjusted_fee(1, true), 2); // min delta of 1
        assert_eq!(adjusted_fee(1, false), 1); // can't go below 1
        assert_eq!(adjusted_fee(0, true), 1); // 0 * 5% = 0, but min delta 1
    }

    #[test]
    fn action_log_bounded() {
        let mut engine = ActionEngine::new(true, false);
        // Fill up the log beyond capacity
        for i in 0..(ACTION_LOG_CAPACITY as u64 + 20) {
            // Space actions apart to avoid cooldown
            let batch = i * (ALERT_COOLDOWN_BATCHES + 10);
            // Need sustained ticks each time
            for j in 0..SUSTAINED_TICKS_LOW {
                engine.evaluate(&make_health(0.35, batch + j as u64), 1);
            }
        }
        assert!(
            engine.action_log().len() <= ACTION_LOG_CAPACITY,
            "action log should be bounded: {}",
            engine.action_log().len()
        );
    }

    #[test]
    fn state_push_action_stored() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = SentinelState::new(true);
            state
                .push_action(SentinelAction {
                    tier: ConfidenceTier::Low,
                    kind: ActionKind::Alert {
                        message: "test alert".into(),
                    },
                    score: 0.4,
                    batch_height: 500,
                    timestamp_ms: 0,
                    dry_run: true,
                })
                .await;
            let actions = state.recent_actions(10).await;
            assert_eq!(actions.len(), 1);
            assert_eq!(actions[0].tier, ConfidenceTier::Low);
        });
    }
}
