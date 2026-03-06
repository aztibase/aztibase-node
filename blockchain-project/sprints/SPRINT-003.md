# Sprint 003: M2 Consensus Round Processing

**Sprint Goal:** Implement basic SynBFT round processing. By sprint end, a node proposes DAG vertices every 400ms, processes received vertices from peers, inserts them into the DAG, evaluates commit rules, and accepts transactions into a mempool.

**Start Date:** 2026-03-05
**End Date:** TBD
**Status:** IN PROGRESS
**Led By:** project-lead

---

## Prerequisites (All Complete)
- [x] Sprint 002 complete (M1: core primitives, 72 tests)
- [x] DagBlock, DagStore, CommitRule implemented
- [x] Libp2pTransport with Gossipsub (6 topics), Kademlia, mDNS
- [x] Node binary with storage + network wiring + graceful shutdown
- [x] Security review passed, cargo-audit clean

---

## Sprint Scope

M2 from the master plan: "Nodes discover each other, gossip messages, and run a simplified consensus round." This sprint focuses on making the consensus engine actually run — proposing vertices, receiving them, growing the DAG, and evaluating commits.

**Not in scope for Sprint 003:**
- VRF leader election (deferred to Sprint 004 — deterministic selection sufficient for now)
- RPC server (deferred — local testing via logs)
- Slashing / equivocation penalties (M4)
- Full finality pipeline (M3)

---

## Phase 1: Consensus Round Engine (consensus-engineer + node-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 1 | Implement `ConsensusConfig` (round duration, wave length, validator identity) | consensus-engineer | -- | DONE | Config struct with serde, defaults to 400ms rounds |
| 2 | Implement `RoundState` (current round, received vertices per round, parent tracking) | consensus-engineer | -- | DONE | Tracks round N, vertices seen, selects 2f+1 parents |
| 3 | Implement vertex proposal: generate DagBlock at each round boundary | consensus-engineer | Tasks 1-2 | DONE | Node produces 1 vertex/round referencing parents from round N-1 |
| 4 | Implement vertex serialization (bincode encode/decode for gossip) | consensus-engineer | Task 3 | DONE | DagBlock <-> bytes roundtrip + test |
| 5 | Implement `ConsensusEngine` with async round loop (400ms interval) | consensus-engineer | Tasks 1-4 | DONE | Engine drives rounds, proposes vertices, processes inbox |
| 6 | Wire ConsensusEngine into node main loop | node-engineer | Task 5 | DONE | Node starts consensus on boot, advances rounds |

**Exit criteria:** Node logs round transitions every 400ms. Vertex proposals generated with parent references. `cargo test -p dendrite-consensus` passes with new tests.

---

## Phase 2: Vertex Reception & DAG Growth (consensus-engineer + p2p-network-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 7 | Wire gossipsub vertex topic to consensus engine inbox | p2p-network-engineer | Phase 1 | DONE | Received gossip bytes routed to consensus |
| 8 | Validate received vertices (round bounds, parent existence, author in validator set) | consensus-engineer | Task 7 | DONE | Invalid vertices rejected with reason logged |
| 9 | Insert valid vertices into DagStore | consensus-engineer | Task 8 | DONE | DAG grows each round, queryable |
| 10 | Broadcast proposed vertices via gossipsub | p2p-network-engineer | Phase 1 Task 4 | DONE | Own vertices published to `dendrite/consensus` topic |
| 11 | Parent selection: pick 2f+1 parents from previous round's received vertices | consensus-engineer | Tasks 2, 9 | DONE | Vertex references correct parents, validated on receipt |

**Exit criteria:** Two-node local test shows vertices flowing between nodes. DagStore on both nodes contains vertices from both validators.

---

## Phase 3: Commit Rule Integration + Mempool (consensus-engineer + node-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 12 | Call CommitRule at anchor rounds (every `wave_length` rounds) | consensus-engineer | Phase 2 | PENDING | Direct/indirect commit evaluated, committed blocks logged |
| 13 | Track committed blocks (mark as final, maintain committed chain) | consensus-engineer | Task 12 | PENDING | Committed blocks never re-evaluated, queryable |
| 14 | Implement `Mempool` struct (bounded priority queue, dedup by tx hash) | node-engineer | -- | PENDING | Insert, remove, peek_batch, contains, len + 4 tests |
| 15 | Wire gossipsub transaction topic to mempool | p2p-network-engineer | Task 14 | PENDING | Received txs enter mempool, broadcast own txs |
| 16 | Include mempool transactions in vertex payload | consensus-engineer | Tasks 14, 3 | PENDING | Proposed vertices carry pending txs (up to size limit) |

**Exit criteria:** Anchor rounds trigger commit evaluation. Mempool accepts transactions and includes them in vertex payloads. 10+ new tests.

---

## Phase 4: Cross-Cutting (security-engineer)

| # | Task | Assigned To | Depends On | Status | Acceptance Criteria |
|---|------|-------------|------------|--------|---------------------|
| 17 | Security review of consensus round engine | security-engineer | Phases 1-3 | PENDING | Equivocation detection, parent validation, round bounds |
| 18 | Run cargo-audit, fix any new findings | security-engineer | All code | PENDING | Zero critical/high CVEs |

**Exit criteria:** Security review complete. No ELEVATED flags.

---

## Sprint Totals

| Metric | Target |
|--------|--------|
| Tasks | 18 |
| Tests (new) | 30+ |
| Total tests | 100+ |
| Crates modified | 3 (consensus, network, node) |
| Security reviews | 1 |

---

## Key Design Decisions

1. **Uncertified DAG pattern (Mysticeti):** Vertices broadcast immediately without 2f+1 ACKs. Equivocation detected retroactively.
2. **Parent requirement:** Each non-genesis vertex references at least 2f+1 parents from round N-1.
3. **Serialization:** bincode for vertex gossip (already a dependency, fast, compact).
4. **No VRF yet:** Deterministic leader selection from Sprint 002 is sufficient for M2. VRF adds complexity best handled in Sprint 004.
5. **Mempool is in-memory:** No persistence needed for M2. Transactions lost on restart is acceptable.

---

## Risk Register

| Risk | Mitigation |
|------|------------|
| 400ms round timer precision on Windows | Use tokio::time::interval with MissedTickBehavior::Skip |
| Multi-node testing complexity | Test with 2-3 nodes on localhost via different ports |
| Parent availability at round boundary | Allow fewer parents in early rounds; require 2f+1 only after round 3 |
| Gossipsub message ordering | DagStore handles out-of-order insertion; parents validated async |

---

## Definition of Done (Sprint 003)

- [ ] All 18 tasks completed or explicitly deferred with justification
- [ ] `cargo check --workspace` passes with zero warnings
- [ ] `cargo test --workspace` passes with 100+ tests total
- [ ] `cargo clippy --workspace` passes with zero warnings
- [ ] `cargo fmt --check` passes
- [ ] All code has BUILD_LOG entries
- [ ] Security review complete (no open ELEVATED flags)
- [ ] STATUS.md updated with post-sprint state
- [ ] Sprint retrospective written
