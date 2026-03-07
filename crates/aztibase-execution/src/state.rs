use std::collections::BTreeMap;

use aztibase_core::commitment::{MerkleProof, Side, StateCommitment, StateProof};
use aztibase_core::hash;

type Address = [u8; 32];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AccountType {
    #[default]
    EOA,
    Contract,
    AIAgent,
}

impl AccountType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::EOA => "EOA",
            AccountType::Contract => "Contract",
            AccountType::AIAgent => "AIAgent",
        }
    }

    pub fn discriminant(&self) -> u8 {
        match self {
            AccountType::EOA => 0,
            AccountType::Contract => 1,
            AccountType::AIAgent => 2,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
    pub code: Vec<u8>,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
    pub account_type: AccountType,
    pub model_id: Option<String>,
}

/// In-memory account state store. Provides get/set operations on accounts
/// and deterministic state root computation.
#[derive(Clone, Debug, Default)]
pub struct AccountState {
    accounts: BTreeMap<Address, Account>,
}

impl AccountState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }

    pub fn iter_accounts(&self) -> impl Iterator<Item = (&Address, &Account)> {
        self.accounts.iter()
    }

    pub fn get_mut(&mut self, address: &Address) -> &mut Account {
        self.accounts.entry(*address).or_default()
    }

    pub fn balance(&self, address: &Address) -> u64 {
        self.accounts.get(address).map_or(0, |a| a.balance)
    }

    pub fn nonce(&self, address: &Address) -> u64 {
        self.accounts.get(address).map_or(0, |a| a.nonce)
    }

    pub fn set_balance(&mut self, address: &Address, balance: u64) {
        self.get_mut(address).balance = balance;
    }

    pub fn increment_nonce(&mut self, address: &Address) {
        self.get_mut(address).nonce += 1;
    }

    pub fn set_code(&mut self, address: &Address, code: Vec<u8>) {
        let acct = self.get_mut(address);
        acct.code = code;
        if acct.account_type == AccountType::EOA {
            acct.account_type = AccountType::Contract;
        }
    }

    pub fn account_type(&self, address: &Address) -> AccountType {
        self.accounts
            .get(address)
            .map_or(AccountType::EOA, |a| a.account_type.clone())
    }

    pub fn set_account_type(&mut self, address: &Address, account_type: AccountType) {
        self.get_mut(address).account_type = account_type;
    }

    pub fn model_id(&self, address: &Address) -> Option<&str> {
        self.accounts
            .get(address)
            .and_then(|a| a.model_id.as_deref())
    }

    pub fn set_model_id(&mut self, address: &Address, model_id: String) {
        self.get_mut(address).model_id = Some(model_id);
    }

    pub fn code(&self, address: &Address) -> Option<&[u8]> {
        self.accounts
            .get(address)
            .filter(|a| !a.code.is_empty())
            .map(|a| a.code.as_slice())
    }

    pub fn set_storage(&mut self, address: &Address, key: Vec<u8>, value: Vec<u8>) {
        self.get_mut(address).storage.insert(key, value);
    }

    pub fn get_storage(&self, address: &Address, key: &[u8]) -> Option<&[u8]> {
        self.accounts
            .get(address)?
            .storage
            .get(key)
            .map(|v| v.as_slice())
    }

    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Compute a deterministic state root by hashing all accounts in sorted order.
    /// Uses a binary Merkle tree built from BLAKE3 hashes of serialized account data.
    pub fn state_root(&self) -> [u8; 32] {
        if self.accounts.is_empty() {
            return [0u8; 32];
        }

        let leaves: Vec<[u8; 32]> = self
            .accounts
            .iter()
            .map(|(addr, acct)| {
                let mut buf = Vec::new();
                buf.extend_from_slice(addr);
                buf.extend_from_slice(&acct.balance.to_le_bytes());
                buf.extend_from_slice(&acct.nonce.to_le_bytes());
                buf.push(acct.account_type.discriminant());
                buf.extend_from_slice(&hash(&acct.code));
                if let Some(ref mid) = acct.model_id {
                    buf.extend_from_slice(&hash(mid.as_bytes()));
                }
                for (k, v) in &acct.storage {
                    buf.extend_from_slice(&hash(k));
                    buf.extend_from_slice(&hash(v));
                }
                hash(&buf)
            })
            .collect();

        merkle_root(&leaves)
    }
}

fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut layer: Vec<[u8; 32]> = leaves.to_vec();
    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        for pair in layer.chunks(2) {
            if pair.len() == 2 {
                let mut combined = [0u8; 64];
                combined[..32].copy_from_slice(&pair[0]);
                combined[32..].copy_from_slice(&pair[1]);
                next.push(hash(&combined));
            } else {
                next.push(pair[0]);
            }
        }
        layer = next;
    }
    layer[0]
}

/// Binary Merkle tree commitment scheme using BLAKE3.
pub struct MerkleCommitment;

impl StateCommitment for MerkleCommitment {
    fn commit(&self, leaves: &[[u8; 32]]) -> [u8; 32] {
        merkle_root(leaves)
    }

