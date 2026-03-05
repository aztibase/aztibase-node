# Build Log -- Dendrite Network

Running diary of build activity. Each entry records who built what, when, and any review notes.
Entries are prepended (newest first).

---

## Log Format

```
### [DATE] -- [ENGINEER] -- [CRATE/AREA]
**Task:** [What was done]
**Sprint:** [Sprint reference]
**Git Ref:** [Commit hash]
**Files Changed:** [List]
**Review Notes:** [Any observations, issues, or follow-ups]
**Security Flags:** [None / Description]
```

---

## Entries

### 2026-03-05 -- security-engineer + ai-integration-engineer -- cross-cutting
**Task:** Phase 5 complete: Security review, AIRuntime, CryptoProvider, cargo-audit
**Sprint:** Sprint 002, Phase 5 (Tasks 21-24)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-core/src/crypto.rs (CryptoProvider trait + DefaultCryptoProvider impl)
- crates/dendrite-core/src/lib.rs (3 new tests for CryptoProvider)
- crates/dendrite-runtime/src/ai_oracle.rs (AIRuntime trait, InferenceRequest/Result, PassthroughRuntime)
- crates/dendrite-runtime/src/lib.rs (exports, 3 new tests for AIRuntime)
- crates/dendrite-execution/src/vm.rs (bounds-checked memory access in host functions)
**Review Notes:**
- Security review: 13-item checklist passed. Found and fixed WASM host function bounds checking (vm.rs). No ELEVATED flags remaining.
- AIRuntime trait with Passthrough/LocalInference/NetworkInference modes. PassthroughRuntime returns empty results, enabling nodes to run without AI hardware.
- CryptoProvider trait abstracts BLAKE3+Ed25519 behind swappable interface for future PQC migration.
- cargo-audit: 5 advisories found, all in transitive deps (wasmtime WASI, ring AES, libp2p lru). None affect our code paths (no WASI, no AES, threads disabled). Documented as acceptable for M1.
- 72 tests total, zero clippy warnings, fmt clean.
**Security Flags:** WASM bounds checking fixed (was missing negative ptr/overflow validation). No remaining ELEVATED flags.

---

### 2026-03-05 -- node-engineer -- dendrite-node
**Task:** Phase 4 complete: Node wiring (storage, network, config, shutdown)
**Sprint:** Sprint 002, Phase 4 (Tasks 17-20)
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (added toml, directories workspace deps)
- crates/dendrite-node/Cargo.toml (added serde, toml, directories deps)
- crates/dendrite-node/src/config.rs (NodeConfig: TOML loading, CLI overrides, defaults, storage path)
- crates/dendrite-node/src/main.rs (full node startup: storage init, network swarm, event loop, graceful shutdown)
- crates/dendrite-network/src/lib.rs (re-exported Multiaddr)
**Review Notes:** Node opens redb storage on startup, creates libp2p swarm with TCP+QUIC, listens on configured addresses, dials boot nodes, runs event loop processing gossip/mDNS/connection events. Graceful shutdown via Ctrl+C with tokio::signal + Notify. Config supports TOML file loading with CLI overrides (--data-dir, --listen, --rpc-addr, --log-level). 6 new tests (config defaults, CLI overrides, TOML roundtrip, storage path, storage open). 66 tests total, zero clippy warnings, fmt clean.
**Security Flags:** None

---

### 2026-03-05 -- smart-contract-engineer -- dendrite-execution
**Task:** Phase 3b complete: WASM execution engine with wasmtime
**Sprint:** Sprint 002, Phase 3b (Tasks 14-16)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-execution/Cargo.toml (added wasmtime dependency)
- crates/dendrite-execution/src/vm.rs (ExecutionEngine: deterministic config, fuel metering, host functions, execution pipeline)
- crates/dendrite-execution/src/lib.rs (exports, 5 tests)
**Review Notes:** wasmtime 28 with deterministic config: SIMD disabled, relaxed SIMD disabled, threads disabled, fuel metering enabled. Three host functions: storage_set, storage_get, emit_event — all with WASM linear memory access. Execution pipeline: compile module → create store with fuel → link host functions → instantiate → call → collect results. Fuel exhaustion trap aborts on Windows (known wasmtime limitation) — tested via fuel consumption tracking instead. 5 tests passing.
**Security Flags:** wasmtime trap handling on Windows causes process abort instead of unwinding — epoch_interruption disabled for now. To be revisited when adding time-based execution limits.

---

