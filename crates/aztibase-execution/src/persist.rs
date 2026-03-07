use aztibase_storage::{
    ACCOUNTS_TABLE, BATCH_ROOTS_TABLE, CONTRACT_CODE_TABLE, CONTRACT_STORAGE_TABLE, STATE_TABLE,
    StateStore, StorageResult, TableDef,
};

use crate::state::{AccountState, AccountType};

type Address = [u8; 32];

fn serialize_account_record(
    balance: u64,
    nonce: u64,
    account_type: &AccountType,
    model_id: &Option<String>,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    buf.extend_from_slice(&balance.to_le_bytes());
    buf.extend_from_slice(&nonce.to_le_bytes());
    buf.push(account_type.discriminant());
    match model_id {
        Some(mid) => {
            let mid_bytes = mid.as_bytes();
            buf.extend_from_slice(&(mid_bytes.len() as u16).to_le_bytes());
            buf.extend_from_slice(mid_bytes);
        }
        None => {
            buf.extend_from_slice(&0u16.to_le_bytes());
        }
    }
    buf
}

struct AccountRecord {
    balance: u64,
    nonce: u64,
    account_type: AccountType,
    model_id: Option<String>,
}

fn deserialize_account_record(data: &[u8]) -> Option<AccountRecord> {
    if data.len() < 16 {
        return None;
    }
    let balance = u64::from_le_bytes(data[..8].try_into().ok()?);
    let nonce = u64::from_le_bytes(data[8..16].try_into().ok()?);

    let account_type = if data.len() > 16 {
        match data[16] {
            0 => AccountType::EOA,
            1 => AccountType::Contract,
            2 => AccountType::AIAgent,
            _ => AccountType::EOA,
        }
    } else {
        AccountType::EOA
    };

    let model_id = if data.len() > 19 {
        let mid_len = u16::from_le_bytes(data[17..19].try_into().ok()?) as usize;
        if mid_len > 0 && data.len() >= 19 + mid_len {
            Some(String::from_utf8_lossy(&data[19..19 + mid_len]).into_owned())
        } else {
            None
        }
    } else {
        None
    };

    Some(AccountRecord {
        balance,
        nonce,
        account_type,
        model_id,
    })
}

fn storage_key(address: &Address, key: &[u8]) -> Vec<u8> {
    let mut composite = Vec::with_capacity(32 + key.len());
    composite.extend_from_slice(address);
    composite.extend_from_slice(key);
    composite
}

/// Flush the in-memory AccountState to redb in a single atomic transaction.
/// Writes accounts, code, and contract storage across three tables.
pub fn flush_state(store: &StateStore, state: &AccountState) -> StorageResult<()> {
    let mut owned: Vec<(TableDef, Vec<u8>, Vec<u8>)> = Vec::new();

    for (address, account) in state.iter_accounts() {
        let record = serialize_account_record(
            account.balance,
            account.nonce,
            &account.account_type,
            &account.model_id,
        );
        owned.push((ACCOUNTS_TABLE, address.to_vec(), record));

        if !account.code.is_empty() {
            owned.push((CONTRACT_CODE_TABLE, address.to_vec(), account.code.clone()));
        }

        for (k, v) in &account.storage {
            owned.push((CONTRACT_STORAGE_TABLE, storage_key(address, k), v.clone()));
        }
    }

    let mut refs: Vec<(TableDef, &[u8], &[u8])> = Vec::with_capacity(owned.len());
    for (t, k, v) in &owned {
        refs.push((*t, k.as_slice(), v.as_slice()));
    }

    store.batch_put_multi(&refs)
}

/// Load full AccountState from redb.
pub fn load_state(store: &StateStore) -> StorageResult<AccountState> {
    let mut state = AccountState::new();

    let accounts = store.iter(ACCOUNTS_TABLE)?;
    for (addr_bytes, record_bytes) in &accounts {
        let address: Address = match addr_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };
        let record = match deserialize_account_record(record_bytes) {
            Some(r) => r,
            None => continue,
        };

        if record.balance > 0 || record.nonce > 0 || record.account_type != AccountType::EOA {
            state.set_balance(&address, record.balance);
            let acct = state.get_mut(&address);
            acct.nonce = record.nonce;
            acct.account_type = record.account_type;
            acct.model_id = record.model_id;
        }
    }

    let code_entries = store.iter(CONTRACT_CODE_TABLE)?;
    for (addr_bytes, code) in code_entries {
        let address: Address = match addr_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };
        state.set_code(&address, code);
    }

    let storage_entries = store.iter(CONTRACT_STORAGE_TABLE)?;
    for (composite_key, value) in storage_entries {
        if composite_key.len() < 32 {
            continue;
        }
        let address: Address = match composite_key[..32].try_into() {
            Ok(a) => a,
            Err(_) => continue,
        };
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

const BASE_FEE_KEY: &[u8] = b"base_fee";

/// Store the current base fee value.
pub fn store_base_fee(store: &StateStore, base_fee: u64) -> StorageResult<()> {
    store.put(STATE_TABLE, BASE_FEE_KEY, &base_fee.to_le_bytes())
}

/// Load the persisted base fee, returning `None` if not set.
pub fn load_base_fee(store: &StateStore) -> StorageResult<Option<u64>> {
    match store.get(STATE_TABLE, BASE_FEE_KEY)? {
        Some(bytes) if bytes.len() == 8 => Ok(Some(u64::from_le_bytes(
            bytes.as_slice().try_into().unwrap(),
        ))),
        _ => Ok(None),
    }
}

/// Retrieve the state root for a committed batch by anchor hash.
pub fn get_batch_root(
    store: &StateStore,
    anchor_hash: &[u8; 32],
) -> StorageResult<Option<[u8; 32]>> {
    match store.get(BATCH_ROOTS_TABLE, anchor_hash)? {
        Some(bytes) if bytes.len() == 32 => {
            Ok(Some(bytes.as_slice().try_into().unwrap_or([0u8; 32])))
        }
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
        std::env::temp_dir().join(format!("aztibase_persist_test_{}_{}", pid, id))
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

    #[test]
    fn base_fee_store_and_load() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert_eq!(load_base_fee(&store).unwrap(), None);

        store_base_fee(&store, 42).unwrap();
        assert_eq!(load_base_fee(&store).unwrap(), Some(42));

        store_base_fee(&store, 1000).unwrap();
        assert_eq!(load_base_fee(&store).unwrap(), Some(1000));

        cleanup(&path);
    }
}
