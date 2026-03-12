# Architecture Decision Records (ADRs) -- Aztibase Network

Every non-obvious technical decision is recorded here. Each ADR is immutable once written -- if a decision is reversed, a new ADR supersedes it with a reference to the original.

---

## ADR Index

| ADR | Title | Date | Status | Decided By |
|-----|-------|------|--------|------------|
| ADR-001 | Switch storage from RocksDB to redb | 2026-03-05 | ACCEPTED | blockchain-architect |
| ADR-002 | Dual-license MIT / Apache-2.0 | 2026-03-05 | ACCEPTED | legal-ip-counsel |
| ADR-003 | Workspace monorepo with 8 crates | 2026-03-05 | ACCEPTED | blockchain-architect |
| ADR-004 | Use blst crate for BLS12-381 (C dependency exception) | 2026-03-06 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-005 | Use axum for JSON-RPC server | 2026-03-06 | ACCEPTED | node-engineer |
| ADR-006 | Post-execution fee collection (not pre-execution escrow) | 2026-03-07 | ACCEPTED | smart-contract-engineer |
| ADR-007 | StateCommitment trait with enum-based proofs | 2026-03-07 | ACCEPTED | blockchain-architect |
| ADR-008 | Metrics via opaque JSON to avoid cross-crate coupling | 2026-03-07 | ACCEPTED | node-engineer |
| ADR-009 | WebSocket gateway via axum upgrade (not separate server) | 2026-03-07 | ACCEPTED | node-engineer |
| ADR-010 | PoUW scoring formula and attestation quorum design | 2026-03-07 | ACCEPTED | blockchain-architect + consensus-engineer |
| ADR-011 | Attestation signature scheme and settlement flow | 2026-03-07 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-012 | Packed SignerBitmap for finality certificates | 2026-03-08 | ACCEPTED | security-engineer + blockchain-architect |
| ADR-013 | Retain BLAKE3 Verkle placeholder over IPA polynomial commitments | 2026-03-08 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-014 | u128 tokenomics with deferred u64→u128 balance migration | 2026-03-09 | ACCEPTED | blockchain-architect + tokenomics-engineer |
| ADR-015 | Equivocation detection window: 100 rounds + persistent proofs | 2026-03-10 | ACCEPTED | consensus-engineer + security-engineer |
| ADR-016 | Consensus-layer transaction size limits (MEV mitigation Phase 1) | 2026-03-10 | ACCEPTED | security-engineer + node-engineer |
| ADR-017 | Quantum migration drill via StateCommitment trait | 2026-03-10 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-018 | RANDAO-style VRF seed accumulation (anti-last-revealer) | 2026-03-10 | ACCEPTED | consensus-engineer + security-engineer |
| ADR-019 | Browser wallet spending limits in WASM | 2026-03-10 | ACCEPTED | security-engineer + p2p-network-engineer |
| ADR-020 | Quorum-signed DHT records (anti-poisoning) | 2026-03-10 | ACCEPTED | p2p-network-engineer + security-engineer |
| ADR-021 | No mempool persistence — re-gossip from peers on restart | 2026-03-10 | ACCEPTED | node-engineer + blockchain-architect |
| ADR-022 | Protocol version negotiation via libp2p identify | 2026-03-10 | ACCEPTED | p2p-network-engineer + blockchain-architect |
| ADR-023 | L2 bridge design — challenge window, proof format, governance gate | 2026-03-11 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-024 | Protocol store persistence via STATE_TABLE with postcard serialization | 2026-03-11 | ACCEPTED | node-engineer + blockchain-architect |
| ADR-025 | Network profiles & mainnet operational hardening | 2026-03-11 | ACCEPTED | blockchain-architect + security-engineer |
| ADR-026 | Public testnet launch infrastructure | 2026-03-11 | ACCEPTED | node-engineer + documentation-engineer |
| ADR-027 | Full state snapshots & mainnet genesis ceremony | 2026-03-11 | ACCEPTED | node-engineer + blockchain-architect |
| ADR-028 | nChain patent FTO preliminary analysis | 2026-03-12 | ACCEPTED | legal-ip-counsel + blockchain-architect |
| ADR-029 | quinn-proto security patch (RUSTSEC-2026-0037) | 2026-03-12 | ACCEPTED | security-engineer |

---

## ADR-001: Switch storage from RocksDB to redb

**Date:** 2026-03-05
**Status:** ACCEPTED
**Decided By:** blockchain-architect
**Git Ref:** dd98662

### Context
The initial design specified RocksDB as the storage backend. During scaffolding, the decision was made to switch to redb.

### Decision
Use redb as the primary embedded database for full nodes. RocksDB remains a consideration for high-throughput validator nodes if benchmarks demand it.

### Rationale
- redb is pure Rust -- no C++ dependency chain, simpler builds, better cross-compilation
- Lighter weight for light/mobile nodes (aligns with M5 goals)
- ACID transactions built-in
- Better alignment with the "Rust-native" tech stack philosophy

### Consequences
- May need to revisit for validator-tier throughput at M7
- Verkle tree implementation sits on top of redb's key-value interface

---

## ADR-002: Dual-license MIT / Apache-2.0

**Date:** 2026-03-05
**Status:** ACCEPTED
**Decided By:** legal-ip-counsel

### Context
Open-source license selection for the Aztibase Network codebase.

### Decision
Dual-license under MIT and Apache-2.0, following the Rust ecosystem convention.

### Rationale
- Maximum compatibility with the Rust ecosystem
- Apache-2.0 provides patent protection
- MIT provides simplicity and broad adoption
- Consistent with dependencies (tokio, libp2p, etc.)

### Consequences
- Contributors must agree to dual-license their contributions
- Enterprise users get patent grant via Apache-2.0

---

## ADR-003: Workspace monorepo with 8 crates

**Date:** 2026-03-05
**Status:** ACCEPTED
**Decided By:** blockchain-architect

### Context
Project structure decision for the Rust codebase.

### Decision
Single Cargo workspace with 8 domain-specific crates:
- aztibase-core (primitives, crypto, types)
- aztibase-consensus (SynBFT, PoUW, DAG)
- aztibase-network (P2P, transport, gossip)
- aztibase-storage (redb, verkle trees)
- aztibase-execution (VM, state, parallel execution)
- aztibase-runtime (contracts, AI oracle, agents)
- aztibase-rpc (JSON-RPC server)
- aztibase-node (binary entry point)

