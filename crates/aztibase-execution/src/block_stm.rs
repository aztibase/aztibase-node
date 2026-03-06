use std::collections::{BTreeMap, HashMap};

type Address = [u8; 32];
type TxIndex = usize;

#[derive(Clone, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum StateKey {
    Balance(Address),
    Nonce(Address),
    Code(Address),
    Storage(Address, Vec<u8>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum StateValue {
    Balance(u64),
    Nonce(u64),
    Code(Vec<u8>),
    StorageSlot(Vec<u8>),
    Deleted,
}

/// Read-set entry: records what value a tx observed for a key.
#[derive(Clone, Debug)]
pub struct ReadEntry {
    pub key: StateKey,
    pub version: Option<TxIndex>,
    pub value: Option<StateValue>,
}

pub type ReadSet = Vec<ReadEntry>;
pub type WriteSet = Vec<(StateKey, StateValue)>;

/// Multi-versioned memory. Each tx_index can write a versioned value for any key.
/// Reads return the latest version written by a tx with index < reader_index.
#[derive(Default)]
pub struct MVMemory {
    data: HashMap<StateKey, BTreeMap<TxIndex, StateValue>>,
}

impl MVMemory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a write from tx at `tx_index`.
    pub fn write(&mut self, tx_index: TxIndex, key: StateKey, value: StateValue) {
        self.data.entry(key).or_default().insert(tx_index, value);
    }

    /// Apply a full write set from a tx.
    pub fn apply_write_set(&mut self, tx_index: TxIndex, write_set: &WriteSet) {
        for (key, value) in write_set {
            self.write(tx_index, key.clone(), value.clone());
        }
    }

    /// Read the latest version of `key` written by a tx with index < `reader_index`.
    /// Returns (writer_tx_index, value) or None if no prior tx wrote this key.
    pub fn read(&self, key: &StateKey, reader_index: TxIndex) -> Option<(TxIndex, &StateValue)> {
        let versions = self.data.get(key)?;
        versions
            .range(..reader_index)
            .next_back()
            .map(|(&idx, val)| (idx, val))
    }

    /// Delete all entries written by `tx_index` (used before re-execution).
    pub fn delete_writes(&mut self, tx_index: TxIndex) {
        for versions in self.data.values_mut() {
            versions.remove(&tx_index);
        }
    }
}

/// View into MVMemory for a specific tx, tracking reads automatically.
pub struct MVView<'a> {
    mv: &'a MVMemory,
    tx_index: TxIndex,
    read_set: Vec<ReadEntry>,
    base_state: &'a crate::state::AccountState,
}

impl<'a> MVView<'a> {
    pub fn new(
        mv: &'a MVMemory,
        tx_index: TxIndex,
        base_state: &'a crate::state::AccountState,
    ) -> Self {
        Self {
            mv,
            tx_index,
            read_set: Vec::new(),
            base_state,
        }
    }

    pub fn read_balance(&mut self, addr: &Address) -> u64 {
        let key = StateKey::Balance(*addr);
        match self.mv.read(&key, self.tx_index) {
            Some((version, StateValue::Balance(b))) => {
                let val = *b;
                self.read_set.push(ReadEntry {
                    key,
                    version: Some(version),
                    value: Some(StateValue::Balance(val)),
                });
                val
            }
            _ => {
                let val = self.base_state.balance(addr);
                self.read_set.push(ReadEntry {
                    key,
                    version: None,
                    value: Some(StateValue::Balance(val)),
                });
                val
            }
        }
    }

    pub fn read_nonce(&mut self, addr: &Address) -> u64 {
        let key = StateKey::Nonce(*addr);
        match self.mv.read(&key, self.tx_index) {
            Some((version, StateValue::Nonce(n))) => {
                let val = *n;
                self.read_set.push(ReadEntry {
                    key,
                    version: Some(version),
                    value: Some(StateValue::Nonce(val)),
                });
                val
            }
            _ => {
                let val = self.base_state.nonce(addr);
                self.read_set.push(ReadEntry {
                    key,
                    version: None,
                    value: Some(StateValue::Nonce(val)),
                });
                val
            }
        }
    }

