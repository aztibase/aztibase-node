use std::collections::HashMap;
use std::io::Cursor;
use std::sync::RwLock;

use anyhow::{Result, bail};
use tract_onnx::prelude::*;

use crate::ai_oracle::{AIRuntime, AIRuntimeMode, InferenceRequest, InferenceResult};

type RunModel = RunnableModel<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

struct RegisteredModel {
    plan: RunModel,
    input_shape: Vec<usize>,
    output_shape: Vec<usize>,
}

/// AI runtime backed by tract for local ONNX inference.
pub struct TractRuntime {
    models: RwLock<HashMap<String, RegisteredModel>>,
}

impl Default for TractRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl TractRuntime {
    pub fn new() -> Self {
        Self {
            models: RwLock::new(HashMap::new()),
        }
    }

    /// Register an ONNX model from raw bytes.
    pub fn register_model(&self, model_id: &str, onnx_bytes: &[u8]) -> Result<()> {
        let mut cursor = Cursor::new(onnx_bytes);
        let model = tract_onnx::onnx()
            .model_for_read(&mut cursor)?
            .into_optimized()?;

        let input_shape: Vec<usize> = model
            .input_fact(0)?
            .shape
            .as_concrete()
            .map(|s| s.to_vec())
            .unwrap_or_default();

        let output_shape: Vec<usize> = model
            .output_fact(0)?
            .shape
            .as_concrete()
            .map(|s| s.to_vec())
            .unwrap_or_default();

        let plan = model.into_runnable()?;

        let registered = RegisteredModel {
            plan,
            input_shape,
            output_shape,
        };

        self.models
            .write()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?
            .insert(model_id.to_string(), registered);

        Ok(())
    }

    /// Remove a previously registered model.
    pub fn unregister_model(&self, model_id: &str) -> bool {
        self.models
            .write()
            .map(|mut m| m.remove(model_id).is_some())
            .unwrap_or(false)
    }
}

impl AIRuntime for TractRuntime {
    fn mode(&self) -> AIRuntimeMode {
        AIRuntimeMode::LocalInference
    }

    fn infer(&self, request: &InferenceRequest) -> Result<InferenceResult> {
        let models = self
            .models
            .read()
            .map_err(|e| anyhow::anyhow!("lock poisoned: {e}"))?;

        let registered = models
            .get(&request.model_id)
            .ok_or_else(|| anyhow::anyhow!("model not found: {}", request.model_id))?;

        let input_len: usize = registered.input_shape.iter().product();
        if input_len == 0 {
            bail!("model has no concrete input shape");
        }

        let element_size = std::mem::size_of::<f32>();
        if request.input.len() != input_len * element_size {
            bail!(
                "input size mismatch: expected {} bytes ({} f32s), got {}",
                input_len * element_size,
                input_len,
                request.input.len()
            );
        }

        let floats: Vec<f32> = request
            .input
            .chunks_exact(element_size)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        let input_tensor =
            tract_ndarray::Array::from_shape_vec(registered.input_shape.as_slice(), floats)?
                .into_tensor();

        let outputs = registered.plan.run(tvec![input_tensor.into()])?;

        let output_tensor = &outputs[0];
        let output_floats = output_tensor
            .as_slice::<f32>()
            .map_err(|e| anyhow::anyhow!("output tensor not f32: {e}"))?;

        let output_bytes: Vec<u8> = output_floats.iter().flat_map(|f| f.to_le_bytes()).collect();

        let compute_units = registered.input_shape.iter().product::<usize>() as u64
            * registered.output_shape.iter().product::<usize>() as u64;

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(request.model_id.as_bytes());
        hash_input.extend_from_slice(&request.input);
        hash_input.extend_from_slice(&output_bytes);
        let deterministic_hash = dendrite_core::hash(&hash_input);

        Ok(InferenceResult {
            output: output_bytes,
            compute_units_used: compute_units,
            model_id: request.model_id.clone(),
            deterministic_hash,
        })
    }

    fn supports_model(&self, model_id: &str) -> bool {
        self.models
            .read()
            .map(|m| m.contains_key(model_id))
            .unwrap_or(false)
    }
}

