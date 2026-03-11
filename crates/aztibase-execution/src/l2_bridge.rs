use std::collections::HashMap;

use serde::{Deserialize, Serialize};

type Address = [u8; 32];

const MAX_ANCHOR_HISTORY: usize = 10;
const MAX_REGISTERED_L2S: usize = 256;
const MAX_SEQUENCER_SET_SIZE: usize = 32;

/// Number of L1 batches before an anchored state root is considered finalized.
pub const BRIDGE_FINALITY_BATCHES: u64 = 100;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct L2Registration {
    pub owner: Address,
    pub l2_chain_id: [u8; 32],
    pub name: String,
    pub sequencer_set: Vec<Address>,
    pub bridge_address: Address,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct L2Anchor {
    pub sequencer: Address,
    pub state_root: [u8; 32],
    pub batch_data_hash: [u8; 32],
    pub l2_block_start: u64,
    pub l2_block_end: u64,
    pub l1_batch_index: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BridgeError {
    L2NotRegistered,
    L2AlreadyRegistered,
    TooManyL2s,
    SequencerSetTooLarge,
    UnauthorizedSequencer,
    StaleBlockRange,
    InsufficientBalance,
    StateRootMismatch,
    AnchorNotFinalized,
    DuplicateWithdrawProof,
    AmountZero,
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::L2NotRegistered => write!(f, "L2 chain not registered"),
            BridgeError::L2AlreadyRegistered => write!(f, "L2 chain already registered"),
            BridgeError::TooManyL2s => write!(f, "too many L2 registrations"),
            BridgeError::SequencerSetTooLarge => write!(f, "sequencer set too large"),
            BridgeError::UnauthorizedSequencer => write!(f, "sender not in sequencer set"),
            BridgeError::StaleBlockRange => {
                write!(f, "L2 block range is not monotonically increasing")
            }
            BridgeError::InsufficientBalance => {
                write!(f, "insufficient balance for bridge deposit")
            }
            BridgeError::StateRootMismatch => {
                write!(f, "state root does not match latest finalized anchor")
            }
            BridgeError::AnchorNotFinalized => write!(f, "anchor not yet finalized"),
            BridgeError::DuplicateWithdrawProof => write!(f, "withdrawal proof already used"),
            BridgeError::AmountZero => write!(f, "bridge amount must be non-zero"),
        }
    }
}

impl std::error::Error for BridgeError {}

/// On-chain registry of L2 chains. Governance-gated registration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct L2Registry {
    chains: HashMap<[u8; 32], L2Registration>,
}

impl L2Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, reg: L2Registration) -> Result<(), BridgeError> {
        if self.chains.contains_key(&reg.l2_chain_id) {
            return Err(BridgeError::L2AlreadyRegistered);
        }
        if self.chains.len() >= MAX_REGISTERED_L2S {
            return Err(BridgeError::TooManyL2s);
        }
        if reg.sequencer_set.len() > MAX_SEQUENCER_SET_SIZE {
            return Err(BridgeError::SequencerSetTooLarge);
        }
        self.chains.insert(reg.l2_chain_id, reg);
        Ok(())
    }

    pub fn get(&self, l2_chain_id: &[u8; 32]) -> Option<&L2Registration> {
        self.chains.get(l2_chain_id)
    }

    pub fn list(&self) -> Vec<&L2Registration> {
        self.chains.values().collect()
    }

    pub fn is_sequencer(&self, l2_chain_id: &[u8; 32], addr: &Address) -> bool {
        self.chains
            .get(l2_chain_id)
            .is_some_and(|reg| reg.sequencer_set.contains(addr))
    }
}

/// Stores anchored L2 state roots on L1, keeping the last N anchors per chain.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct L2AnchorStore {
    anchors: HashMap<[u8; 32], Vec<L2Anchor>>,
}

