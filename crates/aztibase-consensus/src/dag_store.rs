use std::collections::{HashMap, HashSet, VecDeque};

use aztibase_core::BlockHash;
use aztibase_storage::{BLOCKS_TABLE, StateStore, StorageError};
use tracing::debug;

use crate::dag::{DagBlock, DagError};

/// Errors from DagStore operations.
#[derive(Debug, thiserror::Error)]
pub enum DagStoreError {
    #[error("Block not found: {0:?}")]
    BlockNotFound(BlockHash),

    #[error("Missing parent {parent:?} for block {block:?}")]
    MissingParent { block: BlockHash, parent: BlockHash },

    #[error("Duplicate block: {0:?}")]
    DuplicateBlock(BlockHash),

    #[error(transparent)]
    Dag(#[from] DagError),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type DagStoreResult<T> = Result<T, DagStoreError>;

/// In-memory DAG index entry. Tracks relationships without full block data.
struct DagEntry {
    round: u64,
    parents: Vec<BlockHash>,
    children: HashSet<BlockHash>,
}

/// Persistent DAG block store backed by redb with an in-memory relationship index.
///
/// Blocks are serialized to `BLOCKS_TABLE` via `StateStore`. An in-memory index
/// tracks parent/child edges, round membership, and enables efficient traversal
/// without deserializing from disk on every query.
/// Minimum number of rounds to retain below the prune target.
/// Must be ≥ 2 × wave_length to avoid pruning blocks needed by the commit rule.
const DAG_RETENTION_BUFFER: u64 = 16;

pub struct DagStore {
    store: StateStore,
    index: HashMap<BlockHash, DagEntry>,
    rounds: HashMap<u64, Vec<BlockHash>>,
    pruned_through: u64,
    orphan_parents: HashSet<BlockHash>,
}

impl DagStore {
    /// Open a DagStore backed by the given StateStore.
    /// Loads existing blocks from disk into the in-memory index.
    pub fn new(store: StateStore) -> DagStoreResult<Self> {
        let mut dag = Self {
            store,
            index: HashMap::new(),
            rounds: HashMap::new(),
            pruned_through: 0,
            orphan_parents: HashSet::new(),
        };
        dag.rebuild_index()?;
        Ok(dag)
    }

    /// Insert a block into the DAG. All parents must already exist
    /// (except for genesis blocks which have no parents).
    pub fn insert(&mut self, block: DagBlock) -> DagStoreResult<()> {
        let hash = block.hash;

        if self.index.contains_key(&hash) {
            return Err(DagStoreError::DuplicateBlock(hash));
        }

        if !block.is_genesis() {
            for parent_hash in &block.parents {
                if !self.index.contains_key(parent_hash) {
                    return Err(DagStoreError::MissingParent {
                        block: hash,
                        parent: *parent_hash,
                    });
                }
            }

            let parent_rounds: Vec<(BlockHash, u64)> = block
                .parents
                .iter()
                .map(|ph| (*ph, self.index[ph].round))
                .collect();
            block.validate_parent_rounds(&parent_rounds)?;
        }

        let encoded = postcard::to_allocvec(&block)
            .map_err(|e| DagStoreError::Serialization(e.to_string()))?;
        self.store.put(BLOCKS_TABLE, &hash, &encoded)?;

        for parent_hash in &block.parents {
            if let Some(parent_entry) = self.index.get_mut(parent_hash) {
                parent_entry.children.insert(hash);
            }
        }

        self.rounds.entry(block.round).or_default().push(hash);

        self.index.insert(
            hash,
            DagEntry {
                round: block.round,
                parents: block.parents,
                children: HashSet::new(),
            },
        );

        Ok(())
    }

    /// Insert a block without checking parent existence or validating parent rounds.
    /// Used for blocks received from peers where parents may arrive out of order.
    /// The block's hash, round, and parent references are trusted from wire validation.
    pub fn insert_relaxed(&mut self, block: DagBlock) -> DagStoreResult<()> {
        let hash = block.hash;

        if self.index.contains_key(&hash) {
            return Err(DagStoreError::DuplicateBlock(hash));
        }

        let encoded = postcard::to_allocvec(&block)
            .map_err(|e| DagStoreError::Serialization(e.to_string()))?;
        self.store.put(BLOCKS_TABLE, &hash, &encoded)?;

        for parent_hash in &block.parents {
            if let Some(parent_entry) = self.index.get_mut(parent_hash) {
                parent_entry.children.insert(hash);
            } else {
                tracing::warn!(
                    parent = ?parent_hash,
                    block = ?hash,
                    "phantom parent: block references non-existent parent"
                );
                self.orphan_parents.insert(*parent_hash);
            }
        }

        if self.orphan_parents.remove(&hash) {
            debug!(block = ?hash, "resolved orphan parent");
        }

        self.rounds.entry(block.round).or_default().push(hash);

        self.index.insert(
            hash,
            DagEntry {
                round: block.round,
                parents: block.parents,
                children: HashSet::new(),
            },
        );

        Ok(())
    }

    /// Retrieve a block by hash, deserializing from disk.
    pub fn get(&self, hash: &BlockHash) -> DagStoreResult<DagBlock> {
        let raw = self
            .store
            .get(BLOCKS_TABLE, hash)?
            .ok_or(DagStoreError::BlockNotFound(*hash))?;
        postcard::from_bytes(&raw).map_err(|e| DagStoreError::Serialization(e.to_string()))
    }

    /// Check if a block exists in the store.
    pub fn contains(&self, hash: &BlockHash) -> bool {
        self.index.contains_key(hash)
    }

    /// Get all block hashes at a given round.
    pub fn blocks_at_round(&self, round: u64) -> &[BlockHash] {
        self.rounds.get(&round).map_or(&[], |v| v.as_slice())
    }

    /// Get the parent hashes of a block.
    pub fn parents(&self, hash: &BlockHash) -> DagStoreResult<&[BlockHash]> {
        self.index
            .get(hash)
            .map(|e| e.parents.as_slice())
            .ok_or(DagStoreError::BlockNotFound(*hash))
    }

    /// Get the children (blocks that reference this block as a parent).
    pub fn children(&self, hash: &BlockHash) -> DagStoreResult<&HashSet<BlockHash>> {
        self.index
            .get(hash)
            .map(|e| &e.children)
            .ok_or(DagStoreError::BlockNotFound(*hash))
    }

    /// Get the round of a block.
    pub fn round_of(&self, hash: &BlockHash) -> DagStoreResult<u64> {
        self.index
            .get(hash)
            .map(|e| e.round)
            .ok_or(DagStoreError::BlockNotFound(*hash))
    }

    /// Check if `ancestor` is a causal ancestor of `descendant`.
    /// Uses BFS backwards through parent edges.
    pub fn is_ancestor(&self, ancestor: &BlockHash, descendant: &BlockHash) -> bool {
        if ancestor == descendant {
            return true;
        }

        let ancestor_round = match self.index.get(ancestor) {
            Some(e) => e.round,
            None => return false,
        };

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(*descendant);
        visited.insert(*descendant);

        while let Some(current) = queue.pop_front() {
            let entry = match self.index.get(&current) {
                Some(e) => e,
                None => continue,
            };

            // No need to go deeper than the ancestor's round.
            if entry.round <= ancestor_round && current != *ancestor {
                continue;
            }

            for parent in &entry.parents {
                if *parent == *ancestor {
                    return true;
                }
                if visited.insert(*parent) {
                    queue.push_back(*parent);
                }
            }
        }

        false
    }

    /// Return blocks in causal (topological) order from a set of tips.
    /// Produces a deterministic ordering: lower rounds first, ties broken by hash.
    pub fn causal_order(&self, tips: &[BlockHash]) -> DagStoreResult<Vec<BlockHash>> {
        let reachable = self.reachable_set(tips);

        // Filter to hashes actually present in the index (relaxed insert
        // may add parent references to blocks we haven't received yet).
        let local_reachable: HashSet<BlockHash> = reachable
            .iter()
            .copied()
            .filter(|h| self.index.contains_key(h))
            .collect();

        let mut in_degree: HashMap<BlockHash, usize> = HashMap::new();
        for &hash in &local_reachable {
            in_degree.entry(hash).or_insert(0);
            let entry = &self.index[&hash];
            for child in &entry.children {
                if local_reachable.contains(child) {
                    *in_degree.entry(*child).or_insert(0) += 1;
                }
            }
        }

        // Start with blocks that have no parents within the reachable set (in-degree 0).
        // Use a BinaryHeap-like approach but sorted by (round, hash) for determinism.
        let mut ready: Vec<BlockHash> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(h, _)| *h)
            .collect();
        ready.sort_by_key(|h| (self.index[h].round, *h));

        let mut result = Vec::with_capacity(reachable.len());
        let mut queue = VecDeque::from(ready);

        while let Some(current) = queue.pop_front() {
            result.push(current);
            let entry = &self.index[&current];
            let mut newly_ready = Vec::new();
            for child in &entry.children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        newly_ready.push(*child);
                    }
                }
            }
            newly_ready.sort_by_key(|h| (self.index[h].round, *h));
            queue.extend(newly_ready);
        }

        Ok(result)
    }

    /// Remove blocks in rounds older than `committed_round - DAG_RETENTION_BUFFER`
    /// from both the in-memory index and on-disk BLOCKS_TABLE.
    pub fn prune_before(&mut self, committed_round: u64) -> DagStoreResult<u64> {
        let cutoff = committed_round.saturating_sub(DAG_RETENTION_BUFFER);
        if cutoff <= self.pruned_through {
            return Ok(0);
        }

        let mut pruned_hashes: HashSet<BlockHash> = HashSet::new();
        let mut keys_to_delete: Vec<Vec<u8>> = Vec::new();
        for round in self.pruned_through..cutoff {
            if let Some(hashes) = self.rounds.remove(&round) {
                for hash in &hashes {
                    keys_to_delete.push(hash.to_vec());
                    pruned_hashes.insert(*hash);
                    self.index.remove(hash);
                }
            }
        }

        // Clean up stale child references in retained entries.
        if !pruned_hashes.is_empty() {
            for entry in self.index.values_mut() {
                entry.children.retain(|c| !pruned_hashes.contains(c));
            }
        }

        let removed = if !keys_to_delete.is_empty() {
            self.store.delete_batch(BLOCKS_TABLE, &keys_to_delete)?
        } else {
            0
        };

        debug!(
            old_horizon = self.pruned_through,
            new_horizon = cutoff,
            removed,
            "DAG pruned"
        );
        self.pruned_through = cutoff;
        Ok(removed)
    }

    /// The lowest round still retained in the store.
    pub fn pruned_through(&self) -> u64 {
        self.pruned_through
    }

    /// Total number of blocks in the store.
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Check if the store is empty.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// The highest round number seen in the store.
    pub fn highest_round(&self) -> Option<u64> {
        self.rounds.keys().max().copied()
    }

    /// Collect all block hashes reachable by walking parents from the given tips.
    fn reachable_set(&self, tips: &[BlockHash]) -> HashSet<BlockHash> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        for tip in tips {
            if visited.insert(*tip) {
                queue.push_back(*tip);
            }
        }
        while let Some(current) = queue.pop_front() {
            if let Some(entry) = self.index.get(&current) {
                for parent in &entry.parents {
                    if visited.insert(*parent) {
                        queue.push_back(*parent);
                    }
                }
            }
        }
        visited
    }

    /// Rebuild the in-memory index from all blocks persisted in BLOCKS_TABLE.
    fn rebuild_index(&mut self) -> DagStoreResult<()> {
        let all = self.store.iter(BLOCKS_TABLE)?;
        let mut blocks: Vec<DagBlock> = Vec::with_capacity(all.len());

        for (_key, value) in &all {
            let block: DagBlock = postcard::from_bytes(value)
                .map_err(|e| DagStoreError::Serialization(e.to_string()))?;
            blocks.push(block);
        }

        // Sort by round so parents are indexed before children.
        blocks.sort_by_key(|b| b.round);

        for block in blocks {
            let hash = block.hash;
            self.rounds.entry(block.round).or_default().push(hash);

            for parent_hash in &block.parents {
                if let Some(parent_entry) = self.index.get_mut(parent_hash) {
                    parent_entry.children.insert(hash);
                }
            }

            self.index.insert(
                hash,
                DagEntry {
                    round: block.round,
                    parents: block.parents,
                    children: HashSet::new(),
                },
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::DagBlock;
    use aztibase_storage::StateStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("aztibase_dagstore_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn build_dag(dag: &mut DagStore, rounds: u64, validators: &[[u8; 32]]) {
        for v in validators {
            let genesis = DagBlock::genesis(*v, 1000);
            dag.insert(genesis).unwrap();
        }
        for round in 1..=rounds {
            let parents: Vec<BlockHash> = dag.blocks_at_round(round - 1).to_vec();
            for v in validators {
                let block =
                    DagBlock::new(round, *v, parents.clone(), vec![], 1000 + round, None).unwrap();
                dag.insert(block).unwrap();
            }
        }
    }

    #[test]
    fn prune_removes_old_rounds_from_index_and_disk() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        build_dag(&mut dag, 30, &[v1, v2]);

        // 31 rounds (0-30), 2 validators = 62 blocks
        assert_eq!(dag.len(), 62);

        // Prune with committed_round=30 → cutoff = 30-16 = 14
        let removed = dag.prune_before(30).unwrap();
        assert!(removed > 0);

        // Rounds 0..14 should be gone
        assert!(dag.blocks_at_round(0).is_empty());
        assert!(dag.blocks_at_round(13).is_empty());
        // Round 14+ should remain
        assert!(!dag.blocks_at_round(14).is_empty());
        assert!(!dag.blocks_at_round(30).is_empty());

        assert_eq!(dag.pruned_through(), 14);

        cleanup(&path);
    }

    #[test]
    fn pruned_blocks_return_not_found() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let v1 = [1u8; 32];
        build_dag(&mut dag, 30, &[v1]);

        let old_hashes: Vec<BlockHash> = dag.blocks_at_round(0).to_vec();
        assert!(!old_hashes.is_empty());

        dag.prune_before(30).unwrap();

        for hash in &old_hashes {
            assert!(!dag.contains(hash));
            assert!(dag.get(hash).is_err());
        }

        cleanup(&path);
    }

    #[test]
    fn is_ancestor_handles_pruned_range() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let v1 = [1u8; 32];
        build_dag(&mut dag, 30, &[v1]);

        let old_hash = dag.blocks_at_round(0)[0];
        let tip = dag.blocks_at_round(30)[0];

        // Before prune: old_hash is ancestor of tip
        assert!(dag.is_ancestor(&old_hash, &tip));

        dag.prune_before(30).unwrap();

        // After prune: gracefully returns false (pruned ancestor not in index)
        assert!(!dag.is_ancestor(&old_hash, &tip));

        cleanup(&path);
    }

    #[test]
    fn rebuild_index_after_prune_loads_only_retained() {
        let path = test_db_path();
        let path_str = path.to_str().unwrap().to_string();

        {
            let store = StateStore::open(&path_str).unwrap();
            let mut dag = DagStore::new(store).unwrap();
            let v1 = [1u8; 32];
            build_dag(&mut dag, 30, &[v1]);
            dag.prune_before(30).unwrap();
            // 31 blocks originally, pruned rounds 0..14 = 15 blocks removed
            let remaining = dag.len();
            assert!(remaining < 31);
        }

        // Reopen and rebuild
        let store2 = StateStore::open(&path_str).unwrap();
        let dag2 = DagStore::new(store2).unwrap();

        // Should only have the retained blocks
        assert!(dag2.blocks_at_round(0).is_empty());
        assert!(!dag2.blocks_at_round(30).is_empty());

        cleanup(&path);
    }

    #[test]
    fn prune_is_idempotent() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let v1 = [1u8; 32];
        build_dag(&mut dag, 30, &[v1]);

        let first = dag.prune_before(30).unwrap();
        let count_after = dag.len();
        let second = dag.prune_before(30).unwrap();

        assert!(first > 0);
        assert_eq!(second, 0);
        assert_eq!(dag.len(), count_after);

        cleanup(&path);
    }
}
