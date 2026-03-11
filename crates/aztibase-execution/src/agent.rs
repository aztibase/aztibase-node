use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

type Address = [u8; 32];

/// Maximum allowed tx kinds in a policy to prevent abuse.
const MAX_ALLOWED_TX_KINDS: usize = 32;

/// Policy constraints for an autonomous AI agent.
/// Set by the agent's owner (creator) to limit what the agent can do.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentPolicy {
    pub owner: Address,
    pub per_tx_limit: u128,
    pub per_epoch_limit: u128,
    pub allowed_tx_kinds: Vec<u8>,
    pub expiry_epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSpendRecord {
    pub epoch: u64,
    pub total_spent: u128,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("address is not an AI agent")]
    NotAnAgent,
    #[error("no agent policy set")]
    PolicyNotSet,
    #[error("per-tx spending cap exceeded: {0} > limit {1}")]
    PerTxCapExceeded(u128, u128),
    #[error("per-epoch spending cap exceeded: {0} + {1} > limit {2}")]
    PerEpochCapExceeded(u128, u128, u128),
    #[error("tx kind 0x{0:02x} not allowed by policy")]
    TxKindNotAllowed(u8),
    #[error("agent policy expired at epoch {0}, current epoch {1}")]
    PolicyExpired(u64, u64),
    #[error("only the agent owner can modify policy")]
    NotOwner,
    #[error("too many allowed tx kinds: {0} > {MAX_ALLOWED_TX_KINDS}")]
    TooManyAllowedKinds(usize),
}

/// In-memory store for agent policies.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AgentPolicyStore {
    policies: BTreeMap<Address, AgentPolicy>,
    spend_tracker: BTreeMap<Address, AgentSpendRecord>,
}

