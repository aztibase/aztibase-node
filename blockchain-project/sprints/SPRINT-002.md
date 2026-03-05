# Sprint 002: M1 Core Primitives Build-Out

**Sprint Goal:** Bring all 8 crates from STUB/SKELETON to at minimum PARTIAL depth. Complete M1 milestone (core primitives, storage, basic consensus structures, network transport).
**Start Date:** 2026-03-05
**End Date:** TBD
**Status:** PLANNED
**Led By:** project-lead

---

## Prerequisites (All Complete)
- [x] Sprint 001 complete (tracking infrastructure)
- [x] All 12 skills upgraded with deep domain knowledge
- [x] Build-phase rules codified in CLAUDE.md
- [x] GitHub Actions CI pipeline configured
- [x] Reference repos available for study

---

## Sprint Scope

This sprint follows the build dependency order with parallelism where possible. Each phase gate must pass `cargo check` before dependent work begins. All code requires tests and security review.

**Key insight:** The CLAUDE.md design dependency order (consensus → node → p2p) assumes design phase. For implementation, storage must exist before `DagStore` can persist blocks. Tasks 5-6 (pure data structures) have no storage dependency and run parallel with storage build-out.

---

## Phase 1: Storage + Consensus Data Types (PARALLEL)

### Phase 1a: Storage Foundation (node-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 1 | Implement `StateStore` trait (get, put, delete, batch, iterate) | node-engineer | -- | PENDING | Trait + redb backend + 5 tests |
| 2 | Create named table abstraction (blocks, state, tx, receipts, validators, verkle) | node-engineer | Task 1 | PENDING | 6 tables defined, open/close works |
| 3 | Implement batch write operations | node-engineer | Task 2 | PENDING | Atomic multi-table writes + test |
| 4 | Add iterator support for range queries | node-engineer | Task 2 | PENDING | Forward/reverse iteration + test |

**Exit criteria:** `cargo test -p dendrite-storage` passes with 10+ tests. StateStore fully functional.

### Phase 1b: Consensus Data Types (consensus-engineer, parallel with 1a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 5 | Implement `DagBlock` with parent validation | consensus-engineer | -- | PENDING | DAG block with multi-parent support, round monotonicity check, 3 tests |
| 6 | Implement `ValidatorSet` (add, remove, lookup, stake-weighted selection) | consensus-engineer | -- | PENDING | Validator management with stake tracking + 3 tests |

**Exit criteria:** `cargo test -p dendrite-consensus` passes Tasks 5-6 tests. Pure data structures, no storage dependency.

---

## Phase 2: Consensus Persistence (consensus-engineer, needs Phase 1a + 1b)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 7 | Implement `DagStore` (block insertion, parent lookup, causal ordering) | consensus-engineer | Phase 1a + 1b | PENDING | DAG walk, ancestor queries + 4 tests |
| 8 | Implement basic commit rule (direct/indirect commit) | consensus-engineer | Task 7 | PENDING | MystiCeti-style commit logic + 3 tests |

**Exit criteria:** `cargo test -p dendrite-consensus` passes with 10+ total tests. DAG blocks can be created, stored, and ordered.

---

## Phase 3: Network Transport + Execution Stubs (PARALLEL)

### Phase 3a: Network Transport (p2p-network-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 9 | Implement `Libp2pTransport` struct implementing `NetworkTransport` trait | p2p-network-engineer | -- | PENDING | Swarm creation with QUIC + TCP/Noise |
| 10 | Add Gossipsub with 6 topic subscriptions | p2p-network-engineer | Task 9 | PENDING | All 6 Dendrite topics, mesh config |
| 11 | Add Kademlia DHT for peer discovery | p2p-network-engineer | Task 9 | PENDING | Bootstrap, peer routing + test |
| 12 | Add mDNS for local discovery | p2p-network-engineer | Task 9 | PENDING | Local peer finding + test |
| 13 | Implement peer event handling (connection, disconnection, message) | p2p-network-engineer | Task 10 | PENDING | Event loop processes gossip msgs |

**Exit criteria:** `cargo test -p dendrite-network` passes with 8+ tests. Two nodes can discover each other and exchange gossip messages.

### Phase 3b: Execution Stubs (smart-contract-engineer, parallel with 3a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 14 | Implement wasmtime `Engine` + `Store` configuration (deterministic, fuel-metered) | smart-contract-engineer | -- | PENDING | Engine config matches spec, fuel works |
| 15 | Implement host function linker (storage_get, storage_set, emit_event) | smart-contract-engineer | Task 14 | PENDING | 3 host functions callable from WASM |
| 16 | Implement basic contract execution pipeline (load module -> execute -> return) | smart-contract-engineer | Task 15 | PENDING | Execute simple WASM, consume fuel + 3 tests |

**Exit criteria:** `cargo test -p dendrite-execution` passes with 5+ tests. Simple WASM module executes with fuel metering.

---

## Phase 4: Node Wiring (node-engineer, needs Phases 1a + 3a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 17 | Wire storage subsystem into node startup | node-engineer | Phase 1a | PENDING | Node opens redb on startup |
| 18 | Wire network subsystem into node startup | node-engineer | Phase 3a | PENDING | Node starts libp2p swarm |
| 19 | Add node configuration (TOML config file + CLI overrides) | node-engineer | -- | PENDING | Config struct, file loading, clap integration |
| 20 | Implement graceful shutdown (signal handling, cleanup) | node-engineer | Tasks 17-18 | PENDING | SIGTERM/Ctrl-C cleanup + test |

**Exit criteria:** `cargo test -p dendrite-node` passes. `cargo run -p dendrite-node` starts, opens storage, starts networking, shuts down cleanly.

---

## Phase 5: Cross-Cutting (security-engineer + ai-integration-engineer, needs Phases 1-4)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 21 | Security review of all Phase 1-4 code | security-engineer | Phases 1-4 | PENDING | 13-item checklist passed, no ELEVATED flags |
| 22 | Implement `AIRuntime` trait + PASSTHROUGH mode | ai-integration-engineer | -- | PENDING | Trait defined, passthrough returns no-op |
| 23 | Implement `CryptoProvider` trait (quantum-ready abstraction) | security-engineer | -- | PENDING | Trait wrapping Ed25519/BLAKE3, swappable |
| 24 | Run cargo-audit, fix any findings | security-engineer | All code | PENDING | Zero critical/high CVEs |

**Exit criteria:** Security review complete. AI runtime trait defined. Crypto abstracted behind trait for future PQC migration.

---

## Sprint Totals

| Metric | Target |
|--------|--------|
| Tasks | 24 |
| Tests (new) | 50+ |
| Crates upgraded | 7 (all except dendrite-core which is already COMPLETE) |
| Minimum crate depth | PARTIAL for all crates |
| Security reviews | 1 full sprint review |
| ADRs expected | 2-3 (for any non-obvious decisions during implementation) |

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| libp2p v0.54 API changes | Reference examples in `references/rust-libp2p/examples/` |
| wasmtime determinism issues | Disable SIMD + threads, pin version |
| redb performance for block storage | Benchmark early, fallback plan to sled if needed |
| Scope creep | Strict phase gates -- each phase must pass before next starts |

---

## Definition of Done (Sprint 002)

- [ ] All 24 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes with 50+ tests total
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated with post-sprint state
- [ ] Sprint retrospective written
