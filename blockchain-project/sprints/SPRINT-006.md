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

## Phase 1: JSON-RPC Server (Tasks 1-8)

**Owner:** node-engineer
**Goal:** A working JSON-RPC server embedded in the node binary, queryable via HTTP.

### Task 1: RPC transport layer — hyper HTTP server
- Add `hyper` (or `axum`) to workspace dependencies
- Implement bare HTTP server in `dendrite-rpc` that accepts POST requests
- Parse JSON-RPC 2.0 request envelope (id, method, params)
- Return JSON-RPC 2.0 response envelope
- **Tests:** Valid request/response roundtrip, malformed JSON error

### Task 2: RPC method — `dndr_getBalance`
- Query AccountState by address, return balance as hex string
- Handle unknown address (return 0)
- **Tests:** Known balance, unknown address, invalid address format

### Task 3: RPC method — `dndr_getNonce`
- Query AccountState by address, return nonce
- **Tests:** Known nonce, unknown address

### Task 4: RPC method — `dndr_getCode`
- Query contract code by address, return hex-encoded bytecode
- Return null for non-contract accounts
- **Tests:** Contract with code, EOA without code

### Task 5: RPC method — `dndr_sendTransaction`
- Accept raw encoded transaction bytes (hex string)
- Validate via `route_tx()`, reject malformed
- Submit to mempool via channel
- Return transaction hash
- **Tests:** Valid transfer submission, malformed payload rejection

### Task 6: RPC method — `dndr_getTransactionReceipt`
- Return receipt for executed transaction (status, state root, gas used)
- Return null for unknown/pending tx hash
- **Tests:** Known receipt, unknown hash

### Task 7: RPC method — `dndr_blockNumber`
- Return current committed batch count / latest round
- **Tests:** After 0 batches, after N batches

### Task 8: Wire RPC server into node binary
- Start RPC server on configurable port (default 8545)
- RPC server receives shared read access to AccountState + mempool channel
- Add `--rpc-port` CLI flag
- **Tests:** Node config includes RPC port, server starts

**Phase 1 Exit Criteria:**
- 7 RPC methods implemented and tested
- RPC server starts with the node
- At least 14 new tests

---

## Phase 2: Security Hardening (Tasks 9-15)

**Owner:** security-engineer
**Goal:** Address the top MEDIUM findings from Sprint 005 security reviews.

### Task 9: HashSet for committed blocks (SEC-WIRE-005)
- Replace `Vec<BlockHash>` with `HashSet<BlockHash>` in RoundState
- O(1) lookup instead of O(n) linear scan in extract_committed_batch
- **Tests:** Existing commit tests still pass, performance improvement verified

### Task 10: Double-execution guard (SEC-WIRE-003)
- Track last executed `anchor_hash` in ExecutionPipeline
- Skip batches whose anchor_hash was already executed
- On startup, load last anchor from redb
- **Tests:** Duplicate batch rejected, fresh batch accepted

### Task 11: Fatal flush handling (SEC-WIRE-004)
- If `flush_state()` fails, halt the pipeline (return error, stop processing)
- Node logs critical error and shuts down cleanly
- **Tests:** Simulated flush failure halts pipeline

### Task 12: Unique BLS keys in ValidatorSet (SEC-BLS-002)
- Reject `add()` if BLS public key already exists in the set
- **Tests:** Duplicate key rejected, unique keys accepted

### Task 13: Derive quorum internally in build_certificate (SEC-BLS-008)
- Change `build_certificate` to accept `&ValidatorSet` instead of `quorum: usize`
- Derive quorum via `validator_set.quorum_count()`
- Update all call sites
- **Tests:** Existing certificate tests still pass

### Task 14: func_name validation in routing (SEC-ROUTE-006)
- Validate `func_name` matches `^[a-zA-Z_][a-zA-Z0-9_]{0,127}$`
- Return `RoutingError::InvalidFuncName` for violations
- **Tests:** Valid names pass, empty/too-long/special-char names rejected

### Task 15: Strict bincode decoding — reject trailing bytes (SEC-ROUTE-005)
- Switch from `allow_trailing_bytes()` to strict decode
- Ensure `encode()` output is canonical (no trailing bytes)
- **Tests:** Trailing bytes rejected, clean roundtrip still works

**Phase 2 Exit Criteria:**
- 7 MEDIUM findings resolved
- At least 7 new tests
- No regressions

---

## Phase 3: Integration Testing (Tasks 16-19)

**Owner:** node-engineer + smart-contract-engineer
**Goal:** End-to-end tests that exercise the full pipeline.

### Task 16: Integration test — transfer end-to-end
- Create CommittedBatch with encoded transfer transactions
- Feed through ExecutionPipeline
- Verify balances changed in AccountState
- Verify state persisted to redb
- Verify batch root stored
- **Tests:** 1 integration test

### Task 17: Integration test — contract deploy + call end-to-end
- Deploy WASM contract via CommittedBatch
- Call the contract in a subsequent batch
- Verify contract storage updated
- Verify state root changed
- **Tests:** 1 integration test

### Task 18: Integration test — finality certificate for committed batch
- Execute a batch, compute state root
- Generate BLS signatures from validators
- Build and verify finality certificate
- **Tests:** 1 integration test

### Task 19: Integration test — startup recovery
- Flush state to redb
- Create new ExecutionPipeline with same db path
- Verify state recovered correctly
- Execute additional batch on recovered state
- **Tests:** 1 integration test

**Phase 3 Exit Criteria:**
- 4 integration tests covering the full pipeline
- All pass with correct state transitions

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
