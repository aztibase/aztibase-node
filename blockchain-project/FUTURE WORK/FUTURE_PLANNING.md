# Aztibase Network — Future Planning

> Living document for features, ideas, and architectural proposals beyond the current sprint.
> Items here are **not committed to a sprint** yet. They move to sprint plans when prioritized.

---

## FP-001: Community Task Layer — Human-Mediated AI Inference

**Status:** PROPOSAL — see FP-004 for L2 alternative (preferred approach)
**Priority:** HIGH — core to community mission
**Estimated effort:** 2-3 sprints (if L1-native) | see FP-004 for L2 approach
**Author:** Project Lead
**Date:** 2026-03-08

### Vision

Enable disadvantaged and non-technical users to earn AZTB rewards using only a mobile phone and free LLM access (ChatGPT, Gemini, Copilot, etc.). The chain posts AI tasks, mobile users copy prompts into any available LLM, paste the results back, and earn micro-rewards when their answers reach consensus with other workers.

This makes every free LLM API in the world into Aztibase's compute layer, mediated by humans.

### Why This Matters

- **Zero barrier to entry** — no hardware, no stake, no cloud subscription, no technical skill beyond copy-paste
- **Genuinely inclusive** — works on $50 phones with basic internet
- **Economically sound** — businesses pay for verified AI output, workers earn for mediating it
- **Unique positioning** — Aztibase becomes the chain where humans and AI earn together

### How It Works

```
Business posts task + reward
    → Task appears on mobile workers' feeds
        → Worker copies prompt into any free LLM
            → Worker pastes result back to chain
                → 3-5 workers submit independently
                    → Consensus on matching answers
                        → Matching workers get paid
```

### Task Types That Fit This Model

| Task Type | Example | Works? |
|-----------|---------|--------|
| Translation | "Translate to Zulu/Hindi/Spanish" | YES — high demand, regional value |
| Summarization | "Summarize this article in 3 sentences" | YES |
| Classification | "Is this review positive/negative/neutral?" | YES — structured output |
| Data extraction | "Extract name, email, phone from this text" | YES |
| Content moderation | "Is this text harmful? Yes/No" | YES — structured output |
| Question answering | "What are the key dates in this document?" | YES |
| Code generation | "Write a function that does X" | MAYBE — harder to verify |
| Image generation | "Create an image of X" | NO — can't copy-paste images easily |
| Real-time inference | Sub-second response needed | NO — too slow for human mediation |
| Private data | Contains sensitive information | NO — can't paste into third-party LLMs |

### New On-Chain Components

#### 1. New Transaction Types

```
TxKind::PostCommunityTask {
    requester: Address,
    prompt: Vec<u8>,                // the text to process
    task_type: CommunityTaskType,   // Translate / Summarize / Classify / etc.
    output_format: Vec<u8>,         // structured output template
    reward_per_worker: u64,         // AZTB per accepted submission
    worker_count: u8,               // independent submissions needed (3-7)
    gas_budget: u64,                // requester pre-pays gas for all worker submissions
    deadline_round: u64,
    nonce: u64,
    gas_price: u64,
}

TxKind::SubmitCommunityResult {
    worker: Address,
    task_id: Hash,
    result: Vec<u8>,                // the answer
    result_hash: Hash,              // BLAKE3 of normalized result
    signature: Vec<u8>,             // Ed25519
    nonce: u64,
    gas_price: 0,                   // subsidized — requester pre-paid gas
}

TxKind::RegisterCommunityWorker {
    user: Address,
    languages: Vec<String>,         // what they can review/translate
    nonce: u64,
    gas_price: u64,
}
```

#### 2. New State Stores

- **CommunityTaskPool** — like existing `TaskPool` but for human-mediated tasks, indexed by task_type and language
- **WorkerRegistry** — reputation-tracked worker profiles (no stake required, reputation-based)
- **CommunitySettlement** — extends existing `TaskSettlement` pattern for community quorum

#### 3. New Pipeline Handlers

- `PostCommunityTask` execution: validate requester balance covers (reward_per_worker * worker_count + gas_budget), escrow funds, insert into CommunityTaskPool
- `SubmitCommunityResult` execution: validate worker is registered, task exists, deadline not passed, worker hasn't already submitted, record result
- Settlement trigger: when submission count reaches worker_count, run consensus check, pay matching workers, refund requester for non-consensus remainder

### Quality Control / Anti-Cheat

This is the hardest part. If you pay people to click buttons, some will spam.

#### Honeypot Tasks
Mix in tasks with known correct answers. Workers who fail honeypots lose reputation and future task eligibility. Honeypots are indistinguishable from real tasks.

#### Agreement Threshold
Require majority agreement (e.g., 3/5 or 4/7 workers) before any payment is released. Outlier answers are rejected and those workers' reputation is reduced.

#### Reputation Score (Human PoUW)
Mirror the existing `SlidingWindowPoUWScore` pattern but for human workers:

```
community_score = 0.5 * accuracy + 0.3 * consistency + 0.2 * availability
```

- **Accuracy**: % of submissions that matched consensus (sliding window, last 100 tasks)
- **Consistency**: low variance in quality over time
- **Availability**: tasks completed vs tasks assigned

Higher reputation = assigned higher-value tasks and priority in task queue.

#### Gradual Trust Ramp
New workers start with:
- Low-value tasks only (1-2 AZTB)
- Higher honeypot frequency (1 in 3 tasks)
- Lower daily task cap

After proving reliability (e.g., 50+ tasks, 90%+ accuracy):
- Unlock higher-value tasks
- Lower honeypot frequency (1 in 10)
- Higher daily cap

#### Structured Output Formats
Prompts enforce structured answers: "Answer ONLY with: positive / negative / neutral" — structured responses hash-match reliably across different LLMs. Free-form text uses semantic similarity (validator-side embedding comparison) instead of exact hash matching.

### Gas Subsidization Model

**Requester-pays model** (recommended):
- The business posting the task pre-pays gas for ALL worker submissions as part of the task escrow
- Workers submit with `gas_price: 0` — the protocol deducts from the requester's escrowed gas_budget
- Workers earn pure profit, zero cost to participate