/// Receipt for a completed inference operation.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct InferenceReceipt {
    pub request_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub model_id: String,
    pub compute_units: u64,
    pub deterministic_hash: [u8; 32],
}

impl InferenceReceipt {
    pub fn from_request_and_result(request: &InferenceRequest, result: &InferenceResult) -> Self {
        let mut req_buf = Vec::new();
        req_buf.extend_from_slice(request.model_id.as_bytes());
        req_buf.extend_from_slice(&request.input);
        let request_hash = dendrite_core::hash(&req_buf);

        let mut res_buf = Vec::new();
        res_buf.extend_from_slice(result.model_id.as_bytes());
        res_buf.extend_from_slice(&result.output);
        let result_hash = dendrite_core::hash(&res_buf);

        Self {
            request_hash,
            result_hash,
            model_id: result.model_id.clone(),
            compute_units: result.compute_units_used,
            deterministic_hash: result.deterministic_hash,
        }
    }
}

/// Verify an inference result by re-running the model and comparing hashes.
pub fn verify_inference(
    runtime: &TractRuntime,
    request: &InferenceRequest,
    expected_hash: &[u8; 32],
) -> Result<bool> {
    let result = runtime.infer(request)?;
    Ok(&result.deterministic_hash == expected_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use tract_onnx::pb;

    /// Build a minimal ONNX model: y = x + b, where b = [1.0, 1.0, 1.0].
    /// Input shape: [1, 3], output shape: [1, 3].
    fn build_add_model_bytes() -> Vec<u8> {
        let model = pb::ModelProto {
            ir_version: 7,
            opset_import: vec![pb::OperatorSetIdProto {
                domain: String::new(),
                version: 13,
            }],
            graph: Some(pb::GraphProto {
                name: "add_graph".into(),
                input: vec![pb::ValueInfoProto {
                    name: "x".into(),
                    r#type: Some(pb::TypeProto {
                        denotation: String::new(),
                        value: Some(pb::type_proto::Value::TensorType(pb::type_proto::Tensor {
                            elem_type: 1, // FLOAT
                            shape: Some(pb::TensorShapeProto {
                                dim: vec![
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(1),
                                        ),
                                    },
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(3),
                                        ),
                                    },
                                ],
                            }),
                        })),
                    }),
                    doc_string: String::new(),
                }],
                output: vec![pb::ValueInfoProto {
                    name: "y".into(),
                    r#type: Some(pb::TypeProto {
                        denotation: String::new(),
                        value: Some(pb::type_proto::Value::TensorType(pb::type_proto::Tensor {
                            elem_type: 1,
                            shape: Some(pb::TensorShapeProto {
                                dim: vec![
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(1),
                                        ),
                                    },
                                    pb::tensor_shape_proto::Dimension {
                                        denotation: String::new(),
                                        value: Some(
                                            pb::tensor_shape_proto::dimension::Value::DimValue(3),
                                        ),
                                    },
                                ],
                            }),
                        })),
                    }),
                    doc_string: String::new(),
                }],
                node: vec![pb::NodeProto {
                    input: vec!["x".into(), "b".into()],
                    output: vec!["y".into()],
                    name: "add_node".into(),
                    op_type: "Add".into(),
                    domain: String::new(),
                    attribute: vec![],
                    doc_string: String::new(),
                }],
                initializer: vec![pb::TensorProto {
                    name: "b".into(),
                    dims: vec![1, 3],
                    data_type: 1, // FLOAT
                    float_data: vec![1.0, 1.0, 1.0],
                    ..Default::default()
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        model.encode_to_vec()
    }

    fn make_f32_input(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn parse_f32_output(bytes: &[u8]) -> Vec<f32> {
        bytes
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }

    #[test]
    fn tract_runtime_mode() {
        let rt = TractRuntime::new();
        assert_eq!(rt.mode(), AIRuntimeMode::LocalInference);
    }

    #[test]
    fn register_and_supports_model() {
        let rt = TractRuntime::new();
        assert!(!rt.supports_model("add"));

        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();
        assert!(rt.supports_model("add"));
    }

    #[test]
    fn register_invalid_bytes_fails() {
        let rt = TractRuntime::new();
        assert!(rt.register_model("bad", &[0xFF, 0x00]).is_err());
    }

    #[test]
    fn unregister_model() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();
        assert!(rt.unregister_model("add"));
        assert!(!rt.supports_model("add"));
        assert!(!rt.unregister_model("add"));
    }

    #[test]
    fn infer_add_model() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let input = make_f32_input(&[2.0, 3.0, 4.0]);
        let req = InferenceRequest {
            model_id: "add".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let result = rt.infer(&req).unwrap();
        let output = parse_f32_output(&result.output);

        assert_eq!(output, vec![3.0, 4.0, 5.0]);
        assert!(result.compute_units_used > 0);
        assert_eq!(result.model_id, "add");
        assert_ne!(result.deterministic_hash, [0u8; 32]);
    }

    #[test]
    fn infer_deterministic_hash() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let input = make_f32_input(&[1.0, 2.0, 3.0]);
        let req = InferenceRequest {
            model_id: "add".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let r1 = rt.infer(&req).unwrap();
        let r2 = rt.infer(&req).unwrap();
        assert_eq!(r1.deterministic_hash, r2.deterministic_hash);
        assert_eq!(r1.output, r2.output);
    }

    #[test]
    fn infer_unknown_model_fails() {
        let rt = TractRuntime::new();
        let req = InferenceRequest {
            model_id: "nonexistent".into(),
            input: vec![],
            max_compute_units: 1000,
            metadata: Default::default(),
        };
        assert!(rt.infer(&req).is_err());
    }

    #[test]
    fn infer_wrong_input_size_fails() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let req = InferenceRequest {
            model_id: "add".into(),
            input: vec![0u8; 8], // 2 floats instead of 3
            max_compute_units: 1000,
            metadata: Default::default(),
        };
        assert!(rt.infer(&req).is_err());
    }

    #[test]
    fn inference_receipt_creation() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let input = make_f32_input(&[5.0, 6.0, 7.0]);
        let req = InferenceRequest {
            model_id: "add".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let result = rt.infer(&req).unwrap();
        let receipt = InferenceReceipt::from_request_and_result(&req, &result);

        assert_eq!(receipt.model_id, "add");
        assert_eq!(receipt.deterministic_hash, result.deterministic_hash);
        assert!(receipt.compute_units > 0);
        assert_ne!(receipt.request_hash, [0u8; 32]);
        assert_ne!(receipt.result_hash, [0u8; 32]);
    }

    #[test]
    fn verify_inference_succeeds() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let input = make_f32_input(&[1.0, 1.0, 1.0]);
        let req = InferenceRequest {
            model_id: "add".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let result = rt.infer(&req).unwrap();
        assert!(verify_inference(&rt, &req, &result.deterministic_hash).unwrap());
    }

    #[test]
    fn verify_inference_rejects_tampered() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();
        rt.register_model("add", &model_bytes).unwrap();

        let input = make_f32_input(&[1.0, 1.0, 1.0]);
        let req = InferenceRequest {
            model_id: "add".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let fake_hash = [0xFFu8; 32];
        assert!(!verify_inference(&rt, &req, &fake_hash).unwrap());
    }

    #[test]
    fn two_models_coexist() {
        let rt = TractRuntime::new();
        let model_bytes = build_add_model_bytes();

        rt.register_model("model_a", &model_bytes).unwrap();
        rt.register_model("model_b", &model_bytes).unwrap();

        let input = make_f32_input(&[10.0, 20.0, 30.0]);
        let req_a = InferenceRequest {
            model_id: "model_a".into(),
            input: input.clone(),
            max_compute_units: 1000,
            metadata: Default::default(),
        };
        let req_b = InferenceRequest {
            model_id: "model_b".into(),
            input,
            max_compute_units: 1000,
            metadata: Default::default(),
        };

        let r_a = rt.infer(&req_a).unwrap();
        let r_b = rt.infer(&req_b).unwrap();

        assert_eq!(parse_f32_output(&r_a.output), vec![11.0, 21.0, 31.0]);
        assert_eq!(parse_f32_output(&r_b.output), vec![11.0, 21.0, 31.0]);
        // Different model_id means different deterministic hash
        assert_ne!(r_a.deterministic_hash, r_b.deterministic_hash);
    }
}
