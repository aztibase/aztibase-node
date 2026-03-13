# Aztibase Network — Architecture Reference

Quick-reference for all constants, data flows, and system behavior.
Auto-consult this file before searching the codebase for answers.

---

## Chain Identity

| Parameter | Value |
|-----------|-------|
| Chain ID | `0xA27B` (41595) |
| Binary | `aztibase` / `aztibase.exe` |
| Crate prefix | `aztibase-*` (9 crates) |
| RPC prefix | `aztb_` (48 methods) |
| Token | AZTB |
| Consensus | Synaptic Consensus (SynBFT + PoUW) — DAG-based |
| Hash | BLAKE3 |
| Signatures | Ed25519 (transactions), BLS12-381 (finality certs) |
| Finality domain | `b"AZTIBASE_FINALITY_V1"` |

---

## Block (Vertex) Structure

```
DagBlock {
    hash: [u8; 32],         // BLAKE3(round || author || parents || payload || timestamp)
    round: u64,
    author: [u8; 32],       // Validator pubkey (ValidatorId)
    parents: Vec<[u8;32]>,  // Multiple parents (DAG, not chain). Max 20.
    payload: Vec<u8>,       // Length-prefixed txs: [u32 len][bytes]...
    timestamp: u64,         // Unix milliseconds
}
```

Source: `crates/aztibase-consensus/src/dag.rs`

---

## Consensus Constants

| Parameter | Value | Source |
|-----------|-------|--------|
| Round duration | 400 ms | engine.rs:131 |
| Wave length | 4 rounds | engine.rs:132 |
| Liveness timeout | 10 s (round_duration × 25) | engine.rs:386 |
| Max parents per vertex | 20 | engine.rs:133 |
| Max pending txs | 4,096 | engine.rs:134 |
| Max payload per vertex | 256 KiB | engine.rs:796 |
| Parent lookback window | 16 rounds | engine.rs:180 |
| Equivocation prune depth | 100 rounds | engine.rs:18 |
| Max catchup rounds | 100 | engine.rs:19 |
| Checkpoint interval | 1,000 rounds | checkpoint.rs:5 |
| DAG retention buffer | 16 rounds | dag_store.rs:47 |
| Supermajority threshold | 2/3 of validator stake | commit.rs |

---

## Block Production Flow

1. **Threshold clock** advances when ≥2/3 stake has produced blocks at current round (message-driven, no fixed timer)
2. **Fallback**: force-advance after 10 seconds if quorum not reached
3. **Proposal**: `propose_vertex()` → select parents (up to 20 from last 16 rounds) → drain pending txs → create DagBlock → insert into DAG → gossip broadcast
4. **Blocks per round**: one per validator (3 validators = 3 blocks/round)
5. **Empty blocks**: YES — produced even with no pending txs to maintain clock progress
6. **Observed throughput**: ~660 blocks in 25s on 3-node testnet (~26 blocks/sec total)

---

## Wave Commit Rule (Finality)

```
Wave W = 4 rounds:
  Round W×4:     Leader proposes anchor
  Round W×4+1:   Voting (validators reference leader or not)
  Round W×4+2:   Decision round 1
  Round W×4+3:   Decision round 2

Direct commit: >2/3 stake at voting round references leader → COMMITTED
Indirect commit: later committed leader has target in causal history → COMMITTED
```

- Best-case finality: 4 rounds × 400ms = ~1.6 seconds
- Committed batch: anchor + all uncommitted ancestors, topologically sorted by (round asc, hash asc)

---

## Transaction Lifecycle

```
RPC (sendRawTransaction)
  → mempool_tx channel
  → main event loop validates via mempool.insert_checked()
  → gossip broadcast to peers
  → consensus_tx channel → engine.pending_txs queue
  → propose_vertex() packs txs as [u32 len][bytes]...
  → vertex broadcast via gossipsub
  → peers receive, verify, insert into DAG
  → wave commit rule triggers
  → extract_committed_batch() orders all vertices in causal history
  → pipeline.execute_batch() processes transactions:
      Phase 0: Nonce validation
      Phase 1: Gas escrow (total_cost = gas_limit × gas_price + value)
      Phase 2: Execute transaction
      Phase 3: Refund unused gas, collect fees
  → state flush to redb
```

---

## Transaction Types

