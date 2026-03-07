# Sprint 015 — Metrics Telemetry, Light Client Foundations, WebRTC Transport

**Sprint Number:** 015
**Start Date:** 2026-03-07
**End Date:** 2026-03-07
**Status:** COMPLETE
**Milestone:** M4 — Integration Testing + AI + Testnet

---

## Objective

Wire consensus/execution metrics into the node binary with an HTTP telemetry endpoint, lay the light client proof foundation for M5, and scaffold WebRTC transport for browser-node connectivity.

---

## Phases & Tasks

### Phase 1: Metrics Wiring + HTTP Telemetry (Tasks 1–4)

Integrate the `ConsensusMetrics` struct (Sprint 014) into the live node and expose via HTTP.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `NodeMetrics` aggregator: consensus + execution counters in `aztibase-node` | node-engineer | DONE |
| 2 | `GET /metrics` HTTP endpoint on RPC server (JSON format) | node-engineer | DONE |
| 3 | `--metrics` CLI flag to enable metrics endpoint (default: disabled) | node-engineer | DONE |
| 4 | Metrics integration test: spawn node, query `/metrics`, validate JSON schema | node-engineer | DONE |

**Exit Criteria:** MET
- `--metrics` flag starts metrics HTTP listener ✓
- `/metrics` returns JSON with consensus + execution counters ✓
- 2 new tests passing ✓

---

### Phase 2: Light Client Proof Types + Snapshot Hardening (Tasks 5–8)

Extend the proof system for light client verification and harden state snapshots.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `LightClientProof` variant in `StateProof` enum: state root + height + finality cert | blockchain-architect | DONE |
| 6 | `verify_light_client_proof()`: validate finality cert + state root match | blockchain-architect | DONE |
| 7 | `StateSnapshot` gains `height` and `finality_certificate` fields for canonical verification | node-engineer | DONE |
| 8 | Criterion benchmarks: Merkle/Verkle proof verification at 100/1000/10000 leaves, BLS cert verification at 21/100 validators | consensus-engineer | DONE |

**Exit Criteria:** MET
- Light client can verify a state proof against a finality certificate ✓
- Snapshot carries height + finality proof ✓
- Benchmark baselines recorded ✓
- 7 new tests passing ✓

---

### Phase 3: WebRTC Transport Scaffold (Tasks 9–12)

Add WebRTC as an optional transport for browser-node connectivity (M5 prerequisite).

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Add `libp2p-webrtc` dependency (feature-gated: `webrtc`) | p2p-network-engineer | DONE |
| 10 | `WebRtcTransport` struct: initialize WebRTC transport alongside QUIC | p2p-network-engineer | DONE |
| 11 | STUN server config in `NodeConfig`: `stun_servers` field with defaults | p2p-network-engineer | DONE |
| 12 | Integration test: WebRTC transport initialization + peer dial attempt | p2p-network-engineer | DONE |

**Exit Criteria:** MET
- `cargo check --features webrtc` compiles ✓
- WebRTC transport initializes alongside existing QUIC transport ✓
- STUN config in node TOML ✓
- 4 new tests passing ✓

---

### Phase 4: Security Review + Documentation (Tasks 13–15)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: metrics endpoint exposure, light client trust model, WebRTC DTLS | security-engineer | DONE |
| 14 | `cargo clippy` zero warnings, `cargo fmt --check` clean, `cargo test` all passing | security-engineer | DONE |
| 15 | Doc sync: BUILD_LOG, STATUS, CHANGELOG, DECISIONS (ADR-008), sprint retro | project-lead | DONE |

**Exit Criteria:** MET
- 0 ELEVATED security flags ✓
- All 410 tests passing (0 failures, 0 flaky) ✓
- All docs updated ✓

---

## Security Findings

| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| SEC-METRIC-001 | LOW | Metrics JSON snapshot is eventually consistent | DOCUMENTED |
| SEC-WEBRTC-001 | LOW | STUN server URL format not validated | DOCUMENTED |
| SEC-LC-001 | LOW | Finality certificate structural validation deferred to M5 | DOCUMENTED |

**Bonus fix:** Block-STM parallel validation race (pre-existing flaky test) — `finish_validation()` now checks all prior txs are Validated before accepting a valid result.

---

## New Tests: 13
## Total Tests: 410 (76 consensus + 22 core + 140 execution + 15 network + 100 node + 26 rpc + 16 runtime + 15 storage)

---

## Dependencies

- Sprint 014: `ConsensusMetrics`, `StateCommitment` trait, `StateProof` enum, `VerkleTree`
- Sprint 013: `FinalityCertificate`, `ValidatorSet` with BLS keys
- Sprint 009: `StateSnapshot`, state sync protocol

---

## Retrospective

### What went well
- Metrics design via `Arc<RwLock<serde_json::Value>>` cleanly avoided cross-crate coupling (ADR-008)
- LightClientProof integrates seamlessly with existing StateCommitment trait (ADR-007 payoff)
- `libp2p-webrtc` 0.9.0-alpha.1 compiled cleanly on Windows with pure-Rust webrtc-rs (no ADR-001 violation)
- Fixed the long-standing Block-STM parallel validation race that caused flaky tests since Sprint 009

### What could improve
- WebRTC scaffold is minimal — full swarm integration needs Sprint 016+ work
- `libp2p-webrtc` is alpha (0.9.0-alpha.1) — may need version bumps as it stabilizes
- Finality certificate is opaque bytes in LightClientProof — needs structural validation in M5

### Key decisions
- ADR-008: Opaque JSON metrics over shared metrics crate (simpler, extensible)
- `#[serde(default)]` on new snapshot fields for backward compatibility
- Feature-gated WebRTC (`--features webrtc`) to avoid compile-time penalty in default builds
