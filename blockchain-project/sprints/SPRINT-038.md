# Sprint 038 — Validator Staking, Delegation & Slashing (M8-S13)

**Status:** PLANNED
**Started:** —
**Completed:** —
**Engineer(s):** tokenomics-engineer, consensus-engineer, node-engineer, security-engineer

---

## Goal

Wire validator staking into the execution layer: self-stake, delegation, unbonding queue, basic slashing, and epoch reward distribution. After this sprint, validators join/leave the active set dynamically based on stake thresholds, delegators earn proportional rewards, and misbehavior triggers slashing penalties.

## Prerequisites Met

- ValidatorSet with stake-weighted leader selection (consensus crate)
- ComputeCommitment bond pattern (stake lock/refund via TxKind)
- Tokenomics: EmissionSchedule, StakingAPY, validator_epoch_reward, UNBONDING_ROUNDS, DEFAULT_MIN_STAKE, MAX_STAKE_CAP constants
- Governance: ChainParams for runtime-mutable thresholds
- Account balances (u64, pre-ADR-014 migration)

## Design Decisions (Pre-Sprint)

1. **StakingStore** — separate in-memory store (like GovernanceStore), not Account.storage. Validators and delegators tracked in dedicated structs for efficient queries.
2. **Single delegation** — each delegator can delegate to one validator at a time. Multi-delegation deferred to M9.
3. **Unbonding queue** — per-account queue with UNBONDING_ROUNDS delay. Processed in pipeline after each batch.
4. **Slashing** — percentage-based penalty applied to validator + delegators proportionally. Two offense types: equivocation (severe) and downtime (mild).
5. **Reward distribution** — epoch-based. Pipeline calls advance_epoch() at epoch boundary, distributes to active validators + delegators proportionally.

---

## Phase 1: Staking Store & Self-Stake (4 tasks)

### Task 1.1 — StakingStore core types
**File:** `crates/aztibase-execution/src/staking.rs` (NEW)
- `ValidatorStake`: validator_id, self_stake (u64), total_delegated (u64), active (bool), registered_round (u64)
- `Delegation`: delegator address ([u8;32]), validator_id, amount (u64), round_delegated (u64)
- `UnbondingEntry`: owner ([u8;32]), amount (u64), available_round (u64)
- `StakingStore`: validators (HashMap), delegations (HashMap<[u8;32], Delegation>), unbonding_queue (VecDeque<UnbondingEntry>)
- Methods: register_validator, get_validator, remove_validator, effective_stake (self + delegated), is_active (effective_stake >= MIN_STAKE), active_validators()
- `SlashRecord`: validator_id, offense_type, slash_bps, round, amount_slashed
- Constants: EQUIVOCATION_SLASH_BPS = 1000 (10%), DOWNTIME_SLASH_BPS = 50 (0.5%), DOWNTIME_THRESHOLD_ROUNDS = 1000
- **Tests:** register/query/remove validator, effective_stake calculation, active threshold check

### Task 1.2 — TxKind::Stake (0x10) and TxKind::Unstake (0x11)
**File:** `crates/aztibase-execution/src/routing.rs`
- TxKind::Stake: sender stakes amount, becomes validator candidate
- TxKind::Unstake: sender begins unbonding, enters unbonding queue
- Encode/decode: Stake = (amount: u64), Unstake = (amount: u64)
- PREFIX_STAKE = 0x10, PREFIX_UNSTAKE = 0x11
- All match arms updated (nonce, gas_price, sender, gas_limit, encode, decode)
- **Tests:** roundtrip encode/decode for both variants

### Task 1.3 — Pipeline: Stake execution
**File:** `crates/aztibase-node/src/pipeline.rs`
- Sort Stake/Unstake txs in batch ordering
- Stake handler: validate nonce, check balance >= amount + gas, check amount >= DEFAULT_MIN_STAKE (first stake) or additive (top-up), check effective_stake + amount <= MAX_STAKE_CAP, deduct from balance, add to self_stake in StakingStore
- Unstake handler: validate nonce, check self_stake >= amount, check remaining effective_stake >= DEFAULT_MIN_STAKE or full exit, reduce self_stake, push UnbondingEntry (available_round = current_round + UNBONDING_ROUNDS)
- compute_tx_hash for new variants
- **Tests:** stake success, stake insufficient balance, stake exceeds cap, unstake partial, unstake full exit, unstake below minimum

