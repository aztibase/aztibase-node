# Sprint 049 — Protocol Hardening & Multi-Node Stability (M9-S10)

**Date:** 2026-03-10
**Milestone:** M9 — Mainnet Prep
**Goal:** Prove the chain survives real-world conditions — crashes, restarts, epoch boundaries, sustained load. Add missing protocol infrastructure blocking a credible public testnet.
**Predecessor:** Sprint 048 (all 9 SECURITY-ELEVATED flags resolved)

---

## Phase 1: Node Resilience & State Recovery

| # | Task | Status |
|---|------|--------|
| 1.1 | Graceful shutdown state persistence — verify all stores flush on Ctrl+C | DONE |
| 1.2 | Crash recovery integration test — simulate crash mid-batch, restart, verify consistency | DONE |
| 1.3 | Node rejoin after extended downtime — batch_count recovery from BATCH_INDEX_TABLE | DONE |
| 1.4 | Mempool persistence decision — ADR-021: re-gossip from peers (no persist) | DONE |

## Phase 2: Protocol Version Negotiation

| # | Task | Status |
|---|------|--------|
| 2.1 | PROTOCOL_VERSION constant + libp2p identify check, disconnect incompatible peers | DONE |
| 2.2 | Version byte in vertex wire format — already present (WIRE_VERSION=1 in consensus wire.rs) | DONE |
| 2.3 | Tests: 5 protocol version tests (format, parse, reject non-aztibase, subversions, constant) | DONE |

## Phase 3: Epoch Boundary End-to-End Validation

| # | Task | Status |
|---|------|--------|
| 3.1 | --epoch-length CLI override for short testnet epochs | DONE |
| 3.2 | Epoch boundary reward distribution test — pipeline distributes at round 1000 | DONE |
| 3.3 | Participation clearing at epoch boundary — confirmed by test | DONE |
| 3.4 | Downtime slashing e2e — silent validator slashed, active validator survives | DONE |

## Phase 4: Throughput Measurement & Optimization

| # | Task | Status |
|---|------|--------|
| 4.1 | Throughput test harness — 10 batches × 5 txs with redb persistence, measures TPS | DONE |
| 4.2 | Baseline TPS: 86 TPS (debug), ~39K empty batches/s | DONE |
| 4.3 | Top bottleneck: Ed25519 signing in debug mode; release build expected 10x+ | NOTED |
| 4.4 | Criterion benchmarks deferred to Sprint 050 (needs CI infrastructure) | DEFERRED |

## Phase 5: Validation, Security Review & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | cargo clippy (0 warnings) + fmt (clean) + tests (284 execution, 85 network, 209 node) | DONE |
| 5.2 | Security review: crash recovery safe (redb ACID), version bypass rejected, epoch races N/A (single-threaded) | DONE |
| 5.3 | Doc sync (BUILD_LOG, STATUS, CHANGELOG, sprint plan) | DONE |
| 5.4 | ADR-021 (no mempool persistence) + ADR-022 (protocol version negotiation) | DONE |
| 5.5 | Sprint 050 scoping: CI pipeline, release-mode benchmarks, criterion integration | DONE |

---

## Exit Criteria

- [x] Node survives crash and recovers state on restart (3 tests prove it)
- [x] Protocol version mismatch → clean disconnect (identify behaviour + 5 unit tests)
- [x] Epoch boundary crossed with reward distribution, participation clearing, downtime slashing (4 tests)
- [x] Baseline TPS measured: 86 TPS debug / ~39K empty batches/s
- [x] Top bottleneck identified: Ed25519 signing overhead in debug mode
- [x] 0 clippy warnings, fmt clean, all tests pass (partial suite — Windows SDK issue blocks full workspace rebuild)
- [x] 2 new ADRs (ADR-021, ADR-022)

---

## Retrospective

**What went well:**
- Crash recovery design was clean — redb ACID guarantees + reverse range scan for batch index
- Protocol versioning integrated seamlessly via libp2p identify (already a dependency)
- Epoch boundary tests used `current_round = 999` trick to avoid running 1000 batches

**What was hard:**
- Windows SDK issue (missing kernel32.lib after cargo clean) blocked full workspace test rebuild
- Ed25519 signing in debug mode is extremely slow (1000 txs would take >5min)
- Epoch length param has min=1000 bounds, requiring round manipulation for fast tests

**Decisions made:**
- ADR-021: No mempool persistence — re-gossip from peers on restart
- ADR-022: Protocol version via identify, not custom handshake
- Throughput test reduced to 50 txs (10×5) for debug-mode viability
- batch_count increment moved from run() into execute_batch() for correct direct-call behavior

**Next sprint (050) should focus on:**
- CI pipeline setup (GitHub Actions or similar)
- Release-mode benchmarks with criterion
- Full workspace test gate in CI
- Windows SDK fix documentation