### Rationale
- Clean separation of concerns matching engineer domains
- Independent compilation and testing per crate
- Clear dependency graph (core -> consensus -> network -> etc.)
- Each engineer owns their crate(s)

### Consequences
- Cross-crate API design is critical -- breaking changes propagate
- Need clear ownership and review process for shared types in aztibase-core

---

## ADR-004: Use blst crate for BLS12-381 (C dependency exception)

**Date:** 2026-03-06
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer

### Context
BLS finality certificates require BLS12-381 signature primitives (key generation, signing, verification, aggregation). ADR-001 established a pure-Rust dependency policy. BLS12-381 implementations available:
- `blst` (v0.3): C library with Rust bindings. Used by Lighthouse, Prysm, and other production validators. Audited.
- `bls12_381` (zkcrypto): Pure Rust, low-level. No high-level BLS signature API — would require hand-rolling aggregate signature scheme.
- `ark-bls12-381` (arkworks): Pure Rust, heavy dependency tree. Not audited for BLS signature use.

### Decision
Use `blst` v0.3 as a justified exception to the pure-Rust policy. This is the only C dependency in the project.

### Rationale
- BLS signature correctness is security-critical — aggregate signature bugs can forge finality proofs
- `blst` is the only production-audited BLS12-381 implementation available
- Used by Ethereum's two largest consensus clients (Lighthouse, Prysm)
- Rolling our own from low-level primitives introduces unacceptable risk
- `blst` compiles on all target platforms (Linux, macOS, Windows) via `cc`

### Consequences
- Requires C compiler in build toolchain (MSVC on Windows, gcc/clang on Unix)
- Cross-compilation becomes slightly harder
- If a production-grade pure-Rust BLS library emerges, this ADR should be superseded
- Proof-of-possession (PoP) required at validator registration to prevent rogue-key attacks

---

## ADR-005: Use axum for JSON-RPC server

**Date:** 2026-03-06
**Status:** ACCEPTED
**Decided By:** node-engineer

### Context
The node needs an HTTP-based JSON-RPC 2.0 server for external clients (wallets, explorers, CLI tools). Options considered:
- `axum`: tokio-native, minimal, tower middleware ecosystem, maintained by tokio team
- `hyper` (raw): maximum control but requires hand-rolling routing and middleware
- `jsonrpsee`: full JSON-RPC framework with WebSocket support — heavy, opinionated

### Decision
Use axum with a hand-rolled JSON-RPC 2.0 dispatch layer. No dependency on a JSON-RPC framework.

### Rationale
- axum is already tokio-native — zero async runtime conflicts
- Hand-rolled dispatch keeps the RPC layer simple and auditable (one file, ~280 lines)
- jsonrpsee would add significant dependency weight for features we don't need yet (WebSocket, subscriptions)
- 1MB body size limit via `DefaultBodyLimit` prevents DoS via oversized requests
- tower middleware ecosystem available for future rate limiting, auth, CORS

### Consequences
- WebSocket subscriptions (e.g., newBlock events) require additional work when needed (M4+)
- Batch JSON-RPC requests (array of requests) not supported — add if needed
- No built-in OpenRPC/spec generation — acceptable for testnet

---

## ADR-006: Post-execution fee collection (not pre-execution escrow)

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** smart-contract-engineer
**Git Ref:** pending

### Context
Sprint 011 introduces gas fees. Two approaches were considered:
- **Pre-execution escrow**: Deduct `gas_limit * gas_price` before execution, refund unused gas after. Prevents underfunded execution but complicates Block-STM parallel execution (requires per-tx balance locking).
- **Post-execution collection**: Execute first, then deduct `gas_used * gas_price` from sender balance. Simpler but allows transactions to execute even if sender can't fully cover fees.

### Decision
Use post-execution fee collection (`collect_fees()` in pipeline.rs) for Sprint 011. The pre-execution escrow utilities (`escrow_fee()`, `refund_unused()` in fee.rs) are implemented but not wired into the pipeline yet.

### Rationale
- Block-STM parallel execution modifies balances during execution — pre-execution escrow would require coordinating the escrow lock with Block-STM's MVMemory, adding significant complexity
- Post-execution is simpler to implement and test
- Pre-mainnet: zero-fee transactions (gas_price=0) are common in testing, making escrow overhead unnecessary
- The `escrow_fee` / `refund_unused` pattern is ready to wire in when the fee model matures

### Consequences
- SEC-FEE-002: Senders with insufficient balance can still transact (fee capped at available balance via `saturating_sub`)
- Must wire pre-execution escrow before mainnet to prevent fee-free execution abuse
- `collect_fees()` iterates routed txs linearly, matching to receipts by index — assumes 1:1 correspondence

---

## ADR-007: StateCommitment trait with enum-based proofs

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** blockchain-architect
**Git Ref:** pending (Sprint 014)

### Context
The execution layer needs to support both binary Merkle trees (current) and Verkle trees (target). Two proof representation approaches were considered:
- **Trait object proofs**: `Box<dyn Proof>` — flexible but requires downcasting and loses type information
- **Enum-based proofs**: `StateProof::Merkle(MerkleProof) | StateProof::Verkle(VerkleProof)` — closed set, pattern matching, zero allocation

### Decision
Use a `StateCommitment` trait with an enum-based `StateProof` type. The trait defines `commit()`, `prove()`, and `verify()` methods. `MerkleCommitment` and `VerkleCommitment` are concrete implementations.

### Rationale
- Closed enum is exhaustive — compiler catches missing cases when new proof types are added
- No heap allocation for proof dispatch (enum is stack-sized)
- `StateProof` can derive Clone, Debug, PartialEq — trait objects cannot
- Only two proof types exist (Merkle, Verkle) — extensibility is not a concern
- Verification code can pattern-match on proof variant for type-safe handling

### Consequences
- Adding a third commitment scheme requires modifying the `StateProof` enum (acceptable — unlikely to happen)
- Both backends must be in scope wherever `StateProof` is matched (compile-time guarantee, not runtime overhead)
- The trait allows runtime backend selection (e.g., config-driven Merkle vs Verkle)

---

## ADR-008: Metrics via opaque JSON to avoid cross-crate coupling

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** node-engineer
**Git Ref:** pending (Sprint 015)

