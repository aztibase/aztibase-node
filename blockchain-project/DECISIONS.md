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
