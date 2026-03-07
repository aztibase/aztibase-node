use aztibase_core::hash;

/// Feature vector extracted from a transaction for anomaly scoring.
/// Uses a fixed-width representation so the scorer can work with
/// either a real ML model or the built-in heuristic.
pub struct TxFeatures {
    pub value: u64,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub payload_size: u32,
    pub is_contract_deploy: bool,
    pub is_ai_infer: bool,
}

impl TxFeatures {
    fn to_floats(&self) -> [f32; 6] {
        [
            self.value as f32,
            self.gas_price as f32,
            self.gas_limit as f32,
            self.payload_size as f32,
            if self.is_contract_deploy { 1.0 } else { 0.0 },
            if self.is_ai_infer { 1.0 } else { 0.0 },
        ]
    }
}

/// Scores transactions for anomalous behavior.
///
/// Uses a deterministic heuristic that maps transaction features to a
/// score in `[0.0, 1.0]`. When a real ONNX model is loaded via
/// `TractRuntime`, the scorer can delegate to it instead (future work).
///
/// The heuristic produces nonzero but low scores for ordinary transactions,
/// higher scores for unusual patterns (extreme gas, very large payloads,
/// zero-value deploys). This is advisory only — never blocks execution.
pub struct AnomalyScorer {
    high_value_threshold: u64,
    high_gas_threshold: u64,
    large_payload_threshold: u32,
}

impl Default for AnomalyScorer {
    fn default() -> Self {
        Self::new()
    }
}

impl AnomalyScorer {
    pub fn new() -> Self {
        Self {
            high_value_threshold: 1_000_000_000,
            high_gas_threshold: 10_000_000,
            large_payload_threshold: 65_536,
        }
    }

    /// Score a transaction's features. Returns a value in `[0.0, 1.0]`.
    /// Higher scores indicate more anomalous transactions.
    pub fn score(&self, features: &TxFeatures) -> f32 {
        let floats = features.to_floats();

        // Deterministic hash-based base noise (prevents all-zero scores for normal txs).
        let seed = hash(
            &floats
                .iter()
                .flat_map(|f| f.to_le_bytes())
                .collect::<Vec<_>>(),
        );
        let base = (seed[0] as f32) / 255.0 * 0.05;

        let mut score = base;

        if features.value > self.high_value_threshold {
            score += 0.2;
        }
        if features.gas_price > self.high_gas_threshold {
            score += 0.15;
        }
        if features.payload_size > self.large_payload_threshold {
            score += 0.15;
        }
        if features.is_contract_deploy && features.value == 0 {
            score += 0.1;
        }

        score.clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_transfer_low_score() {
        let scorer = AnomalyScorer::new();
        let features = TxFeatures {
            value: 1000,
            gas_price: 1,
            gas_limit: 21_000,
            payload_size: 100,
            is_contract_deploy: false,
            is_ai_infer: false,
        };
        let score = scorer.score(&features);
        assert!(
            score < 0.1,
            "normal transfer should have low score: {score}"
        );
        assert!(score >= 0.0);
    }

    #[test]
    fn high_value_transfer_elevated() {
        let scorer = AnomalyScorer::new();
        let features = TxFeatures {
            value: 10_000_000_000,
            gas_price: 1,
            gas_limit: 21_000,
            payload_size: 100,
            is_contract_deploy: false,
            is_ai_infer: false,
        };
        let score = scorer.score(&features);
        assert!(score >= 0.2, "high value should be elevated: {score}");
    }

    #[test]
    fn zero_value_deploy_elevated() {
        let scorer = AnomalyScorer::new();
        let features = TxFeatures {
            value: 0,
            gas_price: 1,
            gas_limit: 1_000_000,
            payload_size: 5000,
            is_contract_deploy: true,
            is_ai_infer: false,
        };
        let score = scorer.score(&features);
        assert!(
            score >= 0.1,
            "zero-value deploy should be elevated: {score}"
        );
    }

    #[test]
    fn score_deterministic() {
        let scorer = AnomalyScorer::new();
        let features = TxFeatures {
            value: 500,
            gas_price: 10,
            gas_limit: 50_000,
            payload_size: 200,
            is_contract_deploy: false,
            is_ai_infer: true,
        };
        let s1 = scorer.score(&features);
        let s2 = scorer.score(&features);
        assert_eq!(s1, s2);
    }

    #[test]
    fn score_clamped_to_unit() {
        let scorer = AnomalyScorer::new();
        let features = TxFeatures {
            value: u64::MAX,
            gas_price: u64::MAX,
            gas_limit: u64::MAX,
            payload_size: u32::MAX,
            is_contract_deploy: true,
            is_ai_infer: true,
        };
        let score = scorer.score(&features);
        assert!(score <= 1.0, "score must be <= 1.0: {score}");
        assert!(score >= 0.0);
    }
}