| Byte | Kind | Gas Limit |
|------|------|-----------|
| 0x01 | Transfer | 21,000 |
| 0x02 | ContractDeploy | 100,000 |
| 0x03 | ContractCall | 50,000 |
| 0x04 | EvmDeploy | 100,000 |
| 0x05 | EvmCall | 50,000 |
| 0x06 | AiInfer | 100,000 |
| 0x07 | CreateAgent | 53,000 |
| 0x08 | RegisterModel | 100,000 |
| 0x09 | PostTask | 42,000 |
| 0x0A | SubmitAttestation | 50,000 |
| 0x0B | CommitCompute | 75,000 |
| 0x0C | DeregisterCompute | 50,000 |
| 0x0D | DeregisterModel | 60,000 |
| 0x0E | CreateProposal | 100,000 |
| 0x0F | CastVote | 40,000 |
| 0x10 | Stake | 60,000 |
| 0x11 | Unstake | 60,000 |
| 0x12 | Delegate | 60,000 |
| 0x13 | Undelegate | 60,000 |
| 0x14 | SetAgentPolicy | 60,000 |
| 0x15 | AgentExecute | 80,000 |
| 0x16 | AnchorL2State | 80,000 |
| 0x17 | BridgeDeposit | 50,000 |
| 0x18 | BridgeWithdraw | 70,000 |
| 0x19 | RegisterL2 | 100,000 |
| 0x1A | RotateValidatorKey | 60,000 |
| 0x1B | FaucetDrip | 30,000 |

Source: `crates/aztibase-execution/src/routing.rs`, `fee.rs`

---

## Gas & Fee Model (EIP-1559 style)

| Parameter | Value | Source |
|-----------|-------|--------|
| Min base fee | 1 | fee.rs:5 |
| Max base fee | 1,000,000,000 | fee.rs:6 |
| Target gas per batch | 15,000,000 | fee.rs:7 |
| Max gas per batch | 30,000,000 | fee.rs:8 |
| Base fee change denominator | 8 | fee.rs:9 |
| Escrow formula | `gas_limit × gas_price + value` | fee.rs |

---

## Size Limits

| Limit | Value | Source |
|-------|-------|--------|
| Max tx size (mempool) | 256 KiB | mempool.rs:14 |
| Max vertex payload | 256 KiB | engine.rs:796 |
| Max vertex wire size | 512 KiB | wire.rs:5 |
| Max gossipsub message | 2 MiB | gossip.rs:35 |
| Max snapshot file | 128 MiB | snapshot.rs:13 |
| Max RPC body | 1 MiB | config.rs:143 |
| Nonce gap limit | 16 ahead | mempool.rs:9 |

---

## Tokenomics

| Parameter | Value |
|-----------|-------|
| Total supply | 1,000,000,000 AZTB |
| Genesis mint | 400,000,000 (40%) |
| Emission pool | 600,000,000 (60%, ~10 years) |
| Initial annual emission | 120,000,000 (years 1-2) |
| Halving period | 2 years |
| Tail emission | 3,750,000/year (after year 10) |
| Rounds per year | 78,894,000 (at 400ms) |
| Epoch length | 1,000 committed rounds |

**Emission Distribution:**
- Validators: 70%
- PoUW (AI compute): 15%
- Treasury: 10%
- Insurance: 5%

---

## Staking

| Parameter | Value |
|-----------|-------|
| Min stake (default) | 50,000 AZTB |
| Min stake floor (governance) | 10,000 AZTB |
| Min stake ceiling (governance) | 500,000 AZTB |
| Max stake cap | 50,000,000 (5% of supply) |
| Unbonding period | 4,536,000 rounds (~21 days) |
| Equivocation slash | 10% |
| Downtime slash | 0.5% |
| Downtime threshold | 1,000 rounds |
| Default commission | 10% |
| Max APY | 12% |
| Min APY | 3% |

---

## Governance

| Parameter | Value |
|-----------|-------|
| Min voting period | 10 rounds |
| Max voting period | 10,000 rounds |
| Min voters for quorum | 2 |
| Max active proposals | 64 |
| Description length | 10–512 chars |

---

## L2 Bridge

| Parameter | Value |
|-----------|-------|
| Bridge finality | 100 L1 batches |
| Max registered L2s | 256 |
| Max sequencer set | 32 |
| Max anchor history | 10 |

---

## Network

**Gossipsub Topics:**
- `/aztibase/blocks/1.0.0`
- `/aztibase/transactions/1.0.0`
- `/aztibase/consensus/1.0.0`
- `/aztibase/state-sync/1.0.0`
- `/aztibase/ai-proofs/1.0.0`
- `/aztibase/validator-announce/1.0.0`
- `/aztibase/checkpoint-announce/1.0.0`

