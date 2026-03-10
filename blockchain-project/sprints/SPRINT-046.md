# Sprint 046 — Weak Subjectivity Checkpoints (M9-S7)

**Status:** COMPLETE
**Started:** 2026-03-10
**Completed:** 2026-03-10
**Engineer(s):** node-engineer, consensus-engineer, security-engineer

---

## Goal

Implement weak subjectivity checkpoint distribution and validation. New/recovering nodes can verify they are on the correct chain by checking a trusted checkpoint (batch index + state root + finality cert). Checkpoints are emitted every N batches and stored persistently.

## Scope

~8 files: checkpoint types, storage table, raw persistence layer, pipeline emission, CLI config, RPC endpoints.

---

## Phase 1: Checkpoint Data Structures

### Task 1.1 — Checkpoint type in aztibase-consensus
**File:** `crates/aztibase-consensus/src/checkpoint.rs`
- `Checkpoint { batch_index: u64, state_root: [u8;32], finality_cert: Option<FinalityCertificate>, timestamp: u64 }`
- Made `finality_cert` optional (pipeline doesn't have access to finality certs during batch execution)
- CHECKPOINT_INTERVAL constant (default: 1000 batches)
- 7 tests: creation, creation_without_cert, matches, serialization_roundtrip, serialization_without_cert
- **Status:** DONE

---

## Phase 2: Storage & Emission

### Task 2.1 — Checkpoint persistence in StateStore
**File:** `crates/aztibase-storage/src/store.rs`, `crates/aztibase-execution/src/persist.rs`
- `CHECKPOINTS_TABLE: TableDefinition<&[u8], &[u8]>` in storage crate
- Raw byte persistence functions in execution crate (avoids cross-crate dependency on consensus):
  - `store_checkpoint_raw(store, batch_index, data)` — stores serialized checkpoint bytes
  - `get_checkpoint_raw(store, batch_index)` — retrieves by batch index
  - `latest_checkpoint_raw(store)` — returns most recent checkpoint bytes
- 2 new persist tests: checkpoint_store_and_retrieve, checkpoint_latest
- **Status:** DONE

### Task 2.2 — Pipeline emits checkpoints
**File:** `crates/aztibase-node/src/pipeline.rs`
- Every CHECKPOINT_INTERVAL batches, creates Checkpoint with current state_root and timestamp
- Serializes via postcard and stores using `store_checkpoint_raw()`
- Logs checkpoint storage with batch number and state root prefix
- **Status:** DONE

---

## Phase 3: CLI & Startup Validation

### Task 3.1 — CLI trusted checkpoint flag
**File:** `crates/aztibase-node/src/main.rs`
- `--checkpoint <batch_index:state_root_hex>` CLI flag
- On startup, reads stored checkpoint at given batch index
- Deserializes and compares state root against trusted value
- Aborts with error if mismatch or checkpoint not found
- Passes silently if checkpoint matches
- **Status:** DONE

---

## Phase 4: RPC & Tests

### Task 4.1 — Checkpoint RPC endpoints
**File:** `crates/aztibase-rpc/src/server.rs`
- `aztb_getCheckpoint(batch_index)` — returns checkpoint JSON at given batch
- `aztb_latestCheckpoint` — returns most recent checkpoint JSON
- `checkpoint_to_json()` helper for DRY response formatting
- Response: `{ batch_index, state_root (0x-prefixed hex), has_finality_cert, timestamp }`
- **Status:** DONE

### Task 4.2 — Unit tests
- Checkpoint creation and serialization (consensus crate): 5 existing + 2 new
- Store/retrieve roundtrip (execution crate): 2 new persist tests
- RPC dispatch (rpc crate): covered by existing routing tests
- **Status:** DONE

### Task 4.3 — cargo clippy + fmt + test
- 0 clippy warnings, fmt clean, 811 tests pass
- **Status:** DONE

---

## Phase 5: Documentation

### Task 5.1 — Doc updates
- BUILD_LOG.md, STATUS.md, CHANGELOG.md, SPRINT-046.md updated
- **Status:** DONE

---

## Exit Criteria

- [x] Checkpoint struct with batch_index, state_root, finality_cert (optional), timestamp
- [x] Persistent checkpoint storage (raw bytes at persist layer, typed at application layer)
- [x] Pipeline emits checkpoints every CHECKPOINT_INTERVAL batches
- [x] CLI trusted checkpoint validation on startup
- [x] RPC endpoints for checkpoint queries (aztb_getCheckpoint, aztb_latestCheckpoint)
- [x] All tests pass (811 total)
- [x] cargo clippy: 0 warnings, cargo fmt: clean

---

## Retrospective

**What went well:**
- Clean separation of concerns: raw bytes at persistence layer, typed Checkpoint at application layer (pipeline/RPC)
- Making finality_cert optional was the right call — pipeline doesn't produce finality certs during batch execution
- Reused existing CHECKPOINTS_TABLE from storage crate, only needed raw persistence functions in execution

**What was learned:**
- Cross-crate dependency direction matters: execution cannot depend on consensus, so raw byte APIs prevent circular deps
- `is_multiple_of()` is the idiomatic Rust way (clippy enforces it over manual `% == 0`)

**Metrics:**
- Tests: 811 total (112 consensus + 34 core + 272 execution + 70 network + 194 node + 65 rpc + 22 runtime + 22 storage + 20 wasm)
- New tests: 4 (2 consensus + 2 execution persist)
- Security: 0 ELEVATED, 0 MEDIUM
- Duration: 1 session
