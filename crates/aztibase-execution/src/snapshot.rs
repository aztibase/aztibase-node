use serde::{Deserialize, Serialize};

use crate::agent::AgentPolicyStore;
use crate::chain_params::ChainParams;
use crate::governance::GovernanceStore;
use crate::l2_bridge::{BridgeEscrow, BridgeWithdrawProofs, L2AnchorStore, L2Registry};
use crate::staking::StakingStore;
use crate::state::{AccountState, AccountType};
use crate::tokenomics::EmissionTracker;
use aztibase_core::hash;

const SNAPSHOT_VERSION: u8 = 3;
const MAX_SNAPSHOT_SIZE: usize = 128 * 1024 * 1024; // 128 MiB (protocol stores need headroom)

type Address = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AccountEntry {
    address: Address,
    balance: u128,
    nonce: u64,
    code: Vec<u8>,
    storage: Vec<(Vec<u8>, Vec<u8>)>,
    account_type: u8,
    model_id: Option<String>,
}

/// Serialized protocol stores bundled into a snapshot for full node bootstrap.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ProtocolStoreBundle {
    pub staking: StakingStore,
    pub governance: GovernanceStore,
    pub emission: EmissionTracker,
    pub chain_params: ChainParams,
    pub agent_policies: AgentPolicyStore,
    pub l2_registry: L2Registry,
    pub l2_anchors: L2AnchorStore,
    pub bridge_escrow: BridgeEscrow,
    pub bridge_proofs: BridgeWithdrawProofs,
    pub base_fee: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSnapshot {
    version: u8,
    pub batch_index: u64,
    pub state_root: [u8; 32],
    pub height: u64,
    #[serde(default)]
    pub finality_certificate: Vec<u8>,
    accounts: Vec<AccountEntry>,
    #[serde(default)]
    pub protocol_stores: Option<ProtocolStoreBundle>,
}

impl StateSnapshot {
    pub fn version(&self) -> u8 {
        self.version
    }
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
    let accounts = collect_accounts(state);
    StateSnapshot {
        version: SNAPSHOT_VERSION,
        batch_index,
        state_root: state.state_root(),
        height,
        finality_certificate,
        accounts,
        protocol_stores: None,
    }
}

/// Create a full snapshot including all protocol stores for complete node bootstrap.
#[allow(clippy::too_many_arguments)]
pub fn create_full_snapshot(
    state: &AccountState,
    batch_index: u64,
    height: u64,
    finality_certificate: Vec<u8>,
    bundle: ProtocolStoreBundle,
) -> StateSnapshot {
    let accounts = collect_accounts(state);
    StateSnapshot {
        version: SNAPSHOT_VERSION,
        batch_index,
        state_root: state.state_root(),
        height,
        finality_certificate,
        accounts,
        protocol_stores: Some(bundle),
    }
}

fn collect_accounts(state: &AccountState) -> Vec<AccountEntry> {
    state
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
        .collect()
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
    let data =
        postcard::to_allocvec(snapshot).map_err(|e| SnapshotError::Serialize(e.to_string()))?;
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
        postcard::from_bytes(data).map_err(|e| SnapshotError::Deserialize(e.to_string()))?;
    if snapshot.version != SNAPSHOT_VERSION {
        return Err(SnapshotError::UnsupportedVersion(snapshot.version));
    }
    Ok(snapshot)
}

/// Apply a full snapshot, restoring both AccountState and protocol stores.
pub fn apply_full_snapshot(
    snapshot: &StateSnapshot,
) -> Result<(AccountState, ProtocolStoreBundle), SnapshotError> {
    let state = apply_snapshot(snapshot)?;
    let bundle = snapshot.protocol_stores.clone().unwrap_or_default();
    Ok((state, bundle))
}

/// Write a snapshot to a file with a BLAKE3 integrity header.
/// File format: [32-byte BLAKE3 hash] [snapshot bytes]
pub fn write_snapshot_file(
    path: &std::path::Path,
    snapshot: &StateSnapshot,
) -> Result<(), SnapshotError> {
    let data = serialize_snapshot(snapshot)?;
    let integrity = snapshot_hash(&data);
    let mut file_data = Vec::with_capacity(32 + data.len());
    file_data.extend_from_slice(&integrity);
    file_data.extend_from_slice(&data);
    std::fs::write(path, &file_data).map_err(|e| SnapshotError::Serialize(e.to_string()))
}

