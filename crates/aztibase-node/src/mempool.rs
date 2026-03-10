use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};

use aztibase_core::hash;
use aztibase_execution::{SignedTx, routing::route_tx};

type TxHash = [u8; 32];

const MAX_NONCE_GAP: u64 = 16;

/// Priority key for BTreeMap ordering: highest priority first, then by hash for determinism.
type PriorityKey = (Reverse<u64>, TxHash);

pub struct Mempool {
    /// Transactions ordered by (Reverse(priority), hash) — highest priority first.
    ordered: BTreeMap<PriorityKey, Vec<u8>>,
    /// Map from tx hash to its priority (for removal and eviction lookups).
    index: HashMap<TxHash, u64>,
    /// Set of all ever-seen tx hashes (persists after removal to prevent replay).
    /// Bounded to `max_seen` entries; oldest entries are evicted via `seen_order`.
    seen: HashSet<TxHash>,
    seen_order: Vec<TxHash>,
    max_size: usize,
    max_seen: usize,
}

#[allow(dead_code)]
impl Mempool {
    pub fn new(max_size: usize) -> Self {
        let max_seen = max_size * 10;
        Self {
            ordered: BTreeMap::new(),
            index: HashMap::new(),
            seen: HashSet::new(),
            seen_order: Vec::new(),
            max_size,
            max_seen,
        }
    }

    /// Insert a transaction with a given priority.
    /// Returns false if already seen. If the pool is full, evicts the lowest-priority
    /// entry when the new transaction has higher priority; rejects otherwise.
    pub fn insert_with_priority(&mut self, tx: Vec<u8>, priority: u64) -> bool {
        let tx_hash = hash(&tx);

        if self.seen.contains(&tx_hash) {
            return false;
        }

        if self.ordered.len() >= self.max_size {
            let (&lowest_key, _) = match self.ordered.last_key_value() {
                Some(kv) => kv,
                None => return false,
            };
            let lowest_priority = lowest_key.0.0;
            if priority <= lowest_priority {
                return false;
            }
            // Evict lowest-priority entry
            let evicted_hash = lowest_key.1;
            self.ordered.remove(&lowest_key);
            self.index.remove(&evicted_hash);
        }

        self.seen.insert(tx_hash);
        self.seen_order.push(tx_hash);
        if self.seen.len() > self.max_seen {
            let oldest = self.seen_order.remove(0);
            if !self.index.contains_key(&oldest) {
                self.seen.remove(&oldest);
            }
        }
        self.ordered.insert((Reverse(priority), tx_hash), tx);
        self.index.insert(tx_hash, priority);
        true
    }

    /// Insert a transaction with default priority derived from the tx prefix byte.
    pub fn insert(&mut self, tx: Vec<u8>) -> bool {
        let priority = extract_priority(&tx);
        self.insert_with_priority(tx, priority)
    }

    /// Insert a signed envelope with nonce and gas price validation.
    /// `nonce_lookup` returns the current on-chain nonce for a given address.
    /// Rejects stale nonces (tx.nonce < current), far-future nonces (gap > 16),
    /// and transactions below `min_gas_price`.
    pub fn insert_checked<F>(&mut self, tx: Vec<u8>, nonce_lookup: F, min_gas_price: u64) -> bool
    where
        F: Fn(&[u8; 32]) -> u64,
    {
        let (sender, tx_nonce, gas_price) = match decode_sender_nonce_gas(&tx) {
            Some(t) => t,
            None => return false,
        };
        if gas_price < min_gas_price {
            return false;
        }
        let current = nonce_lookup(&sender);
        if tx_nonce < current || tx_nonce > current + MAX_NONCE_GAP {
            return false;
        }
        self.insert(tx)
    }

    /// Remove a transaction by hash.
    pub fn remove(&mut self, tx_hash: &TxHash) -> Option<Vec<u8>> {
        let priority = self.index.remove(tx_hash)?;
        let key = (Reverse(priority), *tx_hash);
        self.ordered.remove(&key)
    }

    /// Get up to `count` transactions for inclusion in a vertex, respecting `max_bytes`.
    /// Returns transactions in priority order (highest first).
    pub fn peek_batch(&self, count: usize, max_bytes: usize) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let mut total_bytes = 0;

