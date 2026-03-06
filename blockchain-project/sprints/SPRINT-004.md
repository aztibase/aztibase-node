# Sprint 004: M2 Closure + M3 Execution Layer

**Sprint Goal:** Close remaining M2 debt (VRF leader election, round pruning, pending_txs cap), then build the execution pipeline so committed DAG vertices produce real state transitions. By sprint end, transactions in committed vertices are deterministically ordered, executed through the WASM VM, and produce verifiable state roots.

**Start Date:** 2026-03-06
**End Date:** TBD
**Status:** COMPLETE
**Led By:** project-lead

---

## Prerequisites (All Complete)
- [x] Sprint 003 complete (93 tests, consensus engine running, mempool, commit rule)
- [x] ConsensusEngine: 400ms rounds, vertex proposal/reception, DAG growth
- [x] CommitRule: wave-based direct/indirect commit at anchor rounds
- [x] Mempool: bounded, dedup, peek/drain batch
- [x] ExecutionEngine: wasmtime with fuel metering, host functions (storage_set/get, emit_event)
- [x] Security review passed (Sprint 003)

---

## Sprint Scope

This sprint bridges M2 (consensus) and M3 (execution). Phase 1 closes M2 technical debt. Phases 2-3 build the execution pipeline: ordering committed vertices, extracting transactions, executing them, and computing state roots.

**Not in scope for Sprint 004:**
- BLS finality certificates (M3 later phase)
- State sync / snapshot (M3 later phase)
- EVM integration via revm (M3 later phase)
- Full Verkle tree (placeholder state root sufficient)
- AI inference pipeline (M4)
- Slashing enforcement (M4)

---

## Phase 1: Close M2 Debt (consensus-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 1 | Add round pruning to `RoundState` — remove entries older than last committed wave | consensus-engineer | -- | DONE | `vertices_by_round` bounded; rounds before last commit pruned; test |
| 2 | Cap `pending_txs` in `ConsensusEngine` (reject when full) | consensus-engineer | -- | DONE | Max 4096 pending txs; new txs dropped when full; test |
| 3 | Implement VRF-based leader election using BLAKE3 as PRF | consensus-engineer | -- | DONE | `vrf_leader_for_round(round, seed)` replaces deterministic selection; seed = hash(round \|\| prev_anchor_hash); test |
| 4 | Wire VRF leader election into `CommitRule` | consensus-engineer | Task 3 | DONE | Anchor rounds use VRF leader; existing commit tests updated |

**Exit criteria:** `vertices_by_round` bounded, `pending_txs` capped, VRF leader election functional. `cargo test -p dendrite-consensus` passes with new tests.

---

## Phase 2: Transaction Ordering + Execution Pipeline (consensus-engineer + smart-contract-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 5 | Implement `CommittedBatch` — deterministic topological ordering of committed vertices | consensus-engineer | Phase 1 | DONE | Committed vertices sorted deterministically; transactions extracted in order; test |
| 6 | Add `TransactionExecutor` trait — abstraction for executing ordered transaction batches | smart-contract-engineer | -- | DONE | Trait with `execute_batch(txs) -> BatchResult`; `BatchResult` contains receipts + state diff |
| 7 | Implement `SimpleTransfer` execution — native token transfer without VM | smart-contract-engineer | Task 6 | DONE | Transfer tx deducts sender, credits receiver; insufficient balance fails cleanly; test |
| 8 | Add `AccountState` — in-memory account store (balance, nonce, code hash) | smart-contract-engineer | -- | DONE | Get/set balance, increment nonce, store code; test |
| 9 | Wire `ConsensusOutput::BlockCommitted` to execution pipeline in node | node-engineer | Tasks 5-7 | DONE | Committed blocks flow from consensus -> ordering -> execution; receipts logged |

**Exit criteria:** Committed vertices produce an ordered transaction list. Simple transfers execute and modify account balances. `cargo test` passes with new tests.

---

