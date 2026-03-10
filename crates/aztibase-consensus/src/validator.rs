use std::collections::HashMap;

use aztibase_core::{BlsPublicKey, ValidatorId, hash};
use serde::{Deserialize, Serialize};

/// A single validator's record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Unique identifier (public key bytes).
    pub id: ValidatorId,
    /// Stake in the smallest token unit.
    pub stake: u128,
    /// Optional BLS public key for finality signatures.
    #[serde(skip)]
    pub bls_pubkey: Option<BlsPublicKey>,
}

#[derive(Clone, Debug)]
struct ValidatorRecord {
    stake: u128,
    bls_pubkey: Option<BlsPublicKey>,
}

/// Manages the active validator set with stake-weighted operations.
///
/// Validators are identified by their 32-byte public key. Each validator
/// has a stake that determines their weight in consensus voting and
/// leader selection.
#[derive(Clone, Debug, Default)]
pub struct ValidatorSet {
    validators: HashMap<ValidatorId, ValidatorRecord>,
    total_stake: u128,
}

impl ValidatorSet {
    /// Create an empty validator set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a validator or update their stake.
    /// Returns the previous stake if the validator was already present.
    pub fn add(&mut self, id: ValidatorId, stake: u128) -> Option<u128> {
        self.add_with_bls(id, stake, None)
    }

    /// Add a validator with an optional BLS public key.
    pub fn add_with_bls(
        &mut self,
        id: ValidatorId,
        stake: u128,
        bls_pubkey: Option<BlsPublicKey>,
    ) -> Option<u128> {
        let old = self
            .validators
            .insert(id, ValidatorRecord { stake, bls_pubkey });
        if let Some(old_rec) = &old {
            self.total_stake = self.total_stake - old_rec.stake + stake;
        } else {
            self.total_stake += stake;
        }
        old.map(|r| r.stake)
    }

    /// Remove a validator. Returns their stake if they existed.
    pub fn remove(&mut self, id: &ValidatorId) -> Option<u128> {
        if let Some(rec) = self.validators.remove(id) {
            self.total_stake -= rec.stake;
            Some(rec.stake)
        } else {
            None
        }
    }

    /// Look up a validator's stake. Returns `None` if not in the set.
    pub fn get(&self, id: &ValidatorId) -> Option<u128> {
        self.validators.get(id).map(|r| r.stake)
    }

    /// Look up a validator's BLS public key.
    pub fn bls_key(&self, id: &ValidatorId) -> Option<&BlsPublicKey> {
        self.validators.get(id).and_then(|r| r.bls_pubkey.as_ref())
    }

    /// Get ordered BLS public keys for all validators that have one.
    /// Sorted by validator ID for deterministic ordering.
    pub fn bls_keys_ordered(&self) -> Vec<(ValidatorId, BlsPublicKey)> {
        let mut pairs: Vec<_> = self
            .validators
            .iter()
            .filter_map(|(id, rec)| rec.bls_pubkey.as_ref().map(|k| (*id, k.clone())))
            .collect();
        pairs.sort_by_key(|(id, _)| *id);
        pairs
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
    pub fn total_stake(&self) -> u128 {
        self.total_stake
    }

    /// Check if a set of validators meets the quorum threshold (>=2/3 of total stake).
    /// Used by the threshold clock to gate round advancement.
    pub fn has_quorum(&self, voter_ids: &[ValidatorId]) -> bool {
        let voting_stake: u128 = voter_ids
            .iter()
            .filter_map(|id| self.validators.get(id).map(|r| r.stake))
            .sum();
        voting_stake * 3 >= self.total_stake * 2
    }

    /// Check if a set of validators holds a supermajority (>2/3 of total stake).
    /// This is the BFT threshold for consensus commits.
    pub fn has_supermajority(&self, voter_ids: &[ValidatorId]) -> bool {
        let voting_stake: u128 = voter_ids
            .iter()
            .filter_map(|id| self.validators.get(id).map(|r| r.stake))
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
        let mut sorted: Vec<(ValidatorId, u128)> = self
            .validators
            .iter()
            .map(|(id, rec)| (*id, rec.stake))
            .collect();
        sorted.sort_by_key(|(id, _)| *id);

        // Stake-weighted round-robin: map round to a position in [0, total_stake).
        let position = round as u128 % self.total_stake;
        let mut cumulative = 0u128;
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

    /// Select a leader using a VRF-like BLAKE3 PRF: hash(round || seed) mapped
    /// to a stake-weighted position. The seed should be derived from the
    /// previous anchor hash to prevent pre-computation beyond one wave.
    pub fn vrf_leader_for_round(&self, round: u64, seed: &[u8; 32]) -> Option<ValidatorId> {
        if self.validators.is_empty() {
            return None;
        }

        let mut preimage = Vec::with_capacity(40);
        preimage.extend_from_slice(&round.to_le_bytes());
        preimage.extend_from_slice(seed);
        let vrf_hash = hash(&preimage);

        let mut sorted: Vec<(ValidatorId, u128)> = self
            .validators
            .iter()
            .map(|(id, rec)| (*id, rec.stake))
            .collect();
        sorted.sort_by_key(|(id, _)| *id);

        let position = u128::from_le_bytes(vrf_hash[..16].try_into().unwrap()) % self.total_stake;
        let mut cumulative = 0u128;
        for (id, stake) in &sorted {
            cumulative += stake;
            if position < cumulative {
                return Some(*id);
            }
        }

        Some(sorted[0].0)
    }

    /// Iterate over all validators as (id, stake) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&ValidatorId, u128)> + '_ {
        self.validators.iter().map(|(id, rec)| (id, rec.stake))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::BlsKeypair;

    #[test]
    fn validator_set_with_bls_keys() {
        let mut vs = ValidatorSet::new();
        let bls1 = BlsKeypair::generate();
        let bls2 = BlsKeypair::generate();

        vs.add_with_bls([1u8; 32], 100, Some(bls1.public_key().clone()));
        vs.add_with_bls([2u8; 32], 200, Some(bls2.public_key().clone()));
        vs.add([3u8; 32], 100);

        assert_eq!(vs.len(), 3);
        assert_eq!(vs.bls_key(&[1u8; 32]), Some(bls1.public_key()));
        assert_eq!(vs.bls_key(&[2u8; 32]), Some(bls2.public_key()));
        assert!(vs.bls_key(&[3u8; 32]).is_none());
    }

    #[test]
    fn bls_keys_ordering() {
        let mut vs = ValidatorSet::new();
        let bls_a = BlsKeypair::generate();
        let bls_b = BlsKeypair::generate();

        // Insert in reverse order
        vs.add_with_bls([2u8; 32], 100, Some(bls_b.public_key().clone()));
        vs.add_with_bls([1u8; 32], 100, Some(bls_a.public_key().clone()));

        let ordered = vs.bls_keys_ordered();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].0, [1u8; 32]);
        assert_eq!(ordered[1].0, [2u8; 32]);
    }
}
