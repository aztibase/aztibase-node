use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

type ValidatorId = [u8; 32];

/// Per-validator performance snapshot for a single epoch.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorProfile {
    pub validator_id: ValidatorId,
    pub vertices_proposed: u64,
    pub vertices_expected: u64,
    pub participation_rate: f32,
    pub equivocations: u64,
    pub empty_payloads: u64,
    pub total_payloads: u64,
    pub avg_payload_bytes: f32,
    pub rewards_earned: u128,
    pub amount_slashed: u128,
    pub reputation_score: f32,
}

/// Chain-level summary for a single epoch, including per-validator profiles.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochSummary {
    pub epoch: u64,
    pub start_batch: u64,
    pub end_batch: u64,
    pub duration_ms: u64,
    pub avg_health_score: f32,
    pub max_health_score: f32,
    pub warning_ticks: u64,
    pub critical_ticks: u64,
    pub actions_taken: u64,
    pub avg_tps: f32,
    pub avg_commit_latency_ms: f32,
    pub equivocations_total: u64,
    pub anomalous_tx_count: u64,
    pub validator_profiles: Vec<ValidatorProfile>,
}

/// Persistent Sentinel memory surviving node restarts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SentinelMemory {
    pub epoch_summaries: VecDeque<EpochSummary>,
    pub last_action_batch: Option<u64>,
    pub last_alert_batch: Option<u64>,
}

const MAX_EPOCH_SUMMARIES: usize = 50;

impl SentinelMemory {
    pub fn push_epoch(&mut self, summary: EpochSummary) {
        if self.epoch_summaries.len() >= MAX_EPOCH_SUMMARIES {
            self.epoch_summaries.pop_front();
        }
        self.epoch_summaries.push_back(summary);
    }

    pub fn latest_epoch(&self) -> Option<&EpochSummary> {
        self.epoch_summaries.back()
    }

    pub fn recent(&self, limit: usize) -> Vec<&EpochSummary> {
        let take = limit.min(self.epoch_summaries.len());
        self.epoch_summaries.iter().rev().take(take).collect()
    }

    pub fn get_epoch(&self, epoch: u64) -> Option<&EpochSummary> {
        self.epoch_summaries.iter().find(|s| s.epoch == epoch)
    }

    pub fn get_validator_profile(&self, validator_id: &ValidatorId) -> Option<&ValidatorProfile> {
        self.epoch_summaries
            .back()?
            .validator_profiles
            .iter()
            .find(|p| &p.validator_id == validator_id)
    }
}

// ── Profile Accumulator ──────────────────────────────────────────────

struct VertexStats {
    count: u64,
    total_payload_bytes: u64,
    empty_count: u64,
}

/// Collects per-validator vertex data during an epoch.
/// Call `record_vertex` for each vertex observed, then `finalize` at epoch boundary.
#[derive(Default)]
pub struct ProfileAccumulator {
    vertex_stats: HashMap<ValidatorId, VertexStats>,
    epoch_start_ms: u64,
    total_rounds: u64,
}

impl ProfileAccumulator {
    pub fn new() -> Self {
        Self {
            vertex_stats: HashMap::new(),
            epoch_start_ms: now_ms(),
            total_rounds: 0,
        }
    }

    pub fn record_vertex(&mut self, author: ValidatorId, payload_len: u64) {
        let entry = self.vertex_stats.entry(author).or_insert(VertexStats {
            count: 0,
            total_payload_bytes: 0,
            empty_count: 0,
        });
        entry.count += 1;
        entry.total_payload_bytes += payload_len;
        if payload_len == 0 {
            entry.empty_count += 1;
        }
    }

    pub fn set_total_rounds(&mut self, rounds: u64) {
        self.total_rounds = rounds;
    }

