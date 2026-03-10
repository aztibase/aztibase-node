# PROJECT STATUS: Aztibase Network

**Last Updated:** 2026-03-10
**Updated By:** node-engineer / consensus-engineer / p2p-network-engineer
**Current Phase:** M9 -- Mainnet Prep (Sprint 9 IN PROGRESS)
**Current Sprint:** Sprint 048 -- COMPLETE (Security Flag Resolution — All 9 ELEVATED Flags Resolved)

---

## Milestone Tracker

| Milestone | Description | Status | Date Started | Date Completed |
|-----------|-------------|--------|--------------|----------------|
| M0 | Design complete (all MASTER_DESIGN sections) | DONE | 2026-03-05 | 2026-03-05 |
| M1 | Core primitives (data types, crypto, basic structs) | DONE | 2026-03-05 | 2026-03-05 |
| M2 | P2P networking + basic consensus | DONE | 2026-03-05 | 2026-03-06 |
| M3 | Execution layer (WASM VM, state management) | DONE | 2026-03-06 | 2026-03-06 |
| M4 | Integration testing + basic AI + testnet | DONE | 2026-03-06 | 2026-03-07 |
| M5 | Light/browser nodes + wallet | DONE | 2026-03-07 | 2026-03-07 |
| M6 | AI compute market + PoUW | DONE | 2026-03-07 | 2026-03-07 |
| M7 | Security audit + hardening | DONE | 2026-03-08 | 2026-03-08 |
| M8 | Public testnet | DONE | 2026-03-08 | 2026-03-10 |
| M9 | Mainnet launch | IN PROGRESS | 2026-03-10 | -- |

---

## Current State

### Completed
- Full design phase (Phases 3-6) delivered
- All 12 skills defined and operational
- Workspace scaffolded: 8 Rust crates in monorepo
- Build-phase tracking infrastructure (5 documents)
- Implementation depth audit (Sprint 001, Task 7)
- Reference repos cloned (MystiCeti, Sui, Lighthouse, rust-libp2p, redb)
- cargo build + cargo test pass (811 tests, 0 failures)
- Sprint 001 closed with retrospective
- Sprint 002 closed: 24/24 tasks, 5 phases complete
- Sprint 003 closed: 18/18 tasks, 4 phases complete
- Sprint 004 closed: 19/19 tasks, 4 phases complete
- Sprint 005 closed: 21/21 tasks, 4 phases complete
- Sprint 006 closed: 23/24 tasks (1 deferred), 4 phases complete
- Sprint 007 complete: receipt store, EVM via revm, AI inference via tract
- Sprint 023 complete: 16/16 tasks, 4 phases (DeregisterCompute stake refund, TaskAssigner pipeline wiring, CommitCompute model validation, DeregisterModel owner auth, M6 closed)
- Sprint 022 complete: 16/16 tasks, 4 phases (TaskPool wiring, SubmitAttestation Ed25519 sig verify, CommitCompute stake bond, settlement flow, 3 RPC endpoints, security review)
- Sprint 021 complete: 16/16 tasks, 4 phases (ModelRegistry, PoUW multi-metric scoring, task marketplace + settlement, security review)
- Sprint 020 complete: 16/16 tasks, 4 phases (WebSocket gateway, RPC subscriptions, wallet management CLI, Docker deployment)
- Sprint 019 complete: 16/16 tasks, 4 phases (WASM light client crate, browser transport bridge + demo, HD wallet derivation, security review)
- Sprint 018 complete: 16/16 tasks, 4 phases (light sync P2P handler, sync loop with peer scoring, wallet transfer broadcast, security review)
- Sprint 017 complete: 16/16 tasks, 4 phases (wallet hardening BIP-39/Argon2id, light node storage, header sync protocol, security review)
- Sprint 016 complete: 16/16 tasks, 4 phases (anomaly scoring, cross-VM bridge, PoUW type foundations, security review)
- Sprint 015 complete: 15/15 tasks, 4 phases (metrics telemetry, light client proofs, WebRTC transport scaffold, Block-STM race fix, security review)
- Sprint 014 complete: 15/15 tasks, 4 phases (consensus resilience, criterion benchmarks, Verkle tree foundations, security review)
- Sprint 013 complete: 15/15 tasks, 4 phases (genesis-driven validator bootstrap, BLS keys in genesis, testnet integration, security review)
- Sprint 012 complete: 15/15 tasks, 4 phases (fee escrow, persistent base fee, genesis config, CLI wallet, security review)
- Sprint 011 complete: 15/15 tasks, 4 phases (signed tx envelopes, nonce enforcement, fee market, security review)
- Sprint 010 complete: 15/15 tasks, 4 phases (rayon parallelism, gossipsub hardening, account abstraction, security review)
- Sprint 009 complete: 16/16 tasks, 4 phases (Block-STM, state sync, EVM precompiles, security review)
- Sprint 008 complete: 17/17 tasks, 4 phases (multi-node testnet, AI pipeline, tx pool, security review)
  - Phase 1: multi-node testnet (wire format, vertex routing, state broadcast, convergence test)
  - Phase 2: AI inference in execution pipeline (TxKind::AiInfer, pipeline routing, inference receipts, model preload)
  - Phase 3: Transaction pool improvements (priority ordering, eviction, hash dedup)
  - Phase 4: Security review (2 MEDIUM — 1 fixed, 1 documented; 4 LOW), cargo-audit clean, docs updated

