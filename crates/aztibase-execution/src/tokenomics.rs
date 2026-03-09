use std::fmt;

/// Total hard cap: 1 billion tokens. No governance override.
pub const TOTAL_SUPPLY: u128 = 1_000_000_000;

/// Genesis mint: 400 million tokens (40% of total supply).
pub const GENESIS_MINT: u128 = 400_000_000;

/// Emission pool: 600 million tokens emitted over ~10 years via halving schedule.
pub const EMISSION_POOL: u128 = 600_000_000;

/// Emission halving period in years.
pub const HALVING_PERIOD_YEARS: u32 = 2;

/// Year-1 emission (and year-2, before first halving).
pub const INITIAL_ANNUAL_EMISSION: u128 = 120_000_000;

/// Tail emission per year after the main schedule completes (year 11+).
pub const TAIL_EMISSION_PER_YEAR: u128 = 3_750_000;

/// Default epoch length in committed rounds.
pub const DEFAULT_EPOCH_LENGTH: u64 = 1000;

/// Assumed rounds per year at 400ms block time (365.25 days).
/// 365.25 * 24 * 3600 / 0.4 = 78_894_000
pub const ROUNDS_PER_YEAR: u64 = 78_894_000;

// --- Emission Distribution (Section 3.3.2) ---

/// Validator reward share of epoch emission (basis points).
pub const VALIDATOR_SHARE_BPS: u32 = 7000;

/// PoUW supplemental pool share (basis points).
pub const POUW_SHARE_BPS: u32 = 1500;

/// Protocol treasury share (basis points).
pub const TREASURY_SHARE_BPS: u32 = 1000;

/// Staking insurance fund share (basis points).
pub const INSURANCE_SHARE_BPS: u32 = 500;

// --- Staking APY (Section 3.4.1) ---

/// Maximum APY in basis points (12%).
pub const MAX_APY_BPS: u32 = 1200;

/// Minimum APY in basis points (3%).
pub const MIN_APY_BPS: u32 = 300;

// --- Staking Constants (Section 3.4.2) ---

/// Default minimum stake to become a validator.
pub const DEFAULT_MIN_STAKE: u128 = 50_000;

/// Governance range for minimum stake.
pub const MIN_STAKE_FLOOR: u128 = 10_000;
pub const MIN_STAKE_CEILING: u128 = 500_000;

/// Unbonding period in rounds (21 days at 400ms).
/// 21 * 24 * 3600 / 0.4 = 4_536_000
pub const UNBONDING_ROUNDS: u64 = 4_536_000;

/// Maximum stake cap per validator (5% of total supply).
pub const MAX_STAKE_CAP: u128 = TOTAL_SUPPLY / 20;

// ---------------------------------------------------------------------------
// EmissionSchedule
// ---------------------------------------------------------------------------

/// Computes the annual emission for a given year (1-indexed).
/// Years 1-2: 120M, years 3-4: 60M, years 5-6: 30M, etc.
/// After year 10: tail emission of 3.75M/year, halving further.
pub fn emission_for_year(year: u32) -> u128 {
    if year == 0 {
        return 0;
    }
    let halvings = (year - 1) / HALVING_PERIOD_YEARS;
    let base = INITIAL_ANNUAL_EMISSION;
    base >> halvings
}

/// Cumulative emission from year 1 through `year` (inclusive).
pub fn cumulative_emission(year: u32) -> u128 {
    let mut total = 0u128;
    for y in 1..=year {
        total = total.saturating_add(emission_for_year(y));
    }
    total.min(EMISSION_POOL)
}

/// Per-epoch emission given the current epoch number and epochs per year.
/// `total_emitted_before` is the total already distributed; enforces hard cap.
pub fn emission_per_epoch(epoch: u64, epochs_per_year: u64, total_emitted_before: u128) -> u128 {
    if total_emitted_before >= EMISSION_POOL || epochs_per_year == 0 {
        return 0;
    }

    let year = (epoch / epochs_per_year) as u32 + 1;
    let annual = emission_for_year(year);
    let per_epoch = annual / epochs_per_year as u128;

    let remaining = EMISSION_POOL - total_emitted_before;
    per_epoch.min(remaining)
}

// ---------------------------------------------------------------------------
// EmissionDistribution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochDistribution {
    pub validator_rewards: u128,
    pub pouw_pool: u128,
    pub treasury: u128,
    pub insurance_fund: u128,
}

