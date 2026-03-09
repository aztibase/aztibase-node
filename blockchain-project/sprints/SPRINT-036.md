# Sprint 036 — M8 Sprint 11: Governance Execution & Chain Parameters

**Goal:** Close the governance loop by adding a runtime-mutable ChainParams registry, executing passed proposals to apply parameter changes, and wiring the pipeline/fee calculator/eviction to read from ChainParams instead of hardcoded constants.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

---

## Phase 1: ChainParams Registry (Tasks 1–5)

Runtime-mutable parameter store with typed defaults, get/set, validation, and change history.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `ChainParams` struct: HashMap<String, ParamValue> with typed `ParamValue` enum (U64, Bool, String) | smart-contract-engineer | DONE |
| 2 | `ParamDef` registry: static table of known param keys with type, default, min/max bounds, description | smart-contract-engineer | DONE |
| 3 | `ChainParams::get(key) -> Option<ParamValue>`, `set(key, value) -> Result`, `list() -> Vec<(key, value)>`, `defaults() -> Self` | smart-contract-engineer | DONE |
| 4 | Seed ChainParams with governance-controllable params: `base_fee_floor`, `base_fee_ceiling`, `target_gas_per_batch`, `max_gas_per_batch`, `base_fee_change_denom`, `max_block_range`, `max_stored_txs`, `max_stored_batch_roots`, `max_stored_receipts` | smart-contract-engineer | DONE |
| 5 | 6 unit tests: get/set roundtrip, type mismatch rejection, bounds validation, defaults populated, list all, unknown key rejection | smart-contract-engineer | DONE |

**Exit criteria:** ChainParams compiles with typed get/set, bounds validation, 6 tests pass ✓

---

## Phase 2: Proposal Execution Engine (Tasks 6–10)

Execute passed governance proposals by applying parameter changes to ChainParams.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 6 | `execute_passed_proposals()` in pipeline: after `finalize_expired()`, iterate Passed proposals, parse param_value to ParamValue, call `chain_params.set()` | node-engineer | DONE |
| 7 | `GovernanceStore::passed_unexecuted() -> Vec<Proposal>`: return Passed proposals not yet Executed | smart-contract-engineer | DONE |
| 8 | On successful set: call `mark_executed(proposal_id)`, log param change with old→new values | node-engineer | DONE |
| 9 | On failed set (invalid value, unknown key, out of bounds): log warning, mark proposal Executed anyway (prevent retry loop), record failure reason | node-engineer | DONE |
| 10 | 4 unit tests: successful execution applies param, failed execution marks executed with error, multiple proposals execute in order, already-executed proposal skipped | smart-contract-engineer | DONE |

**Exit criteria:** Passed proposals auto-execute, ChainParams updated, mark_executed called, 4 tests pass ✓

---

## Phase 3: Pipeline & Fee Integration (Tasks 11–15)

Wire fee calculator, eviction, and RPC to read from ChainParams instead of hardcoded constants.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 11 | `BaseFeeCalculator`: read `base_fee_floor`, `base_fee_ceiling`, `target_gas_per_batch`, `max_gas_per_batch`, `base_fee_change_denom` from shared ChainParams at batch start | smart-contract-engineer | DONE |
| 12 | Eviction: read `max_stored_txs`, `max_stored_batch_roots`, `max_stored_receipts` from ChainParams in pipeline eviction phase | node-engineer | DONE |
| 13 | RPC: `aztb_getBlockRange` reads `max_block_range` from ChainParams; new `aztb_getChainParam(key)` and `aztb_listChainParams()` RPC methods | node-engineer | DONE |
| 14 | Pipeline: `Arc<RwLock<ChainParams>>` shared across pipeline, RPC, fee calculator; `.with_chain_params()` builder | node-engineer | DONE |
| 15 | 4 unit tests: fee calc uses dynamic params, eviction uses dynamic limits, RPC returns current params, param change reflected in next batch | node-engineer | DONE |

**Exit criteria:** No hardcoded fee/eviction constants in hot path, 2 new RPCs, 4 tests pass ✓

---

## Phase 4: Security Review & Documentation (Tasks 16–18)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 16 | Security review: bounds prevent dangerous values (zero gas, zero base fee), only Passed proposals execute, param key whitelist enforced, no unbounded iteration | security-engineer | DONE |
| 17 | `cargo clippy` zero warnings, `cargo fmt --check` clean, all tests pass | security-engineer | DONE |
| 18 | Update BUILD_LOG, STATUS, CHANGELOG, sprint plan, MEMORY | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED, 0 MEDIUM, clippy clean, fmt clean, all tests pass, docs updated