### Crate Status

| Crate | Depth | Owner | M1-Ready | M3-Ready |
|-------|-------|-------|----------|----------|
| aztibase-core | COMPLETE | blockchain-architect | YES | YES |
| aztibase-consensus | PARTIAL | consensus-engineer | YES | YES |
| aztibase-storage | PARTIAL | node-engineer | YES | YES |
| aztibase-network | PARTIAL | p2p-network-engineer | YES | YES |
| aztibase-execution | PARTIAL | smart-contract-engineer | YES | YES |
| aztibase-runtime | PARTIAL | ai-integration-engineer | YES | YES |
| aztibase-rpc | PARTIAL | node-engineer | NO | YES |
| aztibase-node | PARTIAL | node-engineer | YES | YES |
| aztibase-wasm | PARTIAL | p2p-network-engineer | NO | YES |

### In Progress
- Nothing currently in progress

### Blocked
- Nothing currently blocked

### Recently Completed
- Sprint 048 Phase 10: S8-1 — Quorum-signed DHT records (≥2 sigs, round freshness, 8 tests), ADR-020
- Sprint 048 Phase 9: S4-1 — Browser wallet spending limits (per-tx 100 AZTB, session 10K, WASM exports, 5 tests), ADR-019
- Sprint 048 Phase 8: S2-2 — RANDAO-style VRF seed accumulation from causal DAG history, ADR-018
- Sprint 048 Phase 7: S1-2 — Quantum migration drill (Verkle↔Merkle swap proven, 2 tests), ADR-017
- Sprint 048 Phase 6: S1-1 — MEV mitigation Phase 1 (MAX_TX_SIZE 256 KiB at mempool, 2 tests), ADR-016
- Sprint 048 Phase 4: S2-1 — Equivocation window 20→100 rounds, persistent proofs in redb, ADR-015
- Sprint 048 Phase 3: S3-1 — Flash-loan resistant governance: snapshot balances at proposal creation
- Sprint 048 Phase 2: S1-3 — Agent spending pre-execution validation in mempool (defense-in-depth)
- Sprint 048 Phase 1: S4-3 — Checkpoint P2P distribution (request/response + gossip announce topic)
- Sprint 047 Phase 6: Docs — BUILD_LOG, STATUS, CHANGELOG, sprint plan updated
- Sprint 047 Phase 4: Monitoring — Prometheus JSON metrics verified on all 3 nodes
- Sprint 047 Phase 3: Core functionality — RPC verified (nodeInfo, chainId, getBalance, gasPrice, health), consensus producing blocks (56-81 commits in 40s), 2 transfers confirmed (500 + 1000 tokens)
- Sprint 047 Phase 2: Local testnet — 3-node local testnet running, full mesh P2P (2 peers each), 13 bugs found and fixed
- Sprint 047 Phase 1: Release build — cargo build --release (~28 min clean), 47MB binary
- Sprint 046 Phase 5: Docs — BUILD_LOG, STATUS, CHANGELOG, sprint plan updated
- Sprint 046 Phase 4: Tests & validation — 0 clippy warnings, fmt clean, 811 tests pass (4 new), 0 ELEVATED, 0 MEDIUM
- Sprint 046 Phase 3: CLI & RPC — --checkpoint startup validation, aztb_getCheckpoint + aztb_latestCheckpoint endpoints
- Sprint 046 Phase 2: Storage & emission — raw checkpoint persistence (store/get/latest), pipeline emits every 1000 batches
- Sprint 046 Phase 1: Checkpoint type — Checkpoint struct with optional finality_cert, CHECKPOINT_INTERVAL=1000
- Sprint 045 Phase 4: Docs — BUILD_LOG, STATUS, CHANGELOG, sprint plan updated
- Sprint 045 Phase 3: Tests & validation — 0 clippy warnings, fmt clean, 804 tests pass (4 new), S0-1 RESOLVED, 0 ELEVATED, 0 MEDIUM
- Sprint 045 Phase 2: Attestation domain separation — already present (AZTB_ATTESTATION_V1\0 prefix), no changes needed
- Sprint 045 Phase 1: Strict Ed25519 — verify() → verify_strict(), removed Verifier import, rejects malleable S and weak keys
- Sprint 044 Phase 4: Docs + security review — clippy 0 warnings, fmt clean, 796 tests pass (10 new), 0 ELEVATED, 0 MEDIUM
- Sprint 044 Phase 2-3: Genesis hash wiring — TransportConfig genesis_hash field, main.rs pass-through, 10 unit tests
- Sprint 044 Phase 1: Chain-scoped protocol IDs — genesis_hex_prefix(), chain_scoped_topics(), kademlia_protocol(), TOPIC_BASE_NAMES
- Sprint 043 Phase 5: Docs + security review — clippy 0 warnings, fmt clean, 786 tests pass (8 new), 0 ELEVATED, 0 MEDIUM
- Sprint 043 Phase 3: Pipeline execution — SetAgentPolicy (owner auth, AIAgent check), AgentExecute (policy validation, spend tracking, transfer)
- Sprint 043 Phase 1-2: AgentPolicy store, TxKind::SetAgentPolicy (0x14), TxKind::AgentExecute (0x15), aztb_getAgentPolicy RPC
- Sprint 042 Phase 4: Genesis validation — validate_genesis() with 10 error variants, startup enforcement, genesis hash logging
- Sprint 041 Phase 4: Gas enforcement — gas_price >= base_fee validation, estimate_gas for all 19 TxKinds
- Sprint 040 Phase 4: u64→u128 balance migration across all monetary types
- Sprint 039 Phase 4: Security review — clippy 0 warnings, fmt clean, 770+ tests pass (7+ new), 0 ELEVATED, 0 MEDIUM
- Sprint 039 Phase 3: Staking metrics — active_validators, total_staked, slashes_applied in Prometheus + JSON
- Sprint 039 Phase 2: Genesis bootstrap — genesis validators registered in StakingStore, epoch_length as ChainParam
- Sprint 039 Phase 1: Consensus-pipeline bridge — EquivocationDetected output, SlashEvent wiring, ValidatorSet sync, StakingStore→RPC
- Sprint 038 Phase 4: Security review — clippy 0 warnings, fmt clean, 759 tests pass (~34 new), 0 ELEVATED, 0 MEDIUM
- Sprint 038 Phase 3: Staking RPCs + ChainParams — aztb_getValidatorStake, aztb_getDelegation, aztb_getActiveValidators, aztb_getUnbondingStatus; 3 new governance params (min_validator_stake, max_stake_cap, validator_commission_bps)
- Sprint 038 Phase 2: Epoch rewards + validator set — distribute_epoch_rewards with 10% commission, active_set_snapshot, epoch_participation for downtime detection
- Sprint 038 Phase 1: StakingStore + pipeline — 4 TxKinds (Stake/Unstake/Delegate/Undelegate), full pipeline execution, unbonding queue, SlashEvent channel, equivocation + downtime slashing
- Sprint 037 Phase 4: Security review — clippy 0 warnings, fmt clean, 726 tests pass (39 new), 0 ELEVATED, 0 MEDIUM
- Sprint 037 Phase 3: Pipeline & RPC integration — EmissionTracker wired into pipeline, aztb_getEmissionInfo + aztb_getVestingStatus RPCs
- Sprint 037 Phase 1-2: Tokenomics module — 1B hard cap, 2-year halving emission, 8 genesis allocations (400M total) with vesting, piecewise APY curve, epoch reward distribution, 36 unit tests
- Sprint 036 Phase 4: Security review — clippy 0 warnings, fmt clean, 687 tests pass (13 new), 0 ELEVATED, 0 MEDIUM
- Sprint 036 Phase 3: Pipeline & RPC integration — dynamic fee update via update_with_params(), dynamic eviction limits, aztb_getChainParam + aztb_listChainParams RPCs
- Sprint 036 Phase 2: Proposal execution — passed_unexecuted(), auto-execute in pipeline, mark_executed on success or failure
- Sprint 036 Phase 1: ChainParams registry — 9 typed params with bounds, get/set/list, set_from_str, 7 unit tests
- Sprint 035 Phase 4: Security review — clippy 0 warnings, fmt clean, 674 tests pass (8 new), 0 ELEVATED, 0 MEDIUM
- Sprint 035 Phase 3: Governance RPCs — aztb_getProposal (with tally), aztb_listProposals (status filter), proposal finalization in batch loop
- Sprint 035 Phase 2: Governance TxKinds — CreateProposal (0x0E), CastVote (0x0F), pipeline routing + execution, stake-weighted voting
- Sprint 035 Phase 1: GovernanceStore — Proposal/Vote/VoteTally types, create_proposal/cast_vote/finalize_expired, quorum (≥2 voters, >50% approve weight), 6 unit tests
- Sprint 034 Phase 4: Security review — clippy 0 warnings, fmt clean, all tests pass, 0 ELEVATED, 0 MEDIUM
- Sprint 034 Phase 3: Block range & explorer RPCs — getBlockRange (MAX_BLOCK_RANGE=100 pagination), getTransactionsByBatch, getReceiptsByBatch
- Sprint 034 Phase 2: Historical query RPCs — getBlockByNumber, getBlockByHash, getTransactionByHash, getBatchRoot, BATCH_INDEX_TABLE, BATCH_TXS_TABLE, 4 persist tests
- Sprint 034 Phase 1: Archive mode — --archive CLI flag, gated eviction in pipeline, gated DAG+state pruning in consensus engine
- Sprint 033 Phase 4: Security review — 455+ tests (105 consensus + 175 execution + 175 node), 0 ELEVATED, 0 MEDIUM, clippy 0 warnings, fmt clean
- Sprint 033 Phase 3: Disk table eviction — StateStore::delete_batch(), TX_TABLE eviction (500K), BATCH_ROOTS_TABLE eviction (100K), 3 new tests
- Sprint 033 Phase 2: Pipeline memory caps — bounded executed_anchors (VecDeque+HashSet, 10K), per-task attestation cap (32), total attestation buffer cap (2048), disk eviction wired after batch
- Sprint 033 Phase 1: DagStore pruning — prune_before() removes old rounds from index + disk, 16-round retention buffer, 5 new tests
- Sprint 032 Phase 4: Security review — 175 node tests (14 new), 0 ELEVATED, 0 MEDIUM, clippy 0 warnings, fmt clean
- Sprint 031 Phase 4: Security review — 161 node tests, 0 ELEVATED, 0 MEDIUM, clippy 0 warnings, fmt clean
- Sprint 031 Phase 3: Edge case tests — duplicate model registration, deregister with pending tasks, mixed batch types
- Sprint 031 Phase 2: AI compute market e2e — RegisterModel, CommitCompute, PostTask, SubmitAttestation, full lifecycle, DeregisterCompute
- Sprint 031 Phase 1: RPC-driven e2e tests — fee market transfer, multi-transfer stress (20 txs), nonce gap rejection, insufficient balance
- Sprint 031 BUG FIX: Attestation signature verification was using BLAKE3-hashed address as Ed25519 key (always invalid). Fixed by threading raw pubkeys from signed tx envelopes (SEC-ATT-PUBKEY-001 CRITICAL → FIXED)
- Sprint 030 Phase 4: Security review — 60 network tests, 0 ELEVATED, 0 MEDIUM, 1 LOW (WebRTC alpha API), clippy 0 warnings, fmt clean
- Sprint 030 Phase 3: NAT traversal integration — WebRTC config wired into CLI + node config, Dockerfile EXPOSE updated, DCUtR auto-triggers verified, 1 new test
- Sprint 030 Phase 2: WebRTC direct transport — build_libp2p_transport() behind feature flag, TransportConfig enable_webrtc/webrtc_listen_port, 4 new tests
- Sprint 030 Phase 1: DCUtR hole punching — dcutr::Behaviour in swarm, event handling with NatTraversalStats, 3 new tests
- Sprint 029 Phase 4: Security review — 56 network tests, 0 ELEVATED, 0 MEDIUM, 2 LOW, clippy 0 warnings, fmt clean
- Sprint 029 Phase 3: Kademlia bootstrap + message validation — bootstrap on first peer, RoutingUpdated→PeerStore, structural message validation, 3 tests
- Sprint 029 Phase 2: Persistent peer store — PeerStore (redb), address merge/dedup, load_cached_peers on startup, 4 tests
- Sprint 029 Phase 1: Gossipsub scoring retune — per-topic weights, tightened penalties, lower thresholds, 2 tests
- Sprint 028 Phase 4: Security review — 47 network tests, 0 ELEVATED, 0 MEDIUM, 4 LOW (all acceptable), clippy 0 warnings, fmt clean
- Sprint 028 Phase 3: AutoNAT + Relay — autonat probing, NatStatus tracking, relay client with auto-listen on NAT Private, 4 tests
- Sprint 028 Phase 2: Connection filtering — per-IP limits (3), rate limiting (1s), subnet /16 caps (5), IPv4/IPv6, 7 tests
- Sprint 028 Phase 1: Peer reputation — redb-backed PeerReputationStore, tiered bans, score decay, prune/evict, 7 tests
- Sprint 027 Phase 4: Security review — 201 tests pass (148 node + 53 RPC), 0 clippy warnings, fmt clean, 0 new findings
- Sprint 027 Phase 3: Testnet lifecycle — Makefile (setup/start/stop/reset/logs/status), reset-testnet.sh, Dockerfile curl
- Sprint 027 Phase 2: Chain identity RPC — aztb_chainId (0xa27b), aztb_genesisHash (BLAKE3), genesis_hash in RpcState
- Sprint 027 Phase 1: Docker genesis — write_docker_configs(), --docker flag, setup-docker-testnet.sh, /dns4/ boot_nodes
- Sprint 026 Phase 4: Security review — clippy 0 warnings, fmt clean, cargo-deny clean, 53 RPC tests pass, 0 new findings
- Sprint 026 Phase 3: Testnet endpoints — aztb_faucetDrip (10 AZTB, 60s rate limit), aztb_nodeInfo (version/chainId/protocol), GET /health (200 OK), 5 new tests
- Sprint 026 Phase 2: Grafana + Prometheus Docker — prometheus.yml scrape config, grafana provisioning, 2 dashboards (Node Health, Consensus), docker-compose services
- Sprint 026 Phase 1: Prometheus metrics — prometheus-client 0.23, NodeMetrics registry (12 counters/gauges), GET /metrics (Prometheus), GET /metrics/json (backward compat), 4 new tests
- Sprint 025 Phase 4: Fuzz targets + security review — 6 cargo-fuzz targets (core + consensus), cargo-deny clean, clippy 0 warnings, fmt clean
- Sprint 025 Phase 3: Verkle proof verification rewrite — domain-separated BLAKE3 commitments, self-contained proofs, bottom-up verification, 12 new Verkle tests, ADR-013
- Sprint 025 Phase 2: Cryptographic audit — Ed25519/BLAKE3/BLS domain separation verified, 8 known-answer test vectors, BLS PoP confirmed enforced
- Sprint 025 Phase 1: Dependency upgrades — wasmtime v28→v42 (4 CVEs), bincode→postcard (RUSTSEC-2025-0141), cargo-deny clean
- Sprint 024 Phase 4: Security review — 11 findings resolved, 570 tests, M7 milestone 1 complete
- Sprint 023 Phase 4: Security review + M6 close — SEC-COMMIT-OVERWRITE resolved, 0 new ELEVATED/MEDIUM; 555 tests, 0 clippy warnings, fmt clean
- Sprint 023 Phase 3: Compute validation + DeregisterModel — CommitCompute model validation, TxKind::DeregisterModel (0x0D) with owner auth + pending task block, 3 new tests
- Sprint 023 Phase 2: TaskAssigner wiring — PostTask auto-assigns validator via PoUW score, SubmitAttestation enforces assigned_validator, InferenceTask.assigned_validator field, 3 new tests
- Sprint 023 Phase 1: DeregisterCompute — TxKind::DeregisterCompute (0x0C), stake refund, ComputeCommitmentStore::deregister returns Option<ComputeCommitment>, CommitCompute overwrite refund, 5 new tests
- Sprint 022 Phase 4: Security review — SEC-ATT-LEAK-001 LOW FIXED, SEC-COMMIT-OVERWRITE LOW ACCEPTED (resolved in Sprint 023); 544 tests
- Sprint 021 Phase 4: Security review — 0 ELEVATED, 1 MEDIUM fixed (SEC-SYBIL-001: duplicate validator dedup), 3 RPC endpoints, ADR-010; clippy/fmt clean; 500+ tests
- Sprint 021 Phase 3: Task marketplace — TxKind::PostTask (0x09), TaskPool (1024 cap, expiry eviction), TaskAssigner (highest PoUW score), TaskSettlement (reward split), 14 new tests
- Sprint 021 Phase 2: PoUW scoring — SlidingWindowPoUWScore (0.4 acc + 0.3 lat + 0.3 avail), AttestationAggregator (quorum ≥ 2, validator dedup), ValidatorWorkHistory, 7 new tests
- Sprint 021 Phase 1: Model registry — ModelRegistry (register/query/deregister), TxKind::RegisterModel (0x08), ComputeCommitmentStore, pipeline integration, 12 new tests
- Sprint 020 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 3 LOW (SEC-WS-002, SEC-SUB-001, SEC-EXPORT-001); clippy/fmt/test clean; 491 tests
- Sprint 020 Phase 3: Wallet management — wallet list (keyfile dir scan), wallet balance (RPC query), wallet export/import (encrypted JSON roundtrip) (4 new tests)
- Sprint 020 Phase 2: Event subscriptions — aztb_subscribe/aztb_unsubscribe (newHeads/finality topics), broadcast channels, HTTP subscribe returns error (4 new tests)
- Sprint 020 Phase 1: WebSocket gateway — /ws upgrade handler, JSON-RPC dispatch over WS, light sync message handling over WS, max 256 connections (6 new tests incl. 4 subscription tests)
- Sprint 019 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 3 LOW (SEC-WASM-001, SEC-WS-001, SEC-HD-001); clippy/fmt/test clean; 481 tests
- Sprint 019 Phase 3: HD wallet derivation — derive_account(phrase, index) with BLAKE3 domain separation, index 0 backward-compatible, wallet derive CLI (4 new tests)
- Sprint 019 Phase 2: Browser transport bridge — LightClient JS API, WebSocket adapter, IndexedDB header cache, demo HTML page
- Sprint 019 Phase 1: WASM light client core — aztibase-wasm crate, wasm-bindgen exports (verifyHeaderChain, verifyMerkleProof, verifyVerkleProof, verifyLightClientProof), 21 tests
- Sprint 018 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 2 LOW (SEC-P2P-001, SEC-RPC-001); clippy/fmt/test clean; 456 tests
- Sprint 018 Phase 3: Wallet transfer broadcast — sign_transfer_encrypted, broadcast_transaction via reqwest, --passphrase/--rpc CLI flags (3 new tests)
- Sprint 018 Phase 2: Light node sync loop — async P2P sync, peer scoring (failure/latency), header chain verify + LightStore persist (3 new tests)
- Sprint 018 Phase 1: Light sync P2P handler — /aztibase/light-sync/1 request-response protocol, LightSyncCodec (bincode framing, 1MB max), full-node handler (4 new tests)
- Sprint 017 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 3 LOW (SEC-WALLET-001, SEC-LIGHT-001, SEC-SYNC-003); clippy/fmt/test clean; 446 tests
- Sprint 017 Phase 3: Light node header sync — LightSyncMessage (4 variants), LightSyncProtocol state machine, verify_header_chain, --light CLI flag (10 new tests)
- Sprint 017 Phase 2: Light node storage — LightStore (redb, 5 tables), header/cert/proof/wallet CRUD, batch insert, proof eviction (6 new tests)
- Sprint 017 Phase 1: Wallet hardening — BIP-39 mnemonic (24/12-word), BLAKE3 key derivation, Argon2id + ChaCha20-Poly1305 encrypted keyfiles, zeroize (5 new tests)
- Sprint 016 Phase 4: Security review — 0 ELEVATED, 1 MEDIUM (cross-VM reentrancy), 4 LOW, 1 INFO; M4 completion assessment done
- Sprint 016 Phase 3: PoUW type foundations — InferenceTask, InferenceAttestation, PoUWScore trait + StubPoUWScore (8 new tests)
- Sprint 016 Phase 2: Cross-VM bridge — CrossVmCall, wasm_to_evm, evm_to_wasm, depth limit 4 (6 new tests)
- Sprint 016 Phase 1: Transaction anomaly scoring — AnomalyScorer heuristic, anomaly_score on ExecutionReceipt, pipeline wiring (5 new tests)
- Sprint 015 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 3 LOW; Block-STM parallel validation race fixed; clippy/fmt clean
- Sprint 015 Phase 3: WebRTC transport scaffold — libp2p-webrtc feature gate, WebRtcTransport struct, STUN config, 4 new tests
- Sprint 015 Phase 2: Light client proofs — LightClientProof variant, verify_light_client_proof, snapshot height + finality_certificate, Merkle/Verkle/BLS benchmarks (7 new tests)
- Sprint 015 Phase 1: Metrics wiring — MetricsConfig, --metrics CLI flag, GET /metrics HTTP endpoint, consensus+execution JSON counters (2 new tests)
- Sprint 014 Phase 3: Verkle tree foundations — StateCommitment trait, MerkleCommitment, VerkleTree prototype, VerkleCommitment (8 new tests)
- Sprint 014 Phase 2: Performance benchmarking — criterion harness (6 benchmarks), ConsensusMetrics with atomic counters (1 new test)
- Sprint 014 Phase 1: Consensus resilience — equivocation detection, vertex buffering, byzantine + crash recovery integration tests (5 new tests)
- Sprint 013 Phase 3: End-to-end testnet integration — multi-node genesis bootstrap, finality cert integration, local-testnet.sh rewrite, config consolidation (3 new tests)
- Sprint 013 Phase 2: BLS keys in genesis — BLS keypair generation, ValidatorSet carries BLS pubkeys, build/verify_certificate_from_set (5 new tests)
- Sprint 013 Phase 1: Genesis-driven validator bootstrap — validators from genesis config, validator identity from key file, per-node TOML configs (5 new tests)
- Sprint 012 Phase 3: Genesis config + CLI wallet — GenesisConfig TOML, generate_genesis, write_genesis, wallet generate/show/transfer, --genesis flag (7 new tests)
- Sprint 012 Phase 2: Persistent base fee + validation hardening — persistent BaseFeeCalculator, pipeline nonce validation, mempool gas price floor (7 new tests)
- Sprint 012 Phase 1: Pre-execution fee escrow — escrow_fee wired, collect_fees removed, refund_unused, PipelineResult.total_fees_burned (5 new tests)
- Sprint 011 Phase 4: Security review — 0 ELEVATED, 2 MEDIUM documented, 6 LOW
- Sprint 011 Phase 3: Fee market — gas_price on all TxKind variants, collect_fees, BaseFeeCalculator, escrow_fee/refund_unused, aztb_gasPrice + aztb_estimateGas RPC (19 new tests)
- Sprint 011 Phase 2: Nonce enforcement — mempool insert_checked with MAX_NONCE_GAP, nonce-ordered batch execution (5 new tests)
- Sprint 011 Phase 1: Signed tx envelopes — SignedTx type, Ed25519 wire format, verify_and_route pipeline integration (13 new tests)
- Sprint 010 Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 6 LOW documented
- Sprint 010 Phase 3: Account abstraction — AccountType enum, CreateAgent (0x07), aztb_getAccountType RPC, snapshot v2 (11 new tests)
- Sprint 010 Phase 2: Gossipsub hardening — peer scoring, dedup cache, connection limits, message size limits (3 new tests)
- Sprint 010 Phase 1: Rayon-parallel Block-STM — thread-safe MVMemory, Arc scheduler, rayon::scope workers (3 new tests)
- Sprint 009 Phase 4: Security review — 2 ELEVATED fixed (SEC-SYNC-001/002), 8 MEDIUM, 5 LOW, cargo-audit clean (3 new tests)
- Sprint 009 Phase 3: EVM precompiles — ecrecover, SHA-256, RIPEMD-160, identity, modexp via revm built-in pure-Rust impls (12 new tests)
- Sprint 009 Phase 2: State sync protocol — StateSnapshot, SyncMessage (request/response), SnapshotAssembler, bootstrap from peer, main loop wiring (19 new tests)
- Sprint 009 Phase 1: Block-STM parallel execution — MVMemory, Scheduler, BlockSTMExecutor, pipeline integration (16 new tests)
- Sprint 008 Phase 4: Security review — 2 MEDIUM (1 fixed: bounded seen set, 1 documented: model paths), 4 LOW, zero ELEVATED
- Sprint 008 Phase 3: Transaction pool — priority ordering, eviction policy, BLAKE3 hash dedup (10 new tests)
- Sprint 008 Phase 2: AI inference pipeline — TxKind::AiInfer (0x06), pipeline routing to TractRuntime, inference_hash in receipts, model preload config (5 new tests)
- Sprint 008 Phase 1: Multi-node testnet — wire format, vertex routing, state root broadcast, convergence test, launch script (10 new tests)
- Sprint 007 Phase 3: AI inference via tract — TractRuntime, model registry, InferenceReceipt, verify_inference (12 new tests)
- Sprint 007 Phase 2: EVM via revm v36 — evm_deploy, evm_call, CacheDB adapter, dual VM (8 new tests)
- Sprint 007 Phase 1: Receipt store — ExecutionReceipt, store/get, RPC query, pipeline wiring (10 new tests)
- Sprint 006 Phase 4: M3 close — security review (1 MEDIUM fixed: RPC body limit), cargo-audit clean, ADR-005
- Sprint 006 Phase 3: Integration testing (4 e2e tests, 191 total tests)
- Sprint 006 Phase 2: Security hardening (7 MEDIUM findings, 7 new tests, 187 total)
- Sprint 006 Phase 1: JSON-RPC server (6 methods, 15 tests, node wiring, shared state)
- Sprint 005 Phase 4: Security review (zero ELEVATED flags), cargo-audit, docs update
- Sprint 005 Phase 3: BLS finality certificates (blst, aggregation, FinalityCertificate, creation, verification)
- Sprint 005 Phase 2: State persistence to redb, batch root storage, startup recovery
- Sprint 005 Phase 1: Consensus-to-execution wiring, TxKind routing, ExecutionPipeline