**Why not treasury subsidy:**
- Treasury funds are finite (10% of emission)
- Requester-pays is self-sustaining — demand funds supply
- Avoids governance fights over treasury allocation

### Mobile App UX

The light client PWA/app shows a task feed:

```
┌─────────────────────────────────┐
│  AVAILABLE TASKS                │
│                                 │
│  ┌───────────────────────────┐  │
│  │ Translate to Zulu         │  │
│  │ Reward: 3 AZTB            │  │
│  │ "The meeting is at 3pm    │  │
│  │  tomorrow, please bring   │  │
│  │  your ID and proof of..." │  │
│  │                           │  │
│  │  [Copy Prompt]  [Submit]  │  │
│  └───────────────────────────┘  │
│                                 │
│  YOUR STATS                     │
│  Tasks done: 47                 │
│  Reputation: 94%                │
│  Earned today: 8 AZTB           │
└─────────────────────────────────┘
```

1. User taps **[Copy Prompt]** → clipboard contains the formatted prompt
2. User opens ChatGPT / Gemini / any LLM
3. User pastes prompt, gets answer
4. User returns to app, taps **[Submit]**, pastes answer
5. Chain records submission, checks consensus when all workers submit
6. Reward credited to wallet

### Existing Architecture Reuse

| Existing Component | Reuse for Community Tasks |
|-------------------|--------------------------|
| `TaskPool` (BTreeMap + by_model index) | Pattern for `CommunityTaskPool` |
| `AttestationAggregator` (quorum + dedup) | Pattern for community result consensus |
| `TaskSettlement::settle()` (reward split) | Pattern for community reward distribution |
| `SlidingWindowPoUWScore` (multi-metric) | Pattern for worker reputation scoring |
| `aztibase-wasm` (light client WASM) | Host the mobile task feed UI |
| WebSocket subscriptions | Push new tasks to connected workers |
| `LightStore` (redb, 5 tables) | Cache assigned tasks + submission history |
| Ed25519 signing | Workers sign submissions with existing wallet keys |

### Relationship to Existing Validator PoUW

Community tasks complement — not replace — validator AI inference:

| | Validator PoUW | Community Tasks |
|---|---|---|
| **Who** | Server operators with GPUs | Anyone with a phone |
| **What** | Run models on-chain | Mediate free LLM access |
| **Speed** | Seconds | Minutes |
| **Cost** | Hardware + stake | Zero |
| **Quality** | Deterministic (same model = same output) | Probabilistic (consensus among humans) |
| **Best for** | High-volume, real-time inference | Translation, moderation, labeling, verification |

### Open Questions

1. **Semantic similarity for free-form answers** — who runs the embedding model to compare non-structured answers? Validators? Adds compute load.
2. **Task pricing** — how does the market determine fair reward_per_worker? Auction? Fixed by task type? Requester sets price?
3. **Language verification** — how to verify a Zulu translation if validators don't speak Zulu? Rely purely on worker consensus?
4. **Sybil resistance without stake** — reputation-only systems can be gamed with multiple accounts. Consider: phone number verification? Social graph? Gradual trust ramp may be sufficient.
5. **Legal/regulatory** — is paying people for micro-tasks subject to labor law in various jurisdictions? Needs /legal-ip-counsel review.
6. **Prompt injection** — malicious task posters could craft prompts that trick LLMs into harmful output. Workers would unknowingly submit harmful content. Need content filtering on task submission.

### Prior Art / References

- **Effect Network** — decentralized Mechanical Turk on EOS/BSC
- **Human Protocol** — human-in-the-loop AI verification
- **Hivemind / Sahara AI** — data labeling marketplaces
- **Worldcoin** — proof-of-personhood for Sybil resistance (relevant to anti-cheat)
- **Amazon Mechanical Turk** — the original human micro-task platform (centralized)

### Implementation Phases (Tentative)

**Phase 1: Foundation**
- `CommunityTaskType` enum, `PostCommunityTask` / `SubmitCommunityResult` tx types
- `CommunityTaskPool` state store
- Basic hash-match settlement (structured output tasks only)
- Worker registration (no reputation yet)

**Phase 2: Quality & Reputation**
- `WorkerReputation` store with sliding window scoring
- Honeypot task injection system
- Gradual trust ramp (new worker restrictions)
- Task assignment priority by reputation

**Phase 3: Mobile UX**
- WASM/PWA task feed UI
- Copy-to-clipboard + paste-back flow
- WebSocket push notifications for new tasks
- Local task history + earnings dashboard

**Phase 4: Advanced**
- Semantic similarity scoring for free-form answers (validator-side embeddings)
- Language-specific task routing
- Task pricing market (requester auction / worker bidding)
- Sybil resistance improvements

---

## FP-002: Delegation / Staking for Non-Validators

**Status:** PROPOSAL
**Priority:** MEDIUM
**Estimated effort:** 1 sprint
**Date:** 2026-03-08

### Summary

Allow mobile users to delegate AZTB to validators and earn a share of validator rewards. Standard DPoS delegation model.

### New Transaction Types

```
TxKind::Delegate { delegator, validator, amount, nonce, gas_price }
TxKind::Undelegate { delegator, validator, amount, nonce, gas_price }
TxKind::ClaimRewards { delegator, nonce, gas_price }
```

### Key Design Decisions

- **Unbonding period**: How long after undelegating before funds are liquid? (7-21 days typical)
- **Commission rate**: Validator sets a commission % on delegator rewards (5-20% typical)
- **Slashing propagation**: If a validator is slashed, do delegators lose stake too? (most chains: yes, proportionally)
- **Reward distribution**: Per-epoch or claimable accumulation?

### Relationship to FP-001

Delegation gives passive income. Community Tasks give active income. Both serve mobile users but with different effort/reward profiles:

| | Delegation | Community Tasks |
|---|---|---|
| **Effort** | Zero (set and forget) | Active (complete tasks) |
| **Requires AZTB** | Yes (must hold tokens to delegate) | No (earn from zero) |
| **Risk** | Slashing if validator misbehaves | Reputation loss if you spam |
| **Reward source** | Validator emission share | Task requester payment |

