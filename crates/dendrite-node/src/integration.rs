#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use std::sync::Arc;

    use dendrite_consensus::{
        CommittedBatch, ValidatorSet, build_certificate, sign_finality, verify_certificate,
    };
    use dendrite_core::{BlsKeypair, hash};
    use dendrite_execution::{TxKind, compute_contract_address, get_batch_root, load_state};
    use dendrite_storage::StateStore;
    use tokio::sync::mpsc;

    use crate::pipeline::ExecutionPipeline;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path(prefix: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_integ_{prefix}_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn make_batch(anchor: [u8; 32], txs: Vec<Vec<u8>>) -> CommittedBatch {
        CommittedBatch {
            anchor_hash: anchor,
            vertex_order: vec![],
            transactions: txs,
        }
    }

    // ── Task 16: Transfer end-to-end ─────────────────────────────────

    #[tokio::test]
    async fn transfer_end_to_end() {
        let path = test_db_path("xfer");
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = ExecutionPipeline::with_storage(store, rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let anchor = hash(b"batch_xfer_e2e");

        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 5000);

        let batch = make_batch(
            anchor,
            vec![
                TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 1200,
                    nonce: 0,
                }
                .encode(),
                TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 800,
                    nonce: 1,
                }
                .encode(),
            ],
        );

        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.transfer_count, 2);
        assert_eq!(result.contract_count, 0);
        assert_eq!(result.routing_errors, 0);

        let shared = pipeline.shared_state();
        let state = shared.read().await;
        assert_eq!(state.balance(&alice), 3000);
        assert_eq!(state.balance(&bob), 2000);
        assert_eq!(state.nonce(&alice), 2);
        assert_ne!(result.state_root, [0u8; 32]);
        drop(state);
        drop(shared);
        drop(pipeline);

        // Verify persisted to redb
        let store2 = StateStore::open(path.to_str().unwrap()).unwrap();
        let loaded = load_state(&store2).unwrap();
        assert_eq!(loaded.balance(&alice), 3000);
        assert_eq!(loaded.balance(&bob), 2000);

        let root = get_batch_root(&store2, &anchor).unwrap();
        assert_eq!(root, Some(result.state_root));

        cleanup(&path);
    }

    // ── Task 17: Contract deploy + call end-to-end ───────────────────

    #[tokio::test]
    async fn contract_deploy_and_call_end_to_end() {
        let path = test_db_path("contract");
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = ExecutionPipeline::with_storage(store, rx);

        let deployer = [1u8; 32];

        let wasm = wat::parse_str(
            r#"
            (module
                (import "env" "storage_set"
                    (func $storage_set (param i32 i32 i32 i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "counter")
                (data (i32.const 10) "\01\00\00\00")
                (func (export "init")
                    (call $storage_set
                        (i32.const 0) (i32.const 7)
                        (i32.const 10) (i32.const 4)))
            )
            "#,
        )
        .unwrap();

        // Phase 1: Deploy
        let deploy_anchor = hash(b"deploy_batch");
        let deploy_batch = make_batch(
            deploy_anchor,
            vec![
                TxKind::ContractDeploy {
                    deployer,
                    code: wasm,
                    nonce: 0,
                    gas_limit: 1_000_000,
                }
                .encode(),
            ],
        );

        let deploy_result = pipeline.execute_batch(&deploy_batch).await.unwrap();
        assert_eq!(deploy_result.contract_count, 1);

        let contract_addr = compute_contract_address(&deployer, 0);
        let shared = pipeline.shared_state();
        let state = shared.read().await;
        assert!(state.code(&contract_addr).is_some());
        assert_eq!(state.nonce(&deployer), 1);
        let root_after_deploy = state.state_root();
        drop(state);

        // Phase 2: Call
        let call_anchor = hash(b"call_batch");
        let call_batch = make_batch(
            call_anchor,
            vec![
                TxKind::ContractCall {
                    caller: deployer,
                    contract: contract_addr,
                    func_name: "init".into(),
                    args_data: vec![],
                    nonce: 1,
                    gas_limit: 1_000_000,
                }
                .encode(),
            ],
        );

        let call_result = pipeline.execute_batch(&call_batch).await.unwrap();
        assert_eq!(call_result.contract_count, 1);

        let shared = pipeline.shared_state();
        let state = shared.read().await;
        assert_eq!(
            state.get_storage(&contract_addr, b"counter").unwrap(),
            &[1, 0, 0, 0]
        );
        assert_ne!(state.state_root(), root_after_deploy);

        cleanup(&path);
    }

    // ── Task 18: Finality certificate for committed batch ────────────

    #[tokio::test]
    async fn finality_certificate_end_to_end() {
        let path = test_db_path("finality");
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = ExecutionPipeline::with_storage(store, rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 10_000);

        let anchor = hash(b"finality_batch");
        let batch = make_batch(
            anchor,
            vec![
                TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 3000,
                    nonce: 0,
                }
                .encode(),
            ],
        );

        let result = pipeline.execute_batch(&batch).await.unwrap();
        let batch_hash = hash(&anchor);
        let state_root = result.state_root;

        // Generate BLS signatures from 4 validators
        let mut vs = ValidatorSet::new();
        let mut keypairs = Vec::new();
        let mut bls_keys = Vec::new();

        for i in 0..4u8 {
            vs.add([i + 1; 32], 100);
            let kp = BlsKeypair::generate();
            bls_keys.push(kp.public_key().clone());
            keypairs.push(kp);
        }

        let signers: Vec<_> = keypairs[..3]
            .iter()
            .map(|kp| {
                let sig = sign_finality(kp, &batch_hash, &state_root);
                (kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate(batch_hash, state_root, &bls_keys, &signers, &vs).unwrap();

        assert_eq!(cert.batch_hash, batch_hash);
        assert_eq!(cert.state_root, state_root);
        assert!(verify_certificate(&cert, &bls_keys, &vs));

        cleanup(&path);
    }

    // ── Task 19: Startup recovery ────────────────────────────────────

    #[tokio::test]
    async fn startup_recovery_end_to_end() {
        let path = test_db_path("recovery");

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let anchor = hash(b"recovery_batch");
        let state_root;

        // Pipeline 1: execute transfers, flush to redb
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);
            let shared = pipeline.shared_state();
            shared.write().await.set_balance(&alice, 10_000);

            let batch = make_batch(
                anchor,
                vec![
                    TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 4000,
                        nonce: 0,
                    }
                    .encode(),
                    TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 1000,
                        nonce: 1,
                    }
                    .encode(),
                ],
            );

            let result = pipeline.execute_batch(&batch).await.unwrap();
            state_root = result.state_root;
        }

        // Pipeline 2: recover state from redb, verify, execute more
        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(store, rx);

            let shared = pipeline.shared_state();
            let state = shared.read().await;
            assert_eq!(state.balance(&alice), 5000);
            assert_eq!(state.balance(&bob), 5000);
            assert_eq!(state.nonce(&alice), 2);
            assert_eq!(state.state_root(), state_root);
            drop(state);

            // Execute additional batch on recovered state
            let anchor2 = hash(b"recovery_batch_2");
            let batch2 = make_batch(
                anchor2,
                vec![
                    TxKind::Transfer {
                        from: bob,
                        to: alice,
                        value: 2000,
                        nonce: 0,
                    }
                    .encode(),
                ],
            );

            let result2 = pipeline.execute_batch(&batch2).await.unwrap();
            assert_ne!(result2.state_root, state_root);

            let shared = pipeline.shared_state();
            let state = shared.read().await;
            assert_eq!(state.balance(&alice), 7000);
            assert_eq!(state.balance(&bob), 3000);
        }

        cleanup(&path);
    }

    // ── Task 4 (Sprint 007): Receipt persistence end-to-end ──────────

    #[tokio::test]
    async fn receipt_persistence_end_to_end() {
        let path = test_db_path("receipts");
        let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let pipeline = ExecutionPipeline::with_storage(Arc::clone(&store), rx);

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 10_000);

        let anchor = hash(b"receipt_test_batch");
        let batch = make_batch(
            anchor,
            vec![
                TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 3000,
                    nonce: 0,
                }
                .encode(),
                TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 99_999,
                    nonce: 1,
                }
                .encode(),
            ],
        );

        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(!result.receipts[1].success);

        // Verify receipts persisted to redb
        let r0 = dendrite_execution::get_receipt(&store, &result.receipts[0].tx_hash)
            .unwrap()
            .expect("receipt 0 should be persisted");
        assert!(r0.success);
        assert_eq!(r0.gas_used, 21_000);

        let r1 = dendrite_execution::get_receipt(&store, &result.receipts[1].tx_hash)
            .unwrap()
            .expect("receipt 1 should be persisted");
        assert!(!r1.success);
        assert!(r1.error.as_deref().unwrap().contains("insufficient"));

        cleanup(&path);
    }
}
