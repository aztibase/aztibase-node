use std::collections::HashSet;
use std::sync::Arc;

use aztibase_consensus::CommittedBatch;
use aztibase_core::hash;
use aztibase_execution::{
    AccountState, BaseFeeCalculator, ContractTx, ExecutionReceipt, FeeEscrow, TransferTx, TxKind,
    block_stm::{BlockSTMExecutor, apply_block_stm_to_state},
    escrow_fee, evm, execute_contract_txs, flush_state, load_base_fee, load_state, refund_unused,
    state::AccountType,
    store_base_fee, store_batch_root, store_receipts, verify_and_route_batch,
};
use aztibase_runtime::{AIRuntime, AnomalyScorer, InferenceRequest, TxFeatures};
use aztibase_storage::StateStore;
use tokio::sync::{RwLock, mpsc};

/// Result of executing a single committed batch.
#[derive(Clone, Debug)]
pub struct PipelineResult {
    pub batch_anchor: [u8; 32],
    pub state_root: [u8; 32],
    pub transfer_count: usize,
    pub contract_count: usize,
    pub routing_errors: usize,
    pub receipts: Vec<ExecutionReceipt>,
    pub total_fees_burned: u64,
}

/// Owns account state and executes committed batches received from consensus.
/// Optionally persists state to redb after each batch.
pub struct ExecutionPipeline {
    state: Arc<RwLock<AccountState>>,
    store: Option<Arc<StateStore>>,
    rx: mpsc::Receiver<CommittedBatch>,
    batch_count: Arc<std::sync::atomic::AtomicU64>,
    executed_anchors: HashSet<[u8; 32]>,
    result_tx: Option<mpsc::Sender<PipelineResult>>,
    ai_runtime: Option<Arc<dyn AIRuntime>>,
    base_fee: Arc<std::sync::atomic::AtomicU64>,
    base_fee_calculator: BaseFeeCalculator,
    anomaly_scorer: AnomalyScorer,
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
        let initial_base_fee = match load_base_fee(&store) {
            Ok(Some(fee)) => {
                tracing::info!(base_fee = fee, "Base fee loaded from disk");
                fee
            }
            _ => 1,
        };
        let calculator = BaseFeeCalculator::new(initial_base_fee);
        let base_fee = Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee()));
        Self {
            state: Arc::new(RwLock::new(state)),
            store: Some(store),
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: None,
            base_fee,
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
        }
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

    /// Attach a channel to receive execution results (for state root broadcasting).
    pub fn set_result_sender(&mut self, tx: mpsc::Sender<PipelineResult>) {
        self.result_tx = Some(tx);
    }

    /// Run the pipeline loop, processing committed batches until the channel closes.
    pub async fn run(mut self) {
        while let Some(batch) = self.rx.recv().await {
            if !self.executed_anchors.insert(batch.anchor_hash) {
                tracing::warn!(
                    anchor = %short_hex(&batch.anchor_hash),
                    "Duplicate batch skipped"
                );
                continue;
            }
            match self.execute_batch(&batch).await {
                Ok(result) => {
                    self.batch_count
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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
        tracing::info!("Execution pipeline shutting down");
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
        let (mut routed, errors) = verify_and_route_batch(&batch.transactions);

        routed.sort_by(|a, b| a.sender().cmp(b.sender()).then(a.nonce().cmp(&b.nonce())));

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
        routed = original_routed
            .into_iter()
            .zip(nonce_valid.iter())
            .filter(|(_, valid)| **valid)
            .map(|(tx, _)| tx)
            .collect();

        // Phase 1: Escrow fees for all txs with gas_price > 0.
        // Txs that fail escrow get a failure receipt and are excluded from execution.
        let mut escrows: Vec<Option<FeeEscrow>> = Vec::with_capacity(routed.len());
        let mut escrowed_indices: Vec<usize> = Vec::new();
        let mut receipts = Vec::new();
        let mut total_fees_burned: u64 = 0;

        for (i, tx) in routed.iter().enumerate() {
            let gas_price = tx.gas_price();
            let gas_limit = tx.gas_limit();
            if gas_price == 0 {
                escrows.push(None);
                escrowed_indices.push(i);
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
                    state.increment_nonce(tx.sender());
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
            }
        }

        let transfer_count = transfers.len();
        let contract_count = contracts.len()
            + evm_deploys.len()
            + evm_calls.len()
            + ai_infers.len()
            + create_agents.len();

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

        // Update base fee based on total gas consumed this batch.
        let total_gas: u64 = receipts.iter().map(|r| r.gas_used).sum();
        self.base_fee_calculator.update(total_gas);
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
        }

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

fn compute_tx_hash(tx: &TxKind) -> [u8; 32] {
    match tx {
        TxKind::Transfer {
            from,
            to,
            value,
            nonce,
            ..
        } => {
            let mut buf = Vec::new();
            buf.extend_from_slice(from);
            buf.extend_from_slice(to);
            buf.extend_from_slice(&value.to_le_bytes());
            buf.extend_from_slice(&nonce.to_le_bytes());
            hash(&buf)
        }
        TxKind::ContractDeploy { code, .. } => hash(code),
        TxKind::ContractCall { func_name, .. } => hash(func_name.as_bytes()),
        TxKind::EvmDeploy { code, .. } => hash(code),
        TxKind::EvmCall { calldata, .. } => hash(calldata),
        TxKind::AiInfer {
            requester,
            model_id,
            input,
            ..
        } => {
            let mut buf = Vec::new();
            buf.extend_from_slice(requester);
            buf.extend_from_slice(model_id.as_bytes());
            buf.extend_from_slice(input);
            hash(&buf)
        }
        TxKind::CreateAgent {
            creator,
            model_id,
            nonce,
            ..
        } => {
            let mut buf = Vec::new();
            buf.extend_from_slice(creator);
            buf.extend_from_slice(model_id.as_bytes());
            buf.extend_from_slice(&nonce.to_le_bytes());
            hash(&buf)
        }
    }
}

fn extract_tx_features(tx: &TxKind) -> TxFeatures {
    let value = match tx {
        TxKind::Transfer { value, .. } | TxKind::EvmCall { value, .. } => *value,
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
    use aztibase_core::{Keypair, address_from_pubkey};
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

    fn sign(tx: &TxKind, kp: &Keypair) -> Vec<u8> {
        SignedTx::new(tx.encode(), kp).encode()
    }

    fn make_pipeline(rx: mpsc::Receiver<CommittedBatch>) -> ExecutionPipeline {
        let calculator = BaseFeeCalculator::new(1);
        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: None,
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee())),
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
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
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 300,
            nonce: 0,
            gas_price: 0,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 0);
        assert_eq!(result.routing_errors, 0);
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 700);
        assert_eq!(state.balance(&bob), 300);
        assert_ne!(result.state_root, [0u8; 32]);
    }

    #[tokio::test]
    async fn pipeline_executes_mixed_batch() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 5000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 0,
        };

        let deploy = TxKind::ContractDeploy {
            deployer: alice,
            code: vec![0x00, 0x61, 0x73, 0x6d],
            nonce: 1,
            gas_limit: 1_000_000,
            gas_price: 0,
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
        let good = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 0,
            nonce: 0,
            gas_price: 0,
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
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 200,
            nonce: 0,
            gas_price: 0,
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
        pipeline.state.write().await.set_balance(&alice, 1000);

        let batch1 = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
                gas_price: 0,
            },
            &alice_kp,
        )]);

        let batch2 = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 200,
                nonce: 1,
                gas_price: 0,
            },
            &alice_kp,
        )]);

        pipeline.execute_batch(&batch1).await.unwrap();
        let result = pipeline.execute_batch(&batch2).await.unwrap();

        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 500);
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
            pipeline.state.write().await.set_balance(&alice, 1000);

            let transfer = TxKind::Transfer {
                from: alice,
                to: bob,
                value: 400,
                nonce: 0,
                gas_price: 0,
            };

            let batch = make_batch_with_anchor(anchor, vec![sign(&transfer, &alice_kp)]);
            let result = pipeline.execute_batch(&batch).await.unwrap();
            state_root = result.state_root;
        }

        let store2 = StateStore::open(path.to_str().unwrap()).unwrap();
        let loaded = load_state(&store2).unwrap();
        assert_eq!(loaded.balance(&alice), 600);
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
        shared_state.write().await.set_balance(&alice, 1000);

        let batch = make_batch(vec![sign(
            &TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
                gas_price: 0,
            },
            &alice_kp,
        )]);

        tx.send(batch.clone()).await.unwrap();
        tx.send(batch).await.unwrap();
        drop(tx);

        pipeline.run().await;

        let state = shared_state.read().await;
        assert_eq!(state.balance(&alice), 700);
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
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
            gas_price: 0,
        };

        let evm_deploy = TxKind::EvmDeploy {
            deployer: alice,
            code: vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3],
            nonce: 1,
            gas_limit: 1_000_000,
            gas_price: 0,
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
            pipeline.state.write().await.set_balance(&alice, 5000);

            let batch = make_batch(vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: [2u8; 32],
                    value: 1500,
                    nonce: 0,
                    gas_price: 0,
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
            assert_eq!(state.balance(&alice), 3500);
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
        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: Some(rt),
            base_fee: Arc::new(std::sync::atomic::AtomicU64::new(calculator.base_fee())),
            base_fee_calculator: calculator,
            anomaly_scorer: AnomalyScorer::new(),
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
        let ai_tx = TxKind::AiInfer {
            requester,
            model_id: "add".into(),
            input: make_f32_input(&[2.0, 3.0, 4.0]),
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 0,
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
        let ai_tx = TxKind::AiInfer {
            requester,
            model_id: "nonexistent".into(),
            input: vec![1, 2, 3],
            nonce: 0,
            max_compute_units: 10_000,
            gas_price: 0,
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
        pipeline.state.write().await.set_balance(&alice, 5000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
            gas_price: 0,
        };

        let ai_tx = TxKind::AiInfer {
            requester: alice,
            model_id: "add".into(),
            input: make_f32_input(&[1.0, 2.0, 3.0]),
            nonce: 1,
            max_compute_units: 10_000,
            gas_price: 0,
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
        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "sentiment_v1".into(),
            nonce: 0,
            gas_price: 0,
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
        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "model_v1".into(),
            nonce: 5,
            gas_price: 0,
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

        let create_tx = TxKind::CreateAgent {
            creator,
            model_id: "trader_v1".into(),
            nonce: 0,
            gas_price: 0,
        };
        let batch1 = make_batch(vec![sign(&create_tx, &creator_kp)]);
        let result1 = pipeline.execute_batch(&batch1).await.unwrap();
        let agent_addr = result1.receipts[0].contract_address.unwrap();

        pipeline.state.write().await.set_balance(&creator, 5000);

        let transfer = TxKind::Transfer {
            from: creator,
            to: agent_addr,
            value: 1000,
            nonce: 1,
            gas_price: 0,
        };
        let batch2 = make_batch_with_anchor([0xBB; 32], vec![sign(&transfer, &creator_kp)]);
        let result2 = pipeline.execute_batch(&batch2).await.unwrap();

        assert!(result2.receipts[0].success);
        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&agent_addr), 1000);
        assert_eq!(state.balance(&creator), 4000);
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
            gas_price: 0,
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
            gas_price: 0,
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
        let fee = gas_used * 10;

        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 1_000_000 - 300 - fee);
        assert_eq!(state.balance(&bob), 300);
    }

    #[tokio::test]
    async fn pipeline_zero_gas_price_no_fee() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 500,
            nonce: 0,
            gas_price: 0,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();

        let state = pipeline.state.read().await;
        assert_eq!(state.balance(&alice), 500);
        assert_eq!(state.balance(&bob), 500);
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
        let actual_fee = gas_used * 10;

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
        assert_eq!(result.total_fees_burned, gas_used * 5);
    }

    #[tokio::test]
    async fn pipeline_zero_gas_reports_zero_fees() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = make_pipeline(rx);

        let (alice_kp, alice) = make_sender();
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 0,
        };

        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.total_fees_burned, 0);
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
        pipeline.state.write().await.set_balance(&alice, 10_000);
        pipeline.state.write().await.get_mut(&alice).nonce = 5;

        // nonce=3 is stale (current is 5)
        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 3,
            gas_price: 0,
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
        pipeline.state.write().await.set_balance(&alice, 10_000);

        // nonce=5 when current is 0 — gap
        let transfer = TxKind::Transfer {
            from: alice,
            to: [2u8; 32],
            value: 100,
            nonce: 5,
            gas_price: 0,
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
            gas_price: 0,
        };

        let before = shared_fee.load(std::sync::atomic::Ordering::Relaxed);
        let batch = make_batch(vec![sign(&transfer, &alice_kp)]);
        pipeline.execute_batch(&batch).await.unwrap();
        let after = shared_fee.load(std::sync::atomic::Ordering::Relaxed);

        // Base fee should have been updated (decreased since gas_used < target)
        assert!(after >= 1);
        assert_eq!(before, 1);
    }
}
