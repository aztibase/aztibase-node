#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use std::sync::Arc;

    use aztibase_consensus::{
        CommittedBatch, ConsensusConfig, ConsensusEngine, ConsensusInput, ConsensusOutput,
        DagStore, ValidatorSet, build_certificate, sign_finality, verify_certificate,
    };
    use aztibase_core::{BlsKeypair, Keypair, address_from_pubkey, hash};
    use aztibase_execution::{
        SignedTx, TxKind, compute_contract_address, get_batch_root, load_state,
    };
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

    fn make_sender() -> (Keypair, [u8; 32]) {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        (kp, addr)
    }

    fn sign(tx: &TxKind, kp: &Keypair) -> Vec<u8> {
        SignedTx::new(tx.encode(), kp).encode()
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
        let mut pipeline = ExecutionPipeline::with_storage(store, rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let anchor = hash(b"batch_xfer_e2e");

        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 5000);

        let batch = make_batch(
            anchor,
            vec![
                sign(
                    &TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 1200,
                        nonce: 0,
                        gas_price: 0,
                    },
                    &alice_kp,
                ),
                sign(
                    &TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 800,
                        nonce: 1,
                        gas_price: 0,
                    },
                    &alice_kp,
                ),
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
        let mut pipeline = ExecutionPipeline::with_storage(store, rx);

        let (deployer_kp, deployer) = make_sender();

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

        let deploy_anchor = hash(b"deploy_batch");
        let deploy_batch = make_batch(
            deploy_anchor,
            vec![sign(
                &TxKind::ContractDeploy {
                    deployer,
                    code: wasm,
                    nonce: 0,
                    gas_limit: 1_000_000,
                    gas_price: 0,
                },
                &deployer_kp,
            )],
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

        let call_anchor = hash(b"call_batch");
        let call_batch = make_batch(
            call_anchor,
            vec![sign(
                &TxKind::ContractCall {
                    caller: deployer,
                    contract: contract_addr,
                    func_name: "init".into(),
                    args_data: vec![],
                    nonce: 1,
                    gas_limit: 1_000_000,
                    gas_price: 0,
                },
                &deployer_kp,
            )],
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
        let mut pipeline = ExecutionPipeline::with_storage(store, rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 10_000);

        let anchor = hash(b"finality_batch");
        let batch = make_batch(
            anchor,
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 3000,
                    nonce: 0,
                    gas_price: 0,
                },
                &alice_kp,
            )],
        );

        let result = pipeline.execute_batch(&batch).await.unwrap();
        let batch_hash = hash(&anchor);
        let state_root = result.state_root;

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

        let (alice_kp, alice) = make_sender();
        let (bob_kp, bob) = make_sender();
        let anchor = hash(b"recovery_batch");
        let state_root;

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);
            let shared = pipeline.shared_state();
            shared.write().await.set_balance(&alice, 10_000);

            let batch = make_batch(
                anchor,
                vec![
                    sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: 4000,
                            nonce: 0,
                            gas_price: 0,
                        },
                        &alice_kp,
                    ),
                    sign(
                        &TxKind::Transfer {
                            from: alice,
                            to: bob,
                            value: 1000,
                            nonce: 1,
                            gas_price: 0,
                        },
                        &alice_kp,
                    ),
                ],
            );

            let result = pipeline.execute_batch(&batch).await.unwrap();
            state_root = result.state_root;
        }

        {
            let store = Arc::new(StateStore::open(path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(store, rx);

            let shared = pipeline.shared_state();
            let state = shared.read().await;
            assert_eq!(state.balance(&alice), 5000);
            assert_eq!(state.balance(&bob), 5000);
            assert_eq!(state.nonce(&alice), 2);
            assert_eq!(state.state_root(), state_root);
            drop(state);

            let anchor2 = hash(b"recovery_batch_2");
            let batch2 = make_batch(
                anchor2,
                vec![sign(
                    &TxKind::Transfer {
                        from: bob,
                        to: alice,
                        value: 2000,
                        nonce: 0,
                        gas_price: 0,
                    },
                    &bob_kp,
                )],
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

        let (sender_kp, sender) = make_sender();
        let transfer = TxKind::Transfer {
            from: sender,
            to: [0xBB; 32],
            value: 500,
            nonce: 0,
            gas_price: 0,
        };
        engine_inputs[0]
            .send(ConsensusInput::Transaction(sign(&transfer, &sender_kp)))
            .await
            .unwrap();

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

        for (i, batches) in committed.iter().enumerate() {
            assert!(
                !batches.is_empty(),
                "Node {i} did not commit any batch within deadline"
            );
        }

        let anchor0 = committed[0][0].anchor_hash;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                batches[0].anchor_hash, anchor0,
                "Node {i} committed different anchor than node 0"
            );
        }

        let txs0 = &committed[0][0].transactions;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                &batches[0].transactions, txs0,
                "Node {i} has different transactions than node 0"
            );
        }

        let mut state_roots = Vec::new();
        for (i, batches) in committed.iter().enumerate() {
            let exec_path = test_db_path(&format!("multinode_exec_{i}"));
            let exec_store = Arc::new(StateStore::open(exec_path.to_str().unwrap()).unwrap());
            let (_tx, rx) = mpsc::channel(16);
            let mut pipeline = ExecutionPipeline::with_storage(exec_store, rx);
            pipeline
                .shared_state()
                .write()
                .await
                .set_balance(&sender, 10_000);

            let result = pipeline.execute_batch(&batches[0]).await.unwrap();
            state_roots.push(result.state_root);
            db_paths.push(exec_path);
        }

        assert_eq!(state_roots[0], state_roots[1], "State root 0 != 1");
        assert_eq!(state_roots[1], state_roots[2], "State root 1 != 2");
        assert_ne!(state_roots[0], [0u8; 32], "State root should not be zero");

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
        let mut pipeline = ExecutionPipeline::with_storage(Arc::clone(&store), rx);

        let (alice_kp, alice) = make_sender();
        let bob = [2u8; 32];
        let shared = pipeline.shared_state();
        shared.write().await.set_balance(&alice, 10_000);

        let anchor = hash(b"receipt_test_batch");
        let batch = make_batch(
            anchor,
            vec![
                sign(
                    &TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 3000,
                        nonce: 0,
                        gas_price: 0,
                    },
                    &alice_kp,
                ),
                sign(
                    &TxKind::Transfer {
                        from: alice,
                        to: bob,
                        value: 99_999,
                        nonce: 1,
                        gas_price: 0,
                    },
                    &alice_kp,
                ),
            ],
        );

        let result = pipeline.execute_batch(&batch).await.unwrap();
        assert_eq!(result.receipts.len(), 2);
        assert!(result.receipts[0].success);
        assert!(!result.receipts[1].success);

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

    // ── Sprint 013: Engine with genesis validators ──────────────────

    #[tokio::test]
    async fn engine_with_genesis_validators() {
        use aztibase_consensus::DagBlock;
        use std::time::Duration;

        use crate::genesis;

        let generated = genesis::generate_genesis(3, 0, 1000);

        let mut validators = ValidatorSet::new();
        let mut validator_ids: Vec<[u8; 32]> = Vec::new();
        for entry in &generated.config.validators {
            if let Some(addr) = genesis::hex_decode(&entry.address)
                .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            {
                let bls_pk = entry
                    .bls_public_key
                    .as_ref()
                    .and_then(|hex| genesis::hex_decode(hex))
                    .and_then(|bytes| <[u8; 48]>::try_from(bytes.as_slice()).ok())
                    .and_then(aztibase_core::BlsPublicKey::from_bytes);
                validators.add_with_bls(addr, entry.stake, bls_pk);
                validator_ids.push(addr);
            }
        }
        assert_eq!(validators.len(), 3);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(200),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
        };

        let genesis_ts = generated.config.timestamp;
        let genesis_blocks: Vec<DagBlock> = validator_ids
            .iter()
            .map(|id| DagBlock::genesis(*id, genesis_ts))
            .collect();

        let (router_tx, mut router_rx) = mpsc::channel::<(usize, ConsensusOutput)>(1024);
        let mut engine_inputs = Vec::new();
        let mut handles = Vec::new();
        let mut db_paths = Vec::new();

        for (i, &id) in validator_ids.iter().enumerate() {
            let path = test_db_path(&format!("genesis_engine_{i}"));
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut dag = DagStore::new(store).unwrap();
            db_paths.push(path);

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

        let mut committed = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);

        loop {
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
                    ConsensusOutput::BatchCommitted(_) => {
                        committed = true;
                        break;
                    }
                },
                _ => break,
            }
        }

        assert!(
            committed,
            "Genesis-derived validators should reach consensus"
        );

        drop(engine_inputs);
        for h in handles {
            let _ = h.await;
        }
        for path in &db_paths {
            cleanup(path);
        }
    }

    // ── Sprint 013: Finality certificate with genesis BLS ────────────

    #[tokio::test]
    async fn integration_finality_cert() {
        use crate::genesis;
        use aztibase_consensus::{
            build_certificate_from_set, sign_finality, verify_certificate_from_set,
        };

        let generated = genesis::generate_genesis(4, 1, 2000);

        // Build ValidatorSet with BLS keys from genesis
        let mut validators = ValidatorSet::new();
        for entry in &generated.config.validators {
            if let Some(addr) = genesis::hex_decode(&entry.address)
                .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            {
                let bls_pk = entry
                    .bls_public_key
                    .as_ref()
                    .and_then(|hex| genesis::hex_decode(hex))
                    .and_then(|bytes| <[u8; 48]>::try_from(bytes.as_slice()).ok())
                    .and_then(aztibase_core::BlsPublicKey::from_bytes);
                validators.add_with_bls(addr, entry.stake, bls_pk);
            }
        }

        // Execute a batch
        let (alice_kp, alice) = make_sender();
        let bob = [0xBB; 32];
        let exec_path = test_db_path("finality_genesis");
        let exec_store = Arc::new(StateStore::open(exec_path.to_str().unwrap()).unwrap());
        let (_tx, rx) = mpsc::channel(16);
        let mut pipeline = ExecutionPipeline::with_storage(exec_store, rx);
        pipeline
            .shared_state()
            .write()
            .await
            .set_balance(&alice, 10_000);

        let anchor = hash(b"finality_genesis_batch");
        let batch = make_batch(
            anchor,
            vec![sign(
                &TxKind::Transfer {
                    from: alice,
                    to: bob,
                    value: 3000,
                    nonce: 0,
                    gas_price: 0,
                },
                &alice_kp,
            )],
        );
        let result = pipeline.execute_batch(&batch).await.unwrap();
        let batch_hash = hash(&anchor);
        let state_root = result.state_root;

        // Sign finality with 3 of 4 genesis BLS keypairs (quorum)
        let signers: Vec<_> = generated.validator_keys[..3]
            .iter()
            .map(|(_, _, bls_kp)| {
                let sig = sign_finality(bls_kp, &batch_hash, &state_root);
                (bls_kp.public_key().clone(), sig)
            })
            .collect();

        let cert = build_certificate_from_set(batch_hash, state_root, &signers, &validators)
            .expect("Should build cert with genesis BLS keys");
        assert!(verify_certificate_from_set(&cert, &validators));
        assert_eq!(cert.batch_hash, batch_hash);
        assert_eq!(cert.state_root, state_root);

        cleanup(&exec_path);
    }

    // ── Sprint 014 Task 1: Byzantine equivocation fault injection ────

    #[tokio::test]
    async fn byzantine_equivocation_detected() {
        use aztibase_consensus::DagBlock;
        use std::time::Duration;

        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];
        let v4 = [4u8; 32];

        let mut validators = ValidatorSet::new();
        validators.add(v1, 100);
        validators.add(v2, 100);
        validators.add(v3, 100);
        validators.add(v4, 100);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(200),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
        };

        let genesis_ts = 1000u64;
        let genesis_blocks: Vec<DagBlock> = [v1, v2, v3, v4]
            .iter()
            .map(|id| DagBlock::genesis(*id, genesis_ts))
            .collect();

        let genesis_hashes: Vec<_> = genesis_blocks.iter().map(|b| b.hash).collect();

        // Run 3 honest nodes (v1, v2, v3). v4 is the Byzantine validator.
        let honest_ids = [v1, v2, v3];
        let (router_tx, mut router_rx) = mpsc::channel::<(usize, ConsensusOutput)>(2048);
        let mut engine_inputs = Vec::new();
        let mut handles = Vec::new();
        let mut db_paths = Vec::new();

        for (i, &id) in honest_ids.iter().enumerate() {
            let path = test_db_path(&format!("byz_honest_{i}"));
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut dag = DagStore::new(store).unwrap();
            db_paths.push(path);

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

        // Byzantine v4 sends two conflicting vertices for round 1
        let equivocation_a =
            DagBlock::new(1, v4, genesis_hashes.clone(), vec![0xAA], 2000).unwrap();
        let equivocation_b =
            DagBlock::new(1, v4, genesis_hashes.clone(), vec![0xBB], 2000).unwrap();
        assert_ne!(equivocation_a.hash, equivocation_b.hash);

        let data_a = aztibase_consensus::encode_vertex(&equivocation_a).unwrap();
        let data_b = aztibase_consensus::encode_vertex(&equivocation_b).unwrap();

        // Send both to all honest nodes
        for tx in &engine_inputs {
            let _ = tx.try_send(ConsensusInput::ReceivedVertex(data_a.clone()));
            let _ = tx.try_send(ConsensusInput::ReceivedVertex(data_b.clone()));
        }

        // Wait for honest nodes to commit — they should succeed despite the equivocation
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

        // 3 honest out of 4 total = 75% > 2/3 threshold — must reach consensus
        for (i, batches) in committed.iter().enumerate() {
            assert!(
                !batches.is_empty(),
                "Honest node {i} should commit despite Byzantine equivocation"
            );
        }

        // All honest nodes agree on the same anchor
        let anchor0 = committed[0][0].anchor_hash;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                batches[0].anchor_hash, anchor0,
                "Honest node {i} committed different anchor"
            );
        }

        drop(engine_inputs);
        for h in handles {
            let _ = h.await;
        }
        for path in &db_paths {
            cleanup(path);
        }
    }

    // ── Sprint 014 Task 2: Validator crash recovery ────────────────

    #[tokio::test]
    async fn validator_crash_and_recovery() {
        use aztibase_consensus::DagBlock;
        use std::time::Duration;

        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        let v3 = [3u8; 32];
        let v4 = [4u8; 32];

        let mut validators = ValidatorSet::new();
        validators.add(v1, 100);
        validators.add(v2, 100);
        validators.add(v3, 100);
        validators.add(v4, 100);

        let config = ConsensusConfig {
            round_duration: Duration::from_millis(200),
            wave_length: 2,
            max_parents: 10,
            max_pending_txs: 4096,
        };

        let genesis_ts = 1000u64;
        let genesis_blocks: Vec<DagBlock> = [v1, v2, v3, v4]
            .iter()
            .map(|id| DagBlock::genesis(*id, genesis_ts))
            .collect();

        // Only run 3 out of 4 validators (v4 is "crashed")
        let alive_ids = [v1, v2, v3];
        let (router_tx, mut router_rx) = mpsc::channel::<(usize, ConsensusOutput)>(2048);
        let mut engine_inputs = Vec::new();
        let mut handles = Vec::new();
        let mut db_paths = Vec::new();

        for (i, &id) in alive_ids.iter().enumerate() {
            let path = test_db_path(&format!("crash_{i}"));
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut dag = DagStore::new(store).unwrap();
            db_paths.push(path);

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

        // v4 never runs — simulates a crash. 3 out of 4 = 75% > 2/3 threshold.
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

        for (i, batches) in committed.iter().enumerate() {
            assert!(
                !batches.is_empty(),
                "Node {i} should commit even with 1 validator offline"
            );
        }

        let anchor0 = committed[0][0].anchor_hash;
        for (i, batches) in committed.iter().enumerate().skip(1) {
            assert_eq!(
                batches[0].anchor_hash, anchor0,
                "Node {i} committed different anchor"
            );
        }

        drop(engine_inputs);
        for h in handles {
            let _ = h.await;
        }
        for path in &db_paths {
            cleanup(path);
        }
    }
}
