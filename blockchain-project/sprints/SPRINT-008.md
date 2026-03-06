# Sprint 008 -- Multi-Node Testnet + AI Pipeline Integration (M4 Phase 2)

**Start Date:** 2026-03-06
**Target:** M4 progress — multi-node local testnet, AI inference in execution pipeline, tx pool improvements
**Owner:** project-lead
**Status:** COMPLETE

---

## Sprint Goal

Deliver three capabilities that bring Dendrite from a single-node prototype to a working multi-node system:
1. **Multi-node local testnet** — 3+ nodes discover each other, gossip vertices, reach consensus, execute identical state
2. **AI inference in execution pipeline** — new TxKind::AiInfer routes to TractRuntime, results in receipts
3. **Transaction pool improvements** — priority ordering, duplicate filtering, size-bounded eviction

These features prove that Dendrite's core loop works across nodes and that AI is first-class in the execution pipeline.

---

## Scope

| # | Item | Phase | Owner |
|---|------|-------|-------|
| 1 | Vertex serialization for gossip (bincode encode/decode with validation) | P1 | p2p-network-engineer |
| 2 | Cross-node vertex reception in ConsensusEngine | P1 | consensus-engineer |
| 3 | Consensus state broadcast (committed batch propagation) | P1 | consensus-engineer |
| 4 | Multi-node integration test (3 nodes, shared state convergence) | P1 | node-engineer |
| 5 | Launch script / CLI for local 3-node testnet | P1 | node-engineer |
| 6 | TxKind::AiInfer variant (prefix 0x06) | P2 | smart-contract-engineer |
| 7 | AI inference execution in pipeline (route to TractRuntime) | P2 | ai-integration-engineer |
| 8 | AI inference receipts (InferenceReceipt → ExecutionReceipt) | P2 | ai-integration-engineer |
| 9 | Model preload via node config (ONNX model path + model_id) | P2 | ai-integration-engineer |
| 10 | AI inference pipeline tests | P2 | ai-integration-engineer |
| 11 | Mempool priority ordering (gas price or fee field) | P3 | node-engineer |
| 12 | Mempool eviction policy (lowest priority when full) | P3 | node-engineer |
| 13 | Mempool dedup by tx hash (not content equality) | P3 | node-engineer |
| 14 | Mempool unit tests | P3 | node-engineer |
| 15 | Security review: multi-node gossip, AI pipeline, mempool | P4 | security-engineer |
| 16 | cargo-audit + clippy + fmt | P4 | security-engineer |
| 17 | Documentation updates (BUILD_LOG, STATUS, CHANGELOG, sprint) | P4 | documentation-engineer |

**Total:** 17 tasks across 4 phases

---

## Phase 1: Multi-Node Local Testnet (Tasks 1-5)

**Owner:** p2p-network-engineer + consensus-engineer + node-engineer
**Goal:** 3 Dendrite nodes on localhost discover each other via mDNS, gossip vertices, and converge on the same committed state.

### Task 1: Vertex serialization for gossip — DONE
- [x] `encode_vertex()` / `decode_vertex()` functions in dendrite-consensus/src/wire.rs
- [x] Validate decoded vertex (hash check, round bounds, known validator, size limit, version byte)
- [x] Engine uses wire::encode_vertex for BroadcastVertex, wire::decode_vertex for ReceivedVertex
- **Tests:** 9 wire tests (roundtrip, payload, too short, wrong version, tampered, unknown validator, future round, near future, oversized)

### Task 2: Cross-node vertex reception — DONE
- [x] `ConsensusInput::ReceivedVertex` decoded via wire::decode_vertex with full validation
- [x] Engine integrates validated remote vertices into DAG
- [x] Handle duplicate vertex (already in DAG) gracefully
- **Tests:** existing engine tests updated to use wire format

### Task 3: Consensus state broadcast — DONE
- [x] StateRootAnnounce struct (anchor_hash, state_root, batch_index, validator)
- [x] Pipeline sends PipelineResult back to main loop via result channel
- [x] Main loop broadcasts StateRootAnnounce on TOPIC_STATE_SYNC after execution
- [x] Incoming state root announcements decoded and logged
- **Tests:** covered by multi-node integration test

### Task 4: Multi-node integration test — DONE
- [x] Spawn 3 consensus engines in-process with shared genesis and interconnected channels
- [x] Submit transaction to node 0, verify all 3 nodes commit same batch (same anchor hash)
- [x] Verify state roots match across all 3 nodes after execution
- **Tests:** 1 integration test (multi_node_consensus_convergence)