### Next Up
1. Sprint 049 planning — All 9 ELEVATED flags resolved, mainnet prep continues

### Deferred to M9+ (Locked in FUTURE_PLANNING.md)

| Feature | FP # | Code Status | Why Deferred |
|---------|------|-------------|-------------|
| Privacy / TEE / Encrypted Inference | FP-006 | 0% | Requires TEE libraries + validator hardware changes |
| Selective Disclosure Identity | FP-007 | 0% | Depends on FP-006 + BBS+ signature maturity |
| Autonomous AI Agent Transactions | FP-008 | 20% (account type only) | Needs staking (Sprint 038) + privacy layer |
| ZK Proof System Integration | FP-009 | 0% | Research-dependent, evaluating RISC Zero / SP1 |
| L2 Community Task Chain | FP-004 | 0% | Needs L1 bridge primitives + sequencer design |
| Mobile Light Client App | FP-003 | 0% | Needs WASM tx signing exports |

### What CAN Be Claimed Today (Code-Backed)
- AI inference as native protocol operation (TxKind::AiInfer, TractRuntime, 13 e2e tests)
- Multi-metric PoUW scoring (accuracy 40% + latency 30% + availability 30%)
- Model royalties via on-chain registry (RegisterModel, 5% royalty on inference)
- Server-independent browser nodes (WebRTC + WASM light client)
- EIP-1559 fee burning (governance-controlled)
- Full tokenomics (emission, vesting, APY curves, staking, delegation, slashing, 759 tests)

