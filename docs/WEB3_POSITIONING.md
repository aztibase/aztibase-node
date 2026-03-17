# Aztibase Network -- Web3 Positioning Brief

**Date:** March 16, 2026
**Version:** 1.0

> Aztibase Network is the first Layer-1 blockchain where AI inference is a protocol primitive, not an afterthought -- built for a world where centralized servers are a liability, not a convenience.

---

## The Landscape

Web3 is the architectural thesis that users should own their data, identity, and assets through cryptographic keys and decentralized protocols, not platform terms of service. Layer-1 blockchains are the infrastructure layer that makes this possible -- they replace centralized servers with validator networks, databases with on-chain state, and corporate logins with self-sovereign wallets.

As of March 2026, the L1 space has fragmented into specializations: DeFi chains optimizing for financial throughput, privacy protocols pursuing anonymity, DePIN networks coordinating physical hardware, and a growing cluster of projects claiming AI integration. The AI-infrastructure market exceeds $150 billion and is growing at 40%+ annually. Decentralized compute projects collectively hold $7 billion+ in market cap, led by Bittensor ($3B).

The convergence of AI and blockchain is inevitable. But integration so far has been shallow -- middleware layers, compute-only networks, or branding exercises stapled onto existing architectures. No production chain combines AI-native consensus, general-purpose smart contracts, protocol-level privacy, and genuine server-independence into a single coherent design.

Aztibase Network occupies this gap.

---

## What Aztibase Is

Aztibase is a Layer-1, AI-native, server-independent blockchain built entirely in Rust. It uses DAG-based consensus (Synaptic Consensus) to achieve sub-second finality at 10,000+ TPS, with no central points of failure.

AI lives inside the consensus mechanism, the state model, the execution layer, and the economic design. It is not a sidecar, a subnet, or a middleware integration. Removing AI from Aztibase would require redesigning the consensus, the state model, the execution layer, and the tokenomics simultaneously. That is what "first-class citizen" means.

The name encodes the architecture. Dendrites are the branching input structures of biological neurons that receive, integrate, and process signals from many sources. The DAG topology of Synaptic Consensus is a dendritic structure. The AI-native design mirrors how dendrites gather distributed intelligence. The name teaches what the chain does.

**Token:** AZTB | **Chain ID:** 0xA27B | **License:** Dual MIT / Apache-2.0

---

## Three Pillars

### 1. AI-Native Consensus

Every "AI blockchain" in 2024-2026 falls into one of three categories: AI-only networks with no general smart contracts (Bittensor), middleware that bridges AI to existing chains (Ritual), or token mergers that rebrand separate architectures under one ticker (ASI Alliance). None of them make the chain itself intelligent.

Aztibase takes a different path. Proof of Useful Work (PoUW) validators earn rewards for performing verified AI inference -- real computation, not hash puzzles. Validator scoring weights accuracy (40%), latency (30%), and availability (30%), preventing the garbage-output problem that plagues speed-only evaluation. The state model includes AIAgent accounts, ModelRegistry entries, InferenceRequest objects, and InferenceAttestation records as protocol-level primitives. Smart contracts call `ai_inference()` as a built-in operation. AI inference fees create deflationary pressure through a 5% burn mechanism.

Mining on Aztibase produces real value.

### 2. Server-Independence

Most blockchains claim decentralization but depend on centralized infrastructure for practical use. Over 70% of Ethereum transactions route through three RPC providers (Infura, Alchemy, QuickNode). Solana validators require datacenter hardware. Browser users interact through centralized APIs, never touching the P2P network directly.

Aztibase enforces server-independence as a hard architectural constraint. Every full node is its own RPC endpoint. Browser users run WASM-compiled light clients connecting via WebRTC directly to the P2P mesh -- no intermediary. Peer discovery uses Kademlia DHT, gossipsub, mDNS, and hardcoded bootstraps. NAT traversal uses incentivized decentralized relays. Verkle tree state proofs are constant-size, enabling mobile and browser verification without downloading full state. BLS aggregate signatures compress finality certificates from 4.3KB to 96 bytes.

If every centralized server on the internet goes offline, Aztibase keeps running.

### 3. Privacy as Protocol

Following the thesis that privacy creates irreversible network effects -- bridging tokens between chains is trivial, but bridging secrets is impossible -- Aztibase implements privacy as selective disclosure at the protocol layer. Once users commit private state, that privacy guarantee cannot follow them to another chain. This creates organic lock-in that is architecturally, not just economically, defensible.

