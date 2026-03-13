# AZTIBASE NETWORK - MASTER DESIGN DOCUMENT

## Status: M9 Mainnet Prep — Implementation Complete, Audit Fixes Applied

---

## SECTION 0: ARCHITECT'S STACK PROPOSAL (Baseline for Skill Review)

> This section is the initial stack recommendation from the lead architect.
> Each skill MUST review the decisions within their domain and either ACCEPT, CHALLENGE, or REFINE them.
> Challenges must be backed by research. The blockchain-architect holds final say on conflicts.

### Core Runtime: Rust

- No garbage collector - deterministic performance for consensus timing
- Memory safety without runtime overhead
- First-class async (tokio) for P2P networking
- WASM compilation target - enables browser/light nodes
- Industry standard: Solana, Polkadot/Substrate, NEAR, Aptos

### P2P Networking: libp2p (Rust implementation)

- DHT-based peer discovery (Kademlia) - no central servers after bootstrap
- NAT traversal (hole punching, relay protocols)
- Gossipsub for block/transaction propagation
- mDNS for local network discovery
- Noise protocol for encrypted connections
- WebRTC transport for browser nodes

### Consensus Layer: Custom (Rust)

- Pluggable module architecture
- BFT variant as baseline, evolving toward unique mechanism per research
- AI-validation hooks at consensus level
- Must support server-independent operation

### Smart Contract VM: WebAssembly (WASM)

- Contracts compile from Rust, AssemblyScript, or C
- Sandboxed execution
- Optional EVM compatibility via `revm`
- Deterministic execution guaranteed

### AI Integration: ONNX Runtime (Rust bindings) - OPEN FOR REVIEW

- Lightweight on-node inference
- No Python dependency
- Models: anomaly detection, fraud scoring, contract auditing
- Alternative candidates: `tract`, `candle`, custom inference
- ai-integration-engineer has authority to refine this choice

### Storage: redb + Verkle Trees (UPDATED per ADR-001)

- redb for local node state (pure Rust, ACID, no C deps — ADR-001)
- Dual storage architecture: state store (raw key-value) + state commitment (following Sei's proven pattern)
- Verkle trees for state verification (constant-size proofs, 100x smaller than Merkle proofs)
- ARCHITECT NOTE: Verkle trees are not post-quantum secure. Binary Merkle tree + SNARK fallback path must be designed in parallel. See Section 1, subsection 1.5 for full rationale.
- State pruning from genesis (EIP-4444-style historical pruning)
- Target: full node storage under 100GB for first year
- node-engineer has authority to refine storage choices within these constraints

### Cryptography

- Hashing: BLAKE3 (faster than SHA-256, cryptographically secure)
- Signatures: Ed25519 (via ed25519-dalek)
- Serialization: postcard (wire — length-prefixed) / postcard (internal — ADR-024)

### Node Types (Server-Independence Design)

| Node Type | Purpose | Resources |
|-----------|---------|-----------|
| Full Node | Complete state, validation | 8GB RAM, SSD |
| Light Node | Header-only, SPV proofs | 512MB RAM |
| Browser Node | WASM-compiled, WebRTC P2P | Modern browser |
| Mobile Node | Light client | Standard phone |

### Server Independence Architecture

1. No bootstrap server dependency - hardcoded initial peers + DNS seeds, then Kademlia DHT
2. No centralized API - every node is its own RPC endpoint
3. Browser-native nodes via WASM + WebRTC
4. IPFS for ancillary storage
5. Gossip-based propagation - no message broker
6. Embedded light client - mobile wallets connect directly to P2P network

### Developer SDKs (Separate from Core)

- TypeScript SDK for dApp developers
- Python SDK for AI/ML developers
- Rust SDK for core protocol contributors

### STACK RULINGS (blockchain-architect, 2026-03-05)

#### STACK RULING: P2P Networking (libp2p)
- **Challenged by:** research-analyst (RESEARCH_BRIEF.md, Section 7.2)
- **Original proposal:** libp2p (Rust implementation) as sole P2P stack
- **Challenge:** libp2p maintenance crisis -- Shipyard ceased support for go-libp2p and js-libp2p as of Sept 2025. Gossipsub "slows down under stress." Connection overhead makes home validation unreliable.
- **Ruling:** COMPROMISE
- **Justification:** rust-libp2p has separate, more active maintainers than go/js implementations. The maintenance crisis is real but confined to Go/JS. However, the research findings demand a mitigation strategy. **Decision:**
  1. USE rust-libp2p as the primary P2P backend (its maintainers are independent from Shipyard).
  2. BUILD a `NetworkTransport` trait abstraction layer over libp2p so the P2P backend can be swapped if rust-libp2p also degrades.
  3. CONTRIBUTE upstream to rust-libp2p maintenance (budgeted developer time).
  4. DESIGN the gossip layer to tolerate Gossipsub's stress limitations by implementing message batching and priority-based propagation.
  5. For browser nodes, use WebRTC transport directly (libp2p-webrtc) with decentralized TURN relays.

#### STACK RULING: State Management (Merkle Patricia Trie -> Verkle Trees)
- **Challenged by:** research-analyst (RESEARCH_BRIEF.md, Section 8.2)
- **Original proposal:** Merkle Patricia Trie for state verification
- **Challenge:** Verkle trees provide 100x smaller proofs with constant-size commitments. Ethereum is migrating to Verkle trees. Sei's dual storage architecture is production-proven.
- **Ruling:** ACCEPT CHALLENGE
- **Justification:** Verkle trees are strictly superior for our use case (light clients, browser nodes, mobile nodes all benefit from smaller proofs). The post-quantum concern is real but manageable -- we design a migration path to binary Merkle trees + SNARKs from day one. Dual storage (state store + state commitment) is adopted from Sei's proven architecture. See Section 0 Storage update above.

#### STACK RULING: Consensus Direction (BFT -> DAG + PoUW Hybrid)
- **Challenged by:** research-analyst (RESEARCH_BRIEF.md, Section 2.6)
- **Original proposal:** Pluggable BFT variant as baseline
- **Challenge:** DAG-based BFT + PoUW with ML verification identified as most promising unexplored territory. Sui's Mysticeti achieves 39ms finality at 100k TPS.
- **Ruling:** ACCEPT CHALLENGE
- **Justification:** The research is compelling. DAG consensus is production-proven (Sui, Aptos). PoUW aligns with our AI-native mission. **Decision:**
  1. Primary consensus: DAG-based BFT (Mysticeti-inspired, not copied -- conduct FTO analysis per LEGAL_LANDSCAPE.md Section 3.2 re: nChain patents).
  2. Secondary layer: PoUW for AI compute verification, operating as an optional validator commitment.
  3. The consensus-engineer skill will design the specific DAG protocol. This ruling sets the direction, not the implementation.
  4. Multi-metric evaluation for PoUW (not speed-only) to avoid Bittensor's garbage-output problem.
  **Patent flag:** nChain holds 1,308 patents on consensus-related technologies. FTO analysis is MANDATORY before finalizing any consensus design.

#### STACK RULING: AI Runtime
- **Challenged by:** None (open for review per Section 0)
- **Original proposal:** ONNX Runtime (Rust bindings)
- **Ruling:** REFINE
- **Justification:** Research confirms three viable options: ONNX Runtime (MIT), tract (MIT/Apache 2.0), candle (MIT/Apache 2.0). All are license-compatible per LEGAL_LANDSCAPE.md Section 7.3. **Decision:**
  1. PRIMARY: `tract` (Sonos) -- Rust-native, MIT/Apache 2.0 dual-licensed, purpose-built for embedded inference. Best fit for on-node lightweight inference.
  2. SECONDARY: `candle` (HuggingFace) -- for more complex model support when tract is insufficient.
  3. ONNX Runtime remains as optional backend for nodes with sufficient resources.
  4. ai-integration-engineer retains authority to refine within these constraints.
  5. All AI inference is OFF-CHAIN with ON-CHAIN verification (following Ritual's architecture pattern per RESEARCH_BRIEF.md Section 4.3).

#### STACK RULING: Open-Source License
- **Aligned with:** legal-ip-counsel (LEGAL_LANDSCAPE.md Section 8)
- **Ruling:** ACCEPT RECOMMENDATION
- **Decision:** Dual MIT OR Apache-2.0 license. Apache 2.0 patent grant is critical given 58% of blockchain litigation comes from NPEs. Join COPA. Use DCO for contributor sign-off initially.

---

## SECTION 1: BLOCKCHAIN ARCHITECTURE

---
**Contributed by:** blockchain-architect
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete), LEGAL_LANDSCAPE.md (complete), Section 0 stack proposal (reviewed and updated)

---

### 1.1 Core Design Philosophy

Aztibase Network is built on three pillars that, in combination, create a genuinely unique position in the blockchain landscape as of March 2026. No existing chain combines all three. This is our moat.

**Pillar 1: AI as a First-Class Protocol Citizen**

AI is not an afterthought, a sidechain, or an oracle service. It is woven into the protocol at every layer:
- Consensus rewards useful AI computation (PoUW layer)
- The state model has native primitives for AI agents, models, and inference requests
- Smart contracts can invoke verified AI inference as a built-in operation, not an external call
- AI agents have on-chain identity and can transact autonomously (Know Your Agent / KYA framework per a16z's 2026 research)

This is architecturally distinct from Bittensor (which is an AI-only network with no general smart contracts), Ritual (which is a bridge layer between AI and existing chains), and the ASI Alliance (which is a merger of three separate architectures). Aztibase Network makes AI a native protocol primitive alongside tokens, contracts, and accounts.

*Ref: RESEARCH_BRIEF.md Sections 4.1-4.5, 5.2, 5.6*

**Pillar 2: True Server-Independence**

Every node is sovereign. No RPC providers, no centralized indexers, no bootstrap servers that are single points of failure. The architecture achieves this through:
- DHT-based peer discovery with multiple bootstrap strategies (Kademlia + DNS seeds + mDNS for local networks)
- Every node exposes its own RPC endpoint (no Infura/Alchemy dependency)
- Browser-native participation via WASM + WebRTC (not a wrapper around a centralized API)
- Incentivized relay infrastructure (full nodes earn fees for NAT traversal relay services)
- No centralized message brokers -- gossip-based propagation only

This is not just a feature; it is an architectural constraint that shapes every other decision.

*Ref: RESEARCH_BRIEF.md Section 7.1-7.5*

**Pillar 3: Privacy at the Protocol Level**

Following a16z's 2026 thesis that "privacy creates chain lock-in through a privacy network effect," Aztibase Network implements privacy as a protocol-level capability, not an application-layer add-on:
- All transactions support optional privacy (selective disclosure)
- ZKP-based privacy with regulatory compliance hooks (prove attributes without revealing data)
- Agent-to-agent private channels for confidential AI inference
- Privacy-preserving state proofs for light clients

Privacy at the protocol level creates a moat that is architecturally impossible to replicate by chains that bolted on privacy as an afterthought. Bridging tokens is trivial; bridging secrets is hard.

*Ref: RESEARCH_BRIEF.md Section 5.1*
*Legal flag: LEGAL_LANDSCAPE.md Section 4 -- privacy features must support selective disclosure for regulatory compliance (AML/KYC). Full anonymity is not the goal; user-controlled privacy with compliance hooks is.*

---

### 1.2 Chain Structure

#### 1.2.1 Block Format

Aztibase Network uses a **DAG-based block structure** rather than a traditional linear chain.

**Block header:**
```
GenesisBlockHeader {
    version:            u16,
    slot:               u64,             // Logical time slot
    round:              u64,             // DAG round number
    author:             ValidatorId,     // Block proposer
    parents:            Vec<BlockHash>,  // Multiple parent references (DAG edges)
    state_root:         VerkleRoot,      // Verkle tree root of post-execution state
    transactions_root:  Hash,            // Root of transaction Merkle tree
    receipts_root:      Hash,            // Root of execution receipts
    ai_commitment_root: Hash,            // Root of AI work commitments (PoUW layer)
    timestamp:          u64,             // Unix timestamp (milliseconds)
    signature:          Signature,       // Ed25519 signature of proposer
}
```

**Block body:**
```
GenesisBlockBody {
    transactions:       Vec<Transaction>,
    ai_attestations:    Vec<AIAttestation>,     // Verified AI work results
    privacy_proofs:     Vec<ZKProof>,           // Privacy proofs for shielded transactions
    validator_set_diff: Option<ValidatorDiff>,  // Validator set changes (if any)
}
```

**Key design decisions:**

- **Multiple parents (DAG):** Each block references 2+ parent blocks from the same DAG round. This allows validators to propose blocks in parallel, increasing throughput without increasing latency. Follows the pattern proven by Sui's Mysticeti (39ms finality at 100k TPS) and validated by academic research on DAG-BFT protocols.
- **AI commitment root:** A dedicated Merkle root for AI work commitments. This makes AI attestations verifiable at the header level without downloading the full block -- critical for light clients that need to verify AI computation proofs.
- **Privacy proofs in body:** ZK proofs for shielded transactions are included in the block body, not headers, to keep headers lightweight.

*Ref: RESEARCH_BRIEF.md Section 2.1 (DAG consensus), Section 1.1 (Sui's Mysticeti)*

#### 1.2.2 Block Size and Block Time

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| **Target block time** | 400ms | Sub-second finality is table stakes (RESEARCH_BRIEF.md Section 5.5). 400ms matches Sei's proven performance while leaving headroom for DAG consensus overhead. |
| **Max block size** | 2MB initial, protocol-governed expansion | Start conservative. Celestia started at 2MB and is planning expansion to 1GB. Our modular architecture allows the same trajectory without hard forks. |
| **Target finality** | <1 second | DAG-based BFT achieves this through parallel block proposal. Target is 500ms end-to-end finality under normal conditions. |
| **Target throughput** | 10,000+ TPS at launch | Conservative relative to benchmarks (Monad achieves 10k TPS production, Sui benchmarks 100k TPS). Real-world performance always falls below benchmarks. |

#### 1.2.3 State Model: Hybrid Account + Object

Aztibase Network uses a **hybrid account-based model with first-class objects**, drawing from both Ethereum's account model and Sui's object model:

- **Accounts:** Standard externally-owned accounts (EOAs) and contract accounts, similar to Ethereum. Familiar to developers, compatible with existing tooling.
- **Objects:** First-class on-chain objects for assets that benefit from independent ownership and parallel processing. AI models, AI agent identities, NFTs, and complex assets are objects.
- **Why hybrid:** Pure account-based (Ethereum) limits parallelism because transactions touching the same account must be serialized. Pure object-based (Sui) breaks developer expectations and makes EVM compatibility harder. The hybrid model gives us parallel execution for independent objects while maintaining account-based compatibility for DeFi and standard operations.
- **Agent accounts:** A new account type for AI agents, with on-chain identity (KYA), operational constraints, spending limits, and linkage to a human principal. This is a protocol-level primitive, not a smart contract pattern.

*Ref: RESEARCH_BRIEF.md Section 1.1 (Sui objects), Section 1.2 (Aptos Block-STM parallelism), Section 5.2 (agent infrastructure)*

**Parallel execution strategy:**
- Transactions touching different objects execute in parallel (Sui pattern)
- Transactions touching the same account use optimistic parallel execution with conflict detection (Block-STM / Sei pattern)
- AI inference requests are inherently parallelizable (different model invocations are independent)

---

### 1.3 Overall Architecture: Modular Monolith

Aztibase Network adopts a **modular monolith** architecture -- a single unified chain with clearly separated internal layers that can be independently upgraded, but NOT a multi-chain/rollup architecture.

**Why not fully modular (Celestia-style)?**
- Fully modular architectures (separate DA, execution, settlement chains) add latency, complexity, and cross-layer trust assumptions.
- Our server-independence mandate is harder to achieve across multiple separate chains.
- Celestia's DA-only model means "its value is indirect" (RESEARCH_BRIEF.md Section 1.3) -- we want direct value capture.

**Why not fully monolithic (old Ethereum-style)?**
- Monolithic chains resist upgrades and create tight coupling between layers.
- Our AI-native features will evolve faster than consensus -- they must be independently upgradeable.

**The modular monolith compromise:**
- Single chain, single validator set, single token
- Internally separated into four layers with clean interfaces
- Each layer can be upgraded independently via on-chain governance (Substrate-style forkless upgrades via WASM meta-protocol)

#### Layer Architecture

```
+------------------------------------------------------------------+
|                    LAYER 4: APPLICATION                            |
|  Smart contracts (WASM + EVM compat) | AI oracles | Agent runtime |
+------------------------------------------------------------------+
|                    LAYER 3: EXECUTION                             |
|  Transaction processing | Parallel VM | State transitions         |
|  Object resolution | Privacy proof verification                   |
+------------------------------------------------------------------+
|                    LAYER 2: CONSENSUS                             |
|  DAG-BFT (primary) | PoUW (secondary) | Validator management     |
|  Block ordering | Finality gadget                                 |
+------------------------------------------------------------------+
|                    LAYER 1: NETWORK + DATA                        |
|  P2P (rust-libp2p) | Gossip | Block propagation | State sync     |
|  Storage (redb + Verkle) | Data availability                  |
+------------------------------------------------------------------+
```

**Layer 1 -- Network + Data:**
- rust-libp2p with `NetworkTransport` abstraction (per stack ruling)
- Gossipsub for block/transaction propagation with priority batching
- Kademlia DHT for peer discovery
- WebRTC transport for browser nodes
- redb with dual storage architecture (state store + state commitment)
- Verkle trees for state commitments
- Data availability sampling for light client verification

**Layer 2 -- Consensus:**
- DAG-based BFT as the primary consensus mechanism (consensus-engineer designs specifics)
- PoUW secondary layer for AI compute verification
- Validator set management with stake-weighted participation
- Block ordering: DAG vertices are partially ordered by causal dependencies, then totally ordered by a deterministic ordering rule for execution
- Finality gadget provides economic finality once 2/3+ stake has committed

**Layer 3 -- Execution:**
- Custom VM with WASM compilation target
- Optimistic parallel execution with conflict detection (Block-STM pattern)
- Object resolution engine for hybrid account+object state model
- ZKP verification for privacy proofs and AI attestations
- Fee market with priority transactions and congestion management FROM GENESIS (Solana lesson: RESEARCH_BRIEF.md Section 9.2)

**Layer 4 -- Application:**
- WASM smart contract runtime (contracts compile from Rust, AssemblyScript, C)
- EVM compatibility layer via `revm` (secondary execution environment for developer adoption)
- AI Oracle Interface: standardized primitive for requesting inference, verifying results, composing AI outputs with contract logic
- Agent Runtime: execution environment for autonomous AI agents with on-chain identity and spending constraints
- Governance module for protocol upgrades

*Ref: RESEARCH_BRIEF.md Section 1.3 (Celestia modularity), Section 6.2-6.3 (framework decisions), Section 9.2 (Solana congestion lessons)*

---

### 1.4 Server-Independence Architecture

Server-independence is not a feature; it is a hard architectural constraint. Every design decision must pass the test: "Does this work if every centralized server in the world goes offline?"

#### 1.4.1 Bootstrap Strategy (Zero Central Servers)

The bootstrap problem -- how does a new node find the network? -- is solved through a layered strategy:

| Priority | Method | Description | Central dependency |
|----------|--------|-------------|-------------------|
| 1 | **Hardcoded genesis peers** | 50+ geographically distributed initial peers compiled into the binary | None (embedded in code) |
| 2 | **DNS seed records** | Multiple independent DNS seeds operated by different parties | DNS infrastructure (distributed) |
| 3 | **Kademlia DHT** | Once connected to ANY peer, DHT discovery finds the rest | None |
| 4 | **mDNS local discovery** | Find peers on the same local network | None |
| 5 | **QR code / manual peer exchange** | Offline peer sharing for censorship-resistant bootstrap | None |
| 6 | **DHT peer caching** | Nodes persist known peers to disk; on restart, try cached peers first | None |

After initial bootstrap, the node operates entirely through Kademlia DHT. No ongoing dependency on bootstrap infrastructure.

#### 1.4.2 State Sync

New nodes joining the network need chain state. Three strategies, progressively trustless:

1. **Snapshot sync** (fastest): Download a recent state snapshot from peers, verify the Verkle state root against the finalized chain, then sync forward from the snapshot slot. Trust assumption: the finalized chain is honest (same as all PoS chains).
2. **Fast sync** (medium): Download block headers from genesis, verify the header chain, then download the latest state. Verifies the full header chain but trusts state proofs.
3. **Full sync** (most trustless): Replay every block from genesis. Only needed for archival purposes.

Light clients use **header sync only** + Verkle state proofs for any data they need. The constant-size Verkle proofs make this practical even on mobile.

#### 1.4.3 Chain Selection Rule

- **Finalized chain:** Blocks with 2/3+ stake commitment are irreversible. This is the canonical chain.
- **Unfinalized tip:** For the unfinalized DAG tip, the chain selection rule follows the "heaviest DAG" -- the DAG branch with the most cumulative stake-weighted vertices is preferred.
- **Fork resolution:** DAG-BFT should produce very few forks (unlike PoW). When forks occur in the unfinalized window, validators converge on the branch with the most parent references from honest validators.

#### 1.4.4 Every Node is a Server

- Every full node exposes a local JSON-RPC endpoint.
- Wallets, dApps, and SDKs connect to the local node, not to a cloud RPC provider.
- For users who cannot run a full node, browser light nodes connect directly to the P2P network via WebRTC -- no centralized intermediary.
- The SDK defaults to local-first: discover local node, then P2P light client, then (as last resort) a configurable remote RPC.

#### 1.4.5 Incentivized Relay Infrastructure

NAT traversal is the Achilles heel of P2P networks. Aztibase Network solves this economically:
- Full nodes that serve as TURN relays for NAT-traversal earn a small share of transaction fees.
- This creates an economic incentive to run publicly reachable nodes.
- The protocol tracks relay service quality (uptime, bandwidth, latency) and rewards accordingly.
- Target: near-100% NAT traversal success rate with <2s join latency (per RESEARCH_BRIEF.md Section 7.3).

---

### 1.5 State Management: Verkle Trees with Post-Quantum Escape Hatch

**Primary:** Verkle trees for state commitments.
- Constant-size proofs regardless of tree depth
- 100x smaller proofs than Merkle Patricia Tries
- Critical for light clients, browser nodes, and mobile nodes
- Aligned with Ethereum's roadmap (Kaustinen testnet)

**Post-quantum concern:** Verkle trees use elliptic curve-based polynomial commitments (e.g., IPA or KZG) that are NOT secure against quantum computers. The RESEARCH_BRIEF.md Section 8.2 flags this directly.

**Mitigation -- designed-in escape hatch:**
1. The `StateCommitment` trait is abstract. Verkle trees implement it, but so can binary Merkle trees + SNARKs.
2. The protocol governance can switch the state commitment scheme via a forkless upgrade.
3. All state proofs include a `commitment_scheme_version` field.
4. Binary Merkle tree + SNARK implementation is maintained as an alternative backend from day one (not deployed, but tested).

**Dual storage (from Sei's architecture):**
- **State Store:** Raw key-value store (redb) optimized for read/write latency. Used by the execution layer.
- **State Commitment:** Verkle tree structure used for generating proofs. Updated asynchronously after execution.
- This separation reduces I/O contention and allows the execution engine to operate at full speed without waiting for tree updates.

*Ref: RESEARCH_BRIEF.md Section 8.2 (Verkle trees, quantum concerns, Sei dual storage)*

---

### 1.6 AI-Native Architecture: Where AI Lives in the Protocol

This section defines WHERE in the architecture AI integrates. The ai-integration-engineer skill defines HOW.

#### 1.6.1 Protocol-Level AI Primitives

The following are first-class protocol objects, not smart contract abstractions:

| Primitive | Layer | Description |
|-----------|-------|-------------|
| `AIAgent` | Layer 4 (Application) | On-chain identity for autonomous AI agents. Contains: owner (human principal), operational constraints, spending limits, capability declarations, KYA attestation. |
| `ModelRegistry` | Layer 3 (Execution) | On-chain registry of AI model fingerprints (hashes). Models are stored off-chain; only their identity and verification metadata live on-chain. |
| `InferenceRequest` | Layer 3 (Execution) | A transaction type that requests AI inference. Contains: model ID, input hash, maximum fee, required proof type (ZKP / TEE attestation / multi-party verification). |
| `InferenceAttestation` | Layer 2 (Consensus) | Verified result of AI inference. Produced by PoUW validators who performed the computation. Contains: result hash, proof, validator signatures. |
| `AIComputeCommitment` | Layer 2 (Consensus) | Validator's commitment to perform AI computation as part of PoUW. Staked and slashable. |

#### 1.6.2 AI at the Consensus Layer (PoUW)

AI integrates into consensus through the Proof of Useful Work secondary layer:

```
Standard DAG-BFT consensus (all validators):
  - Block proposal, voting, finality
  - This runs regardless of AI workload
  - This is the security backbone

PoUW overlay (opt-in by validators with GPU/compute resources):
  - Validators commit to process AI inference requests
  - Results are verified through one or more of:
    (a) ZK proofs of correct inference (most trustless, highest overhead)
    (b) TEE attestation (hardware-based, medium trust)
    (c) Multi-party redundant execution (n-of-m agreement, simplest)
  - Verified results become InferenceAttestations included in blocks
  - Validators earn additional rewards for verified useful work
  - Quality metrics: accuracy, latency, availability (NOT just speed)
```

**Why PoUW is a secondary layer, not the primary consensus:**
- Primary consensus must be simple, proven, and fast. DAG-BFT satisfies this.
- PoUW introduces verification complexity and novel attack surfaces (RESEARCH_BRIEF.md Section 2.2: "Gaming the system is a real threat").
- Making PoUW optional means the chain functions even if no validators opt into AI compute.
- This is a lesson from Bittensor: coupling AI quality evaluation directly to block production creates fragility (RESEARCH_BRIEF.md Section 9.4).

#### 1.6.3 AI at the Execution Layer

The execution layer provides:
- **AI Oracle precompile:** Smart contracts can call `ai_inference(model_id, input)` as a precompiled operation. The execution engine routes this to the PoUW layer and returns the verified result.
- **Agent execution context:** AI agents execute within a sandboxed context with their declared capabilities and spending limits enforced at the VM level.
- **Inference result caching:** Identical inference requests return cached results (with freshness guarantees) to avoid redundant computation.

#### 1.6.4 AI at the Application Layer

- **Agent-to-agent payment channels:** Protocol-native payment channels optimized for microtransactions between AI agents (x402 pattern per a16z research).
- **Model marketplace primitives:** On-chain model registration, licensing, and usage metering.
- **Composable AI outputs:** Inference results are typed on-chain objects that can be consumed by other contracts -- enabling chains of AI inference within a single transaction.

*Ref: RESEARCH_BRIEF.md Section 4.3 (Ritual's off-chain compute + on-chain verification), Section 4.5 (key gaps), Section 5.2 (agent infrastructure), Section 10.4 (AI-native recommendations)*

---

### 1.7 Privacy Architecture

#### 1.7.1 Selective Disclosure Model

Aztibase Network implements a **selective disclosure** privacy model, not full anonymity:

- **Public transactions:** Default. Full transparency, identical to Ethereum.
- **Shielded transactions:** Opt-in. Transaction details (sender, receiver, amount) are hidden behind ZK proofs. The chain verifies correctness without seeing the data.
- **Selective disclosure:** Shielded transaction participants can produce disclosure proofs for specific parties (auditors, regulators, counterparties) without revealing data to the public chain.

This design satisfies both the privacy moat thesis (RESEARCH_BRIEF.md Section 5.1) and regulatory compliance requirements (LEGAL_LANDSCAPE.md Section 4 -- AML/KYC obligations across US, EU, and SA jurisdictions).

#### 1.7.2 Privacy for AI

AI inference introduces unique privacy requirements:
- **Model weight privacy:** Model owners may not want to expose weights. TEE-based inference or homomorphic encryption protects them.
- **Input privacy:** Users may not want their inference inputs visible. Encrypted inputs with ZK proof of correct processing.
- **Output privacy:** Inference results can be shielded and selectively disclosed.

This creates a "private AI compute market" -- one of our three target killer applications.

*Legal flag: LEGAL_LANDSCAPE.md Section 4.1 -- the SEC's evolving framework may classify privacy features differently. Monitor the CLARITY Act's treatment of privacy-preserving protocols. Design privacy as user-controlled (the user chooses to shield), not protocol-enforced (everything is private by default), to reduce regulatory risk.*

---

### 1.8 Framework for Downstream Skills

The following architectural decisions are BINDING constraints for all downstream skills. Each skill must design within these boundaries or raise a formal CHALLENGE.

#### For consensus-engineer (Section 2):

1. **Consensus type:** DAG-based BFT with PoUW secondary layer. The primary consensus mechanism must achieve <1s finality and 10,000+ TPS.
2. **DAG structure:** Validators propose blocks in parallel. Each block references multiple parents. A deterministic ordering rule produces a total order for execution.
3. **PoUW is optional:** Validators opt into AI compute. The chain must function normally with zero PoUW participants.
4. **Verification for PoUW:** Support at least two of: ZK proofs, TEE attestation, multi-party redundant execution. Multi-metric quality evaluation (not speed-only).
5. **Slashing:** Provably incorrect AI results must be slashable. Standard BFT double-signing/equivocation slashing applies.
6. **Patent constraint:** Conduct FTO analysis against nChain patent portfolio (1,308 patents) before finalizing consensus design. (LEGAL_LANDSCAPE.md Section 3.2)

#### For node-engineer (Section 4):

1. **Four node types:** Full, Light, Browser, Mobile. All are first-class citizens.
2. **Full node target:** 8GB RAM, consumer SSD, <100GB storage first year.
3. **Light node:** Header-only + Verkle state proofs. 512MB RAM. Must be able to verify any state claim without trusting the full node.
4. **Browser node:** WASM-compiled light client. WebRTC P2P. IndexedDB/OPFS for persistence. Runs in a standard browser tab.
5. **Mobile node:** Light client. Must handle intermittent connectivity, battery constraints, iOS background restrictions.
6. **Storage:** redb with dual storage architecture (state store + state commitment). Verkle trees for state commitments.
7. **State pruning:** From genesis. Nodes should be able to prune historical state beyond a configurable retention period while maintaining verifiability.
8. **Every node is an RPC endpoint.** No architectural dependency on centralized RPC providers.

#### For p2p-network-engineer (Section 8):

1. **P2P stack:** rust-libp2p with `NetworkTransport` abstraction trait.
2. **Peer discovery:** Kademlia DHT primary, DNS seeds secondary, mDNS tertiary.
3. **Block propagation:** Gossipsub with priority-based message batching to mitigate stress-performance degradation.
4. **Browser transport:** WebRTC via libp2p-webrtc.
5. **NAT traversal:** ICE with decentralized incentivized TURN relays.
6. **Encryption:** Noise protocol for all connections.
7. **Bootstrap:** Multi-strategy as defined in Section 1.4.1. No single point of failure.
8. **Congestion management:** Rate limiting and backpressure mechanisms at the network layer.

#### For security-engineer (Section 5):

1. **Threat model must include:** DAG-specific attacks (vertex withholding, selective parent referencing), PoUW gaming (garbage AI outputs, adversarial model manipulation), privacy proof soundness, agent autonomy abuse (runaway agents draining funds).
2. **Client diversity:** The architecture must support multiple client implementations from early development (Solana lesson: RESEARCH_BRIEF.md Section 9.2).
3. **Cryptographic assumptions:** BLAKE3 (hashing), Ed25519 (signatures), Verkle tree commitments (state). Flag any concerns with these choices.
4. **Post-quantum readiness:** The Verkle-to-binary-Merkle migration path must be security-reviewed.

#### For ai-integration-engineer (Section 6):

1. **On-chain AI primitives are defined above in Section 1.6.** Your job is the implementation design, not the architectural placement.
2. **All AI inference is off-chain with on-chain verification.** No ML models run inside the VM.
3. **Inference runtime:** Primary is `tract`, secondary is `candle`. See stack ruling above.
4. **Verification methods:** Design at least two of ZKP, TEE attestation, multi-party redundant execution.
5. **Agent identity:** Design the KYA (Know Your Agent) framework implementation within the `AIAgent` protocol primitive.
6. **Model registry:** Design the model fingerprinting, versioning, and licensing mechanism within the `ModelRegistry` primitive.
7. **Learn from Bittensor's failures:** Multi-metric quality evaluation, adversarial-resistant scoring, no speed-only metrics. (RESEARCH_BRIEF.md Section 4.1, 9.4)

#### For smart-contract-engineer (Section 7):

1. **Primary VM:** Custom WASM runtime (wasmtime as runtime engine per stack ruling).
2. **Secondary VM:** EVM compatibility via `revm`. This is for developer adoption, not primary innovation.
3. **Contracts compile from:** Rust, AssemblyScript, C (WASM targets). Solidity (EVM compat layer).
4. **AI Oracle precompile:** Contracts can call `ai_inference()` as a precompiled operation. You design the interface.
5. **Fee market:** From genesis. Priority fees + local fee markets + congestion-based base fee. (Solana lesson)
6. **Deterministic execution:** All execution must be deterministic. AI inference is non-deterministic by nature -- the AI Oracle resolves this by returning verified attestations, not raw model outputs.
7. **NO custom smart contract language.** We use existing languages. Sui and Aptos proved that custom languages (Move) create developer adoption barriers. (RESEARCH_BRIEF.md Section 1.1, 1.2)

#### For tokenomics-engineer (Section 3):

1. **Single native token.** Not a multi-token model (Berachain's tri-token model creates user confusion per RESEARCH_BRIEF.md Section 1.6).
2. **All minting/burning mechanisms must be bounded.** No unbounded recursive minting. (Terra lesson: RESEARCH_BRIEF.md Section 9.1)
3. **PoUW rewards come from a dedicated allocation**, not inflation. AI compute is paid for by users requesting inference, with a protocol fee going to PoUW validators.
4. **Relay incentives (NAT traversal TURN relays) come from transaction fees**, not inflation.
5. **Fair launch model preferred** to reduce securities classification risk. (LEGAL_LANDSCAPE.md Section 4.1)
6. **Economic model must be formally verifiable** under adversarial conditions. Simulate death spirals before launch.

---

### 1.9 Target Use Cases (Architectural Focus)

The architecture is optimized for three killer applications (per RESEARCH_BRIEF.md Section 10.5 Recommendation 15):

1. **Private AI Compute Markets:** Users submit encrypted inference requests. PoUW validators process them in TEEs or via ZK-verified computation. Results are returned with cryptographic proof of correctness. Neither the model weights nor the user's data are exposed to the public chain.

2. **Privacy-Preserving Identity and Credentials:** Self-sovereign identity with selective disclosure. Prove you are over 18 without revealing your birthdate. Prove you are KYC-verified without revealing your identity to the counterparty. The `AIAgent` primitive extends this to AI entities.

3. **Agent-to-Agent Autonomous Payments:** AI agents with on-chain identity, spending limits, and capability declarations transact with each other using protocol-native payment channels. Microtransactions for API calls, GPU time, data access. Human principals set constraints; agents operate within them.

These are not "features to add later." The architecture is designed around them. Every layer decision -- the hybrid state model, the PoUW consensus layer, the privacy primitives, the AI Oracle, the agent accounts -- exists to make these three use cases native and efficient.

---

### 1.10 Legal Alignment Summary

| Architectural Decision | Legal Concern | Mitigation |
|------------------------|---------------|------------|
| DAG-BFT consensus | nChain patent portfolio (1,308 patents) | FTO analysis mandatory (LEGAL_LANDSCAPE.md 3.2) |
| PoUW for AI compute | AI+consensus patent filings growing | File provisional patents on novel mechanisms; join COPA (LEGAL_LANDSCAPE.md 3.4) |
| Privacy features | AML/KYC regulations across jurisdictions | Selective disclosure, not full anonymity; compliance hooks built in (LEGAL_LANDSCAPE.md 4.1-4.3) |
| Native token | Howey Test / securities classification | Fair launch model; sufficient decentralization; no yield promises (LEGAL_LANDSCAPE.md 4.1) |
| Open-source code | Patent trolls (58% of blockchain litigation) | Dual MIT/Apache 2.0 for patent grant; join COPA; DCO for contributions (LEGAL_LANDSCAPE.md 3.3, 8) |
| "Genesis" naming | Multiple name collisions flagged | Defer to naming-council; prepare distinctiveness arguments or alternatives (LEGAL_LANDSCAPE.md 6, 9.4) |

---

### 1.11 What We Are NOT Building

Clarity on non-goals is as important as goals:

1. **Not an L2 / rollup.** We are a sovereign L1. No dependency on Ethereum or any other chain for settlement or data availability.
2. **Not an AI-only network.** Unlike Bittensor, Aztibase Network supports general smart contracts, DeFi, and arbitrary applications. AI is a first-class citizen, not the only citizen.
3. **Not EVM-first.** EVM compatibility is a secondary execution environment for developer adoption. Innovation happens in the WASM VM.
4. **Not a platform controlled by a single entity.** Protocol, not platform. No single entity controls the validator set or governance. (Enterprise blockchain failure lesson: RESEARCH_BRIEF.md Section 9.3)
5. **Not fully anonymous.** Privacy is user-controlled and compliance-friendly. Not a "dark chain."

---

**Stack challenges raised:** None (all challenges from research have been addressed via stack rulings in Section 0)
**Conflicts raised:** None
**Sign-off:** blockchain-architect

---

## SECTION 2: CONSENSUS MECHANISM

---
**Contributed by:** consensus-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete), LEGAL_LANDSCAPE.md (complete), Section 0 stack rulings (reviewed), Section 1 architecture (reviewed)

---

### CONSENSUS DESIGN: Synaptic Consensus (SynBFT + PoUW)

```
- Type: Novel Hybrid — DAG-based BFT ("SynBFT") with Proof of Useful Work overlay
- Finality: Absolute (deterministic) — Time to finality: <800ms under normal conditions
- Block time: 400ms target (per round)
- Throughput: 10,000+ TPS at launch, architecture supports 50,000+ with validator scaling
- Validator set: 100-200 active validators, stake-weighted selection with rotation
- Energy profile: Low (PoS-based BFT + optional PoUW using existing AI compute demand)
- Server-independence: Fully P2P leader rotation via VRF; no central coordinator
- AI integration points: PoUW layer, InferenceAttestation in block body, AIComputeCommitment in consensus voting
- Attack resistance: BFT 1/3 fault tolerance, PoUW verification via multi-method attestation, DAG equivocation detection, economic slashing
- Stack implications: STACK CHALLENGE raised on signature scheme (BLS required alongside Ed25519)
```

---

### 2.1 Overview: What is Synaptic Consensus?

Synaptic Consensus is named for the way biological neural synapses form connections — many-to-many, parallel, strengthening through repeated validated signaling. It is a two-layer consensus design:

1. **SynBFT (Layer A):** A DAG-based Byzantine Fault Tolerant protocol that provides ordering and finality. This is the security backbone. ALL validators participate in this layer.

2. **PoUW Overlay (Layer B):** A Proof of Useful Work layer where validators with AI compute resources commit to processing inference requests. This layer is opt-in and produces `InferenceAttestation` objects that are included in blocks. PoUW does NOT determine block validity — it determines supplemental rewards and AI service availability.

**Why two layers:** Coupling AI verification directly to block production creates fragility (Bittensor lesson — RESEARCH_BRIEF.md Section 9.4). SynBFT ensures the chain operates at full speed and security even if zero validators opt into PoUW. PoUW provides the AI-native value proposition without jeopardizing liveness.

---

### 2.2 SynBFT: The DAG-BFT Protocol

#### 2.2.1 Design Lineage and Differentiation

SynBFT draws on research from several DAG-BFT protocols but copies none:

| Protocol | What we learn | Where we diverge |
|----------|--------------|-----------------|
| **Narwhal/Tusk** (Danezis et al., 2022) | Separate data availability (Narwhal mempool DAG) from ordering (Tusk/Bullshark) | We do NOT separate mempool from consensus into two protocols. SynBFT is a single unified protocol with DAG structure. Narwhal's two-phase approach adds complexity and latency. |
| **Mysticeti** (Sui, 2024) | Uncertified DAG — skip the "certify before reference" step to reduce latency | We adopt the uncertified DAG approach. Validators can reference blocks they have received even without waiting for 2f+1 acknowledgments. This is the key to sub-second finality. |
| **Shoal++** (2025 research) | Pipelining and leader reputation for DAG ordering | We incorporate leader reputation scoring but use a different reputation model that includes PoUW participation as a factor (not just consensus performance). |
| **Avalanche** (Team Rocket, 2020) | Repeated random subsampling for probabilistic finality | We do NOT use probabilistic finality. SynBFT provides deterministic BFT finality. Avalanche's probabilistic model is unsuitable for financial applications requiring absolute finality guarantees. |
| **IOTA Tangle** | DAG without blocks (each transaction is a vertex) | We do NOT go blockless. Blocks aggregate transactions for efficiency in gossip propagation, execution batching, and state root computation. |

**Key innovation in SynBFT:** The integration of an "AI reputation signal" into leader scoring. Validators who consistently deliver verified, high-quality PoUW results receive a reputation bonus that increases their probability of being selected as the ordering leader (the "anchor" proposer). This creates a virtuous cycle: validators are incentivized to invest in AI compute capacity because it improves their consensus standing, which improves their base consensus rewards. However, the AI reputation signal is bounded to at most 15% of total leader score — preventing AI-rich validators from dominating consensus.

#### 2.2.2 DAG Structure

**Rounds and Vertices:**

SynBFT operates in discrete rounds, each targeting 400ms.

- Each round, every active validator MAY broadcast one **vertex** (block).
- A vertex contains: a batch of transactions, references to parent vertices from the previous round(s), and optionally `InferenceAttestation` objects from the PoUW layer.
- Each vertex MUST reference at least 2f+1 vertices from the immediately preceding round (where f = floor((n-1)/3) for n validators). This ensures causal connectivity.
- Vertices MAY additionally reference vertices from older rounds ("skip edges") to improve connectivity in the presence of stragglers.

```
Round r:     [V_a]   [V_b]   [V_c]   [V_d]   ... (parallel proposals)
              |\ \   /| \    / |  \  / |
              | \ \ / |  \  /  |   \/  |
              |  \ X  |   \/   |   /\  |
              |   / \ |   /\   |  /  \ |
Round r-1:   [V_e]   [V_f]   [V_g]   [V_h]   ... (referenced parents)
```

**Uncertified DAG:**

Following Mysticeti's insight, SynBFT uses an **uncertified** DAG. Validators broadcast vertices and reference other vertices they have received, without waiting for a quorum of acknowledgments before referencing. This eliminates one full round-trip of communication compared to Narwhal-style certified DAGs.

The tradeoff: equivocating validators (proposing two different vertices in the same round) are harder to detect immediately. SynBFT handles this through **retroactive equivocation detection** (Section 2.7.5).

#### 2.2.3 Ordering: Anchor-Based Commit Rule

**The ordering problem:** A DAG provides a partial order (causal dependencies). We need a total order for deterministic execution.

**Anchor election:**

Every k rounds (k=4 by default, configurable via governance), one round is designated an **anchor round**. In an anchor round, a designated **anchor proposer** is elected. The anchor proposer's vertex, if it gathers sufficient support, becomes the **anchor vertex** that commits a batch of DAG vertices to a total order.

**Anchor proposer selection:**

The anchor proposer is selected via a **Verifiable Random Function (VRF)** seeded by the previous anchor's hash, weighted by a composite score:

```
AnchorScore(v) = 0.70 * StakeWeight(v)
               + 0.15 * ConsensusReputation(v)
               + 0.15 * PoUWReputation(v)
```

Where:
- `StakeWeight(v)` = validator v's stake as a fraction of total stake
- `ConsensusReputation(v)` = rolling score based on recent consensus participation (timely vertex proposals, correct parent references, uptime). Decays over 1000 rounds.
- `PoUWReputation(v)` = rolling score based on verified AI compute delivered (quality-weighted, not quantity-only). Zero for validators not participating in PoUW. Decays over 2000 rounds.

The VRF output combined with the AnchorScore distribution determines the anchor proposer. The VRF ensures unpredictability; the score weighting ensures that reliable, high-quality validators are favored without being deterministic.

**Commit rule:**

An anchor vertex at round r commits if:
1. The anchor vertex is **reachable** from at least 2f+1 vertices in round r+1 (i.e., at least 2f+1 validators in the next round referenced a causal path that includes the anchor).
2. Once committed, the anchor retrospectively orders ALL uncommitted vertices reachable from it using a deterministic topological sort (round number ascending, then by BLAKE3 hash of vertex content for ties within the same round).

This is similar to Bullshark's commit rule but with the addition of skip-edge support and the AI reputation signal in anchor selection.

**Pipelining:**

Anchor rounds are pipelined. While round r's anchor is being committed, round r+k's anchor proposer is already being determined. This means multiple anchors can be "in flight" simultaneously, with commits happening as soon as the support threshold is met. Under normal conditions (>2/3 honest, reasonable network latency), an anchor commits within 2-3 rounds after its proposal — yielding finality in approximately 800ms at 400ms round times.

#### 2.2.4 Finality

**Type: Absolute (deterministic).**

Once an anchor vertex commits, all vertices it orders are **final and irreversible** under the BFT assumption that fewer than 1/3 of validators (by stake) are Byzantine. This is the same finality guarantee as Tendermint/CometBFT, PBFT, and HotStuff — but achieved with higher throughput due to the DAG's parallelism.

**Finality timing:**
- **Normal conditions** (all validators online, <200ms network latency): ~800ms from transaction submission to finality.
- **Degraded conditions** (up to 1/3 validators offline): finality degrades gracefully. Anchor commits take more rounds as fewer vertices are available for support. Worst case with exactly 1/3 offline: ~2-3 seconds.
- **Network partition:** See Section 2.6.

**No probabilistic fallback.** Unlike Avalanche or Nakamoto-style protocols, there is no window where a "finalized" transaction can be reverted. This is critical for financial applications, AI inference commitments, and agent-to-agent payments.

---

### 2.3 PoUW Integration: Proof of Useful Work for AI

#### 2.3.1 What Constitutes "Useful Work"

Useful work in Aztibase Network is **AI inference verification**. Specifically:

1. **Inference Execution:** A PoUW validator receives an `InferenceRequest` from the transaction pool. The request specifies a model (by hash from `ModelRegistry`), input data (by hash), and a required verification method.

2. **Computation:** The validator executes the inference off-chain using the specified model via `tract`, `candle`, or ONNX Runtime.

3. **Attestation:** The validator produces an `InferenceAttestation` containing the result hash, the verification proof, and the validator's signature.

4. **Inclusion:** The attestation is included in the validator's next DAG vertex (block body). Once the vertex is committed via SynBFT, the attestation is finalized on-chain.

**What is NOT useful work:**
- Arbitrary hash puzzles (PoW waste)
- Training full models (too long, too hard to verify, gaming risk)
- Benchmark tasks with known answers (trivially gameable)

**Why inference verification, not training:**
- Inference has bounded compute time (predictable for scheduling)
- Inference outputs are deterministic for a given model + input (verifiable)
- Inference is the primary demand-side use case (applications need inference, not training)
- Training verification is an unsolved research problem at scale

#### 2.3.2 Verification Without Re-Execution

The critical challenge: how do you verify an inference result without re-executing the entire computation? Aztibase Network supports three verification methods, ordered by trust level:

**Method A: Multi-Party Redundant Execution (MPRE)**

- The simplest and most immediately deployable method.
- An inference request is assigned to m validators (default m=3). Each executes independently.
- If n-of-m results agree (default 2-of-3), the result is accepted.
- Disagreeing validators are flagged for investigation. Persistent disagreement leads to slashing.
- **Tradeoff:** Requires m times the compute. Acceptable for high-value inference requests.
- **Determinism handling:** Floating-point non-determinism across hardware is handled by quantizing model outputs to a fixed precision (e.g., 16-bit fixed-point comparison) before consensus comparison. The full-precision result is stored, but agreement is checked at reduced precision.

**Method B: TEE Attestation (Trusted Execution Environment)**

- Validators execute inference inside a TEE (Intel SGX, AMD SEV, ARM TrustZone).
- The TEE produces a hardware-signed attestation proving: (a) the correct model was loaded, (b) the correct input was used, (c) the output is genuine.
- **Tradeoff:** Requires TEE-capable hardware. Trust shifts to hardware manufacturer. Side-channel attacks on TEEs are a known research area (but practically difficult to exploit at scale).
- **Aztibase Network does NOT require TEE for consensus participation.** TEE is one verification option for PoUW, not a consensus requirement.

**Method C: ZK Proofs of Inference (zkML)**

- The gold standard for trustless verification. A ZK proof demonstrates correct execution of the inference computation without revealing the computation itself.
- **Current state (March 2026):** zkML is advancing rapidly. zkVM provers are approaching ~10,000x overhead (RESEARCH_BRIEF.md Section 5.3). For small models (anomaly detection, fraud scoring — our initial target models), ZK proofs are practical.
- **For large models:** ZK proofs remain impractical due to prover time. MPRE or TEE should be used.
- **Tradeoff:** Highest trust, highest computational overhead. Best suited for high-value, small-model inference requests.

**Method selection is per-request.** The `InferenceRequest` transaction specifies which verification method(s) are acceptable. The protocol does not mandate a single method — this allows the market to find the right tradeoff for each use case.

#### 2.3.3 PoUW Lifecycle

```
1. User submits InferenceRequest transaction
   ├── model_id: Hash (from ModelRegistry)
   ├── input_hash: Hash
   ├── max_fee: u64
   ├── verification_method: MPRE | TEE | ZK | ANY
   └── urgency: NORMAL | PRIORITY

2. InferenceRequest enters the PoUW mempool (separate from transaction mempool)

3. PoUW scheduler assigns request to eligible validators
   ├── Selection: weighted by stake, PoUW reputation, and hardware capability
   ├── For MPRE: assigns to m validators
   ├── For TEE/ZK: assigns to 1 validator (proof is self-verifying)
   └── Assignment is deterministic from VRF seed (auditable, non-manipulable)

4. Assigned validators execute inference off-chain
   ├── Download model from off-chain storage (IPFS, verified by model_id hash)
   ├── Download input from off-chain storage (verified by input_hash)
   ├── Execute inference via tract/candle/ONNX Runtime
   └── Generate verification proof per selected method

5. Validator produces InferenceAttestation
   ├── request_id: Hash
   ├── result_hash: Hash
   ├── proof: VerificationProof (MPRE signature | TEE attestation | ZK proof)
   ├── compute_time_ms: u64
   ├── validator_id: ValidatorId
   └── signature: Signature

6. InferenceAttestation included in validator's next DAG vertex (block body)

7. SynBFT commits the vertex → attestation is finalized on-chain

8. Requesting contract/user can read the verified result
```

#### 2.3.4 Quality Metrics (Avoiding the Bittensor Problem)

PoUW validators are evaluated on multiple dimensions, explicitly to avoid Bittensor's speed-only metric that led to garbage outputs (RESEARCH_BRIEF.md Section 4.1, 9.4):

| Metric | Weight | Description |
|--------|--------|-------------|
| **Correctness** | 40% | Agreement with other attestations (MPRE), valid proof (TEE/ZK). Binary pass/fail for individual requests; rolling average for reputation. |
| **Availability** | 25% | Fraction of assigned inference requests that the validator completed within the deadline. Measures reliability. |
| **Latency** | 20% | Time from assignment to attestation submission. Normalized against model complexity. Faster is better, but only after correctness and availability. |
| **Diversity** | 15% | Willingness to serve a diversity of model types and sizes, not just the cheapest/fastest models. Prevents validators from cherry-picking easy requests. |

These metrics feed into the `PoUWReputation(v)` score used in anchor proposer selection (Section 2.2.3).

#### 2.3.5 AIComputeCommitment and Block Validity

An `AIComputeCommitment` is a validator's staked declaration that they will perform PoUW for a specific epoch (1 epoch = 1000 rounds = ~400 seconds). The commitment includes:

```
AIComputeCommitment {
    validator_id:    ValidatorId,
    epoch:           u64,
    hardware_profile: HardwareAttestation,  // Self-declared compute capability
    models_supported: Vec<ModelHash>,        // Models the validator can run
    max_concurrent:  u16,                    // Max simultaneous inference requests
    stake_locked:    u64,                    // Additional stake locked for PoUW (slashable)
}
```

**Effect on block validity:**

- `AIComputeCommitment` does NOT affect block validity in SynBFT. A block without any AI content is fully valid.
- However, the `ai_commitment_root` in the block header (defined in Section 1.2.1) MUST correctly reflect any `InferenceAttestation` objects in the block body. If the Merkle root is incorrect, the block is invalid.
- Validators who submit an `AIComputeCommitment` but fail to deliver on assigned inference requests within the epoch lose reputation and may be slashed (see Section 2.7).

---

### 2.4 Validator Selection and Management

#### 2.4.1 Validator Set Composition

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| **Minimum validator set** | 21 | Minimum for meaningful BFT (f=7, tolerates 7 Byzantine validators). Below this, the chain halts rather than operating unsafely. |
| **Target validator set** | 100-200 | Balances decentralization with DAG communication overhead. Each validator must receive and validate vertices from all others each round. At 200 validators, this is 200 vertices per 400ms — manageable with modern hardware and networking. |
| **Maximum validator set** | 400 | Hard cap. Beyond this, DAG vertex propagation latency exceeds the 400ms round time on realistic networks. Can be increased via governance if network conditions improve. |
| **Minimum stake** | Protocol-governed (set by governance, not hardcoded) | Avoids Berachain's 250,000 BERA barrier problem (RESEARCH_BRIEF.md Section 1.6). Initial recommendation: low enough for enthusiast participation, adjusted by governance based on security needs. |

#### 2.4.2 Validator Selection Mechanism

**Stake-Weighted Random Sampling with Caps:**

1. Any account with at least the minimum stake can register as a validator candidate.
2. At each epoch boundary (every 1000 rounds), the active validator set is resampled.
3. Selection probability is proportional to stake, BUT:
   - **Maximum stake influence:** No single validator can hold more than 5% of the total effective stake for selection purposes. Stake above 5% is "capped" — it still earns proportional rewards but does not increase selection probability. This prevents plutocratic capture.
   - **Minimum diversity:** At least 30% of validator slots are reserved for validators outside the top-20 by stake. This ensures geographic and operational diversity.

4. Validators who have been in the active set for more than 10 consecutive epochs are subject to a **cooldown rotation** — they must sit out for 1 epoch before re-entering. This prevents indefinite incumbency and encourages a broader validator ecosystem.

5. **Emergency re-selection:** If more than 1/3 of the active set goes offline simultaneously (detected by consecutive rounds without their vertices), an emergency re-selection draws from the candidate pool to maintain >2/3 availability.

#### 2.4.3 Preventing Centralization

Multiple mechanisms work in concert:

- **Stake cap (5%):** Prevents whale dominance.
- **Diversity quota (30%):** Ensures smaller validators participate.
- **Cooldown rotation:** Prevents permanent incumbents.
- **Consumer hardware target:** Full node requirements (8GB RAM, SSD per Section 1.2) mean validators do not need datacenter hardware for SynBFT participation. PoUW requires more resources, but PoUW is optional.
- **No delegation concentration:** If delegation is implemented, delegated stake counts toward the 5% cap of the delegate. This prevents Lido-style concentration (where one liquid staking protocol controls >30% of stake).
- **Geographic diversity incentive:** Validators self-declare geographic region. The selection algorithm applies a mild preference (5% bonus to selection probability) for underrepresented regions. This is advisory, not enforceable (validators can lie about location), but creates a nudge toward diversity.

---

### 2.5 Finality Deep Dive

#### 2.5.1 Absolute Finality Guarantees

SynBFT provides **absolute BFT finality**: once an anchor vertex commits, all transactions it orders are irreversible under the assumption that <1/3 of stake is Byzantine.

**This is not probabilistic.** There is no "confirmation count" or "confidence score." A committed transaction is final. Period. This is essential for:
- Financial settlement (DeFi, payments)
- AI inference commitments (you cannot un-do a computation)
- Agent-to-agent payments (microtransactions need instant finality)

#### 2.5.2 Time to Finality Analysis

| Condition | Rounds to anchor commit | Time to finality |
|-----------|------------------------|-----------------|
| **Optimal** (all validators online, <100ms latency) | 2 rounds after anchor proposal | ~800ms |
| **Normal** (>90% online, <200ms latency) | 2-3 rounds | 800ms-1.2s |
| **Degraded** (67-90% online, mixed latency) | 3-5 rounds | 1.2s-2.0s |
| **Minimum viable** (exactly 67% online) | 5-8 rounds | 2.0s-3.2s |
| **Below threshold** (<67% online) | No commit possible | Chain halts (safety over liveness) |

The chain prioritizes **safety over liveness**. If fewer than 2/3 of validators (by stake) are producing vertices, the chain halts rather than finalizing potentially incorrect state. This is the standard BFT tradeoff and is appropriate for a financial blockchain.

#### 2.5.3 Network Partition Behavior

**During a partition:**
- Each partition continues producing DAG vertices locally but cannot commit anchors (neither partition has 2/3 of total stake, assuming a roughly even split).
- Transactions accumulate in the DAG but are not finalized.
- No state transitions are applied to either partition's execution layer.
- This prevents double-spend and state divergence.

**After partition heals:**
- Validators from both sides exchange vertices they missed.
- The DAG reconnects. The next anchor round has full participation and can commit.
- All accumulated vertices from both partitions are ordered and executed.
- Transactions that conflict across partitions are resolved by the total ordering rule (the anchor's deterministic sort determines which transaction "wins").

**Worst case:** A prolonged partition (>10 minutes) triggers an alert to all validators. Validators in the minority partition gracefully stop proposing to reduce DAG bloat. When the partition heals, only the majority partition's DAG is retained, and the minority partition's validators re-sync.

---

### 2.6 AI Hooks in Consensus

#### 2.6.1 Where AI Validates at the Consensus Level

AI does NOT validate transactions or determine block validity in SynBFT. This is deliberate — AI introduces opacity and adversarial attack surfaces into what must be a transparent, deterministic process (RESEARCH_BRIEF.md Section 2.3).

AI hooks in consensus are limited to:

1. **PoUW reputation influences anchor selection** (Section 2.2.3) — indirect influence only, capped at 15%.
2. **InferenceAttestation objects ride in block bodies** — they are committed to the chain via SynBFT like any other data, but do not affect SynBFT's operation.
3. **AIComputeCommitment affects validator responsibilities** — validators who commit to PoUW must deliver or face slashing.

This separation is a core design principle: **the consensus layer is AI-aware but not AI-dependent.**

#### 2.6.2 InferenceAttestation in Consensus Flow

```
InferenceAttestation lifecycle in consensus:

1. PoUW validator completes inference → produces InferenceAttestation

2. Attestation is included in the validator's DAG vertex (block body)
   ├── Block body field: ai_attestations: Vec<AIAttestation>
   └── Block header field: ai_commitment_root must be correct Merkle root

3. Other validators receiving this vertex:
   ├── Verify the ai_commitment_root matches the attestations in the body
   ├── For MPRE: check if this attestation agrees with others for the same request
   ├── For TEE: verify the TEE signature chain
   ├── For ZK: verify the ZK proof (fast verification, ~1ms for small proofs)
   └── If verification fails: mark the vertex as invalid, do not reference it

4. If the vertex is committed via anchor:
   └── InferenceAttestation becomes part of finalized state
   └── Requesting contract can consume the result
```

#### 2.6.3 AIComputeCommitment and Consensus Incentives

Validators who submit `AIComputeCommitment`:
- Are assigned inference requests by the PoUW scheduler
- Earn supplemental rewards from inference fees (paid by requesters)
- Build PoUW reputation that improves their anchor selection probability
- Lock additional stake that is slashable for failure to deliver

Validators who do NOT submit `AIComputeCommitment`:
- Participate in SynBFT normally
- Earn standard consensus rewards
- Have zero PoUW reputation (15% of anchor score is zero for them)
- Still fully valid validators — no penalty for not doing PoUW

This ensures the chain functions identically whether 0% or 100% of validators opt into PoUW.

---

### 2.7 Attack Resistance

#### 2.7.1 Byzantine Fault Tolerance (>1/3 Attack / "51% Attack")

SynBFT tolerates up to f = floor((n-1)/3) Byzantine validators by stake weight. This is standard BFT security. To compromise consensus, an attacker needs >1/3 of total stake (not >1/2 as in PoW).

**Mitigations beyond standard BFT:**
- Stake cap (5% per validator) means an attacker needs to control at least 7 separate validator identities to reach 1/3 — increasing the cost and complexity of a stake accumulation attack.
- Cooldown rotation means an attacker's validators cycle out periodically, requiring continuous re-entry.
- Slashing for equivocation (Section 2.7.5) means detected attacks have severe economic consequences.

#### 2.7.2 Nothing-at-Stake

The nothing-at-stake problem (validators voting on multiple forks without cost) is **largely inapplicable** to DAG-BFT:
- In SynBFT, validators propose ONE vertex per round. Proposing two different vertices in the same round is **equivocation** and is slashable.
- There are no "forks" in the traditional sense — the DAG structure absorbs concurrent proposals into a single structure.
- The anchor commit rule requires 2f+1 support, so a validator cannot profitably vote for conflicting anchors.

#### 2.7.3 Long-Range Attacks

Long-range attacks (rewriting history from a past state) are addressed by:
- **Absolute finality:** Committed blocks cannot be reverted. There is no longest-chain rule to exploit.
- **Weak subjectivity checkpoints:** New nodes syncing must obtain a recent finalized checkpoint from a trusted source (or multiple independent sources). This is the same approach used by Ethereum PoS.
- **Validator set bonding period:** Validators who unbond their stake must wait a full unbonding period (e.g., 21 days, governance-adjustable) before stake is returned. During this period, they can still be slashed for past misbehavior discovered retroactively.

#### 2.7.4 Sybil Attacks

Sybil resistance comes from staking:
- Creating many validator identities without proportional stake provides no advantage.
- The minimum stake requirement creates a floor cost for each Sybil identity.
- The 5% stake cap means splitting stake across identities is the only option — but splitting stake doesn't increase total influence; it only distributes the same weight.
- For PoUW, Sybil validators must also possess actual compute hardware to earn PoUW rewards, adding a real-world cost.

#### 2.7.5 DAG-Specific Attacks

**Equivocation (dual vertex proposal):**
- A validator proposes two different vertices for the same round, sending each to a different subset of peers.
- **Detection:** When any honest validator sees two vertices from the same author in the same round, it broadcasts an **equivocation proof** (both vertex headers signed by the same key).
- **Consequence:** Immediate slashing of the equivocator's stake. Both vertices are marked invalid. The equivocator is removed from the active set.

**Selective parent referencing (censorship):**
- A validator deliberately omits references to certain validators' vertices, attempting to delay their inclusion in the committed order.
- **Detection:** The protocol tracks "parent coverage" — each vertex must reference at least 2f+1 vertices from the previous round. A validator that consistently references only a subset of the available vertices (below the expected coverage given network conditions) accumulates a censorship score.
- **Consequence:** High censorship score reduces ConsensusReputation, reducing anchor selection probability. Extreme cases trigger investigation and potential slashing.

**Vertex withholding:**
- A validator produces a vertex but delays broadcasting it, attempting to manipulate the DAG structure.
- **Detection:** If a validator's vertex for round r is not received by any peer until round r+3 or later, it is treated as late and receives reduced weight in reputation scoring.
- **Consequence:** Persistent withholding reduces ConsensusReputation. Vertices withheld beyond a threshold (e.g., 10 rounds) are discarded entirely.

**DAG flooding:**
- An attacker broadcasts an excessive number of vertices (exceeding the 1-per-round limit) to overwhelm network bandwidth.
- **Detection:** Each validator's vertex rate is enforced: exactly 1 vertex per round per validator. Excess vertices are dropped at the network layer.
- **Consequence:** Violating vertices are ignored. Network layer rate-limiting (per Section 1.3, Layer 1) provides additional protection.

#### 2.7.6 PoUW-Specific Attacks

**Garbage inference outputs:**
- Validators return random or minimal-effort results to claim PoUW rewards.
- **Mitigations:**
  - MPRE: requires agreement with other independent executors.
  - TEE: hardware attestation proves correct execution.
  - ZK: mathematical proof of correct computation.
  - Multi-metric quality evaluation penalizes low-quality results (Section 2.3.4).
  - Repeated garbage outputs → reputation collapse → loss of PoUW assignments → eventually slashing.

**Model substitution:**
- A validator claims to run model A but actually runs a cheaper model B.
- **Mitigations:**
  - TEE attestation includes model hash verification.
  - ZK proofs are tied to the specific model circuit.
  - MPRE: disagreement with honest executors who ran the correct model.
  - Model hashes in `ModelRegistry` are verified against the off-chain model content hash.

**Adversarial inference requests (DoS via compute):**
- An attacker submits many expensive inference requests to exhaust PoUW validator compute.
- **Mitigations:**
  - Inference requests require fees proportional to estimated compute cost.
  - PoUW validators declare `max_concurrent` in their commitment — they cannot be overloaded beyond their declared capacity.
  - Fee market for inference requests: high demand increases fees, self-regulating the load.

---

### 2.8 Server-Independence in Consensus

#### 2.8.1 No Central Coordinator

SynBFT requires NO central coordinator for any operation:

- **Leader election:** Anchor proposers are selected via VRF seeded by on-chain randomness (previous anchor hash). Every validator can independently compute who the next anchor proposer should be.
- **Round synchronization:** Validators synchronize rounds via the DAG structure itself — a vertex referencing round r parents implicitly signals that the proposer is in round r+1. Wall-clock synchronization is loose (NTP-level, ~100ms accuracy is sufficient). No central time server is needed.
- **Vertex propagation:** Gossipsub (via rust-libp2p) distributes vertices to all validators. No message broker, no relay server.
- **View synchronization:** If validators fall out of sync (e.g., due to network delays), they resynchronize by observing the DAG's structure. A validator that is behind simply references whatever latest vertices it has received.

#### 2.8.2 Leader Election in P2P

The anchor proposer is elected purely from on-chain data:

```
anchor_seed = BLAKE3(previous_anchor_hash || anchor_round_number)
vrf_output = VRF_prove(validator_secret_key, anchor_seed)
selection_score = vrf_output * AnchorScore(validator)
// Highest selection_score among active validators wins
```

Every validator computes this independently and deterministically. There is no election protocol, no voting on leaders, no coordinator. The VRF ensures unpredictability (you cannot predict the anchor proposer more than k rounds in advance) and verifiability (anyone can verify the VRF proof to confirm the selected proposer is legitimate).

#### 2.8.3 Consensus Behind NAT

Validators behind NAT participate via the P2P layer's NAT traversal mechanisms (Section 1.4.5):

- **ICE/STUN:** ~85% of NAT configurations are traversable without relay.
- **Incentivized TURN relays:** For the remaining ~15%, relay infrastructure operated by full nodes provides connectivity. Consensus messages are small (vertex headers are ~500 bytes; full vertices with transactions are typically <100KB) and tolerant of ~50ms additional relay latency.
- **Minimum requirement:** A validator MUST be reachable by at least 2f+1 other validators within the round time (400ms). If a validator is consistently unreachable, it is rotated out of the active set.

There is no requirement for validators to have public IP addresses or run in datacenters. Home validators behind consumer NATs are explicitly supported, consistent with the server-independence mandate.

---

### 2.9 Stack Evaluation and Challenges

#### 2.9.1 Rust + tokio: ACCEPTED

Rust with tokio async is well-suited for SynBFT:
- Deterministic performance (no GC pauses) is critical for 400ms round timing.
- tokio's multi-threaded runtime handles concurrent vertex reception and validation efficiently.
- The Rust ecosystem has mature libraries for all required cryptographic operations.
- **No challenge raised.**

#### 2.9.2 BLAKE3 Hashing: ACCEPTED

BLAKE3 is appropriate for consensus:
- Used for vertex hashing, transaction Merkle roots, VRF seed computation.
- Faster than SHA-256, which reduces per-round processing time.
- Cryptographically secure with no known weaknesses.
- Tree hashing mode is useful for Merkle root computation.
- **No challenge raised.**

#### 2.9.3 Ed25519 Signatures: CHALLENGED (STACK CHALLENGE RAISED)

> **STACK CHALLENGE [consensus-engineer -> blockchain-architect]: Signature Scheme**
>
> **What is challenged:** Ed25519 as the SOLE signature scheme for consensus.
>
> **Proposed change:** Add BLS12-381 signatures as a REQUIRED secondary signature scheme for consensus, while retaining Ed25519 for transaction signing and general use.
>
> **Justification:**
>
> SynBFT requires validators to verify signatures on every vertex from every other validator each round. With 200 validators and 400ms rounds, each validator must verify ~200 signatures per round. Ed25519 verification is fast (~70 microseconds per signature), so 200 verifications take ~14ms — manageable.
>
> However, the anchor commit rule requires proving that 2f+1 validators support the anchor. Transmitting and verifying 134+ individual Ed25519 signatures is expensive in bandwidth and verification time, especially for light clients verifying finality proofs.
>
> **BLS signatures solve this through aggregation:**
> - 134 BLS signatures aggregate into a single signature of the same size (~96 bytes).
> - Verification of the aggregate is a single pairing operation (~1.5ms), not 134 individual verifications.
> - Light clients need only verify ONE aggregate signature to confirm finality, instead of 134 Ed25519 signatures.
> - This is the same approach used by Ethereum's beacon chain for committee attestations.
>
> **Proposed architecture:**
> - **Ed25519:** Used for transaction signing, general authentication, and individual vertex signatures within the DAG (fast individual verification).
> - **BLS12-381:** Used for aggregate finality proofs (anchor commit attestations). Each validator signs the anchor commitment with their BLS key. The aggregate signature is included in the finality certificate.
>
> **Impact:**
> - Adds ~2MB to the binary for BLS library (`blst` crate, Apache 2.0 licensed).
> - Validators must generate and manage a BLS keypair in addition to their Ed25519 keypair.
> - BLS individual signing is slower than Ed25519 (~1ms vs ~50us), but this is per-anchor, not per-vertex. At one anchor every 4 rounds, this is negligible.
>
> **Alternative considered:** Schnorr multi-signatures (MuSig2). These also support aggregation but require interactive signing rounds between participants, which is incompatible with our asynchronous DAG structure. BLS aggregation is non-interactive — validators sign independently and any node can aggregate the signatures.
>
> **Risk if not adopted:** Light clients and browser nodes cannot efficiently verify finality without downloading and checking 134+ individual signatures per anchor commit. This directly undermines the server-independence mandate and the viability of light/browser/mobile nodes for finality verification.

#### 2.9.4 Node Type Participation in Consensus

| Node Type | Consensus Role |
|-----------|---------------|
| **Full Node (Validator)** | Proposes vertices, validates all vertices, participates in anchor commits. Must be in the active validator set. |
| **Full Node (Non-Validator)** | Validates all vertices, maintains full DAG state, serves as a relay and data source for light clients. Does NOT propose vertices. |
| **Light Node** | Verifies finality certificates (aggregate BLS signatures on anchor commits). Can verify individual state proofs via Verkle proofs. Does NOT validate the full DAG. |
| **Browser Node** | Same as Light Node but running in WASM. Verifies finality certificates and state proofs. |
| **Mobile Node** | Same as Light Node. May cache recent finality certificates for offline verification. |

---

### 2.10 Patent Risk Assessment

The following elements of Synaptic Consensus are flagged for FTO (Freedom-to-Operate) analysis against the nChain patent portfolio (1,308 patents) and other holders:

#### 2.10.1 Elements Requiring FTO Analysis

| Element | Potential Patent Conflict | Risk | Mitigation |
|---------|--------------------------|------|------------|
| **DAG-based BFT ordering** | nChain holds patents on "consensus-based electronic ledgers" (U.S. Patent 12,032,677, under reexamination). DAG consensus is covered by academic papers (Narwhal, Bullshark, Mysticeti) with open implementations. | MEDIUM | SynBFT's anchor-based commit rule with AI reputation scoring is novel. Differentiate clearly from patented approaches. The reexamination of nChain's patent suggests it may not survive. Join COPA for defensive coverage. |
| **VRF-based leader election** | VRF leader election is used by Algorand (MIT-licensed), Cardano (Apache 2.0). The technique originates from Micali et al. academic research. | LOW | Well-established technique with open-source precedents. No known patent claims on VRF leader election per se. |
| **PoUW with AI inference** | AI + consensus mechanism patents are an emerging filing area (LEGAL_LANDSCAPE.md Section 3.2). nChain and IBM are filing in this area. Specific combination of PoUW + inference verification may be novel. | MEDIUM-HIGH | File provisional patent on the Synaptic Consensus PoUW design (multi-method verification, quality metrics, reputation integration) before publication. Publish defensive disclosure for elements not worth patenting. |
| **Aggregate BLS signatures for finality** | Used by Ethereum beacon chain (Apache 2.0 / GPLv3). The `blst` library is Apache 2.0. BLS aggregation is from academic research (Boneh-Lynn-Shacham, 2001). | LOW | Well-established cryptographic technique with open implementations. No known patent barrier. |
| **Stake-weighted selection with caps** | Various PoS mechanisms are patented or patent-pending. The specific cap + rotation + diversity quota combination may be novel. | LOW-MEDIUM | The individual elements (stake weighting, caps, rotation) are well-known. The specific combination should be documented in a defensive disclosure. |

#### 2.10.2 Recommended Patent Actions

1. **MANDATORY:** Conduct formal FTO analysis against nChain's consensus patent portfolio before finalizing SynBFT design. Focus on U.S. Patent 12,032,677 and related family members.
2. **RECOMMENDED:** File provisional patent on the "Synaptic Consensus" mechanism — specifically the combination of DAG-BFT with AI reputation-weighted anchor selection and multi-method PoUW verification. Filing is defensive (to prevent others from patenting it), not offensive (we will pledge to COPA).
3. **RECOMMENDED:** Publish defensive disclosures for: the equivocation detection mechanism, the censorship scoring system, and the PoUW quality metrics framework.
4. **RECOMMENDED:** Join COPA before any public disclosure of the consensus design.

*Ref: LEGAL_LANDSCAPE.md Section 3.2 (patent risks), Section 3.4 (COPA recommendation), Section 9.1 (critical risks #5)*

---

### 2.11 Formal Properties

For downstream skills (security-engineer in particular), the following properties are claimed for SynBFT and should be formally verified:

1. **Safety:** If two honest validators commit different values for the same slot, then more than f validators are Byzantine. (Standard BFT safety.)
2. **Liveness:** If fewer than f validators are Byzantine and the network is eventually synchronous (GST assumption), then every submitted transaction is eventually committed. (Standard BFT liveness under partial synchrony.)
3. **Finality:** A committed anchor's ordering is immutable — no future anchor can reorder transactions ordered by a past committed anchor.
4. **PoUW Isolation:** The liveness and safety of SynBFT are independent of the PoUW layer. If all PoUW validators go offline simultaneously, SynBFT continues operating normally (with PoUWReputation dropping to zero for all validators, which only affects the 15% PoUW component of anchor scores).
5. **Equivocation Accountability:** Any equivocating validator can be identified and its equivocation proven to all honest validators from on-chain evidence.

---

### 2.12 Parameter Summary

| Parameter | Default Value | Governance-Adjustable | Rationale |
|-----------|---------------|----------------------|-----------|
| Round time | 400ms | Yes (range: 200ms-2000ms) | Matches Sei's proven 400ms. Lower bound limited by network propagation; upper bound by UX expectations. |
| Anchor interval (k) | 4 rounds | Yes (range: 2-10) | 4 rounds = 1.6s between anchors. Lower k = faster finality but more anchor overhead. |
| Minimum validator set | 21 | No (hardcoded minimum) | Below 21, BFT provides insufficient fault tolerance. |
| Target validator set | 100-200 | Yes | Governance adjusts based on network capacity. |
| Maximum validator set | 400 | Yes (with supermajority) | Requires 2/3 governance approval to change. |
| Stake cap per validator | 5% | Yes (range: 2%-10%) | Lower cap = more decentralized, higher cap = more efficient. |
| Diversity quota | 30% | Yes (range: 10%-50%) | Percentage of slots reserved for non-top-20 validators. |
| Cooldown rotation | 10 epochs | Yes (range: 5-50) | Consecutive epochs before mandatory 1-epoch cooldown. |
| Epoch length | 1000 rounds (~400s) | Yes (range: 500-5000) | Period for validator set resampling and PoUW commitment. |
| Unbonding period | 21 days | Yes (range: 7-90 days) | Stake lock after validator exit. Must exceed any dispute resolution period. |
| PoUW reputation weight in anchor score | 15% | Yes (range: 0%-25%) | Cap on AI influence over consensus leader selection. |
| MPRE redundancy (m) | 3 | Yes (range: 2-7) | Number of redundant executors for multi-party verification. |
| MPRE agreement threshold (n-of-m) | 2-of-3 | Yes | Minimum agreement for MPRE acceptance. |

---

**Stack challenges raised:**
- STACK CHALLENGE: Signature scheme — BLS12-381 required alongside Ed25519 for aggregate finality proofs. See Section 2.9.3 for full justification.

**Conflicts raised:** None

**Sign-off:** consensus-engineer

## SECTION 3: TOKENOMICS

---
**Contributed by:** tokenomics-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete, especially Section 9.1 Terra/Luna lessons, Section 9.5 common failures, Section 17 economic model guidance), LEGAL_LANDSCAPE.md (complete, especially Section 4 regulatory environment, Howey Test, FSCA, MiCA), Section 0 stack rulings (reviewed), Section 2 consensus mechanism (reviewed -- SynBFT + PoUW, VRF-weighted selection 70/15/15, 5% stake cap, 30% diversity quota, 21-day unbonding, 100-200 validators), Section 4 node architecture (reviewed -- PoUW tiers with 1.2-1.5x/2-3x/4-8x earnings premiums), Section 5 security model (reviewed -- economic attack vectors E1-E6, flash loan governance, MEV, death spiral), Section 6 AI integration (reviewed -- AI compute marketplace, EIP-1559 inference fee market, 3-tier PoUW), Section 7 smart contract layer (reviewed -- fuel-based gas metering, EIP-1559 base fee burn + tips, AI inference gas pricing)

---

### TOKENOMICS MODEL

```
- Coin name candidates:    [10 candidates listed in Section 3.10 - PENDING LEGAL CLEARANCE]
- Total supply:            1,000,000,000 (1 billion) hard cap with disinflationary emission
- Initial distribution:    See Section 3.2 for full breakdown
- Emission schedule:       10-year disinflationary curve, halving every 2 years
- Staking rewards:         3-12% APY (dynamically adjusted by staking ratio)
- Transaction fees:        EIP-1559 dynamic base fee (burned) + priority tips (to validators)
- Burn mechanism:          Base fee burn + inference protocol fee burn + governance burn
- Validator incentives:    Consensus rewards + PoUW earnings + priority tips
- Inflation target:        Year 1: ~5.0%, declining to <1% by year 8, deflationary at maturity
- Utility hooks:           Gas payment, staking, governance, AI inference payment, model registration deposit, relay bonding, agent spending limits
- Securities analysis:     See Section 3.9 for comprehensive Howey Test assessment
```

---

### 3.1 Token Fundamentals

#### 3.1.1 Supply Model: Hard Cap with Disinflationary Emission

**Total Supply: 1,000,000,000 tokens (1 billion). Hard cap. No governance override.**

The hard cap is a non-negotiable design choice for the following reasons:

1. **Terra/Luna lesson (RESEARCH_BRIEF.md Section 9.1):** Unbounded minting mechanisms create death spiral risk. Terra's collapse was caused by reflexive unlimited minting. Aztibase Network's supply cap eliminates this attack vector entirely. There is no mechanism in the protocol that can create tokens beyond the hard cap, regardless of market conditions.

2. **Securities law clarity:** A fixed supply with transparent emission avoids the "expectation of profit from efforts of others" prong of the Howey Test more effectively than an uncapped token where a team controls inflation. The emission schedule is algorithmic and immutable -- no team discretion.

3. **Economic predictability:** Validators, delegators, and ecosystem participants can model their expected returns against a known supply schedule. This reduces speculative behavior and encourages utility-focused participation.

4. **Deflationary path:** Combined with the EIP-1559 base fee burn (Section 3.6), a fixed cap means the effective circulating supply can decrease over time as the chain matures and burns exceed emission. This creates a sustainable long-term value model without requiring perpetual growth.

**Why not uncapped with target inflation?** Ethereum's uncapped model works because ETH had 8 years of ecosystem lock-in before the merge made it deflationary. A new chain cannot rely on that momentum. A hard cap provides a stronger credible commitment to token holders and is simpler to analyze for regulatory compliance.

#### 3.1.2 Denomination

The native token has 18 decimal places (consistent with Ethereum's ETH and most ERC-20 tokens, ensuring ecosystem tooling compatibility):

```
1 token          = 1,000,000,000,000,000,000 base units (10^18)
Smallest unit    = 1 base unit (10^-18 tokens)
```

The smallest unit name will be determined alongside the coin name by naming-council (Phase 5c). For this document, we use "base units" and "tokens" as placeholders.

#### 3.1.3 Precision Requirements

> **STACK BASELINE REVIEW [tokenomics-engineer]: Numeric Precision**
>
> All token economic calculations (staking rewards, fee distribution, burn amounts, delegation rewards) require precise fixed-point arithmetic. Floating-point arithmetic is unacceptable for financial calculations due to non-determinism across platforms and accumulating rounding errors.
>
> **Requirements:**
> - All balance and fee calculations MUST use unsigned 128-bit integers (u128) representing base units (10^18).
> - Reward calculations that involve fractional multiplication (e.g., APY computation) MUST use fixed-point arithmetic with at least 27 decimal places of precision internally, truncated to 18 decimal places for storage. Rust's `u256` (via the `uint` crate, MIT/Apache 2.0) provides sufficient headroom for intermediate calculations.
> - Division operations MUST truncate (floor), not round, to ensure determinism and prevent rounding-based inflation.
> - Reward distribution MUST be computed per-epoch and accumulated, not computed retroactively over long periods (to prevent precision loss).
>
> **Assessment:** Rust's native u128 is sufficient for balances up to ~3.4 x 10^38 base units (~340 billion tokens at 18 decimals). This is 340x the total supply -- adequate. For intermediate calculations involving multiplication of two u128 values, u256 is required to avoid overflow. The `uint` crate (MIT/Apache 2.0, used by `revm` already in our dependency tree) provides u256. **No stack challenge raised -- Rust's numeric types plus `uint` are sufficient.**

---

### 3.2 Initial Distribution

#### 3.2.1 Genesis Allocation

The genesis block mints 400,000,000 tokens (40% of total supply). The remaining 600,000,000 tokens (60%) are emitted through the emission schedule (Section 3.3) to validators and the protocol over 10 years.

| Allocation | Tokens | % of Total | % of Genesis | Vesting | Rationale |
|------------|--------|-----------|-------------|---------|-----------|
| **Protocol Treasury** | 100,000,000 | 10.0% | 25.0% | None (governance-controlled spending) | Funds ecosystem development, grants, security audits, bug bounties. Governance-controlled from day one. |
| **Ecosystem Development Fund** | 80,000,000 | 8.0% | 20.0% | 4-year linear unlock with 6-month cliff | Developer grants, hackathons, integration bounties, bridge incentives, DeFi seeding. Cliff prevents immediate dumping. |
| **Core Team & Contributors** | 60,000,000 | 6.0% | 15.0% | 4-year linear vesting with 12-month cliff | Compensates founding engineers, researchers, and contributors. 12-month cliff aligns incentives with long-term success. No tokens available at launch. |
| **Foundation Reserve** | 40,000,000 | 4.0% | 10.0% | 2-year lock, then 3-year linear unlock | Long-term strategic reserve for partnerships, exchange listings, legal defense fund, emergency protocol upgrades. |
| **Community Airdrop** | 40,000,000 | 4.0% | 10.0% | Immediate (at genesis) with 50% time-locked 90 days | Distributed to early testnet participants, community contributors, node operators during testnet phase. Encourages genuine participation, not speculation. |
| **Validator Bootstrap Pool** | 40,000,000 | 4.0% | 10.0% | 6-month linear unlock with staking requirement | Bootstraps the initial validator set. Recipients must stake tokens to validate -- cannot sell without unstaking (21-day unbonding). Ensures minimum security at launch. |
| **AI Ecosystem Fund** | 20,000,000 | 2.0% | 5.0% | 3-year linear unlock with 6-month cliff | Dedicated to AI model creators, PoUW incentive bootstrapping, AI researcher grants, and compute marketplace seeding. Separate from general ecosystem fund to ensure AI development is not deprioritized. |
| **Liquidity Provision** | 20,000,000 | 2.0% | 5.0% | Immediate (at genesis) | Provides initial DEX liquidity and exchange market-making. Necessary for price discovery and utility from day one. |

**Total Genesis Mint: 400,000,000 tokens (40%)**

#### 3.2.2 Circulating Supply at Launch

At genesis, the immediately liquid supply is:

```
Community Airdrop (50% immediate):  20,000,000
Liquidity Provision:                20,000,000
----------------------------------------------
Day-1 Circulating:                  40,000,000 (4% of total supply)
```

This low initial circulating supply (4%) is deliberate:
- Prevents large-scale selling pressure at launch
- Most tokens are locked or vesting, aligning holder incentives with network growth
- Validator Bootstrap Pool requires staking (further reducing sell pressure)
- Comparable to successful launches: Sui launched with ~5% circulating, Aptos with ~13%

#### 3.2.3 Anti-Concentration Rules

To prevent plutocratic capture (aligned with Section 2.4.3 anti-centralization mechanisms):

1. **No single entity or affiliated group receives more than 6% of total supply** in the genesis allocation.
2. **Core Team allocation is distributed across individual contributors**, not held by a single corporate entity.
3. **Foundation Reserve requires multi-sig governance** (5-of-9 signers from geographically distributed, independent parties) for any disbursement.
4. **Community Airdrop uses a quadratic distribution** -- diminishing returns for larger participants to favor broad distribution over whale accumulation.

---

### 3.3 Emission Schedule

#### 3.3.1 Disinflationary Curve

The remaining 600,000,000 tokens (60% of total supply) are emitted over approximately 10 years through a halving schedule:

| Year | Annual Emission | Cumulative Emitted | Total in Existence | Inflation Rate (of existing supply) |
|------|----------------|-------------------|-------------------|-------------------------------------|
| 1 | 120,000,000 | 120,000,000 | 520,000,000 | ~30.0% of 400M genesis (high early incentive) |
| 2 | 120,000,000 | 240,000,000 | 640,000,000 | ~23.1% |
| 3 | 60,000,000 | 300,000,000 | 700,000,000 | ~9.4% |
| 4 | 60,000,000 | 360,000,000 | 760,000,000 | ~8.6% |
| 5 | 30,000,000 | 390,000,000 | 790,000,000 | ~3.9% |
| 6 | 30,000,000 | 420,000,000 | 820,000,000 | ~3.8% |
| 7 | 15,000,000 | 435,000,000 | 835,000,000 | ~1.8% |
| 8 | 15,000,000 | 450,000,000 | 850,000,000 | ~1.8% |
| 9 | 7,500,000 | 457,500,000 | 857,500,000 | ~0.9% |
| 10 | 7,500,000 | 465,000,000 | 865,000,000 | ~0.9% |

**Remaining after year 10:** 135,000,000 tokens unissued. These continue at a tail emission of 3,750,000/year (halving again) until the cap is reached (approximately year 46 at diminishing rates). This tail emission ensures validators always receive some block reward even as transaction fees become the primary revenue source.

**Why halving, not smooth decay?** Halving is the most widely understood emission model in crypto (Bitcoin's 4-year halving is universally known). A 2-year halving provides faster initial bootstrapping than Bitcoin's 4-year cycle, appropriate for a chain that needs to reach critical mass quickly. The predictability of halving events enables validators to plan their operations.

#### 3.3.2 Emission Distribution

Each epoch's emission is distributed as follows:

```
Epoch Emission Distribution:
  70% --> Validator Rewards (consensus participation)
  15% --> PoUW Supplemental Pool (AI compute incentives)
  10% --> Protocol Treasury (ongoing development funding)
   5% --> Staking Insurance Fund (covers slashing socialization)
```

**Validator Rewards (70%):** Distributed pro-rata to active validators based on their consensus participation in the epoch. A validator that proposed vertices in every round receives full share; one that missed 20% of rounds receives 80% share. This directly rewards reliability.

**PoUW Supplemental Pool (15%):** Distributed to validators who opted into PoUW and delivered verified AI inference during the epoch. Distribution is weighted by the validator's PoUW quality score (Section 2.3.4: 40% correctness, 25% availability, 20% latency, 15% diversity). This pool exists even if no inference requests are submitted -- it incentivizes validators to maintain AI compute readiness. Undistributed PoUW pool tokens (if no PoUW validators exist) are redirected to the Protocol Treasury.

**Protocol Treasury (10%):** Accumulates continuously to fund ongoing development. Governance controls spending (Section 3.7).

**Staking Insurance Fund (5%):** Accumulates to cover slashing socialization events. If a validator is slashed and delegators suffer losses, this fund provides partial compensation. If the fund exceeds a governance-set cap (default: 2% of total supply), excess redirects to the Protocol Treasury.

---

### 3.4 Staking Economics

#### 3.4.1 Staking Reward Model

Staking rewards come from two sources:
1. **Block emission rewards** (from the 70% validator allocation in Section 3.3.2)
2. **Transaction priority fees** (tips from users, per Section 7.7.1)

The effective APY depends on the total staking ratio (fraction of circulating supply staked):

```
Target staking ratio: 50% of circulating supply

APY Adjustment Curve:
  if staking_ratio < 20%:  APY = 12%   (maximum, emergency incentive to attract stakers)
  if staking_ratio = 30%:  APY = 10%
  if staking_ratio = 40%:  APY = 8%
  if staking_ratio = 50%:  APY = 6%    (target equilibrium)
  if staking_ratio = 60%:  APY = 4%
  if staking_ratio = 70%:  APY = 3%    (minimum, discourages over-staking that reduces liquidity)
  if staking_ratio > 80%:  APY = 3%    (floor)

The curve is implemented as a piecewise linear function:
  base_apy = max(3%, min(12%, 18% - 0.2 * staking_ratio_percent))
```

**Why this range:**
- **12% maximum** is competitive with established PoS chains (Cosmos: 14-20%, Polkadot: 14-18%, Ethereum: 3-5%) and sufficient to bootstrap staking. It is NOT an unsustainable yield promise (Terra's 20% Anchor yield required $6M/day subsidy -- RESEARCH_BRIEF.md Section 9.1). Our 12% max is funded entirely by emission, not by recursive mechanisms.
- **3% minimum** ensures validators remain economically viable even when the chain is heavily staked. Below 3%, the opportunity cost of locking capital exceeds the reward, causing unstaking cascades.
- **50% target** balances security (more stake = more attack cost) with liquidity (too much staked = illiquid market, poor DeFi ecosystem).

#### 3.4.2 Minimum Stake

**The minimum stake to become a validator is governance-adjustable, not hardcoded.**

- **Initial setting:** 50,000 tokens
- **Governance range:** 10,000 - 500,000 tokens

**Rationale:**
- 50,000 tokens at launch represents 0.005% of total supply -- accessible to enthusiast validators while providing meaningful economic commitment.
- Berachain's 250,000 BERA barrier (RESEARCH_BRIEF.md Section 1.6) created extreme centralization. We avoid this by starting low and letting governance adjust based on security needs.
- The 5% stake cap (Section 2.4.2) means even well-funded validators cannot dominate.
- As the token price appreciates, governance can lower the minimum to maintain accessibility.

#### 3.4.3 Delegation Mechanism

Delegation allows token holders who cannot or choose not to operate validator infrastructure to participate in staking and earn rewards.

**Delegation rules:**
1. Any token holder can delegate to any registered validator candidate.
2. Delegated stake counts toward the validator's total stake for selection probability.
3. **Delegated stake counts toward the 5% cap** -- preventing Lido-style concentration where one delegation protocol controls >30% of stake (Section 2.4.3).
4. Validators set a commission rate (0-20%, governance-capped maximum) deducted from delegator rewards.
5. Delegators share in slashing proportionally to their delegation.
6. Delegation and undelegation follow the same 21-day unbonding period as direct staking.

**Liquid staking consideration:**

Liquid staking protocols (minting a derivative token representing staked assets) will inevitably emerge. The protocol does not natively implement liquid staking but is designed to be compatible with it. The key constraint: **any liquid staking protocol's delegated stake counts toward the 5% cap per validator it delegates to**. If a liquid staking protocol delegates to 20 validators evenly, each receives 1/20th of the protocol's total stake -- all subject to individual caps.

To further prevent liquid staking concentration risk:
- Governance can set a **maximum delegation from a single address** to a single validator (default: 2% of total supply).
- If a liquid staking protocol exceeds 25% of total stake, governance may activate an **overcrowding penalty** that progressively reduces rewards for the largest delegation sources.

#### 3.4.4 Slashing Conditions

Slashing aligns economic incentives with honest validator behavior. Conditions and amounts are calibrated to be punitive enough to deter misbehavior but not so severe that honest operational failures (hardware crash, network partition) cause devastating losses.

| Offense | Slash Amount | Evidence | Recovery |
|---------|-------------|---------|----------|
| **Equivocation (dual vertex, Section 2.7.5)** | 5% of validator's total stake | Two signed vertices for same round from same key | None. Permanent ejection from current epoch. May re-register next epoch with remaining stake. |
| **Prolonged downtime (>50% of rounds missed in an epoch)** | 0.1% of stake per epoch of violation | On-chain vertex participation records | Automatic after returning to >90% participation for 3 consecutive epochs. |
| **PoUW failure (committed but failed to deliver, Section 2.3.5)** | 1% of PoUW-locked stake per failed request (capped at 10% per epoch) | On-chain InferenceRequest assignment vs. attestation records | Automatic after 3 epochs of >90% PoUW delivery rate. |
| **PoUW fraud (incorrect inference results, Section 2.7.6)** | 10% of total stake | MPRE disagreement, TEE attestation failure, or ZK proof invalidation | None. Permanent PoUW ban for current validator identity. May re-register with new commitment. |
| **Censorship (persistent, Section 2.7.5)** | 2% of stake, escalating 1% per additional epoch | Censorship score exceeding threshold for 3+ consecutive epochs | Automatic after censorship score normalizes for 5 epochs. |
| **Governance manipulation (flash loan attack, Section 5.1.6 E1)** | 100% of stake used in the manipulative vote | On-chain evidence of governance vote + same-block unstaking or loan repayment | None. Permanent ban. |

**Slashing socialization:** When a validator is slashed, delegators lose proportionally. The Staking Insurance Fund (5% of emission, Section 3.3.2) provides partial compensation to delegators affected by slashing. Compensation is capped at 50% of delegator losses and requires a governance claim process (to prevent moral hazard).

**Anti-flash-loan governance protection (responding to SECURITY FLAG S3-1):**
- Governance voting requires tokens to be **staked for a minimum of 1 full epoch** before becoming vote-eligible.
- Governance proposals have a **7-day voting period** followed by a **2-day execution delay** (timelock).
- Voting power is based on **time-weighted staking** (tokens staked for longer have linearly increasing weight, capped at 2x after 90 days).
- These mechanisms make flash loan governance attacks economically prohibitive: an attacker would need to stake tokens for at least 1 epoch (~400 seconds minimum, but governance voting epochs are 7 days) and cannot unstake during the voting period.

#### 3.4.5 Unbonding Period

**21 days** (governance-adjustable, range: 7-90 days per Section 2.12).

This aligns with the consensus-engineer's design and provides:
- Sufficient time for retroactive slashing of misbehavior discovered after the fact
- Protection against long-range attacks (Section 2.7.3)
- Parity with Cosmos ecosystem (21 days) and near Ethereum's ~27-day withdrawal queue
- Not so long that it discourages participation (Polkadot's 28-day unbonding is considered a friction point)

---

### 3.5 Fee Model

#### 3.5.1 Transaction Fee Structure

Aztibase Network uses the fuel-based EIP-1559 model defined in Section 7.1.3 and Section 7.7:

```
Transaction Fee = fuel_consumed * (base_fee_per_fuel + priority_fee_per_fuel)

Where:
  base_fee_per_fuel:     Algorithmically determined, burned (deflationary)
  priority_fee_per_fuel: User-specified tip, paid to the vertex proposer (validator)
  fuel_consumed:         Actual fuel used (unused fuel refunded)
```

**Base fee adjustment:**
- Adjusts every anchor commit (~1.6 seconds) based on block utilization
- If blocks are >50% full (by fuel): base fee increases up to 12.5%
- If blocks are <50% full: base fee decreases up to 12.5%
- This creates a self-regulating market that maintains 50% block utilization as equilibrium

**Initial base fee:** Set at genesis to target a transaction cost of approximately $0.001 USD equivalent for a simple token transfer (21,000 fuel units). The exact initial value is calibrated at launch based on initial token price discovery.

#### 3.5.2 Fee Revenue Distribution

```
Standard Transaction Fees:
  Base fee portion  --> 100% BURNED (permanent supply reduction)
  Priority fee      --> 100% to vertex proposer (the validator who included the transaction)

AI Inference Fees (from PoUW marketplace):
  85% --> PoUW validator who performed the inference
  10% --> Protocol Treasury
   5% --> Model creator (royalty, per ModelRegistry licensing terms, Section 6.9.1)

  Of the 85% to PoUW validator:
    The validator receives the full 85% as direct income.
    This is SEPARATE from consensus emission rewards.

  Of the 10% to Protocol Treasury:
    50% is burned (5% of total inference fee)
    50% enters the treasury (5% of total inference fee)
```

**Net effect on inference fees:** 5% burned, 5% to treasury, 5% to model creator, 85% to compute provider. This split incentivizes: compute provision (85% is highly competitive with centralized cloud pricing), model creation (5% royalty creates a market for model quality), protocol sustainability (5% treasury), and deflationary pressure (5% burn).

#### 3.5.3 Fee Dynamics: High vs. Low Demand

**High demand (bull market / high activity):**
- Base fee rises automatically via EIP-1559 mechanism
- More fuel consumed = more base fee burned = higher deflationary pressure
- Priority fees increase as users compete for inclusion = higher validator income
- AI inference fees rise as PoUW capacity becomes congested (EIP-1559-style for inference, Section 6.6.2)
- Net effect: strong burn, high validator revenue, self-regulating congestion

**Low demand (bear market / low activity):**
- Base fee decreases to minimum (governance-set floor, default: 1 base unit per fuel)
- Minimal burn occurs
- Emission rewards become the primary validator income source
- AI inference fees decrease, making the compute marketplace more competitive vs. centralized alternatives
- Net effect: emission slightly exceeds burn, maintaining validator incentives during downturns

**Equilibrium point:** The chain reaches burn-emission equilibrium when:
```
Annual base fee burn + inference fee burn >= Annual emission

At 50% block utilization (target) with 10,000 TPS average:
  ~315 billion transactions/year
  At 21,000 avg fuel per tx and base fee = 100 base units/fuel:
  Annual burn = ~661 trillion base units = ~661,000 tokens/year

  Year 5 emission = 30,000,000 tokens/year
  Required avg base fee for equilibrium: ~4,540x the initial base fee

  This implies equilibrium requires substantial chain utilization.
  Realistically, burn-emission equilibrium is expected in years 5-8
  depending on adoption trajectory. See Section 3.8 for scenario modeling.
```

---

### 3.6 PoUW Incentives

#### 3.6.1 PoUW Earnings vs. Consensus-Only Earnings

Validators who opt into PoUW earn from three streams:

```
Consensus-Only Validator Income:
  1. Emission rewards (share of 70% validator allocation)
  2. Priority fees (tips from included transactions)

PoUW Validator Additional Income:
  3. PoUW emission pool (share of 15% PoUW allocation, quality-weighted)
  4. Inference fees (85% of each inference fee paid by requesters)
```

Expected earnings multipliers by tier (relative to consensus-only):

| PoUW Tier | Additional Emission | Inference Fee Income | Total Expected Multiple | Hardware Cost |
|-----------|--------------------|--------------------|----------------------|---------------|
| **None (consensus only)** | 0% | $0 | 1.0x (baseline) | ~$50/month (8GB RAM, SSD, broadband) |
| **Tier 1 (CPU)** | +15-30% from PoUW pool | Low-moderate (small model inference) | 1.2-1.5x | ~$100/month (+16 cores, 32GB RAM) |
| **Tier 2 (Consumer GPU)** | +30-60% from PoUW pool | Moderate-high (medium model inference) | 2.0-3.0x | ~$300/month (+RTX 3060/4070) |
| **Tier 3 (Professional GPU)** | +60-120% from PoUW pool | High (large model inference) | 4.0-8.0x | ~$800-2000/month (+A10/A100) |

These multiples align with Section 4.5.4's expected earnings premiums and are designed so that each tier's incremental hardware cost is justified by the incremental earnings at moderate utilization (>30% of PoUW capacity utilized).

#### 3.6.2 Incentive Alignment: Preventing Gaming

**Problem 1: Validators doing only PoUW, neglecting consensus.**

Solution: PoUW earnings are CONDITIONAL on consensus participation. A validator must maintain >90% consensus participation rate (vertex proposals, parent references) to be eligible for PoUW rewards. If consensus participation drops below 90%, PoUW rewards are reduced linearly to zero at 70% participation. Below 70%, the validator is ejected from the active set entirely (Section 2.4.2).

**Problem 2: Validators doing only consensus, ignoring PoUW commitments.**

Solution: The `AIComputeCommitment` (Section 2.3.5) is a binding promise. Validators who commit to PoUW but fail to deliver are slashed (Section 3.4.4: 1% of PoUW-locked stake per failed request). Validators who do NOT commit to PoUW face no penalty -- PoUW is genuinely optional.

**Problem 3: Validators cherry-picking only easy/profitable inference requests.**

Solution: The diversity metric in PoUW quality scoring (15% weight, Section 2.3.4) penalizes validators who only serve cheap models. Validators that serve a diverse range of model types and sizes earn higher PoUW reputation, which increases their PoUW pool share AND their anchor selection probability (15% of AnchorScore). This creates a positive feedback loop: diverse service -> better reputation -> more consensus influence -> more rewards.

**Problem 4: PoUW validators extracting MEV from inference requests.**

Solution: The PoUW scheduler assigns inference requests deterministically from the VRF seed (Section 2.3.3). Validators cannot choose which requests to serve. The assignment is auditable on-chain. A validator that refuses or reorders assignments accumulates PoUW failure records and is slashed.

#### 3.6.3 AI Compute Market Revenue Model

The AI compute marketplace generates protocol revenue through inference fees:

```
Revenue flow per inference request:
  User pays:           compute_units * (base_inference_fee + priority_fee)

  Distribution:
    85% --> PoUW validator (compute provider)
    10% --> Protocol (5% burned, 5% treasury)
     5% --> Model creator (royalty)

  Protocol revenue = 10% of all inference fees
  Protocol burn = 5% of all inference fees (additional deflationary pressure)
```

**Projected revenue at scale (assumptions: 1,000 inference requests/second avg, $0.01 avg fee per request):**
- Annual inference fee volume: ~$315 million
- Protocol revenue (10%): ~$31.5 million/year
- Protocol burn (5%): ~$15.75 million/year in token burns
- PoUW validator earnings (85%): ~$268 million/year distributed across PoUW validators

These projections are for the mature state (year 3-5+). Early years will have significantly lower inference volume. See Section 3.8 for scenario modeling.

---

### 3.7 Burn Mechanisms

#### 3.7.1 Base Fee Burn (Primary)

All transaction base fees are permanently burned (per EIP-1559 model, Section 7.7.1):

```
base_fee_burn_per_tx = fuel_consumed * base_fee_per_fuel
```

This is the largest burn mechanism and scales directly with chain utilization. Higher adoption = more transactions = more burn = more deflationary pressure.

#### 3.7.2 Inference Fee Burn

5% of all AI inference fees are burned (Section 3.5.2). This links the AI compute economy directly to token scarcity -- as the AI marketplace grows, so does deflationary pressure.

#### 3.7.3 State Rent / Storage Deposit Burn

A portion of the new storage slot surcharge (20,000 fuel, Section 7.9) is burned rather than refundable. Specifically:

```
New state slot creation: 20,000 fuel surcharge
  - 50% (10,000 fuel equivalent) is burned permanently
  - 50% (10,000 fuel equivalent) is refundable upon state deletion

This incentivizes state-efficient contracts AND creates burn proportional to state growth.
```

#### 3.7.4 Governance Burn

The Protocol Treasury can execute token burns via governance vote. This provides a deflationary lever if the treasury accumulates excess funds beyond operational needs. The governance burn requires a supermajority vote (>67% of participating voters).

#### 3.7.5 Burn Rate Modeling

```
Burn Rate Scenarios (annual):

Conservative (Year 1, 500 TPS avg):
  Base fee burn:      ~50,000 tokens/year
  Inference burn:     ~5,000 tokens/year
  Storage burn:       ~10,000 tokens/year
  Total burn:         ~65,000 tokens/year
  Emission:           120,000,000 tokens/year
  Net inflation:      ~119,935,000 tokens (overwhelmingly inflationary)

Moderate (Year 3, 3,000 TPS avg, 100 inf/sec):
  Base fee burn:      ~1,500,000 tokens/year
  Inference burn:     ~500,000 tokens/year
  Storage burn:       ~200,000 tokens/year
  Total burn:         ~2,200,000 tokens/year
  Emission:           60,000,000 tokens/year
  Net inflation:      ~57,800,000 tokens (still inflationary, burn rising)

Aggressive (Year 5, 8,000 TPS avg, 500 inf/sec):
  Base fee burn:      ~15,000,000 tokens/year
  Inference burn:     ~5,000,000 tokens/year
  Storage burn:       ~2,000,000 tokens/year
  Total burn:         ~22,000,000 tokens/year
  Emission:           30,000,000 tokens/year
  Net inflation:      ~8,000,000 tokens (approaching equilibrium)

Mature (Year 7+, 10,000+ TPS avg, 1,000+ inf/sec):
  Base fee burn:      ~40,000,000 tokens/year
  Inference burn:     ~15,000,000 tokens/year
  Storage burn:       ~5,000,000 tokens/year
  Total burn:         ~60,000,000 tokens/year
  Emission:           15,000,000 tokens/year
  NET DEFLATIONARY:   ~-45,000,000 tokens/year (supply decreasing)
```

**Path to deflationary equilibrium:** Under the moderate scenario, burn exceeds emission around year 6-7. Under the aggressive scenario, it happens in year 5-6. Under the conservative scenario, the chain remains inflationary through year 10 but with declining inflation rate. The hard cap ensures that even in the worst case, total supply never exceeds 1 billion.

---

### 3.8 Protocol Treasury

#### 3.8.1 Treasury Funding Sources

```
Treasury Income Streams:
  1. Genesis allocation:           100,000,000 tokens (10% of total)
  2. Ongoing emission:             10% of each epoch's emission (Section 3.3.2)
  3. Inference protocol fee:       5% of all AI inference fees (Section 3.5.2)
  4. Slashing proceeds:            50% of slashed tokens (remaining 50% burned)
  5. Model registration deposits:  Retained until model removal
  6. Expired/unclaimed rewards:    Validator rewards unclaimed for >365 days
```

#### 3.8.2 Treasury Governance

Treasury spending requires on-chain governance approval:

```
Spending Tiers:
  Micro grants (<10,000 tokens):     Simple majority (>50%), 3-day voting, 1-day timelock
  Standard grants (10K-1M tokens):   Simple majority (>50%), 7-day voting, 2-day timelock
  Large grants (1M-10M tokens):      Supermajority (>67%), 14-day voting, 7-day timelock
  Strategic (>10M tokens):           Supermajority (>75%), 21-day voting, 14-day timelock

All treasury proposals require a deposit of 1,000 tokens (refunded if proposal reaches quorum, burned if not).
Quorum: 10% of staked tokens must vote for result to be valid.
```

#### 3.8.3 Treasury Allocation Categories

```
Recommended allocation ranges (governance-adjustable):
  Engineering & Development:    30-50% (protocol upgrades, client development, tooling)
  Security:                     15-25% (audits, bug bounties, formal verification)
  Ecosystem Grants:             15-25% (dApp developers, integration partners)
  AI Ecosystem:                 10-20% (model creators, AI researcher grants, compute subsidies)
  Operations:                    5-10% (legal, regulatory compliance, exchange relations)
  Reserve:                       5-15% (emergency fund, held unallocated)
```

---

### 3.9 Securities Law Analysis and Economic Sustainability

#### 3.9.1 Howey Test Assessment

The Howey Test (LEGAL_LANDSCAPE.md Section 4.1) evaluates whether a token is a security based on four prongs:

| Prong | Assessment | Analysis |
|-------|-----------|---------|
| **1. Investment of money** | LIKELY MET | Any purchase of tokens with fiat or crypto constitutes an investment of money. This prong is almost always met. |
| **2. Common enterprise** | PARTIALLY MET | Token holders' fortunes are linked to the network's success (horizontal commonality). However, there is no "pooling of funds" managed by a common promoter -- validators independently earn rewards from protocol-level mechanisms. |
| **3. Expectation of profit** | RISK AREA | Token holders may expect price appreciation. However, the token has genuine utility (gas payment, staking, AI inference, governance) that provides non-speculative reasons to hold. The EIP-1559 burn mechanism means holding for "profit" depends on network usage, not team efforts. |
| **4. Efforts of others** | WEAKEST PRONG FOR CLASSIFICATION | Once the network is live and decentralized, profits (if any) derive from the decentralized network's operation, not from a central team. The 4-year team vesting with 12-month cliff, governance-controlled treasury, and 5% stake cap all demonstrate decentralization. |

**Overall assessment: MEDIUM RISK during initial distribution, LOW RISK post-decentralization.**

The token may be classified as a security during the pre-launch and early post-launch period (while the team retains significant influence). After sufficient decentralization (target: 12-18 months post-launch), the token should transition to a non-security digital commodity under current SEC guidance (LEGAL_LANDSCAPE.md Section 4.1: "investment contracts can end when promises are fulfilled").

#### 3.9.2 Design Choices That Mitigate Securities Classification

1. **Utility-first token:** The token is required for gas, staking, governance, and AI inference. It has clear functional utility beyond speculation.
2. **No ICO or public token sale:** The genesis distribution uses airdrops (community), validator bootstrapping (service-based), and locked allocations. No "investment round" where tokens are sold for fundraising.
3. **Decentralized from launch:** 100-200 validators, 5% stake cap, 30% diversity quota, and governance-controlled treasury mean no single entity controls the network.
4. **Algorithmic emission:** Token issuance follows an immutable algorithmic schedule. No team discretion over minting.
5. **Burns reduce supply:** The EIP-1559 burn means the token's scarcity is driven by network usage (demand-side), not team marketing (supply-side manipulation).
6. **Team tokens locked 12 months:** Team members cannot sell tokens for 12 months, reducing "efforts of others" narrative.

#### 3.9.3 FSCA (South Africa) Compliance

Per LEGAL_LANDSCAPE.md Section 4.3:

- Crypto assets are classified as **financial products** under FAIS since October 2022.
- If the Aztibase Network entity is SA-based, CASP licensing is mandatory.
- The token economic model must be disclosed in the CASP license application business plan.

**Tokenomics-specific FSCA considerations:**
1. The staking mechanism may be classified as a "financial service" (advisory or intermediary). Delegation specifically creates an intermediary relationship. Legal counsel must determine if the protocol developer entity requires CASP licensing for staking infrastructure.
2. The Travel Rule (FIC Directive 9) applies to transfers >ZAR 25,000. This is a node-level / exchange-level compliance requirement, not a tokenomics design issue, but the token's denomination and transfer mechanics must support Travel Rule metadata attachment.
3. The governance mechanism (token-weighted voting) may trigger FAIS requirements if governance votes affect "financial products." Engage SA legal counsel specifically on this point.

#### 3.9.4 EU MiCA Classification

Under MiCA (LEGAL_LANDSCAPE.md Section 4.2):

- The native token is likely classified as a **utility token** (provides access to a service -- gas, compute, governance).
- Utility tokens are subject to lighter MiCA requirements than ARTs (Asset-Referenced Tokens) or EMTs (E-Money Tokens).
- A **crypto-asset white paper** must be published before any token offering in the EU.
- The white paper must include: description of the token, rights and obligations, underlying technology, risks, and the emission/distribution schedule.

**Key MiCA risk:** If the token is offered to EU persons before the white paper is published and the CASP authorization is obtained, fines up to EUR 500,000 per violation apply. **Recommendation: Prepare MiCA-compliant white paper in parallel with mainnet development. Target completion 3 months before any EU-facing launch.**

#### 3.9.5 Regulatory Risk Mitigation

```
Risk Mitigation Matrix:

1. US Securities Risk:
   - No token sale to US persons during initial distribution
   - Fair launch model (airdrop + staking bootstrap, no ICO)
   - Engage US securities counsel before any token generation event
   - Monitor CLARITY Act progress -- may provide safe harbor

2. EU MiCA Risk:
   - Prepare compliant white paper
   - Establish EU legal entity in crypto-friendly jurisdiction
   - Obtain CASP authorization before July 1, 2026 deadline
   - Budget EUR 150,000 for CASP capital requirements

3. SA FSCA Risk:
   - Determine whether protocol developer is a CASP
   - If yes, apply for FSP license immediately
   - Ensure key personnel pass regulatory examinations
   - Implement Travel Rule compliance

4. General:
   - Token utility must be functional at launch (not just a promise)
   - Avoid marketing language that implies investment returns
   - Decentralize governance as rapidly as possible
   - Maintain detailed legal compliance documentation
```

#### 3.9.6 Economic Sustainability Scenarios

**Scenario A: Bull Market (High Adoption)**

```
Assumptions:
  - Token price: 10x from launch within 2 years
  - TPS: 5,000 avg by year 2, 10,000+ by year 3
  - AI inference: 500+ requests/sec by year 2
  - Staking ratio: 40-50%
  - Validator count: 150-200

Outcomes:
  - Validator revenue is high (emission + fees + PoUW)
  - Strong deflationary pressure from burns (may approach burn > emission by year 3)
  - Risk: speculative staking driving ratio above 70% (reducing liquidity)
  - Mitigation: APY curve reduces rewards at high staking ratios (Section 3.4.1)
  - Risk: excessive MEV extraction as DeFi activity surges
  - Mitigation: DAG parallelism reduces MEV opportunity + future encrypted mempool (SECURITY FLAG S1-1)
  - Overall: economically healthy, self-sustaining
```

**Scenario B: Bear Market (Low Activity)**

```
Assumptions:
  - Token price: -80% from peak
  - TPS: 200-500 avg
  - AI inference: 10-50 requests/sec
  - Staking ratio: 25-35% (some validators exit due to unprofitability)
  - Validator count: 50-80

Outcomes:
  - Fee revenue is minimal; emission rewards are primary validator income
  - APY curve adjusts upward (low staking ratio -> higher APY, up to 12%)
  - Burns are negligible; chain is net inflationary
  - Risk: validator count drops toward minimum (21)
  - Mitigation: Validator Bootstrap Pool tokens remain staked (6-month unlock)
  - Mitigation: PoUW remains attractive if AI inference demand persists (decoupled from token speculation)
  - Risk: treasury runs low if token-denominated
  - Mitigation: treasury holds diversified assets (governance can convert to stablecoins)
  - Overall: survivable for 5+ years on emission alone. Not dependent on fees for validator viability.
```

**Scenario C: Stagnant Adoption (Flat)**

```
Assumptions:
  - Token price: flat to -30%
  - TPS: 500-1,500 avg (modest DeFi, limited AI usage)
  - AI inference: 50-200 requests/sec
  - Staking ratio: 45-55%
  - Validator count: 80-120

Outcomes:
  - Moderate fee revenue; emission is still majority of validator income
  - Burns occur but do not approach emission (net inflationary at declining rate)
  - PoUW validators earning modest supplemental income
  - Risk: developer attrition due to lack of growth narrative
  - Mitigation: Ecosystem Development Fund + AI Ecosystem Fund provide sustained grant funding
  - Risk: competing chains poach validators with higher rewards
  - Mitigation: PoUW income stream is unique -- no competing chain offers AI compute earnings
  - Overall: economically sustainable but requires active community building
```

**Self-Sustainability Threshold:**

The chain is economically self-sustaining (validators profitable without relying on emission) when:

```
Average validator annual income from fees > Average validator operating cost

Conservative operating cost: $600/year (consensus only), $3,600/year (PoUW T1), $14,400/year (PoUW T3)

With 100 validators at consensus-only:
  Required total annual fee revenue: $60,000
  At $0.001 per transaction: 60,000,000 transactions/year = ~2 TPS avg

With 100 validators including 30 PoUW T2 ($3,600/year) and 20 PoUW T3 ($14,400/year):
  Required total fee + inference revenue: ~$504,000/year
  At avg inference fee of $0.01: ~50M inferences/year = ~1.6 inferences/sec avg

These thresholds are achievable within year 2-3 under moderate adoption.
During year 1, emission subsidizes validator operations by design.
```

**Validator Count Drop Below Minimum:**

If the active validator count drops below 21:

```
Emergency Protocol:
1. Chain HALTS (safety over liveness, per Section 2.5.2)
2. Emergency emission bonus: remaining Validator Bootstrap Pool tokens released
   as emergency incentive for new validator registrations
3. Foundation Reserve can be deployed (via emergency multi-sig) to subsidize validators
4. Minimum stake is automatically halved to lower the barrier to entry
5. If count remains below 21 for >7 days, governance activates
   "survival mode": reduced validator set minimum (minimum 7 validators)
   with increased security assumptions
```

**Long-Term Sustainability Without Perpetual Growth:**

The tokenomics model is explicitly designed to NOT depend on perpetual growth:

1. **Emission is bounded:** The hard cap ensures inflation approaches zero over time.
2. **Validator income has multiple streams:** Even if one revenue source (e.g., DeFi fees) disappears, others (PoUW inference fees, emission) remain.
3. **Tail emission exists:** After year 10, a small tail emission (~3.75M tokens/year, declining) ensures validators always receive some block reward.
4. **AI compute demand is independent of crypto speculation:** The PoUW marketplace provides utility-driven demand for the token. Users pay for inference in native tokens regardless of market sentiment.
5. **Treasury accumulation during good times funds operations during bad times.**
6. **No algorithmic stability mechanisms:** There is no stablecoin or mint/burn mechanism that could create a death spiral (Terra lesson). The token price floats freely.

---

### 3.10 Coin Name Candidates

**PRELIMINARY - PENDING LEGAL CLEARANCE**

The following 10 name candidates are submitted for Phase 5a legal clearance by legal-ip-counsel. These names reflect the chain's AI-native identity, utility focus, server-independence, and desire for trademark distinctiveness (suggestive to fanciful on the distinctiveness spectrum per LEGAL_LANDSCAPE.md Section 6.1).

| # | Name | Ticker | Rationale | Distinctiveness |
|---|------|--------|-----------|----------------|
| 1 | **Synap** | SYNP | From "synapse" -- reflects the Synaptic Consensus mechanism. Short, memorable, unique in crypto. No known collision. | Fanciful |
| 2 | **Axon** | AXN | Neural axon -- the transmission line in neural networks. Suggests speed and connectivity. Unique, 4 letters. | Arbitrary (existing word in new context) |
| 3 | **Nura** | NURA | Derived from "neural." Feminine-coded names are rare in crypto, providing distinctiveness. Clean, easy to pronounce globally. | Fanciful |
| 4 | **Cortex** | CTX | The brain's outer layer where complex thought occurs. Suggests intelligence and layered architecture. Note: there is a "Cortex" (CTXC) project -- legal must assess collision risk. | Suggestive |
| 5 | **Velum** | VLM | Latin for "veil" or "sail" -- evokes both privacy (veiled) and movement (sail). Unique in crypto. | Fanciful |
| 6 | **Soma** | SOMA | Cell body of a neuron -- the computation center. Suggests core processing. Short, phonetically distinct. Note: Soma is also a Huxley/mythology term -- assess cultural connotations. | Arbitrary |
| 7 | **Aztibase** | AZTB | Neural aztibases receive signals -- reflects the P2P network receiving and processing data. Scientifically grounded, unique. | Suggestive |
| 8 | **Quen** | QUEN | Neologism -- evokes "query" and "queen" (processing authority). Short, globally pronounceable, zero collision risk expected. | Fanciful |
| 9 | **Thalos** | THAL | Evokes "thalamus" (brain's relay center for sensory signals) and "thalos" (Greek connotation of flourishing). Unique. | Fanciful |
| 10 | **Lumen** | LMN | Unit of light / enlightenment. Suggests clarity, intelligence, transparency. Note: Stellar Lumens uses "lumen" -- legal MUST assess collision. | Arbitrary |

**Critical notes for legal-ip-counsel:**
- Candidates #4 (Cortex) and #10 (Lumen) have known potential collisions and are included for assessment, not recommendation.
- All tickers must be checked against CoinMarketCap and CoinGecko databases.
- Domain availability (.com, .io, .org) must be verified for all candidates.
- Phonetic similarity analysis against top 150 crypto names required.
- International language check (offensive meanings) required for all candidates.

**These names are NOT finalized. Legal clearance is mandatory before any name is adopted.**

---

### 3.11 Stack Evaluation and Challenges

#### 3.11.1 Numeric Precision: ASSESSED (No Challenge)

Per Section 3.1.3, Rust's u128 for balances and the `uint` crate's u256 for intermediate calculations are sufficient for all tokenomics computations. The `uint` crate is already in the dependency tree via `revm`. No additional dependencies required.

**Key implementation requirements:**
- All token arithmetic in u128 base units (10^18 denomination)
- Intermediate multiplication in u256 to prevent overflow
- Floor division (truncation) for determinism
- Per-epoch reward accumulation (not retroactive long-period computation)

#### 3.11.2 Storage Overhead: ASSESSED (No Challenge)

The tokenomics model adds the following to per-validator on-chain state:

```
Per validator:
  - Stake amount:              u128 (16 bytes)
  - Delegated stake total:     u128 (16 bytes)
  - Commission rate:           u16  (2 bytes)
  - Reward accumulator:        u128 (16 bytes)
  - PoUW reward accumulator:   u128 (16 bytes)
  - Slashing history:          Vec<SlashRecord> (~64 bytes per record, max 10)
  - Unbonding entries:         Vec<UnbondEntry> (~32 bytes per entry, max 7)
  Total per validator: ~750 bytes (well within redb per-key limits)

Per delegator (per delegation):
  - Delegated amount:          u128 (16 bytes)
  - Validator address:         [u8; 32] (32 bytes)
  - Reward accumulator:        u128 (16 bytes)
  - Delegation epoch:          u64  (8 bytes)
  Total per delegation: ~72 bytes

Global economic state:
  - Total supply (emitted):    u128 (16 bytes)
  - Total burned:              u128 (16 bytes)
  - Current epoch emission:    u128 (16 bytes)
  - Treasury balance:          u128 (16 bytes)
  - Insurance fund balance:    u128 (16 bytes)
  - Staking ratio:             u64  (8 bytes)
  - Current base fee:          u128 (16 bytes)
  Total global: ~104 bytes
```

At 200 validators with 10,000 delegators average: ~870KB total economic state. This is negligible relative to the redb state store budget. **No challenge raised.**

#### 3.11.3 Computation Concerns

**Reward distribution computation:**

Reward distribution per epoch (every ~400 seconds) across 200 validators + delegators must complete within the epoch transition window. The computation is:

```
For each validator:
  1. Calculate consensus participation rate (count missed rounds)
  2. Calculate PoUW quality score (if applicable)
  3. Compute validator share of emission
  4. Distribute to delegators (pro-rata minus commission)

  Operations per validator: ~50 multiplications, ~50 additions
  Total for 200 validators: ~10,000 operations
  With u256 arithmetic: ~10,000 * ~100ns = ~1ms total
```

This is well within the 400-second epoch window. **No computation concern.**

---

**Stack challenges raised:** None. Rust's numeric types and the existing `uint` crate dependency are sufficient.

**Conflicts raised:** None. The tokenomics design aligns with all referenced sections (consensus, node, security, AI integration, smart contract).

**Sign-off:** tokenomics-engineer

## SECTION 4: NODE ARCHITECTURE

---
**Contributed by:** node-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete), Section 0 stack rulings (reviewed), Section 1 architecture (reviewed), Section 2 consensus mechanism (reviewed, including BLS12-381 stack challenge)

---

### NODE ARCHITECTURE

Aztibase Network defines five node types. Every node type is a first-class protocol citizen. No node type requires a central server to function. The design enables graceful degradation: a node can start as a browser light client and upgrade to a full validator without re-syncing from scratch.

---

### 4.1 Full Node Architecture

The full node is the backbone of the network. It stores the complete current state, validates all DAG vertices, maintains the full mempool, and serves data to light/browser/mobile clients. Every full node is also a local RPC endpoint -- no Infura/Alchemy dependency.

#### 4.1.1 Storage Engine: redb (ACCEPTED with refinements)

**Choice: redb** (Apache 2.0 license option per LEGAL_LANDSCAPE.md)

**Justification:**

| Criterion | redb | sled | redb |
|-----------|---------|------|------|
| Maturity | Production-proven at Facebook, Ethereum (geth), Solana, Sui, CockroachDB | Pre-1.0 (0.34.x), known data loss bugs in 2023-2024 | 1.x stable but young, limited production deployments |
| Write throughput | Excellent (LSM-tree optimized for write-heavy workloads) | Good | Good for single-writer workloads |
| Read performance | Good (bloom filters, block cache, prefix seek) | Good (B-tree, cache-friendly) | Good (B-tree, MVCC) |
| Compression | Native LZ4/Snappy/Zstd per column family | None built-in | None built-in |
| Column families | Yes (critical for dual storage architecture) | No (separate trees only) | Yes (tables) |
| Tuning surface | Extensive (hundreds of options -- both strength and weakness) | Minimal | Minimal |
| WASM compilation | No (C++ dependency) | No (uses `std::fs`) | Possible but not targeted |
| Concurrent access | Multi-threaded reads, single-writer with WAL | Lock-free reads, serialized writes | Single-writer MVCC |
| Memory control | Fine-grained block cache, write buffer sizing | Less control | Moderate control |

**Why redb wins for full nodes:**

1. **Dual storage architecture demands column families.** Section 1.5 mandates separate state store and state commitment stores. redb column families provide logical separation with shared write-ahead log and compaction management. This is exactly how Sei implements their dual storage -- we follow the proven pattern.

2. **Write-heavy workload profile.** At 10,000+ TPS with 400ms rounds, the storage engine must handle sustained high-throughput writes. redb's LSM-tree architecture is purpose-built for this workload. B-tree engines (sled, redb) are read-optimized and suffer write amplification under sustained write pressure.

3. **Compression is mandatory for the <100GB first-year target.** redb's per-column-family compression (Zstd for cold data, LZ4 for hot data) reduces storage 2-4x. Without native compression, sled/redb would require an application-level compression layer that adds complexity and latency.

4. **Production track record in blockchain.** Ethereum's geth, Solana, Sui, Aptos, and CockroachDB all use redb. The failure modes, tuning patterns, and operational knowledge base are deep. sled has known data corruption issues that are disqualifying for a financial system.

**redb tuning profile for Aztibase Network full nodes:**

```
Column Families:
  cf_state_store       -- Raw key-value state (account balances, contract storage, object data)
                       -- Optimized for point lookups and range scans
                       -- Compression: LZ4 (fast, moderate ratio)
                       -- Block cache: 1GB (configurable)

  cf_state_commitment  -- Verkle tree nodes (internal nodes + leaf commitments)
                       -- Optimized for sequential writes (tree updates) and random reads (proof generation)
                       -- Compression: Zstd (higher ratio, commitment data is cold-ish)
                       -- Block cache: 512MB (configurable)

  cf_dag_vertices      -- DAG vertex headers and bodies (block data)
                       -- Append-mostly workload
                       -- Compression: Zstd level 3
                       -- TTL-based compaction for pruned nodes

  cf_transactions      -- Transaction index (hash -> vertex location)
                       -- Write-once, read-many
                       -- Compression: LZ4
                       -- Bloom filter: 10 bits per key

  cf_receipts          -- Execution receipts
                       -- Write-once, read-occasionally
                       -- Compression: Zstd level 6 (aggressive -- receipts are rarely read)

  cf_ai_attestations   -- InferenceAttestation objects
                       -- Indexed by request_id and model_id
                       -- Compression: LZ4

Write-Ahead Log:
  -- Shared WAL across all column families
  -- WAL recycling enabled (reduces SSD wear)
  -- Sync mode: fdatasync per write batch (not per individual write)

Compaction:
  -- Level compaction for cf_state_store (read-optimized after compaction)
  -- Universal compaction for cf_dag_vertices (write-optimized, append-mostly)
  -- Max background compaction threads: 4 (configurable based on CPU cores)

Memory budget:
  -- Total redb memory target: 2-3GB on 8GB system
  -- Write buffer: 256MB total across all column families
  -- Block cache: 1.5GB shared (weighted by column family priority)
```

#### 4.1.2 State Model Implementation

The hybrid account+object model (Section 1.2.3) is stored as follows:

**Key schema in `cf_state_store`:**

```
Account state:
  key:   0x01 || account_address (32 bytes)
  value: AccountState { nonce, balance, code_hash, storage_root, account_type }

Object state:
  key:   0x02 || object_id (32 bytes)
  value: ObjectState { owner, type_tag, version, data_hash, data }

AI Agent state:
  key:   0x03 || agent_id (32 bytes)
  value: AIAgentState { owner, kya_hash, constraints, spending_limit, spent_epoch, capabilities }

Contract storage:
  key:   0x04 || contract_address (32 bytes) || storage_key (32 bytes)
  value: storage_value (variable length)

Model registry entry:
  key:   0x05 || model_hash (32 bytes)
  value: ModelMetadata { owner, version, size, format, license_terms, verification_methods }
```

**Key schema in `cf_state_commitment`:**

```
Verkle tree internal node:
  key:   0x10 || node_path (variable length, max 32 bytes)
  value: VerkleInternalNode { commitment, children_hashes }

Verkle tree leaf:
  key:   0x11 || leaf_path (32 bytes)
  value: VerkleLeaf { key, value_hash, commitment }
```

**Indexing strategy:**

- Primary index: key prefix byte determines entity type, enabling efficient prefix scans per entity type.
- Secondary indices are maintained in a separate column family (`cf_indices`) for queries like "all objects owned by address X" or "all agents with model capability Y." These indices are derived data and can be rebuilt from the state store.
- The Verkle tree structure in `cf_state_commitment` mirrors the state store but organized by tree path rather than entity type. Updates to `cf_state_commitment` are batched and applied asynchronously after execution (per Sei's dual storage pattern, Section 1.5).

#### 4.1.3 Block/DAG Processing Pipeline

```
                         Network Layer (Gossipsub)
                                |
                                v
                    +------------------------+
                    |   Vertex Reception     |
                    |   - Deserialize        |
                    |   - Basic validation   |
                    |   - Signature check    |
                    |   - Duplicate filter   |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   DAG Integrity Check  |
                    |   - Parent references  |
                    |     exist and valid    |
                    |   - Round number       |
                    |     consistent         |
                    |   - No equivocation    |
                    |     (author+round      |
                    |      uniqueness)       |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   Transaction          |
                    |   Validation           |
                    |   - Signature verify   |
                    |   - Nonce check        |
                    |   - Balance check      |
                    |   - Gas/fee check      |
                    |   - Remove from        |
                    |     mempool            |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   AI Attestation       |
                    |   Verification         |
                    |   - MPRE agreement     |
                    |   - TEE sig chain      |
                    |   - ZK proof verify    |
                    |   - ai_commitment_root |
                    |     correctness        |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   Anchor Processing    |
                    |   (if anchor round)    |
                    |   - VRF verify         |
                    |   - Commit rule check  |
                    |   - Topological sort   |
                    |   - Determine          |
                    |     execution order    |
                    +------------------------+
                                |
                                v
                    +------------------------+
                    |   Execution Engine     |
                    |   - Execute tx batch   |
                    |   - Parallel exec      |
                    |     (Block-STM)        |
                    |   - State transitions  |
                    |   - Receipt generation |
                    +------------------------+
                                |
                    +-----------+-----------+
                    |                       |
                    v                       v
          +----------------+     +-------------------+
          | State Store    |     | State Commitment  |
          | (sync write)   |     | (async update)    |
          | cf_state_store |     | cf_state_commit   |
          +----------------+     | Verkle tree       |
                                 | rebuild           |
                                 +-------------------+
```

**Pipeline concurrency model:**

- Vertex reception runs on a dedicated tokio task pool (2 threads). Incoming vertices are validated in parallel since validation of vertex A is independent of vertex B (until anchor ordering).
- DAG integrity checks are sequential per vertex but pipelined: while vertex N is being integrity-checked, vertex N+1 is being deserialized.
- Transaction validation is parallelized across the batch within a vertex using rayon (thread pool sized to CPU cores - 2).
- Execution is parallelized via the Block-STM optimistic concurrency model: transactions are speculatively executed in parallel, conflicts are detected, and conflicting transactions are re-executed sequentially.
- State commitment (Verkle tree update) is decoupled from execution via an async channel. The execution engine writes to `cf_state_store` synchronously, then enqueues the diff for the Verkle tree updater. This means the Verkle root for round N may lag by 1-2 rounds, which is acceptable because finality proofs reference the state root at the anchor commit point, not at every round.

#### 4.1.4 Mempool Design

The mempool buffers transactions before inclusion in DAG vertices. Aztibase Network uses a **priority-tiered mempool** with two pools:

**Transaction Mempool:**

```
TransactionMempool {
    pending:       BTreeMap<Priority, VecDeque<Transaction>>,  // Ready for inclusion
    queued:        HashMap<Address, BTreeMap<Nonce, Transaction>>,  // Future nonce gap
    max_size:      50_000 transactions (configurable),
    max_per_account: 64 transactions,
    eviction:      lowest-priority-first when full,
}

Priority = base_fee_bid * urgency_multiplier
```

- **Pending pool:** Transactions with correct nonce (next expected for their account). Sorted by priority (fee bid). Validators draw from the top of this pool when proposing vertices.
- **Queued pool:** Transactions with future nonces (gap between account's current nonce and transaction nonce). Automatically promoted to pending when the gap fills.
- **Eviction policy:** When the mempool is full, the lowest-priority transaction is evicted. Transactions below the minimum base fee are rejected outright.
- **Deduplication:** Transactions are keyed by hash. Receiving a duplicate is a no-op.
- **Propagation:** New valid transactions are gossiped to peers via Gossipsub. Validators who are about to propose a vertex may suppress gossiping transactions they intend to include (reducing redundant propagation).

**PoUW Mempool (separate):**

```
PoUWMempool {
    requests:      BTreeMap<Fee, VecDeque<InferenceRequest>>,
    max_size:      10_000 requests,
    assignment_map: HashMap<RequestId, Vec<ValidatorId>>,  // Tracks which validators are assigned
}
```

- InferenceRequests are kept in a separate pool because they follow a different lifecycle (assignment to specific validators, not inclusion by any validator).
- The PoUW scheduler draws from this pool and assigns requests to eligible validators based on their `AIComputeCommitment` declarations.

#### 4.1.5 Sync Strategies

Three sync strategies for new full nodes joining the network:

**Strategy 1: Snapshot Sync (recommended default, ~30 minutes)**

```
1. Connect to peers via DHT
2. Request latest finalized anchor hash from multiple peers (verify agreement)
3. Download the state snapshot corresponding to the finalized anchor
   - State snapshot = serialized cf_state_store + cf_state_commitment
   - Snapshot is chunked (4MB chunks) and downloaded from multiple peers in parallel
   - Each chunk includes a Verkle proof against the state root in the anchor header
4. Verify: the downloaded state's Verkle root matches the anchor header's state_root
5. Reconstruct local redb from the verified snapshot
6. Begin following the DAG from the anchor point forward (live sync)
7. Optionally: backfill historical DAG vertices in the background
```

- **Trust model:** Trusts that the finalized chain (2/3+ stake committed) is honest. This is the same trust assumption as all PoS snapshot sync mechanisms.
- **Storage:** Only current state is stored. Historical state is not downloaded. This is a pruned node by default.
- **Time estimate:** For a 50GB state at 100 Mbps download: ~67 minutes. With parallel chunked download from 8+ peers: ~30 minutes.

**Strategy 2: Fast Sync (~2-6 hours)**

```
1. Download all DAG vertex headers from genesis (or from a weak subjectivity checkpoint)
2. Verify the header chain: parent references, signatures, anchor commit proofs (BLS aggregate)
3. Download the latest state snapshot (same as snapshot sync step 3-5)
4. Verify: the state snapshot matches the header chain's latest finalized state root
5. Begin live sync
```

- **Trust model:** Verifies the full header chain (all finality proofs) but trusts state proofs for the snapshot. More secure than pure snapshot sync.
- **Storage:** Headers from genesis + current state. Headers are lightweight (~500 bytes each, ~1.2GB for the first year at 400ms rounds).

**Strategy 3: Full Sync (archive, ~days)**

```
1. Download all DAG vertices from genesis (headers + bodies)
2. Re-execute every transaction in order
3. Verify state roots at every anchor point
4. Result: complete historical state at every point
```

- **Trust model:** Fully trustless. Verifies everything from genesis.
- **Use case:** Archive nodes, auditors, researchers. Not for regular participants.

**DAG-specific sync considerations:**

- Unlike linear chains, DAG sync must handle the partial ordering. The syncing node downloads vertices round-by-round, verifying parent references as it goes.
- Vertices from the same round can be downloaded and validated in parallel (they are independent until the anchor ordering step).
- The syncing node maintains a "DAG frontier" -- the set of latest rounds it has fully validated. It requests vertices beyond the frontier from peers.

#### 4.1.6 Minimum Hardware Requirements (Full Node)

| Resource | Minimum | Recommended | Rationale |
|----------|---------|-------------|-----------|
| **CPU** | 4 cores, 2.5 GHz+ (x86_64 or ARM64) | 8 cores, 3.0 GHz+ | Block-STM parallel execution benefits from cores. 4 cores is floor for concurrent vertex validation + execution + networking. |
| **RAM** | 8 GB | 16 GB | redb block cache (2GB) + execution engine (1GB) + mempool (500MB) + networking (500MB) + OS overhead (2GB) + headroom. 8GB is tight but feasible with tuned redb. |
| **Storage** | 100 GB SSD (NVMe preferred) | 250 GB NVMe SSD | <100GB first year target (Section 1). SSD is mandatory -- redb's LSM compaction on HDD is catastrophically slow. NVMe preferred for write-heavy workload. |
| **Network** | 25 Mbps symmetric | 100 Mbps symmetric | 200 validators broadcasting vertices with transactions every 400ms. Each vertex ~100KB average. Inbound: ~200 * 100KB / 0.4s = ~50 MB/s peak burst, but with Gossipsub batching, sustained requirement is ~25 Mbps. |
| **OS** | Linux (Ubuntu 22.04+), macOS, Windows 10+ | Linux | Rust cross-platform. Linux is primary target for validators. |

**Consumer hardware viability assessment:**

A mid-range 2024 desktop (Ryzen 5 / Intel i5, 16GB RAM, 500GB NVMe SSD, 100 Mbps broadband) comfortably exceeds all recommended requirements. A Raspberry Pi 5 (8GB) is at the absolute floor of minimum requirements -- usable for non-validator full nodes with aggressive redb tuning. This meets the server-independence mandate: no datacenter hardware required for full participation.

---

### 4.2 Light Node Architecture

Light nodes verify the chain without storing or executing the full state. They rely on Verkle state proofs to verify any claim about current state, and BLS aggregate finality proofs to confirm finality. They are the primary node type for users who want to interact with the chain without running a full node.

#### 4.2.1 Storage Engine: redb (REFINED from Section 0 proposal)

**Choice: redb** for light nodes.

**Justification against alternatives:**

| Criterion | redb | sled | redb |
|-----------|------|------|---------|
| Binary size overhead | ~300KB | ~1.5MB | ~5MB (C++ static link) |
| Memory footprint | ~10-50MB configurable | ~50-200MB | ~200MB+ even with minimal config |
| Dependency complexity | Pure Rust, zero C dependencies | Pure Rust, zero C dependencies | Requires C++ toolchain, cmake |
| Crash safety | ACID with checksums | Claimed ACID but known bugs | ACID, battle-tested |
| Concurrent readers | Yes (MVCC) | Yes (lock-free) | Yes |
| Maturity for financial data | 1.x stable, used in Bitcoin ordinals tooling | Pre-1.0, data loss reports | Battle-tested |
| Embedded suitability | Excellent (designed for embedded) | Good | Poor (designed for server workloads) |

**Why redb over sled:**

sled's pre-1.0 status and documented data corruption issues in 2023-2024 are disqualifying for a financial application, even at the light client level. redb reached 1.0 stability, has ACID guarantees with checksum verification, and is being used in Bitcoin ecosystem tooling (ord/ordinals). Its pure-Rust implementation eliminates the C++ toolchain dependency that makes redb painful for cross-compilation and mobile builds.

**Why not redb:**

redb is overkill for light nodes. A light node stores only: headers, finality certificates, a small set of cached Verkle proofs, and local wallet state. This is <1GB of data with minimal write throughput. redb's 5MB binary overhead, 200MB+ memory floor, and C++ dependency are unjustified for this workload.

**redb schema for light nodes:**

```
Table: headers
  key:   round_number (u64)
  value: GenesisBlockHeader (serialized, ~500 bytes)

Table: finality_certs
  key:   anchor_round (u64)
  value: FinalityCertificate { anchor_hash, aggregate_bls_signature, signer_bitfield }

Table: verkle_proof_cache
  key:   state_key (variable)
  value: CachedVerkleProof { proof, verified_at_round, value }
  -- TTL: proofs older than 1000 rounds (~400s) are evicted

Table: wallet_state
  key:   address (32 bytes)
  value: LocalWalletState { last_known_balance, last_known_nonce, pending_txs }

Table: peer_cache
  key:   peer_id
  value: PeerInfo { multiaddr, last_seen, reliability_score }
```

#### 4.2.2 Verification via Verkle Proofs

Verkle proofs are the killer feature for light clients. Unlike Merkle SPV proofs that grow logarithmically with tree size, Verkle proofs are **constant-size** regardless of tree depth.

**How light node verification works:**

```
Light node wants to verify: "Account 0xABC has balance 1000 GEN"

1. Light node has: finality certificate for anchor round R
   (verified via BLS aggregate signature -- one pairing operation)

2. Light node requests from any full node peer:
   "Give me a Verkle proof for account 0xABC against state root at round R"

3. Full node responds with:
   VerkleProof {
     leaf_value: AccountState { balance: 1000, ... },
     proof: Vec<[u8; 48]>,  // Constant-size IPA/KZG proof elements
     commitment_path: Vec<VerkleCommitment>,
   }

4. Light node verifies:
   a. The proof is valid against the state_root in the anchor header at round R
   b. The anchor header at round R is covered by a valid finality certificate
   c. The finality certificate's BLS aggregate signature verifies against the
      known validator set

5. Result: The light node has cryptographic proof that the balance claim is
   correct, verified against the finalized chain, without trusting any
   individual full node.
```

**Verkle proof sizes (estimated):**

| Proof type | Size | Equivalent Merkle proof |
|------------|------|------------------------|
| Single key existence | ~500-700 bytes | ~2-5 KB (depth-dependent) |
| Multi-key batch proof | ~700-1200 bytes (for up to 100 keys) | ~10-50 KB |
| Non-existence proof | ~500-700 bytes | ~3-6 KB |

This constant-size property means light clients can verify arbitrary state claims with minimal bandwidth, making mobile and browser nodes practical for state verification.

#### 4.2.3 BLS Aggregate Finality Proof Verification

Per the consensus-engineer's BLS12-381 stack challenge (Section 2.9.3), light nodes verify finality using aggregate BLS signatures.

**Finality certificate structure:**

```
FinalityCertificate {
    anchor_hash:           Hash,          // The committed anchor vertex hash
    anchor_round:          u64,
    state_root:            VerkleRoot,    // State root at this anchor
    signer_bitfield:       BitVec,        // Which validators signed (bit per validator)
    aggregate_signature:   BLS12_381Sig,  // Single 96-byte aggregate signature
    validator_set_hash:    Hash,          // Hash of the validator set for this epoch
}
```

**Light node verification:**

```
1. Obtain the validator set for the epoch (downloaded once per epoch, ~1000 rounds)
   - Validator set = Vec<(ValidatorId, BLS_PublicKey, StakeWeight)>
   - Verified by checking validator_set_hash against the previous epoch's finality cert

2. For each finality certificate:
   a. Extract the set of signing validators from signer_bitfield
   b. Compute the aggregate public key: sum of all signers' BLS public keys
   c. Verify: BLS_verify(aggregate_pubkey, anchor_hash, aggregate_signature)
   d. Check: total stake of signers >= 2/3 of total stake

3. One pairing operation (~1.5ms on desktop, ~5ms on mobile)
   vs. 134+ Ed25519 verifications (~9.4ms on desktop, ~30ms+ on mobile)
```

**This is why BLS is essential for light clients.** The consensus-engineer's stack challenge is fully supported from the node architecture perspective. Without BLS aggregation, light/browser/mobile nodes would need to download and verify 134+ individual signatures per finality certificate, which is prohibitive for bandwidth-constrained and compute-constrained environments.

> **STACK CHALLENGE ENDORSEMENT [node-engineer -> blockchain-architect]: BLS12-381 Signatures**
>
> node-engineer ENDORSES the consensus-engineer's stack challenge on BLS12-381 (Section 2.9.3). Without BLS aggregate signatures, the viability of light nodes, browser nodes, and mobile nodes for independent finality verification is severely compromised. The bandwidth savings (96 bytes vs ~4.3KB for 134 Ed25519 signatures) and verification time savings (1 pairing vs 134 verifications) are critical for resource-constrained node types.
>
> **Additional consideration:** The `blst` crate (BLS12-381 implementation) compiles to WASM via `wasm32-unknown-unknown` target. This has been tested by multiple projects (Lodestar, Chainsafe). Browser nodes can verify BLS aggregate proofs in WASM. This addresses the browser node viability concern directly.

#### 4.2.4 Light Node Capabilities and Limitations

**What light nodes CAN do:**

- Verify finality of any anchor (via BLS aggregate finality certificate)
- Verify any state claim (account balance, object ownership, contract storage value) via Verkle proofs
- Submit transactions to the network (gossip to connected full node peers)
- Verify transaction inclusion (Merkle proof of transaction in a committed vertex)
- Verify AI inference attestation results (the attestation is on-chain data, verifiable via Verkle proof)
- Serve as an RPC endpoint for local applications (wallet, dApp frontend)
- Participate in gossip (relay transactions and vertex headers to other light nodes)

**What light nodes CANNOT do:**

- Validate full vertex content (they do not have the state needed for transaction execution)
- Propose DAG vertices (not validators)
- Execute smart contracts locally (no full state)
- Generate Verkle proofs (requires full state tree)
- Serve state to other light nodes (they only have cached proofs, not the full tree)
- Run PoUW inference (no AI runtime)

**Trust model:**

Light nodes trust that the finalized chain (2/3+ BLS-attested) is honest. They do NOT trust individual full nodes -- every state claim is verified via Verkle proofs against the finalized state root. This is a significant improvement over Ethereum-style light clients, which rely on honest majority assumptions for state queries without cryptographic verification.

#### 4.2.5 Minimum Hardware Requirements (Light Node)

| Resource | Minimum | Recommended | Rationale |
|----------|---------|-------------|-----------|
| **CPU** | 2 cores, 1.5 GHz+ | 4 cores | BLS verification (~5ms per finality cert), Verkle proof verification (~1ms per proof). Minimal compute. |
| **RAM** | 512 MB | 1 GB | redb (~20MB) + networking (~100MB) + proof cache (~50MB) + OS. 512MB is tight but achievable. |
| **Storage** | 2 GB | 10 GB | Headers (~1.2GB/year) + finality certs (~100MB/year) + proof cache + wallet state. Pruned headers for very old rounds reduce this further. |
| **Network** | 1 Mbps | 10 Mbps | Header sync only (~500 bytes * 2.5 headers/sec = ~10 Kbps steady state). Proof requests are on-demand. |

A Raspberry Pi Zero 2 W (512MB RAM, microSD, WiFi) meets minimum requirements. Any smartphone from the last 5 years exceeds recommended requirements.

---

### 4.3 Browser Node (WASM)

The browser node is a light client compiled to WebAssembly and running in a standard browser tab. It connects to the P2P network via WebRTC and stores state in IndexedDB. No plugins, extensions, or native code required.

#### 4.3.1 WASM Compilation Strategy

**Target:** `wasm32-unknown-unknown` compiled via `wasm-pack` with `wasm-bindgen` for JavaScript interop.

**Crate compatibility assessment:**

| Crate | WASM-compatible | Notes |
|-------|----------------|-------|
| `blst` (BLS12-381) | Yes | Compiles to WASM. Used by Lodestar/Chainsafe. ~800KB WASM binary contribution. |
| `ed25519-dalek` | Yes | Pure Rust, compiles cleanly to WASM. |
| `blake3` | Yes | Pure Rust implementation. The SIMD-optimized paths are disabled in WASM; falls back to portable implementation. ~2x slower than native but still sub-microsecond per hash. |
| `redb` | No | Uses `std::fs` for file I/O. Not usable in browser. Browser uses IndexedDB instead. |
| `libp2p` (core) | Partial | `libp2p-webrtc` supports WASM target. `libp2p-tcp`, `libp2p-quic` do NOT (no raw socket access in browsers). `libp2p-gossipsub` compiles to WASM. `libp2p-kad` compiles to WASM. |
| `tokio` | No (full) | `tokio` does not compile to WASM. Use `wasm-bindgen-futures` + browser event loop instead. |
| `redb` | Possible but not targeted | Pure Rust, no C deps. Browser nodes use in-memory or IndexedDB instead. |
| `serde` / `postcard` | Yes | Pure Rust serialization. Compiles cleanly. |
| `tract` (AI runtime) | No | Depends on `ndarray` with native optimizations. Not targeted for browser. Browser nodes do not run inference. |
| `verkle-trie` (proof verification only) | Yes (with feature flags) | The proof verification path is pure math (field operations, polynomial evaluation). Proof generation requires full tree and is not WASM-targeted. |

**WASM binary size budget:**

| Component | Estimated WASM Size |
|-----------|-------------------|
| Core types + serialization | ~200 KB |
| Cryptography (blake3 + ed25519-dalek + blst) | ~1.2 MB |
| libp2p-webrtc + gossipsub + kademlia | ~800 KB |
| Verkle proof verification | ~300 KB |
| Application logic (header sync, proof cache, RPC) | ~500 KB |
| **Total (uncompressed)** | **~3 MB** |
| **Total (gzip compressed)** | **~1.0-1.2 MB** |

This is comparable to a moderately-sized JavaScript bundle and within acceptable browser loading times (~2-3 seconds on 3G, <1 second on broadband).

#### 4.3.2 WebRTC Transport

Browser nodes connect to the P2P network via WebRTC DataChannels through `libp2p-webrtc`:

**Signaling (no central server):**

- Initial signaling uses the libp2p Circuit Relay v2 protocol. The browser node connects to a known relay node (full node with public IP) via WebSocket, then establishes WebRTC connections to other peers through the relay.
- Once WebRTC connections are established, they are direct (peer-to-peer, no relay for data transfer unless behind symmetric NAT).
- Relay nodes are discovered via the DHT. The browser node maintains a list of known relay nodes and rotates through them. No single relay is a dependency.
- Decentralized TURN relays (incentivized full nodes per Section 1.4.5) handle the ~15% of connections that fail ICE negotiation.

**Connection management:**

```
BrowserConnectionManager {
    max_peers:          12 (limited to reduce browser resource usage),
    min_peers:          4  (minimum for reliable gossip),
    peer_types:         prefer full nodes (they can serve Verkle proofs),
    reconnect_interval: 30 seconds (if below min_peers),
    keepalive:          WebRTC DataChannel heartbeats every 15 seconds,
    bandwidth_limit:    1 Mbps inbound, 500 Kbps outbound (configurable),
}
```

#### 4.3.3 Browser Storage Strategy

**Primary: IndexedDB**

```
IndexedDB stores:
  "headers"          -- Recent block headers (last 1000 rounds, ~500KB)
  "finality_certs"   -- Finality certificates (last 100 anchors, ~50KB)
  "proof_cache"      -- Cached Verkle proofs (LRU, max 10MB)
  "wallet_state"     -- Local wallet data (accounts, pending transactions)
  "peer_cache"       -- Known peer addresses for reconnection
  "config"           -- User preferences, key material (encrypted)
```

**Secondary: In-memory (for ephemeral data)**

```
In-memory:
  Mempool of user's own pending transactions
  Active WebRTC connection state
  Current round tracking
  Gossip message deduplication bloom filter
```

**Storage limits:**

- IndexedDB quota varies by browser (Chrome: ~60% of disk, Firefox: ~50%, Safari: ~1GB on mobile). Genesis browser node targets <50MB total IndexedDB usage, well within all browser limits.
- OPFS (Origin Private File System) is available in Chrome 86+, Firefox 111+, Safari 15.2+ as an alternative with better performance for sequential access. However, IndexedDB is sufficient for light client workloads and has broader compatibility.

#### 4.3.4 Browser Node Capabilities

**What browser nodes CAN do (subset of light node):**

- Verify finality certificates (BLS aggregate, ~5-10ms in WASM)
- Verify Verkle state proofs (~2ms in WASM)
- Submit transactions to the network via WebRTC
- Query account balances and object state (via full node peers + Verkle proof verification)
- Display transaction history (for monitored addresses)
- Act as a local RPC endpoint for the dApp running in the same page

**What browser nodes CANNOT do (in addition to light node limitations):**

- Persist state across browser restarts reliably (user may clear data)
- Maintain persistent peer connections (tab can be closed)
- Serve as relay nodes for other peers
- Run for extended periods without user interaction (browser may throttle/suspend background tabs)

**Graceful degradation:**

- If the browser node cannot establish WebRTC connections (e.g., very restrictive firewall), it falls back to WebSocket connections to full nodes. This is less decentralized but maintains functionality.
- If IndexedDB is full or unavailable, the browser node operates in pure in-memory mode (no persistence, fresh sync on every page load).

#### 4.3.5 Browser Compatibility Matrix

| Browser | WebRTC DataChannels | IndexedDB | WASM | SharedArrayBuffer | Overall Support |
|---------|-------------------|-----------|------|-------------------|----------------|
| Chrome 90+ | Full | Full | Full | Yes (with COOP/COEP headers) | Full |
| Firefox 100+ | Full | Full | Full | Yes (with COOP/COEP headers) | Full |
| Safari 15.4+ | Full | Full (1GB limit on mobile) | Full | Yes (since 15.2) | Full |
| Edge 90+ (Chromium) | Full | Full | Full | Yes | Full |
| Chrome Android 90+ | Full | Full | Full | Yes | Full |
| Safari iOS 15.4+ | Full | Quota-limited | Full | Yes | Supported (with storage caveats) |
| Firefox Android 100+ | Full | Full | Full | Yes | Full |
| Opera 76+ | Full | Full | Full | Yes | Full |

**Required HTTP headers for full functionality:**

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

These headers enable SharedArrayBuffer, which is needed for multi-threaded WASM (cryptographic operations in Web Workers). Without these headers, the browser node falls back to single-threaded operation with ~2x slower BLS verification.

**Minimum browser requirements:** Any browser supporting WebRTC + WASM + IndexedDB, which is effectively all browsers updated since 2022.

---

### 4.4 Mobile Node

Mobile nodes are light client variants optimized for the constraints of smartphones: limited battery, intermittent connectivity, background process restrictions, and smaller screens.

#### 4.4.1 Platform Strategy

**Both Android and iOS are first-class targets.**

| Aspect | Android | iOS |
|--------|---------|-----|
| **Implementation** | Rust core compiled as shared library (`.so`) via NDK, exposed via JNI to Kotlin UI | Rust core compiled as static library (`.a`), exposed via C FFI to Swift UI |
| **Storage** | redb (same as light node) on app-internal storage | redb on app-internal storage |
| **Networking** | libp2p with TCP/QUIC transport (not limited to WebRTC like browsers) | libp2p with TCP/QUIC transport |
| **Background execution** | WorkManager for periodic sync (minimum 15-minute interval) | BGAppRefreshTask (system-determined intervals, typically 15-60 min) |
| **Minimum OS** | Android 8.0 (API 26) -- ~95% of active devices | iOS 15 -- ~95% of active devices |

**Shared Rust core:**

The mobile node shares the same Rust light client codebase as the desktop light node. Platform-specific code is limited to:
- FFI bridge layer (JNI for Android, C FFI for iOS)
- Background task scheduling
- Notification handling
- Secure key storage (Android Keystore / iOS Keychain)
- Network reachability monitoring

#### 4.4.2 Battery Optimization Strategies

**1. Header sync batching:**
- Instead of syncing every header as it arrives (every 400ms), the mobile node batches header sync. When in foreground, sync every 5 seconds (batch of ~12 headers). When in background, sync only at system-scheduled intervals (15-60 minutes).

**2. Selective finality tracking:**
- Mobile nodes do NOT track every finality certificate. They sync finality certificates at configurable intervals (e.g., every 10th anchor) and verify the latest one when the user opens the app.
- For transaction confirmation, the mobile node requests the specific finality certificate covering the user's transaction on-demand.

**3. Peer connection management:**
- Foreground: maintain 4-6 peer connections.
- Background: drop all connections. Reconnect on wake.
- Use QUIC (UDP-based) for faster connection establishment on wake (~1 RTT vs TCP's 3-way handshake).

**4. Crypto operation batching:**
- BLS verification and Verkle proof verification are batched. Rather than verifying proofs one-at-a-time, the mobile node accumulates proof requests and verifies them in a single compute burst, then sleeps. This is more battery-efficient than many small wakeups.

**5. Network coalescing:**
- All network requests are coalesced into bursts aligned with the OS power-management schedule. No trickle connections.

**Expected battery impact:**
- Foreground active use: comparable to a web browser with active WebSocket connection. No significant additional drain.
- Background sync (15-minute intervals): <1% battery per hour. Negligible.

#### 4.4.3 Mobile Light Client Design

The mobile node is functionally identical to the desktop light node (Section 4.2) with these modifications:

- **Reduced proof cache:** 5MB max (vs 10MB for desktop light node). Proofs are evicted more aggressively.
- **Reduced header retention:** 500 rounds (vs 1000 for desktop). Older headers are pruned.
- **Offline verification:** The mobile node caches the latest finality certificate and the user's account state proof. If offline, the user can still view their last-known balance with a "verified as of round X" indicator.
- **Push notification integration:** Full nodes that the mobile node has established trust with can send push notifications (via platform push services) when a watched transaction is finalized. The mobile node wakes, syncs the relevant finality certificate, verifies it, and shows the confirmation.

#### 4.4.4 Minimum Hardware Requirements (Mobile Node)

| Resource | Minimum | Notes |
|----------|---------|-------|
| **RAM** | 256 MB available to app | redb (~10MB) + networking (~50MB) + crypto (~20MB) + UI. 256MB app allocation is standard on any phone with 2GB+ total RAM. |
| **Storage** | 100 MB | Headers + finality certs + proof cache + wallet state + app binary. |
| **CPU** | ARM64 (ARMv8) | BLS verification ~5-10ms. Any smartphone from 2017+ has sufficient compute. |
| **Network** | Intermittent connectivity tolerated | Sync on reconnect. No always-on requirement. |
| **OS** | Android 8.0+ / iOS 15+ | For NDK/FFI support and background task APIs. |

---

### 4.5 Validator Node

A validator node is a full node that additionally participates in SynBFT consensus by proposing DAG vertices, participating in anchor commits, and optionally performing PoUW inference.

#### 4.5.1 Additional Requirements Beyond Full Node

| Resource | Full Node (Min) | Validator (Min) | Validator + PoUW (Min) | Rationale |
|----------|----------------|-----------------|----------------------|-----------|
| **CPU** | 4 cores | 8 cores | 8 cores + GPU | Vertex proposal + validation + VRF computation. PoUW inference requires GPU or high-core CPU. |
| **RAM** | 8 GB | 16 GB | 32 GB | Additional memory for BLS key management, VRF computation state, higher mempool capacity, and signing pipeline. PoUW models may require significant RAM. |
| **Storage** | 100 GB SSD | 250 GB NVMe | 500 GB NVMe | Validators should retain more history for equivocation detection and serving sync to other nodes. PoUW models cached on disk. |
| **Network** | 25 Mbps | 100 Mbps symmetric | 100 Mbps symmetric | Validators must broadcast vertices every round and receive all other vertices. Consistent low-latency connectivity is more important than raw bandwidth. |
| **Uptime** | Best-effort | 99%+ (reputation penalty for downtime) | 99%+ | Missing rounds reduces ConsensusReputation and may trigger cooldown rotation. |

#### 4.5.2 DAG Vertex Proposal Pipeline

Every 400ms round, the validator executes this pipeline:

```
Round Timer Fires (400ms interval)
        |
        v
+---------------------------+
| 1. VRF Evaluation         |   (~0.1ms)
|    Compute VRF proof for  |
|    this round. Determines |
|    if this validator is   |
|    the anchor proposer.   |
+---------------------------+
        |
        v
+---------------------------+
| 2. Transaction Selection  |   (~2-5ms)
|    Draw top-priority txs  |
|    from mempool.          |
|    Max: fit within 2MB    |
|    block size limit.      |
|    Include AI attestations|
|    if PoUW validator.     |
+---------------------------+
        |
        v
+---------------------------+
| 3. Parent Selection       |   (~0.5ms)
|    Select >= 2f+1 parent  |
|    vertices from round-1. |
|    Prefer vertices with   |
|    high coverage (avoid   |
|    censorship scoring).   |
|    Add skip edges to      |
|    older rounds if some   |
|    round-1 vertices are   |
|    missing.               |
+---------------------------+
        |
        v
+---------------------------+
| 4. Vertex Construction    |   (~1-2ms)
|    Build GenesisBlockBody |
|    Compute tx Merkle root |
|    Compute AI commitment  |
|    root.                  |
+---------------------------+
        |
        v
+---------------------------+
| 5. Signing                |   (~0.1ms Ed25519)
|    Sign header with       |   (~1ms BLS, anchor only)
|    Ed25519 key.           |
|    If anchor round: also  |
|    sign with BLS key.     |
+---------------------------+
        |
        v
+---------------------------+
| 6. Broadcast              |   (~1ms)
|    Gossipsub publish to   |
|    "genesis/dag/vertices" |
|    topic.                 |
+---------------------------+

Total pipeline: ~5-10ms per round
Budget within 400ms round: ample headroom (390ms for receiving
and validating other validators' vertices).
```

#### 4.5.3 VRF Computation Requirements

VRF (Verifiable Random Function) is computed each anchor round (every 4th round by default) to determine the anchor proposer.

**VRF implementation:** Ed25519-based VRF (ECVRF-EDWARDS25519-SHA512-TAI per RFC 9381).

- **Prove (generate VRF output + proof):** ~0.15ms on modern hardware.
- **Verify (check another validator's VRF proof):** ~0.25ms per proof.
- **Per round cost:** Each validator verifies up to 200 VRF proofs per anchor round = ~50ms. This fits comfortably within the 400ms round budget.

**Key management:**

```
ValidatorKeyBundle {
    ed25519_keypair:  Ed25519KeyPair,    // Transaction signing, vertex signing, VRF
    bls_keypair:      BLS12_381KeyPair,  // Aggregate finality proofs
    // Both stored in platform-appropriate secure storage:
    //   Linux: encrypted file with OS keyring passphrase
    //   HSM: supported via PKCS#11 interface for production validators
}
```

#### 4.5.4 PoUW Capability (AI Inference Hardware)

PoUW is optional. Validators who opt in must meet additional hardware requirements based on the models they commit to serving:

| PoUW Tier | Hardware | Example Models | Expected Earnings Premium |
|-----------|----------|---------------|-------------------------|
| **Tier 1: CPU-only** | 16+ cores, 32GB RAM, AVX2 support | Small classifiers, anomaly detection, fraud scoring (<100M params) | 1.2-1.5x base rewards |
| **Tier 2: Consumer GPU** | NVIDIA RTX 3060+ (8GB VRAM) or equivalent | Medium models, NLP inference, image classification (<1B params) | 2-3x base rewards |
| **Tier 3: Professional GPU** | NVIDIA A10/A100 or equivalent (24GB+ VRAM) | Large language models, complex inference (1B-7B params) | 4-8x base rewards |

**AI runtime stack on validator:**

```
Validator AI Stack:
  tract (primary)     -- CPU inference, small models, deterministic
  candle (secondary)  -- GPU inference, larger models, CUDA/Metal
  ONNX Runtime (opt)  -- Maximum compatibility, largest models

Model cache:
  Local disk cache of models from ModelRegistry (IPFS-backed)
  Prefetch models listed in AIComputeCommitment
  Max cache size: configurable (default 50GB)
```

**Determinism handling for MPRE:**

- tract produces deterministic results for the same model + input on the same architecture (IEEE 754 compliance).
- Cross-architecture non-determinism (x86 vs ARM, different GPU architectures) is handled by quantizing comparison outputs to 16-bit fixed-point for MPRE agreement checking (per Section 2.3.2).

#### 4.5.5 Validators Behind NAT

Validators are explicitly supported behind consumer NAT:

1. **ICE negotiation:** On startup, the validator performs ICE to determine its NAT type and establish connectivity. ~85% of consumer NAT configurations allow direct peer-to-peer connections via STUN.

2. **Relay fallback:** For symmetric NAT (~15% of cases), the validator connects through incentivized TURN relays (other full nodes). Consensus messages are small:
   - Vertex headers: ~500 bytes
   - Full vertices: ~100KB average
   - At 400ms rounds, this is ~250KB/s sustained through the relay -- well within relay capacity.

3. **UPnP/NAT-PMP:** The node automatically attempts to open a port via UPnP or NAT-PMP on the local router. If successful, the validator becomes directly reachable without relay.

4. **Latency budget:** The 400ms round time provides generous latency budget. A validator behind a relay adds ~20-50ms of additional latency per message, leaving >300ms for actual processing. Performance analysis shows that validators with up to ~150ms additional relay latency can maintain full ConsensusReputation scores.

5. **Reachability requirement:** A validator MUST be reachable by at least 2f+1 other validators within the round time. If a validator is persistently unreachable (>10 consecutive rounds with zero peers receiving its vertices), it is temporarily removed from the active set and must re-enter at the next epoch boundary.

---

### 4.6 State Pruning Strategy

State pruning is essential for keeping full node storage requirements within the <100GB first-year target and for long-term sustainability.

#### 4.6.1 Pruning Architecture

Aztibase Network uses a **layered pruning strategy** with three independently configurable retention periods:

```
+-----------------------------------------------------------------+
|                    RETENTION LAYERS                               |
+-----------------------------------------------------------------+
| Layer           | Default Retention | Prunable?  | Archive Node  |
|-----------------|-------------------|------------|---------------|
| Current state   | Always retained   | No         | Always        |
| Recent state    | 1000 epochs       | Yes        | Always        |
|   snapshots     | (~4.6 days)       |            |               |
| DAG vertices    | 10,000 epochs     | Yes        | Always        |
|   (headers)     | (~46 days)        |            |               |
| DAG vertices    | 1,000 epochs      | Yes        | Always        |
|   (bodies)      | (~4.6 days)       |            |               |
| Transaction     | 10,000 epochs     | Yes        | Always        |
|   receipts      | (~46 days)        |            |               |
| AI attestations | 5,000 epochs      | Yes        | Always        |
|                 | (~23 days)        |            |               |
+-----------------------------------------------------------------+
```

#### 4.6.2 How Pruning Maintains Verkle Tree Integrity

The critical challenge: pruning old state while maintaining the integrity of the Verkle tree for proof generation.

**Approach: Snapshot + diff model**

```
1. Every N epochs (default: 100 epochs, ~46 minutes), a STATE SNAPSHOT is taken.
   - The snapshot is a complete serialization of cf_state_store at that point.
   - The snapshot's Verkle root matches the state_root in the corresponding
     anchor header.

2. Between snapshots, the node stores STATE DIFFS:
   - Each committed anchor produces a diff: { key -> (old_value, new_value) }
   - Diffs are stored in cf_state_store with a snapshot_id prefix.

3. To generate a Verkle proof for a historical state (within retention period):
   - Start from the nearest snapshot
   - Apply diffs forward/backward to reach the target round
   - Generate the proof from the reconstructed state

4. Pruning:
   - When a snapshot ages beyond the retention period (1000 epochs), it is deleted.
   - All diffs between the pruned snapshot and the next retained snapshot are also deleted.
   - The Verkle tree nodes for the pruned state are garbage-collected.
   - The CURRENT state and its Verkle tree are NEVER pruned.
```

**Verkle tree garbage collection:**

- Verkle tree internal nodes that are only referenced by pruned state versions are deleted.
- The garbage collector runs as a low-priority background task during periods of low I/O.
- To avoid accidentally deleting nodes still needed by the current tree, the GC uses a mark-and-sweep approach:
  1. Mark all nodes reachable from the current state root.
  2. Mark all nodes reachable from retained snapshot state roots.
  3. Sweep (delete) all unmarked nodes.

#### 4.6.3 Archive vs. Pruned Node

| Aspect | Pruned Node (default) | Archive Node |
|--------|----------------------|-------------|
| Current state | Full | Full |
| Historical state | Last ~4.6 days | All from genesis |
| DAG headers | Last ~46 days | All from genesis |
| DAG bodies | Last ~4.6 days | All from genesis |
| Transaction receipts | Last ~46 days | All from genesis |
| Storage (year 1) | <100 GB | ~500 GB - 1 TB |
| Storage (year 3) | ~150 GB | ~2-5 TB |
| Can serve snapshot sync | Yes (current state) | Yes (any historical state) |
| Can serve full sync | No (missing historical blocks) | Yes |
| Can generate historical proofs | Within retention window | For any point in history |

**Archive node incentive:** Archive nodes are valuable for the ecosystem (block explorers, analytics, auditing). The tokenomics-engineer should consider incentive mechanisms for archive node operators (Section 3).

#### 4.6.4 State Expiry (Future Enhancement)

Beyond pruning (which removes old versions of state), state expiry removes state entries that have not been accessed for a long period:

- **Concept:** State entries (accounts, objects, contract storage) that have not been read or written for M epochs are "expired" -- their data is removed from `cf_state_store` but a cryptographic witness (hash of the expired data) is retained in the Verkle tree.
- **Resurrection:** A user can resurrect expired state by providing the original data (which matches the witness hash). The data is re-inserted into the state store.
- **Why deferred:** State expiry is complex and controversial (see Ethereum's years-long debate). Aztibase Network launches WITHOUT state expiry. The dual storage architecture and pruning strategy keep storage manageable for the first several years. State expiry is designed as a possible future protocol upgrade via governance.

---

### 4.7 Graceful Node Upgrade Path

A key design requirement: nodes can start as one type and upgrade to another without re-syncing from scratch.

```
Browser Node -> Light Node:
  - User installs native client
  - Import peer cache and wallet state from browser (export/import via QR or file)
  - Begin light node header sync from the browser node's last known finality cert
  - No re-sync needed; just continue from where the browser node left off

Light Node -> Full Node:
  - Initiate snapshot sync of full state (this IS a download, ~50GB)
  - While snapshot downloads, the light node continues operating normally
  - Once snapshot is verified and loaded, switch to full node mode
  - Begin validating and storing full DAG vertices going forward
  - redb is initialized alongside redb during the transition
  - Once full node is operational, redb can be deprecated (or kept for
    lightweight access patterns)

Full Node -> Validator Node:
  - Stake the minimum required amount on-chain
  - Generate BLS keypair (if not already done)
  - Register validator candidate transaction
  - Wait for epoch boundary for potential selection into active set
  - No storage or sync changes needed -- full node already has complete state

Validator Node -> Validator + PoUW:
  - Install AI runtime (tract/candle)
  - Download and cache target models from ModelRegistry
  - Submit AIComputeCommitment transaction
  - Begin receiving inference assignments at next epoch
```

---

### 4.8 Node Type Participation Summary

| Capability | Full Node | Light Node | Browser Node | Mobile Node | Validator | Validator+PoUW |
|------------|-----------|------------|-------------|-------------|-----------|---------------|
| Store full state | Yes | No | No | No | Yes | Yes |
| Validate all vertices | Yes | No | No | No | Yes | Yes |
| Verify finality (BLS) | Yes | Yes | Yes | Yes | Yes | Yes |
| Verify state (Verkle) | Yes (generate) | Yes (verify) | Yes (verify) | Yes (verify) | Yes (generate) | Yes (generate) |
| Submit transactions | Yes | Yes | Yes | Yes | Yes | Yes |
| Propose DAG vertices | No | No | No | No | Yes | Yes |
| Run AI inference | No | No | No | No | No | Yes |
| Serve RPC locally | Yes | Yes | Yes | Yes | Yes | Yes |
| Relay for other nodes | Yes | Partial | No | No | Yes | Yes |
| Persist state reliably | Yes | Yes | Limited | Yes | Yes | Yes |
| Run on consumer HW | Yes | Yes | Yes (browser) | Yes (phone) | Yes (8-core) | Depends on tier |

---

### 4.9 Stack Evaluation and Challenges

#### 4.9.1 redb for Full Nodes: ACCEPTED

redb is the correct choice for full nodes and validators. The LSM-tree architecture, column family support, compression, and proven blockchain track record are unmatched. No challenge raised.

#### 4.9.2 redb for Light Nodes: REFINED (was sled/redb in Section 0)

> **STACK REFINEMENT [node-engineer -> blockchain-architect]: Light Node Storage**
>
> Section 0 listed "sled, redb" as alternatives for light nodes. This refinement selects **redb** specifically and **rejects sled**.
>
> **Justification:** sled's pre-1.0 status and documented data corruption issues in the 2023-2024 timeframe make it unsuitable for storing financial data (even the limited data a light node holds). redb has reached 1.0 stability, provides ACID guarantees with checksums, and is pure Rust with zero C dependencies. It is the right embedded database for light and mobile nodes.
>
> **This is a refinement, not a challenge** -- Section 0 already included redb as a candidate and granted node-engineer authority to refine storage choices.

#### 4.9.3 WASM Compilation Feasibility: ASSESSED

**Feasible with constraints.** The critical crates for browser node operation (blst, ed25519-dalek, blake3, libp2p-webrtc, libp2p-gossipsub, libp2p-kad, serde, postcard, verkle proof verification) all compile to `wasm32-unknown-unknown`. The primary constraint is that `tokio` does not compile to WASM, requiring the use of `wasm-bindgen-futures` and the browser's event loop instead. This is a well-understood pattern used by multiple production WASM applications.

**Risk flag:** `libp2p-webrtc` for WASM is functional but has seen limited production deployment as of March 2026. The Webcoin project (Bitcoin in browser via WebRTC) provides a reference point, but Aztibase Network's use case (continuous header sync + Verkle proof requests) is more demanding than Webcoin's (occasional block header verification). Recommend allocating dedicated testing effort to browser node stability.

#### 4.9.4 BLS12-381 Stack Challenge: ENDORSED

See Section 4.2.3 for the full endorsement. The node-engineer fully supports the consensus-engineer's BLS12-381 challenge. Without BLS aggregate signatures, light nodes, browser nodes, and mobile nodes cannot efficiently verify finality. The `blst` crate's WASM compatibility is confirmed, addressing the browser node viability concern.

#### 4.9.5 Resource Requirements vs. Node-Friendliness: FLAGGED

> **STACK FLAG [node-engineer -> blockchain-architect]: Network Bandwidth Concern**
>
> At 200 validators each broadcasting a ~100KB vertex every 400ms, the raw gossip bandwidth is ~50 MB/s peak inbound for a full node. While Gossipsub's message batching and deduplication reduce this significantly (estimated 60-70% reduction), the steady-state bandwidth requirement of ~25 Mbps symmetric is at the upper end of consumer broadband in many regions.
>
> **This does not threaten viability** -- 25 Mbps symmetric is available on most broadband connections in North America, Europe, and East Asia. However, it may limit full node participation in regions with slower internet infrastructure (parts of Africa, South Asia, South America).
>
> **Mitigation recommendations:**
> 1. Implement compressed block propagation (compact blocks pattern from Bitcoin) -- can reduce bandwidth 50-80%.
> 2. Gossipsub message deduplication and batch propagation (already planned per Section 0 stack ruling on libp2p).
> 3. Full node operators can configure the number of Gossipsub mesh peers (fewer peers = less bandwidth, slightly slower propagation).
> 4. Light nodes have minimal bandwidth requirements (~1 Mbps) and are the appropriate choice for bandwidth-constrained environments.
>
> **Priority:** STANDARD (not blocking, but p2p-network-engineer should design compressed propagation from day one).

#### 4.9.6 Storage Growth Projection

| Time Period | Pruned Full Node | Archive Node | Light Node |
|-------------|-----------------|-------------|------------|
| Launch | ~5 GB | ~5 GB | ~100 MB |
| 3 months | ~25 GB | ~50 GB | ~500 MB |
| 6 months | ~50 GB | ~150 GB | ~800 MB |
| 1 year | ~80-100 GB | ~400-500 GB | ~1.5 GB |
| 2 years | ~120-150 GB | ~1-1.5 TB | ~3 GB |
| 3 years | ~150-200 GB | ~2-3 TB | ~5 GB |

Assumptions: 10,000 TPS average, 400ms rounds, Zstd compression on cold data, state pruning with 1000-epoch retention, header pruning with 10,000-epoch retention.

The <100GB first-year target (Section 1) is achievable for pruned full nodes. Archive nodes will require dedicated storage infrastructure beyond year 1, which is expected and acceptable (archive nodes are specialized infrastructure, not consumer nodes).

---

**Stack challenges raised:**
- STACK CHALLENGE ENDORSEMENT: BLS12-381 signatures (endorsing consensus-engineer's challenge from Section 2.9.3). See Section 4.2.3.
- STACK REFINEMENT: Light node storage -- redb selected, sled rejected. See Section 4.9.2.
- STACK FLAG: Network bandwidth concern for full nodes in bandwidth-constrained regions. See Section 4.9.5.

**Conflicts raised:** None

**Sign-off:** node-engineer

## SECTION 5: SECURITY MODEL

---
**Contributed by:** security-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete), LEGAL_LANDSCAPE.md (complete), Section 0 stack rulings (reviewed), Section 1 architecture (reviewed), Section 2 consensus mechanism (reviewed), Section 4 node architecture (reviewed), Section 8 P2P network design (reviewed)

---

### SECURITY MODEL

This section defines the comprehensive security posture for Aztibase Network. As security-engineer, I hold elevated authority per ORCHESTRATION.md -- security challenges raised here MUST be addressed by the blockchain-architect before proceeding. Every section of the MASTER_DESIGN.md has been reviewed for security weaknesses, and findings are documented below alongside the complete threat model, cryptographic standards review, and AI security monitoring design.

---

### 5.1 Threat Matrix

The following threat matrix covers all identified attack vectors across eight domains. Severity ratings use a 4-tier scale: CRITICAL (chain-breaking, immediate exploitation potential), HIGH (significant financial loss or consensus disruption), MEDIUM (degraded performance or limited exploit window), LOW (theoretical or mitigated by design).

#### 5.1.1 Consensus Attacks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| C1 | **Byzantine validators (>1/3 stake)** | CRITICAL | BFT 1/3 fault tolerance; 5% stake cap requires controlling 7+ identities; cooldown rotation; slashing | An attacker accumulating >1/3 of total stake through gradual purchase on secondary markets. Mitigation: stake cap limits influence per identity. Residual: coordinated collusion among 7+ independent validators remains theoretically possible. |
| C2 | **DAG equivocation (dual vertex proposal)** | HIGH | Retroactive equivocation detection via signed evidence; immediate slashing; both vertices marked invalid; ejection from active set (Section 2.7.5) | Brief window (~1 round, 400ms) where conflicting vertices may propagate before detection. Uncertified DAG design (per Mysticeti) widens this window slightly vs. certified DAGs. See SECURITY FLAG S2-1 below. |
| C3 | **Vertex withholding** | MEDIUM | Late vertices (>3 rounds) receive reduced reputation weight; vertices >10 rounds are discarded; persistent withholding reduces ConsensusReputation (Section 2.7.5) | Strategic short-delay withholding (1-2 rounds) to gain information advantage before releasing. Residual: minor MEV extraction opportunity in the 400-800ms window. |
| C4 | **Selective parent referencing (censorship)** | MEDIUM | Mandatory 2f+1 parent references; censorship scoring tracks parent coverage deviation; high censorship score degrades reputation; extreme cases trigger slashing (Section 2.7.5) | Subtle censorship (consistently excluding 1-2 validators while meeting 2f+1 threshold) is harder to detect. Residual: low-rate targeted censorship may persist below detection thresholds. |
| C5 | **Selfish DAG mining (strategic vertex timing)** | MEDIUM | Round-based DAG structure limits timing manipulation; vertices are bound to specific rounds; the anchor commit rule uses deterministic topological sort, removing ordering advantage from timing | A validator delaying its vertex to observe others' vertices and include more profitable transactions. Residual: ~200ms information advantage within a round is possible but yields minimal benefit given deterministic ordering. |
| C6 | **Anchor proposer manipulation** | HIGH | VRF-based anchor selection ensures unpredictability; AnchorScore weights are bounded (stake 70%, reputation 30%); VRF output is verifiable by all validators (Section 2.2.3) | If VRF implementation has bias or predictability, anchor selection becomes exploitable. See SECURITY FLAG S2-2 below. |
| C7 | **Long-range attack (history rewrite)** | MEDIUM | Absolute BFT finality (committed blocks are irreversible); weak subjectivity checkpoints for new nodes; 21-day unbonding period (Section 2.7.3) | New nodes syncing without a recent trusted checkpoint could be fed a fabricated history. Residual: requires social consensus on checkpoint distribution. |
| C8 | **Nothing-at-stake** | LOW | Largely inapplicable to DAG-BFT: one vertex per round enforced, no traditional forks, equivocation is detectable and slashable (Section 2.7.2) | Negligible. DAG structure eliminates the fork-voting incentive. |
| C9 | **Finality delay attack (preventing anchor commits)** | HIGH | Anchor commits require 2f+1 support; attacker needs >1/3 stake to prevent commits; chain halts safely rather than finalizing incorrect state (Section 2.5.2) | Liveness degradation: an attacker with >1/3 stake can halt the chain. Safety is preserved but availability is lost. This is an inherent BFT tradeoff. |

#### 5.1.2 Network Attacks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| N1 | **Eclipse attack** | HIGH | Peer diversity policy (max 30% per /16 subnet, min 3 ASNs); outbound connection preference (min 30% outbound); anchor-based eclipse detection (no finality certs for >8s triggers alarm + peer rotation); finality certificate cross-checking via random DHT peers (Section 8.7.1) | Sophisticated attacker controlling infrastructure across multiple ASNs and subnets. Residual: eclipse of a single non-validator node is possible but limited in impact (cannot affect consensus). Eclipse of a validator triggers missed-round detection and ejection. |
| N2 | **Sybil attack (network layer)** | MEDIUM | Stake-gated Gossipsub peer scoring (+50 for validators); DHT record signature validation; connection rate limiting (10/sec, 3 per IP); resource-based peer scoring (Section 8.7.2) | Sybil nodes can populate DHT routing tables but cannot forge validator signatures or finality certificates. Residual: DHT pollution may slow peer discovery but does not compromise chain security. |
| N3 | **DDoS on P2P layer** | HIGH | Per-peer rate limiting (100 msg/sec, 1 MB/sec); per-topic rate limits; connection limits per IP (3); known validator whitelist; emergency mode restricting to known peers only (Section 8.7.3) | Volumetric DDoS overwhelming the node's network interface. Residual: ISP-level or infrastructure-level DDoS protection is outside protocol scope. Validators should use DDoS-protected hosting or VPN services. |
| N4 | **BGP hijacking** | HIGH | QUIC (primary transport) provides built-in mutual authentication via TLS 1.3; Noise XX provides mutual authentication for TCP; PeerId verification after handshake; encrypted connections prevent MITM | BGP hijacking can redirect traffic but cannot decrypt or forge authenticated messages. Residual: availability impact (traffic black-holed). Mitigation: multiple bootstrap mechanisms and DHT re-routing. |
| N5 | **Routing attacks (traffic analysis)** | MEDIUM | All connections encrypted (Noise/TLS/DTLS); no plaintext mode exists; QUIC multiplexes streams (harder to identify message types from traffic patterns) | Traffic volume correlation can reveal validator identity and activity patterns. Residual: Tor/VPN integration for validators desiring anonymity is optional but not built into the protocol. |
| N6 | **Gossipsub mesh manipulation** | MEDIUM | Peer scoring with heavy penalty for invalid messages (-100); IP colocation penalty; behavior penalty decay; graylist threshold disconnects worst offenders (Section 8.7.4) | Attacker maintaining just-above-threshold scores to remain in mesh while subtly degrading propagation. Residual: bounded by Gossipsub's heartbeat mesh repair (700ms). |
| N7 | **WebRTC signaling interception** | MEDIUM | Initial WebSocket is to ANY full node (not a central server); WebRTC uses DTLS encryption; PeerId verification post-connection; browser can try multiple bootstrap nodes in parallel (Section 8.5.1) | If all available bootstrap full nodes are compromised, a browser node could be eclipsed at initial connection. Residual: mitigated by having 50+ bootstrap peers operated by independent parties. |

#### 5.1.3 Smart Contract Attacks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| SC1 | **Reentrancy** | HIGH | WASM sandbox enforces structured execution (no arbitrary callbacks during host calls); recommended checks-effects-interactions pattern in SDK; revm (EVM compat layer) must implement reentrancy guards at the precompile level | EVM compatibility layer inherits Ethereum's reentrancy surface. Residual: EVM contracts must follow standard patterns. AI anomaly detection (Section 5.4) provides runtime monitoring. |
| SC2 | **Integer overflow/underflow** | MEDIUM | Rust's default overflow checking (panic in debug, wrap in release); WASM contracts compiled from Rust inherit this; AssemblyScript and C contracts require explicit checked arithmetic in SDK | Non-Rust contract languages (C, AssemblyScript) do not have automatic overflow protection. Residual: SDK must enforce checked arithmetic via library wrappers. |
| SC3 | **Access control bypass** | HIGH | Protocol-level agent constraints (spending limits, capability declarations enforced at VM level per Section 1.6.3); contract-level access control is developer responsibility; AI audit tooling for contract review | Misconfigured access control in user-deployed contracts. Residual: cannot be fully prevented at the protocol level. AI-based contract auditing (Section 5.4.3) mitigates. |
| SC4 | **Oracle manipulation** | HIGH | AI Oracle is a protocol primitive (not an external service); InferenceAttestations are verified via MPRE/TEE/ZK before being available to contracts; multi-method verification prevents single-point oracle manipulation (Section 2.3.2) | MPRE with 2-of-3 agreement is vulnerable if 2 of 3 selected validators collude. Residual: for high-value inference, users should specify ZK verification or increase MPRE redundancy. |
| SC5 | **Front-running / MEV** | HIGH | Priority fee market from genesis (not retrofitted); DAG structure reduces MEV opportunity (multiple validators propose in parallel, reducing single-proposer ordering advantage); AI-based MEV detection (Section 5.4.1) | Validators can still order transactions within their own vertex. Residual: intra-vertex MEV is possible. Future mitigation: encrypted mempool (commit-reveal scheme) or threshold decryption. See SECURITY FLAG S1-1 below. |
| SC6 | **Flash loan attacks** | MEDIUM | Fee market creates cost for large transactions; AI anomaly detection monitors for flash loan patterns; protocol does not natively support flash loans (must be implemented by DeFi contracts) | Flash loans are a DeFi contract feature, not a protocol vulnerability. Residual: DeFi contracts on Aztibase Network can implement flash loans with their own risk profiles. |
| SC7 | **WASM sandbox escape** | CRITICAL | wasmtime runtime provides hardware-enforced sandboxing; memory isolation between contracts; bounded execution (gas metering); no raw syscall access from WASM | Zero-day vulnerabilities in wasmtime. Residual: critical dependency on wasmtime security. Mitigation: track wasmtime CVEs, maintain rapid update capability, consider running wasmtime with additional OS-level sandboxing (seccomp on Linux). See STACK SECURITY REVIEW Section 5.7. |

#### 5.1.4 Cryptographic Risks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| CR1 | **BLAKE3 security margin** | LOW | BLAKE3 uses a 7-round ChaCha-based compression function; 256-bit output provides 128-bit collision resistance; no known attacks exist; see Section 5.2.1 for detailed analysis | BLAKE3 is newer than SHA-3 with less cryptanalysis history. Residual: theoretical risk of future cryptanalytic breakthrough. Acceptable given the migration path via `StateCommitment` trait abstraction. |
| CR2 | **Ed25519 implementation weaknesses** | MEDIUM | ed25519-dalek is a well-audited pure-Rust implementation; cofactor handling and malleability are addressed in the latest versions; RFC 8032 compliance | Signature malleability in older Ed25519 implementations could enable transaction ID mutation. Residual: ensure ed25519-dalek uses strict verification (reject non-canonical signatures). See SECURITY FLAG S0-1 below. |
| CR3 | **BLS12-381 pairing attacks** | MEDIUM | BLS12-381 is a well-studied pairing-friendly curve with 128-bit security; `blst` library is audited and production-proven (Ethereum beacon chain); rogue key attacks prevented by proof-of-possession (Section 5.2.3) | Side-channel attacks on pairing computations. Residual: HSM-based BLS signing for validators mitigates. |
| CR4 | **Quantum threat to elliptic curves** | HIGH (long-term) | Verkle-to-binary-Merkle migration path designed in (Section 1.5); `StateCommitment` trait abstraction; `commitment_scheme_version` field in proofs; post-quantum signature migration plan (Section 5.5) | Cryptographically relevant quantum computers are estimated 10-15 years away (NIST timeline). Residual: if timeline accelerates, emergency migration needed. See Section 5.5 for full quantum readiness assessment. |
| CR5 | **Verkle tree polynomial commitment compromise** | HIGH (long-term) | IPA/KZG commitments rely on discrete log hardness (elliptic curves) -- quantum-vulnerable; binary Merkle tree + SNARK fallback maintained as alternative backend (Section 1.5) | If quantum computers arrive before migration, all state proofs become forgeable. Residual: requires proactive migration. See Section 5.5. |
| CR6 | **Side-channel attacks in browser WASM** | MEDIUM | COOP/COEP headers required for SharedArrayBuffer (multi-threaded WASM); timing attacks in WASM are limited by browser timer resolution; cryptographic operations use constant-time implementations | Spectre/Meltdown variants in browser environments. Residual: browser vendors patch these, but zero-day exposure exists. Browser nodes should not hold high-value keys. See SECURITY FLAG S4-1 below. |

#### 5.1.5 AI-Specific Risks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| AI1 | **Adversarial inputs to on-chain AI** | HIGH | Input validation at the InferenceRequest level; model-specific input schema enforcement in ModelRegistry; AI anomaly detection monitors for adversarial input patterns | Adversarial examples that subtly manipulate model outputs (e.g., pixel perturbations in image classifiers). Residual: adversarial robustness is an open ML research problem. Mitigated by MPRE (multiple independent executions would need to be simultaneously fooled). |
| AI2 | **Model poisoning** | HIGH | Models are identified by hash in ModelRegistry; model content is verified against hash on download from IPFS; model updates create new registry entries (immutable versioning); model reputation scoring based on attestation history | A malicious model registered in ModelRegistry that produces subtly wrong outputs. Residual: model quality assessment is fundamentally difficult. Mitigated by user/community reputation systems and opt-in model auditing. |
| AI3 | **Inference manipulation (result tampering)** | HIGH | Three-method verification (MPRE 2-of-3 agreement, TEE hardware attestation, ZK proofs of correct execution); slashing for incorrect attestations (Section 2.3.2, 2.7.6) | MPRE collusion (2 of 3 validators agree on wrong result). Residual: increase m for high-value requests. TEE side-channel attacks are known but practically difficult. ZK proofs are mathematically sound. |
| AI4 | **PoUW gaming (garbage outputs)** | HIGH | Multi-metric quality evaluation (correctness 40%, availability 25%, latency 20%, diversity 15%); reputation collapse for low-quality outputs; progressive slashing; model hash verification prevents model substitution (Section 2.3.4, 2.7.6) | Validators producing minimally-correct but low-quality results (meeting correctness threshold but not actually useful). Residual: quality assessment beyond correctness requires application-specific evaluation, which is hard to standardize. |
| AI5 | **AI reputation manipulation for consensus advantage** | MEDIUM | PoUW reputation capped at 15% of AnchorScore; gaming PoUW reputation to dominate anchor selection requires sustained high-quality inference delivery (which is actual useful work); reputation decays over 2000 rounds (Section 2.2.3) | A well-resourced attacker investing in legitimate AI compute to gain the 15% anchor selection advantage. Residual: this is by design -- the 15% cap limits the influence. The attacker would be providing genuine useful work. |
| AI6 | **Model extraction via inference queries** | MEDIUM | TEE-based inference protects model weights; ZK proofs do not reveal model internals; rate limiting on inference requests per model; model owners can set access controls in ModelRegistry | Repeated inference queries to reconstruct model behavior (model stealing). Residual: fundamental tension between inference availability and model privacy. Rate limiting and query cost create economic barriers. |

#### 5.1.6 Economic Attacks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| E1 | **Flash loan governance attacks** | HIGH | Governance participation requires staked tokens (not borrowed); stake lock-up period for governance voting; time-locked proposal execution (voting period + execution delay) | If governance uses token-weighted voting without lock-up, flash loans can swing votes. Residual: tokenomics-engineer MUST design governance with anti-flash-loan mechanisms. See SECURITY FLAG S3-1 below. |
| E2 | **MEV extraction by validators** | HIGH | DAG parallelism reduces single-validator ordering power; priority fee market; AI-based MEV detection; future encrypted mempool capability (Section 5.1.3 SC5) | Validators can order transactions within their vertex. Residual: intra-vertex MEV. See SECURITY FLAG S1-1. |
| E3 | **Governance capture** | HIGH | 5% stake cap per validator; 30% diversity quota; cooldown rotation; no delegation concentration beyond 5% cap (Section 2.4.3) | Coordinated entities controlling multiple validator identities. Residual: identity-level Sybil resistance is imperfect. Social layer governance oversight recommended. |
| E4 | **Validator collusion** | HIGH | 5% cap requires 7+ colluding parties for 1/3 threshold; slashing for detected misbehavior; economic cost of stake accumulation; cooldown rotation disrupts persistent collusion groups | Silent collusion (validators coordinating off-chain without detectable on-chain misbehavior). Residual: undetectable coordinated behavior is an open problem in all BFT systems. |
| E5 | **Death spiral (token value collapse)** | CRITICAL | Single token model (no algorithmic stability mechanism to spiral); bounded minting/burning (Section 1.8 constraints for tokenomics); PoUW rewards from dedicated allocation, not inflation; formal verification of economic model under adversarial conditions required before launch | If token value drops sharply, validator incentive to stake diminishes, potentially reducing security. Residual: common to all PoS chains. Mitigated by minimum validator set (21) and community-driven recovery. Terra-style death spirals are structurally prevented by no algorithmic stability mechanism. |
| E6 | **Relay incentive gaming** | MEDIUM | Relay bandwidth reports cross-referenced with client reports; relay reputation tracking on-chain; poor-quality relays lose bond; relay connections have 30-minute duration limits (Section 8.4.4) | Colluding relay and client nodes inflating usage reports. Residual: cross-referencing limits but does not eliminate fabricated reports. |

#### 5.1.7 Node-Level Attacks

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| NL1 | **State bloat** | MEDIUM | State pruning with configurable retention (default ~4.6 days for state snapshots); Zstd compression on cold data; <100GB first-year target; state expiry as future governance-enabled upgrade (Section 4.6) | Long-term state growth beyond pruning retention still accumulates. Residual: state expiry deferred to future upgrade. Pruning keeps manageable for multi-year horizon. |
| NL2 | **Mempool flooding** | HIGH | Priority-tiered mempool with max 50,000 transactions; max 64 per account; eviction of lowest-priority transactions; minimum base fee rejection; separate PoUW mempool with 10,000 request cap (Section 4.1.4) | Sustained flood of minimum-fee transactions consuming mempool capacity. Residual: legitimate transactions with higher fees displace flood transactions via priority eviction. Economic cost to attacker scales with flood duration. |
| NL3 | **Resource exhaustion (CPU)** | MEDIUM | Gas metering in WASM VM; bounded execution per block (2MB max block size); parallel execution limited by available cores; rate limiting at network layer | Complex transactions consuming maximum gas. Residual: gas costs should be calibrated to reflect actual computation cost. Gas pricing review is required. |
| NL4 | **Storage attacks (write amplification)** | MEDIUM | redb write batching (WAL sync per batch, not per write); configurable compaction threads (max 4); column family separation isolates workloads; Zstd/LZ4 compression reduces write volume (Section 4.1.1) | LSM-tree compaction storms during high write throughput. Residual: redb tuning mitigates but SSD wear is a long-term concern for validators. NVMe recommended. |
| NL5 | **Disk space exhaustion** | MEDIUM | State pruning enforced by default; configurable retention periods; pruned full node targets <100GB first year; storage growth projections documented (Section 4.9.6) | Misconfigured or archive nodes exhausting disk. Residual: operational concern, not protocol vulnerability. Node monitoring recommended. |

#### 5.1.8 Browser/Mobile Specific Vectors

| # | Attack Vector | Severity | Mitigation | Residual Risk |
|---|--------------|----------|------------|---------------|
| BM1 | **WASM sandbox escape** | HIGH | Browser WASM sandbox is maintained by browser vendors (Chrome V8, Firefox SpiderMonkey); Genesis WASM code runs within the browser's existing sandbox; no raw system access | Zero-day WASM sandbox escape in a browser engine. Residual: browser vendors are the first line of defense. Genesis browser nodes do not store high-value keys by default. |
| BM2 | **WebRTC vulnerabilities** | MEDIUM | WebRTC is maintained by browser vendors; DTLS encryption is mandatory; PeerId verification after connection; libp2p-webrtc handles WebRTC-specific security (Section 8.5) | WebRTC ICE candidate leaking local IP addresses. Residual: known WebRTC privacy issue; browsers are adding mitigations (mDNS candidates). Does not affect security, only privacy. |
| BM3 | **Side-channel in browser (timing attacks)** | MEDIUM | COOP/COEP headers enable SharedArrayBuffer with site isolation; constant-time crypto implementations (blst, ed25519-dalek); browser timer resolution limits (performance.now() precision reduced post-Spectre) | Microarchitectural side-channels (Spectre variants). Residual: browser-node should not be used for high-value key management. |
| BM4 | **IndexedDB data tampering** | LOW | IndexedDB is origin-locked (browser same-origin policy); sensitive data (keys) encrypted before storage; Verkle proofs and finality certificates are cryptographically self-verifying (tampered data fails verification) | Malicious browser extension accessing IndexedDB. Residual: user's browser security posture is outside protocol scope. Recommend: browser node should not store high-value private keys. |
| BM5 | **Mobile background process termination** | LOW | Mobile nodes designed for intermittent connectivity; sync-on-wake with batch verification; cached finality certificates for offline verification; no always-on requirement (Section 4.4) | Not a security attack per se, but reliability concern. Residual: mobile node may have stale state. "Verified as of round X" indicator informs user. |
| BM6 | **Mobile key storage compromise** | MEDIUM | Android Keystore / iOS Keychain for key material; hardware-backed key storage on supported devices; biometric unlock for transaction signing | Device compromise (rooting/jailbreaking) exposes key storage. Residual: standard mobile security posture applies. High-value users should use hardware wallets. |

---

### 5.2 Cryptographic Standards Review

#### 5.2.1 BLAKE3 for Hashing: APPROVED

**Decision: APPROVED with note.**

BLAKE3 is used for transaction hashing, Merkle root computation, VRF seed computation, and general-purpose hashing throughout Aztibase Network.

**Security analysis:**

| Property | BLAKE3 | SHA-3 (Keccak) | SHA-256 |
|----------|--------|----------------|---------|
| Output size | 256-bit (configurable) | 256-bit | 256-bit |
| Collision resistance | 128-bit | 128-bit | 128-bit |
| Preimage resistance | 256-bit | 256-bit | 256-bit |
| Construction | Merkle-Damgard with ChaCha-based compression (7 rounds) | Sponge (Keccak-f[1600], 24 rounds) | Merkle-Damgard (SHA-2 family, 64 rounds) |
| Cryptanalysis history | Since 2020 (~6 years) | Since 2008 (~18 years, NIST competition winner) | Since 2001 (~25 years) |
| Best known attack | None beyond generic | None beyond generic | None beyond generic |
| Performance | ~4x faster than SHA-256, ~6x faster than SHA-3 | Slower than SHA-256 on most platforms | Baseline |
| Tree hashing | Native (parallel) | Requires application-level | Requires application-level |

**Assessment:**

- BLAKE3's 7-round ChaCha compression function provides a comfortable security margin. The BLAKE family (BLAKE, BLAKE2, BLAKE3) has undergone significant cryptanalysis as BLAKE was a SHA-3 finalist. No practical attacks exist.
- The reduced round count (7 vs. ChaCha20's 20 rounds) is compensated by the wider state and the tree hashing construction. BLAKE2's 10-12 rounds have withstood over a decade of analysis; BLAKE3's 7 rounds provide slightly less margin but remain well within safe territory for 128-bit collision resistance.
- SHA-3 has more cryptanalysis history but is significantly slower. For a blockchain processing 10,000+ TPS with 400ms rounds, the performance advantage of BLAKE3 is operationally significant.
- **Risk acceptance:** BLAKE3 is a well-designed, conservatively secure hash function. The `StateCommitment` trait abstraction provides a migration path if a weakness is ever discovered. APPROVED for all uses.

**Note:** The Kademlia DHT uses SHA-256 for key space compatibility with libp2p's implementation (Section 8.2.2). This is acceptable -- SHA-256 is used only for DHT key derivation, not for security-critical protocol operations. No security concern.

#### 5.2.2 Ed25519 for Signatures: APPROVED with MANDATORY hardening

**Decision: APPROVED with mandatory implementation requirements.**

Ed25519 is used for transaction signing, DAG vertex signatures, VRF computation, and general authentication.

**Security analysis:**

- Ed25519 provides ~128-bit security against classical computers.
- The ed25519-dalek crate is well-audited and widely used in the Rust ecosystem.
- Ed25519 is faster than ECDSA (secp256k1) for both signing and verification.
- Ed448 (providing ~224-bit security) was considered but rejected: the additional security margin is unnecessary against classical computers, and Ed448 is ~3x slower. Against quantum computers, both Ed25519 and Ed448 are equally broken (Shor's algorithm). The quantum defense is post-quantum signature migration (Section 5.5), not a larger curve.

**Mandatory hardening requirements:**

> **SECURITY FLAG S0-1 [security-engineer -> blockchain-architect]: Ed25519 Strict Verification**
>
> The ed25519-dalek crate MUST be configured with **strict verification mode** (reject non-canonical signatures, enforce cofactor checking). Signature malleability in lax verification modes can enable transaction ID mutation attacks. Specifically:
>
> 1. Use `ed25519_dalek::Verifier::verify_strict()` not `verify()`.
> 2. Reject signatures with non-canonical S values (S >= L where L is the group order).
> 3. Reject small-order public keys (torsion components).
> 4. Document the exact ed25519-dalek version and feature flags in the security manifest.
>
> **Priority: SECURITY-ELEVATED.** Incorrect Ed25519 verification has caused real vulnerabilities in production blockchains (Stellar, Monero).

#### 5.2.3 BLS12-381 for Aggregate Signatures: APPROVED

**Decision: APPROVED.** The consensus-engineer's BLS12-381 stack challenge (Section 2.9.3) is ENDORSED from a security perspective.

**Security analysis:**

- BLS12-381 provides 128-bit security (matching Ed25519 and BLAKE3).
- The `blst` library (Supranational) is the most widely audited BLS implementation, used by Ethereum's beacon chain across multiple client implementations.
- BLS signature aggregation is mathematically sound and has been in production on Ethereum since December 2020 (beacon chain genesis).

**Mandatory security requirements for BLS:**

1. **Proof of Possession (PoP):** Each validator MUST register a proof-of-possession for their BLS public key (sign their own public key with the corresponding private key). This prevents rogue key attacks where an attacker registers a public key that is the additive inverse of another validator's key, allowing the attacker to forge aggregate signatures. PoP verification must occur at validator registration time.

2. **Domain separation:** BLS signing must use distinct domain separation tags for different message types (finality certificates vs. other potential future uses). This prevents cross-protocol signature reuse attacks.

3. **Subgroup checking:** All BLS public keys and signatures must be verified as elements of the correct subgroup (G1 or G2 depending on the chosen pairing scheme). Invalid subgroup elements can be used in attacks on aggregate verification.

#### 5.2.4 Noise Protocol for P2P Encryption: APPROVED

**Decision: APPROVED.** No security concerns.

- Noise XX provides mutual authentication with forward secrecy.
- The handshake completes in 1.5 round trips with ephemeral Diffie-Hellman.
- Compromise of static keys does not reveal past traffic (forward secrecy).
- No CA dependency (unlike TLS) aligns with server-independence mandate.
- TLS 1.3 for QUIC and DTLS for WebRTC are both standards-compliant and secure.
- All connections are encrypted with no plaintext mode. APPROVED.

#### 5.2.5 Verkle Tree Commitment Scheme: APPROVED with CRITICAL quantum caveat

**Decision: CONDITIONALLY APPROVED.** Approved for launch with mandatory quantum migration path.

**Cryptographic soundness:**

Verkle trees using IPA (Inner Product Argument) or KZG (Kate-Zaverucha-Goldberg) commitments are cryptographically sound under the discrete logarithm assumption on elliptic curves. The constant-size proof property and 100x reduction vs. Merkle proofs are verified.

**Critical concern: quantum vulnerability.**

Both IPA and KZG commitments rely on the hardness of the discrete logarithm problem on elliptic curves. A sufficiently powerful quantum computer running Shor's algorithm would break these commitments, allowing an attacker to forge state proofs (claim any account balance, forge object ownership, fabricate AI attestations).

**This is the most critical long-term cryptographic risk in the entire design.**

The architect's mitigation (Section 1.5) is sound:
- `StateCommitment` trait abstraction allows switching commitment scheme via governance.
- Binary Merkle tree + SNARK alternative is maintained as a tested fallback.
- `commitment_scheme_version` field enables graceful migration.

**Additional requirement from security-engineer:**

> **SECURITY FLAG S1-2 [security-engineer -> blockchain-architect]: Quantum Migration Drill**
>
> The binary Merkle tree + SNARK fallback MUST be tested in a production-equivalent environment (testnet) at least annually. This is not optional -- it is a security requirement. The migration path must be exercised, not just designed, to ensure it actually works when needed. Include in the annual security audit scope.
>
> **Priority: SECURITY-ELEVATED (long-term).**

---

### 5.3 Security Review of Each Existing Section

#### 5.3.1 Section 1 (Architecture): Security Review

**Overall assessment: SOUND with flags.**

The modular monolith architecture with clean layer separation is a security strength -- it limits the blast radius of vulnerabilities in any single layer. The server-independence mandate eliminates centralized attack targets.

**Security flags:**

> **SECURITY FLAG S1-1 [security-engineer -> blockchain-architect]: MEV and Transaction Privacy**
>
> Section 1 defines a priority fee market from genesis (correct) but does not include an encrypted mempool or commit-reveal scheme for transaction ordering. This leaves validators free to inspect pending transactions in the mempool and extract MEV (front-running, sandwich attacks).
>
> **Recommendation:** Design an optional encrypted mempool capability (threshold decryption by the validator set) as a Phase 2 feature. Transactions that opt into privacy are encrypted until the ordering is determined by the anchor commit, then decrypted for execution. This does not need to be in the launch version but MUST be in the roadmap.
>
> **Priority: SECURITY-ELEVATED.** MEV extraction is the single largest user-facing security concern in production blockchains (Ethereum MEV exceeds $1B extracted).

> **SECURITY FLAG S1-3 [security-engineer -> blockchain-architect]: Agent Account Runaway Risk**
>
> Section 1.2.3 defines AI Agent accounts with spending limits and operational constraints enforced at the VM level. The security concern: if a spending limit is set too high or if the constraint logic has a bug, an agent could drain its principal's funds autonomously.
>
> **Requirement:** Agent spending limits MUST be enforced at the consensus layer (not just the VM layer) as a hard cap per epoch. A transaction from an agent exceeding its epoch spending limit must be rejected at the transaction validation stage (Section 4.1.3, step 3), NOT at the VM execution stage. This ensures that a VM bug cannot bypass spending limits.
>
> **Priority: SECURITY-ELEVATED.**

**Structural strengths identified:**

1. Hybrid account+object model limits parallel execution attack surface (objects are independent, preventing cross-contamination).
2. Data availability sampling for light clients is a security-positive feature.
3. Privacy as selective disclosure (not full anonymity) is the correct security/compliance balance.
4. The separation of AI inference (off-chain) from verification (on-chain) is a critical security design -- it prevents untrusted AI code from executing inside the consensus-critical VM.

#### 5.3.2 Section 2 (Consensus): Synaptic Consensus Attack Resistance Review

**Overall assessment: SOUND. Well-designed BFT consensus with appropriate DAG-specific attack mitigations.**

**Security flags:**

> **SECURITY FLAG S2-1 [security-engineer -> consensus-engineer]: Uncertified DAG Equivocation Window**
>
> The uncertified DAG design (per Mysticeti) trades a wider equivocation detection window for lower latency. In a certified DAG, equivocation is impossible because blocks must gather 2f+1 acknowledgments before being referenced. In the uncertified DAG, equivocation is detected retroactively.
>
> **Concern:** During the detection window (1-2 rounds, 400-800ms), an equivocating validator's conflicting vertices may be referenced by different honest validators, creating a temporary DAG inconsistency. The anchor commit rule resolves this (only one branch is committed), but the inconsistency creates a window for information asymmetry exploitation.
>
> **Requirement:** The equivocation detection mechanism MUST be specified more precisely:
> 1. How quickly does an equivocation proof propagate to all validators?
> 2. What happens to honest validators that already referenced the equivocating vertex?
> 3. Is there a risk that the anchor proposer can selectively choose which equivocating vertex to include?
>
> These questions must be answered in a formal security analysis of SynBFT before mainnet.
>
> **Priority: SECURITY-ELEVATED.**

> **SECURITY FLAG S2-2 [security-engineer -> consensus-engineer]: VRF Implementation Specification**
>
> Section 2.8.2 specifies VRF-based anchor selection using Ed25519 (ECVRF-EDWARDS25519-SHA512-TAI per RFC 9381). The security of anchor election depends entirely on the VRF's unpredictability and unbiasability.
>
> **Requirements:**
> 1. The VRF implementation MUST be exactly RFC 9381 compliant. No custom modifications.
> 2. The anchor_seed construction (`BLAKE3(previous_anchor_hash || anchor_round_number)`) must be analyzed for bias: if an anchor proposer can choose NOT to publish their anchor (sacrificing rewards) to influence the next anchor_seed, the VRF is biasable. This is the "last revealer" problem.
> 3. **Recommendation:** Use a commit-reveal VRF or RANDAO-style randomness accumulation to prevent last-revealer bias. The current design with a single previous anchor hash as seed is susceptible.
>
> **Priority: SECURITY-ELEVATED.**

> **SECURITY FLAG S2-3 [security-engineer -> consensus-engineer]: MPRE Floating-Point Determinism**
>
> Section 2.3.2 states that MPRE handles cross-architecture floating-point non-determinism by quantizing to 16-bit fixed-point for comparison. This is pragmatic but requires precise specification:
> 1. What quantization scheme? Truncation? Rounding? Which rounding mode?
> 2. What tolerance for agreement? Exact match at 16-bit or within epsilon?
> 3. Is the quantization scheme specified in the InferenceRequest or is it global?
>
> Imprecise quantization can either cause false disagreements (honest validators slashed for legitimate floating-point differences) or allow attackers to produce outputs that are "close enough" to the correct result while being meaningfully different.
>
> **Requirement:** Publish a formal specification of the MPRE comparison protocol including quantization scheme, tolerance, and edge case handling before mainnet.
>
> **Priority: STANDARD (does not block architecture, but must be resolved before launch).**

**Strengths identified:**

1. PoUW isolation from primary consensus is a critical security design. Chain liveness does not depend on AI workloads.
2. Multi-metric PoUW quality evaluation (not speed-only) directly addresses the Bittensor failure mode.
3. 5% stake cap + diversity quota + cooldown rotation is a strong anti-centralization stack.
4. 21-day unbonding period exceeds the typical dispute resolution window, providing adequate slashing coverage.

#### 5.3.3 Section 4 (Nodes): Node Hardening Requirements

**Overall assessment: SOUND. Well-structured node type hierarchy with appropriate security considerations.**

**Security flags:**

> **SECURITY FLAG S4-1 [security-engineer -> node-engineer]: Browser Node Key Management**
>
> Section 4.3.3 stores key material in IndexedDB with encryption. Browser nodes MUST NOT be the primary wallet for high-value accounts. The browser environment is inherently less secure than native applications due to:
> 1. Malicious browser extensions can access IndexedDB.
> 2. Cross-site scripting (XSS) in the hosting page could access WASM memory.
> 3. Browser developer tools allow direct memory inspection.
>
> **Requirement:** The browser node SDK MUST:
> 1. Default to a "spending account" model with low balance limits.
> 2. Support hardware wallet integration (WebUSB/WebHID for Ledger/Trezor) for high-value transactions.
> 3. Display clear warnings when a user loads a large balance into a browser wallet.
> 4. Encrypt private keys in IndexedDB using a user-provided passphrase (not a derived key stored in the same IndexedDB).
>
> **Priority: SECURITY-ELEVATED.**

> **SECURITY FLAG S4-2 [security-engineer -> node-engineer]: redb Access Control**
>
> Section 4.1.1 defines detailed redb column family configuration but does not specify access control or encryption at rest.
>
> **Requirements:**
> 1. redb data directory MUST have restricted file permissions (0700 on Unix, equivalent on Windows).
> 2. Validators SHOULD enable encryption at rest for the redb data directory (OS-level or redb env encryption).
> 3. The WAL (write-ahead log) contains unencrypted transaction data -- if the node's disk is compromised, transaction data is exposed. For validators handling sensitive PoUW inference data, WAL encryption is recommended.
>
> **Priority: STANDARD (operational security, not protocol vulnerability).**

> **SECURITY FLAG S4-3 [security-engineer -> node-engineer]: Snapshot Sync Trust Verification**
>
> Section 4.1.5 Strategy 1 (Snapshot Sync) downloads state from peers and verifies the Verkle root against the finalized anchor. The trust model states: "trusts that the finalized chain is honest."
>
> **Concern:** The Verkle root verification is sound ONLY if the node has a trusted finality certificate. A node performing its first-ever sync must obtain a trusted finality certificate from somewhere. Section 2.7.3 mentions weak subjectivity checkpoints but the mechanism for distributing these to brand-new nodes is not specified.
>
> **Requirement:** Specify the weak subjectivity checkpoint distribution mechanism:
> 1. Embed a recent checkpoint in every release binary (updated with each release).
> 2. Provide a command-line flag for users to specify a trusted checkpoint from an independent source.
> 3. Support querying multiple independent sources for checkpoint cross-validation.
> 4. Document the maximum safe checkpoint age.
>
> **Priority: SECURITY-ELEVATED.** A node that accepts a false checkpoint accepts a false chain.

**Strengths identified:**

1. Separate PoUW mempool prevents AI request flooding from affecting regular transaction processing.
2. redb selection for light nodes (over sled) is the correct security-informed choice given sled's data corruption history.
3. Mobile node battery optimization design (batch verification, sync-on-wake) does not compromise security -- all verification is still performed.
4. The node upgrade path (browser -> light -> full -> validator) maintains security guarantees at each transition.

#### 5.3.4 Section 8 (P2P): Network Layer Security Review

**Overall assessment: WELL-DESIGNED. Comprehensive network security with defense-in-depth.**

**Security flags:**

> **SECURITY FLAG S8-1 [security-engineer -> p2p-network-engineer]: DHT Poisoning Resistance**
>
> Section 8.2.2 uses Kademlia with k=20 and adopts S/Kademlia's disjoint lookup paths. However, the DHT stores critical records (validator sets, relay providers, chain tips) that, if poisoned, could mislead nodes.
>
> **Additional requirements:**
> 1. All Genesis-specific DHT records (validator sets, relays, chain tips) MUST be signed by a quorum of validators (not just a single validator). A single validator's signature is insufficient because a malicious validator could advertise false records.
> 2. DHT record freshness: records MUST include a slot/round number and a signature. Nodes MUST reject records older than their latest known finality certificate.
> 3. Consider implementing DHT record validation hooks in libp2p-kad to reject unsigned or stale records at the DHT level (before they enter the routing table).
>
> **Priority: SECURITY-ELEVATED.**

> **SECURITY FLAG S8-2 [security-engineer -> p2p-network-engineer]: Relay Trust Model**
>
> Section 8.4.4 describes incentivized TURN relays. A relay in the data path can observe metadata (who is communicating, message sizes, timing) even though it cannot decrypt content.
>
> **Concern:** A malicious relay could:
> 1. Selectively delay or drop messages for specific validators (targeted censorship).
> 2. Correlate communication patterns to identify validator-to-validator relationships.
> 3. Collude with other entities to deanonymize validators behind NAT.
>
> **Mitigations already in design:** Multiple relay connections (3+), relay reputation, relay rotation.
>
> **Additional requirement:** Validators behind symmetric NAT SHOULD distribute their relay connections across 3+ different relay operators (not just 3 connections to the same operator). The peer diversity policy should apply to relay selection, not just peer selection.
>
> **Priority: STANDARD.**

**Strengths identified:**

1. Six-layer bootstrap strategy with no single point of failure is excellent.
2. Gossipsub peer scoring with validator prioritization (+50) ensures consensus messages get mesh priority.
3. Eclipse detection via finality certificate monitoring is a novel and effective defense.
4. Compact block relay (95% bandwidth reduction) reduces the attack surface for bandwidth-exhaustion DoS.
5. Encrypted-by-default with no plaintext mode eliminates entire classes of eavesdropping attacks.

---

### 5.4 AI Security Monitoring Design

Aztibase Network integrates AI-based monitoring at the protocol level for real-time threat detection. This design defines WHAT the AI monitors and HOW it responds. The ai-integration-engineer implements the model architecture; this section defines the security requirements.

#### 5.4.1 Transaction Anomaly Detection

**Purpose:** Detect abnormal transaction patterns that may indicate attacks, exploits, or economic manipulation.

```
TransactionAnomalyMonitor {
    // Inputs (per epoch window):
    - Transaction volume histogram (by sender, by contract, by type)
    - Fee distribution (outlier detection)
    - Value transfer graph (sudden large flows, circular flows)
    - New contract deployment rate
    - Cross-contract call depth distribution

    // Detection targets:
    - Flash loan patterns: large borrow + multiple DEX interactions + repay in single tx
    - Sandwich attacks: buy-target_tx-sell sequences from same sender within same vertex
    - Token drain: rapid balance decrease from a contract (possible exploit)
    - Governance attack: sudden stake concentration or voting power shift
    - Wash trading: circular value transfers between related addresses
    - Unusual gas consumption: transactions consuming abnormally high gas

    // Output:
    - Anomaly score per transaction (0.0 - 1.0)
    - Anomaly classification (flash_loan, sandwich, drain, governance, wash, gas_abuse)
    - Alert threshold: score > 0.8 triggers alert
    - Reporting: anomaly data published to genesis/security/alerts topic
}
```

#### 5.4.2 Consensus Behavior Monitoring

**Purpose:** Detect validator misbehavior that falls below the slashing threshold but indicates potential coordination or attack preparation.

```
ConsensusBehaviorMonitor {
    // Inputs (per epoch):
    - Vertex proposal timing per validator (mean, variance, outliers)
    - Parent reference patterns per validator (who they reference, who they skip)
    - VRF output distribution (statistical uniformity test)
    - Anchor commit participation rates
    - PoUW attestation quality scores over time
    - Validator set entry/exit patterns

    // Detection targets:
    - Coordinated timing: multiple validators consistently proposing at the same
      offset within the round (possible coordination signal)
    - Selective referencing: validator consistently avoiding references to
      specific other validators (below slashing threshold but suspicious)
    - VRF clustering: anchor selection outcomes deviating from expected
      distribution (possible VRF bias)
    - Stake concentration: gradual increase in stake controlled by
      correlated addresses
    - PoUW quality degradation: validator's inference quality declining
      over time (possible transition to gaming)

    // Output:
    - Validator behavior score per epoch (0.0 - 1.0, lower = more suspicious)
    - Coordination detection alerts (group of validators behaving correlatively)
    - Long-term trend analysis (gradual behavioral shifts)
    - Reporting: consensus anomaly data published to genesis/security/consensus topic
}
```

#### 5.4.3 Smart Contract Exploit Detection

**Purpose:** Detect potential smart contract exploits in real-time by monitoring execution patterns.

```
ContractExploitMonitor {
    // Inputs (per block/anchor commit):
    - Contract call traces (caller, callee, depth, value, gas used)
    - State changes per contract execution (balance diffs, storage writes)
    - Contract deployment bytecode analysis (known vulnerability patterns)
    - Historical contract interaction patterns (baseline behavior)

    // Detection targets:
    - Reentrancy: recursive call patterns with state modifications between calls
    - Price oracle manipulation: large trades immediately before oracle price reads
    - Unusual state changes: contract storage modifications outside normal patterns
    - Self-destruct after drain: contract destroyed after large value withdrawal
    - Proxy upgrade to malicious implementation
    - Access control anomalies: privileged functions called by non-privileged addresses

    // Output:
    - Exploit probability score per contract interaction (0.0 - 1.0)
    - Exploit type classification
    - Affected contract addresses and estimated value at risk
    - Reporting: contract anomaly data published to genesis/security/contracts topic
}
```

#### 5.4.4 Network Health Monitoring

**Purpose:** Detect network-level attacks and degradation using the metrics exposed by Section 8.13.

```
NetworkHealthMonitor {
    // Inputs (from NetworkHealthMetrics):
    - Peer count and diversity score over time
    - Gossip propagation latency histograms
    - Bandwidth utilization trends
    - NAT traversal success rates
    - DHT query latency and success rates
    - Eclipse alarm trigger frequency
    - Invalid message counts

    // Detection targets:
    - Eclipse attack in progress: dropping peer diversity + stale finality certs
    - DDoS: spike in invalid messages + rate limiting triggers
    - Network partition: bimodal peer connectivity + anchor commit delays
    - Gossip degradation: increasing propagation latency beyond normal variance
    - DHT poisoning: increasing DHT query failures or inconsistent record responses
    - Sybil influx: rapid increase in new peer connections from similar subnets

    // Output:
    - Network health score (0.0 - 1.0)
    - Attack classification and confidence
    - Recommended response (alert/investigate/auto-respond)
    - Reporting: network health data published to genesis/security/network topic
}
```

#### 5.4.5 Threat Response Model

**When AI detects a threat, the response follows a graduated model:**

| Confidence Level | Response | Human Involvement |
|-----------------|----------|-------------------|
| **Low (0.5-0.7)** | Log anomaly. Increment monitoring frequency for the affected entity. No automatic action. | None (visible in dashboard). |
| **Medium (0.7-0.85)** | Publish alert to `genesis/security/alerts` topic. Notify validator operators via the monitoring API. Increase monitoring resolution. | Validator operators review alert. No protocol-level action. |
| **High (0.85-0.95)** | Publish high-priority alert. Auto-increase rate limiting for suspected attacker addresses. Auto-trigger eclipse detection for network anomalies. Flag transactions from suspicious addresses for manual review by block explorers. | Validator operators expected to investigate. Governance can vote on response. |
| **Critical (>0.95)** | Publish emergency alert. For consensus attacks: auto-raise evidence to slashing mechanism if equivocation proof exists. For contract exploits: publish exploit warning on-chain (contracts can subscribe). For network attacks: auto-activate emergency mode (restrict to known peers). | Validators must acknowledge. Governance emergency proposal if needed. |

**Critical design principle: AI NEVER autonomously slashes, freezes funds, or reverts transactions.** The AI monitoring system is an advisory and detection layer. Slashing is triggered only by cryptographic proof of misbehavior (equivocation proofs, invalid attestations), not by AI classification alone. This prevents false positives from the AI system causing financial harm.

The only automatic responses are defensive (increase rate limiting, restrict to known peers, raise evidence to slashing mechanism that then independently verifies). The AI does not make economic decisions.

---

### 5.5 Quantum Readiness Assessment

#### 5.5.1 Current Quantum Threat Assessment

| Component | Quantum Vulnerability | Impact if Broken | Timeline Estimate |
|-----------|---------------------|------------------|------------------|
| Ed25519 signatures | Shor's algorithm breaks ECDLP | Attacker can forge signatures, steal funds from any address | 10-15 years (NIST estimate) |
| BLS12-381 signatures | Shor's algorithm breaks pairing assumption | Attacker can forge finality certificates | 10-15 years |
| BLAKE3 hashing | Grover's algorithm reduces security to 128-bit (already the effective level) | Minimal impact -- 128-bit preimage resistance is sufficient even with Grover | Not a concern |
| Verkle tree (IPA/KZG) | Shor's algorithm breaks discrete log | Attacker can forge state proofs (claim any balance, forge ownership) | 10-15 years |
| Noise/TLS encryption | Shor's breaks DH key exchange | Past traffic can be decrypted (harvest-now-decrypt-later) | 10-15 years for decryption; harvesting is possible NOW |
| WASM execution | Not quantum-related | N/A | N/A |

#### 5.5.2 Harvest-Now-Decrypt-Later (HNDL) Risk

**Current risk level: MEDIUM.**

Nation-state actors may be recording encrypted P2P traffic today for future decryption when quantum computers become available. For Aztibase Network, this means:

- Transaction content (sender, receiver, amount) encrypted in transit could be decrypted later.
- For public transactions, this is not a concern (they are public on-chain anyway).
- For shielded/privacy transactions, HNDL is a real concern: privacy guarantees could retroactively fail.

**Mitigation recommendation:**
- Implement hybrid key exchange for privacy-critical connections: Noise XX with an additional post-quantum KEM (e.g., ML-KEM/Kyber, NIST-standardized). This provides quantum resistance for key exchange while maintaining compatibility.
- This is a Phase 2 recommendation, not launch-blocking.

#### 5.5.3 Post-Quantum Signature Migration Plan

**Phase 1 (Launch - Year 2): Preparation**
- All signature verification code uses a `SignatureScheme` trait abstraction.
- The block header includes a `signature_scheme_version` field (initially 1 = Ed25519, 2 = BLS12-381).
- Monitor NIST Post-Quantum Cryptography (PQC) standardization: ML-DSA (Dilithium), SLH-DSA (SPHINCS+), FN-DSA (Falcon) are the finalists.
- Implement PQC signature verification in a test branch. Do NOT deploy yet -- PQC standards are still maturing.

**Phase 2 (Year 2-3): Dual Signature Period**
- Introduce PQC signature support alongside classical signatures.
- Accounts can optionally register a PQC public key in addition to their Ed25519 key.
- Transactions can include both classical and PQC signatures (dual-signed).
- Validators begin registering PQC keys for BLS finality certificate fallback.
- State commitment migration from Verkle (IPA/KZG) to binary Merkle tree + lattice-based commitment begins on testnet.

**Phase 3 (Year 3-5): Mandatory Migration**
- Governance vote to set a deadline for PQC migration.
- After the deadline, all new accounts must have PQC keys.
- Classical-only signatures are still accepted for existing accounts (backward compatibility) but deprecated.
- State commitment switched to quantum-safe scheme via governance upgrade.

**Phase 4 (Year 5+): Classical Deprecation**
- Classical signatures deprecated and eventually rejected.
- Full quantum resistance achieved.

**PQC Signature Candidates (March 2026 NIST status):**

| Algorithm | Type | Signature Size | Public Key Size | Verification Speed | Status |
|-----------|------|---------------|----------------|-------------------|--------|
| ML-DSA (Dilithium-2) | Lattice | ~2.4 KB | ~1.3 KB | Fast | NIST FIPS 204 (finalized) |
| SLH-DSA (SPHINCS+-128s) | Hash-based | ~7.9 KB | 32 bytes | Slower | NIST FIPS 205 (finalized) |
| FN-DSA (Falcon-512) | Lattice (NTRU) | ~666 bytes | ~897 bytes | Fast | NIST draft (finalizing) |

**Recommendation:** FN-DSA (Falcon-512) for transaction signatures due to smallest signature size (~666 bytes vs. ~2.4 KB for ML-DSA). ML-DSA for validator/BLS replacement due to faster verification and implementation maturity. Final selection deferred to Phase 2 when standards are fully finalized and implementation libraries are audited.

**Impact on light/browser/mobile nodes:** PQC signatures are larger (666 bytes to 7.9 KB vs. 64 bytes for Ed25519). This increases bandwidth for finality certificates and state proofs. Compact representation and batch verification techniques for PQC are active research areas and should be tracked.

#### 5.5.4 Verkle-to-Post-Quantum State Commitment Migration

The architect's design (Section 1.5) provides the migration framework via the `StateCommitment` trait. The security-engineer's requirements:

1. The binary Merkle tree + SNARK fallback uses a SNARK-friendly hash function (e.g., Poseidon or Rescue) for the Merkle tree and a post-quantum-secure outer proof system.
2. Alternatively, a hash-based commitment scheme (binary Merkle tree with SLH-DSA-style hash constructions) provides quantum safety without SNARKs, at the cost of larger proofs.
3. The migration MUST be exercised on testnet annually (SECURITY FLAG S1-2).
4. Proof sizes will increase (from ~500 bytes Verkle to ~2-5 KB Merkle, or ~10-20 KB with PQ-safe SNARKs). Light client bandwidth budgets must be recalculated.

---

### 5.6 Key Management

#### 5.6.1 Key Types and Hierarchy

```
Aztibase Network Key Hierarchy:

Master Seed (BIP-39 mnemonic or raw entropy)
  |
  +-- Account Key (Ed25519)
  |     Purpose: transaction signing, identity
  |     Derivation: SLIP-0010 / BIP-32 adapted for Ed25519
  |     Path: m/44'/GEN_COIN_TYPE'/account'/0'/address_index'
  |
  +-- Validator Consensus Key (Ed25519)
  |     Purpose: DAG vertex signing, VRF computation
  |     Derivation: separate from account key
  |     Path: m/44'/GEN_COIN_TYPE'/validator'/0'/0'
  |
  +-- Validator BLS Key (BLS12-381)
  |     Purpose: aggregate finality certificate signing
  |     Derivation: EIP-2333 (Ethereum-compatible BLS key derivation)
  |     Path: m/12381/GEN_COIN_TYPE/0/0
  |
  +-- Encryption Key (X25519)
        Purpose: Noise protocol static key, P2P identity
        Derivation: from Ed25519 key via birational map
```

#### 5.6.2 Wallet Security Standards

**End-user wallets (browser, mobile, desktop):**

| Security Level | Implementation | Use Case |
|---------------|----------------|----------|
| **Basic** | Software wallet, encrypted key file, passphrase-derived key encryption (Argon2id with >=256MB memory, >=3 iterations) | Low-value accounts, testing |
| **Standard** | Software wallet + optional 2FA (TOTP) for transaction approval, encrypted key in OS keychain (Android Keystore / iOS Keychain / GNOME Keyring / macOS Keychain) | Regular users |
| **High-security** | Hardware wallet (Ledger, Trezor) via USB/NFC. Browser nodes via WebUSB/WebHID. Private keys never leave the hardware device. | High-value accounts |
| **Maximum** | Multi-signature (M-of-N) account requiring multiple hardware wallets for transaction approval. | Institutional, treasury |

**Key derivation requirements:**
- BIP-39 mnemonic (12 or 24 words) as the human-readable backup format.
- Argon2id for passphrase-based key encryption (NOT PBKDF2, NOT bcrypt -- Argon2id is the current NIST recommendation for password-based key derivation, providing memory-hardness against GPU/ASIC attacks).
- Minimum 256MB memory parameter for Argon2id (prevents commodity GPU brute-forcing).

#### 5.6.3 Validator Key Security (HSM Recommendations)

**Minimum (consumer validators):**
- Ed25519 and BLS keys stored in encrypted files on the validator's disk.
- File permissions restricted (0600 on Unix).
- Passphrase required at node startup (not stored on disk).
- Validator signs transactions and vertices in-process using in-memory keys.

**Recommended (professional validators):**
- HSM (Hardware Security Module) for key storage and signing.
- Supported HSMs via PKCS#11 interface (as noted in Section 4.5.3):
  - YubiHSM 2 (~$650): Supports Ed25519 natively. BLS12-381 may require custom firmware or software signing with HSM-protected seed.
  - Thales Luna: Enterprise-grade. Supports custom algorithms.
  - AWS CloudHSM / Azure Dedicated HSM: Cloud-based HSM for cloud-hosted validators.
- Key ceremony for BLS key generation: use threshold BLS key generation (DKG) if the validator is operated by multiple parties.
- HSM-based signing adds ~1-5ms latency per signature, which is within the 400ms round budget.

**Mandatory for all validators:**
- BLS proof-of-possession registration at validator onboarding (Section 5.2.3).
- Key rotation capability: validators MUST be able to rotate their consensus keys (Ed25519 and BLS) via an on-chain key rotation transaction signed by the old key.
- Key rotation does not require unstaking or re-entering the validator set.
- Recommended rotation interval: every 6-12 months or immediately upon suspected compromise.

#### 5.6.4 Key Recovery and Social Recovery

- BIP-39 mnemonic backup is the primary recovery mechanism.
- Social recovery (Shamir's Secret Sharing or smart contract-based recovery) is recommended as an SDK-level feature, not a protocol-level primitive.
- Protocol-level account recovery: a time-locked recovery address can be registered for each account. If the primary key is lost, the recovery address can claim the account after a delay period (e.g., 30 days), during which the original key can cancel the recovery (preventing unauthorized recovery attempts).

---

### 5.7 Stack Security Review

Every major dependency in the stack is evaluated for known CVEs, security track record, and supply chain risk.

| Component | Version Target | License | Known CVEs (as of March 2026) | Security Assessment | Decision |
|-----------|---------------|---------|------------------------------|---------------------|----------|
| **Rust (language/compiler)** | Latest stable | MIT/Apache 2.0 | Periodic CVEs in std library (promptly patched). No fundamental design flaws. | Rust's memory safety model eliminates entire classes of vulnerabilities (buffer overflows, use-after-free, data races). | **APPROVED** |
| **tokio** | 1.x | MIT | CVE-2023-22466 (net module, patched). Generally strong security record. | Production-proven async runtime. Well-maintained. | **APPROVED** |
| **rust-libp2p** | Latest | MIT/Apache 2.0 | Periodic CVEs in specific sub-crates (noise, yamux). All promptly patched. | Maintenance concern for go/js implementations does NOT affect Rust implementation. Active maintainers. | **APPROVED** with monitoring |
| **libp2p-webrtc** | Latest | MIT/Apache 2.0 | Limited CVE history due to relative newness. | **MEDIUM-HIGH RISK** (per Section 8.10.1). Limited production deployment. | **CONDITIONALLY APPROVED** -- requires dedicated security testing before mainnet browser node launch |
| **redb** | Latest stable (Apache 2.0 option) | Dual GPL2/Apache 2.0 | Periodic memory safety CVEs (C++ codebase). Use rust-rocksdb bindings with safety wrappers. | Proven at massive scale. C++ dependency is the primary security concern (memory safety not guaranteed). | **APPROVED** with requirement: pin specific audited version; do not auto-update without testing |
| **redb** | 1.x | MIT/Apache 2.0 | No known CVEs. Young project with limited security audit history. | Pure Rust (memory-safe). ACID with checksums. Used in Bitcoin ordinals tooling. | **APPROVED** for light/mobile nodes (low-value data) |
| **wasmtime** | Latest stable | Apache 2.0 + LLVM exception | Multiple historical CVEs (sandbox escapes patched). Bytecode Alliance actively maintains. CVE-2023-41880, CVE-2024-30264 (both patched). | **CRITICAL DEPENDENCY.** Wasmtime's sandbox is the security boundary between user-submitted contract code and the host system. | **APPROVED** with requirements: 1) Subscribe to Bytecode Alliance security advisories. 2) Update within 48 hours of any sandbox-related CVE. 3) Run wasmtime with OS-level sandboxing (seccomp-bpf on Linux, pledge/unveil on OpenBSD) as defense-in-depth. 4) Conduct annual third-party security audit of our wasmtime integration. |
| **ed25519-dalek** | Latest stable | BSD-3-Clause | CVE-2022-3225 (timing side-channel, patched in 2.x). | Well-audited. Must use strict verification mode (SECURITY FLAG S0-1). | **APPROVED** with strict verification requirement |
| **blake3** | Latest stable | CC0/Apache 2.0 | No known CVEs. | Pure Rust. Minimal attack surface. | **APPROVED** |
| **blst** (BLS12-381) | Latest stable | Apache 2.0 | No known CVEs. Audited by NCC Group (2020) and others. | C implementation with Rust bindings. Used by Ethereum beacon chain (production since Dec 2020). Memory-safe operations via Rust wrapper. | **APPROVED** |
| **tract** (AI runtime) | Latest stable | MIT/Apache 2.0 | No known CVEs. | Pure Rust. Deterministic inference. Smaller attack surface than ONNX Runtime (C++). | **APPROVED** |
| **candle** (AI runtime) | Latest stable | MIT/Apache 2.0 | No known CVEs. Relatively young project. | Pure Rust. GPU operations via CUDA bindings (C interface). | **APPROVED** with monitoring for GPU-related CVEs |
| **ONNX Runtime** | Latest stable | MIT | Multiple historical CVEs (C++ codebase). CVE-2024-27099, CVE-2024-27100 (patched). | C++ dependency with larger attack surface. Optional backend only. | **CONDITIONALLY APPROVED** -- use only on validator nodes where performance justifies the risk. Not for light/browser/mobile nodes (which don't need inference). |
| **serde + postcard** | Latest stable | MIT/Apache 2.0 | serde: no significant CVEs. postcard: no known CVEs. | Pure Rust serialization (postcard for both wire and internal per ADR-024). Must validate deserialized input sizes to prevent memory exhaustion from malformed data. | **APPROVED** with input validation requirement |

**Supply chain security requirements:**

1. **Cargo.lock pinning:** All dependency versions MUST be pinned in Cargo.lock. No floating version ranges in production builds.
2. **cargo-audit:** Run `cargo audit` in CI/CD pipeline. Fail the build on any advisory with severity >= HIGH.
3. **cargo-vet:** Implement supply chain auditing via `cargo vet`. Require audit attestations for all direct dependencies.
4. **Reproducible builds:** Production binaries MUST be reproducibly buildable. This allows independent parties to verify that the distributed binary matches the source code.
5. **Dependency review process:** New dependency additions require security-engineer review. No new dependencies with copyleft licenses (per LEGAL_LANDSCAPE.md Section 7.5).

---

### 5.8 Security Flags Summary

All security flags raised in this section, consolidated for tracking:

| Flag ID | Target | Description | Priority |
|---------|--------|-------------|----------|
| **S0-1** | blockchain-architect | Ed25519 strict verification mode mandatory | SECURITY-ELEVATED |
| **S1-1** | blockchain-architect | MEV mitigation: encrypted mempool in roadmap | SECURITY-ELEVATED |
| **S1-2** | blockchain-architect | Quantum migration drill (annual testnet exercise) | SECURITY-ELEVATED (long-term) |
| **S1-3** | blockchain-architect | Agent spending limits enforced at consensus layer, not just VM | SECURITY-ELEVATED |
| **S2-1** | consensus-engineer | Uncertified DAG equivocation window: formal analysis required | SECURITY-ELEVATED |
| **S2-2** | consensus-engineer | VRF last-revealer bias: consider commit-reveal or RANDAO | SECURITY-ELEVATED |
| **S2-3** | consensus-engineer | MPRE quantization scheme: formal specification required | STANDARD |
| **S3-1** | tokenomics-engineer | Governance must be flash-loan resistant (stake lock-up for voting) | SECURITY-ELEVATED |
| **S4-1** | node-engineer | Browser node key management: spending limits, hardware wallet support, warnings | SECURITY-ELEVATED |
| **S4-2** | node-engineer | redb access control and encryption at rest | STANDARD |
| **S4-3** | node-engineer | Weak subjectivity checkpoint distribution mechanism specification | SECURITY-ELEVATED |
| **S8-1** | p2p-network-engineer | DHT record signing by validator quorum, freshness validation | SECURITY-ELEVATED |
| **S8-2** | p2p-network-engineer | Relay trust model: diversity requirement for relay selection | STANDARD |

**Total: 13 security flags. 9 SECURITY-ELEVATED (must be addressed before proceeding). 4 STANDARD (should be addressed before mainnet).**

---

### 5.9 Annual Security Audit Scope

The following MUST be included in annual third-party security audits:

1. **Consensus protocol formal verification:** Safety and liveness proofs for SynBFT under the stated adversary model.
2. **wasmtime integration review:** Custom host functions, gas metering, sandbox boundaries.
3. **Cryptographic implementation review:** Ed25519 (strict mode), BLS12-381 (PoP, subgroup checks), BLAKE3 usage, VRF implementation (RFC 9381 compliance).
4. **P2P protocol security:** Gossipsub configuration, peer scoring, eclipse detection, DHT poisoning resistance.
5. **Smart contract SDK security:** Reentrancy protection, overflow handling, access control patterns in the provided libraries.
6. **AI monitoring system accuracy:** False positive/negative rates, response model appropriateness.
7. **Key management review:** Key derivation, storage, rotation, and recovery mechanisms.
8. **Post-quantum migration readiness:** Annual exercise of Verkle-to-Merkle migration on testnet (SECURITY FLAG S1-2).
9. **Dependency audit:** CVE review for all dependencies, supply chain attestation verification.

---

**Stack challenges raised:**
- STACK CHALLENGE ENDORSEMENT: BLS12-381 signatures (endorsing consensus-engineer's challenge from Section 2.9.3). BLS12-381 is APPROVED with mandatory proof-of-possession, domain separation, and subgroup checking requirements.

**Conflicts raised:**
- 9 SECURITY-ELEVATED flags raised across Sections 0, 1, 2, 3, 4, and 8. All must be addressed by the respective skill owners before proceeding to Phase 5. See Section 5.8 for the complete list.

> CONFLICT [security-engineer -> blockchain-architect]: Agent spending limit enforcement layer
> Priority: SECURITY-ELEVATED
> Agent spending limits in Section 1.2.3 are described as "enforced at the VM level." They MUST be enforced at the consensus/transaction-validation layer (pre-execution) as defense-in-depth. A VM bug should not be able to bypass spending limits.

> CONFLICT [security-engineer -> consensus-engineer]: VRF last-revealer bias
> Priority: SECURITY-ELEVATED
> The current VRF seed construction (BLAKE3(previous_anchor_hash || round)) is susceptible to last-revealer bias. An anchor proposer can choose not to publish to influence the next seed. Recommend commit-reveal VRF or RANDAO-style randomness.

**Sign-off:** security-engineer

## SECTION 6: AI INTEGRATION

---
**Contributed by:** ai-integration-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete, especially Sections 4.1-4.5 AI-native survey, 2.2-2.3 PoUW/AI-assisted consensus), Section 0 stack rulings (reviewed, including AI Runtime ruling: tract primary, candle secondary, ONNX optional), Section 1 architecture (reviewed, especially 1.6 AI-native architecture with 5 AI primitives), Section 2 consensus (reviewed, especially 2.3 PoUW integration, 2.6 AI hooks, PoUW tiers), Section 4 node architecture (reviewed, all five node types, PoUW validator tiers, WASM compatibility assessment)

---

### AI INTEGRATION DESIGN

Aztibase Network treats AI as a first-class protocol citizen. This section defines the complete implementation design for AI integration across all protocol layers — from consensus-level anomaly detection to the AI compute marketplace. Every design decision respects two hard constraints: (1) AI must never be required for basic chain operation (graceful degradation), and (2) AI features must not compromise decentralization by demanding datacenter hardware for participation.

---

### 6.1 Layer 1 — Protocol-Level AI

Protocol-level AI operates at the network and data layer. These are lightweight models that run on every full node and validator, providing security intelligence as a protocol service. They are advisory — they flag anomalies for human or automated review but do not autonomously reject transactions or blocks.

#### 6.1.1 Transaction Anomaly Detection

**Purpose:** Identify suspicious transactions before inclusion in DAG vertices. Flagged transactions are still included (censorship resistance is paramount) but are annotated with an anomaly score that downstream systems (wallets, dApps, exchanges) can use for risk assessment.

**Model Architecture: Ensemble of Lightweight Models**

The transaction anomaly detector is not a single model but an ensemble of three complementary models, each catching different attack patterns:

**Model A: Gradient Boosted Decision Tree (GBDT) — Structural Anomalies**

```
Runtime:          tract (ONNX format)
Architecture:     LightGBM-style GBDT, 200 trees, max depth 8
Parameters:       ~50,000 (< 1 MB serialized)
Inference time:   < 0.1ms per transaction (CPU)
Input features:   (22 features per transaction)
  - tx_value_log:          log10(transaction value in native token)
  - tx_fee_ratio:          fee / avg_fee_last_1000_blocks
  - sender_age_epochs:     epochs since sender account creation
  - sender_tx_count:       total transactions sent by this account
  - sender_balance_ratio:  tx_value / sender_balance
  - recipient_age_epochs:  epochs since recipient creation (0 if new)
  - recipient_is_contract: boolean
  - contract_call_depth:   estimated call depth (0 for transfers)
  - gas_limit_ratio:       gas_limit / avg_gas_limit_last_1000
  - nonce_gap:             tx_nonce - expected_nonce (normally 0)
  - input_data_size:       bytes of calldata
  - time_since_last_tx:    seconds since sender's last transaction
  - hourly_tx_count:       sender's tx count in last 3600 seconds
  - daily_value_sent:      total value sent by sender in last 24h
  - unique_recipients_24h: distinct recipients from sender in 24h
  - is_token_transfer:     boolean (detected from calldata pattern)
  - token_approval_flag:   boolean (detected ERC-20 approve pattern)
  - is_ai_inference_req:   boolean (InferenceRequest transaction type)
  - is_agent_tx:           boolean (sent by AIAgent account type)
  - agent_spend_ratio:     if agent, amount / remaining_epoch_limit
  - sender_is_validator:   boolean
  - mempool_pressure:      current mempool fullness (0.0 - 1.0)

Output:           anomaly_score: f32 (0.0 = normal, 1.0 = highly anomalous)
Threshold:        > 0.7 triggers ANOMALY flag on transaction receipt
```

**Model B: Isolation Forest — Statistical Outliers**

```
Runtime:          Custom Rust implementation (no external dependency)
Architecture:     Isolation Forest, 100 trees, subsample 256
Parameters:       ~200 KB serialized (tree structure)
Inference time:   < 0.05ms per transaction (CPU)
Input features:   Same 22 features as Model A (shared feature extraction)
Output:           outlier_score: f32 (0.0 = inlier, 1.0 = outlier)
Threshold:        > 0.8 triggers OUTLIER flag
```

**Model C: Temporal Pattern Detector — Sequence Anomalies**

```
Runtime:          tract (ONNX format)
Architecture:     1D-CNN with 3 convolutional layers + global average pooling
                  Conv1D(22->64, kernel=3) -> ReLU -> Conv1D(64->32, kernel=3) -> ReLU -> Conv1D(32->16, kernel=3) -> GAP -> Dense(16->1) -> Sigmoid
Parameters:       ~15,000 (< 200 KB)
Inference time:   < 0.2ms per transaction (CPU)
Input:            Sliding window of last 8 transactions from the same sender
                  (8 x 22 feature matrix)
Output:           sequence_anomaly_score: f32 (0.0 = normal pattern, 1.0 = anomalous sequence)
Threshold:        > 0.75 triggers SEQUENCE_ANOMALY flag
```

**Ensemble Decision:**

```
final_anomaly_score = 0.50 * model_a_score + 0.25 * model_b_score + 0.25 * model_c_score

if final_anomaly_score > 0.7:
    transaction_receipt.flags |= ANOMALY_DETECTED
    transaction_receipt.anomaly_detail = {
        structural: model_a_score,
        statistical: model_b_score,
        temporal: model_c_score,
    }
```

**Why this ensemble and not a single neural network:**

1. GBDTs excel at tabular data with mixed feature types. They are also fully deterministic across platforms (no floating-point divergence for tree traversal — the decision path is integer comparison only).
2. Isolation Forests catch novel attack patterns that supervised models miss (they detect statistical anomalies without needing labeled attack examples).
3. The temporal CNN catches sequence-based attacks (rapid drain patterns, bot-like trading sequences, gas price manipulation patterns) that single-transaction models miss.
4. The ensemble is more robust to adversarial evasion than any single model — an attacker must simultaneously fool three different detection paradigms.

**What this does NOT do:** The anomaly detector does NOT reject transactions. It annotates. Censorship resistance is inviolable. A transaction with anomaly_score = 1.0 is still included in the DAG and executed. The score is metadata in the receipt — useful for wallets showing warnings, exchanges requiring additional confirmation, and the peer reputation system (Section 6.3.2).

#### 6.1.2 Block-Level Pattern Analysis

**Purpose:** Detect macro-level attack patterns that are invisible at the individual transaction level: MEV extraction patterns, coordinated wash trading, validator collusion signals, and sudden liquidity drains.

**Model Architecture: Sliding Window Aggregator + Decision Tree**

```
Runtime:          Custom Rust (aggregation) + tract (ONNX GBDT, 100 trees, depth 6)
Parameters:       ~25,000 (< 500 KB)
Frequency:        Once per anchor commit (every ~1.6 seconds, not every round)
Input features:   (18 features, aggregated over the committed batch of vertices)
  - tx_count:              total transactions in committed batch
  - avg_anomaly_score:     mean transaction anomaly score in batch
  - max_anomaly_score:     maximum transaction anomaly score in batch
  - anomaly_count:         count of transactions with anomaly_score > 0.7
  - unique_senders:        distinct sender addresses
  - unique_recipients:     distinct recipient addresses
  - value_concentration:   Gini coefficient of transaction values
  - fee_stddev:            standard deviation of fees in batch
  - contract_call_ratio:   fraction of transactions that are contract calls
  - new_account_count:     accounts created in this batch
  - ai_request_count:      InferenceRequest transactions in batch
  - agent_tx_ratio:        fraction of transactions from AIAgent accounts
  - circular_flow_score:   detected circular value flows (A->B->C->A patterns)
  - top_sender_dominance:  fraction of txs from the single most active sender
  - avg_gas_utilization:   mean gas_used / gas_limit across batch
  - validator_self_tx:     count of txs where sender is the anchor proposer
  - time_since_prev_anchor: actual wall-clock time since previous anchor (should be ~1.6s)
  - vertex_count:          number of DAG vertices in this committed batch

Output:           block_risk_level: enum { NORMAL, ELEVATED, HIGH, CRITICAL }
                  block_risk_details: Vec<RiskFactor>
```

**Risk level actions:**

| Level | Trigger | Action |
|-------|---------|--------|
| NORMAL | risk_score < 0.3 | No action. |
| ELEVATED | 0.3 <= risk_score < 0.6 | Log to monitoring. Include risk annotation in block receipt. |
| HIGH | 0.6 <= risk_score < 0.85 | Alert network monitoring dashboards. Increase MPRE redundancy for inference requests in next epoch (from 3 to 5 executors). |
| CRITICAL | risk_score >= 0.85 | Broadcast network-wide alert via gossip. Trigger automated investigation: validators independently re-verify the flagged batch transactions. Flag for governance review. |

**Crucially:** Even CRITICAL risk level does NOT halt the chain or revert blocks. Finality is absolute (Section 2.5). Block-level analysis is forensic and alerting, not punitive at the consensus layer.

#### 6.1.3 AI-Assisted Consensus Validation

Per Section 2.6.1, AI does NOT validate transactions or determine block validity in SynBFT. This is a firm architectural boundary that this section respects and reinforces.

AI assists consensus in exactly three bounded ways:

1. **PoUW reputation feeds into anchor selection** (Section 2.2.3) — PoUWReputation is computed from AI quality metrics. The ai-integration-engineer defines how those quality metrics are calculated (see Section 6.6.3), but the consensus weight is capped at 15% per the consensus-engineer's design.

2. **InferenceAttestation verification during vertex validation** (Section 2.6.2) — When a full node receives a vertex containing InferenceAttestations, it must verify the attestation proofs. The AI integration layer provides the verification functions:
   - MPRE: Compare result hashes at quantized precision (16-bit fixed-point). Agreement check is a simple hash comparison — no AI involved in verification itself.
   - TEE: Verify the TEE attestation certificate chain. This is a cryptographic verification, not AI.
   - ZK: Verify the ZK proof. This is a mathematical verification (~1ms for small proofs per Section 2.3.2).

3. **Anomaly scores as soft signals** — The transaction and block-level anomaly scores (Sections 6.1.1-6.1.2) are available to validators as advisory information. A validator MAY choose to deprioritize highly anomalous transactions in their vertex (placing them lower in the transaction batch), but MUST NOT exclude them solely based on anomaly score. This is enforced socially (community norms) and monitored (a validator that systematically excludes high-anomaly-score transactions will accumulate censorship score per Section 2.7.5).

**What AI explicitly does NOT do in consensus:**
- Does not vote on block validity
- Does not determine transaction ordering
- Does not influence finality decisions
- Does not modify slashing conditions
- Does not autonomously trigger any consensus state change

This separation ensures that consensus remains transparent, deterministic, and formally verifiable (Section 2.11) regardless of AI model behavior.

---

### 6.2 Layer 2 — Smart Contract AI

Smart contract AI provides automated security analysis for contracts deployed on Aztibase Network. These features operate at the execution layer (Layer 3 of the architecture per Section 1.3) and interact with the WASM and EVM runtimes.

#### 6.2.1 Automated Pre-Deployment Contract Auditing

**When:** Every contract deployment transaction triggers the audit pipeline. The audit result is included in the deployment receipt.

**Architecture: Static Analysis + ML Classification**

The audit pipeline has two stages:

**Stage 1: Deterministic Static Analysis (no AI)**

```
Language:         Rust (compiled into the execution engine)
Trigger:          On every CREATE/CREATE2 opcode (EVM) or contract instantiation (WASM)
Checks:
  - Reentrancy patterns: detect external calls followed by state changes
    (SWC-107, the cause of the DAO hack)
  - Integer overflow/underflow: detect unchecked arithmetic in Solidity < 0.8
    (SWC-101)
  - Unprotected selfdestruct/delegatecall (SWC-106, SWC-112)
  - tx.origin authentication (SWC-115)
  - Unbounded loops (gas DoS risk)
  - Storage collision patterns (proxy contracts)
  - Uninitialized storage pointers
  - Missing access control on state-mutating functions

Output:           Vec<StaticAuditFinding { severity, location, pattern_id, description }>
Deterministic:    Yes — same bytecode always produces same findings
```

**Stage 2: ML Vulnerability Classification**

```
Runtime:          tract (ONNX format)
Architecture:     1D-CNN over bytecode + fully connected head
                  Input: contract bytecode (padded/truncated to 4096 bytes)
                  Conv1D(1->32, kernel=5, stride=2) -> ReLU -> Conv1D(32->64, kernel=5, stride=2) -> ReLU ->
                  GlobalAvgPool -> Dense(64->32) -> ReLU -> Dense(32->8) -> Sigmoid (multi-label)
Parameters:       ~120,000 (< 1.5 MB)
Inference time:   < 5ms per contract (CPU)
Input:            Raw contract bytecode as 1D byte sequence
Output:           8 vulnerability probability scores:
  - p_reentrancy:        probability of reentrancy vulnerability
  - p_overflow:          probability of arithmetic vulnerability
  - p_access_control:    probability of missing access control
  - p_oracle_manipulation: probability of price oracle manipulation risk
  - p_flash_loan_risk:   probability of flash loan attack surface
  - p_rugpull_pattern:   probability of owner-can-drain-funds pattern
  - p_proxy_risk:        probability of unsafe upgrade patterns
  - p_unusual_pattern:   probability of novel/unusual code pattern (catch-all)
```

**Audit Report (included in deployment receipt):**

```
ContractAuditReport {
    contract_address: Address,
    deployment_tx:    TxHash,
    static_findings:  Vec<StaticAuditFinding>,
    ml_risk_scores:   MLRiskScores,
    overall_risk:     RiskLevel (LOW / MEDIUM / HIGH / CRITICAL),
    audit_model_version: ModelVersion,
    timestamp:        u64,
}

Overall risk = max(
    highest_static_severity,
    if any ml_score > 0.8: CRITICAL,
    if any ml_score > 0.5: HIGH,
    if any ml_score > 0.3: MEDIUM,
    else: LOW
)
```

**The audit does NOT prevent deployment.** Any contract can be deployed on Aztibase Network regardless of audit results. The audit report is informational — attached to the deployment receipt and queryable on-chain. This design:
- Preserves permissionless deployment (censorship resistance)
- Provides automatic security intelligence that wallets and dApps can display
- Creates a public, immutable audit trail for every contract
- Gives DeFi protocols an automated first-pass security signal

#### 6.2.2 Runtime Exploit Detection

**Purpose:** Monitor executing contracts in real-time for exploit patterns that are invisible to static analysis.

**Architecture: Rule Engine + Lightweight Classifier**

```
Execution layer hook: After each internal message call during contract execution

Rule engine (deterministic, no AI):
  - Reentrancy detector: flag if contract C is re-entered while C's storage
    is in a modified-but-uncommitted state
  - Flash loan detector: flag if a single transaction borrows, manipulates
    price oracle, and repays within one execution
  - Abnormal gas pattern: flag if gas consumption for a function call deviates
    >3 standard deviations from its historical mean (tracked per function selector)
  - Balance drain detector: flag if >90% of a contract's native token balance
    is transferred in a single transaction to a previously unseen address

ML classifier (runs only when rules trigger):
  Runtime:        tract (ONNX GBDT, 50 trees, depth 5)
  Parameters:     ~10,000 (< 200 KB)
  Input features: 12 features from the triggering execution context
    - call_depth, value_transferred, gas_used_so_far, storage_writes_count,
      external_calls_count, is_delegatecall, caller_is_eoa, caller_age,
      target_contract_age, function_selector_frequency, flash_loan_in_tx,
      price_deviation_from_oracle
  Output:         exploit_probability: f32 (0.0 - 1.0)
  Inference time: < 0.1ms
```

**Response to detected exploits:**

Runtime exploit detection is advisory. It emits events that monitoring systems can subscribe to:

```
ExploitAlert {
    tx_hash:        TxHash,
    contract:       Address,
    alert_type:     REENTRANCY | FLASH_LOAN | GAS_ANOMALY | BALANCE_DRAIN | ML_FLAGGED,
    confidence:     f32,
    execution_step: u64,
    details:        String,
}
```

These alerts are emitted as protocol-level events (not smart contract events) and are gossiped to all subscribed nodes. Monitoring dashboards, automated circuit breakers in DeFi protocols, and wallet UIs can react to these alerts. The protocol itself does not halt or revert the transaction — that would violate deterministic execution guarantees.

**Future enhancement (governance-gated):** A protocol-level "emergency pause" for contracts that accumulate multiple HIGH-confidence exploit alerts in a short window. This requires governance approval to activate and is NOT included in the initial design due to the censorship resistance implications.

#### 6.2.3 Gas/Fee Optimization via AI-Assisted Resource Pricing

**Purpose:** Improve the accuracy of gas cost estimation for AI-related operations and dynamically adjust fee parameters based on network conditions.

**Architecture: Regression Model for Inference Cost Prediction**

```
Runtime:          tract (ONNX linear regression with polynomial features)
Parameters:       ~500 (< 10 KB)
Inference time:   < 0.01ms
Input features:
  - model_parameter_count: from ModelRegistry metadata
  - model_format:          ONNX / custom / quantized
  - input_size_bytes:      size of inference input
  - verification_method:   MPRE / TEE / ZK (encoded as one-hot)
  - current_pouw_load:     fraction of PoUW capacity in use
  - historical_compute_ms: rolling average compute time for this model

Output:           estimated_gas_units: u64
                  estimated_wall_time_ms: u32
```

This model predicts the gas cost of an InferenceRequest BEFORE it is submitted, allowing users and agents to set appropriate fee levels. The prediction feeds into the fee market:

- If estimated cost exceeds the user's max_fee, the request is rejected pre-submission (client-side).
- The base fee for inference requests adjusts dynamically: if PoUW capacity utilization exceeds 80%, the base fee increases exponentially (EIP-1559-style for inference).
- This creates a self-regulating market that prevents PoUW overload without centralized rate limiting.

---

### 6.3 Layer 3 — Network AI

Network-layer AI monitors the P2P health of Aztibase Network and protects against network-level attacks.

#### 6.3.1 P2P Health Monitoring

**Purpose:** Detect network degradation, partition events, and infrastructure attacks before they impact consensus.

**Architecture: Time-Series Anomaly Detector**

```
Runtime:          Custom Rust (Exponentially Weighted Moving Average + threshold)
No ML model:      This is deliberately simple. P2P health monitoring must be
                  maximally reliable and must not introduce AI failure modes
                  into network monitoring itself.

Metrics monitored (per node, locally):
  - peer_count:                current connected peers (5s interval)
  - vertex_propagation_delay:  time from vertex timestamp to local receipt (per vertex)
  - gossip_message_rate:       messages/sec on each gossip topic (10s window)
  - failed_connection_rate:    failed dials / total dials (1-minute window)
  - bandwidth_utilization:     inbound + outbound bytes/sec (10s window)
  - dht_query_latency:         response time for Kademlia queries (per query)
  - nat_traversal_success:     ICE success rate (1-hour window)

Alert thresholds (configurable):
  - peer_count < min_peers: CONNECTIVITY_ALERT
  - vertex_propagation_delay > 2 * round_time (800ms): PROPAGATION_ALERT
  - gossip_message_rate drops > 50% from 1-hour average: GOSSIP_DEGRADATION
  - failed_connection_rate > 30%: CONNECTION_FAILURE_ALERT
  - bandwidth_utilization > 90% of configured limit: BANDWIDTH_SATURATION
```

**Why no ML here:** Network health monitoring is the foundation upon which all other AI features depend. Adding ML complexity to this layer creates a circular dependency — if the network is degraded, can the ML model that detects network degradation still function? By using simple statistical thresholds (EWMA + fixed bounds), network health monitoring remains robust even under adversarial conditions.

#### 6.3.2 Peer Reputation Scoring

**Purpose:** Score peers based on their behavior to inform connection management, gossip mesh formation, and resource allocation decisions.

**Architecture: Multi-Factor Score with Decay**

```
Runtime:          Custom Rust (no external AI dependency)
Computation:      Per-peer, updated on every interaction

PeerReputationScore {
    // Behavior scores (0.0 = worst, 1.0 = best)
    gossip_quality:        f32,  // Valid messages / total messages received
    latency_consistency:   f32,  // Low variance in response times
    data_availability:     f32,  // Successful responses to data requests / total requests
    protocol_compliance:   f32,  // Correctly formatted messages / total messages
    anomaly_relay_rate:    f32,  // 1.0 - (anomalous txs relayed / total txs relayed)
                                 // Peers that disproportionately relay anomalous txs are penalized

    // Composite score
    composite: f32 = 0.30 * gossip_quality
             + 0.20 * latency_consistency
             + 0.25 * data_availability
             + 0.15 * protocol_compliance
             + 0.10 * anomaly_relay_rate

    // Time decay: composite decays toward 0.5 (neutral) with half-life of 24 hours
    // This ensures new peers start neutral and past behavior eventually ages out
}
```

**Usage of peer reputation:**
- Gossipsub mesh formation: prefer high-reputation peers as mesh neighbors.
- Connection eviction: when at max peers, evict the lowest-reputation peer.
- Data request routing: prefer high-reputation peers for Verkle proof requests, state sync chunks.
- Rate limiting: low-reputation peers receive lower rate limits for data requests.

**Why not a neural network for peer reputation:** Peer reputation scoring must be (a) interpretable — a node operator must be able to understand why a peer was scored low, (b) deterministic — given the same interaction history, the score must be the same, and (c) lightweight — computed on every interaction with every peer. A neural network provides none of these properties.

#### 6.3.3 DDoS and Spam Detection

**Purpose:** Detect and mitigate network-layer denial-of-service attacks.

**Architecture: Rate Limiting + Statistical Anomaly Detection**

```
Layer 1 defense (no AI, always active):
  - Per-IP rate limiting: max 100 connections/minute from a single IP
  - Per-PeerId rate limiting: max 50 messages/second per peer
  - Transaction spam filter: reject transactions below minimum base fee
  - Gossip deduplication: bloom filter rejects duplicate message IDs
  - Connection slot reservation: 20% of slots reserved for known-good peers
    (peers with reputation > 0.7 from Section 6.3.2)

Layer 2 defense (statistical, lightweight AI):
  Runtime:        Custom Rust (statistical models)
  Method:         Track message rate distribution across all peers.
                  A peer whose message rate exceeds mean + 3*stddev for 30+ seconds
                  is flagged as potential DDoS source.
  Action:         Flagged peers are temporarily throttled (rate limit reduced to 10%
                  of normal) for 5 minutes. Repeated flags within 1 hour trigger
                  a 1-hour disconnect.

Layer 3 defense (adaptive, ML-assisted):
  Runtime:        tract (ONNX GBDT, 50 trees, depth 4)
  Parameters:     ~8,000 (< 100 KB)
  Frequency:      Every 10 seconds (batch analysis)
  Input features: (8 features, aggregated per-peer over 10-second window)
    - message_rate, unique_message_ratio, avg_message_size,
      protocol_violation_count, connection_age, reconnection_count,
      message_type_entropy, peer_reputation_score
  Output:         ddos_probability: f32
  Threshold:      > 0.9 triggers HARD_DISCONNECT (peer banned for 1 hour)
                  > 0.7 triggers THROTTLE (reduced rate limits)
```

---

### 6.4 AI Runtime Decision (FINAL)

This section constitutes the FINAL AI runtime recommendation, exercising the authority granted in the SKILL definition to refine the architect's stack ruling.

#### 6.4.1 Runtime Evaluation Per Use Case

| Use Case | tract | candle | ONNX Runtime | Custom Rust | Recommendation |
|----------|-------|--------|-------------|-------------|----------------|
| **Tx anomaly detection** (GBDT, <1MB) | Excellent. Deterministic GBDT inference. <0.1ms. ONNX format. | Overkill. Designed for neural nets. | Works but 5MB binary overhead for a <1MB model. | Best option for Isolation Forest. | **tract** for GBDT, **custom** for Isolation Forest |
| **Tx anomaly detection** (1D-CNN, <200KB) | Excellent. CNN inference well-supported. <0.2ms. | Works. Unnecessary GPU path for this model size. | Works. | Feasible but manually implementing CNN is error-prone. | **tract** |
| **Block-level analysis** (GBDT, <500KB) | Excellent. Same as tx anomaly GBDT. | Overkill. | Works. | Feasible. | **tract** |
| **Contract audit ML** (1D-CNN, <1.5MB) | Excellent. <5ms inference. | Works. | Works. | Feasible but unnecessary complexity for CNN. | **tract** |
| **Runtime exploit detection** (GBDT, <200KB) | Excellent. <0.1ms. | Overkill. | Overkill. | Feasible. | **tract** |
| **Gas cost prediction** (linear regression, <10KB) | Overkill for linear regression. | Overkill. | Overkill. | **Best fit.** 5 lines of Rust code. | **Custom Rust** |
| **DDoS detection** (GBDT, <100KB) | Excellent. | Overkill. | Overkill. | Feasible. | **tract** |
| **PoUW inference — small models** (<100M params, CPU) | Excellent. Deterministic. Best for MPRE. | Works. Less deterministic. | Works. Widest format support. | Not feasible at this complexity. | **tract** |
| **PoUW inference — medium models** (<1B params, GPU) | Limited GPU support. | **Best fit.** CUDA/Metal GPU acceleration. HuggingFace model support. | Works. CUDA via providers. | Not feasible. | **candle** |
| **PoUW inference — large models** (1-7B params, GPU) | Not feasible (no GPU memory management for large models). | Works for some architectures. | **Best fit.** Most optimized for large model inference. Execution providers for TensorRT, CUDA, DirectML. | Not feasible. | **ONNX Runtime** |

#### 6.4.2 WASM Compatibility Assessment

| Runtime | WASM Target | Assessment |
|---------|-------------|------------|
| **tract** | `wasm32-unknown-unknown` | **Partial.** tract core compiles to WASM. The `tract-onnx` parser and `tract-core` inference engine work in WASM. However, performance is 3-5x slower than native due to lack of SIMD and threading. WASM SIMD proposal (128-bit) is available in all modern browsers but tract's SIMD paths are AVX2/NEON-specific. Feasible for very small models (<100KB) with acceptable latency. |
| **candle** | `wasm32-unknown-unknown` | **Partial.** candle has experimental WASM support. CPU-only in WASM (no GPU). Performance is 5-10x slower than native. Not practical for real-time inference in browser. |
| **ONNX Runtime** | `wasm32-unknown-unknown` | **Yes (via ort-web).** The ONNX Runtime team maintains a WASM build (`ort-web`). It runs with WASM SIMD and WebGL/WebGPU backends. However, the WASM binary is ~5-8MB, which exceeds our browser node binary budget. |
| **Custom Rust** | `wasm32-unknown-unknown` | **Yes.** Isolation forests and linear regression are trivially WASM-compatible. No external dependencies. <10KB additional WASM. |

**Browser node AI recommendation:** Browser nodes run ONLY the custom Rust models (Isolation Forest for statistical outlier detection, linear regression for gas estimation). All tract/candle/ONNX models are excluded from browser builds. This keeps the browser WASM binary within the ~3MB budget (Section 4.3.1) while providing minimal but meaningful AI capability.

#### 6.4.3 Model Size Budgets Per Node Type

| Node Type | AI Runtime | Max Model Memory | Max Models Loaded | Total AI Memory Budget |
|-----------|-----------|-----------------|------------------|----------------------|
| **Full Node** | tract + custom Rust | 50 MB | 6 (tx ensemble + block + contract audit + runtime exploit + DDoS + gas) | 100 MB |
| **Light Node** | custom Rust only | 5 MB | 2 (Isolation Forest + gas estimation) | 10 MB |
| **Browser Node** | custom Rust (WASM) | 2 MB | 2 (Isolation Forest + gas estimation) | 5 MB |
| **Mobile Node** | custom Rust only | 5 MB | 2 (Isolation Forest + gas estimation) | 10 MB |
| **Validator** | tract + custom Rust | 50 MB (consensus AI) | 6 (same as full node) | 100 MB (consensus AI) |
| **Validator + PoUW Tier 1** | tract | 2 GB | Configurable | 4 GB |
| **Validator + PoUW Tier 2** | tract + candle | 8 GB (VRAM) | Configurable | 16 GB |
| **Validator + PoUW Tier 3** | tract + candle + ONNX RT | 24 GB (VRAM) | Configurable | 48 GB |

#### 6.4.4 CPU/Memory Impact on Node Requirements

| Node Type | Base RAM (Section 4) | AI RAM Addition | New Total RAM |
|-----------|---------------------|-----------------|---------------|
| Full Node | 8 GB min | +100 MB | 8 GB min (negligible increase) |
| Light Node | 512 MB min | +10 MB | 512 MB min (negligible) |
| Browser Node | Browser tab | +5 MB | No change to browser requirements |
| Mobile Node | 256 MB app | +10 MB | 266 MB (within original budget) |
| Validator | 16 GB min | +100 MB | 16 GB min (negligible) |
| Validator + PoUW T1 | 32 GB min | +4 GB | 32 GB min (within original) |
| Validator + PoUW T2 | 32 GB min | +16 GB | 48 GB recommended |
| Validator + PoUW T3 | 32 GB min | +48 GB | 64 GB+ recommended |

**Assessment:** The AI integration adds negligible resource overhead to non-PoUW nodes. The protocol-level models (anomaly detection, contract audit, exploit detection) total ~4MB of model data and ~100MB of runtime memory. This is within the headroom of Section 4's original hardware requirements. PoUW validators require significant additional resources, but PoUW is explicitly opt-in and already accounts for GPU/memory in the tiered design (Section 4.5.4).

#### 6.4.5 FINAL Runtime Recommendation

> **STACK RULING REQUEST [ai-integration-engineer -> blockchain-architect]: AI Runtime Refinement**
>
> **Current ruling (Section 0):** Primary: tract. Secondary: candle. Optional: ONNX Runtime.
>
> **Proposed refinement:** Accept the current ruling with the following specifics:
>
> 1. **tract** — Primary runtime for ALL protocol-level AI (anomaly detection, contract audit, exploit detection, DDoS detection) and PoUW Tier 1 inference. Used on full nodes, validators, and any node running ONNX-format models. This is the only AI runtime compiled into the base node binary.
>
> 2. **Custom Rust implementations** — For models where tract is overkill: Isolation Forest (statistical anomaly detection), linear regression (gas estimation), peer reputation scoring, and all EWMA/threshold-based monitoring. These are compiled into ALL node types including browser and mobile. No external dependency.
>
> 3. **candle** — Secondary runtime, loaded as a dynamic library (not compiled into base binary). Required only for PoUW Tier 2+ validators running GPU-accelerated inference. Provides CUDA/Metal acceleration for medium-sized models.
>
> 4. **ONNX Runtime** — Optional runtime, loaded as a dynamic library. Required only for PoUW Tier 3 validators running large models. Provides TensorRT/CUDA execution providers for maximum large-model performance.
>
> **Key refinement from original ruling:** The addition of "Custom Rust implementations" as a distinct tier below tract, specifically for the lightest models. This was implicit in the original ruling but should be explicit because it determines what AI capability browser and mobile nodes have.
>
> **WASM strategy:** Only custom Rust AI code compiles to WASM. tract, candle, and ONNX Runtime are excluded from the WASM build. Browser/mobile nodes get statistical anomaly detection and gas estimation only.
>
> **Justification:** This tiered approach ensures:
> - Base node binary stays small (~3MB WASM for browser, ~20MB native for full node — AI adds <2MB native, <10KB WASM)
> - Protocol-level AI runs on consumer hardware (all models total <4MB, <100MB RAM, <1ms per transaction)
> - PoUW validators can scale AI capability with hardware investment
> - No node type is forced to install GPU libraries or large ML frameworks

---

### 6.5 Model Governance

AI models embedded in the protocol require a governance framework that ensures model quality, prevents poisoning, and enables upgrades without hard forks.

#### 6.5.1 Model Categories and Governance Requirements

| Category | Examples | Governance Level | Update Frequency |
|----------|---------|-----------------|-----------------|
| **Protocol-Critical** | Tx anomaly ensemble, block-level analyzer | Supermajority governance vote (2/3 of staked tokens) | Quarterly at most, with 30-day review period |
| **Protocol-Advisory** | Contract audit ML, runtime exploit detection, DDoS detection | Standard governance vote (>50% of participating voters) | Monthly at most, with 14-day review period |
| **PoUW Registry** | Models in the ModelRegistry available for inference | Permissionless registration, governance-controlled removal | Continuous (anyone can register; removal requires vote) |

#### 6.5.2 Model Proposal and Validation Process

**For Protocol-Critical and Protocol-Advisory models:**

```
MODEL UPDATE LIFECYCLE:

1. PROPOSAL
   - Proposer submits ModelUpdateProposal transaction:
     ├── model_category: ProtocolCritical | ProtocolAdvisory
     ├── model_id: existing model being updated (or NEW for first deployment)
     ├── new_model_hash: BLAKE3 hash of the serialized model
     ├── model_format: ONNX | CustomRust
     ├── model_size_bytes: u64
     ├── performance_report: {
     │     accuracy, precision, recall, f1_score,
     │     false_positive_rate, false_negative_rate,
     │     inference_time_p50, inference_time_p99,
     │     test_dataset_hash, test_methodology
     │   }
     ├── training_provenance: {
     │     dataset_description, training_method, hyperparameters,
     │     training_hardware, reproducibility_instructions
     │   }
     ├── diff_from_previous: summary of changes
     └── deposit: required deposit (slashable if model is malicious)

2. REVIEW PERIOD
   - Protocol-Critical: 30 days
   - Protocol-Advisory: 14 days
   - During review, any validator can:
     a. Download the model from IPFS (verified by model_hash)
     b. Run the model against the published test dataset
     c. Run the model against their own test data
     d. Submit a ModelReview transaction (APPROVE / REJECT / FLAG)
     e. If FLAG: must include evidence of a problem
        (false positive examples, adversarial inputs that fool the model,
         performance degradation measurements)

3. VALIDATION QUORUM
   - Minimum 20% of active validators must submit a ModelReview
   - Of those, the threshold for acceptance:
     Protocol-Critical: 2/3 APPROVE (of reviewed votes)
     Protocol-Advisory: >50% APPROVE (of reviewed votes)
   - If quorum is not met, the review period extends by 7 days (once)
   - If still no quorum after extension, the proposal expires

4. ACTIVATION
   - Accepted models are activated at the next epoch boundary
   - All nodes download the new model from IPFS and verify the hash
   - The old model remains active as fallback for 1 epoch (2 epochs for Protocol-Critical)
   - If >10% of nodes report errors loading the new model,
     automatic rollback to the previous version

5. ROLLBACK
   - Any validator can submit a ModelRollbackProposal at any time
   - If the new model shows degraded performance in production:
     a. EMERGENCY rollback: 1/3 of validators sign a rollback attestation → immediate rollback to previous version, governance vote follows within 48 hours
     b. STANDARD rollback: standard governance vote (same threshold as update)
   - Rolled-back model proposer's deposit is slashed if the rollback was due to model malice (not just poor performance)
```

#### 6.5.3 Model Versioning

```
ModelVersion {
    model_id:       Hash,          // Stable identifier (hash of model purpose + name)
    version:        u32,           // Monotonically increasing version number
    model_hash:     Hash,          // BLAKE3 hash of the serialized model artifact
    activation_epoch: u64,         // Epoch when this version became active
    status:         Active | Deprecated | RolledBack,
    predecessor:    Option<Hash>,  // Previous version's model_hash
}
```

All model versions are immutable on-chain records. Even rolled-back models retain their history for auditability. The full version chain from genesis is available for any model ID.

#### 6.5.4 Anti-Poisoning Safeguards

Model poisoning — submitting a model that appears to work correctly but contains a subtle backdoor or bias — is the primary adversarial risk to model governance.

**Safeguard 1: Adversarial Testing Mandate**

Every ModelUpdateProposal must include an adversarial test report showing:
- Model behavior on known adversarial inputs for the model class (e.g., for anomaly detection: does the model flag known historical exploits? does it miss crafted evasion examples?)
- Performance on a held-out dataset that the proposer did NOT use for training
- At least 3 independent validators must reproduce the reported performance metrics before voting APPROVE

**Safeguard 2: Canary Deployment**

New Protocol-Critical models deploy in "canary mode" for the first epoch:
- The new model runs alongside the old model
- Both models process every input
- Outputs are compared
- If the new model's output disagrees with the old model on >5% of inputs, an automatic investigation is triggered (not rollback — disagreement might mean the new model is better)
- Validators manually review the disagreeing cases

**Safeguard 3: Differential Privacy for Training Data**

Model proposals should demonstrate that the training process did not memorize specific transaction patterns (which could be used to create blind spots). Training methodology should include differential privacy guarantees (epsilon <= 8.0 for Protocol-Critical models). This is verified through the training provenance metadata.

**Safeguard 4: Model Diversity**

The anomaly detection ensemble (Section 6.1.1) uses three fundamentally different model architectures (GBDT, Isolation Forest, CNN). A poisoned model in one architecture is unlikely to evade the other two. Model updates that reduce architectural diversity require elevated scrutiny (Protocol-Critical governance threshold regardless of category).

**Safeguard 5: Hash-Locked Model Artifacts**

Models are stored on IPFS and referenced by BLAKE3 hash on-chain. A model artifact cannot be modified after proposal without changing its hash, which would invalidate the governance vote. There is no mechanism to update a model in-place — every change requires a new proposal.

---

### 6.6 AI Compute Market

The PoUW layer (Section 2.3) creates the foundation for a decentralized AI compute marketplace. This section defines how that marketplace operates.

#### 6.6.1 Market Architecture

```
AI COMPUTE MARKET FLOW:

Demand Side:                              Supply Side:
+------------------+                      +------------------+
| Smart Contract   |                      | PoUW Validator   |
| or User submits  | ----InferenceReq---> | (Tier 1/2/3)     |
| InferenceRequest |                      | executes model   |
+------------------+                      | produces proof   |
        |                                 +------------------+
        |                                         |
        v                                         v
+------------------+                      +------------------+
| Fee is escrowed  |                      | InferenceAttest  |
| in protocol      |                      | included in DAG  |
| account          |                      | vertex           |
+------------------+                      +------------------+
        |                                         |
        +-------------> On finality: <------------+
                    Fee distributed:
                    - 85% to executing validator(s)
                    - 10% to protocol treasury
                    - 5% to model creator (if model has creator_royalty flag)
```

#### 6.6.2 Pricing Mechanism

**Dynamic Fee Market for Inference**

The inference fee market operates independently from the transaction fee market (they serve different resources — AI compute vs. block space):

```
InferenceFeeMarket {
    // Base fee: adjusts per epoch based on PoUW utilization
    base_fee_per_compute_unit: u64,

    // Adjustment rule (EIP-1559-style):
    // If utilization > 80%: base_fee *= 1.125 (12.5% increase per epoch)
    // If utilization < 50%: base_fee *= 0.875 (12.5% decrease per epoch)
    // Otherwise: base_fee unchanged

    // Priority fee: user-specified tip above base fee
    // Validators prefer higher-priority requests when capacity is constrained

    // Compute units: model-specific, registered in ModelRegistry
    // compute_units = f(model_params, input_size, verification_method)
    // MPRE requests cost 3x single execution (3 validators)
    // TEE requests cost 1.2x (TEE overhead)
    // ZK requests cost 5-20x (proof generation)
}
```

**Compute Unit Calculation:**

```
compute_units(request) =
    model_base_cost                      // From ModelRegistry, set by model creator
    * input_size_multiplier              // Scales with input dimensions
    * verification_multiplier            // 1.0 for single, 3.0 for MPRE, 1.2 for TEE, 10.0 for ZK
    * urgency_multiplier                 // 1.0 for NORMAL, 2.0 for PRIORITY

total_fee = compute_units * (base_fee_per_compute_unit + priority_fee)
```

#### 6.6.3 Quality Assurance for Inference Results

Quality assurance implements the multi-metric framework from Section 2.3.4, with the following enforcement mechanisms:

**Quality enforcement pipeline:**

```
1. CORRECTNESS VERIFICATION (per-request)
   - MPRE: 2-of-3 agreement at quantized precision (16-bit fixed-point)
   - TEE: valid attestation certificate chain (Intel/AMD CA root)
   - ZK: valid proof verification (mathematical, ~1ms)
   - Failed verification → result rejected, validator slashed

2. STATISTICAL QUALITY MONITORING (per-epoch)
   For each PoUW validator, compute:
   - correctness_rate = verified_correct / total_assigned
   - availability_rate = completed / total_assigned
   - latency_percentiles = p50, p95, p99 of completion times
   - diversity_score = entropy of model types served

   Quality score = 0.40 * correctness_rate
                 + 0.25 * availability_rate
                 + 0.20 * (1.0 - normalized_latency_p95)
                 + 0.15 * diversity_score

3. REPUTATION UPDATE
   PoUWReputation(v) = 0.8 * previous_reputation + 0.2 * current_epoch_quality_score
   (Exponential moving average with 80% weight on history)

4. CONSEQUENCES
   - quality_score < 0.3 for 3 consecutive epochs → PoUW assignment suspended
   - correctness_rate < 0.5 for any epoch → immediate investigation, stake at risk
   - availability_rate < 0.5 for 5 consecutive epochs → removed from PoUW eligibility for 10 epochs
```

#### 6.6.4 Differentiation from Competitors

| Dimension | Bittensor (TAO) | Ritual | Fetch.ai / ASI Alliance | Aztibase Network |
|-----------|----------------|--------|------------------------|---------------|
| **AI's role** | AI IS the chain (subnets are AI tasks) | AI bridge layer (Infernet connects off-chain AI to on-chain) | AI agents + marketplace (separate architectures merged) | AI is one of several native protocol capabilities alongside smart contracts, privacy, and payments |
| **Compute verification** | Subnet-specific evaluation (often speed-based, led to garbage outputs — RESEARCH_BRIEF.md 4.1) | Dual proof sharding (TEE + ZK) | Limited verification | Triple verification (MPRE + TEE + ZK), multi-metric quality scoring |
| **General smart contracts** | No (AI-only network) | Limited (EVM++ focused on AI precompiles) | Limited (agent-focused) | Full WASM + EVM support alongside AI |
| **Privacy** | None | Partnership with Nillion for blind compute | Limited | Native protocol-level privacy (selective disclosure, shielded inference) |
| **Incentive design** | Speed-based metrics → garbage outputs | Fee market (early stage) | Token-staking model | Multi-metric (correctness 40%, availability 25%, latency 20%, diversity 15%) with explicit anti-gaming |
| **Model governance** | Subnet owner controls model selection | Centralized model registry | Per-protocol governance | On-chain governance with adversarial testing, canary deployment, emergency rollback |
| **Node-friendliness** | Light nodes projected to 1TB by 2025 (RESEARCH_BRIEF.md 4.1) | Unknown (testnet only) | Not focused on consumer hardware | Protocol AI on consumer hardware (<4MB models, <100MB RAM). PoUW is opt-in only. |
| **Server-independence** | Governance concentrated in 3 members + 12 senate validators | Early stage, centralized | Merger complexity | Full P2P, no central coordinator, VRF leader election |

**Aztibase Network's unique positioning:** No other chain combines AI compute verification with general smart contracts, protocol-level privacy, and consumer-grade node requirements. Bittensor is AI-only. Ritual requires off-chain infrastructure. ASI Alliance is a merger of three separate systems. Aztibase Network embeds AI as one pillar of a complete, self-contained blockchain.

---

### 6.7 Graceful Degradation

Every node type must function correctly even if all AI features fail. AI is a first-class citizen but not a load-bearing wall.

#### 6.7.1 AI Feature Matrix by Node Type

| AI Feature | Full Node | Light Node | Browser Node | Mobile Node | Validator | Validator+PoUW |
|------------|-----------|------------|-------------|-------------|-----------|---------------|
| Tx anomaly detection (full ensemble) | Yes | No | No | No | Yes | Yes |
| Tx anomaly detection (Isolation Forest only) | Yes | Yes | Yes (WASM) | Yes | Yes | Yes |
| Block-level analysis | Yes | No | No | No | Yes | Yes |
| Contract audit (static) | Yes | No | No | No | Yes | Yes |
| Contract audit (ML) | Yes | No | No | No | Yes | Yes |
| Runtime exploit detection | Yes | No | No | No | Yes | Yes |
| Gas cost AI estimation | Yes | Yes | Yes (WASM) | Yes | Yes | Yes |
| P2P health monitoring | Yes | Yes | Partial | Partial | Yes | Yes |
| Peer reputation scoring | Yes | Yes | Simplified | Simplified | Yes | Yes |
| DDoS detection (full) | Yes | No | No | No | Yes | Yes |
| DDoS detection (rate limits only) | Yes | Yes | Yes | Yes | Yes | Yes |
| PoUW inference execution | No | No | No | No | No | Yes |
| Verify InferenceAttestations | Yes | Via Verkle proof | Via Verkle proof | Via Verkle proof | Yes | Yes |

#### 6.7.2 Failure Mode: AI Runtime Unavailable

If the AI runtime (tract) fails to load or crashes on a full node:

```
GRACEFUL DEGRADATION SEQUENCE:

1. Log error: "AI runtime unavailable: {reason}"
2. Set node flag: ai_degraded = true
3. All AI features switch to PASSTHROUGH mode:
   - Transaction anomaly detection: all transactions scored 0.0 (normal)
   - Block-level analysis: all blocks marked NORMAL
   - Contract audit: ML stage skipped, static analysis only
   - Runtime exploit detection: rule engine only (no ML classifier)
   - DDoS detection: rate limits only (no ML layer)
4. Node continues normal operation:
   - Vertex validation: proceeds normally (AI attestation verification
     is cryptographic, not ML — it uses ZK/TEE/hash comparison, not AI models)
   - Transaction processing: unaffected
   - Consensus participation: unaffected (AI is advisory only)
5. Periodically retry AI runtime initialization (every 60 seconds)
6. If AI runtime recovers: resume full AI features, log recovery
```

**Impact of AI degradation on validators:**
- A validator with ai_degraded = true can still participate in SynBFT consensus fully.
- Its ConsensusReputation is unaffected (AI features are not part of consensus).
- Its PoUWReputation will decay if it is a PoUW validator (cannot process inference requests without AI runtime), but this only affects up to 15% of its anchor score.
- The validator will not be slashed for AI degradation — slashing requires provably incorrect behavior, not unavailability.

#### 6.7.3 Minimum Viable AI for Light/Browser/Mobile Nodes

These node types run ONLY:

1. **Isolation Forest anomaly detector** (custom Rust, <200KB, WASM-compatible): Provides basic statistical outlier detection on transactions the node processes. This is not comprehensive but catches obvious anomalies (extreme values, known attack patterns).

2. **Gas estimation model** (custom Rust linear regression, <10KB, WASM-compatible): Provides inference cost prediction so users can estimate fees before submitting InferenceRequests.

3. **Rate-limiting based spam protection** (custom Rust, no ML): Basic rate limits on peer connections and messages.

4. **Peer reputation scoring** (custom Rust, simplified): Reduced feature set (gossip quality + data availability only, omitting anomaly relay rate which requires full anomaly detection).

These features total <500KB of code and <5MB of runtime memory. They provide meaningful security benefit without requiring any ML framework dependency.

If even these minimal AI features fail, the node operates as a pure cryptographic light client — verifying BLS finality certificates and Verkle proofs only. This is perfectly functional for sending/receiving transactions and checking balances.

---

### 6.8 KYA (Know Your Agent) Framework Implementation

The `AIAgent` protocol primitive (Section 1.6.1) requires a concrete identity framework for on-chain AI agents. This section defines the KYA implementation.

#### 6.8.1 Agent Identity Structure

```
AIAgentIdentity {
    agent_id:           AgentId (derived from creator's address + nonce),
    creator:            Address (human principal who created the agent),
    kya_attestation: {
        purpose:        String (max 256 bytes — human-readable description),
        capabilities:   Vec<Capability> (declared operations the agent can perform),
        model_ids:      Vec<ModelHash> (models the agent uses, from ModelRegistry),
        data_sources:   Vec<DataSourceDeclaration> (external data the agent accesses),
        autonomy_level: LOW | MEDIUM | HIGH,
            // LOW: every transaction requires creator co-signature
            // MEDIUM: operates within spending limits, alerts creator on anomalies
            // HIGH: fully autonomous within declared constraints
        contact:        String (URI for the responsible human — email, ENS, etc.),
    },
    operational_constraints: {
        max_tx_per_epoch:       u64,
        max_value_per_tx:       u64,
        max_value_per_epoch:    u64,
        allowed_contracts:      Vec<Address> | ALL,
        allowed_operations:     Vec<OperationType> | ALL,
        expiry_epoch:           Option<u64> (agent deactivates after this epoch),
    },
    status:             Active | Suspended | Deactivated,
    creation_epoch:     u64,
    last_updated_epoch: u64,
}
```

#### 6.8.2 Agent Lifecycle

```
1. CREATION: Creator submits CreateAgent transaction with KYA attestation and initial constraints. Requires deposit (returned on deactivation minus any slashing).

2. OPERATION: Agent transacts within its declared constraints. Any transaction exceeding constraints is rejected at the execution layer (pre-execution check, not consensus check — this is deterministic).

3. UPDATE: Creator can update KYA attestation and constraints. Updates to constraints that INCREASE limits require a 1-epoch delay (cooling period for safety). Decreases are immediate.

4. SUSPENSION: Creator can immediately suspend (freeze) an agent. Protocol can suspend an agent if it accumulates anomaly scores above threshold for 3 consecutive epochs. Suspended agents cannot transact.

5. DEACTIVATION: Creator deactivates the agent. Remaining deposit returned after unbonding period.
```

#### 6.8.3 Agent Accountability

- Every transaction from an AIAgent includes the `agent_id` in the transaction metadata. This is not optional — it is enforced at the protocol level.
- The creator's address is permanently linked to the agent. If the agent causes harm (exploit, fund loss), the creator's deposit is at risk.
- Agent transactions are distinctly typed in the mempool and DAG vertices, enabling the anomaly detection ensemble (Section 6.1.1) to apply agent-specific heuristics.

---

### 6.9 ModelRegistry Implementation

The `ModelRegistry` protocol primitive (Section 1.6.1) provides on-chain model fingerprinting, versioning, and licensing.

#### 6.9.1 Registry Entry Structure

```
ModelRegistryEntry {
    model_hash:         Hash (BLAKE3 of serialized model artifact),
    owner:              Address (model creator/publisher),
    metadata: {
        name:           String (max 128 bytes),
        description:    String (max 1024 bytes),
        version:        SemVer,
        format:         ONNX | SafeTensors | Custom,
        parameter_count: u64,
        input_schema:   IOSchema (describes expected input format),
        output_schema:  IOSchema (describes output format),
        compute_cost:   u64 (base compute units for inference),
        min_hardware:   HardwareRequirement (CPU_ONLY | GPU_8GB | GPU_24GB),
    },
    licensing: {
        license_type:   OpenSource | Commercial | PerInference,
        creator_royalty: f32 (0.0 - 0.15, max 15% of inference fee),
        usage_terms_hash: Hash (IPFS CID of full license terms),
    },
    storage: {
        ipfs_cid:       String (IPFS content identifier for model artifact),
        size_bytes:     u64,
        checksum:       Hash (redundant integrity check),
    },
    verification: {
        supported_methods: Vec<MPRE | TEE | ZK>,
        deterministic:     bool (can this model produce identical results across hardware?),
        zk_circuit_hash:   Option<Hash> (if ZK verification is supported, hash of the verification circuit),
    },
    status:             Active | Deprecated | Removed,
    registered_epoch:   u64,
    usage_count:        u64 (total inference requests served),
}
```

#### 6.9.2 Registration Process

```
MODEL REGISTRATION:

1. Publisher submits RegisterModel transaction with metadata + deposit
2. Model artifact must be available on IPFS at the declared CID
3. The protocol verifies:
   a. model_hash matches the BLAKE3 hash of the artifact at ipfs_cid
   b. parameter_count and size_bytes are consistent with the artifact
   c. Deposit meets minimum (proportional to model size — larger models require larger deposits to prevent registry spam)
4. Model is registered with status = Active
5. Any PoUW validator can begin serving inference for this model

REMOVAL (governance):
- Any token holder can submit a ModelRemovalProposal
- Reasons: licensing violation, consistently bad outputs, security risk
- Standard governance vote (>50% of participating voters)
- Removed models cannot receive new inference requests
- Existing in-flight requests complete normally
```

---

### 6.10 Stack Challenges

#### 6.10.1 AI Runtime Stack: ACCEPTED WITH REFINEMENT

> **STACK RULING ACCEPTANCE [ai-integration-engineer -> blockchain-architect]**
>
> The stack ruling in Section 0 (tract primary, candle secondary, ONNX Runtime optional) is **ACCEPTED** with the following refinement:
>
> **Addition: Custom Rust as an explicit fourth tier** for the lightest models (Isolation Forest, linear regression, peer scoring). This tier is the ONLY AI code that compiles to WASM and runs on browser/mobile nodes. It has zero external ML library dependencies and adds <10KB to the WASM binary.
>
> **Justification:** The original ruling implicitly permits custom implementations but does not explicitly address the browser/mobile AI capability question. Making "Custom Rust" an explicit tier clarifies that browser and mobile nodes DO have AI features (statistical anomaly detection, gas estimation) without requiring tract or any ML framework in WASM. This closes the gap between "all AI inference is off-chain" (true for PoUW) and "AI is a first-class citizen" (true at the protocol level — even light clients benefit from basic anomaly detection).
>
> This is a refinement, not a challenge. The original ruling's framework is correct.

#### 6.10.2 Floating-Point Determinism: FLAGGED

> **STACK FLAG [ai-integration-engineer -> blockchain-architect]: Cross-Architecture Floating-Point Determinism**
>
> tract produces deterministic results on the SAME hardware architecture. Across architectures (x86 vs ARM, different CPU microarchitectures), IEEE 754 edge cases (NaN handling, denormal flushing, FMA fusion) can produce divergent results for identical inputs.
>
> **Impact:** MPRE verification requires comparing results from different validators who may run different hardware. Section 2.3.2 addresses this with "quantize to 16-bit fixed-point for comparison." This section confirms that approach is correct and sufficient for all models specified in this design:
>
> - GBDT models: decision tree traversal is integer comparison. Leaf values are the only floating-point outputs. Quantizing to 16-bit fixed-point preserves >99.99% of the information for all anomaly scores and probabilities.
> - CNN models: floating-point divergence accumulates through layers, but the output (a probability between 0.0 and 1.0) differs by at most ~0.001 across architectures for well-conditioned models. 16-bit fixed-point (precision ~0.00003) is more than sufficient.
> - Isolation Forest: path lengths are integers. Anomaly scores are derived from mean path lengths. Divergence is negligible.
>
> **Recommendation:** No change to consensus design needed. The 16-bit quantization approach in Section 2.3.2 is validated from the AI perspective. However, the protocol should include a `comparison_precision` field in the MPRE protocol that can be adjusted per-model (some models may require 32-bit comparison for high-precision outputs like financial predictions).
>
> **Priority:** STANDARD (not blocking).

---

**Stack challenges raised:**
- STACK RULING ACCEPTANCE WITH REFINEMENT: AI Runtime — adding explicit "Custom Rust" tier for browser/mobile AI. See Section 6.10.1.
- STACK FLAG: Cross-architecture floating-point determinism in MPRE. See Section 6.10.2.

**Conflicts raised:** None

**Sign-off:** ai-integration-engineer

## SECTION 7: SMART CONTRACT LAYER

---
**Contributed by:** smart-contract-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete), Section 0 stack rulings (reviewed), Section 1 architecture (reviewed -- hybrid account+object model, modular monolith Layer 3/4, AI Oracle precompile, fee market from genesis), Section 2 consensus (reviewed -- SynBFT 400ms rounds, DAG structure, PoUW InferenceAttestation lifecycle, BLS stack challenge), Section 4 node architecture (reviewed -- five node types, redb dual storage, browser WASM compilation, Verkle proofs for light clients), Section 6 AI integration (reviewed -- tract-based audit pipeline, KYA framework, ModelRegistry, inference verification)

---

### SMART CONTRACT LAYER

The smart contract layer defines how user-deployed code executes on Aztibase Network. It spans Layer 3 (Execution) and Layer 4 (Application) of the modular monolith architecture. Every design decision below is constrained by: deterministic execution across all node types, AI-native integration as a protocol primitive, browser node interoperability via light client Verkle proofs, and the 400ms round budget established by SynBFT.

---

### 7.1 Virtual Machine Design

#### 7.1.1 Runtime Selection: wasmtime (ACCEPTED with refinements)

**Choice: wasmtime** as the primary WASM execution engine.

**Evaluation matrix:**

| Criterion | wasmtime | wasmer | Custom VM |
|-----------|----------|--------|-----------|
| Maintainer | Bytecode Alliance (Mozilla, Fastly, Intel, Red Hat) | Wasmer Inc. (single company) | Aztibase Network team |
| License | Apache 2.0 | MIT | N/A |
| Production users | Fastly Compute@Edge, Shopify, Fermyon, Wasmcloud | NEAR Protocol, Cloudflare (partial) | N/A |
| Deterministic execution | Cranelift compiler with determinism flags; fuel-based metering; configurable epoch interruption | Singlepass compiler (deterministic but less optimized); fuel metering added later | Full control |
| Security | Extensive fuzzing (OSS-Fuzz, libFuzzer), 60+ CVEs addressed, spectre mitigations, sandboxed by design | Fewer CVEs tracked, smaller security team, some sandbox escape issues in 2023 | Unproven |
| WASI support | Reference implementation of WASI (lead implementor) | WASI support (follower) | Must implement |
| Compilation speed | Cranelift: ~10ms for typical contract | Singlepass: ~5ms (fastest), Cranelift backend also available | Varies |
| Execution speed | Cranelift generates near-native code; ~1.2-1.5x native performance | Singlepass: ~2-3x native; LLVM backend: ~1.1x native (but slow compile) | Varies |
| Memory safety | Rust-native, linear memory sandboxing, stack depth enforcement | Rust-native, similar sandboxing | Must build |
| Ecosystem momentum | Growing (WASI standard, component model) | Shrinking relative to wasmtime since 2024 | Zero |
| Fuel metering | Native `fuel` API -- inject fuel at entry, trap on exhaustion | Added fuel API in 2024, less mature | Must build |

**Why wasmtime over wasmer:**

1. **Bytecode Alliance governance.** wasmtime is maintained by a multi-company consortium (Mozilla, Fastly, Intel, Red Hat), not a single startup. This reduces bus-factor risk. NEAR uses wasmer, but NEAR's wasmer dependency has been a maintenance burden -- they maintain a custom fork (`near-wasmer`) with blockchain-specific patches that regularly fall behind upstream.

2. **Determinism guarantees.** wasmtime's Cranelift compiler is designed for deterministic compilation -- the same WASM bytecode always produces the same native code on the same target architecture. This is critical for consensus: all validators must execute the same instructions and get the same result. wasmer's Singlepass compiler is also deterministic but produces slower code; wasmer's LLVM backend is faster but non-deterministic across LLVM versions.

3. **Fuel-based metering is native.** wasmtime's fuel API is mature and battle-tested. You set a fuel limit before execution and the engine traps when fuel is exhausted. This is cleaner than instruction-counting approaches (which require WASM bytecode rewriting) and more predictable than wall-clock timeouts.

4. **Security track record.** Bytecode Alliance runs continuous fuzzing, has a formal security advisory process, and actively mitigates speculative execution attacks. For a blockchain VM where every contract is potentially adversarial code, this security posture is non-negotiable.

5. **WASI and component model.** wasmtime leads WASI standardization. While we do not expose full WASI to contracts (see Section 7.1.5), the component model provides a clean mechanism for defining host function interfaces that contracts can import.

**Why not custom VM:**

Building a custom VM is a multi-year effort with no ecosystem tooling. The research brief notes that Fuel's custom FuelVM, despite technical merit, resulted in "600+ TPS in production well below benchmark claims" partly due to the engineering effort diverted to VM development. We allocate engineering resources to our differentiators (AI integration, privacy, PoUW), not to reimplementing a WASM runtime.

#### 7.1.2 Deterministic Execution Guarantees

All contract execution must be bit-for-bit identical across every validator. WASM provides a strong baseline for determinism, but several areas require explicit handling:

**Guaranteed deterministic by WASM spec:**
- Integer arithmetic (32-bit and 64-bit)
- Memory load/store operations
- Control flow (branches, loops, function calls)
- Table operations

**Requires explicit handling:**

| Non-determinism source | Mitigation |
|------------------------|------------|
| **Floating-point NaN payloads** | wasmtime canonicalizes NaN values (all NaN payloads are set to a single canonical value). This is enabled by default in wasmtime's `Config`. |
| **Floating-point operation ordering** | WASM's FP semantics are fully specified (IEEE 754-2019). wasmtime follows the spec. No mitigation needed beyond enabling canonical NaN. |
| **System calls / host functions** | All host functions return deterministic results. No access to wall-clock time, random number generators, or external I/O from within contract execution. Timestamps are passed as transaction context, not queried. |
| **Memory growth** | `memory.grow` is deterministic in WASM (returns the previous page count or -1). We set a maximum memory limit (see Section 7.1.4) and trap on OOM. |
| **Multi-threading** | WASM threads (shared memory + atomics) are DISABLED in the contract VM. All contract execution is single-threaded. Parallel execution happens at the transaction level (Block-STM), not within a single contract call. |
| **SIMD instructions** | WASM SIMD is ENABLED (useful for cryptographic operations in contracts). SIMD operations in WASM are fully deterministic per spec. |
| **Fuel consumption** | Fuel deduction is deterministic -- the same WASM instruction sequence consumes the same fuel on every validator. |

**Verification:** A determinism test suite will execute a corpus of adversarial contracts (designed to probe every edge case above) across multiple platforms (x86_64 Linux, x86_64 macOS, aarch64 Linux) and verify bit-for-bit identical execution traces and state outputs.

#### 7.1.3 Gas Model: Fuel-Based Metering

Aztibase Network uses **fuel-based metering** via wasmtime's native fuel API. This replaces traditional instruction counting or gas table approaches.

**How fuel works:**

```
1. Transaction specifies max_fuel (converted from the user's gas limit)
2. Before contract execution, the VM is configured with max_fuel
3. wasmtime decrements fuel for each WASM instruction executed
4. If fuel reaches zero, execution traps (OutOfFuel)
5. Remaining fuel is refunded to the sender (unused gas refund)
6. Consumed fuel is converted to the native token fee
```

**Fuel cost schedule (base units per operation class):**

| Operation Class | Fuel Cost | Rationale |
|-----------------|-----------|-----------|
| **Arithmetic (i32/i64)** | 1 | Baseline. Single-cycle operations. |
| **Arithmetic (f32/f64)** | 2 | Slightly more expensive due to FPU pipeline. |
| **Memory load/store** | 2 | Linear memory access, predictable cost. |
| **Memory grow (per page, 64KB)** | 1,000 | Memory allocation is expensive and affects node resources. |
| **Control flow (branch/call)** | 1 | Standard control flow is cheap. |
| **Indirect call (table lookup)** | 3 | Table lookup + branch. |
| **SIMD operation** | 3 | Vector operations on 128-bit lanes. |
| **Host function call (base)** | 100 | Context switch from WASM to host. Actual cost added by host function. |
| **State read (per 32 bytes)** | 500 | redb point lookup from cf_state_store. |
| **State write (per 32 bytes)** | 2,500 | redb write + deferred Verkle tree update. Writes are 5x reads. |
| **State delete** | 500 | Refundable portion -- state deletion reduces storage burden. |
| **BLAKE3 hash (per 32 bytes)** | 50 | Precompiled host function. |
| **Ed25519 verify** | 3,000 | Precompiled host function. Approximately 70us wall time. |
| **BLS verify** | 15,000 | Precompiled host function. Approximately 1.5ms wall time. |
| **Verkle proof verify** | 5,000 | Precompiled host function. Approximately 0.5ms wall time. |
| **AI inference request** | 50,000 (base) + model-specific | Precompiled host function. See Section 7.7 for AI gas pricing. |
| **Log emit (per byte)** | 5 | Event emission. Stored in receipts. |
| **Cross-contract call (base)** | 10,000 | New execution context. See Section 7.5.4. |

**Dynamic pricing:**

The fuel costs above are the **base schedule**. Aztibase Network implements an EIP-1559-style dynamic fee market:

- **Base fee per fuel unit:** Adjusts every anchor commit based on block utilization. If blocks are >50% full (by fuel), the base fee increases; if <50%, it decreases. The adjustment rate is bounded to +/- 12.5% per anchor.
- **Priority fee (tip):** Users bid a priority fee above the base fee to incentivize inclusion. Validators receive tips; base fees are burned (deflationary pressure).
- **Congestion pricing from genesis:** Per architect's mandate (Section 1.3, referencing Solana's congestion lesson), the fee market is active from block 1. No "free period" that creates technical debt.

**Fuel-to-token conversion:**

```
fee_paid = fuel_consumed * (base_fee_per_fuel + priority_fee_per_fuel)
fee_burned = fuel_consumed * base_fee_per_fuel
fee_to_validator = fuel_consumed * priority_fee_per_fuel
```

The tokenomics-engineer (Section 3) determines the initial base fee level and the GEN-to-fuel exchange rate. The smart contract layer only defines the metering mechanism.

#### 7.1.4 Resource Limits

| Resource | Limit | Rationale |
|----------|-------|-----------|
| **Max WASM memory** | 16 MB (256 WASM pages) | Sufficient for all reasonable contract logic. Prevents memory exhaustion attacks. Each page is 64KB. |
| **Max stack depth** | 1,024 frames | Prevents stack overflow. wasmtime enforces this natively via Cranelift's stack limit. |
| **Max fuel per transaction** | 100,000,000 (100M fuel) | Upper bound on computation per transaction. At base fuel costs, this allows ~100M basic operations or ~40,000 state reads. |
| **Max WASM binary size** | 2 MB (compressed) | Contracts above this size are rejected at deployment. Typical contracts are 100-500KB. |
| **Max execution wall time** | 5 seconds | Hard timeout via wasmtime epoch interruption. Even if fuel is not exhausted, execution that exceeds wall time is aborted. Prevents pathological cases where fuel metering underestimates real cost. |
| **Max cross-contract call depth** | 32 | Prevents unbounded reentrancy chains. See Section 7.5.4. |
| **Max event logs per transaction** | 256 | Prevents log flooding. |
| **Max state keys touched per transaction** | 10,000 | Bounds the state diff size for Verkle tree updates. |

#### 7.1.5 Host Function Interface (Syscalls)

Contracts interact with the chain state and protocol features through a defined set of host functions. These are the ONLY external interfaces available to WASM contracts -- there is no raw system access.

**State access:**

```rust
// Read a value from contract storage
fn state_read(key: &[u8]) -> Option<Vec<u8>>;

// Write a value to contract storage
fn state_write(key: &[u8], value: &[u8]);

// Delete a key from contract storage
fn state_delete(key: &[u8]);

// Check if a key exists without reading the full value
fn state_has(key: &[u8]) -> bool;

// Read an account's balance
fn balance_of(address: &Address) -> u128;

// Transfer native tokens from the current contract to an address
fn transfer(to: &Address, amount: u128) -> Result<(), TransferError>;
```

**Execution context:**

```rust
// Get the address of the calling account/contract
fn caller() -> Address;

// Get the address of the current contract
fn self_address() -> Address;

// Get the transaction origin (EOA that initiated the transaction)
fn tx_origin() -> Address;

// Get the current round number
fn current_round() -> u64;

// Get the current epoch
fn current_epoch() -> u64;

// Get the block timestamp (from the DAG vertex header, NOT wall clock)
fn block_timestamp() -> u64;

// Get remaining fuel
fn fuel_remaining() -> u64;

// Get the value (native tokens) sent with this call
fn value() -> u128;
```

**Cryptographic precompiles:**

```rust
// BLAKE3 hash
fn blake3(data: &[u8]) -> [u8; 32];

// Ed25519 signature verification
fn ed25519_verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool;

// BLS12-381 signature verification
fn bls_verify(public_key: &[u8; 48], message: &[u8], signature: &[u8; 96]) -> bool;

// BLS aggregate signature verification
fn bls_aggregate_verify(public_keys: &[[u8; 48]], message: &[u8], signature: &[u8; 96]) -> bool;

// Keccak-256 (for EVM compatibility)
fn keccak256(data: &[u8]) -> [u8; 32];

// SHA-256 (for external system interoperability)
fn sha256(data: &[u8]) -> [u8; 32];
```

**Cross-contract calls:**

```rust
// Call another contract
fn call_contract(
    address: &Address,
    function_selector: &[u8; 4],
    args: &[u8],
    value: u128,
    fuel_limit: u64,
) -> Result<Vec<u8>, CallError>;

// Delegate call (execute another contract's code in the current contract's context)
fn delegate_call(
    address: &Address,
    function_selector: &[u8; 4],
    args: &[u8],
    fuel_limit: u64,
) -> Result<Vec<u8>, CallError>;

// Static call (read-only, no state mutations allowed)
fn static_call(
    address: &Address,
    function_selector: &[u8; 4],
    args: &[u8],
    fuel_limit: u64,
) -> Result<Vec<u8>, CallError>;
```

**AI Oracle interface:**

```rust
// Request AI inference (synchronous within transaction -- blocks until PoUW result available)
// This is the AI Oracle precompile defined in Section 1.6.3
fn ai_inference(
    model_id: &[u8; 32],         // Model hash from ModelRegistry
    input: &[u8],                  // Serialized input data
    verification: VerificationMethod, // MPRE | TEE | ZK | ANY
    max_fuel: u64,                 // Maximum fuel for this inference
) -> Result<InferenceResult, InferenceError>;

// Query a cached inference result (if a matching attestation exists on-chain)
fn ai_inference_cached(
    model_id: &[u8; 32],
    input_hash: &[u8; 32],
) -> Option<InferenceResult>;

// Check if a model is registered and active
fn model_exists(model_id: &[u8; 32]) -> bool;
```

**Event emission:**

```rust
// Emit an indexed event log
fn emit_event(topics: &[[u8; 32]], data: &[u8]);
```

**Object operations (hybrid state model):**

```rust
// Create a new on-chain object owned by the caller
fn object_create(type_tag: &[u8], data: &[u8]) -> ObjectId;

// Transfer object ownership
fn object_transfer(object_id: &ObjectId, new_owner: &Address) -> Result<(), ObjectError>;

// Read object data
fn object_read(object_id: &ObjectId) -> Option<ObjectData>;

// Mutate object data (only by owner or authorized contract)
fn object_mutate(object_id: &ObjectId, data: &[u8]) -> Result<(), ObjectError>;

// Delete (burn) an object
fn object_delete(object_id: &ObjectId) -> Result<(), ObjectError>;
```

**What is explicitly NOT exposed:**
- Wall-clock time (use `block_timestamp()` for time-dependent logic)
- Random number generation (use VRF-derived randomness via a precompile -- see below)
- File system access
- Network access
- Thread creation
- Any WASI capability beyond the defined host functions

**Randomness precompile:**

```rust
// Get verifiable randomness derived from the DAG's VRF outputs
// Not available until the anchor round's VRF is committed (prevent front-running)
fn vrf_randomness(seed: &[u8]) -> [u8; 32];
```

This returns `BLAKE3(anchor_vrf_output || seed)`, providing per-contract randomness that is verifiable and unpredictable before the anchor commit. The seed allows contracts to derive multiple independent random values from a single VRF output.

---

### 7.2 EVM Compatibility Strategy

#### 7.2.1 Decision: Dual VM with revm (ACCEPTED with architectural boundary)

**Strategy: Dual VM -- WASM primary, EVM secondary via revm.**

Aztibase Network runs two execution environments:

1. **WASM VM (wasmtime):** Primary. All new contract development, AI integration, object model, and protocol innovation happens here.
2. **EVM (revm):** Secondary. Provides Solidity/Vyper compatibility for developer adoption and DeFi primitives migration.

**Evaluation of alternatives:**

| Strategy | Pros | Cons | Verdict |
|----------|------|------|---------|
| **Dual VM (WASM + revm)** | Access to Ethereum's massive contract ecosystem; familiar tooling (Hardhat, Foundry, Remix); DeFi primitives from day one; developers can migrate incrementally | Maintenance burden of two VMs; state interop complexity; security surface doubles | **SELECTED** |
| **EVM-to-WASM transpilation** | Single VM runtime; potential for optimization | Transpilation is lossy and fragile; Solidity's EVM assumptions (256-bit stack, storage layout) do not map cleanly to WASM; no production-proven transpiler exists; debugging is a nightmare (two layers of compilation) | REJECTED |
| **No EVM compat** | Simplest architecture; all engineering focus on WASM | Forfeits Ethereum's developer ecosystem; requires rebuilding all DeFi primitives from scratch; Move language on Sui/Aptos proves that non-EVM creates severe adoption friction (RESEARCH_BRIEF.md Section 1.1) | REJECTED |
| **EVM-only (no WASM)** | Maximum ecosystem compatibility | Cannot innovate on VM design; cannot support AI Oracle as native precompile; stuck with EVM's limitations (256-bit stack, no objects, no SIMD) | REJECTED |

**Why dual VM is worth the complexity:**

The research brief is unambiguous: "EVM compatibility" appears in 5 of 9 innovative chains, and "Developer adoption failure" is rated High likelihood / Critical impact in the risk matrix (Section 10.6). Sui and Aptos, despite technical superiority, have smaller ecosystems than EVM chains because Move created an adoption barrier. We cannot repeat this mistake.

However, EVM cannot be our primary VM because:
- The AI Oracle precompile requires a richer host function interface than EVM opcodes allow
- The object model (first-class on-chain objects) has no EVM analog
- EVM's 256-bit stack machine is architecturally limited for SIMD, complex data structures, and performance
- Browser nodes cannot run a full EVM -- WASM is the only viable browser execution target

The dual VM approach captures ecosystem access while preserving innovation freedom.

#### 7.2.2 revm Integration Architecture

**revm** (Rust EVM) is a standalone, pure-Rust EVM implementation used by Reth (Paradigm's Ethereum execution client) and several other projects. It is Apache 2.0 / MIT dual-licensed.

**Architecture:**

```
                    Transaction Router
                    (checks contract type)
                           |
              +------------+------------+
              |                         |
              v                         v
    +------------------+     +------------------+
    | WASM VM          |     | EVM (revm)       |
    | (wasmtime)       |     |                  |
    | - Native         |     | - Solidity/Vyper |
    |   contracts      |     |   contracts      |
    | - AI Oracle      |     | - ERC-20/721     |
    | - Object model   |     |   compatible     |
    | - Full host API  |     | - Standard EVM   |
    +--------+---------+     |   opcodes        |
             |               +--------+---------+
             |                        |
             +--------+-------+-------+
                      |       |
                      v       v
              +------------------+
              | Shared State     |
              | (cf_state_store) |
              | Verkle tree      |
              +------------------+
```

**Contract type identification:**

- Contracts are tagged at deployment with their VM type: `ContractType::WASM` or `ContractType::EVM`.
- The transaction router inspects the target contract's type and dispatches to the appropriate VM.
- The tag is stored as part of the contract account's metadata in cf_state_store.

**Shared state:**

Both VMs read from and write to the same underlying state store. This is critical for composability -- a WASM contract can hold ERC-20 tokens deployed on the EVM side, and vice versa.

- **EVM contracts** use the standard Ethereum storage layout (256-bit key-value slots at contract address).
- **WASM contracts** use the host function state API (arbitrary byte keys under contract address prefix).
- Both map to the same redb column family (cf_state_store) with the key prefix scheme defined in Section 4.1.2.

**Cross-VM calls:**

A WASM contract can call an EVM contract and vice versa via a **bridge precompile**:

```
WASM contract --[call_contract]--> EVM contract:
  1. WASM host function encodes the call as an EVM transaction
  2. revm executes the EVM contract
  3. Return value is decoded and returned to the WASM caller
  4. If the EVM call reverts, the WASM call receives a CallError

EVM contract --[CALL opcode to bridge address]--> WASM contract:
  1. A special bridge precompile address (0xFF...01) receives the CALL
  2. The bridge decodes the calldata as a WASM contract address + function selector + args
  3. wasmtime executes the WASM contract
  4. Return value is ABI-encoded and returned to the EVM caller
```

**Fuel costs for cross-VM calls:**

Cross-VM calls have a base cost of 25,000 fuel (higher than same-VM calls at 10,000) due to the encoding/decoding overhead and context switching between execution engines.

**EVM limitations on Aztibase Network:**

EVM contracts on Aztibase Network have access to standard EVM opcodes plus:
- `BLAKE3` precompile (in addition to standard `SHA256`, `KECCAK256`)
- Ed25519 verify precompile
- Cross-VM bridge precompile
- VRF randomness precompile (at a precompile address)

EVM contracts do NOT have access to:
- AI Oracle (must call through a WASM bridge contract)
- Object model (objects are a WASM-only primitive)
- Advanced host functions (direct state iteration, etc.)

This asymmetry is deliberate: the EVM layer is for compatibility, not for innovation. Developers who need AI features or object model access use the WASM VM.

#### 7.2.3 Migration Path for Solidity Developers

1. **Day 1:** Deploy existing Solidity contracts unchanged on the EVM layer. Use Hardhat/Foundry/Remix with a Aztibase Network RPC endpoint. Most ERC-20, ERC-721, DeFi protocols work with zero modifications.
2. **Day 1+:** Solidity contracts can call WASM contracts via the bridge precompile. Gradually move performance-critical or AI-dependent logic to WASM while keeping the Solidity frontend.
3. **Incremental:** Learn Rust or AssemblyScript for WASM development. SDKs provide familiar patterns (see Section 7.3). New features and standards are WASM-first.
4. **Long-term:** As the WASM ecosystem matures, Solidity developers optionally migrate entirely. The EVM layer remains operational indefinitely -- no forced migration.

---

### 7.3 Supported Contract Languages

#### 7.3.1 Language Matrix

| Language | Target | Developer Experience | Ecosystem | Primary Use Case |
|----------|--------|---------------------|-----------|-----------------|
| **Rust** | WASM | Steep learning curve, maximum power | Excellent (cargo, crates.io, rust-analyzer) | Complex contracts, AI integration, performance-critical logic, protocol-level extensions |
| **AssemblyScript** | WASM | TypeScript-like syntax, moderate learning curve | Growing (npm packages, familiar tooling) | Web developer onboarding, simpler contracts, prototyping |
| **Solidity** | EVM (revm) | Familiar to 200,000+ blockchain developers | Massive (OpenZeppelin, Hardhat, Foundry, Remix) | EVM-compatible DeFi, token standards, migration from Ethereum |
| **Vyper** | EVM (revm) | Python-like, auditable, limited features | Moderate (used by Curve, Yearn) | Security-focused EVM contracts |

**NO custom DSL.** Per the architect's mandate (Section 1.8): "NO custom smart contract language." The research confirms that Sui's Move and Fuel's Sway, despite technical merits, created developer adoption barriers. We use existing languages with existing tooling.

#### 7.3.2 Rust SDK (`genesis-sdk-rs`)

The primary development experience for WASM contracts.

**What it provides:**

```rust
// genesis-sdk-rs provides:

// 1. Procedural macros for contract entry points
#[genesis_contract]
mod my_token {
    use genesis_sdk::prelude::*;

    #[state]
    struct TokenState {
        name: String,
        symbol: String,
        total_supply: u128,
        balances: Map<Address, u128>,
        allowances: Map<(Address, Address), u128>,
    }

    #[init]  // Constructor -- called once at deployment
    fn init(state: &mut TokenState, name: String, symbol: String, initial_supply: u128) {
        state.name = name;
        state.symbol = symbol;
        state.total_supply = initial_supply;
        state.balances.insert(caller(), initial_supply);
    }

    #[call]  // Public function
    fn transfer(state: &mut TokenState, to: Address, amount: u128) -> Result<(), Error> {
        let from = caller();
        let from_balance = state.balances.get(&from).unwrap_or(0);
        require!(from_balance >= amount, "insufficient balance");
        state.balances.insert(from, from_balance - amount);
        let to_balance = state.balances.get(&to).unwrap_or(0);
        state.balances.insert(to, to_balance + amount);
        emit!(Transfer { from, to, amount });
        Ok(())
    }

    #[view]  // Read-only function (static call)
    fn balance_of(state: &TokenState, account: Address) -> u128 {
        state.balances.get(&account).unwrap_or(0)
    }

    // 2. Event definitions via derive macro
    #[event]
    struct Transfer {
        #[indexed] from: Address,
        #[indexed] to: Address,
        amount: u128,
    }
}
```

**SDK components:**

- `genesis-sdk-core`: Type definitions (Address, ObjectId, etc.), ABI encoding/decoding
- `genesis-sdk-macros`: Procedural macros (`#[genesis_contract]`, `#[state]`, `#[call]`, `#[view]`, `#[event]`, `#[init]`)
- `genesis-sdk-storage`: Lazy-loading state abstraction over host functions (`Map<K, V>`, `Vec<T>`, `Value<T>`)
- `genesis-sdk-ai`: AI Oracle SDK bindings (`ai_inference()`, `model_exists()`, result parsing)
- `genesis-sdk-objects`: Object model SDK (`object_create()`, `object_transfer()`, typed objects)
- `genesis-sdk-testing`: Local test harness that mocks all host functions. Contracts test without a running chain.

**Build toolchain:**

```
cargo genesis build   -- Compiles to wasm32-unknown-unknown, runs wasm-opt, size check
cargo genesis test    -- Runs tests against local mock VM
cargo genesis audit   -- Submits WASM binary to local AI auditor (tract-based)
cargo genesis deploy  -- Deploys to testnet/mainnet (compile + audit + submit tx)
```

#### 7.3.3 AssemblyScript SDK (`genesis-sdk-as`)

For web developers familiar with TypeScript.

**What it provides:**

```typescript
// genesis-sdk-as provides:
import { Address, Map, caller, emit, require, state } from "genesis-sdk-as";

@contract
class MyToken {
    name: string = "";
    symbol: string = "";
    totalSupply: u128 = 0;
    balances: Map<Address, u128> = new Map();

    @init
    constructor(name: string, symbol: string, initialSupply: u128) {
        this.name = name;
        this.symbol = symbol;
        this.totalSupply = initialSupply;
        this.balances.set(caller(), initialSupply);
    }

    @call
    transfer(to: Address, amount: u128): void {
        const from = caller();
        const fromBalance = this.balances.get(from) || 0;
        require(fromBalance >= amount, "insufficient balance");
        this.balances.set(from, fromBalance - amount);
        this.balances.set(to, (this.balances.get(to) || 0) + amount);
        emit("Transfer", [from, to], amount);
    }

    @view
    balanceOf(account: Address): u128 {
        return this.balances.get(account) || 0;
    }
}
```

**Tradeoffs vs. Rust:**
- Easier onboarding for web developers (TypeScript syntax)
- Smaller compiled WASM binaries (AssemblyScript produces lean WASM)
- Less powerful: no advanced generics, limited trait/interface system, less mature tooling
- Recommended for: simple token contracts, NFT marketplaces, governance participation, dApp frontend contracts

#### 7.3.4 Solidity/Vyper (EVM Layer)

Standard Solidity/Vyper compilation targeting the EVM. Developers use existing toolchains:

- **Hardhat:** With `genesis-hardhat-plugin` providing network configuration, deployment scripts, and verification integration
- **Foundry:** With `genesis-foundry-plugin` providing `forge test` and `forge deploy` support
- **Remix:** Via the Aztibase Network RPC endpoint (standard JSON-RPC compatible)
- **OpenZeppelin:** Standard OpenZeppelin contracts (ERC-20, ERC-721, AccessControl, etc.) deploy unchanged

---

### 7.4 Contract Standards

Aztibase Network defines native standards for common contract patterns. These are reference implementations in the Rust SDK with equivalent AssemblyScript and (where applicable) EVM implementations.

#### 7.4.1 GEN-20: Fungible Token Standard

**Equivalent to:** ERC-20 (with improvements)

```rust
// GEN-20 interface
trait Gen20 {
    fn name() -> String;
    fn symbol() -> String;
    fn decimals() -> u8;
    fn total_supply() -> u128;
    fn balance_of(account: Address) -> u128;
    fn transfer(to: Address, amount: u128) -> Result<(), Error>;
    fn approve(spender: Address, amount: u128) -> Result<(), Error>;
    fn allowance(owner: Address, spender: Address) -> u128;
    fn transfer_from(from: Address, to: Address, amount: u128) -> Result<(), Error>;

    // Improvements over ERC-20:
    fn transfer_and_call(to: Address, amount: u128, data: Vec<u8>) -> Result<Vec<u8>, Error>;
    // ^ Combines transfer + notification in a single call (prevents the approve/transferFrom dance)
    // Inspired by ERC-677 and ERC-1363

    fn permit(owner: Address, spender: Address, amount: u128, deadline: u64, signature: Signature) -> Result<(), Error>;
    // ^ Gasless approvals via signature (inspired by ERC-2612)
}
```

**Key improvements over ERC-20:**
- `transfer_and_call` eliminates the common two-transaction pattern (approve + transferFrom). Reduces gas cost and improves UX.
- `permit` enables gasless token approvals via off-chain signatures.
- Overflow-safe by default (Rust's u128 does not overflow silently; the SDK wraps arithmetic in checked operations).

#### 7.4.2 GEN-721: Non-Fungible Token Standard

**Equivalent to:** ERC-721/ERC-1155 (unified)

```rust
// GEN-721 interface (supports both single NFTs and semi-fungible tokens)
trait Gen721 {
    fn name() -> String;
    fn symbol() -> String;

    // Single NFTs (ERC-721 equivalent)
    fn owner_of(token_id: u256) -> Option<Address>;
    fn transfer(to: Address, token_id: u256) -> Result<(), Error>;
    fn approve(spender: Address, token_id: u256) -> Result<(), Error>;

    // Semi-fungible (ERC-1155 equivalent)
    fn balance_of_batch(accounts: Vec<Address>, token_ids: Vec<u256>) -> Vec<u128>;
    fn transfer_batch(to: Address, token_ids: Vec<u256>, amounts: Vec<u128>) -> Result<(), Error>;

    // Metadata
    fn token_uri(token_id: u256) -> Option<String>;

    // Genesis-specific: Object model integration
    fn as_object(token_id: u256) -> Option<ObjectId>;
    // ^ Returns the on-chain object ID for NFTs backed by first-class objects.
    // This bridges the NFT standard with the hybrid account+object model.
}
```

**Object model integration:** NFTs on Aztibase Network can optionally be backed by first-class on-chain objects (Section 1.2.3). This means an NFT is not just a mapping entry in a contract -- it is an independent protocol-level object with its own ownership, transferability, and parallel execution properties. The `as_object()` method bridges the standard token interface with the object model.

#### 7.4.3 GEN-AGENT: AI Agent Contract Standard (NEW)

This is a Aztibase Network-specific standard with no equivalent in existing blockchain ecosystems. It defines how AI agents interact with smart contracts.

```rust
// GEN-AGENT interface
trait GenAgent {
    // Identity
    fn agent_id() -> AgentId;           // On-chain agent identity (protocol primitive)
    fn owner() -> Address;              // Human principal who controls this agent
    fn kya_attestation() -> Hash;       // Know Your Agent attestation hash

    // Capabilities
    fn capabilities() -> Vec<Capability>; // What this agent is authorized to do
    fn is_capable(cap: &Capability) -> bool;

    // Constraints (enforced by the VM, not just by convention)
    fn spending_limit_per_epoch() -> u128;   // Max GEN tokens spendable per epoch
    fn spent_this_epoch() -> u128;           // Amount already spent
    fn allowed_contracts() -> Vec<Address>;  // Contracts this agent may call (whitelist)
    fn allowed_models() -> Vec<ModelHash>;   // AI models this agent may invoke

    // Operations
    fn execute_action(action: AgentAction) -> Result<ActionResult, AgentError>;
    // ^ The agent's main entry point. Actions are typed and validated against capabilities.

    fn request_inference(
        model_id: ModelHash,
        input: Vec<u8>,
        max_fuel: u64,
    ) -> Result<InferenceResult, AgentError>;
    // ^ Convenience wrapper around ai_inference() with agent-specific constraints applied.

    // Governance
    fn update_constraints(new_constraints: AgentConstraints, owner_signature: Signature) -> Result<(), Error>;
    // ^ Only the owner can update constraints. Requires signature proof.

    fn pause() -> Result<(), Error>;   // Owner can pause the agent
    fn resume() -> Result<(), Error>;  // Owner can resume the agent
    fn terminate() -> Result<(), Error>; // Owner can permanently terminate the agent
}

// Agent capabilities enum
enum Capability {
    TransferTokens { max_per_tx: u128 },
    CallContract { address: Address, methods: Vec<[u8; 4]> },
    RequestInference { models: Vec<ModelHash> },
    CreateObjects { type_tags: Vec<String> },
    AgentToAgentPayment { max_per_tx: u128 },
}
```

**VM enforcement:** Agent constraints are not just advisory. The execution engine checks every host function call against the agent's declared capabilities:
- If an agent calls `transfer()` exceeding its `spending_limit_per_epoch`, the call reverts.
- If an agent calls a contract not in its `allowed_contracts` whitelist, the call reverts.
- If an agent invokes `ai_inference()` with a model not in its `allowed_models`, the call reverts.

These checks happen at the host function level, not in contract code, so they cannot be bypassed.

#### 7.4.4 GEN-GOV: Governance Contract Standard

```rust
// GEN-GOV interface
trait GenGov {
    // Proposal lifecycle
    fn propose(description: String, actions: Vec<GovernanceAction>) -> Result<ProposalId, Error>;
    fn vote(proposal_id: ProposalId, support: VoteType, reason: Option<String>) -> Result<(), Error>;
    fn execute(proposal_id: ProposalId) -> Result<(), Error>;
    fn cancel(proposal_id: ProposalId) -> Result<(), Error>;

    // Delegation
    fn delegate(to: Address) -> Result<(), Error>;
    fn undelegate() -> Result<(), Error>;

    // Queries
    fn proposal_state(proposal_id: ProposalId) -> ProposalState;
    fn voting_power(account: Address) -> u128;
    fn quorum() -> u128;

    // Timelock
    fn timelock_delay() -> u64;  // Rounds between proposal passage and execution
}

enum VoteType { For, Against, Abstain }
enum ProposalState { Pending, Active, Succeeded, Defeated, Queued, Executed, Cancelled, Expired }
```

This follows the Governor pattern (OpenZeppelin) adapted for Aztibase Network's DAG-based timing (rounds instead of block numbers).

#### 7.4.5 GEN-MULTI: Multi-Signature and Account Abstraction Standard

```rust
// GEN-MULTI interface
trait GenMulti {
    fn owners() -> Vec<Address>;
    fn threshold() -> u32;  // n-of-m threshold for execution

    fn submit_transaction(to: Address, value: u128, data: Vec<u8>) -> Result<TxId, Error>;
    fn confirm(tx_id: TxId) -> Result<(), Error>;
    fn revoke_confirmation(tx_id: TxId) -> Result<(), Error>;
    fn execute(tx_id: TxId) -> Result<Vec<u8>, Error>;

    // Account abstraction: custom signature verification
    fn validate_user_op(user_op: UserOperation) -> Result<(), Error>;
    // ^ Allows the multi-sig to act as a smart account with custom validation logic.
    // Supports social recovery, hardware key verification, passkey authentication.

    // Management
    fn add_owner(owner: Address) -> Result<(), Error>;      // Requires threshold signatures
    fn remove_owner(owner: Address) -> Result<(), Error>;   // Requires threshold signatures
    fn change_threshold(new_threshold: u32) -> Result<(), Error>;
}
```

Account abstraction is a native feature: any contract implementing `validate_user_op` can serve as a smart account. This follows the ERC-4337 pattern but is protocol-native (not requiring a separate bundler/entrypoint infrastructure).

---

### 7.5 Contract Lifecycle

#### 7.5.1 Deployment Process

```
Developer workflow:

1. WRITE contract in Rust / AssemblyScript / Solidity
   -- Use genesis-sdk or standard Solidity toolchain

2. COMPILE to WASM or EVM bytecode
   -- Rust:           cargo genesis build -> contract.wasm
   -- AssemblyScript:  npx genesis-as build -> contract.wasm
   -- Solidity:        forge build / hardhat compile -> contract.evm

3. LOCAL TEST
   -- Rust:           cargo genesis test (mock VM)
   -- AssemblyScript:  npx genesis-as test
   -- Solidity:        forge test / hardhat test

4. AI AUDIT (pre-deployment, mandatory)
   -- Submit WASM/EVM bytecode to the on-chain AI audit system
   -- AI auditor (tract-based, running on PoUW validators) analyzes:
      -- Reentrancy vulnerabilities
      -- Integer overflow/underflow patterns
      -- Unchecked external calls
      -- State manipulation risks
      -- Access control gaps
      -- Economic attack vectors (flash loan patterns, price oracle manipulation)
      -- Gas/fuel griefing patterns
   -- Result: PASS / WARNING / FAIL
      -- PASS:    Contract can deploy. Audit attestation stored on-chain.
      -- WARNING: Contract can deploy with a warning flag. Flag is visible to users.
      -- FAIL:    Contract deployment is BLOCKED until issues are resolved.
   -- Audit cost: paid in GEN tokens (fuel cost of AI inference for the audit)

5. DEPLOY transaction
   -- Transaction type: ContractDeploy
   -- Fields: bytecode, constructor_args, contract_type (WASM|EVM), audit_attestation_id
   -- Validator verifies: audit_attestation_id is valid and matches the bytecode hash
   -- Contract address: BLAKE3(deployer_address || deployer_nonce)
   -- Constructor executes. Initial state is written.

6. POST-DEPLOYMENT
   -- Contract is live and callable
   -- Contract metadata (source hash, audit result, deploy tx) is queryable
   -- Runtime AI monitoring begins (Section 7.6)
```

**Audit bypass for governance-approved contracts:**

Protocol-level contracts (token contract, governance contract, staking contract) deployed via governance proposal bypass the AI audit (they are audited through the governance review process). All user-deployed contracts go through AI audit.

#### 7.5.2 Upgrade Mechanism

Aztibase Network supports **native contract upgradability** without the proxy pattern overhead.

**Approach: Versioned code with immutable storage**

```
ContractAccount {
    address:         Address,
    code_version:    u64,           // Incremented on each upgrade
    code_hash:       Hash,          // Hash of current WASM/EVM bytecode
    code:            Vec<u8>,       // Current bytecode
    storage_root:    VerkleRoot,    // Contract storage (persistent across upgrades)
    upgrade_authority: UpgradeAuth, // Who can upgrade this contract
    is_frozen:       bool,          // Immutable flag (opt-in, irreversible)
}

enum UpgradeAuth {
    Owner(Address),                          // Single owner can upgrade
    MultiSig { addresses: Vec<Address>, threshold: u32 }, // Multi-sig upgrade
    Governance(ProposalId),                  // Only governance can upgrade
    Timelock { authority: Address, delay: u64 }, // Upgrade with timelock delay
    Immutable,                               // Cannot be upgraded (opt-in, irreversible)
}
```

**Upgrade process:**

```
1. New bytecode compiled and AI-audited (same as fresh deployment)
2. Upgrade transaction submitted by the upgrade_authority
3. Validator verifies:
   -- Caller matches upgrade_authority
   -- New bytecode passes AI audit
   -- If Timelock: sufficient delay has elapsed since upgrade proposal
   -- Contract is not frozen (is_frozen == false)
4. code_hash and code are updated. code_version increments.
5. Storage is PRESERVED. The new code operates on the existing storage.
6. An optional migration function can be called atomically with the upgrade:
   fn migrate(old_version: u64, new_version: u64) -> Result<(), Error>;
```

**Why native upgradability over proxy patterns:**

- Proxy patterns (delegatecall to implementation) add 2,600+ gas overhead per call in EVM.
- Proxy patterns are a common source of vulnerabilities (storage collision, uninitialized proxy).
- Native upgradability is simpler, cheaper, and auditable (the upgrade authority is explicit on-chain).
- The `Immutable` option allows contracts to opt into permanent immutability, satisfying DeFi's "code is law" requirements.

**EVM contracts** use the standard proxy pattern (TransparentProxy / UUPS) since EVM does not support native upgradability. The WASM advantage is a migration incentive.

#### 7.5.3 Contract Storage Model (Mapping to Verkle Trees)

Contract state maps to the Verkle tree through the dual storage architecture:

```
Contract storage key in cf_state_store:
  0x04 || contract_address (32 bytes) || storage_key (32 bytes)

For WASM contracts:
  storage_key = BLAKE3(user_defined_key)
  -- The SDK's Map<K,V> hashes keys before storage to ensure uniform key distribution.

For EVM contracts:
  storage_key = keccak256(slot_number || mapping_key)
  -- Standard Ethereum storage layout (solidity compiler-determined).

Both map to the same Verkle tree structure:
  Verkle path = BLAKE3(0x04 || contract_address || storage_key)[:31]
  -- 31-byte path ensures uniform distribution across the Verkle tree.
```

**State costs:**
- First write to a new storage slot: 2,500 fuel + 20,000 fuel "cold storage" surcharge (total: 22,500). This covers the cost of adding a new leaf to the Verkle tree.
- Subsequent writes to an existing slot: 2,500 fuel (warm storage, Verkle leaf update only).
- Read from an existing slot: 500 fuel (cold) or 100 fuel (warm, already accessed in this transaction).
- Delete: 500 fuel, with a 10,000 fuel refund (capped at 50% of total transaction fuel). Incentivizes state cleanup.

The cold/warm distinction follows Ethereum's EIP-2929 pattern: the first access to a storage slot in a transaction is "cold" (requires Verkle tree traversal from root); subsequent accesses are "warm" (cached in the execution context).

#### 7.5.4 Cross-Contract Invocation

**Call semantics:**

| Call Type | State Context | msg.sender | Use Case |
|-----------|--------------|------------|----------|
| `call_contract` | Callee's storage | Caller's address | Standard contract interaction |
| `delegate_call` | Caller's storage | Original caller | Library pattern, proxy upgrades |
| `static_call` | Read-only (any mutation reverts) | Caller's address | Safe read-only queries |

**Reentrancy protection:**

Aztibase Network implements **protocol-level reentrancy protection** rather than relying on contract-level guards:

1. **Call depth limit:** Maximum 32 nested calls. Exceeding this reverts the entire transaction.
2. **Reentrancy flag (opt-in):** Contracts can declare `#[no_reentrancy]` on functions. The VM tracks a per-contract reentrancy flag; if a function marked `#[no_reentrancy]` is called while the same contract is already on the call stack, the call reverts.
3. **Checks-effects-interactions enforcement:** The AI audit (Section 7.6) checks for reentrancy patterns and flags violations. This is not runtime-enforced but provides a strong pre-deployment safety net.
4. **State snapshot on call:** Before each cross-contract call, the VM snapshots the caller's state. If the callee reverts, the caller's state is restored to the snapshot. This is standard EVM behavior, extended to WASM.

**Atomic composability:**

All cross-contract calls within a single transaction are atomic. If any call in the chain reverts, the entire transaction reverts. This ensures composability guarantees identical to Ethereum's.

---

### 7.6 AI-Powered Auditing Integration

#### 7.6.1 Pre-Deployment AI Audit

**What it checks:**

The AI audit system uses a `tract`-based inference pipeline running on PoUW validators. The audit model is trained on a dataset of known vulnerable contracts and security patterns. This integrates with the AI integration layer's AuditModel pipeline (Section 6) -- the smart contract layer defines WHAT is checked; the AI layer defines HOW the model runs.

| Check Category | Specific Checks | Severity |
|----------------|-----------------|----------|
| **Reentrancy** | External calls before state updates; cross-contract call patterns; delegate_call into untrusted code | CRITICAL |
| **Integer safety** | Unchecked arithmetic (relevant for AssemblyScript; Rust SDK uses checked math by default); overflow/underflow in token operations | HIGH |
| **Access control** | Missing owner checks on privileged functions; unprotected upgrade functions; unrestricted minting | HIGH |
| **Economic attacks** | Flash loan vulnerability patterns; price oracle manipulation; sandwich attack susceptibility | HIGH |
| **State manipulation** | Unauthorized storage writes; storage collision in proxy patterns (EVM); unbounded loops over state | MEDIUM |
| **Fuel griefing** | Patterns that consume excessive fuel on failure (forcing callers to pay for failed operations); unbounded iteration | MEDIUM |
| **Logic errors** | Incorrect comparison operators; off-by-one errors; missing edge case handling | WARNING |
| **Code quality** | Unused storage slots; dead code; unnecessarily complex patterns | INFO |

**Audit result structure:**

```
AuditAttestation {
    bytecode_hash:    Hash,          // Hash of the audited bytecode
    audit_model_id:   ModelHash,     // Which audit model was used
    result:           AuditResult,   // PASS | WARNING | FAIL
    findings:         Vec<AuditFinding>, // Detailed findings
    confidence:       f32,           // Model confidence (0.0-1.0)
    auditor_validators: Vec<ValidatorId>, // Which PoUW validators performed the audit
    timestamp:        u64,
}

struct AuditFinding {
    category:    String,       // "reentrancy", "access_control", etc.
    severity:    Severity,     // CRITICAL, HIGH, MEDIUM, WARNING, INFO
    location:    CodeLocation, // Byte offset in WASM or instruction offset in EVM
    description: String,       // Human-readable description
    suggestion:  Option<String>, // Suggested fix
}
```

**Audit economics:**
- Audit cost is proportional to bytecode size and complexity (estimated by the number of functions and control flow paths).
- Typical audit: 10,000-100,000 fuel (roughly $0.10-$1.00 at expected fee levels).
- The AI audit is NOT a substitute for human security audits for high-value contracts. It is a baseline safety net that catches common patterns.

**False positive handling:**
- Developers can submit an **audit appeal** if they believe a FAIL result is a false positive.
- The appeal routes to a second, independent audit model (different model architecture, different training data).
- If the second model passes, the contract deploys with a "PASS_ON_APPEAL" flag.
- Persistent disagreement between models flags the contract for community human review.

#### 7.6.2 Runtime Monitoring Hooks

The AI layer observes contract execution in real-time through monitoring hooks embedded in the execution engine:

**Hook points:**

```
1. PRE_EXECUTION:  Before each transaction's contract calls begin
   - Input: transaction data, caller, target contract, value
   - AI checks: known attack pattern matching, abnormal value transfers

2. STATE_DIFF:     After execution, before state commit
   - Input: set of state changes (key, old_value, new_value)
   - AI checks: anomalous state changes (e.g., token supply changed without mint event,
                 balance jumped by >1000x, ownership transferred without approval)

3. CROSS_CONTRACT: On each cross-contract call
   - Input: caller, callee, function selector, value, call depth
   - AI checks: unusual call patterns (e.g., contract calling itself repeatedly,
                 calls to known exploit contracts in a blocklist)

4. POST_EXECUTION: After all calls complete, before receipt finalization
   - Input: execution trace summary, total fuel consumed, events emitted
   - AI checks: aggregate anomaly detection (combination of factors that individually
                 seem normal but together indicate an exploit)
```

**Performance budget:** Runtime monitoring must fit within the 400ms round budget. The monitoring model is lightweight (anomaly detection, not deep analysis). Target: <5ms overhead per transaction for AI monitoring. This is achievable with `tract` running a small classifier model.

**Monitoring is advisory, not blocking** (by default). Monitoring findings are logged but do not halt execution. The circuit breaker (Section 7.6.3) is the mechanism for halting.

#### 7.6.3 Circuit Breaker: AI-Triggered Execution Halt

**Can AI halt a contract mid-execution?**

**Answer: No.** A contract cannot be halted mid-execution within a single transaction. WASM execution is atomic -- once a transaction begins executing, it runs to completion or reverts. Interrupting execution mid-way would violate atomicity guarantees.

**What CAN happen:**

The circuit breaker operates at the **transaction admission** level, not the execution level:

```
Circuit Breaker Architecture:

1. DETECTION: Runtime monitoring (Section 7.6.2) detects anomaly
   -- Single-transaction anomaly: flagged but not halted (too late)
   -- Multi-transaction pattern: detected across recent rounds

2. ALERT: Monitoring system emits a CircuitBreakerAlert
   -- contract_address: Address of suspicious contract
   -- threat_level: LOW | MEDIUM | HIGH | CRITICAL
   -- evidence: Vec<AnomalyEvidence>
   -- recommended_action: MONITOR | THROTTLE | PAUSE

3. RESPONSE (based on threat_level):

   LOW: Log only. No action.

   MEDIUM: THROTTLE
   -- Future transactions targeting this contract are deprioritized in mempool
   -- Maximum fuel per transaction to this contract is reduced by 50%
   -- Alert is broadcast to validators via gossip

   HIGH: PAUSE (requires validator quorum)
   -- A PauseProposal is submitted to the active validator set
   -- If 2/3 of validators (by stake) sign the PauseProposal within 10 rounds:
      -- The contract is PAUSED: all calls to it revert immediately
      -- The pause is recorded on-chain with evidence
      -- The contract owner is notified
   -- UNPAUSE requires either:
      -- Contract owner submits fix + passes AI audit, AND 2/3 validators approve
      -- OR governance proposal to unpause (community override)
   -- False positive protection: contract owner can submit an emergency appeal
      that triggers human review within 24 hours

   CRITICAL: EMERGENCY PAUSE (fast path)
   -- If 3 or more validators independently detect the same CRITICAL anomaly:
      -- Each submits an EmergencyPauseVote in their next DAG vertex
      -- If 1/3+1 of validators submit EmergencyPauseVote within 5 rounds:
         -- Contract is immediately paused (lower threshold than HIGH)
      -- Full 2/3 quorum must confirm within 100 rounds or the pause is lifted
   -- This fast path exists for active exploits where waiting for 2/3 is too slow
```

**Key design principles:**
- **No single entity can pause a contract.** Even CRITICAL requires 1/3+1 validator agreement.
- **Pausing is temporary and reversible.** The contract owner always has a path to resolution.
- **The AI recommends; validators decide.** AI detection is the trigger, but the pause action requires validator consensus. AI alone cannot halt anything.
- **False positive cost is bounded.** A false-positive pause results in temporary (minutes to hours) contract unavailability, not permanent damage. The appeal process provides recourse.

---

### 7.7 Gas Model (Detailed)

#### 7.7.1 Fee Structure Summary

```
Transaction fee = fuel_consumed * fee_per_fuel

fee_per_fuel = base_fee + priority_fee

base_fee:     Protocol-determined, adjusts per anchor based on utilization
              Burned (deflationary)

priority_fee: User-specified tip to validator
              Goes to the block proposer (validator who included the tx)

fuel_consumed: Actual fuel used during execution (not the max_fuel specified)
               Unused fuel is NOT charged
```

#### 7.7.2 Operation Pricing Philosophy

| Category | Pricing Principle | Rationale |
|----------|-------------------|-----------|
| **Compute** | Proportional to CPU cycles | Prevents compute DoS |
| **State reads** | Cost reflects I/O + Verkle proof generation burden | Reads are cheap but not free; proof generation serves light clients |
| **State writes** | 5x reads; includes Verkle tree update cost | Writes are expensive: they grow state size, increase proof costs, and require disk I/O |
| **State creation** | Surcharge on first write to new slot | New state entries grow the Verkle tree permanently (until deleted) |
| **State deletion** | Refund (capped at 50% of tx fuel) | Incentivize state cleanup to reduce long-term storage burden |
| **AI inference** | Market-priced; base cost + model-specific multiplier | AI compute is heterogeneous; one-size-fits-all pricing does not work |
| **Cross-VM calls** | 2.5x same-VM calls | Encoding/decoding and context switch overhead |
| **Event emission** | Per-byte cost | Events are stored in receipts; cost reflects storage impact |

#### 7.7.3 AI Inference Gas Pricing

AI inference is special because its cost varies by orders of magnitude depending on the model:

```
ai_inference_fuel = base_inference_cost + (model_complexity * compute_multiplier)

base_inference_cost = 50,000 fuel  (covers: host call overhead, request routing, result verification)

model_complexity:
  Tier 1 (small classifiers, <10M params):    multiplier = 1x     (total ~50,000-100,000 fuel)
  Tier 2 (medium models, 10M-100M params):    multiplier = 10x    (total ~500,000-1,000,000 fuel)
  Tier 3 (large models, 100M-1B params):      multiplier = 100x   (total ~5,000,000-10,000,000 fuel)
  Tier 4 (very large, >1B params):            multiplier = 1000x  (total ~50,000,000-100,000,000 fuel)

compute_multiplier is stored in the ModelRegistry for each registered model.
It is initially set by the model registrant and validated by PoUW validators
through benchmarking. Governance can override if gaming is detected.
```

**Why not purely dynamic?** Pure dynamic pricing (auction-based) creates unpredictable costs for contract developers. The tier-based multiplier provides cost predictability while the base fee mechanism handles overall congestion. Model registrants set the compute multiplier based on actual inference cost; validators verify through execution.

#### 7.7.4 Relationship to Tokenomics

The gas model interfaces with tokenomics (Section 3) through three channels:

1. **Fee burn (base fee):** All base fees are burned. This creates deflationary pressure proportional to chain utilization. The tokenomics-engineer determines the initial base fee level.
2. **Validator tips (priority fee):** Tips go to the vertex proposer who included the transaction. This is a direct incentive for validators to include transactions.
3. **PoUW inference fees:** Inference fees are split: a protocol fee (%) is burned, and the remainder goes to the PoUW validator who performed the inference. The split ratio is governance-adjustable.

---

### 7.8 Stack Evaluation and Challenges

#### 7.8.1 wasmtime: ACCEPTED

wasmtime is the correct WASM runtime for Aztibase Network's contract VM. Its Bytecode Alliance governance, fuel-based metering, determinism guarantees, and security posture are superior to wasmer for blockchain use. See Section 7.1.1 for the full evaluation.

No challenge raised. The architect's stack ruling specifying wasmtime (Section 1.8) is confirmed.

#### 7.8.2 revm: ACCEPTED with architectural boundary

revm provides EVM compatibility at acceptable engineering cost. The dual-VM architecture (Section 7.2) isolates the EVM layer as a secondary execution environment.

**Dependency assessment:**
- revm is Apache 2.0 / MIT dual-licensed (compatible per LEGAL_LANDSCAPE.md)
- Maintained by Paradigm (active, well-funded maintainer)
- Used by Reth, which is becoming a reference Ethereum execution client
- Binary size impact: ~3-5MB additional
- Update cadence: revm must track EVM spec changes (Ethereum hard forks). This is ongoing maintenance cost but bounded -- we do not need to track every EVM precompile, only those relevant to our users.

**Risk flag:** revm tracks Ethereum's EVM spec. If Ethereum makes breaking EVM changes (unlikely but possible in a post-Verkle world), we must decide whether to follow. Recommendation: follow Ethereum mainnet EVM spec with a 6-month delay, giving time to assess impact.

No challenge raised.

#### 7.8.3 STACK CHALLENGE: wasmtime version pinning

> **STACK CHALLENGE [smart-contract-engineer -> blockchain-architect]: wasmtime Version Determinism**
>
> **What is challenged:** Implicit assumption that any wasmtime version can be used.
>
> **Proposed constraint:** The protocol MUST pin a specific wasmtime version per epoch and all validators MUST use the same version. wasmtime version upgrades are protocol upgrades (governance-approved).
>
> **Justification:**
>
> wasmtime's Cranelift compiler may produce different native code across versions (optimizations change, bug fixes alter codegen). While the WASM semantics are identical, fuel consumption may differ if the compiler generates different instruction sequences for the same WASM input. Two validators running different wasmtime versions could disagree on the exact fuel consumed by a contract, causing consensus failure.
>
> This is not a theoretical concern. Ethereum's go-ethereum vs Nethermind client diversity has caused consensus splits due to subtle execution differences (2023 Shapella incident). For a single-client chain in early development, version pinning is the correct approach.
>
> **Proposed mechanism:**
> - The protocol specifies a `vm_version` in the genesis config and subsequent governance proposals.
> - `vm_version` maps to an exact wasmtime release (e.g., `vm_version: 3` -> wasmtime 28.0.0).
> - Validators that connect with a mismatched `vm_version` are rejected by peers.
> - Version upgrades follow the standard governance proposal flow with a mandatory 7-day testnet soak period.
>
> **Impact:** Limits wasmtime upgrade velocity. Acceptable tradeoff for consensus safety.

#### 7.8.4 Browser Node Contract Interaction: ASSESSED

Browser nodes cannot execute contracts locally (they are light clients). Browser node interaction with contracts works through:

1. **Read operations:** Browser node requests state via Verkle proof from a full node. Full node returns the state value + proof. Browser node verifies the proof against the finalized state root. This works for querying balances, token ownership, governance proposal state, etc.

2. **Write operations:** Browser node constructs and signs a transaction locally, then submits it to the network via gossip. The transaction is executed by validators. The browser node monitors for the transaction's inclusion in a committed anchor and verifies via finality certificate + receipt proof.

3. **Simulation (optional):** For UX purposes (e.g., estimating fuel cost), the browser node can request a `call` simulation from a full node. The full node simulates the contract call and returns the result + estimated fuel. The browser node cannot verify the simulation result cryptographically (it is off-chain), but can submit the actual transaction and verify the on-chain result.

This is consistent with the node architecture's design (Section 4.3.4) and does not require changes to the contract layer.

---

### 7.9 Parameter Summary

| Parameter | Default Value | Governance-Adjustable | Rationale |
|-----------|---------------|----------------------|-----------|
| Max WASM memory | 16 MB (256 pages) | Yes (range: 4-64 MB) | Balance between contract capability and node resource protection |
| Max stack depth | 1,024 frames | Yes (range: 256-4096) | Prevent stack overflow; allow complex call chains |
| Max fuel per tx | 100,000,000 | Yes | Upper bound on single-tx computation |
| Max WASM binary size | 2 MB compressed | Yes (range: 512KB-8MB) | Keep deployment costs reasonable |
| Max execution wall time | 5 seconds | Yes (range: 1-30 seconds) | Hard safety net for pathological fuel undercount |
| Max cross-contract depth | 32 | Yes (range: 8-64) | Balance composability with reentrancy risk |
| Cold state read fuel | 500 | Yes | Adjustable as hardware improves |
| Cold state write fuel | 2,500 | Yes | Adjustable as hardware improves |
| New storage slot surcharge | 20,000 | Yes | Incentivize state-efficient contracts |
| State deletion refund | 10,000 (capped 50% of tx fuel) | Yes | Incentivize cleanup |
| Cross-contract call base fuel | 10,000 | Yes | Context switch overhead |
| Cross-VM call base fuel | 25,000 | Yes | Encoding + context switch overhead |
| Base fee adjustment rate | +/- 12.5% per anchor | Yes (range: 1%-25%) | EIP-1559 style, tuned for 400ms rounds |
| AI audit requirement | Mandatory for user contracts | Yes (can be relaxed via governance) | Baseline safety net |
| Circuit breaker HIGH threshold | 2/3 validator agreement | No (hardcoded BFT assumption) | Security-critical parameter |
| Circuit breaker CRITICAL threshold | 1/3+1 validator agreement | Yes (range: 1/4 to 1/2) | Fast-path for active exploits |
| wasmtime version | Pinned per governance | Yes (governance proposal required) | Consensus determinism |

---

**Stack challenges raised:**
- STACK CHALLENGE: wasmtime version pinning -- all validators MUST use the same wasmtime version, upgraded via governance. See Section 7.8.3 for full justification.

**Conflicts raised:** None

**Sign-off:** smart-contract-engineer

## SECTION 8: P2P NETWORK DESIGN

---
**Contributed by:** p2p-network-engineer
**Date:** 2026-03-05
**Status:** DRAFT
**Dependencies met:** RESEARCH_BRIEF.md (complete, including Section 7 server-independence and libp2p maintenance crisis flag), Section 0 stack rulings (reviewed, including libp2p COMPROMISE ruling with NetworkTransport abstraction), Section 1 architecture (reviewed, including Section 1.4 server-independence and Section 1.4.5 incentivized relays), Section 2 consensus (reviewed, including SynBFT 400ms rounds, 200 validators, DAG vertex propagation, BLS finality certificates), Section 4 node architecture (reviewed, including all five node types, bandwidth flag, WebRTC browser transport, NAT traversal for validators)

---

### P2P NETWORK DESIGN

Aztibase Network's networking layer is the foundation upon which server-independence stands or falls. Every design decision below has been evaluated against the zero-central-server mandate. The P2P layer must simultaneously serve five node types with radically different resource profiles, propagate DAG vertices within 400ms round budgets, traverse consumer NATs reliably, and resist network-layer attacks -- all without a single centralized component after initial bootstrap.

---

### 8.1 Protocol Stack

#### 8.1.1 Stack Summary

| Layer | Choice | Justification |
|-------|--------|---------------|
| **Framework** | rust-libp2p with `NetworkTransport` abstraction trait | Per architect's stack ruling (Section 0). rust-libp2p has independent, active maintainers separate from the Shipyard-affected go/js implementations. The `NetworkTransport` trait provides backend swappability if rust-libp2p degrades. |
| **Transport (Full/Validator)** | QUIC (primary), TCP+Noise (fallback) | QUIC provides 0-RTT connection resumption, built-in encryption (TLS 1.3), multiplexed streams, and UDP-based NAT friendliness. TCP+Noise fallback for environments where UDP is blocked. |
| **Transport (Browser)** | WebRTC DataChannels via libp2p-webrtc | Only viable browser-to-peer transport without server dependency. WebSocket fallback to full nodes when WebRTC fails. |
| **Transport (Mobile)** | QUIC (primary), TCP+Noise (fallback) | Same as full node. QUIC's 0-RTT resumption is critical for intermittent mobile connectivity -- reconnection in 1 RTT vs TCP's 3-way handshake. |
| **Discovery** | Kademlia DHT (primary), DNS seeds (secondary), mDNS (local), peer caching (persistence) | Layered strategy per Section 1.4.1. No single discovery mechanism is a dependency. |
| **Gossip** | Gossipsub v1.1 with priority batching and flow control | Production-proven in Ethereum. Mesh-based gossip with peer scoring. Priority batching mitigates the stress-performance degradation flagged in RESEARCH_BRIEF.md Section 7.2. |
| **Encryption** | Noise XX handshake (TCP), TLS 1.3 (QUIC), DTLS (WebRTC) | All connections encrypted by default. Noise XX for TCP provides mutual authentication with forward secrecy. QUIC and WebRTC handle their own encryption natively. |
| **Identity** | libp2p Identify protocol + Ed25519 PeerIds | PeerId derived from Ed25519 public key. Identify protocol exchanges supported protocols and listen addresses on connection. |
| **Stream Multiplexing** | yamux (TCP), native (QUIC) | yamux provides multiplexed streams over TCP. QUIC provides native stream multiplexing -- no additional muxer needed. |

#### 8.1.2 Transport Selection by Node Type

| Node Type | Primary Transport | Secondary Transport | Why |
|-----------|------------------|--------------------|----|
| **Full Node** | QUIC (UDP/443) | TCP+Noise+yamux (TCP/443) | QUIC for performance and NAT friendliness. TCP fallback when UDP is blocked by enterprise firewalls (~5-10% of networks). |
| **Validator** | QUIC (UDP/443) | TCP+Noise+yamux (TCP/443) | Same as full node. Validators have no special transport requirements -- consensus messages ride on the same streams as block data. |
| **Light Node** | QUIC (UDP/443) | TCP+Noise+yamux (TCP/443) | Same as full node but with fewer concurrent streams and lower bandwidth allocation. |
| **Browser Node** | WebRTC DataChannels | WebSocket (wss://) to full nodes | WebRTC is the only browser transport that enables true P2P without a server. WebSocket fallback connects to full nodes directly -- less decentralized but functional. |
| **Mobile Node** | QUIC (UDP/443) | TCP+Noise+yamux (TCP/443) | QUIC's 0-RTT resumption is critical. Mobile connections drop frequently; QUIC resumes in 1 RTT where TCP requires full handshake + Noise renegotiation. |

#### 8.1.3 The NetworkTransport Abstraction Layer

Per the architect's stack ruling, all P2P operations go through a `NetworkTransport` trait that abstracts over the libp2p backend:

```rust
#[async_trait]
pub trait NetworkTransport: Send + Sync + 'static {
    /// Dial a peer by multiaddr. Returns a bidirectional stream.
    async fn dial(&self, addr: Multiaddr) -> Result<Connection, TransportError>;

    /// Listen on the given address. Returns incoming connections.
    async fn listen(&self, addr: Multiaddr) -> Result<Listener, TransportError>;

    /// Publish a message to a gossip topic.
    async fn gossip_publish(&self, topic: &str, data: Vec<u8>) -> Result<(), GossipError>;

    /// Subscribe to a gossip topic. Returns a stream of messages.
    async fn gossip_subscribe(&self, topic: &str) -> Result<MessageStream, GossipError>;

    /// Find peers providing a given key in the DHT.
    async fn dht_find_providers(&self, key: &[u8]) -> Result<Vec<PeerInfo>, DhtError>;

    /// Store a provider record in the DHT.
    async fn dht_provide(&self, key: &[u8]) -> Result<(), DhtError>;

    /// Find a peer by PeerId.
    async fn dht_find_peer(&self, peer_id: &PeerId) -> Result<PeerInfo, DhtError>;

    /// Get current peer count and connection stats.
    fn stats(&self) -> NetworkStats;

    /// Get the local peer ID.
    fn local_peer_id(&self) -> &PeerId;
}
```

The primary implementation is `Libp2pTransport`, wrapping rust-libp2p's `Swarm`. If rust-libp2p's maintenance degrades, an alternative implementation (e.g., custom QUIC-based transport) can be developed behind the same trait without changing any upstream code.

---

### 8.2 Peer Discovery

#### 8.2.1 Bootstrap Strategy (Layered, Zero Single Points of Failure)

The bootstrap sequence for a new node joining the network with only a genesis config file:

```
NODE STARTUP SEQUENCE:

1. Load genesis config (embedded in binary or provided as file)
   ├── Contains: genesis block hash, chain ID, initial validator set
   ├── Contains: 50+ hardcoded bootstrap peer multiaddrs
   │            (geographically distributed across 6+ continents)
   └── Contains: 5+ DNS seed hostnames (operated by independent parties)

2. Attempt peer cache (if node has run before)
   ├── Load cached peers from disk (redb table: peer_cache)
   ├── Try connecting to top-20 most recently seen peers
   └── If >= 4 connections established → skip to step 6

3. Attempt hardcoded bootstrap peers (parallel)
   ├── Dial all 50+ bootstrap peers simultaneously
   ├── QUIC 0-RTT where possible (from previous sessions)
   ├── Target: establish 8+ connections within 5 seconds
   └── If >= 4 connections → proceed to step 5

4. Attempt DNS seeds (if bootstrap peers insufficient)
   ├── Resolve TXT records from DNS seed hostnames
   │   Format: TXT "genesis-peer=<multiaddr>"
   ├── DNS seeds return randomized subsets of known active peers
   ├── Dial resolved peers
   └── If still < 4 connections → attempt mDNS (step 4b)

4b. Attempt mDNS local discovery
   ├── Broadcast mDNS query for _genesis._p2p._udp.local
   ├── Useful for local networks, testing, and censorship scenarios
   └── Any local Genesis node responds with its multiaddr

5. Kademlia DHT bootstrap
   ├── Once connected to ANY peer, initiate Kademlia FIND_NODE
   │   for the node's own PeerId (populates routing table)
   ├── Perform iterative closest-node queries to fill k-buckets
   ├── Target: routing table with 200+ entries within 30 seconds
   └── DHT now provides ongoing peer discovery -- no further
       bootstrap infrastructure needed

6. Peer management (ongoing)
   ├── Maintain target peer count per node type:
   │   Full/Validator: 40-60 peers
   │   Light: 12-20 peers
   │   Browser: 4-12 peers
   │   Mobile: 4-8 peers (foreground) / 0 (background)
   ├── Periodically refresh DHT routing table (every 10 minutes)
   ├── Persist top-50 most reliable peers to disk cache
   └── Replace disconnected peers via DHT random walks
```

**Key guarantee:** A node can join the network if it can reach even ONE existing peer. The 50+ hardcoded peers plus DNS seeds make this extremely likely even under adverse conditions.

#### 8.2.2 Kademlia DHT Configuration

```
Kademlia Parameters:
  k (replication factor):     20
  alpha (parallelism):        3
  bucket_size:                20
  record_ttl:                 36 hours
  provider_record_ttl:        24 hours
  record_republish_interval:  12 hours
  routing_table_refresh:      10 minutes
  query_timeout:              60 seconds

Key space:
  Node IDs derived from SHA-256(Ed25519_public_key)
  (BLAKE3 is used chain-wide, but Kademlia XOR distance
   works on any fixed-size hash; we use SHA-256 for
   interoperability with the libp2p Kademlia implementation)
```

**DHT record types stored:**

| Record Type | Key | Value | Purpose |
|-------------|-----|-------|---------|
| Peer routing | PeerId hash | Multiaddr set | Finding peers by ID |
| Validator set provider | `genesis/validators/<epoch>` | Signed validator set | Light nodes discovering current validators |
| Relay provider | `genesis/relays` | Relay node multiaddrs | Browser/NAT'd nodes finding relay infrastructure |
| Chain tip provider | `genesis/tip` | Latest finalized anchor hash | New nodes finding current chain head |

#### 8.2.3 Peer Diversity Requirements

To resist eclipse attacks (Section 8.7), peer connections must satisfy diversity constraints:

```
PeerDiversityPolicy {
    // No more than 30% of peers from the same /16 subnet
    max_per_subnet_16:  0.30,

    // No more than 50% of peers from the same /8 subnet
    max_per_subnet_8:   0.50,

    // At least 3 distinct ASNs represented in peer set
    min_distinct_asns:  3,

    // At least 2 peers discovered via different methods
    //   (e.g., at least 1 from DHT + 1 from bootstrap)
    min_discovery_diversity: 2,

    // For validators: at least 30% of peers must be
    //   other validators (ensures consensus message path)
    validator_peer_ratio: 0.30,  // validators only
}
```

These constraints are enforced at the connection manager level. When accepting new connections or selecting peers to dial, the diversity policy is checked. Connections that would violate diversity are rejected in favor of more diverse candidates.

---

### 8.3 Gossip Protocol

#### 8.3.1 Gossipsub v1.1 Evaluation and Configuration

**Decision: Use Gossipsub v1.1** with custom extensions for priority batching and flow control.

**Why Gossipsub v1.1:**
- Production-proven in Ethereum's beacon chain (handling ~500K validators).
- Mesh-based gossip with peer scoring provides Sybil resistance at the gossip layer.
- Supports message validation before propagation (critical for filtering invalid vertices).
- Heartbeat-based mesh maintenance self-heals from peer churn.
- Flood publishing mode available for high-priority messages (finality certificates).

**Known limitation (from RESEARCH_BRIEF.md Section 7.2):** Gossipsub "slows down under stress" -- propagation latency increases when message volume spikes. This is mitigated by priority batching (Section 8.3.3).

#### 8.3.2 Gossip Topics

| Topic | Message Type | Publishers | Subscribers | Avg Message Size | Frequency |
|-------|-------------|-----------|------------|-----------------|-----------|
| `genesis/dag/vertices` | Serialized DAG vertex (header + body) | Validators (active set) | Full nodes, validators | ~100 KB avg, 2 MB max | 200 msgs/400ms round (all validators) |
| `genesis/dag/headers` | Vertex header only | Full nodes (re-broadcast) | Light, browser, mobile nodes | ~500 bytes | 200 msgs/400ms round |
| `genesis/txs` | New transactions | Any node with transactions | Full nodes, validators | ~250 bytes avg | Thousands/second |
| `genesis/finality` | Finality certificates (BLS aggregate) | Validators who witnessed anchor commit | All nodes | ~200 bytes | 1 per anchor (every 4 rounds, ~1.6s) |
| `genesis/ai/attestations` | InferenceAttestation objects | PoUW validators | Full nodes, validators | ~500 bytes - 2 KB | Variable (depends on PoUW demand) |
| `genesis/sync/requests` | State sync chunk requests | Syncing nodes | Full nodes (serving sync) | ~100 bytes | Burst during sync |

**Topic separation rationale:** Separating vertex headers from full vertices allows light/browser/mobile nodes to subscribe only to headers and finality certificates, reducing their bandwidth by >99% compared to full nodes. Full nodes subscribe to all topics.

#### 8.3.3 Priority Batching (Mitigating Gossipsub Stress Degradation)

To address the Gossipsub stress-performance issue flagged in RESEARCH_BRIEF.md Section 7.2, Aztibase Network implements a priority-based message batching layer on top of Gossipsub:

```
MessagePriority {
    CRITICAL = 0,   // Finality certificates, equivocation proofs
    HIGH     = 1,   // DAG vertices from current round
    MEDIUM   = 2,   // DAG vertices from previous rounds (late arrivals)
    LOW      = 3,   // New transactions
    BULK     = 4,   // AI attestations, sync responses
}

BatchingConfig {
    // Messages are accumulated in a priority queue
    // and flushed at these intervals or thresholds:
    flush_interval_ms:       50,    // Flush every 50ms (8x per round)
    max_batch_bytes:         512KB, // Flush when batch reaches 512KB
    critical_immediate:      true,  // CRITICAL messages bypass batching

    // Under backpressure (outbound queue > threshold):
    backpressure_threshold:  2MB,
    backpressure_action:     drop_lowest_priority,
    // LOW and BULK messages are dropped first
    // CRITICAL and HIGH are never dropped
}
```

**How this works in practice:**
1. When a validator broadcasts a vertex, it is tagged as HIGH priority.
2. The batching layer accumulates outbound messages for up to 50ms, sorted by priority.
3. At flush time, the highest-priority messages are sent first.
4. Under network stress (outbound queue backing up), LOW and BULK messages are deferred or dropped. CRITICAL and HIGH messages always get through.
5. This ensures DAG vertex propagation meets the 400ms round timing even when transaction gossip volume spikes.

#### 8.3.4 Mesh Configuration

```
GossipsubConfig {
    // Mesh parameters (per topic)
    mesh_n:              8,     // Target mesh size
    mesh_n_low:          6,     // Minimum before grafting new peers
    mesh_n_high:         12,    // Maximum before pruning excess peers
    gossip_lazy:         6,     // Peers to gossip metadata to (IHAVE)
    gossip_factor:       0.25,  // Fraction of non-mesh peers for IHAVE

    // Heartbeat
    heartbeat_interval:  700ms, // Slightly longer than round time to avoid
                                // thrashing during round boundaries

    // Message validation
    validate_inline:     true,  // Validate before propagating
    max_message_size:    2MB + 1KB,  // Slightly over max vertex size for headers

    // History
    history_length:      5,     // Number of heartbeat windows to remember
    history_gossip:      3,     // Windows to include in IHAVE messages

    // Flood publish (for CRITICAL messages)
    flood_publish:       true,  // Flood to ALL connected peers, not just mesh

    // Scoring (see Section 8.7.4)
    // [configured separately]
}
```

**Fanout behavior:** For topics a node publishes to but is not subscribed (e.g., a validator publishing to `genesis/dag/headers` which it doesn't subscribe to because it gets full vertices), Gossipsub maintains a fanout of 6 peers for that topic. Fanout peers are selected from known subscribers and maintained across heartbeats.

#### 8.3.5 Gossip Latency vs. 400ms Round Timing

**Latency budget analysis:**

| Phase | Time Budget | Notes |
|-------|------------|-------|
| Vertex construction (validator) | 5-10ms | Per Section 4.5.2 |
| Gossipsub mesh propagation | 50-150ms | 3-4 hops in mesh (log2(200) ~ 7.6 hops in theory, but mesh topology provides shorter paths) |
| Validation by receiving node | 10-20ms | Signature check, parent verification, basic sanity |
| Total one-way propagation | 65-180ms | Under normal conditions |
| Round budget remaining | 220-335ms | Ample headroom for receiving and processing all vertices |

**Key observation:** With 200 validators and a mesh size of 8, a message reaches all validators in approximately ceil(log8(200)) = 3 hops. At 50ms per hop (including validation), propagation completes in ~150ms. The 400ms round time provides comfortable margin.

**Degraded conditions:** If network latency increases (e.g., intercontinental links under congestion), propagation may take 200-300ms. The round timer tolerates this because the anchor commit rule (Section 2.2.3) requires support from 2f+1 validators in the NEXT round, not immediate propagation. A vertex received late in round R is still valid as a parent reference in round R+1.

---

### 8.4 NAT Traversal

#### 8.4.1 NAT Traversal Architecture

Aztibase Network uses a layered NAT traversal strategy, progressing from cheapest to most expensive:

```
NAT TRAVERSAL HIERARCHY:

Layer 1: Direct connectivity (public IP)
  ├── Nodes with public IPs need no NAT traversal
  ├── UPnP / NAT-PMP port mapping attempted on startup
  │   Success rate: ~60% of consumer routers
  └── If successful, node advertises its public addr via Identify

Layer 2: AutoNAT detection
  ├── libp2p AutoNAT protocol detects NAT type on startup
  ├── Queries 4+ peers to verify reachability
  ├── Classifies node as: Public, PrivateWithMapping, Private
  └── Determines which traversal method to use

Layer 3: STUN-based hole punching (ICE)
  ├── For Cone NAT (full-cone, restricted-cone, port-restricted)
  ├── Uses libp2p DCUtR (Direct Connection Upgrade through Relay)
  ├── Process:
  │   1. Both peers connect to a relay node
  │   2. Exchange connection information via relay
  │   3. Simultaneously send packets to each other's
  │      observed addresses (hole punch)
  │   4. If successful, upgrade to direct connection
  │      (relay is no longer in the data path)
  ├── Success rate: ~85% of consumer NAT configurations
  └── Latency: 1-3 RTTs for hole punch (~100-300ms one-time)

Layer 4: Incentivized TURN relay (fallback)
  ├── For symmetric NAT and double NAT (~15% of cases)
  ├── Uses libp2p Circuit Relay v2
  ├── Relay nodes are incentivized full nodes (Section 1.4.5)
  ├── Data flows through relay (not direct P2P)
  ├── Additional latency: 20-50ms per hop
  ├── Bandwidth: relays cap per-connection at 1 Mbps
  │   (sufficient for consensus messages and light client data)
  └── Success rate: ~100% (as long as any relay is reachable)
```

**Overall NAT traversal success rate estimate:**

| NAT Type | Prevalence | Traversal Method | Success Rate | Combined |
|----------|-----------|-----------------|-------------|---------|
| Public IP / UPnP success | ~35% | Direct | 100% | 35% |
| Full-cone NAT | ~20% | STUN hole punch | ~95% | 19% |
| Restricted-cone NAT | ~25% | STUN hole punch | ~85% | 21.25% |
| Port-restricted NAT | ~10% | STUN hole punch | ~70% | 7% |
| Symmetric NAT | ~8% | TURN relay | ~100% | 8% |
| Double NAT / CGNAT | ~2% | TURN relay | ~100% | 2% |
| **Total** | **100%** | | | **~92.25% direct + 100% with relay** |

**Net result:** ~92% of nodes achieve direct P2P connections. The remaining ~8% operate through TURN relays with 20-50ms additional latency, which is within the 400ms round budget.

#### 8.4.2 AutoNAT Configuration

```rust
AutoNatConfig {
    // Number of peers to probe for reachability
    probe_count:           4,

    // Interval between re-probes (NAT mapping can change)
    reprobing_interval:    Duration::from_secs(300),  // 5 minutes

    // Timeout for individual probe
    probe_timeout:         Duration::from_secs(10),

    // If at least 3/4 probes confirm reachability,
    // classify as Public
    confidence_threshold:  0.75,

    // Maximum concurrent probe connections
    max_concurrent_probes: 2,
}
```

#### 8.4.3 DCUtR (Direct Connection Upgrade through Relay) Configuration

```rust
DcutrConfig {
    // Maximum time to attempt hole punch before giving up
    hole_punch_timeout:    Duration::from_secs(10),

    // Number of simultaneous hole punch attempts
    max_simultaneous:      3,

    // Retry interval if hole punch fails (try different relay)
    retry_interval:        Duration::from_secs(30),

    // Maximum retries before falling back to permanent relay
    max_retries:           3,
}
```

#### 8.4.4 Incentivized TURN Relay Infrastructure

Per Section 1.4.5, full nodes serving as TURN relays earn a share of transaction fees. The relay infrastructure design:

```
RELAY NODE REQUIREMENTS:
  - Must be a full node with public IP (or UPnP-mapped)
  - Must have >= 50 Mbps symmetric bandwidth
  - Must maintain >= 99% uptime over rolling 7-day window
  - Must stake a minimum relay bond (prevents Sybil relay flooding)

RELAY PROTOCOL (Circuit Relay v2):
  - Connection limit per relay:      500 simultaneous relayed connections
  - Bandwidth limit per connection:  1 Mbps (sufficient for consensus + light client)
  - Connection duration limit:       30 minutes (renewable, prevents squatting)
  - Data transfer limit:             100 MB per 30-minute session

RELAY DISCOVERY:
  - Relay nodes register as providers for key "genesis/relays" in DHT
  - Nodes behind NAT query DHT for relays, sorted by:
    1. Latency (prefer geographically close relays)
    2. Reputation score (tracked on-chain)
    3. Current load (prefer less-loaded relays)
  - Nodes maintain connections to 3+ relay nodes for redundancy

RELAY INCENTIVES:
  - Relay nodes report bandwidth served per epoch
  - Reports are verified by cross-referencing with client reports
  - Relay rewards drawn from transaction fee pool (not inflation)
  - Quality metrics: uptime, latency, bandwidth served, connection success rate
  - Poor-quality relays lose reputation and eventually lose relay bond
```

#### 8.4.5 Validators Behind NAT

Validators behind NAT are explicitly supported (per Section 2.8.3 and Section 4.5.5):

**Minimum connectivity requirement:** A validator MUST be reachable by at least 2f+1 other validators within the round time (400ms). This is verified by monitoring vertex propagation latency.

**Validator NAT strategy:**

1. **On startup:** Validator runs AutoNAT, attempts UPnP/NAT-PMP, and initiates DCUtR hole punching to known validators.
2. **Steady state:** Validator maintains direct connections to 30+ other validators (via hole-punched or direct connections). For unreachable validators (symmetric NAT), connections route through incentivized relays.
3. **Latency overhead:** Relay adds 20-50ms. At 400ms rounds, a validator with 150ms total additional latency can still participate fully (Section 4.5.5).
4. **Failure mode:** If a validator cannot reach 2f+1 peers within 10 consecutive rounds, it is temporarily removed from the active set. This protects consensus liveness while allowing re-entry when connectivity improves.

**Consensus message sizes through relays:**

| Message Type | Size | Frequency | Bandwidth via Relay |
|-------------|------|-----------|-------------------|
| Vertex header (outbound) | ~500 bytes | 1 per 400ms | ~10 Kbps |
| Full vertex (outbound) | ~100 KB | 1 per 400ms | ~2 Mbps |
| Incoming vertices (all validators) | ~100 KB x 200 | Per round | Partial -- only from relay-connected peers |
| Finality certificate | ~200 bytes | 1 per 1.6s | Negligible |

A validator behind symmetric NAT, relaying through 3 relay nodes, consumes approximately 200-500 Kbps of relay bandwidth -- well within the 1 Mbps per-connection relay cap.

---

### 8.5 WebRTC for Browser Nodes

#### 8.5.1 Browser-to-Network Without a Signaling Server

The traditional WebRTC model requires a signaling server to exchange SDP (Session Description Protocol) offers and answers between peers. Aztibase Network eliminates this dependency using libp2p Circuit Relay v2 as the signaling channel:

```
BROWSER NODE CONNECTION SEQUENCE:

1. Browser node loads WASM binary (from any hosting -- CDN, IPFS, local file)

2. Initial connection via WebSocket to a bootstrap full node
   ├── The genesis config includes WebSocket multiaddrs for bootstrap nodes
   │   Format: /dns4/bootstrap1.genesis.network/tcp/443/wss/p2p/<PeerId>
   ├── WebSocket is used ONLY for initial signaling, not for data
   └── Multiple bootstrap nodes tried in parallel for redundancy

3. Via the WebSocket connection, the browser requests Circuit Relay v2
   ├── The bootstrap full node acts as a temporary relay
   ├── Through the relay, the browser exchanges WebRTC SDP with target peers
   └── This replaces the traditional "signaling server" -- the relay IS the signaler

4. WebRTC DataChannel established to target peers
   ├── ICE negotiation occurs via the relay-signaled SDP
   ├── If ICE succeeds (browser has non-symmetric NAT): direct P2P connection
   │   The relay is no longer in the data path
   ├── If ICE fails (browser behind symmetric NAT): data flows through relay
   └── The browser attempts connections to 4-12 peers

5. Once WebRTC connections are established:
   ├── Browser subscribes to Gossipsub topics: genesis/dag/headers, genesis/finality
   ├── Browser participates in Kademlia DHT (discovers more peers)
   ├── Original WebSocket connection can be dropped
   │   (browser is now connected via WebRTC P2P)
   └── Browser node is fully operational

6. Ongoing peer management:
   ├── Discover new peers via DHT
   ├── Establish WebRTC connections to new peers (using existing peers as relays for signaling)
   ├── Maintain 4-12 peer connections
   └── No ongoing dependency on the initial bootstrap WebSocket endpoint
```

**Key insight:** The WebSocket connection in step 2 is to ANY full node, not a dedicated signaling server. Every full node can serve as a relay/signaler. The browser node discovers additional full nodes via DHT and can re-bootstrap through any of them if the initial connection drops.

#### 8.5.2 Browser Node Protocol Support

| Protocol | Supported in Browser | Notes |
|----------|---------------------|-------|
| WebRTC DataChannels | Yes | Primary transport |
| WebSocket (wss://) | Yes (fallback) | For initial signaling and degraded-mode operation |
| Gossipsub v1.1 | Yes | Subscribes to header and finality topics only |
| Kademlia DHT | Yes | Participates in peer discovery |
| Circuit Relay v2 (client) | Yes | Uses relays for signaling and fallback data transfer |
| Circuit Relay v2 (server) | **No** | Browser nodes cannot serve as relays (unreliable uptime) |
| AutoNAT | **No** | Not meaningful in browser (no listen addresses) |
| TCP / QUIC | **No** | Browser sandbox prevents raw socket access |
| mDNS | **No** | Browser cannot send multicast DNS |
| Block-STM execution | **No** | No full state for execution |
| Verkle proof generation | **No** | Requires full state tree |
| Verkle proof verification | Yes | Pure math operations compile to WASM |
| BLS aggregate verification | Yes | blst compiles to WASM |

#### 8.5.3 Browser Node Bandwidth Limits

```
BrowserBandwidthPolicy {
    max_inbound:           1 Mbps,      // Per Section 4.3.2
    max_outbound:          500 Kbps,    // Browser should not be a heavy uploader
    max_single_peer:       250 Kbps,    // No single peer dominates bandwidth
    burst_allowance:       2 Mbps,      // Short bursts for proof downloads
    burst_window:          5 seconds,

    // Estimated steady-state usage:
    //   Headers: ~500 bytes * 2.5/sec = ~10 Kbps
    //   Finality certs: ~200 bytes * 0.6/sec = ~1 Kbps
    //   Verkle proof requests (on-demand): ~5 KB per request
    //   DHT maintenance: ~5 Kbps
    //   Total sustained: ~20-50 Kbps (well under 1 Mbps limit)
}
```

#### 8.5.4 WebSocket Fallback Strategy

If WebRTC connections fail entirely (very restrictive corporate firewall, browser incompatibility), the browser node degrades to WebSocket-only mode:

```
WebSocket Fallback Mode:
  - Connect to 2-4 full nodes via WebSocket (wss://)
  - Receive header + finality data via WebSocket
  - Submit transactions via WebSocket
  - Request Verkle proofs via WebSocket

  Limitations:
  - NOT true P2P (connected to full nodes, not peer mesh)
  - Trust model degrades: must trust connected full nodes for data delivery
  - Still verifies finality (BLS aggregate) and state (Verkle proofs) cryptographically
  - Acceptable degradation: less decentralized, but still self-verifying
```

---

### 8.6 Bandwidth Optimization

#### 8.6.1 Compressed Block Propagation (Compact Blocks)

Per the node-engineer's bandwidth flag (Section 4.9.5), compressed block propagation is implemented from day one.

**Compact Block Relay Protocol:**

Inspired by Bitcoin's BIP-152 compact blocks, adapted for DAG vertices:

```
COMPACT VERTEX RELAY:

When a validator broadcasts a new vertex:

1. First broadcast: CompactVertex
   CompactVertex {
       header:           GenesisBlockHeader,      // Full header (~500 bytes)
       short_tx_ids:     Vec<ShortTxId>,          // 6-byte truncated tx hashes
       prefilled_txs:    Vec<Transaction>,         // Txs likely missing from peers' mempools
       ai_attestations:  Vec<AIAttestation>,       // Full attestations (small, not in mempool)
   }

   ShortTxId = SipHash(tx_hash, header_nonce)[0..6]  // 6 bytes per tx

   Size reduction:
     Average vertex: 400 transactions * 250 bytes = 100 KB
     Compact vertex: 500 (header) + 400 * 6 (short IDs) + 2KB (prefilled) = ~5 KB
     Compression ratio: ~95% for well-connected nodes with overlapping mempools

2. Receiving node:
   ├── Match short_tx_ids against local mempool
   ├── For matched txs: reconstruct full vertex from mempool + header
   ├── For unmatched txs: request via GetTransactions message
   └── Verify: reconstructed vertex hash matches header

3. If reconstruction fails (too many missing txs):
   ├── Request full vertex (fallback)
   └── This is expensive but rare for well-connected nodes
       Expected fallback rate: <5% of vertices

EXPECTED BANDWIDTH SAVINGS (full nodes):
  Without compact relay:  200 validators * 100 KB * 2.5 rounds/sec = ~50 MB/s peak
  With compact relay:     200 validators * 5 KB * 2.5/sec = ~2.5 MB/s peak
  Plus tx gossip:         ~5,000 txs/sec * 250 bytes = ~1.25 MB/s
  Total with compact:     ~3.75 MB/s = ~30 Mbps peak
  Steady state:           ~15-20 Mbps (not all vertices arrive simultaneously)
```

#### 8.6.2 Message Deduplication

```
DeduplicationConfig {
    // Bloom filter for recently seen message hashes
    bloom_filter_size:     2^20,  // ~1M entries
    bloom_false_positive:  0.001, // 0.1% false positive rate
    bloom_rotation:        60s,   // Rotate bloom filter every 60 seconds

    // LRU cache for recent message hashes (exact dedup)
    lru_cache_size:        50_000,

    // Two-layer dedup:
    //   1. Check bloom filter (fast, probabilistic)
    //   2. If bloom says "maybe seen", check LRU (exact)
    //   3. If definitely new: process, add to both
    //   4. If definitely seen: drop silently
}
```

Gossipsub's built-in message ID deduplication is supplemented by this application-layer deduplication to catch messages that arrive via multiple mesh paths.

#### 8.6.3 Transaction Deduplication Across Vertices

Since multiple validators may include the same transaction in their vertices for the same round, the network implements:

1. **Mempool synchronization:** Validators periodically exchange mempool summaries (bloom filters of tx hashes) with mesh peers. This allows the compact vertex relay to achieve higher hit rates.

2. **Transaction suppression:** When a validator includes a transaction in a vertex, it optionally announces `TX_INCLUDED(tx_hash, vertex_id)` to mesh peers. Peers receiving this can deprioritize gossiping that transaction.

3. **Execution-layer dedup:** The Block-STM execution engine (Section 4.1.3) deduplicates transactions during the anchor ordering phase. Duplicate transactions across vertices in the same commit batch are executed only once.

#### 8.6.4 Compression

All wire messages are compressed using LZ4 (fast compression, moderate ratio):

```
CompressionConfig {
    algorithm:          LZ4,        // Fast compression/decompression
    min_message_size:   256 bytes,  // Don't compress tiny messages
    compression_level:  1,          // Fastest (LZ4 level 1 = ~3 Gbps throughput)

    // Expected compression ratios:
    //   Vertex bodies: 1.5-2x (transactions have moderate entropy)
    //   Headers: 1.2-1.5x (small, structured)
    //   Verkle proofs: 1.3-1.8x
    //   AI attestations: 1.4-1.7x
}
```

**Why LZ4 over Zstd for wire format:** redb uses Zstd for storage (higher compression ratio, acceptable decompression speed for disk reads). For wire messages, LZ4's ~3 Gbps throughput (compression) and ~5 Gbps (decompression) mean negligible CPU overhead per message, which is critical when processing 200 vertices per 400ms round. Zstd at level 3 achieves ~500 Mbps, which could add measurable latency at high message rates.

---

### 8.7 Network Security

#### 8.7.1 Eclipse Attack Prevention

An eclipse attack isolates a node by controlling all its peer connections, allowing the attacker to feed it a false view of the chain.

**Defenses:**

1. **Peer diversity policy (Section 8.2.3):** Limits peers per subnet and requires multiple ASNs. An attacker must control infrastructure across many networks to eclipse a node.

2. **Outbound connection preference:** Nodes prefer outbound (self-initiated) connections over inbound connections. Outbound connections are harder for an attacker to control because the victim chooses which peers to dial.

```
ConnectionPolicy {
    max_inbound:     70% of total peers,
    min_outbound:    30% of total peers,
    // For a full node with 50 peers: at least 15 outbound
}
```

3. **Anchor-based detection:** If a node stops receiving new finality certificates for >5 anchor intervals (~8 seconds), it triggers an "eclipse alarm":
   - Disconnect from 50% of current peers (random selection).
   - Attempt to connect to bootstrap peers and DNS seeds.
   - Perform a fresh DHT walk to discover new peers.
   - If connectivity to the legitimate network is restored, log the incident and blacklist the eclipsing peers.

4. **Finality certificate cross-checking:** Nodes periodically query random DHT peers (not their mesh peers) for the latest finality certificate. If the response differs from what their mesh peers report, an eclipse may be in progress.

#### 8.7.2 Sybil Resistance at Network Layer

Sybil attacks at the network layer (flooding the DHT or gossip mesh with attacker-controlled nodes) are mitigated by:

1. **Proof of stake gating for gossip mesh priority:** Gossipsub peer scoring (Section 8.7.4) gives higher scores to peers whose PeerId corresponds to a staked validator. Non-staked peers can participate but receive lower mesh priority.

2. **DHT provider record validation:** DHT records for Genesis-specific keys (validator sets, relays, chain tips) must include valid signatures. Sybil nodes cannot forge these records without the corresponding private keys.

3. **Connection rate limiting:**
```
RateLimitConfig {
    max_new_connections_per_second:  10,
    max_new_connections_per_minute:  100,
    max_connections_from_same_ip:    3,
    max_connections_from_same_subnet_16: 10,
    backoff_on_failed_handshake:     30 seconds,
}
```

4. **Resource-based gating:** Nodes that consume disproportionate resources (bandwidth, DHT queries) without contributing proportionally (relaying messages, serving data) are deprioritized via peer scoring.

#### 8.7.3 DDoS Mitigation

```
DDoSMitigation {
    // Per-peer rate limits
    max_messages_per_second_per_peer:  100,
    max_bytes_per_second_per_peer:     1 MB,

    // Per-topic rate limits
    max_vertices_per_second:           600,   // 200 validators * 3 (buffer for retransmits)
    max_txs_per_second:                20_000, // Generous but bounded
    max_finality_certs_per_second:     10,     // Very bounded

    // Connection-level
    max_pending_connections:            200,
    handshake_timeout:                 10 seconds,

    // IP-level
    max_connections_per_ip:            3,     // Except for known validators
    known_validator_whitelist:         true,  // Validators get relaxed limits

    // Response to detected flood:
    //   1. Rate-limit offending peer
    //   2. If persistent: disconnect + temporary ban (1 hour)
    //   3. If coordinated (multiple IPs): activate emergency mode
    //       - Restrict new connections to known peers only
    //       - Increase peer scoring thresholds
    //       - Log for manual investigation
}
```

#### 8.7.4 Gossipsub Peer Scoring

Gossipsub v1.1's peer scoring system is configured to reward honest behavior and penalize malicious or negligent peers:

```
GossipsubPeerScoring {
    // Topic-specific scoring for genesis/dag/vertices:
    topic_weight:                  1.0,
    time_in_mesh_weight:           0.5,
    time_in_mesh_quantum:          1 second,
    time_in_mesh_cap:              3600.0,
    first_message_deliveries_weight: 1.0,
    first_message_deliveries_cap:    100.0,
    first_message_deliveries_decay:  0.99,  // Per heartbeat
    mesh_message_deliveries_weight: -1.0,   // Penalty for NOT delivering
    mesh_message_deliveries_threshold: 5,
    mesh_message_deliveries_cap:     20,
    mesh_message_deliveries_activation: 5 seconds,
    mesh_message_deliveries_window:    2 seconds,
    mesh_message_deliveries_decay:     0.97,
    invalid_message_deliveries_weight: -100.0,  // Heavy penalty for invalid messages
    invalid_message_deliveries_decay:   0.5,    // Slow decay (bad reputation persists)

    // Application-specific scoring:
    app_specific_weight:           1.0,
    // app_specific_score computed per-peer:
    //   +50 if peer is a staked validator
    //   +20 if peer has been connected > 1 hour
    //   +10 if peer has served valid Verkle proofs
    //   -30 if peer has sent invalid vertices
    //   -50 if peer has been involved in equivocation relay
    //   -100 if peer has been identified as eclipse attacker

    // IP colocation penalty
    ip_colocation_factor_weight:   -50.0,
    ip_colocation_factor_threshold: 3,  // >3 peers on same IP = penalty

    // Behavior penalty
    behaviour_penalty_weight:      -10.0,
    behaviour_penalty_decay:        0.99,

    // Thresholds
    gossip_threshold:              -100,   // Below this: no gossip
    publish_threshold:             -200,   // Below this: no publish
    graylist_threshold:            -300,   // Below this: disconnect
    opportunistic_graft_threshold:  50,    // Above this: candidate for grafting
}
```

**Validator prioritization:** Peers identified as staked validators (via their PeerId matching the on-chain validator set) receive a +50 application-specific score bonus. This ensures that gossip mesh paths preferentially include validators, which is critical for DAG vertex propagation within the 400ms round budget.

#### 8.7.5 Encrypted Connections

**All connections are encrypted. There is no plaintext mode.**

| Transport | Encryption | Forward Secrecy | Mutual Authentication |
|-----------|-----------|-----------------|----------------------|
| QUIC | TLS 1.3 (built-in) | Yes (ephemeral ECDH) | Yes (PeerId verification) |
| TCP | Noise XX handshake | Yes (ephemeral DH) | Yes (static key exchange) |
| WebRTC | DTLS 1.2+ (built-in) | Yes (ephemeral ECDH) | Yes (fingerprint verification) |
| WebSocket | TLS 1.3 (wss://) | Yes | Server-authenticated (browser model) |

**Noise XX handshake details:**
- Used for TCP connections.
- Both parties exchange static public keys (Ed25519) during the handshake.
- The static public key is verified against the expected PeerId (derived from Ed25519 public key).
- Ephemeral Diffie-Hellman provides forward secrecy -- compromise of the static key does not reveal past traffic.
- Handshake completes in 1.5 round trips (3 messages: -> e, <- e ee s es, -> s se).

**PeerId verification:** After the encrypted channel is established, the Identify protocol confirms the remote peer's PeerId. If the PeerId doesn't match the expected value (e.g., from a DHT lookup or bootstrap list), the connection is terminated.

---

### 8.8 Server-Independence Verification

The following checklist from the p2p-network-engineer SKILL.md is explicitly verified against the design:

#### [PASS] Can a new node join the network with only a genesis config file?

**Yes.** The genesis config contains 50+ hardcoded bootstrap peer multiaddrs and 5+ DNS seed hostnames. A new node dials these peers in parallel (Section 8.2.1). Once connected to ANY single peer, Kademlia DHT discovery finds the rest of the network. The genesis config is embedded in the binary -- no external download required.

**Extreme scenario:** If all 50+ bootstrap peers and all DNS seeds are simultaneously unreachable, the node can still join via:
- mDNS (if another Genesis node is on the local network)
- QR code / manual peer exchange (offline peer sharing)
- Peer cache from a previous run

#### [PASS] Can nodes discover peers without a central registry?

**Yes.** Kademlia DHT is the primary discovery mechanism after bootstrap (Section 8.2.2). The DHT is fully distributed -- every node participates in storing and serving routing information. There is no central registry, directory server, or tracker. DNS seeds are used only during bootstrap and are operated by multiple independent parties (no single operator controls all seeds).

#### [PASS] Can browser nodes connect without a WebSocket server?

**Qualified pass with nuance.** Browser nodes require an initial WebSocket connection to ANY full node for signaling (Section 8.5.1, step 2). This is not a "WebSocket server" in the traditional sense -- it is any full node in the network, and the browser tries multiple in parallel. Once WebRTC connections are established, the WebSocket can be dropped.

**Degradation scenario:** If WebRTC fails entirely, the browser falls back to WebSocket-only mode (Section 8.5.4). In this mode, the browser connects to full nodes via WebSocket -- this is less decentralized but still self-verifying (BLS finality verification and Verkle proof verification still work). The browser is never dependent on a single specific server.

**Mitigation for the "needs a full node" concern:** Every full node and validator is a potential WebSocket endpoint. With thousands of full nodes in the network, the availability of WebSocket endpoints is equivalent to the availability of the network itself.

#### [PASS] Can the network heal from 50% node loss?

**Yes.** The network survives and recovers from 50% node loss through multiple mechanisms:

1. **Consensus:** SynBFT continues operating as long as >2/3 of stake (by validators) is online. If 50% of ALL nodes go offline but validators have higher uptime, consensus continues uninterrupted. If 50% of validators go offline, consensus halts temporarily but resumes when they return or the emergency re-selection mechanism (Section 2.4.2) activates.

2. **DHT recovery:** Kademlia's replication factor (k=20) means each DHT record is stored on 20 nodes. Losing 50% of nodes still leaves ~10 copies of each record. The DHT self-heals by re-replicating records to new closest nodes as it discovers the loss.

3. **Gossip mesh repair:** Gossipsub's heartbeat mechanism detects missing mesh peers within 700ms. The mesh automatically grafts new peers to replace lost ones. Within 2-3 heartbeats (~2 seconds), the gossip mesh is fully repaired.

4. **Peer re-discovery:** Nodes that lose peers immediately perform DHT lookups to find replacements. The peer diversity policy (Section 8.2.3) ensures the recovered peer set is diverse, not clustered.

5. **State sync availability:** As long as some full nodes survive, new/recovering nodes can perform snapshot sync (Section 4.1.5) from the survivors.

#### [PASS] Can nodes behind symmetric NAT participate?

**Yes.** Nodes behind symmetric NAT use Circuit Relay v2 (incentivized TURN relays) as described in Section 8.4.1, Layer 4. The relay infrastructure is decentralized (operated by incentivized full nodes, discoverable via DHT). Relay connections add 20-50ms latency, which is within the 400ms round budget for validators.

**Validator viability behind symmetric NAT:** Per Section 8.4.5, a validator behind symmetric NAT routing through relays adds ~50ms latency. The round budget analysis shows validators with up to 150ms additional latency maintain full ConsensusReputation scores. Symmetric NAT validators are viable.

**Cost:** Relay connections consume relay resources. The incentivized relay system (Section 8.4.4) ensures relays are economically sustainable. Nodes behind symmetric NAT indirectly pay for relay services through transaction fees.

#### [PASS] Is there a fallback if DHT bootstrapping fails?

**Yes.** The layered bootstrap strategy (Section 8.2.1) provides six independent discovery methods:

1. Peer cache (previous run)
2. Hardcoded bootstrap peers (50+, embedded in binary)
3. DNS seeds (5+, independent operators)
4. mDNS (local network)
5. QR code / manual peer exchange
6. DHT (once connected to any peer)

DHT is method #6. If it fails (e.g., a bug in the Kademlia implementation), the node falls back to maintaining connections discovered via methods 1-5. The node operates with reduced peer discovery capability but remains functional.

**Additional fallback:** The `NetworkTransport` abstraction layer (Section 8.1.3) allows replacing the DHT implementation without changing the rest of the stack. If Kademlia proves unreliable, an alternative DHT or peer discovery mechanism can be swapped in.

---

### 8.9 Bandwidth Requirements Table

#### 8.9.1 Per Node Type

| Node Type | Upload (sustained) | Download (sustained) | Peak (burst) | Monthly Estimate |
|-----------|--------------------|---------------------|-------------|-----------------|
| **Full Node** | 10-15 Mbps | 15-20 Mbps | 50 Mbps | ~5-7 TB |
| **Full Node (with compact relay)** | 5-8 Mbps | 8-12 Mbps | 25 Mbps | ~2.5-4 TB |
| **Validator** | 8-12 Mbps | 12-18 Mbps | 30 Mbps | ~3.5-5 TB |
| **Validator (with compact relay)** | 4-7 Mbps | 7-12 Mbps | 20 Mbps | ~2-3.5 TB |
| **Light Node** | 50-200 Kbps | 100-500 Kbps | 2 Mbps | ~10-40 GB |
| **Browser Node** | 20-100 Kbps | 50-200 Kbps | 2 Mbps | ~5-15 GB/session-month |
| **Mobile Node** | 20-100 Kbps | 50-200 Kbps | 1 Mbps | ~3-10 GB |
| **Relay Node (full + relay)** | 15-25 Mbps | 20-30 Mbps | 60 Mbps | ~6-10 TB |

**Notes:**
- "With compact relay" assumes 90% mempool overlap between peers (typical for well-connected nodes). New network or poorly-connected nodes will see lower compression ratios.
- Validator upload is lower than full node because validators broadcast their own vertex (once) and rely on Gossipsub mesh for re-propagation, while full nodes actively relay all vertices.
- Light/Browser/Mobile bandwidth is dominated by header sync and on-demand proof requests. Background DHT maintenance adds ~5 Kbps steady state.
- Monthly estimates assume 24/7 operation (full/validator/relay) or typical usage patterns (light/browser/mobile).

#### 8.9.2 Bandwidth Breakdown for Full Node (with compact relay)

| Traffic Type | Upload | Download |
|-------------|--------|----------|
| Compact vertex relay (outbound gossip) | 2-3 Mbps | - |
| Compact vertex reception (inbound gossip) | - | 3-5 Mbps |
| Transaction gossip | 1-2 Mbps | 1-2 Mbps |
| Full vertex requests (compact miss fallback) | 0.5-1 Mbps | 1-2 Mbps |
| Finality certificates | <0.1 Mbps | <0.1 Mbps |
| AI attestation gossip | 0.2-0.5 Mbps | 0.2-0.5 Mbps |
| DHT maintenance | <0.1 Mbps | <0.1 Mbps |
| Serving light client proofs | 0.5-1 Mbps | - |
| Serving sync (amortized) | 0.5-1 Mbps | - |
| **Total** | **~5-8 Mbps** | **~8-12 Mbps** |

**Assessment against the 25 Mbps flag (Section 4.9.5):** With compact block relay implemented, the sustained bandwidth for full nodes drops from ~25 Mbps to ~15-20 Mbps total (upload + download). This is achievable on most consumer broadband connections globally. The node-engineer's bandwidth flag is addressed -- compact block relay reduces the concern from "upper end of consumer broadband" to "comfortable for most broadband connections." Regions with <10 Mbps broadband should run light nodes.

---

### 8.10 Stack Challenges

#### 8.10.1 rust-libp2p: ACCEPTED with Mitigations

**Assessment:** rust-libp2p is the correct choice for Aztibase Network's P2P layer. The maintenance crisis flagged in RESEARCH_BRIEF.md Section 7.2 is real but primarily affects go-libp2p and js-libp2p. The rust-libp2p implementation has independent, active maintainers and a healthy release cadence.

**Specific rust-libp2p component readiness assessment:**

| Component | Maturity | Risk | Notes |
|-----------|----------|------|-------|
| `libp2p-quic` | Stable | Low | Based on quinn. Production-ready. |
| `libp2p-tcp` | Stable | Low | Battle-tested. |
| `libp2p-noise` | Stable | Low | Standard Noise XX implementation. |
| `libp2p-gossipsub` | Stable | Low-Medium | v1.1 implemented. Stress degradation is a known characteristic, not a bug -- mitigated by priority batching. |
| `libp2p-kad` | Stable | Low | Standard Kademlia implementation. |
| `libp2p-identify` | Stable | Low | Simple protocol, well-tested. |
| `libp2p-autonat` | Stable | Low | NAT detection works reliably. |
| `libp2p-dcutr` | Stable | Low-Medium | Hole punching success depends on NAT type, not implementation quality. |
| `libp2p-relay` (Circuit Relay v2) | Stable | Medium | Core protocol is stable. Resource management (connection limits, bandwidth caps) may need custom hardening for relay incentive model. |
| `libp2p-webrtc` | **Maturing** | **Medium-High** | Functional but limited production deployment as of March 2026. This is the highest-risk component. Browser WASM target works but has not been stress-tested at scale. |
| `libp2p-yamux` | Stable | Low | Standard stream multiplexer. |
| `libp2p-mdns` | Stable | Low | Simple mDNS responder. |

**Identified gaps requiring custom work:**

1. **Priority message batching (Section 8.3.3):** Gossipsub does not natively support message priority or batching. This must be implemented as a layer between the application and Gossipsub. Estimated effort: 2-3 weeks.

2. **Compact block relay (Section 8.6.1):** Not a libp2p feature. Must be implemented as a custom protocol on top of libp2p's request-response pattern. Estimated effort: 3-4 weeks.

3. **Relay incentive tracking:** Circuit Relay v2 does not natively support bandwidth metering or incentive tracking. Custom extensions needed to report relay usage for on-chain incentive calculation. Estimated effort: 2-3 weeks.

4. **Enhanced peer scoring integration:** Gossipsub peer scoring needs integration with on-chain validator set data (for the +50 validator bonus). This requires a bridge between the consensus layer's validator set and the networking layer's peer scoring. Estimated effort: 1-2 weeks.

5. **WebRTC browser stability hardening:** Given the medium-high risk assessment for libp2p-webrtc, dedicated testing and hardening effort is recommended. Estimated effort: 4-6 weeks of testing and bug-fixing.

**Total custom work estimate:** 12-18 weeks of engineering effort for P2P layer customization, on top of the standard libp2p integration.

#### 8.10.2 Gossipsub v1.1 vs. Alternatives

| Protocol | Pros | Cons | Decision |
|----------|------|------|----------|
| **Gossipsub v1.1** | Production-proven (Ethereum), peer scoring, message validation, flood publish for critical messages | Stress degradation, mesh maintenance overhead | **SELECTED** (with priority batching mitigation) |
| **Episub** | Designed for higher throughput than Gossipsub, tree-structured propagation | Not production-deployed, limited implementation maturity, complex tree repair logic | Rejected: insufficient maturity for a financial system |
| **Custom gossip** | Full control, can optimize for DAG vertex propagation specifically | Massive engineering effort, unproven at scale, security risks from novel protocol | Rejected: not justified given Gossipsub v1.1 adequacy with mitigations |
| **Direct push (no gossip)** | Simplest, lowest latency for small validator sets | Does not scale beyond ~50 validators, no redundancy | Rejected: need to support 200-400 validators |

**Conclusion:** Gossipsub v1.1 with priority batching is the right choice. Its stress degradation is a known, bounded limitation that is effectively mitigated by the batching layer. The alternatives are either immature (Episub) or unjustifiably expensive to build (custom).

#### 8.10.3 Kademlia DHT vs. Alternatives

| DHT | Pros | Cons | Decision |
|-----|------|------|----------|
| **Kademlia** | Standard, well-understood, production-proven (IPFS, Ethereum, libp2p), XOR distance is simple and effective | O(log n) lookup latency, susceptible to eclipse if not defended | **SELECTED** |
| **Coral DSHT** | Locality-aware, reduces latency for geographically close peers | Complex, limited implementation, not in libp2p | Rejected: benefits don't justify custom implementation |
| **S/Kademlia** | Sybil-resistant variant of Kademlia | Adds complexity (parallel lookups, disjoint paths), partially addressed by our peer diversity policy | Not selected, but elements (disjoint lookup paths) incorporated into our Kademlia configuration |

**Kademlia enhancement adopted from S/Kademlia:** We configure Kademlia to use disjoint lookup paths (alpha=3 with disjoint path selection) to resist routing table poisoning attacks. This is supported by rust-libp2p's Kademlia configuration.

#### 8.10.4 Noise Protocol vs. TLS 1.3

| Protocol | Use Case in Genesis | Justification |
|----------|-------------------|---------------|
| **Noise XX** | TCP connections | Mutual authentication (both peers prove identity). Simpler than TLS. No certificate authority dependency. Integrates with libp2p identity model. |
| **TLS 1.3** | QUIC connections (mandatory) | QUIC mandates TLS 1.3. Using libp2p's QUIC transport, TLS is configured with self-signed certificates derived from the Ed25519 identity key. No external CA dependency. |
| **DTLS** | WebRTC connections (mandatory) | WebRTC mandates DTLS. Handled by the browser's WebRTC stack. |

**No challenge raised.** Noise for TCP and native encryption for QUIC/WebRTC is the correct, standard approach. There is no benefit to replacing Noise with TLS for TCP connections -- Noise is simpler, has no CA dependency, and integrates naturally with libp2p's identity model.

---

### 8.11 Network Topology and Global Operation

#### 8.11.1 Expected Network Topology

```
                        GLOBAL GOSSIP MESH
    ┌─────────────────────────────────────────────────┐
    │                                                   │
    │   [V1]──[V2]──[V3]    Validator Mesh (dense)    │
    │    │ \  / │ \  / │     8-12 mesh peers each      │
    │   [V4]──[V5]──[V6]    QUIC transport             │
    │    │      │      │                                │
    │   [F1]──[F2]──[F3]    Full Node Mesh             │
    │    │\     │      │\    8 mesh peers each          │
    │   [F4] [F5]──[F6] [F7] QUIC/TCP transport       │
    │    │    │      │    │                              │
    │   [R1] [R2]   [R3]      Relay Nodes              │
    │    ↕    ↕      ↕        (Full nodes + relay)      │
    │   [N1] [N2]   [N3]      NAT'd nodes via relay    │
    │                                                   │
    │   [L1]─[L2]   [L3]─[L4]  Light Nodes            │
    │    (connected to full nodes for proofs)           │
    │                                                   │
    │   [B1]⋯[B2]   [B3]       Browser Nodes          │
    │    (WebRTC to full nodes and each other)          │
    │                                                   │
    │   [M1]  [M2]  [M3]       Mobile Nodes            │
    │    (QUIC to full nodes, intermittent)             │
    └─────────────────────────────────────────────────┘
```

#### 8.11.2 Multi-Continent Latency Considerations

| Route | Typical RTT | Impact on 400ms Round |
|-------|------------|----------------------|
| Same continent | 10-50ms | Negligible |
| US-Europe | 80-120ms | ~30% of round budget, manageable |
| US-Asia | 150-200ms | ~50% of round budget, tight |
| Europe-Asia | 100-150ms | ~37% of round budget, manageable |
| US-Australia | 180-250ms | ~62% of round budget, very tight |

**Implication:** Validators in geographically distant regions (e.g., US-to-Australia) will experience some propagation delay. The anchor commit rule's 2f+1 support threshold (checked at round R+1, not immediately) provides tolerance. Validators with high inter-continental latency may see slightly reduced ConsensusReputation (due to late vertex arrivals) but can still participate fully.

**Optimization:** Gossipsub's mesh topology naturally forms "proximity clusters" -- peers with lower latency tend to be preferred for mesh positions over time (via first-message-delivery scoring). This creates a topology where messages propagate quickly within a continent and then bridge between continents via inter-continental mesh links.

---

### 8.12 Connection Lifecycle and Resource Management

#### 8.12.1 Connection Limits by Node Type

| Node Type | Max Inbound | Max Outbound | Max Total | Rationale |
|-----------|------------|-------------|-----------|-----------|
| Full Node | 100 | 50 | 150 | Serves light clients, relays, and mesh peers |
| Validator | 150 | 75 | 200 | Higher limits for consensus connectivity and serving |
| Light Node | 15 | 10 | 20 | Minimal -- only needs header/proof access |
| Browser Node | 8 | 6 | 12 | Browser resource constraints |
| Mobile Node | 6 | 4 | 8 | Battery and memory constraints |
| Relay Node | 300 | 50 | 500 | Must handle relayed connections (up to 500) plus own mesh |

#### 8.12.2 Connection Pruning Strategy

When a node exceeds its connection limit:

```
PruningStrategy {
    // Score each connection:
    score = gossipsub_score * 0.4
          + time_connected * 0.2
          + bytes_useful_served * 0.2
          + diversity_value * 0.2

    // diversity_value: higher if this peer is from an
    // underrepresented subnet/ASN in our peer set

    // Prune lowest-scoring connections first
    // NEVER prune:
    //   - Validators we have active consensus paths with
    //   - Our relay connections (if we're behind NAT)
    //   - Peers we're actively syncing from
}
```

---

### 8.13 Monitoring and Health Metrics

The P2P layer exposes the following metrics for monitoring and AI-based network health analysis (for the ai-integration-engineer to consume):

```
NetworkHealthMetrics {
    // Connectivity
    peer_count:                  Gauge,
    peer_count_by_type:          GaugeVec<NodeType>,
    connections_inbound:         Gauge,
    connections_outbound:        Gauge,
    peer_diversity_score:        Gauge,  // 0-1, computed from diversity policy

    // Gossip
    gossip_messages_sent:        CounterVec<Topic>,
    gossip_messages_received:    CounterVec<Topic>,
    gossip_messages_dropped:     CounterVec<Topic, Reason>,
    gossip_propagation_latency:  HistogramVec<Topic>,  // Time from creation to receipt
    gossip_mesh_size:            GaugeVec<Topic>,

    // Bandwidth
    bytes_sent_total:            Counter,
    bytes_received_total:        Counter,
    bytes_sent_per_second:       Gauge,
    bytes_received_per_second:   Gauge,
    compact_relay_hit_rate:      Gauge,  // % of txs resolved from mempool

    // NAT
    nat_type:                    Gauge,  // Public/ConeNAT/SymmetricNAT
    relay_connections_active:    Gauge,
    hole_punch_success_rate:     Gauge,

    // DHT
    dht_routing_table_size:      Gauge,
    dht_query_latency:           Histogram,
    dht_query_success_rate:      Gauge,

    // Security
    peers_banned:                Counter,
    peers_rate_limited:          Counter,
    eclipse_alarms_triggered:    Counter,
    invalid_messages_received:   Counter,
}
```

---

**Stack challenges raised:**
- No new stack challenges raised. The architect's libp2p COMPROMISE ruling (Section 0) with the `NetworkTransport` abstraction layer is the correct approach. The priority batching mitigation adequately addresses Gossipsub's stress degradation. The `libp2p-webrtc` component is flagged as medium-high risk and requires dedicated testing effort, but this is an implementation concern, not a stack challenge.

**Conflicts raised:** None

**Sign-off:** p2p-network-engineer

## SECTION 9: NAMING & BRANDING

---
**Contributed by:** naming-council
**Date:** 2026-03-05
**Status:** CONDITIONAL RECOMMENDATION (awaiting legal re-clearance for primary pick)
**Dependencies met:** LEGAL_CLEARANCE_REPORT.md (complete), MASTER_DESIGN.md Sections 1-2 (reviewed for identity alignment)

---

### 9.1 Legal Gate

Legal clearance was received on March 5, 2026. Of 10 candidates evaluated:
- **1 GREEN:** Quen (QEN)
- **4 YELLOW:** Synap, Nura, Velum, Aztibase
- **5 RED (permanently blocked):** Axon, Cortex, Soma, Thalos, Lumen

### 9.2 Primary Recommendation: Aztibase (AZTB) -- CONDITIONAL

| Attribute | Value |
|-----------|-------|
| **Blockchain name** | Aztibase Network |
| **Coin name** | Aztibase |
| **Ticker** | AZTB |
| **Tagline** | "Where Intelligence Branches" |
| **Naming score** | 78/100 |
| **Legal status** | YELLOW -- requires re-clearance from legal-ip-counsel |

**Rationale:** Aztibases are the signal-receiving branches of biological neurons. The name maps directly to Aztibase Network's architecture: DAG-based topology (branching graph structure), AI-native consensus (distributed intelligence gathering), and server-independence (no central hub, signals received from many sources). The branching visual metaphor provides exceptional logo and brand design potential. Ticker AZTB is clean across all crypto and stock exchanges.

**Legal risk:** Aztibase Systems Inc. holds Class 9 software trademarks. Requires formal opposition analysis. Ticker changed from DND (taken) to AZTB (clean). See NAMING_REPORT.md for full risk assessment and mitigation strategy.

**Action required:** legal-ip-counsel must re-evaluate Aztibase with AZTB ticker and "Aztibase Network" distinctiveness argument before this recommendation can be finalized.

### 9.3 Fallback: Quen (QEN) -- GREEN

| Attribute | Value |
|-----------|-------|
| **Blockchain name** | Quen Network |
| **Coin name** | Quen |
| **Ticker** | QEN |
| **Tagline** | "Intelligence Without Servers" |
| **Naming score** | 57/100 |
| **Legal status** | GREEN -- fully cleared |

**Warning:** "Quen" is phonetically identical to "Qwen" (Alibaba's flagship AI model brand, unified under this name on March 2, 2026). For an AI-native blockchain, this creates severe brand confusion, SEO challenges, and derivative perception. Legally clean but practically compromised. See NAMING_REPORT.md Section "Critical Finding" for full analysis.

### 9.4 Trademark Registration Roadmap (Upon Finalization)

| Priority | Jurisdiction | Classes | Timeline |
|----------|-------------|---------|----------|
| 1 | USPTO (United States) | 9, 36, 42 | File ITU within 2 weeks |
| 2 | EUIPO (European Union) | 9, 36, 42 | File within 4 weeks |
| 3 | CIPC (South Africa) | 9, 36, 42 | File within 6 weeks |
| 4 | WIPO Madrid Protocol | 9, 36, 42 | File within 8 weeks |

### 9.5 Domain & Social Media Strategy

**Primary domain targets (in priority order):**
1. aztibase.network / quen.network
2. aztibase.io / quen.io
3. aztibase.xyz / quen.xyz
4. aztibase.ai / quen.ai (if available)
5. aztibase.com / quen.com (acquisition, likely premium)

**Social handles to reserve immediately upon legal finalization:**
- X/Twitter: @AztibaseNetwork (or @QuenNetwork)
- GitHub: aztibase-network (or quen-network) organization
- Discord: Aztibase Network (or Quen Network) server
- Telegram: @AztibaseNetwork (or @QuenNetwork)

### 9.6 Naming Council Notes

The full evaluation of all candidates, scoring methodology, linguistic safety checks across 9 languages, and detailed justification is in **NAMING_REPORT.md**. Key finding: the Qwen (Alibaba) phonetic collision with Quen was not captured in the legal clearance (which correctly focused on trademark/crypto registries) but is a critical branding concern for an AI-native chain. This drove the recommendation toward Aztibase despite its YELLOW legal status.

**Sign-off:** naming-council

---

## CHANGE LOG
| Date | Skill | Section | Change |
|------|-------|---------|--------|
| 2026-03-05 | architect | Section 0 | Initial stack proposal created |
| 2026-03-05 | architect | Section 0 | Updated Storage from Merkle Patricia Trie to Verkle Trees + dual storage per research findings |
| 2026-03-05 | architect | Section 0 | Added 5 STACK RULINGS: P2P (compromise), State Mgmt (accept), Consensus (accept), AI Runtime (refine), License (accept) |
| 2026-03-05 | architect | Section 1 | Complete blockchain architecture contribution: chain structure, layered architecture, design philosophy, server-independence, AI-native architecture, privacy model, downstream skill framework |
| 2026-03-05 | consensus-engineer | Section 2 | Complete consensus mechanism contribution: Synaptic Consensus (SynBFT + PoUW). DAG-BFT with anchor-based ordering, AI reputation-weighted leader selection, multi-method PoUW verification, validator selection with stake caps and diversity quotas. STACK CHALLENGE raised: BLS12-381 signatures needed alongside Ed25519. |
| 2026-03-05 | node-engineer | Section 4 | Complete node architecture contribution: Full node (redb, dual storage, DAG processing pipeline, priority mempool, 3 sync strategies, 8GB RAM target), Light node (redb, Verkle proof verification, BLS finality verification, 512MB RAM target), Browser node (WASM compilation strategy, WebRTC transport, IndexedDB storage, browser compat matrix), Mobile node (Android+iOS, battery optimization, light client variant), Validator node (vertex proposal pipeline, VRF computation, PoUW tiers, NAT traversal), State pruning (snapshot+diff model, archive vs pruned, Verkle GC). STACK ENDORSEMENT: BLS12-381 (supporting consensus-engineer). STACK REFINEMENT: redb over sled for light nodes. STACK FLAG: bandwidth concern for full nodes. |
| 2026-03-05 | p2p-network-engineer | Section 8 | Complete P2P network design: Protocol stack (QUIC primary, TCP+Noise fallback, WebRTC for browsers, NetworkTransport abstraction), Peer discovery (6-layer bootstrap strategy, Kademlia DHT with k=20, peer diversity policy for eclipse resistance), Gossipsub v1.1 with priority batching and flow control (6 gossip topics, mesh size 8, 50ms batch flush), NAT traversal (AutoNAT + DCUtR hole punching ~85% + incentivized TURN relays ~15%, validators behind NAT supported), WebRTC browser nodes (Circuit Relay v2 for signaling, no dedicated signaling server, WebSocket fallback), Bandwidth optimization (compact block relay ~95% reduction, LZ4 wire compression, bloom filter dedup, transaction suppression), Network security (eclipse prevention via diversity + anchor detection, Sybil resistance via stake-gated scoring, DDoS rate limiting, Gossipsub peer scoring, Noise/TLS/DTLS encryption), Server-independence checklist (all 6 items PASS), Bandwidth requirements table (full node 8-12 Mbps with compact relay, light node 100-500 Kbps, browser 50-200 Kbps). No stack challenges raised. libp2p-webrtc flagged as medium-high risk requiring dedicated testing. |
| 2026-03-05 | naming-council | Section 9 | Complete naming and branding contribution: Legal gate review (1 GREEN, 4 YELLOW, 5 RED). Critical finding: Quen (QEN) is phonetically identical to Alibaba's Qwen AI brand -- severe branding risk for AI-native chain. Primary recommendation: Aztibase Network (AZTB), scoring 78/100 -- exceptional AI/DAG meaning resonance, clean ticker, strong visual identity. CONDITIONAL on legal re-clearance (Aztibase Systems Inc. Class 9 trademark requires opposition analysis). Fallback: Quen (QEN), scoring 57/100 -- legally clean but brand-compromised. Trademark roadmap: USPTO, EUIPO, CIPC, WIPO Madrid Protocol. Full evaluation in NAMING_REPORT.md. |
| 2026-03-05 | security-engineer | Section 5 | Complete security model contribution: Threat matrix (44 attack vectors across 8 domains: consensus, network, smart contract, cryptographic, AI-specific, economic, node-level, browser/mobile), Cryptographic standards review (BLAKE3 APPROVED, Ed25519 APPROVED with strict verification mandate, BLS12-381 APPROVED with PoP/domain separation/subgroup checking, Noise APPROVED, Verkle trees CONDITIONALLY APPROVED with quantum migration requirement), Security review of all existing sections (Sections 1, 2, 4, 8 reviewed with 13 security flags raised -- 9 SECURITY-ELEVATED, 4 STANDARD), AI security monitoring design (4 monitors: transaction anomaly, consensus behavior, contract exploit, network health + graduated threat response model with principle that AI never autonomously slashes/freezes), Quantum readiness assessment (4-phase PQC migration plan, HNDL risk analysis, Verkle-to-Merkle migration requirements), Key management (key hierarchy, wallet security tiers, HSM recommendations for validators, key rotation and recovery), Stack security review (15 dependencies evaluated with CVE analysis, supply chain security requirements including cargo-audit/cargo-vet/reproducible builds). STACK ENDORSEMENT: BLS12-381. 2 CONFLICTS raised: agent spending limit enforcement layer, VRF last-revealer bias. |