## Phase 3: WASM Contract Execution + State Roots (smart-contract-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 10 | Add contract deployment transaction type — store WASM bytecode in `AccountState` | smart-contract-engineer | Task 8 | DONE | Deploy tx stores code; code retrievable by address; test |
| 11 | Add contract call transaction type — load code, execute via `ExecutionEngine` | smart-contract-engineer | Tasks 8, 10 | DONE | Call tx loads WASM, runs function, persists storage changes; test |
| 12 | Wire `ExecutionEngine` storage to `AccountState` (host functions read/write real state) | smart-contract-engineer | Tasks 8, 11 | DONE | `storage_set`/`storage_get` persist across calls; test |
| 13 | Implement state root computation — BLAKE3 Merkle hash of sorted account state | smart-contract-engineer | Task 8 | DONE | Deterministic root from account state; same state = same root; test |
| 14 | Add execution receipts — gas used, status, events, state root after batch | smart-contract-engineer | Tasks 7, 11, 13 | DONE | Each tx produces a receipt; batch produces aggregate root; test |

**Exit criteria:** WASM contracts deploy and execute with persisted state. State roots computed deterministically. `cargo test` passes with new tests.

---

## Phase 4: Security Review + Documentation (security-engineer + documentation-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 15 | Security review of VRF leader election | security-engineer | Phase 1 | DONE | VRF seed not manipulable; leader distribution fair; no grinding attack |
| 16 | Security review of execution pipeline | security-engineer | Phases 2-3 | DONE | No double-spend, no nonce skip, no overflow in balance arithmetic |
| 17 | Security review of WASM contract execution | security-engineer | Phase 3 | DONE | Fuel limits enforced; host function bounds checked; no reentrancy via host calls |
| 18 | Run cargo-audit, fix any new findings | security-engineer | All code | DONE | Zero critical/high CVEs |
| 19 | Update STATUS.md, CHANGELOG.md, BUILD_LOG.md | documentation-engineer | All phases | DONE | All docs current |

**Exit criteria:** Security review complete. No ELEVATED flags. All docs updated.

---

## Sprint Totals

| Metric | Target |
|--------|--------|
| Tasks | 19 |
| Tests (new) | 25+ |
| Total tests | 118+ |
| Crates modified | 4 (consensus, execution, node, core) |
| Security reviews | 3 (VRF, execution, WASM) |

---

## Key Design Decisions

1. **VRF via BLAKE3 PRF:** True VRF (e.g., ECVRF) deferred — BLAKE3(round || seed) provides unpredictable-but-deterministic leader selection sufficient for M2/M3. Seed = hash of previous anchor hash, preventing pre-computation beyond one wave.
2. **AccountState in-memory first:** Persistence to redb deferred to next sprint. In-memory HashMap sufficient for correctness testing. State is rebuilt from committed vertices on restart.
3. **SimpleTransfer as native execution:** Token transfers don't go through WASM. Native execution is faster and simpler. WASM is for smart contracts.
4. **BLAKE3 Merkle state root:** Placeholder for Verkle tree. Sorted key-value pairs hashed into binary Merkle tree using BLAKE3. Sufficient for determinism verification; swappable later.
5. **No parallel execution yet:** Block-STM pattern deferred. Sequential execution is correct and simpler. Parallelism is an optimization for M4+.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| VRF seed manipulation by anchor proposer | Seed includes previous anchor hash — proposer doesn't know next seed until commit |
| Balance overflow/underflow | Use checked arithmetic (checked_add, checked_sub) everywhere |
| WASM contracts consuming unbounded memory | wasmtime memory limits already configured (16MB); fuel metering enforced |
| State root divergence between nodes | Deterministic ordering + sequential execution + sorted Merkle = same root |
| Account nonce gaps from failed txs | Failed txs still increment nonce (Ethereum-style) to prevent replay |

---

## Definition of Done (Sprint 004)

- [ ] All 19 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes with 118+ tests
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated with post-sprint state
- [ ] Sprint retrospective written