### Context
The node binary needs to expose consensus and execution metrics via an HTTP endpoint served by the RPC crate. However, `aztibase-rpc` must not depend on `aztibase-consensus` (would create a dependency cycle through `aztibase-node`). Three approaches were considered:
- **Shared metrics crate**: New `aztibase-metrics` crate that both consensus and RPC depend on
- **Trait-based metrics**: Define a `MetricsProvider` trait in a shared location
- **Opaque JSON**: Pass metrics as `Arc<RwLock<serde_json::Value>>` from node binary to RPC server

### Decision
Use `Arc<RwLock<serde_json::Value>>` as an opaque metrics container. The node binary (which has access to both consensus and RPC) updates the JSON periodically. The RPC server serves it at `GET /metrics` without understanding its structure.

### Rationale
- Zero new crates — avoids workspace bloat for a single use case
- No cross-crate coupling — RPC sees only `serde_json::Value`
- Extensible — any new metrics can be added to the JSON without changing RPC code
- `serde_json` is already a workspace dependency
- Trade-off: no compile-time schema enforcement (acceptable for telemetry)

### Consequences
- Metrics JSON is eventually consistent — readers may see partially-updated snapshots
- No compile-time type safety on metric names (mitigated by tests)
- If structured metrics are needed later (e.g., Prometheus format), this can be refactored without changing the RPC interface

---

## ADR-009: WebSocket gateway via axum upgrade (not separate server)

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** node-engineer
**Git Ref:** pending (Sprint 020)

### Context
The browser light client (Sprint 019) requires a WebSocket connection to a full node for header sync and event streaming. ADR-005 chose axum for HTTP-only JSON-RPC. Three approaches for WebSocket support:
- **Separate WebSocket server**: Run a second listener (e.g., on port 9945) with a dedicated WS library
- **jsonrpsee migration**: Replace hand-rolled dispatch with jsonrpsee (has built-in WS+subscriptions)
- **axum upgrade**: Add `/ws` route to existing axum router using `axum::extract::ws`, sharing state

### Decision
Add WebSocket support via axum's built-in `ws` feature on the same port as the HTTP RPC server. WebSocket clients connect to `/ws` and can send JSON-RPC requests, light sync messages, and subscriptions over the same connection.

### Rationale
- Zero new dependencies — axum already supports WebSocket via the `ws` feature flag
- Same port for HTTP and WS — simplifies deployment (one port to expose/firewall)
- Shared `RpcState` — WebSocket handler reuses existing `dispatch()` for JSON-RPC methods
- jsonrpsee migration would require rewriting all existing RPC handlers for no clear benefit
- Subscription model (broadcast channels + per-client forwarding tasks) is simple and auditable

### Consequences
- WebSocket and HTTP share the same `DefaultBodyLimit` (1MB) — appropriate for both
- Subscription tasks are per-client, spawned on subscribe, aborted on unsubscribe or disconnect
- Max 256 concurrent WebSocket connections enforced at upgrade time; production deployments should add per-IP limits via reverse proxy

---

## ADR-010: PoUW scoring formula and attestation quorum design

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** blockchain-architect + consensus-engineer
**Git Ref:** pending (Sprint 021)

### Context
The Proof of Useful Work (PoUW) system needs a concrete scoring formula to rank validators by their AI compute performance. The formula must balance multiple metrics (accuracy, speed, availability) and resist gaming. Additionally, attestation consensus requires a quorum mechanism to prevent single-validator results from being accepted.

### Decision
1. **Scoring formula**: `0.4 * accuracy + 0.3 * latency_score + 0.3 * availability_score`
   - `accuracy` = accepted_attestations / total_attestations in sliding window
   - `latency_score` = 1.0 - (avg_latency / max_latency), clamped to [0, 1]
   - `availability_score` = min(tasks_completed / expected_tasks_per_window, 1.0)
2. **Attestation quorum**: Minimum 2 validators must submit matching `result_hash` for a task result to be accepted. Attestations are deduplicated by `validator_id`.
3. **Settlement**: Reward is split equally among quorum validators, with remainder distributed to the first validators in order.

### Rationale
- **Weighted multi-metric** prevents gaming: a validator can't get high scores by being fast but inaccurate, or accurate but rarely available
- **40% accuracy weight** makes correctness the most important factor — matches the network's AI-native priority
- **30/30 latency/availability split** ensures validators can't game by cherry-picking easy tasks (availability penalizes inactivity)
- **Quorum ≥ 2** is the minimum viable decentralization — prevents single-validator collusion while keeping overhead low
- **Validator dedup** prevents Sybil: a validator running multiple attestation submissions counts as one vote
- **Equal reward split** is simple and fair — more sophisticated mechanisms (proportional to stake, score-weighted) can be added in M7

### Consequences
- Validators with consistently accurate, fast, and available inference will earn higher PoUW scores and be preferred for task assignment
- The formula parameters (weights, max_latency, expected_tasks_per_window) are configurable per-network
- StubPoUWScore remains available as fallback for networks without AI compute validators
- Future work: weighted reward distribution, dynamic quorum based on task reward value, attestation signature verification

---

## ADR-011: Attestation Signature Scheme and Settlement Flow

**Date:** 2026-03-07
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer
**Git Ref:** pending (Sprint 022)

### Context

Sprint 022 wires the task execution loop end-to-end: PostTask → TaskPool → SubmitAttestation → quorum check → reward settlement. The attestation flow requires signature verification to ensure only the claimed validator submitted the attestation, and settlement must be atomic to prevent partial reward distribution.

### Decision

1. **Ed25519 signatures on attestation_hash**: Validators sign `attestation_hash = BLAKE3(task_id || result_hash || compute_units || validator_id)` using their Ed25519 keypair. The pipeline verifies via `aztibase_core::PublicKey::from_bytes().verify()` — no new dependencies.

2. **TaskSettlement as reusable settlement primitive**: The `TaskSettlement::settle()` function encapsulates aggregation + reward splitting. The pipeline delegates to it rather than inlining the logic, keeping settlement logic testable in isolation.

3. **Atomic settlement**: On quorum (≥ 2 matching result_hash), the pipeline atomically removes the task from the pool, credits validator balances, and cleans up state storage and attestation buffer in a single `execute_batch` pass.

4. **Attestation buffer cleanup on expiry**: When tasks expire and are evicted from the TaskPool, their corresponding attestation buffer entries are also cleaned up to prevent unbounded memory growth.