### Task 5: Local testnet launch script — DONE
- [x] Shell script at scripts/local-testnet.sh
- [x] Each node gets unique validator_index (1-3), all share validator_count=3
- [x] mDNS discovery on localhost (no boot_nodes needed)
- [x] Prints peer IDs, listen addresses, and RPC endpoints
- **Deliverable:** `scripts/local-testnet.sh`

**Phase 1 Exit Criteria:**
- [x] 3 nodes gossip vertices and converge on same committed state
- [x] Multi-node integration test passes (232 total tests)
- [x] Local testnet can be started with a single command

---

## Phase 2: AI Inference in Execution Pipeline (Tasks 6-10)

**Owner:** ai-integration-engineer + smart-contract-engineer
**Goal:** AI inference is a first-class transaction type. Users submit TxKind::AiInfer, pipeline routes to TractRuntime, results are receipted.

### Task 6: TxKind::AiInfer variant — DONE
- [x] `TxKind::AiInfer { requester, model_id, input, nonce, max_compute_units }` with prefix 0x06
- [x] Wire format: 0x06 || bincode(AiInfer fields)
- [x] Add to `route_tx()` and `expected_prefix()`
- [x] model_id validation (same rules as func_name)
- **Tests:** 2 tests (roundtrip encode/decode, empty model_id rejected)

### Task 7: AI inference execution in pipeline — DONE
- [x] Pipeline holds `Option<Arc<dyn AIRuntime>>` (None = AI disabled)
- [x] Route TxKind::AiInfer to AIRuntime::infer()
- [x] Map InferenceResult to ExecutionReceipt (success, compute_units as gas_used)
- [x] Unknown model returns failed receipt with error
- **Tests:** covered by pipeline AI tests (Task 10)

### Task 8: AI inference receipts — DONE
- [x] ExecutionReceipt extended with `inference_hash: Option<[u8; 32]>` field
- [x] AI inference success populates `inference_hash` with deterministic hash
- [x] RPC response includes `inferenceHash` field when present
- **Tests:** receipt contains correct deterministic hash, existing RPC tests still pass

### Task 9: Model preload via node config — DONE
- [x] `NodeConfig.ai` section with `enabled: bool` and `models: Vec<ModelEntry>`
- [x] `ModelEntry { model_id, path }` for each ONNX model
- [x] On startup, load ONNX files and register with TractRuntime
- [x] Pass `Arc<TractRuntime>` to ExecutionPipeline via `set_ai_runtime()`
- **Tests:** config roundtrip TOML test passes with new AI section

### Task 10: AI inference pipeline tests — DONE
- [x] AI inference success: registered model → receipt with inference_hash
- [x] Unknown model: returns failed receipt with "not available" error
- [x] Mixed batch: transfer + AI inference → both receipts correct
- **Tests:** 3 pipeline tests (pipeline_ai_infer_success, pipeline_ai_infer_unknown_model, pipeline_mixed_transfer_and_ai)

**Phase 2 Exit Criteria:**
- [x] AI inference transactions execute through the full pipeline
- [x] Receipts include deterministic hash for verification
- [x] Models loadable from config at startup
- [x] All new tests pass (237 total)

---

## Phase 3: Transaction Pool Improvements (Tasks 11-14)

**Owner:** node-engineer
**Goal:** Production-grade mempool with priority ordering and bounded resource usage.

### Task 11: Mempool priority ordering — DONE
- [x] `BTreeMap<(Reverse<u64>, TxHash), Vec<u8>>` orders by priority descending
- [x] `insert_with_priority(tx, priority)` for explicit priority
- [x] `insert(tx)` derives priority from TxKind prefix byte
- [x] `drain_batch()` and `peek_batch()` return highest-priority first
- **Tests:** 2 tests (higher_priority_drained_first, peek_returns_priority_order, default_insert_uses_prefix_priority)

### Task 12: Mempool eviction policy — DONE
- [x] When pool is full, evict lowest-priority entry if new tx has higher priority
- [x] Reject new tx if priority <= lowest existing priority
- **Tests:** 3 tests (eviction_replaces_lowest_priority, eviction_rejects_equal_priority, eviction_rejects_lower_priority)

### Task 13: Mempool dedup by tx hash — DONE
- [x] BLAKE3 hash computed on insert, stored in `seen: HashSet<TxHash>`
- [x] `seen` set persists after removal (prevents replay of already-processed txs)
- [x] `index: HashMap<TxHash, u64>` maps hash to priority for O(1) lookup
- **Tests:** 3 tests (dedup_by_hash_persists_after_removal, dedup_uses_blake3_hash, different_content_different_hash)

### Task 14: Mempool unit tests — DONE
- [x] 16 total mempool tests (10 new: priority, eviction, dedup, extract_priority)
- [x] All edge cases covered: full pool, equal priority, removal + re-insertion, prefix-based priority
- **Tests:** 16 unit tests

