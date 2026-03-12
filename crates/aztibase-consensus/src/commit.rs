use aztibase_core::{BlockHash, ValidatorId};

use crate::dag_store::DagStore;
use crate::validator::ValidatorSet;

/// The outcome of evaluating a leader's commit status.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LeaderStatus {
    /// The leader block was committed (hash of the committed block).
    Commit(BlockHash),
    /// The leader was skipped (no block can be committed for this round).
    Skip(u64),
    /// Not enough information yet to decide.
    Undecided(u64),
}

/// Configuration for the commit rule.
#[derive(Clone, Debug)]
pub struct CommitConfig {
    /// Number of rounds per wave (leader, voting, decision).
    pub wave_length: u64,
    /// VRF seed for leader election. When set, uses BLAKE3 PRF instead of
    /// deterministic round-robin.
    pub vrf_seed: Option<[u8; 32]>,
}

impl Default for CommitConfig {
    fn default() -> Self {
        Self {
            wave_length: 4,
            vrf_seed: None,
        }
    }
}

/// Evaluates whether leaders can be committed using direct and indirect rules.
///
/// The commit rule operates in "waves" of `wave_length` rounds:
/// - Round `w * wave_length`: leader round (one validator proposes)
/// - Round `w * wave_length + 1`: voting round (validators reference the leader or not)
/// - Round `w * wave_length + wave_length - 1`: decision round (certificates form)
///
/// A leader is **directly committed** if enough decision-round blocks form
/// certificates (chains of votes reaching supermajority).
///
/// A leader is **indirectly committed** if a later committed leader has it
/// in its causal history.
pub struct CommitRule<'a> {
    dag: &'a DagStore,
    validators: &'a ValidatorSet,
    config: CommitConfig,
}

impl<'a> CommitRule<'a> {
    pub fn new(dag: &'a DagStore, validators: &'a ValidatorSet, config: CommitConfig) -> Self {
        Self {
            dag,
            validators,
            config,
        }
    }

    /// Determine the leader for a given round. Uses VRF-based selection when
    /// a seed is configured, falling back to deterministic round-robin.
    pub fn leader_for_round(&self, round: u64) -> Option<ValidatorId> {
        match &self.config.vrf_seed {
            Some(seed) => self.validators.vrf_leader_for_round(round, seed),
            None => self.validators.leader_for_round(round),
        }
    }

    /// Find the leader block at the given round (the block authored by the elected leader).
    pub fn leader_block(&self, round: u64) -> Option<BlockHash> {
        let leader_id = self.leader_for_round(round)?;
        self.dag
            .blocks_at_round(round)
            .iter()
            .find(|hash| {
                self.dag
                    .get(hash)
                    .map(|b| b.author == leader_id)
                    .unwrap_or(false)
            })
            .copied()
    }

    /// Try to directly commit a leader at the given wave.
    ///
    /// Direct commit: the leader round block has enough support from
    /// voting round blocks, certified by decision round blocks.
    /// Simplified: we check if >2/3 of stake at the voting round
    /// references the leader block.
    pub fn try_direct_commit(&self, wave: u64) -> LeaderStatus {
        let leader_round = wave * self.config.wave_length;
        let voting_round = leader_round + 1;

        let leader_hash = match self.leader_block(leader_round) {
            Some(h) => h,
            None => return LeaderStatus::Skip(leader_round),
        };

        let voting_blocks = self.dag.blocks_at_round(voting_round);
        if voting_blocks.is_empty() {
            return LeaderStatus::Undecided(leader_round);
        }

        let mut supporters = Vec::new();
        let mut non_supporters = Vec::new();

        for &vh in voting_blocks {
            let block = match self.dag.get(&vh) {
                Ok(b) => b,
                Err(_) => continue,
            };
            if block.parents.contains(&leader_hash) {
                supporters.push(block.author);
            } else {
                non_supporters.push(block.author);
            }
        }

        if self.validators.has_supermajority(&supporters) {
            return LeaderStatus::Commit(leader_hash);
        }

        if self.validators.has_supermajority(&non_supporters) {
            return LeaderStatus::Skip(leader_round);
        }

        LeaderStatus::Undecided(leader_round)
    }

    /// Try to indirectly commit a leader using a later committed leader as anchor.
    ///
    /// If a committed anchor leader has the target leader in its causal history,
    /// the target leader is also committed.
    pub fn try_indirect_commit(&self, wave: u64, anchor: &BlockHash) -> LeaderStatus {
        let leader_round = wave * self.config.wave_length;
        let leader_hash = match self.leader_block(leader_round) {
            Some(h) => h,
            None => return LeaderStatus::Skip(leader_round),
        };

        if self.dag.is_ancestor(&leader_hash, anchor) {
            LeaderStatus::Commit(leader_hash)
        } else {
            LeaderStatus::Skip(leader_round)
        }
    }

    /// Run the full commit sequence for waves up to `max_wave`.
    /// Returns the list of committed leader block hashes in wave order.
    pub fn try_commit(&self, max_wave: u64) -> Vec<LeaderStatus> {
        let mut results = Vec::new();
        let mut last_committed_anchor: Option<BlockHash> = None;

        for wave in 0..=max_wave {
            let status = self.try_direct_commit(wave);
            let final_status = match &status {
                LeaderStatus::Commit(h) => {
                    last_committed_anchor = Some(*h);
                    status
                }
                LeaderStatus::Skip(_) => status,
                LeaderStatus::Undecided(_) => {
                    if let Some(anchor) = &last_committed_anchor {
                        let indirect = self.try_indirect_commit(wave, anchor);
                        if let LeaderStatus::Commit(h) = &indirect {
                            last_committed_anchor = Some(*h);
                        }
                        indirect
                    } else {
                        status
                    }
                }
            };
            results.push(final_status);
        }

        results
    }
}
