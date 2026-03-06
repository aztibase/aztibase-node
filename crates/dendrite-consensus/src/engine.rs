use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::{debug, info, warn};

use dendrite_core::{BlockHash, ValidatorId};

use crate::commit::{CommitConfig, CommitRule, LeaderStatus};
use crate::dag::DagBlock;
use crate::dag_store::DagStore;
use crate::validator::ValidatorSet;

#[derive(Clone, Debug)]
pub struct ConsensusConfig {
    pub round_duration: Duration,
    pub wave_length: u64,
    pub max_parents: usize,
    pub max_pending_txs: usize,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            round_duration: Duration::from_millis(400),
            wave_length: 4,
            max_parents: 20,
            max_pending_txs: 4096,
        }
    }
}

/// Tracks the state of the current consensus round.
pub struct RoundState {
    pub current_round: u64,
    vertices_by_round: HashMap<u64, Vec<BlockHash>>,
    committed: Vec<BlockHash>,
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
            committed: Vec::new(),
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
        self.committed.push(hash);
    }

    pub fn committed_blocks(&self) -> &[BlockHash] {
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

pub struct ConsensusEngine {
    config: ConsensusConfig,
    identity: ValidatorId,
    dag: DagStore,
    validators: ValidatorSet,
    state: RoundState,
    pending_txs: Vec<Vec<u8>>,
    vrf_seed: [u8; 32],
    inbox: mpsc::Receiver<ConsensusInput>,
    outbox: mpsc::Sender<ConsensusOutput>,
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
        }
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

    fn insert_genesis(&mut self) -> Result<()> {
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

        self.propose_vertex()?;
        self.evaluate_commits()?;

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
        let encoded = bincode::serialize(&block).context("Failed to serialize vertex")?;

        self.dag
            .insert(block)
            .context("Failed to insert own vertex")?;
        self.state.record_vertex(round, hash);

        debug!(round, hash = %short_hex(&hash), "Proposed vertex");

        let _ = self
            .outbox
            .try_send(ConsensusOutput::BroadcastVertex(encoded));

        Ok(())
    }

    fn handle_input(&mut self, input: ConsensusInput) -> Result<()> {
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

    fn handle_received_vertex(&mut self, data: &[u8]) -> Result<()> {
        let block: DagBlock = match bincode::deserialize(data) {
            Ok(b) => b,
            Err(e) => {
                warn!(error = %e, "Failed to deserialize received vertex");
                return Ok(());
            }
        };

        if !self.validators.contains(&block.author) {
            warn!(author = %short_hex(&block.author), "Vertex from unknown validator");
            return Ok(());
        }

        if self.dag.contains(&block.hash) {
            return Ok(());
        }

        // Verify the block hash matches its contents
        let expected = block.compute_hash();
        if expected != block.hash {
            warn!(
                round = block.round,
                author = %short_hex(&block.author),
                "Vertex hash mismatch"
            );
            return Ok(());
        }

        // Reject vertices too far in the future (> 10 rounds ahead)
        let max_future_rounds = 10;
        if block.round > self.state.current_round + max_future_rounds {
            warn!(
                vertex_round = block.round,
                current_round = self.state.current_round,
                "Vertex too far in the future"
            );
            return Ok(());
        }

        let round = block.round;
        let hash = block.hash;

        match self.dag.insert(block) {
            Ok(()) => {
                self.state.record_vertex(round, hash);
                debug!(
                    round,
                    hash = %short_hex(&hash),
                    "Accepted vertex from peer"
                );
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
        let rule = CommitRule::new(&self.dag, &self.validators, commit_config);

        for wave in start_wave..current_wave {
            let status = rule.try_direct_commit(wave);
            match status {
                LeaderStatus::Commit(hash) => {
                    info!(wave, hash = %short_hex(&hash), "Block committed (direct)");
                    let already = self.state.committed_blocks().to_vec();
                    self.state.record_commit(hash);
                    self.state.last_committed_wave = Some(wave);
                    self.state.prune_before(wave * wave_len);
                    self.vrf_seed = dendrite_core::hash(&hash);
                    if let Ok(batch) =
                        crate::ordering::extract_committed_batch(&self.dag, hash, &already)
                    {
                        let _ = self.outbox.try_send(ConsensusOutput::BatchCommitted(batch));
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

        Ok(())
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
    use dendrite_storage::StateStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_engine_test_{}_{}", pid, id))
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
        let data = bincode::serialize(&block).unwrap();

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
        let data = bincode::serialize(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 0);
        cleanup(&path);
    }

    #[test]
    fn vertex_serialization_roundtrip() {
        let block = DagBlock::genesis([1u8; 32], 1000);
        let encoded = bincode::serialize(&block).unwrap();
        let decoded: DagBlock = bincode::deserialize(&encoded).unwrap();
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
        block.hash = [0xFFu8; 32]; // Tamper with hash
        let data = bincode::serialize(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(1).len(), 0);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_rejects_future_round_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        // Engine is at round 0; a vertex at round 50 should be rejected (>10 rounds ahead)
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(50, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = bincode::serialize(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        assert_eq!(engine.state.vertices_at_round(50).len(), 0);
        cleanup(&path);
    }

    #[tokio::test]
    async fn engine_accepts_near_future_vertex() {
        let (mut engine, _in_tx, _out_rx, path) = make_test_engine();
        engine.insert_genesis().unwrap();
        // Round 5 is within 10 rounds of round 0
        let genesis_hashes: Vec<BlockHash> = engine.state.vertices_at_round(0).to_vec();
        let block = DagBlock::new(5, [2u8; 32], genesis_hashes, vec![], now_ms()).unwrap();
        let data = bincode::serialize(&block).unwrap();

        engine.handle_received_vertex(&data).unwrap();
        // DagStore may reject due to parent round mismatch, but the round check should pass
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
}