Community Tasks (FP-001) is higher priority because it enables earning from zero — delegation requires already having AZTB.

---

## FP-003: Mobile Light Client App

**Status:** PROPOSAL
**Priority:** HIGH (required for FP-001)
**Estimated effort:** 1 sprint
**Date:** 2026-03-08

### Summary

Build a mobile-first PWA or React Native app wrapping `aztibase-wasm` with:
- Wallet (key generation, balance, send/receive)
- Header sync + proof verification
- Transaction signing and broadcast
- Community Task feed (FP-001)
- Delegation UI (FP-002)

### Missing WASM Exports (Prerequisites)

The `aztibase-wasm` crate currently verifies but cannot create transactions. Needs:
- `signTransaction(txJson, secretKey)` — construct + sign a transaction
- `buildTransfer(from, to, amount, nonce)` — build a transfer tx
- `deriveKeypair(mnemonic, index)` — key derivation in WASM
- `getAddress(publicKey)` — derive address from pubkey

### Deployment Options

| Option | Pros | Cons |
|--------|------|------|
| **PWA** | No app store, works everywhere, fastest to ship | No background sync, limited push notifications |
| **React Native + WASM** | Native feel, push notifications, biometrics | App store approval, build complexity |
| **Rust FFI (uniffi)** | Best performance, full Rust stack | Most complex to build and maintain |

PWA recommended for first release, native app later.

---

## FP-004: L2 Architecture — Community Task Chain as Sovereign Rollup

**Status:** PROPOSAL — PREFERRED APPROACH (supersedes FP-001 as L1-native)
**Priority:** HIGH
**Estimated effort:** 3-4 sprints (1 sprint L1 primitives + 2-3 sprints L2 chain)
**Author:** Project Lead
**Date:** 2026-03-08

### Why L2 Instead of L1

FP-001 proposed building community tasks directly into L1. After further analysis, an L2 approach is architecturally superior:

| Concern | L1-native (FP-001) | L2 rollup (FP-004) |
|---------|--------------------|--------------------|
| **L1 complexity** | 3+ new tx types, new stores, settlement logic in core chain | L1 stays clean — adds only bridge + anchoring |
| **Iteration speed** | Every change = potential hard fork | L2 upgrades independently, no L1 coordination |
| **Gas model** | Must hack subsidization into L1 fee logic | L2 defines own gas rules — free for workers by default |
| **Throughput** | Community tasks compete with financial txs for L1 block space | L2 has dedicated throughput, posts proofs to L1 |
| **Security trade-off** | Full BFT security (overkill for 3 AZTB micro-tasks) | Lighter guarantees appropriate for micro-tasks |
| **Ecosystem** | Only Aztibase team can build it | Third parties can build their own L2s too |

### Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│  L2: Community Task Chain                           │
│                                                     │
│  - Own block production (1-2s blocks, cheap/free)   │
│  - Own gas rules (zero gas for workers)             │
│  - Task pool, worker registry, reputation system    │
│  - Community settlement logic                       │
│  - Posts state roots to L1 every N minutes          │
│                                                     │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐             │
│  │ Worker  │  │ Worker  │  │ Worker  │  (phones)    │
│  └────┬────┘  └────┬────┘  └────┬────┘             │
│       └────────────┼────────────┘                   │
│                    ▼                                │
│           L2 Sequencer(s)                           │
│           (decentralized over time)                 │
└────────────────────┬────────────────────────────────┘
                     │  State root + proof batch
                     │  (one L1 tx per epoch)
                     ▼
┌─────────────────────────────────────────────────────┐
│  L1: Aztibase Network                               │
│                                                     │
│  - Consensus (SynBFT + PoUW)                        │
│  - AZTB token settlement + finality                 │
│  - L2 state root anchoring                          │
│  - Bridge: lock/unlock AZTB between L1 ↔ L2         │
│  - Validator PoUW (AI inference)                    │
└─────────────────────────────────────────────────────┘
```

### L2 Model: Sovereign Rollup (Recommended)

Three rollup models were considered:

| Model | How it works | Fit for community tasks? |
|-------|-------------|------------------------|
| **Optimistic** | L2 posts state to L1, 7-day fraud proof window | Overkill — 7-day withdrawal delay unacceptable for micro-earnings |
| **Validity (ZK)** | L2 posts state + ZK proof, instant L1 finality | Too complex to build — ZK circuits for task logic are heavy |
| **Sovereign** | L2 posts data to L1 for availability, validates itself | Best fit — simple, fast, appropriate security for micro-tasks |

**Sovereign rollup** means:
- L1 provides **data availability + ordering** (anyone can reconstruct L2 state from L1 data)
- L2 validates its own blocks (L2 light clients verify L2 state)
- L1 does NOT verify L2 execution (no fraud proofs, no ZK proofs)
- Security guarantee: as long as L2 has at least 1 honest full node, invalid state can be detected

This is appropriate because:
- Community task disputes are low-value (1-10 AZTB) — full L1 security is overkill
- Reputation slashing handles cheaters at the L2 level
- Workers care about speed and zero fees, not maximum security

### What L1 Needs (Minimal Changes)

Only two new primitives on L1:

#### 1. L2 State Anchoring

```rust
TxKind::AnchorL2State {
    l2_chain_id: [u8; 32],      // identifies which L2
    sequencer: Address,          // who submitted this anchor
    state_root: [u8; 32],       // L2 state root at this point
    batch_data_hash: [u8; 32],  // hash of L2 block data for DA
    l2_block_range: (u64, u64), // L2 blocks covered by this anchor
    nonce: u64,
    gas_price: u64,
}
```

L1 stores the latest state root per `l2_chain_id`. Anyone can query it to verify L2 state.

#### 2. Bridge Contract (WASM or EVM on L1)

```
Lock AZTB on L1  → Event emitted → L2 sequencer mints wrapped AZTB on L2
Burn wAZTB on L2 → Proof submitted to L1 → L1 bridge unlocks AZTB to user
```

Two transactions:

```rust
TxKind::BridgeDeposit {
    depositor: Address,
    l2_chain_id: [u8; 32],
    l2_recipient: Address,      // may differ from L1 address
    amount: u64,
    nonce: u64,
    gas_price: u64,
}

