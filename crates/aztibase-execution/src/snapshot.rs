use serde::{Deserialize, Serialize};

use crate::state::{AccountState, AccountType};
use aztibase_core::hash;

const SNAPSHOT_VERSION: u8 = 2;
const MAX_SNAPSHOT_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

type Address = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccountEntry {
    address: Address,
    balance: u64,
    nonce: u64,
    code: Vec<u8>,
    storage: Vec<(Vec<u8>, Vec<u8>)>,
    account_type: u8,
    model_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    version: u8,
    pub batch_index: u64,
    pub state_root: [u8; 32],
    /// Committed block height at which this snapshot was taken.
    pub height: u64,
    /// Serialized finality certificate proving this snapshot is canonical.
    #[serde(default)]
    pub finality_certificate: Vec<u8>,
    accounts: Vec<AccountEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("snapshot too large: {0} bytes (max {MAX_SNAPSHOT_SIZE})")]
    TooLarge(usize),
    #[error("serialization failed: {0}")]
    Serialize(String),
    #[error("deserialization failed: {0}")]
    Deserialize(String),
    #[error("state root mismatch: expected {expected}, got {actual}")]
    RootMismatch { expected: String, actual: String },
    #[error("unsupported snapshot version: {0}")]
    UnsupportedVersion(u8),
}

pub fn create_snapshot(state: &AccountState, batch_index: u64) -> StateSnapshot {
    create_snapshot_with_finality(state, batch_index, batch_index, vec![])
}

pub fn create_snapshot_with_finality(
    state: &AccountState,
    batch_index: u64,
    height: u64,
    finality_certificate: Vec<u8>,
) -> StateSnapshot {
    let accounts = state
        .iter_accounts()
        .map(|(addr, acct)| AccountEntry {
            address: *addr,
            balance: acct.balance,
            nonce: acct.nonce,
            code: acct.code.clone(),
            storage: acct
                .storage
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            account_type: acct.account_type.discriminant(),
            model_id: acct.model_id.clone(),
        })
        .collect();

    StateSnapshot {
        version: SNAPSHOT_VERSION,
        batch_index,
        state_root: state.state_root(),
        height,
        finality_certificate,
        accounts,
    }
}

/// Compact header hash for snapshot identification.
pub fn snapshot_header_hash(snapshot: &StateSnapshot) -> [u8; 32] {
    let mut data = Vec::new();
    data.extend_from_slice(&snapshot.batch_index.to_le_bytes());
    data.extend_from_slice(&snapshot.height.to_le_bytes());
    data.extend_from_slice(&snapshot.state_root);
    hash(&data)
}

pub fn serialize_snapshot(snapshot: &StateSnapshot) -> Result<Vec<u8>, SnapshotError> {
    let data = bincode::serialize(snapshot).map_err(|e| SnapshotError::Serialize(e.to_string()))?;
    if data.len() > MAX_SNAPSHOT_SIZE {
        return Err(SnapshotError::TooLarge(data.len()));
    }
    Ok(data)
}

pub fn deserialize_snapshot(data: &[u8]) -> Result<StateSnapshot, SnapshotError> {
    if data.len() > MAX_SNAPSHOT_SIZE {
        return Err(SnapshotError::TooLarge(data.len()));
    }
    let snapshot: StateSnapshot =
        bincode::deserialize(data).map_err(|e| SnapshotError::Deserialize(e.to_string()))?;
    if snapshot.version != SNAPSHOT_VERSION {
        return Err(SnapshotError::UnsupportedVersion(snapshot.version));
    }
    Ok(snapshot)
}

pub fn apply_snapshot(snapshot: &StateSnapshot) -> Result<AccountState, SnapshotError> {
    let mut state = AccountState::new();

    for entry in &snapshot.accounts {
        state.set_balance(&entry.address, entry.balance);
        let acct = state.get_mut(&entry.address);
        acct.nonce = entry.nonce;
        if !entry.code.is_empty() {
            acct.code = entry.code.clone();
        }
        acct.account_type = match entry.account_type {
            0 => AccountType::EOA,
            1 => AccountType::Contract,
            2 => AccountType::AIAgent,
            _ => AccountType::EOA,
        };
        acct.model_id = entry.model_id.clone();
        for (k, v) in &entry.storage {
            acct.storage.insert(k.clone(), v.clone());
        }
    }

    let computed_root = state.state_root();
    if computed_root != snapshot.state_root {
        return Err(SnapshotError::RootMismatch {
            expected: hex_short(&snapshot.state_root),
            actual: hex_short(&computed_root),
        });
    }

    Ok(state)
}

/// Compute a BLAKE3 integrity hash over the raw snapshot bytes.
pub fn snapshot_hash(data: &[u8]) -> [u8; 32] {
    hash(data)
}

