# Changelog -- Dendrite Network

All notable changes to this project are documented here, organized by milestone.
Format follows [Keep a Changelog](https://keepachangelog.com/).

---

## [M4] -- Integration Testing + AI + Testnet (IN PROGRESS)

### Added
- Vertex wire format: version-prefixed encoding with size limits, hash integrity, validator check, round proximity (2026-03-06)
- `encode_vertex()` / `decode_vertex()` in dendrite-consensus/wire.rs (2026-03-06)
- `StateRootAnnounce`: broadcast state roots on TOPIC_STATE_SYNC after batch execution (2026-03-06)
- Execution pipeline result channel: `PipelineResult` sent back to main loop for state root broadcasting (2026-03-06)
- Multi-node consensus convergence test: 3 engines, shared genesis, async vertex routing, state root verification (2026-03-06)
- Local testnet launch script: `scripts/local-testnet.sh` starts 3 nodes on localhost (2026-03-06)
- Transaction receipt store: ExecutionReceipt persisted to redb RECEIPTS_TABLE with atomic batch writes (2026-03-06)
- `dndr_getTransactionReceipt` RPC method: query receipts by tx hash (2026-03-06)
- EVM execution via revm v36: `evm_deploy()` and `evm_call()` with CacheDB state adapter (2026-03-06)
- Dual VM architecture: WASM (wasmtime) + EVM (revm) coexisting in same AccountState (2026-03-06)
- TxKind::EvmDeploy (0x04) and TxKind::EvmCall (0x05) wire format variants (2026-03-06)
- TractRuntime: AIRuntime implementation with real ONNX model loading via tract-onnx (2026-03-06)
- Model registry: register/unregister ONNX models with thread-safe RwLock storage (2026-03-06)
- AI inference pipeline: f32 input → tract tensor → model run → f32 output serialization (2026-03-06)
- InferenceReceipt: deterministic BLAKE3 hash of (model_id || input || output) for verification (2026-03-06)
- `verify_inference()` stub: re-runs inference and compares deterministic hash (2026-03-06)
- `TxKind::AiInfer` (0x06): first-class AI inference transaction type with model_id, input, max_compute_units (2026-03-06)
- AI inference in execution pipeline: AiInfer transactions routed to TractRuntime via `dyn AIRuntime` trait (2026-03-06)
- `ExecutionReceipt.inference_hash`: deterministic hash field for AI inference verification (2026-03-06)
- `inferenceHash` field in `dndr_getTransactionReceipt` RPC response for AI transactions (2026-03-06)
- `NodeConfig.ai` section: model preload from TOML config with model_id + ONNX path (2026-03-06)
- Mempool priority ordering: `BTreeMap<(Reverse<priority>, TxHash)>` drains highest-priority first (2026-03-06)
- Mempool eviction: full pool evicts lowest-priority entry for higher-priority newcomer (2026-03-06)
- Mempool BLAKE3 hash dedup: `seen` set persists after removal to prevent tx replay (2026-03-06)

### Changed
- MSRV bumped from 1.85 to 1.88 for revm v36 compatibility (2026-03-06)
- Pipeline now collects receipts from transfers, WASM contracts, and EVM operations (2026-03-06)

### Security
- SEC-AI-001: 64 MiB model size cap prevents memory exhaustion via oversized ONNX models (2026-03-06)
- SEC-EVM-004: revm with `default-features = false, features = ["std"]` — no C dependencies (2026-03-06)
- EVM nonce validation and chain ID enforcement (0xDE0D) on both deploy and call (2026-03-06)
- AI input size validation rejects mismatched input before model execution (2026-03-06)
- SEC-MEM-001 FIXED: Mempool `seen` set bounded to max_size×10 with FIFO eviction — prevents unbounded memory growth (2026-03-06)
- SEC-AI-001 FIXED: Model path traversal blocked — paths must be under data_dir (canonicalized check) (2026-03-06)
- Sprint 008 security review: 2 MEDIUM (both fixed), 4 LOW — zero ELEVATED flags (2026-03-06)
- cargo-audit: 6 transitive vulns (ring, wasmtime ×3, tracing-subscriber), 7 warnings — all transitive, no action needed (2026-03-06)

### Testing
- 248 tests total (57 new): wire format (9), multi-node convergence (1), receipt store (5), receipt RPC (4), receipt e2e (1), EVM engine (3), EVM routing (2), EVM pipeline (3), tract runtime (13), AI routing (2), AI pipeline (3), mempool (11) (2026-03-06)

---

## [M3] -- Execution Layer (COMPLETE)

### Added
- JSON-RPC 2.0 server: axum-based HTTP server with 6 methods (dndr_getBalance, dndr_getNonce, dndr_getCode, dndr_sendTransaction, dndr_blockNumber, dndr_getStateRoot) (2026-03-06)
- RPC server wired into node binary with shared state via Arc<RwLock<AccountState>> (2026-03-06)
- Transaction submission via RPC flows to mempool and consensus engine (2026-03-06)
- ExecutionPipeline: consensus-to-execution wiring via tokio::mpsc channel (2026-03-06)
- TxKind routing: prefix-byte wire format (0x01=Transfer, 0x02=Deploy, 0x03=Call) (2026-03-06)
- State persistence: flush/load AccountState to redb across 4 tables (2026-03-06)
- Batch root storage: state root per committed batch, queryable by anchor hash (2026-03-06)
- Startup recovery: node loads persisted state from redb on launch (2026-03-06)
- BLS12-381 primitives: BlsKeypair, BlsPublicKey, BlsSignature via blst crate (2026-03-06)
- BLS signature aggregation and verification (2026-03-06)
- FinalityCertificate: batch hash + state root + aggregated BLS sig + signer bitmap (2026-03-06)
- Certificate creation (build_certificate) and verification (verify_certificate) (2026-03-06)

### Changed
- ConsensusOutput::BatchCommitted now carries full CommittedBatch instead of BlockHash (2026-03-06)
- Separate redb databases for consensus DAG and execution state (2026-03-06)

### Security
- ADR-004: blst C dependency justified exception for BLS12-381 (2026-03-06)
- Security review: consensus-execution wiring, state persistence, BLS certificates, tx routing (2026-03-06)
- BLS Proof-of-Possession: prevents rogue-key attacks on aggregate signatures (2026-03-06)
- BLS DST switched from NUL to POP ciphersuite for safe aggregation (2026-03-06)
- Finality message domain separator (DENDRITE_FINALITY_V1) prevents cross-protocol replay (2026-03-06)
- Atomic state flush: single batch_put_multi() transaction for crash safety (2026-03-06)
- Safe deserialization: all unwrap() in persistence replaced with graceful error handling (2026-03-06)
- Duplicate signer deduplication in build_certificate (2026-03-06)
- cargo-audit: transitive advisories (ring, wasmtime WASI, lru, bincode) — no new critical issues (2026-03-06)
- SEC-WIRE-005: HashSet for committed blocks — O(1) dedup in extract_committed_batch (2026-03-06)
- SEC-WIRE-003: Double-execution guard — anchor_hash dedup prevents re-executing batches (2026-03-06)
- SEC-WIRE-004: Fatal flush handling — pipeline halts on persistence failure (2026-03-06)
- SEC-BLS-002: Unique BLS keys enforcement — duplicate keys rejected in certificate operations (2026-03-06)
- SEC-BLS-008: Quorum derived from ValidatorSet — prevents caller-supplied quorum manipulation (2026-03-06)
- SEC-ROUTE-006: func_name validation — rejects injection via malformed function names (2026-03-06)
- SEC-ROUTE-005: Strict bincode decoding — trailing bytes rejected (2026-03-06)

### Testing
- Integration test: transfer end-to-end — balances, nonces, redb persistence, batch root (2026-03-06)
- Integration test: contract deploy + call end-to-end — WASM deploy, storage_set call, state root change (2026-03-06)
- Integration test: finality certificate — BLS sign, aggregate, build + verify certificate (2026-03-06)
- Integration test: startup recovery — flush to redb, reload, verify state, execute more batches (2026-03-06)

### Milestone
- **M3 COMPLETE**: Execution layer feature-complete — WASM VM, state management, persistence, RPC, security hardened, 191 tests (2026-03-06)
- SEC-RPC-001: 1MB request body size limit on RPC server via axum DefaultBodyLimit (2026-03-06)
- ADR-005: axum chosen for JSON-RPC server (tokio-native, minimal, auditable) (2026-03-06)

---

## [M2] -- P2P Networking + Basic Consensus (Complete)

### Added
- ConsensusEngine: async round loop at 400ms with vertex proposal/reception (2026-03-06)
- VRF-based leader election via BLAKE3 PRF with stake-weighted selection (2026-03-06)
- CommittedBatch: deterministic topological ordering of committed vertices (2026-03-06)
- AccountState: in-memory account store with balance, nonce, code, contract storage (2026-03-06)
- SimpleTransfer execution: native token transfers with nonce/balance validation (2026-03-06)
- WASM contract deployment and execution with persisted storage (2026-03-06)
- BLAKE3 Merkle state root computation over sorted account state (2026-03-06)
- Mempool: bounded priority queue with dedup (2026-03-06)
- Round pruning: bounded vertices_by_round with 2-round safety buffer (2026-03-06)

### Security
- Security review: 12-item checklist passed, zero ELEVATED flags (Sprint 004) (2026-03-06)
- VRF seed derived from committed anchor hash -- not manipulable by proposer (2026-03-06)
- Balance arithmetic: checked subtraction + saturating_add -- no overflow/underflow (2026-03-06)
- cargo-audit: transitive advisories documented, no new issues affecting our code (2026-03-06)

---

## [M1] -- Core Primitives (Complete)

### Added
- CryptoProvider trait for quantum-ready crypto abstraction (2026-03-05)
- AIRuntime trait with Passthrough mode for non-AI nodes (2026-03-05)
- Node startup wiring: storage, network, TOML config, graceful shutdown (2026-03-05)
- Build-phase tracking infrastructure: STATUS.md, DECISIONS.md, CHANGELOG.md, BUILD_LOG.md (2026-03-05)

### Changed
- Switched storage backend from RocksDB to redb (ADR-001) (2026-03-05)

### Security
- Fixed WASM host function bounds checking (negative ptr, overflow, out-of-bounds) (2026-03-05)
- Security review: 13-item checklist passed, zero ELEVATED flags (2026-03-05)
- cargo-audit: 5 transitive advisories documented, none exploitable in our code (2026-03-05)

---

## [M0] -- Design Phase (Complete)

### Added
- Full design phase deliverables (2026-03-05):
  - GENESIS_CHAIN_MASTER_PLAN.md -- definitive blueprint
  - MASTER_DESIGN.md -- full technical design (9 sections, all engineers)
  - RESEARCH_BRIEF.md -- competitive analysis and market research
  - LEGAL_LANDSCAPE.md -- IP and regulatory landscape
  - LEGAL_CLEARANCE_REPORT.md -- name/ticker clearance
  - NAMING_REPORT.md -- final naming decision (Dendrite Network / DNDR)
  - ORCHESTRATION.md -- team protocol and dependency graph
  - dendrite-visual-explainer.html -- interactive visual explainer
- 12 specialist skills defined in .claude/skills/
- Workspace scaffolded: 8 Rust crates in Cargo workspace
- Initial crate structure with module stubs for all domains
