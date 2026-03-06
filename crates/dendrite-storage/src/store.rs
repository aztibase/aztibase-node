use std::path::Path;

use anyhow::Result;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use tracing::{debug, info};

// ── Table Definitions ──────────────────────────────────────────────
// Each table maps a byte-slice key to a byte-slice value.
// Higher-level types are serialized via bincode before storage.

/// DAG blocks keyed by block hash (32 bytes).
pub const BLOCKS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("blocks");

/// World state keyed by account/object address (32 bytes).
pub const STATE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("state");

/// Transactions keyed by transaction hash (32 bytes).
pub const TX_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("transactions");

/// Transaction receipts keyed by transaction hash (32 bytes).
pub const RECEIPTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("receipts");

/// Validator records keyed by validator ID (32 bytes).
pub const VALIDATORS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("validators");

/// Verkle tree nodes keyed by node hash (32 bytes).
pub const VERKLE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("verkle");

/// Account records keyed by address (32 bytes). Value: bincode(balance, nonce).
pub const ACCOUNTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("accounts");

/// Contract WASM bytecode keyed by contract address (32 bytes).
pub const CONTRACT_CODE_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("contract_code");

/// Contract storage keyed by address||key (32 + N bytes). Value: raw bytes.
pub const CONTRACT_STORAGE_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("contract_storage");

/// Committed batch state roots keyed by anchor hash (32 bytes).
pub const BATCH_ROOTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("batch_roots");

/// All table definitions for batch initialization.
const ALL_TABLES: [TableDefinition<&[u8], &[u8]>; 10] = [
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

// ── Error Type ─────────────────────────────────────────────────────

/// Storage-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] redb::DatabaseError),

    #[error("Table error: {0}")]
    Table(#[from] redb::TableError),

    #[error("Transaction error: {0}")]
    Transaction(Box<redb::TransactionError>),

    #[error("Commit error: {0}")]
    Commit(#[from] redb::CommitError),

    #[error("Storage error: {0}")]
    Storage(#[from] redb::StorageError),

    #[error("Key not found")]
    NotFound,
}

impl From<redb::TransactionError> for StorageError {
    fn from(e: redb::TransactionError) -> Self {
        StorageError::Transaction(Box::new(e))
    }
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

/// Entry for cross-table batch writes: (table, key, value).
/// Table lifetime is separate from key/value lifetime to allow static table
/// definitions with borrowed data.
pub type MultiTableEntry<'a> = (
    TableDefinition<'static, &'static [u8], &'static [u8]>,
    &'a [u8],
    &'a [u8],
);

// ── StateStore ─────────────────────────────────────────────────────

/// Embedded key-value store backed by redb.
///
/// Provides typed table access, batch writes, and range iteration.
/// All operations are ACID-compliant and crash-safe.
pub struct StateStore {
    db: Database,
}

impl StateStore {
    /// Open (or create) a database at the given path.
    /// Initializes all 6 tables on first open.
    pub fn open(path: &str) -> Result<Self> {
        let db = Database::create(Path::new(path))?;
        info!(path = %path, "Storage database opened");

        // Ensure all tables exist by opening them in a write transaction.
        let write_txn = db.begin_write()?;
        for table_def in &ALL_TABLES {
            write_txn.open_table(*table_def)?;
        }
        write_txn.commit()?;
        debug!("All 6 tables initialized");

        Ok(Self { db })
    }

    // ── Single-key operations ──────────────────────────────────────

    /// Get a value from a table by key. Returns `None` if not found.
    pub fn get(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        key: &[u8],
    ) -> StorageResult<Option<Vec<u8>>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        let value = tbl.get(key)?;
        Ok(value.map(|v| v.value().to_vec()))
    }

    /// Put a value into a table. Overwrites any existing value.
    pub fn put(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        key: &[u8],
        value: &[u8],
    ) -> StorageResult<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(table)?;
            tbl.insert(key, value)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Delete a key from a table. Returns the old value if it existed.
    pub fn delete(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        key: &[u8],
    ) -> StorageResult<Option<Vec<u8>>> {
        let write_txn = self.db.begin_write()?;
        let old = {
            let mut tbl = write_txn.open_table(table)?;
            tbl.remove(key)?.map(|v| v.value().to_vec())
        };
        write_txn.commit()?;
        Ok(old)
    }

    /// Check if a key exists in a table.
    pub fn contains(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        key: &[u8],
    ) -> StorageResult<bool> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        Ok(tbl.get(key)?.is_some())
    }

    // ── Batch operations ───────────────────────────────────────────

    /// Atomically write multiple key-value pairs to a single table.
    /// All writes succeed or all fail — no partial commits.
    pub fn batch_put(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        entries: &[(&[u8], &[u8])],
    ) -> StorageResult<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(table)?;
            for (key, value) in entries {
                tbl.insert(*key, *value)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Atomically write to multiple tables in a single transaction.
    /// Each entry is (table, key, value). All succeed or all fail.
    pub fn batch_put_multi(&self, entries: &[MultiTableEntry<'_>]) -> StorageResult<()> {
        let write_txn = self.db.begin_write()?;
        for (table_def, key, value) in entries {
            let mut tbl = write_txn.open_table(*table_def)?;
            tbl.insert(*key, *value)?;
        }
        write_txn.commit()?;
        Ok(())
    }

    // ── Range / iteration ──────────────────────────────────────────

    /// Iterate all entries in a table in forward key order.
    /// Returns Vec of (key, value) pairs.
    pub fn iter(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        let mut results = Vec::new();
        let iter = tbl.iter()?;
        for entry in iter {
            let (k, v) = entry?;
            results.push((k.value().to_vec(), v.value().to_vec()));
        }
        Ok(results)
    }

    /// Range query: return entries where `start <= key < end` in forward order.
    pub fn range(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        start: &[u8],
        end: &[u8],
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        let mut results = Vec::new();
        let range = tbl.range(start..end)?;
        for entry in range {
            let (k, v) = entry?;
            results.push((k.value().to_vec(), v.value().to_vec()));
        }
        Ok(results)
    }

    /// Range query in reverse order: return entries where `start <= key < end`
    /// from last to first.
    pub fn range_reverse(
        &self,
        table: TableDefinition<&[u8], &[u8]>,
        start: &[u8],
        end: &[u8],
    ) -> StorageResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        let mut results = Vec::new();
        let range = tbl.range(start..end)?;
        for entry in range.rev() {
            let (k, v) = entry?;
            results.push((k.value().to_vec(), v.value().to_vec()));
        }
        Ok(results)
    }

    /// Count entries in a table.
    pub fn len(&self, table: TableDefinition<&[u8], &[u8]>) -> StorageResult<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(table)?;
        Ok(tbl.len()?)
    }

    /// Check if a table is empty.
    pub fn is_empty(&self, table: TableDefinition<&[u8], &[u8]>) -> StorageResult<bool> {
        Ok(self.len(table)? == 0)
    }
}
