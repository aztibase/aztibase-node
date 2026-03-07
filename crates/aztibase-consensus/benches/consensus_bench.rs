use criterion::{Criterion, criterion_group, criterion_main};

use aztibase_consensus::{CommitConfig, CommitRule, DagBlock, DagStore, ValidatorSet};
use aztibase_storage::StateStore;
use std::sync::atomic::{AtomicU32, Ordering};

static BENCH_COUNTER: AtomicU32 = AtomicU32::new(0);

fn bench_db_path() -> std::path::PathBuf {
    let id = BENCH_COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("aztibase_bench_{}_{}", pid, id))
}

fn cleanup(path: &std::path::Path) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("lock"));
}

fn make_validators(n: usize) -> (ValidatorSet, Vec<[u8; 32]>) {
    let mut vs = ValidatorSet::new();
    let ids: Vec<[u8; 32]> = (0..n).map(|i| [(i + 1) as u8; 32]).collect();
    for id in &ids {
        vs.add(*id, 100);
    }
    (vs, ids)
}

fn bench_vertex_creation(c: &mut Criterion) {
    let ts = 1000u64;
    let parent = aztibase_core::hash(b"parent");
    let author = [1u8; 32];

    c.bench_function("vertex_creation", |b| {
        let mut round = 1u64;
        b.iter(|| {
            let _ = DagBlock::new(round, author, vec![parent], vec![0u8; 128], ts);
            round += 1;
        });
    });
}

fn bench_dag_insertion(c: &mut Criterion) {
    c.bench_function("dag_insert_100_vertices", |b| {
        b.iter_with_setup(
            || {
                let path = bench_db_path();
                let store = StateStore::open(path.to_str().unwrap()).unwrap();
                let dag = DagStore::new(store).unwrap();
                (dag, path)
            },
            |(mut dag, path)| {
                let (_, ids) = make_validators(3);
                for id in &ids {
                    dag.insert(DagBlock::genesis(*id, 1000)).unwrap();
                }

                let mut prev_hashes: Vec<_> = dag.blocks_at_round(0).to_vec();

                for round in 1..=34 {
                    let mut new_hashes = Vec::new();
                    for id in &ids {
                        let block =
                            DagBlock::new(round, *id, prev_hashes.clone(), vec![], 1000 + round)
                                .unwrap();
                        new_hashes.push(block.hash);
                        dag.insert(block).unwrap();
                    }
                    prev_hashes = new_hashes;
                }
                cleanup(&path);
            },
        );
    });
}

fn bench_commit_evaluation(c: &mut Criterion) {
    c.bench_function("commit_rule_evaluation", |b| {
        b.iter_with_setup(
            || {
                let path = bench_db_path();
                let store = StateStore::open(path.to_str().unwrap()).unwrap();
                let mut dag = DagStore::new(store).unwrap();
                let (vs, ids) = make_validators(3);

                for id in &ids {
                    dag.insert(DagBlock::genesis(*id, 1000)).unwrap();
                }

                let mut prev: Vec<_> = dag.blocks_at_round(0).to_vec();
                for round in 1..=8 {
                    let mut next = Vec::new();
                    for id in &ids {
                        let block =
                            DagBlock::new(round, *id, prev.clone(), vec![], 1000 + round).unwrap();
                        next.push(block.hash);
                        dag.insert(block).unwrap();
                    }
                    prev = next;
                }

                (dag, vs, path)
            },
            |(dag, vs, path)| {
                let config = CommitConfig {
                    wave_length: 2,
                    vrf_seed: None,
                };
                let rule = CommitRule::new(&dag, &vs, config);
                for wave in 0..4 {
                    let _ = rule.try_direct_commit(wave);
                }
                cleanup(&path);
            },
        );
    });
}

criterion_group!(
    benches,
    bench_vertex_creation,
    bench_dag_insertion,
    bench_commit_evaluation,
);
criterion_main!(benches);
