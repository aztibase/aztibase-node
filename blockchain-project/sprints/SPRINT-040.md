# Sprint 040 — u64→u128 Balance Migration (M9-S1)

**Status:** COMPLETE
**Started:** 2026-03-10
**Completed:** 2026-03-10
**Engineer(s):** blockchain-architect, smart-contract-engineer, node-engineer, security-engineer
**ADR:** ADR-014 (u128 tokenomics with deferred u64→u128 balance migration)

---

## Goal

Migrate all monetary values (balance, stake, fee, value, amount) from u64 to u128 across the entire codebase. This is the single biggest pre-M9 blocker — u64 maxes at ~18.4 AZTB which is far below the 1B hard cap. M8 is formally CLOSED with this sprint marking the start of M9 prep.

## Prerequisites Met

- ADR-014 accepted: u128 tokenomics types, deferred migration plan
- tokenomics.rs already uses u128 for emission/vesting/APY
- All M8 subsystems wired and tested (770+ tests)
- Storage uses postcard — u128 serialization supported natively

## Scope

~18 files, ~350 functions, ~80 test functions across 6 crates.

---

## Phase 1: Core Type Changes (Tier 1)

### Task 1.1 — Account.balance u64→u128
**File:** `crates/aztibase-execution/src/state.rs`
- Change `Account { balance: u64 }` → `Account { balance: u128 }`
- Update `balance()`, `set_balance()`, all arithmetic
- **Status:** DONE

### Task 1.2 — TransferTx.value u64→u128
**File:** `crates/aztibase-core/src/types.rs`
- Change `TransferTx { value: u64 }` → `TransferTx { value: u128 }`
- **Status:** DONE

### Task 1.3 — FeeEscrow u64→u128
**File:** `crates/aztibase-execution/src/fee.rs`
- Change `max_fee`, `gas_price`, `gas_limit` to u128
- Update `escrow_fee()`, `refund_unused()`, `collect_fees()`
- **Status:** DONE

### Task 1.4 — TxKind value/amount/stake fields u64→u128
**File:** `crates/aztibase-execution/src/routing.rs`
- Update Transfer, Stake, Unstake, Delegate, Undelegate, CommitCompute variants
- **Status:** DONE

### Task 1.5 — Staking types u64→u128
**File:** `crates/aztibase-execution/src/staking.rs`
- ValidatorStake, Delegation, UnbondingEntry, SlashRecord amount fields
- All methods: register_validator, add_stake, delegate, slash, rewards
- **Status:** DONE

---

## Phase 2: Consensus + Storage Layer

### Task 2.1 — ValidatorRecord.stake u64→u128
**File:** `crates/aztibase-consensus/src/validator.rs`
- Change stake field, `add()`, `total_stake()`
- **Status:** DONE

### Task 2.2 — ComputeProvider.committed_stake u64→u128
**File:** `crates/aztibase-consensus/src/pouw.rs`
- Change committed_stake, total_committed_stake()
- **Status:** DONE

### Task 2.3 — AccountRecord u64→u128 in persist.rs
**File:** `crates/aztibase-execution/src/persist.rs`
- Change AccountRecord.balance, all load/store functions
- postcard handles u128 natively — no format migration needed for new data
- **Status:** DONE

### Task 2.4 — Snapshot format u64→u128
**File:** `crates/aztibase-execution/src/snapshot.rs`
- AccountEntry.balance field
- **Status:** DONE

### Task 2.5 — Genesis types u64→u128
**File:** `crates/aztibase-node/src/genesis.rs`
- ValidatorEntry.stake, AccountEntry.balance
- **Status:** DONE

---

## Phase 3: Pipeline + Execution + Supporting

### Task 3.1 — Pipeline execute_batch u64→u128
**File:** `crates/aztibase-node/src/pipeline.rs`
- All fee/balance operations, escrow, rewards, refunds
- **Status:** DONE

### Task 3.2 — Block-STM read_balance u64→u128
**File:** `crates/aztibase-execution/src/block_stm.rs`
- read_balance() return type, all concurrency paths
- **Status:** DONE

### Task 3.3 — EVM balance interop u64→u128
**File:** `crates/aztibase-execution/src/evm.rs`
- read_balance, set_balance through CacheDB adapter
- NOTE: revm uses U256 internally — u128 maps cleanly
- **Status:** DONE

### Task 3.4 — Parallel executor u64→u128
**File:** `crates/aztibase-execution/src/parallel.rs`
- TransferTx value field
- **Status:** DONE

### Task 3.5 — Light node + anomaly scorer u64→u128
**Files:** `crates/aztibase-storage/src/light.rs`, `crates/aztibase-runtime/src/anomaly.rs`
- LocalWalletState.balance, TxFeatures value/gas fields
- **Status:** DONE

---

## Phase 4: RPC + Tests + Security Review

### Task 4.1 — RPC response format check
**File:** `crates/aztibase-rpc/src/server.rs`
- Balance already returned as hex string `0x{balance:x}` — u128 hex works
- Verify staking RPC handlers handle u128
- **Status:** DONE

### Task 4.2 — Prometheus metrics u128 handling
**File:** `crates/aztibase-rpc/src/metrics.rs`
- u128→i64 cast for Prometheus gauge — use saturating cast
- **Status:** DONE

### Task 4.3 — Test suite update (~80 test functions)
**Files:** All test modules
- Update hardcoded u64 literals, add u128-range tests
- **Status:** DONE

### Task 4.4 — Security review + cargo clippy/fmt/test
- Overflow safety audit (u128 arithmetic)
- Serialization roundtrip verification
- 0 clippy warnings, fmt clean, all tests pass
- **Status:** DONE

### Task 4.5 — Documentation updates
- BUILD_LOG.md, CHANGELOG.md, STATUS.md
- **Status:** DONE

---

## Exit Criteria

- [x] Account.balance is u128 everywhere
- [x] All monetary fields (stake, fee, value, amount) are u128
- [x] Storage serialization/deserialization works with u128
- [x] RPC responses handle u128 values correctly
- [x] All 766 tests pass with u128 types
- [x] No u64 truncation in any monetary path
- [x] cargo clippy: 0 warnings
- [x] cargo fmt: clean
- [x] Security: 0 ELEVATED, 0 MEDIUM

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Storage format break for existing data | MEDIUM | postcard handles u128 natively; no existing mainnet data |
| Type cascade — one change breaks 100+ sites | HIGH | Work in tiers: core types first, then consumers |
| EVM U256↔u128 conversion edge cases | LOW | revm already handles U256; u128 fits cleanly |
| Prometheus i64 overflow for large balances | LOW | Saturating cast; testnet values won't exceed i64 |
