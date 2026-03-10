use std::collections::HashMap;

use serde::{Deserialize, Serialize};

type Address = [u8; 32];

const MIN_DESCRIPTION_LEN: usize = 10;
const MAX_DESCRIPTION_LEN: usize = 512;
const MAX_PARAM_KEY_LEN: usize = 64;
const MAX_PARAM_VALUE_LEN: usize = 128;
const MIN_VOTING_PERIOD: u64 = 10;
const MAX_VOTING_PERIOD: u64 = 10_000;
const MIN_VOTERS_FOR_QUORUM: usize = 2;
const MAX_ACTIVE_PROPOSALS: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: [u8; 32],
    pub proposer: Address,
    pub description: String,
    pub param_key: String,
    pub param_value: String,
    pub start_round: u64,
    pub end_round: u64,
    pub status: ProposalStatus,
    /// Balance snapshot taken at proposal creation. Voters are weighted
    /// by their balance at this point, preventing flash-loan attacks (S3-1).
    #[serde(default)]
    pub snapshot_balances: HashMap<Address, u128>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub voter: Address,
    pub proposal_id: [u8; 32],
    pub approve: bool,
    pub weight: u128,
}

#[derive(Clone, Debug)]
pub struct VoteTally {
    pub approve_weight: u128,
    pub reject_weight: u128,
    pub voter_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GovernanceError {
    ProposalNotFound,
    ProposalNotActive,
    AlreadyVoted,
    DescriptionTooShort,
    DescriptionTooLong,
    ParamKeyTooLong,
    ParamValueTooLong,
    InvalidVotingPeriod,
    TooManyActiveProposals,
    ZeroStakeWeight,
}

impl std::fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GovernanceError::ProposalNotFound => write!(f, "proposal not found"),
            GovernanceError::ProposalNotActive => write!(f, "proposal is not active"),
            GovernanceError::AlreadyVoted => write!(f, "voter has already voted on this proposal"),
            GovernanceError::DescriptionTooShort => {
                write!(f, "description too short (min {MIN_DESCRIPTION_LEN} chars)")
            }
            GovernanceError::DescriptionTooLong => {
                write!(f, "description too long (max {MAX_DESCRIPTION_LEN} chars)")
            }
            GovernanceError::ParamKeyTooLong => {
                write!(f, "param_key too long (max {MAX_PARAM_KEY_LEN} chars)")
            }
            GovernanceError::ParamValueTooLong => {
                write!(f, "param_value too long (max {MAX_PARAM_VALUE_LEN} chars)")
            }
            GovernanceError::InvalidVotingPeriod => {
                write!(
                    f,
                    "voting_period must be between {MIN_VOTING_PERIOD} and {MAX_VOTING_PERIOD}"
                )
            }
            GovernanceError::TooManyActiveProposals => {
                write!(f, "too many active proposals (max {MAX_ACTIVE_PROPOSALS})")
            }
            GovernanceError::ZeroStakeWeight => write!(f, "voter has zero stake weight"),
        }
    }
}

impl std::error::Error for GovernanceError {}

pub struct CreateProposalParams {
    pub id: [u8; 32],
    pub proposer: Address,
    pub description: String,
    pub param_key: String,
    pub param_value: String,
    pub current_round: u64,
    pub voting_period: u64,
    /// Balance snapshot at proposal creation time. Voters are weighted
    /// by this snapshot rather than live balances.
    pub snapshot_balances: HashMap<Address, u128>,
}

pub struct GovernanceStore {
    proposals: HashMap<[u8; 32], Proposal>,
    votes: HashMap<[u8; 32], Vec<Vote>>,
}