### 2026-03-05 -- p2p-network-engineer -- dendrite-network
**Task:** Phase 3a complete: libp2p transport with Gossipsub, Kademlia, mDNS
**Sprint:** Sprint 002, Phase 3a (Tasks 9-13)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-network/Cargo.toml (added futures dependency)
- crates/dendrite-network/src/behaviour.rs (DendriteBehaviour: combined NetworkBehaviour with gossipsub + kademlia + mDNS)
- crates/dendrite-network/src/gossip.rs (6 Dendrite topics, gossipsub config with strict validation, content-based dedup)
- crates/dendrite-network/src/discovery.rs (Kademlia DHT with /dendrite/kad/1.0.0 protocol, replication factor 20, disjoint query paths; mDNS for local discovery)
- crates/dendrite-network/src/transport.rs (Libp2pTransport: SwarmBuilder with TCP/Noise + QUIC, topic subscription, event handling loop with mDNS auto-peering)
- crates/dendrite-network/src/lib.rs (exports, 8 tests)
**Review Notes:** libp2p 0.54 with SwarmBuilder API. Transport creates swarm with TCP/Noise/Yamux + QUIC, subscribes to all 6 topics on construction. Event loop handles gossipsub messages, mDNS discovery/expiry (auto-adds to gossipsub + kademlia), connection lifecycle. Kademlia in Server mode with 60s query timeout. 8 tests: config creation, topic validation, swarm creation, peer ID, topic subscription count, TCP listening.
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer -- dendrite-consensus
**Task:** Phase 2 complete: DagStore + CommitRule implementations
**Sprint:** Sprint 002, Phase 2 (Tasks 7-8)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-consensus/src/dag_store.rs (DagStore: insert, get, parent/child lookup, ancestor check, causal ordering, persistence via redb)
- crates/dendrite-consensus/src/commit.rs (CommitRule: direct commit via supermajority voting, indirect commit via anchor, wave-based leader election)
- crates/dendrite-consensus/src/lib.rs (exports, 10 new tests -- 7 DagStore + 3 CommitRule)
- crates/dendrite-consensus/Cargo.toml (added bincode dependency)
**Review Notes:** DagStore: in-memory index + redb persistence, parent validation on insert, BFS ancestor traversal with round-based pruning, deterministic topological sort for causal ordering, rebuild_index from disk on startup. CommitRule: MystiCeti-inspired wave structure (configurable wave_length), direct commit checks >2/3 voting stake, indirect commit via causal ancestry from anchor block. 26 consensus tests total (16 existing + 10 new). Zero clippy warnings, fmt clean.
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer -- dendrite-consensus
**Task:** Phase 1b complete: DagBlock + ValidatorSet implementations
**Sprint:** Sprint 002, Phase 1b (Tasks 5-6)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-consensus/src/dag.rs (DagBlock with hash, genesis, parent validation)
- crates/dendrite-consensus/src/validator.rs (ValidatorSet with stake, supermajority, leader selection)
- crates/dendrite-consensus/src/lib.rs (exports, 16 tests)
**Review Notes:** DagBlock: deterministic hash (BLAKE3 of round+author+parents+payload+timestamp), genesis block factory, parent round monotonicity validation. ValidatorSet: add/remove/get/contains, total_stake tracking, BFT supermajority check (>2/3), deterministic stake-weighted leader selection. 16 tests passing. Zero clippy warnings.
**Security Flags:** None

---

### 2026-03-05 -- node-engineer -- dendrite-storage
**Task:** Phase 1a complete: StateStore trait, 6 named tables, batch writes, range iteration
**Sprint:** Sprint 002, Phase 1a (Tasks 1-4)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-storage/src/store.rs (complete rewrite — StateStore with redb backend)
- crates/dendrite-storage/src/lib.rs (exports, 15 tests)
**Review Notes:** Full CRUD (get/put/delete/contains), batch_put (single table), batch_put_multi (cross-table atomic), iter/range/range_reverse. All 6 tables (blocks, state, tx, receipts, validators, verkle) initialized on open. StorageError with boxed TransactionError to satisfy clippy. Zero clippy warnings, fmt clean. 15 tests passing (exit criteria was 10+).
**Security Flags:** None

---

### 2026-03-05 -- project-lead -- Project Infrastructure
**Task:** Established build-phase documentation trail
**Sprint:** Pre-Sprint
**Git Ref:** pending
**Files Changed:**
- blockchain-project/STATUS.md (new)
- blockchain-project/DECISIONS.md (new)
- blockchain-project/BUILD_LOG.md (new)
- blockchain-project/sprints/SPRINT-001.md (new)
- CHANGELOG.md (new)
- blockchain-project/ORCHESTRATION.md (updated -- build-phase protocol)
**Review Notes:** Design phase had excellent tracking via ORCHESTRATION.md. Build phase needed its own tracking infrastructure. All five tracking documents now in place.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Storage
**Task:** Switched storage backend from RocksDB to redb
**Sprint:** Pre-Sprint
**Git Ref:** dd98662
**Files Changed:** crates/dendrite-storage/ (Cargo.toml, src/)
**Review Notes:** Pure Rust dependency, better cross-compilation story. See ADR-001.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Project Scaffold
**Task:** Initial project structure, all crate scaffolding, skill definitions
**Sprint:** Pre-Sprint
**Git Ref:** 86f194b
**Files Changed:** Full workspace creation -- Cargo.toml, 8 crates, .claude/skills/, blockchain-project/ docs
**Review Notes:** Foundation commit. All design docs, skill fleet, and crate stubs.
**Security Flags:** None
