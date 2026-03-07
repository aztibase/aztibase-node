use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use aztibase_core::commitment::StateCommitment;
use aztibase_core::hash;
use aztibase_execution::{
    AccountState, MerkleCommitment, TransferTx, VerkleCommitment, execute_transfers,
};

fn make_state_with_accounts(n: usize) -> AccountState {
    let mut state = AccountState::new();
    for i in 0..n {
        let addr = hash(&(i as u64).to_le_bytes());
        state.set_balance(&addr, 1_000_000);
    }
    state
}

fn bench_state_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_root");
    for size in [100, 1000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let state = make_state_with_accounts(n);
            b.iter(|| {
                let _ = state.state_root();
            });
        });
    }
    group.finish();
}

fn bench_transfer_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("transfer_execution");
    for batch_size in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(batch_size),
            &batch_size,
            |b, &n| {
                b.iter_with_setup(
                    || {
                        let mut state = AccountState::new();
                        let mut txs = Vec::with_capacity(n);
                        for i in 0..n {
                            let from = hash(&(i as u64).to_le_bytes());
                            let to = hash(&((i + n) as u64).to_le_bytes());
                            state.set_balance(&from, 1_000_000);
                            txs.push(TransferTx {
                                hash: hash(&(i as u64).to_le_bytes()),
                                from,
                                to,
                                value: 100,
                                nonce: 0,
                            });
                        }
                        (state, txs)
                    },
                    |(mut state, txs)| {
                        let _ = execute_transfers(&mut state, &txs);
                    },
                );
            },
        );
    }
    group.finish();
}

fn bench_single_transfer(c: &mut Criterion) {
    c.bench_function("single_transfer_latency", |b| {
        let from = hash(b"alice");
        let to = hash(b"bob");
        b.iter_with_setup(
            || {
                let mut state = AccountState::new();
                state.set_balance(&from, 1_000_000_000);
                state
            },
            |mut state| {
                let tx = TransferTx {
                    hash: hash(b"tx"),
                    from,
                    to,
                    value: 100,
                    nonce: 0,
                };
                let _ = execute_transfers(&mut state, &[tx]);
            },
        );
    });
}

fn make_leaves(n: usize) -> Vec<[u8; 32]> {
    (0..n).map(|i| hash(&(i as u64).to_le_bytes())).collect()
}

fn bench_merkle_prove_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("merkle_proof_verification");
    let commitment = MerkleCommitment;
    for size in [100, 1000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let leaves = make_leaves(n);
            let root = commitment.commit(&leaves);
            let proof = commitment.prove(&leaves, n / 2).unwrap();
            b.iter(|| {
                commitment.verify(&root, &leaves[n / 2], &proof);
            });
        });
    }
    group.finish();
}

fn bench_verkle_prove_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("verkle_proof_verification");
    let commitment = VerkleCommitment;
    for size in [100, 1000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            let leaves = make_leaves(n);
            let root = commitment.commit(&leaves);
            let proof = commitment.prove(&leaves, n / 2).unwrap();
            b.iter(|| {
                commitment.verify(&root, &leaves[n / 2], &proof);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_state_root,
    bench_transfer_execution,
    bench_single_transfer,
    bench_merkle_prove_verify,
    bench_verkle_prove_verify,
);
criterion_main!(benches);
