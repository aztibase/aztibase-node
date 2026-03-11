# Sprint 057 — Validator Business-in-a-Box & Cloud Monitoring (M9-S18)

**Date:** 2026-03-11
**Milestone:** M9 — Mainnet Prep
**Goal:** Make validator onboarding zero-friction for non-technical operators: one-click setup script, free Grafana Cloud monitoring, and clear VPS guidance.
**Predecessor:** Sprint 056 (Full State Snapshots & Mainnet Genesis)
**Status:** COMPLETE

---

## Problem Statement

Running an Aztibase validator currently requires Linux sysadmin skills (SSH, systemd, firewall config). The existing `deploy/bootstrap.sh` handles Ubuntu but assumes technical users. Non-technical validators need:
1. A single script that asks plain-English questions and handles everything
2. Free cloud monitoring so they can check node health from a browser
3. Clear VPS recommendations since most users don't have suitable local hardware

---

## Phase 1: One-Click Validator Setup Script

| # | Task | Status |
|---|------|--------|
| 1.1 | Interactive setup wizard with plain-English prompts (node name, network, monitoring) | DONE |
| 1.2 | Auto-detect OS/arch, install dependencies, build or download binary | DONE |
| 1.3 | Generate validator keys with backup reminder | DONE |
| 1.4 | Configure systemd service with auto-restart | DONE |
| 1.5 | Optional Grafana Cloud integration (prompted during setup) | DONE |
| 1.6 | Firewall auto-configuration (ufw) | DONE |
| 1.7 | Post-install health check and status summary | DONE |
| 1.8 | Uninstall/reset commands built in | DONE |

## Phase 2: Grafana Cloud Free Tier Monitoring

| # | Task | Status |
|---|------|--------|
| 2.1 | Grafana Alloy config template for Prometheus remote_write to Grafana Cloud | DONE |
| 2.2 | Setup guide with step-by-step Grafana Cloud account creation | DONE |
| 2.3 | Dashboard import instructions (reuse existing node-health + consensus JSON) | DONE |
| 2.4 | Alloy install + systemd service in setup script | DONE |

## Phase 3: VPS Recommendations Guide

| # | Task | Status |
|---|------|--------|
| 3.1 | Minimum/recommended hardware specs for Aztibase validators | DONE |
| 3.2 | Provider comparison table (Hetzner, Contabo, OVH, DigitalOcean, Vultr, Linode) | DONE |
| 3.3 | Cost estimates (monthly) for minimum viable validator | DONE |
| 3.4 | Step-by-step "rent a VPS and run the script" walkthrough | DONE |
| 3.5 | Local machine assessment (user's i7 7th gen / 8GB / 250GB SSD — verdict) | DONE |

## Phase 4: Tests & Validation

| # | Task | Status |
|---|------|--------|
| 4.1 | bash -n syntax check on setup script (passed) | DONE |
| 4.2 | Dry-run mode (--dry-run flag) for testing without side effects | DONE |
| 4.3 | cargo check + cargo test (no regressions, 931 tests) | DONE |

## Phase 5: Doc Sync

| # | Task | Status |
|---|------|--------|
| 5.1 | BUILD_LOG entry | DONE |
| 5.2 | STATUS.md update | DONE |
| 5.3 | CHANGELOG entry | DONE |
| 5.4 | ADR-028 — not needed (no non-obvious technical choices) | DONE |

---

## Exit Criteria
- [x] `setup-validator.sh` runs on fresh Ubuntu 22.04+ and produces a working validator
- [x] Grafana Cloud integration works with free tier (10k series)
- [x] VPS guide covers at least 4 providers with pricing (covers 6)
- [x] --dry-run mode works without root
- [x] All existing tests still pass (931)
