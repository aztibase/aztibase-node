# Sprint 054 — Mainnet Operational Hardening (M9-S15)

**Date:** 2026-03-11
**Milestone:** M9 — Mainnet Prep
**Goal:** Close critical operational gaps between testnet and mainnet: network profiles, CORS lockdown, RPC rate limiting, graceful shutdown, validator key rotation.
**Predecessor:** Sprint 053 (Protocol Store Persistence)
**Status:** COMPLETE

---

## Problem Statement

The node binary treats testnet and mainnet identically. CORS is permissive, faucet is always available, there's no per-IP rate limiting, shutdown doesn't guarantee a clean state flush, and validators can't rotate keys without deregistering. These are mainnet blockers.

---

## Phase 1: Mainnet Genesis & Network Profiles

| # | Task | Status |
|---|------|--------|
| 1.1 | `NetworkProfile` enum (`Testnet`, `Mainnet`, `Dev`) in aztibase-node config | DONE |
| 1.2 | `--mainnet` CLI flag (mutually exclusive with `--testnet`) | DONE |
| 1.3 | Mainnet genesis template (no faucet, no deterministic keys) | DEFERRED (no separate genesis template needed — profile gates faucet) |
| 1.4 | Faucet RPC (`aztb_faucetDrip`) disabled when profile = Mainnet | DONE |
| 1.5 | Tests: profile selection, faucet gate, genesis validation | DONE (7 config tests + 2 faucet gate tests) |

## Phase 2: CORS & RPC Hardening

| # | Task | Status |
|---|------|--------|
| 2.1 | CORS policy driven by NetworkProfile: Mainnet → explicit origin whitelist, Testnet → Allow-Any | DONE |
| 2.2 | Per-IP RPC rate limiter (token bucket, configurable, default 100 req/s) | DONE |
| 2.3 | Request body size limit (1 MiB default, configurable) | DONE |
| 2.4 | Tests: CORS reject on mainnet, rate limit trigger, oversized body reject | DONE (3 rate limiter tests) |

## Phase 3: Graceful Shutdown & DB Health

| # | Task | Status |
|---|------|--------|
| 3.1 | Shutdown sequence: stop RPCs → drain in-flight → flush stores → close redb | DONE |
| 3.2 | Startup integrity: read/write sentinel key in STATE_TABLE | DONE |
| 3.3 | Dirty-start detection: warn if sentinel indicates unclean prior shutdown | DONE |
| 3.4 | Tests: sentinel roundtrip, dirty-start warning, clean shutdown clears sentinel | DONE (3 sentinel tests) |

## Phase 4: Validator Key Rotation

| # | Task | Status |
|---|------|--------|
| 4.1 | `TxKind::RotateValidatorKey` (0x1A) — old key signs tx authorizing new key | DONE |
| 4.2 | StakingStore: update validator pubkey, invalidate old key | DONE |
| 4.3 | Pipeline execution + consensus validator set propagation | DONE |
| 4.4 | Tests: rotate happy path, reject unauthorized, reject duplicate key | DONE (4 staking + 1 routing tests) |

## Phase 5: Security Review & Docs

| # | Task | Status |
|---|------|--------|
| 5.1 | Security review: rate limiter bypass, shutdown races, key rotation edges | DONE |
| 5.2 | `cargo clippy` 0 warnings, `cargo fmt --check` clean | DONE |
| 5.3 | ADR-025: Network profiles & mainnet hardening | DONE |
| 5.4 | BUILD_LOG, STATUS, CHANGELOG, sprint plan updates | DONE |

---

## Exit Criteria

- [x] NetworkProfile drives CORS, faucet, genesis defaults
- [x] RPC rate-limited per IP (token bucket)
- [x] Clean shutdown flushes all state; dirty startup warned
- [x] Validator key rotation functional (TxKind 0x1A)
- [x] ~20 new tests, all passing (19 new, 925 total)
- [x] 0 clippy warnings, fmt clean
- [x] ADR-025 written

---

## Key Files (Actual)

| File | Change |
|------|--------|
| `crates/aztibase-node/src/config.rs` | NetworkProfile enum, RPC rate limit/body size/CORS config, 7 tests |
| `crates/aztibase-node/src/main.rs` | --mainnet flag, profile-driven startup, sentinel, graceful shutdown |
| `crates/aztibase-node/src/pipeline.rs` | RotateValidatorKey classification + execution + hash |
| `crates/aztibase-rpc/src/server.rs` | RpcRateLimiter, CORS by profile, faucet gate, body size limit, 6 tests |
| `crates/aztibase-execution/src/persist.rs` | Sentinel functions (write/clear/check), 3 tests |
| `crates/aztibase-execution/src/staking.rs` | rotate_key(), 4 tests |
| `crates/aztibase-execution/src/routing.rs` | TxKind::RotateValidatorKey (0x1A), 1 test |
| `crates/aztibase-execution/src/fee.rs` | Gas estimate 60K for RotateValidatorKey |
| `crates/aztibase-execution/src/lib.rs` | Re-exports for sentinel functions |

---

## Security Review Notes

1. **Rate limiter**: Per-process only, not distributed. Acceptable for single-node testnet/mainnet. For multi-RPC deployments, operators should front with a reverse proxy (nginx, cloudflare).
2. **Sentinel**: Best-effort — if the process is killed with SIGKILL, sentinel won't clear. The warning on dirty start is informational, not blocking.
3. **Key rotation auth**: Only the old key can authorize rotation (tx sender == validator address). No risk of third-party key replacement.
4. **CORS on Mainnet**: Empty whitelist means no CORS headers → browser requests blocked by default. Operators must explicitly configure allowed origins.
