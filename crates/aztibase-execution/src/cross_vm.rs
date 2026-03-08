use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::evm;
use crate::state::AccountState;
use crate::vm::{EngineConfig, ExecutionEngine};
use aztibase_core::{TxHash, hash};

type Address = [u8; 32];

const MAX_CROSS_VM_DEPTH: u32 = 4;

/// Describes a cross-VM call: the source VM calls into the target VM.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossVmCall {
    pub caller: Address,
    pub target: Address,
    pub calldata: Vec<u8>,
    pub gas_limit: u64,
    pub depth: u32,
}

/// Tracks active cross-VM call targets to prevent reentrancy within a
/// single execution context. A contract that is currently executing
/// cannot be called again until it returns.
#[derive(Clone, Debug, Default)]
pub struct ReentrancyGuard {
    active_targets: HashSet<Address>,
}

impl ReentrancyGuard {
    pub fn new() -> Self {
        Self {
            active_targets: HashSet::new(),
        }
    }

    pub fn enter(&mut self, target: &Address) -> bool {
        self.active_targets.insert(*target)
    }

    pub fn exit(&mut self, target: &Address) {
        self.active_targets.remove(target);
    }

    pub fn is_active(&self, target: &Address) -> bool {
        self.active_targets.contains(target)
    }
}

/// Result of a cross-VM call.
#[derive(Clone, Debug)]
pub struct CrossVmResult {
    pub success: bool,
    pub return_data: Vec<u8>,
    pub gas_used: u64,
    pub error: Option<String>,
}

/// WASM → EVM bridge: a WASM contract calls an EVM contract.
///
/// Executes `evm_call` on the shared `AccountState` as an internal call
/// (nonce is not checked for internal bridge calls).
pub fn wasm_to_evm(
    state: &mut AccountState,
    call: &CrossVmCall,
    guard: &mut ReentrancyGuard,
) -> CrossVmResult {
    if call.depth >= MAX_CROSS_VM_DEPTH {
        return CrossVmResult {
            success: false,
            return_data: Vec::new(),
            gas_used: 0,
            error: Some(format!(
                "cross-VM call depth exceeded: {} >= {MAX_CROSS_VM_DEPTH}",
                call.depth
            )),
        };
    }

    if !guard.enter(&call.target) {
        return CrossVmResult {
            success: false,
            return_data: Vec::new(),
            gas_used: 0,
            error: Some("reentrancy detected: target contract is already executing".into()),
        };
    }

    let tx_hash = compute_bridge_hash(&call.caller, &call.target, &call.calldata, call.depth);
    let nonce = state.nonce(&call.caller);

    let receipt = evm::evm_call(
        state,
        tx_hash,
        &call.caller,
        &call.target,
        &call.calldata,
        nonce,
        call.gas_limit,
        0,
    );

    guard.exit(&call.target);

    CrossVmResult {
        success: receipt.success,
        return_data: receipt
            .error
            .as_ref()
            .map(|e| e.as_bytes().to_vec())
            .unwrap_or_default(),
        gas_used: receipt.gas_used,
        error: receipt.error,
    }
}

/// EVM → WASM bridge: an EVM precompile calls a WASM contract.
///
/// Loads the WASM bytecode from the target account and executes the
/// specified function. Returns output as raw bytes.
pub fn evm_to_wasm(
    state: &mut AccountState,
    call: &CrossVmCall,
    guard: &mut ReentrancyGuard,
) -> CrossVmResult {
    if call.depth >= MAX_CROSS_VM_DEPTH {
        return CrossVmResult {
            success: false,
            return_data: Vec::new(),
            gas_used: 0,
            error: Some(format!(
                "cross-VM call depth exceeded: {} >= {MAX_CROSS_VM_DEPTH}",
                call.depth
            )),
        };
    }

    if !guard.enter(&call.target) {
        return CrossVmResult {
            success: false,
            return_data: Vec::new(),
            gas_used: 0,
            error: Some("reentrancy detected: target contract is already executing".into()),
        };
    }

    let code = match state.code(&call.target) {
        Some(c) => c.to_vec(),
        None => {
            guard.exit(&call.target);
            return CrossVmResult {
                success: false,
                return_data: Vec::new(),
                gas_used: 0,
                error: Some("target has no WASM code".into()),
            };
        }
    };

    let func_name = extract_func_name(&call.calldata);

    let engine = match ExecutionEngine::new(EngineConfig {
        fuel_limit: call.gas_limit,
        ..Default::default()
    }) {
        Ok(e) => e,
        Err(e) => {
            guard.exit(&call.target);
            return CrossVmResult {
                success: false,
                return_data: Vec::new(),
                gas_used: 0,
                error: Some(format!("wasm engine init failed: {e}")),
            };
        }
    };

    let existing_storage = state
        .get(&call.target)
        .map(|a| a.storage.clone())
        .unwrap_or_default();

    let result = match engine.execute_with_storage(&code, &func_name, &[], existing_storage) {
        Ok(result) => {
            for (k, v) in &result.storage {
                state.set_storage(&call.target, k.clone(), v.clone());
            }

            CrossVmResult {
                success: true,
                return_data: result
                    .values
                    .first()
                    .map(|v| match v {
                        wasmtime::Val::I32(n) => n.to_le_bytes().to_vec(),
                        wasmtime::Val::I64(n) => n.to_le_bytes().to_vec(),
                        _ => Vec::new(),
                    })
                    .unwrap_or_default(),
                gas_used: result.fuel_consumed,
                error: None,
            }
        }
        Err(e) => CrossVmResult {
            success: false,
            return_data: Vec::new(),
            gas_used: call.gas_limit,
            error: Some(format!("wasm execution failed: {e}")),
        },
    };

    guard.exit(&call.target);
    result
}