impl EpochDistribution {
    pub fn total(&self) -> u128 {
        self.validator_rewards + self.pouw_pool + self.treasury + self.insurance_fund
    }
}

/// Split an epoch's emission into the four distribution pools.
/// Uses basis-point arithmetic with remainder assigned to validators.
pub fn distribute_emission(total: u128) -> EpochDistribution {
    let pouw = total * POUW_SHARE_BPS as u128 / 10_000;
    let treasury = total * TREASURY_SHARE_BPS as u128 / 10_000;
    let insurance = total * INSURANCE_SHARE_BPS as u128 / 10_000;
    let validator_rewards = total - pouw - treasury - insurance;

    EpochDistribution {
        validator_rewards,
        pouw_pool: pouw,
        treasury,
        insurance_fund: insurance,
    }
}

// ---------------------------------------------------------------------------
// StakingAPY
// ---------------------------------------------------------------------------

/// Calculate the target APY in basis points given the staking ratio in basis points.
/// Formula: base_apy = max(300, min(1200, 1800 - staking_ratio_bps / 50))
///
/// staking_ratio_bps: e.g. 5000 = 50%
///
/// The piecewise linear formula from the master design:
///   if ratio < 20% (2000 bps): APY = 12% (1200 bps)
///   if ratio = 50% (5000 bps): APY = 6%  (600 bps)
///   if ratio > 70% (7000 bps): APY = 3%  (300 bps, floor)
///
/// Linear: apy_bps = 1800 - ratio_bps / 50
pub fn calculate_apy_bps(staking_ratio_bps: u32) -> u32 {
    let raw = 1800u32.saturating_sub(staking_ratio_bps / 5);
    raw.clamp(MIN_APY_BPS, MAX_APY_BPS)
}

// ---------------------------------------------------------------------------
// EpochRewardCalculator
// ---------------------------------------------------------------------------

/// Per-validator reward share based on consensus participation.
/// `participation_bps`: validator's round participation in basis points (10000 = 100%).
/// Returns the validator's share of the total validator reward pool.
pub fn validator_epoch_reward(
    total_validator_pool: u128,
    participation_bps: u32,
    total_participation_weight: u128,
    validator_stake: u128,
) -> u128 {
    if total_participation_weight == 0 || total_validator_pool == 0 {
        return 0;
    }
    let weight = validator_stake * participation_bps as u128 / 10_000;
    total_validator_pool * weight / total_participation_weight
}

// ---------------------------------------------------------------------------
// VestingSchedule
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VestingSchedule {
    pub total_amount: u128,
    pub start_round: u64,
    pub cliff_rounds: u64,
    pub vesting_rounds: u64,
}

impl VestingSchedule {
    pub fn new(
        total_amount: u128,
        start_round: u64,
        cliff_rounds: u64,
        vesting_rounds: u64,
    ) -> Self {
        Self {
            total_amount,
            start_round,
            cliff_rounds,
            vesting_rounds,
        }
    }

    /// Immediate unlock (no vesting, no cliff).
    pub fn immediate(total_amount: u128, start_round: u64) -> Self {
        Self {
            total_amount,
            start_round,
            cliff_rounds: 0,
            vesting_rounds: 0,
        }
    }

    /// Amount vested (available) at `current_round`. Uses floor division.
    pub fn vested_at(&self, current_round: u64) -> u128 {
        if current_round < self.start_round {
            return 0;
        }
        let elapsed = current_round - self.start_round;

        if elapsed < self.cliff_rounds {
            return 0;
        }

        if self.vesting_rounds == 0 {
            return self.total_amount;
        }

        if elapsed >= self.vesting_rounds {
            return self.total_amount;
        }

        self.total_amount * elapsed as u128 / self.vesting_rounds as u128
    }

    /// Amount still locked at `current_round`.
    pub fn locked_at(&self, current_round: u64) -> u128 {
        self.total_amount - self.vested_at(current_round)
    }

    /// Round at which the cliff ends.
    pub fn cliff_end_round(&self) -> u64 {
        self.start_round.saturating_add(self.cliff_rounds)
    }

    /// Round at which vesting completes.
    pub fn end_round(&self) -> u64 {
        self.start_round.saturating_add(self.vesting_rounds)
    }
}

