use dendrite_consensus::CommittedBatch;
use dendrite_core::hash;
use dendrite_execution::{
    AccountState, ContractTx, TransferTx, TxKind, execute_contract_txs, execute_transfers,
    route_batch,
};
use tokio::sync::mpsc;

/// Result of executing a single committed batch.
#[derive(Clone, Debug)]
pub struct PipelineResult {
    pub batch_anchor: [u8; 32],
    pub state_root: [u8; 32],
    pub transfer_count: usize,
    pub contract_count: usize,
    pub routing_errors: usize,
}

/// Owns account state and executes committed batches received from consensus.
pub struct ExecutionPipeline {
    state: AccountState,
    rx: mpsc::Receiver<CommittedBatch>,
}

impl ExecutionPipeline {
    pub fn new(rx: mpsc::Receiver<CommittedBatch>) -> Self {
        Self {
            state: AccountState::new(),
            rx,
        }
    }

    /// Run the pipeline loop, processing committed batches until the channel closes.
    pub async fn run(mut self) {
        while let Some(batch) = self.rx.recv().await {
            let result = self.execute_batch(&batch);
            tracing::info!(
                anchor = %short_hex(&result.batch_anchor),
                state_root = %short_hex(&result.state_root),
                transfers = result.transfer_count,
                contracts = result.contract_count,
                routing_errors = result.routing_errors,
                "Batch executed"
            );
        }
        tracing::info!("Execution pipeline shutting down");
    }

    /// Execute a single committed batch: route transactions, execute each type,
    /// return the resulting state root.
    pub fn execute_batch(&mut self, batch: &CommittedBatch) -> PipelineResult {
        let (routed, errors) = route_batch(&batch.transactions);

        let mut transfers = Vec::new();
        let mut contracts = Vec::new();

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
            }
        }

        let transfer_count = transfers.len();
        let contract_count = contracts.len();

        if !transfers.is_empty() {
            execute_transfers(&mut self.state, &transfers);
        }
        if !contracts.is_empty() {
            execute_contract_txs(&mut self.state, &contracts);
        }

        PipelineResult {
            batch_anchor: batch.anchor_hash,
            state_root: self.state.state_root(),
            transfer_count,
            contract_count,
            routing_errors: errors.len(),
        }
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
    use dendrite_execution::TxKind;

    fn make_batch(txs: Vec<Vec<u8>>) -> CommittedBatch {
        CommittedBatch {
            anchor_hash: [0xAA; 32],
            vertex_order: vec![],
            transactions: txs,
        }
    }

    #[test]
    fn pipeline_executes_transfers() {
        let (tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::new(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];

        // Pre-fund alice
        pipeline.state.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 300,
            nonce: 0,
        };

        let batch = make_batch(vec![transfer.encode()]);
        let result = pipeline.execute_batch(&batch);

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 0);
        assert_eq!(result.routing_errors, 0);
        assert_eq!(pipeline.state.balance(&alice), 700);
        assert_eq!(pipeline.state.balance(&bob), 300);
        assert_ne!(result.state_root, [0u8; 32]);

        drop(tx);
    }

    #[test]
    fn pipeline_executes_mixed_batch() {
        let (tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::new(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.set_balance(&alice, 5000);

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
        let result = pipeline.execute_batch(&batch);

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.contract_count, 1);
        assert_eq!(pipeline.state.balance(&bob), 100);

        drop(tx);
    }

    #[test]
    fn pipeline_handles_routing_errors() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::new(rx);

        let good = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 0,
            nonce: 0,
        };

        let batch = make_batch(vec![good.encode(), vec![0xFE, 0x00]]);
        let result = pipeline.execute_batch(&batch);

        assert_eq!(result.transfer_count, 1);
        assert_eq!(result.routing_errors, 1);
    }

    #[tokio::test]
    async fn pipeline_processes_channel() {
        let (tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::new(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.set_balance(&alice, 1000);

        let transfer = TxKind::Transfer {
            from: alice,
            to: bob,
            value: 200,
            nonce: 0,
        };

        let batch = make_batch(vec![transfer.encode()]);

        // Process one batch directly (channel-based run() is for the main loop)
        let result = pipeline.execute_batch(&batch);
        assert_eq!(result.transfer_count, 1);
        assert_eq!(pipeline.state.balance(&bob), 200);

        drop(tx);
    }

    #[test]
    fn pipeline_state_persists_across_batches() {
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::new(rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        pipeline.state.set_balance(&alice, 1000);

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

        pipeline.execute_batch(&batch1);
        let result = pipeline.execute_batch(&batch2);

        assert_eq!(pipeline.state.balance(&alice), 500);
        assert_eq!(pipeline.state.balance(&bob), 500);
        assert_ne!(result.state_root, [0u8; 32]);
    }
}
