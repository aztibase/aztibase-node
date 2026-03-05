use std::collections::{HashMap, HashSet, VecDeque};

use dendrite_core::BlockHash;
use dendrite_storage::{BLOCKS_TABLE, StateStore, StorageError};

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
pub struct DagStore {
    store: StateStore,
    index: HashMap<BlockHash, DagEntry>,
    rounds: HashMap<u64, Vec<BlockHash>>,
}

impl DagStore {
    /// Open a DagStore backed by the given StateStore.
    /// Loads existing blocks from disk into the in-memory index.
    pub fn new(store: StateStore) -> DagStoreResult<Self> {
        let mut dag = Self {
            store,
            index: HashMap::new(),
            rounds: HashMap::new(),
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

        let encoded =
            bincode::serialize(&block).map_err(|e| DagStoreError::Serialization(e.to_string()))?;
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

    /// Retrieve a block by hash, deserializing from disk.
    pub fn get(&self, hash: &BlockHash) -> DagStoreResult<DagBlock> {
        let raw = self
            .store
            .get(BLOCKS_TABLE, hash)?
            .ok_or(DagStoreError::BlockNotFound(*hash))?;
        bincode::deserialize(&raw).map_err(|e| DagStoreError::Serialization(e.to_string()))
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

        let mut in_degree: HashMap<BlockHash, usize> = HashMap::new();
        for &hash in &reachable {
            in_degree.entry(hash).or_insert(0);
            let entry = &self.index[&hash];
            for child in &entry.children {
                if reachable.contains(child) {
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
            let block: DagBlock = bincode::deserialize(value)
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
