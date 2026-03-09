# Sprint 031 — M8 Sprint 6: End-to-End Smoke Tests & Integration Testing

**Goal:** Fill the critical testing gaps: RPC-driven transaction flows, full AI compute market lifecycle (RegisterModel → CommitCompute → PostTask → SubmitAttestation → Settlement), multi-transfer stress, and fee market integration. All tests exercise the pipeline through the same paths production code uses.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

---

## Phase 1: RPC-Driven Transaction E2E Tests (Tasks 1–4) ✓

Existing integration tests call `pipeline.execute_batch()` directly. These tests verify the full path: construct signed transaction → submit through pipeline → verify state changes + receipts.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `fee_market_transfer_e2e`: Transfer with non-zero gas_price — verify fee escrow, gas deduction, receipt gas_used=21000, sender balance = initial - value - fee | node-engineer | DONE |
| 2 | `multi_transfer_batch_stress`: 20 sequential transfers from same sender — verify all nonces honored, final balances correct, all receipts success=true | node-engineer | DONE |
| 3 | `nonce_gap_rejection_e2e`: Submit txs with nonce gap (0, 2 — skip 1) — verify second tx fails with nonce error, first succeeds | node-engineer | DONE |
| 4 | `insufficient_balance_receipt_e2e`: Transfer exceeding balance — verify receipt success=false, error contains "insufficient", sender balance unchanged | node-engineer | DONE |

**Exit criteria:** 4 new e2e tests pass, all exercise pipeline.execute_batch with signed transactions ✓

---

## Phase 2: AI Compute Market Lifecycle E2E (Tasks 5–10) ✓

Full pipeline: register model → commit compute → post task → submit attestation → verify settlement. This is the most complex untested flow.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `register_model_e2e`: RegisterModel tx → verify model appears in ModelRegistry via pipeline shared state, receipt success=true, nonce incremented | ai-integration-engineer | DONE |
| 6 | `commit_compute_e2e`: CommitCompute tx with valid BLS PoP → verify validator in ComputeCommitmentStore, stake deducted from balance | consensus-engineer | DONE |
| 7 | `post_task_e2e`: PostTask tx referencing registered model → verify task in TaskPool, reward escrowed from requester, assigned_validator set | ai-integration-engineer | DONE |
| 8 | `submit_attestation_e2e`: SubmitAttestation with valid Ed25519 sig from assigned validator → verify attestation buffered, receipt success=true | consensus-engineer | DONE |
| 9 | `full_ai_lifecycle_e2e`: Complete flow: RegisterModel → CommitCompute → PostTask → 2x SubmitAttestation (quorum) → verify settlement payouts to validators + task removed from pool | ai-integration-engineer | DONE |
| 10 | `deregister_compute_refund_e2e`: CommitCompute → DeregisterCompute → verify stake refunded, validator removed from store | consensus-engineer | DONE |

**Exit criteria:** 6 new e2e tests pass, full AI compute lifecycle verified end-to-end ✓

**Bug discovered:** Attestation signature verification was non-functional through signed tx flow — `address_from_pubkey()` returns BLAKE3 hash (irreversible), so `PublicKey::from_bytes(validator_address)` always failed. Fixed by adding `verify_and_route_batch_with_pubkeys()` that threads raw Ed25519 pubkeys from signed envelopes. See SEC-ATT-PUBKEY-001.

---

## Phase 3: Contract + Cross-VM + Edge Case Tests (Tasks 11–14) ✓

| # | Task | Owner | Status |
|---|------|-------|--------|
| 11 | `evm_deploy_and_call_e2e`: Deferred — existing `contract_deploy_and_call_end_to_end` covers this path | smart-contract-engineer | DEFERRED |
| 12 | `duplicate_model_registration_e2e`: Register same model_id twice — verify second tx fails, first model unchanged | ai-integration-engineer | DONE |
| 13 | `deregister_model_with_pending_tasks_blocked`: RegisterModel → PostTask → DeregisterModel — verify deregister fails (pending tasks exist) | ai-integration-engineer | DONE |
| 14 | `batch_mixed_tx_types_e2e`: Single batch with Transfer + ContractDeploy + RegisterModel — verify all execute correctly, receipts match, state consistent | node-engineer | DONE |

**Exit criteria:** 3 new e2e tests pass + 1 deferred (covered by existing test), edge cases covered ✓

---

## Phase 4: Security Review & Documentation (Tasks 15–16) ✓

| # | Task | Owner | Status |
|---|------|-------|--------|
| 15 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, all 161 tests pass | security-engineer | DONE |
| 16 | BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan updated. Memory updated | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED findings, clippy/fmt/test clean, all docs updated ✓

---

## Security Findings

| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| SEC-ATT-PUBKEY-001 | CRITICAL | Attestation sig verification used BLAKE3 hash as Ed25519 key — always invalid | FIXED |

---

## Dependencies

- All phases build on existing pipeline infrastructure from Sprints 021-023 (AI market) and 011-012 (fee market)
- Phase 2 requires BLS keypair generation (aztibase_core::BlsKeypair)
- Phase 2 requires Ed25519 signing for attestations

## Risks

- Windows LNK1318 linker PDB limit may surface with increased test count — `cargo clean` if needed (did not occur)
- Full workspace test run may hit disk space limits — run per-crate if needed (managed with CARGO_INCREMENTAL=0)

## Retrospective

**What went well:**
- E2e tests uncovered a real production bug (SEC-ATT-PUBKEY-001) — attestation signatures could never verify through signed tx flow
- All 13 new tests pass alongside 148 existing tests (161 total)
- Bug fix is minimal and surgical — 2 new functions + 3 lines changed in pipeline

**What could improve:**
- The attestation bug should have been caught in Sprint 022 when SubmitAttestation was first implemented
- Need a testing pattern that always routes through signed tx envelopes, not direct TxKind construction

**Metrics:**
- Tests: 161 (148 existing + 13 new)
- Clippy warnings: 0
- Security findings: 1 CRITICAL (fixed)
- Lines changed: ~600 (tests) + ~50 (bug fix)
