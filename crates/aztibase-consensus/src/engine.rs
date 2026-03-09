use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::{debug, info, warn};

use aztibase_core::{BlockHash, ValidatorId};

use crate::commit::{CommitConfig, CommitRule, LeaderStatus};
use crate::dag::DagBlock;
use crate::dag_store::DagStore;
use crate::validator::ValidatorSet;
use crate::wire;

const MAX_BUFFERED_VERTICES: usize = 64;
const EQUIVOCATION_PRUNE_DEPTH: u64 = 20;

/// Lightweight consensus metrics, updated atomically by the engine.
#[derive(Debug, Default)]
pub struct ConsensusMetrics {
    pub vertices_proposed: AtomicU64,
    pub vertices_received: AtomicU64,
    pub commits: AtomicU64,
    pub rounds_advanced: AtomicU64,
    pub equivocations: AtomicU64,
    pub last_commit_latency_us: AtomicU64,
}

impl ConsensusMetrics {
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            vertices_proposed: self.vertices_proposed.load(AtomicOrdering::Relaxed),
            vertices_received: self.vertices_received.load(AtomicOrdering::Relaxed),
            commits: self.commits.load(AtomicOrdering::Relaxed),
            rounds_advanced: self.rounds_advanced.load(AtomicOrdering::Relaxed),
            equivocations: self.equivocations.load(AtomicOrdering::Relaxed),
            last_commit_latency_us: self.last_commit_latency_us.load(AtomicOrdering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub vertices_proposed: u64,
    pub vertices_received: u64,
    pub commits: u64,
    pub rounds_advanced: u64,
    pub equivocations: u64,
    pub last_commit_latency_us: u64,
}

#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    pub round_duration: Duration,
    pub wave_length: u64,
    pub max_parents: usize,
    pub max_pending_txs: usize,
    pub archive: bool,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            round_duration: Duration::from_millis(400),
            wave_length: 4,
            max_parents: 20,
            max_pending_txs: 4096,
            archive: false,
        }
    }
}

/// Tracks the state of the current consensus round.
pub struct RoundState {
    pub current_round: u64,
    vertices_by_round: HashMap<u64, Vec<BlockHash>>,
    committed: HashSet<BlockHash>,
    last_committed_wave: Option<u64>,
    prune_horizon: u64,
}

impl Default for RoundState {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundState {
    pub fn new() -> Self {
        Self {
            current_round: 0,
            vertices_by_round: HashMap::new(),
            committed: HashSet::new(),
            last_committed_wave: None,
            prune_horizon: 0,
        }
    }

    pub fn record_vertex(&mut self, round: u64, hash: BlockHash) {
        self.vertices_by_round.entry(round).or_default().push(hash);
    }

    pub fn vertices_at_round(&self, round: u64) -> &[BlockHash] {
        self.vertices_by_round
            .get(&round)
            .map_or(&[], |v| v.as_slice())
    }

    /// Select parents from the previous round. Takes up to `max_parents` hashes.
    pub fn select_parents(&self, max_parents: usize) -> Vec<BlockHash> {
        if self.current_round == 0 {
            return Vec::new();
        }
        let prev = self.vertices_at_round(self.current_round - 1);
        prev.iter().take(max_parents).copied().collect()
    }

    pub fn record_commit(&mut self, hash: BlockHash) {
        self.committed.insert(hash);
    }

    pub fn committed_blocks(&self) -> &HashSet<BlockHash> {
        &self.committed
    }

    /// Remove round entries older than `committed_round` to bound memory usage.
    /// Keeps a 2-round buffer below the committed round for parent lookups.
    pub fn prune_before(&mut self, committed_round: u64) {
        let safe = committed_round.saturating_sub(2);
        if safe <= self.prune_horizon {
            return;
        }
        for round in self.prune_horizon..safe {
            self.vertices_by_round.remove(&round);
        }
        self.prune_horizon = safe;
    }