/// Read a snapshot from a file and verify BLAKE3 integrity.
pub fn read_snapshot_file(path: &std::path::Path) -> Result<StateSnapshot, SnapshotError> {
    let file_data = std::fs::read(path).map_err(|e| SnapshotError::Deserialize(e.to_string()))?;
    if file_data.len() < 33 {
        return Err(SnapshotError::Deserialize("file too small".into()));
    }
    let expected_hash: [u8; 32] = file_data[..32]
        .try_into()
        .map_err(|_| SnapshotError::Deserialize("bad header".into()))?;
    let snapshot_data = &file_data[32..];
    let actual_hash = snapshot_hash(snapshot_data);
    if expected_hash != actual_hash {
        return Err(SnapshotError::RootMismatch {
            expected: hex_short(&expected_hash),
            actual: hex_short(&actual_hash),
        });
    }
    deserialize_snapshot(snapshot_data)
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
        let bytes = postcard::to_allocvec(&snap).unwrap();
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

    #[test]
    fn full_snapshot_includes_protocol_stores() {
        let state = sample_state();
        let mut bundle = ProtocolStoreBundle::default();
        bundle.base_fee = 42;
        let snap = create_full_snapshot(&state, 100, 100, vec![], bundle);
        assert!(snap.protocol_stores.is_some());
        assert_eq!(snap.protocol_stores.as_ref().unwrap().base_fee, 42);

        let bytes = serialize_snapshot(&snap).unwrap();
        let restored = deserialize_snapshot(&bytes).unwrap();
        assert!(restored.protocol_stores.is_some());
        assert_eq!(restored.protocol_stores.as_ref().unwrap().base_fee, 42);
    }

    #[test]
    fn apply_full_snapshot_restores_bundle() {
        let state = sample_state();
        let mut bundle = ProtocolStoreBundle::default();
        bundle.base_fee = 99;
        let snap = create_full_snapshot(&state, 50, 50, vec![], bundle);
        let (restored_state, restored_bundle) = apply_full_snapshot(&snap).unwrap();
        assert_eq!(restored_state.state_root(), state.state_root());
        assert_eq!(restored_bundle.base_fee, 99);
    }

    #[test]
    fn snapshot_without_protocol_stores_defaults() {
        let state = sample_state();
        let snap = create_snapshot(&state, 10);
        assert!(snap.protocol_stores.is_none());
        let (_, bundle) = apply_full_snapshot(&snap).unwrap();
        assert_eq!(bundle.base_fee, 0);
    }

    #[test]
    fn snapshot_file_roundtrip() {
        let state = sample_state();
        let mut bundle = ProtocolStoreBundle::default();
        bundle.base_fee = 77;
        let snap = create_full_snapshot(&state, 25, 25, vec![0xAB], bundle);

        let path =
            std::env::temp_dir().join(format!("aztibase_snapfile_test_{}", std::process::id()));
        write_snapshot_file(&path, &snap).unwrap();
        let loaded = read_snapshot_file(&path).unwrap();
        assert_eq!(loaded.batch_index, 25);
        assert_eq!(loaded.protocol_stores.as_ref().unwrap().base_fee, 77);
        assert_eq!(loaded.finality_certificate, vec![0xAB]);

        let (restored_state, _) = apply_full_snapshot(&loaded).unwrap();
        assert_eq!(restored_state.state_root(), state.state_root());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn snapshot_file_detects_corruption() {
        let state = sample_state();
        let snap = create_snapshot(&state, 1);
        let path =
            std::env::temp_dir().join(format!("aztibase_snapcorrupt_test_{}", std::process::id()));
        write_snapshot_file(&path, &snap).unwrap();

        let mut data = std::fs::read(&path).unwrap();
        data[0] ^= 0xFF; // corrupt the hash
        std::fs::write(&path, &data).unwrap();

        let err = read_snapshot_file(&path).unwrap_err();
        assert!(matches!(err, SnapshotError::RootMismatch { .. }));

        let _ = std::fs::remove_file(&path);
    }
}
