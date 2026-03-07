# Sprint 016 — Transaction Anomaly Scoring, Cross-VM Bridge, PoUW Foundations

**Sprint Number:** 016
**Start Date:** 2026-03-07
**End Date:** 2026-03-07
**Status:** COMPLETE
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
| 1 | `AnomalyScorer` struct: wraps `TractRuntime`, scores transaction features (sender history, value, gas) | ai-integration-engineer | DONE |
| 2 | `score_transaction()` in execution pipeline: compute anomaly score (0.0-1.0) for each tx, attach to `ExecutionReceipt` | smart-contract-engineer | DONE |
| 3 | `anomaly_score` field on `ExecutionReceipt` with `#[serde(default)]` backward compat | smart-contract-engineer | DONE |
| 4 | Anomaly scoring integration test: submit transfers + AI infer txs, verify receipts carry scores | ai-integration-engineer | DONE |

**Exit Criteria:**
- Every executed transaction produces an anomaly score in its receipt
- `TractRuntime` used when available, fallback 0.0 in passthrough mode
- 4 new tests passing

---

### Phase 2: Cross-VM Bridge (Tasks 5-8)

Enable WASM contracts to call EVM contracts and vice versa through a bridge precompile.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `CrossVmCall` type: caller VM, target address, calldata, gas_limit, return buffer | smart-contract-engineer | DONE |
| 6 | WASM host function `cross_vm_call()`: WASM contract calls EVM contract via `evm_call` | smart-contract-engineer | DONE |
| 7 | EVM precompile at address `0x0100`: EVM contract calls WASM contract via `vm_execute` | smart-contract-engineer | DONE |
| 8 | Cross-VM integration tests: WASM calls EVM, EVM calls WASM, nested call depth limit (max 4) | smart-contract-engineer | DONE |

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
| 9 | `InferenceTask` type: task_id, model_id, input_hash, requester, reward, deadline_round | consensus-engineer | DONE |
| 10 | `InferenceAttestation` type: task_id, result_hash, compute_units, validator_id, signature | consensus-engineer | DONE |
| 11 | `PoUWScore` trait: `useful_work_score(validator, window)` returning 0.0-1.0, stub impl | consensus-engineer | DONE |
| 12 | PoUW type serialization tests + integration with `Vertex` metadata field | consensus-engineer | DONE |

**Exit Criteria:**
- All PoUW types defined with bincode + serde serialization
- `PoUWScore` trait with stub implementation returning 0.0
- Types integrate with existing `Vertex` structure
- 4 new tests passing

---

### Phase 4: Security Review + M4 Assessment (Tasks 13-16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: anomaly scoring (input validation, model trust boundary) | security-engineer | DONE |
| 14 | Security review: cross-VM bridge (reentrancy, call depth, gas forwarding) | security-engineer | DONE |
| 15 | Security review: PoUW types (attestation forgery, task replay) | security-engineer | DONE |
| 16 | M4 completion assessment: gap analysis against master plan Week 4 goals | project-lead | DONE |

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

---

## Security Findings

| ID | Severity | Component | Description | Status |
|----|----------|-----------|-------------|--------|
| SEC-ANOMALY-001 | LOW | Anomaly scorer | Heuristic is advisory only, hash-based noise prevents gaming | DOCUMENTED |
| SEC-BRIDGE-001 | LOW | Cross-VM | Call depth limited to 4, prevents stack overflow | DOCUMENTED |
| SEC-BRIDGE-002 | LOW | Cross-VM | func_name extraction falls back to "main" on invalid UTF-8 | DOCUMENTED |
| SEC-BRIDGE-003 | MEDIUM | Cross-VM | Cross-VM reentrancy possible within depth limit; full reentrancy guard deferred to M7 | DOCUMENTED |
| SEC-POUW-001 | LOW | PoUW | Attestation signature structural validation deferred to M6 | DOCUMENTED |
| SEC-POUW-002 | INFO | PoUW | StubPoUWScore returns 0.0; no influence on consensus until M6 | DOCUMENTED |

**Summary:** 0 ELEVATED, 1 MEDIUM, 4 LOW, 1 INFO

---

## M4 Completion Assessment

### Master Plan Week 4 Goals vs. Actual

| Goal | Status | Notes |
|------|--------|-------|
| Multi-node testnet (4-7 validators) | DONE (Sprint 008) | Wire format, vertex routing, convergence test |
| Transaction throughput benchmarking | DONE (Sprint 014) | Criterion benchmarks for consensus + execution |
| Finality timing verification | DONE (Sprint 005) | BLS finality certificates |
| Basic fault injection | DONE (Sprint 014) | Byzantine + crash recovery tests |
| tract AI integration | DONE (Sprint 007+016) | TractRuntime + anomaly scoring in receipts |
| EVM integration | DONE (Sprint 007) | revm wired, ERC-20 deploys work |
| Cross-VM bridge | DONE (Sprint 016) | WASM↔EVM with depth limiting |
| Genesis config | DONE (Sprint 012) | TOML format, CLI generation |
| CLI wallet | DONE (Sprint 012) | generate/show/transfer |
| Basic node operator guide | PARTIAL | Per-node TOML configs exist, Docker deferred |

### M4 → M5 Transition Criteria
- M4 is substantially complete. Remaining gap: Docker image + deployment docs (lower priority).
- M5 focus: Light/browser nodes (WebRTC already scaffolded), wallet UI, full Verkle trees.

---

## Retrospective

### What went well
- Anomaly scoring integrated cleanly into existing pipeline with backward-compatible receipt field
- Cross-VM bridge leverages existing `evm_call` and `ExecutionEngine` — minimal new code
- PoUW types are clean and self-contained, ready for M6 implementation
- 19 new tests across 3 crates, all passing

### What to improve
- Cross-VM reentrancy needs a proper guard (mutex or call-stack tracking) before production
- Anomaly scorer should evolve to use actual ONNX model when training data is available
- PoUW attestation verification needs cryptographic signature checking (M6)

### Metrics
- Tests: 425 total (was 406)
- New tests: 19 (5 anomaly + 6 cross-VM + 8 PoUW)
- Clippy: 0 warnings
- Fmt: clean
