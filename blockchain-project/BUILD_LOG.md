# Build Log -- Dendrite Network

Running diary of build activity. Each entry records who built what, when, and any review notes.
Entries are prepended (newest first).

---

## Log Format

```
### [DATE] -- [ENGINEER] -- [CRATE/AREA]
**Task:** [What was done]
**Sprint:** [Sprint reference]
**Git Ref:** [Commit hash]
**Files Changed:** [List]
**Review Notes:** [Any observations, issues, or follow-ups]
**Security Flags:** [None / Description]
```

---

## Entries

### 2026-03-05 -- consensus-engineer -- dendrite-consensus
**Task:** Phase 2 complete: DagStore + CommitRule implementations
**Sprint:** Sprint 002, Phase 2 (Tasks 7-8)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-consensus/src/dag_store.rs (DagStore: insert, get, parent/child lookup, ancestor check, causal ordering, persistence via redb)
- crates/dendrite-consensus/src/commit.rs (CommitRule: direct commit via supermajority voting, indirect commit via anchor, wave-based leader election)
- crates/dendrite-consensus/src/lib.rs (exports, 10 new tests -- 7 DagStore + 3 CommitRule)
- crates/dendrite-consensus/Cargo.toml (added bincode dependency)
**Review Notes:** DagStore: in-memory index + redb persistence, parent validation on insert, BFS ancestor traversal with round-based pruning, deterministic topological sort for causal ordering, rebuild_index from disk on startup. CommitRule: MystiCeti-inspired wave structure (configurable wave_length), direct commit checks >2/3 voting stake, indirect commit via causal ancestry from anchor block. 26 consensus tests total (16 existing + 10 new). Zero clippy warnings, fmt clean.
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer -- dendrite-consensus
**Task:** Phase 1b complete: DagBlock + ValidatorSet implementations
**Sprint:** Sprint 002, Phase 1b (Tasks 5-6)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-consensus/src/dag.rs (DagBlock with hash, genesis, parent validation)
- crates/dendrite-consensus/src/validator.rs (ValidatorSet with stake, supermajority, leader selection)
- crates/dendrite-consensus/src/lib.rs (exports, 16 tests)
**Review Notes:** DagBlock: deterministic hash (BLAKE3 of round+author+parents+payload+timestamp), genesis block factory, parent round monotonicity validation. ValidatorSet: add/remove/get/contains, total_stake tracking, BFT supermajority check (>2/3), deterministic stake-weighted leader selection. 16 tests passing. Zero clippy warnings.
**Security Flags:** None

---

### 2026-03-05 -- node-engineer -- dendrite-storage
**Task:** Phase 1a complete: StateStore trait, 6 named tables, batch writes, range iteration
**Sprint:** Sprint 002, Phase 1a (Tasks 1-4)
**Git Ref:** pending
**Files Changed:**
- crates/dendrite-storage/src/store.rs (complete rewrite — StateStore with redb backend)
- crates/dendrite-storage/src/lib.rs (exports, 15 tests)
**Review Notes:** Full CRUD (get/put/delete/contains), batch_put (single table), batch_put_multi (cross-table atomic), iter/range/range_reverse. All 6 tables (blocks, state, tx, receipts, validators, verkle) initialized on open. StorageError with boxed TransactionError to satisfy clippy. Zero clippy warnings, fmt clean. 15 tests passing (exit criteria was 10+).
**Security Flags:** None

---

### 2026-03-05 -- project-lead -- Project Infrastructure
**Task:** Established build-phase documentation trail
**Sprint:** Pre-Sprint
**Git Ref:** pending
**Files Changed:**
- blockchain-project/STATUS.md (new)
- blockchain-project/DECISIONS.md (new)
- blockchain-project/BUILD_LOG.md (new)
- blockchain-project/sprints/SPRINT-001.md (new)
- CHANGELOG.md (new)
- blockchain-project/ORCHESTRATION.md (updated -- build-phase protocol)
**Review Notes:** Design phase had excellent tracking via ORCHESTRATION.md. Build phase needed its own tracking infrastructure. All five tracking documents now in place.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Storage
**Task:** Switched storage backend from RocksDB to redb
**Sprint:** Pre-Sprint
**Git Ref:** dd98662
**Files Changed:** crates/dendrite-storage/ (Cargo.toml, src/)
**Review Notes:** Pure Rust dependency, better cross-compilation story. See ADR-001.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Project Scaffold
**Task:** Initial project structure, all crate scaffolding, skill definitions
**Sprint:** Pre-Sprint
**Git Ref:** 86f194b
**Files Changed:** Full workspace creation -- Cargo.toml, 8 crates, .claude/skills/, blockchain-project/ docs
**Review Notes:** Foundation commit. All design docs, skill fleet, and crate stubs.
**Security Flags:** None
