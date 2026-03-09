# Sprint 037 — M8 Sprint 12: Token Supply, Emission & Vesting

**Goal:** Implement token supply hard cap, disinflationary emission schedule, genesis allocations with vesting, and epoch-based reward distribution — the economic foundation for the Aztibase testnet.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

**Design Reference:** MASTER_DESIGN.md Section 3.1–3.4

**Key Constraint:** Current balances use `u64`. Tokenomics types use `u128` internally for precision. Full balance migration to `u128` with 18-decimal base units is deferred to a dedicated migration sprint (pre-M9). ADR-014 documents this decision.

---

## Phase 1: Token Supply & Emission Schedule (Tasks 1–5)

Core economic constants and the disinflationary emission calculator.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Create `tokenomics.rs` module in aztibase-execution with `TokenSupply` constants (1B cap, 400M genesis, denomination) | tokenomics-engineer | DONE |
| 2 | Implement `EmissionSchedule` — 2-year halving curve, `emission_for_year(y)`, `emission_per_epoch(epoch, epoch_duration)` | tokenomics-engineer | DONE |
| 3 | Implement `EmissionDistribution` — split epoch emission into 4 pools (70% validator, 15% PoUW, 10% treasury, 5% insurance) | tokenomics-engineer | DONE |
| 4 | Implement `StakingAPY` — piecewise linear curve (12% at <20% ratio → 3% floor at >70%), `calculate_apy(staking_ratio_bps)` | tokenomics-engineer | DONE |
| 5 | Unit tests: emission curve over 10 years, total never exceeds 600M emitted, APY curve boundary values, distribution split sums to 100% | tokenomics-engineer | DONE |

**Exit criteria:** `EmissionSchedule` produces correct values per MASTER_DESIGN.md Section 3.3.1 table. APY curve matches 3.4.1 formula. ✓

---

## Phase 2: Vesting Schedules & Genesis Allocations (Tasks 6–9)

Genesis allocation types and vesting mechanics per Section 3.2.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 6 | Implement `VestingSchedule` — cliff + linear unlock, `vested_amount(total, current_round, start_round)` | tokenomics-engineer | DONE |
| 7 | Implement `GenesisAllocation` — 8 allocation categories (Treasury, EcoDev, Team, Foundation, Airdrop, ValidatorBootstrap, AIFund, Liquidity) with amounts and vesting params | tokenomics-engineer | DONE |
| 8 | Implement `genesis_allocations()` returning the full allocation table, `total_genesis_mint()` validation (must equal 400M) | tokenomics-engineer | DONE |
| 9 | Unit tests: each allocation vests correctly, cliff blocks early withdrawal, full vest at schedule end, genesis total = 400M, anti-concentration (<6% per entity) | tokenomics-engineer | DONE |

**Exit criteria:** All 8 genesis allocations match MASTER_DESIGN.md Section 3.2.1 table. Vesting math is deterministic (floor division only). ✓

---

## Phase 3: Epoch Reward Calculator & RPC (Tasks 10–13)

Wire emission into reward distribution and expose via RPC.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 10 | Implement `EpochRewardCalculator` — compute per-validator reward share based on participation ratio, distribute from 70% validator pool | tokenomics-engineer | DONE |
| 11 | Wire `EmissionSchedule` + `EpochRewardCalculator` into pipeline — track current_epoch, total_emitted, enforce hard cap | node-engineer | DONE |
| 12 | Add `aztb_getEmissionInfo` RPC — returns current_epoch, emission_rate, total_emitted, total_supply, staking_ratio | node-engineer | DONE |
| 13 | Add `aztb_getVestingStatus(allocation)` RPC — returns total, vested, locked, cliff_round, end_round for a genesis allocation | node-engineer | DONE |

**Exit criteria:** Emission info RPC returns correct values. Vesting RPC shows locked vs available for each allocation. ✓

---

## Phase 4: Security Review & Docs (Tasks 14–16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 14 | Security review: overflow in u128 arithmetic, emission cap enforcement, vesting cannot be bypassed, no floating-point | security-engineer | DONE |
| 15 | ADR-014: u64→u128 balance migration strategy (deferred, document rationale) | blockchain-architect | DONE |
| 16 | cargo clippy (0 warnings), cargo fmt --check, cargo test, BUILD_LOG + STATUS + CHANGELOG | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED, 0 MEDIUM security findings. All docs updated. ✓

---

## Notes

- All tokenomics math uses `u128` with floor division for determinism
- No `f32`/`f64` anywhere in tokenomics code — all fixed-point integer arithmetic
- Emission amounts are in "base units" (conceptually 10^18 per token, but stored as u128)
- The current u64 balance system continues to work; tokenomics types track supply/emission separately
- Epoch = a fixed number of committed rounds (governance-adjustable, default TBD)