TxKind::BridgeWithdraw {
    withdrawer: Address,        // L1 recipient
    l2_chain_id: [u8; 32],
    amount: u64,
    l2_burn_proof: Vec<u8>,     // proof that wAZTB was burned on L2
    l2_state_root: [u8; 32],   // must match latest anchored state root
    nonce: u64,
    gas_price: u64,
}
```

#### 3. L2 Registry (Optional, Governance-Gated)

```rust
TxKind::RegisterL2 {
    owner: Address,
    l2_chain_id: [u8; 32],
    name: String,               // "Community Task Chain"
    sequencer_set: Vec<Address>,
    bridge_address: Address,
    metadata_uri: String,       // off-chain metadata (docs, RPC endpoints)
    nonce: u64,
    gas_price: u64,
}
```

Wallets and explorers use this registry to discover L2s.

### What The L2 Contains (Separate Codebase)

The L2 is its own chain — potentially a lighter fork of the Aztibase node, or a purpose-built Rust binary. It contains everything from FP-001 that was originally proposed for L1:

#### L2 Transaction Types
- `PostCommunityTask` — businesses post tasks with rewards
- `SubmitCommunityResult` — workers submit answers (zero gas)
- `RegisterCommunityWorker` — worker registration with language preferences
- `ClaimRewards` — workers withdraw earned wAZTB

#### L2 State
- `CommunityTaskPool` — active tasks, indexed by type and language
- `WorkerRegistry` — reputation scores, trust tiers, task history
- `SettlementEngine` — quorum checking, reward distribution
- `HoneypotStore` — known-answer tasks for anti-cheat

#### L2 Gas Model
- Worker submissions: **zero gas** (sequencer absorbs cost, funded by task requester fees)
- Task posting: small fee in wAZTB (covers sequencer costs + L1 anchoring)
- Withdrawals to L1: small fee (covers L1 bridge gas)

#### L2 Block Production
- Block time: 1-2 seconds (faster than L1's 400ms rounds — L2 doesn't need BFT consensus for micro-tasks)
- Sequencer: starts centralized (Aztibase team), decentralizes over time via sequencer rotation
- Anchoring: posts state root to L1 every 5-10 minutes (configurable)

### Anti-Cheat (Same as FP-001, Enforced at L2)

All quality control from FP-001 applies — honeypots, agreement threshold, reputation scoring, gradual trust ramp. These are L2-level concerns, not L1.

### The Bridge Flow for a Mobile Worker

```
1. Worker creates wallet on phone (L1 address)
2. Worker registers on L2 (RegisterCommunityWorker — free)
3. Business deposits 1000 AZTB to L2 via BridgeDeposit on L1
4. Business posts task on L2 with 500 wAZTB reward
5. Worker completes task, earns 3 wAZTB on L2 (instant, free)
6. Worker accumulates earnings on L2
7. When ready, worker bridges wAZTB back to L1 via BridgeWithdraw
8. Worker now has real AZTB on L1 — can hold, trade, delegate, or spend
```

Workers only pay L1 gas when they bridge out, not when they earn. They can accumulate on L2 and bridge in batches.

### Ecosystem Expansion

The L2 infrastructure isn't just for community tasks. Once bridge + anchoring exists, anyone can launch an L2:

| Potential L2 | Purpose | Who builds it |
|-------------|---------|--------------|
| **Community Task Chain** | Human-mediated AI, micro-tasks | Aztibase team (first L2) |
| **Gaming Chain** | Fast game state, cheap NFTs | Third-party game studios |
| **Regional DeFi** | Localized lending/savings for underserved markets | DeFi teams |
| **Data Marketplace** | Buy/sell labeled datasets (from community tasks) | Data companies |
| **Social Chain** | Decentralized social feeds, content moderation | Social app developers |

All L2s share AZTB as the base currency, Aztibase L1 as the settlement layer.

### Relationship to Other FPs

| FP | Relationship |
|----|-------------|
| **FP-001** | Community task logic moves from L1 to L2. FP-001 becomes the L2 spec, not L1 spec. |
| **FP-002** | Delegation stays on L1 — it's a core economic primitive. |
| **FP-003** | Mobile app connects to BOTH L1 (wallet, delegation) and L2 (task feed, submissions). |

### Implementation Phases

**Phase 1: L1 Primitives (1 sprint)**
- `AnchorL2State` tx type + execution in pipeline
- `BridgeDeposit` / `BridgeWithdraw` tx types + lock/unlock logic
- `L2Registry` store (optional)
- `L2AnchorStore` — tracks latest state root per l2_chain_id
- RPC: `aztb_getL2State`, `aztb_listL2s`, `aztb_getBridgeBalance`

**Phase 2: L2 Chain MVP (1-2 sprints)**
- Separate repository: `aztibase-community-l2`
- Minimal sequencer (single operator, Aztibase team)
- `PostCommunityTask` + `SubmitCommunityResult` + basic hash-match settlement
- Worker registration (no reputation yet)
- L1 anchoring on a timer (every 10 minutes)
- Bridge relay: watch L1 deposits, mint wAZTB on L2

**Phase 3: Quality + Mobile UX (1 sprint)**
- Reputation scoring (sliding window, same pattern as PoUW)
- Honeypot injection system
- Gradual trust ramp
- Mobile PWA: task feed, copy-prompt, paste-answer, earnings dashboard
- WebSocket push for new tasks

**Phase 4: Decentralization + Ecosystem (future)**
- Sequencer rotation / decentralization
- SDK for third-party L2 builders
- Cross-L2 messaging (L2 ↔ L2 via L1 relay)
- L2 explorer + bridge UI

### Open Questions

1. **Sequencer decentralization timeline** — start centralized for speed, but when/how to decentralize? PoS sequencer rotation? Shared sequencer with other L2s?
2. **Bridge security** — sovereign rollup means L1 trusts L2's own validation for withdrawals. What if L2 sequencer posts a fraudulent state root? Consider: withdrawal delay + challenge period (lighter than full optimistic rollup)?
3. **Data availability** — does L2 post full block data to L1 (expensive but maximum DA) or just state roots (cheap but weaker DA)? Could use a separate DA layer in future.
4. **Cross-L2 composability** — if multiple L2s exist, can a community task on L2-A trigger a DeFi action on L2-B? Needs L1 as message relay.
5. **wAZTB fungibility** — is wAZTB on the community L2 fungible with wAZTB on a gaming L2? Or are they isolated? Isolated is simpler, fungible is better UX.
6. **Legal** — does operating an L2 sequencer create different regulatory obligations than running an L1 validator? Needs /legal-ip-counsel review.

### Prior Art

- **Celestia** — modular DA layer, sovereign rollup pioneer
- **Optimism Superchain** — multiple L2s sharing a common stack
- **Arbitrum Orbit** — L3 app-chains settling on Arbitrum L2
- **Starknet** — validity rollup with app-specific chains (Madara)
- **Avail / EigenDA** — dedicated DA layers for rollups

---

## FP-005: Project Assessment & Launch Roadmap

**Status:** REFERENCE — consolidated from strategic review session
**Priority:** HIGH — informs all future sprint planning
**Author:** Project Lead
**Date:** 2026-03-08

---

### 5.1 Code Review Summary

| Metric | Value |
|---|---|
| Production Rust LOC | ~27,700 |
| Crates | 9 (acyclic dependency graph) |
| Tests | 473 |
| Fuzz targets | 6 (3 core, 3 consensus) |
| ADRs | 13 |
| Sprints completed | 26 |
| Documentation | ~15,000 lines |
| `unsafe` blocks | 0 |

**Quality ratings:**

| Dimension | Rating | Notes |
|---|---|---|
| Code quality | A | Clean, idiomatic Rust, Result/Option throughout, no `.unwrap()` in library code |
| Architecture | A | Professional crate separation, clean dependency graph, trait-based abstractions |
| Security awareness | A | Formal threat model, zero unsafe, crypto domain separation, 11 SEC-* issues resolved |
| Documentation | A+ | Better than most funded projects — 15K lines of design docs, ADRs, sprint plans |
| Feature scope | B+ | Ambitious (dual-VM, AI-native, DAG consensus) — all scaffolded, not all battle-tested |
| Production readiness | C+ | Testnet-ready, not mainnet-ready (expected for current stage) |

---

### 5.2 Industry Comparison

| Project | Language | LOC (approx) | Team Size | Dev Time | Status |
|---|---|---|---|---|---|
| Bitcoin Core | C++ | ~250K | 30+ core | 15 years | Mainnet |
| Geth (Ethereum) | Go | ~400K | 20+ core | 10 years | Mainnet |
| Sui | Rust | ~500K+ | 50+ (Mysten) | 3 years | Mainnet |
| Solana (validator) | Rust | ~300K+ | 30+ (Anza) | 5 years | Mainnet |
| Lighthouse (Eth CL) | Rust | ~200K | 10+ (Sigma Prime) | 5 years | Mainnet |
| Aptos | Rust/Move | ~400K+ | 50+ | 3 years | Mainnet |
| Reth | Rust | ~150K+ | 15+ (Paradigm) | 2 years | Mainnet |
| **Aztibase** | **Rust** | **~28K** | **1 developer** | **26 sprints** | **Testnet-ready** |

**Assessment:** At ~10-15% of a production L1 by code volume. The hardest 10-15% (architecture, crypto, core protocol) is done. Remaining work is scale, hardening, and ecosystem.

**Competitive advantages:**
- AI-native at protocol level (no production L1 has this)
- Dual-VM (WASM + EVM) with cross-VM bridge and reentrancy protection
- Consumer hardware validator target (8 GB RAM vs Solana's 128 GB)
- Pure Rust philosophy (single C exception: blst for BLS12-381)
- Solo-developer output rivaling small funded teams

---

### 5.3 Gap Analysis — What Can Be Closed Solo vs. Requires Team

#### Achievable solo (~12-15 sprints total)

| Gap | Description | Estimated sprints |
|---|---|---|
| Networking hardening | Eclipse attack resistance, NAT traversal, peer reputation scoring. Patterns documented in `references/rust-libp2p/` and `references/lighthouse/`. | 3-4 |
| Consensus adversarial testing | Partition simulation, Byzantine fault injection, liveness testing. Infrastructure exists, needs test scenarios. | 2-3 |
| State pruning & scale | Archive vs full node modes, extended state eviction. Count-based patterns from Sprint 024 as foundation. | 2-3 |
| Verkle IPA commitment | Swap BLAKE3 placeholder when a mature pure-Rust IPA crate ships. `StateCommitment` trait designed for this. | 1-2 (when crate exists) |
| Ecosystem basics | Block explorer, TypeScript SDK (wrapping WASM crate), CLI wallet improvements. | 3-4 |

#### Requires scaling beyond solo

| Gap | Why solo isn't enough | What's needed |
|---|---|---|
| Client diversity | A second independent implementation requires a separate team writing from spec. | 1 additional team (3-5 engineers) |
| Formal verification | Consensus liveness/safety proofs (TLA+, Coq, Lean). Specialized academic work. | 1 formal methods researcher |
| Full security audit | Self-review is good but mainnet-grade requires independent third-party audit. | Budget: $200K-$500K |
| Ecosystem at scale | Wallets, DEX integrations, bridges, indexers, faucets, docs sites, DevRel. | 5-10 people |
| 24/7 validator operations | Production network requires on-call infrastructure, incident response, upgrade coordination. | DevOps team + validator community |

---

### 5.4 Validator Requirements

#### BFT Validator Count (3f+1 formula)

| Fault tolerance (f) | Validators needed | Can survive |
|---|---|---|
| 1 | 4 | 1 malicious/offline validator |
| 2 | 7 | 2 malicious/offline |
| 3 | 10 | 3 malicious/offline |
| 5 | 16 | 5 malicious/offline |
| 10 | 31 | 10 malicious/offline |
| 33 | 100 | 33 malicious/offline |

**From MASTER_DESIGN.md:**
- Minimum validator set: 21 (for meaningful BFT, f=7)
- Target validator set: 100-200 (balances decentralization with DAG overhead)

#### Phased validator growth

| Phase | Validators | Purpose |
|---|---|---|
| Dev testnet (current) | 3 | Functional testing (f=0, no fault tolerance) |
| Public testnet | 4-10 | BFT functional (f=1 to f=3), real consensus dynamics |
| Incentivized testnet | 15-25 | External operators, stress-test onboarding, economics validation |
| Mainnet genesis | 20-50 | Credible launch (Sui launched with ~100 but had $300M funding) |

**Key insight:** Independence matters more than count. 10 validators run by 10 different operators in 10 jurisdictions > 100 validators run by 3 whales on AWS. Requirements: geographic distribution, operator diversity, no single validator > 33% of stake.

#### Hardware requirements per validator

**Minimum (consensus only):**

| Resource | Requirement | Rationale |
|---|---|---|
| CPU | 4 cores | DAG vertex validation, BLS signature verification |
| RAM | 8 GB | Mempool (10K tx), DAG state, Block-STM MVMemory |
| Storage | 100 GB SSD | redb state DB + consensus DB |
| Network | 50 Mbps symmetric | 400ms block time vertex propagation |
| OS | Linux (Ubuntu 22.04+) | Standard for validators |

**Recommended (consensus + PoUW compute):**

| Resource | Requirement | Rationale |
|---|---|---|
| CPU | 8+ cores | Parallel tx execution, AI attestation verification |
| RAM | 16-32 GB | WASM/EVM execution, tract ONNX model loading |
| Storage | 500 GB NVMe SSD | State growth, receipt history |
| Network | 100+ Mbps symmetric | Full gossipsub propagation |
| GPU | Optional | Only for AI inference PoUW rewards |

**Industry comparison:**

| Network | Min RAM | Min CPU | Min Storage |
|---|---|---|---|
| Ethereum | 16 GB | 4 cores | 2 TB SSD |
| Solana | 128 GB | 12 cores | 2 TB NVMe |
| Sui | 32 GB | 8 cores | 4 TB NVMe |
| Cosmos | 16 GB | 4 cores | 500 GB SSD |
| **Aztibase** | **8 GB** | **4 cores** | **100 GB SSD** |

Consumer hardware target is a major advantage — hobbyist validators can participate.

---

### 5.5 Infrastructure & Funding Strategy

#### Free/near-free testnet deployment

**Single machine (current):**
Run 4-7 validator containers via Docker Compose on existing PC (16 GB RAM). Cost: $0.

**Oracle Cloud Always Free tier (per account, forever):**

| Resource | Amount |
|---|---|
| ARM instances (Ampere A1) | 4 OCPUs + 24 GB RAM total |
| AMD instances (E2.1.Micro) | 2 instances, 1 GB RAM each |
| Boot volume storage | 200 GB total |
| Outbound data | 10 TB/month |

**Recommended Oracle split — Option A (4 validators):**

| Instance | CPUs | RAM | Role |
|---|---|---|---|
| validator-1 | 1 OCPU | 6 GB | Validator |
| validator-2 | 1 OCPU | 6 GB | Validator |
| validator-3 | 1 OCPU | 6 GB | Validator |
| validator-4 | 1 OCPU | 6 GB | Validator |

**Recommended Oracle split — Option B (3 validators + monitoring):**

| Instance | CPUs | RAM | Role |
|---|---|---|---|
| validator-1 | 1 OCPU | 8 GB | Validator |
| validator-2 | 1 OCPU | 8 GB | Validator |
| validator-3 | 1 OCPU | 8 GB | Validator |
| monitoring | 1 OCPU | 4 GB | Prometheus + Grafana |

**Maximum squeeze — Option C (6 deployments):**

| Instance | CPUs | RAM | Role |
|---|---|---|---|
| validator 1-3 | 1 OCPU each | 4 GB each | 3 validators |
| monitoring | 0.5 OCPU | 4 GB | Prometheus + Grafana |
| micro-1 (AMD) | 1/8 OCPU | 1 GB | RPC endpoint / faucet |
| micro-2 (AMD) | 1/8 OCPU | 1 GB | Block explorer |

**Important:** Oracle limits one Always Free account per person. Creating multiple accounts violates ToS and risks termination of all accounts.

#### Other free-tier providers (one account each, all legitimate)

| Provider | Free offer | Use for |
|---|---|---|
| Oracle Cloud | 4 ARM cores, 24 GB RAM (always free) | 4 validators |
| Google Cloud | e2-micro (1 GB), always free | 1 light node |
| Azure | B1s (1 GB), 750 hrs/month for 12 months | 1 light node |
| AWS | t3.micro (1 GB), 12 months | 1 light node |
| Fly.io | 3 shared VMs, 256 MB each | 1 lightweight service |

**Combined total: 4 validators + 3-4 auxiliary services, all free, all legitimate.**

#### Beyond free tier — validator community bootstrapping

Validators pay for themselves — that's the token economy. Bootstrapping strategies:

1. **Testnet validator program:** "Run an Aztibase validator, earn mainnet allocation." Enthusiasts run nodes on their own hardware for future token promise. Cosmos, Celestia, and dozens of others launched this way.
2. **Existing assets:** aztibase.com, @aztibase on X, registered company — enough credibility to recruit 10-20 testnet operators.
3. **Grants:** Web3 Foundation, Ethereum Foundation, Protocol Labs, Gitcoin/Optimism RetroPGF.

#### What actually costs money

| Need | When | Estimated cost |
|---|---|---|
| Security audit | Pre-mainnet | $200-500K |
| Domain/legal | Already done | $0 |
| Cloud for CI/CD | Now | $0-20/month (GitHub Actions free tier) |
| Testnet infra | Now | $0-50/month (Docker on PC + Oracle free tier) |
| First hire | Post-traction | Variable |

---

### 5.6 Code Distribution for Validators

#### Options for sharing code with validators

| Approach | Pros | Cons |
|---|---|---|
| **Full open source** (public repo) | Validators audit code, security researchers find bugs for free, builds trust | Competitors can see everything |
| **Prebuilt binaries only** | Source stays private, validators just run the binary | Less trust, harder to audit |
| **Hybrid** (recommended) | Protocol code public, strategy/planning private | Two repos to maintain |

**Recommended hybrid approach:**

| Public repo (`aztibase/aztibase-node`) | Private repo (current workspace) |
|---|---|
| All 9 crates (core, consensus, storage, network, execution, runtime, rpc, node, wasm) | Sprint plans, skills, internal docs |
| Docker setup scripts | BUILD_LOG, STATUS, DECISIONS |
| README, LICENSE, basic docs | `.claude/` directory |
| WASM light client | `blockchain-project/` planning docs |
| Monitoring configs | `references/` (third-party code) |

**Never publish (in any repo):**
- `.claude/` directory (AI workflow and skills)
- Private keys / genesis secrets
- `blockchain-project/*.md` internal strategy docs
- `references/` (third-party code you don't own)
- `.env` files, `memory/` session context

**Key insight:** The competitive advantage is NOT the code — it's the architecture decisions, brand (aztibase.com, @aztibase, registered company), validator community, execution speed (26 sprints of momentum), and AI-native design. Ethereum has hundreds of forks; none of them matter.

---

### 5.7 Phased Launch Roadmap

```
NOW              → Dev testnet (Docker on local PC, 3 validators)
Solo work        → Hardened testnet (12-15 sprints)
                   - Networking hardening (3-4 sprints)
                   - Adversarial consensus testing (2-3 sprints)
                   - State pruning & scale (2-3 sprints)
                   - Ecosystem basics (3-4 sprints)
                   - Verkle IPA swap (1-2 sprints, when crate exists)
Public testnet   → Oracle Cloud free tier (4 validators)
                   + validator recruitment program
First hires      → 2-3 engineers (consensus, networking, ecosystem)
Audit            → Third-party security audit ($200-500K)
Community        → Validator program, developer grants
Mainnet          → With 5-10 person core team

Key milestones:
- 4 validators on Oracle free tier          → BFT functional
- 10 community validators                   → Credible decentralization
- Security audit complete                   → Mainnet-grade assurance
- 20-50 validators at genesis              → Mainnet launch
```

---

### 5.8 Reference: Industry Launch Comparisons

| Network | Testnet validators | Mainnet launch validators | Funding at launch |
|---|---|---|---|
| Sui | 4 (devnet) → 20 (testnet) | ~100 | $336M |
| Aptos | 25 (testnet) | ~100 | $350M |
| Solana | ~10 (testnet) | ~500 | $25M |
| Cosmos Hub | 50 (genesis) | ~175 | ~$17M (ICO) |
| Avalanche | 5 (testnet) | ~1,000 | $42M |
| Reth | N/A (execution client) | N/A | Paradigm-backed |
| **Aztibase** | **3 (dev)** | **TBD (20-50 target)** | **$0 (bootstrapped)** |

Lean launches are viable. Solana and Cosmos both started with modest resources relative to later-stage projects.

---

## FP-006: Privacy Layer — TEE + Encrypted Inference

**Status:** PLANNED — M9+ (post-mainnet hardening)
**Priority:** HIGH — required before "private AI" can be claimed
**Estimated effort:** 3-4 sprints
**Author:** Project Lead
**Date:** 2026-03-09
**Code status:** ZERO CODE — design only

### Problem

All inference requests and results are currently transparent on-chain. Anyone can see what model was called, what input was sent, and what output was returned. This makes the "private AI" value proposition undeliverable today.

### What Needs to Be Built

#### Phase 1: Encrypted Inference Requests (1 sprint)
- `EncryptedInferenceRequest`: input encrypted with validator's public key (X25519 or similar)
- Validator decrypts in-memory, runs inference, returns encrypted result
- Only requester + assigned validator see plaintext
- New TxKind or extension to existing AiInfer with encrypted payload
- Key exchange protocol between requester and validator

#### Phase 2: TEE Integration (1-2 sprints)
- Intel SGX / AMD SEV / ARM TrustZone enclave support for validators
- Model loaded inside enclave, inference runs inside enclave
- Remote attestation: validator proves to network that they're running genuine code in a genuine enclave
- Attestation certificate posted on-chain (verifiable by light clients)
- Neither model weights nor input data are exposed to the validator's host OS

#### Phase 3: ZK Inference Verification (1 sprint, research-heavy)
- ZK proof that inference was computed correctly without revealing input/output
- Likely requires specialized ZK circuits per model architecture (expensive to build)
- Alternative: optimistic verification with fraud proofs (cheaper, weaker guarantees)
- Evaluate: RISC Zero, SP1, or Jolt for general-purpose ZK-VM approach

### Dependencies
- Mature Rust TEE libraries (gramine-rs, fortanix-edp, or similar)
- ZK proof system selection (no pure-Rust production ZK-VM exists today that handles ML inference)
- Validator hardware requirements increase (TEE-capable CPUs)

### Risk
- TEE adds hardware requirements — conflicts with "consumer hardware" validator target
- ZK inference proofs are cutting-edge research — may not be production-ready for 1-2 years
- Mitigation: TEE is optional (validators opt in for premium tasks), non-TEE validators handle public inference

---

## FP-007: Selective Disclosure Identity

**Status:** PLANNED — M9+ (post-privacy layer)
**Priority:** MEDIUM — enterprise adoption driver
**Estimated effort:** 2-3 sprints
**Author:** Project Lead
**Date:** 2026-03-09
**Code status:** ZERO CODE — design only

### Problem

Proving attributes (age, KYC status, nationality) currently requires exposing full identity documents. No on-chain primitive exists for "prove X without revealing Y."

### What Needs to Be Built

#### Phase 1: Credential Schema + Issuance (1 sprint)
- `CredentialSchema`: defines what attributes a credential contains (e.g., "age", "country", "kyc_level")
- `IssuedCredential`: signed by an issuer (KYC provider, government, etc.), stored off-chain (user's device)
- `CredentialIssuer` registry on-chain: trusted issuers with public keys
- W3C Verifiable Credentials alignment for interop

#### Phase 2: ZK Selective Disclosure (1-2 sprints)
- User generates a ZK proof: "I hold a credential from issuer X that says attribute Y satisfies condition Z"
- Example: "I have a KYC credential from Issuer A that says my age >= 18" — without revealing name, DOB, or anything else
- On-chain verifier contract or protocol-level verification
- Proof system: BBS+ signatures (privacy-preserving, supports selective disclosure natively) or Groth16/PLONK circuits

#### Phase 3: Protocol Integration (1 sprint)
- `TxKind::PresentCredential`: attach a ZK proof to any transaction as authorization
- Smart contracts can require credential proofs as preconditions (e.g., "only KYC-verified addresses can use this DEX pool")
- Governance: proposals can require credential proofs from voters

### Dependencies
- FP-006 (privacy layer) for encrypted credential storage
- BBS+ or similar ZK-friendly signature scheme (Rust crate maturity TBD)
- Credential issuer ecosystem (third-party KYC providers, identity platforms)

### Risk
- Chicken-and-egg: credentials need issuers, issuers need users, users need utility
- Mitigation: start with self-issued credentials (prove ownership of another chain's address, prove testnet participation) before external issuers

---

## FP-008: Autonomous AI Agent Transactions

**Status:** PLANNED — M9 (pre-mainnet)
**Priority:** HIGH — core to "agent economy" pitch
**Estimated effort:** 1-2 sprints
**Author:** Project Lead
**Date:** 2026-03-09
**Code status:** 20% — AIAgent account type exists, no autonomous execution

### Current State

- `TxKind::CreateAgent` (0x07) creates an AIAgent account on-chain
- Agent has balance, nonce, model_id
- BUT: agents cannot initiate transactions — they are passive accounts
- No spending limits, no authorization delegation, no agent-to-agent payments

### What Needs to Be Built

#### Phase 1: Agent Authorization Framework (1 sprint)
- `AgentPolicy`: spending cap (per-tx, per-epoch), allowed TxKinds, allowed recipients, expiry round
- `TxKind::SetAgentPolicy`: human principal sets/updates agent constraints
- `TxKind::AgentExecute`: agent-initiated transaction, validated against policy before execution
- Agent signing: agent has its own keypair (derived at creation), can sign txs within policy bounds
- Pipeline: validate AgentExecute against stored policy, reject if over budget or unauthorized

#### Phase 2: Agent-to-Agent Communication (1 sprint)
- `TxKind::AgentMessage`: lightweight on-chain message between agents (structured data, not free-form)
- Agent discovery: query agents by model_id or capability
- Payment channels: agents can open micro-payment channels for high-frequency interactions
- Composability: agent can call another agent's inference endpoint and pay automatically

### Dependencies
- Sprint 038 staking (agents may need to stake for certain operations)
- FP-006 privacy layer (agents handling sensitive data need encrypted communication)

### Risk
- Autonomous spending by AI agents is a regulatory grey area
- Mitigation: human principal always sets policy; agent cannot exceed policy bounds; policy revocation is immediate

---

## FP-009: ZK Proof System Integration

**Status:** PLANNED — M9+ (research-dependent)
**Priority:** MEDIUM — enables FP-006, FP-007, and future scaling
**Estimated effort:** 2-3 sprints
**Author:** Project Lead
**Date:** 2026-03-09
**Code status:** ZERO CODE — only a placeholder StateCommitment trait exists

### Problem

Multiple features depend on ZK proofs (private inference verification, selective disclosure, potential ZK rollup support), but no ZK proof system is integrated.

### Options Under Evaluation

| System | Approach | Rust Support | Maturity | Fit |
|--------|----------|-------------|----------|-----|
| **RISC Zero** | ZK-VM (runs arbitrary RISC-V programs in ZK) | Yes (pure Rust guest programs) | Production | Best general-purpose fit |
| **SP1** (Succinct) | ZK-VM (RISC-V, optimized prover) | Yes | Production | Faster proofs than RISC Zero |
| **Jolt** (a16z) | ZK-VM (RISC-V, lookup-based) | Yes | Early | Promising, less mature |
| **Halo2** (PSE/Zcash) | Custom circuits | Yes | Production | Flexible but complex |
| **Groth16** (arkworks) | Custom circuits, trusted setup | Yes (arkworks) | Mature | Fast verification, setup ceremony needed |
| **BBS+** | Selective disclosure signatures | Partial | Research | Best for FP-007 identity specifically |

### Recommended Approach
1. Start with **RISC Zero** or **SP1** for general-purpose ZK-VM (can prove arbitrary Rust code)
2. Add **BBS+** for identity-specific selective disclosure (FP-007)
3. Evaluate custom circuits (Halo2) only if ZK-VM performance is insufficient for inference verification

### What Needs to Be Built
- `ZkVerifier` trait in aztibase-core (verify proof against public inputs)
- `TxKind::SubmitZkProof`: post a ZK proof on-chain for verification
- Proof storage: receipts include proof verification status
- Precompile or native opcode for ZK verification in WASM/EVM contracts

### Dependencies
- Pure Rust requirement: RISC Zero and SP1 both have C/C++ dependencies in their provers. Verification can be pure Rust. ADR needed (similar to ADR-004 for blst).

---

## Future Planning Index

| FP | Title | Status | Priority | Code Status |
|----|-------|--------|----------|-------------|
| FP-001 | Community Task Layer (L1 proposal → see FP-004) | PROPOSAL | HIGH | 0% |
| FP-002 | Delegation / Staking | SUPERSEDED by Sprint 038 | — | Sprint 038 |
| FP-003 | Mobile Light Client App | PROPOSAL | HIGH | 0% |
| FP-004 | L2 Community Task Chain (sovereign rollup) | PROPOSAL | HIGH | 0% |
| FP-005 | Project Assessment & Launch Roadmap | REFERENCE | HIGH | N/A |
| FP-006 | Privacy Layer — TEE + Encrypted Inference | PLANNED (M9+) | HIGH | 0% |
| FP-007 | Selective Disclosure Identity | PLANNED (M9+) | MEDIUM | 0% |
| FP-008 | Autonomous AI Agent Transactions | PLANNED (M9) | HIGH | 20% |
| FP-009 | ZK Proof System Integration | PLANNED (M9+) | MEDIUM | 0% |
