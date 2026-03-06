use revm::Context;
use revm::context::TxEnv;
use revm::context_interface::result::ExecutionResult;
use revm::database::{CacheDB, EmptyDB};
use revm::handler::{ExecuteEvm, MainBuilder, MainContext};
use revm::primitives::{Address as EvmAddress, Bytes, TxKind as EvmTxKind, U256};

use crate::contract::ContractReceipt;
use crate::state::AccountState;
use aztibase_core::TxHash;

type Address = [u8; 32];

const AZTIBASE_CHAIN_ID: u64 = 0xA27B;

fn to_evm_address(addr: &Address) -> EvmAddress {
    let mut buf = [0u8; 20];
    buf.copy_from_slice(&addr[..20]);
    EvmAddress::from(buf)
}

fn from_evm_address(addr: &EvmAddress) -> Address {
    let mut buf = [0u8; 32];
    buf[..20].copy_from_slice(addr.as_slice());
    buf
}

fn build_db(state: &AccountState, addresses: &[Address]) -> CacheDB<EmptyDB> {
    let mut db = CacheDB::new(EmptyDB::default());
    for addr in addresses {
        let evm_addr = to_evm_address(addr);
        let info = revm::state::AccountInfo {
            balance: U256::from(state.balance(addr)),
            nonce: state.nonce(addr),
            code_hash: revm::primitives::KECCAK_EMPTY,
            account_id: None,
            code: state
                .code(addr)
                .map(|c| revm::bytecode::Bytecode::new_legacy(Bytes::copy_from_slice(c))),
        };
        db.insert_account_info(evm_addr, info);

        if let Some(acct) = state.get(addr) {
            for (k, v) in &acct.storage {
                let slot = U256::from_be_slice(k);
                let value = U256::from_be_slice(v);
                let _ = db.insert_account_storage(evm_addr, slot, value);
            }
        }
    }
    db
}

fn apply_state_changes(state: &mut AccountState, evm_state: revm::state::EvmState) {
    for (evm_addr, account) in &evm_state {
        let addr = from_evm_address(evm_addr);
        let info = &account.info;

        let balance_u64 = info.balance.try_into().unwrap_or(u64::MAX);
        state.set_balance(&addr, balance_u64);
        state.get_mut(&addr).nonce = info.nonce;

        if let Some(ref code) = info.code {
            let bytecode = code.original_byte_slice();
            if !bytecode.is_empty() {
                state.set_code(&addr, bytecode.to_vec());
            }
        }

        for (slot, value) in &account.storage {
            let key_buf = slot.to_be_bytes::<32>();
            let val_buf = value.present_value().to_be_bytes::<32>();
            state.set_storage(&addr, key_buf.to_vec(), val_buf.to_vec());
        }
    }
}

/// Deploy EVM bytecode. Returns a ContractReceipt with the created address.
pub fn evm_deploy(
    state: &mut AccountState,
    tx_hash: TxHash,
    deployer: &Address,
    code: &[u8],
    nonce: u64,
    gas_limit: u64,
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

    let db = build_db(state, &[*deployer]);
    let ctx = Context::mainnet().with_db(db).modify_cfg_chained(|cfg| {
        cfg.chain_id = AZTIBASE_CHAIN_ID;
    });

    let tx = match TxEnv::builder()
        .caller(to_evm_address(deployer))
        .kind(EvmTxKind::Create)
        .data(Bytes::copy_from_slice(code))
        .gas_limit(gas_limit)
        .nonce(nonce)
        .gas_price(0)
        .chain_id(Some(AZTIBASE_CHAIN_ID))
        .build()
    {
        Ok(tx) => tx,
        Err(e) => {
            state.increment_nonce(deployer);
            return ContractReceipt {
                tx_hash,
                success: false,
                gas_used: 21_000,
                contract_address: None,
                events: vec![],
                error: Some(format!("tx build failed: {e}")),
            };
        }
    };

    let mut evm = ctx.build_mainnet();
    match evm.transact(tx) {
        Ok(result_and_state) => {
            let exec_result = result_and_state.result;
            let evm_state = result_and_state.state;

            apply_state_changes(state, evm_state);
            state.increment_nonce(deployer);

            let gas_used = exec_result.gas().spent();
            match exec_result {
                ExecutionResult::Success { output, .. } => {
                    let created = output.address().map(from_evm_address);
                    ContractReceipt {
                        tx_hash,
                        success: true,
                        gas_used,
                        contract_address: created,
                        events: vec![],
                        error: None,
                    }
                }
                ExecutionResult::Revert { output, .. } => ContractReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    contract_address: None,
                    events: vec![],
                    error: Some(format!("revert: 0x{}", hex_encode(&output))),
                },
                ExecutionResult::Halt { reason, .. } => ContractReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    contract_address: None,
                    events: vec![],
                    error: Some(format!("halt: {reason:?}")),
                },
            }
        }
        Err(e) => {
            state.increment_nonce(deployer);
            ContractReceipt {
                tx_hash,
                success: false,
                gas_used: 21_000,
                contract_address: None,
                events: vec![],
                error: Some(format!("evm error: {e}")),
            }
        }
    }
}

