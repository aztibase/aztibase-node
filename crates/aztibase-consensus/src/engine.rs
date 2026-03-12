use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use aztibase_core::{BlockHash, Keypair, ValidatorId};

use crate::commit::{CommitConfig, CommitRule, LeaderStatus};
use crate::dag::DagBlock;
use crate::dag_store::DagStore;
use crate::validator::ValidatorSet;
use crate::wire;

const EQUIVOCATION_PRUNE_DEPTH: u64 = 100;
const MAX_CATCHUP_ROUNDS: usize = 100;

/// Message-driven threshold clock for DAG-BFT round advancement.
///
/// Tracks which validators have produced blocks at the current round.
/// When a quorum (>=2/3 stake) of blocks is seen, the clock advances
/// to the next round, signaling that the validator may propose.
///
/// Follows the Mysticeti/Sui pattern: safety via quorum-gated rounds,
/// liveness via timeout-forced advancement.
struct ThresholdClock {
    round: u64,
    seen: HashMap<u64, HashSet<ValidatorId>>,
}

impl ThresholdClock {
    fn new() -> Self {
        Self {
            round: 0,
            seen: HashMap::new(),
        }
    }

    /// Register a block and try to advance through as many rounds as possible.
    /// Tracks authors per-round so out-of-order arrivals are handled correctly.
    /// Returns `true` if the clock advanced at least one round.
    fn add_block(
        &mut self,
        author: ValidatorId,
        block_round: u64,
        validators: &ValidatorSet,
    ) -> bool {
        if block_round < self.round {
            return false;
        }

        self.seen.entry(block_round).or_default().insert(author);

        let mut advanced = false;
        while let Some(authors) = self.seen.get(&self.round) {
            let ids: Vec<ValidatorId> = authors.iter().copied().collect();
            if validators.has_quorum(&ids) {
                self.seen.remove(&self.round);
                self.round += 1;
                advanced = true;
            } else {
                break;
            }
        }

        // Evict stale entries well behind the current round.
        self.seen.retain(|&r, _| r >= self.round);

        advanced
    }

    fn get_round(&self) -> u64 {
        self.round
    }

    fn force_advance(&mut self) {
        self.seen.remove(&self.round);
        self.round += 1;
    }
}

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

    /// Select parents from recent rounds. Scans a lookback window to handle
    /// round drift between validators communicating over a real network.
    pub fn select_parents(&self, max_parents: usize) -> Vec<BlockHash> {
        if self.current_round == 0 {
            return Vec::new();
        }

        let mut parents = Vec::new();
        let lookback = 16u64.min(self.current_round);
        let start = self.current_round - lookback;

        for round in (start..self.current_round).rev() {
            for &hash in self.vertices_at_round(round) {
                if parents.len() >= max_parents {
                    return parents;
                }
                parents.push(hash);
            }
        }
        parents
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
    /// Replace the validator set (sent from pipeline at epoch boundaries).
    UpdateValidatorSet(crate::validator::ValidatorSet),
    /// Peer count changed (sent from the network layer).
    PeerCountChanged(u64),
}

/// Messages flowing out of the consensus engine.
#[derive(Debug)]
pub enum ConsensusOutput {
    /// A vertex to broadcast via gossipsub.
    BroadcastVertex(Vec<u8>),
    /// A batch of transactions was committed via DAG consensus.
    BatchCommitted(crate::ordering::CommittedBatch),
    /// A validator produced conflicting vertices in the same round.
    EquivocationDetected {
        author: [u8; 32],
        round: u64,
        existing_hash: [u8; 32],
        duplicate_hash: [u8; 32],
    },
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

pub struct ConsensusEngine {
    config: ConsensusConfig,
    identity: ValidatorId,
    signing_key: Keypair,
    dag: DagStore,
    validators: ValidatorSet,
    pub state: RoundState,
    pending_txs: VecDeque<Vec<u8>>,
    vrf_seed: [u8; 32],
    inbox: mpsc::Receiver<ConsensusInput>,
    outbox: mpsc::Sender<ConsensusOutput>,
    seen_authors: HashMap<(u64, ValidatorId), BlockHash>,
    equivocations_detected: u64,
    metrics: Arc<ConsensusMetrics>,
    round_start: Instant,
    threshold_clock: ThresholdClock,
    last_proposed_round: u64,
    peer_count: u64,
}

impl ConsensusEngine {
    pub fn new(
        config: ConsensusConfig,
        identity: ValidatorId,
        signing_key: Keypair,
        dag: DagStore,
        validators: ValidatorSet,
        inbox: mpsc::Receiver<ConsensusInput>,
        outbox: mpsc::Sender<ConsensusOutput>,
    ) -> Self {
        Self {
            config,
            identity,
            signing_key,
            dag,
            validators,
            state: RoundState::new(),
            pending_txs: VecDeque::new(),
            vrf_seed: [0u8; 32],
            inbox,
            outbox,
            seen_authors: HashMap::new(),
            equivocations_detected: 0,
            metrics: Arc::new(ConsensusMetrics::default()),
            round_start: Instant::now(),
            threshold_clock: ThresholdClock::new(),
            last_proposed_round: 0,
            peer_count: 0,
        }
    }

