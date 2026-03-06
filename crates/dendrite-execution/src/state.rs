use std::collections::BTreeMap;

use dendrite_core::hash;

type Address = [u8; 32];

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub balance: u64,
    pub nonce: u64,
    pub code: Vec<u8>,
    pub storage: BTreeMap<Vec<u8>, Vec<u8>>,
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
        self.get_mut(address).code = code;
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
                buf.extend_from_slice(&hash(&acct.code));
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
}
