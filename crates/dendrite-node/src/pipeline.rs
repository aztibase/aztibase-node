use std::collections::HashSet;
use std::sync::Arc;

use dendrite_consensus::CommittedBatch;
use dendrite_core::hash;
use dendrite_execution::{
    AccountState, ContractTx, ExecutionReceipt, TransferTx, TxKind, evm, execute_contract_txs,
    execute_transfers, flush_state, load_state, route_batch, store_batch_root, store_receipts,
};
use dendrite_storage::StateStore;
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
        }
    }

    /// Shared read handle to account state (for RPC server).
    pub fn shared_state(&self) -> Arc<RwLock<AccountState>> {
        Arc::clone(&self.state)
    }

    /// Shared batch counter (for RPC server).
    pub fn shared_batch_count(&self) -> Arc<std::sync::atomic::AtomicU64> {
        Arc::clone(&self.batch_count)
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
            }
        }

        let transfer_count = transfers.len();
        let contract_count = contracts.len() + evm_deploys.len() + evm_calls.len();

        let mut state = self.state.write().await;
        let mut receipts = Vec::new();

        if !transfers.is_empty() {
            let batch_result = execute_transfers(&mut state, &transfers);
            for tr in &batch_result.receipts {
                receipts.push(ExecutionReceipt {
                    tx_hash: tr.tx_hash,
                    success: matches!(tr.status, dendrite_execution::TxStatus::Success),
                    gas_used: tr.gas_used,
                    contract_address: None,
                    error: match &tr.status {
                        dendrite_execution::TxStatus::Success => None,
                        dendrite_execution::TxStatus::InsufficientBalance => {
                            Some("insufficient balance".into())
                        }
                        dendrite_execution::TxStatus::NonceMismatch { expected, got } => {
                            Some(format!("nonce mismatch: expected {expected}, got {got}"))
                        }
                    },
                });
            }
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
            });
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
    use dendrite_execution::{TxKind, get_batch_root, load_state};
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_pipeline_test_{}_{}", pid, id))
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
}