    pub fn into_read_set(self) -> ReadSet {
        self.read_set
    }
}

// ── Scheduler ──────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxStatus {
    ReadyToExecute,
    Executing,
    Executed,
    Validated,
    Aborting,
}

#[derive(Debug)]
pub enum SchedulerTask {
    Execute(TxIndex),
    Validate(TxIndex),
    Done,
}

/// Coordinates execution and validation of transactions in Block-STM order.
pub struct Scheduler {
    statuses: Vec<TxStatus>,
    validation_idx: usize,
}

impl Scheduler {
    pub fn new(num_txs: usize) -> Self {
        Self {
            statuses: vec![TxStatus::ReadyToExecute; num_txs],
            validation_idx: 0,
        }
    }

    pub fn next_task(&mut self) -> SchedulerTask {
        // First, check for txs that need validation
        while self.validation_idx < self.statuses.len() {
            if self.statuses[self.validation_idx] == TxStatus::Executed {
                let idx = self.validation_idx;
                self.validation_idx += 1;
                return SchedulerTask::Validate(idx);
            } else if self.statuses[self.validation_idx] == TxStatus::Validated {
                self.validation_idx += 1;
                continue;
            } else {
                break;
            }
        }

        // Then, check for txs ready to execute
        for i in 0..self.statuses.len() {
            if self.statuses[i] == TxStatus::ReadyToExecute {
                self.statuses[i] = TxStatus::Executing;
                return SchedulerTask::Execute(i);
            }
        }

        // Check if all are validated
        if self.statuses.iter().all(|s| *s == TxStatus::Validated) {
            return SchedulerTask::Done;
        }

        // There are txs still executing — in single-threaded mode this shouldn't happen,
        // but return Done to avoid infinite loops.
        SchedulerTask::Done
    }

    pub fn finish_execution(&mut self, tx_index: TxIndex) {
        if tx_index < self.statuses.len() {
            self.statuses[tx_index] = TxStatus::Executed;
        }
    }

    pub fn finish_validation(&mut self, tx_index: TxIndex, valid: bool) {
        if tx_index >= self.statuses.len() {
            return;
        }
        if valid {
            self.statuses[tx_index] = TxStatus::Validated;
        } else {
            self.statuses[tx_index] = TxStatus::ReadyToExecute;
            // All txs after this one that were validated must be re-validated
            for i in (tx_index + 1)..self.statuses.len() {
                if self.statuses[i] == TxStatus::Validated {
                    self.statuses[i] = TxStatus::Executed;
                }
            }
            // Reset validation pointer to the failed tx
            if tx_index < self.validation_idx {
                self.validation_idx = tx_index;
            }
        }
    }

    pub fn status(&self, tx_index: TxIndex) -> Option<TxStatus> {
        self.statuses.get(tx_index).copied()
    }

    pub fn num_txs(&self) -> usize {
        self.statuses.len()
    }
}

// ── Block-STM Executor ─────────────────────────────────────────────

/// Output from executing a single transfer via Block-STM.
#[derive(Clone, Debug)]
pub struct TxOutput {
    pub tx_index: TxIndex,
    pub success: bool,
    pub gas_used: u64,
    pub error: Option<String>,
}

/// Block-STM executor for transfer transactions.
/// Executes optimistically, validates read sets, re-executes on conflict.
pub struct BlockSTMExecutor {
    mv_memory: MVMemory,
    scheduler: Scheduler,
    read_sets: Vec<ReadSet>,
    write_sets: Vec<WriteSet>,
    outputs: Vec<Option<TxOutput>>,
}

impl BlockSTMExecutor {
    /// Execute a batch of transfers using Block-STM.
    /// Returns outputs in the same order as input transactions.
    pub fn execute(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
    ) -> Vec<TxOutput> {
        Self::execute_full(txs, base_state).0
    }