    fn prove(&self, leaves: &[[u8; 32]], index: usize) -> Option<StateProof> {
        if index >= leaves.len() || leaves.is_empty() {
            return None;
        }
        if leaves.len() == 1 {
            return Some(StateProof::Merkle(MerkleProof {
                leaf_hash: leaves[0],
                siblings: vec![],
            }));
        }

        let mut siblings = Vec::new();
        let mut layer: Vec<[u8; 32]> = leaves.to_vec();
        let mut pos = index;

        while layer.len() > 1 {
            let sibling_pos = if pos.is_multiple_of(2) {
                pos + 1
            } else {
                pos - 1
            };
            if sibling_pos < layer.len() {
                let side = if pos.is_multiple_of(2) {
                    Side::Right
                } else {
                    Side::Left
                };
                siblings.push((layer[sibling_pos], side));
            }

            let mut next = Vec::with_capacity(layer.len().div_ceil(2));
            for pair in layer.chunks(2) {
                if pair.len() == 2 {
                    let mut combined = [0u8; 64];
                    combined[..32].copy_from_slice(&pair[0]);
                    combined[32..].copy_from_slice(&pair[1]);
                    next.push(hash(&combined));
                } else {
                    next.push(pair[0]);
                }
            }
            layer = next;
            pos /= 2;
        }

        Some(StateProof::Merkle(MerkleProof {
            leaf_hash: leaves[index],
            siblings,
        }))
    }

    fn verify(&self, root: &[u8; 32], leaf: &[u8; 32], proof: &StateProof) -> bool {
        let StateProof::Merkle(mp) = proof else {
            return false;
        };
        if mp.leaf_hash != *leaf {
            return false;
        }

        let mut current = *leaf;
        for (sibling, side) in &mp.siblings {
            let mut combined = [0u8; 64];
            match side {
                Side::Right => {
                    combined[..32].copy_from_slice(&current);
                    combined[32..].copy_from_slice(sibling);
                }
                Side::Left => {
                    combined[..32].copy_from_slice(sibling);
                    combined[32..].copy_from_slice(&current);
                }
            }
            current = hash(&combined);
        }
        current == *root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_balance_is_zero() {
        let state = AccountState::new();
        assert_eq!(state.balance(&[1u8; 32]), 0);
    }

    #[test]
    fn set_and_get_balance() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.set_balance(&addr, 1000);
        assert_eq!(state.balance(&addr), 1000);
    }

    #[test]
    fn increment_nonce() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.increment_nonce(&addr);
        state.increment_nonce(&addr);
        assert_eq!(state.nonce(&addr), 2);
    }

    #[test]
    fn set_and_get_code() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        assert!(state.code(&addr).is_none());
        state.set_code(&addr, vec![0x00, 0x61, 0x73, 0x6d]);
        assert_eq!(state.code(&addr).unwrap(), &[0x00, 0x61, 0x73, 0x6d]);
    }

    #[test]
    fn contract_storage_roundtrip() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.set_storage(&addr, b"key".to_vec(), b"value".to_vec());
        assert_eq!(state.get_storage(&addr, b"key").unwrap(), b"value");
        assert!(state.get_storage(&addr, b"missing").is_none());
    }

    #[test]
    fn state_root_deterministic() {
        let mut s1 = AccountState::new();
        let mut s2 = AccountState::new();
        let addr = [1u8; 32];

        s1.set_balance(&addr, 100);
        s2.set_balance(&addr, 100);
        assert_eq!(s1.state_root(), s2.state_root());
    }

    #[test]
    fn state_root_changes_on_mutation() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.set_balance(&addr, 100);
        let root1 = state.state_root();
        state.set_balance(&addr, 200);
        let root2 = state.state_root();
        assert_ne!(root1, root2);
    }

    #[test]
    fn empty_state_root() {
        let state = AccountState::new();
        assert_eq!(state.state_root(), [0u8; 32]);
    }

    #[test]
    fn default_account_type_is_eoa() {
        let state = AccountState::new();
        assert_eq!(state.account_type(&[1u8; 32]), AccountType::EOA);
    }

    #[test]
    fn set_code_promotes_to_contract() {
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.set_code(&addr, vec![0x00, 0x61]);
        assert_eq!(state.account_type(&addr), AccountType::Contract);
    }

    #[test]
    fn ai_agent_account_type_roundtrip() {
        let mut state = AccountState::new();
        let addr = [2u8; 32];
        state.set_account_type(&addr, AccountType::AIAgent);
        state.set_model_id(&addr, "sentiment_v1".into());
        assert_eq!(state.account_type(&addr), AccountType::AIAgent);
        assert_eq!(state.model_id(&addr), Some("sentiment_v1"));
    }

    #[test]
    fn account_type_affects_state_root() {
        let mut s1 = AccountState::new();
        let mut s2 = AccountState::new();
        let addr = [3u8; 32];

        s1.set_balance(&addr, 100);
        s2.set_balance(&addr, 100);
        assert_eq!(s1.state_root(), s2.state_root());

        s2.set_account_type(&addr, AccountType::AIAgent);
        assert_ne!(s1.state_root(), s2.state_root());
    }

    #[test]
    fn trait_object_dispatch() {
        let mc: Box<dyn StateCommitment> = Box::new(MerkleCommitment);
        let leaves = vec![hash(b"a"), hash(b"b"), hash(b"c")];
        let root = mc.commit(&leaves);
        assert_eq!(root, merkle_root(&leaves));
    }

    #[test]
    fn merkle_proof_roundtrip() {
        let mc = MerkleCommitment;
        let leaves: Vec<[u8; 32]> = (0..8u8).map(|i| hash(&[i])).collect();
        let root = mc.commit(&leaves);

        for i in 0..leaves.len() {
            let proof = mc.prove(&leaves, i).unwrap();
            assert!(
                mc.verify(&root, &leaves[i], &proof),
                "Proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn merkle_verify_tampered() {
        let mc = MerkleCommitment;
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| hash(&[i])).collect();
        let root = mc.commit(&leaves);
        let proof = mc.prove(&leaves, 0).unwrap();

        let fake_leaf = hash(b"tampered");
        assert!(!mc.verify(&root, &fake_leaf, &proof));
    }
}