    pub fn tracked_rounds(&self) -> usize {
        self.vertices_by_round.len()
    }
}

/// Messages flowing into the consensus engine.
#[derive(Debug)]
pub enum ConsensusInput {
    /// A vertex received from the network (serialized DagBlock).
    ReceivedVertex(Vec<u8>),
    /// A transaction to include in the next vertex payload.
    Transaction(Vec<u8>),
}

/// Messages flowing out of the consensus engine.
#[derive(Debug)]
pub enum ConsensusOutput {
    /// A vertex to broadcast via gossipsub.
    BroadcastVertex(Vec<u8>),
    /// A batch of transactions was committed via DAG consensus.
    BatchCommitted(crate::ordering::CommittedBatch),
}

/// State root announcement broadcast to peers after executing a committed batch.
/// Peers compare this against their own execution result and log divergence.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StateRootAnnounce {
    pub anchor_hash: aztibase_core::BlockHash,
    pub state_root: [u8; 32],
    pub batch_index: u64,
    pub validator: aztibase_core::ValidatorId,
}

/// A decoded vertex waiting for its parents to arrive.
struct BufferedVertex {
    block: DagBlock,
    missing_parents: Vec<BlockHash>,
}

pub struct ConsensusEngine {
    config: ConsensusConfig,
    identity: ValidatorId,
    dag: DagStore,
    validators: ValidatorSet,
    pub state: RoundState,
    pending_txs: Vec<Vec<u8>>,
    vrf_seed: [u8; 32],
    inbox: mpsc::Receiver<ConsensusInput>,
    outbox: mpsc::Sender<ConsensusOutput>,
    seen_authors: HashMap<(u64, ValidatorId), BlockHash>,
    buffered: VecDeque<BufferedVertex>,
    equivocations_detected: u64,
    metrics: Arc<ConsensusMetrics>,
    round_start: Instant,
}

impl ConsensusEngine {
    pub fn new(
        config: ConsensusConfig,
        identity: ValidatorId,
        dag: DagStore,
        validators: ValidatorSet,
        inbox: mpsc::Receiver<ConsensusInput>,
        outbox: mpsc::Sender<ConsensusOutput>,
    ) -> Self {
        Self {
            config,
            identity,
            dag,
            validators,
            state: RoundState::new(),
            pending_txs: Vec::new(),
            vrf_seed: [0u8; 32],
            inbox,
            outbox,
            seen_authors: HashMap::new(),
            buffered: VecDeque::new(),
            equivocations_detected: 0,
            metrics: Arc::new(ConsensusMetrics::default()),
            round_start: Instant::now(),
        }
    }