5. **CommitCompute stake bond**: Validators lock `committed_stake` as a bond when registering as compute providers. The bond is deducted from their balance immediately; refund on deregistration is deferred to a future sprint.

### Rationale

- **Reusing aztibase_core::PublicKey**: Avoids adding ed25519-dalek as a direct dependency to the node crate. The core crate already wraps it with strict verification (rejects non-canonical S values, preventing signature malleability).
- **TaskSettlement delegation**: Keeps pipeline code focused on orchestration. Settlement logic (quorum check, reward division, remainder handling) is unit-testable without constructing full batches.
- **Attestation buffer cleanup**: Without cleanup, attestation buffers for tasks that expire before quorum would leak memory indefinitely. Cleaning up during the expiry pass ensures bounded growth.

### Consequences

- Validators must have a valid Ed25519 keypair to submit attestations (already required for transaction signing)
- Settlement is deterministic across all nodes (same quorum → same payouts)
- CommitCompute overwrites previous commitments without refunding prior stake — a known limitation addressed by future deregistration support
- The attestation buffer is ephemeral (in-memory only) — node restarts lose pending attestations, which is acceptable since unfinished quorums will eventually expire

---

## ADR-012: Packed SignerBitmap for finality certificates

**Date:** 2026-03-08
**Status:** ACCEPTED
**Decided By:** security-engineer + blockchain-architect
**Git Ref:** pending

### Context
`FinalityCertificate.signer_bitmap` used `Vec<bool>` (1 byte per validator). With 100+ validators, this wastes 7/8 of bandwidth in every finality cert transmitted to light clients.

### Decision
Replace `Vec<bool>` with a custom `SignerBitmap` struct backed by `Vec<u8>` (packed bits). Each bit represents one validator's signing status.

### Rationale
- 8x bandwidth reduction (1 bit per validator vs 1 byte)
- Zero new dependencies (hand-rolled packed bitmap vs bitvec crate)
- Only consensus `FinalityCertificate` changed; light sync/WASM types remain `Vec<bool>` for JSON interop

### Consequences
- Breaking serialization change for `FinalityCertificate` (acceptable pre-testnet)
- Future: if validator set grows >256, bitmap is still compact (32 bytes for 256 validators)

---

## ADR-013: Retain BLAKE3 Verkle placeholder over IPA polynomial commitments

**Date:** 2026-03-08
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer
**Git Ref:** pending

### Context
Sprint 025 Task 9 required evaluating whether to replace the BLAKE3-based Verkle tree with true IPA (Inner Product Argument) polynomial commitments over the Banderwagon curve. The original placeholder used raw BLAKE3 hashes as node commitments, which made proofs trivially forgeable (no binding to the tree structure).

### Options Evaluated
1. **verkle-trie crate** — Ethereum-focused, depends on banderwagon/ipa-multipoint. Immature (pre-1.0), tightly coupled to Ethereum's specific stem structure. Not pure Rust (C deps via arkworks).
2. **ipa-multipoint crate** — Low-level IPA math. Would require building the entire tree structure ourselves. Pre-alpha quality.
3. **Hand-rolled Pedersen/IPA** — Full control but months of cryptographic engineering. High audit burden.
4. **Domain-separated BLAKE3 commitments** — Keep the hash-based approach but make it properly verifiable with domain separation, full sibling commitments at each level, and bottom-up verification.

### Decision
Retain BLAKE3 domain-separated commitments (option 4). The tree structure (width-256 inner nodes, stem-based navigation) already matches the production IPA design. The commitment math is the only difference — BLAKE3 is binding but not hiding, while IPA would be both.

### Rationale
- No mature pure-Rust IPA crate exists that meets ADR-001 (no C/C++ build deps)
- The verification structure is identical: bottom-up recomputation of inner-node commitments from 256 children
- Domain separation (AZTB_VERKLE_INNER, AZTB_VERKLE_LEAF) prevents cross-domain collision
- Proofs are now properly verifiable (not trivially forgeable) — the security improvement is real
- Swapping BLAKE3 for IPA later requires changing only `leaf_commitment()` and `inner_commitment()` — the tree structure and proof format are stable

### Consequences
- Verkle proofs are large (256 × 32 bytes per level) compared to IPA proofs (~500 bytes)
- No hiding property (all commitments are deterministic) — acceptable for a public blockchain
- Future: when a mature pure-Rust IPA crate emerges, swap commitment functions only

---

## ADR-014: u128 tokenomics with deferred u64→u128 balance migration

**Date:** 2026-03-09
**Status:** ACCEPTED
**Decided By:** blockchain-architect + tokenomics-engineer
**Git Ref:** pending

### Context
MASTER_DESIGN.md Section 3.1.3 requires u128 for all balance calculations and u256 for intermediate reward math. Current account balances use u64 throughout the codebase (Account.balance, fee escrow, RPC responses, ~100+ call sites across 8 crates).

### Decision
Build tokenomics types (EmissionTracker, VestingSchedule, EpochDistribution, StakingAPY) using u128 internally. Defer the full u64→u128 balance migration to a dedicated sprint before M9 (mainnet).

### Rationale
- u64 max is ~18.4×10^18. Without 18-decimal base units, u64 comfortably holds billions of tokens for testnet operations.
- Migrating Account.balance to u128 touches AccountState, fee.rs, routing.rs, persist.rs, state_root hashing, snapshot serialization, Block-STM, RPC responses, and all test assertions — a cross-cutting change requiring its own sprint.
- Tokenomics u128 types can track emission/vesting/rewards independently and convert to u64 at account-credit boundaries during the testnet phase.
- The migration path is clear: replace `balance: u64` with `balance: u128` in Account, update serialization formats, adjust all call sites.

### Consequences
- Tokenomics emission amounts are in "whole token" units (not 18-decimal base units) during testnet
- Account balances remain u64 — sufficient for testnet but must be migrated before mainnet
- Conversion overflow is impossible since total supply (1B) fits in u64 without 18 decimals

---

## ADR-015: Equivocation detection window: 100 rounds + persistent proofs

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** consensus-engineer + security-engineer
**Git Ref:** pending (Sprint 048)

### Context
The equivocation detection window was 20 rounds (~8 seconds at 400ms/round). This means equivocations older than 20 rounds were unprovable — the `seen_authors` tracker was pruned. During network partitions or high-latency conditions, an equivocator could evade detection by delaying duplicate vertex delivery past the 8-second window.

