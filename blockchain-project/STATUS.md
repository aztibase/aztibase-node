# PROJECT STATUS: Dendrite Network

**Last Updated:** 2026-03-06
**Updated By:** project-lead
**Current Phase:** M3 -- Execution Layer (COMPLETE)
**Current Sprint:** Sprint 006 -- COMPLETE (JSON-RPC + Security Hardening + M3 Close)

---

## Milestone Tracker

| Milestone | Description | Status | Date Started | Date Completed |
|-----------|-------------|--------|--------------|----------------|
| M0 | Design complete (all MASTER_DESIGN sections) | DONE | 2026-03-05 | 2026-03-05 |
| M1 | Core primitives (data types, crypto, basic structs) | DONE | 2026-03-05 | 2026-03-05 |
| M2 | P2P networking + basic consensus | DONE | 2026-03-05 | 2026-03-06 |
| M3 | Execution layer (WASM VM, state management) | DONE | 2026-03-06 | 2026-03-06 |
| M4 | Integration testing + basic AI + testnet | NOT STARTED | -- | -- |
| M5 | Light/browser nodes + wallet | NOT STARTED | -- | -- |
| M6 | AI compute market + PoUW | NOT STARTED | -- | -- |
| M7 | Security audit + hardening | NOT STARTED | -- | -- |
| M8 | Public testnet | NOT STARTED | -- | -- |
| M9 | Mainnet launch | NOT STARTED | -- | -- |

---

## Current State

### Completed
- Full design phase (Phases 3-6) delivered
- All 12 skills defined and operational
- Workspace scaffolded: 8 Rust crates in monorepo
- Build-phase tracking infrastructure (5 documents)
- Implementation depth audit (Sprint 001, Task 7)
- Reference repos cloned (MystiCeti, Sui, Lighthouse, rust-libp2p, redb)
- cargo build + cargo test pass (191 tests, 0 failures)
- Sprint 001 closed with retrospective
- Sprint 002 closed: 24/24 tasks, 5 phases complete
- Sprint 003 closed: 18/18 tasks, 4 phases complete
- Sprint 004 closed: 19/19 tasks, 4 phases complete
- Sprint 005 closed: 21/21 tasks, 4 phases complete
- Sprint 006 closed: 23/24 tasks (1 deferred), 4 phases complete

### Crate Status

| Crate | Depth | Owner | M1-Ready | M3-Ready |
|-------|-------|-------|----------|----------|
| dendrite-core | COMPLETE | blockchain-architect | YES | YES |
| dendrite-consensus | PARTIAL | consensus-engineer | YES | YES |
| dendrite-storage | PARTIAL | node-engineer | YES | YES |
| dendrite-network | PARTIAL | p2p-network-engineer | YES | YES |
| dendrite-execution | PARTIAL | smart-contract-engineer | YES | YES |
| dendrite-runtime | PARTIAL | ai-integration-engineer | YES | NO |
| dendrite-rpc | PARTIAL | node-engineer | NO | YES |
| dendrite-node | PARTIAL | node-engineer | YES | YES |

### In Progress
- Nothing currently in progress (M3 complete, Sprint 007 planning next)

### Blocked
- Nothing currently blocked

### Recently Completed
- Sprint 006 Phase 4: M3 close — security review (1 MEDIUM fixed: RPC body limit), cargo-audit clean, ADR-005
- Sprint 006 Phase 3: Integration testing (4 e2e tests, 191 total tests)
- Sprint 006 Phase 2: Security hardening (7 MEDIUM findings, 7 new tests, 187 total)
- Sprint 006 Phase 1: JSON-RPC server (6 methods, 15 tests, node wiring, shared state)
- Sprint 005 Phase 4: Security review (zero ELEVATED flags), cargo-audit, docs update
- Sprint 005 Phase 3: BLS finality certificates (blst, aggregation, FinalityCertificate, creation, verification)
- Sprint 005 Phase 2: State persistence to redb, batch root storage, startup recovery
- Sprint 005 Phase 1: Consensus-to-execution wiring, TxKind routing, ExecutionPipeline

### Next Up
1. Sprint 007 planning (M4: EVM via revm, AI integration, state sync, testnet prep)

---

## Open Risks

| Risk | Severity | Owner | Status |
|------|----------|-------|--------|
| Dendrite Systems coexistence agreement not initiated | MEDIUM | legal-ip-counsel | OPEN |
| Domain acquisition pending | MEDIUM | legal-ip-counsel | OPEN |
| nChain patent FTO analysis not started | HIGH | legal-ip-counsel | OPEN |
| wasmtime trap handling on Windows | LOW | smart-contract-engineer | KNOWN |
| Transitive dep advisories (ring, wasmtime WASI, lru, bincode) | LOW | security-engineer | DOCUMENTED |
| BLS rogue-key attack without PoP | MEDIUM | consensus-engineer | DOCUMENTED (ADR-004) |

---

## Legal Conditions Tracker

| # | Condition | Status |
|---|-----------|--------|
| 1 | Use DNDR ticker (not DND) | SATISFIED |
| 2 | Coexistence agreement with Dendrite Systems Inc. | PENDING |
| 3 | Trademark filing strategy (Classes 36/42 first, then 9) | PENDING |
| 4 | Domain acquisition before public announcement | PENDING |
| 5 | Matrix Dendrite FAQ entry | PENDING |