    pub fn metrics(&self) -> Arc<ConsensusMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Run the consensus loop. Advances rounds on a timer, processes incoming
    /// vertices, proposes new vertices, and evaluates commit rules.
    pub async fn run(&mut self) -> Result<()> {
        self.insert_genesis()?;

        let mut ticker = tokio::time::interval(self.config.round_duration);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

        // Skip the first immediate tick
        ticker.tick().await;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.advance_round()?;
                }
                msg = self.inbox.recv() => {
                    match msg {
                        Some(input) => self.handle_input(input)?,
                        None => {
                            info!("Consensus inbox closed, shutting down");
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn insert_genesis(&mut self) -> Result<()> {
        if self.dag.is_empty() {
            for (id, _) in self.validators.iter() {
                let genesis = DagBlock::genesis(*id, now_ms());
                let hash = genesis.hash;
                self.dag
                    .insert(genesis)
                    .context("Failed to insert genesis")?;
                self.state.record_vertex(0, hash);
            }
            info!(
                validators = self.validators.len(),
                "Genesis blocks inserted"
            );
        } else {
            // Rebuild state from existing DAG
            let highest = self.dag.highest_round().unwrap_or(0);
            for round in 0..=highest {
                for &hash in self.dag.blocks_at_round(round) {
                    self.state.record_vertex(round, hash);
                }
            }
            self.state.current_round = highest + 1;
            info!(
                round = self.state.current_round,
                "Resumed from existing DAG"
            );
        }
        Ok(())
    }

    fn advance_round(&mut self) -> Result<()> {
        let round = self.state.current_round;
        debug!(round, "Advancing round");

        self.round_start = Instant::now();
        self.propose_vertex()?;
        self.evaluate_commits()?;
        self.prune_equivocation_tracker();
        self.metrics
            .rounds_advanced
            .fetch_add(1, AtomicOrdering::Relaxed);

        self.state.current_round += 1;
        Ok(())
    }

    fn propose_vertex(&mut self) -> Result<()> {
        let round = self.state.current_round;
        let parents = self.state.select_parents(self.config.max_parents);

        // Round 0 is genesis — already inserted
        if round == 0 {
            return Ok(());
        }

        if parents.is_empty() {
            warn!(round, "No parents available, skipping proposal");
            return Ok(());
        }

        let quorum = self.validators.quorum_count();
        if parents.len() < quorum && round > 3 {
            warn!(
                round,
                parents = parents.len(),
                quorum,
                "Fewer parents than quorum threshold"
            );
        }

        let payload = self.drain_pending_txs();
        let block = DagBlock::new(round, self.identity, parents, payload, now_ms())
            .context("Failed to create vertex")?;

        let hash = block.hash;
        let encoded = wire::encode_vertex(&block).context("Failed to encode vertex")?;

        self.seen_authors.insert((round, self.identity), hash);
        self.dag
            .insert(block)
            .context("Failed to insert own vertex")?;
        self.state.record_vertex(round, hash);

        debug!(round, hash = %short_hex(&hash), "Proposed vertex");
        self.metrics
            .vertices_proposed
            .fetch_add(1, AtomicOrdering::Relaxed);

        let _ = self
            .outbox
            .try_send(ConsensusOutput::BroadcastVertex(encoded));

        Ok(())
    }

    pub fn handle_input(&mut self, input: ConsensusInput) -> Result<()> {
        match input {
            ConsensusInput::ReceivedVertex(data) => {
                self.handle_received_vertex(&data)?;
            }
            ConsensusInput::Transaction(tx) => {
                if self.pending_txs.len() < self.config.max_pending_txs {
                    self.pending_txs.push(tx);
                } else {
                    debug!("Pending tx queue full, dropping transaction");
                }
            }
        }
        Ok(())
    }

    pub fn handle_received_vertex(&mut self, data: &[u8]) -> Result<()> {
        let block = match wire::decode_vertex(data, &self.validators, self.state.current_round) {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "Rejected incoming vertex");
                return Ok(());
            }
        };

        if self.dag.contains(&block.hash) {
            return Ok(());
        }

        if self.is_equivocation(&block) {
            return Ok(());
        }

        self.metrics
            .vertices_received
            .fetch_add(1, AtomicOrdering::Relaxed);
        self.try_insert_vertex(block)
    }

    fn is_equivocation(&mut self, block: &DagBlock) -> bool {
        let key = (block.round, block.author);
        if let Some(&existing) = self.seen_authors.get(&key)
            && existing != block.hash
        {
            self.equivocations_detected += 1;
            self.metrics
                .equivocations
                .fetch_add(1, AtomicOrdering::Relaxed);
            warn!(
                round = block.round,
                author = %short_hex(&block.author),
                existing = %short_hex(&existing),
                duplicate = %short_hex(&block.hash),
                total = self.equivocations_detected,
                "Equivocation detected — dropping vertex"
            );
            return true;
        }
        self.seen_authors.insert(key, block.hash);
        false
    }

    fn try_insert_vertex(&mut self, block: DagBlock) -> Result<()> {
        let round = block.round;
        let hash = block.hash;

        let missing: Vec<BlockHash> = block
            .parents
            .iter()
            .filter(|p| !self.dag.contains(p))
            .copied()
            .collect();

        if !missing.is_empty() && !block.is_genesis() {
            if self.buffered.len() < MAX_BUFFERED_VERTICES {
                debug!(
                    round,
                    hash = %short_hex(&hash),
                    missing_count = missing.len(),
                    "Buffered vertex with missing parents"
                );
                self.buffered.push_back(BufferedVertex {
                    block,
                    missing_parents: missing,
                });
            }
            return Ok(());
        }

        match self.dag.insert(block) {
            Ok(()) => {
                self.state.record_vertex(round, hash);
                debug!(round, hash = %short_hex(&hash), "Accepted vertex from peer");
                self.drain_buffered();
            }
            Err(e) => {
                debug!(error = %e, "Rejected vertex");
            }
        }

        Ok(())
    }

    fn evaluate_commits(&mut self) -> Result<()> {
        let round = self.state.current_round;
        let wave_len = self.config.wave_length;

        if round < wave_len {
            return Ok(());
        }

        let current_wave = round / wave_len;
        let start_wave = self.state.last_committed_wave.map_or(0, |w| w + 1);

        if start_wave > current_wave {
            return Ok(());
        }

        let commit_config = CommitConfig {
            wave_length: wave_len,
            vrf_seed: Some(self.vrf_seed),
        };

        let mut last_prune_round = None;

        {
            let rule = CommitRule::new(&self.dag, &self.validators, commit_config);

            for wave in start_wave..current_wave {
                let status = rule.try_direct_commit(wave);
                match status {
                    LeaderStatus::Commit(hash) => {
                        let latency = self.round_start.elapsed();
                        info!(wave, hash = %short_hex(&hash), "Block committed (direct)");
                        self.metrics.commits.fetch_add(1, AtomicOrdering::Relaxed);
                        self.metrics
                            .last_commit_latency_us
                            .store(latency.as_micros() as u64, AtomicOrdering::Relaxed);
                        self.state.record_commit(hash);
                        self.state.last_committed_wave = Some(wave);
                        if !self.config.archive {
                            self.state.prune_before(wave * wave_len);
                            last_prune_round = Some(wave * wave_len);
                        }
                        self.vrf_seed = aztibase_core::hash(&hash);
                        match crate::ordering::extract_committed_batch(
                            &self.dag,
                            hash,
                            self.state.committed_blocks(),
                        ) {
                            Ok(batch) => {
                                if let Err(e) =
                                    self.outbox.try_send(ConsensusOutput::BatchCommitted(batch))
                                {
                                    tracing::error!(
                                        "Failed to send committed batch to execution: {e} — batch may be lost"
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to extract committed batch: {e}");
                            }
                        }
                    }
                    LeaderStatus::Skip(r) => {
                        debug!(wave, round = r, "Leader skipped");
                        self.state.last_committed_wave = Some(wave);
                    }
                    LeaderStatus::Undecided(_) => {
                        break;
                    }
                }
            }
        }

        // Prune DAG after releasing the CommitRule borrow (skip in archive mode)
        if !self.config.archive
            && let Some(prune_round) = last_prune_round
            && let Err(e) = self.dag.prune_before(prune_round)
        {
            tracing::warn!("DAG prune failed: {e}");
        }

        Ok(())
    }

    pub fn equivocations_detected(&self) -> u64 {
        self.equivocations_detected
    }

    pub fn buffered_count(&self) -> usize {
        self.buffered.len()
    }

    fn drain_buffered(&mut self) {
        let mut made_progress = true;
        while made_progress {
            made_progress = false;
            let mut remaining = VecDeque::new();
            while let Some(mut entry) = self.buffered.pop_front() {
                entry.missing_parents.retain(|p| !self.dag.contains(p));

                if entry.missing_parents.is_empty() {
                    let round = entry.block.round;
                    let hash = entry.block.hash;
                    match self.dag.insert(entry.block) {
                        Ok(()) => {
                            self.state.record_vertex(round, hash);
                            debug!(round, hash = %short_hex(&hash), "Inserted buffered vertex");
                            made_progress = true;
                        }
                        Err(e) => {
                            debug!(error = %e, "Buffered vertex rejected on insert");
                        }
                    }
                } else {
                    remaining.push_back(entry);
                }
            }
            self.buffered = remaining;
        }
    }

    fn prune_equivocation_tracker(&mut self) {
        let cutoff = self
            .state
            .current_round
            .saturating_sub(EQUIVOCATION_PRUNE_DEPTH);
        self.seen_authors.retain(|&(round, _), _| round >= cutoff);
    }

    fn drain_pending_txs(&mut self) -> Vec<u8> {
        if self.pending_txs.is_empty() {
            return Vec::new();
        }
        let mut payload = Vec::new();
        let max_payload = 1024 * 256; // 256KB per vertex
        while let Some(tx) = self.pending_txs.first() {
            if payload.len() + tx.len() + 4 > max_payload {
                break;
            }
            let tx = self.pending_txs.remove(0);
            payload.extend_from_slice(&(tx.len() as u32).to_le_bytes());
            payload.extend_from_slice(&tx);
        }
        payload
    }
}

fn short_hex(hash: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        hash[0], hash[1], hash[2], hash[3]
    )
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_storage::StateStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_engine_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn make_test_engine() -> (
        ConsensusEngine,
        mpsc::Sender<ConsensusInput>,
        mpsc::Receiver<ConsensusOutput>,
        std::path::PathBuf,
    ) {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let dag = DagStore::new(store).unwrap();

        let mut validators = ValidatorSet::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];
        validators.add(v1, 100);
        validators.add(v2, 100);
        validators.add(v3, 100);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(50),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
            archive: false,
        };

        let (in_tx, in_rx) = mpsc::channel(64);
        let (out_tx, out_rx) = mpsc::channel(64);

        let engine = ConsensusEngine::new(config, v1, dag, validators, in_rx, out_tx);

        (engine, in_tx, out_rx, path)
    }

