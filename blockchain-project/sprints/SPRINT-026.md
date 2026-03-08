# Sprint 026 — M8 Sprint 1: Prometheus Metrics + Testnet Infrastructure

**Milestone:** M8 (Public Testnet) — Sprint 1 of 3
**Start Date:** 2026-03-08
**Status:** COMPLETE

---

## Goal

Introduce Prometheus-format metrics export, Grafana dashboards via Docker Compose, a testnet faucet RPC, and node health/info endpoints — establishing the operational foundation for public testnet launch.

---

## Phase 1: Prometheus Metrics Integration (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add `prometheus-client` crate (pure Rust, 0.23) to workspace dependencies. Verify cargo-deny clean. | node-engineer | DONE |
| 2 | Create `NodeMetrics` in aztibase-rpc: typed counters (vertices_proposed, vertices_received, commits, rounds_advanced, equivocations, txs_processed) and gauges (block_height, base_fee, peer_count, mempool_size, pending_tasks, last_commit_latency_us). 4 tests. | node-engineer | DONE |
| 3 | Dual-format endpoint: `GET /metrics` returns Prometheus text (text/plain; version=0.0.4), `GET /metrics/json` returns JSON for backward compatibility. 3 tests. | node-engineer | DONE |
| 4 | Wire `NodeMetrics` into main.rs consensus/execution loops. Replace `Arc<RwLock<Value>>` with registry. `inc_txs_processed` on each batch. | node-engineer | DONE |

**Exit Criteria:** `GET /metrics` returns valid Prometheus text format. `GET /metrics/json` returns JSON. All existing tests pass. ✅

---

## Phase 2: Grafana + Prometheus Docker Stack (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Add `monitoring/prometheus.yml` scrape config: 5s interval, targets validator1-3 on port 9944. | node-engineer | DONE |
| 6 | Add Prometheus (v2.53.0) and Grafana (v11.1.0) services to `docker-compose.yml`. Grafana auto-provisioned with Prometheus datasource. Persistent volumes. | node-engineer | DONE |
| 7 | Create Grafana dashboard JSON: Node Health — block height, TPS (rate), peer count, mempool size, base fee, pending AI tasks, total txs. | node-engineer | DONE |
| 8 | Create Grafana dashboard JSON: Consensus — rounds advanced, commit rate, commit latency (ms), vertices proposed/received, equivocations, total commits. | node-engineer | DONE |

**Exit Criteria:** `docker-compose up` starts validators + Prometheus + Grafana. Dashboards auto-provision on Grafana port 3000. ✅

---

## Phase 3: Testnet Operational Endpoints (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | `aztb_faucetDrip` RPC method: testnet-only (gated by chain_id 0xA27B), 10 AZTB per drip, rate-limited to 1 per address per 60s. 3 tests (success, rate limited, invalid address). | node-engineer | DONE |
| 10 | `aztb_nodeInfo` RPC method: returns version (from CARGO_PKG_VERSION), chain_id, block_height, protocol_version. 1 test. | node-engineer | DONE |
| 11 | `GET /health` HTTP endpoint: returns 200 OK with `{"status":"ok","blockHeight":N,"chainId":"0xa27b"}`. 1 test. | node-engineer | DONE |
| 12 | Update `docker-compose.yml` healthcheck to use `GET /health`. Expose Grafana (3000) and Prometheus (9090) ports. | node-engineer | DONE |

**Exit Criteria:** Faucet works on testnet chain_id only. `/health` returns 200. `aztb_nodeInfo` returns node metadata. Docker stack complete. ✅

---

## Phase 4: Security Review + Sprint Close (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: faucet rate limiting (no treasury drain), health endpoint (no info leak), metrics (no sensitive data exposed). 0 findings above LOW. | security-engineer | DONE |
| 14 | `cargo clippy` zero warnings, `cargo fmt --check` clean, `cargo-deny check` clean (warnings only: duplicate transitive deps). | security-engineer | DONE |
| 15 | All 53 RPC tests pass including 9 new tests for faucet, nodeInfo, health, metrics Prometheus, and metrics JSON endpoints. | security-engineer | DONE |
| 16 | Doc updates: BUILD_LOG, STATUS, CHANGELOG, sprint retrospective. Memory update. | documentation-engineer | DONE |

**Exit Criteria:** Zero security findings above LOW. All docs updated. Sprint retrospective written. ✅

---

## Dependencies

- Phase 1 must complete before Phase 2 (Prometheus needs metrics endpoint)
- Phase 3 is independent of Phase 2 (can overlap)
- Phase 4 depends on all prior phases

---

## New Dependencies

| Crate | Version | Pure Rust | Purpose |
|-------|---------|-----------|---------|
| `prometheus-client` | 0.23 | YES | Prometheus metrics registry + text encoding |

---

## New RPC Methods

| Method | Params | Returns |
|--------|--------|---------|
| `aztb_faucetDrip` | `{address: hex}` | `{address, amount: 10, balance}` |
| `aztb_nodeInfo` | none | `{version, chainId, blockHeight, protocolVersion}` |

## New HTTP Endpoints

| Endpoint | Method | Content-Type | Purpose |
|----------|--------|-------------|---------|
| `GET /metrics` | GET | text/plain; version=0.0.4 | Prometheus metrics |
| `GET /metrics/json` | GET | application/json | JSON metrics (backward compat) |
| `GET /health` | GET | application/json | Node health check |

---

## Sprint Retrospective

### What went well
- prometheus-client 0.23 integrated seamlessly — pure Rust, zero C deps, aligns with ADR-001
- Dual-format metrics preserves ADR-008 backward compatibility while enabling Grafana scraping
- Faucet rate limiting is simple and effective — HashMap<Address, Instant> with 60s cooldown
- /health endpoint simplifies Docker healthchecks (GET vs POST JSON-RPC)
- Grafana dashboards auto-provision via file provisioning — zero manual setup

### What could improve
- Full workspace test suite hit disk space limits on Windows (PDB files + debug symbols)
- `cargo clean` required before full test runs on this machine
- Faucet drip amount (10 AZTB) and cooldown (60s) are hardcoded — could be configurable via TOML

### Key decisions
- Kept JSON metrics at `/metrics/json` for ADR-008 backward compatibility
- Faucet uses direct state mutation (not a transaction) — simpler for testnet, not suitable for mainnet
- Grafana default credentials: admin/aztibase — must change before public deployment

### Metrics
- 16/16 tasks complete
- 0 new security findings
- 9 new tests (4 metrics + 3 faucet + 1 nodeInfo + 1 health)
- 53 total RPC tests, clippy 0 warnings, fmt clean
- 1 new dependency (prometheus-client 0.23, pure Rust)
- 2 new Grafana dashboards, 3 new HTTP endpoints, 2 new RPC methods
