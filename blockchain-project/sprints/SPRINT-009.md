# Sprint 009 — Parallel Execution + State Sync + EVM Precompiles

**Status:** IN PROGRESS
**Start Date:** 2026-03-06
**Goal:** Make the testnet production-viable — parallel tx execution for throughput, state sync for node onboarding, EVM precompiles for Solidity compatibility.

---

## Phase 1: Block-STM Parallel Execution (Tasks 1-5) — DONE

### Task 1: Multi-versioned memory (MVMemory) — DONE
- [x] `StateKey` enum: Balance(Address), Nonce(Address), Code(Address), Storage(Address, Vec<u8>)
- [x] `MVMemory` struct: tracks writes per (tx_index, key), reads return latest version before tx_index
- [x] `WriteSet` / `ReadSet` types for per-tx tracking
- [x] Tests: write/read ordering, version visibility, no cross-contamination (4 tests)

### Task 2: Block-STM scheduler — DONE
- [x] `TxStatus` enum: ReadyToExecute, Executing, Executed, Validated, Aborting
- [x] `Scheduler` struct: tracks status of each tx index, hands out next task
- [x] `next_task()` returns Execute(idx) or Validate(idx) or Done
- [x] Re-execution on validation failure: mark dependents for re-execution
- [x] Tests: scheduling order, re-execution after conflict (3 tests)

### Task 3: Block-STM executor core — DONE
- [x] `BlockSTMExecutor` struct: owns MVMemory + Scheduler
- [x] `execute(txs, base_state) -> Vec<TxOutput>` and `execute_full() -> (Vec<TxOutput>, Vec<WriteSet>)`
- [x] Single-threaded execution loop (parallel via rayon deferred to Sprint 010)
- [x] Each tx executes against MVMemory via MVView, records read/write sets
- [x] Validation: check all reads still return same values after earlier txs commit
- [x] Re-execute on conflict until all txs validated
- [x] Tests: independent txs, conflicting txs, nonce mismatch, insufficient balance, empty batch (6 tests)

### Task 4: Transfer execution adapter — DONE
- [x] `MVView` wrapper: reads from MVMemory with automatic read-set tracking, falls back to base AccountState
- [x] Handles balance reads/writes, nonce checks via read_balance/read_nonce
- [x] `apply_block_stm_to_state()` applies write sets to AccountState
- [x] Tests: MVView reads from base, MVView reads from mv_memory, state application matches sequential (3 tests)

### Task 5: Pipeline integration — DONE
- [x] `ExecutionPipeline` uses `BlockSTMExecutor::execute_full` for all transfer batches
- [x] Falls back to sequential for contract/EVM/AI txs (Block-STM for transfers only in v1)
- [x] Determinism: all 14 existing pipeline tests pass unchanged (proves correctness equivalence)
- [x] Mixed batches: transfers via Block-STM, contracts/EVM/AI via sequential execution

**Phase 1 Exit Criteria:**
- [x] Block-STM produces identical state roots to sequential execution (block_stm_state_application test)
- [x] All existing 264 tests still pass (16 new Block-STM tests added)
- [x] New tests cover MVMemory (4), scheduler (3), executor (6), MVView (2), state application (1)

---

## Phase 2: State Sync Protocol (Tasks 6-9)

### Task 6: State snapshot creation
- [ ] `StateSnapshot` struct: serialized AccountState + batch index + state root
- [ ] `create_snapshot(state, batch_index) -> StateSnapshot`
- [ ] Snapshot serialization via bincode with size validation
- [ ] Tests: snapshot creation, serialization roundtrip

### Task 7: Snapshot request/response messages
- [ ] `SyncRequest` / `SyncResponse` message types on TOPIC_STATE_SYNC
- [ ] Request: peer asks for snapshot at latest batch index
- [ ] Response: peer sends serialized snapshot (chunked if > 1MB)
- [ ] Wire format with version byte (reuse pattern from wire.rs)
- [ ] Tests: request/response roundtrip, chunking