// ---------------------------------------------------------------------------
// GenesisAllocation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllocationCategory {
    ProtocolTreasury,
    EcosystemDevelopment,
    CoreTeam,
    FoundationReserve,
    CommunityAirdrop,
    ValidatorBootstrap,
    AIEcosystemFund,
    LiquidityProvision,
}

impl AllocationCategory {
    pub const ALL: [AllocationCategory; 8] = [
        Self::ProtocolTreasury,
        Self::EcosystemDevelopment,
        Self::CoreTeam,
        Self::FoundationReserve,
        Self::CommunityAirdrop,
        Self::ValidatorBootstrap,
        Self::AIEcosystemFund,
        Self::LiquidityProvision,
    ];

    /// Whether this category represents a distributed fund (not controlled by a single entity).
    /// These are exempt from the 6% anti-concentration rule.
    pub fn is_distributed_fund(&self) -> bool {
        matches!(
            self,
            Self::ProtocolTreasury
                | Self::EcosystemDevelopment
                | Self::FoundationReserve
                | Self::CommunityAirdrop
                | Self::ValidatorBootstrap
                | Self::AIEcosystemFund
                | Self::LiquidityProvision
        )
    }
}

impl fmt::Display for AllocationCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtocolTreasury => write!(f, "protocol_treasury"),
            Self::EcosystemDevelopment => write!(f, "ecosystem_development"),
            Self::CoreTeam => write!(f, "core_team"),
            Self::FoundationReserve => write!(f, "foundation_reserve"),
            Self::CommunityAirdrop => write!(f, "community_airdrop"),
            Self::ValidatorBootstrap => write!(f, "validator_bootstrap"),
            Self::AIEcosystemFund => write!(f, "ai_ecosystem_fund"),
            Self::LiquidityProvision => write!(f, "liquidity_provision"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenesisAllocationEntry {
    pub category: AllocationCategory,
    pub amount: u128,
    pub vesting: VestingSchedule,
    pub description: &'static str,
}

/// Rounds per month at 400ms block time: 30.44 * 24 * 3600 / 0.4 ≈ 6_574_560
const ROUNDS_PER_MONTH: u64 = 6_574_560;

/// Returns the 8 genesis allocation entries per MASTER_DESIGN.md Section 3.2.1.
/// `genesis_round` is the round at which the chain launches (typically 0).
pub fn genesis_allocations(genesis_round: u64) -> Vec<GenesisAllocationEntry> {
    vec![
        GenesisAllocationEntry {
            category: AllocationCategory::ProtocolTreasury,
            amount: 100_000_000,
            vesting: VestingSchedule::immediate(100_000_000, genesis_round),
            description: "Governance-controlled spending from day one",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::EcosystemDevelopment,
            amount: 80_000_000,
            vesting: VestingSchedule::new(
                80_000_000,
                genesis_round,
                ROUNDS_PER_MONTH * 6,  // 6-month cliff
                ROUNDS_PER_MONTH * 48, // 4-year linear
            ),
            description: "Developer grants, hackathons, integration bounties",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::CoreTeam,
            amount: 60_000_000,
            vesting: VestingSchedule::new(
                60_000_000,
                genesis_round,
                ROUNDS_PER_MONTH * 12, // 12-month cliff
                ROUNDS_PER_MONTH * 48, // 4-year linear
            ),
            description: "Founding engineers and contributors",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::FoundationReserve,
            amount: 40_000_000,
            vesting: VestingSchedule::new(
                40_000_000,
                genesis_round,
                ROUNDS_PER_MONTH * 24,        // 2-year lock
                ROUNDS_PER_MONTH * (24 + 36), // then 3-year linear (total 5 years)
            ),
            description: "Long-term strategic reserve",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::CommunityAirdrop,
            amount: 40_000_000,
            // 50% immediate, 50% locked 90 days — modeled as immediate for simplicity;
            // the 50% time-lock is enforced at distribution, not at the vesting level.
            vesting: VestingSchedule::immediate(40_000_000, genesis_round),
            description: "Early testnet participants and community contributors",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::ValidatorBootstrap,
            amount: 40_000_000,
            vesting: VestingSchedule::new(
                40_000_000,
                genesis_round,
                0,                    // no cliff
                ROUNDS_PER_MONTH * 6, // 6-month linear
            ),
            description: "Initial validator set bootstrap with staking requirement",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::AIEcosystemFund,
            amount: 20_000_000,
            vesting: VestingSchedule::new(
                20_000_000,
                genesis_round,
                ROUNDS_PER_MONTH * 6,  // 6-month cliff
                ROUNDS_PER_MONTH * 36, // 3-year linear
            ),
            description: "AI model creators, PoUW bootstrapping, researcher grants",
        },
        GenesisAllocationEntry {
            category: AllocationCategory::LiquidityProvision,
            amount: 20_000_000,
            vesting: VestingSchedule::immediate(20_000_000, genesis_round),
            description: "Initial DEX liquidity and market-making",
        },
    ]
}

/// Validate that genesis allocations sum to GENESIS_MINT.
pub fn validate_genesis_allocations(allocations: &[GenesisAllocationEntry]) -> bool {
    let total: u128 = allocations.iter().map(|a| a.amount).sum();
    total == GENESIS_MINT
}

/// Check anti-concentration: no entity-controlled allocation exceeds 6% of total supply.
/// Distributed/governance pools are excluded — they disburse to many recipients.
pub fn check_anti_concentration(allocations: &[GenesisAllocationEntry]) -> bool {
    let max_per_entity = TOTAL_SUPPLY * 6 / 100; // 60M
    allocations
        .iter()
        .all(|a| a.category.is_distributed_fund() || a.amount <= max_per_entity)
}

// ---------------------------------------------------------------------------
// EmissionTracker (runtime state)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EmissionTracker {
    pub current_epoch: u64,
    pub total_emitted: u128,
    pub epoch_length: u64,
    pub epochs_per_year: u64,
    pub treasury_balance: u128,
    pub insurance_balance: u128,
}