### Decision
1. Increase `EQUIVOCATION_PRUNE_DEPTH` from 20 to 100 rounds (~40 seconds).
2. Store equivocation proofs (both conflicting vertex hashes) persistently in redb via `EQUIVOCATION_PROOFS_TABLE`.
3. Pass equivocation evidence (existing_hash, duplicate_hash) through `ConsensusOutput::EquivocationDetected` to the execution pipeline for persistent storage.

### Formal Bounds
- **Detection window**: 100 rounds × 400ms = 40 seconds. A well-connected honest node will see equivocating vertices within this window.
- **Memory overhead**: ~100 entries × (8 round + 32 author + 32 hash) = ~7.2KB per validator. For 200 validators: ~1.44MB. Negligible.
- **Persistent proof**: Once detected, the proof survives node restart. Can be submitted to other validators for independent verification.
- **Gap**: Equivocations delayed >40 seconds remain undetectable by the in-memory tracker, but the persistent proof store means once-caught equivocations are never lost.
- **Finality certificate coverage**: Finality certs don't directly prove absence of equivocations. The slash mechanism is the enforcement layer.

### Rationale
- 100 rounds covers 5× the original window, handling realistic partition durations
- Persistent proofs close the "restart amnesia" gap where a node could forget a detected equivocation
- Memory cost is negligible even at maximum validator set size (400)

### Consequences
- Slightly more memory per node for equivocation tracking (~1.44MB vs ~0.29MB)
- New `EQUIVOCATION_PROOFS_TABLE` in redb (14 tables total)
- Future: proofs can be gossiped to enable cross-node equivocation verification

---

## ADR-016: Consensus-layer transaction size limits (MEV mitigation Phase 1)

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** security-engineer + node-engineer
**Git Ref:** pending (Sprint 048)

### Context
MEV (Maximal Extractable Value) attacks often involve injecting large transactions to manipulate ordering or consume block space. A full encrypted mempool (commit-reveal) is the gold standard but requires significant protocol changes. As Phase 1 mitigation, a simpler approach was evaluated.

### Decision
Add a consensus-layer `MAX_TX_SIZE` constant (256 KiB) enforced at the mempool boundary. Transactions exceeding this limit are rejected before entering the mempool. Phase 2 (encrypted mempool with commit-reveal) is roadmapped for post-mainnet.

### Rationale
- 256 KiB covers all legitimate tx types (transfers ~100B, contract deploys ~100KB, AI tasks ~10KB)
- Prevents bandwidth-based MEV attacks where oversized payloads starve honest transactions
- Zero protocol complexity — a single size check in `insert_with_priority()`
- Does not address ordering-based MEV (commit-reveal Phase 2 handles that)

### Consequences
- Oversized transactions are silently dropped at the mempool boundary
- Contract deployments exceeding 256 KiB need chunked deployment (standard practice)
- Phase 2 encrypted mempool remains the full MEV solution

---

## ADR-017: Quantum migration drill via StateCommitment trait

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer
**Git Ref:** pending (Sprint 048)

### Context
Post-quantum cryptography migration is a known future requirement. The state commitment scheme (currently Verkle with BLAKE3) must be swappable without chain halt. The `StateCommitment` trait (ADR-007) already provides the abstraction — but no tests existed proving the migration path actually works.

### Decision
Add migration drill tests that prove: (1) both `VerkleCommitment` and `MerkleCommitment` independently produce valid roots and proofs, (2) cross-scheme verification correctly fails (Verkle proof against Merkle root and vice versa), (3) the trait abstraction allows runtime scheme selection.

### Rationale
- The binary Merkle fallback path (`MerkleCommitment`) was already implemented but untested as a migration target
- Migration drill tests are cheap insurance against "it should work" assumptions
- No code changes needed — just tests proving the existing abstraction is sound
- Documents the migration path: change `StateCommitment` impl at epoch boundary

### Consequences
- Confidence that commitment scheme migration is a configuration change, not a protocol change
- Future quantum-safe hash functions (e.g., SPHINCS+) can be added as new `StateCommitment` impls
- No runtime overhead — migration tests run only in CI

---

## ADR-018: RANDAO-style VRF seed accumulation (anti-last-revealer)

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** consensus-engineer + security-engineer
**Git Ref:** pending (Sprint 048)

### Context
The VRF seed used for leader election was updated as `hash(anchor_hash)` — a single anchor vertex determined the next seed. A malicious last-revealer who controls the anchor can compute the resulting seed before publishing, allowing them to withhold if the outcome is unfavorable (last-revealer bias).

### Decision
Accumulate the VRF seed RANDAO-style: `new_seed = hash(prev_seed || anchor_hash || causal_vertex_hashes)`. The causal history of the anchor (all uncommitted vertices in its DAG cone) is mixed into the seed, making it dependent on contributions from multiple validators.

### Rationale
- Single-source seed (`hash(anchor)`) gives the anchor author full control of the next random value
- Mixing `prev_seed` creates chain dependency — seed manipulation requires controlling multiple consecutive rounds
- Mixing causal vertices adds randomness from every validator who published a vertex in the anchor's DAG cone
- Cost: one `causal_order()` call per commit (already computed for execution ordering)

### Consequences
- Seed is now dependent on O(n) validator contributions per commit, not O(1)
- A last-revealer must control the anchor AND predict all causal vertices to bias the seed — exponentially harder
- Slight increase in seed computation cost (causal_order traversal), negligible vs. execution cost
- Deterministic across all honest nodes (same DAG → same causal order → same seed)

---

## ADR-019: Browser wallet spending limits in WASM

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** security-engineer + p2p-network-engineer
**Git Ref:** pending (Sprint 048)

### Context
Browser-based wallets (via aztibase-wasm) hold private keys in JavaScript/WASM memory, which is more vulnerable than hardware wallets or native apps. A compromised browser extension or XSS attack could drain funds instantly without any safety net.

### Decision
Add client-side spending limits in the browser wallet module: (1) `BROWSER_PER_TX_LIMIT` = 100 AZTB per transaction, (2) `BROWSER_SPENDING_LIMIT` = 10,000 AZTB total session cap, (3) `HIGH_VALUE_WARNING_THRESHOLD` = 1,000 AZTB balance warning. These are advisory — the WASM module exports `checkBrowserBalance()` and `checkBrowserTx()` for frontends to call before signing.

