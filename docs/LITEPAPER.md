# Aztibase Network — Litepaper

**Version 1.0 — March 2026**
**aztibase.com | @aztibase**

---

## The Problem

AI agents need a blockchain that understands them. Current chains treat AI as an afterthought — inference happens off-chain, verification is impossible, and 12-second block times make per-inference micropayments impractical.

Aztibase is the first Layer 1 blockchain built from the ground up for AI agent economies.

---

## What Makes Aztibase Different

### AI-Native at the Protocol Level

Every Aztibase node runs an ONNX inference engine. AI isn't a smart contract bolted on top — it's part of the node software itself.

- **Model Registry**: Register AI models on-chain with metadata, pricing, and version tracking
- **Inference Execution**: Nodes execute AI inference tasks and produce verifiable results
- **AI Sentinel**: An autonomous health monitor that watches the chain using machine learning, detecting anomalies in real time and taking corrective action
- **Agent Identity**: On-chain accounts for autonomous AI agents with capability policies and spending limits

### Dual Virtual Machine

Aztibase runs both EVM and WASM smart contracts side by side, with a cross-VM bridge enabling calls between them:

- **EVM (revm)**: Deploy Solidity contracts. Compatible with existing Ethereum tooling
- **WASM (wasmtime)**: Deploy Rust, C, or AssemblyScript contracts for maximum performance
- **Cross-VM Bridge**: WASM contracts can call EVM contracts and vice versa

### DAG-Based Consensus (Synaptic Consensus)

Unlike linear blockchains, Aztibase uses a Directed Acyclic Graph where multiple blocks can be proposed simultaneously:

- **400ms block time** (vs 12s Ethereum, 400ms Solana)
- **Sub-1.6s finality** (4 consensus rounds)
- **No leader bottleneck**: All validators propose blocks concurrently
- **Byzantine fault tolerant**: Tolerates up to 1/3 malicious validators

### Server Independence

The network operates without any centralized infrastructure:

- Pure P2P via libp2p (Kademlia DHT, gossipsub, QUIC + WebRTC)
- Every node is its own RPC endpoint
- No bootstrap server dependency after initial peer discovery
- Browser nodes via WASM + WebRTC (no server relay)

---

## Architecture

```
                    Aztibase Node
    ┌──────────────────────────────────────┐
    │  Consensus (DAG-BFT)                 │
    │  ├── Vertex proposal + voting        │
    │  ├── Finality certificates           │
    │  └── Equivocation detection          │
    │                                      │
    │  Execution Pipeline                  │
    │  ├── 27+ transaction types           │
    │  ├── EVM (revm) + WASM (wasmtime)    │
    │  ├── Block-STM parallel execution    │
    │  ├── EIP-1559 dynamic gas pricing    │
    │  └── AI inference routing            │
    │                                      │
    │  Storage (redb)                      │
    │  ├── Verkle tree state commitments   │
    │  ├── Account + contract storage      │
    │  └── Pure Rust, no C dependencies    │
    │                                      │
    │  Networking (libp2p)                 │
    │  ├── QUIC + WebRTC transport         │
    │  ├── Gossipsub message propagation   │
    │  ├── Block sync with pipelining      │
    │  └── Peer reputation scoring         │
    │                                      │
    │  AI Runtime (tract/ONNX)             │
    │  ├── Model registry                  │
    │  ├── Inference task marketplace      │
    │  ├── Anomaly detection scorer        │
    │  └── Sentinel autonomous monitor     │
    │                                      │
    │  JSON-RPC + WebSocket API            │
    │  └── 57+ methods, Prometheus metrics │
    └──────────────────────────────────────┘
```

### What's Built and Running

| Component | Status | Details |
|-----------|--------|---------|
| DAG Consensus | Live | Sub-second finality on public testnet |
| EVM Contracts | Live | Deploy, call, read via aztb_call/eth_call |
| WASM Contracts | Live | Full deploy/call with fuel metering |
| Cross-VM Bridge | Live | Bidirectional EVM-WASM calls |
| Block-STM Parallelism | Live | Concurrent transfer execution |
| AI Sentinel | Live | Autonomous health scoring, 3-tier action engine |
| Dynamic Gas (EIP-1559) | Live | Adaptive base fee |
| DEX (AMM) | Live | WASZTB/tUSDC pair with liquidity |
| Oracle | Live | CoinGecko price feed, on-chain updates |
| Block Sync | Live | 5x pipelined catch-up, validator recovery |
| TypeScript SDK | Published | @aztibase/sdk, 21 tests |
| Chrome Wallet | Published | Extension with staking UI |
| Indexer + Explorer | Live | SQLite-backed, REST API |

---

## Tokenomics

### AZTB Token

| Parameter | Value |
|-----------|-------|
| Total Supply | 1,000,000,000 AZTB |
| Genesis Mint | 400,000,000 (40%) |
| Emission Pool | 600,000,000 (60%) over ~10 years |
| Block Time | 400ms |
| Consensus | DAG-BFT (Synaptic Consensus) |

### Genesis Allocation (400M)