**Mesh:** target=3, low=2, high=12, outbound_min=1
**Heartbeat:** 500ms
**Duplicate cache:** 120s
**Min message sizes:** blocks=64B, tx=32B, consensus=32B
**Max connections:** 50

**Topic weights:** consensus=2.0, blocks/validator-announce/checkpoint=1.5, tx=1.0, state-sync/ai-proofs=0.5

---

## RPC

| Parameter | Value |
|-----------|-------|
| Default listen | 127.0.0.1:9944 |
| Rate limit | 100 req/s per IP |
| Faucet drip | 1,000,000 AZTB |
| Faucet cooldown | 60 seconds |
| Max WS connections | 256 |
| Max WS per IP | 8 |
| Max subscriptions/client | 16 |
| Max block range query | 100 |
| Chain ID | 0xA27B |

---

## Storage (redb Tables)

1. `BLOCKS_TABLE` — DAG blocks by hash
2. `STATE_TABLE` — World state by account address
3. `TX_TABLE` — Transactions by tx hash
4. `RECEIPTS_TABLE` — Receipts by tx hash
5. `VALIDATORS_TABLE` — Validator records
6. `VERKLE_TABLE` — Verkle tree nodes
7. `ACCOUNTS_TABLE` — Account records (balance, nonce)
8. `CONTRACT_CODE_TABLE` — WASM bytecode
9. `CONTRACT_STORAGE_TABLE` — Contract storage
10. `BATCH_ROOTS_TABLE` — State roots by anchor hash
11. `BATCH_INDEX_TABLE` — Batch number → anchor hash
12. `BATCH_TXS_TABLE` — Tx lists by anchor hash
13. `CHECKPOINTS_TABLE` — Weak subjectivity checkpoints
14. `EQUIVOCATION_PROOFS_TABLE` — Equivocation records

**Eviction limits:**
- MAX_EXECUTED_ANCHORS: 10,000 (ring buffer)
- MAX_STORED_TXS: 500,000
- MAX_STORED_BATCH_ROOTS: 100,000
- MAX_ATTESTATIONS_PER_TASK: 32
- MAX_ATTESTATION_BUFFER_TASKS: 2,048

---

## Browser Wallet Limits

| Parameter | Value |
|-----------|-------|
| Per-tx limit | 100 AZTB |
| Session spending limit | 10,000 AZTB |
| High-value warning | 1,000 AZTB |

---

## Snapshot Format

- Version: 3
- Max size: 128 MiB
- Integrity: BLAKE3 hash appended
- CLI export: `aztibase snapshot export --output <path>`
- CLI bootstrap: `aztibase --snapshot <path>`

---

## Crate Map

| Crate | Purpose |
|-------|---------|
| `aztibase-core` | Hash, types, BlockHash, ValidatorId |
| `aztibase-storage` | redb StateStore, table definitions |
| `aztibase-consensus` | DAG, engine, commit rule, wire, ordering, checkpoints, finality |
| `aztibase-execution` | Tx routing, fee model, state, staking, governance, tokenomics, AI market, L2 bridge, snapshots |
| `aztibase-network` | libp2p transport, gossipsub, kademlia, peer scoring |
| `aztibase-node` | Binary, pipeline, mempool, config, genesis, main event loop |
| `aztibase-rpc` | JSON-RPC server, all endpoints, WebSocket subscriptions |
| `aztibase-runtime` | Runtime environment |
| `aztibase-wasm` | Browser wallet, tx signing (WASM target) |

---

## Default Node Ports

| Port | Protocol | Purpose |
|------|----------|---------|
| 30333 | TCP + QUIC | P2P |
| 9944 | HTTP + WS | RPC |
| 9000 | UDP | WebRTC (optional, feature-gated) |

---

## Testnet Config (3-node local)

- Node1: P2P 30333, RPC 9944, boot_nodes=[]
- Node2: P2P 30334, RPC 9945, boot_nodes=[node1, node3]
- Node3: P2P 30335, RPC 9946, boot_nodes=[node1, node2]
- Start: `bash start-testnet.sh`
- Reset: `taskkill //F //IM aztibase.exe` then delete `data/node*/db`, `execution_db`, `*.redb`, `*.log`

---

## JSON-RPC Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |

---

## Serialization

| Context | Format | Source |
|---------|--------|--------|
| Internal storage (redb values) | postcard | ADR-024 |
| P2P wire format | postcard (length-prefixed) | wire.rs |
| RPC | JSON-RPC 2.0 | server.rs |
| WASM bridge | postcard | aztibase-wasm |