### Rationale
- Client-side limits are defense-in-depth — they don't prevent a determined attacker but raise the bar for opportunistic exploits
- WASM-exported functions let any frontend framework integrate safety checks
- Limits are constants, not hardcoded protocol rules — frontends can choose to enforce or ignore
- No protocol changes needed — purely client-side advisory layer

### Consequences
- Frontends must call check functions before signing (not enforced at protocol level)
- Power users can bypass limits by using CLI or direct RPC (acceptable — they accept the risk)
- Future: hardware wallet integration would remove the need for client-side limits

---

## ADR-020: Quorum-signed DHT records (anti-poisoning)

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** p2p-network-engineer + security-engineer
**Git Ref:** pending (Sprint 048)

### Context
Kademlia DHT records are used for validator set announcements, relay provider lists, and chain tip propagation. A single malicious node can poison the DHT by publishing false records (e.g., advertising non-existent validators or wrong chain tips). Standard Kademlia has no authentication mechanism.

### Decision
Introduce `SignedDhtRecord` — a wrapper around DHT values that includes: (1) `kind` enum (ValidatorSet/RelayProvider/ChainTip), (2) `round` for freshness, (3) `signatures` vec of `DhtRecordSignature` (pubkey + sig bytes), (4) `data` payload. Validation requires minimum quorum signatures (`MIN_QUORUM_SIGNATURES=2`), freshness within `MAX_RECORD_AGE_ROUNDS=10,000`, and callback-based signature verification.

### Rationale
- Quorum requirement (≥2 signatures) prevents single-node poisoning — an attacker must compromise multiple validators
- Round-based freshness prevents replay of stale records (e.g., old validator sets)
- Callback-based sig verification decouples DHT validation from specific crypto implementation
- Kind enum enables per-type validation rules in the future (e.g., stricter quorum for ValidatorSet)

### Consequences
- DHT writes require coordinating signatures from ≥2 validators (adds latency to record publication)
- Records older than 10,000 rounds (~67 minutes at 400ms) are rejected — ensures liveness
- Unsigned legacy records are rejected — all DHT records must use the new format
- Future: per-kind quorum thresholds (e.g., ValidatorSet requires ≥ f+1 signatures)

---

## ADR-021: No mempool persistence — re-gossip from peers on restart

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** node-engineer + blockchain-architect
**Git Ref:** pending (Sprint 049)

### Context
The mempool is in-memory only. On node restart, all pending transactions are lost. The question is whether to persist the mempool to redb or accept the loss.

### Decision
Do not persist the mempool. Pending transactions are recovered via gossipsub re-propagation from peers.

### Rationale
- Gossipsub already re-propagates unconfirmed transactions to new peers on connect
- Persisting mempool adds write amplification to the hot path (every tx insert hits disk)
- Stale transactions in a persisted mempool may have expired nonces or changed base fees
- Validators that restart quickly will re-receive pending txs from the mesh within seconds
- Ethereum, Sui, and most L1s do not persist mempools — industry standard

### Consequences
- Solo-node restarts lose pending transactions (acceptable — they are re-gossiped)
- No additional redb write overhead on the transaction insertion path
- Mempool contents are ephemeral by design

---

## ADR-022: Protocol version negotiation via libp2p identify

**Date:** 2026-03-10
**Status:** ACCEPTED
**Decided By:** p2p-network-engineer + blockchain-architect
**Git Ref:** pending (Sprint 049)

### Context
Nodes have no mechanism to reject peers running incompatible protocol versions. A protocol-breaking upgrade will silently partition the network unless version negotiation is enforced.

### Decision
Add a `PROTOCOL_VERSION` constant (starting at 1). Include it in the libp2p identify agent string as `aztibase/1`. On peer identification, parse the remote version and disconnect peers with mismatched major versions. Add a version byte to the vertex wire format; reject vertices with unrecognized versions.

### Rationale
- libp2p identify is already part of the swarm — no new protocol needed
- Agent string parsing is standard practice (Lighthouse, Prysm use similar schemes)
- Vertex version byte costs 1 byte per vertex with zero parsing overhead
- Disconnecting incompatible peers prevents silent consensus splits during upgrades

### Consequences
- All nodes must be upgraded together for major version bumps (expected for pre-mainnet)
- Minor version differences are tolerated (only major version triggers disconnect)
- Future: version negotiation can gate feature flags (e.g., compact blocks, new TxKinds)

---

## ADR-023: L2 bridge design — challenge window, proof format, governance gate

**Date:** 2026-03-11
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer
**Git Ref:** Sprint 052

### Context
Sprint 052 adds L1 bridge primitives for sovereign rollups. Several design decisions had to be made about the bridge architecture: how L2 chains are registered, how state roots are anchored, how deposits/withdrawals work, and how to prevent double-spend.

### Decision
1. **Amount type: u128** — FP-004 spec used u64 for bridge amounts. Changed to u128 for consistency with Transfer, Stake, Delegate, and all other monetary types. Prevents silent truncation at high values.
2. **RegisterL2 governance-gated** — Only governance-approved addresses can register L2 chains. Provides on-chain L2 discovery via L2Registry.
3. **Challenge window: 100 batches** — L2 state roots become "finalized" after 100 L1 batches (~40s at 400ms block time). Withdrawals require a finalized state root.
4. **Proof format: opaque bytes** — `l2_burn_proof` is treated as opaque; L1 only checks proof hash uniqueness (double-spend prevention via BridgeWithdrawProofs). Actual proof verification is L2-specific and deferred to Phase 2.
5. **Sequencer authorization** — Only sequencers listed in the L2 registry's sequencer_set can submit AnchorL2State for that l2_chain_id.
6. **Escrow consistency** — Bridge escrow balances are decremented on successful withdraw (not just on deposit), ensuring escrow.balance() accurately reflects locked funds.

### Rationale
- u128 avoids a class of bugs where large bridge amounts silently truncate
- Governance gating prevents spam L2 registrations and provides a trust anchor
- 100-batch finality window balances security (time for challenges) against UX (not too long)
- Opaque proofs allow supporting any L2 proof system without L1 changes
- Escrow unlock on withdraw was caught in security review — without it, escrow balances would only increase, creating a misleading view of locked funds

