---
title: Introduction
description: What is Aztibase Network and why it exists
---

Aztibase is a Layer-1, AI-native, server-independent blockchain built entirely in Rust. It uses DAG-based consensus (Synaptic Consensus) to achieve sub-second finality at 10,000+ TPS, with no central points of failure.

## What Makes Aztibase Different

### Server-Independence
Every component of Aztibase is designed to function if all centralized servers go offline. Peer discovery uses Kademlia DHT, transport runs over QUIC and WebRTC, and block propagation uses gossipsub mesh networking. There are no DNS dependencies, no API gateways, and no centralized relay servers in the critical path.

### DAG Consensus
Unlike linear blockchains where blocks form a single chain, Aztibase blocks (called vertices) have multiple parents, forming a Directed Acyclic Graph. This means multiple validators can produce blocks simultaneously without conflicts, dramatically increasing throughput.

The consensus mechanism — **Synaptic Consensus (SynBFT)** — achieves finality in 4 rounds (~1.6 seconds) with a 2/3 supermajority stake threshold.

### AI-Native Design
AI capabilities are built into the protocol layer, not added as an afterthought:
- **Model Registry**: On-chain registration of AI models with versioning
- **Compute Market**: Decentralized marketplace for AI inference providers
- **Attestation Protocol**: Cryptographic verification that off-chain AI computations were performed correctly
- **Proof of Useful Work (PoUW)**: Validators earn additional rewards by contributing AI compute

### Dual VM
Aztibase runs both WASM (via wasmtime) and EVM (via revm) execution environments side by side. Developers can deploy smart contracts in Rust, AssemblyScript, or Solidity without needing bridges between the two ecosystems.

## Architecture at a Glance

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| Async Runtime | tokio |
| P2P | rust-libp2p (QUIC + WebRTC) |
| Consensus | Synaptic Consensus (SynBFT + PoUW) |
| VM | wasmtime (WASM) + revm (EVM) |
| Storage | redb (pure Rust embedded DB) |
| State | Verkle trees |
| Hashing | BLAKE3 |
| Signatures | Ed25519 (txs), BLS12-381 (finality) |
| License | Dual MIT / Apache-2.0 |

## Chain Identity

| Parameter | Value |
|-----------|-------|
| Token | AZTB |
| Chain ID | `0xA27B` (41595) |
| RPC Prefix | `aztb_` |
| Default P2P Port | 30333 |
| Default RPC Port | 9944 |

## Crate Map

Aztibase is organized as a Rust workspace with 9 crates:

| Crate | Purpose |
|-------|---------|
| `aztibase-core` | Hash types, BlockHash, ValidatorId |
| `aztibase-storage` | redb StateStore, table definitions |
| `aztibase-consensus` | DAG, engine, commit rule, wire protocol, checkpoints |
| `aztibase-execution` | Tx routing, fees, staking, governance, tokenomics, AI market |
| `aztibase-network` | libp2p transport, gossipsub, kademlia, peer scoring |
| `aztibase-node` | Binary, pipeline, mempool, config, genesis, event loop |
| `aztibase-rpc` | JSON-RPC server, WebSocket subscriptions |
| `aztibase-runtime` | Runtime environment |
| `aztibase-wasm` | Browser wallet, tx signing (WASM target) |

## Next Steps

- [Quick Start](/guides/quickstart/) — Build from source and run a local node
- [Run a Validator](/guides/run-validator/) — Join the testnet as a validator
- [Architecture Overview](/architecture/overview/) — Deep dive into the system design
- [API Reference](/api/rpc/) — Full JSON-RPC documentation
