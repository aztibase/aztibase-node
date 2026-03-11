use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

type Address = [u8; 32];

pub const EQUIVOCATION_SLASH_BPS: u32 = 1000;
pub const DOWNTIME_SLASH_BPS: u32 = 50;
pub const DOWNTIME_THRESHOLD_ROUNDS: u64 = 1000;
pub const MAX_UNBONDING_ENTRIES: usize = 10_000;
pub const DEFAULT_COMMISSION_BPS: u32 = 1000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorStake {
    pub validator_id: Address,
    pub self_stake: u128,
    pub total_delegated: u128,
    pub active: bool,
    pub registered_round: u64,
}

impl ValidatorStake {
    pub fn effective_stake(&self) -> u128 {
        self.self_stake.saturating_add(self.total_delegated)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Delegation {
    pub delegator: Address,
    pub validator_id: Address,
    pub amount: u128,
    pub round_delegated: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnbondingEntry {
    pub owner: Address,
    pub amount: u128,
    pub available_round: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OffenseType {
    Equivocation,
    Downtime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashRecord {
    pub validator_id: Address,
    pub offense_type: OffenseType,
    pub slash_bps: u32,
    pub round: u64,
    pub amount_slashed: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StakingError {
    ValidatorNotFound,
    ValidatorAlreadyRegistered,
    InsufficientStake,
    BelowMinimumStake,
    ExceedsMaxStakeCap,
    NoDelegation,
    AlreadyDelegated,
    ValidatorNotActive,
    UnbondingQueueFull,
    ZeroAmount,
}

impl std::fmt::Display for StakingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValidatorNotFound => write!(f, "validator not found"),
            Self::ValidatorAlreadyRegistered => write!(f, "validator already registered"),
            Self::InsufficientStake => write!(f, "insufficient stake"),
            Self::BelowMinimumStake => write!(f, "below minimum stake"),
            Self::ExceedsMaxStakeCap => write!(f, "exceeds maximum stake cap"),
            Self::NoDelegation => write!(f, "no delegation found"),
            Self::AlreadyDelegated => write!(f, "already delegated to a validator"),
            Self::ValidatorNotActive => write!(f, "validator is not active"),
            Self::UnbondingQueueFull => write!(f, "unbonding queue full"),
            Self::ZeroAmount => write!(f, "amount must be non-zero"),
        }
    }
}

impl std::error::Error for StakingError {}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StakingStore {
    validators: HashMap<Address, ValidatorStake>,
    delegations: HashMap<Address, Delegation>,
    unbonding_queue: VecDeque<UnbondingEntry>,
    slash_history: Vec<SlashRecord>,
}

impl StakingStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_validator(
        &mut self,
        validator_id: Address,
        self_stake: u128,
        min_stake: u128,
        max_cap: u128,
        round: u64,
    ) -> Result<(), StakingError> {
        if self_stake == 0 {
            return Err(StakingError::ZeroAmount);
        }
        if self.validators.contains_key(&validator_id) {
            return Err(StakingError::ValidatorAlreadyRegistered);
        }
        if self_stake < min_stake {
            return Err(StakingError::BelowMinimumStake);
        }
        if self_stake > max_cap {
            return Err(StakingError::ExceedsMaxStakeCap);
        }
        self.validators.insert(
            validator_id,
            ValidatorStake {
                validator_id,
                self_stake,
                total_delegated: 0,
                active: true,
                registered_round: round,
            },
        );
        Ok(())
    }

    pub fn add_stake(
        &mut self,
        validator_id: Address,
        amount: u128,
        max_cap: u128,
    ) -> Result<(), StakingError> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }
        let v = self
            .validators
            .get_mut(&validator_id)
            .ok_or(StakingError::ValidatorNotFound)?;
        let new_effective = v.effective_stake().saturating_add(amount);
        if new_effective > max_cap {
            return Err(StakingError::ExceedsMaxStakeCap);
        }
        v.self_stake = v.self_stake.saturating_add(amount);
        Ok(())
    }