### Consequences
- L2 registration requires governance approval (slower onboarding, but more secure)
- Withdrawals are delayed by ~40s finality window (acceptable for cross-layer transfers)
- Future L2-specific proof verification can be added without changing L1 wire format
- 4 new TxKinds (0x16-0x19) consume prefix space (245 remaining)

---

## ADR-024: Protocol store persistence via STATE_TABLE with postcard serialization

**Date:** 2026-03-11
**Status:** ACCEPTED
**Decided By:** node-engineer + blockchain-architect
**Git Ref:** Sprint 053

### Context
8 in-memory protocol stores (StakingStore, GovernanceStore, EmissionTracker, ChainParams, AgentPolicyStore, L2Registry, L2AnchorStore, BridgeEscrow, BridgeWithdrawProofs) were lost on node restart. This is a mainnet showstopper — validators would lose stake records, governance proposals, emission tracking, and bridge escrow balances.

### Decision
1. **Reuse STATE_TABLE with prefixed keys** — Rather than creating 9 new redb tables, serialize each store as a single blob under a domain-prefixed key (e.g., `b"staking_store"`) in the existing STATE_TABLE.
2. **postcard serialization** — Use postcard (already a workspace dependency) for binary serialization. Compact, fast, no-std compatible.
3. **Flush-after-batch pattern** — All stores are flushed atomically after each batch commit, alongside flush_state and store_base_fee.
4. **Load-on-startup with defaults** — On startup, each store is loaded from disk with graceful fallback to defaults if the key is missing or deserialization fails.
5. **Serde derives on all store types** — Added `Serialize, Deserialize` to all store structs and their contained types.

### Rationale
- Single-table approach avoids redb table proliferation (14 tables already exist)
- postcard is 10-100x faster than JSON for serialization and produces smaller blobs
- Flush-after-batch ensures consistency: protocol state matches account state at every batch boundary
- Default fallback ensures forward compatibility when new stores are added

### Consequences
- Node restart now recovers full protocol state without replaying from genesis
- STATE_TABLE size grows by ~100KB per store (negligible vs account state)
- Adding a new store requires: serde derives, flush/load functions, pipeline wiring

---

## ADR-025: Network profiles & mainnet operational hardening

**Date:** 2026-03-11
**Status:** ACCEPTED
**Decided By:** blockchain-architect + security-engineer
**Git Ref:** Sprint 054

### Context
The node binary treated testnet and mainnet identically. CORS was always permissive, faucet was available on all chains, there was no per-IP rate limiting, shutdown didn't guarantee a clean state flush, and validators couldn't rotate keys without deregistering.

### Decision
1. **NetworkProfile enum** (`Dev`, `Testnet`, `Mainnet`) drives configuration behavior: CORS policy, faucet availability, and future genesis defaults.
2. **Per-IP token bucket rate limiter** for RPC endpoints (default 100 req/s, configurable) prevents abuse.
3. **Profile-driven CORS**: `Mainnet` → explicit origin whitelist; `Testnet`/`Dev` → permissive (`Allow-Any`).
4. **Faucet gating by profile**: `aztb_faucetDrip` disabled when `profile = Mainnet`.
5. **DB sentinel pattern**: write `b"running"` to STATE_TABLE on startup, clear on clean shutdown. Dirty-start detection warns the operator.
6. **Graceful shutdown**: SIGINT handler flushes all protocol stores, clears sentinel, then exits.
7. **Validator key rotation** via `TxKind::RotateValidatorKey` (0x1A): old key signs a tx authorizing migration to a new pubkey. StakingStore atomically re-keys validator entry + delegations + unbonding queue.
8. **Configurable request body size limit** (default 1 MiB) to prevent oversized payloads.

### Rationale
- Profile enum is cheaper than a full "chain spec" abstraction and covers all current divergence points
- Token bucket is simple, stateless (per-process), and sufficient for single-node RPC protection
- Sentinel pattern is a proven database health check (SQLite WAL, PostgreSQL pg_control)
- Key rotation as a first-class TxKind avoids the unstake→restake dance that would cause temporary liveness loss

### Consequences
- Operators must use `--mainnet` flag for mainnet deployments (default is `dev`)
- Mainnet nodes reject faucet requests at the RPC layer
- Dirty-start warning gives operators visibility into unclean shutdowns
- Key rotation is a single atomic operation from the validator's perspective

---

## ADR-026: Public testnet launch infrastructure

**Date:** 2026-03-11
**Status:** ACCEPTED
**Decided By:** node-engineer + documentation-engineer
**Git Ref:** Sprint 055

### Context
Sprint 047 proved the node works on a 3-node local testnet. Sprints 050-054 added CI, benchmarks, persistence, and hardening. But there was no public-facing infrastructure for external validators or developers to join a testnet — no canonical genesis distribution, no faucet UI, no join instructions, and no deployment tooling.

### Decision
1. **Canonical testnet genesis**: Pre-generated `testnet/genesis/` directory with deterministic genesis.toml, validator keys, and per-node TOML configs. Uses the existing `testnet_genesis()` for `--testnet` flag, plus generated configs for cloud seed nodes.
2. **Seed node configs**: 3 TOML files in `deploy/seed-nodes/` targeting `testnet{1,2,3}.aztibase.com` with profile=testnet, metrics enabled, public bind addresses.
3. **Faucet web UI**: Static HTML at `faucet/index.html` — vanilla JS, same dark theme as explorer, calls `aztb_faucetDrip` via JSON-RPC, client-side 60s cooldown. Configurable RPC endpoint via `?rpc=` URL param.
4. **Testnet landing page**: Static HTML at `testnet/index.html` — network details, developer guide (4 steps), validator guide (6 steps), seed node table, RPC method summary.
5. **Deployment helpers**: systemd unit file (`deploy/systemd/aztibase.service`), cloud bootstrap script (`deploy/bootstrap.sh`) for Ubuntu 22.04+ (install deps, build, configure, start).
6. **All static pages are Vercel-deployable** (same pattern as explorer) — no server-side rendering needed.

### Rationale
- Static HTML pages (no framework) match the explorer pattern and deploy trivially to Vercel/Netlify/S3
- Separate `faucet/` and `testnet/` directories allow independent deployment to faucet.aztibase.com and testnet.aztibase.com
- Systemd unit is the standard Linux service management — avoids Docker dependency for operators
- Bootstrap script is idempotent and builds from source (no pre-built binary distribution yet)