/// Call an EVM contract. Returns a ContractReceipt.
#[allow(clippy::too_many_arguments)]
pub fn evm_call(
    state: &mut AccountState,
    tx_hash: TxHash,
    caller: &Address,
    contract: &Address,
    calldata: &[u8],
    nonce: u64,
    gas_limit: u64,
    value: u64,
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

    let db = build_db(state, &[*caller, *contract]);
    let ctx = Context::mainnet().with_db(db).modify_cfg_chained(|cfg| {
        cfg.chain_id = AZTIBASE_CHAIN_ID;
    });

    let tx = match TxEnv::builder()
        .caller(to_evm_address(caller))
        .kind(EvmTxKind::Call(to_evm_address(contract)))
        .data(Bytes::copy_from_slice(calldata))
        .gas_limit(gas_limit)
        .nonce(nonce)
        .gas_price(0)
        .value(U256::from(value))
        .chain_id(Some(AZTIBASE_CHAIN_ID))
        .build()
    {
        Ok(tx) => tx,
        Err(e) => {
            state.increment_nonce(caller);
            return ContractReceipt {
                tx_hash,
                success: false,
                gas_used: 21_000,
                contract_address: None,
                events: vec![],
                error: Some(format!("tx build failed: {e}")),
            };
        }
    };

    let mut evm = ctx.build_mainnet();
    match evm.transact(tx) {
        Ok(result_and_state) => {
            let exec_result = result_and_state.result;
            let evm_state = result_and_state.state;

            apply_state_changes(state, evm_state);
            state.increment_nonce(caller);

            let gas_used = exec_result.gas().spent();
            match exec_result {
                ExecutionResult::Success { .. } => ContractReceipt {
                    tx_hash,
                    success: true,
                    gas_used,
                    contract_address: Some(*contract),
                    events: vec![],
                    error: None,
                },
                ExecutionResult::Revert { output, .. } => ContractReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    contract_address: None,
                    events: vec![],
                    error: Some(format!("revert: 0x{}", hex_encode(&output))),
                },
                ExecutionResult::Halt { reason, .. } => ContractReceipt {
                    tx_hash,
                    success: false,
                    gas_used,
                    contract_address: None,
                    events: vec![],
                    error: Some(format!("halt: {reason:?}")),
                },
            }
        }
        Err(e) => {
            state.increment_nonce(caller);
            ContractReceipt {
                tx_hash,
                success: false,
                gas_used: 21_000,
                contract_address: None,
                events: vec![],
                error: Some(format!("evm error: {e}")),
            }
        }
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::hash;

    #[test]
    fn evm_deploy_simple_contract() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        // Minimal EVM init code: stores 0x42 at memory[0] and returns 32 bytes
        let bytecode = vec![
            0x60, 0x42, // PUSH1 0x42
            0x60, 0x00, // PUSH1 0x00
            0x52, // MSTORE
            0x60, 0x20, // PUSH1 0x20
            0x60, 0x00, // PUSH1 0x00
            0xf3, // RETURN
        ];

        let receipt = evm_deploy(
            &mut state,
            hash(b"deploy-evm-1"),
            &deployer,
            &bytecode,
            0,
            1_000_000,
        );

        assert!(
            receipt.success,
            "deploy should succeed: {:?}",
            receipt.error
        );
        assert!(receipt.contract_address.is_some());
        assert!(receipt.gas_used > 0);
    }

    #[test]
    fn evm_deploy_nonce_mismatch() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        let receipt = evm_deploy(
            &mut state,
            hash(b"nonce-bad"),
            &deployer,
            &[0x60, 0x00, 0x60, 0x00, 0xf3],
            5, // wrong nonce
            1_000_000,
        );

        assert!(!receipt.success);
        assert!(receipt.error.as_deref().unwrap().contains("nonce"));
    }

    #[test]
    fn evm_deploy_and_call() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        // Init code: deploys a 1-byte runtime (STOP = 0x00)
        let init_code = vec![
            0x60, 0x00, // PUSH1 0x00 (STOP opcode)
            0x60, 0x00, // PUSH1 0x00 (memory offset)
            0x53, // MSTORE8
            0x60, 0x01, // PUSH1 0x01 (length)
            0x60, 0x00, // PUSH1 0x00 (offset)
            0xf3, // RETURN
        ];

        let deploy_receipt = evm_deploy(
            &mut state,
            hash(b"deploy-for-call"),
            &deployer,
            &init_code,
            0,
            1_000_000,
        );

        assert!(
            deploy_receipt.success,
            "deploy failed: {:?}",
            deploy_receipt.error
        );
        let contract_addr = deploy_receipt.contract_address.unwrap();

        let call_receipt = evm_call(
            &mut state,
            hash(b"call-evm-1"),
            &deployer,
            &contract_addr,
            &[],
            1,
            1_000_000,
            0,
        );

        assert!(
            call_receipt.success,
            "call failed: {:?}",
            call_receipt.error
        );
        assert!(call_receipt.gas_used > 0);
    }
}
