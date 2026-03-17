use std::collections::VecDeque;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

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

const FEATURE_NAMES: [&str; 15] = [
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

/// Scores chain health from a 15-feature vector.
///
/// Uses a deterministic heuristic; upgrading to an ONNX autoencoder
/// via `TractRuntime` is planned for Tier 2 once baseline testnet data
/// is collected.
pub struct ChainHealthScorer {
    prev_batch_height: u64,
    prev_tps: f32,
    prev_base_fee: f32,
    latency_samples: VecDeque<f32>,
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
        }
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

        let score = self.heuristic_score(&features);

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
    enabled: bool,
    rpc_latest: Option<Arc<RwLock<Option<serde_json::Value>>>>,
    rpc_history: Option<Arc<RwLock<Vec<serde_json::Value>>>>,
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
            enabled,
            rpc_latest: None,
            rpc_history: None,
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

/// Runs the Sentinel tick loop.
///
/// Reads shared node state every `interval_batches` batches, scores health,
/// and pushes to `SentinelState` for RPC/WebSocket consumption.
pub async fn run_sentinel(
    state: Arc<SentinelState>,
    handles: SentinelHandles,
    interval_batches: u64,
) {
    let mut scorer = ChainHealthScorer::new();
    let mut last_scored_batch: u64 = 0;
    let mut last_seen_batch: u64 = 0;
    let mut last_progress_time = std::time::Instant::now();
    let mut stall_reported = false;
    const STALL_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(30);

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let current_batch = handles.batch_count.load(Ordering::Relaxed);

        // Track progress against the last *seen* batch, not last scored
        if current_batch > last_seen_batch {
            last_seen_batch = current_batch;
            last_progress_time = std::time::Instant::now();
            stall_reported = false;
        }

        // Detect chain stall — no new batches for 30+ seconds
        let stalled = last_progress_time.elapsed() > STALL_THRESHOLD && current_batch > 0;
        if stalled && !stall_reported {
            stall_reported = true;
            let stall_secs = last_progress_time.elapsed().as_secs();
            let stall_input = SentinelInput {
                batch_height: current_batch,
                commit_latency_us: 0,
                txs_processed: 0,
                base_fee: handles.base_fee.load(Ordering::Relaxed),
                equivocations: 0,
                active_validators: handles.active_validators.load(Ordering::Relaxed),
                total_gas_used: 0,
                gas_limit: 0,
                empty_batches: 0,
                total_batches_window: 0,
                ms_since_last_finality: stall_secs * 1000,
                peer_count: handles.peer_count.load(Ordering::Relaxed),
                mempool_size: handles.mempool_size.load(Ordering::Relaxed),
            };
            let health = scorer.score(&stall_input);
            tracing::warn!(
                score = health.score,
                level = ?health.level,
                stall_secs = stall_secs,
                batch = current_batch,
                "Chain stall detected — no new batches"
            );
            state.push(health).await;
            continue;
        }

        if current_batch < last_scored_batch + interval_batches {
            continue;
        }

        let input = SentinelInput {
            batch_height: current_batch,
            commit_latency_us: handles.commit_latency_us.load(Ordering::Relaxed),
            txs_processed: handles.txs_processed.load(Ordering::Relaxed),
            base_fee: handles.base_fee.load(Ordering::Relaxed),
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

        state.push(health).await;
        last_scored_batch = current_batch;
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
}