    /// Merge externally-collected vertex counts (from shared consensus state).
    pub fn merge_vertex_counts(&mut self, counts: HashMap<ValidatorId, (u64, u64)>) {
        for (vid, (count, bytes)) in counts {
            let entry = self.vertex_stats.entry(vid).or_insert(VertexStats {
                count: 0,
                total_payload_bytes: 0,
                empty_count: 0,
            });
            entry.count += count;
            entry.total_payload_bytes += bytes;
        }
    }

    /// Produce final profiles for all validators in the active set.
    pub fn finalize(
        &mut self,
        active_set: &[([u8; 32], u128)],
        credits: &[([u8; 32], u128)],
        slashed_validators: &[[u8; 32]],
        slash_amounts: &HashMap<[u8; 32], u128>,
        equivocation_counts: &HashMap<[u8; 32], u64>,
    ) -> Vec<ValidatorProfile> {
        let duration_ms = now_ms().saturating_sub(self.epoch_start_ms);
        let expected_per_validator = self.total_rounds.max(1);

        let credit_map: HashMap<[u8; 32], u128> = credits.iter().copied().collect();
        let slashed_set: std::collections::HashSet<[u8; 32]> =
            slashed_validators.iter().copied().collect();

        let profiles: Vec<ValidatorProfile> = active_set
            .iter()
            .map(|(vid, _stake)| {
                let stats = self.vertex_stats.get(vid);
                let proposed = stats.map_or(0, |s| s.count);
                let total_bytes = stats.map_or(0, |s| s.total_payload_bytes);
                let empty = stats.map_or(0, |s| s.empty_count);
                let total_payloads = proposed;

                let participation_rate = if expected_per_validator > 0 {
                    (proposed as f32 / expected_per_validator as f32).min(1.0)
                } else {
                    0.0
                };

                let avg_payload_bytes = if proposed > 0 {
                    total_bytes as f32 / proposed as f32
                } else {
                    0.0
                };

                let rewards_earned = credit_map.get(vid).copied().unwrap_or(0);
                let amount_slashed = if slashed_set.contains(vid) {
                    slash_amounts.get(vid).copied().unwrap_or(0)
                } else {
                    0
                };
                let equivocations = equivocation_counts.get(vid).copied().unwrap_or(0);

                let reputation_score =
                    compute_reputation(participation_rate, equivocations, amount_slashed > 0);

                ValidatorProfile {
                    validator_id: *vid,
                    vertices_proposed: proposed,
                    vertices_expected: expected_per_validator,
                    participation_rate,
                    equivocations,
                    empty_payloads: empty,
                    total_payloads,
                    avg_payload_bytes,
                    rewards_earned,
                    amount_slashed,
                    reputation_score,
                }
            })
            .collect();

        self.vertex_stats.clear();
        self.epoch_start_ms = now_ms();
        self.total_rounds = 0;

        let _ = duration_ms; // used later for EpochSummary.duration_ms
        profiles
    }
}

/// Weighted composite reputation in [0.0, 1.0].
///
/// Weights:
/// - 60% participation (0.0 if absent, 1.0 if perfect)
/// - 25% equivocation penalty (each equivocation costs 0.25, capped at full penalty)
/// - 15% slashing penalty (binary: slashed this epoch = -0.15)
fn compute_reputation(participation_rate: f32, equivocations: u64, was_slashed: bool) -> f32 {
    let participation_component = participation_rate * 0.60;
    let equivocation_penalty = (equivocations as f32 * 0.25).min(0.25);
    let slash_penalty: f32 = if was_slashed { 0.15 } else { 0.0 };
    (participation_component + 0.25 - equivocation_penalty + 0.15 - slash_penalty).clamp(0.0, 1.0)
}

// ── Pattern Detector ─────────────────────────────────────────────────

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternAlert {
    ValidatorDegrading {
        validator_id: ValidatorId,
        recent_rates: Vec<f32>,
    },
    LatencyCreep {
        recent_latencies_ms: Vec<f32>,
    },
    EpochBoundaryStress {
        recent_max_scores: Vec<f32>,
    },
    StakeCentralization {
        validator_id: ValidatorId,
        pct: f32,
    },
    HealthDeteriorating {
        recent_avg_scores: Vec<f32>,
    },
}

