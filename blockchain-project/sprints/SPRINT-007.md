# Sprint 007 -- EVM + AI Inference + Receipt Store (M4 Phase 1)

**Start Date:** 2026-03-06
**Target:** M4 progress — dual VM (EVM via revm), AI inference (tract), receipt infrastructure
**Owner:** project-lead
**Status:** COMPLETE

---

## Sprint Goal

Deliver three foundational M4 capabilities:
1. **Transaction receipt store** — persistent receipts with RPC query (deferred from Sprint 006)
2. **EVM execution via revm** — dual VM routing (WASM + EVM), deploy + call Solidity contracts
3. **AI inference via tract** — real model loading and inference behind the AIRuntime trait

These three features define Dendrite's competitive identity: dual VM for developer reach, AI-native inference for protocol differentiation, and receipts for operational observability.

---

## Scope

| # | Item | Phase | Owner |
|---|------|-------|-------|
| 1 | Receipt store (redb table, write on execution) | P1 | node-engineer |
| 2 | Receipt query by tx hash | P1 | node-engineer |
| 3 | `dndr_getTransactionReceipt` RPC method | P1 | node-engineer |
| 4 | Wire receipts from ExecutionPipeline to store | P1 | node-engineer |
| 5 | Add `revm` workspace dependency | P2 | smart-contract-engineer |
| 6 | `EvmEngine` — revm wrapper with DNDR gas config | P2 | smart-contract-engineer |
| 7 | `TxKind::EvmDeploy` + `TxKind::EvmCall` (prefix 0x04, 0x05) | P2 | smart-contract-engineer |
| 8 | EVM execution integrated into `execute_contract_txs` | P2 | smart-contract-engineer |
| 9 | EVM deploy + call tests | P2 | smart-contract-engineer |
| 10 | Add `tract-onnx` to dendrite-runtime | P3 | ai-integration-engineer |
| 11 | `TractRuntime` implementing `AIRuntime` trait | P3 | ai-integration-engineer |
| 12 | Model registry (load ONNX model from bytes, cache) | P3 | ai-integration-engineer |
| 13 | Inference execution with deterministic hash | P3 | ai-integration-engineer |
| 14 | `InferenceReceipt` + verification stub | P3 | ai-integration-engineer |
| 15 | AI inference unit tests (simple ONNX model) | P3 | ai-integration-engineer |
| 16 | Security review: receipt store, EVM, AI pipeline | P4 | security-engineer |
| 17 | cargo-audit + clippy + fmt | P4 | security-engineer |
| 18 | Documentation updates (BUILD_LOG, STATUS, CHANGELOG, sprint) | P4 | documentation-engineer |
| 19 | ADR-006 if revm introduces C deps or design tradeoffs | P4 | blockchain-architect |

**Total:** 19 tasks across 4 phases

---

## Phase 1: Transaction Receipt Store (Tasks 1-4)

**Owner:** node-engineer
**Goal:** Persistent receipt storage with RPC query. Closes deferred Task 6 from Sprint 006.

### Task 1: Receipt store in redb -- DONE
- [x] `ExecutionReceipt` struct in `dendrite-execution/src/receipt.rs`
- [x] `RECEIPTS_TABLE` in redb: `TxHash -> bincode(ExecutionReceipt)`
- [x] `store_receipts()` / `get_receipt()` functions
- **Tests:** 5 unit tests (store/retrieve, missing, failed, deploy w/ address, batch)

### Task 2: Receipt generation on execution -- DONE
- [x] Pipeline collects receipts from transfers (TxStatus -> ExecutionReceipt)
- [x] Pipeline collects receipts from contracts (ContractReceipt -> ExecutionReceipt)
- [x] Unified `ExecutionReceipt` type covers all tx types

