use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use aztibase_core::hash;
use aztibase_execution::{
    AccountState, StateSnapshot, apply_snapshot, create_snapshot, deserialize_snapshot,
    flush_state, serialize_snapshot,
};
use aztibase_storage::StateStore;

const SYNC_VERSION: u8 = 1;
const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB per chunk
const MAX_CHUNKS: u32 = 64; // 64 MiB / 1 MiB = max 64 chunks

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SyncMessage {
    SnapshotRequest {
        version: u8,
        requester: [u8; 32],
    },
    SnapshotResponse {
        version: u8,
        batch_index: u64,
        state_root: [u8; 32],
        total_chunks: u32,
        chunk_index: u32,
        chunk_hash: [u8; 32],
        data: Vec<u8>,
    },
}

pub fn encode_sync_message(msg: &SyncMessage) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(msg).map_err(|e| e.to_string())
}

pub fn decode_sync_message(data: &[u8]) -> Result<SyncMessage, String> {
    let msg: SyncMessage = postcard::from_bytes(data).map_err(|e| e.to_string())?;
    match &msg {
        SyncMessage::SnapshotRequest { version, .. }
        | SyncMessage::SnapshotResponse { version, .. } => {
            if *version != SYNC_VERSION {
                return Err(format!("unsupported sync version: {version}"));
            }
        }
    }
    Ok(msg)
}

pub fn build_snapshot_request(requester: [u8; 32]) -> SyncMessage {
    SyncMessage::SnapshotRequest {
        version: SYNC_VERSION,
        requester,
    }
}

pub fn build_snapshot_response(
    state: &AccountState,
    batch_index: u64,
) -> Result<Vec<SyncMessage>, String> {
    let snapshot = create_snapshot(state, batch_index);
    let state_root = snapshot.state_root;
    let data = serialize_snapshot(&snapshot).map_err(|e| e.to_string())?;

    let total_chunks = data.len().div_ceil(CHUNK_SIZE) as u32;
    let mut messages = Vec::with_capacity(total_chunks as usize);

    for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
        messages.push(SyncMessage::SnapshotResponse {
            version: SYNC_VERSION,
            batch_index,
            state_root,
            total_chunks,
            chunk_index: i as u32,
            chunk_hash: hash(chunk),
            data: chunk.to_vec(),
        });
    }

    Ok(messages)
}

/// Reassembles chunked snapshot responses into a full StateSnapshot.
/// Validates chunk count, ordering, integrity hashes, and state root.
#[derive(Debug)]
pub struct SnapshotAssembler {
    expected_chunks: u32,
    batch_index: u64,
    state_root: [u8; 32],
    chunks: Vec<Option<Vec<u8>>>,
}

impl SnapshotAssembler {
    pub fn new(total_chunks: u32, batch_index: u64, state_root: [u8; 32]) -> Result<Self, String> {
        if total_chunks == 0 || total_chunks > MAX_CHUNKS {
            return Err(format!(
                "invalid total_chunks: {total_chunks} (must be 1..={MAX_CHUNKS})"
            ));
        }
        Ok(Self {
            expected_chunks: total_chunks,
            batch_index,
            state_root,
            chunks: vec![None; total_chunks as usize],
        })
    }

    pub fn add_chunk(
        &mut self,
        chunk_index: u32,
        chunk_hash: [u8; 32],
        data: Vec<u8>,
    ) -> Result<(), String> {
        if chunk_index >= self.expected_chunks {
            return Err(format!(
                "chunk index {chunk_index} out of range (total: {})",
                self.expected_chunks
            ));
        }
        if data.len() > CHUNK_SIZE + 1024 {
            return Err(format!(
                "chunk {chunk_index} too large: {} bytes (max {})",
                data.len(),
                CHUNK_SIZE + 1024
            ));
        }
        let actual_hash = hash(&data);
        if actual_hash != chunk_hash {
            return Err(format!("chunk {chunk_index} integrity check failed"));
        }
        self.chunks[chunk_index as usize] = Some(data);
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.chunks.iter().all(|c| c.is_some())
    }

    pub fn assemble(self) -> Result<StateSnapshot, String> {
        if !self.is_complete() {
            return Err("incomplete: not all chunks received".into());
        }
        let full_data: Vec<u8> = self.chunks.into_iter().flat_map(|c| c.unwrap()).collect();
        let snapshot = deserialize_snapshot(&full_data).map_err(|e| e.to_string())?;
        if snapshot.batch_index != self.batch_index {
            return Err(format!(
                "batch_index mismatch: expected {}, got {}",
                self.batch_index, snapshot.batch_index
            ));
        }
        if snapshot.state_root != self.state_root {
            return Err("state_root in snapshot doesn't match response header".into());
        }
        Ok(snapshot)
    }
}