pub struct PatternDetector {
    min_epochs: usize,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self { min_epochs: 3 }
    }
}

impl PatternDetector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn detect(&self, summaries: &VecDeque<EpochSummary>) -> Vec<PatternAlert> {
        if summaries.len() < self.min_epochs {
            return Vec::new();
        }

        let mut alerts = Vec::new();

        self.check_validator_degradation(summaries, &mut alerts);
        self.check_latency_creep(summaries, &mut alerts);
        self.check_epoch_boundary_stress(summaries, &mut alerts);
        self.check_stake_centralization(summaries, &mut alerts);
        self.check_health_deterioration(summaries, &mut alerts);

        alerts
    }

    fn check_validator_degradation(
        &self,
        summaries: &VecDeque<EpochSummary>,
        alerts: &mut Vec<PatternAlert>,
    ) {
        let recent: Vec<&EpochSummary> = summaries.iter().rev().take(5).collect();
        if recent.len() < 3 {
            return;
        }

        let mut all_validators: std::collections::HashSet<ValidatorId> =
            std::collections::HashSet::new();
        for s in &recent {
            for p in &s.validator_profiles {
                all_validators.insert(p.validator_id);
            }
        }

        for vid in all_validators {
            let rates: Vec<f32> = recent
                .iter()
                .rev()
                .filter_map(|s| {
                    s.validator_profiles
                        .iter()
                        .find(|p| p.validator_id == vid)
                        .map(|p| p.participation_rate)
                })
                .collect();

            if rates.len() >= 3 && is_declining(&rates) {
                alerts.push(PatternAlert::ValidatorDegrading {
                    validator_id: vid,
                    recent_rates: rates,
                });
            }
        }
    }

    fn check_latency_creep(
        &self,
        summaries: &VecDeque<EpochSummary>,
        alerts: &mut Vec<PatternAlert>,
    ) {
        let latencies: Vec<f32> = summaries
            .iter()
            .rev()
            .take(5)
            .map(|s| s.avg_commit_latency_ms)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        if latencies.len() >= 4 && is_increasing_by_pct(&latencies, 0.05) {
            alerts.push(PatternAlert::LatencyCreep {
                recent_latencies_ms: latencies,
            });
        }
    }

    fn check_epoch_boundary_stress(
        &self,
        summaries: &VecDeque<EpochSummary>,
        alerts: &mut Vec<PatternAlert>,
    ) {
        let max_scores: Vec<f32> = summaries
            .iter()
            .rev()
            .take(5)
            .map(|s| s.max_health_score)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let high_count = max_scores.iter().filter(|&&s| s > 0.5).count();
        if high_count >= 3 {
            alerts.push(PatternAlert::EpochBoundaryStress {
                recent_max_scores: max_scores,
            });
        }
    }

    fn check_stake_centralization(
        &self,
        summaries: &VecDeque<EpochSummary>,
        alerts: &mut Vec<PatternAlert>,
    ) {
        let latest = match summaries.back() {
            Some(s) => s,
            None => return,
        };

        let total_rewards: u128 = latest
            .validator_profiles
            .iter()
            .map(|p| p.rewards_earned)
            .sum();

        if total_rewards == 0 {
            return;
        }

        for p in &latest.validator_profiles {
            let pct = p.rewards_earned as f64 / total_rewards as f64;
            if pct > 0.33 && latest.validator_profiles.len() > 1 {
                alerts.push(PatternAlert::StakeCentralization {
                    validator_id: p.validator_id,
                    pct: pct as f32,
                });
            }
        }
    }

    fn check_health_deterioration(
        &self,
        summaries: &VecDeque<EpochSummary>,
        alerts: &mut Vec<PatternAlert>,
    ) {
        let avg_scores: Vec<f32> = summaries
            .iter()
            .rev()
            .take(5)
            .map(|s| s.avg_health_score)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        if avg_scores.len() >= 5 && is_increasing_by_pct(&avg_scores, 0.0) {
            alerts.push(PatternAlert::HealthDeteriorating {
                recent_avg_scores: avg_scores,
            });
        }
    }
}

