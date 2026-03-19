use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};

const PEER_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("peer_reputation");

const BAN_TIER_1_THRESHOLD: f64 = -100.0;
const BAN_TIER_2_THRESHOLD: f64 = -200.0;
const BAN_TIER_3_THRESHOLD: f64 = -500.0;

const BAN_TIER_1_SECS: u64 = 3600;
const BAN_TIER_2_SECS: u64 = 86400;
const BAN_TIER_3_SECS: u64 = 604800;

const SCORE_DECAY_PER_HOUR: f64 = 5.0;
const MAX_REPUTATION_ENTRIES: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffenseSeverity {
    Low,
    Medium,
    High,
}

impl OffenseSeverity {
    fn penalty(self) -> f64 {
        match self {
            Self::Low => -5.0,
            Self::Medium => -25.0,
            Self::High => -100.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReputation {
    pub score: f64,
    pub last_seen: u64,
    pub last_decay: u64,
    pub banned_until: Option<u64>,
    pub offenses: u16,
}

impl PeerReputation {
    fn new(now: u64) -> Self {
        Self {
            score: 0.0,
            last_seen: now,
            last_decay: now,
            banned_until: None,
            offenses: 0,
        }
    }

    pub fn is_banned(&self, now: u64) -> bool {
        self.banned_until.is_some_and(|until| now < until)
    }

    fn apply_decay(&mut self, now: u64) {
        if self.score >= 0.0 {
            self.last_decay = now;
            return;
        }
        let elapsed_hours = now.saturating_sub(self.last_decay) / 3600;
        if elapsed_hours > 0 {
            self.score = (self.score + elapsed_hours as f64 * SCORE_DECAY_PER_HOUR).min(0.0);
            self.last_decay = now;
        }
    }

    fn apply_offense(&mut self, severity: OffenseSeverity, now: u64) {
        self.score += severity.penalty();
        self.offenses = self.offenses.saturating_add(1);
        self.last_seen = now;

        if self.score <= BAN_TIER_3_THRESHOLD {
            self.banned_until = Some(now + BAN_TIER_3_SECS);
        } else if self.score <= BAN_TIER_2_THRESHOLD {
            self.banned_until = Some(now + BAN_TIER_2_SECS);
        } else if self.score <= BAN_TIER_1_THRESHOLD {
            self.banned_until = Some(now + BAN_TIER_1_SECS);
        }
    }
}

pub struct PeerReputationStore {
    db: Database,
}

impl PeerReputationStore {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let db = Database::create(path)?;
        let write_txn = db.begin_write()?;
        write_txn.open_table(PEER_TABLE)?;
        write_txn.commit()?;
        Ok(Self { db })
    }

    pub fn get(&self, peer_id: &[u8]) -> anyhow::Result<Option<PeerReputation>> {
        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_TABLE)?;
        match tbl.get(peer_id)? {
            Some(v) => {
                let rep: PeerReputation = postcard::from_bytes(v.value())?;
                Ok(Some(rep))
            }
            None => Ok(None),
        }
    }

    fn put(&self, peer_id: &[u8], rep: &PeerReputation) -> anyhow::Result<()> {
        let encoded = postcard::to_allocvec(rep)?;
        let write_txn = self.db.begin_write()?;
        {
            let mut tbl = write_txn.open_table(PEER_TABLE)?;
            tbl.insert(peer_id, encoded.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    pub fn record_offense(
        &self,
        peer_id: &[u8],
        severity: OffenseSeverity,
    ) -> anyhow::Result<PeerReputation> {
        let now = unix_now();
        let mut rep = self
            .get(peer_id)?
            .unwrap_or_else(|| PeerReputation::new(now));
        rep.apply_decay(now);
        rep.apply_offense(severity, now);
        self.put(peer_id, &rep)?;
        Ok(rep)
    }

    pub fn record_seen(&self, peer_id: &[u8]) -> anyhow::Result<()> {
        let now = unix_now();
        let mut rep = self
            .get(peer_id)?
            .unwrap_or_else(|| PeerReputation::new(now));
        rep.apply_decay(now);
        rep.last_seen = now;
        self.put(peer_id, &rep)
    }

    pub fn is_banned(&self, peer_id: &[u8]) -> anyhow::Result<bool> {
        let now = unix_now();
        match self.get(peer_id)? {
            Some(rep) => Ok(rep.is_banned(now)),
            None => Ok(false),
        }
    }

    pub fn prune_stale(&self, max_age_secs: u64) -> anyhow::Result<u64> {
        let cutoff = unix_now().saturating_sub(max_age_secs);

        let read_txn = self.db.begin_read()?;
        let tbl = read_txn.open_table(PEER_TABLE)?;
        let mut stale_keys = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let rep: PeerReputation = postcard::from_bytes(v.value())?;
            if rep.last_seen < cutoff && !rep.is_banned(unix_now()) {
                stale_keys.push(k.value().to_vec());
            }
        }
        drop(tbl);
        drop(read_txn);

        let removed = stale_keys.len() as u64;
        if !stale_keys.is_empty() {
            let write_txn = self.db.begin_write()?;
            {
                let mut tbl = write_txn.open_table(PEER_TABLE)?;
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
        let tbl = read_txn.open_table(PEER_TABLE)?;
        let total = tbl.len()?;
        if total <= MAX_REPUTATION_ENTRIES {
            return Ok(0);
        }
        let to_remove = total - MAX_REPUTATION_ENTRIES;

        let mut entries: Vec<(Vec<u8>, u64)> = Vec::new();
        for entry in tbl.iter()? {
            let (k, v) = entry?;
            let rep: PeerReputation = postcard::from_bytes(v.value())?;
            entries.push((k.value().to_vec(), rep.last_seen));
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
                let mut tbl = write_txn.open_table(PEER_TABLE)?;
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
        let tbl = read_txn.open_table(PEER_TABLE)?;
        Ok(tbl.len()?)
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
        std::env::temp_dir().join(format!("aztibase_rep_test_{}_{}.redb", pid, id))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    #[test]
    fn banned_peer_rejected() {
        let path = test_db_path();
        let store = PeerReputationStore::open(&path).unwrap();
        let peer = b"peer_banned_test";

        for _ in 0..2 {
            store.record_offense(peer, OffenseSeverity::High).unwrap();
        }

        assert!(store.is_banned(peer).unwrap());

        let rep = store.get(peer).unwrap().unwrap();
        assert!(rep.score <= BAN_TIER_1_THRESHOLD);
        assert!(rep.banned_until.is_some());
        assert_eq!(rep.offenses, 2);
        cleanup(&path);
    }

    #[test]
    fn offense_accumulation() {
        let path = test_db_path();
        let store = PeerReputationStore::open(&path).unwrap();
        let peer = b"peer_offense_test";

        store.record_offense(peer, OffenseSeverity::Low).unwrap();
        let rep = store.get(peer).unwrap().unwrap();
        assert!((rep.score - (-5.0)).abs() < f64::EPSILON);
        assert_eq!(rep.offenses, 1);

        store.record_offense(peer, OffenseSeverity::Medium).unwrap();
        let rep = store.get(peer).unwrap().unwrap();
        assert!((rep.score - (-30.0)).abs() < f64::EPSILON);
        assert_eq!(rep.offenses, 2);

        assert!(!store.is_banned(peer).unwrap());
        cleanup(&path);
    }

    #[test]
    fn score_decay_over_time() {
        let path = test_db_path();
        let store = PeerReputationStore::open(&path).unwrap();
        let peer = b"peer_decay_test";

        let mut rep = PeerReputation::new(1000);
        rep.score = -50.0;
        rep.last_decay = 1000;
        store.put(peer, &rep).unwrap();

        let mut loaded = store.get(peer).unwrap().unwrap();
        loaded.apply_decay(1000 + 7200);
        assert!((loaded.score - (-40.0)).abs() < f64::EPSILON);

        let mut neutral = PeerReputation::new(1000);
        neutral.score = -1.0;
        neutral.last_decay = 1000;
        neutral.apply_decay(1000 + 7200);
        assert!(neutral.score <= 0.0);
        cleanup(&path);
    }

    #[test]
    fn ban_expiry() {
        let now = unix_now();
        let mut rep = PeerReputation::new(now);
        rep.banned_until = Some(now + 10);

        assert!(rep.is_banned(now));
        assert!(rep.is_banned(now + 9));
        assert!(!rep.is_banned(now + 10));
        assert!(!rep.is_banned(now + 100));
    }

    #[test]
    fn ban_tiers() {
        let now = 10_000u64;
        let mut rep = PeerReputation::new(now);

        rep.apply_offense(OffenseSeverity::High, now);
        assert!(rep.is_banned(now));
        assert_eq!(rep.banned_until, Some(now + BAN_TIER_1_SECS));

        rep.apply_offense(OffenseSeverity::High, now);
        assert_eq!(rep.banned_until, Some(now + BAN_TIER_2_SECS));

        for _ in 0..3 {
            rep.apply_offense(OffenseSeverity::High, now);
        }
        assert_eq!(rep.banned_until, Some(now + BAN_TIER_3_SECS));
    }

    #[test]
    fn prune_stale_peers() {
        let path = test_db_path();
        let store = PeerReputationStore::open(&path).unwrap();

        let now = unix_now();
        let old = PeerReputation {
            score: 0.0,
            last_seen: now.saturating_sub(100_000),
            last_decay: now.saturating_sub(100_000),
            banned_until: None,
            offenses: 0,
        };
        store.put(b"old_peer", &old).unwrap();
        store.record_seen(b"fresh_peer").unwrap();

        let pruned = store.prune_stale(86400).unwrap();
        assert_eq!(pruned, 1);
        assert!(store.get(b"old_peer").unwrap().is_none());
        assert!(store.get(b"fresh_peer").unwrap().is_some());
        cleanup(&path);
    }

    #[test]
    fn unknown_peer_not_banned() {
        let path = test_db_path();
        let store = PeerReputationStore::open(&path).unwrap();
        assert!(!store.is_banned(b"nobody").unwrap());
        cleanup(&path);
    }
}
