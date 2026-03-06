#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use std::sync::Arc;

    use aztibase_consensus::{
        CommittedBatch, ConsensusConfig, ConsensusEngine, ConsensusInput, ConsensusOutput,
        DagStore, ValidatorSet, build_certificate, sign_finality, verify_certificate,
    };
    use aztibase_core::{BlsKeypair, hash};
    use aztibase_execution::{TxKind, compute_contract_address, get_batch_root, load_state};
    use aztibase_storage::StateStore;
    use tokio::sync::mpsc;

    use crate::pipeline::ExecutionPipeline;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path(prefix: &str) -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_integ_{prefix}_{}_{}", pid, id))
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

    // ── Sprint 008 Task 4: Multi-node consensus convergence ──────────

    #[tokio::test]
    async fn multi_node_consensus_convergence() {
        use aztibase_consensus::DagBlock;
        use std::time::Duration;

        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];

        let mut validators = ValidatorSet::new();
        validators.add(v1, 100);
        validators.add(v2, 100);
        validators.add(v3, 100);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(200),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
        };

        // Shared genesis blocks with fixed timestamp so all DAGs are identical
        let genesis_ts = 1000u64;
        let genesis_blocks: Vec<DagBlock> = [v1, v2, v3]
            .iter()
            .map(|id| DagBlock::genesis(*id, genesis_ts))
            .collect();

        let identities = [v1, v2, v3];
        let mut engine_inputs = Vec::new();
        let mut handles = Vec::new();
        let mut db_paths = Vec::new();

        let (router_tx, mut router_rx) = mpsc::channel::<(usize, ConsensusOutput)>(1024);

        for (i, &id) in identities.iter().enumerate() {
            let path = test_db_path(&format!("multinode_{i}"));
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut dag = DagStore::new(store).unwrap();
            db_paths.push(path);

            // Pre-seed DAG with identical genesis blocks
            for g in &genesis_blocks {
                dag.insert(g.clone()).unwrap();
            }

            let (in_tx, in_rx) = mpsc::channel::<ConsensusInput>(512);
            let (out_tx, mut out_rx) = mpsc::channel::<ConsensusOutput>(512);

            let mut engine =
                ConsensusEngine::new(config.clone(), id, dag, validators.clone(), in_rx, out_tx);

            engine_inputs.push(in_tx);

            handles.push(tokio::spawn(async move {
                let _ = engine.run().await;
            }));

            let rtx = router_tx.clone();
            tokio::spawn(async move {
                while let Some(output) = out_rx.recv().await {
                    if rtx.send((i, output)).await.is_err() {
                        break;
                    }
                }
            });
        }
        drop(router_tx);

        // Submit a transaction to node 0
        let transfer = TxKind::Transfer {
            from: [1u8; 32],
            to: [2u8; 32],
            value: 500,
            nonce: 0,
        };
        engine_inputs[0]
            .send(ConsensusInput::Transaction(transfer.encode()))
            .await
            .unwrap();

        // Route vertices and collect committed batches
        let mut committed: Vec<Vec<CommittedBatch>> = vec![Vec::new(), Vec::new(), Vec::new()];
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

        loop {
            if committed.iter().all(|c| !c.is_empty()) {
                break;
            }

            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }

            match tokio::time::timeout(remaining, router_rx.recv()).await {
                Ok(Some((node_idx, output))) => match output {
                    ConsensusOutput::BroadcastVertex(data) => {
                        for (j, tx) in engine_inputs.iter().enumerate() {
                            if j != node_idx {
                                let _ = tx.try_send(ConsensusInput::ReceivedVertex(data.clone()));
                            }
                        }
                    }
                    ConsensusOutput::BatchCommitted(batch) => {
                        committed[node_idx].push(batch);
                    }
                },
                _ => break,
            }
        }

        // All 3 nodes should have committed at least one batch
        for (i, batches) in committed.iter().enumerate() {
            assert!(
                !batches.is_empty(),
                "Node {i} did not commit any batch within deadline"
            );
        }

        // Verify all nodes committed a batch with the same anchor hash
        let anchor0 = committed[0][0].anchor_hash;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                batches[0].anchor_hash, anchor0,
                "Node {i} committed different anchor than node 0"
            );
        }

        // Verify all nodes have the same transaction set
        let txs0 = &committed[0][0].transactions;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                &batches[0].transactions, txs0,
                "Node {i} has different transactions than node 0"
            );
        }

        // Execute the committed batch on 3 independent pipelines
        let mut state_roots = Vec::new();
        for (i, batches) in committed.iter().enumerate() {
            let exec_path = test_db_path(&format!("multinode_exec_{i}"));
            let exec_store = Arc::new(StateStore::open(exec_path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let pipeline = ExecutionPipeline::with_storage(exec_store, rx);
            pipeline
                .shared_state()
                .write()
                .await
                .set_balance(&v1, 10_000);

            let result = pipeline.execute_batch(&batches[0]).await.unwrap();
            state_roots.push(result.state_root);
            db_paths.push(exec_path);
        }

        // All state roots must match
        assert_eq!(state_roots[0], state_roots[1], "State root 0 != 1");
        assert_eq!(state_roots[1], state_roots[2], "State root 1 != 2");
        assert_ne!(state_roots[0], [0u8; 32], "State root should not be zero");

        // Cleanup
        drop(engine_inputs);
        for h in handles {
            let _ = h.await;
        }
        for path in &db_paths {
            cleanup(path);
        }
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
        let r0 = aztibase_execution::get_receipt(&store, &result.receipts[0].tx_hash)
            .unwrap()
            .expect("receipt 0 should be persisted");
        assert!(r0.success);
        assert_eq!(r0.gas_used, 21_000);

        let r1 = aztibase_execution::get_receipt(&store, &result.receipts[1].tx_hash)
            .unwrap()
            .expect("receipt 1 should be persisted");
        assert!(!r1.success);
        assert!(r1.error.as_deref().unwrap().contains("insufficient"));

        cleanup(&path);
    }
}
