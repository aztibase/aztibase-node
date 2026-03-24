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
const MAX_REVERT_REASON_BYTES: usize = 1024;

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

fn load_account(db: &mut CacheDB<EmptyDB>, state: &AccountState, addr: &Address) {
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

fn build_db(state: &AccountState, addresses: &[Address]) -> CacheDB<EmptyDB> {
    let mut db = CacheDB::new(EmptyDB::default());
    let mut loaded = std::collections::HashSet::new();

    for addr in addresses {
        load_account(&mut db, state, addr);
        loaded.insert(*addr);
    }

    for (addr, acct) in state.iter_accounts() {
        if !loaded.contains(addr) && !acct.code.is_empty() {
            load_account(&mut db, state, addr);
        }
    }

    db
}

fn apply_state_changes(state: &mut AccountState, evm_state: revm::state::EvmState) {
    for (evm_addr, account) in &evm_state {
        let addr = from_evm_address(evm_addr);
        let info = &account.info;

        let balance_u128: u128 = info.balance.try_into().unwrap_or(u128::MAX);
        state.set_balance(&addr, balance_u128);
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
                    error: Some(format!("revert: 0x{}", hex_encode_bounded(&output))),
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
    value: u128,
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
                    error: Some(format!("revert: 0x{}", hex_encode_bounded(&output))),
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

pub struct StaticCallResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub gas_used: u64,
    pub error: Option<String>,
}

pub fn evm_static_call(
    state: &AccountState,
    caller: &Address,
    contract: &Address,
    calldata: &[u8],
    gas_limit: u64,
) -> StaticCallResult {
    let db = build_db(state, &[*caller, *contract]);
    let ctx = Context::mainnet().with_db(db).modify_cfg_chained(|cfg| {
        cfg.chain_id = AZTIBASE_CHAIN_ID;
    });

    let tx = match TxEnv::builder()
        .caller(to_evm_address(caller))
        .kind(EvmTxKind::Call(to_evm_address(contract)))
        .data(Bytes::copy_from_slice(calldata))
        .gas_limit(gas_limit)
        .nonce(state.nonce(caller))
        .gas_price(0)
        .value(U256::ZERO)
        .chain_id(Some(AZTIBASE_CHAIN_ID))
        .build()
    {
        Ok(tx) => tx,
        Err(e) => {
            return StaticCallResult {
                success: false,
                output: vec![],
                gas_used: 0,
                error: Some(format!("tx build failed: {e}")),
            };
        }
    };

    let mut evm = ctx.build_mainnet();
    match evm.transact(tx) {
        Ok(result_and_state) => {
            let gas_used = result_and_state.result.gas().spent();
            match result_and_state.result {
                ExecutionResult::Success { output, .. } => {
                    let bytes = match output {
                        revm::context_interface::result::Output::Call(b) => b.to_vec(),
                        revm::context_interface::result::Output::Create(b, _) => b.to_vec(),
                    };
                    StaticCallResult {
                        success: true,
                        output: bytes,
                        gas_used,
                        error: None,
                    }
                }
                ExecutionResult::Revert { output, .. } => StaticCallResult {
                    success: false,
                    output: output.to_vec(),
                    gas_used,
                    error: Some(format!("revert: 0x{}", hex_encode_bounded(&output))),
                },
                ExecutionResult::Halt { reason, .. } => StaticCallResult {
                    success: false,
                    output: vec![],
                    gas_used,
                    error: Some(format!("halt: {reason:?}")),
                },
            }
        }
        Err(e) => StaticCallResult {
            success: false,
            output: vec![],
            gas_used: 0,
            error: Some(format!("evm error: {e}")),
        },
    }
}

fn hex_encode_bounded(bytes: &[u8]) -> String {
    let truncated = bytes.len() > MAX_REVERT_REASON_BYTES;
    let slice = if truncated {
        &bytes[..MAX_REVERT_REASON_BYTES]
    } else {
        bytes
    };
    let mut s = String::with_capacity(slice.len() * 2 + 12);
    for b in slice {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    if truncated {
        s.push_str("..truncated");
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
    fn evm_deploy_gas_nonzero_and_bounded() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        let bytecode = vec![0x60, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];
        let receipt = evm_deploy(
            &mut state,
            hash(b"gas-check-deploy"),
            &deployer,
            &bytecode,
            0,
            1_000_000,
        );
        assert!(receipt.success);
        assert!(
            receipt.gas_used > 21_000,
            "deploy should cost more than base tx"
        );
        assert!(
            receipt.gas_used < 1_000_000,
            "gas_used must be less than gas_limit"
        );
    }

    #[test]
    fn evm_call_gas_nonzero_and_bounded() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        let init_code = vec![0x60, 0x00, 0x60, 0x00, 0x53, 0x60, 0x01, 0x60, 0x00, 0xf3];
        let deploy_receipt = evm_deploy(
            &mut state,
            hash(b"gas-check-call-deploy"),
            &deployer,
            &init_code,
            0,
            1_000_000,
        );
        assert!(deploy_receipt.success);
        let contract_addr = deploy_receipt.contract_address.unwrap();

        let call_receipt = evm_call(
            &mut state,
            hash(b"gas-check-call"),
            &deployer,
            &contract_addr,
            &[],
            1,
            500_000,
            0,
        );
        assert!(call_receipt.success);
        assert!(call_receipt.gas_used > 0, "call should consume gas");
        assert!(
            call_receipt.gas_used < 500_000,
            "gas_used must be less than gas_limit"
        );
    }

    #[test]
    fn evm_static_call_reads_without_modifying_state() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        // Runtime: PUSH1 0x42, PUSH1 0, MSTORE, PUSH1 32, PUSH1 0, RETURN
        let runtime = [0x60u8, 0x42, 0x60, 0x00, 0x52, 0x60, 0x20, 0x60, 0x00, 0xf3];
        // Init: PUSH10 <runtime>, PUSH1 0, MSTORE, PUSH1 10, PUSH1 22, RETURN
        let mut init_code = vec![0x69];
        init_code.extend_from_slice(&runtime);
        init_code.extend_from_slice(&[0x60, 0x00, 0x52, 0x60, 0x0a, 0x60, 0x16, 0xf3]);

        let receipt = evm_deploy(
            &mut state,
            hash(b"static-deploy"),
            &deployer,
            &init_code,
            0,
            1_000_000,
        );
        assert!(receipt.success);
        let contract_addr = receipt.contract_address.unwrap();
        let balance_before = state.balance(&deployer);
        let nonce_before = state.nonce(&deployer);

        let result = evm_static_call(&state, &deployer, &contract_addr, &[], 1_000_000);
        assert!(result.success);
        assert_eq!(result.output.len(), 32);
        assert_eq!(result.output[31], 0x42);
        assert!(result.gas_used > 0);

        assert_eq!(state.balance(&deployer), balance_before);
        assert_eq!(state.nonce(&deployer), nonce_before);
    }

    #[test]
    fn evm_static_call_nonexistent_contract() {
        let state = AccountState::new();
        let caller = [1u8; 32];
        let fake_contract = [99u8; 32];
        let result = evm_static_call(&state, &caller, &fake_contract, &[0xDE, 0xAD], 1_000_000);
        assert!(result.success);
        assert!(result.output.is_empty());
    }

    #[test]
    fn evm_revert_reason_truncated() {
        let mut state = AccountState::new();
        let deployer = [1u8; 32];
        state.set_balance(&deployer, 1_000_000_000);

        // Deploy a contract that always reverts with large data:
        // PUSH2 0x0800 PUSH1 0x00 REVERT (reverts with 2048 zero bytes from memory)
        let init_code = vec![
            // Runtime code: PUSH2 0x0800, PUSH1 0x00, REVERT
            0x61, 0x08, 0x00, // PUSH2 2048
            0x60, 0x00, // PUSH1 0
            0xfd, // REVERT
        ];

        // We'll deploy runtime = the revert code, using a deployer that stores it
        let mut deploy_code = Vec::new();
        // Copy init_code into memory and return it as deployed code
        for (i, byte) in init_code.iter().enumerate() {
            deploy_code.push(0x60); // PUSH1
            deploy_code.push(*byte);
            deploy_code.push(0x60); // PUSH1
            deploy_code.push(i as u8);
            deploy_code.push(0x53); // MSTORE8
        }
        deploy_code.push(0x60); // PUSH1 len
        deploy_code.push(init_code.len() as u8);
        deploy_code.push(0x60); // PUSH1 0
        deploy_code.push(0x00);
        deploy_code.push(0xf3); // RETURN

        let deploy_receipt = evm_deploy(
            &mut state,
            hash(b"revert-deploy"),
            &deployer,
            &deploy_code,
            0,
            2_000_000,
        );
        assert!(deploy_receipt.success);
        let contract_addr = deploy_receipt.contract_address.unwrap();

        let call_receipt = evm_call(
            &mut state,
            hash(b"revert-call"),
            &deployer,
            &contract_addr,
            &[],
            1,
            1_000_000,
            0,
        );
        assert!(!call_receipt.success);
        let err = call_receipt.error.unwrap();
        assert!(err.starts_with("revert: 0x"));
        // The hex output should be bounded: 1024 bytes = 2048 hex chars + "..truncated"
        assert!(
            err.len() < 2048 * 2 + 50,
            "revert reason should be truncated"
        );
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
