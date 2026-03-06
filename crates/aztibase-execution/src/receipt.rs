use aztibase_core::TxHash;
use serde::{Deserialize, Serialize};

use aztibase_storage::{RECEIPTS_TABLE, StateStore, StorageResult};

type Address = [u8; 32];

/// Unified receipt for all transaction types (transfers, WASM contracts, EVM, AI inference).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    pub tx_hash: TxHash,
    pub success: bool,
    pub gas_used: u64,
    pub contract_address: Option<Address>,
    pub error: Option<String>,
    /// Deterministic hash for AI inference verification (None for non-AI transactions).
    pub inference_hash: Option<[u8; 32]>,
}

/// Store a batch of receipts atomically alongside state flush.
pub fn store_receipts(store: &StateStore, receipts: &[ExecutionReceipt]) -> StorageResult<()> {
    let serialized: Vec<(TxHash, Vec<u8>)> = receipts
        .iter()
        .filter_map(|r| bincode::serialize(r).ok().map(|data| (r.tx_hash, data)))
        .collect();

    let refs: Vec<(&[u8], &[u8])> = serialized
        .iter()
        .map(|(hash, data)| (hash.as_slice(), data.as_slice()))
        .collect();

    store.batch_put(RECEIPTS_TABLE, &refs)
}

/// Retrieve a receipt by transaction hash.
pub fn get_receipt(
    store: &StateStore,
    tx_hash: &TxHash,
) -> StorageResult<Option<ExecutionReceipt>> {
    match store.get(RECEIPTS_TABLE, tx_hash)? {
        Some(bytes) => match bincode::deserialize(&bytes) {
            Ok(receipt) => Ok(Some(receipt)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::hash;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_receipt_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    #[test]
    fn store_and_retrieve_receipt() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let tx_hash = hash(b"test-tx-1");
        let receipt = ExecutionReceipt {
            tx_hash,
            success: true,
            gas_used: 21_000,
            contract_address: None,
            error: None,
            inference_hash: None,
        };

        store_receipts(&store, &[receipt.clone()]).unwrap();
        let loaded = get_receipt(&store, &tx_hash).unwrap().unwrap();

        assert_eq!(loaded.tx_hash, tx_hash);
        assert!(loaded.success);
        assert_eq!(loaded.gas_used, 21_000);
        assert!(loaded.contract_address.is_none());
        assert!(loaded.error.is_none());

        cleanup(&path);
    }

    #[test]
    fn missing_receipt_returns_none() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let missing = hash(b"nonexistent");
        assert!(get_receipt(&store, &missing).unwrap().is_none());

        cleanup(&path);
    }

    #[test]
    fn store_failed_receipt_with_error() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let tx_hash = hash(b"failed-tx");
        let receipt = ExecutionReceipt {
            tx_hash,
            success: false,
            gas_used: 21_000,
            contract_address: None,
            error: Some("insufficient balance".into()),
            inference_hash: None,
        };

        store_receipts(&store, &[receipt]).unwrap();
        let loaded = get_receipt(&store, &tx_hash).unwrap().unwrap();

        assert!(!loaded.success);
        assert_eq!(loaded.error.as_deref(), Some("insufficient balance"));

        cleanup(&path);
    }

    #[test]
    fn store_deploy_receipt_with_contract_address() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let tx_hash = hash(b"deploy-tx");
        let contract_addr = [0xCC; 32];
        let receipt = ExecutionReceipt {
            tx_hash,
            success: true,
            gas_used: 150_000,
            contract_address: Some(contract_addr),
            error: None,
            inference_hash: None,
        };

        store_receipts(&store, &[receipt]).unwrap();
        let loaded = get_receipt(&store, &tx_hash).unwrap().unwrap();

        assert_eq!(loaded.contract_address, Some(contract_addr));
        assert_eq!(loaded.gas_used, 150_000);

        cleanup(&path);
    }

    #[test]
    fn batch_store_multiple_receipts() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let receipts: Vec<ExecutionReceipt> = (0..5)
            .map(|i| {
                let tx_hash = hash(format!("tx-{i}").as_bytes());
                ExecutionReceipt {
                    tx_hash,
                    success: i % 2 == 0,
                    gas_used: 21_000 * (i + 1),
                    contract_address: None,
                    error: if i % 2 != 0 {
                        Some("failed".into())
                    } else {
                        None
                    },
                    inference_hash: None,
                }
            })
            .collect();

        store_receipts(&store, &receipts).unwrap();

        for (i, r) in receipts.iter().enumerate() {
            let loaded = get_receipt(&store, &r.tx_hash).unwrap().unwrap();
            assert_eq!(loaded.success, i % 2 == 0);
            assert_eq!(loaded.gas_used, 21_000 * (i as u64 + 1));
        }

        cleanup(&path);
    }
}
