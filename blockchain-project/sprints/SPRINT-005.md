# Sprint 005: M3 Execution Integration + State Persistence

**Sprint Goal:** Wire the execution pipeline into the live node so committed DAG vertices produce real state transitions end-to-end. Persist AccountState to redb so state survives restarts. Add transaction type routing so the node can dispatch transfers, contract deploys, and contract calls from raw payloads. Implement BLS finality certificates so light clients can verify committed state without replaying the DAG.

**Start Date:** 2026-03-06
**End Date:** TBD
**Status:** IN PROGRESS
**Led By:** project-lead

---

## Prerequisites (All Complete)
- [x] Sprint 004 complete (122 tests, VRF leader election, execution pipeline, WASM contracts)
- [x] CommittedBatch: deterministic topological ordering of committed vertices
- [x] AccountState: in-memory with balance, nonce, code, contract storage
- [x] SimpleTransfer + WASM contract execution working with receipts
- [x] State root: BLAKE3 Merkle tree over sorted accounts
- [x] Security review passed (Sprint 004, no ELEVATED flags)

---

## Sprint Scope

This sprint makes the execution pipeline real: consensus commits flow through the node binary into state transitions persisted in redb. Transaction routing eliminates the manual type distinction. BLS finality certificates provide cryptographic proof of committed state for light clients.

**Not in scope for Sprint 005:**
- EVM integration via revm (M3 later phase)
- Full Verkle tree (BLAKE3 Merkle placeholder continues)
- State sync / snapshot protocol (M3 later phase)
- AI inference pipeline (M4)
- Block-STM parallel execution (M4+)
- Slashing enforcement (M4)

---

## Phase 1: Node-Level Consensus-to-Execution Wiring (node-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 1 | Add `ExecutionPipeline` to node — owns AccountState, receives committed batches | node-engineer | -- | DONE | Struct holds AccountState + processes CommittedBatch; unit test |
| 2 | Wire ConsensusEngine commit events to ExecutionPipeline via channel | node-engineer | Task 1 | DONE | tokio::mpsc channel carries CommittedBatch from consensus to execution; test |
| 3 | Add transaction type enum: `Transfer`, `ContractDeploy`, `ContractCall` | smart-contract-engineer | -- | DONE | `TxKind` enum with bincode serialization; test roundtrip |
| 4 | Implement `TxRouter` — classify raw payload bytes into typed transactions | smart-contract-engineer | Task 3 | DONE | Parse prefix byte to determine type; invalid prefix returns error; test |
| 5 | Integrate TxRouter into ExecutionPipeline — dispatch to correct executor | node-engineer | Tasks 1, 4 | DONE | Pipeline routes transfers to native exec, deploys/calls to WASM exec; test |

**Exit criteria:** Committed vertices flow through the node from consensus → ordering → routing → execution → state update. `cargo test` passes with new tests.

---

## Phase 2: State Persistence to redb (node-engineer + smart-contract-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 6 | Define redb table layout for account state (accounts, contract_code, contract_storage) | node-engineer | -- | DONE | Four redb tables (accounts, contract_code, contract_storage, batch_roots); test open/close |
| 7 | Implement `PersistentAccountState` — redb-backed AccountState with same interface | smart-contract-engineer | Task 6 | DONE | flush_state/load_state functions with same data as in-memory; state root roundtrip verified |
| 8 | Add state flush after each committed batch — write dirty accounts to redb | node-engineer | Tasks 5, 7 | DONE | After execution, changed accounts written to redb; verified by re-read; test |
| 9 | Implement state recovery on node startup — load AccountState from redb | node-engineer | Task 7 | DONE | Node starts, loads last persisted state, resumes from correct balances; test |
| 10 | Add state root to committed block metadata — stored alongside block in redb | node-engineer | Tasks 8, 13 (Phase 1 state root) | DONE | State root persisted per committed batch; queryable by batch hash; test |

**Exit criteria:** Account state persists across node restarts. State roots stored per batch. `cargo test` passes with new tests.

---

