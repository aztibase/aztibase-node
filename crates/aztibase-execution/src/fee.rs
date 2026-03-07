use crate::state::AccountState;

type Address = [u8; 32];

const MIN_BASE_FEE: u64 = 1;
const MAX_BASE_FEE: u64 = 1_000_000_000;
const TARGET_GAS_PER_BATCH: u64 = 15_000_000;
const MAX_GAS_PER_BATCH: u64 = 30_000_000;
const BASE_FEE_CHANGE_DENOMINATOR: u64 = 8;

/// Escrow result: the maximum fee that was locked.
pub struct FeeEscrow {
    pub sender: Address,
    pub max_fee: u64,
    pub gas_limit: u64,
    pub gas_price: u64,
}

/// Attempt to escrow `gas_limit * gas_price` from the sender's balance.
/// Returns `None` if the sender cannot afford the fee + value transfer.
pub fn escrow_fee(
    state: &mut AccountState,
    sender: &Address,
    gas_limit: u64,
    gas_price: u64,
    value: u64,
) -> Option<FeeEscrow> {
    let max_fee = gas_limit.checked_mul(gas_price)?;
    let total_cost = max_fee.checked_add(value)?;
    let balance = state.balance(sender);
    if balance < total_cost {
        return None;
    }
    state.set_balance(sender, balance - max_fee);
    Some(FeeEscrow {
        sender: *sender,
        max_fee,
        gas_limit,
        gas_price,
    })
}

/// Refund unused gas after execution. Returns the actual fee paid.
pub fn refund_unused(state: &mut AccountState, escrow: &FeeEscrow, gas_used: u64) -> u64 {
    let capped_used = gas_used.min(escrow.gas_limit);
    let actual_fee = capped_used.saturating_mul(escrow.gas_price);
    let refund = escrow.max_fee.saturating_sub(actual_fee);
    if refund > 0 {
        let balance = state.balance(&escrow.sender);
        state.set_balance(&escrow.sender, balance + refund);
    }
    actual_fee
}

/// EIP-1559-style base fee calculator.
/// Adjusts based on gas usage relative to target: above target → increase, below → decrease.
pub struct BaseFeeCalculator {
    base_fee: u64,
}

impl BaseFeeCalculator {
    pub fn new(initial_base_fee: u64) -> Self {
        Self {
            base_fee: initial_base_fee.max(MIN_BASE_FEE),
        }
    }

    pub fn base_fee(&self) -> u64 {
        self.base_fee
    }

    /// Update the base fee after a batch. `gas_used` is the total gas consumed by the batch.
    pub fn update(&mut self, gas_used: u64) {
        if gas_used > TARGET_GAS_PER_BATCH {
            let excess = gas_used - TARGET_GAS_PER_BATCH;
            let delta =
                (self.base_fee * excess) / TARGET_GAS_PER_BATCH / BASE_FEE_CHANGE_DENOMINATOR;
            self.base_fee = self.base_fee.saturating_add(delta.max(1));
        } else {
            let deficit = TARGET_GAS_PER_BATCH - gas_used;
            let delta =
                (self.base_fee * deficit) / TARGET_GAS_PER_BATCH / BASE_FEE_CHANGE_DENOMINATOR;
            self.base_fee = self.base_fee.saturating_sub(delta);
        }
        self.base_fee = self.base_fee.clamp(MIN_BASE_FEE, MAX_BASE_FEE);
    }

    /// Estimate gas for a transaction type. Returns a conservative estimate.
    pub fn estimate_gas(tx_prefix: u8) -> u64 {
        match tx_prefix {
            0x01 => 21_000,  // Transfer
            0x02 => 100_000, // ContractDeploy
            0x03 => 50_000,  // ContractCall
            0x04 => 100_000, // EvmDeploy
            0x05 => 50_000,  // EvmCall
            0x06 => 100_000, // AiInfer
            0x07 => 53_000,  // CreateAgent
            _ => 21_000,
        }
    }

    /// Check if a transaction's gas_price meets the current base fee.
    pub fn validate_gas_price(&self, gas_price: u64) -> bool {
        gas_price >= self.base_fee
    }

    pub fn max_gas_per_batch() -> u64 {
        MAX_GAS_PER_BATCH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escrow_deducts_max_fee() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, 1_000_000);

