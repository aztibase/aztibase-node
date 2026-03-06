use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AIRuntimeMode {
    Passthrough,
    LocalInference,
    NetworkInference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub model_id: String,
    pub input: Vec<u8>,
    pub max_compute_units: u64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub output: Vec<u8>,
    pub compute_units_used: u64,
    pub model_id: String,
    pub deterministic_hash: [u8; 32],
}

/// Abstraction over AI inference execution.
///
/// In PASSTHROUGH mode, inference requests are acknowledged but not executed —
/// the node operates without AI capabilities. This allows nodes to participate
/// in consensus and block production without requiring AI hardware.
pub trait AIRuntime: Send + Sync {
    fn mode(&self) -> AIRuntimeMode;
    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResult>;
    fn supports_model(&self, model_id: &str) -> bool;
}

/// No-op runtime that acknowledges requests without executing inference.
pub struct PassthroughRuntime;

impl AIRuntime for PassthroughRuntime {
    fn mode(&self) -> AIRuntimeMode {
        AIRuntimeMode::Passthrough
    }

    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResult> {
        Ok(InferenceResult {
            output: Vec::new(),
            compute_units_used: 0,
            model_id: request.model_id.clone(),
            deterministic_hash: [0u8; 32],
        })
    }

    fn supports_model(&self, _model_id: &str) -> bool {
        false
    }
}
