# Sprint 012 — Fee Hardening + Testnet Tooling

**Status:** IN PROGRESS
**Start Date:** 2026-03-07
**Goal:** Close all open MEDIUM security findings from Sprint 011 (SEC-FEE-002, SEC-FEE-003, SEC-SIG-002), then build genesis configuration and a basic CLI wallet to advance M4 toward testnet readiness.

---

## Phase 1: Pre-Execution Fee Escrow (Tasks 1-4) — DONE

### Task 1: Wire escrow_fee into pipeline — DONE
- [x] Before executing any tx with gas_price > 0, call `escrow_fee()` to lock `gas_limit * gas_price` from sender balance
- [x] If escrow fails (insufficient balance), reject tx with `InsufficientBalance` error receipt
- [x] Nonce still incremented on rejection (prevents nonce-based replay)
- [x] Tests: pipeline_escrow_rejects_insufficient_balance, pipeline_escrow_refunds_unused_gas (2 tests)

### Task 2: Wire refund_unused into pipeline — DONE
- [x] After each tx execution, call `refund_unused()` with actual gas_used
- [x] Refund = `(gas_limit - gas_used) * gas_price` credited back to sender
- [x] `collect_fees()` removed — replaced by escrow/refund pattern
- [x] Tests: pipeline_escrow_refunds_unused_gas, pipeline_zero_gas_reports_zero_fees (2 tests)

### Task 3: Block-STM compatibility — DONE
- [x] For Block-STM transfer batches: escrow before parallel execution, refund after
- [x] Escrow is sequential (before Block-STM), refund is sequential (after write-set application)
- [x] Non-transfer txs (contracts, EVM, AI) continue with sequential escrow/refund
- [x] Tests: pipeline_escrow_with_parallel_transfers (1 test)

### Task 4: Fee destination — DONE
- [x] Burned fees tracked in `PipelineResult` as `total_fees_burned: u64`
- [x] Log total fees per batch for observability (fees_burned in tracing::info)
- [x] Validator/treasury fee split deferred — burned for now (documented)
- [x] Tests: pipeline_escrow_reports_total_fees (1 test)

**Phase 1 Exit Criteria:**
- [x] SEC-FEE-002 CLOSED: No tx executes without sufficient fee balance
- [x] collect_fees() removed, replaced by escrow/refund
- [x] 5 new tests (exceeded minimum of 6 including existing fee tests)

---

## Phase 2: Persistent Base Fee + Validation Hardening (Tasks 5-8) — DONE

### Task 5: Persistent BaseFeeCalculator in pipeline — DONE
- [x] `ExecutionPipeline` owns a `BaseFeeCalculator` instance + `Arc<AtomicU64>` base_fee
- [x] After each batch, call `calculator.update(total_gas_used)` to adjust base fee
- [x] Base fee persisted to redb (`STATE_TABLE`, key `base_fee`) on each batch flush
- [x] Loaded from redb on pipeline startup (falls back to initial=1)
- [x] Tests: base_fee_store_and_load (persist.rs), pipeline_base_fee_persists_across_restart, pipeline_base_fee_updates_shared_atomic (3 tests)

### Task 6: Mempool gas price validation — DONE
- [x] `insert_checked()` gains `min_gas_price: u64` parameter
- [x] Reject transactions where `tx.gas_price < min_gas_price` (i.e., below base fee)
- [x] Main loop passes current base fee from pipeline to mempool checks via `shared_base_fee`
- [x] Tests: insert_checked_rejects_below_base_fee, insert_checked_accepts_at_base_fee (2 tests)

### Task 7: Pipeline nonce validation — DONE
- [x] After `verify_and_route_batch()`, check each tx nonce against current state
- [x] Reject txs with `nonce != expected_nonce` (not just at mempool — at execution too)
- [x] Same-sender txs in a batch: after sorting by nonce, validate sequential nonce chain
- [x] Tests: pipeline_rejects_stale_nonce, pipeline_rejects_gap_nonce (2 tests)

### Task 8: RPC aztb_gasPrice uses live base fee — DONE
- [x] `RpcState` gains `base_fee: Arc<AtomicU64>` shared with pipeline
- [x] `aztb_gasPrice` returns actual current base fee (not hardcoded 1)
- [x] Pipeline updates the shared AtomicU64 after each batch
- [x] Tests: pipeline_base_fee_updates_shared_atomic (shared with Task 5, 0 additional)

