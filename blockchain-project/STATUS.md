# PROJECT STATUS: Aztibase Network

**Last Updated:** 2026-03-07
**Updated By:** project-lead
**Current Phase:** M5 -- Light/Browser Nodes + Wallet (IN PROGRESS)
**Current Sprint:** Sprint 018 -- COMPLETE (Light Node P2P Integration & Wallet Transfers)

---

## Milestone Tracker

| Milestone | Description | Status | Date Started | Date Completed |
|-----------|-------------|--------|--------------|----------------|
| M0 | Design complete (all MASTER_DESIGN sections) | DONE | 2026-03-05 | 2026-03-05 |
| M1 | Core primitives (data types, crypto, basic structs) | DONE | 2026-03-05 | 2026-03-05 |
| M2 | P2P networking + basic consensus | DONE | 2026-03-05 | 2026-03-06 |
| M3 | Execution layer (WASM VM, state management) | DONE | 2026-03-06 | 2026-03-06 |
| M4 | Integration testing + basic AI + testnet | DONE | 2026-03-06 | 2026-03-07 |
| M5 | Light/browser nodes + wallet | IN PROGRESS | 2026-03-07 | -- |
| M6 | AI compute market + PoUW | NOT STARTED | -- | -- |
| M7 | Security audit + hardening | NOT STARTED | -- | -- |
| M8 | Public testnet | NOT STARTED | -- | -- |
| M9 | Mainnet launch | NOT STARTED | -- | -- |

---

## Current State

### Completed
- Full design phase (Phases 3-6) delivered
- All 12 skills defined and operational
- Workspace scaffolded: 8 Rust crates in monorepo
- Build-phase tracking infrastructure (5 documents)
- Implementation depth audit (Sprint 001, Task 7)
- Reference repos cloned (MystiCeti, Sui, Lighthouse, rust-libp2p, redb)
- cargo build + cargo test pass (456 tests, 0 failures)
- Sprint 001 closed with retrospective
- Sprint 002 closed: 24/24 tasks, 5 phases complete
- Sprint 003 closed: 18/18 tasks, 4 phases complete
- Sprint 004 closed: 19/19 tasks, 4 phases complete
- Sprint 005 closed: 21/21 tasks, 4 phases complete
- Sprint 006 closed: 23/24 tasks (1 deferred), 4 phases complete
- Sprint 007 complete: receipt store, EVM via revm, AI inference via tract
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

### In Progress
- Sprint 019 planning

### Blocked
- Nothing currently blocked

### Recently Completed
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
1. Sprint 019: TBD (candidate areas: browser WASM light node, full Verkle IPA/KZG, Docker deployment, HD wallet derivation)

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

---

## Legal Conditions Tracker

| # | Condition | Status |
|---|-----------|--------|
| 1 | Use AZTB ticker (not DND) | SATISFIED |
| 2 | Coexistence agreement with Aztibase Systems Inc. | PENDING |
| 3 | Trademark filing strategy (Classes 36/42 first, then 9) | PENDING |
| 4 | Domain acquisition before public announcement | PENDING |
| 5 | Matrix Aztibase FAQ entry | PENDING |
