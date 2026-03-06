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