    #[test]
    fn round_state_tracks_vertices() {
        let mut state = RoundState::new();
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        state.record_vertex(0, h1);
        state.record_vertex(0, h2);
        assert_eq!(state.vertices_at_round(0).len(), 2);
        assert_eq!(state.vertices_at_round(1).len(), 0);
    }

    #[test]
    fn round_state_selects_parents() {
        let mut state = RoundState::new();
        state.current_round = 1;
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        state.record_vertex(0, h1);
        state.record_vertex(0, h2);
        let parents = state.select_parents(10);
        assert_eq!(parents.len(), 2);
    }

    #[test]
    fn round_state_no_parents_at_genesis() {
        let state = RoundState::new();
        assert!(state.select_parents(10).is_empty());
    }

    #[test]
    fn round_state_max_parents_capped() {
        let mut state = RoundState::new();
        state.current_round = 1;
        for i in 0..20u8 {
            state.record_vertex(0, [i; 32]);
        }
        let parents = state.select_parents(5);
        assert_eq!(parents.len(), 5);
    }

    #[tokio::test]
    async fn engine_inserts_genesis() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        assert_eq!(engine.dag.len(), 3);
        assert_eq!(engine.state.vertices_at_round(0).len(), 3);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_proposes_vertex() {
        let (mut engine, _in_tx, mut out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        engine.state.current_round = 1;
        engine.propose_vertex().unwrap();

        assert_eq!(engine.dag.len(), 4);
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);

        // Check that vertex was sent to outbox
        let output = out_rx.try_recv().unwrap();
        assert!(matches!(output, ConsensusOutput::BroadcastVertex(_)));
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_handles_received_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_rejects_unknown_validator() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, [99u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 0);
        cleanup(&path);
    }