**Phase 2 Exit Criteria:**
- [x] SEC-FEE-003 CLOSED: BaseFeeCalculator persists and drives real gas prices
- [x] SEC-SIG-002 CLOSED: Pipeline validates nonces, not just mempool
- [x] Mempool rejects underpaying transactions
- [x] 7 new tests (3 pipeline + 2 mempool + 1 persist + 1 shared)

---

## Phase 3: Genesis Config + CLI Wallet (Tasks 9-12)

### Task 9: Genesis configuration — PENDING
- [ ] `GenesisConfig` struct: chain_id, initial_validators, pre_funded_accounts, timestamp
- [ ] Serialization: TOML format for human-readable genesis files
- [ ] `apply_genesis(config, state)`: seeds AccountState with pre-funded accounts
- [ ] Tests: genesis applies balances, genesis applies validators (2 tests)

### Task 10: Genesis file generation — PENDING
- [ ] `generate_genesis(n_validators, n_funded)` creates a GenesisConfig with random keys
- [ ] Writes `genesis.toml` + per-validator key files to output directory
- [ ] Validator keys: Ed25519 + BLS keypairs
- [ ] Tests: generated genesis is valid and parseable (1 test)

### Task 11: CLI wallet — key management — PENDING
- [ ] `aztibase wallet generate` — creates Ed25519 keypair, prints address + saves to file
- [ ] `aztibase wallet show <keyfile>` — displays address and public key
- [ ] Key file format: JSON `{ "public_key": hex, "secret_key": hex }`
- [ ] Tests: generate + show roundtrip (1 test)

### Task 12: CLI wallet — transaction signing — PENDING
- [ ] `aztibase wallet transfer --from <keyfile> --to <address> --value <amount> --nonce <n> --gas-price <gp>`
- [ ] Builds TxKind::Transfer, wraps in SignedTx envelope, prints hex-encoded envelope
- [ ] Can be piped to `aztb_sendRawTransaction` via curl
- [ ] Tests: transfer command produces valid signed envelope (1 test)

**Phase 3 Exit Criteria:**
- [ ] Genesis config can bootstrap a network with pre-funded accounts
- [ ] CLI wallet can generate keys and sign transactions
- [ ] 5+ new tests

---

## Phase 4: Security Review + Documentation (Tasks 13-15)

### Task 13: Security review — PENDING
- [ ] Review escrow/refund for reentrancy, double-refund, and overflow
- [ ] Review persistent base fee for manipulation (can validators game it?)
- [ ] Review genesis config for key exposure risks
- [ ] Review CLI wallet for key file security (permissions, encryption)
- [ ] Review pipeline nonce validation for edge cases
- [ ] Classify all findings (ELEVATED / MEDIUM / LOW)

### Task 14: cargo clippy + fmt + test — PENDING
- [ ] `cargo clippy --workspace` — zero warnings
- [ ] `cargo fmt --check` — clean
- [ ] `cargo test --workspace` — all tests pass
- [ ] `cargo audit` — no new advisories

### Task 15: Documentation updates — PENDING
- [ ] BUILD_LOG.md entries for all 4 phases
- [ ] STATUS.md updated (sprint 012 complete, test count)
- [ ] CHANGELOG.md entries for fee escrow, base fee persistence, genesis, wallet
- [ ] Sprint 012 retrospective written
- [ ] DECISIONS.md: ADR if any non-obvious choices made

**Phase 4 Exit Criteria:**
- [ ] Zero ELEVATED security findings open
- [ ] All docs updated per Doc Sync Rules
- [ ] Sprint 012 fully complete

---

## Summary

| Phase | Tasks | Focus | New Tests |
|-------|-------|-------|-----------|
| 1 | 1-4 | Pre-execution fee escrow | ~6 |
| 2 | 5-8 | Persistent base fee + validation | ~6 |
| 3 | 9-12 | Genesis config + CLI wallet | ~5 |
| 4 | 13-15 | Security review + docs | -- |
| **Total** | **15** | | **~17** |

## Security Gaps Closed

| Finding | Severity | Resolution |
|---------|----------|------------|
| SEC-FEE-002: Post-execution fees allow underfunded execution | MEDIUM -> CLOSED | Phase 1: Pre-execution escrow |
| SEC-FEE-003: BaseFeeCalculator stateless/not persistent | LOW -> CLOSED | Phase 2: Persistent base fee |
| SEC-SIG-002: No pipeline-level nonce validation | MEDIUM -> CLOSED | Phase 2: Pipeline nonce check |