    pub fn metrics(&self) -> Arc<ConsensusMetrics> {
        Arc::clone(&self.metrics)
    }

    /// Run the consensus loop. Uses a message-driven threshold clock:
    /// rounds advance when a quorum of blocks is received, not on a timer.
    /// A liveness timeout forces advancement if the clock stalls.
    pub async fn run(&mut self) -> Result<()> {
        info!(
            identity = %short_hex(&self.identity),
            validators = self.validators.len(),
            total_stake = self.validators.total_stake(),
            is_validator = self.validators.contains(&self.identity),
            "Consensus engine starting"
        );

        self.insert_genesis()?;

        let genesis_count = self.state.vertices_at_round(0).len();
        info!(genesis_blocks = genesis_count, "Genesis blocks in DAG");

        // Seed threshold clock with genesis blocks (round 0).
        let mut seeded = 0usize;
        for &hash in self.state.vertices_at_round(0) {
            if let Ok(block) = self.dag.get(&hash) {
                let advanced = self
                    .threshold_clock
                    .add_block(block.author, 0, &self.validators);
                seeded += 1;
                debug!(
                    author = %short_hex(&block.author),
                    clock = self.threshold_clock.get_round(),
                    advanced,
                    "Seeded threshold clock with genesis block"
                );
            } else {
                warn!(hash = %short_hex(&hash), "Failed to read genesis block from DAG");
            }
        }

        info!(
            seeded,
            clock_round = self.threshold_clock.get_round(),
            last_proposed = self.last_proposed_round,
            "Threshold clock seeded — waiting for peers before first proposal"
        );

        // Wait for at least 1 peer before the first proposal.
        // Without peers, gossipsub silently drops published vertices (dedup cache
        // prevents re-broadcast), leaving other nodes permanently behind.
        let peer_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        while self.peer_count == 0 {
            tokio::select! {
                _ = tokio::time::sleep_until(peer_deadline) => {
                    warn!("No peers after 30s — starting consensus without peers");
                    break;
                }
                msg = self.inbox.recv() => {
                    match msg {
                        Some(input) => {
                            let was_peer_update = matches!(&input, ConsensusInput::PeerCountChanged(_));
                            self.handle_input(input)?;
                            if was_peer_update && self.peer_count > 0 {
                                info!(peer_count = self.peer_count, "First peer connected — starting consensus");
                                break;
                            }
                        }
                        None => return Ok(()),
                    }
                }
            }
        }

        self.try_advance()?;

        let timeout_duration = self.config.round_duration * 25;

        loop {
            tokio::select! {
                _ = tokio::time::sleep(timeout_duration) => {
                    info!(
                        clock = self.threshold_clock.get_round(),
                        proposed = self.last_proposed_round,
                        "Liveness timeout, forcing round advance"
                    );
                    self.threshold_clock.force_advance();
                    self.try_advance()?;
                }
                msg = self.inbox.recv() => {
                    match msg {
                        Some(input) => {
                            let is_vertex = matches!(&input, ConsensusInput::ReceivedVertex(_));
                            self.handle_input(input)?;
                            if is_vertex {
                                self.try_advance()?;
                            }
                        }
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

    /// Propose and evaluate commits for all rounds where the threshold clock
    /// has advanced past our last proposal.
    fn try_advance(&mut self) -> Result<()> {
        let clock = self.threshold_clock.get_round();
        let last = self.last_proposed_round;
        if clock <= last {
            debug!(
                clock,
                last_proposed = last,
                "try_advance: clock not ahead, skipping"
            );
            return Ok(());
        }
        debug!(
            clock,
            last_proposed = last,
            gap = clock - last,
            "try_advance: clock ahead"
        );

        let mut advances = 0;
        while self.threshold_clock.get_round() > self.last_proposed_round
            && advances < MAX_CATCHUP_ROUNDS
        {
            let round = self.last_proposed_round + 1;
            self.state.current_round = round;
            self.round_start = Instant::now();
            self.propose_vertex()?;
            self.evaluate_commits()?;
            self.prune_equivocation_tracker();
            self.metrics
                .rounds_advanced
                .fetch_add(1, AtomicOrdering::Relaxed);
            self.last_proposed_round = round;
            self.state.current_round = round + 1;
            advances += 1;
        }
        Ok(())
    }

    /// RANDAO-style VRF seed accumulation (S2-2).
    /// Mixes the previous seed with the anchor hash AND all vertex hashes
    /// in the committed batch's causal history. This prevents a single
    /// last-revealer from biasing the next seed by withholding their anchor,
    /// since randomness from multiple validators is already mixed in.
    fn accumulate_vrf_seed(
        prev_seed: &[u8; 32],
        anchor_hash: &BlockHash,
        dag: &DagStore,
        committed: &HashSet<BlockHash>,
    ) -> [u8; 32] {
        let mut buf = Vec::with_capacity(64 + 32 * 4);
        buf.extend_from_slice(prev_seed);
        buf.extend_from_slice(anchor_hash);

        if let Ok(causal) = dag.causal_order(&[*anchor_hash]) {
            for h in &causal {
                if !committed.contains(h) {
                    buf.extend_from_slice(h);
                }
            }
        }

        aztibase_core::hash(&buf)
    }

    pub fn insert_genesis(&mut self) -> Result<()> {
        if self.dag.is_empty() {
            for (id, _) in self.validators.iter() {
                let genesis = DagBlock::genesis(*id, 0);
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

        let pending_count = self.pending_txs.len();
        let payload = self.drain_pending_txs();
        let block = DagBlock::new(
            round,
            self.identity,
            parents,
            payload.clone(),
            now_ms(),
            Some(&self.signing_key),
        )
        .context("Failed to create vertex")?;

        let hash = block.hash;
        let encoded = wire::encode_vertex(&block).context("Failed to encode vertex")?;

        self.seen_authors.insert((round, self.identity), hash);
        self.dag
            .insert(block)
            .context("Failed to insert own vertex")?;
        self.state.record_vertex(round, hash);

        if pending_count > 0 {
            info!(round, hash = %short_hex(&hash), txs = pending_count, payload_bytes = payload.len(), "Proposed vertex WITH transactions");
        } else {
            debug!(round, hash = %short_hex(&hash), "Proposed vertex");
        }
        self.metrics
            .vertices_proposed
            .fetch_add(1, AtomicOrdering::Relaxed);

        match self
            .outbox
            .try_send(ConsensusOutput::BroadcastVertex(encoded))
        {
            Ok(()) => {
                debug!(round, hash = %short_hex(&hash), "Vertex broadcast queued");
            }
            Err(e) => {
                warn!(round, error = %e, "Failed to queue vertex for broadcast");
            }
        }

        Ok(())
    }

    pub fn handle_input(&mut self, input: ConsensusInput) -> Result<()> {
        match input {
            ConsensusInput::ReceivedVertex(data) => {
                self.handle_received_vertex(&data)?;
            }
            ConsensusInput::Transaction(tx) => {
                if self.pending_txs.len() < self.config.max_pending_txs {
                    info!(
                        tx_len = tx.len(),
                        pending = self.pending_txs.len() + 1,
                        "Transaction added to consensus pending queue"
                    );
                    self.pending_txs.push_back(tx);
                } else {
                    warn!("Pending tx queue full, dropping transaction");
                }
            }
            ConsensusInput::UpdateValidatorSet(new_set) => {
                info!(
                    validators = new_set.len(),
                    "Validator set updated from staking epoch"
                );
                self.validators = new_set;
            }
            ConsensusInput::PeerCountChanged(count) => {
                self.peer_count = count;
                debug!(peer_count = count, "Peer count updated in consensus engine");
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
            let _ = self.outbox.try_send(ConsensusOutput::EquivocationDetected {
                author: block.author,
                round: block.round,
                existing_hash: existing,
                duplicate_hash: block.hash,
            });
            return true;
        }
        self.seen_authors.insert(key, block.hash);
        false
    }

    fn try_insert_vertex(&mut self, block: DagBlock) -> Result<()> {
        let round = block.round;
        let hash = block.hash;
        let author = block.author;

        // Use relaxed insert: accept blocks even when parents are missing.
        // In a DAG with asynchronous delivery, parents may arrive out of order.
        // The commit rule only checks block.parents references, not parent existence.
        match self.dag.insert_relaxed(block) {
            Ok(()) => {
                self.state.record_vertex(round, hash);
                self.threshold_clock
                    .add_block(author, round, &self.validators);
                debug!(round, hash = %short_hex(&hash), "Accepted vertex from peer");
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
                        self.state.last_committed_wave = Some(wave);
                        if !self.config.archive {
                            self.state.prune_before(wave * wave_len);
                            last_prune_round = Some(wave * wave_len);
                        }
                        self.vrf_seed = Self::accumulate_vrf_seed(
                            &self.vrf_seed,
                            &hash,
                            &self.dag,
                            self.state.committed_blocks(),
                        );
                        match crate::ordering::extract_committed_batch(
                            &self.dag,
                            hash,
                            self.state.committed_blocks(),
                        ) {
                            Ok(batch) => {
                                for vh in &batch.vertex_order {
                                    self.state.record_commit(*vh);
                                }
                                let mut pending = Some(ConsensusOutput::BatchCommitted(batch));
                                for attempt in 0..100 {
                                    let m = pending.take().expect("retry invariant");
                                    match self.outbox.try_send(m) {
                                        Ok(()) => break,
                                        Err(mpsc::error::TrySendError::Full(returned)) => {
                                            if attempt == 0 {
                                                tracing::warn!(
                                                    "Outbox full, retrying committed batch delivery"
                                                );
                                            }
                                            pending = Some(returned);
                                            std::thread::sleep(Duration::from_millis(10));
                                        }
                                        Err(mpsc::error::TrySendError::Closed(_)) => {
                                            tracing::error!(
                                                "Outbox closed, cannot deliver committed batch"
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                self.state.record_commit(hash);
                                tracing::warn!("Failed to extract committed batch: {e}");
                            }
                        }
                    }
                    LeaderStatus::Skip(r) => {
                        debug!(wave, round = r, "Leader skipped");
                        self.state.last_committed_wave = Some(wave);
                    }
                    LeaderStatus::Undecided(r) => {
                        let voting_round = r + 1;
                        if round > voting_round + wave_len * 8 {
                            debug!(wave, round = r, "Force-skipping stale undecided wave");
                            self.state.last_committed_wave = Some(wave);
                        } else {
                            break;
                        }
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
        while let Some(tx) = self.pending_txs.front() {
            if payload.len() + tx.len() + 4 > max_payload {
                break;
            }
            let tx = self.pending_txs.pop_front().unwrap();
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

    fn test_keypair(seed: u8) -> aztibase_core::Keypair {
        aztibase_core::Keypair::from_secret_bytes(&[seed; 32])
    }

    fn test_validator_id(seed: u8) -> [u8; 32] {
        *test_keypair(seed).public_key().as_bytes()
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

        let kp1 = test_keypair(1);
        let kp2 = test_keypair(2);
        let kp3 = test_keypair(3);
        let v1 = *kp1.public_key().as_bytes();
        let v2 = *kp2.public_key().as_bytes();
        let v3 = *kp3.public_key().as_bytes();

        let mut validators = ValidatorSet::new();
        validators.add(v1, 100);
        validators.add(v2, 100);
        validators.add(v3, 100);
        validators.set_ed25519_key(&v1, v1);
        validators.set_ed25519_key(&v2, v2);
        validators.set_ed25519_key(&v3, v3);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(50),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
            archive: false,
        };

        let (in_tx, in_rx) = mpsc::channel(64);
        let (out_tx, out_rx) = mpsc::channel(64);

        let engine = ConsensusEngine::new(config, v1, kp1, dag, validators, in_rx, out_tx);

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

        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, v2, genesis_hashes, vec![], now_ms(), Some(&kp2)).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_rejects_unknown_validator() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let kp99 = test_keypair(99);
        let v99 = *kp99.public_key().as_bytes();
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, v99, genesis_hashes, vec![], now_ms(), Some(&kp99)).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 0);
        cleanup(&path);
    }

    #[test]
    fn vertex_serialization_roundtrip() {
        let v1 = test_validator_id(1);
        let block = DagBlock::genesis(v1, 1000);
        let encoded = crate::wire::encode_vertex(&block).unwrap();
        let mut vs = ValidatorSet::new();
        vs.add(v1, 100);
        let decoded = crate::wire::decode_vertex(&encoded, &vs, 0).unwrap();
        assert_eq!(decoded.hash, block.hash);
        assert_eq!(decoded.round, block.round);
        assert_eq!(decoded.author, block.author);
    }

    #[tokio::test]
    async fn engine_rejects_tampered_hash() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let mut block = DagBlock::new(1, v2, genesis_hashes, vec![], now_ms(), Some(&kp2)).unwrap();
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
        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(150, v2, genesis_hashes, vec![], now_ms(), Some(&kp2)).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(150).len(), 0);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_accepts_near_future_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(5, v2, genesis_hashes, vec![], now_ms(), Some(&kp2)).unwrap();
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
            aztibase_core::Keypair::from_secret_bytes(&[1u8; 32]),
            dag,
            validators,
            in_rx,
            out_tx,
        );

        engine.pending_txs.push_back(vec![1, 2, 3]);
        engine.pending_txs.push_back(vec![4, 5]);
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

        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();

        // First vertex from v2 at round 1
        let block_a =
            DagBlock::new(1, v2, genesis_hashes.clone(), vec![1], now_ms(), Some(&kp2)).unwrap();
        let data_a = crate::wire::encode_vertex(&block_a).unwrap();
        engine.handle_received_vertex(&data_a).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);
        assert_eq!(engine.equivocations_detected(), 0);

        // Second, different vertex from v2 at round 1 (equivocation)
        let block_b = DagBlock::new(1, v2, genesis_hashes, vec![2], now_ms(), Some(&kp2)).unwrap();
        let data_b = crate::wire::encode_vertex(&block_b).unwrap();
        engine.handle_received_vertex(&data_b).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1); // not added
        assert_eq!(engine.equivocations_detected(), 1);

        cleanup(&path);
    }

    #[tokio::test]
    async fn relaxed_insert_out_of_order() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let kp2 = test_keypair(2);
        let kp3 = test_keypair(3);
        let v2 = test_validator_id(2);
        let v3 = test_validator_id(3);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();

        let block_r1 =
            DagBlock::new(1, v2, genesis_hashes.clone(), vec![], now_ms(), Some(&kp2)).unwrap();
        let r1_hash = block_r1.hash;

        // Round-2 vertex referencing round-1 (which doesn't exist yet in DAG).
        let block_r2 = DagBlock::new(2, v3, vec![r1_hash], vec![], now_ms(), Some(&kp3)).unwrap();
        let data_r2 = crate::wire::encode_vertex(&block_r2).unwrap();

        // Relaxed insert: r2 goes in even though parent r1 is missing.
        engine.handle_received_vertex(&data_r2).unwrap();
        assert_eq!(engine.state.vertices_at_round(2).len(), 1);

        // Now insert r1 — also accepted.
        let data_r1 = crate::wire::encode_vertex(&block_r1).unwrap();
        engine.handle_received_vertex(&data_r1).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 1);

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
        let mut engine = ConsensusEngine::new(
            config,
            [1u8; 32],
            aztibase_core::Keypair::from_secret_bytes(&[1u8; 32]),
            dag,
            validators,
            in_rx,
            out_tx,
        );

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

        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(1, v2, genesis_hashes, vec![], now_ms(), Some(&kp2)).unwrap();
        let data = crate::wire::encode_vertex(&block).unwrap();
        engine.handle_received_vertex(&data).unwrap();
        let snap_recv = metrics.snapshot();
        assert_eq!(snap_recv.vertices_received, 1);

        cleanup(&path);
    }

    #[tokio::test]
    async fn equivocation_emits_output() {
        let (mut engine, _in_tx, mut out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();

        let kp2 = test_keypair(2);
        let v2 = test_validator_id(2);
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();

        let block_a =
            DagBlock::new(1, v2, genesis_hashes.clone(), vec![1], now_ms(), Some(&kp2)).unwrap();
        let data_a = crate::wire::encode_vertex(&block_a).unwrap();
        engine.handle_received_vertex(&data_a).unwrap();

        // Drain the BroadcastVertex if any
        while out_rx.try_recv().is_ok() {}

        let block_b = DagBlock::new(1, v2, genesis_hashes, vec![2], now_ms(), Some(&kp2)).unwrap();
        let data_b = crate::wire::encode_vertex(&block_b).unwrap();
        engine.handle_received_vertex(&data_b).unwrap();

        let output = out_rx.try_recv().unwrap();
        match output {
            ConsensusOutput::EquivocationDetected {
                author,
                round,
                existing_hash,
                duplicate_hash,
            } => {
                assert_eq!(author, v2);
                assert_eq!(round, 1);
                assert_ne!(existing_hash, duplicate_hash);
            }
            other => panic!("Expected EquivocationDetected, got {other:?}"),
        }
        cleanup(&path);
    }

    #[tokio::test]
    async fn update_validator_set_replaces_validators() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        assert_eq!(engine.validators.len(), 3);

        let mut new_set = ValidatorSet::new();
        new_set.add([10u8; 32], 500);
        new_set.add([20u8; 32], 500);
        engine
            .handle_input(ConsensusInput::UpdateValidatorSet(new_set))
            .unwrap();

        assert_eq!(engine.validators.len(), 2);
        assert!(engine.validators.contains(&[10u8; 32]));
        assert!(!engine.validators.contains(&[1u8; 32]));
        cleanup(&path);
    }

    #[tokio::test]
    async fn run_produces_vertices_via_timeout() {
        let (engine, in_tx, mut out_rx, path) = make_test_engine();
        let metrics = engine.metrics();

        let handle = tokio::spawn(async move {
            let mut engine = engine;
            engine.run().await.unwrap();
        });

        // Simulate a peer connecting so the engine starts proposing.
        in_tx
            .send(ConsensusInput::PeerCountChanged(1))
            .await
            .unwrap();

        // Wait for at least one BroadcastVertex from genesis advance + one timeout
        let mut vertex_count = 0u32;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(200), out_rx.recv()).await {
                Ok(Some(ConsensusOutput::BroadcastVertex(_))) => {
                    vertex_count += 1;
                    if vertex_count >= 2 {
                        break;
                    }
                }
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(_) => {}
            }
        }

        assert!(
            vertex_count >= 1,
            "Expected at least 1 vertex from run(), got {vertex_count}"
        );

        let snap = metrics.snapshot();
        assert!(
            snap.vertices_proposed >= 1,
            "Expected vertices_proposed >= 1, got {}",
            snap.vertices_proposed
        );
        assert!(
            snap.rounds_advanced >= 1,
            "Expected rounds_advanced >= 1, got {}",
            snap.rounds_advanced
        );

        handle.abort();
        cleanup(&path);
    }

    #[test]
    fn vrf_seed_accumulates_from_multiple_hashes() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let g1 = DagBlock::genesis([1u8; 32], 1000);
        let g2 = DagBlock::genesis([2u8; 32], 1000);
        let g1h = g1.hash;
        let g2h = g2.hash;
        dag.insert(g1).unwrap();
        dag.insert(g2).unwrap();

        let child = DagBlock::new(1, [1u8; 32], vec![g1h, g2h], vec![], 2000, None).unwrap();
        let ch = child.hash;
        dag.insert(child).unwrap();

        let prev_seed = [42u8; 32];
        let committed = HashSet::new();

        let seed = ConsensusEngine::accumulate_vrf_seed(&prev_seed, &ch, &dag, &committed);

        // Seed is not just hash(anchor) — it incorporates causal history
        let naive_seed = aztibase_core::hash(&ch);
        assert_ne!(seed, naive_seed);

        // Deterministic
        let seed2 = ConsensusEngine::accumulate_vrf_seed(&prev_seed, &ch, &dag, &committed);
        assert_eq!(seed, seed2);

        // Different prev_seed → different output
        let seed3 = ConsensusEngine::accumulate_vrf_seed(&[99u8; 32], &ch, &dag, &committed);
        assert_ne!(seed, seed3);

        cleanup(&path);
    }
}