/// Returns true if each element is strictly less than its successor.
fn is_declining(values: &[f32]) -> bool {
    values.windows(2).all(|w| w[0] > w[1])
}

/// Returns true if each element is at least `min_pct` larger than the previous.
fn is_increasing_by_pct(values: &[f32], min_pct: f32) -> bool {
    values.windows(2).all(|w| {
        if w[0] == 0.0 {
            w[1] > 0.0
        } else {
            (w[1] - w[0]) / w[0].abs() >= min_pct
        }
    })
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_vid(n: u8) -> ValidatorId {
        let mut id = [0u8; 32];
        id[0] = n;
        id
    }

    fn make_profile(vid: ValidatorId, participation: f32, rewards: u128) -> ValidatorProfile {
        ValidatorProfile {
            validator_id: vid,
            vertices_proposed: (participation * 100.0) as u64,
            vertices_expected: 100,
            participation_rate: participation,
            equivocations: 0,
            empty_payloads: 0,
            total_payloads: (participation * 100.0) as u64,
            avg_payload_bytes: 500.0,
            rewards_earned: rewards,
            amount_slashed: 0,
            reputation_score: compute_reputation(participation, 0, false),
        }
    }

    fn make_summary(epoch: u64, profiles: Vec<ValidatorProfile>) -> EpochSummary {
        EpochSummary {
            epoch,
            start_batch: epoch * 1000,
            end_batch: (epoch + 1) * 1000,
            duration_ms: 400_000,
            avg_health_score: 0.1,
            max_health_score: 0.2,
            warning_ticks: 0,
            critical_ticks: 0,
            actions_taken: 0,
            avg_tps: 10.0,
            avg_commit_latency_ms: 200.0,
            equivocations_total: 0,
            anomalous_tx_count: 0,
            validator_profiles: profiles,
        }
    }

    #[test]
    fn accumulator_record_and_finalize() {
        let mut acc = ProfileAccumulator::new();
        let v1 = test_vid(1);
        let v2 = test_vid(2);

        for _ in 0..80 {
            acc.record_vertex(v1, 500);
        }
        for _ in 0..100 {
            acc.record_vertex(v2, 300);
        }
        acc.set_total_rounds(100);

        let active_set = vec![(v1, 100_000u128), (v2, 200_000u128)];
        let credits = vec![(v1, 500u128), (v2, 1000u128)];
        let profiles = acc.finalize(&active_set, &credits, &[], &HashMap::new(), &HashMap::new());

        assert_eq!(profiles.len(), 2);
        let p1 = profiles.iter().find(|p| p.validator_id == v1).unwrap();
        assert_eq!(p1.vertices_proposed, 80);
        assert!((p1.participation_rate - 0.80).abs() < 0.01);
        assert_eq!(p1.rewards_earned, 500);

        let p2 = profiles.iter().find(|p| p.validator_id == v2).unwrap();
        assert_eq!(p2.vertices_proposed, 100);
        assert!((p2.participation_rate - 1.0).abs() < 0.01);
    }

    #[test]
    fn accumulator_empty_payloads_tracked() {
        let mut acc = ProfileAccumulator::new();
        let v1 = test_vid(1);
        acc.record_vertex(v1, 0);
        acc.record_vertex(v1, 100);
        acc.record_vertex(v1, 0);
        acc.set_total_rounds(10);

        let profiles = acc.finalize(&[(v1, 100_000)], &[], &[], &HashMap::new(), &HashMap::new());
        let p = &profiles[0];
        assert_eq!(p.empty_payloads, 2);
        assert_eq!(p.total_payloads, 3);
    }

    #[test]
    fn accumulator_merge_vertex_counts() {
        let mut acc = ProfileAccumulator::new();
        let v1 = test_vid(1);
        acc.record_vertex(v1, 100);

        let mut external = HashMap::new();
        external.insert(v1, (5u64, 2500u64));
        acc.merge_vertex_counts(external);
        acc.set_total_rounds(10);

        let profiles = acc.finalize(&[(v1, 100_000)], &[], &[], &HashMap::new(), &HashMap::new());
        assert_eq!(profiles[0].vertices_proposed, 6);
    }

    #[test]
    fn accumulator_slashed_validator() {
        let mut acc = ProfileAccumulator::new();
        let v1 = test_vid(1);
        acc.record_vertex(v1, 100);
        acc.set_total_rounds(10);

        let mut slash_amounts = HashMap::new();
        slash_amounts.insert(v1, 5000u128);

        let profiles = acc.finalize(
            &[(v1, 100_000)],
            &[],
            &[v1],
            &slash_amounts,
            &HashMap::new(),
        );
        assert_eq!(profiles[0].amount_slashed, 5000);
        assert!(profiles[0].reputation_score < 0.9);
    }

    #[test]
    fn reputation_perfect_validator() {
        let score = compute_reputation(1.0, 0, false);
        assert!((score - 1.0).abs() < 0.01);
    }

    #[test]
    fn reputation_absent_validator() {
        let score = compute_reputation(0.0, 0, false);
        assert!((score - 0.40).abs() < 0.01);
    }

    #[test]
    fn reputation_equivocating_validator() {
        let score = compute_reputation(1.0, 1, false);
        assert!(score < 1.0);
        assert!(score > 0.5);
    }

    #[test]
    fn reputation_slashed_validator() {
        let score = compute_reputation(1.0, 0, true);
        assert!((score - 0.85).abs() < 0.01);
    }

    #[test]
    fn sentinel_memory_push_and_cap() {
        let mut mem = SentinelMemory::default();
        for i in 0..60 {
            mem.push_epoch(make_summary(i, vec![]));
        }
        assert_eq!(mem.epoch_summaries.len(), MAX_EPOCH_SUMMARIES);
        assert_eq!(mem.epoch_summaries.front().unwrap().epoch, 10);
        assert_eq!(mem.epoch_summaries.back().unwrap().epoch, 59);
    }

    #[test]
    fn sentinel_memory_get_epoch() {
        let mut mem = SentinelMemory::default();
        mem.push_epoch(make_summary(5, vec![]));
        mem.push_epoch(make_summary(6, vec![]));
        assert!(mem.get_epoch(5).is_some());
        assert!(mem.get_epoch(7).is_none());
    }

    #[test]
    fn sentinel_memory_get_validator_profile() {
        let mut mem = SentinelMemory::default();
        let v1 = test_vid(1);
        let profile = make_profile(v1, 0.95, 1000);
        mem.push_epoch(make_summary(0, vec![profile]));
        assert!(mem.get_validator_profile(&v1).is_some());
        assert!(mem.get_validator_profile(&test_vid(2)).is_none());
    }

    #[test]
    fn sentinel_memory_serialization_roundtrip() {
        let mut mem = SentinelMemory::default();
        let v1 = test_vid(1);
        mem.push_epoch(make_summary(0, vec![make_profile(v1, 0.9, 500)]));
        mem.last_action_batch = Some(42);
        mem.last_alert_batch = Some(100);

        let bytes = postcard::to_allocvec(&mem).unwrap();
        let loaded: SentinelMemory = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(loaded.epoch_summaries.len(), 1);
        assert_eq!(loaded.last_action_batch, Some(42));
        assert_eq!(loaded.last_alert_batch, Some(100));
    }

    #[test]
    fn pattern_detector_validator_degradation() {
        let detector = PatternDetector::new();
        let v1 = test_vid(1);

        let mut summaries = VecDeque::new();
        summaries.push_back(make_summary(0, vec![make_profile(v1, 0.98, 100)]));
        summaries.push_back(make_summary(1, vec![make_profile(v1, 0.91, 100)]));
        summaries.push_back(make_summary(2, vec![make_profile(v1, 0.84, 100)]));

        let alerts = detector.detect(&summaries);
        assert!(
            alerts
                .iter()
                .any(|a| matches!(a, PatternAlert::ValidatorDegrading { .. }))
        );
    }

    #[test]
    fn pattern_detector_no_alert_stable() {
        let detector = PatternDetector::new();
        let v1 = test_vid(1);

        let mut summaries = VecDeque::new();
        summaries.push_back(make_summary(0, vec![make_profile(v1, 0.95, 100)]));
        summaries.push_back(make_summary(1, vec![make_profile(v1, 0.96, 100)]));
        summaries.push_back(make_summary(2, vec![make_profile(v1, 0.94, 100)]));

        let alerts = detector.detect(&summaries);
        assert!(
            !alerts
                .iter()
                .any(|a| matches!(a, PatternAlert::ValidatorDegrading { .. }))
        );
    }

    #[test]
    fn pattern_detector_latency_creep() {
        let detector = PatternDetector::new();

        let mut summaries = VecDeque::new();
        for (i, lat) in [100.0f32, 110.0, 120.0, 135.0].iter().enumerate() {
            let mut s = make_summary(i as u64, vec![]);
            s.avg_commit_latency_ms = *lat;
            summaries.push_back(s);
        }

        let alerts = detector.detect(&summaries);
        assert!(
            alerts
                .iter()
                .any(|a| matches!(a, PatternAlert::LatencyCreep { .. }))
        );
    }

    #[test]
    fn pattern_detector_stake_centralization() {
        let detector = PatternDetector::new();
        let v1 = test_vid(1);
        let v2 = test_vid(2);
        let v3 = test_vid(3);

        let mut summaries = VecDeque::new();
        let profiles = vec![
            make_profile(v1, 1.0, 7000),
            make_profile(v2, 1.0, 2000),
            make_profile(v3, 1.0, 1000),
        ];
        summaries.push_back(make_summary(0, profiles.clone()));
        summaries.push_back(make_summary(1, profiles.clone()));
        summaries.push_back(make_summary(2, profiles));

        let alerts = detector.detect(&summaries);
        assert!(
            alerts
                .iter()
                .any(|a| matches!(a, PatternAlert::StakeCentralization { .. }))
        );
    }

    #[test]
    fn pattern_detector_health_deterioration() {
        let detector = PatternDetector::new();

        let mut summaries = VecDeque::new();
        for (i, score) in [0.05f32, 0.08, 0.12, 0.15, 0.20].iter().enumerate() {
            let mut s = make_summary(i as u64, vec![]);
            s.avg_health_score = *score;
            summaries.push_back(s);
        }

        let alerts = detector.detect(&summaries);
        assert!(
            alerts
                .iter()
                .any(|a| matches!(a, PatternAlert::HealthDeteriorating { .. }))
        );
    }

    #[test]
    fn pattern_detector_too_few_epochs() {
        let detector = PatternDetector::new();
        let summaries = VecDeque::new();
        assert!(detector.detect(&summaries).is_empty());

        let mut one = VecDeque::new();
        one.push_back(make_summary(0, vec![]));
        assert!(detector.detect(&one).is_empty());
    }

    #[test]
    fn is_declining_works() {
        assert!(is_declining(&[0.98, 0.91, 0.84]));
        assert!(!is_declining(&[0.91, 0.95, 0.84]));
        assert!(!is_declining(&[0.90, 0.90, 0.90]));
    }

    #[test]
    fn is_increasing_by_pct_works() {
        assert!(is_increasing_by_pct(&[100.0, 110.0, 125.0], 0.05));
        assert!(!is_increasing_by_pct(&[100.0, 103.0, 106.0], 0.05));
    }
}
