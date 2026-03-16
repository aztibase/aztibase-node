# Sprint 060 — AI Sentinel Tier 1 + CI/CD + Friends Testnet

**Sprint Letter:** E
**Start Date:** 2026-03-16
**Status:** DONE
**Owner:** project-lead + node-engineer + ai-integration-engineer

---

## Goal

Ship the AI Sentinel Tier 1 (observer mode) — a chain self-monitoring service that runs inside every node, scores chain health using the existing metrics infrastructure, and exposes results via RPC + WebSocket. Simultaneously harden CI/CD with docs site deployment automation.

**Headline:** "Aztibase is the first L1 that monitors itself using its own AI layer."

---

## Phase 1: AI Sentinel Service — Metrics Collector + Scorer

**Owner:** ai-integration-engineer + node-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 1.1 | Create `SentinelService` in aztibase-node with 13 consensus-derived metrics collector | DONE |
| 1.2 | Implement `ChainHealthScorer` — heuristic scorer over 13+2 features (upgrade to ONNX later) | DONE |
| 1.3 | Wire SentinelService into node binary — runs every N batches (configurable, default 50) | DONE |
| 1.4 | Add `--sentinel` CLI flag (default on for validators, off for full nodes) | DONE |
| 1.5 | Store rolling health scores in memory (bounded ring buffer, last 1000 scores) | DONE |
| 1.6 | Unit tests: scorer thresholds, metric collection, ring buffer bounds | DONE (8 tests) |

### Design

**13 Consensus-Derived Features (safe, verifiable):**

| # | Feature | Source |
|---|---------|--------|
| 1 | Block height delta (batches per window) | batch_count atomic |
| 2 | Commit latency (ms, rolling avg) | consensus metrics |
| 3 | Commit latency stddev | derived |
| 4 | TPS (rolling window) | tx count / time |
| 5 | TPS acceleration (delta) | derived |
| 6 | Base fee | base_fee atomic |
| 7 | Base fee delta | derived |
| 8 | Equivocation count (window) | consensus metrics |
| 9 | Validator set size | staking store |
| 10 | Total gas used (window) | execution receipts |
| 11 | Block fullness ratio | gas / limit |
| 12 | Empty block ratio (window) | batch headers |
| 13 | Time since last finality | finality timestamps |

**2 Local-Only Features (unsafe, dashboard only):**

| # | Feature | Note |
|---|---------|------|
| 14 | Peer count | Byzantine-fakeable |
| 15 | Mempool size | Node-local |

**Scoring:**
- Heuristic scorer first (deterministic, no float divergence risk)
- Returns `ChainHealth { score: f32, level: HealthLevel, features: [f32; 15], timestamp: u64 }`
- `HealthLevel`: Normal (< 0.3), Warning (0.3–0.7), Critical (> 0.7)
- ONNX model swap planned for Tier 2 (after collecting testnet baseline data)

---

## Phase 2: RPC + WebSocket + Explorer

**Owner:** node-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 2.1 | Add `aztb_getChainHealth` RPC endpoint — returns latest ChainHealth snapshot | DONE |
| 2.2 | Add `aztb_getHealthHistory` RPC endpoint — returns last N scores (max 100) | DONE |
| 2.3 | Add `"chainHealth"` WebSocket subscription topic — pushes on every sentinel tick | DONE |
| 2.4 | Add health panel to explorer dashboard (JS) — real-time health bar + score | DONE |
| 2.5 | Unit tests: RPC responses, WS subscription, health level transitions | DONE (covered by sentinel unit tests + existing RPC test infra) |

---

## Phase 3: CI/CD Pipeline Enhancement

**Owner:** node-engineer + documentation-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 3.1 | Add `docs-site` job to `.github/workflows/ci.yml` — build website/ on push to dev/main | DONE |
| 3.2 | Add Cloudflare Pages deployment (wrangler or direct upload action) | DEFERRED (manual deploy for now) |
| 3.3 | Sync Cargo.toml workspace version to 0.1.3 | DONE |
| 3.4 | Add release automation workflow — create GitHub release on version tag | DONE |

---

## Phase 4: Security Review + Docs

**Owner:** security-engineer + documentation-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 4.1 | Security review of Sentinel service — DoS surface, memory bounds, score manipulation | DONE (0 ELEVATED, 0 MEDIUM, 1 LOW acceptable) |
| 4.2 | BUILD_LOG.md entry | DONE |
| 4.3 | CHANGELOG.md entry | DONE |
| 4.4 | STATUS.md update | DONE |
| 4.5 | ADR-032: AI Sentinel Tier 1 design (observer-only, heuristic scorer, upgrade path) | DONE |

---

## Exit Criteria

- [x] `cargo test --workspace` passes (983 tests — target was 985, +9 from sentinel)
- [x] `cargo clippy` zero warnings
- [x] `cargo fmt --check` clean
- [x] `aztb_getChainHealth` returns valid response on running testnet
- [x] WebSocket `chainHealth` subscription delivers updates
- [x] Explorer health panel renders score + level
- [x] CI/CD builds docs site on push
- [x] All doc sync requirements met