    /// Execute a batch and return both outputs and per-tx write sets.
    /// The write sets can be passed to `apply_block_stm_to_state` to update state.
    pub fn execute_full(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
    ) -> (Vec<TxOutput>, Vec<WriteSet>) {
        let n = txs.len();
        if n == 0 {
            return (Vec::new(), Vec::new());
        }

        let mut executor = Self {
            mv_memory: MVMemory::new(),
            scheduler: Scheduler::new(n),
            read_sets: vec![Vec::new(); n],
            write_sets: vec![Vec::new(); n],
            outputs: vec![None; n],
        };

        loop {
            match executor.scheduler.next_task() {
                SchedulerTask::Execute(idx) => {
                    executor.execute_tx(idx, txs, base_state);
                    executor.scheduler.finish_execution(idx);
                }
                SchedulerTask::Validate(idx) => {
                    let valid = executor.validate_tx(idx, base_state);
                    executor.scheduler.finish_validation(idx, valid);
                }
                SchedulerTask::Done => break,
            }
        }

        let outputs = executor
            .outputs
            .into_iter()
            .enumerate()
            .map(|(i, o)| {
                o.unwrap_or(TxOutput {
                    tx_index: i,
                    success: false,
                    gas_used: 0,
                    error: Some("not executed".into()),
                })
            })
            .collect();

        (outputs, executor.write_sets)
    }

    fn execute_tx(
        &mut self,
        idx: TxIndex,
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
    ) {
        // Clear previous writes for this tx (if re-executing)
        self.mv_memory.delete_writes(idx);
        self.read_sets[idx].clear();
        self.write_sets[idx].clear();

        let tx = &txs[idx];
        let mut view = MVView::new(&self.mv_memory, idx, base_state);

        let sender_nonce = view.read_nonce(&tx.from);
        if tx.nonce != sender_nonce {
            let ws = vec![(
                StateKey::Nonce(tx.from),
                StateValue::Nonce(sender_nonce + 1),
            )];
            self.read_sets[idx] = view.into_read_set();
            self.mv_memory.apply_write_set(idx, &ws);
            self.write_sets[idx] = ws;
            self.outputs[idx] = Some(TxOutput {
                tx_index: idx,
                success: false,
                gas_used: 21_000,
                error: Some(format!(
                    "nonce mismatch: expected {sender_nonce}, got {}",
                    tx.nonce
                )),
            });
            return;
        }

        let sender_balance = view.read_balance(&tx.from);
        if sender_balance < tx.value {
            let ws = vec![(
                StateKey::Nonce(tx.from),
                StateValue::Nonce(sender_nonce + 1),
            )];
            self.read_sets[idx] = view.into_read_set();
            self.mv_memory.apply_write_set(idx, &ws);
            self.write_sets[idx] = ws;
            self.outputs[idx] = Some(TxOutput {
                tx_index: idx,
                success: false,
                gas_used: 21_000,
                error: Some("insufficient balance".into()),
            });
            return;
        }

        let receiver_balance = view.read_balance(&tx.to);
        let read_set = view.into_read_set();

        let ws = vec![
            (
                StateKey::Balance(tx.from),
                StateValue::Balance(sender_balance - tx.value),
            ),
            (
                StateKey::Balance(tx.to),
                StateValue::Balance(receiver_balance.saturating_add(tx.value)),
            ),
            (
                StateKey::Nonce(tx.from),
                StateValue::Nonce(sender_nonce + 1),
            ),
        ];

        self.read_sets[idx] = read_set;
        self.mv_memory.apply_write_set(idx, &ws);
        self.write_sets[idx] = ws;
        self.outputs[idx] = Some(TxOutput {
            tx_index: idx,
            success: true,
            gas_used: 21_000,
            error: None,
        });
    }