### Task 8: Node bootstrap from snapshot
- [ ] On startup, if local state is empty, request snapshot from peers
- [ ] Apply received snapshot to AccountState and redb
- [ ] Verify state root matches announced roots from peers
- [ ] Tests: bootstrap from snapshot, state root verification

### Task 9: Snapshot protocol integration
- [ ] Wire into main event loop: handle SyncRequest/SyncResponse on TOPIC_STATE_SYNC
- [ ] Periodic snapshot creation (every N batches)
- [ ] Tests: end-to-end snapshot sync between two nodes

**Phase 2 Exit Criteria:**
- [ ] New node can bootstrap state from existing peer
- [ ] State roots match after sync
- [ ] All tests pass

---

## Phase 3: EVM Precompiles (Tasks 10-13)

### Task 10: Identity + SHA-256 + RIPEMD-160 precompiles
- [ ] Identity (0x04): returns input unchanged
- [ ] SHA-256 (0x02): sha2 crate, pure Rust
- [ ] RIPEMD-160 (0x03): ripemd crate, pure Rust
- [ ] Gas cost calculations per EIP-2028
- [ ] Tests: known test vectors for each

### Task 11: ecrecover precompile (0x01)
- [ ] secp256k1 signature recovery via k256 crate (pure Rust)
- [ ] Input: hash(32) + v(32) + r(32) + s(32) → recovered address(20)
- [ ] Tests: known ecrecover test vectors from Ethereum

### Task 12: modexp precompile (0x05)
- [ ] Big integer modular exponentiation
- [ ] Use num-bigint crate (pure Rust)
- [ ] Gas calculation per EIP-2565
- [ ] Tests: known modexp test vectors

### Task 13: Register precompiles with revm
- [ ] Create custom `PrecompileSet` for Aztibase
- [ ] Wire into `evm_deploy` / `evm_call` context
- [ ] Skip bn128 (complex, deferred — revm includes via arkworks)
- [ ] Tests: Solidity contract calling ecrecover via EVM execution

**Phase 3 Exit Criteria:**
- [ ] ecrecover, sha256, ripemd160, identity, modexp all working
- [ ] Solidity contracts can use precompiles through EVM
- [ ] All tests pass

---

## Phase 4: Security Review + Documentation (Tasks 14-16)

### Task 14: Security review
- [ ] Block-STM: race conditions, determinism under re-execution, read set validation
- [ ] State sync: snapshot integrity, DoS via large snapshots, state root verification
- [ ] Precompiles: input validation, gas correctness, no panics on malformed input
- [ ] Rate findings as LOW/MEDIUM/ELEVATED

### Task 15: cargo-audit + clippy + fmt
- [ ] `cargo audit` — document any new advisories
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo fmt --check` — clean

### Task 16: Documentation updates
- [ ] BUILD_LOG.md: entries for each phase
- [ ] STATUS.md: M4 progress update
- [ ] CHANGELOG.md: new features, security items
- [ ] Sprint plan: mark tasks DONE, write retrospective

**Phase 4 Exit Criteria:**
- [ ] Security review complete, no open ELEVATED flags
- [ ] All docs updated
- [ ] Sprint retrospective written

---

## Definition of Done (Sprint 009)

- [ ] All 16 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes
- [ ] `cargo test --workspace` passes (target: 280+ tests)
- [ ] `cargo clippy --workspace` zero warnings
- [ ] `cargo fmt --check` clean
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated

---

## Sprint 009 Candidates for Next Sprint (010)

1. Rayon parallelism for Block-STM (multi-threaded execution)
2. Block-STM for contract/EVM txs (read/write tracking in VM)
3. WebRTC transport for browser nodes
4. AI compute marketplace stubs (PoUW reward distribution)
5. Verkle tree state commitment (replace BLAKE3 Merkle placeholder)
6. bn128 precompiles (ecAdd, ecMul, ecPairing)
7. Light client protocol
