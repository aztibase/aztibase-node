---
title: Architecture Overview
description: System architecture of the Aztibase Network
---

Aztibase is structured as a layered system with clear separation of concerns across 9 Rust crates.

## System Layers

```
┌─────────────────────────────────────────────┐
│  L4: Smart Contracts (WASM + EVM)           │
├─────────────────────────────────────────────┤
│  L3: Consensus (SynBFT + PoUW)             │
├─────────────────────────────────────────────┤
│  L2: Networking (libp2p + gossipsub)        │
├─────────────────────────────────────────────┤
│  L1: Cryptography (BLAKE3 + Ed25519 + BLS) │
└─────────────────────────────────────────────┘
```

## Block (Vertex) Structure

Blocks in Aztibase are called **vertices** because they exist in a DAG, not a linear chain. Each vertex can have multiple parents.

```
DagBlock {
    hash: [u8; 32],         // BLAKE3(round || author || parents || payload || timestamp)
    round: u64,             // Consensus round number
    author: [u8; 32],       // Validator pubkey (ValidatorId)
    parents: Vec<[u8;32]>,  // Multiple parents (DAG). Max 20.
    payload: Vec<u8>,       // Length-prefixed txs: [u32 len][bytes]...
    timestamp: u64,         // Unix milliseconds
}
```

## Consensus Constants

| Parameter | Value |
|-----------|-------|
| Round duration | 400 ms |
| Wave length | 4 rounds |
| Liveness timeout | 10 s |
| Max parents per vertex | 20 |
| Max pending txs | 4,096 |
| Max payload per vertex | 256 KiB |
| Supermajority threshold | 2/3 of validator stake |
| Checkpoint interval | 1,000 rounds |

## Block Production Flow

1. **Threshold clock** advances when ≥2/3 stake has produced blocks at the current round (message-driven, no fixed timer)
2. **Fallback**: force-advance after 10 seconds if quorum not reached
3. **Proposal**: select parents (up to 20 from last 16 rounds) → drain pending txs → create DagBlock → insert into DAG → gossip broadcast
4. **Blocks per round**: one per validator (3 validators = 3 blocks/round)
5. **Empty blocks**: produced even with no pending txs to maintain clock progress

## Transaction Lifecycle

```
RPC (sendRawTransaction)
  → mempool insertion + validation
  → gossip broadcast to peers
  → consensus engine pending queue
  → vertex proposal (packs txs into payload)
  → vertex broadcast via gossipsub
  → peers receive, verify, insert into DAG
  → wave commit rule triggers
  → extract committed batch (topologically sorted)
  → execution pipeline:
      Phase 0: Nonce validation
      Phase 1: Gas escrow (gas_limit × gas_price + value)
      Phase 2: Execute transaction
      Phase 3: Refund unused gas, collect fees
  → state flush to redb
```

## Node Types

Aztibase supports 5 node types:

| Type | Description |
|------|-------------|
| **Full Node** | Stores complete state, validates all transactions |
| **Validator** | Full node + participates in consensus, produces blocks |
| **Light Node** | Syncs headers only, verifies proofs on demand |
| **Browser Node** | WASM-based light client running in web browsers |
| **Mobile Node** | Optimized light client for mobile devices |

## Storage

All node types use **redb** (pure Rust embedded database) with 14 tables:

| Table | Purpose |
|-------|---------|
| `BLOCKS_TABLE` | DAG blocks by hash |
| `STATE_TABLE` | World state by account address |
| `TX_TABLE` | Transactions by tx hash |
| `RECEIPTS_TABLE` | Receipts by tx hash |
| `VALIDATORS_TABLE` | Validator records |
| `VERKLE_TABLE` | Verkle tree nodes |
| `ACCOUNTS_TABLE` | Account records (balance, nonce) |
| `CONTRACT_CODE_TABLE` | WASM bytecode |
| `CONTRACT_STORAGE_TABLE` | Contract storage |
| `BATCH_ROOTS_TABLE` | State roots by anchor hash |
| `BATCH_INDEX_TABLE` | Batch number → anchor hash |
| `BATCH_TXS_TABLE` | Tx lists by anchor hash |
| `CHECKPOINTS_TABLE` | Weak subjectivity checkpoints |
| `EQUIVOCATION_PROOFS_TABLE` | Equivocation records |

## Network

**Gossipsub Topics:**
- `/aztibase/blocks/1.0.0`
- `/aztibase/transactions/1.0.0`
- `/aztibase/consensus/1.0.0`
- `/aztibase/state-sync/1.0.0`
- `/aztibase/ai-proofs/1.0.0`
- `/aztibase/validator-announce/1.0.0`
- `/aztibase/checkpoint-announce/1.0.0`

**Mesh Parameters:** target=3, low=2, high=12, outbound_min=1
**Max connections:** 50
**Heartbeat:** 500ms

## Serialization

| Context | Format |
|---------|--------|
| Internal storage (redb) | postcard |
| P2P wire format | postcard (length-prefixed) |
| RPC | JSON-RPC 2.0 |
| WASM bridge | postcard |

## Default Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 30333 | TCP + QUIC | P2P |
| 9944 | HTTP + WS | RPC |
| 9000 | UDP | WebRTC (optional) |