    #[test]
    fn vertex_serialization_roundtrip() {
        let block = DagBlock::genesis([1u8; 32], 1000);
        let encoded = crate::wire::encode_vertex(&block).unwrap();
        let mut vs = ValidatorSet::new();
        vs.add([1u8; 32], 100);
        let decoded = crate::wire::decode_vertex(&encoded, &vs, 0).unwrap();
        assert_eq!(decoded.hash, block.hash);
        assert_eq!(decoded.round, block.round);
        assert_eq!(decoded.author, block.author);
    }

    #[tokio::test]
    async fn engine_rejects_tampered_hash() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let mut block = DagBlock::new(1, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        block.hash = [0xFFu8; 32];
        let mut data = vec![1u8]; // version byte
        data.extend_from_slice(&postcard::to_allocvec(&block).unwrap());

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 0);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_rejects_future_round_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(50, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(50).len(), 0);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_accepts_near_future_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(5, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        cleanup(&path);
    }

    #[test]
    fn drain_pending_txs_packs_payload() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let dag = DagStore::new(store).unwrap();
        let validators = ValidatorSet::new();
        let (_, in_rx) = mpsc::channel(1);
        let (out_tx, _) = mpsc::channel(1);

        let mut engine = ConsensusEngine::new(
            ConsensusConfig::default(),
            [1u8; 32],
            dag,
            validators,
            in_rx,
            out_tx,
        );

        engine.pending_txs.push(vec![1, 2, 3]);
        engine.pending_txs.push(vec![4, 5]);
        let payload = engine.drain_pending_txs();