### Task 1.4 — Unbonding queue processing
**File:** `crates/aztibase-node/src/pipeline.rs` + `crates/aztibase-execution/src/staking.rs`
- StakingStore::process_unbonding(current_round): iterate unbonding_queue, release matured entries, return Vec<(address, amount)>
- Pipeline: after batch persistence, call process_unbonding(), credit released amounts to account balances
- Cap: MAX_UNBONDING_ENTRIES = 10_000 (reject new Unstake if full)
- **Tests:** unbonding matures after UNBONDING_ROUNDS, early withdrawal rejected, queue overflow rejected

---

## Phase 2: Delegation & Rewards (4 tasks)

### Task 2.1 — TxKind::Delegate (0x12) and TxKind::Undelegate (0x13)
**File:** `crates/aztibase-execution/src/routing.rs`
- Delegate: sender delegates amount to validator_id
- Undelegate: sender begins undelegation (enters same unbonding queue)
- Encode/decode: Delegate = (validator_id: [u8;32], amount: u64), Undelegate = ()
- PREFIX_DELEGATE = 0x12, PREFIX_UNDELEGATE = 0x13
- **Tests:** roundtrip encode/decode for both variants

### Task 2.2 — Pipeline: Delegation execution
**File:** `crates/aztibase-node/src/pipeline.rs`
- Delegate handler: validate nonce, check validator exists + active, check no existing delegation (single delegation rule), check balance >= amount + gas, deduct from balance, add Delegation to StakingStore, increase validator's total_delegated
- Undelegate handler: validate nonce, find delegation, remove from StakingStore, decrease validator's total_delegated, push UnbondingEntry
- **Tests:** delegate success, delegate to non-existent validator, double delegation rejected, undelegate with no delegation, undelegate enters unbonding

### Task 2.3 — Epoch reward distribution
**File:** `crates/aztibase-node/src/pipeline.rs` + `crates/aztibase-execution/src/staking.rs`
- StakingStore::distribute_epoch_rewards(validator_pool: u64, round: u64): for each active validator, compute reward via validator_epoch_reward() proportional to effective_stake, split between self_stake and delegators proportionally
- Pipeline: at epoch boundary (configurable, e.g., every 10_000 rounds), call EmissionTracker::advance_epoch() then distribute_epoch_rewards()
- Rewards credited directly to account balances (auto-compound for self-stake, liquid for delegators)
- Validator commission: 10% default (hardcoded for now, governance-mutable in future sprint)
- **Tests:** single validator reward, multi-validator proportional split, delegator reward with commission, zero-participation no reward

### Task 2.4 — Active validator set sync
**File:** `crates/aztibase-node/src/pipeline.rs`
- After epoch boundary: rebuild ValidatorSet from StakingStore::active_validators()
- Only validators with effective_stake >= DEFAULT_MIN_STAKE are in the active set
- ValidatorSet mutation: add new validators, remove exited ones, update stakes
- Pipeline holds Arc<RwLock<ValidatorSet>> for consensus engine sync
- **Tests:** validator activated on reaching threshold, validator removed on full unstake, delegator pushes validator above threshold

---

## Phase 3: Slashing & RPC (4 tasks)

### Task 3.1 — Equivocation slashing
**File:** `crates/aztibase-execution/src/staking.rs` + `crates/aztibase-node/src/pipeline.rs`
- StakingStore::slash_validator(validator_id, slash_bps, offense_type, round): reduce self_stake by percentage, reduce each delegator proportionally, record SlashRecord
- Wire into consensus: when ConsensusEngine detects equivocation (already detected in Sprint 014), emit a SlashEvent
- Pipeline: on SlashEvent, call slash_validator(EQUIVOCATION_SLASH_BPS=1000)
- Slashed amount burned (removed from circulation, not redistributed)
- **Tests:** equivocation slash reduces self_stake 10%, delegator slashed proportionally, slash caps at available stake

