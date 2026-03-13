---
title: Synaptic Consensus
description: DAG-based consensus mechanism with Proof of Useful Work
---

Aztibase uses **Synaptic Consensus**, a DAG-based Byzantine Fault Tolerant consensus protocol combining SynBFT (for ordering) and Proof of Useful Work (for AI compute incentives).

## How It Works

### DAG Structure

Unlike linear blockchains, Aztibase blocks form a **Directed Acyclic Graph** (DAG). Each block (vertex) references multiple parent blocks, allowing parallel block production by multiple validators.

```
Round 3:  [V1] ─── [V2] ─── [V3]
           │╲   ╲╱   │╲   ╲╱   │
Round 2:  [V1] ─── [V2] ─── [V3]
           │╲   ╲╱   │╲   ╲╱   │
Round 1:  [V1] ─── [V2] ─── [V3]
           │╲   ╲╱   │╲   ╲╱   │
Round 0:  [V1] ─── [V2] ─── [V3]
```

Each validator produces exactly one block per round. Blocks reference up to 20 parents from the last 16 rounds.

### Wave Commit Rule

Finality is achieved through **waves** — groups of 4 rounds:

```
Wave W:
  Round W×4:     Leader proposes anchor block
  Round W×4+1:   Voting (validators reference leader or not)
  Round W×4+2:   Decision round 1
  Round W×4+3:   Decision round 2
```

**Direct commit:** If >2/3 of stake at the voting round references the leader's anchor → the anchor is **committed**.

**Indirect commit:** If a later committed leader has the target anchor in its causal history → the target is also **committed**.

### Finality

- Best-case finality: **4 rounds × 400ms = ~1.6 seconds**
- Committed batch: anchor + all uncommitted ancestors, topologically sorted by (round ascending, hash ascending)

### Threshold Clock

Round advancement is message-driven, not timer-based:
- Advances when ≥2/3 of validator stake has produced blocks at the current round
- Fallback: force-advance after 10 seconds if quorum not reached

## Performance

| Metric | Value |
|--------|-------|
| Block time | 400 ms |
| Finality | ~1.6 seconds (4 rounds) |
| Throughput | 10,000+ TPS target |
| Observed (testnet) | ~26 blocks/sec on 3-node testnet |
| Liveness timeout | 10 seconds |

## Proof of Useful Work (PoUW)

Beyond standard block production, validators can earn additional rewards by contributing useful AI computation:

- **Task posting**: Users submit AI inference tasks on-chain
- **Compute commitment**: Validators with GPU/TPU resources commit to process tasks
- **Attestation**: Multiple validators verify inference results
- **Rewards**: 15% of block emissions go to PoUW participants

## Fault Tolerance

Synaptic Consensus tolerates up to **f < n/3** Byzantine validators (where n = total validators by stake weight).

### Slashing Conditions

| Violation | Penalty |
|-----------|---------|
| Equivocation (double-voting) | 10% of stake |
| Downtime (1,000+ rounds offline) | 0.5% of stake |

Equivocation proofs are stored on-chain and can be submitted by any node that detects the violation.

## Comparison

| Feature | Aztibase (SynBFT) | Ethereum (Gasper) | Sui (Mysticeti) | Monad (MonadBFT) |
|---------|-------------------|-------------------|-----------------|------------------|
| Structure | DAG | Linear chain | DAG | Linear chain |
| Finality | ~1.6s | ~12 min | ~0.5s | ~0.8s |
| Block time | 400ms | 12s | ~0.4s | 0.4s |
| BFT threshold | 2/3 stake | 2/3 stake | 2/3 stake | 2/3 stake |
| AI-native | Yes | No | No | No |
