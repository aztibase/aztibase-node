use std::path::Path;

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};

use crate::block_sync::SyncBatch;

const BATCH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("batch_archive");

const DEFAULT_MAX_BATCHES: u64 = 10_000;

pub struct BatchArchive {
    db: Database,
    max_batches: u64,
}

impl BatchArchive {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        Self::open_with_cap(path, DEFAULT_MAX_BATCHES)
    }

    pub fn open_with_cap(path: &Path, max_batches: u64) -> anyhow::Result<Self> {
        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        write_txn.open_table(BATCH_TABLE)?;
        write_txn.commit()?;
        Ok(Self { db, max_batches })
    }

    pub fn store_batch(&self, batch: &SyncBatch) -> anyhow::Result<()> {
        let encoded = postcard::to_allocvec(batch)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(BATCH_TABLE)?;
            tbl.insert(batch.index, encoded.as_slice())?;
        }
        write_txn.commit()?;
        self.prune_excess()?;
        Ok(())
    }

    pub fn store_batches(&self, batches: &[SyncBatch]) -> anyhow::Result<()> {
        if batches.is_empty() {
            return Ok(());
        }
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(BATCH_TABLE)?;
            for batch in batches {
                let encoded = postcard::to_allocvec(batch)?;
                tbl.insert(batch.index, encoded.as_slice())?;
            }
        }
        write_txn.commit()?;
        self.prune_excess()?;
        Ok(())
    }

    pub fn get_batch(&self, index: u64) -> anyhow::Result<Option<SyncBatch>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        match tbl.get(index)? {
            Some(v) => {
                let batch: SyncBatch = postcard::from_bytes(v.value())?;
                Ok(Some(batch))
            }
            None => Ok(None),
        }
    }

    pub fn get_range(&self, from: u64, count: u64) -> anyhow::Result<Vec<SyncBatch>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        let end = from.saturating_add(count);
        let mut result = Vec::new();
        for entry in tbl.range(from..end)? {
            let (_, v) = entry?;
            let batch: SyncBatch = postcard::from_bytes(v.value())?;
            result.push(batch);
        }
        Ok(result)
    }

    pub fn latest_index(&self) -> anyhow::Result<Option<u64>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        match tbl.last()? {
            Some((k, _)) => Ok(Some(k.value())),
            None => Ok(None),
        }
    }

    pub fn count(&self) -> anyhow::Result<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        Ok(tbl.len()?)
    }

    pub fn load_recent(&self, max: usize) -> anyhow::Result<Vec<SyncBatch>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        let total = tbl.len()? as usize;
        let skip = total.saturating_sub(max);
        let mut result = Vec::with_capacity(max.min(total));
        for (i, entry) in tbl.iter()?.enumerate() {
            if i < skip {
                continue;
            }
            let (_, v) = entry?;
            let batch: SyncBatch = postcard::from_bytes(v.value())?;
            result.push(batch);
        }
        Ok(result)
    }

    fn prune_excess(&self) -> anyhow::Result<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(BATCH_TABLE)?;
        let total = tbl.len()?;
        if total <= self.max_batches {
            return Ok(0);
        }
        let to_remove = total - self.max_batches;
        let mut keys: Vec<u64> = Vec::with_capacity(to_remove as usize);
        for entry in tbl.iter()? {
            if keys.len() as u64 >= to_remove {
                break;
            }
            let (k, _) = entry?;
            keys.push(k.value());
        }
        drop(tbl);
        drop(read_txn);

        let removed = keys.len() as u64;
        if !keys.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(BATCH_TABLE)?;
                for key in &keys {
                    tbl.remove(*key)?;
                }
            }
            write_txn.commit()?;
        }
        Ok(removed)
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
        std::env::temp_dir().join(format!("aztibase_batch_archive_test_{}_{}.redb", pid, id))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn make_batch(index: u64) -> SyncBatch {
        SyncBatch {
            index,
            anchor_hash: [index as u8; 32],
            state_root: [(index + 1) as u8; 32],
            transactions: vec![vec![0xAA, 0xBB]],
        }
    }

    #[test]
    fn open_creates_db() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path);
        assert!(archive.is_ok());
        assert_eq!(archive.unwrap().count().unwrap(), 0);
        cleanup(&path);
    }

    #[test]
    fn store_and_retrieve() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();

        let batch = make_batch(42);
        archive.store_batch(&batch).unwrap();

        let loaded = archive.get_batch(42).unwrap().unwrap();
        assert_eq!(loaded.index, 42);
        assert_eq!(loaded.anchor_hash, [42u8; 32]);
        assert_eq!(loaded.transactions.len(), 1);
        assert_eq!(archive.count().unwrap(), 1);
        assert_eq!(archive.latest_index().unwrap(), Some(42));

        cleanup(&path);
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();
        assert!(archive.get_batch(999).unwrap().is_none());
        cleanup(&path);
    }

    #[test]
    fn store_batches_bulk() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();

        let batches: Vec<SyncBatch> = (1..=50).map(make_batch).collect();
        archive.store_batches(&batches).unwrap();

        assert_eq!(archive.count().unwrap(), 50);
        assert_eq!(archive.latest_index().unwrap(), Some(50));
        assert_eq!(archive.get_batch(25).unwrap().unwrap().index, 25);

        cleanup(&path);
    }

    #[test]
    fn get_range_returns_subset() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();

        let batches: Vec<SyncBatch> = (1..=100).map(make_batch).collect();
        archive.store_batches(&batches).unwrap();

        let range = archive.get_range(10, 5).unwrap();
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].index, 10);
        assert_eq!(range[4].index, 14);

        cleanup(&path);
    }

    #[test]
    fn prune_excess_removes_oldest() {
        let path = test_db_path();
        let archive = BatchArchive::open_with_cap(&path, 5).unwrap();

        let batches: Vec<SyncBatch> = (1..=10).map(make_batch).collect();
        archive.store_batches(&batches).unwrap();

        assert_eq!(archive.count().unwrap(), 5);
        assert!(archive.get_batch(1).unwrap().is_none());
        assert!(archive.get_batch(5).unwrap().is_none());
        assert!(archive.get_batch(6).unwrap().is_some());
        assert!(archive.get_batch(10).unwrap().is_some());

        cleanup(&path);
    }

    #[test]
    fn load_recent_returns_tail() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();

        let batches: Vec<SyncBatch> = (1..=20).map(make_batch).collect();
        archive.store_batches(&batches).unwrap();

        let recent = archive.load_recent(5).unwrap();
        assert_eq!(recent.len(), 5);
        assert_eq!(recent[0].index, 16);
        assert_eq!(recent[4].index, 20);

        cleanup(&path);
    }

    #[test]
    fn persists_across_reopen() {
        let path = test_db_path();

        {
            let archive = BatchArchive::open(&path).unwrap();
            let batches: Vec<SyncBatch> = (1..=10).map(make_batch).collect();
            archive.store_batches(&batches).unwrap();
        }

        {
            let archive = BatchArchive::open(&path).unwrap();
            assert_eq!(archive.count().unwrap(), 10);
            assert_eq!(archive.get_batch(5).unwrap().unwrap().index, 5);
            assert_eq!(archive.latest_index().unwrap(), Some(10));
        }

        cleanup(&path);
    }

    #[test]
    fn empty_store_batches_is_noop() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();
        archive.store_batches(&[]).unwrap();
        assert_eq!(archive.count().unwrap(), 0);
        cleanup(&path);
    }

    #[test]
    fn load_recent_when_fewer_than_requested() {
        let path = test_db_path();
        let archive = BatchArchive::open(&path).unwrap();

        let batches: Vec<SyncBatch> = (1..=3).map(make_batch).collect();
        archive.store_batches(&batches).unwrap();

        let recent = archive.load_recent(100).unwrap();
        assert_eq!(recent.len(), 3);

        cleanup(&path);
    }
}