impl GovernanceStore {
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
            votes: HashMap::new(),
        }
    }

    pub fn create_proposal(&mut self, params: CreateProposalParams) -> Result<(), GovernanceError> {
        if params.description.len() < MIN_DESCRIPTION_LEN {
            return Err(GovernanceError::DescriptionTooShort);
        }
        if params.description.len() > MAX_DESCRIPTION_LEN {
            return Err(GovernanceError::DescriptionTooLong);
        }
        if params.param_key.len() > MAX_PARAM_KEY_LEN {
            return Err(GovernanceError::ParamKeyTooLong);
        }
        if params.param_value.len() > MAX_PARAM_VALUE_LEN {
            return Err(GovernanceError::ParamValueTooLong);
        }
        if !(MIN_VOTING_PERIOD..=MAX_VOTING_PERIOD).contains(&params.voting_period) {
            return Err(GovernanceError::InvalidVotingPeriod);
        }

        let active_count = self
            .proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Active)
            .count();
        if active_count >= MAX_ACTIVE_PROPOSALS {
            return Err(GovernanceError::TooManyActiveProposals);
        }

        let proposal = Proposal {
            id: params.id,
            proposer: params.proposer,
            description: params.description,
            param_key: params.param_key,
            param_value: params.param_value,
            start_round: params.current_round,
            end_round: params.current_round + params.voting_period,
            status: ProposalStatus::Active,
            snapshot_balances: params.snapshot_balances,
        };
        self.proposals.insert(params.id, proposal);
        self.votes.insert(params.id, Vec::new());
        Ok(())
    }

    /// Cast a vote. Weight is determined from the proposal's balance snapshot,
    /// not the caller-provided live balance. This prevents flash-loan attacks.
    pub fn cast_vote(
        &mut self,
        voter: Address,
        proposal_id: [u8; 32],
        approve: bool,
        _stake_weight: u128,
    ) -> Result<(), GovernanceError> {
        let proposal = self
            .proposals
            .get(&proposal_id)
            .ok_or(GovernanceError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(GovernanceError::ProposalNotActive);
        }

        let snapshot_weight = proposal.snapshot_balances.get(&voter).copied().unwrap_or(0);
        if snapshot_weight == 0 {
            return Err(GovernanceError::ZeroStakeWeight);
        }

        let votes = self.votes.entry(proposal_id).or_default();
        if votes.iter().any(|v| v.voter == voter) {
            return Err(GovernanceError::AlreadyVoted);
        }

        votes.push(Vote {
            voter,
            proposal_id,
            approve,
            weight: snapshot_weight,
        });
        Ok(())
    }

    pub fn get(&self, proposal_id: &[u8; 32]) -> Option<&Proposal> {
        self.proposals.get(proposal_id)
    }

    pub fn list(&self, status_filter: Option<&ProposalStatus>) -> Vec<&Proposal> {
        self.proposals
            .values()
            .filter(|p| status_filter.is_none_or(|s| p.status == *s))
            .collect()
    }

    pub fn tally(&self, proposal_id: &[u8; 32]) -> Option<VoteTally> {
        let votes = self.votes.get(proposal_id)?;
        let mut approve_weight = 0u128;
        let mut reject_weight = 0u128;
        for v in votes {
            if v.approve {
                approve_weight += v.weight;
            } else {
                reject_weight += v.weight;
            }
        }
        Some(VoteTally {
            approve_weight,
            reject_weight,
            voter_count: votes.len(),
        })
    }

    pub fn finalize_expired(&mut self, current_round: u64) -> Vec<Proposal> {
        let expired_ids: Vec<[u8; 32]> = self
            .proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Active && current_round >= p.end_round)
            .map(|p| p.id)
            .collect();

        let mut finalized = Vec::new();

        for id in expired_ids {
            let tally = self.tally(&id);
            let proposal = self.proposals.get_mut(&id).unwrap();

            let passed = tally.is_some_and(|t| {
                t.voter_count >= MIN_VOTERS_FOR_QUORUM && t.approve_weight > t.reject_weight
            });

            proposal.status = if passed {
                ProposalStatus::Passed
            } else {
                ProposalStatus::Rejected
            };

            finalized.push(proposal.clone());
        }

        finalized
    }

    pub fn mark_executed(&mut self, proposal_id: &[u8; 32]) -> bool {
        if let Some(p) = self.proposals.get_mut(proposal_id)
            && p.status == ProposalStatus::Passed
        {
            p.status = ProposalStatus::Executed;
            return true;
        }
        false
    }

    pub fn passed_unexecuted(&self) -> Vec<Proposal> {
        self.proposals
            .values()
            .filter(|p| p.status == ProposalStatus::Passed)
            .cloned()
            .collect()
    }

    pub fn votes_for(&self, proposal_id: &[u8; 32]) -> &[Vote] {
        self.votes
            .get(proposal_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

impl Default for GovernanceStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id(n: u8) -> [u8; 32] {
        [n; 32]
    }

    fn test_addr(n: u8) -> Address {
        [n; 32]
    }

    fn default_snapshot() -> HashMap<Address, u128> {
        let mut snap = HashMap::new();
        snap.insert(test_addr(0xB0), 1000);
        snap.insert(test_addr(0xB1), 500);
        snap.insert(test_addr(0xB2), 300);
        snap
    }

    fn test_proposal(id: [u8; 32], desc: &str, key: &str, val: &str) -> CreateProposalParams {
        CreateProposalParams {
            id,
            proposer: test_addr(0xA0),
            description: desc.to_string(),
            param_key: key.to_string(),
            param_value: val.to_string(),
            current_round: 100,
            voting_period: 50,
            snapshot_balances: default_snapshot(),
        }
    }

    #[test]
    fn proposal_lifecycle() {
        let mut store = GovernanceStore::new();
        let id = test_id(1);

        store
            .create_proposal(test_proposal(
                id,
                "Increase base fee floor to 5",
                "base_fee_floor",
                "5",
            ))
            .unwrap();

        let proposal = store.get(&id).unwrap();
        assert_eq!(proposal.status, ProposalStatus::Active);
        assert_eq!(proposal.start_round, 100);
        assert_eq!(proposal.end_round, 150);
        assert_eq!(proposal.proposer, test_addr(0xA0));
    }

    #[test]
    fn vote_tallying() {
        let mut store = GovernanceStore::new();
        let id = test_id(2);

        store
            .create_proposal(test_proposal(
                id,
                "Set max block range to 200",
                "max_block_range",
                "200",
            ))
            .unwrap();

        store.cast_vote(test_addr(0xB0), id, true, 1000).unwrap();
        store.cast_vote(test_addr(0xB1), id, true, 500).unwrap();
        store.cast_vote(test_addr(0xB2), id, false, 300).unwrap();

        let tally = store.tally(&id).unwrap();
        assert_eq!(tally.approve_weight, 1500);
        assert_eq!(tally.reject_weight, 300);
        assert_eq!(tally.voter_count, 3);
    }

    #[test]
    fn quorum_threshold() {
        let mut store = GovernanceStore::new();
        let id = test_id(3);

        store
            .create_proposal(test_proposal(
                id,
                "Single voter should not pass",
                "some_param",
                "42",
            ))
            .unwrap();

        store.cast_vote(test_addr(0xB0), id, true, 10_000).unwrap();

        let finalized = store.finalize_expired(200);
        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].status, ProposalStatus::Rejected);
    }

    #[test]
    fn double_vote_rejection() {
        let mut store = GovernanceStore::new();
        let id = test_id(4);

        store
            .create_proposal(test_proposal(
                id,
                "Double vote test proposal",
                "param",
                "val",
            ))
            .unwrap();

        store.cast_vote(test_addr(0xB0), id, true, 500).unwrap();

        let err = store
            .cast_vote(test_addr(0xB0), id, false, 500)
            .unwrap_err();
        assert_eq!(err, GovernanceError::AlreadyVoted);
    }

    #[test]
    fn expired_proposal_finalization() {
        let mut store = GovernanceStore::new();
        let id = test_id(5);

        store
            .create_proposal(test_proposal(
                id,
                "Proposal that should pass",
                "param",
                "val",
            ))
            .unwrap();

        store.cast_vote(test_addr(0xB0), id, true, 1000).unwrap();
        store.cast_vote(test_addr(0xB1), id, true, 500).unwrap();

        let finalized = store.finalize_expired(150);
        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].status, ProposalStatus::Passed);

        assert!(store.mark_executed(&id));
        assert_eq!(store.get(&id).unwrap().status, ProposalStatus::Executed);
    }

    #[test]
    fn snapshot_weight_used_not_live_balance() {
        let mut store = GovernanceStore::new();
        let id = test_id(10);

        store
            .create_proposal(test_proposal(
                id,
                "Snapshot weight test proposal",
                "param",
                "val",
            ))
            .unwrap();

        // B0 has 1000 in snapshot — passing 9999 as live weight should be ignored
        store.cast_vote(test_addr(0xB0), id, true, 9999).unwrap();
        let tally = store.tally(&id).unwrap();
        assert_eq!(tally.approve_weight, 1000); // snapshot weight, not 9999
    }

    #[test]
    fn voter_not_in_snapshot_rejected() {
        let mut store = GovernanceStore::new();
        let id = test_id(11);

        store
            .create_proposal(test_proposal(
                id,
                "No-snapshot voter test proposal",
                "param",
                "val",
            ))
            .unwrap();

        // 0xCC is not in the snapshot
        let err = store
            .cast_vote(test_addr(0xCC), id, true, 5000)
            .unwrap_err();
        assert_eq!(err, GovernanceError::ZeroStakeWeight);
    }

    #[test]
    fn voter_retains_original_snapshot_weight() {
        let mut store = GovernanceStore::new();
        let id = test_id(12);

        store
            .create_proposal(test_proposal(
                id,
                "Retained weight test proposal",
                "param",
                "val",
            ))
            .unwrap();

        // B1 had 500 at snapshot — even if live balance changes, vote uses 500
        store.cast_vote(test_addr(0xB1), id, true, 0).unwrap();
        let tally = store.tally(&id).unwrap();
        assert_eq!(tally.approve_weight, 500);
    }

    #[test]
    fn empty_store() {
        let store = GovernanceStore::new();
        assert!(store.get(&test_id(99)).is_none());
        assert!(store.list(None).is_empty());
        assert!(store.tally(&test_id(99)).is_none());
        assert_eq!(store.votes_for(&test_id(99)).len(), 0);
    }
}