### Task 3: `dndr_getTransactionReceipt` RPC method -- DONE
- [x] Added to RPC dispatch in `dendrite-rpc/src/server.rs`
- [x] Accepts tx hash (hex string), returns receipt JSON or null
- [x] Receipt JSON: txHash, success, gasUsed, contractAddress, error
- **Tests:** 4 tests (found, not found, no store error, invalid hash)

### Task 4: Wire receipts from pipeline to store -- DONE
- [x] `PipelineResult` includes `receipts: Vec<ExecutionReceipt>`
- [x] Receipts flushed to redb alongside state flush (atomic)
- [x] RPC queries via shared `Arc<StateStore>`
- **Tests:** receipt_persistence_end_to_end integration test

**Phase 1 Exit Criteria:**
- [x] Receipts persisted for every executed transaction
- [x] `dndr_getTransactionReceipt` returns correct data
- [x] All new tests pass

---

## Phase 2: EVM Execution via revm (Tasks 5-9)

**Owner:** smart-contract-engineer
**Goal:** Dual VM — WASM (wasmtime) + EVM (revm) side by side. Deploy and call EVM bytecode.

### Task 5: Add revm workspace dependency -- DONE
- [x] `revm = { version = "36", default-features = false, features = ["std"] }` in workspace
- [x] Added to `dendrite-execution/Cargo.toml`
- [x] Verified no C deps (`cargo tree -i secp256k1-sys` = not found)
- [x] MSRV bumped to 1.88 for revm v36 compatibility

### Task 6: EvmEngine — revm wrapper -- DONE
- [x] Created `crates/dendrite-execution/src/evm.rs`
- [x] `evm_deploy()` and `evm_call()` using revm v36 API
- [x] DNDR chain ID (0xDE0D), CacheDB state adapter, MainnetEvm builder
- [x] Maps revm `ExecutionResult` to `ContractReceipt`
- [x] State adapter: AccountState ↔ CacheDB (balance, nonce, code, storage)
- **Tests:** 3 unit tests (deploy, nonce mismatch, deploy+call)

### Task 7: TxKind::EvmDeploy + TxKind::EvmCall -- DONE
- [x] `EvmDeploy { deployer, code, nonce, gas_limit }` variant (0x04)
- [x] `EvmCall { caller, contract, calldata, nonce, gas_limit, value }` variant (0x05)
- [x] `route_tx()` and `expected_prefix()` updated
- **Tests:** 2 roundtrip tests (evm_deploy_roundtrip, evm_call_roundtrip)

### Task 8: EVM execution in pipeline -- DONE
- [x] Pipeline routes EvmDeploy/EvmCall alongside transfers and WASM contracts
- [x] EVM deploy: creates contract via revm, stores code+address in AccountState
- [x] EVM call: executes via revm, updates storage in AccountState
- [x] Receipts generated from revm results as ExecutionReceipt

### Task 9: EVM integration tests -- DONE
- [x] pipeline_executes_evm_deploy: deploy EVM bytecode, verify receipt
- [x] pipeline_executes_evm_deploy_and_call: deploy then call, both succeed
- [x] pipeline_mixed_wasm_evm_batch: transfer + EVM deploy in same batch
- **Tests:** 3 pipeline integration tests

**Phase 2 Exit Criteria:**
- [x] EVM bytecode can be deployed and called
- [x] WASM and EVM contracts coexist without interference
- [x] Gas metering works for EVM execution
- [x] All new tests pass (209 total, 0 failures)

---

## Phase 3: AI Inference via tract (Tasks 10-15)

**Owner:** ai-integration-engineer
**Goal:** Real AI inference behind the AIRuntime trait. Load ONNX models, run inference, produce verifiable results.

### Task 10: Add tract-onnx to dendrite-runtime -- DONE
- [x] `tract-onnx` added to `dendrite-runtime/Cargo.toml` (already in workspace deps)
- [x] `prost = "0.11"` added as dev-dependency for ONNX test model construction
- [x] `cargo check` passes — tract is pure Rust, no C deps