        // 4 bytes len + 3 bytes data + 4 bytes len + 2 bytes data = 13
        assert_eq!(payload.len(), 13);
        assert!(engine.pending_txs.is_empty());
        cleanup(&path);
    }

    #[test]
    fn round_pruning_removes_old_rounds() {
        let mut state = RoundState::new();
        for round in 0..20u64 {
            state.record_vertex(round, [round as u8; 32]);
        }
        assert_eq!(state.tracked_rounds(), 20);

        state.prune_before(10);
        // Rounds 0..8 pruned (10 - 2 buffer = 8), rounds 8..19 remain
        assert_eq!(state.vertices_at_round(7).len(), 0);
        assert_eq!(state.vertices_at_round(8).len(), 1);
        assert_eq!(state.vertices_at_round(10).len(), 1);
    }

    #[test]
    fn round_pruning_is_idempotent() {
        let mut state = RoundState::new();
        for round in 0..10u64 {
            state.record_vertex(round, [round as u8; 32]);
        }
        state.prune_before(5);
        let count_after_first = state.tracked_rounds();
        state.prune_before(5);
        assert_eq!(state.tracked_rounds(), count_after_first);
    }

    #[tokio::test]
    async fn equivocation_detection_unit() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();

        // First vertex from v2 at round 1
        let block_a =
            DagBlock::new(1, [2u8; 32], genesis_hashes.clone(), vec![1], now_ms()).unwrap();
        let data_a = crate::wire::encode_vertex(&block_a).unwrap();
        engine.handle_received_vertex(&data_a).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);
        assert_eq!(engine.equivocations_detected(), 0);

        // Second, different vertex from v2 at round 1 (equivocation)
        let block_b = DagBlock::new(1, [2u8; 32], genesis_hashes, vec![2], now_ms()).unwrap();
        let data_b = crate::wire::encode_vertex(&block_b).unwrap();
        engine.handle_received_vertex(&data_b).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1); // not added
        assert_eq!(engine.equivocations_detected(), 1);

        cleanup(&path);
    }

    #[tokio::test]
    async fn buffered_vertex_insertion() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();

        // Create a round-1 vertex from v2
        let block_r1 =
            DagBlock::new(1, [2u8; 32], genesis_hashes.clone(), vec![], now_ms()).unwrap();
        let r1_hash = block_r1.hash;

        // Create a round-2 vertex from v3 that references block_r1
        let block_r2 = DagBlock::new(2, [3u8; 32], vec![r1_hash], vec![], now_ms()).unwrap();
        let data_r2 = crate::wire::encode_vertex(&block_r2).unwrap();

        // Insert r2 first — parent r1 is missing, so it gets buffered
        engine.handle_received_vertex(&data_r2).unwrap();
        assert_eq!(engine.state.vertices_at_round(2).len(), 0);
        assert_eq!(engine.buffered_count(), 1);

        // Now insert r1 — this should trigger drain_buffered and insert r2
        let data_r1 = crate::wire::encode_vertex(&block_r1).unwrap();
        engine.handle_received_vertex(&data_r1).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);
        assert_eq!(engine.state.vertices_at_round(2).len(), 1);
        assert_eq!(engine.buffered_count(), 0);

        cleanup(&path);
    }

    #[test]
    fn pending_txs_cap_enforced() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let dag = DagStore::new(store).unwrap();
        let validators = ValidatorSet::new();
        let (_, in_rx) = mpsc::channel(1);
        let (out_tx, _) = mpsc::channel(1);

        let config = ConsensusConfig {
            max_pending_txs: 3,
            ..ConsensusConfig::default()
        };
        let mut engine = ConsensusEngine::new(config, [1u8; 32], dag, validators, in_rx, out_tx);

        for i in 0..5u8 {
            engine
                .handle_input(ConsensusInput::Transaction(vec![i]))
                .unwrap();
        }
        assert_eq!(engine.pending_txs.len(), 3);
        cleanup(&path);
    }

    #[tokio::test]
    async fn metrics_counter_increments() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        let metrics = engine.metrics();
        engine.insert_genesis().unwrap();

        let snap_before = metrics.snapshot();
        assert_eq!(snap_before.vertices_proposed, 0);
        assert_eq!(snap_before.vertices_received, 0);

        engine.state.current_round = 1;
        engine.propose_vertex().unwrap();
        let snap_after = metrics.snapshot();
        assert_eq!(snap_after.vertices_proposed, 1);

        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();
        engine.handle_received_vertex(&data).unwrap();
        let snap_recv = metrics.snapshot();
        assert_eq!(snap_recv.vertices_received, 1);

        cleanup(&path);
    }
}