| Category | Amount | Vesting |
|----------|--------|---------|
| Protocol Treasury | 100M (25%) | Immediate, governance-controlled |
| Ecosystem Development | 80M (20%) | 6-month cliff, 4-year linear |
| Core Team | 60M (15%) | 12-month cliff, 4-year linear |
| Foundation Reserve | 40M (10%) | 2-year lock, 3-year linear |
| Community Airdrop | 40M (10%) | Immediate |
| Validator Bootstrap | 40M (10%) | 6-month linear |
| AI Ecosystem Fund | 20M (5%) | 6-month cliff, 3-year linear |
| Liquidity Provision | 20M (5%) | Immediate |

### Emission Schedule

Validator rewards, AI compute incentives, and treasury funding come from the emission pool:

| Years | Annual Emission | Cumulative |
|-------|----------------|------------|
| 1-2 | 120M/year | 240M |
| 3-4 | 60M/year | 360M |
| 5-6 | 30M/year | 420M |
| 7-8 | 15M/year | 450M |
| 9-10 | 7.5M/year | 465M |
| 11+ | 3.75M/year (tail) | Approaches 600M |

### Emission Distribution

| Recipient | Share |
|-----------|-------|
| Validator Rewards | 70% |
| AI Compute (PoUW) | 15% |
| Protocol Treasury | 10% |
| Staking Insurance | 5% |

### Staking

| Parameter | Value |
|-----------|-------|
| Minimum Stake | 50,000 AZTB (governance range: 10K-500K) |
| Maximum Stake | 50,000,000 AZTB (5% of supply) |
| Unbonding Period | 21 days |
| Target APY | 3-12% |
| Equivocation Slash | 10% |
| Downtime Slash | 0.5% |

---

## AI Integration

### Three-Layer AI Stack

**Layer 1 — Anomaly Detection (per-transaction)**
Every transaction is scored for anomalous behavior using an ONNX autoencoder model running inside the node. High scores flag potential attacks or unusual patterns.

**Layer 2 — Sentinel Health Monitor (per-epoch)**
The AI Sentinel aggregates network metrics (commit latency, TPS, peer count, equivocations) and produces a chain health score. It operates in three tiers:
- Tier 1: Observe and report
- Tier 2: ONNX model-based scoring with confidence levels
- Tier 3: Autonomous action (throttle gas, alert validators, trigger emergency responses)

**Layer 3 — Compute Marketplace**
Model creators register models on-chain. Users post inference tasks with reward amounts. Compute providers execute tasks, submit attestations, and receive payment. The chain verifies results.

### Why This Matters

AI agents generating autonomous transactions need a chain that can:
1. Verify inference results (not just trust an oracle)
2. Settle micropayments per-inference (400ms finality enables this)
3. Manage agent identity and spending policies on-chain
4. Detect and respond to anomalous agent behavior automatically

No other L1 has this stack built into the protocol layer.

---

## Technology

| Component | Choice | Why |
|-----------|--------|-----|
| Language | Rust | No GC, memory safety, WASM target |
| Async Runtime | Tokio | Production-grade async I/O |
| P2P | libp2p | NAT traversal, QUIC, WebRTC |
| EVM | revm | Production Ethereum execution engine |
| WASM VM | wasmtime | Fastest WASM runtime |
| Storage | redb | Pure Rust, ACID, no C dependencies |
| State | Verkle trees | Constant-size proofs |
| Hashing | BLAKE3 | Fastest cryptographic hash |
| Signatures | Ed25519 | Industry standard |
| AI Runtime | tract (ONNX) | Pure Rust inference, no Python |
| Serialization | postcard | Compact binary, no-std compatible |

### Codebase

- 9 Rust crates in monorepo
- 1,040+ tests, zero flaky
- Zero clippy warnings
- 34 Architecture Decision Records
- MIT / Apache-2.0 dual license

---

## Roadmap

### Completed

- Layer 1 blockchain (consensus, execution, networking, storage)
- Public testnet on VPS with monitoring (Grafana + VictoriaMetrics)
- Dual VM (EVM + WASM) with cross-VM bridge
- AI Sentinel with ONNX inference
- On-chain governance (proposals, voting, parameter changes)
- L2 bridge primitives (RegisterL2, AnchorL2State, BridgeDeposit, BridgeWithdraw)
- AMM DEX with liquidity pools
- Oracle with live price feeds
- TypeScript SDK, Chrome wallet extension
- Block explorer and indexer

### Next

- Security audit
- Mainnet launch with 100+ validators
- AI Model Marketplace (register, price, and trade AI models)
- Agent Identity Registry (on-chain DID for AI agents)
- Content Provenance Registry (C2PA-style proof of AI-generated content)
- Name Service (.aztb domains)
- Community Task Chain (L2 for AI compute tasks)

---

## How to Participate

### Run a Validator
Download the binary from GitHub, stake AZTB, and start earning rewards.
Minimum hardware: 2 vCPU, 4 GB RAM, 50 GB SSD (~$4/month on Hetzner).

### Build on Aztibase
Deploy EVM (Solidity) or WASM (Rust) contracts. Full TypeScript SDK available.

### Join the Community
- Website: aztibase.com
- Twitter: @aztibase
- GitHub: github.com/aztibase/aztibase-node

---

## Team

Built by a solo engineer in South Africa. Every line of code — consensus, networking, execution, AI, wallet, explorer, SDK — written from scratch in Rust.

Aztibase (Pty) Ltd is registered with the South African CIPC.

---

**Aztibase Network — The Chain Where AI Agents Live**
