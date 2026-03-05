# DENDRITE NETWORK -- GENESIS CHAIN MASTER PLAN

**Version:** 1.0
**Date:** March 5, 2026
**Status:** COMPLETE -- Phase 6 Final Deliverable
**Assembled by:** blockchain-architect
**Classification:** Definitive Blueprint for Implementation

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Blockchain Name](#2-blockchain-name)
3. [Coin Name and Ticker](#3-coin-name-and-ticker)
4. [Trademark Registration Plan](#4-trademark-registration-plan)
5. [Regulatory Compliance Notes](#5-regulatory-compliance-notes)
6. [Consensus Mechanism](#6-consensus-mechanism)
7. [Node Architecture](#7-node-architecture)
8. [Tokenomics](#8-tokenomics)
9. [AI Integration Design](#9-ai-integration-design)
10. [Smart Contract Layer](#10-smart-contract-layer)
11. [P2P Network Design](#11-p2p-network-design)
12. [Security Model](#12-security-model)
13. [Technology Stack Summary](#13-technology-stack-summary)
14. [Open-Source Licensing Strategy](#14-open-source-licensing-strategy)
15. [4-Week Build Roadmap](#15-4-week-build-roadmap)
16. [What Makes Dendrite Network Unique](#16-what-makes-dendrite-network-unique)
17. [Unresolved Items and Risks](#17-unresolved-items-and-risks)

---

## 1. Executive Summary

Dendrite Network is a Layer-1 blockchain designed from the ground up to treat artificial intelligence as a first-class protocol citizen. It is not another chain that "adds AI" as a marketing feature bolted onto conventional architecture. AI lives inside the consensus mechanism, the state model, the smart contract execution layer, and the economic design. Every architectural decision -- from the DAG-based consensus to the five-node-type hierarchy to the burn mechanics -- exists to serve three converging theses: that AI compute will become the most demanded commodity on the internet, that privacy creates irreversible network effects, and that true decentralization means zero dependency on centralized servers.

The blockchain industry as of March 2026 is crowded with generalist chains competing on speed metrics and EVM compatibility. Meanwhile, AI-blockchain integration remains shallow: Bittensor runs an AI-only network with no general smart contracts; Ritual bridges AI to existing chains through an intermediary layer; the ASI Alliance merged three separate architectures without unifying them. No production chain combines AI-native consensus, general-purpose smart contracts, protocol-level privacy, and genuine server-independence into a single coherent design.

Dendrite Network occupies this gap.

**Core architecture at a glance:**

- **Consensus:** Synaptic Consensus (SynBFT + PoUW) -- a DAG-based BFT protocol with Proof of Useful Work overlay for AI inference verification. Sub-second deterministic finality. 10,000+ TPS at launch.
- **State model:** Hybrid account + object model enabling parallel execution (Block-STM pattern) with first-class AI agent accounts.
- **Smart contracts:** Dual VM (WASM via wasmtime + EVM via revm). No custom language. Rust, AssemblyScript, and Solidity supported from day one.
- **AI integration:** Three-layer AI system (protocol, smart contract, network). Off-chain inference with on-chain verification. AI compute marketplace with 85/10/5 fee split (compute provider / protocol / model creator).
- **Privacy:** Selective disclosure model with ZK proofs. Not full anonymity -- user-controlled privacy with regulatory compliance hooks.
- **P2P:** QUIC-primary networking via rust-libp2p with NetworkTransport abstraction. Six-layer bootstrap strategy. Browser nodes via WebRTC. No central servers after bootstrap.
- **Token:** 1 billion hard cap. EIP-1559 base fee burn. Path to deflationary equilibrium by year 5-7.

The name "Dendrite" reflects the chain's identity: dendrites are the branching input structures of biological neurons that receive, integrate, and process signals from many sources. The DAG topology of Synaptic Consensus is a dendritic structure. The AI-native design mirrors how dendrites gather distributed intelligence. The name teaches what the chain does.

---

## 2. Blockchain Name

**Name:** Dendrite Network
**Legal Status:** CONDITIONAL GREEN (cleared by legal-ip-counsel, March 5, 2026)

The name scored 78/100 in the naming council's evaluation -- the highest of all candidates assessed. It was selected over Quen (QEN), which held full GREEN legal clearance but scored only 57/100 due to a devastating phonetic collision with Alibaba's Qwen AI model family (both pronounced /kwen/).

### The 5 Legal Conditions for GREEN Clearance

1. **TICKER:** Must use DNDR, not DND. The DND ticker is blocked by Diamond DND, DungeonSwap, and TSX:DND. DNDR is confirmed clear across all major crypto and stock exchanges. STATUS: SATISFIED.

2. **COEXISTENCE AGREEMENT:** Before filing a USPTO trademark application in Class 9, proactively contact Dendrite Systems Inc. to negotiate a coexistence agreement. The company is unfunded with an archived product. Estimated cost: $5,000-$15,000.

3. **TRADEMARK FILING STRATEGY:** File Classes 36 and 42 first (minimal overlap with Dendrite Systems Inc.), then file Class 9 only after the coexistence agreement is secured. Use narrowly drafted goods/services descriptions: "Blockchain protocol software for decentralized consensus, cryptocurrency transaction processing, and distributed AI computation."

4. **DOMAIN ACQUISITION:** Before public announcement, secure at minimum one primary domain. Priority: dendrite.network, then dendritenetwork.io, dendrite.xyz, or dendritenetwork.com.

5. **MATRIX DENDRITE AWARENESS:** The Matrix homeserver project also uses the name "Dendrite." This is not a trademark obstacle but creates minor developer community confusion. Include a brief FAQ entry in early community materials.

### Why "Dendrite" is the Right Name

- The branching structure of biological dendrites maps directly to the DAG consensus topology.
- Dendrites are the receiving structures of neurons -- they gather signals from many sources and integrate them. This mirrors the chain's distributed AI computation model.
- The name signals genuine neuroscience literacy, not marketing-grade AI buzzwords.
- No phonetic collision with any major tech or crypto brand.
- Extraordinary visual design potential: branching dendritic structures provide natural logo and identity material.

**Tagline:** "Where Intelligence Branches"

---

## 3. Coin Name and Ticker

**Coin Name:** Dendrite
**Ticker:** DNDR
**Legal Status:** CLEAR -- DNDR is unused across all crypto registries (CoinMarketCap, CoinGecko) and all major stock exchanges.

The blockchain is "Dendrite Network." The coin is "Dendrite." The ticker is "DNDR." This unified branding keeps the identity simple: the network is the brand, the coin carries the brand name, the ticker is distinctive and collision-free.

**Denomination:** 18 decimal places (consistent with Ethereum/ERC-20 tooling compatibility). The smallest unit name will be determined during community engagement.

---

## 4. Trademark Registration Plan

### Filing Strategy

Classes 36 (cryptocurrency financial services) and 42 (blockchain SaaS platform) are filed first because they have minimal overlap with Dendrite Systems Inc.'s Class 9 marks. Class 9 (blockchain protocol software) follows only after the coexistence agreement is secured.

### Registration Roadmap

| Priority | Jurisdiction | Classes | Timeline | Est. Cost |
|----------|-------------|---------|----------|-----------|
| 1 | USPTO (United States) | 36, 42 (ITU) | Within 2 weeks of legal clearance | $2,500-$5,000 |
| 2 | EUIPO (European Union) | 9, 36, 42 | Within 4 weeks | $3,000-$6,000 |
| 3 | CIPC (South Africa) | 9, 36, 42 | Within 6 weeks | $1,500-$3,000 |
| 4 | WIPO Madrid Protocol | 9, 36, 42 | Within 8 weeks | $5,000-$10,000 |
| 5 | USPTO Class 9 | 9 | After coexistence agreement | $1,000-$2,000 |

**Total estimated trademark budget: $13,000-$26,000**

### Immediate Actions

1. Engage trademark attorney for Dendrite Systems Inc. coexistence negotiation.
2. Confirm DNDR ticker clean across all crypto and stock exchanges.
3. Acquire primary domain (dendrite.network preferred).
4. Reserve social handles: @DendriteNetwork on X/Twitter, GitHub (org), Discord, Telegram.

---

## 5. Regulatory Compliance Notes

### Howey Test Assessment (United States)

| Prong | Assessment |
|-------|-----------|
| Investment of money | LIKELY MET -- purchasing tokens with fiat or crypto qualifies. |
| Common enterprise | PARTIALLY MET -- token holders' fortunes are linked to network success, but no pooling of funds under a common promoter. |
| Expectation of profit | RISK AREA -- mitigated by genuine utility (gas, staking, AI inference, governance) and EIP-1559 burn mechanism. |
| Efforts of others | WEAKEST PRONG -- once decentralized, profits derive from the network's operation, not a central team. 4-year team vesting, governance-controlled treasury, and 5% stake cap demonstrate decentralization. |

**Overall:** MEDIUM RISK during initial distribution, LOW RISK post-decentralization. The token may initially be classified as a security and should transition to a non-security digital commodity after sufficient decentralization (target: 12-18 months).

**Key mitigations:** No ICO or public token sale. Fair launch model (airdrop + validator bootstrap). Algorithmic emission with no team discretion. Team tokens locked 12 months.

### FSCA Requirements (South Africa)

- Crypto assets are classified as financial products under FAIS since October 2022.
- If the entity is SA-based, CASP licensing is mandatory (FSP license from FSCA).
- Key personnel must pass regulatory examinations.
- Travel Rule applies to transfers above ZAR 25,000 (~USD 1,500).
- Determine whether the protocol development entity qualifies as a CASP.

### MiCA Compliance (European Union)

- If serving EU users, CASP authorization required before July 1, 2026.
- Must prepare and publish a MiCA-compliant crypto-asset white paper before any token offering in the EU.
- Establish an EU legal entity in a crypto-friendly jurisdiction (Lithuania, Malta, France, or Germany).
- Budget EUR 150,000 for CASP capital requirements.

### Recommended Entity Structure

- **Protocol development:** Non-CASP entity (if structured as open-source software development only).
- **Foundation:** Swiss or Cayman Islands foundation for governance and treasury management.
- **EU operations:** EU-based entity for CASP licensing.
- **SA operations:** SA-based entity if team is SA-based, with FSCA licensing.

---

## 6. Consensus Mechanism

### Synaptic Consensus: SynBFT + PoUW

Synaptic Consensus is a two-layer consensus design named for how biological synapses form connections -- many-to-many, parallel, strengthening through repeated validated signaling.

**Layer A -- SynBFT (Security backbone):**
A DAG-based Byzantine Fault Tolerant protocol providing ordering and absolute finality. ALL validators participate. The design draws from Mysticeti (uncertified DAG for reduced latency), Bullshark (anchor-based commit rule), and Shoal++ (pipelining), but copies none. The key innovation is an "AI reputation signal" integrated into leader scoring: validators who deliver verified, high-quality PoUW results receive a bounded reputation bonus (capped at 15% of total anchor score) that increases their anchor proposer selection probability.

**Layer B -- PoUW (AI value layer):**
A Proof of Useful Work overlay where validators with AI compute resources opt in to process inference requests. PoUW does NOT determine block validity. It produces InferenceAttestation objects included in blocks. The chain functions identically whether 0% or 100% of validators opt into PoUW.

### How SynBFT Works

1. **Rounds:** SynBFT operates in 400ms rounds. Each round, every active validator may broadcast one vertex (block) containing transactions and optionally AI attestations.
2. **DAG structure:** Each vertex references at least 2f+1 parent vertices from the previous round. Validators propose in parallel -- no single leader bottleneck.
3. **Uncertified DAG:** Following Mysticeti, validators reference vertices without waiting for quorum acknowledgment, eliminating one full round-trip.
4. **Anchor ordering:** Every 4th round, a VRF-elected anchor proposer's vertex commits a batch of DAG vertices to a deterministic total order.
5. **Anchor selection:** VRF-based, weighted by AnchorScore = 70% StakeWeight + 15% ConsensusReputation + 15% PoUWReputation.
6. **Commit rule:** An anchor commits when 2f+1 validators in the next round reference a causal path including it. Committed transactions are irreversible.

### Key Parameters

| Parameter | Value |
|-----------|-------|
| Round time | 400ms |
| Target finality | <800ms (normal), <2s (degraded) |
| Throughput | 10,000+ TPS at launch |
| Validator set | 100-200 active (21 minimum, 400 maximum) |
| Stake cap | 5% per validator |
| Diversity quota | 30% of slots for non-top-20 validators |
| Epoch length | 1000 rounds (~400 seconds) |
| Unbonding period | 21 days |
| Finality type | Absolute (deterministic BFT), not probabilistic |

### PoUW Verification Methods

Inference results are verified without full re-execution through three methods (per-request selectable):

1. **MPRE (Multi-Party Redundant Execution):** 2-of-3 agreement among independent executors. Simplest, most immediately deployable. Floating-point non-determinism handled via 16-bit fixed-point quantization for comparison.
2. **TEE Attestation:** Hardware-signed proof of correct model loading, input, and output. Medium trust assumption.
3. **ZK Proofs of Inference (zkML):** Mathematical proof of correct computation. Highest trust, highest overhead. Practical for small models today; large model support improving.

### PoUW Quality Metrics

Validators are scored on four dimensions to avoid Bittensor's speed-only metric problem:

| Metric | Weight | Description |
|--------|--------|-------------|
| Correctness | 40% | Agreement with other attestations or valid proof |
| Availability | 25% | Fraction of assigned requests completed on time |
| Latency | 20% | Time from assignment to submission, normalized by model complexity |
| Diversity | 15% | Willingness to serve diverse model types, not just cheap ones |

---

## 7. Node Architecture

Dendrite Network defines five node types. Every type is a first-class protocol citizen. No type requires a central server. The design enables graceful upgrade: a browser light client can upgrade to a full validator without re-syncing from scratch.

### Node Type Specifications

| Node Type | Purpose | CPU | RAM | Storage | Network | Key Technology |
|-----------|---------|-----|-----|---------|---------|----------------|
| **Full Node** | Complete state, validation, local RPC | 4+ cores | 8 GB min | 100 GB SSD | 25 Mbps | RocksDB (dual storage), Verkle trees |
| **Light Node** | Header-only, Verkle proof verification | 2+ cores | 512 MB min | 2 GB | 1 Mbps | redb, BLS aggregate finality certs |
| **Browser Node** | WASM-compiled light client in browser tab | Browser | Browser tab | IndexedDB (<50MB) | 1 Mbps | WASM (wasm32), WebRTC, IndexedDB |
| **Mobile Node** | Light client for smartphones | ARM64 | 256 MB app | 100 MB | Intermittent | Rust core via JNI/FFI, redb |
| **Validator** | Full node + consensus participation + optional PoUW | 8+ cores | 16-64 GB | 250-500 GB NVMe | 100 Mbps | Ed25519 + BLS12-381 keys, VRF, optional GPU |

### PoUW Validator Tiers

| Tier | Hardware | Models Supported | Earnings Premium |
|------|----------|-----------------|-----------------|
| Tier 1 (CPU) | 16+ cores, 32GB RAM | Small classifiers, anomaly detection (<100M params) | 1.2-1.5x |
| Tier 2 (Consumer GPU) | RTX 3060+ (8GB VRAM) | Medium NLP, image classification (<1B params) | 2-3x |
| Tier 3 (Professional GPU) | A10/A100 (24GB+ VRAM) | Large language models (1-7B params) | 4-8x |

### Server-Independence Design

- **Every full node is its own RPC endpoint.** No Infura/Alchemy dependency.
- **Browser nodes connect directly to P2P via WebRTC.** No centralized intermediary.
- **6-layer bootstrap** ensures network joinability even under censorship: hardcoded peers, DNS seeds, Kademlia DHT, mDNS, QR code peer exchange, peer caching.
- **Validators behind consumer NAT are explicitly supported.** ICE/STUN handles ~85% of NATs; incentivized TURN relays handle the rest.

### Graceful Upgrade Path

A user can start with a browser light client, upgrade to a desktop light node (redb-based), then to a full node (RocksDB), and finally to a validator -- each step adding capability without discarding previous state. The Rust core is shared across light, mobile, and full node implementations.

### Storage Architecture

Dual storage following Sei's production-proven pattern:
- **State Store (RocksDB cf_state_store):** Raw key-value optimized for read/write latency. Used by the execution engine.
- **State Commitment (RocksDB cf_state_commitment):** Verkle tree structure for proof generation. Updated asynchronously after execution to reduce I/O contention.

Six column families: state_store, state_commitment, dag_vertices, transactions, receipts, ai_attestations. Per-family compression (LZ4 for hot data, Zstd for cold). Target: <100GB first year storage.

---

## 8. Tokenomics

### Supply Model

**Total Supply: 1,000,000,000 (1 billion). Hard cap. No governance override.**

The hard cap is non-negotiable: Terra's collapse proved that unbounded minting creates death spiral risk. A fixed supply with algorithmic emission avoids the "efforts of others" prong of the Howey Test and provides economic predictability.

### Genesis Allocation (40% of total supply)

| Allocation | Tokens | % of Total | Vesting |
|------------|--------|-----------|---------|
| Protocol Treasury | 100,000,000 | 10% | Governance-controlled |
| Ecosystem Development | 80,000,000 | 8% | 4-year linear, 6-month cliff |
| Core Team & Contributors | 60,000,000 | 6% | 4-year linear, 12-month cliff |
| Foundation Reserve | 40,000,000 | 4% | 2-year lock, then 3-year linear |
| Community Airdrop | 40,000,000 | 4% | 50% immediate, 50% locked 90 days |
| Validator Bootstrap Pool | 40,000,000 | 4% | 6-month linear, staking required |
| AI Ecosystem Fund | 20,000,000 | 2% | 3-year linear, 6-month cliff |
| Liquidity Provision | 20,000,000 | 2% | Immediate |

**Day-1 circulating supply: 40,000,000 tokens (4%)** -- deliberately low to prevent sell pressure.

### Emission Schedule (60% of total supply over ~10 years)

Disinflationary 2-year halving:

| Year | Annual Emission | Cumulative | Inflation Rate |
|------|----------------|------------|---------------|
| 1-2 | 120,000,000/yr | 240,000,000 | ~23-30% |
| 3-4 | 60,000,000/yr | 360,000,000 | ~8-9% |
| 5-6 | 30,000,000/yr | 420,000,000 | ~3-4% |
| 7-8 | 15,000,000/yr | 450,000,000 | ~1.8% |
| 9-10 | 7,500,000/yr | 465,000,000 | ~0.9% |

Tail emission continues at halving rates until cap is reached (~year 46). Validators always receive some block reward.

**Emission distribution per epoch:** 70% validator rewards, 15% PoUW pool, 10% protocol treasury, 5% staking insurance fund.

### Staking Economics

- **Target staking ratio:** 50% of circulating supply.
- **APY range:** 3-12%, dynamically adjusted. Below 20% staked: 12% emergency incentive. At 50% target: 6%. Above 80%: 3% floor.
- **Minimum validator stake:** 50,000 tokens (governance-adjustable, range 10K-500K).
- **Delegation:** Supported, with delegated stake counting toward the 5% per-validator cap. Commission range: 0-20%.
- **Slashing:** Equivocation (5%), prolonged downtime (0.1%/epoch), PoUW failure (1% of PoUW stake), PoUW fraud (10%), censorship (2% escalating).

### Fee Model

**EIP-1559 dynamic base fee (burned) + priority tips (to validators).**

- Base fee adjusts every anchor commit (~1.6s) based on block utilization.
- Target: 50% block utilization at equilibrium.
- AI inference fees: 85% to compute provider, 5% burned, 5% to treasury, 5% to model creator.

### Burn Mechanics

Four burn mechanisms create deflationary pressure:

1. **Base fee burn:** All transaction base fees permanently burned. Scales with chain utilization.
2. **Inference fee burn:** 5% of all AI inference fees burned.
3. **State storage burn:** 50% of new storage slot surcharge burned permanently.
4. **Governance burn:** Treasury can burn tokens via supermajority vote.

### Path to Deflationary Equilibrium

| Scenario | When Burn Exceeds Emission |
|----------|--------------------------|
| Conservative (500 TPS avg) | Net inflationary through year 10 |
| Moderate (3,000 TPS, 100 inf/sec) | Year 6-7 |
| Aggressive (8,000 TPS, 500 inf/sec) | Year 5-6 |
| Mature (10,000+ TPS, 1,000+ inf/sec) | Year 4-5 (net deflationary: ~-45M tokens/year) |

### Market Scenarios

**Bull (high adoption):** Strong deflationary pressure. Risk of over-staking (mitigated by APY curve). Economically self-sustaining.

**Bear (low activity):** Emission rewards sustain validators. PoUW income decoupled from token speculation. APY curve increases rewards at low staking ratios. Survivable for 5+ years on emission alone.

**Stagnant (flat):** Moderate fee revenue. Ecosystem and AI funds provide sustained grant funding. PoUW income stream is unique competitive advantage.

---

## 9. AI Integration Design

### Three-Layer Architecture

Dendrite Network integrates AI at every protocol layer, but with a critical constraint: AI must never be required for basic chain operation. Every AI feature degrades gracefully.

#### Layer 1 -- Protocol-Level AI (runs on every full node)

Lightweight advisory models that monitor network health and flag anomalies. These models are small (total <5MB), fast (<0.5ms inference), and advisory-only -- they never reject transactions or halt consensus.

- **Transaction Anomaly Detection:** Ensemble of three models -- GBDT (structural), Isolation Forest (statistical), 1D-CNN (temporal). 22 features per transaction. Scores annotated in receipts, not used for censorship.
- **Block-Level Pattern Analysis:** GBDT analyzing 18 aggregate features per anchor commit. Detects MEV extraction, coordinated wash trading, validator collusion signals.
- **AI-Assisted Consensus Validation:** AI does NOT validate blocks. AI feeds into PoUW reputation (capped at 15% of anchor score) and provides anomaly scores as soft advisory signals.

#### Layer 2 -- Smart Contract AI (execution layer)

- **Automated Pre-Deployment Auditing:** Two-stage pipeline (deterministic static analysis + ML bytecode classifier). Checks reentrancy, overflow, access control, oracle manipulation, rugpull patterns, proxy risks. Audit reports attached to deployment receipts. Does NOT prevent deployment -- permissionless deployment preserved.
- **Runtime Exploit Detection:** Rule engine + lightweight GBDT. Monitors executing contracts for reentrancy, flash loan patterns, abnormal gas consumption, balance drains. Emits advisory alerts, does not halt transactions.
- **Gas/Fee Optimization:** Regression model predicting AI inference gas costs. Feeds into EIP-1559-style dynamic pricing for inference requests.

#### Layer 3 -- Network AI (P2P layer)

- **P2P Health Monitoring:** EWMA + threshold-based (deliberately no ML -- network monitoring must be maximally reliable). Monitors peer count, vertex propagation delay, gossip rates, NAT traversal success.
- **Peer Reputation Scoring:** Multi-factor composite score (gossip quality, latency consistency, data availability, protocol compliance). Informs mesh formation, eviction, rate limiting.
- **DDoS/Spam Detection:** Three-layer defense: rate limiting (no AI), statistical anomaly detection (lightweight), ML classifier (GBDT, 8 features, used only when statistical layer triggers).

### AI Compute Marketplace

The PoUW layer creates a decentralized AI compute market:

1. Users submit InferenceRequest transactions specifying model, input, verification method, and max fee.
2. PoUW scheduler assigns requests to eligible validators via deterministic VRF-based assignment.
3. Validators execute inference off-chain using tract (CPU), candle (consumer GPU), or ONNX Runtime (professional GPU).
4. Verified InferenceAttestations are included in DAG vertices and finalized on-chain.
5. Requesting contracts consume verified results via the `ai_inference()` precompile.

### The 5 AI Primitives

These are first-class protocol objects, not smart contract abstractions:

| Primitive | Layer | Description |
|-----------|-------|-------------|
| AIAgent | Application | On-chain identity for autonomous AI agents: owner, constraints, spending limits, KYA attestation |
| ModelRegistry | Execution | On-chain registry of AI model fingerprints. Models stored off-chain; only identity and verification metadata on-chain |
| InferenceRequest | Execution | Transaction type requesting AI inference: model ID, input hash, max fee, verification method |
| InferenceAttestation | Consensus | Verified inference result: result hash, proof, validator signatures |
| AIComputeCommitment | Consensus | Validator's staked commitment to perform AI computation for an epoch |

### Model Governance

- Models registered on-chain with content hash, owner, version, format, license terms, and supported verification methods.
- Model updates create new registry entries (immutable history).
- On-chain governance can flag or deprecate models (e.g., for safety concerns).
- Model creator royalties: 5% of inference fees for registered models.

### Graceful Degradation

If all AI features are disabled or no PoUW validators exist:
- SynBFT consensus continues at full speed (PoUW reputation drops to zero; 15% of anchor score becomes zero for all validators equally).
- Smart contract execution continues (AI Oracle returns "no PoUW providers available" error, which contracts handle via fallback logic).
- Transaction processing continues (anomaly scores default to 0.0; no flags).
- The chain is a fully functional general-purpose blockchain without AI.

---

## 10. Smart Contract Layer

### Dual VM Architecture

**Primary: WASM via wasmtime (Bytecode Alliance, Apache 2.0)**
- Cranelift compiler for deterministic, near-native execution (~1.2-1.5x native).
- Native fuel-based gas metering.
- Full access to AI Oracle precompile, object model, and all host functions.
- Contracts written in Rust (genesis-sdk-rs) or AssemblyScript (genesis-sdk-as).

**Secondary: EVM via revm (Paradigm, MIT/Apache 2.0)**
- Full Solidity/Vyper compatibility. OpenZeppelin contracts deploy unchanged.
- Hardhat, Foundry, and Remix supported via standard JSON-RPC.
- Access to bridge precompile for cross-VM calls to WASM contracts.
- Does NOT have direct access to AI Oracle or object model (use WASM bridge).

Both VMs share the same underlying state store, enabling composability: a WASM contract can hold ERC-20 tokens deployed on the EVM side.

### Contract Standards

| Standard | Type | Description |
|----------|------|-------------|
| GEN-20 | Fungible Token | Equivalent to ERC-20 with native WASM implementation + cross-VM bridge |
| GEN-721 | Non-Fungible Token | Equivalent to ERC-721, leveraging the object model for true ownership |
| GEN-AGENT | AI Agent Contract | Standard interface for AI agent management: registration, KYA, constraints, spending |
| GEN-GOV | Governance | On-chain governance proposals, voting, timelock execution |
| GEN-MULTI | Multi-Signature | M-of-N multi-signature accounts with configurable policies |

### AI Auditing

Every contract deployment triggers a two-stage audit pipeline:
1. **Deterministic static analysis:** Reentrancy, overflow, unprotected selfdestruct, access control, unbounded loops, storage collisions.
2. **ML classification:** 1D-CNN over bytecode producing 8 vulnerability probability scores (reentrancy, overflow, access control, oracle manipulation, flash loan risk, rugpull, proxy risk, unusual patterns).

Audit reports are attached to deployment receipts and queryable on-chain. Deployment is never blocked -- censorship resistance is inviolable.

### Gas Model

Fuel-based metering via wasmtime's native API. Key costs:

| Operation | Fuel Cost |
|-----------|-----------|
| Basic arithmetic | 1 |
| Memory load/store | 2 |
| State read (32 bytes) | 500 |
| State write (32 bytes) | 2,500 |
| Ed25519 verify | 3,000 |
| BLS verify | 15,000 |
| Cross-contract call | 10,000 |
| Cross-VM call | 25,000 |
| AI inference request | 50,000 (base) + model-specific |

Dynamic pricing via EIP-1559: base fee adjusts every anchor commit. Congestion pricing active from genesis (avoiding Solana's congestion lesson).

---

## 11. P2P Network Design

### Protocol Stack

| Layer | Choice | Justification |
|-------|--------|---------------|
| Framework | rust-libp2p + NetworkTransport abstraction trait | Swappable backend if libp2p degrades |
| Transport (Full/Validator) | QUIC primary, TCP+Noise fallback | 0-RTT resumption, built-in encryption, UDP NAT friendliness |
| Transport (Browser) | WebRTC DataChannels via libp2p-webrtc | Only browser P2P transport without server dependency |
| Transport (Mobile) | QUIC primary | Critical for intermittent connectivity |
| Discovery | Kademlia DHT + DNS seeds + mDNS + peer caching | Layered, no single point of failure |
| Gossip | Gossipsub v1.1 with priority batching | Production-proven, peer scoring, flow control |
| Encryption | Noise XX (TCP), TLS 1.3 (QUIC), DTLS (WebRTC) | All connections encrypted by default |

### 6-Layer Bootstrap Strategy

1. **Hardcoded genesis peers:** 50+ geographically distributed peers compiled into binary.
2. **DNS seed records:** 5+ independent DNS seeds returning randomized active peer subsets.
3. **Kademlia DHT:** Once connected to ANY peer, DHT discovery finds the rest.
4. **mDNS local discovery:** Find peers on same local network (censorship-resistant).
5. **QR code / manual peer exchange:** Offline peer sharing for extreme censorship scenarios.
6. **DHT peer caching:** Nodes persist known peers to disk; on restart, try cached peers first.

**After initial bootstrap, no ongoing dependency on bootstrap infrastructure.**

### NAT Traversal

- ICE/STUN handles ~85% of consumer NAT configurations.
- Incentivized TURN relays (full nodes earning transaction fee share) handle the remaining ~15%.
- Protocol tracks relay quality (uptime, bandwidth, latency) and rewards accordingly.
- UPnP/NAT-PMP attempted automatically for direct reachability.
- Validators behind relay add ~20-50ms latency per message -- within the 400ms round budget.

### Bandwidth Optimization

- **Compact block relay:** Transmit transaction hashes instead of full transactions (peers likely already have them). ~95% bandwidth reduction for block propagation.
- **Gossipsub priority batching:** Consensus-critical messages (vertices) prioritized over non-critical (transaction gossip) during high load.
- **Connection limits per node type:** Full/Validator: 40-60 peers. Light: 12-20. Browser: 4-12. Mobile: 4-8.

### Server-Independence Verification

The architecture passes the test: "Does this work if every centralized server in the world goes offline?"
- Peer discovery: DHT + mDNS + cached peers = no server needed.
- RPC: every node is its own endpoint.
- Block propagation: Gossipsub = no message broker.
- Browser participation: WebRTC = no relay server for data (only for initial signaling, which uses libp2p Circuit Relay v2 via other peers).
- NAT traversal: incentivized decentralized relays = no centralized TURN server.

---

## 12. Security Model

### Threat Matrix Summary

The security engineer identified 44 attack vectors across 7 categories:

| Category | Vectors Assessed | Critical/High | Key Threats |
|----------|-----------------|---------------|-------------|
| Consensus Attacks | 8 | 3 | Byzantine >1/3, equivocation, long-range |
| Economic Attacks | 6 | 3 | Flash loan governance, MEV, death spiral |
| Smart Contract | 7 | 2 | Reentrancy, oracle manipulation |
| Network Attacks | 8 | 3 | Eclipse, DDoS, DHT poisoning |
| AI-Specific | 5 | 2 | PoUW gaming, adversarial model manipulation |
| Privacy | 5 | 2 | ZK soundness, metadata leakage |
| Operational | 5 | 1 | Key compromise, supply chain |

### Cryptographic Standards

| Component | Algorithm | Purpose | Quantum Status |
|-----------|-----------|---------|---------------|
| Hashing | BLAKE3 | All hashing (block, tx, VRF seed) | Safe (128-bit effective security with Grover) |
| Signatures | Ed25519 (via ed25519-dalek, strict mode) | Transaction signing, vertex signing, VRF | Vulnerable (10-15 year horizon) |
| Aggregate Signatures | BLS12-381 (via blst) | Finality certificates for light clients | Vulnerable (10-15 year horizon) |
| State Commitment | Verkle trees (IPA/KZG) | State proofs for light clients | Vulnerable (designed-in escape hatch) |
| Serialization | Protobuf (wire), Bincode (internal) | Data encoding | N/A |
| Key Derivation | Argon2id (256MB+ memory) | Passphrase-based key encryption | Safe |

### AI Security Monitoring

Four monitoring subsystems run on every full node:

1. **Transaction Anomaly Monitor:** Ensemble model scoring each transaction 0.0-1.0. Advisory only.
2. **Consensus Behavior Monitor:** Detects validator coordination, selective referencing, VRF clustering, stake concentration.
3. **Contract Exploit Monitor:** Real-time detection of reentrancy, flash loan patterns, balance drains.
4. **Network Health Monitor:** Peer diversity, propagation latency, DHT health, DDoS detection.

**Critical principle:** AI NEVER autonomously slashes, freezes funds, or reverts transactions. AI is advisory and detection only. Slashing is triggered only by cryptographic proof of misbehavior.

### Quantum Readiness

**4-phase migration plan:**

1. **Phase 1 (Launch-Year 2):** All crypto uses trait abstractions (SignatureScheme, StateCommitment). Monitor NIST PQC standardization. Implement PQC verification in test branch.
2. **Phase 2 (Year 2-3):** Introduce PQC signature support alongside classical. Accounts optionally register PQC keys. Dual-signing supported. Verkle-to-Merkle migration on testnet.
3. **Phase 3 (Year 3-5):** Governance deadline for PQC migration. New accounts require PQC keys. State commitment switched to quantum-safe scheme.
4. **Phase 4 (Year 5+):** Classical signatures deprecated. Full quantum resistance.

**PQC candidates:** FN-DSA (Falcon-512) for transaction signatures (smallest at ~666 bytes). ML-DSA (Dilithium-2) for validator signatures.

### Key Management

**Key hierarchy from BIP-39 master seed:**
- Account Key (Ed25519): transaction signing, identity
- Validator Consensus Key (Ed25519): vertex signing, VRF
- Validator BLS Key (BLS12-381): aggregate finality certificates
- Encryption Key (X25519): Noise protocol, P2P identity

**Wallet security levels:** Basic (encrypted keyfile + Argon2id), Standard (OS keychain + optional 2FA), High-security (hardware wallet via WebUSB), Maximum (multi-sig M-of-N).

**Validator key security:** HSM support via PKCS#11 (YubiHSM 2, Thales Luna, cloud HSMs). Key rotation required every 6-12 months.

---

## 13. Technology Stack Summary

| Layer | Technology | License | Justification |
|-------|-----------|---------|---------------|
| **Language** | Rust | MIT/Apache 2.0 | Memory safety without GC, deterministic performance, first-class async (tokio), WASM compilation target. Industry standard for blockchain. |
| **Async Runtime** | tokio | MIT | Production-proven, multi-threaded, battle-tested. |
| **P2P Framework** | rust-libp2p | MIT/Apache 2.0 | DHT, Gossipsub, WebRTC, Noise. Independent maintainers from go/js implementations. NetworkTransport abstraction for swappability. |
| **Consensus Transport** | QUIC (primary) | N/A (protocol) | 0-RTT, built-in encryption, UDP NAT friendliness, multiplexed streams. |
| **State Storage (Full)** | RocksDB (Apache 2.0 option) | Dual GPL2/Apache 2.0 | Dual storage architecture, column families, compression, production-proven at scale. |
| **State Storage (Light/Mobile)** | redb | MIT/Apache 2.0 | Pure Rust, ACID, lightweight, no C++ dependency. Used in Bitcoin ordinals tooling. |
| **State Commitment** | Verkle Trees | Research | 100x smaller proofs than Merkle. Critical for light/browser/mobile nodes. Post-quantum escape hatch designed in. |
| **Hashing** | BLAKE3 | CC0/Apache 2.0 | Faster than SHA-256, cryptographically secure, tree hashing mode. |
| **Signatures** | Ed25519 (ed25519-dalek) | BSD-3-Clause | Fast, well-audited. Strict verification mode mandatory. |
| **Aggregate Signatures** | BLS12-381 (blst) | Apache 2.0 | 134 signatures aggregate to 96 bytes. Essential for light client finality verification. Audited by NCC Group. |
| **WASM Runtime** | wasmtime | Apache 2.0 + LLVM exception | Bytecode Alliance. Fuel metering, Cranelift determinism, extensive fuzzing, WASM compilation to browser. |
| **EVM Runtime** | revm | MIT/Apache 2.0 | Paradigm's pure-Rust EVM. Used by Reth. Full Solidity compatibility. |
| **AI Runtime (Primary)** | tract (Sonos) | MIT/Apache 2.0 | Rust-native, deterministic ONNX inference, <0.1ms for small models. |
| **AI Runtime (Secondary)** | candle (HuggingFace) | MIT/Apache 2.0 | GPU inference via CUDA/Metal for medium models. |
| **AI Runtime (Optional)** | ONNX Runtime (Microsoft) | MIT | Maximum compatibility for large models. Validator-only. |
| **Serialization** | Protobuf (prost) + Bincode + serde | MIT/Apache 2.0 | Protobuf for wire format, Bincode for internal. |
| **Numeric Precision** | u128 native + uint crate (u256) | MIT/Apache 2.0 | All token arithmetic in u128 base units. u256 for intermediate calculations. |

### Stack Rulings

| Decision | Ruling | Rationale |
|----------|--------|-----------|
| P2P: libp2p vs. alternative | COMPROMISE | Use rust-libp2p with NetworkTransport abstraction. Contribute upstream. |
| State: Merkle vs. Verkle | ACCEPT CHALLENGE | Verkle trees adopted with post-quantum escape hatch. |
| Consensus: BFT vs. DAG+PoUW | ACCEPT CHALLENGE | DAG-BFT (SynBFT) + PoUW overlay adopted. |
| AI Runtime: ONNX vs. alternatives | REFINE | tract primary, candle secondary, ONNX optional. |
| License: MIT vs. Apache vs. dual | ACCEPT RECOMMENDATION | Dual MIT/Apache-2.0. Join COPA. |
| Signatures: Ed25519 vs. Ed25519+BLS | PENDING (stack challenge raised) | BLS12-381 required alongside Ed25519 for aggregate finality proofs. Endorsed by node-engineer and security-engineer. Architect ruling pending but adoption is assumed in all downstream designs. |

---

## 14. Open-Source Licensing Strategy

### License: Dual MIT OR Apache-2.0

This follows the Rust ecosystem convention and provides:
- **MIT:** Maximum adoption and compatibility.
- **Apache 2.0:** Explicit patent grant from all contributors (critical given 58% of blockchain litigation comes from NPEs). Patent retaliation clause deters attacks.
- **Dual licensing** gives downstream users flexibility.

### Dependency Compatibility

All proposed dependencies are GREEN (compatible):
- libp2p, tokio, serde, blake3, ed25519-dalek: MIT/Apache 2.0
- RocksDB: Select Apache 2.0 option (not GPLv2)
- wasmtime: Apache 2.0 + LLVM exception
- tract, candle: MIT/Apache 2.0
- blst: Apache 2.0
- revm: MIT/Apache 2.0

**No GPL, AGPL, or SSPL dependencies.** Any such dependency would be a blocking issue.

### COPA Membership

Join the Crypto Open Patent Alliance before any public disclosure of the consensus design. COPA provides:
- Defensive patent pool (300+ companies including Square, Coinbase, Kraken).
- Commitment to never assert crypto patents offensively.
- The COPA + Unified Patents "Blockchain Zone" actively challenges NPE patents.
- Free to join.

### Contributor Agreement

- **Phase 1:** DCO (Developer Certificate of Origin) with git sign-off. Minimal friction.
- **Phase 2:** Lightweight CLA (based on Apache ICLA) when corporate contributors join.
- **Automated:** CLA bot on GitHub for sign-off tracking.

---

## 15. 4-Week Build Roadmap

This roadmap covers the first 4 weeks of implementation. The goal is a minimal testnet bootstrap by week 4. The team assumption is 3-5 core Rust engineers with blockchain experience.

### Week 1: Core Primitives

**Goal:** Foundational data structures, cryptography, and a minimal standalone node binary.

| Day | Deliverable | Details |
|-----|------------|---------|
| 1-2 | Project scaffolding | Cargo workspace with crates: `dendrite-types`, `dendrite-crypto`, `dendrite-storage`, `dendrite-consensus`, `dendrite-network`, `dendrite-vm`, `dendrite-node`. CI/CD pipeline with cargo-audit and cargo-clippy. Dual MIT/Apache-2.0 license headers. |
| 2-3 | Core types | `GenesisBlockHeader`, `GenesisBlockBody`, `Transaction`, `Account`, `ObjectState`, `AIAgentState`, `InferenceRequest`, `InferenceAttestation`, `AIComputeCommitment`. Serde serialization (Bincode internal, Protobuf wire). |
| 3-4 | Cryptography | BLAKE3 hashing wrappers. Ed25519 keypair generation, signing, verification (strict mode). BLS12-381 keypair, signing, aggregation, verification (with proof-of-possession). VRF implementation (ECVRF-EDWARDS25519-SHA512-TAI per RFC 9381). |
| 4-5 | Storage engine | RocksDB integration with 6 column families. Dual storage architecture (state store + state commitment). Basic key-value state read/write/delete. Genesis state initialization. |
| 5 | Verkle tree stub | `StateCommitment` trait definition. Placeholder implementation using binary Merkle tree (production Verkle tree implementation is a multi-week effort; Merkle placeholder enables all other work to proceed). |

**Week 1 milestone:** A node binary that can initialize genesis state, create and sign transactions, hash blocks, and persist state to RocksDB. No networking, no consensus.

### Week 2: P2P Networking + Basic Consensus

**Goal:** Nodes discover each other, gossip messages, and run a simplified consensus round.

| Day | Deliverable | Details |
|-----|------------|---------|
| 1-2 | NetworkTransport trait + libp2p implementation | `NetworkTransport` trait definition. `Libp2pTransport` implementation wrapping rust-libp2p Swarm. QUIC transport. Noise encryption for TCP fallback. |
| 2-3 | Peer discovery | Kademlia DHT integration. Hardcoded bootstrap peers (for testnet). mDNS for local testing. Peer caching to redb. |
| 3-4 | Gossipsub | Topic configuration: `dendrite/dag/vertices`, `dendrite/txs`, `dendrite/security/alerts`. Message serialization. Peer scoring (basic). Priority batching stub. |
| 4-5 | Basic SynBFT | Simplified single-round consensus: validators propose vertices, reference parents, basic DAG structure. No anchor commits yet. No VRF leader election. Round timer (400ms). Vertex broadcast via Gossipsub. |
| 5 | Mempool | Transaction mempool with priority sorting. Basic deduplication. Accept transactions via local RPC. |

**Week 2 milestone:** Multiple nodes discover each other via DHT/mDNS, gossip transactions, propose DAG vertices with parent references, and maintain a growing DAG structure. No finality yet.

### Week 3: Execution Layer

**Goal:** WASM contract execution, state transitions, and anchor-based finality.

| Day | Deliverable | Details |
|-----|------------|---------|
| 1-2 | wasmtime integration | Fuel-based metering. Host function interface (state_read, state_write, caller, balance_of, transfer, blake3, ed25519_verify, emit_event). Resource limits (16MB memory, 1024 stack, 100M max fuel). |
| 2-3 | Transaction execution | Transaction router. Simple transfer execution. Contract deployment (WASM binary stored in state). Contract call dispatch. Receipt generation. |
| 3-4 | Anchor commit | VRF-based anchor proposer election. Anchor commit rule (2f+1 support check). Deterministic topological sort for total ordering. State root computation after ordered execution. |
| 4-5 | BLS finality certificates | BLS signing of anchor commits. Aggregate signature construction. Finality certificate structure. Light client verification function. |
| 5 | State sync (basic) | Snapshot export/import. New node can download state snapshot from a peer and begin following the DAG. |

**Week 3 milestone:** Nodes achieve deterministic finality via anchor commits. WASM contracts deploy and execute. State transitions produce verifiable state roots. A new node can sync state from peers.

### Week 4: Integration, AI, Testnet

**Goal:** End-to-end integration testing, basic AI inference, and testnet bootstrap.

| Day | Deliverable | Details |
|-----|------------|---------|
| 1-2 | Integration testing | Multi-node testnet (4-7 local validators). Transaction throughput benchmarking. Finality timing verification. Basic fault injection (kill validators, verify chain continues with >2/3). |
| 2-3 | Basic AI inference | tract integration. Single model: GBDT transaction anomaly detector (~50K params). Load model at node startup. Score transactions. Annotate receipts with anomaly scores. |
| 3 | EVM integration (basic) | revm wired to shared state store. Deploy a simple ERC-20 Solidity contract. Verify it executes correctly. Cross-VM bridge is a later milestone. |
| 4 | Testnet bootstrap | Genesis config with 5-10 pre-funded validator accounts. Public testnet documentation. Node operator guide. Basic CLI wallet. Docker image for easy node deployment. |
| 5 | Testnet launch | Boot the testnet. Submit transactions. Deploy test contracts. Verify AI anomaly scoring. Document results and known issues. |

**Week 4 milestone:** A live testnet with 5-10 validators achieving sub-second finality, executing WASM and basic EVM contracts, with AI transaction anomaly scoring operational. This is the proof-of-concept that validates the architecture.

### Post-Week-4 Priority Queue

1. Full Verkle tree implementation (replacing Merkle placeholder).
2. PoUW subsystem (inference request routing, attestation pipeline, multi-method verification).
3. Browser node (WASM compilation, WebRTC transport, IndexedDB storage).
4. Full EVM compatibility (all precompiles, cross-VM bridge).
5. Privacy layer (shielded transactions, ZK proof verification).
6. Mobile node SDK.
7. Governance module.
8. Contract audit AI pipeline (static analysis + ML classifier).
9. Mainnet preparation (security audit, formal verification, load testing).

---

## 16. What Makes Dendrite Network Unique

The blockchain space in March 2026 is saturated with Layer-1 chains competing on the same axes: TPS benchmarks, EVM compatibility, and speculative token mechanics. Dendrite Network does not compete on these axes. It occupies a position that no existing chain has claimed, because that position requires making three architectural commitments simultaneously -- commitments that are individually challenging and collectively unprecedented.

### Commitment 1: AI as a Protocol Primitive, Not an Afterthought

Every "AI blockchain" announced in 2024-2026 falls into one of three categories:

- **AI-only networks (Bittensor):** A compute marketplace with no general smart contracts. You cannot build DeFi, NFTs, or arbitrary applications. Bittensor's speed-only evaluation metric led to garbage outputs. Its architecture is architecturally incapable of supporting the composable AI+DeFi applications that will define the next wave.

- **AI bridge layers (Ritual):** A middleware service that connects AI inference to existing chains. Adds latency, complexity, and trust assumptions. The AI compute happens elsewhere; the blockchain only records results. This is useful but not transformative -- it does not make the chain itself intelligent.

- **AI mergers (ASI Alliance / Fetch.ai + SingularityNET + Ocean):** Three separate architectures bolted together under a shared token. The technical integration is cosmetic. Each component retains its own consensus, state model, and limitations.

Dendrite Network is none of these. AI lives inside the consensus mechanism (PoUW validators earn rewards for verified inference), the state model (AIAgent, ModelRegistry, InferenceRequest, InferenceAttestation are protocol-level objects, not smart contract abstractions), the execution layer (contracts call `ai_inference()` as a built-in operation), and the economic design (AI inference fees create deflationary pressure through the 5% burn mechanism). Removing AI from Dendrite would require redesigning the consensus, the state model, the execution layer, and the tokenomics. That is what "first-class citizen" means.

### Commitment 2: True Server-Independence

Most blockchains claim decentralization but depend on centralized infrastructure for practical use. Over 70% of Ethereum transactions route through three RPC providers (Infura, Alchemy, QuickNode). Solana validators require datacenter hardware. Browser users interact with blockchains through centralized APIs, never touching the P2P network directly.

Dendrite Network enforces server-independence as a hard architectural constraint. Every full node is its own RPC endpoint. Browser users run WASM-compiled light clients connecting via WebRTC to the P2P network -- no centralized intermediary. NAT traversal uses incentivized decentralized relays, not corporate TURN servers. A new node can join the network if it can reach even one existing peer out of 50+ hardcoded bootstrap nodes, DNS seeds, mDNS local discovery, or cached peers.

This is not a feature that can be added later. It is a constraint that shapes every other decision -- from the Verkle tree state proofs (constant-size, enabling mobile verification) to the BLS aggregate signatures (96 bytes for a finality certificate instead of 4.3KB of individual Ed25519 signatures).

### Commitment 3: Privacy as a Protocol-Level Capability

Following a16z's 2026 thesis that "privacy creates chain lock-in through a privacy network effect," Dendrite implements privacy as selective disclosure at the protocol level. Bridging tokens between chains is trivial; bridging secrets is impossible. Once users commit private state to Dendrite, they cannot take that privacy guarantee with them to another chain. This creates organic lock-in that is architecturally, not just economically, defensible.

Privacy is not full anonymity. It is user-controlled selective disclosure with regulatory compliance hooks (prove you are KYC-verified without revealing your identity). This positions Dendrite for enterprise and institutional adoption while preserving individual privacy.

### The Intersection Creates the Moat

Each commitment alone is achievable. Bittensor does AI. Bitcoin does server-independence. Zcash does privacy. No chain does all three simultaneously, because the architectural requirements conflict:

- AI-native consensus (PoUW) requires compute-heavy validation, which pushes toward datacenter hardware -- conflicting with server-independence.
- Privacy requires ZK proofs, which are computationally expensive -- conflicting with consumer hardware validation.
- Server-independence requires every node to verify everything independently -- conflicting with the trust assumptions that simplify AI and privacy implementations.

Dendrite resolves these conflicts through careful architectural choices: PoUW is optional (the chain runs without it), Verkle proofs enable lightweight verification (browser nodes verify state without full computation), and privacy is selective (public by default, private when chosen). The resolution is not a compromise -- it is a design that makes each feature reinforce the others.

### Three Killer Applications

The architecture is optimized for three use cases that sit at the intersection of all three commitments:

1. **Private AI Compute Markets:** Users submit encrypted inference requests. PoUW validators process them in TEEs or via ZK-verified computation. Neither model weights nor user data are exposed. No existing chain enables this natively.

2. **Privacy-Preserving Identity:** Self-sovereign identity with selective disclosure. Prove attributes without revealing data. The AIAgent primitive extends this to AI entities with on-chain identity and spending constraints.

3. **Agent-to-Agent Autonomous Payments:** AI agents with on-chain identity transact using protocol-native payment channels. Microtransactions for API calls, GPU time, data access. Human principals set constraints; agents operate within them.

These are not features to add later. Every layer decision -- the hybrid state model, the PoUW consensus layer, the privacy primitives, the AI Oracle, the agent accounts -- exists to make these three use cases native and efficient.

---

## 17. Unresolved Items and Risks

This section provides an honest accounting of open issues. A credible plan acknowledges what remains unknown.

### Security Flags (13 Open, 9 SECURITY-ELEVATED)

| Flag | Target | Description | Priority |
|------|--------|-------------|----------|
| S0-1 | blockchain-architect | Ed25519 strict verification mode must be mandatory in all signature paths | SECURITY-ELEVATED |
| S1-1 | blockchain-architect | MEV mitigation: encrypted mempool should be on the roadmap | SECURITY-ELEVATED |
| S1-2 | blockchain-architect | Quantum migration drill: annual testnet exercise of Verkle-to-Merkle migration | SECURITY-ELEVATED |
| S1-3 | blockchain-architect | Agent spending limits must be enforced at consensus layer (pre-execution), not just VM | SECURITY-ELEVATED |
| S2-1 | consensus-engineer | Uncertified DAG equivocation window: formal analysis required to bound exposure time | SECURITY-ELEVATED |
| S2-2 | consensus-engineer | VRF last-revealer bias: anchor proposer can suppress publication to influence next seed. Recommend commit-reveal VRF or RANDAO | SECURITY-ELEVATED |
| S2-3 | consensus-engineer | MPRE quantization scheme: formal specification of 16-bit fixed-point comparison | STANDARD |
| S3-1 | tokenomics-engineer | Governance must be flash-loan resistant (addressed via epoch-lock voting but needs formal verification) | SECURITY-ELEVATED |
| S4-1 | node-engineer | Browser node key management: enforce spending limits, hardware wallet support, warnings | SECURITY-ELEVATED |
| S4-2 | node-engineer | RocksDB access control and encryption at rest | STANDARD |
| S4-3 | node-engineer | Weak subjectivity checkpoint distribution mechanism must be specified | SECURITY-ELEVATED |
| S8-1 | p2p-network-engineer | DHT records must be signed by validator quorum with freshness validation | SECURITY-ELEVATED |
| S8-2 | p2p-network-engineer | Relay trust: distribute relay connections across 3+ operators, not just 3 connections | STANDARD |

**All 9 SECURITY-ELEVATED flags must be addressed before mainnet. The 4 STANDARD flags must be addressed before mainnet but are not blocking for testnet.**

### Stack Challenge Pending Architect Ruling

**BLS12-381 alongside Ed25519:** The consensus-engineer raised a stack challenge requesting BLS12-381 as a required secondary signature scheme for aggregate finality proofs. The node-engineer and security-engineer both endorsed this challenge. Without BLS aggregation, light/browser/mobile nodes cannot efficiently verify finality (134 individual Ed25519 signatures vs. 1 aggregate BLS signature). The architect has not yet issued a formal ruling, but all downstream designs assume BLS adoption. The `blst` crate is Apache 2.0 licensed, compiles to WASM, and is production-proven in Ethereum's beacon chain. **Recommendation: Accept the challenge.**

### Patent Risks Requiring FTO Analysis

| Element | Risk | Action Required |
|---------|------|----------------|
| DAG-based BFT ordering | MEDIUM -- nChain holds U.S. Patent 12,032,677 (under reexamination) | Formal FTO analysis against nChain portfolio (1,308 patents). MANDATORY before finalizing consensus design. |
| PoUW with AI inference | MEDIUM-HIGH -- emerging patent area, nChain and IBM filing | File provisional patent on Synaptic Consensus PoUW design. Defensive, not offensive (pledge to COPA). |
| AI + consensus combinations | MEDIUM -- rapidly increasing filings | Publish defensive disclosures for equivocation detection, censorship scoring, quality metrics. |
| Aggregate BLS for finality | LOW -- Ethereum uses this (Apache 2.0/GPLv3 open implementations) | No action needed beyond standard monitoring. |
| Stake-weighted selection with caps | LOW-MEDIUM -- individual elements well-known | Document specific combination in defensive disclosure. |

**Overall patent risk is manageable but requires proactive FTO analysis and COPA membership before any public disclosure.**

### Dendrite Systems Inc. Coexistence Agreement

Dendrite Systems Inc. holds 4 trademarks in Class 9 (computer software) for "Dendrite" related to AI web agents and browser extensions. The company appears unfunded with an archived product. A trademark coexistence agreement is the recommended path: proactively contact them, negotiate terms ($5,000-$15,000 estimated), and secure written agreement that both parties can use "Dendrite" in their respective fields without opposition.

**Risk if coexistence fails:** The Class 9 trademark filing would face opposition. Classes 36 and 42 would still proceed. The project could operate as "Dendrite Network" with trademarks in Classes 36/42 while seeking alternative resolution for Class 9.

**Risk if not addressed:** Dendrite Systems Inc. could oppose the USPTO Class 9 filing, delaying registration by 12-18 months and costing $20,000-$50,000 in legal fees.

### Technical Unknowns

1. **Verkle tree implementation maturity:** Ethereum's Verkle tree implementation (Kaustinen testnet) is not yet production-deployed on mainnet. Our implementation will be among the first production deployments. Risk: undiscovered edge cases in proof generation or verification. Mitigation: binary Merkle tree fallback is maintained and tested.

2. **zkML verification overhead:** ZK proofs of inference for large models remain impractical (>10,000x overhead). For small models (our initial target), ZK is feasible. Risk: if the market demands ZK verification for large models before overhead improves, MPRE and TEE must carry the full load. Mitigation: three verification methods provide redundancy.

3. **Browser node WebRTC reliability:** libp2p-webrtc has limited production deployment history compared to libp2p-tcp or libp2p-quic. Risk: connection stability, signaling edge cases, browser compatibility issues. Mitigation: WebSocket fallback to full nodes when WebRTC fails.

4. **Cross-architecture floating-point determinism for MPRE:** The 16-bit fixed-point quantization scheme for MPRE agreement checking needs formal specification and cross-platform testing (x86 vs. ARM, different GPU architectures). Risk: edge cases where honest validators disagree and are incorrectly flagged. Mitigation: conservative quantization threshold and dispute resolution period before slashing.

5. **Post-quantum signature sizes and light client bandwidth:** PQC signatures (666 bytes for FN-DSA vs. 64 bytes for Ed25519) will significantly increase finality certificate sizes when migration occurs. Light client bandwidth budgets must be recalculated. This is a year 2-3 concern, not launch-blocking.

### Regulatory Unknowns

1. **CLARITY Act status:** If passed, may provide a safe harbor for utility tokens. If not passed, the current regulatory ambiguity persists. Monitor closely.
2. **FSCA stablecoin regulation:** Expected clarity in 2026. If Dendrite Network supports stablecoin functionality, additional FSCA requirements may apply.
3. **MiCA enforcement strictness:** First penalties are being assessed. Actual enforcement patterns will clarify compliance requirements.
4. **Privacy feature regulatory treatment:** The SEC's evolving framework may classify privacy features differently. Design choice (user-controlled, not protocol-enforced privacy) reduces but does not eliminate this risk.

---

## Appendix: Document Sources

This master plan synthesizes the following Phase 3-5 deliverables:

- **MASTER_DESIGN.md** -- Complete 9-section architecture document (blockchain-architect, consensus-engineer, tokenomics-engineer, node-engineer, security-engineer, ai-integration-engineer, smart-contract-engineer, p2p-network-engineer, naming-council)
- **RESEARCH_BRIEF.md** -- Comprehensive market and technology research (research-analyst)
- **LEGAL_LANDSCAPE.md** -- Trademark, patent, regulatory, and licensing analysis (legal-ip-counsel)
- **LEGAL_CLEARANCE_REPORT.md** -- 10 name candidates assessed, Dendrite upgraded to CONDITIONAL GREEN (legal-ip-counsel)
- **NAMING_REPORT.md** -- Name evaluation and final recommendation: Dendrite Network / DNDR (naming-council)
- **ORCHESTRATION.md** -- Skill fleet execution protocol and dependency graph

---

*This document is the definitive blueprint for building Dendrite Network. It is designed to stand alone as a complete reference for any engineer, investor, legal counsel, or community member who needs to understand what Dendrite Network is, why it exists, and how it will be built.*

*Assembled by: blockchain-architect*
*Date: March 5, 2026*
*Phase 6 -- COMPLETE*
