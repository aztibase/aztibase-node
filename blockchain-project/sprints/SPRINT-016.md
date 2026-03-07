# Sprint 016 — Transaction Anomaly Scoring, Cross-VM Bridge, PoUW Foundations

**Sprint Number:** 016
**Start Date:** 2026-03-07
**End Date:** TBD
**Status:** IN PROGRESS
**Milestone:** M4 — Integration Testing + AI + Testnet

---

## Objective

Complete the remaining M4 requirements: wire transaction anomaly scoring through the execution pipeline, build the WASM-EVM cross-VM bridge for contract interop, and lay the PoUW (Proof of Useful Work) type foundations for M6. Close with security review.

---

## Phases & Tasks

### Phase 1: Transaction Anomaly Scoring (Tasks 1-4)

Wire the existing `TractRuntime` into the execution pipeline so every transaction receives an anomaly score in its receipt.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `AnomalyScorer` struct: wraps `TractRuntime`, scores transaction features (sender history, value, gas) | ai-integration-engineer | PENDING |
| 2 | `score_transaction()` in execution pipeline: compute anomaly score (0.0-1.0) for each tx, attach to `ExecutionReceipt` | smart-contract-engineer | PENDING |
| 3 | `anomaly_score` field on `ExecutionReceipt` with `#[serde(default)]` backward compat | smart-contract-engineer | PENDING |
| 4 | Anomaly scoring integration test: submit transfers + AI infer txs, verify receipts carry scores | ai-integration-engineer | PENDING |

**Exit Criteria:**
- Every executed transaction produces an anomaly score in its receipt
- `TractRuntime` used when available, fallback 0.0 in passthrough mode
- 4 new tests passing

---

### Phase 2: Cross-VM Bridge (Tasks 5-8)

Enable WASM contracts to call EVM contracts and vice versa through a bridge precompile.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `CrossVmCall` type: caller VM, target address, calldata, gas_limit, return buffer | smart-contract-engineer | PENDING |
| 6 | WASM host function `cross_vm_call()`: WASM contract calls EVM contract via `evm_call` | smart-contract-engineer | PENDING |
| 7 | EVM precompile at address `0x0100`: EVM contract calls WASM contract via `vm_execute` | smart-contract-engineer | PENDING |
| 8 | Cross-VM integration tests: WASM calls EVM, EVM calls WASM, nested call depth limit (max 4) | smart-contract-engineer | PENDING |

**Exit Criteria:**
- WASM contract can invoke EVM contract and read return data
- EVM contract can invoke WASM contract via precompile
- Nested cross-VM call depth limited to 4
- 4 new tests passing

---

### Phase 3: PoUW Type Foundations (Tasks 9-12)

Define the core types and interfaces for the Proof of Useful Work subsystem. No execution yet — types, traits, and serialization only. This unblocks M6 implementation.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | `InferenceTask` type: task_id, model_id, input_hash, requester, reward, deadline_round | consensus-engineer | PENDING |
| 10 | `InferenceAttestation` type: task_id, result_hash, compute_units, validator_id, signature | consensus-engineer | PENDING |
| 11 | `PoUWScore` trait: `useful_work_score(validator, window)` returning 0.0-1.0, stub impl | consensus-engineer | PENDING |
| 12 | PoUW type serialization tests + integration with `Vertex` metadata field | consensus-engineer | PENDING |

**Exit Criteria:**
- All PoUW types defined with bincode + serde serialization
- `PoUWScore` trait with stub implementation returning 0.0
- Types integrate with existing `Vertex` structure
- 4 new tests passing

---

### Phase 4: Security Review + M4 Assessment (Tasks 13-16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: anomaly scoring (input validation, model trust boundary) | security-engineer | PENDING |
| 14 | Security review: cross-VM bridge (reentrancy, call depth, gas forwarding) | security-engineer | PENDING |
| 15 | Security review: PoUW types (attestation forgery, task replay) | security-engineer | PENDING |
| 16 | M4 completion assessment: gap analysis against master plan Week 4 goals | project-lead | PENDING |

**Exit Criteria:**
- 0 ELEVATED security findings (or all addressed)
- All MEDIUM findings documented with mitigation plan
- cargo clippy zero warnings, cargo fmt clean
- M4 gap analysis documented with clear M5 transition criteria
- BUILD_LOG, STATUS, CHANGELOG updated
- 16 new tests total (4 per phase)

---

## Dependencies

- Phase 1 depends on: `TractRuntime` (Sprint 007), `ExecutionReceipt` (Sprint 007), execution pipeline (Sprint 005)
- Phase 2 depends on: `evm_call`/`evm_deploy` (Sprint 007), `vm_execute` (Sprint 005), precompiles (Sprint 009)
- Phase 3 depends on: `Vertex` type (Sprint 002), consensus engine (Sprint 003)
- Phase 4 depends on: Phases 1-3

## Risks

| Risk | Mitigation |
|------|------------|
| Anomaly model accuracy without real training data | Use synthetic scoring function; real model training is M6+ |
| Cross-VM reentrancy | Strict call depth limit (4), per-call gas metering |
| PoUW gaming via trivial work | Deferred — M6 will implement multi-method verification |