    fn validate_tx(&self, idx: TxIndex, base_state: &crate::state::AccountState) -> bool {
        for entry in &self.read_sets[idx] {
            let current = self.mv_memory.read(&entry.key, idx);
            match (&entry.version, current) {
                // Both read from base state
                (None, None) => {
                    let base_val = self.read_base(&entry.key, base_state);
                    if entry.value != base_val {
                        return false;
                    }
                }
                // Read from same version
                (Some(v1), Some((v2, val))) if *v1 == v2 => {
                    if entry.value.as_ref() != Some(val) {
                        return false;
                    }
                }
                // Version changed — read set invalidated
                _ => return false,
            }
        }
        true
    }

    fn read_base(
        &self,
        key: &StateKey,
        base_state: &crate::state::AccountState,
    ) -> Option<StateValue> {
        match key {
            StateKey::Balance(addr) => Some(StateValue::Balance(base_state.balance(addr))),
            StateKey::Nonce(addr) => Some(StateValue::Nonce(base_state.nonce(addr))),
            StateKey::Code(addr) => base_state.code(addr).map(|c| StateValue::Code(c.to_vec())),
            StateKey::Storage(addr, k) => base_state
                .get_storage(addr, k)
                .map(|v| StateValue::StorageSlot(v.to_vec())),
        }
    }
}

