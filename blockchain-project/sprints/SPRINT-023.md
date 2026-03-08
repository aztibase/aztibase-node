# Sprint 023 — M6 Closure: Deregistration, Task Assignment, Compute Validation

**Milestone:** M6 (AI Compute Market + PoUW) — FINAL SPRINT
**Start Date:** 2026-03-07
**Status:** COMPLETE

---

## Goal

Close out M6 by implementing the three remaining gaps: compute provider deregistration with stake refund (resolves SEC-COMMIT-OVERWRITE), automatic task assignment via TaskAssigner, and compute commitment validation against the model registry. On completion, the AI compute market is end-to-end functional: register model -> commit compute -> post task -> assign validator -> submit attestation -> settle reward -> deregister.

---

## Phase 1: DeregisterCompute + Stake Refund (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `TxKind::DeregisterCompute` (0x0C) variant in routing.rs with encode/decode + roundtrip test | smart-contract-engineer | DONE |
| 2 | Pipeline execution: lookup existing commitment in ComputeCommitmentStore, refund committed_stake to validator balance, remove from store; fail if no commitment exists | node-engineer | DONE |
| 3 | `ComputeCommitmentStore::deregister(validator_id)` method returning Option<ComputeCommitment> for refund processing | consensus-engineer | DONE |
| 4 | RPC: `aztb_deregisterCompute` docs + 3 tests (happy path, no-commitment-fails, balance-restored) | node-engineer | DONE |

**Exit Criteria:** Validators can deregister and reclaim their staked funds. SEC-COMMIT-OVERWRITE resolved — overwrite now refunds prior stake.

---

## Phase 2: TaskAssigner Pipeline Wiring (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Wire `TaskAssigner::select_validator` into PostTask execution: after task insertion, select best validator from ComputeCommitmentStore candidates for that model | node-engineer | DONE |
| 6 | Add `assigned_validator: Option<Address>` to `InferenceTask`; populate on assignment; include in `aztb_getTaskStatus` response | node-engineer | DONE |
| 7 | SubmitAttestation: validate that attesting validator matches assigned_validator (if set), reject mismatched attestations | node-engineer | DONE |
| 8 | Remove `#[allow(dead_code)]` from task_pool module; 3 tests (assignment-selects-best, unassigned-if-no-candidates, attestation-rejects-wrong-validator) | node-engineer | DONE |

**Exit Criteria:** PostTask automatically assigns a validator via PoUW score. Only the assigned validator (or any, if none assigned) can submit attestations.

---

## Phase 3: Compute Commitment Validation + Model Deregistration (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | CommitCompute pipeline: validate that all `supported_models` in the commitment exist in ModelRegistry; reject if any model is unregistered | node-engineer | DONE |
| 10 | `TxKind::DeregisterModel` (0x0D) variant: owner-only deregistration, fail if tasks pending for that model | smart-contract-engineer | DONE |
| 11 | Pipeline execution for DeregisterModel: verify sender == model owner, check no pending tasks in TaskPool, call ModelRegistry::deregister | node-engineer | DONE |
| 12 | RPC: update `aztb_listModels` to include deregistered count; 3 tests (deregister-success, non-owner-fails, pending-tasks-blocks) | node-engineer | DONE |

**Exit Criteria:** Compute commitments validated against registry. Model owners can deregister models when no tasks are pending.

---

## Phase 4: Security Review + M6 Close (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: DeregisterCompute stake refund (double-refund, reentrancy), DeregisterModel (owner auth), TaskAssigner (score manipulation) | security-engineer | DONE |
| 14 | `cargo clippy` zero warnings + `cargo fmt --check` clean + all tests pass | node-engineer | DONE |
| 15 | Doc sync: BUILD_LOG, CHANGELOG, STATUS.md (M6 COMPLETE), DECISIONS.md if needed | docs-engineer | DONE |
| 16 | M6 exit assessment: verify all AI compute market flows are end-to-end functional, update STATUS.md milestone tracker | project-lead | DONE |

**Exit Criteria:** M6 closed. All security flags addressed. Full test suite green. Documentation current.

---

## Deferred (M7+)

- **Verkle IPA/KZG polynomial commitments**: BLAKE3 placeholder works for all current use cases. True polynomial commitments require substantial crypto library work (IPA prover/verifier, multi-scalar multiplication). Deferred to M7 security hardening phase where it can be done properly with cryptographic audit.
- **Multi-validator task assignment**: Current design assigns one best validator. Future sprints may support k-of-n assignment for redundancy.
- **Slashing for missed assignments**: Validators assigned but not attesting within deadline. Requires slashing mechanism (M7).

---

## Retrospective

**What went well:**
- Clean deregistration pattern: returning `Option<ComputeCommitment>` from `deregister()` enabled simple refund logic
- TaskAssigner integration was straightforward once `assigned_validator` field was added to InferenceTask
- SEC-COMMIT-OVERWRITE from Sprint 022 fully resolved by refunding prior stake on CommitCompute overwrite
- Model validation in CommitCompute catches invalid commitments early

**What to watch:**
- Pipeline file is growing large (~3000+ lines). Consider splitting into submodules in M7
- SlidingWindowPoUWScore scorer is instantiated per-PostTask — acceptable now but should be shared state for production

**Metrics:**
- 16/16 tasks completed
- 11 new tests added (555 total, 0 failures)
- 0 clippy warnings, fmt clean
- 0 new ELEVATED or MEDIUM security findings
- M6 milestone closed
