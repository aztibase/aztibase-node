pub mod agent;
pub mod ai_oracle;
pub mod contracts;
pub mod tract_runtime;

pub use ai_oracle::{
    AIRuntime, AIRuntimeMode, InferenceRequest, InferenceResult, PassthroughRuntime,
};
pub use contracts::ContractRuntime;
pub use tract_runtime::{InferenceReceipt, TractRuntime, verify_inference};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_mode() {
        let rt = PassthroughRuntime;
        assert_eq!(rt.mode(), AIRuntimeMode::Passthrough);
    }

    #[test]
    fn passthrough_returns_empty_result() {
        let rt = PassthroughRuntime;
        let req = InferenceRequest {
            model_id: "test-model".into(),
            input: vec![1, 2, 3],
            max_compute_units: 1000,
            metadata: Default::default(),
        };
        let result = rt.infer(&req).unwrap();
        assert!(result.output.is_empty());
        assert_eq!(result.compute_units_used, 0);
        assert_eq!(result.model_id, "test-model");
    }

    #[test]
    fn passthrough_supports_no_models() {
        let rt = PassthroughRuntime;
        assert!(!rt.supports_model("anything"));
        assert!(!rt.supports_model("gpt-4"));
    }
}