        let escrow = escrow_fee(&mut state, &sender, 21_000, 10, 0).unwrap();
        assert_eq!(escrow.max_fee, 210_000);
        assert_eq!(state.balance(&sender), 790_000);
    }

    #[test]
    fn escrow_fails_insufficient_balance() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, 100);

        assert!(escrow_fee(&mut state, &sender, 21_000, 10, 0).is_none());
        assert_eq!(state.balance(&sender), 100);
    }

    #[test]
    fn escrow_considers_value() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        // max_fee = 210_000, value = 100_000, total = 310_000
        state.set_balance(&sender, 310_000);
        assert!(escrow_fee(&mut state, &sender, 21_000, 10, 100_000).is_some());

        let mut state2 = AccountState::new();
        state2.set_balance(&sender, 309_999);
        assert!(escrow_fee(&mut state2, &sender, 21_000, 10, 100_000).is_none());
    }

    #[test]
    fn refund_returns_unused_gas() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, 1_000_000);

        let escrow = escrow_fee(&mut state, &sender, 21_000, 10, 0).unwrap();
        assert_eq!(state.balance(&sender), 790_000);

        let fee = refund_unused(&mut state, &escrow, 10_000);
        assert_eq!(fee, 100_000);
        assert_eq!(state.balance(&sender), 900_000);
    }

    #[test]
    fn refund_caps_at_gas_limit() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, 1_000_000);

        let escrow = escrow_fee(&mut state, &sender, 21_000, 10, 0).unwrap();
        let fee = refund_unused(&mut state, &escrow, 50_000);
        assert_eq!(fee, 210_000);
        assert_eq!(state.balance(&sender), 790_000);
    }

    #[test]
    fn refund_zero_gas_used() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, 500_000);

        let escrow = escrow_fee(&mut state, &sender, 21_000, 10, 0).unwrap();
        let fee = refund_unused(&mut state, &escrow, 0);
        assert_eq!(fee, 0);
        assert_eq!(state.balance(&sender), 500_000);
    }

    #[test]
    fn base_fee_increases_above_target() {
        let mut calc = BaseFeeCalculator::new(100);
        calc.update(TARGET_GAS_PER_BATCH * 2);
        assert!(calc.base_fee() > 100);
    }

    #[test]
    fn base_fee_decreases_below_target() {
        let mut calc = BaseFeeCalculator::new(100);
        calc.update(0);
        assert!(calc.base_fee() < 100);
    }

    #[test]
    fn base_fee_floors_at_minimum() {
        let mut calc = BaseFeeCalculator::new(1);
        calc.update(0);
        assert_eq!(calc.base_fee(), MIN_BASE_FEE);
    }

    #[test]
    fn base_fee_caps_at_maximum() {
        let mut calc = BaseFeeCalculator::new(MAX_BASE_FEE);
        calc.update(MAX_GAS_PER_BATCH);
        assert!(calc.base_fee() <= MAX_BASE_FEE);
    }

    #[test]
    fn base_fee_stable_at_target() {
        let mut calc = BaseFeeCalculator::new(1000);
        let before = calc.base_fee();
        calc.update(TARGET_GAS_PER_BATCH);
        assert_eq!(calc.base_fee(), before);
    }

    #[test]
    fn validate_gas_price_accepts_above_base() {
        let calc = BaseFeeCalculator::new(100);
        assert!(calc.validate_gas_price(100));
        assert!(calc.validate_gas_price(200));
        assert!(!calc.validate_gas_price(99));
    }

    #[test]
    fn estimate_gas_returns_known_values() {
        assert_eq!(BaseFeeCalculator::estimate_gas(0x01), 21_000);
        assert_eq!(BaseFeeCalculator::estimate_gas(0x07), 53_000);
        assert_eq!(BaseFeeCalculator::estimate_gas(0xFF), 21_000);
    }

    #[test]
    fn escrow_overflow_protection() {
        let mut state = AccountState::new();
        let sender = [1u8; 32];
        state.set_balance(&sender, u64::MAX);

        assert!(escrow_fee(&mut state, &sender, u64::MAX, u64::MAX, 0).is_none());
    }
}