fn hex_short(bytes: &[u8; 32]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}..{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[28], bytes[29], bytes[30], bytes[31]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state() -> AccountState {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);
        state.set_balance(&bob, 500);
        state.increment_nonce(&alice);
        state.set_code(&alice, vec![0x00, 0x61, 0x73, 0x6d]);
        state.set_storage(&alice, b"key".to_vec(), b"value".to_vec());
        state
    }

    #[test]
    fn create_snapshot_captures_state() {
        let state = sample_state();
        let snap = create_snapshot(&state, 42);
        assert_eq!(snap.batch_index, 42);
        assert_eq!(snap.state_root, state.state_root());
        assert_eq!(snap.version, SNAPSHOT_VERSION);
        assert_eq!(snap.accounts.len(), 2);
    }

    #[test]
    fn serialization_roundtrip() {
        let state = sample_state();
        let snap = create_snapshot(&state, 10);
        let bytes = serialize_snapshot(&snap).unwrap();
        let restored = deserialize_snapshot(&bytes).unwrap();
        assert_eq!(restored.batch_index, 10);
        assert_eq!(restored.state_root, snap.state_root);
        assert_eq!(restored.accounts.len(), snap.accounts.len());
    }

    #[test]
    fn apply_snapshot_restores_state() {
        let state = sample_state();
        let snap = create_snapshot(&state, 5);
        let restored = apply_snapshot(&snap).unwrap();
        assert_eq!(restored.state_root(), state.state_root());
        assert_eq!(restored.balance(&[1u8; 32]), 1000);
        assert_eq!(restored.balance(&[2u8; 32]), 500);
        assert_eq!(restored.nonce(&[1u8; 32]), 1);
        assert_eq!(
            restored.code(&[1u8; 32]).unwrap(),
            &[0x00, 0x61, 0x73, 0x6d]
        );
        assert_eq!(restored.get_storage(&[1u8; 32], b"key").unwrap(), b"value");
    }

    #[test]
    fn apply_snapshot_detects_tampered_root() {
        let state = sample_state();
        let mut snap = create_snapshot(&state, 5);
        snap.state_root = [0xFF; 32];
        let err = apply_snapshot(&snap).unwrap_err();
        assert!(matches!(err, SnapshotError::RootMismatch { .. }));
    }

    #[test]
    fn empty_state_snapshot_roundtrip() {
        let state = AccountState::new();
        let snap = create_snapshot(&state, 0);
        let bytes = serialize_snapshot(&snap).unwrap();
        let restored_snap = deserialize_snapshot(&bytes).unwrap();
        let restored = apply_snapshot(&restored_snap).unwrap();
        assert_eq!(restored.state_root(), [0u8; 32]);
        assert_eq!(restored.account_count(), 0);
    }

    #[test]
    fn snapshot_hash_deterministic() {
        let state = sample_state();
        let snap = create_snapshot(&state, 1);
        let bytes = serialize_snapshot(&snap).unwrap();
        let h1 = snapshot_hash(&bytes);
        let h2 = snapshot_hash(&bytes);
        assert_eq!(h1, h2);
        assert_ne!(h1, [0u8; 32]);
    }

    #[test]
    fn rejects_unsupported_version() {
        let state = sample_state();
        let mut snap = create_snapshot(&state, 1);
        snap.version = 99;
        let bytes = bincode::serialize(&snap).unwrap();
        let err = deserialize_snapshot(&bytes).unwrap_err();
        assert!(matches!(err, SnapshotError::UnsupportedVersion(99)));
    }

    #[test]
    fn snapshot_with_finality_roundtrip() {
        let state = sample_state();
        let cert = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let snap = create_snapshot_with_finality(&state, 10, 42, cert.clone());
        assert_eq!(snap.height, 42);
        assert_eq!(snap.finality_certificate, cert);

        let bytes = serialize_snapshot(&snap).unwrap();
        let restored = deserialize_snapshot(&bytes).unwrap();
        assert_eq!(restored.height, 42);
        assert_eq!(restored.finality_certificate, cert);
        assert_eq!(restored.batch_index, 10);
    }

    #[test]
    fn snapshot_header_hash_deterministic() {
        let state = sample_state();
        let snap = create_snapshot(&state, 5);
        let h1 = snapshot_header_hash(&snap);
        let h2 = snapshot_header_hash(&snap);
        assert_eq!(h1, h2);
        assert_ne!(h1, [0u8; 32]);
    }

    #[test]
    fn snapshot_header_hash_varies_by_height() {
        let state = sample_state();
        let s1 = create_snapshot_with_finality(&state, 5, 10, vec![]);
        let s2 = create_snapshot_with_finality(&state, 5, 20, vec![]);
        assert_ne!(snapshot_header_hash(&s1), snapshot_header_hash(&s2));
    }
}