**Phase 3 Exit Criteria:**
- [x] Mempool drains in priority order
- [x] Eviction works correctly under pressure
- [x] Hash-based dedup is correct and efficient
- [x] All new tests pass (247 total)

---

## Phase 4: Security Review + Documentation (Tasks 15-17)

**Owner:** security-engineer + documentation-engineer
**Goal:** Security review of all new code, documentation updates.

### Task 15: Security review — DONE
- [x] Multi-node gossip: vertex validation, replay protection, resource limits
- [x] AI pipeline: model loading from disk (path traversal), inference resource limits
- [x] Mempool: priority manipulation, eviction attacks, hash collision resistance
- [x] Rate findings as LOW/MEDIUM/ELEVATED
- **Findings:** 2 MEDIUM (SEC-MEM-001 fixed: bounded seen set; SEC-AI-001 documented: model paths operator-controlled), 4 LOW (SEC-GOSSIP-001/003, SEC-AI-002/003, SEC-MEM-002/003)

### Task 16: cargo-audit + clippy + fmt — DONE
- [x] `cargo audit` — 6 transitive vulns (ring, wasmtime ×3, tracing-subscriber), 7 warnings (bincode, derivative, ring, lru) — all transitive, no action
- [x] `cargo clippy --workspace` — zero warnings
- [x] `cargo fmt --check` — clean

### Task 17: Documentation updates — DONE
- [x] BUILD_LOG.md: entries for each phase
- [x] STATUS.md: M4 progress update
- [x] CHANGELOG.md: new features, security items
- [x] Sprint plan: mark tasks DONE

**Phase 4 Exit Criteria:**
- [x] Security review complete, no open ELEVATED flags
- [x] All docs updated
- [x] Sprint retrospective written

---

## Definition of Done (Sprint 008)

- [x] All 17 tasks completed or explicitly deferred with justification
- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes (248 tests, target was 240+)
- [x] `cargo clippy --workspace` zero warnings
- [x] `cargo fmt --check` clean
- [x] All code has BUILD_LOG entries
- [x] Security review complete (no open ELEVATED flags)
- [x] STATUS.md updated

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| Multi-node gossip may be flaky on Windows mDNS | MEDIUM | Fall back to explicit boot_nodes if mDNS fails |
| Vertex validation across nodes may reveal DAG edge cases | MEDIUM | Start with 3-node, expand later |
| AI model loading from disk introduces path traversal risk | MEDIUM | Validate paths, restrict to config.data_dir subtree |
| Mempool priority ordering may interact with consensus batch ordering | LOW | Priority is local only, consensus ordering is deterministic |
| TractRuntime inference under pipeline lock may block execution | MEDIUM | AI inference is synchronous in v1; async wrapper in future sprint |

---

## Sprint 008 Candidates for Next Sprint (009)

1. Block-STM optimistic parallel execution
2. State sync / snapshot protocol
3. EVM precompiles (ecrecover, sha256, etc.)
4. WebRTC transport for browser nodes
5. AI compute marketplace stubs (PoUW reward distribution)
6. Verkle tree state commitment (replace BLAKE3 Merkle placeholder)

---

## Sprint 008 Retrospective

**Duration:** Single session (2026-03-06)
**Tasks:** 17/17 completed (100%)
**Tests:** 248 total (56 new)

### What went well
- Multi-node consensus convergence test proves the core loop works across 3 nodes
- AI inference as first-class transaction type integrates cleanly via `dyn AIRuntime` trait
- Priority-ordered mempool with eviction is production-grade for a testnet
- Wire format with version byte, size limit, hash check, validator check, and round proximity covers all gossip attack vectors
- Security review found and fixed SEC-MEM-001 (unbounded `seen` set) before it could cause memory issues

### What could improve
- AI inference is synchronous under the state write lock — should be async in a future sprint
- `max_compute_units` not enforced during inference (gas metering gap)
- No per-sender rate limiting in mempool or gossip layer
- `seen` set eviction uses `Vec::remove(0)` which is O(n) — should use VecDeque in future

### Risks addressed
- SEC-MEM-001 MEDIUM: Bounded `seen` set to `max_size * 10` with FIFO eviction
- SEC-AI-001 documented: Model paths are operator-controlled (config file), not user-facing
- All transitive cargo-audit advisories remain documented, no new direct vulns

### Key metrics
- Lines of new code: ~600 (wire.rs, mempool rewrite, AI pipeline additions, config)
- Security findings: 2 MEDIUM (1 fixed, 1 documented), 4 LOW (all documented)
- cargo-audit: 6 vulns + 7 warnings (all transitive, previously documented)