### Task 3.2 — Downtime slashing
**File:** `crates/aztibase-execution/src/staking.rs` + `crates/aztibase-node/src/pipeline.rs`
- Track validator participation: rounds_participated / rounds_expected per epoch
- If participation < 50% over DOWNTIME_THRESHOLD_ROUNDS, apply DOWNTIME_SLASH_BPS (0.5%)
- Computed at epoch boundary alongside reward distribution
- Mild: no forced exit, just balance reduction
- **Tests:** active validator no slash, low-participation validator slashed 0.5%, boundary (50% exactly) no slash

### Task 3.3 — Staking RPC endpoints
**File:** `crates/aztibase-rpc/src/server.rs`
- `aztb_getValidatorStake(validator_id)`: self_stake, total_delegated, effective_stake, active, registered_round, slash_history
- `aztb_getDelegation(address)`: validator_id, amount, round_delegated (or null if none)
- `aztb_getActiveValidators()`: list of active validators with stakes, sorted by effective_stake descending
- `aztb_getUnbondingStatus(address)`: pending unbonding entries with amounts + available_round
- Wire StakingStore into RpcState via Arc<RwLock<StakingStore>>
- **Tests:** 4 RPC handler tests (one per endpoint)

### Task 3.4 — ChainParams: staking governance params
**File:** `crates/aztibase-execution/src/chain_params.rs`
- Add 3 new governance-controllable params: min_validator_stake, max_stake_cap, validator_commission_bps
- StakingStore reads from ChainParams at stake/delegate time
- Bounds: min_validator_stake (10_000 to 500_000), max_stake_cap (1M to 100M), validator_commission_bps (0 to 3000)
- **Tests:** param bounds validation, staking reads dynamic min_stake

---

## Phase 4: Security Review & Docs (4 tasks)

### Task 4.1 — Security review
**Engineer:** security-engineer
- Review all new staking/slashing code for:
  - Stake overflow (u64 limits vs MAX_STAKE_CAP)
  - Unbonding queue DoS (MAX_UNBONDING_ENTRIES cap)
  - Slashing re-entrancy (slash during unbonding)
  - Delegation to self (should be rejected — use Stake instead)
  - Reward rounding errors (floor division, remainder handling)
  - Double-slash prevention (same offense, same round)
- Run cargo clippy (0 warnings), cargo fmt --check (clean)
- Run full test suite

### Task 4.2 — cargo clippy + fmt + test
- Zero clippy warnings
- fmt clean
- All tests pass (target: ~770+ total)

### Task 4.3 — Documentation updates
- BUILD_LOG.md entry
- CHANGELOG.md entry
- STATUS.md update
- Sprint plan: mark all tasks DONE

### Task 4.4 — Sprint retrospective
- What went well / what to improve
- Test count delta
- Security findings summary
- Next sprint candidates

---

## Exit Criteria

- [ ] StakingStore with validator self-stake, delegation, unbonding queue
- [ ] 4 new TxKinds: Stake (0x10), Unstake (0x11), Delegate (0x12), Undelegate (0x13)
- [ ] Pipeline executes all 4 staking tx types with balance checks
- [ ] Unbonding queue with UNBONDING_ROUNDS delay, processed after each batch
- [ ] Epoch reward distribution wired (EmissionTracker → StakingStore → balances)
- [ ] Active validator set rebuilt at epoch boundaries from stake thresholds
- [ ] Equivocation slashing (10%) wired from consensus detection
- [ ] Downtime slashing (0.5%) computed at epoch boundary
- [ ] 4 new RPC endpoints for staking queries
- [ ] 3 new governance-controllable staking params
- [ ] Security review: 0 ELEVATED, 0 MEDIUM
- [ ] ~40+ new tests, clippy 0 warnings, fmt clean

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| u64 stake overflow with large delegations | MEDIUM | MAX_STAKE_CAP enforced, checked arithmetic |
| Unbonding queue memory growth | LOW | MAX_UNBONDING_ENTRIES = 10,000 cap |
| Slash during unbonding period | MEDIUM | Unbonding entries also slashed proportionally |
| Reward rounding favors last validator | LOW | Floor division, remainder to treasury |
| Validator set churn at epoch boundary | LOW | Hysteresis: exit only on full unstake, not dip below threshold |
