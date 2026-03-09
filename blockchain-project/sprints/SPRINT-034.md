# Sprint 034 — M8 Sprint 9: Archive Node & Historical Queries

**Goal:** Add archive node mode that retains full history (no eviction, no DAG pruning), and expose historical query RPCs (block-by-number, block-by-hash, transaction-by-hash, block-range) so block explorers and indexers can read the chain.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

---

## Phase 1: Archive Mode (Tasks 1–4)

Add `--archive` CLI flag and `archive` config field that disables all eviction and DAG pruning, enabling full history retention.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add `archive: bool` to `NodeConfig` (default `false`), TOML-serializable | node-engineer | DONE |
| 2 | Add `--archive` CLI flag to `Cli` struct, wire into `NodeConfig` | node-engineer | DONE |
| 3 | Gate eviction calls in `pipeline.rs`: skip `evict_old_receipts`, `evict_old_transactions`, `evict_old_batch_roots` when `archive = true` | node-engineer | DONE |
| 4 | Gate DAG pruning: pass `archive` flag to consensus engine, skip `dag.prune_before()` when archive mode | consensus-engineer | DONE |

**Exit criteria:** `--archive` flag compiles, eviction + pruning skipped when set, existing tests pass

---

## Phase 2: Historical Query RPCs (Tasks 5–9)

Add RPC methods for querying historical blocks and transactions by hash/number.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `aztb_getBlockByNumber(round)` — query BLOCKS_TABLE, return serialized block at given round | node-engineer | DONE |
| 6 | `aztb_getBlockByHash(hash)` — query BLOCKS_TABLE by block hash, return block data | node-engineer | DONE |
| 7 | `aztb_getTransactionByHash(hash)` — query TX_TABLE by tx hash, return transaction data | node-engineer | DONE |
| 8 | `aztb_getBatchRoot(batch_hash)` — query BATCH_ROOTS_TABLE, return state root for a committed batch | node-engineer | DONE |
| 9 | 4 unit tests: one per new RPC method, verify round-trip store→query | node-engineer | DONE |

**Exit criteria:** 4 new RPC methods working, 4 tests pass

---

## Phase 3: Block Range & Explorer Support (Tasks 10–14)

Add paginated range queries for block explorer backends.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 10 | `aztb_getBlockRange(from, to, limit)` — paginated block range query using `StateStore::range()`, max 100 blocks per call | node-engineer | DONE |
| 11 | `aztb_getTransactionsByBatch(batch_hash)` — return all transaction hashes associated with a batch | node-engineer | DONE |
| 12 | `aztb_getReceiptsByBatch(batch_hash)` — return all receipts for transactions in a batch | node-engineer | DONE |
| 13 | 4 unit tests: range pagination, empty range, batch tx listing, batch receipts | node-engineer | DONE |
| 14 | Validate all existing tests pass with new code (455+ tests) | node-engineer | DONE |

**Exit criteria:** 3 new RPC methods, pagination enforced, 4 tests pass, no regressions

---

## Phase 4: Security Review & Documentation (Tasks 15–17)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 15 | Security review: validate range query bounds (no unbounded iteration), pagination limits enforced, archive mode doesn't leak extra data through RPC | security-engineer | DONE |
| 16 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, all tests pass | security-engineer | DONE |
| 17 | BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan updated, memory updated | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED findings, clippy/fmt/test clean, all docs updated

---

## Security Considerations

- Range queries MUST enforce a hard limit (MAX_BLOCK_RANGE = 100) to prevent DoS via unbounded reads
- Archive mode increases disk usage — document in node operator guide
- Historical queries on non-archive nodes may return `null` for evicted data (expected, not an error)
- No new tables needed — queries use existing BLOCKS_TABLE, TX_TABLE, RECEIPTS_TABLE, BATCH_ROOTS_TABLE

## Dependencies

- Phase 1 uses existing eviction functions + DAG prune (Sprint 033)
- Phase 2 uses existing StateStore::get() and postcard deserialization
- Phase 3 uses existing StateStore::range() for bounded iteration
- All phases build on existing RPC dispatch (server.rs)

## Risks

- Windows LNK1318 linker PDB limit with growing test count — use CARGO_INCREMENTAL=0
- Archive nodes will use significantly more disk — acceptable for testnet, document for operators
