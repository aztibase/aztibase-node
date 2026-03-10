use std::path::Path;

use anyhow::Result;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::StorageResult;

pub const HEADERS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("lc_headers");
pub const FINALITY_CERTS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("lc_finality");
pub const PROOF_CACHE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("lc_proofs");
pub const WALLET_STATE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("lc_wallet");
pub const PEER_CACHE_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("lc_peers");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightHeader {
    pub round: u64,
    pub author: [u8; 32],
    pub parents: Vec<[u8; 32]>,
    pub state_root: [u8; 32],
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LightFinalityCert {
    pub anchor_round: u64,
    pub batch_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub aggregate_signature: Vec<u8>,
    pub signer_bitmap: Vec<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedProof {
    pub state_key: Vec<u8>,
    pub proof_data: Vec<u8>,
    pub at_round: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocalWalletState {
    pub balance: u128,
    pub nonce: u64,
    pub pending_tx_hashes: Vec<[u8; 32]>,
}

pub struct LightStore {
    db: Database,
}

impl LightStore {
    pub fn open(path: &Path) -> Result<Self> {
        let db = Database::create(path)?;
        info!(path = %path.display(), "Light client database opened");

        let write_txn = db.begin_write()?;
        write_txn.open_table(HEADERS_TABLE)?;
        write_txn.open_table(FINALITY_CERTS_TABLE)?;
        write_txn.open_table(PROOF_CACHE_TABLE)?;
        write_txn.open_table(WALLET_STATE_TABLE)?;
        write_txn.open_table(PEER_CACHE_TABLE)?;
        write_txn.commit()?;

        Ok(Self { db })
    }

    // ── Headers ────────────────────────────────────────────────────

    pub fn store_header(&self, header: &LightHeader) -> StorageResult<()> {
        let encoded = postcard::to_allocvec(header).expect("header serialization");
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(HEADERS_TABLE)?;
            tbl.insert(header.round, encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn store_headers_batch(&self, headers: &[LightHeader]) -> StorageResult<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(HEADERS_TABLE)?;
            for h in headers {
                let encoded = postcard::to_allocvec(h).expect("header serialization");
                tbl.insert(h.round, encoded.as_slice())?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_header(&self, round: u64) -> StorageResult<Option<LightHeader>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(HEADERS_TABLE)?;
        match tbl.get(round)? {
            Some(v) => {
                let header: LightHeader = postcard::from_bytes(v.value()).expect("header deser");
                Ok(Some(header))
            }
            None => Ok(None),
        }
    }

    pub fn latest_header_round(&self) -> StorageResult<Option<u64>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(HEADERS_TABLE)?;
        match tbl.last()? {
            Some((k, _)) => Ok(Some(k.value())),
            None => Ok(None),
        }
    }

    pub fn header_count(&self) -> StorageResult<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(HEADERS_TABLE)?;
        Ok(tbl.len()?)
    }

    // ── Finality Certificates ──────────────────────────────────────

    pub fn store_finality_cert(&self, cert: &LightFinalityCert) -> StorageResult<()> {
        let encoded = postcard::to_allocvec(cert).expect("cert serialization");
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(FINALITY_CERTS_TABLE)?;
            tbl.insert(cert.anchor_round, encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_finality_cert(&self, anchor_round: u64) -> StorageResult<Option<LightFinalityCert>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(FINALITY_CERTS_TABLE)?;
        match tbl.get(anchor_round)? {
            Some(v) => {
                let cert: LightFinalityCert = postcard::from_bytes(v.value()).expect("cert deser");
                Ok(Some(cert))
            }
            None => Ok(None),
        }
    }

    pub fn latest_finality_round(&self) -> StorageResult<Option<u64>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(FINALITY_CERTS_TABLE)?;
        match tbl.last()? {
            Some((k, _)) => Ok(Some(k.value())),
            None => Ok(None),
        }
    }

    // ── Proof Cache ────────────────────────────────────────────────

    pub fn cache_proof(&self, proof: &CachedProof) -> StorageResult<()> {
        let encoded = postcard::to_allocvec(proof).expect("proof serialization");
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(PROOF_CACHE_TABLE)?;
            tbl.insert(proof.state_key.as_slice(), encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_cached_proof(&self, state_key: &[u8]) -> StorageResult<Option<CachedProof>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PROOF_CACHE_TABLE)?;
        match tbl.get(state_key)? {
            Some(v) => {
                let proof: CachedProof = postcard::from_bytes(v.value()).expect("proof deser");
                Ok(Some(proof))
            }
            None => Ok(None),
        }
    }

    pub fn evict_stale_proofs(&self, current_round: u64, max_age: u64) -> StorageResult<u64> {
        let cutoff = current_round.saturating_sub(max_age);
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PROOF_CACHE_TABLE)?;

        let mut stale_keys = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let proof: CachedProof = postcard::from_bytes(v.value()).expect("proof deser");
            if proof.at_round < cutoff {
                stale_keys.push(k.value().to_vec());
            }
        }
        drop(tbl);
        drop(read_txn);

        let count = stale_keys.len() as u64;
        if !stale_keys.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PROOF_CACHE_TABLE)?;
                for key in &stale_keys {
                    tbl.remove(key.as_slice())?;
                }
            }
            write_txn.commit()?;
        }
        Ok(count)
    }

    /// Count cached proofs, then evict the oldest entries if the count
    /// exceeds `MAX_PROOF_CACHE_ENTRIES`. Returns the number evicted.
    pub fn evict_excess_proofs(&self) -> StorageResult<u64> {
        const MAX_PROOF_CACHE_ENTRIES: u64 = 4096;
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PROOF_CACHE_TABLE)?;
        let total = tbl.len()?;
        if total <= MAX_PROOF_CACHE_ENTRIES {
            return Ok(0);
        }
        let to_remove = total - MAX_PROOF_CACHE_ENTRIES;

        // Collect oldest entries by lowest at_round
        let mut entries: Vec<(Vec<u8>, u64)> = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let proof: CachedProof = postcard::from_bytes(v.value()).expect("proof deser");
            entries.push((k.value().to_vec(), proof.at_round));
        }
        drop(tbl);
        drop(read_txn);

        entries.sort_by_key(|(_, round)| *round);
        let keys_to_remove: Vec<Vec<u8>> = entries
            .into_iter()
            .take(to_remove as usize)
            .map(|(k, _)| k)
            .collect();

        let removed = keys_to_remove.len() as u64;
        if !keys_to_remove.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PROOF_CACHE_TABLE)?;
                for key in &keys_to_remove {
                    tbl.remove(key.as_slice())?;
                }
            }
            write_txn.commit()?;
        }
        Ok(removed)
    }

    // ── Wallet State ───────────────────────────────────────────────

    pub fn store_wallet_state(
        &self,
        address: &[u8; 32],
        state: &LocalWalletState,
    ) -> StorageResult<()> {
        let encoded = postcard::to_allocvec(state).expect("wallet state serialization");
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(WALLET_STATE_TABLE)?;
            tbl.insert(address.as_slice(), encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get_wallet_state(&self, address: &[u8; 32]) -> StorageResult<Option<LocalWalletState>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(WALLET_STATE_TABLE)?;
        match tbl.get(address.as_slice())? {
            Some(v) => {
                let state: LocalWalletState =
                    postcard::from_bytes(v.value()).expect("wallet deser");
                Ok(Some(state))
            }
            None => Ok(None),
        }
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
        std::env::temp_dir().join(format!("aztibase_light_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn sample_header(round: u64) -> LightHeader {
        LightHeader {
            round,
            author: [round as u8; 32],
            parents: vec![[0u8; 32]],
            state_root: [round as u8; 32],
            timestamp: 1000 + round,
        }
    }

    fn sample_cert(anchor_round: u64) -> LightFinalityCert {
        LightFinalityCert {
            anchor_round,
            batch_hash: [anchor_round as u8; 32],
            state_root: [anchor_round as u8; 32],
            aggregate_signature: vec![0u8; 96],
            signer_bitmap: vec![true, true, true, false],
        }
    }

    #[test]
    fn open_creates_light_db() {
        let path = test_db_path();
        let store = LightStore::open(&path);
        assert!(store.is_ok());
        cleanup(&path);
    }

    #[test]
    fn header_crud() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        assert!(store.get_header(1).unwrap().is_none());
        assert_eq!(store.header_count().unwrap(), 0);

        let h = sample_header(1);
        store.store_header(&h).unwrap();

        let loaded = store.get_header(1).unwrap().unwrap();
        assert_eq!(loaded.round, 1);
        assert_eq!(loaded.timestamp, 1001);
        assert_eq!(store.header_count().unwrap(), 1);

        cleanup(&path);
    }

    #[test]
    fn header_batch_insert() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        let headers: Vec<_> = (1..=10).map(sample_header).collect();
        store.store_headers_batch(&headers).unwrap();

        assert_eq!(store.header_count().unwrap(), 10);
        assert_eq!(store.latest_header_round().unwrap(), Some(10));
        assert_eq!(store.get_header(5).unwrap().unwrap().round, 5);

        cleanup(&path);
    }

    #[test]
    fn finality_cert_crud() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        assert!(store.get_finality_cert(1).unwrap().is_none());

        let cert = sample_cert(5);
        store.store_finality_cert(&cert).unwrap();

        let loaded = store.get_finality_cert(5).unwrap().unwrap();
        assert_eq!(loaded.anchor_round, 5);
        assert_eq!(loaded.signer_bitmap, vec![true, true, true, false]);
        assert_eq!(store.latest_finality_round().unwrap(), Some(5));

        cleanup(&path);
    }

    #[test]
    fn proof_cache_and_eviction() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        let old_proof = CachedProof {
            state_key: b"account_alice".to_vec(),
            proof_data: vec![1, 2, 3],
            at_round: 100,
        };
        let recent_proof = CachedProof {
            state_key: b"account_bob".to_vec(),
            proof_data: vec![4, 5, 6],
            at_round: 1500,
        };

        store.cache_proof(&old_proof).unwrap();
        store.cache_proof(&recent_proof).unwrap();

        let loaded = store.get_cached_proof(b"account_alice").unwrap().unwrap();
        assert_eq!(loaded.at_round, 100);

        // Evict proofs older than 1000 rounds from current round 2000
        let evicted = store.evict_stale_proofs(2000, 1000).unwrap();
        assert_eq!(evicted, 1);

        assert!(store.get_cached_proof(b"account_alice").unwrap().is_none());
        assert!(store.get_cached_proof(b"account_bob").unwrap().is_some());

        cleanup(&path);
    }

    #[test]
    fn proof_cache_excess_eviction() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        for i in 0u64..10 {
            let proof = CachedProof {
                state_key: format!("key_{i}").into_bytes(),
                proof_data: vec![i as u8],
                at_round: i * 10,
            };
            store.cache_proof(&proof).unwrap();
        }

        // 10 proofs stored, but MAX is 4096 so no eviction needed
        let evicted = store.evict_excess_proofs().unwrap();
        assert_eq!(evicted, 0);

        cleanup(&path);
    }

    #[test]
    fn wallet_state_crud() {
        let path = test_db_path();
        let store = LightStore::open(&path).unwrap();

        let addr = [0xAAu8; 32];
        assert!(store.get_wallet_state(&addr).unwrap().is_none());

        let state = LocalWalletState {
            balance: 1_000_000,
            nonce: 5,
            pending_tx_hashes: vec![[0xBBu8; 32]],
        };
        store.store_wallet_state(&addr, &state).unwrap();

        let loaded = store.get_wallet_state(&addr).unwrap().unwrap();
        assert_eq!(loaded.balance, 1_000_000);
        assert_eq!(loaded.nonce, 5);
        assert_eq!(loaded.pending_tx_hashes.len(), 1);

        cleanup(&path);
    }
}
