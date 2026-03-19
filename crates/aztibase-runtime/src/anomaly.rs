use std::path::Path;

use aztibase_core::hash;
use tract_onnx::prelude::*;

const NUM_TX_FEATURES: usize = 6;

/// Feature vector extracted from a transaction for anomaly scoring.
/// Uses a fixed-width representation so the scorer can work with
/// either a real ML model or the built-in heuristic.
pub struct TxFeatures {
    pub value: u128,
    pub gas_price: u64,
    pub gas_limit: u64,
    pub payload_size: u32,
    pub is_contract_deploy: bool,
    pub is_ai_infer: bool,
}

impl TxFeatures {
    fn to_floats(&self) -> [f32; NUM_TX_FEATURES] {
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

/// Normalization parameters for the ONNX tx anomaly model.
#[derive(serde::Deserialize)]
struct TxNormParams {
    mins: Vec<f32>,
    ranges: Vec<f32>,
    threshold: f32,
}

/// ONNX autoencoder for tx anomaly detection.
/// Same pattern as sentinel Tier 2: normalize → inference → reconstruction error.
struct OnnxTxScorer {
    model: TypedModel,
    params: TxNormParams,
}

impl OnnxTxScorer {
    fn load(model_dir: &Path) -> Option<Self> {
        let onnx_path = model_dir.join("tx_anomaly_v1.onnx");
        let params_path = model_dir.join("tx_anomaly_v1_params.json");

        if !onnx_path.exists() || !params_path.exists() {
            return None;
        }

        let params_json = match std::fs::read_to_string(&params_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to read tx anomaly params");
                return None;
            }
        };
        let params: TxNormParams = match serde_json::from_str(&params_json) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse tx anomaly params");
                return None;
            }
        };

        if params.mins.len() != NUM_TX_FEATURES || params.ranges.len() != NUM_TX_FEATURES {
            tracing::warn!("Tx anomaly ONNX params dimension mismatch");
            return None;
        }

        let inference_model = match tract_onnx::onnx().model_for_path(&onnx_path) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to load tx anomaly ONNX model");
                return None;
            }
        };

        let typed = match inference_model
            .with_input_fact(
                0,
                InferenceFact::dt_shape(f32::datum_type(), [1, NUM_TX_FEATURES as i64]),
            )
            .and_then(|m| m.into_optimized())
        {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to optimize tx anomaly ONNX model");
                return None;
            }
        };

        tracing::info!(threshold = params.threshold, "Tx anomaly ONNX model loaded");
        Some(Self {
            model: typed,
            params,
        })
    }

    fn score(&self, features: &[f32; NUM_TX_FEATURES]) -> f32 {
        let normalized: Vec<f32> = features
            .iter()
            .zip(self.params.mins.iter().zip(self.params.ranges.iter()))
            .map(|(f, (min, range))| {
                if *range == 0.0 {
                    0.0
                } else {
                    (f - min) / range
                }
            })
            .collect();

        let input =
            match tract_ndarray::Array2::from_shape_vec((1, NUM_TX_FEATURES), normalized.clone()) {
                Ok(a) => a,
                Err(_) => return 0.0,
            };

        let plan = match self.model.clone().into_runnable() {
            Ok(p) => p,
            Err(_) => return 0.0,
        };

        let output = match plan.run(tvec!(TValue::from_const(input.into_arc_tensor()))) {
            Ok(r) => r,
            Err(_) => return 0.0,
        };

        let view = match output[0].to_array_view::<f32>() {
            Ok(v) => v,
            Err(_) => return 0.0,
        };
        let reconstructed: Vec<f32> = view.iter().copied().collect();
        if reconstructed.len() < NUM_TX_FEATURES {
            return 0.0;
        }

        let mse: f32 = normalized
            .iter()
            .zip(reconstructed.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            / NUM_TX_FEATURES as f32;

        let ratio = mse / self.params.threshold;
        (ratio * 0.3).clamp(0.0, 1.0)
    }
}

/// Scores transactions for anomalous behavior.
///
/// Uses an ONNX autoencoder when a model is available at
/// `{model_dir}/tx_anomaly_v1.onnx`, otherwise falls back to
/// a deterministic heuristic. This is advisory only — never blocks execution.
pub struct AnomalyScorer {
    high_value_threshold: u128,
    high_gas_threshold: u64,
    large_payload_threshold: u32,
    onnx: Option<OnnxTxScorer>,
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
            onnx: None,
        }
    }

    pub fn with_model_dir(mut self, model_dir: &Path) -> Self {
        self.onnx = OnnxTxScorer::load(model_dir);
        self
    }

    pub fn is_onnx(&self) -> bool {
        self.onnx.is_some()
    }

    /// Score a transaction's features. Returns a value in `[0.0, 1.0]`.
    /// Higher scores indicate more anomalous transactions.
    pub fn score(&self, features: &TxFeatures) -> f32 {
        let floats = features.to_floats();

        if let Some(ref onnx) = self.onnx {
            return onnx.score(&floats);
        }

        self.heuristic_score(features, &floats)
    }

    fn heuristic_score(&self, features: &TxFeatures, floats: &[f32; NUM_TX_FEATURES]) -> f32 {
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
            value: u128::MAX,
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

    #[test]
    fn without_model_uses_heuristic() {
        let scorer = AnomalyScorer::new();
        assert!(!scorer.is_onnx());
    }

    #[test]
    fn with_missing_model_dir_falls_back() {
        let scorer = AnomalyScorer::new().with_model_dir(Path::new("/nonexistent/path"));
        assert!(!scorer.is_onnx());
        // Still scores using heuristic
        let features = TxFeatures {
            value: 1000,
            gas_price: 1,
            gas_limit: 21_000,
            payload_size: 100,
            is_contract_deploy: false,
            is_ai_infer: false,
        };
        let score = scorer.score(&features);
        assert!(score >= 0.0 && score <= 1.0);
    }
}