        for tx in self.ordered.values() {
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
    /// Returns transactions in priority order (highest first).
    pub fn drain_batch(&mut self, count: usize, max_bytes: usize) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let mut to_remove = Vec::new();
        let mut total_bytes = 0;

        for (key, tx) in &self.ordered {
            if result.len() >= count {
                break;
            }
            if total_bytes + tx.len() > max_bytes {
                break;
            }
            total_bytes += tx.len();
            result.push(tx.clone());
            to_remove.push(*key);
        }

        for key in &to_remove {
            self.ordered.remove(key);
            self.index.remove(&key.1);
        }

        result
    }

    pub fn contains(&self, tx_hash: &TxHash) -> bool {
        self.index.contains_key(tx_hash)
    }

    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }
}

/// Decode a signed envelope to extract (sender_address, nonce, gas_price).
fn decode_sender_nonce_gas(raw: &[u8]) -> Option<([u8; 32], u64, u64)> {
    let signed = SignedTx::decode(raw).ok()?;
    let tx = route_tx(&signed.payload).ok()?;
    Some((signed.sender_address(), tx.nonce(), tx.gas_price()))
}

/// Extract priority from raw transaction bytes.
/// For signed envelopes (magic 0xAA), parse the inner payload prefix.
/// Falls back to first byte for raw payloads.
fn extract_priority(tx: &[u8]) -> u64 {
    if let Ok(signed) = SignedTx::decode(tx) {
        signed.payload.first().copied().unwrap_or(0) as u64
    } else {
        tx.first().copied().unwrap_or(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::{Keypair, address_from_pubkey};
    use aztibase_execution::{SignedTx, TxKind};

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
        // Same priority as existing — rejected
        assert!(!pool.insert_with_priority(vec![3], 0));
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

    // ── Priority ordering tests ──────────────────────────────────

    #[test]
    fn higher_priority_drained_first() {
        let mut pool = Mempool::new(100);
        let low = vec![1, 1, 1];
        let high = vec![2, 2, 2];
        let mid = vec![3, 3, 3];

        pool.insert_with_priority(low.clone(), 10);
        pool.insert_with_priority(high.clone(), 100);
        pool.insert_with_priority(mid.clone(), 50);

        let batch = pool.drain_batch(3, 1024);
        assert_eq!(batch[0], high);
        assert_eq!(batch[1], mid);
        assert_eq!(batch[2], low);
    }

    #[test]
    fn peek_returns_priority_order() {
        let mut pool = Mempool::new(100);
        pool.insert_with_priority(vec![10], 1);
        pool.insert_with_priority(vec![20], 100);
        pool.insert_with_priority(vec![30], 50);

        let batch = pool.peek_batch(3, 1024);
        assert_eq!(batch[0], vec![20]);
        assert_eq!(batch[1], vec![30]);
        assert_eq!(batch[2], vec![10]);
    }

    // ── Eviction tests ───────────────────────────────────────────

    #[test]
    fn eviction_replaces_lowest_priority() {
        let mut pool = Mempool::new(2);
        pool.insert_with_priority(vec![1], 10);
        pool.insert_with_priority(vec![2], 20);
        assert_eq!(pool.len(), 2);

        // Higher priority than lowest (10) — should evict vec![1]
        assert!(pool.insert_with_priority(vec![3], 30));
        assert_eq!(pool.len(), 2);

        let batch = pool.drain_batch(10, 1024);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0], vec![3]); // priority 30
        assert_eq!(batch[1], vec![2]); // priority 20
    }

    #[test]
    fn eviction_rejects_equal_priority() {
        let mut pool = Mempool::new(2);
        pool.insert_with_priority(vec![1], 10);
        pool.insert_with_priority(vec![2], 10);

        // Same priority as lowest — rejected
        assert!(!pool.insert_with_priority(vec![3], 10));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn eviction_rejects_lower_priority() {
        let mut pool = Mempool::new(2);
        pool.insert_with_priority(vec![1], 50);
        pool.insert_with_priority(vec![2], 100);

        // Lower than lowest (50) — rejected
        assert!(!pool.insert_with_priority(vec![3], 5));
        assert_eq!(pool.len(), 2);
    }

    // ── Hash-based dedup tests ───────────────────────────────────

    #[test]
    fn dedup_by_hash_persists_after_removal() {
        let mut pool = Mempool::new(100);
        let tx = vec![42, 43, 44];
        let tx_hash = hash(&tx);

        assert!(pool.insert(tx.clone()));
        pool.remove(&tx_hash);
        assert!(pool.is_empty());

        // Same tx should be rejected (hash in `seen` set)
        assert!(!pool.insert(tx));
    }

    #[test]
    fn dedup_uses_blake3_hash() {
        let mut pool = Mempool::new(100);
        let tx1 = vec![1, 2, 3];
        let tx2 = vec![1, 2, 3]; // same content → same hash

        assert!(pool.insert(tx1));
        assert!(!pool.insert(tx2));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn different_content_different_hash() {
        let mut pool = Mempool::new(100);
        let tx1 = vec![1, 2, 3];
        let tx2 = vec![4, 5, 6];

        assert!(pool.insert(tx1));
        assert!(pool.insert(tx2));
        assert_eq!(pool.len(), 2);
    }

    #[test]
    fn seen_set_bounded() {
        let mut pool = Mempool::new(2); // max_seen = 20
        pool.max_seen = 5;

        // Insert and drain 6 unique txs — the seen set should cap at 5
        for i in 0u8..6 {
            let tx = vec![0x01, i, i, i];
            pool.insert_with_priority(tx, 100);
        }
        // Pool has 2 (latest inserts after evictions), seen has at most 5
        assert!(pool.seen.len() <= 5);
    }

    #[test]
    fn extract_priority_from_prefix() {
        assert_eq!(extract_priority(&[0x01, 0xFF]), 1);
        assert_eq!(extract_priority(&[0x06, 0x00]), 6);
        assert_eq!(extract_priority(&[]), 0);
    }

    #[test]
    fn default_insert_uses_prefix_priority() {
        let mut pool = Mempool::new(100);
        // Prefix 0x06 (AI infer) should have higher priority than 0x01 (transfer)
        let transfer = vec![0x01, 1, 2, 3];
        let ai_infer = vec![0x06, 4, 5, 6];

        pool.insert(transfer.clone());
        pool.insert(ai_infer.clone());

        let batch = pool.peek_batch(2, 1024);
        assert_eq!(batch[0], ai_infer); // priority 6
        assert_eq!(batch[1], transfer); // priority 1
    }

    // ── Nonce validation tests ──────────────────────────────────

    fn make_signed_transfer(kp: &Keypair, nonce: u64) -> Vec<u8> {
        let sender = address_from_pubkey(kp.public_key().as_bytes());
        let tx = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 100,
            nonce,
            gas_price: 1,
        };
        SignedTx::new(tx.encode(), kp).encode()
    }

    #[test]
    fn insert_checked_accepts_matching_nonce() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = make_signed_transfer(&kp, 0);
        assert!(pool.insert_checked(tx, |_| 0, 0));
    }

    #[test]
    fn insert_checked_accepts_future_within_gap() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = make_signed_transfer(&kp, 16);
        assert!(pool.insert_checked(tx, |_| 0, 0));
    }

    #[test]
    fn insert_checked_rejects_stale_nonce() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = make_signed_transfer(&kp, 3);
        assert!(!pool.insert_checked(tx, |_| 5, 0));
    }

    #[test]
    fn insert_checked_rejects_far_future_nonce() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = make_signed_transfer(&kp, 17);
        assert!(!pool.insert_checked(tx, |_| 0, 0));
    }

    #[test]
    fn insert_checked_rejects_malformed() {
        let mut pool = Mempool::new(100);
        assert!(!pool.insert_checked(vec![0xFF, 0x00], |_| 0, 0));
    }

    #[test]
    fn insert_checked_rejects_below_base_fee() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        // gas_price=1 < min_gas_price=2, should be rejected
        let tx = make_signed_transfer(&kp, 0);
        assert!(!pool.insert_checked(tx, |_| 0, 2));
    }

    #[test]
    fn insert_checked_accepts_at_base_fee() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let sender = address_from_pubkey(kp.public_key().as_bytes());
        let tx = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 5,
        };
        let signed = SignedTx::new(tx.encode(), &kp).encode();
        assert!(pool.insert_checked(signed, |_| 0, 5));
    }

    #[test]
    fn signed_envelope_gets_inner_priority() {
        let kp = Keypair::generate();
        let sender = address_from_pubkey(kp.public_key().as_bytes());
        let transfer = TxKind::Transfer {
            from: sender,
            to: [2u8; 32],
            value: 100,
            nonce: 0,
            gas_price: 1,
        };
        let signed = SignedTx::new(transfer.encode(), &kp).encode();
        assert_eq!(extract_priority(&signed), 1);
    }
}