## Phase 3: BLS Finality Certificates (consensus-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 11 | Add BLS12-381 key types to dendrite-core (BLS public key, secret key, signature) | consensus-engineer | -- | DONE | BLS key generation, sign, verify; uses blst crate; test |
| 12 | Implement BLS signature aggregation — combine 2f+1 validator signatures | consensus-engineer | Task 11 | DONE | Aggregate signatures; verify against aggregate public key; test |
| 13 | Define `FinalityCertificate` struct — batch hash, state root, aggregated BLS signature, signer bitmap | consensus-engineer | Task 12 | DONE | Serializable certificate; contains proof of 2f+1 agreement; test |
| 14 | Implement certificate creation — validators sign committed batch, leader aggregates | consensus-engineer | Tasks 12, 13 | DONE | After commit, validators sign (batch_hash || state_root); leader aggregates into certificate; test |
| 15 | Implement certificate verification — any node can verify with known validator set | consensus-engineer | Tasks 13, 14 | DONE | Verify aggregated sig against signer bitmap + validator set; reject if <2f+1; test |

**Exit criteria:** BLS finality certificates created after commit and verifiable by any node. `cargo test` passes with new tests.

---

## Phase 4: Security Review + Documentation (security-engineer + documentation-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 16 | Security review of consensus-to-execution wiring | security-engineer | Phase 1 | PENDING | No dropped commits, no double-execution, channel backpressure safe |
| 17 | Security review of state persistence | security-engineer | Phase 2 | PENDING | No partial writes, crash-safe flush, no state corruption on restart |
| 18 | Security review of BLS finality certificates | security-engineer | Phase 3 | PENDING | No rogue-key attack, bitmap manipulation, signature malleability |
| 19 | Security review of transaction routing | security-engineer | Phase 1 | PENDING | No type confusion, no deserialization exploits, fuzz-worthy prefix parsing |
| 20 | Run cargo-audit, fix any new findings | security-engineer | All code | PENDING | Zero critical/high CVEs |
| 21 | Update STATUS.md, CHANGELOG.md, BUILD_LOG.md | documentation-engineer | All phases | PENDING | All docs current |

**Exit criteria:** Security review complete. No ELEVATED flags. All docs updated.

---

## Sprint Totals

| Metric | Target |
|--------|--------|
| Tasks | 21 |
| Tests (new) | 30+ |
| Total tests | 152+ |
| Crates modified | 5 (core, consensus, execution, storage, node) |
| Security reviews | 4 (wiring, persistence, BLS, routing) |
| New dependency | blst (BLS12-381) |

---

## Key Design Decisions

1. **TxKind prefix byte:** First byte of payload determines transaction type (0x01=transfer, 0x02=deploy, 0x03=call). Simple, extensible, no complex framing.
2. **PersistentAccountState wraps redb:** Same trait interface as in-memory. Execution code doesn't know which backend it uses.
3. **Three redb tables for state:** `accounts` (address -> Account), `contract_code` (address -> bytes), `contract_storage` (address||key -> value). Keeps queries efficient.
4. **BLS via blst crate:** Industry-standard, pure C with Rust bindings, used by Lighthouse/Prysm. Only C dependency — justified by crypto correctness requirements.
5. **Finality certificate = batch_hash + state_root + aggregated_sig + bitmap:** Minimal structure for light client verification. Bitmap tracks which validators signed.
6. **Channel-based consensus→execution:** tokio::mpsc unbounded initially, bounded with backpressure in hardening sprint.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| blst C dependency violates pure-Rust policy | Justified exception for BLS — no pure-Rust BLS library has production-grade audit. Document in ADR. |
| redb write amplification on every batch | Batch writes in single transaction; benchmark in M4 |
| Channel overflow if execution is slow | Unbounded channel for now; bounded + backpressure in hardening sprint |
| BLS rogue-key attack | Use proof-of-possession (PoP) for validator key registration |
| State corruption on crash during flush | redb ACID transactions — atomic commit or rollback |

---

## Definition of Done (Sprint 005)

- [ ] All 21 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes with 152+ tests
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated with post-sprint state
- [ ] ADR for blst dependency written
- [ ] Sprint retrospective written
