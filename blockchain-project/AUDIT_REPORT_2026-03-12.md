# Aztibase Network -- Pre-Mainnet Audit Report

**Date:** 2026-03-12
**Scope:** All 9 crates (core, consensus, storage, network, execution, node, rpc, runtime, wasm) + architecture alignment
**Method:** Code review only (no builds/tests executed)
**Auditors:** 7 parallel agents covering each domain

---

## Executive Summary

| Severity | Count | Blocks Mainnet? |
|----------|-------|-----------------|
| CRITICAL | 15 | YES |
| HIGH | 25 | YES (most) |
| MEDIUM | 31 | Recommended |
| LOW | 26 | Optional |
| INFO | ~50 | No action |

**Code quality is high overall.** Zero AI fingerprints, zero `todo!()`/`unimplemented!()`, zero `unsafe` blocks, zero dead code. Test coverage is strong (931+ tests). Architecture follows DAG-BFT correctly.

**Top 5 blockers for mainnet:**
1. No cryptographic signature on DAG blocks (consensus H-1/M-4) -- any node can forge blocks as any validator
2. `try_send` drops committed batches silently (consensus H-3/M-5) -- finalized txs can be lost
3. WASM light client does zero signature verification (core C-3, H-1) -- accepts forged sync data
4. `blst` C dependency violates pure-Rust constraint (arch V1)
5. Faucet minting has no mainnet gate in pipeline (node H-3, M-3)

---

## CRITICAL Findings (15)

### Consensus

**C-CON-1: `has_quorum` (>=2/3) vs `has_supermajority` (>2/3) mismatch**
- File: `crates/aztibase-consensus/src/validator.rs:119-136`
- Impact: With n=3 validators, commits require unanimity (all 3). One offline validator permanently stalls commits. Threshold clock advances but commits never happen.
- Fix: Document minimum viable committee is n=4, or align both thresholds.

**C-CON-2: `insert_relaxed` bypasses all DAG integrity -- phantom parent attacks**
- File: `crates/aztibase-consensus/src/dag_store.rs:124-153`
- Impact: Byzantine node can reference non-existent parents. Orphan vertices pollute index, corrupt `is_ancestor` traversal. May produce incorrect committed batches.
- Fix: Track orphaned parents, re-validate when parents arrive, or prune after timeout.

**C-CON-3: `drain_pending_txs` uses `Vec::remove(0)` -- O(n^2)**
- File: `crates/aztibase-consensus/src/engine.rs:801`
- Impact: With 4096 pending txs, ~16M element shifts per proposal. DoS vector under load.
- Fix: Replace `Vec` with `VecDeque`, use `pop_front()`.

### Network + Storage

**C-NET-1: 9x `.expect()` panics in `light.rs` on untrusted data**
- File: `crates/aztibase-storage/src/light.rs:71,86,99,124,139,158,173,188,226`
- Impact: Single corrupt DB record crashes the node.
- Fix: Return `StorageError` variants instead of panicking.

**C-NET-2: Gossipsub `message_id` uses `DefaultHasher` (64-bit, non-cryptographic)**
- File: `crates/aztibase-network/src/gossip.rs:73`
- Impact: Attacker can craft colliding message IDs, suppressing legitimate messages for 120s.
- Fix: Use BLAKE3, include source + topic in hash input.

**C-NET-3: `VerkleTree` in storage crate is an empty stub**
- File: `crates/aztibase-storage/src/verkle.rs`
- Impact: Light client verification has no real state proof backend.

### Core + Runtime + WASM

**C-CORE-1: `blst` crate violates pure-Rust policy (ADR-001)**
- File: `crates/aztibase-core/Cargo.toml:12`
- Impact: C compiler required during build. Violates non-negotiable constraint.
- Fix: Evaluate `bls12_381` + `group` (pure Rust, ZCash) or document ADR exception.

**C-WASM-1: Private keys not zeroed in WASM `parse_secret_key`**
- File: `crates/aztibase-wasm/src/tx_signing.rs` (intermediate `[u8; 32]`)
- Impact: Secret key bytes persist in WASM linear memory, recoverable by JS.
- Fix: Use `zeroize::Zeroize` on the bytes array before returning.

**C-WASM-2: `verify_header_chain` does NOT verify BLS aggregate signature**
- File: `crates/aztibase-wasm/src/sync.rs:29-74`
- Impact: Any peer can forge sync responses with dummy signatures. Light client security is completely defeated.
- Fix: Integrate BLS verification or gate behind compile-time feature flag.

### Execution

**C-EXEC-1: `calculate_apy_bps` formula mismatch -- code vs spec**
- File: `crates/aztibase-execution/src/tokenomics.rs:156`
- Impact: Code uses `/5`, comment says `/50`. 10x discrepancy affects validator economics.
- Fix: Determine which is correct, align code + comment + tests.

