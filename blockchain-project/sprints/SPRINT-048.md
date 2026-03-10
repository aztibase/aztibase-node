# Sprint 048 — Security Flag Resolution (M9-S9)

**Status:** COMPLETE
**Started:** 2026-03-10
**Engineer(s):** security-engineer, p2p-network-engineer, consensus-engineer, blockchain-architect
**Predecessor:** Sprint 047 (Testnet Validation)

---

## Goal

Resolve all 9 SECURITY-ELEVATED flags from the Master Plan before mainnet. These are hard blockers.

## Scope

| Flag | Description | Approach |
|------|-------------|----------|
| S4-3 | Checkpoint distribution mechanism | P2P request/response + gossip announcement |
| S1-3 | Agent spending limits at consensus layer | Pre-execution validation in mempool |
| S3-1 | Flash-loan resistant governance | Snapshot-based vote weight at proposal creation |
| S2-1 | DAG equivocation window formal analysis | Increase window + persistent equivocation proofs |
| S1-1 | MEV mitigation | Tx size limits at mempool (Phase 1); encrypted mempool roadmapped (Phase 2) |
| S1-2 | Quantum migration drill | Migration tests proving StateCommitment scheme swap |
| S2-2 | VRF last-revealer bias | RANDAO-style seed accumulation from causal DAG history |
| S4-1 | Browser key management | WASM spending limits + balance warnings |
| S8-1 | DHT poisoning resistance | Quorum-signed records + round-based freshness |

---

## Phase 1: S4-3 — Checkpoint Distribution Protocol

### Task 1.1 — Extend LightSyncMessage with checkpoint variants
- Add `RequestCheckpoint { batch_index: Option<u64> }` (None = latest)
- Add `ResponseCheckpoint { data: Vec<u8> }` (postcard bytes)
- Update codec frame handling
- **Status:** DONE

### Task 1.2 — Handle checkpoint requests in full-node event loop
- On `RequestCheckpoint`: query store, respond with serialized checkpoint
- On `ResponseCheckpoint`: validate batch_index, store if valid
- **Status:** DONE

### Task 1.3 — Gossip checkpoint announcements
- New gossipsub topic: `checkpoint-announce`
- Pipeline publishes announcement (batch_index + state_root) after storing checkpoint
- Peers request full checkpoint via light-sync on announcement
- **Status:** DONE

### Task 1.4 — Tests
- Unit: request/response roundtrip
- Unit: gossip announcement triggers fetch
- **Status:** DONE

---

## Phase 2: S1-3 — Pre-Execution Agent Spending Limits

### Task 2.1 — Add read-only policy check method
- `AgentPolicyStore::check_spend()` — validates without recording the spend
- Same checks as `validate_spend()` but no side effects
- **Status:** DONE

### Task 2.2 — Wire policy check into mempool
- `Mempool::insert_checked()` gains optional `AgentPolicyStore` parameter
- For AgentExecute txs: reject if policy check fails
- Existing execution-time validation remains as defense-in-depth
- **Status:** DONE

### Task 2.3 — Tests
- Unit: mempool rejects AgentExecute exceeding per-tx limit
- Unit: mempool rejects AgentExecute with expired policy
- Unit: mempool rejects AgentExecute with disallowed tx kind
- Unit: non-agent txs unaffected
- **Status:** DONE

---

## Phase 3: S3-1 — Flash-Loan Resistant Governance

### Task 3.1 — Snapshot balance at proposal creation
- `Proposal` gains `snapshot_balances: HashMap<Address, u128>` field
- On `CreateProposal`: capture all non-zero balances as snapshot
- **Status:** DONE

