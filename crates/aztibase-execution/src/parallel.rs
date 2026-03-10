use aztibase_core::TxHash;

use crate::state::AccountState;

type Address = [u8; 32];

#[derive(Clone, Debug)]
pub struct TransferTx {
    pub hash: TxHash,
    pub from: Address,
    pub to: Address,
    pub value: u128,
    pub nonce: u64,
}

#[derive(Clone, Debug)]
pub enum TxStatus {
    Success,
    InsufficientBalance,
    NonceMismatch { expected: u64, got: u64 },
}

#[derive(Clone, Debug)]
pub struct TxReceipt {
    pub tx_hash: TxHash,
    pub status: TxStatus,
    pub gas_used: u64,
}

#[derive(Clone, Debug)]
pub struct BatchResult {
    pub receipts: Vec<TxReceipt>,
    pub state_root: [u8; 32],
}

/// Execute a batch of simple transfer transactions sequentially.
/// Each transaction is applied in order. Failed transactions still
/// increment the sender's nonce (Ethereum-style).
pub fn execute_transfers(state: &mut AccountState, txs: &[TransferTx]) -> BatchResult {
    let mut receipts = Vec::with_capacity(txs.len());

    for tx in txs {
        let sender_nonce = state.nonce(&tx.from);
        if tx.nonce != sender_nonce {
            state.increment_nonce(&tx.from);
            receipts.push(TxReceipt {
                tx_hash: tx.hash,
                status: TxStatus::NonceMismatch {
                    expected: sender_nonce,
                    got: tx.nonce,
                },
                gas_used: 21_000,
            });
            continue;
        }

        let sender_balance = state.balance(&tx.from);
        if sender_balance < tx.value {
            state.increment_nonce(&tx.from);
            receipts.push(TxReceipt {
                tx_hash: tx.hash,
                status: TxStatus::InsufficientBalance,
                gas_used: 21_000,
            });
            continue;
        }

        state.set_balance(&tx.from, sender_balance - tx.value);
        let receiver_balance = state.balance(&tx.to);
        state.set_balance(&tx.to, receiver_balance.saturating_add(tx.value));
        state.increment_nonce(&tx.from);

        receipts.push(TxReceipt {
            tx_hash: tx.hash,
            status: TxStatus::Success,
            gas_used: 21_000,
        });
    }

    let state_root = state.state_root();
    BatchResult {
        receipts,
        state_root,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_core::hash;

    fn make_tx(from: Address, to: Address, value: u128, nonce: u64) -> TransferTx {
        let mut preimage = Vec::new();
        preimage.extend_from_slice(&from);
        preimage.extend_from_slice(&to);
        preimage.extend_from_slice(&value.to_le_bytes());
        preimage.extend_from_slice(&nonce.to_le_bytes());
        TransferTx {
            hash: hash(&preimage),
            from,
            to,
            value,
            nonce,
        }
    }

    #[test]
    fn simple_transfer() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);

        let tx = make_tx(alice, bob, 300, 0);
        let result = execute_transfers(&mut state, &[tx]);

        assert_eq!(result.receipts.len(), 1);
        assert!(matches!(result.receipts[0].status, TxStatus::Success));
        assert_eq!(state.balance(&alice), 700);
        assert_eq!(state.balance(&bob), 300);
        assert_eq!(state.nonce(&alice), 1);
    }

    #[test]
    fn insufficient_balance() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 100);

        let tx = make_tx(alice, bob, 500, 0);
        let result = execute_transfers(&mut state, &[tx]);

        assert!(matches!(
            result.receipts[0].status,
            TxStatus::InsufficientBalance
        ));
        assert_eq!(state.balance(&alice), 100);
        assert_eq!(state.nonce(&alice), 1);
    }

    #[test]
    fn nonce_mismatch() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);

        let tx = make_tx(alice, bob, 100, 5);
        let result = execute_transfers(&mut state, &[tx]);

        assert!(matches!(
            result.receipts[0].status,
            TxStatus::NonceMismatch {
                expected: 0,
                got: 5
            }
        ));
        assert_eq!(state.balance(&alice), 1000);
    }

    #[test]
    fn batch_execution_sequential() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let carol = [3u8; 32];
        state.set_balance(&alice, 1000);

        let txs = vec![make_tx(alice, bob, 300, 0), make_tx(alice, carol, 200, 1)];
        let result = execute_transfers(&mut state, &txs);

        assert_eq!(result.receipts.len(), 2);
        assert!(matches!(result.receipts[0].status, TxStatus::Success));
        assert!(matches!(result.receipts[1].status, TxStatus::Success));
        assert_eq!(state.balance(&alice), 500);
        assert_eq!(state.balance(&bob), 300);
        assert_eq!(state.balance(&carol), 200);
        assert_eq!(state.nonce(&alice), 2);
    }

    #[test]
    fn batch_result_has_state_root() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);

        let tx = make_tx(alice, bob, 100, 0);
        let result = execute_transfers(&mut state, &[tx]);

        assert_ne!(result.state_root, [0u8; 32]);
        assert_eq!(result.state_root, state.state_root());
    }
}
