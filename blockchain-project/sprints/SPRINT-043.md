# Sprint 043 — Autonomous AI Agent Transactions (M9-S4)

**Status:** COMPLETE
**Started:** 2026-03-10
**Completed:** 2026-03-10
**Engineer(s):** blockchain-architect, smart-contract-engineer, ai-integration-engineer, security-engineer

---

## Goal

Enable AI agents to execute transactions autonomously within human-defined policy constraints. Agents created via CreateAgent can now operate within spending limits, allowed tx types, and expiry windows — enabling the "agent economy" without unbounded spending risk.

## Scope

~4 files, focused changes. AgentPolicy store + 2 new TxKinds + pipeline execution + tests.

---

## Phase 1: AgentPolicy Store & Types

### Task 1.1 — AgentPolicy types
**File:** `crates/aztibase-execution/src/agent.rs` (new)
- `AgentPolicy`: per_tx_limit (u128), per_epoch_limit (u128), allowed_tx_kinds (Vec<u8>), expiry_epoch (u64), owner (Address)
- `AgentPolicyStore`: BTreeMap<Address, AgentPolicy> with get/set/remove
- `AgentSpendTracker`: tracks per-epoch spending per agent (epoch, total_spent)
- `AgentError` enum: NotAnAgent, PolicyNotSet, SpendingCapExceeded, TxKindNotAllowed, PolicyExpired, NotOwner
- **Status:** DONE

### Task 1.2 — Wire into execution lib.rs
**File:** `crates/aztibase-execution/src/lib.rs`
- pub mod agent, re-export key types
- **Status:** DONE

---

## Phase 2: New TxKinds

### Task 2.1 — TxKind::SetAgentPolicy (0x14)
**File:** `crates/aztibase-execution/src/routing.rs`
- Fields: owner (Address), agent (Address), per_tx_limit (u128), per_epoch_limit (u128), allowed_tx_kinds (Vec<u8>), expiry_epoch (u64), nonce (u64), gas_price (u64)
- Only the agent's creator (owner) can set/update the policy
- **Status:** DONE

### Task 2.2 — TxKind::AgentExecute (0x15)
**File:** `crates/aztibase-execution/src/routing.rs`
- Fields: agent (Address), inner_tx_kind (u8), to (Address), value (u128), data (Vec<u8>), nonce (u64), gas_price (u64)
- Agent-initiated transaction — validated against AgentPolicy constraints
- **Status:** DONE

### Task 2.3 — Update encode/decode, nonce(), gas_price(), gas_limit(), sender()
**File:** `crates/aztibase-execution/src/routing.rs`
- Add prefix constants, match arms for both new variants
- gas_limit: SetAgentPolicy=60_000, AgentExecute=80_000
- **Status:** DONE

### Task 2.4 — Update estimate_gas in fee.rs
**File:** `crates/aztibase-execution/src/fee.rs`
- 0x14 → 60_000 (SetAgentPolicy), 0x15 → 80_000 (AgentExecute)
- **Status:** DONE

---

## Phase 3: Pipeline Execution

### Task 3.1 — SetAgentPolicy execution
**File:** `crates/aztibase-node/src/pipeline.rs`
- Verify sender == agent owner (from AccountState)
- Verify agent address is AccountType::AIAgent
- Store AgentPolicy in AgentPolicyStore
- **Status:** DONE

### Task 3.2 — AgentExecute execution
**File:** `crates/aztibase-node/src/pipeline.rs`
- Verify agent address is AccountType::AIAgent
- Load AgentPolicy, check: not expired, tx_kind allowed, per_tx_limit, per_epoch_limit
- Track spending via AgentSpendTracker
- Execute as Transfer from agent's balance (initially only Transfer support)
- **Status:** DONE

---

## Phase 4: Tests

### Task 4.1 — Unit tests
- Test: AgentPolicy store get/set/remove
- Test: AgentSpendTracker epoch tracking and reset
- Test: SetAgentPolicy by non-owner rejected
- Test: SetAgentPolicy on non-agent rejected
- Test: AgentExecute within policy succeeds
- Test: AgentExecute exceeds per_tx_limit rejected
- Test: AgentExecute exceeds per_epoch_limit rejected
- Test: AgentExecute with disallowed tx_kind rejected
- Test: AgentExecute with expired policy rejected
- Test: encode/decode roundtrip for both new TxKinds
- **Status:** DONE

### Task 4.2 — cargo clippy + fmt + test
- **Status:** DONE

---

## Phase 5: Documentation

### Task 5.1 — Doc updates
- BUILD_LOG.md, CHANGELOG.md, STATUS.md, MEMORY.md
- **Status:** DONE

---

## Exit Criteria

- [x] AgentPolicy store with spend tracking
- [x] SetAgentPolicy tx enforces owner-only access
- [x] AgentExecute validates policy constraints (limits, allowed kinds, expiry)
- [x] All tests pass
- [x] cargo clippy: 0 warnings, cargo fmt: clean
