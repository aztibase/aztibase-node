pub mod block_stm;
pub mod contract;
pub mod evm;
pub mod fee;
pub mod light_client;
pub mod parallel;
pub mod persist;
pub mod precompiles;
pub mod receipt;
pub mod routing;
pub mod snapshot;
pub mod state;
pub mod tx;
pub mod verkle;
pub mod vm;

pub use contract::{ContractReceipt, ContractTx, compute_contract_address, execute_contract_txs};
pub use fee::{BaseFeeCalculator, FeeEscrow, escrow_fee, refund_unused};
pub use light_client::{build_light_client_proof, verify_light_client_proof};
pub use parallel::{BatchResult, TransferTx, TxReceipt, TxStatus, execute_transfers};
pub use persist::{
    flush_state, get_batch_root, load_base_fee, load_state, store_base_fee, store_batch_root,
};
pub use receipt::{ExecutionReceipt, get_receipt, store_receipts};
pub use routing::{RoutingError, TxKind, route_batch, route_tx};
pub use snapshot::{
    SnapshotError, StateSnapshot, apply_snapshot, create_snapshot, create_snapshot_with_finality,
    deserialize_snapshot, serialize_snapshot, snapshot_hash, snapshot_header_hash,
};
pub use state::{AccountState, AccountType, MerkleCommitment};
pub use tx::{SignedTx, TxError, verify_and_route, verify_and_route_batch};
pub use verkle::{VerkleCommitment, VerkleTree};
pub use vm::{EngineConfig, ExecutionEngine, ExecutionResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_creates_with_defaults() {
        let engine = ExecutionEngine::new(EngineConfig::default());
        assert!(engine.is_ok());
    }

    #[test]
    fn execute_simple_add() {
        let engine = ExecutionEngine::new(EngineConfig::default()).unwrap();

        let wasm = wat::parse_str(
            r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )
            "#,
        )
        .unwrap();

        let result = engine
            .execute(
                &wasm,
                "add",
                &[wasmtime::Val::I32(3), wasmtime::Val::I32(4)],
            )
            .unwrap();

        assert_eq!(result.values.len(), 1);
        assert_eq!(result.values[0].unwrap_i32(), 7);
    }

    #[test]
    fn fuel_is_consumed() {
        let engine = ExecutionEngine::new(EngineConfig::default()).unwrap();

        let wasm = wat::parse_str(
            r#"
            (module
                (func (export "work") (param i32) (result i32)
                    (local $i i32)
                    (local.set $i (i32.const 0))
                    (block $break
                        (loop $loop
                            (br_if $break (i32.ge_u (local.get $i) (local.get 0)))
                            (local.set $i (i32.add (local.get $i) (i32.const 1)))
                            (br $loop)
                        )
                    )
                    local.get $i
                )
            )
            "#,
        )
        .unwrap();

        let result = engine
            .execute(&wasm, "work", &[wasmtime::Val::I32(100)])
            .unwrap();

        assert!(result.fuel_consumed > 0);
        assert_eq!(result.values[0].unwrap_i32(), 100);
    }

    #[test]
    fn fuel_limit_constrains_execution() {
        let low_fuel = ExecutionEngine::new(EngineConfig {
            fuel_limit: 500,
            ..Default::default()
        })
        .unwrap();

        let high_fuel = ExecutionEngine::new(EngineConfig {
            fuel_limit: 100_000_000,
            ..Default::default()
        })
        .unwrap();

        let wasm = wat::parse_str(
            r#"
            (module
                (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add
                )
            )
            "#,
        )
        .unwrap();

        let args = &[wasmtime::Val::I32(1), wasmtime::Val::I32(2)];
        let r1 = low_fuel.execute(&wasm, "add", args).unwrap();
        let r2 = high_fuel.execute(&wasm, "add", args).unwrap();

        assert_eq!(r1.fuel_consumed, r2.fuel_consumed);
        assert!(r1.fuel_consumed > 0);
        assert!(r1.fuel_consumed < 500);
    }

    #[test]
    fn host_storage_roundtrip() {
        let engine = ExecutionEngine::new(EngineConfig::default()).unwrap();

        let wasm = wat::parse_str(
            r#"
            (module
                (import "env" "storage_set" (func $storage_set (param i32 i32 i32 i32)))
                (import "env" "storage_get" (func $storage_get (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "key")
                (data (i32.const 10) "value")

                (func (export "test_roundtrip") (result i32)
                    (call $storage_set (i32.const 0) (i32.const 3) (i32.const 10) (i32.const 5))
                    (call $storage_get (i32.const 0) (i32.const 3) (i32.const 50) (i32.const 10))
                )
            )
            "#,
        )
        .unwrap();

        let result = engine.execute(&wasm, "test_roundtrip", &[]).unwrap();

        assert_eq!(result.values[0].unwrap_i32(), 5);
        assert_eq!(
            result.storage.get(b"key".as_slice()),
            Some(&b"value".to_vec())
        );
    }
}
