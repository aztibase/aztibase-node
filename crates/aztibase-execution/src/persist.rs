use aztibase_storage::{
    ACCOUNTS_TABLE, BATCH_INDEX_TABLE, BATCH_META_TABLE, BATCH_ROOTS_TABLE, BATCH_TXS_TABLE,
    CHECKPOINTS_TABLE, CONTRACT_CODE_TABLE, CONTRACT_STORAGE_TABLE, EQUIVOCATION_PROOFS_TABLE,
    STATE_TABLE, StateStore, StorageError, StorageResult, TX_TABLE, TableDef,
};

use crate::agent::AgentPolicyStore;
use crate::chain_params::ChainParams;
use crate::governance::GovernanceStore;
use crate::l2_bridge::{BridgeEscrow, BridgeWithdrawProofs, L2AnchorStore, L2Registry};
use crate::staking::StakingStore;
use crate::state::{AccountState, AccountType};
use crate::tokenomics::EmissionTracker;

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

/// Per-batch metadata stored alongside roots and tx lists.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BatchMeta {
    pub timestamp: u64,
    pub gas_used: u64,
}

pub fn store_batch_meta(
    store: &StateStore,
    anchor_hash: &[u8; 32],
    meta: &BatchMeta,
) -> StorageResult<()> {
    let Ok(data) = postcard::to_allocvec(meta) else {
        return Ok(());
    };
    store.put(BATCH_META_TABLE, anchor_hash, &data)
}

