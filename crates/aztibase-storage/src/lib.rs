pub mod light;
pub mod store;
pub mod verkle;

pub use light::{CachedProof, LightFinalityCert, LightHeader, LightStore, LocalWalletState};
pub use redb::TableDefinition;
pub use store::{
    ACCOUNTS_TABLE, BATCH_INDEX_TABLE, BATCH_ROOTS_TABLE, BATCH_TXS_TABLE, BLOCKS_TABLE,
    CHECKPOINTS_TABLE, CONTRACT_CODE_TABLE, CONTRACT_STORAGE_TABLE, RECEIPTS_TABLE, STATE_TABLE,
    StateStore, StorageError, StorageResult, TX_TABLE, VALIDATORS_TABLE, VERKLE_TABLE,
};

pub type TableDef = TableDefinition<'static, &'static [u8], &'static [u8]>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Unique temp dir per test to avoid redb lock conflicts.
    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);
    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        // redb may create a .lock file
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    // ── Task 1: Basic CRUD ─────────────────────────────────────────

    #[test]
    fn test_open_creates_database() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap());
        assert!(store.is_ok());
        cleanup(&path);
    }

    #[test]
    fn test_put_and_get() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"alice", b"100").unwrap();
        let val = store.get(STATE_TABLE, b"alice").unwrap();
        assert_eq!(val, Some(b"100".to_vec()));

        cleanup(&path);
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let val = store.get(STATE_TABLE, b"nobody").unwrap();
        assert_eq!(val, None);

        cleanup(&path);
    }

    #[test]
    fn test_put_overwrites() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"alice", b"100").unwrap();
        store.put(STATE_TABLE, b"alice", b"200").unwrap();
        let val = store.get(STATE_TABLE, b"alice").unwrap();
        assert_eq!(val, Some(b"200".to_vec()));

        cleanup(&path);
    }

    #[test]
    fn test_delete() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"alice", b"100").unwrap();
        let old = store.delete(STATE_TABLE, b"alice").unwrap();
        assert_eq!(old, Some(b"100".to_vec()));

        let val = store.get(STATE_TABLE, b"alice").unwrap();
        assert_eq!(val, None);

        cleanup(&path);
    }

    #[test]
    fn test_delete_nonexistent() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let old = store.delete(STATE_TABLE, b"nobody").unwrap();
        assert_eq!(old, None);

        cleanup(&path);
    }

    #[test]
    fn test_contains() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert!(!store.contains(STATE_TABLE, b"alice").unwrap());
        store.put(STATE_TABLE, b"alice", b"100").unwrap();
        assert!(store.contains(STATE_TABLE, b"alice").unwrap());

        cleanup(&path);
    }

    // ── Task 2: Named tables ───────────────────────────────────────

    #[test]
    fn test_tables_are_isolated() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let key = b"same_key";
        store.put(BLOCKS_TABLE, key, b"block_data").unwrap();
        store.put(STATE_TABLE, key, b"state_data").unwrap();
        store.put(TX_TABLE, key, b"tx_data").unwrap();

        assert_eq!(
            store.get(BLOCKS_TABLE, key).unwrap(),
            Some(b"block_data".to_vec())
        );
        assert_eq!(
            store.get(STATE_TABLE, key).unwrap(),
            Some(b"state_data".to_vec())
        );
        assert_eq!(store.get(TX_TABLE, key).unwrap(), Some(b"tx_data".to_vec()));
        // Other tables should not have this key.
        assert_eq!(store.get(RECEIPTS_TABLE, key).unwrap(), None);
        assert_eq!(store.get(VALIDATORS_TABLE, key).unwrap(), None);
        assert_eq!(store.get(VERKLE_TABLE, key).unwrap(), None);

        cleanup(&path);
    }

    #[test]
    fn test_all_six_tables_writable() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let tables = [
            BLOCKS_TABLE,
            STATE_TABLE,
            TX_TABLE,
            RECEIPTS_TABLE,
            VALIDATORS_TABLE,
            VERKLE_TABLE,
            ACCOUNTS_TABLE,
            CONTRACT_CODE_TABLE,
            CONTRACT_STORAGE_TABLE,
            BATCH_ROOTS_TABLE,
        ];
        for (i, table) in tables.iter().enumerate() {
            let key = format!("key_{}", i);
            let val = format!("val_{}", i);
            store.put(*table, key.as_bytes(), val.as_bytes()).unwrap();
            let got = store.get(*table, key.as_bytes()).unwrap();
            assert_eq!(got, Some(val.into_bytes()));
        }

        cleanup(&path);
    }

    // ── Task 3: Batch operations ───────────────────────────────────

    #[test]
    fn test_batch_put_single_table() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let entries: Vec<(&[u8], &[u8])> = vec![(b"a", b"1"), (b"b", b"2"), (b"c", b"3")];
        store.batch_put(STATE_TABLE, &entries).unwrap();

        assert_eq!(store.get(STATE_TABLE, b"a").unwrap(), Some(b"1".to_vec()));
        assert_eq!(store.get(STATE_TABLE, b"b").unwrap(), Some(b"2".to_vec()));
        assert_eq!(store.get(STATE_TABLE, b"c").unwrap(), Some(b"3".to_vec()));

        cleanup(&path);
    }

    #[test]
    fn test_batch_put_multi_table() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        let entries: Vec<(redb::TableDefinition<&[u8], &[u8]>, &[u8], &[u8])> = vec![
            (BLOCKS_TABLE, b"block1", b"data1"),
            (TX_TABLE, b"tx1", b"data2"),
            (VALIDATORS_TABLE, b"val1", b"data3"),
        ];
        store.batch_put_multi(&entries).unwrap();

        assert_eq!(
            store.get(BLOCKS_TABLE, b"block1").unwrap(),
            Some(b"data1".to_vec())
        );
        assert_eq!(
            store.get(TX_TABLE, b"tx1").unwrap(),
            Some(b"data2".to_vec())
        );
        assert_eq!(
            store.get(VALIDATORS_TABLE, b"val1").unwrap(),
            Some(b"data3".to_vec())
        );

        cleanup(&path);
    }

    // ── Task 4: Range iteration ────────────────────────────────────

    #[test]
    fn test_iter_all() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"a", b"1").unwrap();
        store.put(STATE_TABLE, b"b", b"2").unwrap();
        store.put(STATE_TABLE, b"c", b"3").unwrap();

        let all = store.iter(STATE_TABLE).unwrap();
        assert_eq!(all.len(), 3);
        // redb sorts by bytes, so a < b < c
        assert_eq!(all[0].0, b"a".to_vec());
        assert_eq!(all[1].0, b"b".to_vec());
        assert_eq!(all[2].0, b"c".to_vec());

        cleanup(&path);
    }

    #[test]
    fn test_range_forward() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"a", b"1").unwrap();
        store.put(STATE_TABLE, b"b", b"2").unwrap();
        store.put(STATE_TABLE, b"c", b"3").unwrap();
        store.put(STATE_TABLE, b"d", b"4").unwrap();

        // Range b..d should return b and c (exclusive end)
        let range = store.range(STATE_TABLE, b"b", b"d").unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].0, b"b".to_vec());
        assert_eq!(range[1].0, b"c".to_vec());

        cleanup(&path);
    }

    #[test]
    fn test_range_reverse() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        store.put(STATE_TABLE, b"a", b"1").unwrap();
        store.put(STATE_TABLE, b"b", b"2").unwrap();
        store.put(STATE_TABLE, b"c", b"3").unwrap();
        store.put(STATE_TABLE, b"d", b"4").unwrap();

        let range = store.range_reverse(STATE_TABLE, b"b", b"d").unwrap();
        assert_eq!(range.len(), 2);
        // Reverse: c first, then b
        assert_eq!(range[0].0, b"c".to_vec());
        assert_eq!(range[1].0, b"b".to_vec());

        cleanup(&path);
    }

    #[test]
    fn test_len_and_is_empty() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();

        assert!(store.is_empty(STATE_TABLE).unwrap());
        assert_eq!(store.len(STATE_TABLE).unwrap(), 0);

        store.put(STATE_TABLE, b"a", b"1").unwrap();
        store.put(STATE_TABLE, b"b", b"2").unwrap();

        assert!(!store.is_empty(STATE_TABLE).unwrap());
        assert_eq!(store.len(STATE_TABLE).unwrap(), 2);

        cleanup(&path);
    }
}
