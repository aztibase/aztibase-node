# Sprint 033 — M8 Sprint 8: State Pruning & Bounded Growth

**Goal:** Eliminate all unbounded in-memory and on-disk growth vectors identified during the storage audit. Add round-based pruning to the DagStore (both in-memory index and disk blocks), cap every pipeline buffer, wire up existing but uncalled `prune_before()` in ValidatorWorkHistory, and add eviction to BLOCKS_TABLE and TX_TABLE for full nodes.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

---

## Phase 1: DagStore Round-Based Pruning (Tasks 1–4)

Add `prune_before(round)` to DagStore that removes blocks below a retention horizon from both the in-memory index and the on-disk BLOCKS_TABLE.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `DagStore::prune_before(round)`: remove index entries + disk blocks for rounds < `round - RETENTION_BUFFER`, update `rounds` HashMap, remove orphaned children refs | consensus-engineer | DONE |
| 2 | `DagStore::pruned_round()`: track the lowest retained round, skip already-pruned ranges on repeated calls | consensus-engineer | DONE |
| 3 | Wire `dag.prune_before()` into `ConsensusEngine::evaluate_commits()` after successful commit, using committed wave's anchor round minus retention buffer | consensus-engineer | DONE |
| 4 | 5 unit tests: prune removes old rounds, pruned blocks return `BlockNotFound`, `is_ancestor` handles pruned range, `rebuild_index` after prune loads only retained, prune is idempotent | consensus-engineer | DONE |

**Exit criteria:** DagStore prunes old rounds from RAM + disk, 4 tests pass ✓

---

## Phase 2: Pipeline Memory Caps (Tasks 5–9)

Cap every unbounded in-memory buffer in `ExecutionPipeline`.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `executed_anchors`: convert from `HashSet` to bounded ring buffer (VecDeque + HashSet) — cap at `MAX_EXECUTED_ANCHORS = 10_000`, FIFO eviction | node-engineer | DONE |
| 6 | `attestation_buffer`: add `MAX_ATTESTATIONS_PER_TASK = 32` cap, reject excess attestations for a single task | security-engineer | DONE |
| 7 | Wire eviction calls after batch persistence: `evict_old_receipts`, `evict_old_transactions`, `evict_old_batch_roots` | node-engineer | DONE |
| 8 | `attestation_buffer` total cap: `MAX_ATTESTATION_BUFFER_TASKS = 2048`, evict oldest task_id entries when exceeded | security-engineer | DONE |
| 9 | All existing integration tests pass with bounded buffers — 175 node tests | node-engineer | DONE |

**Exit criteria:** All pipeline buffers bounded, 4 tests pass ✓

---

## Phase 3: Disk Table Eviction (Tasks 10–13)

Add round-aware or count-based eviction to tables that currently grow without bound.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 10 | `StateStore::delete_batch()`: batch delete by key list in a single write transaction (used by DagStore prune + TX eviction) | node-engineer | DONE |
| 11 | TX_TABLE eviction: `evict_old_transactions(store)` — mirrors `evict_old_receipts`, cap at `MAX_STORED_TXS = 500_000` | node-engineer | DONE |
| 12 | BATCH_ROOTS_TABLE eviction: `evict_old_batch_roots(store)` — cap at `MAX_STORED_BATCH_ROOTS = 100_000` | node-engineer | DONE |
| 13 | 3 unit tests: delete_batch atomicity, TX eviction removes oldest, batch roots eviction | node-engineer | DONE |

**Exit criteria:** TX + batch roots tables bounded, delete_batch available, 3 tests pass ✓

---

## Phase 4: Security Review & Documentation (Tasks 14–16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 14 | Security review: verify no data loss for in-flight consensus, confirm pruned blocks can't cause commit rule panic, review eviction ordering | security-engineer | DONE |
| 15 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, all tests pass (455+) | security-engineer | DONE |
| 16 | BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan updated, memory updated | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED findings, clippy/fmt/test clean, all docs updated ✓

---

## Security Considerations

- DagStore prune must retain blocks referenced by uncommitted waves (retention buffer ≥ 2 × wave_length)
- `executed_anchors` eviction must not allow duplicate batch execution — ring buffer still rejects recent duplicates within window
- Attestation buffer cap prevents memory exhaustion from malicious attestation floods
- TX_TABLE eviction is acceptable for full nodes (archive nodes would skip eviction)

## Dependencies

- Phase 1 builds on existing DagStore (dag_store.rs) and ConsensusEngine commit flow
- Phase 2 builds on existing ExecutionPipeline (pipeline.rs) and ValidatorWorkHistory (pouw.rs)
- Phase 3 builds on existing StateStore eviction pattern (evict_oldest)
- Phase 3 Task 10 (delete_batch) is used by Phase 1 Task 1 (DagStore prune) — implement Phase 3 Task 10 first

## Risks

- DagStore prune while consensus is evaluating commits could lose needed blocks — mitigated by retention buffer
- Windows LNK1318 linker PDB limit with growing test count — use CARGO_INCREMENTAL=0
- Eviction during rebuild_index could cause inconsistency — prune only from in-memory, disk prune in same transaction
