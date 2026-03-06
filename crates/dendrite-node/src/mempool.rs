use std::collections::{BTreeMap, HashSet};

use dendrite_core::hash;

type TxHash = [u8; 32];

pub struct Mempool {
    txs: BTreeMap<TxHash, Vec<u8>>,
    seen: HashSet<TxHash>,
    max_size: usize,
}

#[allow(dead_code)]
impl Mempool {
    pub fn new(max_size: usize) -> Self {
        Self {
            txs: BTreeMap::new(),
            seen: HashSet::new(),
            max_size,
        }
    }

    /// Insert a transaction. Returns false if already seen or pool is full.
    pub fn insert(&mut self, tx: Vec<u8>) -> bool {
        let tx_hash = hash(&tx);

        if self.seen.contains(&tx_hash) {
            return false;
        }

        if self.txs.len() >= self.max_size {
            return false;
        }

        self.seen.insert(tx_hash);
        self.txs.insert(tx_hash, tx);
        true
    }

    /// Remove a transaction by hash.
    pub fn remove(&mut self, tx_hash: &TxHash) -> Option<Vec<u8>> {
        self.txs.remove(tx_hash)
    }

    /// Remove multiple transactions by hash (e.g., after inclusion in a block).
    pub fn remove_batch(&mut self, hashes: &[TxHash]) {
        for h in hashes {
            self.txs.remove(h);
        }
    }

    /// Get up to `count` transactions for inclusion in a vertex, respecting `max_bytes`.
    pub fn peek_batch(&self, count: usize, max_bytes: usize) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let mut total_bytes = 0;

        for tx in self.txs.values() {
            if result.len() >= count {
                break;
            }
            if total_bytes + tx.len() > max_bytes {
                break;
            }
            total_bytes += tx.len();
            result.push(tx.clone());
        }

        result
    }

    /// Drain up to `count` transactions for inclusion, removing them from the pool.
    pub fn drain_batch(&mut self, count: usize, max_bytes: usize) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let mut to_remove = Vec::new();
        let mut total_bytes = 0;

        for (hash, tx) in &self.txs {
            if result.len() >= count {
                break;
            }
            if total_bytes + tx.len() > max_bytes {
                break;
            }
            total_bytes += tx.len();
            result.push(tx.clone());
            to_remove.push(*hash);
        }

        for h in &to_remove {
            self.txs.remove(h);
        }

        result
    }

    pub fn contains(&self, tx_hash: &TxHash) -> bool {
        self.txs.contains_key(tx_hash)
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_retrieve() {
        let mut pool = Mempool::new(100);
        let tx = vec![1, 2, 3, 4];
        assert!(pool.insert(tx.clone()));
        assert_eq!(pool.len(), 1);

        let batch = pool.peek_batch(10, 1024);
        assert_eq!(batch.len(), 1);
        assert_eq!(batch[0], tx);
    }

    #[test]
    fn rejects_duplicates() {
        let mut pool = Mempool::new(100);
        let tx = vec![10, 20, 30];
        assert!(pool.insert(tx.clone()));
        assert!(!pool.insert(tx));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn rejects_when_full() {
        let mut pool = Mempool::new(2);
        assert!(pool.insert(vec![1]));
        assert!(pool.insert(vec![2]));
        assert!(!pool.insert(vec![3]));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn remove_by_hash() {
        let mut pool = Mempool::new(100);
        let tx = vec![5, 6, 7];
        let tx_hash = hash(&tx);
        pool.insert(tx);
        assert!(pool.contains(&tx_hash));

        let removed = pool.remove(&tx_hash);
        assert!(removed.is_some());
        assert!(!pool.contains(&tx_hash));
        assert!(pool.is_empty());
    }

    #[test]
    fn drain_batch_removes_from_pool() {
        let mut pool = Mempool::new(100);
        pool.insert(vec![1, 2]);
        pool.insert(vec![3, 4]);
        pool.insert(vec![5, 6]);

        let batch = pool.drain_batch(2, 1024);
        assert_eq!(batch.len(), 2);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn peek_batch_respects_max_bytes() {
        let mut pool = Mempool::new(100);
        pool.insert(vec![0; 100]);
        pool.insert(vec![0; 100]);
        pool.insert(vec![0; 100]);

        let batch = pool.peek_batch(10, 150);
        assert_eq!(batch.len(), 1);
    }
}