/// Apply Block-STM outputs to an AccountState to produce the final state.
/// The write sets are applied in tx order to produce the correct final state.
pub fn apply_block_stm_to_state(state: &mut crate::state::AccountState, write_sets: &[WriteSet]) {
    for ws in write_sets {
        for (key, value) in ws {
            match (key, value) {
                (StateKey::Balance(addr), StateValue::Balance(b)) => {
                    state.set_balance(addr, *b);
                }
                (StateKey::Nonce(addr), StateValue::Nonce(n)) => {
                    let acct = state.get_mut(addr);
                    acct.nonce = *n;
                }
                (StateKey::Code(addr), StateValue::Code(c)) => {
                    state.set_code(addr, c.clone());
                }
                (StateKey::Storage(addr, k), StateValue::StorageSlot(v)) => {
                    state.set_storage(addr, k.clone(), v.clone());
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parallel::TransferTx;
    use crate::state::AccountState;
    use aztibase_core::hash;

    fn make_tx(from: Address, to: Address, value: u64, nonce: u64) -> TransferTx {
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

    // ── MVMemory tests ──────────────────────────────────────────

    #[test]
    fn mv_write_and_read() {
        let mut mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(500));

        let result = mv.read(&key, 1);
        assert_eq!(result.map(|(idx, _)| idx), Some(0));
        match result.unwrap().1 {
            StateValue::Balance(b) => assert_eq!(*b, 500),
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn mv_read_returns_latest_before_reader() {
        let mut mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(100));
        mv.write(2, key.clone(), StateValue::Balance(200));
        mv.write(5, key.clone(), StateValue::Balance(300));

        // Reader at index 3 should see tx 2's write
        let result = mv.read(&key, 3).unwrap();
        assert_eq!(result.0, 2);
        assert_eq!(result.1, &StateValue::Balance(200));

        // Reader at index 1 should see tx 0's write
        let result = mv.read(&key, 1).unwrap();
        assert_eq!(result.0, 0);
        assert_eq!(result.1, &StateValue::Balance(100));

        // Reader at index 0 should see nothing
        assert!(mv.read(&key, 0).is_none());
    }

    #[test]
    fn mv_no_cross_contamination() {
        let mut mv = MVMemory::new();
        let key_a = StateKey::Balance([1u8; 32]);
        let key_b = StateKey::Balance([2u8; 32]);
        mv.write(0, key_a.clone(), StateValue::Balance(100));
        mv.write(0, key_b.clone(), StateValue::Balance(200));

        assert_eq!(mv.read(&key_a, 1).unwrap().1, &StateValue::Balance(100));
        assert_eq!(mv.read(&key_b, 1).unwrap().1, &StateValue::Balance(200));
    }

    #[test]
    fn mv_delete_writes() {
        let mut mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(100));
        mv.write(1, key.clone(), StateValue::Balance(200));

        mv.delete_writes(0);

        // Reader at index 1 should see nothing (tx 0's write deleted)
        assert!(mv.read(&key, 1).is_none());
        // Reader at index 2 should see tx 1's write
        assert_eq!(mv.read(&key, 2).unwrap().1, &StateValue::Balance(200));
    }

    // ── Scheduler tests ─────────────────────────────────────────

    #[test]
    fn scheduler_executes_in_order() {
        let mut sched = Scheduler::new(3);
        assert!(matches!(sched.next_task(), SchedulerTask::Execute(0)));
        sched.finish_execution(0);
        assert!(matches!(sched.next_task(), SchedulerTask::Validate(0)));
    }

    #[test]
    fn scheduler_validates_after_execute() {
        let mut sched = Scheduler::new(2);

        // Execute both
        assert!(matches!(sched.next_task(), SchedulerTask::Execute(0)));
        sched.finish_execution(0);
        assert!(matches!(sched.next_task(), SchedulerTask::Validate(0)));
        sched.finish_validation(0, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Execute(1)));
        sched.finish_execution(1);
        assert!(matches!(sched.next_task(), SchedulerTask::Validate(1)));
        sched.finish_validation(1, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Done));
    }

    #[test]
    fn scheduler_reexecutes_on_validation_failure() {
        let mut sched = Scheduler::new(3);

        // Scheduler interleaves execute/validate. Run through naturally.
        // Execute 0, validate 0 (pass), execute 1, validate 1 (fail),
        // re-execute 1, validate 1 (pass), execute/validate 2
        assert!(matches!(sched.next_task(), SchedulerTask::Execute(0)));
        sched.finish_execution(0);

        assert!(matches!(sched.next_task(), SchedulerTask::Validate(0)));
        sched.finish_validation(0, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Execute(1)));
        sched.finish_execution(1);

        assert!(matches!(sched.next_task(), SchedulerTask::Validate(1)));
        sched.finish_validation(1, false);

        // Tx 1 marked for re-execution
        assert!(matches!(sched.next_task(), SchedulerTask::Execute(1)));
        sched.finish_execution(1);

        assert!(matches!(sched.next_task(), SchedulerTask::Validate(1)));
        sched.finish_validation(1, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Execute(2)));
        sched.finish_execution(2);

        assert!(matches!(sched.next_task(), SchedulerTask::Validate(2)));
        sched.finish_validation(2, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Done));
    }

    // ── Block-STM Executor tests ────────────────────────────────

    #[test]
    fn block_stm_independent_transfers() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let carol = [3u8; 32];
        state.set_balance(&alice, 1000);
        state.set_balance(&bob, 1000);

        // Two independent transfers: alice→carol, bob→carol
        let txs = vec![make_tx(alice, carol, 300, 0), make_tx(bob, carol, 200, 0)];

        let outputs = BlockSTMExecutor::execute(&txs, &state);
        assert_eq!(outputs.len(), 2);
        assert!(outputs[0].success);
        assert!(outputs[1].success);
    }

    #[test]
    fn block_stm_conflicting_transfers() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let carol = [3u8; 32];
        state.set_balance(&alice, 1000);

        // Two transfers from same sender: alice→bob (nonce 0), alice→carol (nonce 1)
        let txs = vec![make_tx(alice, bob, 300, 0), make_tx(alice, carol, 200, 1)];

        let outputs = BlockSTMExecutor::execute(&txs, &state);
        assert_eq!(outputs.len(), 2);
        assert!(outputs[0].success);
        assert!(outputs[1].success);
    }

    #[test]
    fn block_stm_matches_sequential() {
        let mut state_seq = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let carol = [3u8; 32];
        state_seq.set_balance(&alice, 5000);
        state_seq.set_balance(&bob, 3000);

        let txs = vec![
            make_tx(alice, bob, 500, 0),
            make_tx(bob, carol, 200, 0),
            make_tx(alice, carol, 100, 1),
        ];

        // Sequential execution
        let state_stm = state_seq.clone();
        let seq_result = crate::parallel::execute_transfers(&mut state_seq, &txs);

        // Block-STM execution
        let stm_outputs = BlockSTMExecutor::execute(&txs, &state_stm);

        // Apply Block-STM write sets won't work directly — we need to apply outputs.
        // Instead, compare outputs match.
        for (i, (seq_receipt, stm_output)) in seq_result
            .receipts
            .iter()
            .zip(stm_outputs.iter())
            .enumerate()
        {
            assert_eq!(
                matches!(seq_receipt.status, crate::parallel::TxStatus::Success),
                stm_output.success,
                "tx {i} success mismatch"
            );
            assert_eq!(
                seq_receipt.gas_used, stm_output.gas_used,
                "tx {i} gas mismatch"
            );
        }
    }

    #[test]
    fn block_stm_insufficient_balance() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 100);

        let txs = vec![make_tx(alice, bob, 500, 0)];
        let outputs = BlockSTMExecutor::execute(&txs, &state);
        assert!(!outputs[0].success);
        assert!(outputs[0].error.as_ref().unwrap().contains("insufficient"));
    }

    #[test]
    fn block_stm_nonce_mismatch() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);

        let txs = vec![make_tx(alice, bob, 100, 5)];
        let outputs = BlockSTMExecutor::execute(&txs, &state);
        assert!(!outputs[0].success);
        assert!(outputs[0].error.as_ref().unwrap().contains("nonce"));
    }

    #[test]
    fn block_stm_empty_batch() {
        let state = AccountState::new();
        let outputs = BlockSTMExecutor::execute(&[], &state);
        assert!(outputs.is_empty());
    }

    #[test]
    fn block_stm_state_application() {
        let mut base = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        base.set_balance(&alice, 1000);

        let txs = vec![make_tx(alice, bob, 300, 0)];

        // Run Block-STM
        let mut executor = BlockSTMExecutor {
            mv_memory: MVMemory::new(),
            scheduler: Scheduler::new(1),
            read_sets: vec![Vec::new()],
            write_sets: vec![Vec::new()],
            outputs: vec![None],
        };

        loop {
            match executor.scheduler.next_task() {
                SchedulerTask::Execute(idx) => {
                    executor.execute_tx(idx, &txs, &base);
                    executor.scheduler.finish_execution(idx);
                }
                SchedulerTask::Validate(idx) => {
                    let valid = executor.validate_tx(idx, &base);
                    executor.scheduler.finish_validation(idx, valid);
                }
                SchedulerTask::Done => break,
            }
        }

        // Apply write sets to state
        let mut state = base.clone();
        apply_block_stm_to_state(&mut state, &executor.write_sets);

        assert_eq!(state.balance(&alice), 700);
        assert_eq!(state.balance(&bob), 300);
        assert_eq!(state.nonce(&alice), 1);

        // Compare with sequential
        let mut state_seq = base;
        crate::parallel::execute_transfers(&mut state_seq, &txs);
        assert_eq!(state.state_root(), state_seq.state_root());
    }

    // ── MVView tests ────────────────────────────────────────────

    #[test]
    fn mv_view_reads_from_base_state() {
        let mv = MVMemory::new();
        let mut state = AccountState::new();
        state.set_balance(&[1u8; 32], 500);

        let mut view = MVView::new(&mv, 0, &state);
        assert_eq!(view.read_balance(&[1u8; 32]), 500);

        let rs = view.into_read_set();
        assert_eq!(rs.len(), 1);
        assert!(rs[0].version.is_none()); // Read from base
    }

    #[test]
    fn mv_view_reads_from_mv_memory() {
        let mut mv = MVMemory::new();
        mv.write(0, StateKey::Balance([1u8; 32]), StateValue::Balance(999));
        let state = AccountState::new();

        let mut view = MVView::new(&mv, 1, &state);
        assert_eq!(view.read_balance(&[1u8; 32]), 999);

        let rs = view.into_read_set();
        assert_eq!(rs[0].version, Some(0)); // Read from tx 0
    }
}