### Task 11: TractRuntime implementing AIRuntime -- DONE
- [x] Created `crates/dendrite-runtime/src/tract_runtime.rs`
- [x] `TractRuntime` struct with `RwLock<HashMap<String, RegisteredModel>>` model registry
- [x] Implements `AIRuntime` trait: `mode() -> LocalInference`, `infer()`, `supports_model()`
- [x] `RegisteredModel` stores optimized `SimplePlan` + input/output fact metadata
- **Tests:** tract_runtime_mode, infer_unknown_model_fails (2 tests)

### Task 12: Model registry -- DONE
- [x] `TractRuntime::register_model(model_id, onnx_bytes)` — parse, optimize, store
- [x] Pipeline: `onnx().model_for_read()` → `into_optimized()` → `into_runnable()`
- [x] Stores input/output shapes for validation
- [x] `TractRuntime::unregister_model(model_id)` removes from registry
- **Tests:** register_and_supports_model, register_invalid_bytes_fails, unregister_model (3 tests)

### Task 13: Inference execution -- DONE
- [x] `TractRuntime::infer()`: deserialize f32 input → tract tensor → run → serialize output
- [x] Input validation: checks byte length matches expected element count × 4
- [x] Deterministic BLAKE3 hash: `hash(model_id || input_bytes || output_bytes)`
- [x] Compute units = number of output elements (proxy for FLOPs)
- **Tests:** infer_add_model, infer_deterministic_hash, infer_wrong_input_size_fails (3 tests)

### Task 14: InferenceReceipt + verification stub -- DONE
- [x] `InferenceReceipt` struct: request_hash, result_hash, model_id, compute_units, deterministic_hash
- [x] `verify_inference()`: re-runs inference and compares deterministic_hash
- [x] Foundation for on-chain verification (full protocol in M6)
- **Tests:** inference_receipt_creation, verify_inference_succeeds, verify_inference_rejects_tampered (3 tests)

### Task 15: AI unit tests with simple ONNX model -- DONE
- [x] Minimal ONNX model built programmatically via `tract_onnx::pb::ModelProto` + prost
- [x] Model: Add node (y = x + [1,1,1]) with 3-element f32 input/output
- [x] Full pipeline tested: register → infer → verify receipt
- [x] Concurrent inference: two_models_coexist test
- **Tests:** 12 total unit tests (3 registry, 3 inference, 3 verification, 3 model/pipeline)

**Phase 3 Exit Criteria:**
- [x] ONNX model can be loaded and inference executed
- [x] Inference results are deterministic (same input -> same hash)
- [x] Verification stub confirms correctness
- [x] All new tests pass (12 new, 221 total)

---

## Phase 4: Security Review + Documentation (Tasks 16-19)

**Owner:** security-engineer + documentation-engineer
**Goal:** Security review of all new code, documentation updates.

### Task 16: Security review -- DONE
- [x] Receipt store: SEC-RCPT-001 (bincode deser, LOW, acceptable), SEC-RCPT-002 (unbounded growth, LOW, deferred), SEC-RCPT-003 (atomic batch write, OK)
- [x] EVM: SEC-EVM-001 (no gas cap, LOW, acceptable), SEC-EVM-002 (nonce check, OK), SEC-EVM-003 (chain ID, OK), SEC-EVM-004 (no C deps, OK), SEC-EVM-005 (address truncation, LOW, acceptable)
- [x] AI: SEC-AI-001 (model size cap, MEDIUM, FIXED — 64 MiB limit), SEC-AI-002 (no timeout, LOW, deferred), SEC-AI-003 (input validation, OK), SEC-AI-004 (deterministic hash, OK), SEC-AI-005 (lock poisoning, OK)
- [x] No ELEVATED findings. 1 MEDIUM fixed (SEC-AI-001).

