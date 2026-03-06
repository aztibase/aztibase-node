# Changelog -- Dendrite Network

All notable changes to this project are documented here, organized by milestone.
Format follows [Keep a Changelog](https://keepachangelog.com/).

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