### Consequences
- External validators can join with a single bootstrap command
- Developers can get testnet tokens without CLI tooling
- Seed node configs are ready for cloud deployment once DNS records are configured
- The faucet UI depends on a running RPC node with CORS enabled (testnet profile provides this)

---

## ADR-027: Full state snapshots & mainnet genesis ceremony

**Date:** 2026-03-11
**Status:** ACCEPTED
**Decided By:** node-engineer + blockchain-architect
**Git Ref:** Sprint 056

### Context
The snapshot module (Sprint 046+) captured AccountState but not protocol stores (staking, governance, emission, chain params, agent policies, L2 bridge). New validators had to replay from genesis, which becomes impractical at scale. Mainnet also required a genesis ceremony with auditable 400M AZTB tokenomics allocations.

### Decision
1. **ProtocolStoreBundle**: New struct bundling all 9 protocol stores + base_fee, serialized alongside AccountState in snapshots. SNAPSHOT_VERSION bumped 2→3.
2. **Snapshot file format**: `[32-byte BLAKE3 hash][postcard-serialized StateSnapshot]`. MAX_SNAPSHOT_SIZE raised to 128 MiB. Integrity verified on read — hash mismatch rejects the file.
3. **CLI integration**: `aztibase snapshot export --output <path>` writes snapshot from running DB. `aztibase --snapshot <path>` bootstraps a fresh node from file, skipping genesis replay.
4. **Mainnet genesis**: `mainnet_genesis(n_validators)` allocates 400M AZTB across 8 categories (team 15%, investors 10%, ecosystem 25%, community 20%, treasury 15%, validators 5%, advisors 5%, reserve 5%). Placeholder keys derived from BLAKE3 seeds — replaced with real keys at ceremony.
5. **Backward compatibility**: `protocol_stores` field is `Option<ProtocolStoreBundle>` — old snapshots deserialize with `None`, defaulting to empty stores.

### Rationale
- BLAKE3 integrity hash catches file corruption and tampering before deserialization
- postcard serialization is compact and matches existing persistence layer (ADR-024)
- Version bump with Option field gives clean forward migration without breaking old snapshots
- Deterministic placeholder keys allow testing the full allocation math before ceremony

### Consequences
- Validators can bootstrap from a snapshot file in seconds instead of replaying history
- Mainnet genesis is auditable: 8 categories, percentages verifiable in source code
- Snapshot version 2 files still readable (no protocol stores = default empty)
- Ceremony requires replacing placeholder addresses with real multisig keys

---

## Template for New ADRs

```
## ADR-NNN: [Title]

**Date:** [YYYY-MM-DD]
**Status:** PROPOSED | ACCEPTED | SUPERSEDED by ADR-NNN
**Decided By:** [skill-name]
**Git Ref:** [commit hash, if applicable]

### Context
[Why this decision was needed]

### Decision
[What was decided]

### Rationale
[Why this option was chosen over alternatives]

### Consequences
[What follows from this decision -- both positive and negative]
```

---

## ADR-028: nChain Patent FTO Preliminary Analysis

**Date:** 2026-03-12
**Status:** ACCEPTED
**Decided By:** legal-ip-counsel + blockchain-architect
**Git Ref:** (this commit)

### Context
nChain holds ~3,900 patent applications (~1,090 granted), making them the largest blockchain patent holder. Their Patent Pledge only covers BSV implementations. As an independent L1, Aztibase has no protection under this pledge. An FTO analysis was required before mainnet launch.

### Decision
Overall risk assessed as LOW-MEDIUM. Aztibase's architecture is fundamentally different from nChain's BSV-centric portfolio.

Risk by feature:
- **PoUW / AI verification (MEDIUM-HIGH)**: nChain demonstrated verifiable AI inference on BSV (Sept 2024) using ZK proofs. Aztibase uses committee attestation, not ZK proofs — architecturally distinct but closest overlap area.
- **Smart contracts (MEDIUM)**: nChain holds patents on state machines on blockchain (EP 3257191, US 11,194,898). Aztibase uses standard WASM/EVM open standards, not BSV Script.
- **Consensus (MEDIUM → possibly LOW)**: US 12,032,677 covers consensus-based ledgers but is under reexamination by Unified Patents. nChain has zero DAG consensus patents. SynBFT derives from MystiCeti (academic prior art).
- **All other features (LOW)**: DAG structure, Verkle trees, libp2p, BLAKE3/Ed25519/BLS, hybrid account+object model — no nChain overlap.

### Rationale
Aztibase's design choices (DAG-BFT, Verkle trees, dual WASM+EVM, PoUW with attestation) are architecturally distinct from nChain's BSV-focused innovations. The only meaningful overlap is in verifiable AI computation, which uses a different verification mechanism.

### Consequences
- Must ensure PoUW verification remains attestation-based (not ZK-proof-based)
- Consider joining COPA (Cryptocurrency Open Patent Alliance) for defensive coverage
- File defensive publications for novel techniques (SynBFT, PoUW, hybrid model)
- Monitor nChain patent grants quarterly, especially AI/computation filings
- Track US 12,032,677 reexamination outcome

---

## ADR-029: quinn-proto Security Patch (RUSTSEC-2026-0037)

**Date:** 2026-03-12
**Status:** ACCEPTED
**Decided By:** security-engineer
**Git Ref:** (this commit)

### Context
`cargo audit` identified RUSTSEC-2026-0037: DoS vulnerability in quinn-proto 0.11.13 (severity 8.7 HIGH). quinn-proto is a transitive dependency of libp2p-quic and reqwest, used for all P2P transport.

### Decision
Bumped quinn-proto 0.11.13 → 0.11.14 via `cargo update quinn-proto`. Remaining advisories (ring 0.16, tracing-subscriber 0.2, lru 0.12, atomic-polyfill, bincode, derivative) are transitive dependencies locked by libp2p 0.54 and revm 36 — cannot be patched without major version bumps.

### Rationale
Patch-level bump with no breaking changes. Eliminates a high-severity DoS attack surface on all QUIC-based P2P connections.

### Consequences
- QUIC DoS vulnerability eliminated
- 2 remaining vulnerabilities (ring AES panic, tracing-subscriber ANSI injection) are transitive and low-impact for Aztibase's use case
- Will be fully resolved when libp2p releases a version using ring 0.17+
