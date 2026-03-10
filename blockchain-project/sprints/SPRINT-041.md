# Sprint 041 — Gas Price Enforcement & Transaction Validation (M9-S2)

**Status:** COMPLETE
**Started:** 2026-03-10
**Completed:** 2026-03-10
**Engineer(s):** blockchain-architect, smart-contract-engineer, security-engineer
**ADR:** None (enforcing existing design — ADR-006 escrow already wired)

---

## Goal

Close the gas-price bypass vulnerability: transactions with `gas_price=0` currently skip escrow and execute for free, and `validate_gas_price()` is never enforced in the pipeline. This sprint adds mandatory gas price validation, rejects sub-base-fee transactions, and adds gas cost constants for all 19 TxKind variants.

## Security Motivation

- **Current gap**: Pipeline line 369-372 allows `gas_price=0` txs to skip escrow entirely, executing for free
- **Current gap**: `BaseFeeCalculator::validate_gas_price()` exists but is never called in execute_batch
- **Current gap**: `estimate_gas()` only covers 7 of 19 TxKind variants
- **Impact**: Fee-free DoS attack vector — spam the network with gas_price=0 transactions

## Scope

~3 files, focused changes. Pipeline gas enforcement + fee.rs gas estimates + tests.

---

## Phase 1: Gas Price Enforcement in Pipeline

### Task 1.1 — Reject sub-base-fee transactions
**File:** `crates/aztibase-node/src/pipeline.rs`
- Before escrow phase, validate every tx's gas_price >= current base_fee
- Txs with gas_price < base_fee get a failure receipt and are excluded
- Remove the gas_price==0 bypass (all txs must pay fees)
- **Status:** DONE

### Task 1.2 — Update estimate_gas for all 19 TxKind variants
**File:** `crates/aztibase-execution/src/fee.rs`
- Add gas estimates for: RegisterModel (100K), PostTask (42K), SubmitAttestation (50K), CommitCompute (75K), DeregisterCompute (50K), DeregisterModel (60K), CreateProposal (100K), CastVote (40K), Stake (60K), Unstake (60K), Delegate (60K), Undelegate (60K)
- Already covered: Transfer (21K), ContractDeploy (100K), ContractCall (50K), EvmDeploy (100K), EvmCall (50K), AiInfer (100K), CreateAgent (53K)
- **Status:** DONE

---

## Phase 2: Default Gas Price Assignment

### Task 2.1 — TxKind::gas_price() default for zero-price txs
**File:** `crates/aztibase-execution/src/routing.rs`
- Ensure gas_price field is non-zero in TxKind constructors
- Add gas_price() and gas_limit() accessor methods if not present
- **Status:** DONE

---

## Phase 3: Tests

### Task 3.1 — Gas enforcement tests
**File:** `crates/aztibase-node/src/pipeline.rs` (test module) or `integration.rs`
- Test: tx with gas_price=0 is rejected with error
- Test: tx with gas_price < base_fee is rejected
- Test: tx with gas_price >= base_fee passes escrow
- Test: all 19 TxKind variants have gas estimates
- **Status:** DONE

### Task 3.2 — cargo clippy + fmt + test
- 0 clippy warnings, fmt clean, all tests pass
- **Status:** DONE

---

## Phase 4: Documentation

### Task 4.1 — Doc updates
- BUILD_LOG.md, CHANGELOG.md, STATUS.md, MEMORY.md
- **Status:** DONE

---

## Exit Criteria

- [x] No transaction can execute with gas_price=0
- [x] All transactions validated against current base_fee
- [x] estimate_gas covers all 19 TxKind variants
- [x] No fee-free execution path exists
- [x] cargo clippy: 0 warnings
- [x] cargo fmt: clean
- [x] All 766 tests pass
- [x] Security: 0 ELEVATED, 0 MEDIUM

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Breaking existing tests that use gas_price=0 | MEDIUM | Update test helpers to provide valid gas_price |
| Genesis/system txs need free execution | LOW | System txs (if any) can use a bypass flag, but currently none exist |