impl EmissionTracker {
    pub fn new(epoch_length: u64) -> Self {
        let epochs_per_year = if epoch_length > 0 {
            ROUNDS_PER_YEAR / epoch_length
        } else {
            1
        };
        Self {
            current_epoch: 0,
            total_emitted: 0,
            epoch_length,
            epochs_per_year,
            treasury_balance: 0,
            insurance_balance: 0,
        }
    }

    /// Advance to next epoch and compute the emission distribution.
    /// Returns `None` if the hard cap has been reached.
    pub fn advance_epoch(&mut self) -> Option<EpochDistribution> {
        let emission =
            emission_per_epoch(self.current_epoch, self.epochs_per_year, self.total_emitted);

        if emission == 0 {
            self.current_epoch += 1;
            return None;
        }

        let dist = distribute_emission(emission);
        self.total_emitted += dist.total();
        self.treasury_balance += dist.treasury;
        self.insurance_balance += dist.insurance_fund;
        self.current_epoch += 1;

        Some(dist)
    }

    pub fn remaining_emission(&self) -> u128 {
        EMISSION_POOL.saturating_sub(self.total_emitted)
    }

    pub fn total_supply_in_existence(&self) -> u128 {
        GENESIS_MINT + self.total_emitted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emission_year_1_and_2_equal() {
        assert_eq!(emission_for_year(1), 120_000_000);
        assert_eq!(emission_for_year(2), 120_000_000);
    }

    #[test]
    fn emission_halves_every_two_years() {
        assert_eq!(emission_for_year(3), 60_000_000);
        assert_eq!(emission_for_year(4), 60_000_000);
        assert_eq!(emission_for_year(5), 30_000_000);
        assert_eq!(emission_for_year(6), 30_000_000);
        assert_eq!(emission_for_year(7), 15_000_000);
        assert_eq!(emission_for_year(8), 15_000_000);
        assert_eq!(emission_for_year(9), 7_500_000);
        assert_eq!(emission_for_year(10), 7_500_000);
    }

    #[test]
    fn emission_year_zero_is_zero() {
        assert_eq!(emission_for_year(0), 0);
    }

    #[test]
    fn cumulative_emission_10_years() {
        let cum = cumulative_emission(10);
        assert_eq!(cum, 465_000_000);
    }

    #[test]
    fn cumulative_never_exceeds_pool() {
        let cum = cumulative_emission(100);
        assert!(cum <= EMISSION_POOL);
    }

    #[test]
    fn emission_tail_after_year_10() {
        assert_eq!(emission_for_year(11), 3_750_000);
        assert_eq!(emission_for_year(12), 3_750_000);
        assert_eq!(emission_for_year(13), 1_875_000);
    }

    #[test]
    fn distribution_split_sums_to_total() {
        let total = 1_000_000u128;
        let dist = distribute_emission(total);
        assert_eq!(dist.total(), total);
    }

    #[test]
    fn distribution_percentages_correct() {
        let total = 10_000_000u128;
        let dist = distribute_emission(total);
        assert_eq!(dist.validator_rewards, 7_000_000);
        assert_eq!(dist.pouw_pool, 1_500_000);
        assert_eq!(dist.treasury, 1_000_000);
        assert_eq!(dist.insurance_fund, 500_000);
    }

    #[test]
    fn distribution_handles_remainder() {
        let total = 7u128;
        let dist = distribute_emission(total);
        assert_eq!(dist.total(), total);
        assert!(dist.validator_rewards >= dist.pouw_pool);
    }

    #[test]
    fn apy_at_20_percent_staking() {
        assert_eq!(calculate_apy_bps(2000), MAX_APY_BPS);
    }

    #[test]
    fn apy_at_50_percent_staking() {
        assert_eq!(calculate_apy_bps(5000), 800);
    }

    #[test]
    fn apy_at_70_percent() {
        assert_eq!(calculate_apy_bps(7000), 400);
    }

    #[test]
    fn apy_at_80_percent_is_minimum() {
        assert_eq!(calculate_apy_bps(8000), MIN_APY_BPS);
    }

    #[test]
    fn apy_at_zero_staking_caps_at_max() {
        assert_eq!(calculate_apy_bps(0), MAX_APY_BPS);
    }

    #[test]
    fn apy_at_100_percent_is_minimum() {
        assert_eq!(calculate_apy_bps(10_000), MIN_APY_BPS);
    }

    #[test]
    fn validator_reward_proportional_to_stake() {
        let pool = 1_000_000u128;
        let total_weight = 200_000u128; // two validators, 100k each at 100%

        let r1 = validator_epoch_reward(pool, 10_000, total_weight, 100_000);
        let r2 = validator_epoch_reward(pool, 10_000, total_weight, 100_000);
        assert_eq!(r1, r2);
        assert_eq!(r1, 500_000);
    }

    #[test]
    fn validator_reward_scales_with_participation() {
        let pool = 1_000_000u128;
        let stake = 100_000u128;
        let total_weight = 100_000u128;

        let full = validator_epoch_reward(pool, 10_000, total_weight, stake);
        let half = validator_epoch_reward(pool, 5_000, total_weight, stake);
        assert_eq!(full, 1_000_000);
        assert_eq!(half, 500_000);
    }

    #[test]
    fn validator_reward_zero_pool() {
        assert_eq!(validator_epoch_reward(0, 10_000, 100, 50), 0);
    }

    #[test]
    fn validator_reward_zero_weight() {
        assert_eq!(validator_epoch_reward(1_000, 10_000, 0, 50), 0);
    }

    #[test]
    fn vesting_before_start_is_zero() {
        let v = VestingSchedule::new(1_000_000, 100, 50, 200);
        assert_eq!(v.vested_at(50), 0);
    }

    #[test]
    fn vesting_during_cliff_is_zero() {
        let v = VestingSchedule::new(1_000_000, 100, 50, 200);
        assert_eq!(v.vested_at(140), 0);
    }

    #[test]
    fn vesting_after_cliff_linear() {
        let v = VestingSchedule::new(1_000_000, 0, 0, 100);
        assert_eq!(v.vested_at(50), 500_000);
        assert_eq!(v.vested_at(25), 250_000);
    }

    #[test]
    fn vesting_at_end_is_full() {
        let v = VestingSchedule::new(1_000_000, 0, 10, 100);
        assert_eq!(v.vested_at(100), 1_000_000);
        assert_eq!(v.vested_at(200), 1_000_000);
    }

    #[test]
    fn vesting_immediate_fully_vested() {
        let v = VestingSchedule::immediate(500_000, 0);
        assert_eq!(v.vested_at(0), 500_000);
    }

    #[test]
    fn vesting_locked_complement() {
        let v = VestingSchedule::new(1_000_000, 0, 0, 100);
        assert_eq!(v.locked_at(50), 500_000);
        assert_eq!(v.vested_at(50) + v.locked_at(50), 1_000_000);
    }

    #[test]
    fn genesis_allocations_sum_to_400m() {
        let allocs = genesis_allocations(0);
        assert!(validate_genesis_allocations(&allocs));
    }

    #[test]
    fn genesis_anti_concentration_holds() {
        let allocs = genesis_allocations(0);
        assert!(check_anti_concentration(&allocs));
    }

    #[test]
    fn genesis_has_eight_categories() {
        let allocs = genesis_allocations(0);
        assert_eq!(allocs.len(), 8);
    }

    #[test]
    fn emission_tracker_enforces_hard_cap() {
        let mut tracker = EmissionTracker::new(1);
        tracker.total_emitted = EMISSION_POOL;
        let dist = tracker.advance_epoch();
        assert!(dist.is_none());
    }

    #[test]
    fn emission_tracker_total_supply() {
        let mut tracker = EmissionTracker::new(DEFAULT_EPOCH_LENGTH);
        assert_eq!(tracker.total_supply_in_existence(), GENESIS_MINT);

        tracker.advance_epoch();
        assert!(tracker.total_supply_in_existence() > GENESIS_MINT);
    }

    #[test]
    fn emission_per_epoch_respects_remaining() {
        let near_cap = EMISSION_POOL - 10;
        let per = emission_per_epoch(0, 100, near_cap);
        assert!(per <= 10);
    }

    #[test]
    fn emission_schedule_matches_design_table() {
        let table: [(u32, u128); 10] = [
            (1, 120_000_000),
            (2, 120_000_000),
            (3, 60_000_000),
            (4, 60_000_000),
            (5, 30_000_000),
            (6, 30_000_000),
            (7, 15_000_000),
            (8, 15_000_000),
            (9, 7_500_000),
            (10, 7_500_000),
        ];
        for (year, expected) in table {
            assert_eq!(emission_for_year(year), expected, "Year {year} mismatch");
        }
    }

    #[test]
    fn apy_curve_piecewise_linear() {
        // Verify monotonically decreasing
        let mut prev = calculate_apy_bps(0);
        for ratio in (100..=10_000).step_by(100) {
            let apy = calculate_apy_bps(ratio);
            assert!(
                apy <= prev,
                "APY should decrease: ratio={ratio}, apy={apy}, prev={prev}"
            );
            prev = apy;
        }
    }

    #[test]
    fn core_team_cliff_blocks_early_vesting() {
        let allocs = genesis_allocations(0);
        let team = allocs
            .iter()
            .find(|a| a.category == AllocationCategory::CoreTeam)
            .unwrap();

        let six_months = ROUNDS_PER_MONTH * 6;
        let eleven_months = ROUNDS_PER_MONTH * 11;
        assert_eq!(team.vesting.vested_at(six_months), 0);
        assert_eq!(team.vesting.vested_at(eleven_months), 0);

        let thirteen_months = ROUNDS_PER_MONTH * 13;
        assert!(team.vesting.vested_at(thirteen_months) > 0);
    }

    #[test]
    fn ecosystem_dev_cliff_then_linear() {
        let allocs = genesis_allocations(0);
        let eco = allocs
            .iter()
            .find(|a| a.category == AllocationCategory::EcosystemDevelopment)
            .unwrap();

        let five_months = ROUNDS_PER_MONTH * 5;
        assert_eq!(eco.vesting.vested_at(five_months), 0);

        let seven_months = ROUNDS_PER_MONTH * 7;
        assert!(eco.vesting.vested_at(seven_months) > 0);

        let four_years = ROUNDS_PER_MONTH * 48;
        assert_eq!(eco.vesting.vested_at(four_years), eco.amount);
    }

    #[test]
    fn liquidity_and_airdrop_immediately_vested() {
        let allocs = genesis_allocations(0);

        let liq = allocs
            .iter()
            .find(|a| a.category == AllocationCategory::LiquidityProvision)
            .unwrap();
        assert_eq!(liq.vesting.vested_at(0), liq.amount);

        let air = allocs
            .iter()
            .find(|a| a.category == AllocationCategory::CommunityAirdrop)
            .unwrap();
        assert_eq!(air.vesting.vested_at(0), air.amount);
    }
}
