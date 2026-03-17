use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use aztibase_consensus::{
    AttestationAggregator, CHECKPOINT_INTERVAL, Checkpoint, CommittedBatch, ComputeCommitmentStore,
    InferenceAttestation,
};
use aztibase_core::{Hash, hash};
use aztibase_execution::{
    AccountState, AgentPolicyStore, BaseFeeCalculator, BridgeEscrow as BridgeEscrowStore,
    BridgeWithdrawProofs, ChainParams, ContractTx, CreateProposalParams, EmissionTracker,
    ExecutionReceipt, FeeEscrow, GovernanceStore, L2Anchor, L2AnchorStore, L2Registration,
    L2Registry, OffenseType, StakingStore, TransferTx, TxKind,
    block_stm::{BlockSTMExecutor, apply_block_stm_to_state},
    compute_tx_hash, escrow_fee, evm, execute_contract_txs, flush_agent_policies,
    flush_bridge_stores, flush_chain_params, flush_emission, flush_governance, flush_staking,
    flush_state, latest_batch_index, load_agent_policies, load_base_fee, load_bridge_stores,
    load_chain_params, load_emission, load_governance, load_staking, load_state,
    model_registry::{MODEL_REGISTRY_ADDRESS, ModelRegistry},
    refund_unused,
    state::AccountType,
    store_base_fee, store_batch_root, store_receipts, verify_and_route_batch_with_pubkeys,
};
use aztibase_runtime::{AIRuntime, AnomalyScorer, InferenceRequest, TxFeatures};
use aztibase_storage::StateStore;
use tokio::sync::{RwLock, mpsc};

use crate::task_pool::{SettlementResult, TaskAssigner, TaskPool, TaskSettlement};

type SetAgentPolicyEntry = ([u8; 32], [u8; 32], u128, u128, Vec<u8>, u64, u64);
type AgentExecuteEntry = ([u8; 32], u8, [u8; 32], u128, Vec<u8>, u64);
type AnchorL2Entry = ([u8; 32], [u8; 32], [u8; 32], [u8; 32], u64, u64, u64);
type BridgeDepositEntry = ([u8; 32], [u8; 32], [u8; 32], u128, u64);
type BridgeWithdrawEntry = ([u8; 32], [u8; 32], u128, Vec<u8>, [u8; 32], u64);
type RegisterL2Entry = ([u8; 32], [u8; 32], String, Vec<[u8; 32]>, [u8; 32], u64);

const MAX_EXECUTED_ANCHORS: usize = 10_000;
const MAX_ATTESTATIONS_PER_TASK: usize = 32;
const MAX_ATTESTATION_BUFFER_TASKS: usize = 2048;

const MIN_VALIDATOR_STAKE: u128 = 10_000;
const MAX_VALIDATOR_STAKE_CAP: u128 = 100_000_000;
const UNBONDING_ROUNDS: u64 = 4_536_000;

/// Event emitted by consensus when a validator offense is detected.
#[derive(Clone, Debug)]
pub struct SlashEvent {
    pub validator_id: [u8; 32],
    pub offense: OffenseType,
    pub round: u64,
    pub existing_hash: Option<[u8; 32]>,
    pub duplicate_hash: Option<[u8; 32]>,
}

/// Result of executing a single committed batch.
#[derive(Clone, Debug)]
pub struct PipelineResult {
    pub batch_anchor: [u8; 32],
    pub state_root: [u8; 32],
    pub transfer_count: usize,
    pub contract_count: usize,
    pub routing_errors: usize,
    pub receipts: Vec<ExecutionReceipt>,
    pub total_fees_burned: u128,
}

/// Owns account state and executes committed batches received from consensus.
/// Optionally persists state to redb after each batch.
pub struct ExecutionPipeline {
    state: Arc<RwLock<AccountState>>,
    store: Option<Arc<StateStore>>,
    rx: mpsc::Receiver<CommittedBatch>,
    batch_count: Arc<std::sync::atomic::AtomicU64>,
    executed_anchors_set: HashSet<[u8; 32]>,
    executed_anchors_queue: VecDeque<[u8; 32]>,
    result_tx: Option<mpsc::Sender<PipelineResult>>,
    ai_runtime: Option<Arc<dyn AIRuntime>>,
    base_fee: Arc<std::sync::atomic::AtomicU64>,
    base_fee_calculator: BaseFeeCalculator,
    anomaly_scorer: AnomalyScorer,
    task_pool: Arc<RwLock<TaskPool>>,
    pending_task_count: Arc<std::sync::atomic::AtomicU64>,
    current_round: u64,
    attestation_buffer: HashMap<Hash, Vec<InferenceAttestation>>,
    attestation_aggregator: AttestationAggregator,
    compute_commitments: Arc<RwLock<ComputeCommitmentStore>>,
    governance: Arc<RwLock<GovernanceStore>>,
    chain_params: Arc<RwLock<ChainParams>>,
    emission_tracker: Arc<RwLock<EmissionTracker>>,
    staking_store: Arc<RwLock<StakingStore>>,
    slash_rx: mpsc::Receiver<SlashEvent>,
    slash_tx: mpsc::Sender<SlashEvent>,
    consensus_tx: Option<mpsc::Sender<aztibase_consensus::ConsensusInput>>,
    epoch_participation: HashSet<[u8; 32]>,
    consensus_addrs: HashSet<[u8; 32]>,
    agent_policy_store: Arc<RwLock<AgentPolicyStore>>,
    l2_registry: Arc<RwLock<L2Registry>>,
    l2_anchor_store: Arc<RwLock<L2AnchorStore>>,
    bridge_escrow: Arc<RwLock<BridgeEscrowStore>>,
    bridge_withdraw_proofs: Arc<RwLock<BridgeWithdrawProofs>>,
    archive: bool,
    faucet_enabled: bool,
}

