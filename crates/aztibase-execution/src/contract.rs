use aztibase_core::{TxHash, hash};

use crate::state::AccountState;
use crate::vm::{ContractEvent, EngineConfig, ExecutionEngine};

type Address = [u8; 32];

#[derive(Clone, Debug)]
pub enum ContractTx {
    Deploy {
        hash: TxHash,
        deployer: Address,
        code: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
    },
    Call {
        hash: TxHash,
        caller: Address,
        contract: Address,
        func_name: String,
        args_data: Vec<u8>,
        nonce: u64,
        gas_limit: u64,
    },
}

#[derive(Clone, Debug)]
pub struct ContractReceipt {
    pub tx_hash: TxHash,
    pub success: bool,
    pub gas_used: u64,
    pub contract_address: Option<Address>,
    pub events: Vec<ContractEvent>,
    pub error: Option<String>,
}

/// Computes a deterministic contract address from deployer + nonce.
pub fn compute_contract_address(deployer: &Address, nonce: u64) -> Address {
    let mut preimage = Vec::with_capacity(40);
    preimage.extend_from_slice(deployer);
    preimage.extend_from_slice(&nonce.to_le_bytes());
    hash(&preimage)
}

/// Execute contract transactions against account state.
pub fn execute_contract_txs(state: &mut AccountState, txs: &[ContractTx]) -> Vec<ContractReceipt> {
    let engine = match ExecutionEngine::new(EngineConfig::default()) {
        Ok(e) => e,
        Err(e) => {
            return txs
                .iter()
                .map(|tx| {
                    let tx_hash = match tx {
                        ContractTx::Deploy { hash, .. } => *hash,
                        ContractTx::Call { hash, .. } => *hash,
                    };
                    ContractReceipt {
                        tx_hash,
                        success: false,
                        gas_used: 0,
                        contract_address: None,
                        events: vec![],
                        error: Some(format!("Engine init failed: {e}")),
                    }
                })
                .collect();
        }
    };

    txs.iter()
        .map(|tx| execute_one(&engine, state, tx))
        .collect()
}

fn execute_one(
    engine: &ExecutionEngine,
    state: &mut AccountState,
    tx: &ContractTx,
) -> ContractReceipt {
    match tx {
        ContractTx::Deploy {
            hash: tx_hash,
            deployer,
            code,
            nonce,
            gas_limit,
        } => execute_deploy(engine, state, *tx_hash, deployer, code, *nonce, *gas_limit),
        ContractTx::Call {
            hash: tx_hash,
            caller,
            contract,
            func_name,
            args_data: _,
            nonce,
            gas_limit,
        } => execute_call(
            engine, state, *tx_hash, caller, contract, func_name, *nonce, *gas_limit,
        ),
    }
}