    pub fn begin_unstake(
        &mut self,
        validator_id: Address,
        amount: u128,
        min_stake: u128,
        current_round: u64,
        unbonding_rounds: u64,
    ) -> Result<(), StakingError> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }
        if self.unbonding_queue.len() >= MAX_UNBONDING_ENTRIES {
            return Err(StakingError::UnbondingQueueFull);
        }
        let v = self
            .validators
            .get_mut(&validator_id)
            .ok_or(StakingError::ValidatorNotFound)?;
        if v.self_stake < amount {
            return Err(StakingError::InsufficientStake);
        }
        let remaining = v.self_stake - amount;
        if remaining > 0 && remaining < min_stake {
            return Err(StakingError::BelowMinimumStake);
        }
        v.self_stake = remaining;
        if v.effective_stake() == 0 {
            v.active = false;
        }
        self.unbonding_queue.push_back(UnbondingEntry {
            owner: validator_id,
            amount,
            available_round: current_round.saturating_add(unbonding_rounds),
        });
        Ok(())
    }

    pub fn delegate(
        &mut self,
        delegator: Address,
        validator_id: Address,
        amount: u128,
        max_cap: u128,
        round: u64,
    ) -> Result<(), StakingError> {
        if amount == 0 {
            return Err(StakingError::ZeroAmount);
        }
        if self.delegations.contains_key(&delegator) {
            return Err(StakingError::AlreadyDelegated);
        }
        let v = self
            .validators
            .get_mut(&validator_id)
            .ok_or(StakingError::ValidatorNotFound)?;
        if !v.active {
            return Err(StakingError::ValidatorNotActive);
        }
        let new_effective = v.effective_stake().saturating_add(amount);
        if new_effective > max_cap {
            return Err(StakingError::ExceedsMaxStakeCap);
        }
        v.total_delegated = v.total_delegated.saturating_add(amount);
        self.delegations.insert(
            delegator,
            Delegation {
                delegator,
                validator_id,
                amount,
                round_delegated: round,
            },
        );
        Ok(())
    }

    pub fn begin_undelegate(
        &mut self,
        delegator: Address,
        current_round: u64,
        unbonding_rounds: u64,
    ) -> Result<u128, StakingError> {
        if self.unbonding_queue.len() >= MAX_UNBONDING_ENTRIES {
            return Err(StakingError::UnbondingQueueFull);
        }
        let delegation = self
            .delegations
            .remove(&delegator)
            .ok_or(StakingError::NoDelegation)?;
        if let Some(v) = self.validators.get_mut(&delegation.validator_id) {
            v.total_delegated = v.total_delegated.saturating_sub(delegation.amount);
        }
        let amount = delegation.amount;
        self.unbonding_queue.push_back(UnbondingEntry {
            owner: delegator,
            amount,
            available_round: current_round.saturating_add(unbonding_rounds),
        });
        Ok(amount)
    }

    pub fn process_unbonding(&mut self, current_round: u64) -> Vec<(Address, u128)> {
        let mut released = Vec::new();
        while let Some(front) = self.unbonding_queue.front() {
            if front.available_round > current_round {
                break;
            }
            let entry = self.unbonding_queue.pop_front().unwrap();
            released.push((entry.owner, entry.amount));
        }
        released
    }

    pub fn slash_validator(
        &mut self,
        validator_id: Address,
        slash_bps: u32,
        offense_type: OffenseType,
        round: u64,
    ) -> Result<u128, StakingError> {
        let v = self
            .validators
            .get_mut(&validator_id)
            .ok_or(StakingError::ValidatorNotFound)?;
        let self_slash = v.self_stake * slash_bps as u128 / 10_000;
        v.self_stake = v.self_stake.saturating_sub(self_slash);

        let mut total_slashed = self_slash;

        let delegator_addrs: Vec<Address> = self
            .delegations
            .values()
            .filter(|d| d.validator_id == validator_id)
            .map(|d| d.delegator)
            .collect();

        for addr in delegator_addrs {
            if let Some(d) = self.delegations.get_mut(&addr) {
                let d_slash = d.amount * slash_bps as u128 / 10_000;
                d.amount = d.amount.saturating_sub(d_slash);
                total_slashed = total_slashed.saturating_add(d_slash);
            }
        }

        if let Some(v) = self.validators.get_mut(&validator_id) {
            let new_delegated: u128 = self
                .delegations
                .values()
                .filter(|d| d.validator_id == validator_id)
                .map(|d| d.amount)
                .sum();
            v.total_delegated = new_delegated;
            if v.effective_stake() == 0 {
                v.active = false;
            }
        }

        self.slash_history.push(SlashRecord {
            validator_id,
            offense_type,
            slash_bps,
            round,
            amount_slashed: total_slashed,
        });

        Ok(total_slashed)
    }

    pub fn get_validator(&self, id: &Address) -> Option<&ValidatorStake> {
        self.validators.get(id)
    }

    pub fn get_delegation(&self, delegator: &Address) -> Option<&Delegation> {
        self.delegations.get(delegator)
    }

    pub fn active_validators(&self) -> Vec<&ValidatorStake> {
        let mut active: Vec<_> = self.validators.values().filter(|v| v.active).collect();
        active.sort_by_key(|v| Reverse(v.effective_stake()));
        active
    }

    pub fn pending_unbonding(&self, owner: &Address) -> Vec<&UnbondingEntry> {
        self.unbonding_queue
            .iter()
            .filter(|e| &e.owner == owner)
            .collect()
    }

    pub fn slash_history(&self) -> &[SlashRecord] {
        &self.slash_history
    }

    pub fn validator_slash_history(&self, validator_id: &Address) -> Vec<&SlashRecord> {
        self.slash_history
            .iter()
            .filter(|r| &r.validator_id == validator_id)
            .collect()
    }

    pub fn unbonding_queue_len(&self) -> usize {
        self.unbonding_queue.len()
    }

    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    pub fn active_validator_count(&self) -> usize {
        self.validators.values().filter(|v| v.active).count()
    }

    pub fn total_staked(&self) -> u128 {
        self.validators.values().map(|v| v.effective_stake()).sum()
    }

    /// Build the active validator set as (validator_id, effective_stake) pairs.
    /// Only validators with effective_stake >= min_stake are included.
    /// Sorted by effective_stake descending for deterministic ordering.
    pub fn active_set_snapshot(&self, min_stake: u128) -> Vec<([u8; 32], u128)> {
        let mut set: Vec<_> = self
            .validators
            .values()
            .filter(|v| v.active && v.effective_stake() >= min_stake)
            .map(|v| (v.validator_id, v.effective_stake()))
            .collect();
        set.sort_by_key(|(_, s)| Reverse(*s));
        set
    }

    /// Distribute epoch rewards to active validators and their delegators.
    /// Returns a list of (address, reward_amount) credits to apply to balances.
    /// Commission (`commission_bps`) is taken from delegation rewards and given
    /// to the validator.
    pub fn distribute_epoch_rewards(
        &mut self,
        validator_pool: u128,
        commission_bps: u32,
    ) -> Vec<(Address, u128)> {
        if validator_pool == 0 {
            return Vec::new();
        }

        let active: Vec<(Address, u128, u128)> = self
            .validators
            .values()
            .filter(|v| v.active)
            .map(|v| (v.validator_id, v.self_stake, v.total_delegated))
            .collect();

        let total_active_stake: u128 = active.iter().map(|(_, s, d)| *s + *d).sum();

        if total_active_stake == 0 {
            return Vec::new();
        }

        let mut credits: Vec<(Address, u128)> = Vec::new();

        for (vid, self_stake, total_delegated) in &active {
            let effective = *self_stake + *total_delegated;
            let validator_total_reward = validator_pool * effective / total_active_stake;

            if validator_total_reward == 0 {
                continue;
            }

            if *total_delegated == 0 {
                credits.push((vid.to_owned(), validator_total_reward));
                continue;
            }

            let self_share = validator_total_reward * *self_stake / effective;
            let delegation_share = validator_total_reward - self_share;

            let commission = delegation_share * commission_bps as u128 / 10_000;
            let validator_reward = self_share + commission;
            let delegator_pool = delegation_share - commission;

            credits.push((vid.to_owned(), validator_reward));

            let delegators: Vec<(Address, u128)> = self
                .delegations
                .values()
                .filter(|d| d.validator_id == *vid)
                .map(|d| (d.delegator, d.amount))
                .collect();

            for (delegator, amount) in &delegators {
                let d_reward = delegator_pool * *amount / *total_delegated;
                if d_reward > 0 {
                    credits.push((*delegator, d_reward));
                }
            }
        }

        credits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenomics::{DEFAULT_MIN_STAKE, MAX_STAKE_CAP, UNBONDING_ROUNDS};

    const MIN_STAKE: u128 = DEFAULT_MIN_STAKE;
    const MAX_CAP: u128 = MAX_STAKE_CAP;
    const ROUND: u64 = 100;

    fn addr(n: u8) -> Address {
        [n; 32]
    }

    #[test]
    fn register_and_query_validator() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.self_stake, MIN_STAKE);
        assert_eq!(v.total_delegated, 0);
        assert!(v.active);
        assert_eq!(v.effective_stake(), MIN_STAKE);
    }

    #[test]
    fn register_duplicate_rejected() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let err = store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap_err();
        assert_eq!(err, StakingError::ValidatorAlreadyRegistered);
    }

    #[test]
    fn register_below_minimum_rejected() {
        let mut store = StakingStore::new();
        let err = store
            .register_validator(addr(1), MIN_STAKE - 1, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap_err();
        assert_eq!(err, StakingError::BelowMinimumStake);
    }

    #[test]
    fn register_above_cap_rejected() {
        let mut store = StakingStore::new();
        let err = store
            .register_validator(addr(1), MAX_CAP + 1, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap_err();
        assert_eq!(err, StakingError::ExceedsMaxStakeCap);
    }

    #[test]
    fn add_stake_increases_self_stake() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store.add_stake(addr(1), 10_000, MAX_CAP).unwrap();
        assert_eq!(
            store.get_validator(&addr(1)).unwrap().self_stake,
            MIN_STAKE + 10_000
        );
    }

    #[test]
    fn add_stake_exceeds_cap_rejected() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MAX_CAP, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let err = store.add_stake(addr(1), 1, MAX_CAP).unwrap_err();
        assert_eq!(err, StakingError::ExceedsMaxStakeCap);
    }

    #[test]
    fn unstake_partial() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE * 2, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .begin_unstake(addr(1), MIN_STAKE, MIN_STAKE, ROUND, UNBONDING_ROUNDS)
            .unwrap();
        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.self_stake, MIN_STAKE);
        assert!(v.active);
        assert_eq!(store.unbonding_queue_len(), 1);
    }

    #[test]
    fn unstake_full_exit() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .begin_unstake(addr(1), MIN_STAKE, MIN_STAKE, ROUND, UNBONDING_ROUNDS)
            .unwrap();
        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.self_stake, 0);
        assert!(!v.active);
    }

    #[test]
    fn unstake_below_minimum_rejected() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE + 100, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let err = store
            .begin_unstake(addr(1), 101, MIN_STAKE, ROUND, UNBONDING_ROUNDS)
            .unwrap_err();
        assert_eq!(err, StakingError::BelowMinimumStake);
    }

    #[test]
    fn unbonding_matures_after_delay() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .begin_unstake(addr(1), MIN_STAKE, MIN_STAKE, ROUND, 100)
            .unwrap();

        let early = store.process_unbonding(ROUND + 50);
        assert!(early.is_empty());

        let mature = store.process_unbonding(ROUND + 100);
        assert_eq!(mature.len(), 1);
        assert_eq!(mature[0], (addr(1), MIN_STAKE));
        assert_eq!(store.unbonding_queue_len(), 0);
    }

    #[test]
    fn delegate_and_undelegate() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .delegate(addr(2), addr(1), 5_000, MAX_CAP, ROUND)
            .unwrap();

        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.total_delegated, 5_000);
        assert_eq!(v.effective_stake(), MIN_STAKE + 5_000);

        let d = store.get_delegation(&addr(2)).unwrap();
        assert_eq!(d.amount, 5_000);

        let amount = store.begin_undelegate(addr(2), ROUND, 100).unwrap();
        assert_eq!(amount, 5_000);
        assert!(store.get_delegation(&addr(2)).is_none());

        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.total_delegated, 0);
    }

    #[test]
    fn double_delegation_rejected() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .delegate(addr(2), addr(1), 5_000, MAX_CAP, ROUND)
            .unwrap();
        let err = store
            .delegate(addr(2), addr(1), 3_000, MAX_CAP, ROUND)
            .unwrap_err();
        assert_eq!(err, StakingError::AlreadyDelegated);
    }

    #[test]
    fn delegate_to_inactive_rejected() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .begin_unstake(addr(1), MIN_STAKE, MIN_STAKE, ROUND, 100)
            .unwrap();
        let err = store
            .delegate(addr(2), addr(1), 5_000, MAX_CAP, ROUND)
            .unwrap_err();
        assert_eq!(err, StakingError::ValidatorNotActive);
    }

    #[test]
    fn equivocation_slash_reduces_stake() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), 100_000, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .delegate(addr(2), addr(1), 50_000, MAX_CAP, ROUND)
            .unwrap();

        let slashed = store
            .slash_validator(
                addr(1),
                EQUIVOCATION_SLASH_BPS,
                OffenseType::Equivocation,
                ROUND,
            )
            .unwrap();

        assert_eq!(slashed, 15_000);
        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.self_stake, 90_000);
        let d = store.get_delegation(&addr(2)).unwrap();
        assert_eq!(d.amount, 45_000);
    }

    #[test]
    fn downtime_slash_is_mild() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), 100_000, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();

        let slashed = store
            .slash_validator(addr(1), DOWNTIME_SLASH_BPS, OffenseType::Downtime, ROUND)
            .unwrap();

        assert_eq!(slashed, 500);
        assert_eq!(store.get_validator(&addr(1)).unwrap().self_stake, 99_500);
    }

    #[test]
    fn slash_deactivates_if_zero() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();

        store
            .slash_validator(addr(1), 10_000, OffenseType::Equivocation, ROUND)
            .unwrap();

        let v = store.get_validator(&addr(1)).unwrap();
        assert_eq!(v.self_stake, 0);
        assert!(!v.active);
    }

    #[test]
    fn active_validators_sorted_by_stake() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .register_validator(addr(2), MIN_STAKE * 3, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .register_validator(addr(3), MIN_STAKE * 2, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();

        let active = store.active_validators();
        assert_eq!(active.len(), 3);
        assert_eq!(active[0].validator_id, addr(2));
        assert_eq!(active[1].validator_id, addr(3));
        assert_eq!(active[2].validator_id, addr(1));
    }

    #[test]
    fn zero_amount_rejected() {
        let mut store = StakingStore::new();
        assert_eq!(
            store
                .register_validator(addr(1), 0, MIN_STAKE, MAX_CAP, ROUND)
                .unwrap_err(),
            StakingError::ZeroAmount
        );
    }

    #[test]
    fn total_staked_tracks_all() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .register_validator(addr(2), MIN_STAKE * 2, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .delegate(addr(3), addr(1), 10_000, MAX_CAP, ROUND)
            .unwrap();
        assert_eq!(store.total_staked(), MIN_STAKE * 3 + 10_000);
    }

    #[test]
    fn epoch_reward_single_validator() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let credits = store.distribute_epoch_rewards(1_000_000, DEFAULT_COMMISSION_BPS);
        assert_eq!(credits.len(), 1);
        assert_eq!(credits[0], (addr(1), 1_000_000));
    }

    #[test]
    fn epoch_reward_proportional_split() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .register_validator(addr(2), MIN_STAKE * 3, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let credits = store.distribute_epoch_rewards(400_000, DEFAULT_COMMISSION_BPS);
        let v1_reward: u128 = credits
            .iter()
            .filter(|(a, _)| *a == addr(1))
            .map(|(_, r)| r)
            .sum();
        let v2_reward: u128 = credits
            .iter()
            .filter(|(a, _)| *a == addr(2))
            .map(|(_, r)| r)
            .sum();
        assert_eq!(v1_reward, 100_000);
        assert_eq!(v2_reward, 300_000);
    }

    #[test]
    fn epoch_reward_with_delegation_and_commission() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), 90_000, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .delegate(addr(2), addr(1), 10_000, MAX_CAP, ROUND)
            .unwrap();
        // 100% of pool goes to validator 1 (only active)
        // effective = 100_000, self_share = 90%, delegation_share = 10%
        // Commission 10% of delegation_share: 10_000 * 10% = 1_000 to validator
        // Delegator gets: 10_000 - 1_000 = 9_000
        let credits = store.distribute_epoch_rewards(100_000, DEFAULT_COMMISSION_BPS);
        let v1_reward: u128 = credits
            .iter()
            .filter(|(a, _)| *a == addr(1))
            .map(|(_, r)| r)
            .sum();
        let d_reward: u128 = credits
            .iter()
            .filter(|(a, _)| *a == addr(2))
            .map(|(_, r)| r)
            .sum();
        assert_eq!(v1_reward, 91_000); // 90_000 self + 1_000 commission
        assert_eq!(d_reward, 9_000);
    }

    #[test]
    fn epoch_reward_zero_pool_no_credits() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), MIN_STAKE, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        let credits = store.distribute_epoch_rewards(0, DEFAULT_COMMISSION_BPS);
        assert!(credits.is_empty());
    }

    #[test]
    fn slash_history_recorded() {
        let mut store = StakingStore::new();
        store
            .register_validator(addr(1), 100_000, MIN_STAKE, MAX_CAP, ROUND)
            .unwrap();
        store
            .slash_validator(
                addr(1),
                EQUIVOCATION_SLASH_BPS,
                OffenseType::Equivocation,
                ROUND,
            )
            .unwrap();

        let history = store.validator_slash_history(&addr(1));
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].offense_type, OffenseType::Equivocation);
        assert_eq!(history[0].amount_slashed, 10_000);
    }
}
