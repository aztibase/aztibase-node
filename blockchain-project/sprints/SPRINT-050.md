# Sprint 050 — CI Pipeline & Public Testnet Infrastructure (M9-S11)

**Date:** 2026-03-10
**Milestone:** M9 — Mainnet Prep
**Goal:** Build the infrastructure required for a credible public testnet: CI pipeline, release-mode benchmarks, genesis config, faucet, and deployment tooling.
**Predecessor:** Sprint 049 (protocol hardening, crash recovery, throughput baseline)

---

## Phase 1: GitHub Actions CI Pipeline

| # | Task | Status |
|---|------|--------|
| 1.1 | `.github/workflows/ci.yml` — cargo fmt, clippy, test on push/PR | PENDING |
| 1.2 | Cache cargo registry + target dir for fast CI builds | PENDING |
| 1.3 | Release build gate — `cargo build --release` succeeds | PENDING |
| 1.4 | Test coverage badge in README | PENDING |

## Phase 2: Release-Mode Benchmarks

| # | Task | Status |
|---|------|--------|
| 2.1 | Release-mode TPS measurement (reuse Sprint 049 harness) | PENDING |
| 2.2 | Criterion benchmark suite for hot paths (execute_batch, verify_tx, state_root) | PENDING |
| 2.3 | Benchmark results documented in STATUS.md | PENDING |
| 2.4 | CI job to run benchmarks and detect regressions | PENDING |

## Phase 3: Public Testnet Genesis & Config

| # | Task | Status |
|---|------|--------|
| 3.1 | Generate 3 validator keypairs for public testnet boot nodes | PENDING |
| 3.2 | `testnet-genesis.toml` — chain params, initial balances, validator set | PENDING |
| 3.3 | Boot node multiaddrs for public DNS (testnet1/2/3.aztibase.com) | PENDING |
| 3.4 | `--testnet` CLI flag loads bundled genesis config | PENDING |

## Phase 4: Faucet & Deployment

| # | Task | Status |
|---|------|--------|
| 4.1 | RPC faucet endpoint (`aztb_faucetDrip`) — rate-limited, testnet only | PENDING |
| 4.2 | Dockerfile for validator node (multi-stage, release build) | PENDING |
| 4.3 | `docker-compose.yml` for 3-node local testnet | PENDING |
| 4.4 | Deploy script for VPS/cloud validator nodes | PENDING |

## Phase 5: Validation, Security & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | Full workspace test suite in CI (cargo test --workspace) | PENDING |
| 5.2 | Security review — genesis config, faucet rate limits, key management | PENDING |
| 5.3 | RPC API documentation (all 43 methods) | PENDING |
| 5.4 | Doc sync (BUILD_LOG, STATUS, CHANGELOG, sprint plan) | PENDING |
| 5.5 | Sprint 051 scoping — block explorer, wallet UI, public launch | PENDING |

---

## Exit Criteria

- [ ] CI pipeline runs on every push and PR (fmt + clippy + test)
- [ ] Release-mode TPS measured and documented
- [ ] Criterion benchmarks for 3+ hot paths
- [ ] Genesis config for public testnet with 3 boot nodes
- [ ] Faucet endpoint works (rate-limited)
- [ ] Docker image builds and runs a validator
- [ ] docker-compose brings up 3-node testnet
- [ ] RPC API docs for all 43 methods
- [ ] 0 clippy warnings, fmt clean, all tests pass

---

## Retrospective
*(To be filled at sprint close)*