fn execute_deploy(
    _engine: &ExecutionEngine,
    state: &mut AccountState,
    tx_hash: TxHash,
    deployer: &Address,
    code: &[u8],
    nonce: u64,
    _gas_limit: u64,
) -> ContractReceipt {
    let sender_nonce = state.nonce(deployer);
    if nonce != sender_nonce {
        state.increment_nonce(deployer);
        return ContractReceipt {
            tx_hash,
            success: false,
            gas_used: 21_000,
            contract_address: None,
            events: vec![],
            error: Some(format!(
                "nonce mismatch: expected {sender_nonce}, got {nonce}"
            )),
        };
    }

    let contract_addr = compute_contract_address(deployer, nonce);
    state.set_code(&contract_addr, code.to_vec());
    state.increment_nonce(deployer);

    ContractReceipt {
        tx_hash,
        success: true,
        gas_used: 32_000 + code.len() as u64 * 200,
        contract_address: Some(contract_addr),
        events: vec![],
        error: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_call(
    engine: &ExecutionEngine,
    state: &mut AccountState,
    tx_hash: TxHash,
    caller: &Address,
    contract: &Address,
    func_name: &str,
    nonce: u64,
    _gas_limit: u64,
) -> ContractReceipt {
    let sender_nonce = state.nonce(caller);
    if nonce != sender_nonce {
        state.increment_nonce(caller);
        return ContractReceipt {
            tx_hash,
            success: false,
            gas_used: 21_000,
            contract_address: None,
            events: vec![],
            error: Some(format!(
                "nonce mismatch: expected {sender_nonce}, got {nonce}"
            )),
        };
    }

    let code = match state.code(contract) {
        Some(c) => c.to_vec(),
        None => {
            state.increment_nonce(caller);
            return ContractReceipt {
                tx_hash,
                success: false,
                gas_used: 21_000,
                contract_address: None,
                events: vec![],
                error: Some("contract not found".to_string()),
            };
        }
    };

    // Load existing contract storage into host state
    let existing_storage = state
        .get(contract)
        .map(|a| a.storage.clone())
        .unwrap_or_default();

    let result = engine.execute_with_storage(&code, func_name, &[], existing_storage);

    state.increment_nonce(caller);

    match result {
        Ok(exec_result) => {
            for (k, v) in &exec_result.storage {
                state.set_storage(contract, k.clone(), v.clone());
            }
            ContractReceipt {
                tx_hash,
                success: true,
                gas_used: exec_result.fuel_consumed,
                contract_address: Some(*contract),
                events: exec_result.events,
                error: None,
            }
        }
        Err(e) => ContractReceipt {
            tx_hash,
            success: false,
            gas_used: 21_000,
            contract_address: None,
            events: vec![],
            error: Some(e.to_string()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::hash;

    fn deploy_tx(deployer: Address, code: Vec<u8>, nonce: u64) -> ContractTx {
        ContractTx::Deploy {
            hash: hash(&code),
            deployer,
            code,
            nonce,
            gas_limit: 1_000_000,
        }
    }

    fn call_tx(caller: Address, contract: Address, func: &str, nonce: u64) -> ContractTx {
        ContractTx::Call {
            hash: hash(func.as_bytes()),
            caller,
            contract,
            func_name: func.to_string(),
            args_data: vec![],
            nonce,
            gas_limit: 1_000_000,
        }
    }

    #[test]
    fn deploy_stores_code() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        let wasm = vec![0x00, 0x61, 0x73, 0x6d];

        let receipts = execute_contract_txs(&mut state, &[deploy_tx(deployer, wasm.clone(), 0)]);
        assert_eq!(receipts.len(), 1);
        assert!(receipts[0].success);

        let addr = receipts[0].contract_address.unwrap();
        assert_eq!(state.code(&addr).unwrap(), &wasm);
        assert_eq!(state.nonce(&deployer), 1);
    }

    #[test]
    fn deploy_nonce_mismatch() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        let wasm = vec![0x00];

        let receipts = execute_contract_txs(&mut state, &[deploy_tx(deployer, wasm, 5)]);
        assert!(!receipts[0].success);
        assert!(receipts[0].error.as_ref().unwrap().contains("nonce"));
    }

    #[test]
    fn call_nonexistent_contract() {
        let mut state = AccountState::new();
        let caller = [1u8; 32];
        let contract = [99u8; 32];

        let receipts = execute_contract_txs(&mut state, &[call_tx(caller, contract, "run", 0)]);
        assert!(!receipts[0].success);
        assert!(receipts[0].error.as_ref().unwrap().contains("not found"));
    }

    #[test]
    fn deploy_and_call_wasm_contract() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];

        let wasm = wat::parse_str(
            r#"
            (module
                (import "env" "storage_set"
                    (func $storage_set (param i32 i32 i32 i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "mykey")
                (data (i32.const 5) "myval")
                (func (export "init")
                    (call $storage_set
                        (i32.const 0) (i32.const 5)
                        (i32.const 5) (i32.const 5)))
            )
            "#,
        )
        .unwrap();

        let deploy_receipts = execute_contract_txs(&mut state, &[deploy_tx(deployer, wasm, 0)]);
        assert!(deploy_receipts[0].success);
        let contract_addr = deploy_receipts[0].contract_address.unwrap();

        let call_receipts =
            execute_contract_txs(&mut state, &[call_tx(deployer, contract_addr, "init", 1)]);
        assert!(call_receipts[0].success);
        assert_eq!(
            state.get_storage(&contract_addr, b"mykey").unwrap(),
            b"myval"
        );
    }

    #[test]
    fn contract_address_deterministic() {
        let deployer = [1u8; 32];
        let a1 = compute_contract_address(&deployer, 0);
        let a2 = compute_contract_address(&deployer, 0);
        assert_eq!(a1, a2);

        let a3 = compute_contract_address(&deployer, 1);
        assert_ne!(a1, a3);
    }
}
