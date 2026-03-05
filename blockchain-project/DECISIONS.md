# Architecture Decision Records (ADRs) -- Dendrite Network

Every non-obvious technical decision is recorded here. Each ADR is immutable once written -- if a decision is reversed, a new ADR supersedes it with a reference to the original.

---

## ADR Index

| ADR | Title | Date | Status | Decided By |
|-----|-------|------|--------|------------|
| ADR-001 | Switch storage from RocksDB to redb | 2026-03-05 | ACCEPTED | blockchain-architect |
| ADR-002 | Dual-license MIT / Apache-2.0 | 2026-03-05 | ACCEPTED | legal-ip-counsel |
| ADR-003 | Workspace monorepo with 8 crates | 2026-03-05 | ACCEPTED | blockchain-architect |

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
Open-source license selection for the Dendrite Network codebase.

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
- dendrite-core (primitives, crypto, types)
- dendrite-consensus (SynBFT, PoUW, DAG)
- dendrite-network (P2P, transport, gossip)
- dendrite-storage (redb, verkle trees)
- dendrite-execution (VM, state, parallel execution)
- dendrite-runtime (contracts, AI oracle, agents)
- dendrite-rpc (JSON-RPC server)
- dendrite-node (binary entry point)

### Rationale
- Clean separation of concerns matching engineer domains
- Independent compilation and testing per crate
- Clear dependency graph (core -> consensus -> network -> etc.)
- Each engineer owns their crate(s)

### Consequences
- Cross-crate API design is critical -- breaking changes propagate
- Need clear ownership and review process for shared types in dendrite-core

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