Privacy is not full anonymity. It is user-controlled selective disclosure with regulatory compliance hooks: prove you are KYC-verified without revealing your identity. This positions the chain for enterprise and institutional adoption while preserving individual sovereignty.

### The Intersection Creates the Moat

Each pillar alone is achievable. Bittensor does AI. Bitcoin does server-independence. Zcash does privacy. No chain does all three, because the requirements conflict: PoUW pushes toward datacenter hardware (against server-independence), ZK proofs are compute-heavy (against consumer validation), and full self-verification conflicts with the trust assumptions that simplify AI and privacy.

Aztibase resolves these through architectural choices: PoUW is optional (the chain runs without it), Verkle proofs enable lightweight verification, and privacy is selective (public by default, private when chosen). Each feature reinforces the others rather than competing.

---

## Technical Foundation

| Component | Choice |
|-----------|--------|
| Consensus | Synaptic Consensus (SynBFT + PoUW), DAG-based |
| Block time | 400ms target, <1.6s finality (4 rounds) |
| Throughput | 10,000+ TPS at launch |
| VM | WASM (wasmtime) + EVM (revm) -- Rust, AssemblyScript, Solidity |
| State model | Hybrid account + object (parallel execution via Block-STM) |
| State proofs | Verkle trees (100x smaller than Merkle Patricia Tries) |
| Storage | redb (pure Rust embedded DB) |
| Hashing | BLAKE3 |
| Signatures | Ed25519 (transactions), BLS12-381 (finality) |
| P2P | rust-libp2p (QUIC + WebRTC) |
| Language | Rust -- pure, zero C/C++ dependencies |
| Architecture | 9 crates, modular monolith |
| License | Dual MIT / Apache-2.0 |
| Token supply | 1 billion hard cap, EIP-1559 burn, halving every 2 years |

---

## Competitive Landscape

| Category | Project | What They Do | What They Lack |
|----------|---------|--------------|----------------|
| AI-only network | Bittensor (TAO) | Compute marketplace, subnet architecture | No general smart contracts; speed-only scoring produces garbage outputs |
| AI bridge layer | Ritual | Middleware connecting AI to existing chains | Adds latency and trust assumptions; the chain itself is not intelligent |
| AI token merger | ASI Alliance (FET/AGIX/OCEAN) | Unified token across three projects | Cosmetic integration; three separate architectures underneath |
| Decentralized compute | Render, Akash | GPU rental networks | No consensus integration; no on-chain verification of inference results |
| General L1 | Solana, Sui, Aptos | High-TPS smart contract platforms | AI is an application, not a protocol primitive; centralized infra dependencies |

Aztibase is the only project where removing AI would require redesigning the consensus, state model, execution layer, and tokenomics simultaneously.

---

## Proof of Work Done

This is not a whitepaper. The code exists.

- **44,900+ lines** of production Rust across 9 crates
- **983 unit tests** passing, zero flaky, zero Clippy warnings
- **Live 4-node testnet** (3 validators + 1 full node) with public RPC endpoints
- **48 JSON-RPC methods** + WebSocket subscriptions, fully documented
- **WASM smart contract lifecycle** validated on testnet (deploy, initialize, execute, query)
- **Chrome wallet extension** (Manifest V3, 2FA via TOTP + WebAuthn, WASM staking signatures)
- **AI Sentinel Tier 1** shipped (15-feature heuristic health scorer, real-time chain monitoring)
- **Epoch rewards validated** on live testnet (emission math, per-validator splits, cross-node consistency)
- **Dynamic staking** validated (stake, unstake with unbonding, delegate -- all via CLI + RPC)
- **Block sync** validated (full node catches up 0 to 434 blocks in ~45 seconds)
- **32 Architecture Decision Records** documenting every non-trivial technical choice

---

## What Comes Next

- **Friends testnet:** Multi-machine testing across geographic locations via Tailscale mesh
- **AI Sentinel Tier 2:** ML-based anomaly detection replacing heuristic scoring
- **Public testnet:** Open validator onboarding with documentation and tooling
- **Mainnet preparation:** M9 milestone in progress -- security audit, genesis configuration, launch sequencing
- **Token economics:** Already implemented and validated on testnet (emission schedules, epoch rewards, staking, delegation, EIP-1559 burn)

---

## Links

- **Public RPC:** rpc.aztibase.com
- **GitHub Releases:** github.com/aztibase/aztibase-node
- **Documentation:** docs site (Astro Starlight, self-hosted)
- **Domain:** aztibase.com
- **Entity:** Aztibase (Pty) Ltd, South Africa (CIPC registered)
