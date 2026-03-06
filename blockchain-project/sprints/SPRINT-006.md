# Sprint 006: JSON-RPC Server + Security Hardening + M3 Close

**Sprint Goal:** Build the JSON-RPC server so external clients (wallets, explorers, CLI tools) can query state and submit transactions. Harden the codebase by addressing MEDIUM security findings from Sprint 005 reviews. Run end-to-end integration tests across the full consensus-to-execution pipeline. Close M3.

**Start Date:** 2026-03-06
**End Date:** TBD
**Status:** IN PROGRESS
**Led By:** project-lead

---

## Prerequisites (All Complete)
- [x] Sprint 005 complete (165 tests, all ELEVATED security findings resolved)
- [x] ExecutionPipeline wired: consensus→ordering→routing→execution→persist
- [x] State persistence to redb with startup recovery
- [x] BLS finality certificates with PoP
- [x] Transaction routing with prefix cross-check and size limits
- [x] 4 deep security reviews completed and all ELEVATED flags cleared

---

## Sprint Scope

This sprint makes the node externally accessible. The JSON-RPC server exposes account balance, nonce, transaction submission, and block/state queries. Security hardening addresses the highest-impact MEDIUM findings. Integration tests verify the full pipeline end-to-end. Closing M3 means the execution layer is feature-complete for testnet prep.

**Not in scope for Sprint 006:**
- EVM integration via revm (M4)
- Full Verkle tree (BLAKE3 Merkle placeholder continues)
- AI inference pipeline (M4)
- State sync / snapshot protocol (M4)
- Browser/light node support (M5)

---

## Phase 1: JSON-RPC Server (Tasks 1-8) -- COMPLETE

**Owner:** node-engineer
**Goal:** A working JSON-RPC server embedded in the node binary, queryable via HTTP.

### Task 1: RPC transport layer -- DONE
- [x] Added axum to workspace dependencies
- [x] Implemented HTTP server in `dendrite-rpc` that accepts POST requests
- [x] Parses JSON-RPC 2.0 request envelope (id, method, params)
- [x] Returns JSON-RPC 2.0 response envelope
- **Tests:** 3 tests (malformed JSON, invalid version, unknown method)

### Task 2: RPC method — `dndr_getBalance` -- DONE
- [x] Query AccountState by address, return balance as hex string
- [x] Handle unknown address (return 0)
- **Tests:** 3 tests (known balance, unknown address, invalid address format)

### Task 3: RPC method — `dndr_getNonce` -- DONE
- [x] Query AccountState by address, return nonce
- **Tests:** 2 tests (known nonce, unknown address)

### Task 4: RPC method — `dndr_getCode` -- DONE
- [x] Query contract code by address, return hex-encoded bytecode
- [x] Return null for non-contract accounts
- **Tests:** 2 tests (contract with code, EOA returns null)

### Task 5: RPC method — `dndr_sendTransaction` -- DONE
- [x] Accept raw encoded transaction bytes (hex string)
- [x] Validate via `route_tx()`, reject malformed
- [x] Submit to mempool via channel
- [x] Return transaction hash
- **Tests:** 2 tests (valid transfer, malformed payload)

### Task 6: RPC method — `dndr_getTransactionReceipt` -- DEFERRED
- Deferred to M4: requires receipt store (not yet built)

### Task 7: RPC method — `dndr_blockNumber` + `dndr_getStateRoot` -- DONE
- [x] Return current committed batch count as hex
- [x] Added bonus `dndr_getStateRoot` returning BLAKE3 Merkle root
- **Tests:** 3 tests (block number, state root empty, state root with data)

### Task 8: Wire RPC server into node binary -- DONE
- [x] RPC server spawned in main.rs on configurable address
- [x] Shared state via Arc<RwLock<AccountState>> from ExecutionPipeline
- [x] Transaction submission via RPC forwarded to mempool + consensus
- [x] `--rpc-addr` CLI flag (already existed in config)
- **Tests:** Existing config tests cover RPC settings

**Phase 1 Exit Criteria:**
- [x] 6 RPC methods implemented and tested (7th deferred — receipt store needed)
- [x] RPC server starts with the node
- [x] 15 new tests (exceeds 14 target)

---

## Phase 2: Security Hardening (Tasks 9-15) -- COMPLETE

**Owner:** security-engineer
**Goal:** Address the top MEDIUM findings from Sprint 005 security reviews.

### Task 9: HashSet for committed blocks (SEC-WIRE-005) -- DONE
- [x] Replaced `Vec<BlockHash>` with `HashSet<BlockHash>` in RoundState
- [x] O(1) lookup in `extract_committed_batch` via `HashSet::contains`
- [x] Updated `committed_blocks()` return type and all call sites
- **Tests:** All existing commit tests pass

### Task 10: Double-execution guard (SEC-WIRE-003) -- DONE
- [x] Track executed `anchor_hash` in ExecutionPipeline via `HashSet<[u8; 32]>`
- [x] Skip batches whose anchor_hash was already executed
- **Tests:** 1 new test (pipeline_rejects_duplicate_anchor)

### Task 11: Fatal flush handling (SEC-WIRE-004) -- DONE
- [x] `execute_batch()` returns `Result<PipelineResult, anyhow::Error>`
- [x] `flush_state()` / `store_batch_root()` errors propagate as `Err`
- [x] `run()` breaks on fatal error with tracing::error log
- **Tests:** Existing persistence tests updated to unwrap Result