impl L2AnchorStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn anchor(&mut self, l2_chain_id: &[u8; 32], anchor: L2Anchor) -> Result<(), BridgeError> {
        let history = self.anchors.entry(*l2_chain_id).or_default();
        if let Some(latest) = history.last()
            && anchor.l2_block_start <= latest.l2_block_end
        {
            return Err(BridgeError::StaleBlockRange);
        }
        history.push(anchor);
        if history.len() > MAX_ANCHOR_HISTORY {
            history.remove(0);
        }
        Ok(())
    }

    pub fn latest(&self, l2_chain_id: &[u8; 32]) -> Option<&L2Anchor> {
        self.anchors.get(l2_chain_id).and_then(|h| h.last())
    }

    /// Check if the latest anchor for an L2 is finalized (enough L1 batches have passed).
    pub fn is_finalized(&self, l2_chain_id: &[u8; 32], current_l1_batch: u64) -> bool {
        self.latest(l2_chain_id).is_some_and(|a| {
            current_l1_batch.saturating_sub(a.l1_batch_index) >= BRIDGE_FINALITY_BATCHES
        })
    }

    /// Return the latest finalized anchor if one exists.
    pub fn latest_finalized(
        &self,
        l2_chain_id: &[u8; 32],
        current_l1_batch: u64,
    ) -> Option<&L2Anchor> {
        self.anchors.get(l2_chain_id).and_then(|history| {
            history.iter().rev().find(|a| {
                current_l1_batch.saturating_sub(a.l1_batch_index) >= BRIDGE_FINALITY_BATCHES
            })
        })
    }
}

/// Tracks locked AZTB per (l2_chain_id, depositor) for bridge escrow.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BridgeEscrow {
    balances: HashMap<([u8; 32], Address), u128>,
}

impl BridgeEscrow {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lock(&mut self, l2_chain_id: &[u8; 32], depositor: &Address, amount: u128) {
        let key = (*l2_chain_id, *depositor);
        let entry = self.balances.entry(key).or_insert(0);
        *entry = entry.saturating_add(amount);
    }

    pub fn unlock(
        &mut self,
        l2_chain_id: &[u8; 32],
        withdrawer: &Address,
        amount: u128,
    ) -> Result<(), BridgeError> {
        let key = (*l2_chain_id, *withdrawer);
        let entry = self.balances.entry(key).or_insert(0);
        if *entry < amount {
            return Err(BridgeError::InsufficientBalance);
        }
        *entry -= amount;
        Ok(())
    }

    pub fn balance(&self, l2_chain_id: &[u8; 32], account: &Address) -> u128 {
        self.balances
            .get(&(*l2_chain_id, *account))
            .copied()
            .unwrap_or(0)
    }
}

/// Tracks used withdrawal proofs to prevent double-spend.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BridgeWithdrawProofs {
    used: HashMap<[u8; 32], Address>,
}