pub fn get_batch_meta(
    store: &StateStore,
    anchor_hash: &[u8; 32],
) -> StorageResult<Option<BatchMeta>> {
    match store.get(BATCH_META_TABLE, anchor_hash)? {
        Some(bytes) => match postcard::from_bytes::<BatchMeta>(&bytes) {
            Ok(meta) => Ok(Some(meta)),
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

/// Store a weak subjectivity checkpoint keyed by batch index (raw bytes).
pub fn store_checkpoint_raw(
    store: &StateStore,
    batch_index: u64,
    data: &[u8],
) -> StorageResult<()> {
    store.put(CHECKPOINTS_TABLE, &batch_index.to_be_bytes(), data)
}

/// Retrieve raw checkpoint bytes by batch index.
pub fn get_checkpoint_raw(store: &StateStore, batch_index: u64) -> StorageResult<Option<Vec<u8>>> {
    store.get(CHECKPOINTS_TABLE, &batch_index.to_be_bytes())
}

/// Retrieve the most recent checkpoint (highest batch index) as raw bytes.
pub fn latest_checkpoint_raw(store: &StateStore) -> StorageResult<Option<Vec<u8>>> {
    let entries = store.iter(CHECKPOINTS_TABLE)?;
    Ok(entries.last().map(|(_, v)| v.clone()))
}

/// Store an equivocation proof: both conflicting vertex hashes, round, and author.
/// Key: round(u64 BE) || author(32 bytes). Survives node restart.
pub fn store_equivocation_proof(
    store: &StateStore,
    round: u64,
    author: &[u8; 32],
    existing_hash: &[u8; 32],
    duplicate_hash: &[u8; 32],
) -> StorageResult<()> {
    let mut key = Vec::with_capacity(40);
    key.extend_from_slice(&round.to_be_bytes());
    key.extend_from_slice(author);
    let mut val = Vec::with_capacity(64);
    val.extend_from_slice(existing_hash);
    val.extend_from_slice(duplicate_hash);
    store.put(EQUIVOCATION_PROOFS_TABLE, &key, &val)
}

/// Retrieve an equivocation proof by round and author.
/// Returns Some((existing_hash, duplicate_hash)) if proof exists.
pub fn get_equivocation_proof(
    store: &StateStore,
    round: u64,
    author: &[u8; 32],
) -> StorageResult<Option<([u8; 32], [u8; 32])>> {
    let mut key = Vec::with_capacity(40);
    key.extend_from_slice(&round.to_be_bytes());
    key.extend_from_slice(author);
    match store.get(EQUIVOCATION_PROOFS_TABLE, &key)? {
        Some(val) if val.len() == 64 => {
            let existing: [u8; 32] = val[..32].try_into().unwrap();
            let duplicate: [u8; 32] = val[32..].try_into().unwrap();
            Ok(Some((existing, duplicate)))
        }
        _ => Ok(None),
    }
}

/// Retrieve the latest (highest) batch index stored in the database.
/// Returns None if no batches have been persisted.
pub fn latest_batch_index(store: &StateStore) -> StorageResult<Option<u64>> {
    let entries = store.range_reverse(BATCH_INDEX_TABLE, &[0u8; 8], &u64::MAX.to_be_bytes())?;
    match entries.first() {
        Some((k, _)) if k.len() == 8 => {
            Ok(Some(u64::from_be_bytes(k.as_slice().try_into().unwrap())))
        }
        _ => Ok(None),
    }
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

// ---------------------------------------------------------------------------
// Protocol state persistence (Sprint 053)
// ---------------------------------------------------------------------------

const STAKING_KEY: &[u8] = b"staking_store";
const GOVERNANCE_KEY: &[u8] = b"governance_store";
const EMISSION_KEY: &[u8] = b"emission_tracker";
const CHAIN_PARAMS_KEY: &[u8] = b"chain_params";
const AGENT_POLICIES_KEY: &[u8] = b"agent_policies";
const L2_REGISTRY_KEY: &[u8] = b"l2_registry";
const L2_ANCHORS_KEY: &[u8] = b"l2_anchors";
const BRIDGE_ESCROW_KEY: &[u8] = b"bridge_escrow";
const BRIDGE_PROOFS_KEY: &[u8] = b"bridge_proofs";
const SENTINEL_MEMORY_KEY: &[u8] = b"sentinel_memory";

fn flush_serializable<T: serde::Serialize>(
    store: &StateStore,
    key: &[u8],
    value: &T,
) -> StorageResult<()> {
    let data = postcard::to_allocvec(value).map_err(|_| StorageError::NotFound)?;
    store.put(STATE_TABLE, key, &data)
}

fn load_serializable<T: serde::de::DeserializeOwned + Default>(
    store: &StateStore,
    key: &[u8],
) -> StorageResult<T> {
    match store.get(STATE_TABLE, key)? {
        Some(data) => match postcard::from_bytes(&data) {
            Ok(val) => Ok(val),
            Err(e) => {
                tracing::error!(
                    key = %String::from_utf8_lossy(key),
                    error = %e,
                    len = data.len(),
                    "Corrupted protocol store data — refusing to silently replace with default"
                );
                Err(aztibase_storage::StorageError::Serialization(format!(
                    "corrupted data for key '{}': {e}",
                    String::from_utf8_lossy(key)
                )))
            }
        },
        None => Ok(T::default()),
    }
}

pub fn flush_staking(store: &StateStore, staking: &StakingStore) -> StorageResult<()> {
    flush_serializable(store, STAKING_KEY, staking)
}

pub fn load_staking(store: &StateStore) -> StorageResult<StakingStore> {
    load_serializable(store, STAKING_KEY)
}

pub fn flush_governance(store: &StateStore, governance: &GovernanceStore) -> StorageResult<()> {
    flush_serializable(store, GOVERNANCE_KEY, governance)
}

pub fn load_governance(store: &StateStore) -> StorageResult<GovernanceStore> {
    load_serializable(store, GOVERNANCE_KEY)
}

pub fn flush_emission(store: &StateStore, tracker: &EmissionTracker) -> StorageResult<()> {
    flush_serializable(store, EMISSION_KEY, tracker)
}

pub fn load_emission(store: &StateStore) -> StorageResult<EmissionTracker> {
    load_serializable(store, EMISSION_KEY)
}

pub fn flush_chain_params(store: &StateStore, params: &ChainParams) -> StorageResult<()> {
    flush_serializable(store, CHAIN_PARAMS_KEY, params)
}

pub fn load_chain_params(store: &StateStore) -> StorageResult<ChainParams> {
    load_serializable(store, CHAIN_PARAMS_KEY)
}

pub fn flush_agent_policies(store: &StateStore, policies: &AgentPolicyStore) -> StorageResult<()> {
    flush_serializable(store, AGENT_POLICIES_KEY, policies)
}

pub fn load_agent_policies(store: &StateStore) -> StorageResult<AgentPolicyStore> {
    load_serializable(store, AGENT_POLICIES_KEY)
}

pub fn flush_bridge_stores(
    store: &StateStore,
    registry: &L2Registry,
    anchors: &L2AnchorStore,
    escrow: &BridgeEscrow,
    proofs: &BridgeWithdrawProofs,
) -> StorageResult<()> {
    flush_serializable(store, L2_REGISTRY_KEY, registry)?;
    flush_serializable(store, L2_ANCHORS_KEY, anchors)?;
    flush_serializable(store, BRIDGE_ESCROW_KEY, escrow)?;
    flush_serializable(store, BRIDGE_PROOFS_KEY, proofs)
}

pub fn load_bridge_stores(
    store: &StateStore,
) -> StorageResult<(
    L2Registry,
    L2AnchorStore,
    BridgeEscrow,
    BridgeWithdrawProofs,
)> {
    Ok((
        load_serializable(store, L2_REGISTRY_KEY)?,
        load_serializable(store, L2_ANCHORS_KEY)?,
        load_serializable(store, BRIDGE_ESCROW_KEY)?,
        load_serializable(store, BRIDGE_PROOFS_KEY)?,
    ))
}

pub fn flush_sentinel_memory<T: serde::Serialize>(
    store: &StateStore,
    memory: &T,
) -> StorageResult<()> {
    flush_serializable(store, SENTINEL_MEMORY_KEY, memory)
}

pub fn load_sentinel_memory<T: serde::de::DeserializeOwned + Default>(
    store: &StateStore,
) -> StorageResult<T> {
    load_serializable(store, SENTINEL_MEMORY_KEY)
}

/// Flush all protocol stores in sequence.
#[allow(clippy::too_many_arguments)]
pub fn flush_protocol_stores(
    store: &StateStore,
    staking: &StakingStore,
    governance: &GovernanceStore,
    emission: &EmissionTracker,
    chain_params: &ChainParams,
    agent_policies: &AgentPolicyStore,
    l2_registry: &L2Registry,
    l2_anchors: &L2AnchorStore,
    bridge_escrow: &BridgeEscrow,
    bridge_proofs: &BridgeWithdrawProofs,
) -> StorageResult<()> {
    flush_staking(store, staking)?;
    flush_governance(store, governance)?;
    flush_emission(store, emission)?;
    flush_chain_params(store, chain_params)?;
    flush_agent_policies(store, agent_policies)?;
    flush_bridge_stores(store, l2_registry, l2_anchors, bridge_escrow, bridge_proofs)
}

// ── Node Sentinel (clean/dirty shutdown detection) ──────────────────

const SENTINEL_KEY: &[u8] = b"node_sentinel";
const SENTINEL_RUNNING: &[u8] = b"running";

pub fn write_sentinel(store: &StateStore) -> StorageResult<()> {
    store.put(STATE_TABLE, SENTINEL_KEY, SENTINEL_RUNNING)
}

pub fn clear_sentinel(store: &StateStore) -> StorageResult<()> {
    let _ = store.delete(STATE_TABLE, SENTINEL_KEY)?;
    Ok(())
}

pub fn check_sentinel(store: &StateStore) -> StorageResult<bool> {
    Ok(store.get(STATE_TABLE, SENTINEL_KEY)?.is_some())
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
    fn batch_meta_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let anchor = [0xEE; 32];
        let meta = BatchMeta {
            timestamp: 1711234567,
            gas_used: 500_000,
        };

        store_batch_meta(&store, &anchor, &meta).unwrap();
        let loaded = get_batch_meta(&store, &anchor).unwrap().unwrap();
        assert_eq!(loaded.timestamp, 1711234567);
        assert_eq!(loaded.gas_used, 500_000);

        assert!(get_batch_meta(&store, &[0xFF; 32]).unwrap().is_none());

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
    fn checkpoint_store_and_retrieve() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let data = b"checkpoint-data";
        store_checkpoint_raw(&store, 1000, data).unwrap();

        let loaded = get_checkpoint_raw(&store, 1000).unwrap().unwrap();
        assert_eq!(loaded, data);

        assert!(get_checkpoint_raw(&store, 999).unwrap().is_none());

        cleanup(&path);
    }

    #[test]
    fn checkpoint_latest() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert!(latest_checkpoint_raw(&store).unwrap().is_none());

        store_checkpoint_raw(&store, 1000, b"cp1").unwrap();
        store_checkpoint_raw(&store, 2000, b"cp2").unwrap();
        store_checkpoint_raw(&store, 3000, b"cp3").unwrap();

        let latest = latest_checkpoint_raw(&store).unwrap().unwrap();
        assert_eq!(latest, b"cp3");

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

    #[test]
    fn equivocation_proof_store_and_retrieve() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let author = [0xAA; 32];
        let existing = [0x11; 32];
        let duplicate = [0x22; 32];

        assert!(
            get_equivocation_proof(&store, 42, &author)
                .unwrap()
                .is_none()
        );

        store_equivocation_proof(&store, 42, &author, &existing, &duplicate).unwrap();

        let proof = get_equivocation_proof(&store, 42, &author)
            .unwrap()
            .unwrap();
        assert_eq!(proof.0, existing);
        assert_eq!(proof.1, duplicate);

        assert!(
            get_equivocation_proof(&store, 43, &author)
                .unwrap()
                .is_none()
        );

        cleanup(&path);
    }

    #[test]
    fn equivocation_proof_survives_multiple_rounds() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        for round in [10u64, 50, 99] {
            store_equivocation_proof(&store, round, &[0xBB; 32], &[round as u8; 32], &[0xFF; 32])
                .unwrap();
        }

        assert!(
            get_equivocation_proof(&store, 10, &[0xBB; 32])
                .unwrap()
                .is_some()
        );
        assert!(
            get_equivocation_proof(&store, 50, &[0xBB; 32])
                .unwrap()
                .is_some()
        );
        assert!(
            get_equivocation_proof(&store, 99, &[0xBB; 32])
                .unwrap()
                .is_some()
        );
        assert!(
            get_equivocation_proof(&store, 100, &[0xBB; 32])
                .unwrap()
                .is_none()
        );

        cleanup(&path);
    }

    #[test]
    fn latest_batch_index_empty_db() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert_eq!(latest_batch_index(&store).unwrap(), None);

        cleanup(&path);
    }

    #[test]
    fn latest_batch_index_returns_highest() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store_batch_index(&store, 0, &[0xAA; 32]).unwrap();
        store_batch_index(&store, 5, &[0xBB; 32]).unwrap();
        store_batch_index(&store, 3, &[0xCC; 32]).unwrap();

        assert_eq!(latest_batch_index(&store).unwrap(), Some(5));

        cleanup(&path);
    }

    #[test]
    fn crash_recovery_state_consistent() {
        let path = test_db_path();

        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let anchor_a = [0xA0; 32];
        let anchor_b = [0xB0; 32];

        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut state = AccountState::new();
            state.set_balance(&alice, 1000);
            state.set_balance(&bob, 500);
            state.increment_nonce(&alice);

            flush_state(&store, &state).unwrap();
            store_batch_root(&store, &anchor_a, &state.state_root()).unwrap();
            store_batch_index(&store, 0, &anchor_a).unwrap();
            store_base_fee(&store, 10).unwrap();

            state.set_balance(&alice, 800);
            state.set_balance(&bob, 700);
            state.increment_nonce(&alice);

            flush_state(&store, &state).unwrap();
            store_batch_root(&store, &anchor_b, &state.state_root()).unwrap();
            store_batch_index(&store, 1, &anchor_b).unwrap();
            store_base_fee(&store, 15).unwrap();
            // Drop simulates crash — redb commits are already durable.
        }

        // Reopen: verify recovery is consistent.
        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let loaded = load_state(&store).unwrap();
            assert_eq!(loaded.balance(&alice), 800);
            assert_eq!(loaded.balance(&bob), 700);
            assert_eq!(loaded.nonce(&alice), 2);

            let idx = latest_batch_index(&store).unwrap();
            assert_eq!(idx, Some(1));

            let fee = load_base_fee(&store).unwrap();
            assert_eq!(fee, Some(15));

            let root = get_batch_root(&store, &anchor_b).unwrap();
            assert!(root.is_some());
            assert_eq!(root.unwrap(), loaded.state_root());
        }

        cleanup(&path);
    }

    #[test]
    fn staking_store_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut staking = StakingStore::new();
        staking
            .register_validator([1u8; 32], 100_000, 50_000, 1_000_000, 42)
            .unwrap();
        staking
            .delegate([2u8; 32], [1u8; 32], 5_000, 1_000_000, 42)
            .unwrap();

        flush_staking(&store, &staking).unwrap();
        let loaded = load_staking(&store).unwrap();

        let v = loaded.get_validator(&[1u8; 32]).unwrap();
        assert_eq!(v.self_stake, 100_000);
        assert_eq!(v.total_delegated, 5_000);
        assert!(v.active);

        let d = loaded.get_delegation(&[2u8; 32]).unwrap();
        assert_eq!(d.amount, 5_000);

        cleanup(&path);
    }

    #[test]
    fn governance_store_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut gov = GovernanceStore::new();
        let mut snapshot = std::collections::HashMap::new();
        snapshot.insert([0xB0; 32], 1000u128);
        snapshot.insert([0xB1; 32], 500u128);
        gov.create_proposal(crate::governance::CreateProposalParams {
            id: [0x01; 32],
            proposer: [0xA0; 32],
            description: "Test proposal for persistence".into(),
            param_key: "base_fee_floor".into(),
            param_value: "5".into(),
            current_round: 100,
            voting_period: 50,
            snapshot_balances: snapshot,
        })
        .unwrap();
        gov.cast_vote([0xB0; 32], [0x01; 32], true, 1000).unwrap();

        flush_governance(&store, &gov).unwrap();
        let loaded = load_governance(&store).unwrap();

        let p = loaded.get(&[0x01; 32]).unwrap();
        assert_eq!(p.description, "Test proposal for persistence");
        let tally = loaded.tally(&[0x01; 32]).unwrap();
        assert_eq!(tally.approve_weight, 1000);
        assert_eq!(tally.voter_count, 1);

        cleanup(&path);
    }

    #[test]
    fn emission_tracker_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut tracker = EmissionTracker::new(1000);
        tracker.advance_epoch();
        tracker.advance_epoch();
        let epoch_before = tracker.current_epoch;
        let emitted_before = tracker.total_emitted;

        flush_emission(&store, &tracker).unwrap();
        let loaded = load_emission(&store).unwrap();
        assert_eq!(loaded.current_epoch, epoch_before);
        assert_eq!(loaded.total_emitted, emitted_before);
        assert_eq!(loaded.epoch_length, 1000);

        cleanup(&path);
    }

    #[test]
    fn chain_params_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut params = ChainParams::defaults();
        params
            .set("base_fee_floor", crate::chain_params::ParamValue::U64(42))
            .unwrap();

        flush_chain_params(&store, &params).unwrap();
        let loaded = load_chain_params(&store).unwrap();
        assert_eq!(loaded.get_u64("base_fee_floor"), Some(42));

        cleanup(&path);
    }

    #[test]
    fn agent_policies_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut policies = AgentPolicyStore::new();
        policies.set_policy(
            [1u8; 32],
            crate::agent::AgentPolicy {
                owner: [2u8; 32],
                per_tx_limit: 1000,
                per_epoch_limit: 5000,
                allowed_tx_kinds: vec![0x01],
                expiry_epoch: 100,
            },
        );

        flush_agent_policies(&store, &policies).unwrap();
        let loaded = load_agent_policies(&store).unwrap();

        let p = loaded.get_policy(&[1u8; 32]).unwrap();
        assert_eq!(p.per_tx_limit, 1000);
        assert_eq!(p.owner, [2u8; 32]);

        cleanup(&path);
    }

    #[test]
    fn bridge_stores_roundtrip() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let mut registry = L2Registry::new();
        let chain_id = [0xAA; 32];
        registry
            .register(crate::l2_bridge::L2Registration {
                owner: [1u8; 32],
                l2_chain_id: chain_id,
                name: "Test L2".into(),
                sequencer_set: vec![[2u8; 32]],
                bridge_address: [3u8; 32],
            })
            .unwrap();

        let mut anchors = L2AnchorStore::new();
        anchors
            .anchor(
                &chain_id,
                crate::l2_bridge::L2Anchor {
                    sequencer: [2u8; 32],
                    state_root: [0xBB; 32],
                    batch_data_hash: [0xCC; 32],
                    l2_block_start: 0,
                    l2_block_end: 10,
                    l1_batch_index: 50,
                },
            )
            .unwrap();

        let mut escrow = BridgeEscrow::new();
        escrow.lock(&chain_id, &[5u8; 32], 1000);

        let mut proofs = BridgeWithdrawProofs::new();
        proofs.mark_used([0xFF; 32], [6u8; 32]).unwrap();

        flush_bridge_stores(&store, &registry, &anchors, &escrow, &proofs).unwrap();
        let (lr, la, le, lp) = load_bridge_stores(&store).unwrap();

        assert_eq!(lr.get(&chain_id).unwrap().name, "Test L2");
        assert_eq!(la.latest(&chain_id).unwrap().state_root, [0xBB; 32]);
        assert_eq!(le.balance(&chain_id, &[5u8; 32]), 1000);
        assert!(lp.is_used(&[0xFF; 32]));

        cleanup(&path);
    }

    #[test]
    fn empty_db_returns_defaults() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let staking = load_staking(&store).unwrap();
        assert_eq!(staking.validator_count(), 0);

        let gov = load_governance(&store).unwrap();
        assert!(gov.list(None).is_empty());

        let emission = load_emission(&store).unwrap();
        assert_eq!(emission.current_epoch, 0);

        let params = load_chain_params(&store).unwrap();
        assert_eq!(params.get_u64("base_fee_floor"), Some(1));

        let policies = load_agent_policies(&store).unwrap();
        assert_eq!(policies.policy_count(), 0);

        let (reg, anchors, escrow, proofs) = load_bridge_stores(&store).unwrap();
        assert!(reg.list().is_empty());
        assert!(anchors.latest(&[0u8; 32]).is_none());
        assert_eq!(escrow.balance(&[0u8; 32], &[0u8; 32]), 0);
        assert!(!proofs.is_used(&[0u8; 32]));

        cleanup(&path);
    }

    #[test]
    fn protocol_stores_crash_recovery() {
        let path = test_db_path();
        let chain_id = [0xAA; 32];

        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let mut staking = StakingStore::new();
            staking
                .register_validator([1u8; 32], 100_000, 50_000, 1_000_000, 0)
                .unwrap();
            let mut gov = GovernanceStore::new();
            let mut snap = std::collections::HashMap::new();
            snap.insert([0xB0; 32], 1000u128);
            snap.insert([0xB1; 32], 500u128);
            gov.create_proposal(crate::governance::CreateProposalParams {
                id: [0x99; 32],
                proposer: [0xA0; 32],
                description: "Crash test proposal for recovery".into(),
                param_key: "epoch_length".into(),
                param_value: "5000".into(),
                current_round: 0,
                voting_period: 100,
                snapshot_balances: snap,
            })
            .unwrap();
            let mut escrow = BridgeEscrow::new();
            escrow.lock(&chain_id, &[5u8; 32], 42_000);

            flush_staking(&store, &staking).unwrap();
            flush_governance(&store, &gov).unwrap();
            flush_bridge_stores(
                &store,
                &L2Registry::new(),
                &L2AnchorStore::new(),
                &escrow,
                &BridgeWithdrawProofs::new(),
            )
            .unwrap();
        }

        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            let staking = load_staking(&store).unwrap();
            assert_eq!(
                staking.get_validator(&[1u8; 32]).unwrap().self_stake,
                100_000
            );

            let gov = load_governance(&store).unwrap();
            assert!(gov.get(&[0x99; 32]).is_some());

            let (_, _, escrow, _) = load_bridge_stores(&store).unwrap();
            assert_eq!(escrow.balance(&chain_id, &[5u8; 32]), 42_000);
        }

        cleanup(&path);
    }

    #[test]
    fn sentinel_write_and_check() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert!(!check_sentinel(&store).unwrap());
        write_sentinel(&store).unwrap();
        assert!(check_sentinel(&store).unwrap());

        cleanup(&path);
    }

    #[test]
    fn sentinel_clear_resets() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        write_sentinel(&store).unwrap();
        assert!(check_sentinel(&store).unwrap());

        clear_sentinel(&store).unwrap();
        assert!(!check_sentinel(&store).unwrap());

        cleanup(&path);
    }

    #[test]
    fn sentinel_dirty_start_detection() {
        let path = test_db_path();

        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            write_sentinel(&store).unwrap();
            // simulate crash: drop without clearing
        }

        {
            let store = StateStore::open(path.to_str().unwrap()).unwrap();
            assert!(
                check_sentinel(&store).unwrap(),
                "dirty shutdown not detected"
            );
            clear_sentinel(&store).unwrap();
        }

        cleanup(&path);
    }
}