impl ExecutionPipeline {
    /// Create a pipeline with redb persistence. Loads existing state on creation.
    pub fn with_storage(store: Arc<StateStore>, rx: mpsc::Receiver<CommittedBatch>) -> Self {
        let state = match load_state(&store) {
            Ok(s) => {
                tracing::info!(accounts = s.account_count(), "State loaded from disk");
                s
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load state from disk, starting fresh");
                AccountState::new()
            }
        };
        let recovered_batch_count = match latest_batch_index(&store) {
            Ok(Some(idx)) => {
                tracing::info!(last_batch = idx, "Recovered batch index from disk");
                idx + 1
            }
            Ok(None) => 0,
            Err(e) => {
                tracing::warn!(error = %e, "Could not read last batch index, starting from 0");
                0
            }
        };
        let initial_base_fee = match load_base_fee(&store) {
            Ok(Some(fee)) => {
                tracing::info!(base_fee = fee, "Base fee loaded from disk");
                fee
            }
            _ => 1,
        };
        let calculator = BaseFeeCalculator::new(initial_base_fee);
        let base_fee = Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee()));
        let (slash_tx_init, slash_rx_init) = mpsc::channel(64);

        let staking = load_staking(&store).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load staking, starting fresh");
            StakingStore::new()
        });
        let governance = load_governance(&store).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load governance, starting fresh");
            GovernanceStore::new()
        });
        let emission = load_emission(&store).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load emission tracker, starting fresh");
            EmissionTracker::new(aztibase_execution::tokenomics::DEFAULT_EPOCH_LENGTH)
        });
        let chain_params = load_chain_params(&store).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load chain params, starting fresh");
            ChainParams::defaults()
        });
        let agent_policies = load_agent_policies(&store).unwrap_or_else(|e| {
            tracing::warn!(error = %e, "Failed to load agent policies, starting fresh");
            AgentPolicyStore::new()
        });
        let (l2_registry, l2_anchors, escrow, withdraw_proofs) = load_bridge_stores(&store)
            .unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to load bridge stores, starting fresh");
                (
                    L2Registry::new(),
                    L2AnchorStore::new(),
                    BridgeEscrowStore::new(),
                    BridgeWithdrawProofs::new(),
                )
            });

        if recovered_batch_count > 0 {
            tracing::info!(
                staking = "loaded",
                governance = "loaded",
                emission = "loaded",
                chain_params = "loaded",
                agent_policies = "loaded",
                bridge = "loaded",
                "Protocol stores recovered from disk"
            );
        }

        Self {
            state: Arc::new(RwLock::new(state)),
            store: Some(store),
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(recovered_batch_count)),
            executed_anchors_set: HashSet::new(),
            executed_anchors_queue: VecDeque::new(),
            result_tx: None,
            ai_runtime: None,
            base_fee,
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
            task_pool: Arc::new(RwLock::new(TaskPool::new())),
            pending_task_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            current_round: 0,
            attestation_buffer: HashMap::new(),
            attestation_aggregator: AttestationAggregator::new(2),
            compute_commitments: Arc::new(RwLock::new(ComputeCommitmentStore::new())),
            governance: Arc::new(RwLock::new(governance)),
            chain_params: Arc::new(RwLock::new(chain_params)),
            emission_tracker: Arc::new(RwLock::new(emission)),
            staking_store: Arc::new(RwLock::new(staking)),
            slash_rx: slash_rx_init,
            slash_tx: slash_tx_init,
            consensus_tx: None,
            epoch_participation: HashSet::new(),
            consensus_addrs: HashSet::new(),
            agent_policy_store: Arc::new(RwLock::new(agent_policies)),
            l2_registry: Arc::new(RwLock::new(l2_registry)),
            l2_anchor_store: Arc::new(RwLock::new(l2_anchors)),
            bridge_escrow: Arc::new(RwLock::new(escrow)),
            bridge_withdraw_proofs: Arc::new(RwLock::new(withdraw_proofs)),
            archive: false,
            faucet_enabled: true,
        }
    }

    /// Enable archive mode (disables eviction of old data).
    pub fn set_archive(&mut self, archive: bool) {
        self.archive = archive;
    }

    /// Disable faucet drip processing (must be disabled for mainnet).
    #[allow(dead_code)]
    pub fn set_faucet_enabled(&mut self, enabled: bool) {
        self.faucet_enabled = enabled;
    }

    /// Shared compute commitment store (for RPC server).
    pub fn shared_compute_commitments(&self) -> Arc<RwLock<ComputeCommitmentStore>> {
        Arc::clone(&self.compute_commitments)
    }

    /// Shared governance store (for RPC server).
    pub fn shared_governance(&self) -> Arc<RwLock<GovernanceStore>> {
        Arc::clone(&self.governance)
    }

    /// Shared chain parameters (for RPC server and fee calculator).
    pub fn shared_chain_params(&self) -> Arc<RwLock<ChainParams>> {
        Arc::clone(&self.chain_params)
    }

    /// Shared emission tracker (for RPC server).
    pub fn shared_emission_tracker(&self) -> Arc<RwLock<EmissionTracker>> {
        Arc::clone(&self.emission_tracker)
    }

    /// Shared staking store (for RPC server and epoch boundary).
    pub fn shared_staking_store(&self) -> Arc<RwLock<StakingStore>> {
        Arc::clone(&self.staking_store)
    }

    /// Shared agent policy store (for RPC server).
    pub fn shared_agent_policy_store(&self) -> Arc<RwLock<AgentPolicyStore>> {
        Arc::clone(&self.agent_policy_store)
    }

    /// Shared L2 registry (for RPC server). Wired in Phase 3.
    #[allow(dead_code)]
    pub fn shared_l2_registry(&self) -> Arc<RwLock<L2Registry>> {
        Arc::clone(&self.l2_registry)
    }

    /// Shared L2 anchor store (for RPC server). Wired in Phase 3.
    #[allow(dead_code)]
    pub fn shared_l2_anchor_store(&self) -> Arc<RwLock<L2AnchorStore>> {
        Arc::clone(&self.l2_anchor_store)
    }

    /// Shared bridge escrow (for RPC server). Wired in Phase 3.
    #[allow(dead_code)]
    pub fn shared_bridge_escrow(&self) -> Arc<RwLock<BridgeEscrowStore>> {
        Arc::clone(&self.bridge_escrow)
    }

    /// Shared bridge withdraw proofs (for RPC server). Wired in Phase 3.
    #[allow(dead_code)]
    pub fn shared_bridge_withdraw_proofs(&self) -> Arc<RwLock<BridgeWithdrawProofs>> {
        Arc::clone(&self.bridge_withdraw_proofs)
    }

    /// Clone the slash event sender (for consensus → pipeline slashing bridge).
    pub fn slash_sender(&self) -> mpsc::Sender<SlashEvent> {
        self.slash_tx.clone()
    }

    /// Attach the consensus input channel for validator set updates at epoch boundaries.
    pub fn set_consensus_tx(&mut self, tx: mpsc::Sender<aztibase_consensus::ConsensusInput>) {
        self.consensus_tx = Some(tx);
    }

    /// Register genesis validators in the StakingStore so staking RPCs return
    /// data from round 0. Call after `apply_genesis()`.
    #[allow(clippy::type_complexity)]
    pub async fn bootstrap_genesis_validators(
        &mut self,
        validators: &[([u8; 32], u128, Option<[u8; 32]>)],
    ) {
        for &(vid, _, _) in validators {
            self.consensus_addrs.insert(vid);
        }
        let mut staking = self.staking_store.write().await;
        for &(vid, stake, ed25519_pubkey) in validators {
            if staking
                .register_validator_with_keys(
                    vid,
                    stake,
                    0,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                    ed25519_pubkey,
                    None,
                )
                .is_ok()
            {
                tracing::info!(
                    validator = %short_hex(&vid),
                    stake,
                    has_ed25519 = ed25519_pubkey.is_some(),
                    "Genesis validator registered in StakingStore"
                );
            }
        }
    }

    /// Restore all protocol stores from a snapshot bundle.
    pub async fn apply_protocol_bundle(&self, bundle: aztibase_execution::ProtocolStoreBundle) {
        *self.staking_store.write().await = bundle.staking;
        *self.governance.write().await = bundle.governance;
        *self.emission_tracker.write().await = bundle.emission;
        *self.chain_params.write().await = bundle.chain_params;
        *self.agent_policy_store.write().await = bundle.agent_policies;
        *self.l2_registry.write().await = bundle.l2_registry;
        *self.l2_anchor_store.write().await = bundle.l2_anchors;
        *self.bridge_escrow.write().await = bundle.bridge_escrow;
        *self.bridge_withdraw_proofs.write().await = bundle.bridge_proofs;
        self.base_fee
            .store(bundle.base_fee, std::sync::atomic::Ordering::Relaxed);
    }

    /// Attach an AI runtime for inference transaction execution.
    pub fn set_ai_runtime(&mut self, runtime: Arc<dyn AIRuntime>) {
        self.ai_runtime = Some(runtime);
    }

    /// Shared read handle to account state (for RPC server).
    pub fn shared_state(&self) -> Arc<RwLock<AccountState>> {
        Arc::clone(&self.state)
    }

    /// Shared batch counter (for RPC server).
    pub fn shared_batch_count(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.batch_count)
    }

    /// Shared base fee (for RPC server and mempool validation).
    pub fn shared_base_fee(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.base_fee)
    }

    /// Shared pending task count (for RPC server).
    pub fn shared_pending_task_count(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.pending_task_count)
    }

    /// Attach a channel to receive execution results (for state root broadcasting).
    pub fn set_result_sender(&mut self, tx: mpsc::Sender<PipelineResult>) {
        self.result_tx = Some(tx);
    }

    /// Run the pipeline loop, processing committed batches until the channel closes.
    pub async fn run(mut self) {
        while let Some(batch) = self.rx.recv().await {
            if self.executed_anchors_set.contains(&batch.anchor_hash) {
                tracing::warn!(
                    anchor = %short_hex(&batch.anchor_hash),
                    "Duplicate batch skipped"
                );
                continue;
            }
            self.executed_anchors_set.insert(batch.anchor_hash);
            self.executed_anchors_queue.push_back(batch.anchor_hash);
            if self.executed_anchors_queue.len() > MAX_EXECUTED_ANCHORS
                && let Some(oldest) = self.executed_anchors_queue.pop_front()
            {
                self.executed_anchors_set.remove(&oldest);
            }
            match self.execute_batch(&batch).await {
                Ok(result) => {
                    tracing::info!(
                        anchor = %short_hex(&result.batch_anchor),
                        state_root = %short_hex(&result.state_root),
                        transfers = result.transfer_count,
                        contracts = result.contract_count,
                        routing_errors = result.routing_errors,
                        receipts = result.receipts.len(),
                        fees_burned = result.total_fees_burned,
                        "Batch executed"
                    );
                    if let Some(ref tx) = self.result_tx {
                        let _ = tx.try_send(result);
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, "Pipeline halting due to fatal error");
                    break;
                }
            }
        }
        let final_batch = self.batch_count.load(std::sync::atomic::Ordering::Relaxed);
        tracing::info!(
            last_batch = final_batch,
            "Execution pipeline shutting down — state persisted through batch {final_batch}"
        );
    }

    /// Execute a single committed batch: verify signatures, route transactions,
    /// execute each type, flush to disk, return the resulting state root.
    ///
    /// Returns `Err` if state flush or batch root storage fails — the caller
    /// must treat this as fatal and halt.
    pub async fn execute_batch(
        &mut self,
        batch: &CommittedBatch,
    ) -> Result<PipelineResult, anyhow::Error> {
        self.current_round += 1;

        // Evict expired tasks and refund their rewards to requesters.
        {
            let mut pool = self.task_pool.write().await;
            let expired_tasks = pool.drain_expired(self.current_round);
            if !expired_tasks.is_empty() {
                let mut state_guard = self.state.write().await;
                for task in &expired_tasks {
                    let prev = state_guard.balance(&task.requester);
                    state_guard.set_balance(&task.requester, prev + task.reward);
                    tracing::debug!(
                        task_id = %short_hex(&task.task_id),
                        requester = %short_hex(&task.requester),
                        refund = task.reward,
                        "Expired task refunded"
                    );
                }
                drop(state_guard);
            }
            for task in &expired_tasks {
                self.attestation_buffer.remove(&task.task_id);
            }
            self.pending_task_count
                .store(pool.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }

        let (mut routed, errors, sender_pubkeys) =
            verify_and_route_batch_with_pubkeys(&batch.transactions);

        routed.sort_by(|a, b| a.sender().cmp(b.sender()).then(a.nonce().cmp(&b.nonce())));

        for tx in &routed {
            self.epoch_participation.insert(*tx.sender());
        }
        for vid in &self.consensus_addrs {
            self.epoch_participation.insert(*vid);
        }

        let mut state = self.state.write().await;

        // Phase 0: Validate nonces against current state.
        // For same-sender txs sorted by nonce, require sequential chain starting from current nonce.
        let mut nonce_valid = Vec::with_capacity(routed.len());
        let mut nonce_reject_receipts = Vec::new();
        {
            let mut expected_nonces: std::collections::HashMap<[u8; 32], u64> =
                std::collections::HashMap::new();
            for tx in &routed {
                let sender = *tx.sender();
                let expected = expected_nonces
                    .entry(sender)
                    .or_insert_with(|| state.nonce(&sender));
                if tx.nonce() == *expected {
                    nonce_valid.push(true);
                    *expected += 1;
                } else {
                    nonce_valid.push(false);
                    nonce_reject_receipts.push(ExecutionReceipt {
                        tx_hash: compute_tx_hash(tx),
                        success: false,
                        gas_used: 0,
                        contract_address: None,
                        error: Some(format!(
                            "nonce mismatch: expected {}, got {}",
                            expected,
                            tx.nonce()
                        )),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }
        let original_routed = std::mem::take(&mut routed);

        // Serialize ALL transactions for persistent storage before nonce filtering.
        // Without this, nonce-rejected txs would be missing from TX_TABLE
        // even though their receipts and batch index entries exist.
        let all_tx_store_data: Vec<([u8; 32], Vec<u8>)> = original_routed
            .iter()
            .filter_map(|tx| {
                postcard::to_allocvec(tx)
                    .ok()
                    .map(|data| (compute_tx_hash(tx), data))
            })
            .collect();

        routed = original_routed
            .into_iter()
            .zip(nonce_valid.iter())
            .filter(|(_, valid)| **valid)
            .map(|(tx, _)| tx)
            .collect();

        // Phase 1: Validate gas price and escrow fees.
        // All txs must meet the current base fee. Txs below base fee are rejected.
        let current_base_fee = self.base_fee_calculator.base_fee();
        let mut escrows: Vec<Option<FeeEscrow>> = Vec::with_capacity(routed.len());
        let mut escrowed_indices: Vec<usize> = Vec::new();
        let mut receipts = Vec::new();
        let mut total_fees_burned: u128 = 0;

        for (i, tx) in routed.iter().enumerate() {
            if matches!(tx, TxKind::FaucetDrip { .. }) {
                escrows.push(None);
                escrowed_indices.push(i);
                continue;
            }
            let gas_price = tx.gas_price();
            let gas_limit = tx.gas_limit();
            if gas_price < current_base_fee {
                let tx_hash = compute_tx_hash(tx);
                receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 0,
                    contract_address: None,
                    error: Some(format!(
                        "gas price too low: {} < base fee {}",
                        gas_price, current_base_fee
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                escrows.push(None);
                continue;
            }
            let value = match tx {
                TxKind::Transfer { value, .. } => *value,
                TxKind::EvmCall { value, .. } => *value,
                _ => 0,
            };
            match escrow_fee(&mut state, tx.sender(), gas_limit, gas_price, value) {
                Some(esc) => {
                    escrows.push(Some(esc));
                    escrowed_indices.push(i);
                }
                None => {
                    let tx_hash = compute_tx_hash(tx);
                    receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 0,
                        contract_address: None,
                        error: Some("insufficient balance for gas escrow".into()),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    escrows.push(None);
                }
            }
        }

        // Filter routed txs to only those that passed escrow.
        let executable: Vec<&TxKind> = escrowed_indices.iter().map(|&i| &routed[i]).collect();

        let mut transfers = Vec::new();
        let mut contracts = Vec::new();
        let mut evm_deploys = Vec::new();
        let mut evm_calls = Vec::new();
        let mut ai_infers = Vec::new();
        let mut create_agents = Vec::new();
        let mut register_models = Vec::new();
        let mut post_tasks = Vec::new();
        let mut submit_attestations = Vec::new();
        let mut commit_computes = Vec::new();
        let mut deregister_computes = Vec::new();
        let mut deregister_models = Vec::new();
        let mut create_proposals = Vec::new();
        let mut cast_votes = Vec::new();
        #[allow(clippy::type_complexity)]
        let mut stakes: Vec<([u8; 32], u128, u64, Option<[u8; 32]>)> = Vec::new();
        let mut unstakes: Vec<([u8; 32], u128, u64)> = Vec::new();
        let mut delegates: Vec<([u8; 32], [u8; 32], u128, u64)> = Vec::new();
        let mut undelegates: Vec<([u8; 32], u64)> = Vec::new();
        let mut set_agent_policies: Vec<SetAgentPolicyEntry> = Vec::new();
        let mut agent_executes: Vec<AgentExecuteEntry> = Vec::new();
        let mut anchor_l2_states: Vec<AnchorL2Entry> = Vec::new();
        let mut bridge_deposits: Vec<BridgeDepositEntry> = Vec::new();
        let mut bridge_withdraws: Vec<BridgeWithdrawEntry> = Vec::new();
        let mut register_l2s: Vec<RegisterL2Entry> = Vec::new();
        let mut rotate_keys: Vec<([u8; 32], [u8; 32], u64)> = Vec::new();
        let mut faucet_drips: Vec<([u8; 32], [u8; 32], u128)> = Vec::new();
        #[allow(clippy::type_complexity)]
        let mut register_validators: Vec<([u8; 32], u128, u64, Option<[u8; 32]>)> = Vec::new();

        for tx in &executable {
            match tx {
                TxKind::Transfer {
                    from,
                    to,
                    value,
                    nonce,
                    ..
                } => {
                    let mut preimage = Vec::new();
                    preimage.extend_from_slice(from);
                    preimage.extend_from_slice(to);
                    preimage.extend_from_slice(&value.to_le_bytes());
                    preimage.extend_from_slice(&nonce.to_le_bytes());
                    transfers.push(TransferTx {
                        hash: hash(&preimage),
                        from: *from,
                        to: *to,
                        value: *value,
                        nonce: *nonce,
                    });
                }
                TxKind::ContractDeploy {
                    deployer,
                    code,
                    nonce,
                    gas_limit,
                    ..
                } => {
                    contracts.push(ContractTx::Deploy {
                        hash: hash(code),
                        deployer: *deployer,
                        code: code.clone(),
                        nonce: *nonce,
                        gas_limit: *gas_limit,
                    });
                }
                TxKind::ContractCall {
                    caller,
                    contract,
                    func_name,
                    args_data,
                    nonce,
                    gas_limit,
                    ..
                } => {
                    contracts.push(ContractTx::Call {
                        hash: hash(func_name.as_bytes()),
                        caller: *caller,
                        contract: *contract,
                        func_name: func_name.clone(),
                        args_data: args_data.clone(),
                        nonce: *nonce,
                        gas_limit: *gas_limit,
                    });
                }
                TxKind::EvmDeploy {
                    deployer,
                    code,
                    nonce,
                    gas_limit,
                    ..
                } => {
                    evm_deploys.push((*deployer, code.clone(), *nonce, *gas_limit));
                }
                TxKind::EvmCall {
                    caller,
                    contract,
                    calldata,
                    nonce,
                    gas_limit,
                    value,
                    ..
                } => {
                    evm_calls.push((
                        *caller,
                        *contract,
                        calldata.clone(),
                        *nonce,
                        *gas_limit,
                        *value,
                    ));
                }
                TxKind::AiInfer {
                    requester,
                    model_id,
                    input,
                    nonce,
                    max_compute_units,
                    ..
                } => {
                    ai_infers.push((
                        *requester,
                        model_id.clone(),
                        input.clone(),
                        *nonce,
                        *max_compute_units,
                    ));
                }
                TxKind::CreateAgent {
                    creator,
                    model_id,
                    nonce,
                    ..
                } => {
                    create_agents.push((*creator, model_id.clone(), *nonce));
                }
                TxKind::RegisterModel {
                    owner,
                    model_id,
                    fingerprint,
                    compute_cost,
                    min_stake,
                    nonce,
                    ..
                } => {
                    register_models.push((
                        *owner,
                        model_id.clone(),
                        *fingerprint,
                        *compute_cost,
                        *min_stake,
                        *nonce,
                    ));
                }
                TxKind::PostTask {
                    requester,
                    model_id,
                    input_hash,
                    reward,
                    deadline_round,
                    nonce,
                    ..
                } => {
                    post_tasks.push((
                        *requester,
                        model_id.clone(),
                        *input_hash,
                        *reward,
                        *deadline_round,
                        *nonce,
                    ));
                }
                TxKind::SubmitAttestation {
                    validator,
                    task_id,
                    result_hash,
                    compute_units,
                    signature,
                    nonce,
                    ..
                } => {
                    submit_attestations.push((
                        *validator,
                        *task_id,
                        *result_hash,
                        *compute_units,
                        signature.clone(),
                        *nonce,
                    ));
                }
                TxKind::CommitCompute {
                    validator,
                    supported_models,
                    committed_stake,
                    bls_pubkey,
                    bls_pop,
                    nonce,
                    ..
                } => {
                    commit_computes.push((
                        *validator,
                        supported_models.clone(),
                        *committed_stake,
                        bls_pubkey.clone(),
                        bls_pop.clone(),
                        *nonce,
                    ));
                }
                TxKind::DeregisterCompute {
                    validator, nonce, ..
                } => {
                    deregister_computes.push((*validator, *nonce));
                }
                TxKind::DeregisterModel {
                    owner,
                    model_id,
                    nonce,
                    ..
                } => {
                    deregister_models.push((*owner, model_id.clone(), *nonce));
                }
                TxKind::CreateProposal {
                    proposer,
                    description,
                    param_key,
                    param_value,
                    voting_period,
                    nonce,
                    ..
                } => {
                    create_proposals.push((
                        *proposer,
                        description.clone(),
                        param_key.clone(),
                        param_value.clone(),
                        *voting_period,
                        *nonce,
                    ));
                }
                TxKind::CastVote {
                    voter,
                    proposal_id,
                    approve,
                    nonce,
                    ..
                } => {
                    cast_votes.push((*voter, *proposal_id, *approve, *nonce));
                }
                TxKind::Stake {
                    staker,
                    amount,
                    nonce,
                    ..
                } => {
                    let pubkey = sender_pubkeys.get(staker).copied();
                    stakes.push((*staker, *amount, *nonce, pubkey));
                }
                TxKind::Unstake {
                    staker,
                    amount,
                    nonce,
                    ..
                } => {
                    unstakes.push((*staker, *amount, *nonce));
                }
                TxKind::Delegate {
                    delegator,
                    validator_id,
                    amount,
                    nonce,
                    ..
                } => {
                    delegates.push((*delegator, *validator_id, *amount, *nonce));
                }
                TxKind::Undelegate {
                    delegator, nonce, ..
                } => {
                    undelegates.push((*delegator, *nonce));
                }
                TxKind::SetAgentPolicy {
                    owner,
                    agent,
                    per_tx_limit,
                    per_epoch_limit,
                    allowed_tx_kinds,
                    expiry_epoch,
                    nonce,
                    ..
                } => {
                    set_agent_policies.push((
                        *owner,
                        *agent,
                        *per_tx_limit,
                        *per_epoch_limit,
                        allowed_tx_kinds.clone(),
                        *expiry_epoch,
                        *nonce,
                    ));
                }
                TxKind::AgentExecute {
                    agent,
                    inner_tx_kind,
                    to,
                    value,
                    data,
                    nonce,
                    ..
                } => {
                    agent_executes.push((
                        *agent,
                        *inner_tx_kind,
                        *to,
                        *value,
                        data.clone(),
                        *nonce,
                    ));
                }
                TxKind::AnchorL2State {
                    sequencer,
                    l2_chain_id,
                    state_root,
                    batch_data_hash,
                    l2_block_start,
                    l2_block_end,
                    nonce,
                    ..
                } => {
                    anchor_l2_states.push((
                        *sequencer,
                        *l2_chain_id,
                        *state_root,
                        *batch_data_hash,
                        *l2_block_start,
                        *l2_block_end,
                        *nonce,
                    ));
                }
                TxKind::BridgeDeposit {
                    depositor,
                    l2_chain_id,
                    l2_recipient,
                    amount,
                    nonce,
                    ..
                } => {
                    bridge_deposits.push((
                        *depositor,
                        *l2_chain_id,
                        *l2_recipient,
                        *amount,
                        *nonce,
                    ));
                }
                TxKind::BridgeWithdraw {
                    withdrawer,
                    l2_chain_id,
                    amount,
                    l2_burn_proof,
                    l2_state_root,
                    nonce,
                    ..
                } => {
                    bridge_withdraws.push((
                        *withdrawer,
                        *l2_chain_id,
                        *amount,
                        l2_burn_proof.clone(),
                        *l2_state_root,
                        *nonce,
                    ));
                }
                TxKind::RegisterL2 {
                    owner,
                    l2_chain_id,
                    name,
                    sequencer_set,
                    bridge_address,
                    nonce,
                    ..
                } => {
                    register_l2s.push((
                        *owner,
                        *l2_chain_id,
                        name.clone(),
                        sequencer_set.clone(),
                        *bridge_address,
                        *nonce,
                    ));
                }
                TxKind::RotateValidatorKey {
                    validator,
                    new_pubkey,
                    nonce,
                    ..
                } => {
                    rotate_keys.push((*validator, *new_pubkey, *nonce));
                }
                TxKind::FaucetDrip {
                    validator,
                    recipient,
                    amount,
                    ..
                } => {
                    faucet_drips.push((*validator, *recipient, *amount));
                }
                TxKind::RegisterValidator {
                    registrant,
                    amount,
                    nonce,
                    ..
                } => {
                    let pubkey = sender_pubkeys.get(registrant).copied();
                    register_validators.push((*registrant, *amount, *nonce, pubkey));
                }
            }
        }

        let transfer_count = transfers.len();
        let contract_count = contracts.len()
            + evm_deploys.len()
            + evm_calls.len()
            + ai_infers.len()
            + create_agents.len()
            + register_models.len()
            + post_tasks.len()
            + submit_attestations.len()
            + commit_computes.len()
            + deregister_computes.len()
            + deregister_models.len()
            + create_proposals.len()
            + cast_votes.len()
            + stakes.len()
            + unstakes.len()
            + delegates.len()
            + undelegates.len()
            + set_agent_policies.len()
            + agent_executes.len()
            + anchor_l2_states.len()
            + bridge_deposits.len()
            + bridge_withdraws.len()
            + register_l2s.len()
            + rotate_keys.len();

        // Phase 2: Execute transactions.
        let mut exec_receipts = Vec::new();

        if !transfers.is_empty() {
            let (stm_outputs, write_sets) = BlockSTMExecutor::execute_full(&transfers, &state);

            for (i, output) in stm_outputs.iter().enumerate() {
                exec_receipts.push(ExecutionReceipt {
                    tx_hash: transfers[i].hash,
                    success: output.success,
                    gas_used: output.gas_used,
                    contract_address: None,
                    error: output.error.clone(),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
            }

            apply_block_stm_to_state(&mut state, &write_sets);
        }
        if !contracts.is_empty() {
            let contract_receipts = execute_contract_txs(&mut state, &contracts);
            for cr in &contract_receipts {
                exec_receipts.push(ExecutionReceipt {
                    tx_hash: cr.tx_hash,
                    success: cr.success,
                    gas_used: cr.gas_used,
                    contract_address: cr.contract_address,
                    error: cr.error.clone(),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
            }
        }

        for (deployer, code, nonce, gas_limit) in &evm_deploys {
            let tx_hash = hash(code);
            let cr = evm::evm_deploy(&mut state, tx_hash, deployer, code, *nonce, *gas_limit);
            exec_receipts.push(ExecutionReceipt {
                tx_hash: cr.tx_hash,
                success: cr.success,
                gas_used: cr.gas_used,
                contract_address: cr.contract_address,
                error: cr.error,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (caller, contract, calldata, nonce, gas_limit, value) in &evm_calls {
            let tx_hash = hash(calldata);
            let cr = evm::evm_call(
                &mut state, tx_hash, caller, contract, calldata, *nonce, *gas_limit, *value,
            );
            exec_receipts.push(ExecutionReceipt {
                tx_hash: cr.tx_hash,
                success: cr.success,
                gas_used: cr.gas_used,
                contract_address: cr.contract_address,
                error: cr.error,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (requester, model_id, input, _nonce, max_compute_units) in &ai_infers {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(requester);
            preimage.extend_from_slice(model_id.as_bytes());
            preimage.extend_from_slice(input);
            let tx_hash = hash(&preimage);

            let receipt = match &self.ai_runtime {
                Some(runtime) if runtime.supports_model(model_id) => {
                    let req = InferenceRequest {
                        model_id: model_id.clone(),
                        input: input.clone(),
                        max_compute_units: *max_compute_units,
                        metadata: Default::default(),
                    };
                    match runtime.infer(&req) {
                        Ok(result) => ExecutionReceipt {
                            tx_hash,
                            success: true,
                            gas_used: result.compute_units_used,
                            contract_address: None,
                            error: None,
                            inference_hash: Some(result.deterministic_hash),
                            anomaly_score: 0.0,
                        },
                        Err(e) => ExecutionReceipt {
                            tx_hash,
                            success: false,
                            gas_used: 0,
                            contract_address: None,
                            error: Some(format!("inference failed: {e}")),
                            inference_hash: None,
                            anomaly_score: 0.0,
                        },
                    }
                }
                _ => ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 0,
                    contract_address: None,
                    error: Some(format!("model not available: {model_id}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                },
            };
            exec_receipts.push(receipt);
        }

        for (creator, model_id, nonce) in &create_agents {
            let creator_nonce = state.nonce(creator);
            let mut preimage = Vec::new();
            preimage.extend_from_slice(creator);
            preimage.extend_from_slice(model_id.as_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            if *nonce != creator_nonce {
                state.increment_nonce(creator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {creator_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let agent_addr = aztibase_execution::compute_contract_address(creator, *nonce);
            state.set_account_type(&agent_addr, AccountType::AIAgent);
            state.set_model_id(&agent_addr, model_id.clone());
            state.increment_nonce(creator);

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 53_000,
                contract_address: Some(agent_addr),
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (owner, model_id, fingerprint, compute_cost, min_stake, nonce) in &register_models {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(owner);
            preimage.extend_from_slice(model_id.as_bytes());
            preimage.extend_from_slice(fingerprint);
            let tx_hash = hash(&preimage);

            let owner_nonce = state.nonce(owner);
            if *nonce != owner_nonce {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {owner_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
            let result = ModelRegistry::register(
                &mut registry_acct.storage,
                model_id.clone(),
                *owner,
                *fingerprint,
                *compute_cost,
                *min_stake,
                0,
            );
            state.increment_nonce(owner);

            match result {
                Ok(_) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 100_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("register model failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        let mut pending_new_tasks: Vec<aztibase_consensus::InferenceTask> = Vec::new();

        for (requester, model_id, input_hash, reward, deadline_round, nonce) in &post_tasks {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(requester);
            preimage.extend_from_slice(model_id.as_bytes());
            preimage.extend_from_slice(input_hash);
            let tx_hash = hash(&preimage);

            let req_nonce = state.nonce(requester);
            if *nonce != req_nonce {
                state.increment_nonce(requester);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {req_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
            let model_exists =
                ModelRegistry::get(&registry_acct.storage, model_id).is_some_and(|m| m.active);

            if !model_exists {
                state.increment_nonce(requester);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("model not registered: {model_id}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let balance = state.balance(requester);
            if balance < *reward {
                state.increment_nonce(requester);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for task reward".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let new_balance = state.balance(requester) - *reward;
            state.set_balance(requester, new_balance);

            let task = aztibase_consensus::InferenceTask::new(
                model_id.clone(),
                *input_hash,
                *requester,
                *reward,
                *deadline_round,
            );

            let task_key = format!("task:{}", hex::encode(task.task_id));
            let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
            match postcard::to_allocvec(&task) {
                Ok(encoded) => {
                    registry_acct.storage.insert(task_key.into_bytes(), encoded);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Failed to serialize inference task, skipping");
                    continue;
                }
            }

            state.increment_nonce(requester);
            pending_new_tasks.push(task.clone());

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 42_000,
                contract_address: None,
                error: None,
                inference_hash: Some(task.task_id),
                anomaly_score: 0.0,
            });
        }

        // Insert successfully posted tasks into the live TaskPool and assign validators.
        if !pending_new_tasks.is_empty() {
            let commitments_guard = self.compute_commitments.read().await;
            let scorer = aztibase_consensus::SlidingWindowPoUWScore::new(1000.0, 10.0);
            let mut pool = self.task_pool.write().await;
            for mut task in pending_new_tasks {
                let candidates = commitments_guard.validators_for_model(&task.model_id);
                if !candidates.is_empty() {
                    task.assigned_validator =
                        TaskAssigner::select_validator(&candidates, &scorer, 100);
                }
                pool.insert(task);
            }
            drop(commitments_guard);
            self.pending_task_count
                .store(pool.len() as u64, std::sync::atomic::Ordering::Relaxed);
        }

        // Execute SubmitAttestation transactions.
        for (validator, task_id, result_hash, compute_units, signature, nonce) in
            &submit_attestations
        {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(validator);
            preimage.extend_from_slice(task_id);
            preimage.extend_from_slice(result_hash);
            let tx_hash = hash(&preimage);

            let val_nonce = state.nonce(validator);
            if *nonce != val_nonce {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {val_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Verify Ed25519 signature over attestation_hash.
            // Use the raw pubkey from the SignedTx envelope (the validator address
            // is a BLAKE3 hash of the pubkey and can't be reversed).
            let raw_pubkey = sender_pubkeys.get(validator);
            let att = InferenceAttestation::new(
                *task_id,
                *result_hash,
                *compute_units,
                *validator,
                signature.clone(),
            );
            let att_hash = att.attestation_hash();
            let sig_valid = raw_pubkey
                .and_then(aztibase_core::PublicKey::from_bytes)
                .is_some_and(|pk| pk.verify(&att_hash, signature));

            if !sig_valid {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 25_000,
                    contract_address: None,
                    error: Some("invalid attestation signature".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Check task exists in pool and validate assigned validator.
            let task_check = {
                let pool = self.task_pool.read().await;
                pool.get(task_id).map(|t| t.assigned_validator)
            };

            match task_check {
                Some(Some(assigned)) if assigned != *validator => {
                    state.increment_nonce(validator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 25_000,
                        contract_address: None,
                        error: Some(format!(
                            "validator not assigned to this task (assigned: 0x{})",
                            hex::encode(&assigned[..4])
                        )),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    continue;
                }
                Some(_) => {}
                None => {
                    state.increment_nonce(validator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 25_000,
                        contract_address: None,
                        error: Some(format!("task not found: 0x{}", hex::encode(&task_id[..4]))),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    continue;
                }
            }

            // Per-task attestation cap
            let task_atts = self.attestation_buffer.entry(*task_id).or_default();
            if task_atts.len() >= MAX_ATTESTATIONS_PER_TASK {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 25_000,
                    contract_address: None,
                    error: Some("attestation buffer full for task".to_string()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }
            task_atts.push(att);

            // Total buffer cap: evict oldest task entries
            if self.attestation_buffer.len() > MAX_ATTESTATION_BUFFER_TASKS {
                let oldest_key = self.attestation_buffer.keys().next().copied();
                if let Some(key) = oldest_key {
                    self.attestation_buffer.remove(&key);
                }
            }

            state.increment_nonce(validator);

            // Check quorum via TaskSettlement
            let atts = &self.attestation_buffer[task_id];
            let task_opt = {
                let pool = self.task_pool.read().await;
                pool.get(task_id).cloned()
            };
            if let Some(task) = task_opt {
                let result = TaskSettlement::settle(&task, atts, &self.attestation_aggregator);
                if let SettlementResult::Settled {
                    result_hash,
                    payouts,
                    ..
                } = result
                {
                    {
                        let mut pool = self.task_pool.write().await;
                        pool.remove(task_id);
                    }

                    for (validator_id, payout) in &payouts {
                        let prev = state.balance(validator_id);
                        state.set_balance(validator_id, prev + payout);
                    }

                    let task_key = format!("task:{}", hex::encode(task.task_id));
                    let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
                    registry_acct.storage.remove(&task_key.into_bytes());

                    self.pending_task_count.store(
                        {
                            let pool = self.task_pool.read().await;
                            pool.len() as u64
                        },
                        std::sync::atomic::Ordering::Relaxed,
                    );

                    tracing::info!(
                        task_id = %short_hex(&task.task_id),
                        result_hash = %short_hex(&result_hash),
                        validators = payouts.len(),
                        reward = task.reward,
                        "Task settled — quorum reached"
                    );

                    self.attestation_buffer.remove(task_id);
                }
            }

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 50_000,
                contract_address: None,
                error: None,
                inference_hash: Some(*task_id),
                anomaly_score: 0.0,
            });
        }

        // Execute CommitCompute transactions.
        for (validator, supported_models, committed_stake, bls_pubkey, bls_pop, nonce) in
            &commit_computes
        {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(validator);
            for m in supported_models {
                preimage.extend_from_slice(m.as_bytes());
            }
            preimage.extend_from_slice(&committed_stake.to_le_bytes());
            let tx_hash = hash(&preimage);

            let val_nonce = state.nonce(validator);
            if *nonce != val_nonce {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {val_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Verify BLS proof-of-possession (rogue-key attack prevention).
            let pk_arr: Option<[u8; 48]> = bls_pubkey.as_slice().try_into().ok();
            let pop_arr: Option<[u8; 96]> = bls_pop.as_slice().try_into().ok();
            let pop_valid = match (pk_arr, pop_arr) {
                (Some(pk_bytes), Some(pop_bytes)) => {
                    match (
                        aztibase_core::BlsPublicKey::from_bytes(pk_bytes),
                        aztibase_core::BlsSignature::from_bytes(pop_bytes),
                    ) {
                        (Some(pk), Some(sig)) => {
                            aztibase_core::verify_proof_of_possession(&pk, &sig)
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
            if !pop_valid {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("invalid BLS proof-of-possession".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Validate all supported_models exist in the registry.
            let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
            let mut all_models_valid = true;
            for model_id in supported_models {
                let model_exists =
                    ModelRegistry::get(&registry_acct.storage, model_id).is_some_and(|m| m.active);
                if !model_exists {
                    all_models_valid = false;
                    break;
                }
            }
            if !all_models_valid {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("one or more supported_models not registered".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Refund previous commitment stake if overwriting.
            let mut store_guard = self.compute_commitments.write().await;
            if let Some(prev) = store_guard.get(validator).filter(|p| p.active) {
                let refund = prev.committed_stake;
                let cur = state.balance(validator);
                state.set_balance(validator, cur + refund);
            }

            let balance = state.balance(validator);
            if balance < *committed_stake {
                drop(store_guard);
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for compute stake bond".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let new_balance = balance - *committed_stake;
            state.set_balance(validator, new_balance);

            let commitment = aztibase_consensus::ComputeCommitment::new(
                *validator,
                supported_models.clone(),
                *committed_stake,
                bls_pubkey.clone(),
                self.current_round,
            );
            store_guard.register(commitment);
            drop(store_guard);
            state.increment_nonce(validator);

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 75_000,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        // Execute DeregisterCompute transactions.
        for (validator, nonce) in &deregister_computes {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(validator);
            preimage.extend_from_slice(b"deregister_compute");
            let tx_hash = hash(&preimage);

            let val_nonce = state.nonce(validator);
            if *nonce != val_nonce {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {val_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut store_guard = self.compute_commitments.write().await;
            match store_guard.deregister(validator) {
                Some(prev) => {
                    let refund = prev.committed_stake;
                    let cur = state.balance(validator);
                    state.set_balance(validator, cur + refund);
                    drop(store_guard);
                    state.increment_nonce(validator);

                    tracing::info!(
                        validator = %short_hex(validator),
                        refund = refund,
                        "Compute provider deregistered"
                    );

                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 50_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                None => {
                    drop(store_guard);
                    state.increment_nonce(validator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 25_000,
                        contract_address: None,
                        error: Some("no active compute commitment to deregister".into()),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute DeregisterModel transactions.
        for (owner, model_id, nonce) in &deregister_models {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(owner);
            preimage.extend_from_slice(model_id.as_bytes());
            preimage.extend_from_slice(b"deregister");
            let tx_hash = hash(&preimage);

            let owner_nonce = state.nonce(owner);
            if *nonce != owner_nonce {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {owner_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            // Check if any pending tasks exist for this model.
            let has_pending = {
                let pool = self.task_pool.read().await;
                !pool.tasks_for_model(model_id).is_empty()
            };
            if has_pending {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 25_000,
                    contract_address: None,
                    error: Some(format!(
                        "cannot deregister model with pending tasks: {model_id}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let registry_acct = state.get_mut(&MODEL_REGISTRY_ADDRESS);
            let result = ModelRegistry::deregister(&mut registry_acct.storage, model_id, owner);
            state.increment_nonce(owner);

            match result {
                Ok(()) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 25_000,
                        contract_address: None,
                        error: Some(format!("deregister model failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute CreateProposal transactions.
        for (proposer, description, param_key, param_value, voting_period, nonce) in
            &create_proposals
        {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(proposer);
            preimage.extend_from_slice(param_key.as_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let proposer_nonce = state.nonce(proposer);
            if *nonce != proposer_nonce {
                state.increment_nonce(proposer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {proposer_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let proposal_id = hash(&preimage);
            let snapshot_balances: std::collections::HashMap<[u8; 32], u128> = state
                .iter_accounts()
                .filter(|(_, acct)| acct.balance > 0)
                .map(|(addr, acct)| (*addr, acct.balance))
                .collect();
            let mut gov = self.governance.write().await;
            let result = gov.create_proposal(CreateProposalParams {
                id: proposal_id,
                proposer: *proposer,
                description: description.clone(),
                param_key: param_key.clone(),
                param_value: param_value.clone(),
                current_round: self.current_round,
                voting_period: *voting_period,
                snapshot_balances,
            });
            drop(gov);
            state.increment_nonce(proposer);

            match result {
                Ok(()) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 100_000,
                        contract_address: None,
                        error: None,
                        inference_hash: Some(proposal_id),
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("create proposal failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute CastVote transactions.
        for (voter, proposal_id, approve, nonce) in &cast_votes {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(voter);
            preimage.extend_from_slice(proposal_id);
            let tx_hash = hash(&preimage);

            let voter_nonce = state.nonce(voter);
            if *nonce != voter_nonce {
                state.increment_nonce(voter);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {voter_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let stake_weight = state.balance(voter);
            let mut gov = self.governance.write().await;
            let result = gov.cast_vote(*voter, *proposal_id, *approve, stake_weight);
            drop(gov);
            state.increment_nonce(voter);

            match result {
                Ok(()) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 40_000,
                        contract_address: None,
                        error: None,
                        inference_hash: Some(*proposal_id),
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("cast vote failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Finalize expired proposals at the end of each batch.
        {
            let mut gov = self.governance.write().await;
            let finalized = gov.finalize_expired(self.current_round);
            for proposal in &finalized {
                tracing::info!(
                    proposal_id = %short_hex(&proposal.id),
                    status = ?proposal.status,
                    param_key = %proposal.param_key,
                    "Proposal finalized"
                );
            }

            // Execute passed proposals: apply parameter changes to ChainParams.
            let passed = gov.passed_unexecuted();
            if !passed.is_empty() {
                let mut params = self.chain_params.write().await;
                for proposal in &passed {
                    match params.set_from_str(&proposal.param_key, &proposal.param_value) {
                        Ok(old_value) => {
                            gov.mark_executed(&proposal.id);
                            tracing::info!(
                                proposal_id = %short_hex(&proposal.id),
                                param_key = %proposal.param_key,
                                old_value = %old_value,
                                new_value = %proposal.param_value,
                                "Governance proposal executed"
                            );
                        }
                        Err(e) => {
                            gov.mark_executed(&proposal.id);
                            tracing::warn!(
                                proposal_id = %short_hex(&proposal.id),
                                param_key = %proposal.param_key,
                                error = %e,
                                "Governance proposal execution failed"
                            );
                        }
                    }
                }
            }
        }

        // Execute RegisterValidator transactions.
        for (registrant, amount, nonce, ed25519_pubkey) in &register_validators {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(registrant);
            preimage.extend_from_slice(&amount.to_le_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let reg_nonce = state.nonce(registrant);
            if *nonce != reg_nonce {
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {reg_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            if *amount < MIN_VALIDATOR_STAKE {
                state.increment_nonce(registrant);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "minimum stake is {MIN_VALIDATOR_STAKE}, got {amount}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let balance = state.balance(registrant);
            if balance < *amount {
                state.increment_nonce(registrant);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for validator registration".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            let result = staking.register_validator_with_keys(
                *registrant,
                *amount,
                MIN_VALIDATOR_STAKE,
                MAX_VALIDATOR_STAKE_CAP,
                self.batch_count.load(std::sync::atomic::Ordering::Relaxed),
                *ed25519_pubkey,
                None,
            );
            drop(staking);

            match result {
                Ok(()) => {
                    state.set_balance(registrant, balance - *amount);
                    state.increment_nonce(registrant);
                    self.consensus_addrs.insert(*registrant);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 100_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    tracing::info!(
                        validator = %short_hex(registrant),
                        stake = amount,
                        "New validator registered via RegisterValidator tx"
                    );
                }
                Err(e) => {
                    state.increment_nonce(registrant);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("registration failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute Stake transactions.
        for (staker, amount, nonce, _ed25519_pubkey) in &stakes {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(staker);
            preimage.extend_from_slice(&amount.to_le_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let staker_nonce = state.nonce(staker);
            if *nonce != staker_nonce {
                state.increment_nonce(staker);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {staker_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let balance = state.balance(staker);
            if balance < *amount {
                state.increment_nonce(staker);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for stake".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            let result = if staking.get_validator(staker).is_some() {
                staking.add_stake(*staker, *amount, MAX_VALIDATOR_STAKE_CAP)
            } else {
                Err(aztibase_execution::StakingError::ValidatorNotFound)
            };
            drop(staking);

            match result {
                Ok(()) => {
                    state.set_balance(staker, balance - *amount);
                    state.increment_nonce(staker);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(staker);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("stake failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute Unstake transactions.
        for (staker, amount, nonce) in &unstakes {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(staker);
            preimage.extend_from_slice(&amount.to_le_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let staker_nonce = state.nonce(staker);
            if *nonce != staker_nonce {
                state.increment_nonce(staker);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {staker_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            let result = staking.begin_unstake(
                *staker,
                *amount,
                MIN_VALIDATOR_STAKE,
                self.current_round,
                UNBONDING_ROUNDS,
            );
            drop(staking);

            match result {
                Ok(()) => {
                    state.increment_nonce(staker);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(staker);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("unstake failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute Delegate transactions.
        for (delegator, validator_id, amount, nonce) in &delegates {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(delegator);
            preimage.extend_from_slice(validator_id);
            preimage.extend_from_slice(&amount.to_le_bytes());
            let tx_hash = hash(&preimage);

            let del_nonce = state.nonce(delegator);
            if *nonce != del_nonce {
                state.increment_nonce(delegator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {del_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let balance = state.balance(delegator);
            if balance < *amount {
                state.increment_nonce(delegator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for delegation".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            let result = staking.delegate(
                *delegator,
                *validator_id,
                *amount,
                MAX_VALIDATOR_STAKE_CAP,
                self.current_round,
            );
            drop(staking);

            match result {
                Ok(()) => {
                    state.set_balance(delegator, balance - *amount);
                    state.increment_nonce(delegator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(delegator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("delegate failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        // Execute Undelegate transactions.
        for (delegator, nonce) in &undelegates {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(delegator);
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let del_nonce = state.nonce(delegator);
            if *nonce != del_nonce {
                state.increment_nonce(delegator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {del_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            let result = staking.begin_undelegate(*delegator, self.current_round, UNBONDING_ROUNDS);
            drop(staking);

            match result {
                Ok(_amount) => {
                    state.increment_nonce(delegator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(delegator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("undelegate failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        for (owner, agent, per_tx_limit, per_epoch_limit, allowed_tx_kinds, expiry_epoch, nonce) in
            &set_agent_policies
        {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(owner);
            preimage.extend_from_slice(agent);
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let owner_nonce = state.nonce(owner);
            if *nonce != owner_nonce {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {owner_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let agent_type = state.account_type(agent);
            if agent_type != AccountType::AIAgent {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("target address is not an AI agent".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let acct = state.get(agent);
            let is_owner = acct.is_some_and(|a| {
                a.storage
                    .get(b"creator".as_slice())
                    .is_some_and(|v| v.as_slice() == owner)
            });
            if !is_owner {
                // Check model_id creator pattern: the agent was created by owner
                // if the agent's address was derived from the owner's address
                let expected_agent = aztibase_execution::compute_contract_address(owner, 0);
                let is_derived_owner = *agent == expected_agent
                    || (0..100u64)
                        .any(|n| aztibase_execution::compute_contract_address(owner, n) == *agent);
                if !is_derived_owner {
                    state.increment_nonce(owner);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some("only the agent creator can set policy".into()),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    continue;
                }
            }

            if allowed_tx_kinds.len() > 32 {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("too many allowed tx kinds".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let policy = aztibase_execution::AgentPolicy {
                owner: *owner,
                per_tx_limit: *per_tx_limit,
                per_epoch_limit: *per_epoch_limit,
                allowed_tx_kinds: allowed_tx_kinds.clone(),
                expiry_epoch: *expiry_epoch,
            };

            let mut aps = self.agent_policy_store.write().await;
            aps.set_policy(*agent, policy);
            drop(aps);

            state.increment_nonce(owner);
            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 60_000,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (agent, inner_tx_kind, to, value, _data, nonce) in &agent_executes {
            let mut preimage = Vec::new();
            preimage.extend_from_slice(agent);
            preimage.extend_from_slice(to);
            preimage.extend_from_slice(&value.to_le_bytes());
            preimage.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&preimage);

            let agent_nonce = state.nonce(agent);
            if *nonce != agent_nonce {
                state.increment_nonce(agent);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {agent_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let agent_type = state.account_type(agent);
            if agent_type != AccountType::AIAgent {
                state.increment_nonce(agent);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("sender is not an AI agent".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let current_epoch = self.current_round / 1000;
            let mut aps = self.agent_policy_store.write().await;
            let spend_result = aps.validate_spend(agent, *value, *inner_tx_kind, current_epoch);
            drop(aps);

            if let Err(e) = spend_result {
                state.increment_nonce(agent);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("agent policy violation: {e}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let agent_balance = state.balance(agent);
            if agent_balance < *value {
                state.increment_nonce(agent);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("agent has insufficient balance".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            state.set_balance(agent, agent_balance - *value);
            let recipient_balance = state.balance(to);
            state.set_balance(to, recipient_balance.saturating_add(*value));
            state.increment_nonce(agent);

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 80_000,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        // L2 bridge transactions.
        let current_batch_idx = self.batch_count.load(std::sync::atomic::Ordering::Relaxed);

        for (owner, l2_chain_id, name, sequencer_set, bridge_address, nonce) in &register_l2s {
            let mut buf = Vec::new();
            buf.extend_from_slice(owner);
            buf.extend_from_slice(l2_chain_id);
            buf.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&buf);

            let owner_nonce = state.nonce(owner);
            if *nonce != owner_nonce {
                state.increment_nonce(owner);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!(
                        "nonce mismatch: expected {owner_nonce}, got {nonce}"
                    )),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let reg = L2Registration {
                owner: *owner,
                l2_chain_id: *l2_chain_id,
                name: name.clone(),
                sequencer_set: sequencer_set.clone(),
                bridge_address: *bridge_address,
            };
            let mut registry = self.l2_registry.write().await;
            match registry.register(reg) {
                Ok(()) => {
                    state.increment_nonce(owner);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 100_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(owner);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("register L2 failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        for (validator, new_pubkey, nonce) in &rotate_keys {
            let mut buf = Vec::new();
            buf.extend_from_slice(validator);
            buf.extend_from_slice(new_pubkey);
            buf.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&buf);

            let v_nonce = state.nonce(validator);
            if *nonce != v_nonce {
                state.increment_nonce(validator);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {v_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let mut staking = self.staking_store.write().await;
            match staking.rotate_key(*validator, *new_pubkey) {
                Ok(()) => {
                    state.increment_nonce(validator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 60_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    tracing::info!(
                        old = %hex::encode(validator),
                        new = %hex::encode(new_pubkey),
                        "Validator key rotated"
                    );
                }
                Err(e) => {
                    state.increment_nonce(validator);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("key rotation failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        for (sequencer, l2_chain_id, sr, bdh, l2_start, l2_end, nonce) in &anchor_l2_states {
            let mut buf = Vec::new();
            buf.extend_from_slice(sequencer);
            buf.extend_from_slice(l2_chain_id);
            buf.extend_from_slice(sr);
            buf.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&buf);

            let seq_nonce = state.nonce(sequencer);
            if *nonce != seq_nonce {
                state.increment_nonce(sequencer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {seq_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let registry = self.l2_registry.read().await;
            if !registry.is_sequencer(l2_chain_id, sequencer) {
                drop(registry);
                state.increment_nonce(sequencer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("sender not in sequencer set".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }
            drop(registry);

            let anchor = L2Anchor {
                sequencer: *sequencer,
                state_root: *sr,
                batch_data_hash: *bdh,
                l2_block_start: *l2_start,
                l2_block_end: *l2_end,
                l1_batch_index: current_batch_idx,
            };
            let mut anchor_store = self.l2_anchor_store.write().await;
            match anchor_store.anchor(l2_chain_id, anchor) {
                Ok(()) => {
                    state.increment_nonce(sequencer);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: true,
                        gas_used: 80_000,
                        contract_address: None,
                        error: None,
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
                Err(e) => {
                    state.increment_nonce(sequencer);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some(format!("anchor L2 state failed: {e}")),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                }
            }
        }

        for (depositor, l2_chain_id, _l2_recipient, amount, nonce) in &bridge_deposits {
            let mut buf = Vec::new();
            buf.extend_from_slice(depositor);
            buf.extend_from_slice(l2_chain_id);
            buf.extend_from_slice(&amount.to_le_bytes());
            buf.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&buf);

            let dep_nonce = state.nonce(depositor);
            if *nonce != dep_nonce {
                state.increment_nonce(depositor);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {dep_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            if *amount == 0 {
                state.increment_nonce(depositor);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("bridge amount must be non-zero".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let registry = self.l2_registry.read().await;
            if registry.get(l2_chain_id).is_none() {
                drop(registry);
                state.increment_nonce(depositor);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("L2 chain not registered".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }
            drop(registry);

            let balance = state.balance(depositor);
            if balance < *amount {
                state.increment_nonce(depositor);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("insufficient balance for bridge deposit".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            state.set_balance(depositor, balance - *amount);
            let mut escrow = self.bridge_escrow.write().await;
            escrow.lock(l2_chain_id, depositor, *amount);
            state.increment_nonce(depositor);

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 50_000,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (withdrawer, l2_chain_id, amount, l2_burn_proof, l2_sr, nonce) in &bridge_withdraws {
            let mut buf = Vec::new();
            buf.extend_from_slice(withdrawer);
            buf.extend_from_slice(l2_chain_id);
            buf.extend_from_slice(&amount.to_le_bytes());
            buf.extend_from_slice(&nonce.to_le_bytes());
            let tx_hash = hash(&buf);

            let w_nonce = state.nonce(withdrawer);
            if *nonce != w_nonce {
                state.increment_nonce(withdrawer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("nonce mismatch: expected {w_nonce}, got {nonce}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            if *amount == 0 {
                state.increment_nonce(withdrawer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some("bridge amount must be non-zero".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let proof_hash = hash(l2_burn_proof);
            let mut withdraw_proofs = self.bridge_withdraw_proofs.write().await;
            if let Err(e) = withdraw_proofs.mark_used(proof_hash, *withdrawer) {
                drop(withdraw_proofs);
                state.increment_nonce(withdrawer);
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 21_000,
                    contract_address: None,
                    error: Some(format!("bridge withdraw failed: {e}")),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }
            drop(withdraw_proofs);

            let anchor_store = self.l2_anchor_store.read().await;
            let finalized = anchor_store.latest_finalized(l2_chain_id, current_batch_idx);
            match finalized {
                Some(anchor) if anchor.state_root == *l2_sr => {}
                Some(_) => {
                    drop(anchor_store);
                    state.increment_nonce(withdrawer);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some("state root does not match latest finalized anchor".into()),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    continue;
                }
                None => {
                    drop(anchor_store);
                    state.increment_nonce(withdrawer);
                    exec_receipts.push(ExecutionReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 21_000,
                        contract_address: None,
                        error: Some("no finalized anchor for this L2".into()),
                        inference_hash: None,
                        anomaly_score: 0.0,
                    });
                    continue;
                }
            }
            drop(anchor_store);

            let mut escrow = self.bridge_escrow.write().await;
            let _ = escrow.unlock(l2_chain_id, withdrawer, *amount);
            drop(escrow);

            let prev_balance = state.balance(withdrawer);
            state.set_balance(withdrawer, prev_balance.saturating_add(*amount));
            state.increment_nonce(withdrawer);

            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 70_000,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        for (validator, recipient, amount) in &faucet_drips {
            let mut buf = Vec::new();
            buf.extend_from_slice(recipient);
            buf.extend_from_slice(&amount.to_le_bytes());
            let tx_hash = hash(&buf);

            if !self.faucet_enabled {
                tracing::warn!(
                    recipient = %short_hex(recipient),
                    "FaucetDrip rejected — faucet disabled (mainnet)"
                );
                exec_receipts.push(ExecutionReceipt {
                    tx_hash,
                    success: false,
                    gas_used: 0,
                    contract_address: None,
                    error: Some("faucet disabled on mainnet".into()),
                    inference_hash: None,
                    anomaly_score: 0.0,
                });
                continue;
            }

            let prev = state.balance(recipient);
            state.set_balance(recipient, prev.saturating_add(*amount));
            state.increment_nonce(validator);
            exec_receipts.push(ExecutionReceipt {
                tx_hash,
                success: true,
                gas_used: 0,
                contract_address: None,
                error: None,
                inference_hash: None,
                anomaly_score: 0.0,
            });
        }

        // Phase 2.5: Score each executed tx for anomalous behavior.
        for (i, tx) in executable.iter().enumerate() {
            if i >= exec_receipts.len() {
                break;
            }
            let features = extract_tx_features(tx);
            exec_receipts[i].anomaly_score = self.anomaly_scorer.score(&features);
        }

        // Phase 3: Refund unused gas and collect actual fees.
        for (exec_idx, &orig_idx) in escrowed_indices.iter().enumerate() {
            if exec_idx >= exec_receipts.len() {
                break;
            }
            let gas_used = exec_receipts[exec_idx].gas_used;
            if let Some(ref esc) = escrows[orig_idx] {
                let actual_fee = refund_unused(&mut state, esc, gas_used);
                total_fees_burned += actual_fee;
            }
        }

        // Merge nonce-rejection + escrow-failure + execution receipts.
        receipts.extend(exec_receipts);
        receipts.extend(nonce_reject_receipts);

        // Update base fee using governance-controlled parameters.
        let total_gas: u64 = receipts.iter().map(|r| r.gas_used).sum();
        {
            let cp = self.chain_params.read().await;
            self.base_fee_calculator.update_with_params(
                total_gas,
                cp.get_u64("target_gas_per_batch").unwrap_or(15_000_000),
                cp.get_u64("base_fee_change_denom").unwrap_or(8),
                cp.get_u64("base_fee_floor").unwrap_or(1),
                cp.get_u64("base_fee_ceiling").unwrap_or(1_000_000_000),
            );
        }
        self.base_fee.store(
            self.base_fee_calculator.base_fee(),
            std::sync::atomic::Ordering::Relaxed,
        );

        let state_root = state.state_root();

        if let Some(ref store) = self.store {
            flush_state(store, &state)
                .map_err(|e| anyhow::anyhow!("fatal: flush_state failed: {e}"))?;
            store_batch_root(store, &batch.anchor_hash, &state_root)
                .map_err(|e| anyhow::anyhow!("fatal: store_batch_root failed: {e}"))?;
            store_receipts(store, &receipts)
                .map_err(|e| anyhow::anyhow!("fatal: store_receipts failed: {e}"))?;
            store_base_fee(store, self.base_fee_calculator.base_fee())
                .map_err(|e| anyhow::anyhow!("fatal: store_base_fee failed: {e}"))?;

            flush_staking(store, &*self.staking_store.read().await)
                .map_err(|e| anyhow::anyhow!("fatal: flush_staking failed: {e}"))?;
            flush_governance(store, &*self.governance.read().await)
                .map_err(|e| anyhow::anyhow!("fatal: flush_governance failed: {e}"))?;
            flush_emission(store, &*self.emission_tracker.read().await)
                .map_err(|e| anyhow::anyhow!("fatal: flush_emission failed: {e}"))?;
            flush_chain_params(store, &*self.chain_params.read().await)
                .map_err(|e| anyhow::anyhow!("fatal: flush_chain_params failed: {e}"))?;
            flush_agent_policies(store, &*self.agent_policy_store.read().await)
                .map_err(|e| anyhow::anyhow!("fatal: flush_agent_policies failed: {e}"))?;
            flush_bridge_stores(
                store,
                &*self.l2_registry.read().await,
                &*self.l2_anchor_store.read().await,
                &*self.bridge_escrow.read().await,
                &*self.bridge_withdraw_proofs.read().await,
            )
            .map_err(|e| anyhow::anyhow!("fatal: flush_bridge_stores failed: {e}"))?;

            let batch_number = self.batch_count.load(std::sync::atomic::Ordering::Relaxed);
            let _ = aztibase_execution::store_batch_index(store, batch_number, &batch.anchor_hash);

            let tx_hashes: Vec<[u8; 32]> = receipts.iter().map(|r| r.tx_hash).collect();
            let _ = aztibase_execution::store_batch_txs(store, &batch.anchor_hash, &tx_hashes);

            if batch_number > 0 && batch_number.is_multiple_of(CHECKPOINT_INTERVAL) {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let cp = Checkpoint::new(batch_number, state_root, None, now_ms);
                if let Ok(data) = postcard::to_allocvec(&cp) {
                    let _ = aztibase_execution::store_checkpoint_raw(store, batch_number, &data);
                    tracing::info!(
                        batch = batch_number,
                        state_root = %format!("{:02x}{:02x}{:02x}{:02x}",
                            state_root[0], state_root[1], state_root[2], state_root[3]),
                        "Weak subjectivity checkpoint stored"
                    );
                }
            }

            for (h, data) in &all_tx_store_data {
                let _ = aztibase_execution::store_transaction(store, h, data);
            }

            if !self.archive {
                let cp = self.chain_params.read().await;
                let max_receipts = cp.get_u64("max_stored_receipts").unwrap_or(100_000);
                let max_txs = cp.get_u64("max_stored_txs").unwrap_or(500_000);
                let max_roots = cp.get_u64("max_stored_batch_roots").unwrap_or(100_000);
                drop(cp);
                let _ = store.evict_oldest(aztibase_storage::RECEIPTS_TABLE, max_receipts);
                let _ = store.evict_oldest(aztibase_storage::TX_TABLE, max_txs);
                let _ = store.evict_oldest(aztibase_storage::BATCH_ROOTS_TABLE, max_roots);
            }
        }

        // Process pending slash events from consensus.
        {
            let mut pending_slashes = Vec::new();
            while let Ok(event) = self.slash_rx.try_recv() {
                pending_slashes.push(event);
            }
            if !pending_slashes.is_empty() {
                let mut staking = self.staking_store.write().await;
                for event in &pending_slashes {
                    let slash_bps = match event.offense {
                        OffenseType::Equivocation => aztibase_execution::EQUIVOCATION_SLASH_BPS,
                        OffenseType::Downtime => aztibase_execution::DOWNTIME_SLASH_BPS,
                    };
                    if let (Some(store), Some(eh), Some(dh)) = (
                        self.store.as_ref(),
                        event.existing_hash,
                        event.duplicate_hash,
                    ) {
                        let _ = aztibase_execution::store_equivocation_proof(
                            store,
                            event.round,
                            &event.validator_id,
                            &eh,
                            &dh,
                        );
                    }
                    match staking.slash_validator(
                        event.validator_id,
                        slash_bps,
                        event.offense,
                        event.round,
                    ) {
                        Ok(slashed) => {
                            tracing::warn!(
                                validator = %short_hex(&event.validator_id),
                                offense = ?event.offense,
                                slashed_amount = slashed,
                                "Validator slashed"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                validator = %short_hex(&event.validator_id),
                                error = %e,
                                "Slash failed — validator not found"
                            );
                        }
                    }
                }
            }
        }

        // Process unbonding queue: release matured entries back to account balances.
        {
            let mut staking = self.staking_store.write().await;
            let released = staking.process_unbonding(self.current_round);
            drop(staking);
            for (owner, amount) in released {
                let prev = state.balance(&owner);
                state.set_balance(&owner, prev + amount);
            }
        }

        // Epoch boundary: distribute emission rewards to stakers.
        {
            let epoch_length = {
                let cp = self.chain_params.read().await;
                cp.get_u64("epoch_length").unwrap_or(1_000)
            };
            if epoch_length > 0
                && self.current_round > 0
                && self.current_round.is_multiple_of(epoch_length)
            {
                let (epoch_num, dist) = {
                    let mut tracker = self.emission_tracker.write().await;
                    let epoch = tracker.current_epoch;
                    (epoch, tracker.advance_epoch())
                };
                if let Some(dist) = dist {
                    let commission_bps = {
                        let cp = self.chain_params.read().await;
                        cp.get_u64("validator_commission_bps").unwrap_or(1000) as u32
                    };
                    let mut staking = self.staking_store.write().await;
                    let credits =
                        staking.distribute_epoch_rewards(dist.validator_rewards, commission_bps);
                    let active_set = staking.active_set_snapshot(MIN_VALIDATOR_STAKE);

                    {
                        let mut tracker = self.emission_tracker.write().await;
                        tracker.record_reward_event(aztibase_execution::RewardEvent {
                            epoch: epoch_num,
                            round: self.current_round,
                            total_emission: dist.total(),
                            validator_pool: dist.validator_rewards,
                            credits: credits.clone(),
                        });
                    }
                    drop(staking);
                    for (addr, amount) in credits {
                        let prev = state.balance(&addr);
                        state.set_balance(&addr, prev + amount);
                    }
                    // Downtime slashing: validators who didn't participate get slashed.
                    let inactive_validators: Vec<[u8; 32]> = active_set
                        .iter()
                        .filter(|(vid, _)| !self.epoch_participation.contains(vid))
                        .map(|(vid, _)| *vid)
                        .collect();
                    if !inactive_validators.is_empty() {
                        let mut staking = self.staking_store.write().await;
                        for vid in &inactive_validators {
                            if let Ok(slashed) = staking.slash_validator(
                                *vid,
                                aztibase_execution::DOWNTIME_SLASH_BPS,
                                OffenseType::Downtime,
                                self.current_round,
                            ) {
                                tracing::warn!(
                                    validator = %short_hex(vid),
                                    slashed_amount = slashed,
                                    "Downtime slash — validator inactive during epoch"
                                );
                            }
                        }
                    }

                    self.epoch_participation.clear();

                    // Propagate updated validator set to consensus engine.
                    // Only include validators already known to consensus — prevents
                    // adding staked-but-offline validators that would cause phantom
                    // parent desync (see epoch-boundary stall bug).
                    if let Some(ref ctx) = self.consensus_tx {
                        let mut new_vs = aztibase_consensus::ValidatorSet::new();
                        let staking_read = self.staking_store.read().await;
                        for (vid, stake) in &active_set {
                            if !self.consensus_addrs.contains(vid) {
                                tracing::debug!(
                                    validator = %short_hex(vid),
                                    "Skipping validator not in consensus set"
                                );
                                continue;
                            }
                            new_vs.add(*vid, *stake);
                            if let Some(vs) = staking_read.get_validator(vid)
                                && let Some(pk) = vs.ed25519_pubkey
                            {
                                new_vs.set_ed25519_key(vid, pk);
                            }
                        }
                        drop(staking_read);
                        let _ = ctx.try_send(
                            aztibase_consensus::ConsensusInput::UpdateValidatorSet(new_vs),
                        );
                    }

                    tracing::info!(
                        round = self.current_round,
                        active_validators = active_set.len(),
                        validator_pool = %dist.validator_rewards,
                        downtime_slashed = inactive_validators.len(),
                        "Epoch boundary — rewards distributed, validator set refreshed"
                    );
                }
            }
        }

        self.batch_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(PipelineResult {
            batch_anchor: batch.anchor_hash,
            state_root,
            transfer_count,
            contract_count,
            routing_errors: errors.len(),
            receipts,
            total_fees_burned,
        })
    }
}

fn extract_tx_features(tx: &TxKind) -> TxFeatures {
    let value = match tx {
        TxKind::Transfer { value, .. }
        | TxKind::EvmCall { value, .. }
        | TxKind::AgentExecute { value, .. } => *value,
        _ => 0,
    };
    let payload_size = match tx {
        TxKind::ContractDeploy { code, .. } | TxKind::EvmDeploy { code, .. } => code.len() as u32,
        TxKind::ContractCall { args_data, .. } => args_data.len() as u32,
        TxKind::EvmCall { calldata, .. } => calldata.len() as u32,
        TxKind::AiInfer { input, .. } => input.len() as u32,
        _ => 0,
    };
    TxFeatures {
        value,
        gas_price: tx.gas_price(),
        gas_limit: tx.gas_limit(),
        payload_size,
        is_contract_deploy: matches!(tx, TxKind::ContractDeploy { .. } | TxKind::EvmDeploy { .. }),
        is_ai_infer: matches!(tx, TxKind::AiInfer { .. }),
    }
}

fn short_hex(bytes: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::{BlsKeypair, Keypair, address_from_pubkey};
    use aztibase_execution::{SignedTx, TxKind, get_batch_root, load_state};
    use aztibase_runtime::TractRuntime;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_pipeline_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn make_sender() -> (Keypair, [u8; 32]) {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        (kp, addr)
    }

    fn make_bls() -> (Vec<u8>, Vec<u8>) {
        let bls = BlsKeypair::generate();
        (
            bls.public_key().as_bytes().to_vec(),
            bls.proof_of_possession().as_bytes().to_vec(),
        )
    }

    fn sign(tx: &TxKind, kp: &Keypair) -> Vec<u8> {
        SignedTx::new(tx.encode(), kp).encode()
    }

    fn make_pipeline(rx: mpsc::Receiver<CommittedBatch>) -> ExecutionPipeline {
        let calculator = BaseFeeCalculator::new(1);
        let (slash_tx_init, slash_rx_init) = mpsc::channel(64);
        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors_set: HashSet::new(),
            executed_anchors_queue: VecDeque::new(),
            result_tx: None,
            ai_runtime: None,
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee())),
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
            task_pool: Arc::new(RwLock::new(TaskPool::new())),
            pending_task_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            current_round: 0,
            attestation_buffer: HashMap::new(),
            attestation_aggregator: AttestationAggregator::new(2),
            compute_commitments: Arc::new(RwLock::new(ComputeCommitmentStore::new())),
            governance: Arc::new(RwLock::new(GovernanceStore::new())),
            chain_params: Arc::new(RwLock::new(ChainParams::defaults())),
            emission_tracker: Arc::new(RwLock::new(EmissionTracker::new(
                aztibase_execution::tokenomics::DEFAULT_EPOCH_LENGTH,
            ))),
            staking_store: Arc::new(RwLock::new(StakingStore::new())),
            slash_rx: slash_rx_init,
            slash_tx: slash_tx_init,
            consensus_tx: None,
            epoch_participation: HashSet::new(),
            consensus_addrs: HashSet::new(),
            agent_policy_store: Arc::new(RwLock::new(AgentPolicyStore::new())),
            l2_registry: Arc::new(RwLock::new(L2Registry::new())),
            l2_anchor_store: Arc::new(RwLock::new(L2AnchorStore::new())),
            bridge_escrow: Arc::new(RwLock::new(BridgeEscrowStore::new())),
            bridge_withdraw_proofs: Arc::new(RwLock::new(BridgeWithdrawProofs::new())),
            archive: false,
            faucet_enabled: true,
        }
    }

    fn make_batch(txs: Vec<Vec<u8>>) -> CommittedBatch {
        CommittedBatch {
            anchor_hash: [0xAA; 32],
            vertex_order: vec![],
            transactions: txs,
        }
    }

    fn make_batch_with_anchor(anchor: [u8; 32], txs: Vec<Vec<u8>>) -> CommittedBatch {
        CommittedBatch {
            anchor_hash: anchor,
            vertex_order: vec![],
            transactions: txs,
        }
    }

    #[tokio::test]
    async fn pipeline_executes_transfers() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 300,
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 0);
        assert_eq!(result.routing_errors, 0);
        let state = pipeline.state.read().await;
        // 1_000_000 - 300 (value) - 21_000 (gas fee) = 978_700
        assert_eq!(state.balance(&alice), 978_700);
        assert_eq!(state.balance(&bob), 300);
        assert_ne!(result.state_root, [0u8; 32]);
    }

    #[tokio::test]
    async fn pipeline_executes_mixed_batch() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 10_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 1,
        };

        let deploy = TxKind::ContractDeploy {
            deployer: alice,
            code: vec![0x00, 0x61, 0x73, 0x6d],
            nonce: 1,
            gas_limit: 1_000_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp), sign(&deploy, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 1);
        assert_eq!(pipeline.state.read().await.balance(&bob), 100);
    }

    #[tokio::test]
    async fn pipeline_handles_routing_errors() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (kp, sender) = make_sender();
        pipeline.state.write().await.set_balance(&sender, 1_000_000);
        let good = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 0,
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&good, &kp), vec![0xFE, 0x00]]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.routing_errors, 1);
    }

    #[tokio::test]
    async fn pipeline_processes_channel() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 200,
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.transfer_count, 1);
        assert_eq!(pipeline.state.read().await.balance(&bob), 200);
    }

    #[tokio::test]
    async fn pipeline_state_persists_across_batches() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let batch1 = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
                gas_price: 1,
            },
            &alice_kp,
        )]);

        let batch2 = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 200,
                nonce: 1,
                gas_price: 1,
            },
            &alice_kp,
        )]);

        pipeline.execute_batch(&batch1).await.unwrap();
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        let state = pipeline.state.read().await;
        // 1_000_000 - 300 - 21_000 - 200 - 21_000 = 957_500
        assert_eq!(state.balance(&alice), 957_500);
        assert_eq!(state.balance(&bob), 500);
        assert_ne!(result.state_root, [0u8; 32]);
    }

    // ── Persistence tests ──────────────────────────────────────────

    #[tokio::test]
    async fn pipeline_flushes_to_redb() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let anchor = [0xBB; 32];
        let state_root;

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 1_000_000);

            let transfer = TxKind::Transfer {
                from: alice,
                to: bob,
                value: 400,
                nonce: 0,
                gas_price: 1,
            };

            let batch = make_batch_with_anchor(anchor, vec![sign(&transfer, &alice_kp)]);
            let result = pipeline.execute_batch(&batch).await.unwrap();
            state_root = result.state_root;
        }

        let store2 = StateStore::open(path.to_str().unwrap()).unwrap();
        let loaded = load_state(&store2).unwrap();
        // 1_000_000 - 400 - 21_000 = 978_600
        assert_eq!(loaded.balance(&alice), 978_600);
        assert_eq!(loaded.balance(&bob), 400);

        let root = get_batch_root(&store2, &anchor).unwrap();
        assert_eq!(root, Some(state_root));

        cleanup(&path);
    }

    #[tokio::test]
    async fn pipeline_rejects_duplicate_anchor() {
        let (tx, rx) = mpsc::channel(16);
        let pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let shared_state = pipeline.shared_state();
        shared_state.write().await.set_balance(&alice, 1_000_000);

        let batch = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
                gas_price: 1,
            },
            &alice_kp,
        )]);

        tx.send(batch.clone()).await.unwrap();
        tx.send(batch).await.unwrap();
        drop(tx);

        pipeline.run().await;

        let state = shared_state.read().await;
        // 1_000_000 - 300 - 21_000 = 978_700
        assert_eq!(state.balance(&alice), 978_700);
        assert_eq!(state.balance(&bob), 300);
    }

    #[tokio::test]
    async fn pipeline_executes_evm_deploy() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (deployer_kp, deployer) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&deployer, 1_000_000_000);

        let init_code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];

        let evm_deploy = TxKind::EvmDeploy {
            deployer,
            code: init_code,
            nonce: 0,
            gas_limit: 1_000_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&evm_deploy, &deployer_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.contract_count, 1);
        assert_eq!(result.receipts.len(), 1);
        assert!(
            result.receipts[0].success,
            "EVM deploy failed: {:?}",
            result.receipts[0].error
        );
        assert!(result.receipts[0].contract_address.is_some());
    }

    #[tokio::test]
    async fn pipeline_executes_evm_deploy_and_call() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (deployer_kp, deployer) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&deployer, 1_000_000_000);

        let init_code = vec![0x60, 0x00, 0x60, 0x00, 0x53, 0x60, 0x01, 0x60, 0x00, 0xf3];

        let evm_deploy = TxKind::EvmDeploy {
            deployer,
            code: init_code,
            nonce: 0,
            gas_limit: 1_000_000,
            gas_price: 1,
        };

        let batch1 = make_batch(vec![sign(&evm_deploy, &deployer_kp)]);
        let result1 = pipeline.execute_batch(&batch1).await.unwrap();
        assert!(result1.receipts[0].success);
        let contract_addr = result1.receipts[0].contract_address.unwrap();

        let evm_call = TxKind::EvmCall {
            caller: deployer,
            contract: contract_addr,
            calldata: vec![],
            nonce: 1,
            gas_limit: 1_000_000,
            value: 0,
            gas_price: 1,
        };

        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&evm_call, &deployer_kp)]);
        let result2 = pipeline.execute_batch(&batch2).await.unwrap();
        assert_eq!(result2.receipts.len(), 1);
        assert!(
            result2.receipts[0].success,
            "EVM call failed: {:?}",
            result2.receipts[0].error
        );
    }

    #[tokio::test]
    async fn pipeline_mixed_wasm_evm_batch() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 1_000_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 500,
            nonce: 0,
            gas_price: 1,
        };

        let evm_deploy = TxKind::EvmDeploy {
            deployer: alice,
            code: vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3],
            nonce: 1,
            gas_limit: 1_000_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![
            sign(&transfer, &alice_kp),
            sign(&evm_deploy, &alice_kp),
        ]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 1);
        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(
            result.receipts[1].success,
            "EVM in mixed batch: {:?}",
            result.receipts[1].error
        );
        assert_eq!(pipeline.state.read().await.balance(&bob), 500);
    }

    #[tokio::test]
    async fn pipeline_recovers_state_on_startup() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 1_000_000);

            let batch = make_batch(vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: [2u8; 32],
                    value: 1500,
                    nonce: 0,
                    gas_price: 1,
                },
                &alice_kp,
            )]);
            pipeline.execute_batch(&batch).await.unwrap();
        }

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            let state = pipeline.state.read().await;
            // 1_000_000 - 1500 - 21_000 = 977_500
            assert_eq!(state.balance(&alice), 977_500);
            assert_eq!(state.balance(&[2u8; 32]), 1500);
            assert_eq!(state.nonce(&alice), 1);
        }

        cleanup(&path);
    }

    // ── AI inference helpers ──────────────────────────────────────

    fn build_add_model_bytes() -> Vec<u8> {
        use prost::Message;
        use tract_onnx::pb;

        let model = pb::ModelProto {
            ir_version: 7,
            opset_import: vec![pb::OperatorSetIdProto {
                domain: String::new(),
                version: 13,
            }],
            graph: Some(pb::GraphProto {
                name: "add_graph".into(),
                input: vec![pb::ValueInfoProto {
                    name: "x".into(),
                    r#type: Some(pb::TypeProto {
                        denotation: String::new(),
                        value: Some(pb::type_proto::Value::TensorType(pb::type_proto::Tensor {
                            elem_type: 1,
                            shape: Some(pb::TensorShapeProto {
                                dim: vec![
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(1),
                                        ),
                                    },
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(3),
                                        ),
                                    },
                                ],
                            }),
                        })),
                    }),
                    doc_string: String::new(),
                }],
                output: vec![pb::ValueInfoProto {
                    name: "y".into(),
                    r#type: Some(pb::TypeProto {
                        denotation: String::new(),
                        value: Some(pb::type_proto::Value::TensorType(pb::type_proto::Tensor {
                            elem_type: 1,
                            shape: Some(pb::TensorShapeProto {
                                dim: vec![
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(1),
                                        ),
                                    },
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(3),
                                        ),
                                    },
                                ],
                            }),
                        })),
                    }),
                    doc_string: String::new(),
                }],
                node: vec![pb::NodeProto {
                    input: vec!["x".into(), "b".into()],
                    output: vec!["y".into()],
                    name: "add_node".into(),
                    op_type: "Add".into(),
                    domain: String::new(),
                    attribute: vec![],
                    doc_string: String::new(),
                }],
                initializer: vec![pb::TensorProto {
                    name: "b".into(),
                    dims: vec![1, 3],
                    data_type: 1,
                    float_data: vec![1.0, 1.0, 1.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        model.encode_to_vec()
    }

    fn make_ai_pipeline(rx: mpsc::Receiver<CommittedBatch>) -> ExecutionPipeline {
        let rt = Arc::new(TractRuntime::new());
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let calculator = BaseFeeCalculator::new(1);
        let (slash_tx_init, slash_rx_init) = mpsc::channel(64);
        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors_set: HashSet::new(),
            executed_anchors_queue: VecDeque::new(),
            result_tx: None,
            ai_runtime: Some(rt),
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee())),
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
            task_pool: Arc::new(RwLock::new(TaskPool::new())),
            pending_task_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            current_round: 0,
            attestation_buffer: HashMap::new(),
            attestation_aggregator: AttestationAggregator::new(2),
            compute_commitments: Arc::new(RwLock::new(ComputeCommitmentStore::new())),
            governance: Arc::new(RwLock::new(GovernanceStore::new())),
            chain_params: Arc::new(RwLock::new(ChainParams::defaults())),
            emission_tracker: Arc::new(RwLock::new(EmissionTracker::new(
                aztibase_execution::tokenomics::DEFAULT_EPOCH_LENGTH,
            ))),
            staking_store: Arc::new(RwLock::new(StakingStore::new())),
            slash_rx: slash_rx_init,
            slash_tx: slash_tx_init,
            consensus_tx: None,
            epoch_participation: HashSet::new(),
            consensus_addrs: HashSet::new(),
            agent_policy_store: Arc::new(RwLock::new(AgentPolicyStore::new())),
            l2_registry: Arc::new(RwLock::new(L2Registry::new())),
            l2_anchor_store: Arc::new(RwLock::new(L2AnchorStore::new())),
            bridge_escrow: Arc::new(RwLock::new(BridgeEscrowStore::new())),
            bridge_withdraw_proofs: Arc::new(RwLock::new(BridgeWithdrawProofs::new())),
            archive: false,
            faucet_enabled: true,
        }
    }

    fn make_f32_input(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    #[tokio::test]
    async fn pipeline_ai_infer_success() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_ai_pipeline(rx);

        let (kp, requester) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);
        let ai_tx = TxKind::AiInfer {
            requester,
            model_id: "add".into(),
            input: make_f32_input(&[2.0, 3.0, 4.0]),
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&ai_tx, &kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.contract_count, 1);
        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        assert!(result.receipts[0].inference_hash.is_some());
        assert!(result.receipts[0].gas_used > 0);
    }

    #[tokio::test]
    async fn pipeline_ai_infer_unknown_model() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_ai_pipeline(rx);

        let (kp, requester) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);
        let ai_tx = TxKind::AiInfer {
            requester,
            model_id: "nonexistent".into(),
            input: vec![1, 2, 3],
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&ai_tx, &kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        assert!(result.receipts[0].inference_hash.is_none());
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("not available")
        );
    }

    #[tokio::test]
    async fn pipeline_mixed_transfer_and_ai() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_ai_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 1,
        };

        let ai_tx = TxKind::AiInfer {
            requester: alice,
            model_id: "add".into(),
            input: make_f32_input(&[1.0, 2.0, 3.0]),
            nonce: 1,
            max_compute_units: 10_000,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp), sign(&ai_tx, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 1);
        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(result.receipts[0].inference_hash.is_none());
        assert!(result.receipts[1].success);
        assert!(result.receipts[1].inference_hash.is_some());
        assert_eq!(pipeline.state.read().await.balance(&bob), 100);
    }

    #[tokio::test]
    async fn pipeline_create_agent() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (creator_kp, creator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&creator, 1_000_000);
        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "sentiment_v1".into(),
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&create_tx, &creator_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.contract_count, 1);
        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        let agent_addr = result.receipts[0].contract_address.unwrap();

        let state = pipeline.state.read().await;
        assert_eq!(state.account_type(&agent_addr), AccountType::AIAgent);
        assert_eq!(state.model_id(&agent_addr), Some("sentiment_v1"));
        assert_eq!(state.nonce(&creator), 1);
    }

    #[tokio::test]
    async fn pipeline_create_agent_nonce_mismatch() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (creator_kp, creator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&creator, 1_000_000);
        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "model_v1".into(),
            nonce: 5,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&create_tx, &creator_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("nonce mismatch")
        );
    }

    #[tokio::test]
    async fn pipeline_agent_receives_transfer() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (creator_kp, creator) = make_sender();

        pipeline
            .state
            .write()
            .await
            .set_balance(&creator, 1_000_000);
        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "trader_v1".into(),
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&create_tx, &creator_kp)]);
        let result1 = pipeline.execute_batch(&batch1).await.unwrap();
        let agent_addr = result1.receipts[0].contract_address.unwrap();

        // Reset balance after CreateAgent gas was consumed
        pipeline
            .state
            .write()
            .await
            .set_balance(&creator, 1_000_000);

        let transfer = TxKind::Transfer {
            from: creator,
            to: agent_addr,
            value: 1000,
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&transfer, &creator_kp)]);
        let result2 = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(result2.receipts[0].success);
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&agent_addr), 1000);
        // 1_000_000 - 1000 - 21_000 = 978_000
        assert_eq!(state.balance(&creator), 978_000);
        assert_eq!(state.account_type(&agent_addr), AccountType::AIAgent);
    }

    #[tokio::test]
    async fn pipeline_rejects_unsigned_tx() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let raw_unsigned = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 1,
        }
        .encode();

        let batch = make_batch(vec![raw_unsigned]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 0);
        assert_eq!(result.routing_errors, 1);
    }

    #[tokio::test]
    async fn pipeline_rejects_wrong_signer() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (_, addr1) = make_sender();
        let kp2 = Keypair::generate();

        let transfer = TxKind::Transfer {
            from: addr1,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 1,
        };
        let wrong_sig = sign(&transfer, &kp2);

        let batch = make_batch(vec![wrong_sig]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 0);
        assert_eq!(result.routing_errors, 1);
    }

    #[tokio::test]
    async fn pipeline_deducts_fees() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 300,
            nonce: 0,
            gas_price: 10,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(result.receipts[0].success);
        let gas_used = result.receipts[0].gas_used;
        let fee = gas_used as u128 * 10;

        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 1_000_000 - 300 - fee);
        assert_eq!(state.balance(&bob), 300);
    }

    #[tokio::test]
    async fn pipeline_rejects_zero_gas_price() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 500,
            nonce: 0,
            gas_price: 0,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("gas price too low")
        );
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 1_000_000);
        assert_eq!(state.balance(&bob), 0);
    }

    #[tokio::test]
    async fn pipeline_escrow_rejects_insufficient_balance() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        // gas_limit=21_000 * gas_price=10 = 210_000 escrow + value=100 = 210_100 needed
        pipeline.state.write().await.set_balance(&alice, 100_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 10,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("insufficient balance")
        );
        // Balance unchanged (escrow failed, no deduction)
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 100_000);
        assert_eq!(state.balance(&bob), 0);
    }

    #[tokio::test]
    async fn pipeline_escrow_refunds_unused_gas() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 0,
            nonce: 0,
            gas_price: 10,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(result.receipts[0].success);
        let gas_used = result.receipts[0].gas_used;
        let actual_fee = gas_used as u128 * 10;

        // Net deduction should be exactly gas_used * gas_price (escrow - refund)
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 1_000_000 - actual_fee);
        assert!(result.total_fees_burned > 0);
        assert_eq!(result.total_fees_burned, actual_fee);
    }

    #[tokio::test]
    async fn pipeline_escrow_reports_total_fees() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 5,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        let gas_used = result.receipts[0].gas_used;
        assert_eq!(result.total_fees_burned, gas_used as u128 * 5);
    }

    #[tokio::test]
    async fn pipeline_zero_gas_price_rejected_reports_zero_fees() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 0,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        // Tx rejected (gas price below base fee), so no fees burned
        assert_eq!(result.total_fees_burned, 0);
        assert!(!result.receipts[0].success);
    }

    #[tokio::test]
    async fn pipeline_escrow_with_parallel_transfers() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (kp1, addr1) = make_sender();
        let (kp2, addr2) = make_sender();
        let (kp3, addr3) = make_sender();
        let (kp4, addr4) = make_sender();
        let dest = [0xDD; 32];

        {
            let mut state = pipeline.state.write().await;
            state.set_balance(&addr1, 1_000_000);
            state.set_balance(&addr2, 1_000_000);
            state.set_balance(&addr3, 1_000_000);
            state.set_balance(&addr4, 1_000_000);
        }

        let txs: Vec<Vec<u8>> = vec![
            sign(
                &TxKind::Transfer {
                    from: addr1,
                    to: dest,
                    value: 100,
                    nonce: 0,
                    gas_price: 10,
                },
                &kp1,
            ),
            sign(
                &TxKind::Transfer {
                    from: addr2,
                    to: dest,
                    value: 200,
                    nonce: 0,
                    gas_price: 10,
                },
                &kp2,
            ),
            sign(
                &TxKind::Transfer {
                    from: addr3,
                    to: dest,
                    value: 300,
                    nonce: 0,
                    gas_price: 10,
                },
                &kp3,
            ),
            sign(
                &TxKind::Transfer {
                    from: addr4,
                    to: dest,
                    value: 400,
                    nonce: 0,
                    gas_price: 10,
                },
                &kp4,
            ),
        ];

        let batch = make_batch(txs);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 4);
        assert!(result.total_fees_burned > 0);

        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&dest), 100 + 200 + 300 + 400);

        // Each sender pays value + gas_used * gas_price
        for addr in [addr1, addr2, addr3, addr4] {
            assert!(state.balance(&addr) < 1_000_000);
        }
    }

    // ── Phase 2: Base fee persistence + nonce validation tests ──

    #[tokio::test]
    async fn pipeline_base_fee_persists_across_restart() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 10_000_000);

            let transfer = TxKind::Transfer {
                from: alice,
                to: [2u8; 32],
                value: 100,
                nonce: 0,
                gas_price: 10,
            };

            let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
            pipeline.execute_batch(&batch).await.unwrap();

            // Base fee should be stored after batch execution
            let fee_after = pipeline.base_fee.load(std::sync::atomic::Ordering::Relaxed);
            assert!(fee_after >= 1, "base fee should be at least MIN_BASE_FEE");
        }

        // Re-open and verify the base fee was persisted
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let loaded = aztibase_execution::load_base_fee(&store).unwrap();
            assert!(loaded.is_some(), "base fee should be persisted to redb");
            assert!(loaded.unwrap() >= 1);
        }

        cleanup(&path);
    }

    #[tokio::test]
    async fn pipeline_rejects_stale_nonce() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        pipeline.state.write().await.set_balance(&alice, 1_000_000);
        pipeline.state.write().await.get_mut(&alice).nonce = 5;

        // nonce=3 is stale (current is 5)
        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 3,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 0);
        let nonce_err = result.receipts.iter().find(|r| {
            r.error
                .as_ref()
                .map_or(false, |e| e.contains("nonce mismatch"))
        });
        assert!(nonce_err.is_some());
    }

    #[tokio::test]
    async fn pipeline_rejects_gap_nonce() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        // nonce=5 when current is 0 — gap
        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 5,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 0);
        let nonce_err = result.receipts.iter().find(|r| {
            r.error
                .as_ref()
                .map_or(false, |e| e.contains("nonce mismatch"))
        });
        assert!(nonce_err.is_some());
    }

    #[tokio::test]
    async fn pipeline_base_fee_updates_shared_atomic() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);
        let shared_fee = pipeline.shared_base_fee();

        let (alice_kp, alice) = make_sender();
        pipeline.state.write().await.set_balance(&alice, 1_000_000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 1,
        };

        let before = shared_fee.load(std::sync::atomic::Ordering::Relaxed);
        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();
        let after = shared_fee.load(std::sync::atomic::Ordering::Relaxed);

        // Base fee should have been updated (decreased since gas_used < target)
        assert!(after >= 1);
        assert_eq!(before, 1);
    }

    // ── Phase: Model registry + task posting tests ──────────────

    #[tokio::test]
    async fn pipeline_register_model() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let fingerprint = hash(b"model-weights-v1");

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "sentiment_v1".into(),
            fingerprint,
            compute_cost: 500,
            min_stake: 100,
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        assert_eq!(result.receipts[0].gas_used, 100_000);

        let state = pipeline.state.read().await;
        let registry = state
            .get(&aztibase_execution::MODEL_REGISTRY_ADDRESS)
            .unwrap();
        let meta = aztibase_execution::ModelRegistry::get(&registry.storage, "sentiment_v1");
        assert!(meta.is_some());
        let meta = meta.unwrap();
        assert_eq!(meta.owner, owner);
        assert!(meta.active);
    }

    #[tokio::test]
    async fn pipeline_register_duplicate_model_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let fingerprint = hash(b"fp");

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint,
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };

        let batch1 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let reg_tx2 = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint,
            compute_cost: 200,
            min_stake: 0,
            nonce: 1,
            gas_price: 1,
        };

        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&reg_tx2, &owner_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("already registered")
        );
    }

    #[tokio::test]
    async fn pipeline_post_task() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "classifier".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let post_tx = TxKind::PostTask {
            requester,
            model_id: "classifier".into(),
            input_hash: hash(b"test-input"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };

        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&post_tx, &req_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert_eq!(result.receipts.len(), 1);
        assert!(result.receipts[0].success);
        assert!(result.receipts[0].inference_hash.is_some());
        assert_eq!(result.receipts[0].gas_used, 42_000);

        let state = pipeline.state.read().await;
        // 1_000_000 - 500 (reward) - 42_000 (gas fee) = 957_500
        assert_eq!(state.balance(&requester), 957_500);
    }

    #[tokio::test]
    async fn pipeline_post_task_unregistered_model_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (req_kp, requester) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let post_tx = TxKind::PostTask {
            requester,
            model_id: "nonexistent".into(),
            input_hash: hash(b"data"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };

        let batch = make_batch(vec![sign(&post_tx, &req_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("not registered")
        );
        // Gas fee charged even on failure: escrow=42_000, gas_used=21_000, refund=21_000, net=21_000
        assert_eq!(
            pipeline.state.read().await.balance(&requester),
            1_000_000 - 21_000
        );
    }

    #[tokio::test]
    async fn pipeline_post_task_insufficient_reward_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        // Balance enough for gas escrow (42_000) but not for reward (500) after escrow
        pipeline.state.write().await.set_balance(&requester, 42_100);

        let reg = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let post_tx = TxKind::PostTask {
            requester,
            model_id: "m1".into(),
            input_hash: hash(b"data"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };

        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&post_tx, &req_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("insufficient balance")
        );
    }

    // ── Phase: TaskPool wiring tests ────────────────────────────────

    #[tokio::test]
    async fn pipeline_post_task_inserts_into_task_pool() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg = TxKind::RegisterModel {
            owner,
            model_id: "pool_test".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        assert_eq!(
            pipeline
                .pending_task_count
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        let post = TxKind::PostTask {
            requester,
            model_id: "pool_test".into(),
            input_hash: hash(b"input"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&post, &req_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(result.receipts[0].success);
        assert_eq!(
            pipeline
                .pending_task_count
                .load(std::sync::atomic::Ordering::Relaxed),
            1
        );

        let pool = pipeline.task_pool.read().await;
        assert_eq!(pool.len(), 1);
        let task_id = result.receipts[0].inference_hash.unwrap();
        assert!(pool.get(&task_id).is_some());
    }

    #[tokio::test]
    async fn pipeline_evicts_expired_tasks_and_refunds() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg = TxKind::RegisterModel {
            owner,
            model_id: "expiry_test".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        // Post task with deadline_round=3 (expires after round 3)
        let post = TxKind::PostTask {
            requester,
            model_id: "expiry_test".into(),
            input_hash: hash(b"input"),
            reward: 2000,
            deadline_round: 3,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&post, &req_kp)]);
        pipeline.execute_batch(&batch2).await.unwrap();

        // After batch2: 1_000_000 - 42_000 (gas) - 2_000 (reward) = 956_000
        assert_eq!(pipeline.state.read().await.balance(&requester), 956_000);
        assert_eq!(pipeline.task_pool.read().await.len(), 1);

        // Execute empty batch to advance to round 3 — task not yet expired (deadline=3, expired when round>3)
        let batch3 = make_batch_with_anchor([0xCC; 32], vec![]);
        pipeline.execute_batch(&batch3).await.unwrap();
        assert_eq!(pipeline.task_pool.read().await.len(), 1);

        // Execute empty batch to advance to round 4 — now task is expired (4 > 3)
        let batch4 = make_batch_with_anchor([0xDD; 32], vec![]);
        pipeline.execute_batch(&batch4).await.unwrap();

        assert_eq!(pipeline.task_pool.read().await.len(), 0);
        assert_eq!(
            pipeline
                .pending_task_count
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
        // Reward refunded but gas fee stays: 1_000_000 - 42_000 = 958_000
        assert_eq!(pipeline.state.read().await.balance(&requester), 958_000);
    }

    #[tokio::test]
    async fn pipeline_task_pool_tracks_multiple_tasks() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg = TxKind::RegisterModel {
            owner,
            model_id: "multi".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let post1 = TxKind::PostTask {
            requester,
            model_id: "multi".into(),
            input_hash: hash(b"in1"),
            reward: 100,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };
        let post2 = TxKind::PostTask {
            requester,
            model_id: "multi".into(),
            input_hash: hash(b"in2"),
            reward: 200,
            deadline_round: 100,
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor(
            [0xBB; 32],
            vec![sign(&post1, &req_kp), sign(&post2, &req_kp)],
        );
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(result.receipts[1].success);
        assert_eq!(pipeline.task_pool.read().await.len(), 2);
        assert_eq!(
            pipeline
                .pending_task_count
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
    }

    #[tokio::test]
    async fn pipeline_failed_post_task_not_in_pool() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (req_kp, requester) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        // Post task for unregistered model — should fail
        let post = TxKind::PostTask {
            requester,
            model_id: "missing".into(),
            input_hash: hash(b"data"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&post, &req_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert_eq!(pipeline.task_pool.read().await.len(), 0);
        assert_eq!(
            pipeline
                .pending_task_count
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    // ── Phase: DeregisterCompute tests ──────────────────────────

    #[tokio::test]
    async fn pipeline_deregister_compute_refunds_stake() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);

        let (owner_kp, owner) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch0 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch0).await.unwrap();

        let (bls_pk, bls_pop) = make_bls();
        let commit_tx = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 3000,
            bls_pubkey: bls_pk,
            bls_pop,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch_with_anchor([0xBB; 32], vec![sign(&commit_tx, &val_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();
        // 1_000_000 - 75_000 (gas) - 3_000 (stake) = 922_000
        assert_eq!(pipeline.state.read().await.balance(&validator), 922_000);

        let dereg_tx = TxKind::DeregisterCompute {
            validator,
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xCC; 32], vec![sign(&dereg_tx, &val_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(result.receipts[0].success);
        // 922_000 - 50_000 (gas) + 3_000 (stake refund) = 875_000
        assert_eq!(pipeline.state.read().await.balance(&validator), 875_000);
        assert_eq!(pipeline.compute_commitments.read().await.active_count(), 0);
    }

    #[tokio::test]
    async fn pipeline_deregister_compute_no_commitment_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);

        let dereg_tx = TxKind::DeregisterCompute {
            validator,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&dereg_tx, &val_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("no active compute commitment")
        );
    }

    #[tokio::test]
    async fn pipeline_commit_compute_overwrites_refunds_previous() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);

        let (owner_kp, owner) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch0 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch0).await.unwrap();

        let (bls_pk, bls_pop) = make_bls();
        let commit1 = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 2000,
            bls_pubkey: bls_pk.clone(),
            bls_pop: bls_pop.clone(),
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch_with_anchor([0xBB; 32], vec![sign(&commit1, &val_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();
        // 1_000_000 - 75_000 (gas) - 2_000 (stake) = 923_000
        assert_eq!(pipeline.state.read().await.balance(&validator), 923_000);

        let commit2 = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 5000,
            bls_pubkey: bls_pk,
            bls_pop,
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xCC; 32], vec![sign(&commit2, &val_kp)]);
        pipeline.execute_batch(&batch2).await.unwrap();

        // 923_000 - 75_000 (gas escrow) + 2_000 (prev stake refund) - 5_000 (new stake) + 0 (no gas refund) = 845_000
        assert_eq!(pipeline.state.read().await.balance(&validator), 845_000);
    }

    // ── Phase: DeregisterModel tests ────────────────────────────

    #[tokio::test]
    async fn pipeline_deregister_model() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let dereg_tx = TxKind::DeregisterModel {
            owner,
            model_id: "m1".into(),
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&dereg_tx, &owner_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(result.receipts[0].success);
        let state = pipeline.state.read().await;
        let registry = state
            .get(&aztibase_execution::MODEL_REGISTRY_ADDRESS)
            .unwrap();
        let meta = aztibase_execution::ModelRegistry::get(&registry.storage, "m1");
        assert!(meta.is_some());
        assert!(!meta.unwrap().active);
    }

    #[tokio::test]
    async fn pipeline_deregister_model_non_owner_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (other_kp, _other) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        let other_addr = aztibase_core::address_from_pubkey(other_kp.public_key().as_bytes());
        pipeline
            .state
            .write()
            .await
            .set_balance(&other_addr, 1_000_000);
        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let dereg_tx = TxKind::DeregisterModel {
            owner: other_addr,
            model_id: "m1".into(),
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&dereg_tx, &other_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("not model owner")
        );
    }

    #[tokio::test]
    async fn pipeline_deregister_model_with_pending_tasks_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let post_tx = TxKind::PostTask {
            requester,
            model_id: "m1".into(),
            input_hash: hash(b"input"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&post_tx, &req_kp)]);
        pipeline.execute_batch(&batch2).await.unwrap();

        let dereg_tx = TxKind::DeregisterModel {
            owner,
            model_id: "m1".into(),
            nonce: 1,
            gas_price: 1,
        };
        let batch3 = make_batch_with_anchor([0xCC; 32], vec![sign(&dereg_tx, &owner_kp)]);
        let result = pipeline.execute_batch(&batch3).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("pending tasks")
        );
    }

    // ── Phase: CommitCompute model validation test ───────────────

    #[tokio::test]
    async fn pipeline_commit_compute_unregistered_model_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);

        let (bls_pk, bls_pop) = make_bls();
        let commit_tx = TxKind::CommitCompute {
            validator,
            supported_models: vec!["nonexistent_model".into()],
            committed_stake: 1000,
            bls_pubkey: bls_pk,
            bls_pop,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&commit_tx, &val_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("not registered")
        );
        // Gas charged on failure: escrow=75_000, gas_used=21_000, refund=54_000, net=21_000
        assert_eq!(
            pipeline.state.read().await.balance(&validator),
            1_000_000 - 21_000
        );
    }

    #[tokio::test]
    async fn pipeline_commit_compute_invalid_bls_pop_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        let (owner_kp, owner) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);
        pipeline.state.write().await.set_balance(&owner, 1_000_000);

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch0 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch0).await.unwrap();

        // Use a valid BLS pubkey but a PoP from a different keypair.
        let (bls_pk, _) = make_bls();
        let (_, wrong_pop) = make_bls();
        let commit_tx = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 1000,
            bls_pubkey: bls_pk,
            bls_pop: wrong_pop,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch_with_anchor([0xBB; 32], vec![sign(&commit_tx, &val_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("proof-of-possession")
        );
        // Gas charged on failure: escrow=75_000, gas_used=21_000, refund=54_000, net=21_000
        assert_eq!(
            pipeline.state.read().await.balance(&validator),
            1_000_000 - 21_000
        );
    }

    #[tokio::test]
    async fn pipeline_commit_compute_malformed_bls_pop_fails() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (val_kp, validator) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);

        let commit_tx = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 1000,
            bls_pubkey: vec![0xFF; 48],
            bls_pop: vec![0xFF; 96],
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&commit_tx, &val_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert!(!result.receipts[0].success);
        assert!(
            result.receipts[0]
                .error
                .as_ref()
                .unwrap()
                .contains("proof-of-possession")
        );
    }

    // ── Phase: TaskAssigner wiring test ──────────────────────────

    #[tokio::test]
    async fn pipeline_post_task_assigns_validator() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (owner_kp, owner) = make_sender();
        let (val_kp, validator) = make_sender();
        let (req_kp, requester) = make_sender();
        pipeline.state.write().await.set_balance(&owner, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&validator, 1_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&requester, 1_000_000);

        let reg_tx = TxKind::RegisterModel {
            owner,
            model_id: "m1".into(),
            fingerprint: hash(b"fp"),
            compute_cost: 100,
            min_stake: 0,
            nonce: 0,
            gas_price: 1,
        };
        let batch0 = make_batch(vec![sign(&reg_tx, &owner_kp)]);
        pipeline.execute_batch(&batch0).await.unwrap();

        let (bls_pk, bls_pop) = make_bls();
        let commit_tx = TxKind::CommitCompute {
            validator,
            supported_models: vec!["m1".into()],
            committed_stake: 1000,
            bls_pubkey: bls_pk,
            bls_pop,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch_with_anchor([0xBB; 32], vec![sign(&commit_tx, &val_kp)]);
        pipeline.execute_batch(&batch1).await.unwrap();

        let post_tx = TxKind::PostTask {
            requester,
            model_id: "m1".into(),
            input_hash: hash(b"input"),
            reward: 500,
            deadline_round: 100,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch_with_anchor([0xCC; 32], vec![sign(&post_tx, &req_kp)]);
        let result = pipeline.execute_batch(&batch2).await.unwrap();
        assert!(result.receipts[0].success);

        let pool = pipeline.task_pool.read().await;
        let task_id = result.receipts[0].inference_hash.unwrap();
        let task = pool.get(&task_id).unwrap();
        assert_eq!(task.assigned_validator, Some(validator));
    }

    #[tokio::test]
    async fn governance_proposal_executes_chain_param() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (proposer_kp, proposer_addr) = make_sender();
        let (voter1_kp, voter1_addr) = make_sender();
        let (voter2_kp, voter2_addr) = make_sender();

        {
            let mut state = pipeline.state.write().await;
            state.set_balance(&proposer_addr, 10_000_000);
            state.set_balance(&voter1_addr, 5_000_000);
            state.set_balance(&voter2_addr, 3_000_000);
        }

        let create_tx = TxKind::CreateProposal {
            proposer: proposer_addr,
            description: "Raise base fee floor to 100".into(),
            param_key: "base_fee_floor".into(),
            param_value: "100".into(),
            voting_period: 10,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&create_tx, &proposer_kp)]);
        pipeline.current_round = 5;
        let result = pipeline.execute_batch(&batch1).await.unwrap();
        assert!(result.receipts[0].success);

        let vote1 = TxKind::CastVote {
            voter: voter1_addr,
            proposal_id: result.receipts[0].tx_hash,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let vote2 = TxKind::CastVote {
            voter: voter2_addr,
            proposal_id: result.receipts[0].tx_hash,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch(vec![sign(&vote1, &voter1_kp), sign(&vote2, &voter2_kp)]);
        pipeline.current_round = 8;
        let result2 = pipeline.execute_batch(&batch2).await.unwrap();
        assert!(result2.receipts[0].success);
        assert!(result2.receipts[1].success);

        // Advance past voting period (start_round=5, period=10, end_round=15).
        pipeline.current_round = 16;
        let noop_batch = make_batch(vec![]);
        let _ = pipeline.execute_batch(&noop_batch).await.unwrap();

        // Verify the chain param was updated.
        let params = pipeline.chain_params.read().await;
        assert_eq!(params.get_u64("base_fee_floor"), Some(100));

        // Verify proposal is marked Executed.
        let gov = pipeline.governance.read().await;
        let proposal = gov.get(&result.receipts[0].tx_hash).unwrap();
        assert_eq!(
            proposal.status,
            aztibase_execution::ProposalStatus::Executed
        );
    }

    #[tokio::test]
    async fn governance_proposal_invalid_param_still_marks_executed() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (proposer_kp, proposer_addr) = make_sender();
        let (voter1_kp, voter1_addr) = make_sender();
        let (voter2_kp, voter2_addr) = make_sender();

        {
            let mut state = pipeline.state.write().await;
            state.set_balance(&proposer_addr, 10_000_000);
            state.set_balance(&voter1_addr, 5_000_000);
            state.set_balance(&voter2_addr, 3_000_000);
        }

        // Propose a change to an unknown key.
        let create_tx = TxKind::CreateProposal {
            proposer: proposer_addr,
            description: "Change nonexistent param".into(),
            param_key: "nonexistent_key".into(),
            param_value: "42".into(),
            voting_period: 10,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&create_tx, &proposer_kp)]);
        pipeline.current_round = 1;
        let result = pipeline.execute_batch(&batch1).await.unwrap();
        assert!(result.receipts[0].success);

        let proposal_id = result.receipts[0].tx_hash;

        let vote1 = TxKind::CastVote {
            voter: voter1_addr,
            proposal_id,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let vote2 = TxKind::CastVote {
            voter: voter2_addr,
            proposal_id,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch(vec![sign(&vote1, &voter1_kp), sign(&vote2, &voter2_kp)]);
        pipeline.current_round = 5;
        let _ = pipeline.execute_batch(&batch2).await.unwrap();

        // Finalize + attempt execution.
        pipeline.current_round = 12;
        let _ = pipeline.execute_batch(&make_batch(vec![])).await.unwrap();

        // Even though param change failed, proposal should be Executed (prevents retry loop).
        let gov = pipeline.governance.read().await;
        let proposal = gov.get(&proposal_id).unwrap();
        assert_eq!(
            proposal.status,
            aztibase_execution::ProposalStatus::Executed
        );
    }

    #[tokio::test]
    async fn governance_multiple_proposals_execute_in_order() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (proposer_kp, proposer_addr) = make_sender();
        let (v1_kp, v1_addr) = make_sender();
        let (v2_kp, v2_addr) = make_sender();

        {
            let mut state = pipeline.state.write().await;
            state.set_balance(&proposer_addr, 50_000_000);
            state.set_balance(&v1_addr, 5_000_000);
            state.set_balance(&v2_addr, 3_000_000);
        }

        // Create two proposals.
        let create1 = TxKind::CreateProposal {
            proposer: proposer_addr,
            description: "Set max_block_range to 200".into(),
            param_key: "max_block_range".into(),
            param_value: "200".into(),
            voting_period: 10,
            nonce: 0,
            gas_price: 1,
        };
        let create2 = TxKind::CreateProposal {
            proposer: proposer_addr,
            description: "Set max_stored_txs to 750000".into(),
            param_key: "max_stored_txs".into(),
            param_value: "750000".into(),
            voting_period: 10,
            nonce: 1,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![
            sign(&create1, &proposer_kp),
            sign(&create2, &proposer_kp),
        ]);
        pipeline.current_round = 1;
        let r1 = pipeline.execute_batch(&batch1).await.unwrap();
        let pid1 = r1.receipts[0].tx_hash;
        let pid2 = r1.receipts[1].tx_hash;

        // Vote on both.
        let vote1a = TxKind::CastVote {
            voter: v1_addr,
            proposal_id: pid1,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let vote1b = TxKind::CastVote {
            voter: v2_addr,
            proposal_id: pid1,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let vote2a = TxKind::CastVote {
            voter: v1_addr,
            proposal_id: pid2,
            approve: true,
            nonce: 1,
            gas_price: 1,
        };
        let vote2b = TxKind::CastVote {
            voter: v2_addr,
            proposal_id: pid2,
            approve: true,
            nonce: 1,
            gas_price: 1,
        };
        let batch2 = make_batch(vec![
            sign(&vote1a, &v1_kp),
            sign(&vote1b, &v2_kp),
            sign(&vote2a, &v1_kp),
            sign(&vote2b, &v2_kp),
        ]);
        pipeline.current_round = 5;
        let _ = pipeline.execute_batch(&batch2).await.unwrap();

        // Finalize both.
        pipeline.current_round = 12;
        let _ = pipeline.execute_batch(&make_batch(vec![])).await.unwrap();

        let params = pipeline.chain_params.read().await;
        assert_eq!(params.get_u64("max_block_range"), Some(200));
        assert_eq!(params.get_u64("max_stored_txs"), Some(750_000));

        let gov = pipeline.governance.read().await;
        assert_eq!(
            gov.get(&pid1).unwrap().status,
            aztibase_execution::ProposalStatus::Executed
        );
        assert_eq!(
            gov.get(&pid2).unwrap().status,
            aztibase_execution::ProposalStatus::Executed
        );
    }

    #[tokio::test]
    async fn governance_already_executed_proposal_skipped() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (proposer_kp, proposer_addr) = make_sender();
        let (v1_kp, v1_addr) = make_sender();
        let (v2_kp, v2_addr) = make_sender();

        {
            let mut state = pipeline.state.write().await;
            state.set_balance(&proposer_addr, 10_000_000);
            state.set_balance(&v1_addr, 5_000_000);
            state.set_balance(&v2_addr, 3_000_000);
        }

        let create_tx = TxKind::CreateProposal {
            proposer: proposer_addr,
            description: "Set max_block_range to 500".into(),
            param_key: "max_block_range".into(),
            param_value: "500".into(),
            voting_period: 10,
            nonce: 0,
            gas_price: 1,
        };
        let batch1 = make_batch(vec![sign(&create_tx, &proposer_kp)]);
        pipeline.current_round = 1;
        let r = pipeline.execute_batch(&batch1).await.unwrap();
        let pid = r.receipts[0].tx_hash;

        let vote1 = TxKind::CastVote {
            voter: v1_addr,
            proposal_id: pid,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let vote2 = TxKind::CastVote {
            voter: v2_addr,
            proposal_id: pid,
            approve: true,
            nonce: 0,
            gas_price: 1,
        };
        let batch2 = make_batch(vec![sign(&vote1, &v1_kp), sign(&vote2, &v2_kp)]);
        pipeline.current_round = 5;
        let _ = pipeline.execute_batch(&batch2).await.unwrap();

        // Finalize + execute.
        pipeline.current_round = 12;
        let _ = pipeline.execute_batch(&make_batch(vec![])).await.unwrap();
        assert_eq!(
            pipeline
                .chain_params
                .read()
                .await
                .get_u64("max_block_range"),
            Some(500)
        );

        // Run another empty batch — should not re-execute.
        pipeline.current_round = 13;
        let _ = pipeline.execute_batch(&make_batch(vec![])).await.unwrap();

        // Verify still Executed (passed_unexecuted won't return it).
        let gov = pipeline.governance.read().await;
        assert_eq!(
            gov.get(&pid).unwrap().status,
            aztibase_execution::ProposalStatus::Executed
        );
        assert_eq!(
            pipeline
                .chain_params
                .read()
                .await
                .get_u64("max_block_range"),
            Some(500)
        );
    }

    #[tokio::test]
    async fn genesis_bootstrap_registers_validators() {
        let (_tx, rx) = mpsc::channel(1);
        let mut pipeline = make_pipeline(rx);
        let validators = vec![
            ([1u8; 32], 100_000u128, None),
            ([2u8; 32], 200_000u128, None),
        ];
        pipeline.bootstrap_genesis_validators(&validators).await;

        let staking = pipeline.staking_store.read().await;
        let active = staking.active_validators();
        assert_eq!(active.len(), 2);
        assert_eq!(
            staking.get_validator(&[1u8; 32]).unwrap().self_stake,
            100_000
        );
        assert_eq!(
            staking.get_validator(&[2u8; 32]).unwrap().self_stake,
            200_000
        );
    }

    #[tokio::test]
    async fn epoch_length_from_chain_params() {
        let (_tx, rx) = mpsc::channel(1);
        let pipeline = make_pipeline(rx);
        let cp = pipeline.chain_params.read().await;
        let epoch_len = cp.get_u64("epoch_length").unwrap_or(1_000);
        assert_eq!(epoch_len, 1_000);
    }

    #[tokio::test]
    async fn consensus_tx_wired_for_validator_update() {
        let (_tx, rx) = mpsc::channel(1);
        let pipeline = make_pipeline(rx);
        assert!(pipeline.consensus_tx.is_none());
    }

    // ── Phase 1: Crash recovery & resilience tests ──────────────────

    #[tokio::test]
    async fn pipeline_recovers_batch_count_from_disk() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 10_000_000);

            for i in 0u64..5 {
                let batch = make_batch_with_anchor(
                    aztibase_core::hash(&i.to_le_bytes()),
                    vec![sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: 100,
                            nonce: i,
                            gas_price: 1,
                        },
                        &alice_kp,
                    )],
                );
                pipeline.execute_batch(&batch).await.unwrap();
            }
            let count = pipeline.batch_count.load(Ordering::Relaxed);
            assert_eq!(count, 5);
        }

        // Reopen — batch_count should recover from BATCH_INDEX_TABLE.
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            let recovered = pipeline.batch_count.load(Ordering::Relaxed);
            assert_eq!(recovered, 5);
        }

        cleanup(&path);
    }

    #[tokio::test]
    async fn pipeline_crash_recovery_state_consistent() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];

        let initial_balance: u128 = 5_000_000;
        let transfer_value: u128 = 500;
        let gas_cost: u128 = 21_000;

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline
                .state
                .write()
                .await
                .set_balance(&alice, initial_balance);

            for i in 0u64..3 {
                let batch = make_batch_with_anchor(
                    aztibase_core::hash(&i.to_le_bytes()),
                    vec![sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: transfer_value,
                            nonce: i,
                            gas_price: 1,
                        },
                        &alice_kp,
                    )],
                );
                pipeline.execute_batch(&batch).await.unwrap();
            }
            // Drop without explicit shutdown — simulates crash.
        }

        // Reopen and verify state is consistent after "crash".
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let loaded = aztibase_execution::load_state(&store).unwrap();

            let expected_alice = initial_balance - 3 * (transfer_value + gas_cost);
            let expected_bob = 3 * transfer_value;

            assert_eq!(loaded.balance(&alice), expected_alice);
            assert_eq!(loaded.balance(&bob), expected_bob);

            let idx = aztibase_execution::latest_batch_index(&store).unwrap();
            assert_eq!(idx, Some(2)); // 0-indexed: batches 0, 1, 2

            let fee = aztibase_execution::load_base_fee(&store).unwrap();
            assert!(fee.is_some(), "base fee should survive crash");
        }

        cleanup(&path);
    }

    #[tokio::test]
    async fn pipeline_resumes_processing_after_recovery() {
        let path = test_db_path();
        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];

        // Session 1: process 2 batches.
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 10_000_000);

            for i in 0u64..2 {
                let batch = make_batch_with_anchor(
                    aztibase_core::hash(&i.to_le_bytes()),
                    vec![sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: 1000,
                            nonce: i,
                            gas_price: 1,
                        },
                        &alice_kp,
                    )],
                );
                pipeline.execute_batch(&batch).await.unwrap();
            }
        }

        // Session 2: recover and process 2 more batches.
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);

            let batch_before = pipeline.batch_count.load(Ordering::Relaxed);
            assert_eq!(batch_before, 2);

            for i in 2u64..4 {
                let batch = make_batch_with_anchor(
                    aztibase_core::hash(&i.to_le_bytes()),
                    vec![sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: 1000,
                            nonce: i,
                            gas_price: 1,
                        },
                        &alice_kp,
                    )],
                );
                pipeline.execute_batch(&batch).await.unwrap();
            }

            let batch_after = pipeline.batch_count.load(Ordering::Relaxed);
            assert_eq!(batch_after, 4);

            let state = pipeline.state.read().await;
            assert_eq!(state.balance(&bob), 4000);
        }

        cleanup(&path);
    }

    // ── Phase 3: Epoch boundary tests ──────────────────────────────

    #[tokio::test]
    async fn epoch_boundary_distributes_rewards() {
        let path = test_db_path();
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::with_storage(store, rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 100_000_000);

        // Set short epoch length for testing.
        {
            let cp = pipeline.shared_chain_params();
            let mut cp_guard = cp.write().await;
            cp_guard
                .set("epoch_length", aztibase_execution::ParamValue::U64(1000))
                .unwrap();
        }

        // Register alice as a validator with sufficient stake.
        {
            let mut staking = pipeline.staking_store.write().await;
            staking
                .register_validator(
                    alice,
                    MIN_VALIDATOR_STAKE,
                    MIN_VALIDATOR_STAKE,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                )
                .unwrap();
        }

        // Start near epoch boundary so 1 batch triggers it.
        pipeline.current_round = 999;

        let balance_before = pipeline.state.read().await.balance(&alice);

        // Run 1 batch — round becomes 1000, triggering epoch boundary.
        let batch = make_batch_with_anchor(
            aztibase_core::hash(&0u64.to_le_bytes()),
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 10,
                    nonce: 0,
                    gas_price: 1,
                },
                &alice_kp,
            )],
        );
        pipeline.execute_batch(&batch).await.unwrap();

        // After epoch boundary, alice should have received emission rewards.
        let balance_after = pipeline.state.read().await.balance(&alice);
        // Transfer cost: 10 value + 21,000 gas = 21,010.
        // If rewards were distributed, the net loss should be less than just transfer+gas.
        let total_spent: u128 = 10 + 21_000;
        let net_change = balance_before - balance_after;
        assert!(
            net_change < total_spent,
            "Expected rewards to partially offset spending: net_change={net_change}, total_spent={total_spent}"
        );

        cleanup(&path);
    }

    #[tokio::test]
    async fn epoch_boundary_clears_participation() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 100_000_000);

        {
            let cp = pipeline.shared_chain_params();
            let mut cp_guard = cp.write().await;
            cp_guard
                .set("epoch_length", aztibase_execution::ParamValue::U64(1000))
                .unwrap();
        }

        // Register a validator so epoch boundary logic triggers.
        {
            let mut staking = pipeline.staking_store.write().await;
            staking
                .register_validator(
                    alice,
                    MIN_VALIDATOR_STAKE,
                    MIN_VALIDATOR_STAKE,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                )
                .unwrap();
        }

        pipeline.current_round = 999;

        let batch = make_batch_with_anchor(
            aztibase_core::hash(&0u64.to_le_bytes()),
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 10,
                    nonce: 0,
                    gas_price: 1,
                },
                &alice_kp,
            )],
        );
        pipeline.execute_batch(&batch).await.unwrap();

        // After epoch boundary, participation should be cleared.
        assert!(
            pipeline.epoch_participation.is_empty(),
            "Epoch participation should be cleared after epoch boundary"
        );
    }

    #[tokio::test]
    async fn active_validators_not_slashed_at_epoch_boundary() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let validator_b = [0xDD; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 100_000_000);

        {
            let cp = pipeline.shared_chain_params();
            let mut cp_guard = cp.write().await;
            cp_guard
                .set("epoch_length", aztibase_execution::ParamValue::U64(1000))
                .unwrap();
        }

        {
            let mut staking = pipeline.staking_store.write().await;
            staking
                .register_validator(
                    alice,
                    MIN_VALIDATOR_STAKE,
                    MIN_VALIDATOR_STAKE,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                )
                .unwrap();
            staking
                .register_validator(
                    validator_b,
                    MIN_VALIDATOR_STAKE,
                    MIN_VALIDATOR_STAKE,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                )
                .unwrap();
        }
        pipeline.consensus_addrs.insert(alice);
        pipeline.consensus_addrs.insert(validator_b);

        let stake_before = {
            let staking = pipeline.staking_store.read().await;
            staking
                .get_validator(&validator_b)
                .map(|v| v.self_stake)
                .unwrap_or(0)
        };

        pipeline.current_round = 999;

        let batch = make_batch_with_anchor(
            aztibase_core::hash(&0u64.to_le_bytes()),
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 10,
                    nonce: 0,
                    gas_price: 1,
                },
                &alice_kp,
            )],
        );
        pipeline.execute_batch(&batch).await.unwrap();

        let stake_after = {
            let staking = pipeline.staking_store.read().await;
            staking
                .get_validator(&validator_b)
                .map(|v| v.self_stake)
                .unwrap_or(0)
        };

        assert_eq!(
            stake_after, stake_before,
            "Active validators should not be slashed: before={stake_before}, after={stake_after}"
        );

        assert!(
            pipeline.epoch_participation.is_empty(),
            "Participation cleared after epoch boundary"
        );
    }

    #[tokio::test]
    async fn epoch_boundary_propagates_validator_set() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 100_000_000);

        {
            let cp = pipeline.shared_chain_params();
            let mut cp_guard = cp.write().await;
            cp_guard
                .set("epoch_length", aztibase_execution::ParamValue::U64(1000))
                .unwrap();
        }

        {
            let mut staking = pipeline.staking_store.write().await;
            staking
                .register_validator(
                    alice,
                    MIN_VALIDATOR_STAKE,
                    MIN_VALIDATOR_STAKE,
                    MAX_VALIDATOR_STAKE_CAP,
                    0,
                )
                .unwrap();
        }

        pipeline.consensus_addrs.insert(alice);

        // Wire a consensus_tx to capture the UpdateValidatorSet message.
        let (ctx, mut crx) = mpsc::channel(16);
        pipeline.set_consensus_tx(ctx);

        pipeline.current_round = 999;

        let batch = make_batch_with_anchor(
            aztibase_core::hash(&0u64.to_le_bytes()),
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 10,
                    nonce: 0,
                    gas_price: 1,
                },
                &alice_kp,
            )],
        );
        pipeline.execute_batch(&batch).await.unwrap();

        // Check that UpdateValidatorSet was sent to consensus.
        let mut found_update = false;
        while let Ok(msg) = crx.try_recv() {
            if matches!(
                msg,
                aztibase_consensus::ConsensusInput::UpdateValidatorSet(_)
            ) {
                found_update = true;
            }
        }
        assert!(
            found_update,
            "Expected UpdateValidatorSet to be sent at epoch boundary"
        );
    }

    // ── Phase 4: Throughput measurement tests ──────────────────────

    #[tokio::test]
    async fn throughput_baseline_100_transfers() {
        let path = test_db_path();
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(256);
        let mut pipeline = ExecutionPipeline::with_storage(store, rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&alice, 100_000_000_000);

        let batch_count = 10u64;
        let txs_per_batch = 5usize;

        // Pre-sign all transactions.
        let mut batches = Vec::with_capacity(batch_count as usize);
        let mut nonce = 0u64;
        for i in 0..batch_count {
            let mut txs = Vec::with_capacity(txs_per_batch);
            for _ in 0..txs_per_batch {
                txs.push(sign(
                    &TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 10,
                        nonce,
                        gas_price: 1,
                    },
                    &alice_kp,
                ));
                nonce += 1;
            }
            batches.push(make_batch_with_anchor(
                aztibase_core::hash(&i.to_le_bytes()),
                txs,
            ));
        }

        let start = std::time::Instant::now();
        for batch in &batches {
            pipeline.execute_batch(batch).await.unwrap();
        }
        let elapsed = start.elapsed();

        let total_txs = batch_count * txs_per_batch as u64;
        let tps = total_txs as f64 / elapsed.as_secs_f64();
        let batches_per_sec = batch_count as f64 / elapsed.as_secs_f64();

        // Log throughput numbers for baseline tracking.
        println!(
            "THROUGHPUT: {total_txs} txs in {:.2}s = {tps:.0} TPS, {batches_per_sec:.0} batches/s",
            elapsed.as_secs_f64()
        );

        // Sanity: all transfers landed.
        let final_bob = pipeline.state.read().await.balance(&bob);
        assert_eq!(final_bob, total_txs as u128 * 10);

        // Sanity: batch count correct.
        let bc = pipeline.batch_count.load(Ordering::Relaxed);
        assert_eq!(bc, batch_count);

        cleanup(&path);
    }

    #[tokio::test]
    async fn throughput_empty_batches_baseline() {
        let (_tx, rx) = mpsc::channel(256);
        let mut pipeline = make_pipeline(rx);

        let batch_count = 100u64;
        let start = std::time::Instant::now();
        for i in 0..batch_count {
            let batch = make_batch_with_anchor(aztibase_core::hash(&i.to_le_bytes()), vec![]);
            pipeline.execute_batch(&batch).await.unwrap();
        }
        let elapsed = start.elapsed();

        let batches_per_sec = batch_count as f64 / elapsed.as_secs_f64();
        println!(
            "EMPTY BATCH THROUGHPUT: {batch_count} batches in {:.3}s = {batches_per_sec:.0} batches/s",
            elapsed.as_secs_f64()
        );

        assert!(
            batches_per_sec > 50.0,
            "Empty batch processing should exceed 50/s in debug mode"
        );
    }

    // ── L2 Bridge e2e tests ─────────────────────────────────────────

    #[tokio::test]
    async fn bridge_full_roundtrip_e2e() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (gov_kp, gov_addr) = make_sender();
        let (seq_kp, seq_addr) = make_sender();
        let (user_kp, user_addr) = make_sender();
        let l2_chain_id = [0xBB; 32];

        pipeline
            .state
            .write()
            .await
            .set_balance(&gov_addr, 10_000_000);
        pipeline
            .state
            .write()
            .await
            .set_balance(&user_addr, 10_000_000);

        // Step 1: Register L2
        let register = TxKind::RegisterL2 {
            owner: gov_addr,
            l2_chain_id,
            name: "TestRollup".into(),
            sequencer_set: vec![seq_addr],
            bridge_address: [0xCC; 32],
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&register, &gov_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.contract_count, 1);

        // Verify registered
        let reg = pipeline.l2_registry.read().await;
        assert!(reg.get(&l2_chain_id).is_some());
        drop(reg);

        // Step 2: Anchor L2 state (batch 1, so l1_batch_index = 1)
        pipeline
            .state
            .write()
            .await
            .set_balance(&seq_addr, 10_000_000);
        let anchor = TxKind::AnchorL2State {
            sequencer: seq_addr,
            l2_chain_id,
            state_root: [0xDD; 32],
            batch_data_hash: [0xEE; 32],
            l2_block_start: 0,
            l2_block_end: 100,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&anchor, &seq_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.contract_count, 1);

        // Step 3: Bridge deposit (user locks 5000 AZTB)
        let deposit = TxKind::BridgeDeposit {
            depositor: user_addr,
            l2_chain_id,
            l2_recipient: [0xFF; 32],
            amount: 5000,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&deposit, &user_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.contract_count, 1);

        let escrow = pipeline.bridge_escrow.read().await;
        assert_eq!(escrow.balance(&l2_chain_id, &user_addr), 5000);
        drop(escrow);

        // Step 4: Advance past finality window (100 batches)
        for _ in 0..100 {
            let empty = make_batch_with_anchor(aztibase_core::hash(b"empty"), vec![]);
            pipeline.execute_batch(&empty).await.unwrap();
        }

        // Step 5: Bridge withdraw (user reclaims 3000 AZTB)
        let withdraw = TxKind::BridgeWithdraw {
            withdrawer: user_addr,
            l2_chain_id,
            amount: 3000,
            l2_burn_proof: b"proof_data_1".to_vec(),
            l2_state_root: [0xDD; 32],
            nonce: 1,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&withdraw, &user_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.contract_count, 1);

        let state = pipeline.state.read().await;
        let user_bal = state.balance(&user_addr);
        assert!(
            user_bal > 10_000_000 - 5000 - 200_000,
            "user should have received withdrawal, got {user_bal}"
        );
        drop(state);

        // Escrow should reflect 5000 deposited - 3000 withdrawn = 2000 remaining
        let escrow = pipeline.bridge_escrow.read().await;
        assert_eq!(escrow.balance(&l2_chain_id, &user_addr), 2000);
    }

    #[tokio::test]
    async fn bridge_deposit_overflow_u128_max() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (gov_kp, gov_addr) = make_sender();
        let (user_kp, user_addr) = make_sender();
        let l2_chain_id = [0xAA; 32];

        pipeline
            .state
            .write()
            .await
            .set_balance(&gov_addr, 10_000_000);
        pipeline.state.write().await.set_balance(&user_addr, 1000);

        // Register L2
        let register = TxKind::RegisterL2 {
            owner: gov_addr,
            l2_chain_id,
            name: "OverflowTest".into(),
            sequencer_set: vec![[0x99; 32]],
            bridge_address: [0x88; 32],
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&register, &gov_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();

        // Try depositing more than balance — should fail
        let deposit = TxKind::BridgeDeposit {
            depositor: user_addr,
            l2_chain_id,
            l2_recipient: [0xFF; 32],
            amount: u128::MAX,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&deposit, &user_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();

        // Escrow should be empty (deposit failed due to insufficient balance)
        let escrow = pipeline.bridge_escrow.read().await;
        assert_eq!(escrow.balance(&l2_chain_id, &user_addr), 0);
    }

    #[tokio::test]
    async fn bridge_withdraw_unregistered_l2_rejected() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (user_kp, user_addr) = make_sender();
        pipeline
            .state
            .write()
            .await
            .set_balance(&user_addr, 10_000_000);

        // Try deposit on unregistered L2
        let deposit = TxKind::BridgeDeposit {
            depositor: user_addr,
            l2_chain_id: [0xDE; 32],
            l2_recipient: [0xFF; 32],
            amount: 1000,
            nonce: 0,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&deposit, &user_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();

        // Escrow should be empty (L2 not registered)
        let escrow = pipeline.bridge_escrow.read().await;
        assert_eq!(escrow.balance(&[0xDE; 32], &user_addr), 0);
        drop(escrow);

        // Also try withdraw on unregistered L2 — should fail
        let withdraw = TxKind::BridgeWithdraw {
            withdrawer: user_addr,
            l2_chain_id: [0xDE; 32],
            amount: 1000,
            l2_burn_proof: b"fake_proof".to_vec(),
            l2_state_root: [0xAA; 32],
            nonce: 1,
            gas_price: 1,
        };
        let batch = make_batch(vec![sign(&withdraw, &user_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();

        // Balance should only be reduced by gas fees, not the withdrawal amount
        let state = pipeline.state.read().await;
        let bal = state.balance(&user_addr);
        assert!(
            bal > 10_000_000 - 200_000,
            "balance should only lose gas fees, got {bal}"
        );
    }
}
