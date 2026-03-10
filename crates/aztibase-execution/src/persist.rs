use aztibase_storage::{
    ACCOUNTS_TABLE, BATCH_INDEX_TABLE, BATCH_ROOTS_TABLE, BATCH_TXS_TABLE, CONTRACT_CODE_TABLE,
    CONTRACT_STORAGE_TABLE, STATE_TABLE, StateStore, StorageResult, TX_TABLE, TableDef,
};

use crate::state::{AccountState, AccountType};

type Address = [u8; 32];

fn serialize_account_record(
    balance: u128,
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
    balance: u128,
    nonce: u64,
    account_type: AccountType,
    model_id: Option<String>,
}

fn deserialize_account_record(data: &[u8]) -> Option<AccountRecord> {
    if data.len() < 24 {
        return None;
    }
    let balance = u128::from_le_bytes(data[..16].try_into().ok()?);
    let nonce = u64::from_le_bytes(data[16..24].try_into().ok()?);

    let account_type = if data.len() > 24 {
        match data[24] {
            0 => AccountType::EOA,
            1 => AccountType::Contract,
            2 => AccountType::AIAgent,
            _ => AccountType::EOA,
        }
    } else {
        AccountType::EOA
    };

    let model_id = if data.len() > 27 {
        let mid_len = u16::from_le_bytes(data[25..27].try_into().ok()?) as usize;
        if mid_len > 0 && data.len() >= 27 + mid_len {
            Some(String::from_utf8_lossy(&data[27..27 + mid_len]).into_owned())
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

const MAX_STORED_TXS: u64 = 500_000;
const MAX_STORED_BATCH_ROOTS: u64 = 100_000;

/// Evict old transactions if the store exceeds `MAX_STORED_TXS`.
pub fn evict_old_transactions(store: &StateStore) -> StorageResult<u64> {
    store.evict_oldest(TX_TABLE, MAX_STORED_TXS)
}

/// Evict old batch roots if the store exceeds `MAX_STORED_BATCH_ROOTS`.
pub fn evict_old_batch_roots(store: &StateStore) -> StorageResult<u64> {
    store.evict_oldest(BATCH_ROOTS_TABLE, MAX_STORED_BATCH_ROOTS)
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

/// Store the batch number → anchor hash mapping for sequential block queries.
pub fn store_batch_index(
    store: &StateStore,
    batch_number: u64,
    anchor_hash: &[u8; 32],
) -> StorageResult<()> {
    store.put(BATCH_INDEX_TABLE, &batch_number.to_be_bytes(), anchor_hash)
}

/// Look up the anchor hash for a given batch number.
pub fn get_batch_by_number(
    store: &StateStore,
    batch_number: u64,
) -> StorageResult<Option<[u8; 32]>> {
    match store.get(BATCH_INDEX_TABLE, &batch_number.to_be_bytes())? {
        Some(bytes) if bytes.len() == 32 => {
            Ok(Some(bytes.as_slice().try_into().unwrap_or([0u8; 32])))
        }
        _ => Ok(None),
    }
}

/// Store the list of transaction hashes for a committed batch.
pub fn store_batch_txs(
    store: &StateStore,
    anchor_hash: &[u8; 32],
    tx_hashes: &[[u8; 32]],
) -> StorageResult<()> {
    let Ok(data) = postcard::to_allocvec(tx_hashes) else {
        return Ok(());
    };
    store.put(BATCH_TXS_TABLE, anchor_hash, &data)
}

/// Retrieve the list of transaction hashes for a committed batch.
pub fn get_batch_txs(
    store: &StateStore,
    anchor_hash: &[u8; 32],
) -> StorageResult<Option<Vec<[u8; 32]>>> {
    match store.get(BATCH_TXS_TABLE, anchor_hash)? {
        Some(bytes) => match postcard::from_bytes::<Vec<[u8; 32]>>(&bytes) {
            Ok(hashes) => Ok(Some(hashes)),
            Err(_) => Ok(None),
        },
        None => Ok(None),
    }
}

/// Store a raw transaction keyed by its hash.
pub fn store_transaction(
    store: &StateStore,
    tx_hash: &[u8; 32],
    tx_data: &[u8],
) -> StorageResult<()> {
    store.put(TX_TABLE, tx_hash, tx_data)
}

/// Retrieve a raw transaction by its hash.
pub fn get_transaction(store: &StateStore, tx_hash: &[u8; 32]) -> StorageResult<Option<Vec<u8>>> {
    store.get(TX_TABLE, tx_hash)
}

/// Query a range of batch numbers, returning (batch_number, anchor_hash) pairs.
/// Enforces a hard limit of `max_results` entries per call.
pub fn get_batch_range(
    store: &StateStore,
    from: u64,
    to: u64,
    max_results: usize,
) -> StorageResult<Vec<(u64, [u8; 32])>> {
    let start = from.to_be_bytes();
    let end = to.saturating_add(1).to_be_bytes();
    let raw = store.range(BATCH_INDEX_TABLE, &start, &end)?;
    let results: Vec<(u64, [u8; 32])> = raw
        .into_iter()
        .take(max_results)
        .filter_map(|(k, v)| {
            if k.len() == 8 && v.len() == 32 {
                let num = u64::from_be_bytes(k.as_slice().try_into().ok()?);
                let hash: [u8; 32] = v.as_slice().try_into().ok()?;
                Some((num, hash))
            } else {
                None
            }
        })
        .collect();
    Ok(results)
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
    fn delete_batch_removes_keys_atomically() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        for i in 0..5u8 {
            store.put(BATCH_ROOTS_TABLE, &[i; 32], &[0xAA; 32]).unwrap();
        }
        assert_eq!(store.len(BATCH_ROOTS_TABLE).unwrap(), 5);

        let keys: Vec<Vec<u8>> = (0..3u8).map(|i| [i; 32].to_vec()).collect();
        let removed = store.delete_batch(BATCH_ROOTS_TABLE, &keys).unwrap();
        assert_eq!(removed, 3);
        assert_eq!(store.len(BATCH_ROOTS_TABLE).unwrap(), 2);

        cleanup(&path);
    }

    #[test]
    fn evict_old_transactions_caps_table() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        for i in 0..10u8 {
            let key = aztibase_core::hash(&[i]);
            store.put(TX_TABLE, &key, b"tx-data").unwrap();
        }
        assert_eq!(store.len(TX_TABLE).unwrap(), 10);

        let removed = store.evict_oldest(TX_TABLE, 5).unwrap();
        assert_eq!(removed, 5);
        assert_eq!(store.len(TX_TABLE).unwrap(), 5);

        cleanup(&path);
    }

    #[test]
    fn evict_old_batch_roots_caps_table() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        for i in 0..8u8 {
            store.put(BATCH_ROOTS_TABLE, &[i; 32], &[0xBB; 32]).unwrap();
        }

        let removed = store.evict_oldest(BATCH_ROOTS_TABLE, 3).unwrap();
        assert_eq!(removed, 5);
        assert_eq!(store.len(BATCH_ROOTS_TABLE).unwrap(), 3);

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

    #[test]
    fn batch_index_store_and_retrieve() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let anchor_0 = [0xAA; 32];
        let anchor_1 = [0xBB; 32];

        store_batch_index(&store, 0, &anchor_0).unwrap();
        store_batch_index(&store, 1, &anchor_1).unwrap();

        assert_eq!(get_batch_by_number(&store, 0).unwrap(), Some(anchor_0));
        assert_eq!(get_batch_by_number(&store, 1).unwrap(), Some(anchor_1));
        assert_eq!(get_batch_by_number(&store, 99).unwrap(), None);

        cleanup(&path);
    }

    #[test]
    fn batch_txs_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let anchor = [0xCC; 32];
        let tx_hashes = vec![[1u8; 32], [2u8; 32], [3u8; 32]];

        store_batch_txs(&store, &anchor, &tx_hashes).unwrap();
        let loaded = get_batch_txs(&store, &anchor).unwrap().unwrap();
        assert_eq!(loaded, tx_hashes);

        assert!(get_batch_txs(&store, &[0xFF; 32]).unwrap().is_none());

        cleanup(&path);
    }

    #[test]
    fn transaction_store_and_retrieve() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let tx_hash = [0xDD; 32];
        let tx_data = b"serialized-tx-data";

        store_transaction(&store, &tx_hash, tx_data).unwrap();
        let loaded = get_transaction(&store, &tx_hash).unwrap().unwrap();
        assert_eq!(loaded, tx_data);

        assert!(get_transaction(&store, &[0xEE; 32]).unwrap().is_none());

        cleanup(&path);
    }

    #[test]
    fn batch_range_query() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        for i in 0..10u64 {
            store_batch_index(&store, i, &[(i as u8); 32]).unwrap();
        }

        let range = get_batch_range(&store, 3, 7, 100).unwrap();
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].0, 3);
        assert_eq!(range[4].0, 7);

        let limited = get_batch_range(&store, 0, 9, 3).unwrap();
        assert_eq!(limited.len(), 3);

        let empty = get_batch_range(&store, 50, 60, 100).unwrap();
        assert!(empty.is_empty());

        cleanup(&path);
    }
}
