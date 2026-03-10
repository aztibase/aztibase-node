# Sprint 050 — CI Pipeline & Public Testnet Infrastructure (M9-S11)

**Date:** 2026-03-10
**Milestone:** M9 — Mainnet Prep
**Goal:** Build the infrastructure required for a credible public testnet: CI pipeline, release-mode benchmarks, genesis config, faucet, and deployment tooling.
**Predecessor:** Sprint 049 (protocol hardening, crash recovery, throughput baseline)

---

## Phase 1: GitHub Actions CI Pipeline

| # | Task | Status |
|---|------|--------|
| 1.1 | `.github/workflows/ci.yml` — cargo fmt, clippy, test on push/PR | DONE (Sprint 050 prep) |
| 1.2 | Cache cargo registry + target dir for fast CI builds | DONE (Swatinem/rust-cache@v2) |
| 1.3 | Release build gate — `cargo build --release` succeeds | DONE (build-release job) |
| 1.4 | Security audit job in CI (rustsec/audit-check) | DONE (audit job) |

## Phase 2: Release-Mode Benchmarks

| # | Task | Status |
|---|------|--------|
| 2.1 | Release-mode TPS measurement (reuse Sprint 049 harness) | PARTIAL — consensus benches done, execution benches deferred (long-running) |
| 2.2 | Criterion benchmark suite: state_root, transfer_execution, merkle/verkle proof, verify_tx, verify_tx_batch | DONE (7 bench groups, 2 new) |
| 2.3 | Benchmark results documented in STATUS.md | DONE (consensus benches in BUILD_LOG + CHANGELOG) |
| 2.4 | CI bench job (optional — criterion output only, no regression gate yet) | DEFERRED (requires baseline) |

## Phase 3: Public Testnet Genesis & Config

| # | Task | Status |
|---|------|--------|
| 3.1 | Deterministic 3-validator testnet genesis from fixed seeds | DONE — `testnet_genesis()` |
| 3.2 | Faucet account with 100M tokens in testnet genesis | DONE — bundled in `testnet_genesis()` |
| 3.3 | Boot node multiaddrs for public DNS (testnet1/2/3.aztibase.com) | DONE — `testnet_boot_nodes()` |
| 3.4 | `--testnet` CLI flag loads bundled genesis config + boot nodes | DONE — wired into main.rs |
| 3.5 | 3 unit tests (valid, deterministic, boot node format) | DONE |

## Phase 4: Faucet & Deployment

| # | Task | Status |
|---|------|--------|
| 4.1 | RPC faucet endpoint (`aztb_faucetDrip`) — rate-limited, testnet only | DONE (Sprint 026) |
| 4.2 | Dockerfile for validator node (multi-stage, release build) | DONE (Sprint 027) |
| 4.3 | `docker-compose.yml` for 3-node local testnet + monitoring | DONE (Sprint 027) |
| 4.4 | Deploy script for VPS/cloud validator nodes | DONE — `scripts/deploy-validator.sh` |

## Phase 5: Validation, Security & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | Full workspace test suite passes (860+ tests) | DONE |
| 5.2 | Security review — testnet genesis (deterministic keys = well-known, testnet only), deploy script (systemd hardened) | DONE |
| 5.3 | RPC API documentation (all 43 methods) | DONE (docs/rpc-api.md, Sprint 050 prep) |
| 5.4 | Doc sync (BUILD_LOG, STATUS, CHANGELOG, sprint plan) | DONE |
| 5.5 | Sprint 051 scoping — block explorer, wallet UI, public launch | PENDING |

---

## Exit Criteria

- [x] CI pipeline runs on every push and PR (fmt + clippy + test + audit)
- [x] Release-mode benchmarks measured (consensus crate complete, execution deferred)
- [x] Criterion benchmarks for 3+ hot paths (7 bench groups)
- [x] Genesis config for public testnet with 3 boot nodes
- [x] Faucet endpoint works (rate-limited)
- [x] Docker image builds and runs a validator
- [x] docker-compose brings up 3-node testnet
- [x] RPC API docs for all 43 methods
- [x] 0 clippy warnings, fmt clean, 863 tests pass (862 pass, 1 flaky — validator_crash_and_recovery passes on re-run)

---

## Retrospective
*(To be filled at sprint close)*