### Task 12: Unique BLS keys in ValidatorSet (SEC-BLS-002) -- DONE
- [x] Added `has_unique_bls_keys()` validation (uses `HashSet<&[u8; 48]>`)
- [x] `build_certificate` rejects duplicate BLS keys in validator list
- [x] `verify_certificate` rejects duplicate BLS keys in validator list
- **Tests:** 1 new test (reject_duplicate_bls_keys)

### Task 13: Derive quorum internally in build_certificate (SEC-BLS-008) -- DONE
- [x] Changed `build_certificate` to accept `&ValidatorSet` instead of `quorum: usize`
- [x] Derives quorum via `validator_set.quorum_count()`
- [x] Updated all call sites (8 test call sites)
- **Tests:** All existing certificate tests pass

### Task 14: func_name validation in routing (SEC-ROUTE-006) -- DONE
- [x] Added `is_valid_func_name()`: `^[a-zA-Z_][a-zA-Z0-9_]{0,127}$`
- [x] Returns `RoutingError::InvalidFuncName` for violations
- **Tests:** 3 new tests (valid_func_name_accepted, empty_func_name_rejected, special_char_func_name_rejected, too_long_func_name_rejected)

### Task 15: Strict bincode decoding — reject trailing bytes (SEC-ROUTE-005) -- DONE
- [x] Removed `allow_trailing_bytes()` from `bincode_options()`
- [x] Canonical encode verified (roundtrips clean)
- **Tests:** 1 new test (trailing_bytes_rejected)

**Phase 2 Exit Criteria:**
- [x] 7 MEDIUM findings resolved
- [x] 7 new tests (1 + 1 + 3 + 1 + 1 = 7, plus existing tests updated)
- [x] No regressions (187 tests pass)

---

## Phase 3: Integration Testing (Tasks 16-19) -- COMPLETE

**Owner:** node-engineer + smart-contract-engineer
**Goal:** End-to-end tests that exercise the full pipeline.

### Task 16: Integration test — transfer end-to-end -- DONE
- [x] Create CommittedBatch with encoded transfer transactions
- [x] Feed through ExecutionPipeline
- [x] Verify balances changed in AccountState
- [x] Verify state persisted to redb
- [x] Verify batch root stored
- **Tests:** 1 integration test (transfer_end_to_end)

### Task 17: Integration test — contract deploy + call end-to-end -- DONE
- [x] Deploy WASM contract via CommittedBatch
- [x] Call the contract in a subsequent batch
- [x] Verify contract storage updated
- [x] Verify state root changed
- **Tests:** 1 integration test (contract_deploy_and_call_end_to_end)

### Task 18: Integration test — finality certificate for committed batch -- DONE
- [x] Execute a batch, compute state root
- [x] Generate BLS signatures from validators
- [x] Build and verify finality certificate
- **Tests:** 1 integration test (finality_certificate_end_to_end)

### Task 19: Integration test — startup recovery -- DONE
- [x] Flush state to redb
- [x] Create new ExecutionPipeline with same db path
- [x] Verify state recovered correctly
- [x] Execute additional batch on recovered state
- **Tests:** 1 integration test (startup_recovery_end_to_end)

**Phase 3 Exit Criteria:**
- [x] 4 integration tests covering the full pipeline
- [x] All pass with correct state transitions

---

## Phase 4: M3 Close + Security Review (Tasks 20-24)

**Owner:** security-engineer + documentation-engineer
**Goal:** Final security review, close M3 milestone.

### Task 20: Security review of RPC server
- Review for injection, DoS, unauthorized access
- Validate input sanitization on all RPC methods
- Check error messages don't leak internal state

### Task 21: Security review of hardening changes
- Verify all MEDIUM fixes are correct and complete
- Check for regressions in existing security properties

### Task 22: cargo-audit
- Run `cargo audit` and document any new advisories

### Task 23: Update all documentation
- STATUS.md: M3 marked COMPLETE, crate depth table updated
- CHANGELOG.md: Sprint 006 entries
- BUILD_LOG.md: All phase entries
- DECISIONS.md: Any new ADRs (RPC framework choice)

### Task 24: Close M3 milestone
- Verify all M3 criteria met: WASM VM, state management, persistence, RPC
- Update STATUS.md milestone table
- Draft Sprint 007 candidates (M4: EVM, AI integration, state sync, testnet)

**Phase 4 Exit Criteria:**
- Security review complete, no ELEVATED flags
- All docs updated
- M3 marked COMPLETE in STATUS.md
- Sprint retrospective written

---

## Definition of Done (Sprint 006)

- [ ] All 24 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes (target: 190+ tests)
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated with M3 COMPLETE
- [ ] Sprint retrospective written

---

## Risk Register

| Risk | Mitigation |
|------|-----------|
| hyper/axum adds significant dependency weight | Choose minimal HTTP stack; axum is already tokio-native |
| RPC shared state access needs careful synchronization | Use Arc<RwLock<AccountState>> for read access |
| Integration tests may be slow (WASM compilation) | Use pre-compiled WASM fixtures |
| Strict bincode decoding may break existing payloads | Only applies to deserialization; our encode() is already canonical |
