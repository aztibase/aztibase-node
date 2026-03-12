# Sprint 058 — Pre-Mainnet Audit Fixes

**Started:** 2026-03-12
**Goal:** Resolve all 15 CRITICAL + 19 HIGH findings from pre-mainnet audit. Unblock mainnet launch.
**Predecessor:** Sprint 057 (Validator Business-in-a-Box)

---

## Phase 1: Safety (Mainnet Blockers)

| # | Task | Finding | Status |
|---|------|---------|--------|
| 1.1 | Ed25519 signatures on DagBlock — sign on create, verify on receive | H-CON-1 | DONE |
| 1.2 | Replace `try_send` with guaranteed delivery for committed batches | H-CON-3 | DONE |
| 1.3 | Gate FaucetDrip on network profile (reject on mainnet) | H-NODE-3, H-EXEC-3 | DONE |
| 1.4 | Fix `.unwrap()`/`.expect()` in prod code (~30 instances) | C-NODE-1/2/3, C-NET-1, H-EXEC-1/2, H-CON-4 | DONE |
| 1.5 | BLAKE3 gossipsub message_id with source+topic | C-NET-2, H-NET-2 | DONE |

## Phase 2: Correctness

| # | Task | Finding | Status |
|---|------|---------|--------|
| 2.1 | Fix APY formula mismatch (code vs spec) | C-EXEC-1 | DONE |
| 2.2 | Fix VerkleTree leaf_count double-counting on updates | C-EXEC-3 | DONE |
| 2.3 | EpochDistribution overflow protection (checked_add) | C-EXEC-2 | DONE |
| 2.4 | Phantom parent defense in `insert_relaxed` | C-CON-2 | DONE |
| 2.5 | Replace `Vec::remove(0)` with VecDeque (engine + mempool) | C-CON-3, H-NODE-1 | DONE |
| 2.6 | Align wave_length defaults (ConsensusConfig vs CommitConfig) | H-CON-5 | DONE |
| 2.7 | Document n=4 minimum committee or align quorum thresholds | C-CON-1 | DONE |

## Phase 3: Hardening

| # | Task | Finding | Status |
|---|------|---------|--------|
| 3.1 | Zeroize secret keys in WASM tx_signing | C-WASM-1 | DONE |
| 3.2 | WASM header chain BLS signature verification | C-WASM-2, H-WASM-1 | DONE |
| 3.3 | Persist equivocation proofs to redb | H-CON-2 | DONE |
| 3.4 | Rate limiter bucket eviction (bounded HashMap) | H-RPC-4 | DONE |
| 3.5 | CORS restrictive default (Any only in testnet profile) | H-RPC-3 | DONE |
| 3.6 | Inference timeout pre-check + compute unit enforcement | H-RT-1, H-RT-2 | DONE |
| 3.7 | Snapshot bootstrap quorum (2+ peers agree on hash) | H-NODE-2 | DONE |

## Phase 4: Testnet Validation

| # | Task | Status |
|---|------|--------|
| 4.1 | cargo check + clippy (0 warnings) + fmt --check | IN PROGRESS |
| 4.2 | cargo test (all 931+ pass) | IN PROGRESS |
| 4.3 | 3-node local testnet — full mesh, block production | PENDING |
| 4.4 | Faucet drip (testnet) + verify mainnet gate rejects | PENDING |
| 4.5 | Transfer stress test (20+ txs) | PENDING |
| 4.6 | Cross-node state consistency check | PENDING |
| 4.7 | RPC endpoint sweep (49 methods) | PENDING |
| 4.8 | Metrics/monitoring verification | PENDING |

## Session Notes (2026-03-12)

### Critical Bug Found & Fixed: decode_vertex Address vs Pubkey Mismatch
During Phase 4 testnet validation, the 3-node testnet failed to make consensus progress. Root cause: `decode_vertex()` in `wire.rs` assumed `block.author` was a raw Ed25519 public key and called `PublicKey::from_bytes(&block.author)`. In production, `block.author` is a BLAKE3 hash (address) of the pubkey — not the pubkey itself. Every inter-node vertex was rejected with "invalid Ed25519 signature on vertex".

**Fix**: Store Ed25519 public keys alongside validator IDs in `ValidatorSet`. The genesis config now includes `public_key` (Ed25519 hex) in each `ValidatorEntry`. The node parses and stores it via `ValidatorSet::set_ed25519_key()`. `decode_vertex()` looks up the pubkey from the validator set instead of treating the author field as a pubkey.

**Status**: Consensus crate compiles + 114 tests pass. Node crate compiles. Full test suite + testnet re-validation still pending.

---

## Deferred to Post-Launch

- Replace `blst` with pure-Rust BLS (V1) — tracked in FUTURE_PLANNING.md
- Block-STM pipeline integration (G3)
- Object model (G2)
- Privacy primitives (V3)
- Data availability sampling (G5)
- WASM contract execution (G4)

---

## Exit Criteria

- [ ] 0 CRITICAL findings remaining
- [ ] 0 mainnet-blocking HIGH findings remaining
- [ ] cargo clippy: 0 warnings
- [ ] cargo test: all pass (target 950+)
- [ ] 3-node testnet: blocks produced, transfers confirmed, faucet works (testnet), faucet rejected (mainnet profile)
