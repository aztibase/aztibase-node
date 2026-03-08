use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};

const PEER_ADDR_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("peer_addresses");

const MAX_STORED_PEERS: u64 = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPeer {
    pub addrs: Vec<String>,
    pub last_seen: u64,
}

pub struct PeerStore {
    db: Database,
}

impl PeerStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        write_txn.open_table(PEER_ADDR_TABLE)?;
        write_txn.commit()?;
        Ok(Self { db })
    }

    pub fn insert(&self, peer_id: &[u8], addrs: &[String]) -> anyhow::Result<()> {
        let now = unix_now();
        let existing = self.get(peer_id)?;
        let mut merged = match existing {
            Some(mut stored) => {
                for addr in addrs {
                    if !stored.addrs.contains(addr) {
                        stored.addrs.push(addr.clone());
                    }
                }
                stored.last_seen = now;
                stored
            }
            None => StoredPeer {
                addrs: addrs.to_vec(),
                last_seen: now,
            },
        };
        // Cap addresses per peer
        merged.addrs.truncate(8);

        let encoded = postcard::to_allocvec(&merged)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(PEER_ADDR_TABLE)?;
            tbl.insert(peer_id, encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn get(&self, peer_id: &[u8]) -> anyhow::Result<Option<StoredPeer>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_ADDR_TABLE)?;
        match tbl.get(peer_id)? {
            Some(v) => {
                let stored: StoredPeer = postcard::from_bytes(v.value())?;
                Ok(Some(stored))
            }
            None => Ok(None),
        }
    }

    pub fn update_last_seen(&self, peer_id: &[u8]) -> anyhow::Result<()> {
        if let Some(mut stored) = self.get(peer_id)? {
            stored.last_seen = unix_now();
            let encoded = postcard::to_allocvec(&stored)?;
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PEER_ADDR_TABLE)?;
                tbl.insert(peer_id, encoded.as_slice())?;
            }
            write_txn.commit()?;
        }
        Ok(())
    }

    pub fn list_recent(&self, limit: usize) -> anyhow::Result<Vec<(Vec<u8>, StoredPeer)>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_ADDR_TABLE)?;

        let mut entries: Vec<(Vec<u8>, StoredPeer)> = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let stored: StoredPeer = postcard::from_bytes(v.value())?;
            entries.push((k.value().to_vec(), stored));
        }

        entries.sort_by(|a, b| b.1.last_seen.cmp(&a.1.last_seen));
        entries.truncate(limit);
        Ok(entries)
    }

    pub fn prune_stale(&self, max_age_secs: u64) -> anyhow::Result<u64> {
        let cutoff = unix_now().saturating_sub(max_age_secs);

        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_ADDR_TABLE)?;
        let mut stale_keys = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let stored: StoredPeer = postcard::from_bytes(v.value())?;
            if stored.last_seen < cutoff {
                stale_keys.push(k.value().to_vec());
            }
        }
        drop(tbl);
        drop(read_txn);

        let removed = stale_keys.len() as u64;
        if !stale_keys.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PEER_ADDR_TABLE)?;
                for key in &stale_keys {
                    tbl.remove(key.as_slice())?;
                }
            }
            write_txn.commit()?;
        }
        Ok(removed)
    }

    pub fn evict_excess(&self) -> anyhow::Result<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_ADDR_TABLE)?;
        let total = tbl.len()?;
        if total <= MAX_STORED_PEERS {
            return Ok(0);
        }
        let to_remove = total - MAX_STORED_PEERS;

        let mut entries: Vec<(Vec<u8>, u64)> = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let stored: StoredPeer = postcard::from_bytes(v.value())?;
            entries.push((k.value().to_vec(), stored.last_seen));
        }
        drop(tbl);
        drop(read_txn);

        entries.sort_by_key(|(_, seen)| *seen);
        let keys_to_remove: Vec<Vec<u8>> = entries
            .into_iter()
            .take(to_remove as usize)
            .map(|(k, _)| k)
            .collect();

        let removed = keys_to_remove.len() as u64;
        if !keys_to_remove.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PEER_ADDR_TABLE)?;
                for key in &keys_to_remove {
                    tbl.remove(key.as_slice())?;
                }
            }
            write_txn.commit()?;
        }
        Ok(removed)
    }

    pub fn count(&self) -> anyhow::Result<u64> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_ADDR_TABLE)?;
        Ok(tbl.len()?)
    }

    pub fn remove(&self, peer_id: &[u8]) -> anyhow::Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(PEER_ADDR_TABLE)?;
            tbl.remove(peer_id)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_peer_store_test_{}_{}.redb", pid, id))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    #[test]
    fn peer_persists_across_reopen() {
        let path = test_db_path();
        let peer = b"peer_persist_test";
        let addrs = vec!["/ip4/1.2.3.4/tcp/30333".to_string()];

        {
            let store = PeerStore::open(&path).unwrap();
            store.insert(peer, &addrs).unwrap();
            assert_eq!(store.count().unwrap(), 1);
        }

        {
            let store = PeerStore::open(&path).unwrap();
            let stored = store.get(peer).unwrap().unwrap();
            assert_eq!(stored.addrs, addrs);
        }

        cleanup(&path);
    }

    #[test]
    fn stale_peers_pruned() {
        let path = test_db_path();
        let store = PeerStore::open(&path).unwrap();

        let old_peer = StoredPeer {
            addrs: vec!["/ip4/10.0.0.1/tcp/30333".to_string()],
            last_seen: 1000,
        };
        let encoded = postcard::to_allocvec(&old_peer).unwrap();
        let write_txn = store.db.begin_write().unwrap();
        {
            let mut tbl = write_txn.open_table(PEER_ADDR_TABLE).unwrap();
            tbl.insert(b"old_peer".as_slice(), encoded.as_slice())
                .unwrap();
        }
        write_txn.commit().unwrap();

        store
            .insert(b"fresh_peer", &["/ip4/10.0.0.2/tcp/30333".to_string()])
            .unwrap();

        let pruned = store.prune_stale(86400).unwrap();
        assert_eq!(pruned, 1);
        assert!(store.get(b"old_peer").unwrap().is_none());
        assert!(store.get(b"fresh_peer").unwrap().is_some());

        cleanup(&path);
    }

    #[test]
    fn address_merge_deduplicates() {
        let path = test_db_path();
        let store = PeerStore::open(&path).unwrap();
        let peer = b"peer_merge";

        store
            .insert(peer, &["/ip4/1.2.3.4/tcp/30333".to_string()])
            .unwrap();
        store
            .insert(
                peer,
                &[
                    "/ip4/1.2.3.4/tcp/30333".to_string(),
                    "/ip4/5.6.7.8/tcp/30333".to_string(),
                ],
            )
            .unwrap();

        let stored = store.get(peer).unwrap().unwrap();
        assert_eq!(stored.addrs.len(), 2);

        cleanup(&path);
    }

    #[test]
    fn list_recent_respects_limit() {
        let path = test_db_path();
        let store = PeerStore::open(&path).unwrap();

        for i in 0..5u8 {
            let peer_id = [i; 1];
            store
                .insert(&peer_id, &[format!("/ip4/10.0.0.{i}/tcp/30333")])
                .unwrap();
        }

        let recent = store.list_recent(3).unwrap();
        assert_eq!(recent.len(), 3);

        cleanup(&path);
    }
}