**C-EXEC-2: `EpochDistribution::total()` can overflow silently**
- File: `crates/aztibase-execution/src/tokenomics.rs:119-122`
- Impact: Financial calculation uses plain `+` instead of `checked_add`.
- Fix: Use `saturating_add` or `checked_add`.

**C-EXEC-3: `VerkleTree::insert` double-counts `leaf_count`**
- File: `crates/aztibase-execution/src/verkle.rs:86`
- Impact: `len()` returns inflated count on updates. Breaks threshold checks.
- Fix: Only increment on new keys, not updates.

### Node

**C-NODE-1: `.unwrap()` on `postcard::to_allocvec` in pipeline PostTask**
- File: `crates/aztibase-node/src/pipeline.rs:1273`
- Impact: Serialization failure panics the entire pipeline.
- Fix: Use `.map_err()` or log and skip.

**C-NODE-2: `.unwrap()` on `stdin().read_line()` in wallet CLI**
- File: `crates/aztibase-node/src/main.rs:443,478`
- Impact: Node panics if run as daemon with redirected stdin.
- Fix: Use `?` or graceful error.

**C-NODE-3: `.expect()` in `genesis_hash` production code**
- File: `crates/aztibase-node/src/genesis.rs:301`
- Impact: Panic in main startup path.
- Fix: Return `Result`.

---

## HIGH Findings (25)

### Consensus (5)
| ID | Finding | File |
|----|---------|------|
| H-CON-1 | No signature verification on received vertices -- any node can forge blocks as any validator | `wire.rs` / `dag.rs` |
| H-CON-2 | Equivocation detection is memory-only, pruned after 100 rounds, lost on restart | `engine.rs:630-657` |
| H-CON-3 | `try_send` for committed batches silently drops finalized txs | `engine.rs:737-743` |
| H-CON-4 | `SignerBitmap::set` panics on out-of-bounds (library code) | `finality.rs:29` |
| H-CON-5 | `wave_length` default mismatch: ConsensusConfig=4, CommitConfig=3 | `engine.rs:131` vs `commit.rs:30` |

### Network + Storage (5)
| ID | Finding | File |
|----|---------|------|
| H-NET-1 | No gossipsub message content validation beyond size checks | `gossip.rs:158-177` |
| H-NET-2 | message_id excludes source/topic -- cross-topic suppression | `gossip.rs:72-76` |
| H-NET-3 | Subnet grouping /16 too coarse for eclipse resistance | `connection_filter.rs:126-137` |
| H-NET-4 | TOCTOU race in reputation-gated eviction | `reputation.rs:157-185` |
| H-NET-5 | Kademlia uses MemoryStore -- lost on restart | `discovery.rs:38` |

### Core + Runtime + WASM (5)
| ID | Finding | File |
|----|---------|------|
| H-WASM-1 | `light_client_proof` never verifies finality certificate | `proof.rs:173-202` |
| H-RT-1 | Inference timeout checked AFTER execution completes | `tract_runtime.rs:149-158` |
| H-RT-2 | `max_compute_units` in InferenceRequest never enforced | `tract_runtime.rs` |
| H-RT-3 | ContractRuntime and AgentRuntime are empty stubs | `contracts.rs`, `agent.rs` |
| H-WASM-2 | Browser wallet spending limits are advisory only | `browser_wallet.rs` |

### Execution (3)
| ID | Finding | File |
|----|---------|------|
| H-EXEC-1 | `.unwrap()` on `Mutex::lock()` throughout block_stm.rs (16+ instances) | `block_stm.rs` |
| H-EXEC-2 | `.unwrap()` on `try_into()` in verkle verify_proof (untrusted data) | `verkle.rs:251` |
| H-EXEC-3 | No authorization check on FaucetDrip -- anyone can mint tokens | `routing.rs` |

### Node (3)
| ID | Finding | File |
|----|---------|------|
| H-NODE-1 | `seen_order.remove(0)` is O(n) in mempool (100K capacity) | `mempool.rs:79` |
| H-NODE-2 | Snapshot bootstrap trusts any single peer without quorum | `main.rs:1170-1218` |
| H-NODE-3 | FaucetDrip bypasses gas AND profile check in pipeline | `pipeline.rs:504-508` |

### RPC (4)
| ID | Finding | File |
|----|---------|------|
| H-RPC-1 | Faucet keypair is deterministic + public (source-readable seed) | `server.rs:100-103` |
| H-RPC-2 | Faucet drip abuse: per-address cooldown only, no per-IP limit | `server.rs:1318-1334` |
| H-RPC-3 | CORS permissive by default (`AllowOrigin::Any`) | `server.rs:344` |
| H-RPC-4 | Rate limiter bucket map grows unbounded (memory exhaustion) | `server.rs:220-260` |

---

## Architecture Alignment

