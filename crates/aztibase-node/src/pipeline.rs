use std::collections::HashSet;
use std::sync::Arc;

use aztibase_consensus::CommittedBatch;
use aztibase_core::hash;
use aztibase_execution::{
    AccountState, ContractTx, ExecutionReceipt, TransferTx, TxKind,
    block_stm::{BlockSTMExecutor, apply_block_stm_to_state},
    evm, execute_contract_txs, flush_state, load_state, route_batch, store_batch_root,
    store_receipts,
};
use aztibase_runtime::{AIRuntime, InferenceRequest};
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
        Self {
            state: Arc::new(RwLock::new(state)),
            store: Some(store),
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: None,
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

    /// Execute a single committed batch: route transactions, execute each type,
    /// flush to disk, return the resulting state root.
    ///
    /// Returns `Err` if state flush or batch root storage fails — the caller
    /// must treat this as fatal and halt.
    pub async fn execute_batch(
        &self,
        batch: &CommittedBatch,
    ) -> Result<PipelineResult, anyhow::Error> {
        let (routed, errors) = route_batch(&batch.transactions);

        let mut transfers = Vec::new();
        let mut contracts = Vec::new();
        let mut evm_deploys = Vec::new();
        let mut evm_calls = Vec::new();
        let mut ai_infers = Vec::new();

        for tx in &routed {
            match tx {
                TxKind::Transfer {
                    from,
                    to,
                    value,
                    nonce,
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
                } => {
                    ai_infers.push((
                        *requester,
                        model_id.clone(),
                        input.clone(),
                        *nonce,
                        *max_compute_units,
                    ));
                }
            }
        }

        let transfer_count = transfers.len();
        let contract_count =
            contracts.len() + evm_deploys.len() + evm_calls.len() + ai_infers.len();

        let mut state = self.state.write().await;
        let mut receipts = Vec::new();

        if !transfers.is_empty() {
            let (stm_outputs, write_sets) = BlockSTMExecutor::execute_full(&transfers, &state);

            for (i, output) in stm_outputs.iter().enumerate() {
                receipts.push(ExecutionReceipt {
                    tx_hash: transfers[i].hash,
                    success: output.success,
                    gas_used: output.gas_used,
                    contract_address: None,
                    error: output.error.clone(),
                    inference_hash: None,
                });
            }

            apply_block_stm_to_state(&mut state, &write_sets);
        }
        if !contracts.is_empty() {
            let contract_receipts = execute_contract_txs(&mut state, &contracts);
            for cr in &contract_receipts {
                receipts.push(ExecutionReceipt {
                    tx_hash: cr.tx_hash,
                    success: cr.success,
                    gas_used: cr.gas_used,
                    contract_address: cr.contract_address,
                    error: cr.error.clone(),
                    inference_hash: None,
                });
            }
        }

        for (deployer, code, nonce, gas_limit) in &evm_deploys {
            let tx_hash = hash(code);
            let cr = evm::evm_deploy(&mut state, tx_hash, deployer, code, *nonce, *gas_limit);
            receipts.push(ExecutionReceipt {
                tx_hash: cr.tx_hash,
                success: cr.success,
                gas_used: cr.gas_used,
                contract_address: cr.contract_address,
                error: cr.error,
                inference_hash: None,
            });
        }

        for (caller, contract, calldata, nonce, gas_limit, value) in &evm_calls {
            let tx_hash = hash(calldata);
            let cr = evm::evm_call(
                &mut state, tx_hash, caller, contract, calldata, *nonce, *gas_limit, *value,
            );
            receipts.push(ExecutionReceipt {
                tx_hash: cr.tx_hash,
                success: cr.success,
                gas_used: cr.gas_used,
                contract_address: cr.contract_address,
                error: cr.error,
                inference_hash: None,
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
                        },
                        Err(e) => ExecutionReceipt {
                            tx_hash,
                            success: false,
                            gas_used: 0,
                            contract_address: None,
                            error: Some(format!("inference failed: {e}")),
                            inference_hash: None,
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
                },
            };
            receipts.push(receipt);
        }

        let state_root = state.state_root();

        if let Some(ref store) = self.store {
            flush_state(store, &state)
                .map_err(|e| anyhow::anyhow!("fatal: flush_state failed: {e}"))?;
            store_batch_root(store, &batch.anchor_hash, &state_root)
                .map_err(|e| anyhow::anyhow!("fatal: store_batch_root failed: {e}"))?;
            store_receipts(store, &receipts)
                .map_err(|e| anyhow::anyhow!("fatal: store_receipts failed: {e}"))?;
        }

        Ok(PipelineResult {
            batch_anchor: batch.anchor_hash,
            state_root,
            transfer_count,
            contract_count,
            routing_errors: errors.len(),
            receipts,
        })
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
    use aztibase_execution::{TxKind, get_batch_root, load_state};
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

    fn make_pipeline(rx: mpsc::Receiver<CommittedBatch>) -> ExecutionPipeline {
        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: None,
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
        let pipeline = make_pipeline(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 300,
            nonce: 0,
        };

        let batch = make_batch(vec![transfer.encode()]);
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
        let pipeline = make_pipeline(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 5000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
        };

        let deploy = TxKind::ContractDeploy {
            deployer: alice,
            code: vec![0x00, 0x61, 0x73, 0x6d],
            nonce: 1,
            gas_limit: 1_000_000,
        };

        let batch = make_batch(vec![transfer.encode(), deploy.encode()]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 1);
        assert_eq!(pipeline.state.read().await.balance(&bob), 100);
    }

    #[tokio::test]
    async fn pipeline_handles_routing_errors() {
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = make_pipeline(rx);

        let good = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 0,
            nonce: 0,
        };

        let batch = make_batch(vec![good.encode(), vec![0xFE, 0x00]]);
        let result = pipeline.execute_batch(&batch).await.unwrap();

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.routing_errors, 1);
    }

    #[tokio::test]
    async fn pipeline_processes_channel() {
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = make_pipeline(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 200,
            nonce: 0,
        };

        let batch = make_batch(vec![transfer.encode()]);
        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.transfer_count, 1);
        assert_eq!(pipeline.state.read().await.balance(&bob), 200);
    }

    #[tokio::test]
    async fn pipeline_state_persists_across_batches() {
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = make_pipeline(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 1000);

        let batch1 = make_batch(vec![
            TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
            }
            .encode(),
        ]);

        let batch2 = make_batch(vec![
            TxKind::Transfer {
                from: alice,
                to: bob,
                value: 200,
                nonce: 1,
            }
            .encode(),
        ]);

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
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let anchor = [0xBB; 32];
        let state_root;

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&alice, 1000);

            let transfer = TxKind::Transfer {
                from: alice,
                to: bob,
                value: 400,
                nonce: 0,
            };

            let batch = make_batch_with_anchor(anchor, vec![transfer.encode()]);
            let result = pipeline.execute_batch(&batch).await.unwrap();
            state_root = result.state_root;
        }

        // Reopen db and verify flushed state
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

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let shared_state = pipeline.shared_state();
        shared_state.write().await.set_balance(&alice, 1000);

        let batch = make_batch(vec![
            TxKind::Transfer {
                from: alice,
                to: bob,
                value: 300,
                nonce: 0,
            }
            .encode(),
        ]);

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
        let pipeline = make_pipeline(rx);

        let deployer = [1u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&deployer, 1_000_000_000);

        // EVM init code: stores 0x42 at memory[0], returns 32 bytes as runtime
        let init_code = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];

        let evm_deploy = TxKind::EvmDeploy {
            deployer,
            code: init_code,
            nonce: 0,
            gas_limit: 1_000_000,
        };

        let batch = make_batch(vec![evm_deploy.encode()]);
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
        let pipeline = make_pipeline(rx);

        let deployer = [1u8; 32];
        pipeline
            .state
            .write()
            .await
            .set_balance(&deployer, 1_000_000_000);

        // Init code: deploys a 1-byte STOP runtime
        let init_code = vec![0x60, 0x00, 0x60, 0x00, 0x53, 0x60, 0x01, 0x60, 0x00, 0xf3];

        let evm_deploy = TxKind::EvmDeploy {
            deployer,
            code: init_code.clone(),
            nonce: 0,
            gas_limit: 1_000_000,
        };

        let batch1 = make_batch(vec![evm_deploy.encode()]);
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
        };

        let batch2 = make_batch_with_anchor([0xBB; 32], vec![evm_call.encode()]);
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
        let pipeline = make_pipeline(rx);

        let alice = [1u8; 32];
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
        };

        let evm_deploy = TxKind::EvmDeploy {
            deployer: alice,
            code: vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3],
            nonce: 1,
            gas_limit: 1_000_000,
        };

        let batch = make_batch(vec![transfer.encode(), evm_deploy.encode()]);
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

        // First pipeline: execute a transfer and flush
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            pipeline.state.write().await.set_balance(&[1u8; 32], 5000);

            let batch = make_batch(vec![
                TxKind::Transfer {
                    from: [1u8; 32],
                    to: [2u8; 32],
                    value: 1500,
                    nonce: 0,
                }
                .encode(),
            ]);
            pipeline.execute_batch(&batch).await.unwrap();
        }

        // Second pipeline: should recover state from redb
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            let state = pipeline.state.read().await;
            assert_eq!(state.balance(&[1u8; 32]), 3500);
            assert_eq!(state.balance(&[2u8; 32]), 1500);
            assert_eq!(state.nonce(&[1u8; 32]), 1);
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

        ExecutionPipeline {
            state: Arc::new(RwLock::new(AccountState::new())),
            store: None,
            rx,
            batch_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executed_anchors: HashSet::new(),
            result_tx: None,
            ai_runtime: Some(rt),
        }
    }

    fn make_f32_input(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    #[tokio::test]
    async fn pipeline_ai_infer_success() {
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = make_ai_pipeline(rx);

        let ai_tx = TxKind::AiInfer {
            requester: [1u8; 32],
            model_id: "add".into(),
            input: make_f32_input(&[2.0, 3.0, 4.0]),
            nonce: 0,
            max_compute_units: 10_000,
        };

        let batch = make_batch(vec![ai_tx.encode()]);
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
        let pipeline = make_ai_pipeline(rx);

        let ai_tx = TxKind::AiInfer {
            requester: [1u8; 32],
            model_id: "nonexistent".into(),
            input: vec![1, 2, 3],
            nonce: 0,
            max_compute_units: 10_000,
        };

        let batch = make_batch(vec![ai_tx.encode()]);
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
        let pipeline = make_ai_pipeline(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.write().await.set_balance(&alice, 5000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 100,
            nonce: 0,
        };

        let ai_tx = TxKind::AiInfer {
            requester: alice,
            model_id: "add".into(),
            input: make_f32_input(&[1.0, 2.0, 3.0]),
            nonce: 1,
            max_compute_units: 10_000,
        };

        let batch = make_batch(vec![transfer.encode(), ai_tx.encode()]);
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
}