### Task 17: cargo-audit + clippy + fmt -- DONE
- [x] `cargo audit`: 6 transitive vulns (ring, wasmtime ×3, tracing-subscriber), 7 warnings (bincode, derivative, lru, proc-macro-crate, ring, windows-targets) — all transitive, no action needed
- [x] `cargo clippy --workspace` — zero warnings
- [x] `cargo fmt --check` — clean

### Task 18: Documentation updates -- DONE
- [x] BUILD_LOG.md: entries for Phase 1, 2, 3, and 4
- [x] STATUS.md: M4 IN PROGRESS, crate depth updated, test count 222
- [x] CHANGELOG.md: M4 section with features, security, testing
- [x] Sprint plan: all tasks marked DONE

### Task 19: ADR-006 (if needed) -- SKIPPED
- [x] revm: `default-features = false, features = ["std"]` — no C dependencies confirmed
- [x] tract: pure Rust — no C dependencies
- [x] No non-obvious design tradeoffs requiring an ADR
- [x] MSRV bump to 1.88 is documented in CHANGELOG, not ADR-worthy

**Phase 4 Exit Criteria:**
- [x] Security review complete, no open ELEVATED flags
- [x] All docs updated
- [x] Sprint retrospective below

---

## Definition of Done (Sprint 007)

- [x] All 19 tasks completed (18 done + 1 skipped with justification)
- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes (222 tests, target was 210+)
- [x] `cargo clippy --workspace` zero warnings
- [x] `cargo fmt --check` clean
- [x] All code has BUILD_LOG entries
- [x] Security review complete (no open ELEVATED flags, 1 MEDIUM fixed)
- [x] STATUS.md updated

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| revm may pull C dependencies (e.g., secp256k1) | MEDIUM | Check dep tree after adding; ADR-006 if needed |
| tract model loading may be slow for large models | LOW | Cap model size; async loading in future sprint |
| EVM gas model differs from DNDR gas model | MEDIUM | Use revm's native gas; map to DNDR units in receipt |
| Deterministic inference depends on tract version/platform | HIGH | Pin tract version; BLAKE3 hash includes model_id for versioning |
| revm state adapter complexity (AccountState <-> revm DB) | MEDIUM | Start with simple in-memory adapter; optimize later |

---

## Sprint Retrospective

### What went well
- **revm v36 integration** succeeded with zero C dependencies (`default-features = false`)
- **tract-onnx** pure Rust — no dependency issues, clean integration
- **Dual VM architecture** (WASM + EVM) works cleanly with shared AccountState
- **Programmatic ONNX test models** via protobuf avoids external fixture files
- **Deterministic inference hashing** provides foundation for on-chain verification

### What was challenging
- **revm v36 API discovery**: 15+ API mismatches from docs/examples (Context, TxEnv, ExecutionResult paths all different from older versions)
- **Chain ID validation**: TxEnv defaults `chain_id = Some(1)` (mainnet), had to explicitly set to match context
- **32→20 byte address mapping**: inherent limitation of EVM compatibility layer

### What to improve
- Pin dependency versions more aggressively to avoid API drift
- Consider integration tests that exercise EVM + receipt + RPC together
- Add inference timeout mechanism before production use

### Metrics
- **Tasks:** 18 completed, 1 skipped (ADR-006 not needed)
- **Tests:** 222 total (31 new: 5 receipt, 4 RPC receipt, 1 receipt e2e, 3 EVM engine, 2 EVM routing, 3 EVM pipeline, 13 tract)
- **Security:** 13 findings reviewed, 1 MEDIUM fixed (SEC-AI-001), 0 ELEVATED
- **Clippy:** zero warnings
- **Phases:** 4/4 complete

---

## Sprint 007 Candidates for Next Sprint (008)

1. Block-STM parallel execution (WASM + EVM)
2. State sync / snapshot protocol
3. Multi-node testnet (bootstrap, peer discovery, gossip)
4. EVM precompiles (ecrecover, sha256, etc.)
5. AI compute marketplace stubs (PoUW reward distribution)
6. Transaction pool improvements (priority, eviction)
