# Sprint 056 — Full State Snapshots & Mainnet Genesis (M9-S17)

**Date:** 2026-03-11
**Milestone:** M9 — Mainnet Prep
**Goal:** Enable new validators to bootstrap from a snapshot file instead of replaying from genesis. Build a mainnet genesis ceremony with real tokenomics allocations.
**Predecessor:** Sprint 055 (Public Testnet Launch Infrastructure)
**Status:** COMPLETE

---

## Problem Statement

The existing snapshot module captures AccountState (balances, nonces, code, storage) but not protocol stores (staking, governance, emission, chain params, agent policies, L2 bridge). New validators joining a running chain must replay from genesis, which becomes impractical at scale. Additionally, mainnet genesis requires auditable tokenomics allocations (400M supply across 8 categories), not the random keys used for testnet.

---

## Phase 1: Full State Snapshot (protocol stores included)

| # | Task | Status |
|---|------|--------|
| 1.1 | Extend `StateSnapshot` with protocol store fields (staking, governance, emission, chain_params, agent_policies, bridge stores, base_fee) | DONE |
| 1.2 | Update `create_snapshot` / `create_snapshot_with_finality` to capture protocol stores | DONE |
| 1.3 | Update `apply_snapshot` to restore protocol stores | DONE |
| 1.4 | Bump SNAPSHOT_VERSION to 3 (backward-incompatible, versioned deserialization) | DONE |

## Phase 2: Snapshot Export/Import CLI

| # | Task | Status |
|---|------|--------|
| 2.1 | `aztibase snapshot export --output <path>` subcommand (loads state from DB, creates snapshot, writes to file) | DONE |
| 2.2 | `--snapshot <path>` CLI flag on startup (loads snapshot file, applies state, skips genesis replay) | DONE |
| 2.3 | Snapshot integrity check on import (BLAKE3 hash + state root verification) | DONE |

## Phase 3: Mainnet Genesis Ceremony

| # | Task | Status |
|---|------|--------|
| 3.1 | `mainnet_genesis()` function with real tokenomics allocations (400M across 8 categories) | DONE |
| 3.2 | Allocation categories: team (15%), investors (10%), ecosystem (25%), community (20%), treasury (15%), validators (5%), advisors (5%), reserve (5%) | DONE |
| 3.3 | No faucet account in mainnet genesis | DONE |
| 3.4 | Placeholder addresses for each allocation (to be replaced with real keys at ceremony) | DONE |

## Phase 4: Tests

| # | Task | Status |
|---|------|--------|
| 4.1 | Full snapshot roundtrip (with protocol stores) | DONE |
| 4.2 | Snapshot export/import file I/O | DONE |
| 4.3 | Mainnet genesis validation (supply = 400M, no faucet, 8 allocations) | DONE |
| 4.4 | State root verification on import | DONE |
| 4.5 | Version mismatch rejection | DONE |

## Phase 5: Security Review & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | Security review: snapshot deserialization safety, file path validation | DONE |
| 5.2 | ADR-027: Full state snapshots & mainnet genesis design | DONE |
| 5.3 | BUILD_LOG, STATUS, CHANGELOG, sprint plan updates | DONE |

---

## Exit Criteria

- [x] StateSnapshot includes all protocol stores
- [x] `aztibase snapshot export` writes snapshot file
- [x] `aztibase --snapshot` bootstraps node from file
- [x] Mainnet genesis produces 400M supply across 8 allocations
- [x] 7 new tests, all passing (931 total)
- [x] 0 clippy warnings, fmt clean
- [x] ADR-027 written
- [x] All docs updated

---

## Retrospective

### What went well
- ProtocolStoreBundle design cleanly bundles all 9 stores + base_fee with Option<> backward compat
- BLAKE3 integrity hash catches corruption before deserialization — zero unsafe parse paths
- Mainnet genesis math is trivially auditable (integer percentages, no floating point)
- Missing Clone/Debug derives caught at compile time, fixed quickly across 3 files

### What to watch
- Snapshot file has no encryption — operators should protect files at rest for mainnet
- Mainnet genesis placeholder keys must be replaced with real multisig keys at ceremony
- MAX_SNAPSHOT_SIZE (128 MiB) may need adjustment as state grows post-mainnet