impl BridgeWithdrawProofs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a proof as used. Returns error if already used.
    pub fn mark_used(
        &mut self,
        proof_hash: [u8; 32],
        processor: Address,
    ) -> Result<(), BridgeError> {
        if self.used.contains_key(&proof_hash) {
            return Err(BridgeError::DuplicateWithdrawProof);
        }
        self.used.insert(proof_hash, processor);
        Ok(())
    }

    pub fn is_used(&self, proof_hash: &[u8; 32]) -> bool {
        self.used.contains_key(proof_hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain_id_a() -> [u8; 32] {
        let mut id = [0u8; 32];
        id[0] = 0xAA;
        id
    }

    fn make_registration(chain_id: [u8; 32]) -> L2Registration {
        L2Registration {
            owner: [1u8; 32],
            l2_chain_id: chain_id,
            name: "Test L2".into(),
            sequencer_set: vec![[2u8; 32], [3u8; 32]],
            bridge_address: [4u8; 32],
        }
    }

    #[test]
    fn register_l2_happy_path() {
        let mut registry = L2Registry::new();
        let reg = make_registration(chain_id_a());
        assert!(registry.register(reg.clone()).is_ok());
        assert_eq!(registry.get(&chain_id_a()).unwrap().name, "Test L2");
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn register_l2_duplicate_rejected() {
        let mut registry = L2Registry::new();
        let reg = make_registration(chain_id_a());
        registry.register(reg.clone()).unwrap();
        assert_eq!(
            registry.register(reg),
            Err(BridgeError::L2AlreadyRegistered)
        );
    }

    #[test]
    fn sequencer_authorization() {
        let mut registry = L2Registry::new();
        let reg = make_registration(chain_id_a());
        registry.register(reg).unwrap();
        assert!(registry.is_sequencer(&chain_id_a(), &[2u8; 32]));
        assert!(registry.is_sequencer(&chain_id_a(), &[3u8; 32]));
        assert!(!registry.is_sequencer(&chain_id_a(), &[5u8; 32]));
    }

    #[test]
    fn anchor_happy_path() {
        let mut store = L2AnchorStore::new();
        let anchor = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xBB; 32],
            batch_data_hash: [0xCC; 32],
            l2_block_start: 0,
            l2_block_end: 10,
            l1_batch_index: 50,
        };
        assert!(store.anchor(&chain_id_a(), anchor).is_ok());
        let latest = store.latest(&chain_id_a()).unwrap();
        assert_eq!(latest.state_root, [0xBB; 32]);
        assert_eq!(latest.l2_block_end, 10);
    }

    #[test]
    fn anchor_stale_range_rejected() {
        let mut store = L2AnchorStore::new();
        let a1 = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xBB; 32],
            batch_data_hash: [0xCC; 32],
            l2_block_start: 0,
            l2_block_end: 10,
            l1_batch_index: 50,
        };
        store.anchor(&chain_id_a(), a1).unwrap();
        let a2 = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xDD; 32],
            batch_data_hash: [0xEE; 32],
            l2_block_start: 5, // overlaps with previous
            l2_block_end: 15,
            l1_batch_index: 51,
        };
        assert_eq!(
            store.anchor(&chain_id_a(), a2),
            Err(BridgeError::StaleBlockRange)
        );
    }

    #[test]
    fn anchor_finalization() {
        let mut store = L2AnchorStore::new();
        let anchor = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xBB; 32],
            batch_data_hash: [0xCC; 32],
            l2_block_start: 0,
            l2_block_end: 10,
            l1_batch_index: 50,
        };
        store.anchor(&chain_id_a(), anchor).unwrap();
        assert!(!store.is_finalized(&chain_id_a(), 100));
        assert!(store.is_finalized(&chain_id_a(), 150));
        assert!(store.is_finalized(&chain_id_a(), 200));
    }

    #[test]
    fn bridge_deposit_and_withdraw() {
        let mut escrow = BridgeEscrow::new();
        let depositor = [5u8; 32];
        let cid = chain_id_a();

        escrow.lock(&cid, &depositor, 1000);
        assert_eq!(escrow.balance(&cid, &depositor), 1000);

        escrow.lock(&cid, &depositor, 500);
        assert_eq!(escrow.balance(&cid, &depositor), 1500);

        assert!(escrow.unlock(&cid, &depositor, 800).is_ok());
        assert_eq!(escrow.balance(&cid, &depositor), 700);

        assert_eq!(
            escrow.unlock(&cid, &depositor, 1000),
            Err(BridgeError::InsufficientBalance)
        );
    }

    #[test]
    fn withdraw_proof_dedup() {
        let mut proofs = BridgeWithdrawProofs::new();
        let proof_hash = [0xFF; 32];
        let processor = [6u8; 32];

        assert!(!proofs.is_used(&proof_hash));
        assert!(proofs.mark_used(proof_hash, processor).is_ok());
        assert!(proofs.is_used(&proof_hash));
        assert_eq!(
            proofs.mark_used(proof_hash, processor),
            Err(BridgeError::DuplicateWithdrawProof)
        );
    }

    #[test]
    fn unregistered_l2_sequencer_check() {
        let registry = L2Registry::new();
        assert!(!registry.is_sequencer(&chain_id_a(), &[2u8; 32]));
    }

    #[test]
    fn latest_finalized_anchor() {
        let mut store = L2AnchorStore::new();
        let cid = chain_id_a();
        let a1 = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xAA; 32],
            batch_data_hash: [0xBB; 32],
            l2_block_start: 0,
            l2_block_end: 10,
            l1_batch_index: 50,
        };
        store.anchor(&cid, a1).unwrap();
        let a2 = L2Anchor {
            sequencer: [2u8; 32],
            state_root: [0xCC; 32],
            batch_data_hash: [0xDD; 32],
            l2_block_start: 11,
            l2_block_end: 20,
            l1_batch_index: 140,
        };
        store.anchor(&cid, a2).unwrap();

        // At batch 200: a1 is finalized (200-50=150 >= 100), a2 is not (200-140=60 < 100)
        let finalized = store.latest_finalized(&cid, 200).unwrap();
        assert_eq!(finalized.state_root, [0xAA; 32]);

        // At batch 250: both finalized, returns most recent finalized
        let finalized = store.latest_finalized(&cid, 250).unwrap();
        assert_eq!(finalized.state_root, [0xCC; 32]);
    }
}
