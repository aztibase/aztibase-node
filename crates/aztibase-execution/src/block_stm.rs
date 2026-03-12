use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

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
    Balance(u128),
    Nonce(u64),
    Code(Vec<u8>),
    StorageSlot(Vec<u8>),
    Deleted,
}

#[derive(Clone, Debug)]
pub struct ReadEntry {
    pub key: StateKey,
    pub version: Option<TxIndex>,
    pub value: Option<StateValue>,
}

pub type ReadSet = Vec<ReadEntry>;
pub type WriteSet = Vec<(StateKey, StateValue)>;

/// Thread-safe multi-versioned memory. Each tx_index can write a versioned value
/// for any key. Reads return the latest version written by a tx with index < reader_index.
pub struct MVMemory {
    data: Mutex<HashMap<StateKey, BTreeMap<TxIndex, StateValue>>>,
}

impl Default for MVMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl MVMemory {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(HashMap::new()),
        }
    }

    pub fn write(&self, tx_index: TxIndex, key: StateKey, value: StateValue) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        data.entry(key).or_default().insert(tx_index, value);
    }

    pub fn apply_write_set(&self, tx_index: TxIndex, write_set: &WriteSet) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        for (key, value) in write_set {
            data.entry(key.clone())
                .or_default()
                .insert(tx_index, value.clone());
        }
    }

    /// Read the latest version of `key` written by a tx with index < `reader_index`.
    /// Returns (writer_tx_index, value) or None if no prior tx wrote this key.
    pub fn read(&self, key: &StateKey, reader_index: TxIndex) -> Option<(TxIndex, StateValue)> {
        let data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        let versions = data.get(key)?;
        versions
            .range(..reader_index)
            .next_back()
            .map(|(&idx, val)| (idx, val.clone()))
    }

    pub fn delete_writes(&self, tx_index: TxIndex) {
        let mut data = self.data.lock().unwrap_or_else(|e| e.into_inner());
        for versions in data.values_mut() {
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

    pub fn read_balance(&mut self, addr: &Address) -> u128 {
        let key = StateKey::Balance(*addr);
        match self.mv.read(&key, self.tx_index) {
            Some((version, StateValue::Balance(b))) => {
                self.read_set.push(ReadEntry {
                    key,
                    version: Some(version),
                    value: Some(StateValue::Balance(b)),
                });
                b
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
                self.read_set.push(ReadEntry {
                    key,
                    version: Some(version),
                    value: Some(StateValue::Nonce(n)),
                });
                n
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
    Wait,
}

/// Thread-safe scheduler for Block-STM execution and validation ordering.
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

        // There are txs still executing — workers should wait and retry
        SchedulerTask::Wait
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
            let all_prior_valid = (0..tx_index).all(|j| self.statuses[j] == TxStatus::Validated);
            if all_prior_valid {
                self.statuses[tx_index] = TxStatus::Validated;
            } else {
                self.statuses[tx_index] = TxStatus::Executed;
                if tx_index < self.validation_idx {
                    self.validation_idx = tx_index;
                }
            }
        } else {
            self.statuses[tx_index] = TxStatus::ReadyToExecute;
            for i in (tx_index + 1)..self.statuses.len() {
                if self.statuses[i] == TxStatus::Validated {
                    self.statuses[i] = TxStatus::Executed;
                }
            }
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

#[derive(Clone, Debug)]
pub struct TxOutput {
    pub tx_index: TxIndex,
    pub success: bool,
    pub gas_used: u64,
    pub error: Option<String>,
}

/// Block-STM executor for transfer transactions.
/// Supports both single-threaded and multi-threaded (rayon) execution.
pub struct BlockSTMExecutor;

impl BlockSTMExecutor {
    pub fn execute(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
    ) -> Vec<TxOutput> {
        Self::execute_full(txs, base_state).0
    }

    /// Execute a batch and return both outputs and per-tx write sets.
    pub fn execute_full(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
    ) -> (Vec<TxOutput>, Vec<WriteSet>) {
        let n = txs.len();
        if n == 0 {
            return (Vec::new(), Vec::new());
        }

        let mv_memory = Arc::new(MVMemory::new());
        let scheduler = Arc::new(Mutex::new(Scheduler::new(n)));
        let read_sets: Vec<Mutex<ReadSet>> = (0..n).map(|_| Mutex::new(Vec::new())).collect();
        let write_sets: Vec<Mutex<WriteSet>> = (0..n).map(|_| Mutex::new(Vec::new())).collect();
        let outputs: Vec<Mutex<Option<TxOutput>>> = (0..n).map(|_| Mutex::new(None)).collect();

        let read_sets = Arc::new(read_sets);
        let write_sets = Arc::new(write_sets);
        let outputs = Arc::new(outputs);

        // Use rayon for parallel execution when batch is large enough
        if n >= 4 {
            Self::execute_parallel(
                txs,
                base_state,
                &mv_memory,
                &scheduler,
                &read_sets,
                &write_sets,
                &outputs,
            );
        } else {
            Self::execute_sequential(
                txs,
                base_state,
                &mv_memory,
                &scheduler,
                &read_sets,
                &write_sets,
                &outputs,
            );
        }

        let final_outputs: Vec<TxOutput> = Arc::try_unwrap(outputs)
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, m)| {
                m.into_inner().unwrap().unwrap_or(TxOutput {
                    tx_index: i,
                    success: false,
                    gas_used: 0,
                    error: Some("not executed".into()),
                })
            })
            .collect();

        let final_write_sets: Vec<WriteSet> = Arc::try_unwrap(write_sets)
            .unwrap()
            .into_iter()
            .map(|m| m.into_inner().unwrap())
            .collect();

        (final_outputs, final_write_sets)
    }

    fn execute_sequential(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
        mv_memory: &Arc<MVMemory>,
        scheduler: &Arc<Mutex<Scheduler>>,
        read_sets: &Arc<Vec<Mutex<ReadSet>>>,
        write_sets: &Arc<Vec<Mutex<WriteSet>>>,
        outputs: &Arc<Vec<Mutex<Option<TxOutput>>>>,
    ) {
        loop {
            let task = scheduler
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .next_task();
            match task {
                SchedulerTask::Execute(idx) => {
                    Self::execute_tx(
                        idx, txs, base_state, mv_memory, read_sets, write_sets, outputs,
                    );
                    scheduler
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .finish_execution(idx);
                }
                SchedulerTask::Validate(idx) => {
                    let valid = Self::validate_tx(idx, base_state, mv_memory, read_sets);
                    scheduler
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .finish_validation(idx, valid);
                }
                SchedulerTask::Done => break,
                SchedulerTask::Wait => break,
            }
        }
    }

    fn execute_parallel(
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
        mv_memory: &Arc<MVMemory>,
        scheduler: &Arc<Mutex<Scheduler>>,
        read_sets: &Arc<Vec<Mutex<ReadSet>>>,
        write_sets: &Arc<Vec<Mutex<WriteSet>>>,
        outputs: &Arc<Vec<Mutex<Option<TxOutput>>>>,
    ) {
        rayon::scope(|s| {
            let num_workers = rayon::current_num_threads().min(txs.len());
            for _ in 0..num_workers {
                let mv = Arc::clone(mv_memory);
                let sched = Arc::clone(scheduler);
                let rs = Arc::clone(read_sets);
                let ws = Arc::clone(write_sets);
                let outs = Arc::clone(outputs);

                s.spawn(move |_| {
                    loop {
                        let task = sched.lock().unwrap_or_else(|e| e.into_inner()).next_task();
                        match task {
                            SchedulerTask::Execute(idx) => {
                                Self::execute_tx(idx, txs, base_state, &mv, &rs, &ws, &outs);
                                sched
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .finish_execution(idx);
                            }
                            SchedulerTask::Validate(idx) => {
                                let valid = Self::validate_tx(idx, base_state, &mv, &rs);
                                sched
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .finish_validation(idx, valid);
                            }
                            SchedulerTask::Done => break,
                            SchedulerTask::Wait => {
                                std::thread::sleep(std::time::Duration::from_micros(50));
                            }
                        }
                    }
                });
            }
        });
    }

    fn execute_tx(
        idx: TxIndex,
        txs: &[crate::parallel::TransferTx],
        base_state: &crate::state::AccountState,
        mv_memory: &MVMemory,
        read_sets: &[Mutex<ReadSet>],
        write_sets: &[Mutex<WriteSet>],
        outputs: &[Mutex<Option<TxOutput>>],
    ) {
        mv_memory.delete_writes(idx);

        let tx = &txs[idx];
        let mut view = MVView::new(mv_memory, idx, base_state);

        let sender_nonce = view.read_nonce(&tx.from);
        if tx.nonce != sender_nonce {
            let ws = vec![(
                StateKey::Nonce(tx.from),
                StateValue::Nonce(sender_nonce + 1),
            )];
            *read_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = view.into_read_set();
            mv_memory.apply_write_set(idx, &ws);
            *write_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = ws;
            *outputs[idx].lock().unwrap_or_else(|e| e.into_inner()) = Some(TxOutput {
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
            *read_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = view.into_read_set();
            mv_memory.apply_write_set(idx, &ws);
            *write_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = ws;
            *outputs[idx].lock().unwrap_or_else(|e| e.into_inner()) = Some(TxOutput {
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

        *read_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = read_set;
        mv_memory.apply_write_set(idx, &ws);
        *write_sets[idx].lock().unwrap_or_else(|e| e.into_inner()) = ws;
        *outputs[idx].lock().unwrap_or_else(|e| e.into_inner()) = Some(TxOutput {
            tx_index: idx,
            success: true,
            gas_used: 21_000,
            error: None,
        });
    }

    fn validate_tx(
        idx: TxIndex,
        base_state: &crate::state::AccountState,
        mv_memory: &MVMemory,
        read_sets: &[Mutex<ReadSet>],
    ) -> bool {
        let rs = read_sets[idx].lock().unwrap_or_else(|e| e.into_inner());
        for entry in rs.iter() {
            let current = mv_memory.read(&entry.key, idx);
            match (&entry.version, current) {
                (None, None) => {
                    let base_val = Self::read_base(&entry.key, base_state);
                    if entry.value != base_val {
                        return false;
                    }
                }
                (Some(v1), Some((v2, ref val))) if *v1 == v2 => {
                    if entry.value.as_ref() != Some(val) {
                        return false;
                    }
                }
                _ => return false,
            }
        }
        true
    }

    fn read_base(key: &StateKey, base_state: &crate::state::AccountState) -> Option<StateValue> {
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

    // ── MVMemory tests ──────────────────────────────────────────

    #[test]
    fn mv_write_and_read() {
        let mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(500));

        let result = mv.read(&key, 1);
        assert_eq!(result.as_ref().map(|(idx, _)| *idx), Some(0));
        match &result.unwrap().1 {
            StateValue::Balance(b) => assert_eq!(*b, 500),
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn mv_read_returns_latest_before_reader() {
        let mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(100));
        mv.write(2, key.clone(), StateValue::Balance(200));
        mv.write(5, key.clone(), StateValue::Balance(300));

        let result = mv.read(&key, 3).unwrap();
        assert_eq!(result.0, 2);
        assert_eq!(result.1, StateValue::Balance(200));

        let result = mv.read(&key, 1).unwrap();
        assert_eq!(result.0, 0);
        assert_eq!(result.1, StateValue::Balance(100));

        assert!(mv.read(&key, 0).is_none());
    }

    #[test]
    fn mv_no_cross_contamination() {
        let mv = MVMemory::new();
        let key_a = StateKey::Balance([1u8; 32]);
        let key_b = StateKey::Balance([2u8; 32]);
        mv.write(0, key_a.clone(), StateValue::Balance(100));
        mv.write(0, key_b.clone(), StateValue::Balance(200));

        assert_eq!(mv.read(&key_a, 1).unwrap().1, StateValue::Balance(100));
        assert_eq!(mv.read(&key_b, 1).unwrap().1, StateValue::Balance(200));
    }

    #[test]
    fn mv_delete_writes() {
        let mv = MVMemory::new();
        let key = StateKey::Balance([1u8; 32]);
        mv.write(0, key.clone(), StateValue::Balance(100));
        mv.write(1, key.clone(), StateValue::Balance(200));

        mv.delete_writes(0);

        assert!(mv.read(&key, 1).is_none());
        assert_eq!(mv.read(&key, 2).unwrap().1, StateValue::Balance(200));
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

        assert!(matches!(sched.next_task(), SchedulerTask::Execute(0)));
        sched.finish_execution(0);
        assert!(matches!(sched.next_task(), SchedulerTask::Validate(0)));
        sched.finish_validation(0, true);

        assert!(matches!(sched.next_task(), SchedulerTask::Execute(1)));
        sched.finish_execution(1);
        assert!(matches!(sched.next_task(), SchedulerTask::Validate(1)));
        sched.finish_validation(1, false);

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

        let (_, write_sets) = BlockSTMExecutor::execute_full(&txs, &state_seq);
        let mut state_stm = state_seq.clone();
        apply_block_stm_to_state(&mut state_stm, &write_sets);

        crate::parallel::execute_transfers(&mut state_seq, &txs);

        assert_eq!(state_stm.balance(&alice), state_seq.balance(&alice));
        assert_eq!(state_stm.balance(&bob), state_seq.balance(&bob));
        assert_eq!(state_stm.balance(&carol), state_seq.balance(&carol));
        assert_eq!(state_stm.nonce(&alice), state_seq.nonce(&alice));
        assert_eq!(state_stm.nonce(&bob), state_seq.nonce(&bob));
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
    fn block_stm_insufficient_balance() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 50);

        let txs = vec![make_tx(alice, bob, 100, 0)];
        let outputs = BlockSTMExecutor::execute(&txs, &state);
        assert!(!outputs[0].success);
        assert!(outputs[0].error.as_ref().unwrap().contains("insufficient"));
    }

    #[test]
    fn block_stm_empty_batch() {
        let state = AccountState::new();
        let outputs = BlockSTMExecutor::execute(&[], &state);
        assert!(outputs.is_empty());
    }

    #[test]
    fn block_stm_state_application() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        state.set_balance(&alice, 1000);
        state.set_balance(&bob, 500);

        let txs = vec![make_tx(alice, bob, 200, 0)];
        let (_, write_sets) = BlockSTMExecutor::execute_full(&txs, &state);
        apply_block_stm_to_state(&mut state, &write_sets);

        assert_eq!(state.balance(&alice), 800);
        assert_eq!(state.balance(&bob), 700);
        assert_eq!(state.nonce(&alice), 1);
    }

    // ── MVView tests ────────────────────────────────────────────

    #[test]
    fn mv_view_reads_from_base_state() {
        let mv = MVMemory::new();
        let mut state = AccountState::new();
        let addr = [1u8; 32];
        state.set_balance(&addr, 999);

        let mut view = MVView::new(&mv, 0, &state);
        assert_eq!(view.read_balance(&addr), 999);
    }

    #[test]
    fn mv_view_reads_from_mv_memory() {
        let mv = MVMemory::new();
        let state = AccountState::new();
        let addr = [1u8; 32];
        mv.write(0, StateKey::Balance(addr), StateValue::Balance(777));

        let mut view = MVView::new(&mv, 1, &state);
        assert_eq!(view.read_balance(&addr), 777);
    }

    // ── Parallel execution tests ────────────────────────────────

    #[test]
    fn parallel_independent_transfers() {
        let mut state = AccountState::new();
        let accounts: Vec<Address> = (0..10).map(|i| [i as u8; 32]).collect();
        for acc in &accounts {
            state.set_balance(acc, 10_000);
        }

        // 5 independent transfers from different senders
        let txs: Vec<_> = (0..5)
            .map(|i| make_tx(accounts[i], accounts[i + 5], 100, 0))
            .collect();

        let (outputs, write_sets) = BlockSTMExecutor::execute_full(&txs, &state);
        assert!(outputs.iter().all(|o| o.success));

        let mut result = state.clone();
        apply_block_stm_to_state(&mut result, &write_sets);

        for i in 0..5 {
            assert_eq!(result.balance(&accounts[i]), 9_900);
            assert_eq!(result.balance(&accounts[i + 5]), 10_100);
        }
    }

    #[test]
    fn parallel_conflicting_chain() {
        let mut state = AccountState::new();
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let carol = [3u8; 32];
        let dave = [4u8; 32];
        state.set_balance(&alice, 10_000);

        // Chain of transfers from same sender (forces re-execution)
        let txs = vec![
            make_tx(alice, bob, 100, 0),
            make_tx(alice, carol, 200, 1),
            make_tx(alice, dave, 300, 2),
            make_tx(alice, bob, 400, 3),
        ];

        let (outputs, write_sets) = BlockSTMExecutor::execute_full(&txs, &state);
        assert!(outputs.iter().all(|o| o.success));

        let mut result = state.clone();
        apply_block_stm_to_state(&mut result, &write_sets);

        assert_eq!(result.balance(&alice), 10_000 - 100 - 200 - 300 - 400);
        assert_eq!(result.balance(&bob), 500);
        assert_eq!(result.balance(&carol), 200);
        assert_eq!(result.balance(&dave), 300);
        assert_eq!(result.nonce(&alice), 4);
    }

    #[test]
    fn parallel_deterministic_across_runs() {
        let mut state = AccountState::new();
        let accounts: Vec<Address> = (0..8).map(|i| [i as u8; 32]).collect();
        for acc in &accounts {
            state.set_balance(acc, 50_000);
        }

        let txs: Vec<_> = (0..6)
            .map(|i| {
                make_tx(
                    accounts[i % 3],
                    accounts[3 + (i % 5)],
                    100 * (i as u128 + 1),
                    i as u64 / 3,
                )
            })
            .collect();

        let mut roots = Vec::new();
        for _ in 0..5 {
            let (_, write_sets) = BlockSTMExecutor::execute_full(&txs, &state);
            let mut s = state.clone();
            apply_block_stm_to_state(&mut s, &write_sets);
            roots.push(s.state_root());
        }

        for root in &roots[1..] {
            assert_eq!(
                roots[0], *root,
                "Block-STM must be deterministic across runs"
            );
        }
    }
}