impl AgentPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_policy(&mut self, agent: Address, policy: AgentPolicy) {
        self.policies.insert(agent, policy);
    }

    pub fn get_policy(&self, agent: &Address) -> Option<&AgentPolicy> {
        self.policies.get(agent)
    }

    pub fn remove_policy(&mut self, agent: &Address) -> Option<AgentPolicy> {
        self.spend_tracker.remove(agent);
        self.policies.remove(agent)
    }

    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Validate and record a spend against the agent's policy.
    /// Returns Ok(()) if the spend is allowed, Err otherwise.
    pub fn validate_spend(
        &mut self,
        agent: &Address,
        amount: u128,
        tx_kind_prefix: u8,
        current_epoch: u64,
    ) -> Result<(), AgentError> {
        let policy = self.policies.get(agent).ok_or(AgentError::PolicyNotSet)?;

        if current_epoch >= policy.expiry_epoch {
            return Err(AgentError::PolicyExpired(
                policy.expiry_epoch,
                current_epoch,
            ));
        }

        if !policy.allowed_tx_kinds.contains(&tx_kind_prefix) {
            return Err(AgentError::TxKindNotAllowed(tx_kind_prefix));
        }

        if amount > policy.per_tx_limit {
            return Err(AgentError::PerTxCapExceeded(amount, policy.per_tx_limit));
        }

        let record = self
            .spend_tracker
            .entry(*agent)
            .or_insert(AgentSpendRecord {
                epoch: current_epoch,
                total_spent: 0,
            });

        // Reset tracker on new epoch
        if record.epoch != current_epoch {
            record.epoch = current_epoch;
            record.total_spent = 0;
        }

        let new_total = record.total_spent.saturating_add(amount);
        if new_total > policy.per_epoch_limit {
            return Err(AgentError::PerEpochCapExceeded(
                record.total_spent,
                amount,
                policy.per_epoch_limit,
            ));
        }

        record.total_spent = new_total;
        Ok(())
    }

    /// Read-only policy check without recording the spend.
    /// Used for pre-execution validation (mempool gate).
    pub fn check_spend(
        &self,
        agent: &Address,
        amount: u128,
        tx_kind_prefix: u8,
        current_epoch: u64,
    ) -> Result<(), AgentError> {
        let policy = self.policies.get(agent).ok_or(AgentError::PolicyNotSet)?;

        if current_epoch >= policy.expiry_epoch {
            return Err(AgentError::PolicyExpired(
                policy.expiry_epoch,
                current_epoch,
            ));
        }

        if !policy.allowed_tx_kinds.contains(&tx_kind_prefix) {
            return Err(AgentError::TxKindNotAllowed(tx_kind_prefix));
        }

        if amount > policy.per_tx_limit {
            return Err(AgentError::PerTxCapExceeded(amount, policy.per_tx_limit));
        }

        let current_spent = self.epoch_spend(agent, current_epoch);
        let new_total = current_spent.saturating_add(amount);
        if new_total > policy.per_epoch_limit {
            return Err(AgentError::PerEpochCapExceeded(
                current_spent,
                amount,
                policy.per_epoch_limit,
            ));
        }

        Ok(())
    }

    pub fn epoch_spend(&self, agent: &Address, current_epoch: u64) -> u128 {
        self.spend_tracker
            .get(agent)
            .filter(|r| r.epoch == current_epoch)
            .map_or(0, |r| r.total_spent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy(owner: Address) -> AgentPolicy {
        AgentPolicy {
            owner,
            per_tx_limit: 1_000,
            per_epoch_limit: 5_000,
            allowed_tx_kinds: vec![0x01], // Transfer only
            expiry_epoch: 100,
        }
    }

    #[test]
    fn store_set_get_remove() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        let owner = [2u8; 32];

        assert!(store.get_policy(&agent).is_none());
        store.set_policy(agent, test_policy(owner));
        assert!(store.get_policy(&agent).is_some());
        assert_eq!(store.policy_count(), 1);

        let removed = store.remove_policy(&agent);
        assert!(removed.is_some());
        assert!(store.get_policy(&agent).is_none());
        assert_eq!(store.policy_count(), 0);
    }

    #[test]
    fn validate_spend_within_limits() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        assert!(store.validate_spend(&agent, 500, 0x01, 1).is_ok());
        assert_eq!(store.epoch_spend(&agent, 1), 500);
        assert!(store.validate_spend(&agent, 500, 0x01, 1).is_ok());
        assert_eq!(store.epoch_spend(&agent, 1), 1_000);
    }

    #[test]
    fn validate_spend_per_tx_exceeded() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        let err = store.validate_spend(&agent, 1_001, 0x01, 1).unwrap_err();
        assert!(matches!(err, AgentError::PerTxCapExceeded(1_001, 1_000)));
    }

    #[test]
    fn validate_spend_per_epoch_exceeded() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        assert!(store.validate_spend(&agent, 1_000, 0x01, 1).is_ok());
        assert!(store.validate_spend(&agent, 1_000, 0x01, 1).is_ok());
        assert!(store.validate_spend(&agent, 1_000, 0x01, 1).is_ok());
        assert!(store.validate_spend(&agent, 1_000, 0x01, 1).is_ok());
        assert!(store.validate_spend(&agent, 1_000, 0x01, 1).is_ok());
        let err = store.validate_spend(&agent, 1, 0x01, 1).unwrap_err();
        assert!(matches!(
            err,
            AgentError::PerEpochCapExceeded(5_000, 1, 5_000)
        ));
    }

    #[test]
    fn validate_spend_wrong_tx_kind() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        let err = store.validate_spend(&agent, 100, 0x06, 1).unwrap_err();
        assert!(matches!(err, AgentError::TxKindNotAllowed(0x06)));
    }

    #[test]
    fn validate_spend_expired_policy() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        let err = store.validate_spend(&agent, 100, 0x01, 100).unwrap_err();
        assert!(matches!(err, AgentError::PolicyExpired(100, 100)));

        let err = store.validate_spend(&agent, 100, 0x01, 200).unwrap_err();
        assert!(matches!(err, AgentError::PolicyExpired(100, 200)));
    }

    #[test]
    fn epoch_spend_resets_on_new_epoch() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        let owner = [2u8; 32];
        store.set_policy(
            agent,
            AgentPolicy {
                owner,
                per_tx_limit: 5_000,
                per_epoch_limit: 10_000,
                allowed_tx_kinds: vec![0x01],
                expiry_epoch: 100,
            },
        );

        assert!(store.validate_spend(&agent, 4_000, 0x01, 1).is_ok());
        assert_eq!(store.epoch_spend(&agent, 1), 4_000);

        // New epoch resets
        assert!(store.validate_spend(&agent, 3_000, 0x01, 2).is_ok());
        assert_eq!(store.epoch_spend(&agent, 2), 3_000);
        assert_eq!(store.epoch_spend(&agent, 1), 0);
    }

    #[test]
    fn check_spend_read_only_does_not_record() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        assert!(store.check_spend(&agent, 500, 0x01, 1).is_ok());
        assert_eq!(store.epoch_spend(&agent, 1), 0);
        assert!(store.check_spend(&agent, 500, 0x01, 1).is_ok());
        assert_eq!(store.epoch_spend(&agent, 1), 0);
    }

    #[test]
    fn check_spend_rejects_same_as_validate() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];
        store.set_policy(agent, test_policy([2u8; 32]));

        assert!(store.check_spend(&agent, 1_001, 0x01, 1).is_err());
        assert!(store.check_spend(&agent, 100, 0x06, 1).is_err());
        assert!(store.check_spend(&agent, 100, 0x01, 100).is_err());
    }

    #[test]
    fn validate_spend_no_policy() {
        let mut store = AgentPolicyStore::new();
        let agent = [1u8; 32];

        let err = store.validate_spend(&agent, 100, 0x01, 1).unwrap_err();
        assert!(matches!(err, AgentError::PolicyNotSet));
    }
}