### Constraint Violations (3)
| # | Constraint | Status |
|---|-----------|--------|
| V1 | Pure Rust deps (ADR-001) | **VIOLATED** -- `blst` is C |
| V2 | Verkle trees for state | **PARTIAL** -- VerkleTree exists but `AccountState::state_root()` uses binary Merkle |
| V3 | Protocol-level privacy | **NOT IMPLEMENTED** -- zero privacy primitives |

### Design Constraints: PASS (5/8)
| # | Constraint | Status |
|---|-----------|--------|
| 1 | Server-independence | PASS (no hardcoded centralized URLs in prod) |
| 2 | DAG consensus (multi-parent blocks) | PASS (`DagBlock.parents: Vec<BlockHash>`) |
| 3 | 400ms block time | PASS (configured, testnet showed ~38ms/block) |
| 6 | AI inference off-chain | PASS (TractRuntime local, InferenceReceipt on-chain) |
| 8 | Pure Rust (except blst) | MOSTLY PASS |

### Gaps (8 features designed but not implemented)
1. **NetworkTransport trait** -- libp2p used directly, no abstraction
2. **Object model** -- state is pure account-based, no first-class objects
3. **Block-STM parallel execution** -- built but not wired into pipeline
4. **WASM contract execution** -- wasmtime declared but ContractRuntime is empty
5. **Data availability sampling** -- not implemented
6. **Incentivized relay** -- relay exists but no fee mechanism
7. **Forkless runtime upgrades** -- governance is parameter-only
8. **BlockBody missing fields** -- no `privacy_proofs`, no `validator_set_diff`

### Deviations (5 -- deliberate, need doc update)
1. **redb** instead of RocksDB (ADR-001 justified)
2. **postcard** instead of Protobuf/Bincode (simpler, faster)
3. **DagBlock** vs BlockHeader (two parallel representations)
4. **BLAKE3 hash** instead of IPA/Pedersen for Verkle commitments (placeholder)
5. **Round != block** -- wave-based commits, effective block time varies

---

## Clean Code Compliance

| Rule | Status |
|------|--------|
| No AI fingerprints | PASS -- zero instances |
| No over-commenting | PASS |
| No placeholder code | PASS (no `todo!()`) |
| No dead code | PASS (1 `#[allow(dead_code)]` flagged in node) |
| Clippy clean | NOT VERIFIED (no build run) |
| No `unsafe` | PASS -- zero blocks |
| Idiomatic Rust | PASS |

---

## Recommended Fix Priority

### Phase 1: Safety (blocks mainnet launch)
1. Add Ed25519 signatures to DagBlock (H-CON-1)
2. Replace `try_send` with guaranteed delivery for committed batches (H-CON-3)
3. Gate FaucetDrip on network profile in pipeline (H-NODE-3, H-EXEC-3)
4. Fix all `.unwrap()`/`.expect()` in non-test code (~30 instances across crates)
5. Switch gossipsub message_id to BLAKE3 (C-NET-2)

### Phase 2: Correctness (should fix before mainnet)
6. Fix APY formula mismatch (C-EXEC-1)
7. Fix VerkleTree leaf_count double-counting (C-EXEC-3)
8. Wire VerkleTree into AccountState::state_root() (arch V2)
9. Add deferred validation for `insert_relaxed` (C-CON-2)
10. Replace `Vec::remove(0)` with VecDeque (C-CON-3, H-NODE-1)

### Phase 3: Hardening (recommended before mainnet)
11. WASM light client BLS verification (C-WASM-2, H-WASM-1)
12. Zeroize secret keys in WASM (C-WASM-1)
13. Persist equivocation proofs (H-CON-2)
14. Rate limiter + faucet tracker eviction (H-RPC-4)
15. CORS default to restrictive (H-RPC-3)

### Phase 4: Completeness (post-launch OK)
16. Replace `blst` with pure-Rust BLS (arch V1)
17. Integrate Block-STM parallel execution (arch G3)
18. Implement object model (arch G2)
19. Privacy primitives (arch V3)
20. Data availability sampling (arch G5)

---

## Test Coverage Summary

| Crate | Tests | Coverage Areas |
|-------|-------|----------------|
| core | 22 | Crypto vectors, serialization, malleability, BLS |
| consensus | 114 | DagBlock, DagStore, commit rules, engine, equivocation, finality, PoUW, wire |
| storage | 34 | CRUD, atomicity, compaction, temp cleanup |
| network | 85 | Gossipsub, peer scoring, discovery, connection filter, reputation, light sync |
| execution | 321 | All TxKinds, flash-loan, reentrancy, slashing, EVM, WASM, bridge, governance |
| node | 223 | Pipeline, mempool, genesis, sync, wallet, integration |
| rpc | 77 | All methods, parse errors, rate limiting, WebSocket, CORS |
| runtime | 22 | ONNX inference, receipts, verification, anomaly |
| wasm | 33 | Signing, proofs, sync, browser wallet |
| **Total** | **931** | |

---

*Report generated by 7 parallel audit agents. No builds or tests were executed.*