### M8 Scope: Monitoring Infrastructure (Planned)
- **Prometheus**: `prometheus-client` crate (pure Rust), `/metrics` endpoint in Prometheus text format
- **Grafana**: Docker Compose service, auto-provisioned dashboards (Node Health, Consensus)
- **Dashboards**: Block height, TPS, finality latency, peer count, mempool depth, round progression, equivocations
- **Alerting**: Missed rounds, peer drops, mempool overflow (Phase 2+)

---

## Open Risks

| Risk | Severity | Owner | Status |
|------|----------|-------|--------|
| Aztibase Systems coexistence agreement not initiated | MEDIUM | legal-ip-counsel | OPEN |
| Domain acquisition pending | MEDIUM | legal-ip-counsel | OPEN |
| nChain patent FTO analysis not started | HIGH | legal-ip-counsel | OPEN |
| wasmtime trap handling on Windows | LOW | smart-contract-engineer | KNOWN |
| Transitive dep advisories (ring, wasmtime ×4, tracing-subscriber, lru, bincode) | LOW | security-engineer | DOCUMENTED |
| Mempool seen set memory growth (SEC-MEM-001) | MEDIUM | node-engineer | FIXED (bounded to max_size×10) |
| AI model paths from config (SEC-AI-001) | MEDIUM | ai-integration-engineer | FIXED (path traversal blocked) |
| Block-STM parallel validation race | MEDIUM | smart-contract-engineer | FIXED (prior-tx check in finish_validation) |
| BLS rogue-key attack without PoP | MEDIUM | consensus-engineer | DOCUMENTED (ADR-004) |
| Cross-VM reentrancy within depth limit (SEC-BRIDGE-003) | MEDIUM | smart-contract-engineer | DOCUMENTED (M7 guard) |
| Attestation Sybil via duplicate validators (SEC-SYBIL-001) | MEDIUM | consensus-engineer | FIXED (validator dedup in aggregate) |

---

## Legal Conditions Tracker

| # | Condition | Status |
|---|-----------|--------|
| 1 | Use AZTB ticker (not DND) | SATISFIED |
| 2 | Coexistence agreement with Aztibase Systems Inc. | PENDING |
| 3 | Trademark filing strategy (Classes 36/42 first, then 9) | PENDING |
| 4 | Domain acquisition before public announcement | PENDING |
| 5 | Matrix Aztibase FAQ entry | PENDING |
