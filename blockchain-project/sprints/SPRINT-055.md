# Sprint 055 — Public Testnet Launch Infrastructure (M9-S16)

**Date:** 2026-03-11
**Milestone:** M9 — Mainnet Prep
**Goal:** Close the gap between "working local testnet" and "public testnet anyone can join". Ship canonical genesis, faucet UI, testnet landing page, and deployment helpers.
**Predecessor:** Sprint 054 (Mainnet Operational Hardening)
**Status:** COMPLETE

---

## Problem Statement

Sprint 047 proved the node works on a 3-node local testnet. Sprints 050-054 added CI, benchmarks, persistence, and hardening. But there's no public-facing infrastructure for external validators or developers to join a testnet. Missing: canonical genesis distribution, faucet web UI, join instructions, and deployment tooling.

---

## Phase 1: Canonical Testnet Genesis & Seed Node Configs

| # | Task | Status |
|---|------|--------|
| 1.1 | Generate and commit canonical testnet genesis.toml (deterministic, 3 validators + faucet) | DONE |
| 1.2 | Pre-generate 3 seed node TOML configs (testnet1/2/3.aztibase.com) | DONE |
| 1.3 | Systemd unit file for running aztibase as a service | DONE |
| 1.4 | Cloud bootstrap script (Ubuntu 22.04: install deps, build, configure, start) | DONE |

## Phase 2: Faucet Web UI

| # | Task | Status |
|---|------|--------|
| 2.1 | Static HTML faucet page (same dark theme as explorer) | DONE |
| 2.2 | Address input, drip button, tx hash display, rate limit feedback | DONE |
| 2.3 | Configurable RPC endpoint (default: local, override via URL param) | DONE |

## Phase 3: Testnet Landing Page

| # | Task | Status |
|---|------|--------|
| 3.1 | Static HTML landing page with network stats, quick links, join instructions | DONE |
| 3.2 | "Join as Validator" step-by-step guide (build, configure, stake, run) | DONE |
| 3.3 | "Use the Testnet" developer guide (faucet, send tx, query RPC) | DONE |
| 3.4 | Link to explorer, faucet, RPC docs | DONE |

## Phase 4: Deployment Helpers

| # | Task | Status |
|---|------|--------|
| 4.1 | `deploy/systemd/aztibase.service` — production systemd unit | DONE |
| 4.2 | `deploy/bootstrap.sh` — cloud VM bootstrap (build from source + configure) | DONE |
| 4.3 | `deploy/seed-nodes/` — 3 pre-generated seed node configs | DONE |

## Phase 5: Security Review & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | Security review: faucet XSS, CORS, rate limit bypass, script injection | DONE |
| 5.2 | ADR-026: Public testnet launch infrastructure | DONE |
| 5.3 | BUILD_LOG, STATUS, CHANGELOG, sprint plan updates | DONE |

---

## Exit Criteria

- [x] Canonical testnet genesis.toml committed and deterministic
- [x] 3 seed node configs ready for cloud deployment
- [x] Faucet web UI functional (HTML + RPC call)
- [x] Testnet landing page with join instructions
- [x] Systemd unit + bootstrap script for cloud VMs
- [x] Security review clean (no XSS, no injection)
- [x] ADR-026 written
- [x] All docs updated