### Task 3.2 — Use snapshot weight for votes
- `cast_vote()` looks up voter in proposal's snapshot_balances
- Rejects voters not in snapshot (didn't hold tokens at proposal creation)
- Pipeline no longer passes live balance
- **Status:** DONE

### Task 3.3 — Tests
- Unit: voter with balance at proposal creation can vote
- Unit: voter who acquired tokens after proposal creation cannot vote
- Unit: voter who transferred tokens away still has original weight
- **Status:** DONE

---

## Phase 4: S2-1 — DAG Equivocation Window Analysis & Hardening

### Task 4.1 — Increase equivocation tracking window
- `EQUIVOCATION_PRUNE_DEPTH`: 20 → 100 rounds (~40s at 400ms)
- Justification: 20 rounds (8s) too aggressive — network partition can hide equivocations longer
- Memory impact: ~100 entries × 40 bytes ≈ 4KB per validator, negligible
- **Status:** DONE

### Task 4.2 — Persistent equivocation proofs
- Store equivocation evidence in redb: both conflicting vertex hashes + round + author
- `EQUIVOCATION_PROOFS_TABLE` in storage
- Evidence survives node restart, can be submitted to other nodes
- **Status:** DONE

### Task 4.3 — Formal bound documentation
- Document the equivocation exposure analysis in DECISIONS.md (ADR-015)
- Key bounds: detection window, slash latency, finality coverage gap
- **Status:** DONE

### Task 4.4 — Tests
- Unit: equivocation detected at round 99 (within new window)
- Unit: equivocation proof persisted and retrievable
- Unit: old equivocation proof survives prune
- **Status:** DONE

---

## Phase 5: Validation & Docs

### Task 5.1 — cargo clippy + fmt + test
- 0 warnings, 0 failures
- **Status:** DONE

### Task 5.2 — Security review (as /security-engineer)
- Cross-review all 4 flag resolutions
- **Status:** DONE

### Task 5.3 — Doc sync
- BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan updated
- **Status:** DONE

---

## Phase 6: S1-1 — MEV Mitigation (Phase 1)

### Task 6.1 — Consensus-layer transaction size limit
- `MAX_TX_SIZE` = 256 KiB constant in mempool
- Size check at top of `insert_with_priority()` — reject oversized txs before consensus
- Phase 2 (encrypted mempool with commit-reveal) roadmapped for post-mainnet
- **Status:** DONE

### Task 6.2 — Tests
- Unit: rejects oversized transaction (MAX_TX_SIZE + 1)
- Unit: accepts max-size transaction (exactly MAX_TX_SIZE)
- **Status:** DONE

---

## Phase 7: S1-2 — Quantum Migration Drill

### Task 7.1 — Migration tests
- Verify both `VerkleCommitment` and `MerkleCommitment` produce valid roots/proofs independently
- Verify cross-scheme verification correctly fails (Verkle proof vs Merkle root and vice versa)
- Prove `StateCommitment` trait allows runtime scheme selection via `Box<dyn StateCommitment>`
- **Status:** DONE

---

## Phase 8: S2-2 — VRF Last-Revealer Bias Fix

### Task 8.1 — RANDAO-style seed accumulation
- `accumulate_vrf_seed()`: mixes prev_seed + anchor_hash + causal vertex hashes
- Replaces single-source `hash(anchor_hash)` with multi-validator contribution
- **Status:** DONE

### Task 8.2 — Tests
- Unit: seed changes when causal history differs
- **Status:** DONE

---

## Phase 9: S4-1 — Browser Wallet Spending Limits

### Task 9.1 — WASM spending limit module
- `browser_wallet` module in aztibase-wasm crate
- Constants: BROWSER_SPENDING_LIMIT (10K AZTB), BROWSER_PER_TX_LIMIT (100 AZTB), HIGH_VALUE_WARNING_THRESHOLD (1K AZTB)
- `BrowserWalletWarning` enum for structured warnings
- `check_browser_balance()` and `check_browser_tx()` — advisory safety checks
- WASM exports: checkBrowserBalance, checkBrowserTx, browserSpendingLimit, browserPerTxLimit
- **Status:** DONE

### Task 9.2 — Tests
- Unit: balance below threshold returns no warnings
- Unit: high balance triggers warning
- Unit: tx within limit accepted
- Unit: tx exceeding per-tx limit rejected
- Unit: tx exceeding session limit rejected
- **Status:** DONE

---

## Phase 10: S8-1 — DHT Poisoning Resistance

### Task 10.1 — Quorum-signed DHT records
- `SignedDhtRecord` type with kind, round, signatures, data
- `DhtRecordKind` enum: ValidatorSet, RelayProvider, ChainTip
- `validate_dht_record()`: quorum check (≥2 sigs), freshness (≤10K rounds), sig verification via callback
- `dht_signing_payload()`: canonical payload construction for deterministic signing
- **Status:** DONE

### Task 10.2 — Tests
- Unit: valid record passes validation
- Unit: empty data rejected
- Unit: insufficient signatures rejected
- Unit: stale record rejected
- Unit: invalid signature rejected
- Unit: empty signatures rejected
- Unit: canonical signing payload deterministic
- Unit: different kinds produce different payloads
- **Status:** DONE

---

## Phase 11: Validation & Docs (Bottom 5)

### Task 11.1 — cargo clippy + fmt + test
- 700 tests pass, 0 clippy warnings, fmt clean
- **Status:** DONE

### Task 11.2 — Doc sync
- DECISIONS.md: ADR-016 through ADR-020
- BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan updated
- **Status:** DONE

---

## Exit Criteria

- [x] S4-3: Checkpoint P2P distribution works (request/response + gossip announce)
- [x] S1-3: Mempool rejects AgentExecute violating policy
- [x] S3-1: Governance votes use snapshot balance, not live balance
- [x] S2-1: Equivocation window increased to 100 rounds, proofs persisted
- [x] S1-1: MAX_TX_SIZE enforced at mempool boundary
- [x] S1-2: Migration drill proves Verkle↔Merkle swap works
- [x] S2-2: VRF seed accumulates from causal DAG history (anti-last-revealer)
- [x] S4-1: Browser wallet spending limits exported via WASM
- [x] S8-1: DHT records require quorum signatures + freshness
- [x] All 9 SECURITY-ELEVATED flags marked RESOLVED
- [x] 0 clippy warnings, fmt clean
- [x] All tests pass (700+)
