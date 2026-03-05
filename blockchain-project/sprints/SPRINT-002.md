# Sprint 002: M1 Core Primitives Build-Out

**Sprint Goal:** Bring all 8 crates from STUB/SKELETON to at minimum PARTIAL depth. Complete M1 milestone (core primitives, storage, basic consensus structures, network transport).
**Start Date:** 2026-03-05
**End Date:** TBD
**Status:** IN PROGRESS
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
| 1 | Implement `StateStore` trait (get, put, delete, batch, iterate) | node-engineer | -- | DONE | Trait + redb backend + 7 tests |
| 2 | Create named table abstraction (blocks, state, tx, receipts, validators, verkle) | node-engineer | Task 1 | DONE | 6 tables defined + 2 tests |
| 3 | Implement batch write operations | node-engineer | Task 2 | DONE | Atomic single + multi-table writes + 2 tests |
| 4 | Add iterator support for range queries | node-engineer | Task 2 | DONE | Forward/reverse iteration + 4 tests |

**Exit criteria:** `cargo test -p dendrite-storage` passes with 10+ tests. StateStore fully functional.

### Phase 1b: Consensus Data Types (consensus-engineer, parallel with 1a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 5 | Implement `DagBlock` with parent validation | consensus-engineer | -- | DONE | DAG block with multi-parent, genesis, round check, hash + 8 tests |
| 6 | Implement `ValidatorSet` (add, remove, lookup, stake-weighted selection) | consensus-engineer | -- | DONE | Add/remove/get, supermajority, leader selection + 8 tests |

**Exit criteria:** `cargo test -p dendrite-consensus` passes Tasks 5-6 tests. Pure data structures, no storage dependency.

---

## Phase 2: Consensus Persistence (consensus-engineer, needs Phase 1a + 1b)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 7 | Implement `DagStore` (block insertion, parent lookup, causal ordering) | consensus-engineer | Phase 1a + 1b | DONE | DAG walk, ancestor queries + 7 tests |
| 8 | Implement basic commit rule (direct/indirect commit) | consensus-engineer | Task 7 | DONE | MystiCeti-style commit logic + 3 tests |

**Exit criteria:** `cargo test -p dendrite-consensus` passes with 10+ total tests. DAG blocks can be created, stored, and ordered.

---

## Phase 3: Network Transport + Execution Stubs (PARALLEL)

### Phase 3a: Network Transport (p2p-network-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 9 | Implement `Libp2pTransport` struct implementing `NetworkTransport` trait | p2p-network-engineer | -- | DONE | Swarm creation with QUIC + TCP/Noise |
| 10 | Add Gossipsub with 6 topic subscriptions | p2p-network-engineer | Task 9 | DONE | All 6 Dendrite topics, mesh config |
| 11 | Add Kademlia DHT for peer discovery | p2p-network-engineer | Task 9 | DONE | Bootstrap, peer routing + test |
| 12 | Add mDNS for local discovery | p2p-network-engineer | Task 9 | DONE | Local peer finding + test |
| 13 | Implement peer event handling (connection, disconnection, message) | p2p-network-engineer | Task 10 | DONE | Event loop processes gossip msgs |

**Exit criteria:** `cargo test -p dendrite-network` passes with 8+ tests. Two nodes can discover each other and exchange gossip messages.

### Phase 3b: Execution Stubs (smart-contract-engineer, parallel with 3a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 14 | Implement wasmtime `Engine` + `Store` configuration (deterministic, fuel-metered) | smart-contract-engineer | -- | DONE | Engine config matches spec, fuel works |
| 15 | Implement host function linker (storage_get, storage_set, emit_event) | smart-contract-engineer | Task 14 | DONE | 3 host functions callable from WASM |
| 16 | Implement basic contract execution pipeline (load module -> execute -> return) | smart-contract-engineer | Task 15 | DONE | Execute simple WASM, consume fuel + 3 tests |

**Exit criteria:** `cargo test -p dendrite-execution` passes with 5+ tests. Simple WASM module executes with fuel metering.

---

## Phase 4: Node Wiring (node-engineer, needs Phases 1a + 3a)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 17 | Wire storage subsystem into node startup | node-engineer | Phase 1a | DONE | Node opens redb on startup |
| 18 | Wire network subsystem into node startup | node-engineer | Phase 3a | DONE | Node starts libp2p swarm |
| 19 | Add node configuration (TOML config file + CLI overrides) | node-engineer | -- | DONE | Config struct, file loading, clap integration |
| 20 | Implement graceful shutdown (signal handling, cleanup) | node-engineer | Tasks 17-18 | DONE | SIGTERM/Ctrl-C cleanup + test |

**Exit criteria:** `cargo test -p dendrite-node` passes. `cargo run -p dendrite-node` starts, opens storage, starts networking, shuts down cleanly.

---

## Phase 5: Cross-Cutting (security-engineer + ai-integration-engineer, needs Phases 1-4)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 21 | Security review of all Phase 1-4 code | security-engineer | Phases 1-4 | DONE | 13-item checklist passed, no ELEVATED flags |
| 22 | Implement `AIRuntime` trait + PASSTHROUGH mode | ai-integration-engineer | -- | DONE | Trait defined, passthrough returns no-op |
| 23 | Implement `CryptoProvider` trait (quantum-ready abstraction) | security-engineer | -- | DONE | Trait wrapping Ed25519/BLAKE3, swappable |
| 24 | Run cargo-audit, fix any findings | security-engineer | All code | DONE | Zero critical/high CVEs affecting our code |

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

- [x] All 24 tasks completed or explicitly deferred with justification
- [x] `cargo check --workspace` passes with zero warnings
- [x] `cargo test --workspace` passes with 72 tests total (target was 50+)
- [x] `cargo clippy --workspace` passes with zero warnings
- [x] `cargo fmt --check` passes
- [x] All code has BUILD_LOG entries
- [x] Security review complete (no open ELEVATED flags)
- [x] STATUS.md updated with post-sprint state
- [x] Sprint retrospective written

---

## Sprint Retrospective

### What went well
- Parallel phase execution (1a + 1b, 3a + 3b) saved significant time
- Reference repos (MystiCeti, rust-libp2p) were invaluable for API patterns
- Zero security ELEVATED flags at sprint end after bounds-checking fix
- 72 tests exceeds 50+ target by 44%
- All 8 crates now at PARTIAL or COMPLETE depth

### What could improve
- wasmtime trap handling on Windows remains a known limitation (epoch_interruption disabled)
- cargo-audit found 5 advisories in transitive deps -- upgrading libp2p/wasmtime would resolve most
- bincode v1 is unmaintained -- should migrate to bincode v2 or postcard in a future sprint

### Key decisions made
- ADR-001 (redb) validated by real usage -- performs well for M1 workloads
- CryptoProvider trait designed for PQC migration path (Dilithium/Kyber)
- AIRuntime Passthrough mode enables non-AI nodes to participate in consensus

### Metrics
| Metric | Target | Actual |
|--------|--------|--------|
| Tasks completed | 24 | 24 |
| Tests | 50+ | 72 |
| Crates upgraded | 7 | 7 |
| Security reviews | 1 | 1 |
| ELEVATED flags | 0 | 0 |
| Clippy warnings | 0 | 0 |
