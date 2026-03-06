use std::collections::HashSet;

use dendrite_core::BlockHash;

use crate::dag_store::DagStore;

/// A committed batch of transactions extracted from DAG vertices in
/// deterministic topological order.
#[derive(Clone, Debug)]
pub struct CommittedBatch {
    pub anchor_hash: BlockHash,
    pub vertex_order: Vec<BlockHash>,
    pub transactions: Vec<Vec<u8>>,
}

/// Extract and order transactions from a committed anchor vertex and its
/// uncommitted causal history. Vertices are sorted in deterministic
/// topological order (lowest round first, ties broken by hash).
/// Transactions are extracted from each vertex's payload in order.
pub fn extract_committed_batch(
    dag: &DagStore,
    anchor_hash: BlockHash,
    already_committed: &HashSet<BlockHash>,
) -> Result<CommittedBatch, crate::dag_store::DagStoreError> {
    let order = dag.causal_order(&[anchor_hash])?;

    let new_vertices: Vec<BlockHash> = order
        .into_iter()
        .filter(|h| !already_committed.contains(h))
        .collect();

    let mut transactions = Vec::new();
    for vertex_hash in &new_vertices {
        let block = dag.get(vertex_hash)?;
        if !block.payload.is_empty() {
            parse_payload_txs(&block.payload, &mut transactions);
        }
    }

    Ok(CommittedBatch {
        anchor_hash,
        vertex_order: new_vertices,
        transactions,
    })
}

/// Parse length-prefixed transactions from a vertex payload.
/// Format: [u32 len][bytes]... as written by ConsensusEngine::drain_pending_txs.
fn parse_payload_txs(payload: &[u8], out: &mut Vec<Vec<u8>>) {
    let mut offset = 0;
    while offset + 4 <= payload.len() {
        let len = u32::from_le_bytes(payload[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;
        if offset + len > payload.len() {
            break;
        }
        out.push(payload[offset..offset + len].to_vec());
        offset += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::DagBlock;
    use dendrite_storage::StateStore;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_db_path() -> std::path::PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("dendrite_ordering_test_{}_{}", pid, id))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let lock = path.with_extension("lock");
        let _ = std::fs::remove_file(lock);
    }

    fn encode_txs(txs: &[&[u8]]) -> Vec<u8> {
        let mut payload = Vec::new();
        for tx in txs {
            payload.extend_from_slice(&(tx.len() as u32).to_le_bytes());
            payload.extend_from_slice(tx);
        }
        payload
    }

    #[test]
    fn parse_payload_extracts_txs() {
        let payload = encode_txs(&[b"tx1", b"tx2", b"tx3"]);
        let mut txs = Vec::new();
        parse_payload_txs(&payload, &mut txs);
        assert_eq!(txs.len(), 3);
        assert_eq!(txs[0], b"tx1");
        assert_eq!(txs[1], b"tx2");
        assert_eq!(txs[2], b"tx3");
    }

    #[test]
    fn committed_batch_extracts_ordered_txs() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let g1 = DagBlock::genesis([1u8; 32], 1000);
        let g1h = g1.hash;
        dag.insert(g1).unwrap();

        let payload = encode_txs(&[b"hello", b"world"]);
        let b1 = DagBlock::new(1, [1u8; 32], vec![g1h], payload, 2000).unwrap();
        let b1h = b1.hash;
        dag.insert(b1).unwrap();

        let batch = extract_committed_batch(&dag, b1h, &HashSet::new()).unwrap();
        assert_eq!(batch.transactions.len(), 2);
        assert_eq!(batch.transactions[0], b"hello");
        assert_eq!(batch.transactions[1], b"world");
        assert!(batch.vertex_order.contains(&g1h));
        assert!(batch.vertex_order.contains(&b1h));

        cleanup(&path);
    }

    #[test]
    fn committed_batch_excludes_already_committed() {
        let path = test_db_path();
        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let mut dag = DagStore::new(store).unwrap();

        let g1 = DagBlock::genesis([1u8; 32], 1000);
        let g1h = g1.hash;
        dag.insert(g1).unwrap();

        let b1 = DagBlock::new(1, [1u8; 32], vec![g1h], encode_txs(&[b"tx1"]), 2000).unwrap();
        let b1h = b1.hash;
        dag.insert(b1).unwrap();

        let batch = extract_committed_batch(&dag, b1h, &HashSet::from([g1h])).unwrap();
        assert!(!batch.vertex_order.contains(&g1h));
        assert!(batch.vertex_order.contains(&b1h));

        cleanup(&path);
    }
}