fn compute_bridge_hash(caller: &Address, target: &Address, data: &[u8], depth: u32) -> TxHash {
    let mut buf = Vec::new();
    buf.extend_from_slice(caller);
    buf.extend_from_slice(target);
    buf.extend_from_slice(data);
    buf.extend_from_slice(&depth.to_le_bytes());
    hash(&buf)
}

fn extract_func_name(calldata: &[u8]) -> String {
    if calldata.is_empty() {
        return "main".into();
    }
    match std::str::from_utf8(calldata) {
        Ok(s) if !s.is_empty() && s.len() <= 128 => s.to_string(),
        _ => "main".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_limit_rejects() {
        let mut state = AccountState::new();
        let mut guard = ReentrancyGuard::new();
        let call = CrossVmCall {
            caller: [1u8; 32],
            target: [2u8; 32],
            calldata: vec![],
            gas_limit: 100_000,
            depth: MAX_CROSS_VM_DEPTH,
        };
        let r1 = wasm_to_evm(&mut state, &call, &mut guard);
        assert!(!r1.success);
        assert!(r1.error.unwrap().contains("depth exceeded"));

        let r2 = evm_to_wasm(&mut state, &call, &mut guard);
        assert!(!r2.success);
        assert!(r2.error.unwrap().contains("depth exceeded"));
    }

    #[test]
    fn evm_to_wasm_no_code() {
        let mut state = AccountState::new();
        let mut guard = ReentrancyGuard::new();
        let call = CrossVmCall {
            caller: [1u8; 32],
            target: [2u8; 32],
            calldata: b"main".to_vec(),
            gas_limit: 100_000,
            depth: 0,
        };
        let result = evm_to_wasm(&mut state, &call, &mut guard);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("no WASM code"));
    }

    #[test]
    fn evm_to_wasm_executes() {
        let mut state = AccountState::new();
        let mut guard = ReentrancyGuard::new();
        let target = [3u8; 32];

        let wasm = wat::parse_str(
            r#"
            (module
                (func (export "main") (result i32)
                    i32.const 42
                )
            )
            "#,
        )
        .unwrap();
        state.set_code(&target, wasm);

        let call = CrossVmCall {
            caller: [1u8; 32],
            target,
            calldata: b"main".to_vec(),
            gas_limit: 1_000_000,
            depth: 0,
        };
        let result = evm_to_wasm(&mut state, &call, &mut guard);
        assert!(result.success, "error: {:?}", result.error);
        assert!(result.gas_used > 0);
        let val = i32::from_le_bytes(result.return_data[..4].try_into().unwrap());
        assert_eq!(val, 42);
    }

    #[test]
    fn wasm_to_evm_calls() {
        let mut state = AccountState::new();
        let mut guard = ReentrancyGuard::new();
        let caller = [1u8; 32];
        let target = [2u8; 32];
        state.set_balance(&caller, 1_000_000);

        let call = CrossVmCall {
            caller,
            target,
            calldata: vec![],
            gas_limit: 100_000,
            depth: 0,
        };
        let result = wasm_to_evm(&mut state, &call, &mut guard);
        assert!(result.success, "error: {:?}", result.error);
    }

    #[test]
    fn reentrancy_guard_rejects_active_target() {
        let mut state = AccountState::new();
        let mut guard = ReentrancyGuard::new();
        let target = [5u8; 32];

        // Simulate target already executing
        assert!(guard.enter(&target));

        let call = CrossVmCall {
            caller: [1u8; 32],
            target,
            calldata: vec![],
            gas_limit: 100_000,
            depth: 0,
        };

        let r1 = wasm_to_evm(&mut state, &call, &mut guard);
        assert!(!r1.success);
        assert!(r1.error.unwrap().contains("reentrancy detected"));

        let r2 = evm_to_wasm(&mut state, &call, &mut guard);
        assert!(!r2.success);
        assert!(r2.error.unwrap().contains("reentrancy detected"));

        guard.exit(&target);

        // After exit, the target should be callable again
        let r3 = wasm_to_evm(&mut state, &call, &mut guard);
        assert!(r3.success);
    }

    #[test]
    fn cross_vm_call_serialization() {
        let call = CrossVmCall {
            caller: [1u8; 32],
            target: [2u8; 32],
            calldata: vec![0xAA, 0xBB],
            gas_limit: 500_000,
            depth: 2,
        };
        let encoded = postcard::to_allocvec(&call).unwrap();
        let decoded: CrossVmCall = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(decoded.caller, call.caller);
        assert_eq!(decoded.target, call.target);
        assert_eq!(decoded.calldata, call.calldata);
        assert_eq!(decoded.gas_limit, call.gas_limit);
        assert_eq!(decoded.depth, call.depth);
    }

    #[test]
    fn extract_func_name_utf8() {
        assert_eq!(extract_func_name(b"transfer"), "transfer");
        assert_eq!(extract_func_name(b""), "main");
        assert_eq!(extract_func_name(&[0xFF, 0xFE]), "main");
    }
}