/// Bootstrap a node's state from a received snapshot. Applies to both
/// in-memory AccountState and on-disk redb storage.
pub async fn bootstrap_from_snapshot(
    snapshot: &StateSnapshot,
    state: &Arc<RwLock<AccountState>>,
    store: Option<&StateStore>,
) -> Result<(), String> {
    let new_state = apply_snapshot(snapshot).map_err(|e| e.to_string())?;

    if let Some(store) = store {
        flush_state(store, &new_state).map_err(|e| format!("flush failed: {e}"))?;
    }

    let mut guard = state.write().await;
    *guard = new_state;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> AccountState {
        let mut state = AccountState::new();
        state.set_balance(&[1u8; 32], 1000);
        state.set_balance(&[2u8; 32], 500);
        state.increment_nonce(&[1u8; 32]);
        state.set_code(&[1u8; 32], vec![0x00, 0x61, 0x73, 0x6d]);
        state.set_storage(&[1u8; 32], b"key".to_vec(), b"val".to_vec());
        state
    }

    #[test]
    fn request_roundtrip() {
        let req = build_snapshot_request([1u8; 32]);
        let encoded = encode_sync_message(&req).unwrap();
        let decoded = decode_sync_message(&encoded).unwrap();
        match decoded {
            SyncMessage::SnapshotRequest { version, requester } => {
                assert_eq!(version, SYNC_VERSION);
                assert_eq!(requester, [1u8; 32]);
            }
            _ => panic!("expected SnapshotRequest"),
        }
    }

    #[test]
    fn response_single_chunk() {
        let state = sample_state();
        let messages = build_snapshot_response(&state, 42).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0] {
            SyncMessage::SnapshotResponse {
                batch_index,
                state_root,
                total_chunks,
                chunk_index,
                ..
            } => {
                assert_eq!(*batch_index, 42);
                assert_eq!(*state_root, state.state_root());
                assert_eq!(*total_chunks, 1);
                assert_eq!(*chunk_index, 0);
            }
            _ => panic!("expected SnapshotResponse"),
        }
    }

    #[test]
    fn response_message_roundtrip() {
        let state = sample_state();
        let messages = build_snapshot_response(&state, 10).unwrap();
        for msg in &messages {
            let encoded = encode_sync_message(msg).unwrap();
            let decoded = decode_sync_message(&encoded).unwrap();
            assert!(matches!(decoded, SyncMessage::SnapshotResponse { .. }));
        }
    }

    #[test]
    fn assembler_single_chunk() {
        let state = sample_state();
        let messages = build_snapshot_response(&state, 5).unwrap();
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            SyncMessage::SnapshotResponse {
                total_chunks,
                batch_index,
                state_root,
                chunk_index,
                chunk_hash,
                data,
                ..
            } => {
                let mut assembler =
                    SnapshotAssembler::new(*total_chunks, *batch_index, *state_root).unwrap();
                assert!(!assembler.is_complete());
                assembler
                    .add_chunk(*chunk_index, *chunk_hash, data.clone())
                    .unwrap();
                assert!(assembler.is_complete());

                let snapshot = assembler.assemble().unwrap();
                assert_eq!(snapshot.batch_index, 5);
                assert_eq!(snapshot.state_root, state.state_root());
            }
            _ => panic!("expected SnapshotResponse"),
        }
    }

    #[test]
    fn assembler_rejects_bad_chunk_hash() {
        let state = sample_state();
        let messages = build_snapshot_response(&state, 1).unwrap();

        match &messages[0] {
            SyncMessage::SnapshotResponse {
                total_chunks,
                batch_index,
                state_root,
                chunk_index,
                data,
                ..
            } => {
                let mut assembler =
                    SnapshotAssembler::new(*total_chunks, *batch_index, *state_root).unwrap();
                let bad_hash = [0xFF; 32];
                let err = assembler
                    .add_chunk(*chunk_index, bad_hash, data.clone())
                    .unwrap_err();
                assert!(err.contains("integrity"));
            }
            _ => panic!("expected SnapshotResponse"),
        }
    }

    #[test]
    fn assembler_rejects_out_of_range_index() {
        let mut assembler = SnapshotAssembler::new(1, 0, [0u8; 32]).unwrap();
        let err = assembler.add_chunk(5, [0u8; 32], vec![]).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn assembler_incomplete_fails() {
        let assembler = SnapshotAssembler::new(2, 0, [0u8; 32]).unwrap();
        assert!(!assembler.is_complete());
        let err = assembler.assemble().unwrap_err();
        assert!(err.contains("incomplete"));
    }

    #[test]
    fn assembler_rejects_excessive_chunk_count() {
        let err = SnapshotAssembler::new(1000, 0, [0u8; 32]).unwrap_err();
        assert!(err.contains("invalid total_chunks"));
    }

    #[test]
    fn assembler_rejects_zero_chunks() {
        let err = SnapshotAssembler::new(0, 0, [0u8; 32]).unwrap_err();
        assert!(err.contains("invalid total_chunks"));
    }

    #[test]
    fn assembler_rejects_oversized_chunk() {
        let mut assembler = SnapshotAssembler::new(1, 0, [0u8; 32]).unwrap();
        let big_data = vec![0u8; CHUNK_SIZE + 2048];
        let h = hash(&big_data);
        let err = assembler.add_chunk(0, h, big_data).unwrap_err();
        assert!(err.contains("too large"));
    }

    #[test]
    fn rejects_unsupported_sync_version() {
        let req = SyncMessage::SnapshotRequest {
            version: 99,
            requester: [0u8; 32],
        };
        let encoded = postcard::to_allocvec(&req).unwrap();
        let err = decode_sync_message(&encoded).unwrap_err();
        assert!(err.contains("unsupported"));
    }

    #[tokio::test]
    async fn bootstrap_applies_snapshot() {
        let source_state = sample_state();
        let snapshot = create_snapshot(&source_state, 42);

        let target_state = Arc::new(RwLock::new(AccountState::new()));
        bootstrap_from_snapshot(&snapshot, &target_state, None)
            .await
            .unwrap();

        let guard = target_state.read().await;
        assert_eq!(guard.balance(&[1u8; 32]), 1000);
        assert_eq!(guard.balance(&[2u8; 32]), 500);
        assert_eq!(guard.nonce(&[1u8; 32]), 1);
        assert_eq!(guard.state_root(), source_state.state_root());
    }

    #[tokio::test]
    async fn bootstrap_with_persistence() {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CTR: AtomicU32 = AtomicU32::new(0);
        let id = CTR.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("aztibase_sync_test_{}_{}", std::process::id(), id));

        let source_state = sample_state();
        let snapshot = create_snapshot(&source_state, 10);

        let store = StateStore::open(path.to_str().unwrap()).unwrap();
        let target_state = Arc::new(RwLock::new(AccountState::new()));

        bootstrap_from_snapshot(&snapshot, &target_state, Some(&store))
            .await
            .unwrap();

        // Verify in-memory state
        let guard = target_state.read().await;
        assert_eq!(guard.state_root(), source_state.state_root());
        drop(guard);

        // Verify persisted state
        let loaded = aztibase_execution::load_state(&store).unwrap();
        assert_eq!(loaded.state_root(), source_state.state_root());
        assert_eq!(loaded.balance(&[1u8; 32]), 1000);

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("lock"));
    }

    #[tokio::test]
    async fn bootstrap_rejects_tampered_snapshot() {
        let source_state = sample_state();
        let mut snapshot = create_snapshot(&source_state, 1);
        snapshot.state_root = [0xFF; 32];

        let target_state = Arc::new(RwLock::new(AccountState::new()));
        let err = bootstrap_from_snapshot(&snapshot, &target_state, None)
            .await
            .unwrap_err();
        assert!(err.contains("mismatch"));
    }

    #[test]
    fn end_to_end_snapshot_sync() {
        let source = sample_state();
        let response_msgs = build_snapshot_response(&source, 99).unwrap();

        // Simulate network: encode → decode all messages
        let decoded_msgs: Vec<SyncMessage> = response_msgs
            .iter()
            .map(|m| {
                let bytes = encode_sync_message(m).unwrap();
                decode_sync_message(&bytes).unwrap()
            })
            .collect();

        // Reassemble
        let first = &decoded_msgs[0];
        let (total, bi, sr) = match first {
            SyncMessage::SnapshotResponse {
                total_chunks,
                batch_index,
                state_root,
                ..
            } => (*total_chunks, *batch_index, *state_root),
            _ => panic!("expected SnapshotResponse"),
        };

        let mut assembler = SnapshotAssembler::new(total, bi, sr).unwrap();
        for msg in &decoded_msgs {
            if let SyncMessage::SnapshotResponse {
                chunk_index,
                chunk_hash,
                data,
                ..
            } = msg
            {
                assembler
                    .add_chunk(*chunk_index, *chunk_hash, data.clone())
                    .unwrap();
            }
        }

        let snapshot = assembler.assemble().unwrap();
        let restored = apply_snapshot(&snapshot).unwrap();
        assert_eq!(restored.state_root(), source.state_root());
        assert_eq!(restored.balance(&[1u8; 32]), 1000);
    }
}
