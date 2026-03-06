use std::collections::HashMap;

use dendrite_core::ValidatorId;
use serde::{Deserialize, Serialize};

/// A single validator's record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Unique identifier (public key bytes).
    pub id: ValidatorId,
    /// Stake in the smallest token unit.
    pub stake: u64,
}

/// Manages the active validator set with stake-weighted operations.
///
/// Validators are identified by their 32-byte public key. Each validator
/// has a stake that determines their weight in consensus voting and
/// leader selection.
#[derive(Clone, Debug, Default)]
pub struct ValidatorSet {
    validators: HashMap<ValidatorId, u64>,
    total_stake: u64,
}

impl ValidatorSet {
    /// Create an empty validator set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validator or update their stake.
    /// Returns the previous stake if the validator was already present.
    pub fn add(&mut self, id: ValidatorId, stake: u64) -> Option<u64> {
        let old = self.validators.insert(id, stake);
        if let Some(old_stake) = old {
            // Updating: subtract old, add new.
            self.total_stake = self.total_stake - old_stake + stake;
        } else {
            self.total_stake += stake;
        }
        old
    }

    /// Remove a validator. Returns their stake if they existed.
    pub fn remove(&mut self, id: &ValidatorId) -> Option<u64> {
        if let Some(stake) = self.validators.remove(id) {
            self.total_stake -= stake;
            Some(stake)
        } else {
            None
        }
    }

    /// Look up a validator's stake. Returns `None` if not in the set.
    pub fn get(&self, id: &ValidatorId) -> Option<u64> {
        self.validators.get(id).copied()
    }

    /// Check if a validator is in the set.
    pub fn contains(&self, id: &ValidatorId) -> bool {
        self.validators.contains_key(id)
    }

    /// Number of validators.
    pub fn len(&self) -> usize {
        self.validators.len()
    }

    /// Check if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.validators.is_empty()
    }

    /// Total stake across all validators.
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }

    /// Check if a set of validators holds a supermajority (>2/3 of total stake).
    /// This is the BFT threshold for consensus.
    pub fn has_supermajority(&self, voter_ids: &[ValidatorId]) -> bool {
        let voting_stake: u64 = voter_ids
            .iter()
            .filter_map(|id| self.validators.get(id))
            .sum();
        // >2/3 means voting_stake * 3 > total_stake * 2
        voting_stake * 3 > self.total_stake * 2
    }

    /// Select a leader for a given round using stake-weighted deterministic selection.
    /// Returns `None` if the set is empty.
    pub fn leader_for_round(&self, round: u64) -> Option<ValidatorId> {
        if self.validators.is_empty() {
            return None;
        }

        // Sort validators by ID for deterministic ordering.
        let mut sorted: Vec<(ValidatorId, u64)> = self
            .validators
            .iter()
            .map(|(id, stake)| (*id, *stake))
            .collect();
        sorted.sort_by_key(|(id, _)| *id);

        // Stake-weighted round-robin: map round to a position in [0, total_stake).
        let position = round % self.total_stake;
        let mut cumulative = 0u64;
        for (id, stake) in &sorted {
            cumulative += stake;
            if position < cumulative {
                return Some(*id);
            }
        }

        // Fallback (should not happen if total_stake > 0).
        Some(sorted[0].0)
    }

    /// Minimum number of validators needed for a quorum (2f+1).
    /// With n validators and f = (n-1)/3 faulty, quorum = n - f.
    pub fn quorum_count(&self) -> usize {
        let n = self.validators.len();
        if n == 0 {
            return 0;
        }
        let f = (n - 1) / 3;
        n - f
    }

    /// Iterate over all validators as (id, stake) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ValidatorId, &u64)> {
        self.validators.iter()
    }
}
