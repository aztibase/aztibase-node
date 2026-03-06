use dendrite_storage::{
    ACCOUNTS_TABLE, BATCH_ROOTS_TABLE, CONTRACT_CODE_TABLE, CONTRACT_STORAGE_TABLE, StateStore,
    StorageResult,
};

use crate::state::AccountState;

type Address = [u8; 32];

/// Serialized form of an account record (balance + nonce only).
/// Code and storage are stored in separate tables.
fn serialize_account_record(balance: u64, nonce: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(16);
    buf.extend_from_slice(&balance.to_le_bytes());
    buf.extend_from_slice(&nonce.to_le_bytes());
    buf
}

fn deserialize_account_record(data: &[u8]) -> (u64, u64) {
    let balance = u64::from_le_bytes(data[..8].try_into().unwrap());
    let nonce = u64::from_le_bytes(data[8..16].try_into().unwrap());
    (balance, nonce)
}

/// Composite key for contract storage: address (32 bytes) || storage_key.
fn storage_key(address: &Address, key: &[u8]) -> Vec<u8> {
    let mut composite = Vec::with_capacity(32 + key.len());
    composite.extend_from_slice(address);
    composite.extend_from_slice(key);
    composite
}

/// Flush the in-memory AccountState to redb in a single logical batch.
/// Writes accounts, code, and contract storage across three tables.
pub fn flush_state(store: &StateStore, state: &AccountState) -> StorageResult<()> {
    for (address, account) in state.iter_accounts() {
        let record = serialize_account_record(account.balance, account.nonce);
        store.put(ACCOUNTS_TABLE, address, &record)?;

        if !account.code.is_empty() {
            store.put(CONTRACT_CODE_TABLE, address, &account.code)?;
        }

        for (k, v) in &account.storage {
            let composite = storage_key(address, k);
            store.put(CONTRACT_STORAGE_TABLE, &composite, v)?;
        }
    }
    Ok(())
}

/// Load full AccountState from redb.
pub fn load_state(store: &StateStore) -> StorageResult<AccountState> {
    let mut state = AccountState::new();

    let accounts = store.iter(ACCOUNTS_TABLE)?;
    for (addr_bytes, record_bytes) in &accounts {
        if addr_bytes.len() != 32 || record_bytes.len() < 16 {
            continue;
        }
        let address: Address = addr_bytes.as_slice().try_into().unwrap();
        let (balance, nonce) = deserialize_account_record(record_bytes);

        if balance > 0 || nonce > 0 {
            state.set_balance(&address, balance);
            let acct = state.get_mut(&address);
            acct.nonce = nonce;
        }
    }

    let code_entries = store.iter(CONTRACT_CODE_TABLE)?;
    for (addr_bytes, code) in code_entries {
        if addr_bytes.len() != 32 {
            continue;
        }
        let address: Address = addr_bytes.as_slice().try_into().unwrap();
        state.set_code(&address, code);
    }

    let storage_entries = store.iter(CONTRACT_STORAGE_TABLE)?;
    for (composite_key, value) in storage_entries {
        if composite_key.len() <= 32 {
            continue;
        }
        let address: Address = composite_key[..32].try_into().unwrap();
        let key = composite_key[32..].to_vec();
        state.set_storage(&address, key, value);
    }

    Ok(state)
}

/// Store the state root for a committed batch, keyed by anchor hash.
pub fn store_batch_root(
    store: &StateStore,
    anchor_hash: &[u8; 32],
    state_root: &[u8; 32],
) -> StorageResult<()> {
    store.put(BATCH_ROOTS_TABLE, anchor_hash, state_root)
}

/// Retrieve the state root for a committed batch by anchor hash.
pub fn get_batch_root(
    store: &StateStore,
    anchor_hash: &[u8; 32],
) -> StorageResult<Option<[u8; 32]>> {
    match store.get(BATCH_ROOTS_TABLE, anchor_hash)? {
        Some(bytes) if bytes.len() == 32 => Ok(Some(bytes.as_slice().try_into().unwrap())),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_persist_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    #[test]
    fn flush_and_load_balances() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);
        state.set_balance(&bob, 500);
        state.increment_nonce(&alice);

        flush_state(&store, &state).unwrap();

        let loaded = load_state(&store).unwrap();
        assert_eq!(loaded.balance(&alice), 1000);
        assert_eq!(loaded.balance(&bob), 500);
        assert_eq!(loaded.nonce(&alice), 1);
        assert_eq!(loaded.nonce(&bob), 0);

        cleanup(&path);
    }

    #[test]
    fn flush_and_load_code() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut state = AccountState::new();
        let addr = [3u8; 32];
        let wasm = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        state.set_code(&addr, wasm.clone());

        flush_state(&store, &state).unwrap();

        let loaded = load_state(&store).unwrap();
        assert_eq!(loaded.code(&addr).unwrap(), &wasm);

        cleanup(&path);
    }

    #[test]
    fn flush_and_load_contract_storage() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut state = AccountState::new();
        let addr = [4u8; 32];
        state.set_storage(&addr, b"key1".to_vec(), b"val1".to_vec());
        state.set_storage(&addr, b"key2".to_vec(), b"val2".to_vec());

        flush_state(&store, &state).unwrap();

        let loaded = load_state(&store).unwrap();
        assert_eq!(loaded.get_storage(&addr, b"key1").unwrap(), b"val1");
        assert_eq!(loaded.get_storage(&addr, b"key2").unwrap(), b"val2");

        cleanup(&path);
    }

    #[test]
    fn state_root_matches_after_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut state = AccountState::new();
        state.set_balance(&[1u8; 32], 1000);
        state.set_balance(&[2u8; 32], 2000);
        state.increment_nonce(&[1u8; 32]);
        let original_root = state.state_root();

        flush_state(&store, &state).unwrap();
        let loaded = load_state(&store).unwrap();

        assert_eq!(loaded.state_root(), original_root);

        cleanup(&path);
    }

    #[test]
    fn batch_root_store_and_retrieve() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let anchor = [0xAA; 32];
        let root = [0xBB; 32];

        store_batch_root(&store, &anchor, &root).unwrap();
        let loaded = get_batch_root(&store, &anchor).unwrap();
        assert_eq!(loaded, Some(root));

        let missing = get_batch_root(&store, &[0xCC; 32]).unwrap();
        assert_eq!(missing, None);

        cleanup(&path);
    }

    #[test]
    fn load_empty_state() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let loaded = load_state(&store).unwrap();
        assert_eq!(loaded.account_count(), 0);
        assert_eq!(loaded.state_root(), [0u8; 32]);

        cleanup(&path);
    }
}
