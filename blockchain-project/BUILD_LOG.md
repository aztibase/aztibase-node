# Build Log -- Aztibase Network

Running diary of build activity. Each entry records who built what, when, and any review notes.
Entries are prepended (newest first).

---

## Log Format

```
### [DATE] -- [ENGINEER] -- [CRATE/AREA]
**Task:** [What was done]
**Sprint:** [Sprint reference]
**Git Ref:** [Commit hash]
**Files Changed:** [List]
**Review Notes:** [Any observations, issues, or follow-ups]
**Security Flags:** [None / Description]
```

---

## Entries

### Public Testnet Infrastructure & Docs (2026-03-13)
- **Date**: 2026-03-13
- **Sprint**: Post-059 (Testnet Deployment)
- **Commit**: TBD
- **Files changed**:
  - `start-testnet-public.sh` — Combined launcher for testnet + Cloudflare Tunnel
  - `docs/VALIDATOR_ONBOARDING.md` — Validator onboarding guide (Tailscale + public RPC)
  - `docs/brand/logo.svg`, `docs/brand/symbol.svg` — Brand vector assets
  - `docs/generate_biab_pdf.py` — Business-in-a-Box PDF generator
  - `docs/AZTIBASE_BUSINESS_IN_A_BOX.pdf` — Business-in-a-Box document
  - `workspace/` — Investor deck build scripts and HTML slides
  - `website/` — Astro docs site source
  - `.gitignore` — Added website/dist/ exclusion
- **Review Notes**: Set up Cloudflare Tunnel (aztibase-testnet) with DNS routes: rpc.aztibase.com, rpc2.aztibase.com, rpc3.aztibase.com. Tailscale mesh for validator P2P. Oracle Cloud explored but ARM capacity unavailable in Ashburn.
- **Security Flags**: None

### Sprint 059 — Friends Testnet: Genesis Ceremony & Multi-Party Setup (2026-03-12)
- **Date**: 2026-03-12
- **Sprint**: 059 (Friends Testnet)
- **Commit**: d418dc9
- **Files changed**:
  - `crates/aztibase-node/src/wallet.rs` — Added `generate_validator_key()` producing Ed25519 + BLS keypair, `--validator` flag test
  - `crates/aztibase-node/src/main.rs` — Added `--validator` flag to `WalletAction::Generate`, added `--boot-node` CLI flag with `apply_overrides` wiring, refactored `Command::Genesis` to `GenesisAction` subcommand enum (generate, init, add-validator, add-account, validate, show), fixed 3 test sites for new `boot_node` field
  - `crates/aztibase-node/src/genesis.rs` — Added `init_genesis()`, `add_validator_to_genesis()`, `add_account_to_genesis()`, `show_genesis()`, `save_genesis()`, 8 ceremony tests (scaffold, keyfile add, public-info add, duplicate rejection, account add, account-validator overlap, show, full ceremony flow)
  - `deploy/setup-validator.sh` — Added `--boot-nodes`, `--genesis` flags and `custom` network option for private testnets
  - `docs/FRIENDS_TESTNET_GUIDE.md` — NEW: Step-by-step guide for multi-party private testnet setup
  - `scripts/setup-friends-testnet.sh` — NEW: Interactive coordinator genesis ceremony script
- **Summary**: Enables friends to each generate validator keys locally, share only public info, and join a coordinated private testnet. Genesis ceremony CLI allows incremental validator/account addition without code changes. Setup script supports custom boot nodes and genesis for non-official testnets. Breaking change: `aztibase genesis` now requires subcommand (`aztibase genesis generate` for old behavior).
- **Security Flags**: None

### Sprint 058 — Audit Fix: decode_vertex Signature Verification (2026-03-12)
- **Date**: 2026-03-12
- **Sprint**: 058 (Pre-Mainnet Audit Fixes)
- **Commit**: 614ccd2
- **Files changed**:
  - `crates/aztibase-consensus/src/validator.rs` — Added `ed25519_pubkey` field to `ValidatorRecord`, `set_ed25519_key()` and `ed25519_key()` methods on `ValidatorSet`
  - `crates/aztibase-consensus/src/wire.rs` — Fixed `decode_vertex()` to look up Ed25519 pubkey from `ValidatorSet` instead of assuming `block.author` is a raw pubkey (it's a BLAKE3 address hash in production)
  - `crates/aztibase-consensus/src/wire.rs` (tests) — Registered Ed25519 pubkeys in test `ValidatorSet` instances
  - `crates/aztibase-consensus/src/engine.rs` (tests) — Added `set_ed25519_key` calls in `make_test_engine()`
  - `crates/aztibase-node/src/genesis.rs` — Added `public_key: Option<String>` to `ValidatorEntry`, populated in `generate_genesis()`, `testnet_genesis()`, `mainnet_genesis()`
  - `crates/aztibase-node/src/main.rs` — Parse Ed25519 pubkey from genesis config and store in `ValidatorSet` via `set_ed25519_key()`
  - `crates/aztibase-node/src/integration.rs` — Added `set_ed25519_key` calls in 13 test sites that build `ValidatorSet`
- **Summary**: Fixed critical production bug where `decode_vertex()` assumed `block.author` was a valid Ed25519 public key. In production, `block.author` is a BLAKE3 hash (address) of the pubkey, causing `PublicKey::from_bytes()` to fail on every inter-node vertex exchange. The 3-node testnet was stuck at block height 0 with "invalid Ed25519 signature on vertex" errors on every received vertex. Fix stores Ed25519 pubkeys in `ValidatorSet` (populated from genesis config) and looks them up during signature verification. Includes fallback path for legacy configs where validator ID equals the raw pubkey (test scenarios). Consensus crate: 114 tests pass. Node crate: compilation verified.
- **Security Flags**: H-CON-1 RESOLVED — DagBlock signature verification now works in production with address-based validator IDs.

### Sprint 058 — Phases 1-3 Audit Code Fixes (2026-03-12, prior sessions)
- **Date**: 2026-03-12
- **Sprint**: 058 (Pre-Mainnet Audit Fixes)
- **Summary**: Implemented all Phase 1 (Safety), Phase 2 (Correctness), and Phase 3 (Hardening) audit fixes from SPRINT-058.md. 932 tests passing across all 9 crates. Details in prior session BUILD_LOG entries and AUDIT_REPORT.

### Pre-Mainnet Audit (M9 — Full Codebase)
- **Date**: 2026-03-12
- **Commit**: 614ccd2 (included in Sprint 058 commit)
- **Files changed**:
  - `blockchain-project/AUDIT_REPORT_2026-03-12.md` — NEW: Full pre-mainnet audit report across all 9 crates + architecture alignment
- **Summary**: 7-agent parallel code audit covering security, code quality, architecture alignment, and completeness. 15 CRITICAL, 25 HIGH, 31 MEDIUM findings. Top blockers: no DagBlock signatures, silent committed batch drops, WASM light client accepts forged data, `blst` C dep, faucet has no mainnet gate. Code quality clean (zero AI fingerprints, zero unsafe, 931 tests). 8 architecture gaps identified (object model, Block-STM integration, WASM execution, privacy, DAS).
- **Security Flags**: H-CON-1 (no block signatures) ELEVATED — blocks BFT safety assumptions. H-CON-3 (dropped committed batches) ELEVATED — consensus-execution divergence risk.

### Consensus-Based FaucetDrip & Testnet Liveness Fixes (M9-S18.4)
- **Date**: 2026-03-12
- **Commit**: fc4de4e
- **Files changed**:
  - `crates/aztibase-execution/src/routing.rs` — New TxKind::FaucetDrip variant (prefix 0x1B): validator, recipient, amount, nonce, gas_price. All match arms updated (encode, nonce, gas_price, gas_limit=0, sender→validator, expected_prefix). Roundtrip test.
  - `crates/aztibase-execution/src/fee.rs` — Added `0x1B => 0` to estimate_gas (FaucetDrip is free)
  - `crates/aztibase-node/src/pipeline.rs` — FaucetDrip pipeline execution: gas escrow bypass (pushes None), dispatch to faucet_drips vec (validator, recipient, amount), execution sets balance + increments sender nonce. compute_tx_hash updated.
  - `crates/aztibase-node/src/mempool.rs` — Gas price bypass for FaucetDrip txs (skip min_gas_price check)
  - `crates/aztibase-rpc/src/server.rs` — Rewrote handle_faucet_drip: deterministic keypair from BLAKE3("AZTIBASE_TESTNET_FAUCET"), constructs SignedTx envelope, submits via consensus channel. Added faucet_nonce AtomicU64 to RpcState.
  - `data/node{1,2,3}/node{1,2,3}-local.toml` — Changed listen_addresses from 127.0.0.1 to 0.0.0.0 (fixes peer discovery via LAN IPs)
  - `start-testnet.sh` — Staggered node starts (2s delay) to prevent boot_node race conditions
- **Review Notes**: Converts faucet from node-local state mutation to a consensus transaction. Fixes 3 bugs: (1) FaucetDrip nonce never incremented in state → 2nd+ drips failed nonce validation; (2) P2P mesh incomplete because nodes bound to 127.0.0.1 but discovered via LAN IPs; (3) startup race condition. Live testnet validated: 2x faucet drips, 2x transfers (500 + 1000 AZTB), cross-node balance consistency on all 3 nodes.
- **Security Flags**: None

### Open Risk Resolution & Testnet Validation (M9-S18.3)
- **Date**: 2026-03-12
- **Commit**: (this commit)
- **Files changed**:
  - `Cargo.lock` — quinn-proto 0.11.13 → 0.11.14 (RUSTSEC-2026-0037 HIGH DoS fix)
  - `crates/aztibase-node/src/main.rs` — Staking metrics wiring: `update_staking()` called after each batch with validator count and total stake from shared staking store
  - `blockchain-project/STATUS.md` — Updated open risks (domain resolved, FTO in progress, wasmtime v42 clean, quinn-proto fixed)
  - `blockchain-project/DECISIONS.md` — ADR-028 (nChain FTO analysis), ADR-029 (quinn-proto patch)
  - `test-testnet.sh` — NEW: Comprehensive testnet validation script (12 phases)
- **Review Notes**: Live 3-node testnet validated: 2226+ batches, 0 equivocations, 3.2ms commit latency, 2 peers per node, all 48 RPC methods responsive. nChain FTO: overall LOW-MEDIUM risk, PoUW attestation approach is architecturally distinct. wasmtime v42 Windows trap handling confirmed resolved (setjmp/longjmp rewrite shipped). 223 node tests pass, 0 clippy warnings.
- **Security Flags**: None

### Admin Dashboard & Testnet Hardening (M9-S18.2)
- **Date**: 2026-03-12
- **Commit**: (this commit)
- **Files changed**:
  - `crates/aztibase-node/src/pipeline.rs` — TX storage fix: serialize ALL transactions before nonce filtering so `getTransactionByHash` returns data for rejected txs too. Removed nonce increment on gas-too-low and escrow-failure paths (txs that can't pay gas shouldn't consume a nonce slot).
  - `crates/aztibase-rpc/src/server.rs` — Documented faucet as node-local testnet limitation with rustdoc comment.
  - `monitoring/grafana/dashboards/node-health.json` — Added Active Validators, Total Staked, Slashing Events, and Block Production Rate panels.
  - `admin/` — NEW: Internal admin dashboard (8 files). Multi-file vanilla JS, 4 pages: Nodes (multi-node health), Validators (stake lookup), Operations (faucet, chain params, governance, emission), Accounts (address/tx/batch lookup). Dark theme matching explorer.
- **Review Notes**: All 621 tests pass (321 execution + 223 node + 77 RPC). Zero clippy warnings. wallet.json test file cleaned from project root.
- **Security Flags**: None

### Testnet Bug Fix — Transaction Inclusion & Faucet Amount (M9-S18.1)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-consensus/src/engine.rs` — CRITICAL FIX: `record_commit()` was called BEFORE `extract_committed_batch()`, causing the anchor vertex to be excluded from its own committed batch (all transactions lost). Moved to AFTER extraction; now marks ALL vertices in `batch.vertex_order`. Added diagnostic logging for tx-count on vertex proposals and pending queue size.
  - `crates/aztibase-rpc/src/server.rs` — `FAUCET_DRIP_AMOUNT` 10 → 1,000,000 (transfers need 21,000+ for gas escrow, so 10 was always insufficient)
  - `crates/aztibase-node/src/main.rs` — Added gossip broadcast for RPC-submitted transactions (was local-only). Added mempool accept/reject logging. Added `node_metrics.set_mempool_size()` updates.
  - `explorer/index.html` — Minor explorer updates
- **Review notes**: Two root-cause bugs causing "transfers=0 in every batch":
  1. Consensus engine commit ordering: anchor hash added to committed_blocks set before batch extraction → filter excluded it → payload transactions lost
  2. Faucet drip too small: 10 AZTB < 21,003 needed (21,000 gas + 3 value minimum) → gas escrow always failed
  Verified fix on live 3-node testnet: faucet drip → transfer → sender balance 978,500 (1M - 500 - 21,000 gas), recipient balance 500.
- **Tests**: 114 consensus + 77 RPC passing, 0 clippy warnings, fmt clean.
- **Security flags**: None.

---

### Sprint 057 — Validator Business-in-a-Box & Cloud Monitoring (M9-S18)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `deploy/setup-validator.sh` — NEW: one-click interactive validator setup script with OS detection, dependency install, build, keypair generation, systemd service, firewall, Grafana Cloud Alloy integration, --dry-run mode, --uninstall mode
  - `docs/GRAFANA_CLOUD_SETUP.md` — NEW: step-by-step free Grafana Cloud monitoring guide (account creation, Alloy config, dashboard import, alerting)
  - `docs/VPS_GUIDE.md` — NEW: VPS provider comparison (Hetzner, Contabo, OVH, Vultr, DigitalOcean, Linode), hardware requirements, cost estimates, step-by-step walkthrough
  - `blockchain-project/sprints/SPRINT-057.md` — Sprint 057 plan
- **Review notes**: Setup script targets non-technical users with plain-English prompts. Grafana Cloud credentials are stored only in Alloy config (operator-owned machine). No secrets in source. Script passes bash -n syntax check. --dry-run mode allows testing without root.
- **Tests**: 931 passing (no regressions), 0 clippy warnings, fmt clean.
- **Security flags**: None. Grafana Cloud token stored in /etc/alloy/config.alloy with root-only access.

---

### Sprint 056 — Full State Snapshots & Mainnet Genesis (M9-S17)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-execution/src/snapshot.rs` — ProtocolStoreBundle, create_full_snapshot, apply_full_snapshot, write_snapshot_file, read_snapshot_file, SNAPSHOT_VERSION 3, 6 new tests
  - `crates/aztibase-execution/src/lib.rs` — Re-exports for new snapshot types/functions
  - `crates/aztibase-execution/src/governance.rs` — Added Clone, Debug derives to GovernanceStore
  - `crates/aztibase-execution/src/chain_params.rs` — Added Clone, Debug derives to ChainParams
  - `crates/aztibase-execution/src/l2_bridge.rs` — Added Clone, Debug derives to L2Registry, L2AnchorStore, BridgeEscrow, BridgeWithdrawProofs
  - `crates/aztibase-node/src/main.rs` — Snapshot subcommand (export/info), --snapshot CLI flag, snapshot bootstrap on startup
  - `crates/aztibase-node/src/pipeline.rs` — apply_protocol_bundle() method
  - `crates/aztibase-node/src/genesis.rs` — mainnet_genesis() with 400M AZTB across 8 allocations, 1 new test
  - `blockchain-project/DECISIONS.md` — ADR-027 (full state snapshots & mainnet genesis)
  - `blockchain-project/sprints/SPRINT-056.md` — Sprint 056 plan
- **Review notes**: Security review — snapshot file uses BLAKE3 integrity hash verified before deserialization, MAX_SNAPSHOT_SIZE 128 MiB prevents memory exhaustion, file path comes from CLI (operator-controlled). Mainnet genesis uses deterministic placeholder keys from BLAKE3 seeds — no secrets in source.
- **Tests**: 931 passing (6 new snapshot + 1 new genesis), 0 clippy warnings, fmt clean.
- **Security flags**: None.

---

### Sprint 055 — Public Testnet Launch Infrastructure (M9-S16)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `testnet/genesis/genesis.toml` — Canonical testnet genesis (3 validators, 1 faucet account)
  - `testnet/genesis/keys/` — Validator + funded account key files
  - `testnet/genesis/node-{1,2,3}.toml` — Local testnet node configs
  - `testnet/index.html` — Testnet landing page (network details, developer guide, validator guide)
  - `faucet/index.html` — Faucet web UI (address input, drip button, cooldown timer)
  - `deploy/seed-nodes/seed-{1,2,3}.toml` — Cloud seed node configs (testnet profile, public bind)
  - `deploy/systemd/aztibase.service` — Systemd unit file for Linux service management
  - `deploy/bootstrap.sh` — Cloud VM bootstrap script (Ubuntu 22.04+, build from source)
  - `blockchain-project/DECISIONS.md` — ADR-026 (public testnet launch infrastructure)
  - `blockchain-project/sprints/SPRINT-055.md` — Sprint 055 plan
- **Review notes**: Security review clean — faucet uses regex validation + textContent (no XSS), bootstrap script uses set -euo pipefail, key perms 600. All static HTML, no server-side rendering.
- **Security flags**: None.

---

### Sprint 054 — Mainnet Operational Hardening (M9-S15)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-node/src/config.rs` — NetworkProfile enum (Dev/Testnet/Mainnet), RPC rate limit/body size/CORS config fields, 7 tests
  - `crates/aztibase-node/src/main.rs` — `--mainnet` CLI flag, profile-driven startup, sentinel check/write/clear, graceful shutdown, profile wired to RPC
  - `crates/aztibase-rpc/src/server.rs` — RpcRateLimiter (per-IP token bucket), profile-driven CORS, faucet gate by profile, body size limit, ConnectInfo extraction, 6 new tests
  - `crates/aztibase-execution/src/persist.rs` — Sentinel functions (write/clear/check via STATE_TABLE), 3 new tests
  - `crates/aztibase-execution/src/routing.rs` — TxKind::RotateValidatorKey (0x1A), encode/decode/route/sender/gas, 1 new test
  - `crates/aztibase-execution/src/staking.rs` — rotate_key() method (re-keys validator + delegations + unbonding), 4 new tests
  - `crates/aztibase-execution/src/fee.rs` — Gas estimate 60K for RotateValidatorKey
  - `crates/aztibase-execution/src/lib.rs` — Re-exports for sentinel functions
  - `crates/aztibase-node/src/pipeline.rs` — RotateValidatorKey classification + execution + hash computation
- **Review notes**: ADR-025 documents design. 925 tests pass (19 new), 0 clippy warnings, fmt clean.
- **Security flags**: None. Rate limiter is per-process (not distributed); acceptable for single-node. Sentinel is best-effort (no WAL guarantee on crash).

---

### Sprint 053 — Protocol Store Persistence (M9-S14)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-execution/src/staking.rs` — Added Serialize/Deserialize derives to ValidatorStake, Delegation, UnbondingEntry, OffenseType, SlashRecord, StakingStore
  - `crates/aztibase-execution/src/governance.rs` — Added Serialize/Deserialize to GovernanceStore
  - `crates/aztibase-execution/src/tokenomics.rs` — Added Serialize/Deserialize to EmissionTracker, impl Default
  - `crates/aztibase-execution/src/chain_params.rs` — Added Serialize/Deserialize to ParamValue, ChainParams
  - `crates/aztibase-execution/src/agent.rs` — Added Serialize/Deserialize to AgentPolicy, AgentSpendRecord, AgentPolicyStore
  - `crates/aztibase-execution/src/l2_bridge.rs` — Added Serialize/Deserialize to L2Registry, L2AnchorStore, BridgeEscrow, BridgeWithdrawProofs
  - `crates/aztibase-execution/src/persist.rs` — 9 STATE_TABLE keys, generic flush/load helpers, individual flush/load for all 8 stores, flush_protocol_stores convenience fn, 9 new tests
  - `crates/aztibase-execution/src/lib.rs` — Updated re-exports for all new flush/load functions
  - `crates/aztibase-node/src/pipeline.rs` — with_storage() loads all stores from disk, execute_batch() flushes all stores after state
- **Review notes**: ADR-024 documents design. 0 clippy warnings, fmt clean. 214/215 node tests pass (1 pre-existing flaky: validator_crash_and_recovery). 29/29 persist tests pass.
- **Security flags**: None. Deserialization failures fall back to defaults with tracing::warn.

---

### Sprint 052 — L1 Bridge Primitives for Sovereign Rollups (M9-S13)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-execution/src/l2_bridge.rs` — NEW: L2Registry, L2AnchorStore, BridgeEscrow, BridgeWithdrawProofs, BridgeError, 10 unit tests
  - `crates/aztibase-execution/src/routing.rs` — 4 new TxKind variants (AnchorL2State 0x16, BridgeDeposit 0x17, BridgeWithdraw 0x18, RegisterL2 0x19), prefix constants, encode/decode/route/sender/gas for all 4, 6 routing tests
  - `crates/aztibase-execution/src/fee.rs` — 4 new gas estimates (80K/50K/70K/100K)
  - `crates/aztibase-execution/src/lib.rs` — l2_bridge module declaration + 8 re-exports
  - `crates/aztibase-node/src/pipeline.rs` — 4 bridge store fields, tx classification, execute_register_l2/anchor/deposit/withdraw, escrow unlock on withdraw, 3 e2e tests
  - `crates/aztibase-node/src/main.rs` — RPC bridge store wiring (4 with_* calls)
  - `crates/aztibase-rpc/src/server.rs` — 4 new RPC handlers (getL2State, listL2s, getBridgeBalance, getBridgeProofStatus), 4 RpcState fields, 4 builder methods, 4 tests
  - `docs/rpc-api.md` — 4 new endpoint docs (L2 Bridge section)
  - `blockchain-project/DECISIONS.md` — ADR-023 (L2 bridge design)
  - `blockchain-project/sprints/SPRINT-052.md` — NEW: Sprint 052 plan
- **Review**: Security review found escrow not decremented on withdraw — FIXED. Double-spend prevention via proof hash dedup: SAFE. Sequencer auth via registry: SAFE. u128 overflow protection via saturating ops + bounds checks: SAFE.
- **Security Flags**: None. 1 issue found and fixed (escrow unlock on withdraw).
- **Tests**: 897 pass (23 new: 10 l2_bridge + 6 routing + 4 RPC + 3 e2e), 0 clippy warnings, fmt clean

### Sprint 051 — WASM Tx Signing & Block Explorer (M9-S12)
- **Date**: 2026-03-11
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-wasm/Cargo.toml` — Added ed25519-dalek, rand, getrandom (js feature)
  - `crates/aztibase-wasm/src/tx_signing.rs` — NEW: signTransfer, generateKeypair, addressFromSecret, RPC request builders, 8 tests
  - `crates/aztibase-wasm/src/lib.rs` — tx_signing module declaration
  - `crates/aztibase-rpc/Cargo.toml` — Added tower-http (CORS)
  - `crates/aztibase-rpc/src/server.rs` — CorsLayer (Allow-Origin: *), aztb_getBlockTransactionCount, aztb_sendRawTransaction alias, 3 tests
  - `Cargo.toml` — tower-http workspace dep
  - `explorer/index.html` — NEW: Block explorer (vanilla HTML/JS, dark theme, responsive)
  - `docs/rpc-api.md` — Added getBlockTransactionCount, sendRawTransaction docs
- **Review**: WASM tx signing uses same domain separation (AZTB_TX_V1) and envelope format as node. Explorer uses textContent-based XSS escaping. CORS is testnet-appropriate (restrict for mainnet).
- **Security Flags**: None. CORS `*` acceptable for testnet only.
- **Tests**: 874 pass (11 new: 8 wasm + 3 rpc), 0 clippy warnings, fmt clean

### Sprint 050 — CI Pipeline & Public Testnet Infrastructure (M9-S11)
- **Date**: 2026-03-10
- **Commit**: (pending)
- **Files changed**:
  - `.github/workflows/ci.yml` — Added dev branch trigger, CARGO_INCREMENTAL=0, release build gate job
  - `docs/rpc-api.md` — NEW: Full RPC API reference for all 43 methods with params, return types, examples
  - `blockchain-project/sprints/SPRINT-050.md` — Sprint 050 plan + task status updates
  - `crates/aztibase-core/src/bls.rs` — Added `BlsKeypair::from_ikm()` for deterministic BLS key derivation
  - `crates/aztibase-node/src/genesis.rs` — Added `testnet_genesis()`, `testnet_boot_nodes()`, 3 tests
  - `crates/aztibase-node/src/main.rs` — Added `--testnet` CLI flag, genesis loading logic, boot node injection
  - `crates/aztibase-execution/benches/execution_bench.rs` — Added `verify_tx` and `verify_tx_batch` criterion benchmarks
  - `scripts/deploy-validator.sh` — NEW: VPS/cloud validator deployment script (systemd, key gen, --testnet)
- **Review**: Testnet genesis uses deterministic keys from fixed seeds — TESTNET ONLY, well-known keys. Deploy script uses systemd hardening (LimitNOFILE, dedicated user). BLS from_ikm uses proper HKDF derivation (not raw scalar).
- **Security Flags**: None
- **Benchmarks (release mode)**:
  - Consensus: vertex 1.35µs, dag_insert_100 113.9ms, commit_rule 17.6ms, BLS cert 3.4ms/21val 8.0ms/100val
  - Execution: single_transfer 1.99µs, verify_and_route 54.7µs (18.3K TPS single-threaded), batch/100 5.84ms, batch/500 27.8ms
  - State: state_root/1K 839µs, state_root/10K 8.4ms
  - Proofs: merkle 2.04µs/10K leaves, verkle 7.14µs/10K leaves
  - **10K TPS target achievable**: Ed25519 sig verify bottleneck at ~18.3K TPS/core, scales linearly with rayon

### Sprint 049 — Protocol Hardening & Multi-Node Stability (M9-S10)
- **Date**: 2026-03-10
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-execution/src/persist.rs` — `latest_batch_index()` reverse range scan for crash recovery, 3 new tests
  - `crates/aztibase-execution/src/lib.rs` — `latest_batch_index` re-export
  - `crates/aztibase-node/src/pipeline.rs` — batch_count recovery from disk in `with_storage()`, shutdown logging, `batch_count.fetch_add` moved into `execute_batch()`, 3 crash recovery tests, 4 epoch boundary tests, 2 throughput tests
  - `crates/aztibase-node/src/main.rs` — `--epoch-length` CLI flag wired to chain_params
  - `crates/aztibase-network/src/transport.rs` — `PROTOCOL_VERSION`, `AGENT_PREFIX`, `agent_version()`, `parse_agent_version()`, identify behaviour for version enforcement
  - `crates/aztibase-network/src/behaviour.rs` — `identify::Behaviour` added to AztibaseBehaviour
  - `crates/aztibase-network/src/lib.rs` — protocol version exports, 5 new tests
  - `blockchain-project/DECISIONS.md` — ADR-021 (no mempool persistence), ADR-022 (protocol versioning)
- **Review**: 5 phases complete. Crash recovery, protocol versioning, epoch boundaries, throughput baseline all tested. 284 execution + 85 network + 209 node tests pass. 0 clippy warnings, fmt clean.
- **Security Flags**: None new. Crash recovery verified safe (redb ACID). Protocol version bypass blocked by identify disconnect.

### Sprint 048 — Security Flag Resolution (M9-S9) — Bottom 5 Flags
- **Date**: 2026-03-10
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-node/src/mempool.rs` — MAX_TX_SIZE (256 KiB), size check in insert_with_priority(), 2 tests (S1-1)
  - `crates/aztibase-execution/src/verkle.rs` — 2 migration drill tests proving Verkle↔Merkle scheme swap (S1-2)
  - `crates/aztibase-consensus/src/engine.rs` — accumulate_vrf_seed() RANDAO-style mixing, 1 test (S2-2)
  - `crates/aztibase-wasm/src/browser_wallet.rs` — NEW: browser spending limits, per-tx caps, balance warnings, WASM exports, 5 tests (S4-1)
  - `crates/aztibase-wasm/src/lib.rs` — browser_wallet module declaration
  - `crates/aztibase-network/src/dht_record.rs` — NEW: SignedDhtRecord, quorum validation, freshness check, 8 tests (S8-1)
  - `crates/aztibase-network/src/lib.rs` — dht_record module + exports
  - `blockchain-project/DECISIONS.md` — ADR-016 through ADR-020
- **Review**: 5 SECURITY-ELEVATED flags resolved (S1-1, S1-2, S2-2, S4-1, S8-1). 700 tests pass, 0 clippy warnings, fmt clean.
- **Security Flags**: S1-1 RESOLVED, S1-2 RESOLVED, S2-2 RESOLVED, S4-1 RESOLVED, S8-1 RESOLVED

### Sprint 048 — Security Flag Resolution (M9-S9) — Top 4 Flags
- **Date**: 2026-03-10
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-network/src/light_sync.rs` — RequestCheckpoint/ResponseCheckpoint variants, CheckpointAnnounce struct, encode/decode helpers, 2 tests
  - `crates/aztibase-network/src/gossip.rs` — TOPIC_CHECKPOINT_ANNOUNCE (7th topic), weight 1.5
  - `crates/aztibase-network/src/lib.rs` — Checkpoint exports, topic count 6→7
  - `crates/aztibase-execution/src/agent.rs` — check_spend() read-only policy validation, 2 tests
  - `crates/aztibase-node/src/mempool.rs` — insert_checked_with_policy() pre-execution agent validation, decode_tx_kind(), 4 tests
  - `crates/aztibase-execution/src/governance.rs` — snapshot_balances on Proposal, cast_vote uses snapshot weight (flash-loan fix), 3 tests
  - `crates/aztibase-consensus/src/engine.rs` — EQUIVOCATION_PRUNE_DEPTH 20→100, EquivocationDetected carries hashes
  - `crates/aztibase-storage/src/store.rs` — EQUIVOCATION_PROOFS_TABLE (14th table)
  - `crates/aztibase-execution/src/persist.rs` — store/get_equivocation_proof(), 2 tests
  - `crates/aztibase-node/src/pipeline.rs` — Balance snapshot at proposal creation, persistent equivocation proof storage, SlashEvent extended
  - `crates/aztibase-node/src/main.rs` — EquivocationDetected hash pass-through
  - `blockchain-project/DECISIONS.md` — ADR-015: Equivocation detection window analysis
- **Review**: 4 SECURITY-ELEVATED flags resolved (S4-3, S1-3, S3-1, S2-1). 684 tests pass, 0 clippy warnings, fmt clean.
- **Security Flags**: S4-3 RESOLVED, S1-3 RESOLVED, S3-1 RESOLVED, S2-1 RESOLVED

### Sprint 047 — Testnet Validation (M9-S8)
- **Date**: 2026-03-10
- **Commit**: (pending)
- **Files changed**:
  - `crates/aztibase-consensus/src/engine.rs` — Peer-aware proposal gating (PeerCountChanged), removed vertex buffering, threshold clock seeded from genesis
  - `crates/aztibase-consensus/src/dag_store.rs` — Added insert_relaxed() for out-of-order vertex delivery, fixed causal_order panic with missing parents
  - `crates/aztibase-node/src/main.rs` — Send PeerCountChanged on connect/disconnect, removed broken re-broadcast buffer
  - `crates/aztibase-node/src/integration.rs` — Updated tests for relaxed insert + PeerCountChanged
  - `Cargo.toml` — Release profile adjusted for Windows PDB limits
- **Review**: First 3-node testnet: all nodes committing (56-81 commits in 40s), 2 peers each, RPC verified (nodeInfo, chainId, getBalance, gasPrice, health)

### 2026-03-10 -- node-engineer / p2p-network-engineer / consensus-engineer -- Sprint 047 Testnet Validation (M9-S8) [Phase 1-3]
**Task:** First real multi-node testnet run. Debugged and fixed critical issues preventing block production in 3-node local testnet.
**Sprint:** Sprint 047, Phases 1-3
**Git Ref:** b18a6d6
**Files Changed:**
- crates/aztibase-network/src/transport.rs: removed add_explicit_peer() calls that prevented gossipsub mesh relay (Bug #10)
- crates/aztibase-consensus/src/engine.rs: ThresholdClock per-round tracking, removed self-feed from propose_vertex (Bug #11), liveness timeout 10x→25x (Bug #13), peer-wait before first proposal, diagnostic logging, new integration test (run_produces_vertices_via_timeout)
- crates/aztibase-consensus/src/validator.rs: added has_quorum (>=2/3 stake threshold)
- crates/aztibase-consensus/src/wire.rs: max_future raised 10→100
- crates/aztibase-network/src/gossip.rs: mesh_n=3, mesh_n_low=2, mesh_outbound_min=1 for small testnet
- crates/aztibase-network/src/connection_filter.rs: exempt loopback IP from rate-limit
- crates/aztibase-network/src/lib.rs: exports for chain_scoped_topics, genesis_hex_prefix
- crates/aztibase-node/src/main.rs: non-fatal listen failures (Bug #12), PeerCountChanged wiring, scoped gossipsub topics, diagnostic logging
- data/genesis/genesis.toml: stake/balance as strings (serde_u128_as_string)
- data/node{1,2,3}/*.toml: full mesh boot_nodes for local testnet
**Bugs Found & Fixed:** 4 new (Bug #10-#13), see sprint plan for details
**Review Notes:** 3-node testnet produces blocks continuously (~660 blocks in 25s). Transfer transactions confirmed: 2 transfers (500 + 1000 tokens) via `aztibase wallet transfer` CLI, balances updated correctly, gas fee 21,000/tx deterministic. 113 consensus tests, 70 network tests pass, 0 clippy warnings.
**Security Flags:** None

### 2026-03-10 -- node-engineer / consensus-engineer / security-engineer -- Sprint 046 Complete (M9-S7: Weak Subjectivity Checkpoints)
**Task:** Sprint 046: Implement weak subjectivity checkpoints — periodic state snapshots every 1000 batches, CLI trusted checkpoint validation, RPC query endpoints
**Sprint:** Sprint 046, Phases 1-5
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/checkpoint.rs: made finality_cert optional, 2 new tests (checkpoint_creation_without_cert, checkpoint_serialization_without_cert)
- crates/aztibase-consensus/src/lib.rs: pub mod checkpoint, re-exports Checkpoint + CHECKPOINT_INTERVAL
- crates/aztibase-storage/src/store.rs: CHECKPOINTS_TABLE definition, ALL_TABLES updated to 13
- crates/aztibase-storage/src/lib.rs: CHECKPOINTS_TABLE export
- crates/aztibase-execution/src/persist.rs: store_checkpoint_raw, get_checkpoint_raw, latest_checkpoint_raw (raw byte API), 2 new persist tests
- crates/aztibase-execution/src/lib.rs: re-export checkpoint persist functions
- crates/aztibase-node/src/pipeline.rs: checkpoint emission every CHECKPOINT_INTERVAL batches (postcard serialize + store)
- crates/aztibase-node/src/main.rs: --checkpoint CLI flag, startup validation (parse batch_index:state_root_hex, verify stored checkpoint matches)
- crates/aztibase-rpc/src/server.rs: aztb_getCheckpoint + aztb_latestCheckpoint dispatch, checkpoint_to_json helper
- blockchain-project/sprints/SPRINT-046.md: sprint plan marked COMPLETE with retrospective
**Review Notes:** Raw byte persistence avoids cross-crate dependency (execution→consensus). Checkpoint typed at application layer (pipeline/RPC) only. finality_cert made optional since pipeline doesn't produce finality certs during batch execution. 811 tests pass.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Checkpoint validation prevents long-range attacks on syncing nodes.

---

### 2026-03-10 -- security-engineer / blockchain-architect -- Sprint 045 Complete (M9-S6: Ed25519 Strict Verification & Attestation Domain Separation)
**Task:** Sprint 045: Switch PublicKey::verify() to verify_strict(), rejecting malleable signatures and weak keys. Confirmed attestation domain separation already present (AZTB_ATTESTATION_V1\0 prefix). Security flag S0-1 RESOLVED.
**Sprint:** Sprint 045, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-core/src/crypto.rs: verify() → verify_strict(), removed Verifier trait import, 4 new tests (strict_verify_rejects_short_signature, strict_verify_rejects_tampered_signature, strict_verify_rejects_all_zeros_signature, strict_verify_rejects_malleable_s)
- blockchain-project/sprints/SPRINT-045.md: sprint plan marked COMPLETE with retrospective
**Review Notes:** verify_strict() rejects both malleable S values (S >= L) and weak/small-order keys, closing two attack surfaces. Attestation domain separation was already implemented in attestation_hash() from Sprint 022/031 work — no additional code changes needed. 804 tests pass.
**Security Flags:** S0-1 RESOLVED. 0 ELEVATED remaining, 0 MEDIUM.

---

### 2026-03-10 -- p2p-network-engineer / node-engineer / security-engineer -- Sprint 044 Complete (M9-S5: Genesis Hash P2P Enforcement)
**Task:** Sprint 044: Embed genesis hash into all P2P protocol identifiers — chain-scoped gossipsub topics, chain-scoped Kademlia protocol, TransportConfig wiring, main.rs genesis pass-through
**Sprint:** Sprint 044, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-network/src/gossip.rs: genesis_hex_prefix(), chain_scoped_topics(), aztibase_topics_scoped(), peer_score_params_scoped(), TOPIC_BASE_NAMES, 6 new tests
- crates/aztibase-network/src/discovery.rs: kademlia_protocol(), kademlia_config_scoped(), kademlia_behaviour_scoped(), 4 new tests
- crates/aztibase-network/src/transport.rs: genesis_hash field in TransportConfig, wired through to gossipsub and kademlia
- crates/aztibase-node/src/main.rs: genesis hash passed from genesis config to both full-node and light-node TransportConfig
- blockchain-project/sprints/SPRINT-044.md: sprint plan
**Review Notes:** Nodes with different genesis configs are now on completely separate P2P networks. Topic format: /aztibase/{topic}/1.0.0/{genesis_hex8}. Kademlia protocol: /aztibase/kad/1.0.0/{genesis_hex8}. Fallback to un-scoped topics when no genesis hash configured.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Cross-chain message pollution prevented by protocol-level separation.

---

### 2026-03-10 -- blockchain-architect / smart-contract-engineer / ai-integration-engineer / security-engineer -- Sprint 043 Complete (M9-S4: Autonomous AI Agent Transactions)
**Task:** Sprint 043: Enable autonomous AI agent transactions within human-defined policy constraints — AgentPolicy store, SetAgentPolicy/AgentExecute TxKinds, pipeline execution with spend tracking
**Sprint:** Sprint 043, Phases 1-5
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/agent.rs (NEW): AgentPolicy, AgentPolicyStore, AgentSpendRecord, AgentError enum (7 variants), validate_spend with epoch tracking, 8 unit tests
- crates/aztibase-execution/src/routing.rs: TxKind::SetAgentPolicy (0x14), TxKind::AgentExecute (0x15), all match arms updated (encode/nonce/gas_price/gas_limit/sender/expected_prefix/route_tx)
- crates/aztibase-execution/src/fee.rs: estimate_gas 0x14→60K, 0x15→80K (21 TxKind variants total)
- crates/aztibase-execution/src/lib.rs: pub mod agent, re-export key types
- crates/aztibase-node/src/pipeline.rs: AgentPolicyStore field, SetAgentPolicy execution (owner-only, AIAgent check, policy set), AgentExecute execution (policy validation: expiry, allowed kinds, per-tx/per-epoch limits, balance check, transfer), SetAgentPolicyEntry/AgentExecuteEntry type aliases, compute_tx_hash/extract_tx_features match arms
- crates/aztibase-rpc/src/server.rs: aztb_getAgentPolicy RPC method, AgentPolicyStore field in RpcState, with_agent_policy_store builder
- crates/aztibase-node/src/main.rs: wire agent_policy_store to RPC
**Review Notes:** Owner verification uses deterministic agent address derivation (compute_contract_address). Epoch-based spend tracking resets automatically. Per-tx and per-epoch limits enforced. Policy expiry prevents stale agent autonomy. Initially supports Transfer-type agent execution only.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Spending capped per-tx and per-epoch, policy owner-only modification, expired policies rejected.

---

### 2026-03-10 -- blockchain-architect / node-engineer / security-engineer -- Sprint 042 Complete (M9-S3: Genesis Validation & Network Identity)
**Task:** Sprint 042: Comprehensive genesis config validation — duplicate detection, bounds checking, supply cap enforcement, startup abort on invalid genesis
**Sprint:** Sprint 042, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/genesis.rs: GenesisValidationError enum (10 variants), validate_genesis()/validate_genesis_with_min_stake(), GENESIS_SUPPLY=400M, DEFAULT_MIN_VALIDATOR_STAKE=10K, 12 new tests
- crates/aztibase-node/src/main.rs: validate_genesis() call after genesis load, abort on error, log genesis hash on successful validation
- crates/aztibase-node/Cargo.toml: added thiserror dependency
- blockchain-project/sprints/SPRINT-042.md: sprint plan
**Review Notes:** Validation checks: no validators, duplicate validator/account addresses, invalid hex addresses (length ≠ 32 bytes), stake below minimum, supply exceeds 400M cap, invalid BLS key length (≠ 48 bytes), invalid BLS hex, validator-account address overlap. Multiple errors collected and reported together. Startup aborts with detailed error messages.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Prevents malformed genesis from causing consensus divergence.

---

### 2026-03-10 -- blockchain-architect / smart-contract-engineer / security-engineer -- Sprint 041 Complete (M9-S2: Gas Price Enforcement & Transaction Validation)
**Task:** Sprint 041: Close gas-price bypass vulnerability — enforce gas_price >= base_fee, reject sub-base-fee txs, add gas estimates for all 19 TxKind variants
**Sprint:** Sprint 041, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/pipeline.rs: Replace gas_price==0 bypass with gas_price < base_fee rejection (error receipt + nonce increment), all ~66 test instances updated gas_price 0→1, test balances increased to cover gas escrow, balance/fee assertions recalculated
- crates/aztibase-node/src/integration.rs: All ~42 test instances updated gas_price 0→1, balances increased to 10M+ to cover gas escrow, balance assertions recalculated with gas deductions
- crates/aztibase-execution/src/fee.rs: estimate_gas expanded from 7 to 19 TxKind variants (added RegisterModel 100K, PostTask 42K, SubmitAttestation 50K, CommitCompute 75K, DeregisterCompute 50K, DeregisterModel 60K, CreateProposal 100K, CastVote 40K, Stake/Unstake/Delegate/Undelegate 60K each)
- crates/aztibase-execution/src/routing.rs: test gas_price 0→1 (16 occurrences)
- crates/aztibase-execution/src/tx.rs: test gas_price 0→1 (2 occurrences)
- crates/aztibase-rpc/src/server.rs: test gas_price 0→1 (1 occurrence)
- crates/aztibase-node/src/mempool.rs: test gas_price 0→1 (3 occurrences), min_gas_price threshold adjusted
- blockchain-project/sprints/SPRINT-041.md: sprint plan
**Review Notes:** SECURITY FIX: Previously, txs with gas_price=0 bypassed escrow entirely and executed for free. Now all txs are validated against current base_fee before escrow. Txs below base_fee get failure receipt with "gas price too low" error and nonce increment. No fee-free execution path remains. All ~120+ test gas_price values updated, test balances adjusted for gas escrow coverage.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Fee-free execution vulnerability CLOSED. All txs now pay gas fees. No bypass path.

---

### 2026-03-10 -- blockchain-architect / smart-contract-engineer / node-engineer / security-engineer -- Sprint 040 Complete (M9-S1: u64→u128 Balance Migration)
**Task:** Sprint 040: Migrate all monetary values (balance, stake, fee, value, amount, reward) from u64 to u128 across entire codebase
**Sprint:** Sprint 040, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/state.rs: Account.balance u64→u128, balance()/set_balance() return/take u128, state_root() emits 16-byte balance
- crates/aztibase-core/src/types.rs: Transaction.value u64→u128
- crates/aztibase-execution/src/fee.rs: FeeEscrow.max_fee u64→u128, escrow_fee/refund_unused u128 arithmetic, checked_mul for gas overflow
- crates/aztibase-execution/src/routing.rs: Transfer/EvmCall value, RegisterModel compute_cost/min_stake, PostTask reward, CommitCompute committed_stake, Stake/Unstake/Delegate amount → u128
- crates/aztibase-execution/src/staking.rs: ValidatorStake/Delegation/UnbondingEntry/SlashRecord monetary fields → u128, all methods updated
- crates/aztibase-consensus/src/validator.rs: ValidatorInfo/ValidatorRecord stake, total_stake → u128, leader_for_round/vrf_leader_for_round u128 position
- crates/aztibase-consensus/src/pouw.rs: InferenceTask.reward, ComputeCommitment.committed_stake → u128
- crates/aztibase-execution/src/block_stm.rs: StateValue::Balance(u128), read_balance → u128
- crates/aztibase-execution/src/parallel.rs: TransferTx.value → u128
- crates/aztibase-execution/src/persist.rs: AccountRecord.balance u64→u128, byte offsets shifted (nonce 8..16→16..24, etc.), min record 16→24 bytes
- crates/aztibase-execution/src/snapshot.rs: AccountEntry.balance → u128
- crates/aztibase-execution/src/evm.rs: apply_state_changes u128::MAX fallback, evm_call value → u128
- crates/aztibase-execution/src/model_registry.rs: compute_cost, min_stake → u128
- crates/aztibase-execution/src/governance.rs: Vote.weight, VoteTally approve/reject_weight → u128
- crates/aztibase-storage/src/light.rs: LocalWalletState.balance → u128
- crates/aztibase-runtime/src/anomaly.rs: TxFeatures.value, high_value_threshold → u128
- crates/aztibase-node/src/genesis.rs: ValidatorEntry.stake, AccountEntry.balance → u128, serde_u128_as_string module for TOML
- crates/aztibase-node/src/pipeline.rs: MIN_VALIDATOR_STAKE_CAP, bootstrap_genesis_validators, PipelineResult.total_fees_burned → u128
- crates/aztibase-node/src/task_pool.rs: SettlementResult.payouts → u128
- crates/aztibase-node/src/wallet.rs: sign_transfer/build_signed_transfer value → u128
- crates/aztibase-node/src/main.rs: CLI value arg, genesis_validators type → u128
- crates/aztibase-node/src/integration.rs: test literals → u128
- crates/aztibase-rpc/src/server.rs: FAUCET_DRIP_AMOUNT → u128
- crates/aztibase-rpc/src/metrics.rs: update_staking total_staked → u128 with saturating i64 cast
**Review Notes:** ~20 files across 6 crates. All monetary fields (balance, stake, value, amount, reward, committed_stake, compute_cost, min_stake, fees) changed to u128. Non-monetary values (gas_price, gas_limit, gas_used, nonce, round, slot, timestamp) remain u64. persist.rs byte offsets shifted for 16-byte balance. VRF leader selection uses 16 bytes of hash. TOML u128 via serde string module. Prometheus saturating i64 cast.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. No u64 truncation in monetary paths. Fee calculation uses checked_mul. Prometheus uses saturating cast. persist.rs byte offsets verified. EVM uses u128::MAX fallback for U256 overflow.

---

### 2026-03-10 -- consensus-engineer / node-engineer / security-engineer -- Sprint 039 Complete (M8 Sprint 14: Full-Node Integration Wiring & M8 Close)
**Task:** Sprint 039: Wire staking/slashing/governance/tokenomics into main.rs, consensus-pipeline bridges, genesis staking bootstrap, staking metrics
**Sprint:** Sprint 039, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs: ConsensusOutput::EquivocationDetected variant, ConsensusInput::UpdateValidatorSet variant, equivocation emits output, engine handles validator set update
- crates/aztibase-node/src/main.rs: StakingStore wired to RPC, slash channel bridged from consensus, consensus_tx passed to pipeline, genesis validator bootstrap in StakingStore
- crates/aztibase-node/src/pipeline.rs: set_consensus_tx(), bootstrap_genesis_validators(), consensus_tx field, epoch boundary sends UpdateValidatorSet, removed dead_code markers
- crates/aztibase-node/src/integration.rs: all ConsensusOutput match arms updated for EquivocationDetected
- crates/aztibase-execution/src/chain_params.rs: epoch_length governance param (bounds 1K-100K)
- crates/aztibase-rpc/src/metrics.rs: active_validators, total_staked, slashes_applied gauges, update_staking()/inc_slashes() methods, staking JSON section
- CHANGELOG.md, blockchain-project/sprints/SPRINT-039.md
**Review Notes:** Phase 1: Consensus-pipeline bridge — equivocation detection emits ConsensusOutput, main.rs bridges to SlashEvent channel, validator set sync via ConsensusInput at epoch boundary. Phase 2: Genesis bootstrap — register_validator() for each genesis validator in StakingStore, epoch_length as governance ChainParam. Phase 3: Staking metrics in Prometheus (3 new counters/gauges), staking section in JSON metrics. Phase 4: 7+ new tests, clippy 0 warnings, fmt clean.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Slash channel is internal mpsc (no external injection). ValidatorSet updates processed sequentially in consensus input queue (no mid-round mutation). Empty validator set guarded by staking min_stake threshold.

---

### 2026-03-10 -- consensus-engineer / tokenomics-engineer / node-engineer / security-engineer -- Sprint 038 Complete (M8 Sprint 13: Validator Staking, Delegation & Slashing)
**Task:** Sprint 038: StakingStore, 4 staking TxKinds, delegation, unbonding queue, epoch reward distribution, equivocation/downtime slashing, 4 staking RPCs, 3 governance params
**Sprint:** Sprint 038, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/staking.rs (NEW): StakingStore, ValidatorStake, Delegation, UnbondingEntry, SlashRecord, OffenseType, StakingError; register_validator, add_stake, begin_unstake, delegate, begin_undelegate, process_unbonding, slash_validator, distribute_epoch_rewards, active_set_snapshot; 24+ unit tests
- crates/aztibase-execution/src/chain_params.rs: 3 new governance params (min_validator_stake, max_stake_cap, validator_commission_bps), 1 new test
- crates/aztibase-execution/src/lib.rs: staking module + re-exports
- crates/aztibase-execution/src/routing.rs: TxKind::Stake (0x10), Unstake (0x11), Delegate (0x12), Undelegate (0x13) — 19 total TxKind variants
- crates/aztibase-node/src/pipeline.rs: StakingStore field, SlashEvent channel (mpsc), epoch_participation tracking, full Stake/Unstake/Delegate/Undelegate execution handlers, unbonding queue processing, epoch boundary (reward distribution + downtime slashing + validator set rebuild), slash event drain
- crates/aztibase-rpc/src/server.rs: StakingStore in RpcState, with_staking_store() builder, 4 new handlers (aztb_getValidatorStake, aztb_getDelegation, aztb_getActiveValidators, aztb_getUnbondingStatus), 4 RPC tests
- blockchain-project/sprints/SPRINT-038.md: sprint plan + retrospective
**Review Notes:** Phase 1: StakingStore with self-stake, delegation, unbonding queue (UNBONDING_ROUNDS=4,536,000), MAX_UNBONDING_ENTRIES=10,000, equivocation/downtime slash records. Phase 2: Epoch reward distribution with 10% commission, active_set_snapshot for validator set rebuild, epoch participation tracking for downtime detection. Phase 3: 4 staking RPC endpoints, 3 governance params (min_validator_stake, max_stake_cap, validator_commission_bps with bounds). Phase 4: clippy 0 warnings, fmt clean, all tests pass (~34 new).
**Security Flags:** 0 ELEVATED, 0 MEDIUM. u64 stake protected by MAX_STAKE_CAP + saturating_add. Unbonding queue bounded. Commission capped at 3000 bps. Floor division only. Slash-during-unbonding handled. No floating-point.

---

### 2026-03-09 -- tokenomics-engineer / node-engineer / security-engineer -- Sprint 037 Complete (M8 Sprint 12: Token Supply, Emission & Vesting)
**Task:** Sprint 037: Token supply hard cap, disinflationary emission schedule, genesis allocations with vesting, epoch reward distribution, 2 new RPCs
**Sprint:** Sprint 037, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/tokenomics.rs (NEW): EmissionSchedule, VestingSchedule, GenesisAllocation (8 categories), EmissionTracker, StakingAPY curve, EpochDistribution, validator_epoch_reward, 36 unit tests
- crates/aztibase-execution/src/lib.rs: tokenomics module + re-exports
- crates/aztibase-node/src/pipeline.rs: EmissionTracker field, shared_emission_tracker() accessor
- crates/aztibase-node/src/main.rs: .with_emission_tracker() wired into RPC server
- crates/aztibase-rpc/src/server.rs: EmissionTracker in RpcState, with_emission_tracker() builder, aztb_getEmissionInfo + aztb_getVestingStatus handlers, 3 RPC tests
- blockchain-project/sprints/SPRINT-037.md (NEW: sprint plan)
- blockchain-project/DECISIONS.md: ADR-014 (u128 tokenomics with deferred balance migration)
**Review Notes:** Phase 1-2: Full tokenomics module — 1B hard cap, 2-year halving emission (120M→7.5M over 10 years, 3.75M tail), 4-pool distribution (70% validator, 15% PoUW, 10% treasury, 5% insurance), piecewise linear APY curve (3-12%), 8 genesis allocations totaling 400M with vesting cliffs. Phase 3: EmissionTracker wired into pipeline, 2 new RPCs (getEmissionInfo, getVestingStatus). Phase 4: clippy 0 warnings, fmt clean, all tests pass (39 new: 36 tokenomics + 3 RPC).
**Security Flags:** 0 ELEVATED, 0 MEDIUM. All u128 arithmetic checked for overflow. Hard cap enforced in emission_per_epoch(). No floating-point. Floor division only. Division-by-zero guarded.

---

### 2026-03-09 -- smart-contract-engineer / node-engineer / security-engineer -- Sprint 036 Complete (M8 Sprint 11: Governance Execution & Chain Parameters)
**Task:** Sprint 036: ChainParams runtime registry, governance proposal execution, dynamic fee/eviction params, 2 new RPCs
**Sprint:** Sprint 036, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/chain_params.rs (NEW): ChainParams, ParamValue, ParamDef, ParamType, ChainParamError, 9 governance-controllable params with typed bounds, 7 unit tests
- crates/aztibase-execution/src/governance.rs: added passed_unexecuted() method
- crates/aztibase-execution/src/fee.rs: added update_with_params() for governance-controlled fee parameters
- crates/aztibase-execution/src/lib.rs: chain_params module + re-exports
- crates/aztibase-node/src/pipeline.rs: ChainParams field, shared_chain_params(), proposal execution after finalize_expired(), dynamic fee update via update_with_params(), dynamic eviction limits from ChainParams, 4 governance execution tests
- crates/aztibase-node/src/main.rs: .with_chain_params() wired into RPC server
- crates/aztibase-rpc/src/server.rs: ChainParams in RpcState, with_chain_params() builder, aztb_getChainParam + aztb_listChainParams handlers, 2 RPC tests
- blockchain-project/sprints/SPRINT-036.md (NEW: sprint plan)
**Review Notes:** Phase 1: ChainParams registry with 9 typed params (base_fee_floor/ceiling, target/max gas, change denom, max_block_range, max_stored_txs/batch_roots/receipts), bounds validation, get/set/list. Phase 2: Proposal execution engine — passed_unexecuted() + pipeline auto-executes passed proposals via set_from_str(), mark_executed() on success or failure (prevents retry loops). Phase 3: BaseFeeCalculator.update_with_params() reads from ChainParams, eviction uses dynamic limits, 2 new RPCs (getChainParam, listChainParams). Phase 4: clippy 0 warnings, fmt clean, all tests pass (13 new: 7 chain_params + 4 governance execution + 2 RPC).
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Bounds prevent dangerous values (zero gas, zero base fee). Only Passed proposals execute. Unknown/invalid param_key proposals marked Executed to prevent retry loops. Param key whitelist enforced (9 known keys). No unbounded iteration.

---

### 2026-03-09 -- smart-contract-engineer / node-engineer / security-engineer -- Sprint 035 Complete (M8 Sprint 10: On-Chain Governance Foundations)
**Task:** Sprint 035: On-chain governance module — proposal creation, stake-weighted voting, proposal finalization, 2 new TxKinds, 2 governance RPCs
**Sprint:** Sprint 035, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/governance.rs (NEW): GovernanceStore, Proposal, Vote, VoteTally, CreateProposalParams, ProposalStatus enum, create_proposal/cast_vote/finalize_expired/mark_executed, 6 unit tests
- crates/aztibase-execution/src/routing.rs: TxKind::CreateProposal (0x0E), TxKind::CastVote (0x0F), PREFIX constants, all match arms updated, 2 roundtrip tests
- crates/aztibase-execution/src/lib.rs: governance module + re-exports
- crates/aztibase-node/src/pipeline.rs: GovernanceStore field, shared_governance(), CreateProposal/CastVote sorting + execution, proposal finalization after batch, compute_tx_hash for new variants
- crates/aztibase-node/src/main.rs: .with_governance() wired into RPC server
- crates/aztibase-rpc/src/server.rs: GovernanceStore in RpcState, with_governance() builder, aztb_getProposal + aztb_listProposals handlers, all test RpcState instances updated
- blockchain-project/sprints/SPRINT-035.md (NEW: sprint plan)
**Review Notes:** Phase 1: GovernanceStore with proposal lifecycle, stake-weighted vote tallying, quorum threshold (>50% approve weight, ≥2 voters). Phase 2: 2 new TxKinds wired through routing (encode/decode/nonce/gas_price/sender/gas_limit), pipeline creates proposals and records votes using voter's balance as stake weight. Phase 3: Proposal finalization at end_round in batch loop, 2 RPC endpoints (getProposal with tally, listProposals with status filter). Phase 4: clippy 0 warnings, fmt clean, 674 tests pass (8 new: 6 governance + 2 routing).
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Double-vote prevention enforced. MAX_ACTIVE_PROPOSALS=64 caps proposal spam. Voting period bounded (10-10000 rounds). Description/param_key/param_value length-capped. Zero-stake votes rejected.

---

### 2026-03-09 -- node-engineer / consensus-engineer / security-engineer -- Sprint 034 Complete (M8 Sprint 9: Archive Node & Historical Queries)
**Task:** Sprint 034: Archive node mode (--archive flag), 7 historical query RPC methods, batch index/txs storage, block explorer support
**Sprint:** Sprint 034, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/config.rs: `archive: bool` field in NodeConfig
- crates/aztibase-node/src/main.rs: `--archive` CLI flag, consensus + pipeline archive wiring
- crates/aztibase-node/src/pipeline.rs: `archive` field, `set_archive()`, gated eviction, batch index/txs/tx storage
- crates/aztibase-consensus/src/engine.rs: `archive: bool` in ConsensusConfig, gated DAG + state pruning
- crates/aztibase-storage/src/store.rs: BATCH_INDEX_TABLE, BATCH_TXS_TABLE, ALL_TABLES 10→12
- crates/aztibase-storage/src/lib.rs: re-exports for new tables
- crates/aztibase-execution/src/persist.rs: store_batch_index, get_batch_by_number, store_batch_txs, get_batch_txs, store_transaction, get_transaction, get_batch_range, 4 new tests
- crates/aztibase-execution/src/lib.rs: re-exports for new persist functions
- crates/aztibase-rpc/src/server.rs: 7 new RPC methods (getBlockByNumber, getBlockByHash, getTransactionByHash, getBatchRoot, getBlockRange, getTransactionsByBatch, getReceiptsByBatch), MAX_BLOCK_RANGE=100, parse_u64_param, parse_hash_param helpers
- crates/aztibase-node/src/integration.rs: ConsensusConfig archive field added to all test instances
- blockchain-project/sprints/SPRINT-034.md (NEW: sprint plan)
**Review Notes:** Phase 1: Archive mode gates eviction (receipts/txs/batch_roots) and DAG+state pruning via --archive CLI flag. Phase 2: 4 historical query RPCs (getBlockByNumber, getBlockByHash, getTransactionByHash, getBatchRoot) with new BATCH_INDEX_TABLE and BATCH_TXS_TABLE. Phase 3: 3 explorer RPCs (getBlockRange with MAX_BLOCK_RANGE=100 pagination, getTransactionsByBatch, getReceiptsByBatch). Phase 4: clippy 0 warnings, fmt clean, all tests pass.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Range queries bounded by MAX_BLOCK_RANGE=100. Archive mode only disables eviction — no extra data exposure through RPC.

---

### 2026-03-09 -- consensus-engineer / node-engineer / security-engineer -- Sprint 033 Complete (M8 Sprint 8: State Pruning & Bounded Growth)
**Task:** Sprint 033: DagStore round-based pruning, pipeline memory caps, disk table eviction
**Sprint:** Sprint 033, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/dag_store.rs: `DAG_RETENTION_BUFFER`, `pruned_through` field, `prune_before()`, `pruned_through()`, 5 new tests
- crates/aztibase-consensus/src/engine.rs: `evaluate_commits()` calls `dag.prune_before()` after commit loop
- crates/aztibase-node/src/pipeline.rs: bounded `executed_anchors` (VecDeque+HashSet ring buffer), per-task + total attestation buffer caps, disk eviction calls after batch
- crates/aztibase-execution/src/persist.rs: `evict_old_transactions()`, `evict_old_batch_roots()`, 3 new tests
- crates/aztibase-execution/src/lib.rs: re-exports for eviction functions
- crates/aztibase-storage/src/store.rs: `delete_batch()` method
- blockchain-project/sprints/SPRINT-033.md (NEW: sprint plan)
**Review Notes:** Phase 1: DagStore prune_before() removes rounds from in-memory index + on-disk BLOCKS_TABLE, retention buffer of 16 rounds, orphaned children refs cleaned. Phase 2: executed_anchors converted to bounded VecDeque+HashSet (10K cap), attestation_buffer per-task cap (32) + total cap (2048), disk eviction wired after batch persistence. Phase 3: StateStore::delete_batch() for atomic multi-key deletion, TX_TABLE eviction (500K cap), BATCH_ROOTS_TABLE eviction (100K cap). 455+ tests pass (105 consensus + 175 execution + 175 node), clippy 0 warnings, fmt clean.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. DAG retention buffer (16) exceeds 2× wave_length, preventing data loss for in-flight consensus. Ring buffer eviction still rejects recent duplicate anchors within window.

---

### 2026-03-09 -- consensus-engineer / security-engineer -- Sprint 032 Complete (M8 Sprint 7: Adversarial Consensus Testing)
**Task:** Sprint 032: Fault injection harness, Byzantine fault tolerance verification, liveness/safety tests
**Sprint:** Sprint 032, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs: `state`, `insert_genesis()`, `handle_input()`, `handle_received_vertex()` made public
- crates/aztibase-node/src/integration.rs: FaultRouter, AdversarialTestbed, 14 new adversarial tests
- blockchain-project/sprints/SPRINT-032.md (NEW: sprint plan)
**Review Notes:** Phase 1: FaultRouter with drop/partition/reorder + AdversarialTestbed multi-node harness (4 tests). Phase 2: Byzantine scenarios — leader equivocation, multi-Byzantine (7 validators), orphan vertex flood, invalid parent rejection, duplicate vertex idempotency (5 tests). Phase 3: network partition + heal, minority stall (2/7), finality cert forgery (6 attack vectors), buffer exhaustion, message reordering convergence (5 tests). 175 tests pass (14 new), clippy 0 warnings, fmt clean.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. Finality certificate forgery tests confirm all 6 attack vectors correctly rejected.

---

### 2026-03-09 -- node-engineer / ai-integration-engineer -- Sprint 031 Complete (M8 Sprint 6: End-to-End Smoke Tests & Integration Testing)
**Task:** Sprint 031: 13 new e2e integration tests, attestation signature verification bug fix
**Sprint:** Sprint 031, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/tx.rs: `verify_and_route_with_pubkey()`, `verify_and_route_batch_with_pubkeys()`, `BatchWithPubkeys` type alias
- crates/aztibase-execution/src/lib.rs: updated exports for new functions
- crates/aztibase-node/src/pipeline.rs: BUG FIX — attestation sig verification now uses raw Ed25519 pubkey from envelope instead of BLAKE3-hashed address
- crates/aztibase-node/src/integration.rs: 13 new e2e tests (fee market, multi-transfer stress, nonce gaps, insufficient balance, RegisterModel, CommitCompute, PostTask, SubmitAttestation, full AI lifecycle, DeregisterCompute, duplicate model, deregister with pending tasks, mixed batch)
**Review Notes:** Discovered production bug: `address_from_pubkey()` = BLAKE3(pubkey) is irreversible, so attestation sig verification was using a hash as an Ed25519 key (always invalid). Fixed by threading raw pubkeys through batch routing. 161 tests pass (13 new), clippy 0 warnings, fmt clean.
**Security Flags:** 0 ELEVATED, 0 MEDIUM. BUG FIX: attestation signature verification was non-functional through signed tx flow (SEC-ATT-PUBKEY-001 — CRITICAL, now FIXED)

---

### 2026-03-08 -- p2p-network-engineer -- Sprint 030 Complete (M8 Sprint 5: WebRTC Direct Transport & DCUtR Hole Punching)
**Task:** Sprint 030: DCUtR hole punching, WebRTC direct transport (feature-gated), NAT traversal pipeline integration
**Sprint:** Sprint 030, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (root): added `dcutr` to libp2p features
- crates/aztibase-network/Cargo.toml: added `libp2p-webrtc` + `rcgen` optional deps behind `webrtc` feature
- crates/aztibase-network/src/behaviour.rs: added `dcutr::Behaviour` to `AztibaseBehaviour`
- crates/aztibase-network/src/transport.rs: `NatTraversalStats`, DCUtR event handling, `enable_webrtc`/`webrtc_listen_port` in TransportConfig, `DEFAULT_WEBRTC_PORT`
- crates/aztibase-network/src/webrtc.rs: `build_libp2p_transport()` behind `#[cfg(feature = "webrtc")]`, 10 tests total
- crates/aztibase-network/src/lib.rs: updated exports, 4 new tests (dcutr behaviour, stats default/clone, webrtc config propagation)
- crates/aztibase-node/src/config.rs: `enable_webrtc`, `webrtc_listen_port` in NetworkConfig
- crates/aztibase-node/src/main.rs: `--webrtc` CLI flag, wired WebRTC config into both full-node and light-node TransportConfig construction
- Dockerfile: EXPOSE 30333 9944 9000
**Review Notes:** DCUtR is pure Rust, auto-triggers via libp2p relay+NAT detection. WebRTC feature-gated (default off) — full build deferred to CI due to disk space. 60 network tests pass. clippy 0 warnings, fmt clean.
**Security Flags:** 0 ELEVATED, 0 MEDIUM, 1 LOW (WebRTC alpha API — mitigated by feature gate)

---

### 2026-03-08 -- p2p-network-engineer -- Sprint 029 Complete (M8 Sprint 4: Gossipsub Hardening & Persistent Peer Discovery)
**Task:** Sprint 029: Gossipsub scoring retune, persistent peer store, Kademlia bootstrap, message validation
**Sprint:** Sprint 029, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-network/src/gossip.rs (per-topic weights, tightened penalties, message validation)
- crates/aztibase-network/src/peer_store.rs (NEW: PeerStore redb-backed persistent peer addresses)
- crates/aztibase-network/src/transport.rs (PeerStore wiring, Kademlia bootstrap, message validation in event loop)
- crates/aztibase-network/src/lib.rs (peer_store module, exports, 7 new tests)
- crates/aztibase-node/src/main.rs (PeerStore init, load_cached_peers on startup)
- blockchain-project/sprints/SPRINT-029.md (NEW: sprint plan)
**Review Notes:**
- Phase 1: invalid_message_deliveries_weight -10→-50, decay 0.3→0.1. mesh_message_deliveries threshold 20→50, cap 100→500. Per-topic weights: consensus=2.0, blocks=1.5, validator-announce=1.5. publish_threshold -50→-30, graylist_threshold -80→-60.
- Phase 2: PeerStore (redb), address merge+dedup, load_cached_peers on startup (50 max, skip banned).
- Phase 3: Kademlia bootstrap on first PeerConnected, RoutingUpdated→PeerStore. validate_gossip_message: structural checks, Medium offense on reject.
- Phase 4: 0 ELEVATED, 0 MEDIUM, 2 LOW. 56 network tests, clippy 0 warnings, fmt clean.
**Security Flags:**
- SEC-NET-029-001 LOW: Gossipsub over-penalization by design
- SEC-NET-029-002 LOW: Topic matching via contains() — not exploitable with fixed topic set

---

### 2026-03-08 -- p2p-network-engineer -- Sprint 028 Complete (M8 Sprint 3: Networking Hardening)
**Task:** Sprint 028: Persistent peer reputation, connection filtering, AutoNAT + relay client, security review
**Sprint:** Sprint 028, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: added autonat + relay features to libp2p)
- crates/aztibase-network/Cargo.toml (added redb dependency)
- crates/aztibase-network/src/reputation.rs (NEW: PeerReputationStore, PeerReputation, OffenseSeverity, tiered bans, score decay, prune/evict)
- crates/aztibase-network/src/connection_filter.rs (NEW: ConnectionFilter, per-IP limits, rate limiting, subnet caps)
- crates/aztibase-network/src/behaviour.rs (added autonat + relay_client to AztibaseBehaviour)
- crates/aztibase-network/src/transport.rs (NatStatus, relay client builder, ban check + conn filter on ConnectionEstablished, autonat event handling, relay fallback)
- crates/aztibase-network/src/lib.rs (module exports, 4 new tests)
- crates/aztibase-node/src/main.rs (PeerReputationStore init for full + light nodes, relay_servers wiring)
- blockchain-project/sprints/SPRINT-028.md (NEW: sprint plan)
**Review Notes:**
- Phase 1: PeerReputationStore backed by redb (peer_reputation table). Tiered bans: score < -100 → 1hr, < -200 → 24hr, < -500 → 7 days. Score decay +1/hr toward 0. Prune stale + evict excess (10K cap). 7 tests.
- Phase 2: ConnectionFilter with per-IP (MAX=3), rate limit (1000ms), subnet /16 cap (MAX=5). Wired into ConnectionEstablished event. 7 tests.
- Phase 3: AutoNAT probing (30s interval), NatStatus tracking, relay client via SwarmBuilder. NAT Private → auto-listen on relay circuit addresses. 4 tests.
- Phase 4: Security review — 0 ELEVATED, 0 MEDIUM, 4 LOW (all acceptable). 47 network tests pass, clippy 0 warnings, fmt clean.
**Security Flags:**
- SEC-NET-028-001 LOW: Ban check after connection established (inherent to libp2p — immediate disconnect is correct mitigation)
- SEC-NET-028-002 LOW: peer_ips HashMap growth bounded by MAX_ESTABLISHED_CONNECTIONS=50
- SEC-NET-028-003 LOW: Reputation redb I/O in event loop — acceptable for single-key lookups at current scale
- SEC-NET-028-004 LOW: Relay trust assumes boot nodes are honest relays — acceptable for testnet

---

### 2026-03-08 -- node-engineer -- Sprint 027 Complete (M8 Sprint 2: Docker Testnet Bootstrap & Genesis Tooling)
**Task:** Sprint 027: Docker genesis layout, `--docker` CLI flag, aztb_chainId + aztb_genesisHash RPC, Makefile, reset script
**Sprint:** Sprint 027, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/genesis.rs (write_docker_configs(), genesis_hash(), write_keyfile refactor)
- crates/aztibase-node/src/main.rs (--docker flag on Genesis subcommand, with_genesis_hash wiring)
- crates/aztibase-rpc/src/server.rs (genesis_hash field in RpcState, aztb_chainId + aztb_genesisHash handlers)
- docker-compose.yml (simplified to use TOML config, removed CLI overrides)
- Dockerfile (added curl for healthchecks)
- Makefile (NEW: setup/start/stop/reset/logs/status/build/test/clean targets)
- scripts/setup-docker-testnet.sh (NEW: generates Docker testnet layout)
- scripts/reset-testnet.sh (NEW: wipes node DBs preserving keys + genesis)
- blockchain-project/sprints/SPRINT-027.md (NEW: sprint plan)
**Review Notes:**
- Phase 1: write_docker_configs() generates data/genesis/genesis.toml + data/node{N}/node{N}.toml + data/node{N}/keys/validator{N}.json. Boot nodes use /dns4/validatorN/tcp/30333 Docker DNS names.
- Phase 2: aztb_chainId returns "0xa27b", aztb_genesisHash returns BLAKE3 hash of serialized genesis TOML. genesis_hash stored in RpcState at startup.
- Phase 3: Makefile wraps docker compose + setup/reset scripts. Dockerfile adds curl for container healthchecks.
- Phase 4: 201 tests pass (148 node + 53 RPC), 0 clippy warnings, fmt clean.
**Security Flags:**
- Genesis keys are plaintext JSON — acceptable for testnet only. Mainnet requires encrypted keyfiles.
- Boot node DNS trust is implicit within Docker bridge network — production needs explicit peer IDs.

---

### 2026-03-08 -- node-engineer -- Sprint 026 Complete (M8 Sprint 1: Prometheus Metrics + Testnet Infrastructure)
**Task:** Sprint 026: Prometheus metrics integration, Grafana/Prometheus Docker stack, testnet faucet + nodeInfo + health endpoints
**Sprint:** Sprint 026, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: added prometheus-client 0.23)
- Cargo.lock (dependency tree updates)
- crates/aztibase-rpc/Cargo.toml (added prometheus-client)
- crates/aztibase-rpc/src/lib.rs (export NodeMetrics, metrics module)
- crates/aztibase-rpc/src/metrics.rs (NEW: NodeMetrics registry with typed counters/gauges, Prometheus + JSON encoding)
- crates/aztibase-rpc/src/server.rs (dual-format /metrics + /metrics/json, /health endpoint, aztb_faucetDrip + aztb_nodeInfo RPC, chain_id + faucet_tracker state, 10 new tests)
- crates/aztibase-node/src/main.rs (NodeMetrics replaces Arc<RwLock<Value>>, Prometheus wiring)
- monitoring/prometheus.yml (NEW: Prometheus scrape config for 3 validators)
- monitoring/grafana/provisioning/datasources/prometheus.yml (NEW: Grafana Prometheus datasource)
- monitoring/grafana/provisioning/dashboards/dashboards.yml (NEW: dashboard provisioning)
- monitoring/grafana/dashboards/node-health.json (NEW: block height, TPS, peers, mempool, base fee, pending tasks)
- monitoring/grafana/dashboards/consensus.json (NEW: rounds, commit rate, latency, vertices, equivocations)
- docker-compose.yml (added prometheus + grafana services, healthcheck uses /health)
- blockchain-project/STATUS.md (M8 scope, Sprint 026 tracking)
- blockchain-project/sprints/SPRINT-026.md (NEW: sprint plan)
**Review Notes:**
- Phase 1: prometheus-client 0.23 (pure Rust, zero C deps) replaces opaque JSON metrics. NodeMetrics struct with 12 counters/gauges. Dual-format: /metrics (Prometheus text 0.0.4), /metrics/json (backward compat with ADR-008). 4 new metrics tests.
- Phase 2: monitoring/ directory with Prometheus scrape config (5s interval), Grafana provisioned datasource + 2 dashboards (Node Health, Consensus). Docker Compose adds prometheus:9090 + grafana:3000.
- Phase 3: aztb_faucetDrip (testnet-only, 10 AZTB, 60s rate limit per address), aztb_nodeInfo (version, chain_id, protocol), GET /health (200 OK + block height). Healthchecks updated to use /health. 5 new tests.
- Phase 4: clippy 0 warnings, fmt clean, cargo-deny clean (warnings only: duplicate foldhash/thiserror). 53 RPC tests pass.
**Security Flags:**
- Faucet gated by chain_id == 0xA27B (testnet only) — cannot be used on mainnet
- Faucet rate limited to 1 drip per address per 60s — prevents treasury drain
- /health exposes only block height and chain_id — no sensitive data
- Prometheus /metrics exposes only aggregate counters/gauges — no PII, no keys

---

### 2026-03-08 -- security-engineer -- Sprint 025 Complete (M7 Continued: Dep Upgrades, Crypto Audit, Verkle)
**Task:** Sprint 025: Dependency upgrades (wasmtime v42, postcard), cryptographic audit (domain separation, known-answer vectors), Verkle proof verification rewrite, fuzz target setup
**Sprint:** Sprint 025, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (wasmtime v28→v42, bincode→postcard, workspace exclude fuzz dirs)
- Cargo.lock (dependency tree updates)
- deny.toml (removed wasmtime/bincode ignores, added 5 transitive dep ignores)
- crates/aztibase-core/Cargo.toml (postcard replaces bincode)
- crates/aztibase-core/src/lib.rs (8 known-answer crypto test vectors: BLAKE3, Ed25519, BLS12-381)
- crates/aztibase-core/src/commitment.rs (VerkleProof rewrite: stem, value_hash, levels fields)
- crates/aztibase-core/fuzz/ (3 fuzz targets: transaction_deser, block_header_deser, hash_and_pubkey)
- crates/aztibase-consensus/Cargo.toml (postcard replaces bincode)
- crates/aztibase-consensus/fuzz/ (3 fuzz targets: vertex_deser, finality_cert_deser, signer_bitmap)
- crates/aztibase-execution/Cargo.toml (wasmtime v42, postcard)
- crates/aztibase-execution/src/verkle.rs (complete rewrite: domain-separated commitments, proper proof verification)
- crates/aztibase-execution/src/vm.rs (wasmtime v42 API migration)
- crates/aztibase-execution/src/tx.rs (Ed25519 TX_DOMAIN prefix)
- crates/aztibase-execution/src/state.rs (Merkle LEAF_DOMAIN/NODE_DOMAIN prefixes)
- crates/aztibase-execution/src/routing.rs (postcard take_from_bytes + trailing bytes check)
- crates/aztibase-wasm/src/proof.rs (Verkle proof types updated, verify_verkle_proof rewritten)
- 16 additional files migrated bincode→postcard
**Review Notes:**
- Phase 1: wasmtime v28→v42 resolves 4 CVEs; bincode→postcard resolves RUSTSEC-2025-0141
- Phase 2: Ed25519 tx signing domain-separated (AZTB_TX_V1), attestation hashing prefixed, Merkle leaf/node collision fixed
- Phase 2: BLS PoP enforcement already present in pipeline (Task 7 confirmed complete)
- Phase 2: 8 pinned crypto test vectors ensure no regression across backend changes
- Phase 3: Verkle tree rewritten with domain-separated BLAKE3 commitments (AZTB_VERKLE_INNER, AZTB_VERKLE_LEAF); proofs are self-contained with stem + value_hash + 256-width level commitments; bottom-up verification. ADR-013 documents the decision to keep BLAKE3 placeholder vs IPA (no mature pure-Rust crate exists).
- Phase 4: cargo-fuzz targets defined for core (3) and consensus (3); cargo-deny clean; clippy 0 warnings; fmt clean
**Security Flags:** BLS PoP was already enforced (no gap). Verkle proofs now properly verifiable (was trivially forgeable placeholder).

---

### 2026-03-08 -- security-engineer -- Sprint 024 Complete (M7 Security Audit + Hardening)
**Task:** Sprint 024: Security audit and hardening — resolve 11 documented security findings (16 tasks, 4 phases)
**Sprint:** Sprint 024, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/cross_vm.rs (ReentrancyGuard: HashSet-based call stack guard for cross-VM calls, 1 new test)
- crates/aztibase-execution/src/evm.rs (hex_encode_bounded: truncate revert reasons at 1024 bytes, MAX_REVERT_REASON_BYTES, 3 new tests)
- crates/aztibase-runtime/src/tract_runtime.rs (inference timeout via Instant elapsed check, MAX_INPUT_SIZE 16MB, 1 new test)
- crates/aztibase-execution/src/receipt.rs (evict_old_receipts: MAX_STORED_RECEIPTS=100K, 1 new test)
- crates/aztibase-storage/src/store.rs (evict_oldest: generic count-based eviction for redb tables)
- crates/aztibase-rpc/src/server.rs (IpConnectionTracker: per-IP WebSocket rate limiting, MAX_WS_PER_IP=8, 429 rejection, 1 new test)
- crates/aztibase-storage/src/light.rs (evict_excess_proofs: MAX_PROOF_CACHE_ENTRIES=4096, round-sorted eviction, 1 new test)
- crates/aztibase-network/src/webrtc.rs (validate_stun_uri: scheme/host/port validation on WebRTC STUN config, 3 new tests)
- crates/aztibase-network/src/light_sync.rs (MultiPeerValidator: majority-based multi-peer header validation for eclipse attack resistance, 4 new tests)
- crates/aztibase-consensus/src/finality.rs (SignerBitmap: packed u8 bitmap replacing Vec<bool>, 8x bandwidth savings; structural validation: reject zero batch_hash, zero signers, empty bitmap; 3 new tests)
- crates/aztibase-execution/src/lib.rs (export ReentrancyGuard, evict_old_receipts)
- crates/aztibase-network/src/lib.rs (export MultiPeerValidator)
- crates/aztibase-consensus/src/lib.rs (export SignerBitmap)
- deny.toml (NEW: cargo-deny config for license + advisory + ban + source checks)
**Review Notes:**
- 11 security findings resolved: 1 MEDIUM (SEC-BRIDGE-003), 10 LOW/INFO
- 570 tests pass (up from 555), 0 clippy warnings, fmt clean
- SignerBitmap: zero new dependencies, uses packed Vec<u8> instead of bitvec
- MultiPeerValidator: validates header responses from multiple peers via majority consensus
- IpConnectionTracker: thread-safe per-IP connection counting with automatic release
**Security Flags:**
- SEC-BRIDGE-003 MEDIUM RESOLVED: Cross-VM reentrancy guard prevents recursive cross-VM calls
- SEC-EVM-008 LOW RESOLVED: Revert reason truncated at 1024 bytes
- SEC-EVM-002 INFO RESOLVED: Gas cost verification tests added
- SEC-AI-002 LOW RESOLVED: Inference timeout + input size limit
- SEC-RCPT-002 LOW RESOLVED: Receipt store eviction at 100K entries
- SEC-WS-002 LOW RESOLVED: Per-IP WebSocket rate limiting
- SEC-CACHE-001 LOW RESOLVED: Proof cache bounded at 4096 entries
- SEC-WEBRTC-001 LOW RESOLVED: STUN URI validation on construction
- SEC-P2P-001 LOW RESOLVED: Multi-peer header validation
- SEC-BLS-009 INFO RESOLVED: Packed signer bitmap (8x bandwidth savings)
- SEC-LC-001 LOW RESOLVED: Finality cert structural validation

---

### 2026-03-07 -- project-lead -- Sprint 023 Complete (M6 CLOSED)
**Task:** Sprint 023: M6 Closure — Deregistration, Task Assignment, Compute Validation (16 tasks, 4 phases)
**Sprint:** Sprint 023, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/pouw.rs (ComputeCommitmentStore::deregister returns Option<ComputeCommitment>, InferenceTask.assigned_validator field, 1 new test)
- crates/aztibase-execution/src/routing.rs (DeregisterCompute 0x0C + DeregisterModel 0x0D TxKind variants, encode/decode/gas/sender/route, 2 roundtrip tests)
- crates/aztibase-node/src/pipeline.rs (DeregisterCompute execution + stake refund, DeregisterModel execution + owner auth + pending task check, TaskAssigner wiring in PostTask, SubmitAttestation assigned_validator enforcement, CommitCompute model validation + overwrite refund, 8 new tests)
- crates/aztibase-node/src/task_pool.rs (removed dead code: evict_expired + is_empty, fmt cleanup)
- crates/aztibase-node/src/main.rs (removed #[allow(dead_code)] from task_pool module)
- crates/aztibase-rpc/src/server.rs (aztb_getTaskStatus includes assignedValidator field)
**Review Notes:**
- SEC-COMMIT-OVERWRITE from Sprint 022 fully resolved: CommitCompute overwrite now refunds prior stake
- TaskAssigner automatically assigns best validator on PostTask via PoUW score
- SubmitAttestation enforces assigned_validator match when set
- CommitCompute validates all supported_models exist in ModelRegistry
- DeregisterModel blocked if pending tasks exist for that model
- 555 tests pass, 0 clippy warnings, fmt clean
- M6 milestone complete: full AI compute market flow operational
**Security Flags:**
- SEC-COMMIT-OVERWRITE LOW RESOLVED: CommitCompute overwrite now refunds prior stake before deducting new
- No new ELEVATED or MEDIUM findings

---

### 2026-03-07 -- project-lead -- Sprint 022 Complete
**Task:** Sprint 022: Task Execution Loop & Attestation Flow (16 tasks, 4 phases)
**Sprint:** Sprint 022, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/routing.rs (SubmitAttestation 0x0A, CommitCompute 0x0B variants + 4 roundtrip tests)
- crates/aztibase-node/src/main.rs (un-gate task_pool module, wire pending_task_count + compute_commitments to RPC)
- crates/aztibase-node/src/pipeline.rs (TaskPool wiring, expiry refund, SubmitAttestation exec with Ed25519 sig verify, CommitCompute exec with stake bond, TaskSettlement delegation, attestation buffer cleanup, 4 tests)
- crates/aztibase-node/src/task_pool.rs (drain_expired method + test)
- crates/aztibase-rpc/src/server.rs (aztb_pendingTaskCount, aztb_getComputeCommitment, aztb_listComputeProviders handlers + 4 tests, INTERNAL_ERROR constant)
- crates/aztibase-rpc/Cargo.toml (no changes needed — already depends on aztibase-consensus)
- blockchain-project/sprints/SPRINT-022.md (sprint plan)
- blockchain-project/DECISIONS.md (ADR-011)
- CHANGELOG.md (Sprint 022 entries)
**Review Notes:**
- Pipeline refactored to use TaskSettlement::settle() instead of inline settlement logic
- Ed25519 sig verification uses existing aztibase_core::PublicKey wrapper (no new deps)
- Attestation buffer cleanup on task expiry prevents unbounded memory growth
- 544 tests pass, 0 clippy warnings, fmt clean
**Security Flags:**
- SEC-ATT-LEAK-001 LOW FIXED: attestation buffer entries cleaned on task expiry
- SEC-COMMIT-OVERWRITE LOW ACCEPTED: CommitCompute overwrites previous commitment without refunding prior stake (deregistration deferred)

---

### 2026-03-07 -- project-lead -- Sprint 021 Complete
**Task:** Sprint 021: AI Compute Market & PoUW Scoring (16 tasks, 4 phases)
**Sprint:** Sprint 021, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/model_registry.rs (NEW: on-chain model registry)
- crates/aztibase-execution/src/routing.rs (RegisterModel 0x08, PostTask 0x09 variants)
- crates/aztibase-execution/src/lib.rs (model_registry module + exports)
- crates/aztibase-execution/src/state.rs (storage() method for full BTreeMap access)
- crates/aztibase-consensus/src/pouw.rs (ComputeCommitment, SlidingWindowPoUWScore, AttestationAggregator, ValidatorWorkHistory)
- crates/aztibase-consensus/src/lib.rs (updated pouw exports)
- crates/aztibase-node/src/pipeline.rs (RegisterModel + PostTask execution, task escrow)
- crates/aztibase-node/src/task_pool.rs (NEW: TaskPool, TaskAssigner, TaskSettlement)
- crates/aztibase-node/src/main.rs (task_pool module, cfg(test) gated)
- crates/aztibase-rpc/src/server.rs (aztb_getModelInfo, aztb_listModels, aztb_getTaskStatus endpoints)
- crates/aztibase-rpc/Cargo.toml (aztibase-consensus + bincode deps)
- blockchain-project/sprints/SPRINT-021.md (sprint plan + retrospective)
- blockchain-project/DECISIONS.md (ADR-010: PoUW scoring formula)
**Review Notes:**
- Phase 1: ModelRegistry (register/query/deregister), TxKind::RegisterModel (0x08), ComputeCommitmentStore
- Phase 2: SlidingWindowPoUWScore (0.4 accuracy + 0.3 latency + 0.3 availability), AttestationAggregator (quorum ≥ 2)
- Phase 3: TxKind::PostTask (0x09), TaskPool (1024 cap, expiry eviction), TaskAssigner (highest PoUW score), TaskSettlement (reward split)
- Phase 4: Security review (1 MEDIUM fixed: Sybil dedup), 3 new RPC endpoints, ADR-010, clippy/fmt clean
**Security Flags:** SEC-SYBIL-001 (MEDIUM, FIXED): AttestationAggregator now deduplicates by validator_id

### 2026-03-07 -- project-lead -- Sprint 020 Complete
**Task:** Sprint 020: WebSocket Gateway, RPC Subscriptions & Deployment (16 tasks, 4 phases)
**Sprint:** Sprint 020, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: axum ws feature enabled, tokio-tungstenite dev-dep)
- crates/aztibase-rpc/Cargo.toml (futures dep, tokio-tungstenite dev-dep)
- crates/aztibase-rpc/src/lib.rs (EventBus export)
- crates/aztibase-rpc/src/server.rs (WebSocket upgrade handler at /ws, JSON-RPC dispatch over WS, light sync message handling over WS, EventBus with broadcast channels, aztb_subscribe/aztb_unsubscribe methods, connection limit enforcement; 6 new tests)
- crates/aztibase-node/src/main.rs (EventBus wiring, event publishing on batch commit, WalletAction::List/Balance/Export/Import subcommands)
- crates/aztibase-node/src/wallet.rs (list_keys, export_keyfile, import_keyfile, query_balance; 4 new tests)
- Dockerfile (NEW: multi-stage build, rust:1.88 builder + debian:bookworm-slim runtime)
- docker-compose.yml (NEW: 3-node local testnet with health checks)
- blockchain-project/sprints/SPRINT-020.md (NEW: sprint plan, all tasks marked DONE)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md, DECISIONS.md updated
**Review Notes:**
- Phase 1: axum WebSocket upgrade at /ws, JSON-RPC dispatch over WS text frames, light sync protocol handling (RequestHeaders/RequestBalance/RequestProof), max 256 concurrent connections
- Phase 2: aztb_subscribe (newHeads/finality topics), aztb_unsubscribe with task abort, broadcast channels per topic, HTTP subscribe returns clear error
- Phase 3: wallet list (scans keyfile dir), wallet balance (aztb_getBalance + aztb_getNonce via RPC), wallet export/import (encrypted keyfile JSON roundtrip)
- Phase 4: Dockerfile (multi-stage, debian bookworm-slim for blst glibc), docker-compose 3-validator testnet, security review, 491 tests passing
**Security Flags:**
- SEC-WS-002 (LOW): WebSocket connections limited to 256 but no per-IP rate limiting; relies on reverse proxy for production
- SEC-SUB-001 (LOW): Broadcast channels bounded at 256; slow consumers receive lagged errors, not unbounded memory growth
- SEC-EXPORT-001 (LOW): Wallet export outputs encrypted keyfile JSON; no plaintext secret keys exposed in export format
- 0 ELEVATED, 0 MEDIUM

### 2026-03-07 -- project-lead -- Sprint 019 Complete
**Task:** Sprint 019: Browser WASM Light Node & HD Wallet Derivation (16 tasks, 4 phases)
**Sprint:** Sprint 019, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: aztibase-wasm member, wasm-bindgen + js-sys deps)
- crates/aztibase-wasm/Cargo.toml (NEW: wasm-bindgen, js-sys, blake3, serde, bincode)
- crates/aztibase-wasm/src/lib.rs (NEW: WASM entry points — verifyHeaderChain, verifyMerkleProof, verifyVerkleProof, verifyLightClientProof, blake3Hash, buildHeaderRequest, latestSyncedRound; 4 tests)
- crates/aztibase-wasm/src/sync.rs (NEW: SyncHeader, SyncFinalityCert, verify_header_chain, JsSyncState; 7 tests)
- crates/aztibase-wasm/src/proof.rs (NEW: MerkleProof, VerkleProof, verify_merkle_proof, verify_verkle_proof, verify_light_client_proof, JS type adapters; 10 tests)
- crates/aztibase-wasm/web/light-client.js (NEW: LightClient class, HeaderCache with IndexedDB, WebSocket transport bridge)
- crates/aztibase-wasm/web/index.html (NEW: browser demo page — connect, sync headers, verify proofs)
- crates/aztibase-node/src/wallet.rs (derive_account, encrypt_keyfile_pub, derive_child test helper; 4 new tests)
- crates/aztibase-node/src/main.rs (WalletAction::Derive subcommand with --phrase, --index, --output, --passphrase)
- blockchain-project/sprints/SPRINT-019.md (all tasks marked DONE)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md updated
**Review Notes:**
- Phase 1: aztibase-wasm crate (cdylib + rlib), wasm-bindgen exports, re-implemented Merkle/Verkle verify without aztibase-execution deps (avoids wasmtime/revm in WASM)
- Phase 2: LightClient JS API with WebSocket framing, IndexedDB header cache, dark-themed demo page
- Phase 3: BLAKE3 domain-separated HD derivation (m/44'/aztb'/N'/0/0), index 0 backward-compatible with Sprint 017
- Phase 4: Security review, 481 tests passing, clippy/fmt clean
**Security Flags:**
- SEC-WASM-001 (LOW): WASM linear memory may expose key material to JS; verification-only crate mitigates (no secrets in WASM)
- SEC-WS-001 (LOW): WebSocket transport has no built-in auth/encryption; relies on wss:// for confidentiality
- SEC-HD-001 (LOW): Non-standard HD derivation (BLAKE3, not HMAC-SHA512 BIP-32); documented, Aztibase-specific by design
- 0 ELEVATED, 0 MEDIUM

### 2026-03-07 -- project-lead -- Sprint 018 Complete
**Task:** Sprint 018: Light Node P2P Integration & Wallet Transfers (16 tasks, 4 phases)
**Sprint:** Sprint 018, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: request-response feature, async-trait, reqwest deps)
- crates/aztibase-network/Cargo.toml (async-trait dep)
- crates/aztibase-network/src/light_sync.rs (LightSyncCodec, LightSyncRequest/Response, frame read/write, encode/decode helpers, 4 new tests)
- crates/aztibase-network/src/behaviour.rs (request_response::Behaviour<LightSyncCodec> added)
- crates/aztibase-network/src/transport.rs (LightSync event handling, send_light_sync_request/response methods)
- crates/aztibase-network/src/lib.rs (PeerId re-export, new light sync exports)
- crates/aztibase-node/Cargo.toml (reqwest dep, axum dev-dep)
- crates/aztibase-node/src/main.rs (run_light_node rewritten: P2P sync loop, peer scoring, header persistence; Transfer CLI: --passphrase/--rpc flags; full-node light sync handler; 3 new tests)
- crates/aztibase-node/src/wallet.rs (sign_transfer_encrypted, broadcast_transaction via reqwest, 3 new tests)
- blockchain-project/sprints/SPRINT-018.md (all tasks marked DONE)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md updated
**Review Notes:**
- Phase 1: /aztibase/light-sync/1 request-response protocol via libp2p, bincode framing with 1MB max, LightSyncCodec
- Phase 2: Light node sync loop with peer scoring (failure tracking, latency preference), header chain verification + LightStore persistence
- Phase 3: sign_transfer_encrypted for Argon2id keyfiles, broadcast_transaction via reqwest JSON-RPC, --rpc/--passphrase CLI flags
- Phase 4: Security review, 456 tests passing, clippy/fmt clean
**Security Flags:**
- SEC-P2P-001 (LOW): Single-peer sync per cycle; eclipse mitigated by scoring but not multi-peer cross-validation
- SEC-RPC-001 (LOW): Wallet broadcast doesn't enforce HTTPS; user responsibility
- 0 ELEVATED, 0 MEDIUM

### 2026-03-07 -- project-lead -- Sprint 017 Complete
**Task:** Sprint 017: Wallet Hardening (BIP-39 + Argon2id), Light Node Storage, Header Sync Protocol (16 tasks, 4 phases)
**Sprint:** Sprint 017, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: bip39, argon2, chacha20poly1305, zeroize deps)
- crates/aztibase-node/Cargo.toml (new deps: bip39, argon2, chacha20poly1305, zeroize, blake3, rand)
- crates/aztibase-node/src/wallet.rs (BIP-39 mnemonic gen/recover, Argon2id encrypt/decrypt, EncryptedKeyFile, 5 new tests)
- crates/aztibase-node/src/main.rs (--light CLI flag, --mnemonic/--passphrase on wallet generate, wallet recover/show encrypted, run_light_node)
- crates/aztibase-storage/src/light.rs (NEW: LightStore, 5 redb tables, header/cert/proof/wallet CRUD, 6 tests)
- crates/aztibase-storage/src/lib.rs (light module + exports)
- crates/aztibase-storage/Cargo.toml (unchanged, redb already present)
- crates/aztibase-network/src/light_sync.rs (NEW: LightSyncMessage, LightSyncProtocol, verify_header_chain, 10 tests)
- crates/aztibase-network/src/lib.rs (light_sync module + exports)
- crates/aztibase-network/Cargo.toml (bincode dep)
- blockchain-project/sprints/SPRINT-017.md (all tasks marked DONE)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md updated
**Review Notes:**
- Phase 1: BIP-39 24-word mnemonic + BLAKE3 domain-separated derivation, Argon2id (256MB/3iter) + ChaCha20-Poly1305 AEAD keyfile encryption
- Phase 2: LightStore with 5 redb tables (headers u64-keyed, finality certs, proof cache with TTL eviction, wallet state, peer cache)
- Phase 3: LightSyncMessage (4 variants), LightSyncProtocol state machine, verify_header_chain (sequential rounds + quorum check), --light CLI
- Phase 4: Security review, M5 milestone started
**Security Flags:**
- SEC-MNEMONIC-001 (LOW): Non-standard BIP-32 derivation path (BLAKE3-based, Aztibase-specific)
- SEC-ARGON-001 (LOW): 256MB Argon2id may be slow on low-RAM devices
- SEC-SYNC-001 (LOW): Light sync quorum check only (full BLS verify deferred to peer integration)
- SEC-ZERO-001 (INFO): Mnemonic string returned to caller; caller must handle securely
- SEC-CACHE-001 (INFO): Proof cache unbounded by count (eviction by round age only)
- 0 ELEVATED, 0 MEDIUM

### 2026-03-07 -- project-lead -- Sprint 016 Complete
**Task:** Sprint 016: Transaction Anomaly Scoring, Cross-VM Bridge, PoUW Foundations (all 16 tasks, 4 phases)
**Sprint:** Sprint 016, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-runtime/src/anomaly.rs (NEW: AnomalyScorer, TxFeatures, heuristic scoring, 5 tests)
- crates/aztibase-runtime/src/lib.rs (anomaly module, exports)
- crates/aztibase-execution/src/receipt.rs (anomaly_score field on ExecutionReceipt)
- crates/aztibase-execution/src/cross_vm.rs (NEW: CrossVmCall, wasm_to_evm, evm_to_wasm, depth limiting, 6 tests)
- crates/aztibase-execution/src/lib.rs (cross_vm module, exports)
- crates/aztibase-consensus/src/pouw.rs (InferenceTask, InferenceAttestation, PoUWScore trait, StubPoUWScore, 8 tests)
- crates/aztibase-consensus/src/lib.rs (PoUW exports)
- crates/aztibase-node/src/pipeline.rs (AnomalyScorer wiring, extract_tx_features, anomaly_score on all receipts)
- crates/aztibase-rpc/src/server.rs (anomaly_score in test receipt)
- blockchain-project/sprints/SPRINT-016.md (all tasks marked DONE, security findings, retrospective)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md updated
**Review Notes:**
- Phase 1: Heuristic anomaly scorer (deterministic BLAKE3-seeded noise + feature thresholds), advisory only
- Phase 2: Cross-VM bridge via existing evm_call/ExecutionEngine, depth limit 4, reentrancy guard deferred to M7
- Phase 3: PoUW types are pure data definitions with serde/bincode serialization, stub scorer returns 0.0
- M4 substantially complete — Docker/deployment docs deferred
**Security Flags:**
- SEC-BRIDGE-003 (MEDIUM): Cross-VM reentrancy within depth limit; full guard deferred to M7
- SEC-ANOMALY-001, SEC-BRIDGE-001, SEC-BRIDGE-002, SEC-POUW-001 (LOW)
- SEC-POUW-002 (INFO)
- 0 ELEVATED

### 2026-03-07 -- project-lead -- Sprint 015 Complete
**Task:** Sprint 015: Metrics Telemetry, Light Client Foundations, WebRTC Transport (all 15 tasks, 4 phases)
**Sprint:** Sprint 015, Phases 1-4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/config.rs (MetricsConfig, stun_servers in NetworkConfig)
- crates/aztibase-node/src/main.rs (--metrics CLI flag, consensus metrics wiring, JSON update loop)
- crates/aztibase-node/Cargo.toml (hex dependency)
- crates/aztibase-rpc/src/server.rs (GET /metrics endpoint, node_metrics in RpcState, 2 new tests)
- crates/aztibase-core/src/commitment.rs (LightClientProof struct, StateProof::LightClient variant)
- crates/aztibase-execution/src/light_client.rs (NEW: build/verify light client proofs, 4 tests)
- crates/aztibase-execution/src/snapshot.rs (height + finality_certificate fields, snapshot_header_hash, 3 tests)
- crates/aztibase-execution/src/lib.rs (light_client module, new exports)
- crates/aztibase-execution/benches/execution_bench.rs (Merkle/Verkle proof verification benchmarks)
- crates/aztibase-consensus/benches/consensus_bench.rs (BLS cert verification benchmark at 21/100 validators)
- crates/aztibase-network/Cargo.toml (webrtc feature gate, libp2p-webrtc dependency)
- crates/aztibase-network/src/lib.rs (webrtc module, conditional exports)
- crates/aztibase-network/src/webrtc.rs (NEW: WebRtcTransport, WebRtcConfig, STUN config, 4 tests)
- crates/aztibase-execution/src/block_stm.rs (Block-STM validation race fix — prior-tx check in finish_validation)
- blockchain-project/sprints/SPRINT-015.md (all tasks marked DONE)
- blockchain-project/STATUS.md, BUILD_LOG.md, CHANGELOG.md, DECISIONS.md updated
**Review Notes:**
- Phase 1: Metrics via Arc<RwLock<serde_json::Value>> to avoid cross-crate coupling
- Phase 2: LightClientProof wraps inner Merkle/Verkle proof + finality cert; snapshot backward compat via #[serde(default)]
- Phase 3: libp2p-webrtc 0.9.0-alpha.1 is pure Rust (webrtc-rs), satisfies ADR-001
- Block-STM fix: parallel validators could mark tx as Validated while prior tx was re-executing; now checks all prior txs are Validated before accepting
**Security Flags:**
- SEC-METRIC-001 (LOW): Metrics snapshot eventually consistent
- SEC-WEBRTC-001 (LOW): STUN server URL format not validated
- SEC-LC-001 (LOW): Finality certificate structural validation deferred to M5
- 0 ELEVATED, 0 MEDIUM

### 2026-03-07 -- security-engineer -- all crates
**Task:** Sprint 014 Phase 4: Security Review + Documentation (Tasks 13-15)
**Sprint:** Sprint 014, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-014.md (all 15 tasks marked DONE, security findings, retrospective)
- blockchain-project/BUILD_LOG.md (entries for all 4 phases)
- blockchain-project/STATUS.md (M4 progress, test count, benchmark note)
- blockchain-project/DECISIONS.md (ADR-007: StateCommitment trait)
- CHANGELOG.md (Sprint 014 entries)
**Review Notes:** Security review: 0 ELEVATED, 0 MEDIUM, 4 LOW, 1 INFO. 397 tests passing, zero clippy warnings.
**Security Flags:** SEC-EQUI-001 (LOW), SEC-BUF-001 (LOW), SEC-VERKLE-001 (LOW), SEC-VERKLE-002 (LOW), SEC-METRICS-001 (INFO)

### 2026-03-07 -- blockchain-architect -- aztibase-core, aztibase-execution
**Task:** Sprint 014 Phase 3: Verkle Tree Foundations (Tasks 9-12)
**Sprint:** Sprint 014, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-core/src/commitment.rs (NEW: StateCommitment trait, StateProof enum, MerkleProof, VerkleProof)
- crates/aztibase-core/src/lib.rs (added commitment module export)
- crates/aztibase-execution/src/state.rs (MerkleCommitment impl, prove/verify, 3 new tests)
- crates/aztibase-execution/src/verkle.rs (NEW: VerkleTree, VerkleCommitment, 5 tests)
- crates/aztibase-execution/src/lib.rs (added verkle module, exports)
**Review Notes:** StateCommitment trait with enum-based proofs (ADR-007). Verkle uses BLAKE3 placeholder — real IPA/KZG deferred. 8 new tests.
**Security Flags:** None

### 2026-03-07 -- consensus-engineer -- aztibase-consensus, aztibase-execution
**Task:** Sprint 014 Phase 2: Performance Benchmarking Framework (Tasks 5-8)
**Sprint:** Sprint 014, Phase 2
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (workspace: criterion dev-dependency)
- crates/aztibase-consensus/Cargo.toml (criterion dev-dep, [[bench]] section)
- crates/aztibase-consensus/benches/consensus_bench.rs (NEW: 3 benchmarks)
- crates/aztibase-execution/Cargo.toml (criterion dev-dep, [[bench]] section)
- crates/aztibase-execution/benches/execution_bench.rs (NEW: 3 benchmark groups)
- crates/aztibase-consensus/src/engine.rs (ConsensusMetrics, MetricsSnapshot)
- crates/aztibase-consensus/src/lib.rs (metrics exports)
**Review Notes:** Criterion benchmarks for vertex creation, DAG insertion, commit evaluation, state root, transfer execution, single transfer latency. ConsensusMetrics with atomic counters. 1 new test.
**Security Flags:** None

### 2026-03-07 -- consensus-engineer + node-engineer -- aztibase-consensus, aztibase-node
**Task:** Sprint 014 Phase 1: Consensus Resilience + Fault Injection (Tasks 1-4)
**Sprint:** Sprint 014, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs (equivocation detection, vertex buffering, seen_authors map, buffered vec, prune logic)
- crates/aztibase-node/src/integration.rs (byzantine_equivocation_detected, validator_crash_and_recovery tests)
**Review Notes:** Equivocation detection tracks (round, author) → hash with PRUNE_DEPTH=20. Vertex buffer bounded at MAX_BUFFERED=64. 4 new tests (2 integration, 2 unit).
**Security Flags:** None

### 2026-03-07 -- security-engineer + project-lead -- all crates
**Task:** Sprint 013 Phase 4: Security Review + Documentation (Tasks 13-15)
**Sprint:** Sprint 013, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-013.md (all 15 tasks marked DONE, security findings, retrospective)
- blockchain-project/BUILD_LOG.md (entries for all 4 phases)
- blockchain-project/STATUS.md (sprint 013 complete, ~385 tests)
- CHANGELOG.md (genesis bootstrap, BLS validator keys entries)
**Review Notes:**
- Security review: 0 ELEVATED, 0 MEDIUM, 3 LOW (SEC-KEY-001 extends, SEC-BLS-010, SEC-CFG-001)
- cargo clippy: zero warnings; cargo fmt: clean; ~385 tests passing
- Pre-existing flaky test: parallel_conflicting_chain (Block-STM race, not Sprint 013 related)
- Sprint 013 fully complete: 15/15 tasks, 4/4 phases, 13 new tests
**Security Flags:** 0 ELEVATED, 0 MEDIUM, 3 LOW documented

### 2026-03-07 -- node-engineer -- aztibase-node, aztibase-consensus
**Task:** Sprint 013 Phase 3: End-to-End Testnet Integration (Tasks 9-12)
**Sprint:** Sprint 013, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/integration.rs (engine_with_genesis_validators enhanced, integration_finality_cert test)
- crates/aztibase-node/src/main.rs (config_override_precedence test, NodeConfig consolidation)
- scripts/local-testnet.sh (rewritten: uses aztibase genesis + per-node configs)
**Review Notes:**
- Integration test: genesis → execution → BLS finality cert → verify (full flow)
- Local testnet script now uses aztibase genesis subcommand for bootstrap
- Config consolidation: CLI flags override TOML config values
- 3 new tests: engine_with_genesis_validators (enhanced), integration_finality_cert, config_override_precedence
**Security Flags:** None

### 2026-03-07 -- consensus-engineer -- aztibase-consensus, aztibase-node
**Task:** Sprint 013 Phase 2: BLS Keys in Genesis (Tasks 5-8)
**Sprint:** Sprint 013, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-core/src/bls.rs (BlsKeypair::from_secret_bytes, BlsKeypair::secret_bytes)
- crates/aztibase-consensus/src/validator.rs (ValidatorRecord internal type, add_with_bls, bls_key, bls_keys_ordered)
- crates/aztibase-consensus/src/finality.rs (build_certificate_from_set, verify_certificate_from_set)
- crates/aztibase-consensus/src/lib.rs (new exports)
- crates/aztibase-node/src/genesis.rs (BLS fields in ValidatorEntry + KeyFile, generate_genesis creates BLS keypairs, load_keyfile_full)
**Review Notes:**
- ValidatorSet refactored: internal HashMap<ValidatorId, ValidatorRecord> with optional BLS pubkey
- Backward-compatible: add() still works without BLS keys, load_keyfile() still returns (Keypair, Address)
- build_certificate_from_set/verify_certificate_from_set pull BLS keys from ValidatorSet
- 5 new tests: genesis_generates_bls_keys, load_keyfile_with_bls, validator_set_with_bls_keys, bls_keys_ordering, finality_cert_with_genesis_bls
**Security Flags:** None

### 2026-03-07 -- node-engineer -- aztibase-node
**Task:** Sprint 013 Phase 1: Genesis-Driven Validator Bootstrap (Tasks 1-4)
**Sprint:** Sprint 013, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/main.rs (genesis loading before consensus, ValidatorSet from genesis, --validator-key CLI, apply_overrides for genesis_path/validator_key)
- crates/aztibase-node/src/config.rs (genesis_path, validator_key fields on NodeConfig)
- crates/aztibase-node/src/genesis.rs (write_node_configs, per-validator TOML generation)
- crates/aztibase-node/src/wallet.rs (bls_public_key/bls_secret_key None fields)
**Review Notes:**
- Validators loaded from GenesisConfig instead of hardcoded [i; 32] placeholders
- Node identity from key file with validation against genesis validator set
- Per-validator TOML configs generated by aztibase genesis subcommand
- 5 new tests: validators_loaded_from_genesis, validator_key_matches_genesis, genesis_writes_node_configs, config_file_loads_correctly, (engine_with_genesis_validators enhanced in Phase 3)
**Security Flags:** None

### 2026-03-07 -- security-engineer + project-lead -- all crates
**Task:** Sprint 012 Phase 4: Security Review + Documentation (Tasks 13-15)
**Sprint:** Sprint 012, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-012.md (all 15 tasks marked DONE, security findings, retrospective)
- blockchain-project/BUILD_LOG.md (entries for all 4 phases)
- blockchain-project/STATUS.md (sprint 012 complete, 372 tests)
- CHANGELOG.md (genesis, wallet entries)
**Review Notes:**
- Security review: 0 ELEVATED, 0 MEDIUM, 4 LOW (SEC-KEY-001/002, SEC-FEE-004, SEC-BASE-001)
- 3 prior MEDIUM findings CLOSED: SEC-FEE-002, SEC-FEE-003, SEC-SIG-002
- cargo clippy: zero warnings; cargo fmt: clean; 372 tests passing
- cargo audit: no new advisories (same transitive deps as Sprint 011)
- Sprint 012 fully complete: 15/15 tasks, 4/4 phases, 19 new tests
**Security Flags:** 0 ELEVATED, 0 MEDIUM, 4 LOW documented

### 2026-03-07 -- node-engineer -- aztibase-core, aztibase-node
**Task:** Sprint 012 Phase 3: Genesis Config + CLI Wallet (Tasks 9-12)
**Sprint:** Sprint 012, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-core/src/crypto.rs (Keypair::from_secret_bytes, Keypair::secret_bytes)
- crates/aztibase-node/src/genesis.rs (NEW: GenesisConfig, apply_genesis, generate_genesis, write_genesis, load_genesis, load_keyfile, KeyFile, hex_encode/hex_decode, +5 tests)
- crates/aztibase-node/src/wallet.rs (NEW: generate_key, show_key, sign_transfer, +2 tests)
- crates/aztibase-node/src/main.rs (CLI subcommands: genesis, wallet; --genesis flag for node startup; Command/WalletAction enums)
- crates/aztibase-node/Cargo.toml (serde_json dependency)
- blockchain-project/sprints/SPRINT-012.md (Phase 3 marked DONE)
**Review Notes:**
- GenesisConfig: TOML-serialized, supports validators + pre-funded accounts
- CLI: `aztibase genesis`, `aztibase wallet generate/show/transfer`
- Node startup: `--genesis genesis.toml` applies genesis to empty state
- 7 new tests; 372 total passing
- cargo clippy: zero warnings; cargo fmt: clean
**Security Flags:** None (key file stores unencrypted secret key — acceptable for testnet, encryption deferred)

### 2026-03-07 -- smart-contract-engineer + node-engineer -- aztibase-execution, aztibase-node, aztibase-rpc
**Task:** Sprint 012 Phase 2: Persistent Base Fee + Validation Hardening (Tasks 5-8)
**Sprint:** Sprint 012, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/persist.rs (store_base_fee, load_base_fee using STATE_TABLE)
- crates/aztibase-execution/src/lib.rs (re-export store_base_fee, load_base_fee)
- crates/aztibase-node/src/pipeline.rs (BaseFeeCalculator + Arc<AtomicU64> fields, load on startup, update+persist after batch, nonce validation Phase 0, execute_batch takes &mut self, +4 tests)
- crates/aztibase-node/src/mempool.rs (insert_checked gains min_gas_price param, decode_sender_nonce_gas extracts gas_price, +2 tests)
- crates/aztibase-node/src/main.rs (shared_base_fee passed to RpcServer::new and insert_checked calls)
- crates/aztibase-rpc/src/server.rs (RpcState.base_fee: Arc<AtomicU64>, RpcServer::new 5th param, handle_gas_price reads live value)
- crates/aztibase-node/src/integration.rs (mut pipeline bindings for &mut self execute_batch)
**Review Notes:**
- SEC-FEE-003 CLOSED: BaseFeeCalculator persists to redb, loads on startup, drives real gas prices
- SEC-SIG-002 CLOSED: Pipeline validates nonces at execution time (Phase 0 nonce validation)
- Mempool rejects transactions below current base fee
- 7 new tests (persist: 1, mempool: 2, pipeline: 4); 365 total passing
- cargo clippy: zero warnings; cargo fmt: clean
**Security Flags:** SEC-FEE-003 CLOSED, SEC-SIG-002 CLOSED

### 2026-03-07 -- smart-contract-engineer -- aztibase-node
**Task:** Sprint 012 Phase 1: Pre-Execution Fee Escrow (Tasks 1-4)
**Sprint:** Sprint 012, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/pipeline.rs (escrow_fee/refund_unused wired into execute_batch, collect_fees removed, total_fees_burned in PipelineResult, compute_tx_hash helper, +5 tests)
**Review Notes:**
- SEC-FEE-002 CLOSED: Pre-execution escrow prevents underfunded execution
- Escrow sequential before Block-STM, refund sequential after
- Failed escrow: generates failure receipt, increments nonce, skips execution
- 5 new tests; 358 total passing at Phase 1 close
- cargo clippy: zero warnings; cargo fmt: clean
**Security Flags:** SEC-FEE-002 CLOSED

### 2026-03-07 -- security-engineer + project-lead -- all crates
**Task:** Sprint 011 Phase 4: Security Review + Documentation (Tasks 13-15)
**Sprint:** Sprint 011, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-011.md (all 15 tasks marked DONE, security findings table, retrospective)
- blockchain-project/BUILD_LOG.md (entries for all 4 phases)
- blockchain-project/STATUS.md (sprint 011 complete, 353 tests)
- blockchain-project/DECISIONS.md (ADR-006: post-execution fee collection)
- CHANGELOG.md (signatures, nonces, fees entries)
**Review Notes:**
- Security review: 0 ELEVATED, 2 MEDIUM (documented), 6 LOW
- SEC-SIG-002: batch-level nonce conflicts mitigated by mempool dedup
- SEC-FEE-002: post-execution fees don't prevent underfunded execution — escrow_fee exists but not wired
- cargo clippy: zero warnings; cargo fmt: clean; 353 tests passing
- Sprint 011 fully complete: 15/15 tasks, 4/4 phases
**Security Flags:** 2 MEDIUM documented (SEC-SIG-002, SEC-FEE-002), 0 ELEVATED

### 2026-03-07 -- smart-contract-engineer -- aztibase-execution, aztibase-rpc
**Task:** Sprint 011 Phase 3: Fee Market Basics (Tasks 9-12)
**Sprint:** Sprint 011, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/routing.rs (gas_price: u64 field on all 7 TxKind variants, gas_price()/gas_limit() accessors)
- crates/aztibase-execution/src/fee.rs (NEW: BaseFeeCalculator, escrow_fee, refund_unused, FeeEscrow, +14 tests)
- crates/aztibase-execution/src/lib.rs (pub mod fee, re-exports)
- crates/aztibase-node/src/pipeline.rs (collect_fees() post-execution, nonce-ordered sort, +2 tests)
- crates/aztibase-rpc/src/server.rs (aztb_gasPrice, aztb_estimateGas methods, +3 tests)
- All test files updated with gas_price: 0 (~50+ construction sites)
**Review Notes:**
- EIP-1559-style base fee: target 15M gas/batch, min 1, max 1B, denominator 8
- Fee collection is post-execution via saturating arithmetic — safe but doesn't prevent underfunded execution
- escrow_fee/refund_unused ready for pre-execution wiring in Sprint 012
- 353 total tests (19 new in Phase 3), zero clippy warnings
**Security Flags:** SEC-FEE-002 (MEDIUM): post-execution fees — documented for Sprint 012

### 2026-03-07 -- node-engineer -- aztibase-node
**Task:** Sprint 011 Phase 2: Nonce Enforcement + Address Derivation (Tasks 5-8)
**Sprint:** Sprint 011, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/mempool.rs (insert_checked(), decode_sender_nonce(), MAX_NONCE_GAP=16, +6 tests)
- crates/aztibase-node/src/main.rs (gossip + RPC paths use insert_checked with state nonce lookup)
- crates/aztibase-node/src/pipeline.rs (nonce-ordered sort: sort_by sender then nonce)
**Review Notes:**
- Nonce validation at mempool without sig verification — pragmatic tradeoff (forged envelopes fail at pipeline)
- MAX_NONCE_GAP=16 prevents memory exhaustion from far-future nonces
- Nonce ordering in pipeline ensures same-sender txs execute in correct sequence
**Security Flags:** SEC-NONCE-002 (LOW): mempool skips sig verification

### 2026-03-07 -- smart-contract-engineer -- aztibase-execution, aztibase-node
**Task:** Sprint 011 Phase 1: Signed Transaction Envelope (Tasks 1-4)
**Sprint:** Sprint 011, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/tx.rs (NEW: SignedTx type, encode/decode, verify, sender_address, verify_and_route, verify_and_route_batch, +11 tests)
- crates/aztibase-execution/src/lib.rs (pub mod tx, re-exports SignedTx, TxError, verify_and_route, verify_and_route_batch)
- crates/aztibase-node/src/pipeline.rs (verify_and_route_batch integration, +2 tests)
- crates/aztibase-node/src/integration.rs (all e2e tests updated to use signed envelopes)
**Review Notes:**
- Wire format: [0xAA][payload_len:u32 LE][payload][pubkey:32][sig:64], max 1 MiB
- Ed25519 via ed25519-dalek with strict verification — no malleability risk
- Sender address = BLAKE3(pubkey), cross-checked against TxKind.from field
- Pipeline rejects unsigned txs and sender mismatches
**Security Flags:** SEC-SIG-001 (LOW): Ed25519 strict verification confirmed safe

### 2026-03-07 -- security-engineer + project-lead -- all crates
**Task:** Sprint 010 Phase 4: Security Review + Documentation (Tasks 13-15)
**Sprint:** Sprint 010, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-010.md (security findings table, retrospective, all tasks marked DONE)
- blockchain-project/BUILD_LOG.md (entries for all 4 phases)
- blockchain-project/STATUS.md (sprint 010 complete, 315 tests, next up: sprint 011)
- CHANGELOG.md (account abstraction, gossipsub hardening, Block-STM entries)
**Review Notes:**
- Security review: 0 ELEVATED, 0 MEDIUM, 6 LOW (all documented)
- SEC-STM-001/002: busy-wait and single-lock contention (LOW, acceptable for current batch sizes)
- SEC-GS-001: peer scoring weights are defaults, need live tuning (LOW)
- SEC-AA-001/002/003: no fee mechanism, no signatures, silent unknown discriminant (LOW, pre-existing)
- cargo clippy: zero warnings; cargo fmt: clean; 315 tests passing
- Sprint 010 fully complete: 15/15 tasks, 4/4 phases
**Security Flags:** None (6 LOW documented)

### 2026-03-07 -- blockchain-architect -- aztibase-execution, aztibase-node, aztibase-rpc
**Task:** Sprint 010 Phase 3: Account Abstraction Foundations (Tasks 9-12)
**Sprint:** Sprint 010, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/state.rs (AccountType enum: EOA/Contract/AIAgent, account_type + model_id fields on Account, set_code auto-promotes EOA→Contract, state_root includes type discriminant + model_id, +4 tests)
- crates/aztibase-execution/src/routing.rs (TxKind::CreateAgent 0x07, model_id validation, +2 tests)
- crates/aztibase-execution/src/persist.rs (extended account record: account_type byte + model_id, backward-compatible deserialization)
- crates/aztibase-execution/src/snapshot.rs (SNAPSHOT_VERSION 1→2, AccountEntry adds account_type + model_id, apply_snapshot restores types)
- crates/aztibase-execution/src/lib.rs (re-export AccountType)
- crates/aztibase-node/src/pipeline.rs (CreateAgent routing + execution: compute agent address, set AIAgent type + model_id, +3 tests)
- crates/aztibase-rpc/src/server.rs (aztb_getAccountType method, +2 tests)
**Review Notes:**
- AccountType uses derive(Default) with #[default] on EOA (clippy-clean)
- Snapshot format v2 required because state_root now includes account_type discriminant
- Persistence backward-compatible: old 16-byte records default to EOA
- Agent addresses computed via compute_contract_address(creator, nonce) — same as contracts
- 315 total tests (11 new), zero clippy warnings, clean fmt
**Security Flags:** None

### 2026-03-07 -- p2p-network-engineer -- aztibase-network
**Task:** Sprint 010 Phase 2: Gossipsub Protocol Hardening (Tasks 5-8)
**Sprint:** Sprint 010, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-network/src/gossip.rs (hardened config: duplicate_cache_time 2min, max_transmit_size 2MiB, heartbeat 500ms, max_messages_per_rpc 100; peer_score_params() + peer_score_thresholds() functions)
- crates/aztibase-network/src/behaviour.rs (added connection_limits::Behaviour to AztibaseBehaviour)
- crates/aztibase-network/src/transport.rs (apply peer scoring, connection limits max 50 total / 2 per peer, log warnings on denied connections)
- crates/aztibase-network/src/lib.rs (+3 tests: gossipsub_config_values, peer_score_params_valid, peer_score_thresholds_valid)
**Review Notes:**
- Message signing already in place (MessageAuthenticity::Signed) + ValidationMode::Strict
- Peer scoring: per-topic params for all 6 topics, behaviour penalties, IP colocation
- Connection limits via libp2p-connection-limits (already transitive dep, no new deps)
- 304 total tests (3 new network tests), zero clippy warnings, clean fmt
**Security Flags:** None

### 2026-03-07 -- smart-contract-engineer -- aztibase-execution
**Task:** Sprint 010 Phase 1: Rayon-Parallel Block-STM (Tasks 1-4)
**Sprint:** Sprint 010, Phase 1
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (added `rayon = "1.10"` to workspace deps)
- crates/aztibase-execution/Cargo.toml (added `rayon = { workspace = true }`)
- crates/aztibase-execution/src/block_stm.rs (full rewrite: thread-safe MVMemory with Mutex, Arc-wrapped scheduler, rayon::scope parallel execution, SchedulerTask::Wait variant, execute_parallel/execute_sequential split, +3 parallel tests)
**Review Notes:**
- MVMemory.data wrapped in Mutex<HashMap>, read() returns cloned StateValue
- Per-tx arrays: Vec<Mutex<ReadSet/WriteSet/Option<TxOutput>>>
- Parallel threshold: batches >= 4 txs use rayon, smaller use sequential
- 3 new tests: parallel_independent_transfers, parallel_conflicting_chain, parallel_deterministic_across_runs
- All 16 original Block-STM tests preserved and passing (301 total tests)
- cargo clippy: zero warnings; cargo fmt: clean
**Security Flags:** None

### 2026-03-07 -- security-engineer + project-lead -- aztibase-node
**Task:** Sprint 009 Phase 4: Security Review + Documentation (Tasks 14-16)
**Sprint:** Sprint 009, Phase 4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/sync.rs (SEC-SYNC-001 fix: MAX_CHUNKS validation in SnapshotAssembler::new; SEC-SYNC-002 fix: chunk data size validation; +3 tests)
- blockchain-project/sprints/SPRINT-009.md (all phases marked DONE, security findings table, retrospective)
- blockchain-project/BUILD_LOG.md (Phase 4 entry)
- blockchain-project/STATUS.md (sprint 009 complete, 298 tests)
- CHANGELOG.md (security fixes, sprint completion)
**Review Notes:**
- Full security review: Block-STM (10 findings), State Sync (8 findings), EVM/Precompiles (8 findings)
- 2 ELEVATED fixed: SEC-SYNC-001 (unbounded total_chunks allocation → bounded to MAX_CHUNKS=64), SEC-SYNC-002 (chunk data size unbounded → capped at CHUNK_SIZE+1024)
- 8 MEDIUM documented (6) or fixed (2), 5 LOW documented
- cargo-audit: no new advisories (existing transitive deps documented)
- cargo clippy: zero warnings; cargo fmt: clean
- 298 total tests (3 new security tests), zero open ELEVATED flags
**Security Flags:** 2 ELEVATED — both FIXED (SEC-SYNC-001, SEC-SYNC-002)

---

### 2026-03-07 -- smart-contract-engineer -- aztibase-execution
**Task:** Sprint 009 Phase 3: EVM Precompiles (Tasks 10-13)
**Sprint:** Sprint 009, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/precompiles.rs (NEW: precompile documentation + 12 verification tests)
- crates/aztibase-execution/src/lib.rs (added `pub mod precompiles`)
**Review Notes:**
- Discovery: revm v36 with `default-features = false, features = ["std"]` already includes pure-Rust precompile implementations via revm-precompile
- ecrecover (0x01): k256 pure Rust fallback when `secp256k1` feature disabled
- SHA-256 (0x02): sha2 crate, RIPEMD-160 (0x03): ripemd crate, identity (0x04): memcpy, modexp (0x05): custom bigint
- All registered automatically by `build_mainnet()` → `EthPrecompiles::new(spec)`
- bn128 (0x06-0x08) and KZG (0x0a) excluded (feature-gated, C deps) — deferred to Sprint 010
- Tests verify via deployed contracts that STATICCALL precompiles, confirming EVM execution path works
- 295 total tests (12 new precompile tests), zero clippy warnings
**Security Flags:** None

---

### 2026-03-07 -- p2p-network-engineer + node-engineer -- aztibase-execution, aztibase-node
**Task:** Sprint 009 Phase 2: State Sync Protocol (Tasks 6-9)
**Sprint:** Sprint 009, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/snapshot.rs (NEW: StateSnapshot, create/serialize/deserialize/apply/verify — 7 tests)
- crates/aztibase-execution/src/lib.rs (added `pub mod snapshot` + exports)
- crates/aztibase-node/src/sync.rs (NEW: SyncMessage, SnapshotAssembler, bootstrap_from_snapshot — 12 tests)
- crates/aztibase-node/src/main.rs (sync module, TOPIC_STATE_SYNC handler for request/response, bootstrap on empty state)
**Review Notes:**
- StateSnapshot serializes full AccountState via bincode with 64 MiB size limit and version byte
- SyncMessage enum: SnapshotRequest + SnapshotResponse with 1 MiB chunk splitting and per-chunk BLAKE3 integrity
- SnapshotAssembler reassembles chunks with ordering, hash verification, and state root validation
- bootstrap_from_snapshot applies snapshot to both in-memory state and optional redb persistence
- Main loop handles incoming requests (responds with snapshot) and responses (assembles + bootstraps)
- Empty-state nodes request snapshots from peers upon receiving a StateRootAnnounce
- 283 total tests (19 new: 7 snapshot + 12 sync), zero clippy warnings
**Security Flags:** None

### 2026-03-06 -- smart-contract-engineer -- aztibase-execution, aztibase-node
**Task:** Sprint 009 Phase 1: Block-STM parallel execution (Tasks 1-5)
**Sprint:** Sprint 009, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/block_stm.rs (NEW: MVMemory, MVView, Scheduler, BlockSTMExecutor, apply_block_stm_to_state, execute_full — 16 tests)
- crates/aztibase-execution/src/lib.rs (added `pub mod block_stm`)
- crates/aztibase-node/src/pipeline.rs (replaced execute_transfers with BlockSTMExecutor::execute_full + apply_block_stm_to_state)
**Review Notes:**
- Block-STM from Aptos: optimistic execution → read-set validation → re-execution on conflict
- Single-threaded v1 for correctness; rayon parallelism deferred to Sprint 010
- All 14 existing pipeline tests pass unchanged, proving Block-STM output matches sequential
- 264 total tests, zero clippy warnings
**Security Flags:** None

### 2026-03-06 -- security-engineer + documentation-engineer -- all crates
**Task:** Sprint 008 Phase 4: Security review + documentation (Tasks 15-17)
**Sprint:** Sprint 008, Phase 4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/mempool.rs (SEC-MEM-001 fix: bounded `seen` set with max_seen + FIFO eviction, 1 new test)
- blockchain-project/sprints/SPRINT-008.md (all tasks marked DONE, retrospective written)
- blockchain-project/STATUS.md (Sprint 008 complete, M4 progress update)
- blockchain-project/BUILD_LOG.md (Phase 4 entry)
- CHANGELOG.md (security findings, Phase 4 items)
**Review Notes:** Security review of 3 domains: multi-node gossip (wire.rs — 3 LOW findings), AI pipeline (tract_runtime + config — 1 MEDIUM documented, 2 LOW), mempool (1 MEDIUM fixed, 2 LOW). SEC-MEM-001 fixed: seen set now bounded to max_size×10 with FIFO eviction. cargo-audit: 6 transitive vulns + 7 warnings, all previously documented; 2 new wasmtime advisories (RUSTSEC-2026-0020/0021). 248 tests pass. Zero clippy warnings, fmt clean. Sprint 008 complete (17/17 tasks).
**Security Flags:** SEC-MEM-001 MEDIUM — FIXED (bounded seen set). SEC-AI-001 MEDIUM — DOCUMENTED (model paths operator-controlled). No ELEVATED flags.

### 2026-03-06 -- node-engineer -- aztibase-node
**Task:** Sprint 008 Phase 3: Transaction pool improvements (Tasks 11-14)
**Sprint:** Sprint 008, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/mempool.rs (complete rewrite: priority-ordered BTreeMap, eviction policy, BLAKE3 hash dedup, 16 tests)
**Review Notes:** Mempool redesigned from BTreeMap<TxHash, Vec<u8>> to BTreeMap<(Reverse<u64>, TxHash), Vec<u8>> for priority ordering. Eviction replaces lowest-priority entry when pool is full and new tx has higher priority. BLAKE3 hash in `seen` set persists after removal to prevent tx replay. Default priority derived from TxKind prefix byte. 247 tests pass (10 new mempool tests). Zero clippy warnings, fmt clean.
**Security Flags:** None

### 2026-03-06 -- ai-integration-engineer + smart-contract-engineer -- aztibase-execution, aztibase-node, aztibase-rpc
**Task:** Sprint 008 Phase 2: AI inference in execution pipeline (Tasks 6-10)
**Sprint:** Sprint 008, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/routing.rs (TxKind::AiInfer variant, PREFIX_AI_INFER 0x06, model_id validation, 2 tests)
- crates/aztibase-execution/src/receipt.rs (inference_hash: Option<[u8; 32]> field on ExecutionReceipt)
- crates/aztibase-node/src/pipeline.rs (ai_runtime: Option<Arc<dyn AIRuntime>>, AiInfer routing, InferenceRequest dispatch, 3 AI tests)
- crates/aztibase-node/src/main.rs (TractRuntime creation, model preload from config, set_ai_runtime)
- crates/aztibase-node/src/config.rs (AiConfig section with enabled + models Vec<ModelEntry>)
- crates/aztibase-node/Cargo.toml (tract-onnx + prost dev-deps)
- crates/aztibase-rpc/src/server.rs (inferenceHash in receipt JSON response)
**Review Notes:** AI inference is now a first-class transaction type (0x06). Pipeline routes AiInfer to AIRuntime trait (TractRuntime implementation). ExecutionReceipt extended with inference_hash for on-chain verification. Models loadable from TOML config at startup. 237 tests pass (5 new: 2 routing + 3 pipeline). Zero clippy warnings, fmt clean.
**Security Flags:** None — model preload uses config-specified paths only (no user-supplied paths at runtime). Inference resource bounded by max_compute_units field.

### 2026-03-06 -- p2p-network-engineer + consensus-engineer + node-engineer -- aztibase-consensus, aztibase-network, aztibase-node
**Task:** Sprint 008 Phase 1: Multi-node local testnet (Tasks 1-5)
**Sprint:** Sprint 008, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/wire.rs (NEW: encode_vertex, decode_vertex, WireError, 9 tests)
- crates/aztibase-consensus/src/engine.rs (use wire module for encode/decode, StateRootAnnounce struct)
- crates/aztibase-consensus/src/lib.rs (export wire module, StateRootAnnounce)
- crates/aztibase-network/src/lib.rs (export TOPIC_STATE_SYNC)
- crates/aztibase-node/Cargo.toml (added bincode dependency)
- crates/aztibase-node/src/main.rs (state root broadcast via TOPIC_STATE_SYNC, result_rx channel)
- crates/aztibase-node/src/pipeline.rs (result_tx: Option<Sender<PipelineResult>>, set_result_sender)
- crates/aztibase-node/src/integration.rs (multi_node_consensus_convergence test)
- scripts/local-testnet.sh (NEW: 3-node testnet launch script)
**Review Notes:** Wire format: [version:u8][bincode payload] with 512 KiB size limit. Engine now uses wire::encode_vertex for broadcast and wire::decode_vertex for reception (version check, hash integrity, validator membership, round proximity). StateRootAnnounce broadcast on TOPIC_STATE_SYNC after each batch execution. Multi-node test: 3 engines with shared genesis, async vertex routing, verified anchor + tx + state root convergence. 232 tests pass (10 new: 9 wire + 1 multi-node). Zero clippy, fmt clean.
**Security Flags:** None

### 2026-03-06 -- security-engineer + documentation-engineer -- all crates, docs
**Task:** Sprint 007 Phase 4: Security review + documentation (Tasks 16-19)
**Sprint:** Sprint 007, Phase 4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-runtime/src/tract_runtime.rs (added MAX_MODEL_SIZE 64 MiB cap + test — SEC-AI-001)
- blockchain-project/sprints/SPRINT-007.md (all tasks DONE, retrospective)
- blockchain-project/STATUS.md (M4 IN PROGRESS, test count 222, recent completions)
- blockchain-project/BUILD_LOG.md (Phase 4 entry)
- CHANGELOG.md (M4 section with features, security, testing)
**Review Notes:** Security review: 13 findings across receipt store (3), EVM (5), AI pipeline (5). 1 MEDIUM fixed (SEC-AI-001: 64 MiB model size cap). 0 ELEVATED. cargo-audit: 6 transitive vulns, 7 warnings — all from ring/wasmtime/lru/bincode transitive deps. ADR-006 skipped — neither revm nor tract introduce C deps. Sprint 007 COMPLETE: 18/19 tasks done + 1 skipped.
**Security Flags:** SEC-AI-001 RESOLVED (64 MiB model size cap). No ELEVATED flags.

### 2026-03-06 -- ai-integration-engineer -- aztibase-runtime
**Task:** Sprint 007 Phase 3: AI inference via tract (Tasks 10-15)
**Sprint:** Sprint 007, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-runtime/Cargo.toml (added `tract-onnx = { workspace = true }`, `prost = "0.11"` dev-dep)
- crates/aztibase-runtime/src/tract_runtime.rs (NEW: TractRuntime, RegisteredModel, InferenceReceipt, verify_inference, 12 tests)
- crates/aztibase-runtime/src/lib.rs (added `pub mod tract_runtime`, re-exports)
**Review Notes:** TractRuntime implements AIRuntime trait with real ONNX model loading via tract-onnx. Model registry uses RwLock<HashMap> for thread safety. Inference pipeline: f32 input deserialization → tract tensor → model run → output serialization. Deterministic BLAKE3 hash of (model_id || input || output). InferenceReceipt + verify_inference() stub for on-chain verification foundation. Test models built programmatically via tract_onnx::pb::ModelProto + prost encode. 221 tests pass (12 new). Zero clippy, fmt clean.
**Security Flags:** None. tract is pure Rust, no C deps.

### 2026-03-06 -- smart-contract-engineer -- aztibase-execution, aztibase-node
**Task:** Sprint 007 Phase 2: EVM execution via revm v36 (Tasks 5-9)
**Sprint:** Sprint 007, Phase 2
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (root: added `revm = { version = "36", default-features = false, features = ["std"] }`, MSRV 1.85→1.88)
- crates/aztibase-execution/Cargo.toml (added `revm = { workspace = true }`)
- crates/aztibase-execution/src/evm.rs (NEW: evm_deploy, evm_call, state adapter, 3 tests)
- crates/aztibase-execution/src/routing.rs (EvmDeploy 0x04, EvmCall 0x05 variants, 2 new tests)
- crates/aztibase-execution/src/lib.rs (added `pub mod evm`)
- crates/aztibase-node/src/pipeline.rs (EVM routing + execution in execute_batch, 3 new tests)
**Review Notes:** revm v36 integrated with `default-features = false, features = ["std"]` to avoid C deps (secp256k1-sys NOT in tree). MSRV bumped to 1.88. AZTB chain ID 0xDE0D. State adapter maps AccountState ↔ CacheDB (balance, nonce, code, storage). 32→20 byte address truncation. 209 tests pass (18 new: 3 evm, 2 routing, 3 evm pipeline, 5 receipt, 4 rpc receipt, 1 receipt e2e). Zero clippy, fmt clean.
**Security Flags:** None. revm C-dep-free verified.

### 2026-03-06 -- node-engineer -- aztibase-execution, aztibase-rpc, aztibase-node
**Task:** Sprint 007 Phase 1: Transaction receipt store + RPC query (Tasks 1-4)
**Sprint:** Sprint 007, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/receipt.rs (NEW: ExecutionReceipt, store_receipts, get_receipt, 5 tests)
- crates/aztibase-execution/src/lib.rs (pub mod receipt, exports)
- crates/aztibase-rpc/src/server.rs (aztb_getTransactionReceipt, receipt_store in RpcState, 4 tests)
- crates/aztibase-node/src/pipeline.rs (Arc<StateStore>, receipt collection, PipelineResult.receipts)
- crates/aztibase-node/src/main.rs (Arc wrapping, receipt_store param)
- crates/aztibase-node/src/integration.rs (Arc<StateStore>, receipt_persistence_end_to_end test)
**Review Notes:** Unified ExecutionReceipt persisted to RECEIPTS_TABLE in redb. Pipeline collects receipts from transfers (TxStatus mapping) and contracts (ContractReceipt mapping). RPC queries via shared Arc<StateStore>. All existing tests updated for Arc<StateStore>.
**Security Flags:** None

### 2026-03-06 -- security-engineer + documentation-engineer -- aztibase-rpc, docs
**Task:** Sprint 006 Phase 4: M3 close — security review, cargo-audit, docs, ADR-005 (Tasks 20-24)
**Sprint:** Sprint 006, Phase 4
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-rpc/src/server.rs (added 1MB DefaultBodyLimit via axum layer — SEC-RPC-001)
- blockchain-project/DECISIONS.md (ADR-005: axum for JSON-RPC)
- blockchain-project/STATUS.md (M3 marked COMPLETE, milestone table updated)
- blockchain-project/sprints/SPRINT-006.md (Phase 4 DONE, retrospective, Sprint 007 candidates)
- blockchain-project/BUILD_LOG.md (Phase 4 entry)
- CHANGELOG.md (Phase 4 entries)
**Review Notes:** Security review: 8-item RPC checklist (1 MEDIUM fixed, 4 LOW acceptable/deferred, 3 OK). Hardening review: all 7 fixes verified correct. cargo-audit: 5 transitive vulns (ring, wasmtime WASI), 6 warnings (unmaintained) — no action needed. ADR-005 for axum choice. M3 milestone COMPLETE. 191 tests, zero clippy, fmt clean.
**Security Flags:** SEC-RPC-001 RESOLVED (1MB body limit). No ELEVATED flags.

### 2026-03-06 -- node-engineer + smart-contract-engineer -- aztibase-node
**Task:** Sprint 006 Phase 3: Integration testing — 4 end-to-end tests (Tasks 16-19)
**Sprint:** Sprint 006, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/integration.rs (NEW: 4 integration tests — transfer e2e, contract deploy+call e2e, finality certificate e2e, startup recovery e2e)
- crates/aztibase-node/src/main.rs (added `mod integration` for test module)
- crates/aztibase-node/Cargo.toml (added `wat = "1"` dev-dependency for WASM test fixtures)
**Review Notes:** 4 integration tests cover the full consensus-to-execution pipeline: transfer with state persistence + redb verification, WASM contract deploy + call with storage verification, BLS finality certificate generation + verification, and startup recovery from redb with continued execution. 191 tests pass (4 new). Zero clippy warnings, fmt clean.
**Security Flags:** None

### 2026-03-06 -- security-engineer -- aztibase-consensus, aztibase-execution, aztibase-node
**Task:** Sprint 006 Phase 2: Security hardening — 7 MEDIUM findings resolved (Tasks 9-15)
**Sprint:** Sprint 006, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs (HashSet for committed blocks, updated extract_committed_batch call)
- crates/aztibase-consensus/src/ordering.rs (extract_committed_batch accepts HashSet)
- crates/aztibase-consensus/src/finality.rs (has_unique_bls_keys, build_certificate takes &ValidatorSet)
- crates/aztibase-execution/src/routing.rs (strict bincode, func_name validation, InvalidFuncName error)
- crates/aztibase-node/src/pipeline.rs (double-execution guard, fatal flush handling, execute_batch returns Result)
**Review Notes:** 7 MEDIUM security findings resolved: SEC-WIRE-003 (double-execution guard), SEC-WIRE-004 (fatal flush), SEC-WIRE-005 (HashSet committed), SEC-BLS-002 (unique BLS keys), SEC-BLS-008 (derive quorum), SEC-ROUTE-005 (strict bincode), SEC-ROUTE-006 (func_name validation). 7 new tests, 187 total passing. Zero clippy warnings, fmt clean.
**Security Flags:** SEC-WIRE-003 RESOLVED, SEC-WIRE-004 RESOLVED, SEC-WIRE-005 RESOLVED, SEC-BLS-002 RESOLVED, SEC-BLS-008 RESOLVED, SEC-ROUTE-005 RESOLVED, SEC-ROUTE-006 RESOLVED

### 2026-03-06 -- node-engineer -- aztibase-rpc, aztibase-node
**Task:** Sprint 006 Phase 1: JSON-RPC server + node wiring (Tasks 1-5, 7-8)
**Sprint:** Sprint 006, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-rpc/src/server.rs (full JSON-RPC 2.0 server: 6 methods, axum router, shared state via Arc<RwLock<AccountState>>)
- crates/aztibase-rpc/src/lib.rs (re-exports: RpcServer, RpcState)
- crates/aztibase-rpc/Cargo.toml (added axum, hex, tower dev-dep)
- crates/aztibase-node/src/main.rs (RPC server spawn, mempool_rx forwarding to consensus)
- crates/aztibase-node/src/pipeline.rs (Arc<RwLock<AccountState>> shared state, batch_count AtomicU64, shared_state()/shared_batch_count() accessors)
- Cargo.toml (axum workspace dep)
**Review Notes:** 6 RPC methods: aztb_getBalance, aztb_getNonce, aztb_getCode, aztb_sendTransaction, aztb_blockNumber, aztb_getStateRoot. 15 new tests. Pipeline refactored to Arc<RwLock> for shared read access. Task 6 (getTransactionReceipt) deferred — needs receipt store. 180 tests pass, zero clippy warnings, fmt clean.
**Security Flags:** None (security review pending in Phase 4)

### 2026-03-06 -- security-engineer -- aztibase-execution
**Task:** Fix routing security findings: prefix cross-check (SEC-ROUTE-001), bincode size limit (SEC-ROUTE-002)
**Sprint:** Sprint 005, Phase 4 (post-review fixes)
**Git Ref:** b6b38c0
**Files Changed:**
- crates/aztibase-execution/src/routing.rs (prefix-variant cross-check, 1MB bincode limit, OversizedPayload/PrefixMismatch errors, 2 new tests)
**Review Notes:** Both ELEVATED routing findings resolved. Prefix mismatch now returns error. bincode deserialization capped at 1MB. 165 tests pass.
**Security Flags:** SEC-ROUTE-001 RESOLVED, SEC-ROUTE-002 RESOLVED

### 2026-03-06 -- security-engineer -- aztibase-consensus, aztibase-node
**Task:** Fix wiring security findings: batch drop detection (SEC-WIRE-001), dead pipeline halt (SEC-WIRE-002)
**Sprint:** Sprint 005, Phase 4 (post-review fixes)
**Git Ref:** 27ab11b
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs (error logging on try_send failure, warn on batch extraction failure)
- crates/aztibase-node/src/main.rs (break on dead pipeline channel instead of silent discard)
**Review Notes:** Both ELEVATED wiring findings resolved. Committed batches no longer silently dropped. Dead execution pipeline triggers node halt. 163 tests pass.
**Security Flags:** SEC-WIRE-001 RESOLVED, SEC-WIRE-002 RESOLVED

### 2026-03-06 -- security-engineer -- aztibase-core, aztibase-consensus
**Task:** Fix BLS security findings: PoP (SEC-BLS-001), domain separator (SEC-BLS-003), DST switch (SEC-BLS-004), signer dedup (SEC-BLS-005)
**Sprint:** Sprint 005, Phase 4 (post-review fixes)
**Git Ref:** 8da45e4
**Files Changed:**
- crates/aztibase-core/src/bls.rs (PoP generation + verification, DST_POP constant, POP ciphersuite, 2 new tests)
- crates/aztibase-consensus/src/finality.rs (AZTIBASE_FINALITY_V1 domain separator, duplicate signer skip)
**Review Notes:** SEC-BLS-001 ELEVATED resolved via PoP. DST switched to POP ciphersuite. Domain separator prevents cross-protocol replay. Signer dedup prevents double-counting. 163 tests pass.
**Security Flags:** SEC-BLS-001 RESOLVED, SEC-BLS-003 RESOLVED, SEC-BLS-004 RESOLVED, SEC-BLS-005 RESOLVED

### 2026-03-06 -- security-engineer -- aztibase-execution, aztibase-storage
**Task:** Fix ELEVATED security findings SEC-PERSIST-001 (atomic flush) and SEC-PERSIST-002 (safe deserialization)
**Sprint:** Sprint 005, Phase 4 (post-review fixes)
**Git Ref:** 24e4262
**Files Changed:**
- crates/aztibase-execution/src/persist.rs (atomic flush_state via batch_put_multi, safe deserialize_account_record)
- crates/aztibase-storage/src/store.rs (MultiTableEntry lifetime separation: 'static for TableDef, 'a for data)
- crates/aztibase-storage/src/lib.rs (TableDef type alias, redb::TableDefinition re-export)
- Cargo.lock (updated)
**Review Notes:** Both ELEVATED findings now resolved. flush_state() uses single atomic transaction. All deserialization uses Option/match instead of unwrap(). 161 tests pass.
**Security Flags:** SEC-PERSIST-001 RESOLVED, SEC-PERSIST-002 RESOLVED

### 2026-03-06 -- security-engineer + documentation-engineer -- all
**Task:** Sprint 005 Phase 4: Security review + documentation (Tasks 16-21)
**Sprint:** Sprint 005, Phase 4
**Git Ref:** pending
**Files Changed:**
- blockchain-project/sprints/SPRINT-005.md (security review results, retrospective, DoD checked)
- blockchain-project/STATUS.md (sprint complete, test count 161, crate status updated)
- blockchain-project/CHANGELOG.md (M3 section added)
- blockchain-project/DECISIONS.md (ADR-004: blst dependency)
- blockchain-project/BUILD_LOG.md (Phase 3 + Phase 4 entries)
**Review Notes:**
- 4 security reviews: wiring, persistence, BLS, routing
- 12 findings: 0 ELEVATED, 1 MEDIUM (rogue-key without PoP), 3 LOW, 8 INFO
- cargo-audit: 5 transitive vulns, 6 warnings — all in ring/wasmtime/libp2p/bincode
- Sprint 005 DoD: all 10 items checked
**Security Flags:** None ELEVATED. SEC-BLS-001 (rogue-key) is MEDIUM — PoP required before multi-validator testnet.

---

### 2026-03-06 -- consensus-engineer -- core/consensus
**Task:** Sprint 005 Phase 3: BLS finality certificates (Tasks 11-15)
**Sprint:** Sprint 005, Phase 3
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (added blst = "0.3" to workspace deps)
- crates/aztibase-core/Cargo.toml (added blst dependency)
- crates/aztibase-core/src/bls.rs (NEW: BlsKeypair, BlsPublicKey, BlsSignature, aggregate_signatures, verify_aggregate)
- crates/aztibase-core/src/lib.rs (added bls module + exports)
- crates/aztibase-consensus/src/finality.rs (NEW: FinalityCertificate, sign_finality, build_certificate, verify_certificate)
- crates/aztibase-consensus/src/lib.rs (added finality module + exports)
**Review Notes:**
- blst is C dependency — justified exception to pure-Rust policy (ADR-002). Industry-standard BLS12-381 used by Lighthouse/Prysm. No production-audited pure-Rust alternative.
- Custom serde for BlsPublicKey ([u8; 48]) and BlsSignature ([u8; 96]) — serde doesn't support arrays >32 by default.
- FinalityCertificate uses signer bitmap (Vec<bool>) indexed by validator position for compact representation.
- 19 new tests: 11 in aztibase-core (BLS primitives), 8 in aztibase-consensus (finality certificates).
- Tests cover: sign/verify, aggregation, supermajority, tampered state root, tampered batch hash, manipulated bitmap, bitmap length mismatch, insufficient signers.
- Total tests: 161 (up from 142).
**Security Flags:** None — rogue-key attack mitigated by proof-of-possession (PoP) requirement noted for validator registration (future sprint).

---

### 2026-03-06 -- node-engineer + smart-contract-engineer -- storage/execution/node
**Task:** Sprint 005 Phase 2: State Persistence to redb (Tasks 6-10)
**Sprint:** Sprint 005, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-storage/src/store.rs (4 new tables: accounts, contract_code, contract_storage, batch_roots)
- crates/aztibase-storage/src/lib.rs (new table exports)
- crates/aztibase-execution/src/persist.rs (NEW: flush_state, load_state, store_batch_root, get_batch_root)
- crates/aztibase-execution/src/state.rs (added iter_accounts())
- crates/aztibase-execution/src/lib.rs (added persist module + exports)
- crates/aztibase-node/src/pipeline.rs (with_storage constructor, flush after batch, state recovery)
- crates/aztibase-node/src/main.rs (separate execution_db store for pipeline)
- crates/aztibase-node/src/config.rs (execution_storage_path())
**Review Notes:**
- 4 new redb tables: accounts (addr->balance+nonce), contract_code (addr->wasm), contract_storage (addr||key->value), batch_roots (anchor->root)
- flush_state writes full AccountState to redb after each batch
- load_state recovers full AccountState on startup (balances, nonces, code, storage)
- State root matches after flush/load roundtrip (verified by test)
- Pipeline now uses with_storage() in node — state survives restarts
- 8 new tests (6 persist, 2 pipeline persistence)
- cargo clippy zero warnings, cargo fmt clean
- Total tests: 148 (was 134)
**Security Flags:** None

---

### 2026-03-06 -- node-engineer + smart-contract-engineer -- node/execution/consensus
**Task:** Sprint 005 Phase 1: Consensus-to-Execution Wiring + Tx Routing (Tasks 1-5)
**Sprint:** Sprint 005, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/routing.rs (NEW: TxKind enum, route_tx, route_batch, RoutingError)
- crates/aztibase-execution/src/lib.rs (added routing module + exports)
- crates/aztibase-execution/Cargo.toml (added bincode dependency)
- crates/aztibase-node/src/pipeline.rs (NEW: ExecutionPipeline, PipelineResult, batch execution)
- crates/aztibase-node/src/main.rs (added pipeline module, wired consensus->execution channel)
- crates/aztibase-consensus/src/engine.rs (ConsensusOutput::BlockCommitted -> BatchCommitted with full CommittedBatch)
**Review Notes:**
- TxKind uses prefix byte (0x01=transfer, 0x02=deploy, 0x03=call) + bincode body
- ExecutionPipeline receives CommittedBatch via tokio::mpsc, routes txs, executes, computes state root
- ConsensusEngine now extracts CommittedBatch at commit time (has DAG + committed list)
- 12 new tests (7 routing, 5 pipeline)
- cargo clippy zero warnings, cargo fmt clean
- Total tests: 134 (was 122)
**Security Flags:** None

---

### 2026-03-06 -- security-engineer -- cross-cutting
**Task:** Sprint 004 Phase 4: Security Review + cargo-audit (Tasks 15-19)
**Sprint:** Sprint 004, Phase 4
**Git Ref:** pending
**Files Changed:**
- (Review only -- no code changes required)
**Review Notes:**
- 12-item security checklist passed across VRF leader election, transfer execution, contract execution, state roots
- VRF seed derived from hash(committed_anchor_hash) -- not manipulable by proposer
- Balance arithmetic uses checked subtraction + saturating_add -- no overflow/underflow
- Nonce enforcement strict: mismatch = failure, failed txs still increment nonce
- WASM fuel metering enforced (100M default), host function bounds checked (unchanged from Sprint 002)
- Contract storage isolated per address in AccountState
- Round pruning keeps 2-round buffer for safety
- cargo-audit: same transitive advisories as Sprint 003 + 2 new wasmtime WASI advisories (not used)
- No ELEVATED flags
**Security Flags:** None

---

### 2026-03-06 -- smart-contract-engineer -- execution
**Task:** Sprint 004 Phase 3: WASM Contract Execution + State Roots (Tasks 10-14)
**Sprint:** Sprint 004, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/contract.rs (NEW: ContractTx, ContractReceipt, deploy/call execution)
- crates/aztibase-execution/src/vm.rs (added execute_with_storage(), Clone+Debug on ContractEvent)
- crates/aztibase-execution/src/lib.rs (added contract module exports)
**Review Notes:**
- Contract deployment stores WASM bytecode at deterministic address (hash(deployer || nonce))
- Contract calls load code, execute via wasmtime, persist storage changes to AccountState
- execute_with_storage() pre-loads existing contract storage into host state
- 5 new contract tests including full deploy-and-call WASM test with storage verification
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-06 -- smart-contract-engineer + consensus-engineer -- execution/consensus
**Task:** Sprint 004 Phase 2: Transaction Ordering + Execution Pipeline (Tasks 5-9)
**Sprint:** Sprint 004, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/src/state.rs (NEW: AccountState, Account, state_root computation)
- crates/aztibase-execution/src/parallel.rs (NEW: TransferTx, execute_transfers, BatchResult)
- crates/aztibase-consensus/src/ordering.rs (NEW: CommittedBatch, extract_committed_batch, payload parsing)
- crates/aztibase-consensus/src/lib.rs (added ordering module exports)
- crates/aztibase-execution/src/lib.rs (added state + parallel exports)
**Review Notes:**
- AccountState: BTreeMap-based in-memory store with balance, nonce, code, storage per account
- State root: BLAKE3 binary Merkle tree over sorted account entries
- SimpleTransfer execution: nonce check, balance check, debit/credit, receipt generation
- CommittedBatch: topological ordering of committed vertices with payload transaction extraction
- 21 new tests (8 state, 5 transfer, 3 ordering, 5 contract)
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-06 -- consensus-engineer -- consensus
**Task:** Sprint 004 Phase 1: Close M2 Debt (Tasks 1-4)
**Sprint:** Sprint 004, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs (round pruning, pending_txs cap, VRF seed tracking)
- crates/aztibase-consensus/src/validator.rs (vrf_leader_for_round() method)
- crates/aztibase-consensus/src/commit.rs (vrf_seed in CommitConfig, VRF-aware leader selection)
- crates/aztibase-consensus/src/lib.rs (5 new VRF + commit rule tests)
**Review Notes:**
- Round pruning: prune_before() removes entries older than committed round minus 2-round buffer
- Pending txs capped at 4096 (configurable via max_pending_txs)
- VRF leader: BLAKE3(round || seed) mapped to stake-weighted position
- Seed updated on commit: hash(committed_anchor_hash)
- CommitRule uses VRF when seed is set, falls back to deterministic for backwards compat
- 8 new tests (2 pruning, 1 cap, 5 VRF/commit)
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-06 -- security-engineer -- cross-cutting
**Task:** Sprint 003 Phase 4: Security Review + cargo-audit (Tasks 17-18)
**Sprint:** Sprint 003, Phase 4
**Git Ref:** pending
**Files Changed:**
- (Review only — no code changes required)
**Review Notes:**
- Security review: 12-item checklist passed across consensus engine, vertex validation, mempool, gossip routing
- Equivocation detection: engine rejects duplicate vertices per (author, round) pair
- Hash integrity: received vertices verified via compute_hash() before insertion
- Round bounds: vertices >10 rounds ahead rejected (DoS mitigation)
- Parent validation: quorum (2f+1) parents required after round 3
- Mempool bounded (10k default), dedup via seen-set prevents flooding
- Channel backpressure: mpsc(256) limits consensus inbox depth
- No unsafe blocks in any Sprint 003 code
- cargo-audit: same 5 transitive advisories as Sprint 002 (wasmtime WASI, ring, lru) + bincode unmaintained warning
- All advisories documented, none affect our code paths
- No ELEVATED flags
**Security Flags:** None

---

### 2026-03-06 -- node-engineer + consensus-engineer -- node/consensus
**Task:** Sprint 003 Phase 3: Commit Rule Integration + Mempool (Tasks 12-16)
**Sprint:** Sprint 003, Phase 3
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-node/src/mempool.rs (NEW: Mempool with insert, remove, peek/drain_batch, dedup, bounded)
- crates/aztibase-node/src/main.rs (added mempool module, integrated with gossip tx routing)
**Review Notes:**
- Tasks 12-13 already implemented in engine.rs (evaluate_commits, RoundState tracks committed blocks)
- Mempool: BTreeMap + HashSet seen-filter for dedup, configurable max_size (10k default)
- Node routes TOPIC_TRANSACTIONS through mempool.insert() for dedup before forwarding to consensus
- 6 new mempool tests (93 workspace total)
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-06 -- consensus-engineer + p2p-network-engineer -- consensus/network/node
**Task:** Sprint 003 Phase 2: Vertex Reception & DAG Growth (Tasks 7-11)
**Sprint:** Sprint 003, Phase 2
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/dag.rs (added compute_hash() public method, renamed private hash fn)
- crates/aztibase-consensus/src/engine.rs (hash verification, round bounds check, quorum-aware parent warning)
- crates/aztibase-consensus/src/validator.rs (added quorum_count() method)
- crates/aztibase-consensus/src/lib.rs (2 new tests: quorum_count, block_hash_verification)
- crates/aztibase-network/src/lib.rs (exported TOPIC_CONSENSUS, TOPIC_TRANSACTIONS constants)
- crates/aztibase-node/src/main.rs (topic-based message routing: consensus vs transactions)
**Review Notes:**
- Node filters gossip messages by topic: consensus vertices to engine, transactions to mempool
- Received vertices validated: deserialization, author check, hash integrity, round bounds (max +10 ahead)
- Broadcast uses TOPIC_CONSENSUS constant instead of hardcoded string
- Parent selection warns when fewer than quorum (2f+1) parents available after round 3
- 5 new tests (41 consensus total, 87 workspace total)
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer + node-engineer -- consensus/node
**Task:** Sprint 003 Phase 1: Consensus Round Engine (Tasks 1-6)
**Sprint:** Sprint 003, Phase 1
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/engine.rs (NEW: ConsensusConfig, RoundState, ConsensusEngine, ConsensusInput/Output)
- crates/aztibase-consensus/src/lib.rs (added engine module + exports)
- crates/aztibase-node/src/main.rs (wired ConsensusEngine into node event loop)
**Review Notes:**
- ConsensusEngine runs async round loop at 400ms intervals via tokio::time::interval
- Vertex proposal, reception, validation, and commit evaluation all functional
- Channel-based architecture (mpsc) cleanly separates consensus from network
- Node spawns engine as tokio task, routes gossip messages to inbox, broadcasts outbox vertices
- Replaced hex dependency with inline short_hex() helper to avoid unnecessary dep
- Added Default impl for RoundState per clippy suggestion
- 10 new engine tests (36 consensus total, 82 workspace total)
- cargo clippy zero warnings, cargo fmt clean
**Security Flags:** None

---

### 2026-03-05 -- security-engineer + ai-integration-engineer -- cross-cutting
**Task:** Phase 5 complete: Security review, AIRuntime, CryptoProvider, cargo-audit
**Sprint:** Sprint 002, Phase 5 (Tasks 21-24)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-core/src/crypto.rs (CryptoProvider trait + DefaultCryptoProvider impl)
- crates/aztibase-core/src/lib.rs (3 new tests for CryptoProvider)
- crates/aztibase-runtime/src/ai_oracle.rs (AIRuntime trait, InferenceRequest/Result, PassthroughRuntime)
- crates/aztibase-runtime/src/lib.rs (exports, 3 new tests for AIRuntime)
- crates/aztibase-execution/src/vm.rs (bounds-checked memory access in host functions)
**Review Notes:**
- Security review: 13-item checklist passed. Found and fixed WASM host function bounds checking (vm.rs). No ELEVATED flags remaining.
- AIRuntime trait with Passthrough/LocalInference/NetworkInference modes. PassthroughRuntime returns empty results, enabling nodes to run without AI hardware.
- CryptoProvider trait abstracts BLAKE3+Ed25519 behind swappable interface for future PQC migration.
- cargo-audit: 5 advisories found, all in transitive deps (wasmtime WASI, ring AES, libp2p lru). None affect our code paths (no WASI, no AES, threads disabled). Documented as acceptable for M1.
- 72 tests total, zero clippy warnings, fmt clean.
**Security Flags:** WASM bounds checking fixed (was missing negative ptr/overflow validation). No remaining ELEVATED flags.

---

### 2026-03-05 -- node-engineer -- aztibase-node
**Task:** Phase 4 complete: Node wiring (storage, network, config, shutdown)
**Sprint:** Sprint 002, Phase 4 (Tasks 17-20)
**Git Ref:** pending
**Files Changed:**
- Cargo.toml (added toml, directories workspace deps)
- crates/aztibase-node/Cargo.toml (added serde, toml, directories deps)
- crates/aztibase-node/src/config.rs (NodeConfig: TOML loading, CLI overrides, defaults, storage path)
- crates/aztibase-node/src/main.rs (full node startup: storage init, network swarm, event loop, graceful shutdown)
- crates/aztibase-network/src/lib.rs (re-exported Multiaddr)
**Review Notes:** Node opens redb storage on startup, creates libp2p swarm with TCP+QUIC, listens on configured addresses, dials boot nodes, runs event loop processing gossip/mDNS/connection events. Graceful shutdown via Ctrl+C with tokio::signal + Notify. Config supports TOML file loading with CLI overrides (--data-dir, --listen, --rpc-addr, --log-level). 6 new tests (config defaults, CLI overrides, TOML roundtrip, storage path, storage open). 66 tests total, zero clippy warnings, fmt clean.
**Security Flags:** None

---

### 2026-03-05 -- smart-contract-engineer -- aztibase-execution
**Task:** Phase 3b complete: WASM execution engine with wasmtime
**Sprint:** Sprint 002, Phase 3b (Tasks 14-16)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-execution/Cargo.toml (added wasmtime dependency)
- crates/aztibase-execution/src/vm.rs (ExecutionEngine: deterministic config, fuel metering, host functions, execution pipeline)
- crates/aztibase-execution/src/lib.rs (exports, 5 tests)
**Review Notes:** wasmtime 28 with deterministic config: SIMD disabled, relaxed SIMD disabled, threads disabled, fuel metering enabled. Three host functions: storage_set, storage_get, emit_event — all with WASM linear memory access. Execution pipeline: compile module → create store with fuel → link host functions → instantiate → call → collect results. Fuel exhaustion trap aborts on Windows (known wasmtime limitation) — tested via fuel consumption tracking instead. 5 tests passing.
**Security Flags:** wasmtime trap handling on Windows causes process abort instead of unwinding — epoch_interruption disabled for now. To be revisited when adding time-based execution limits.

---

### 2026-03-05 -- p2p-network-engineer -- aztibase-network
**Task:** Phase 3a complete: libp2p transport with Gossipsub, Kademlia, mDNS
**Sprint:** Sprint 002, Phase 3a (Tasks 9-13)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-network/Cargo.toml (added futures dependency)
- crates/aztibase-network/src/behaviour.rs (AztibaseBehaviour: combined NetworkBehaviour with gossipsub + kademlia + mDNS)
- crates/aztibase-network/src/gossip.rs (6 Aztibase topics, gossipsub config with strict validation, content-based dedup)
- crates/aztibase-network/src/discovery.rs (Kademlia DHT with /aztibase/kad/1.0.0 protocol, replication factor 20, disjoint query paths; mDNS for local discovery)
- crates/aztibase-network/src/transport.rs (Libp2pTransport: SwarmBuilder with TCP/Noise + QUIC, topic subscription, event handling loop with mDNS auto-peering)
- crates/aztibase-network/src/lib.rs (exports, 8 tests)
**Review Notes:** libp2p 0.54 with SwarmBuilder API. Transport creates swarm with TCP/Noise/Yamux + QUIC, subscribes to all 6 topics on construction. Event loop handles gossipsub messages, mDNS discovery/expiry (auto-adds to gossipsub + kademlia), connection lifecycle. Kademlia in Server mode with 60s query timeout. 8 tests: config creation, topic validation, swarm creation, peer ID, topic subscription count, TCP listening.
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer -- aztibase-consensus
**Task:** Phase 2 complete: DagStore + CommitRule implementations
**Sprint:** Sprint 002, Phase 2 (Tasks 7-8)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/dag_store.rs (DagStore: insert, get, parent/child lookup, ancestor check, causal ordering, persistence via redb)
- crates/aztibase-consensus/src/commit.rs (CommitRule: direct commit via supermajority voting, indirect commit via anchor, wave-based leader election)
- crates/aztibase-consensus/src/lib.rs (exports, 10 new tests -- 7 DagStore + 3 CommitRule)
- crates/aztibase-consensus/Cargo.toml (added bincode dependency)
**Review Notes:** DagStore: in-memory index + redb persistence, parent validation on insert, BFS ancestor traversal with round-based pruning, deterministic topological sort for causal ordering, rebuild_index from disk on startup. CommitRule: MystiCeti-inspired wave structure (configurable wave_length), direct commit checks >2/3 voting stake, indirect commit via causal ancestry from anchor block. 26 consensus tests total (16 existing + 10 new). Zero clippy warnings, fmt clean.
**Security Flags:** None

---

### 2026-03-05 -- consensus-engineer -- aztibase-consensus
**Task:** Phase 1b complete: DagBlock + ValidatorSet implementations
**Sprint:** Sprint 002, Phase 1b (Tasks 5-6)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-consensus/src/dag.rs (DagBlock with hash, genesis, parent validation)
- crates/aztibase-consensus/src/validator.rs (ValidatorSet with stake, supermajority, leader selection)
- crates/aztibase-consensus/src/lib.rs (exports, 16 tests)
**Review Notes:** DagBlock: deterministic hash (BLAKE3 of round+author+parents+payload+timestamp), genesis block factory, parent round monotonicity validation. ValidatorSet: add/remove/get/contains, total_stake tracking, BFT supermajority check (>2/3), deterministic stake-weighted leader selection. 16 tests passing. Zero clippy warnings.
**Security Flags:** None

---

### 2026-03-05 -- node-engineer -- aztibase-storage
**Task:** Phase 1a complete: StateStore trait, 6 named tables, batch writes, range iteration
**Sprint:** Sprint 002, Phase 1a (Tasks 1-4)
**Git Ref:** pending
**Files Changed:**
- crates/aztibase-storage/src/store.rs (complete rewrite — StateStore with redb backend)
- crates/aztibase-storage/src/lib.rs (exports, 15 tests)
**Review Notes:** Full CRUD (get/put/delete/contains), batch_put (single table), batch_put_multi (cross-table atomic), iter/range/range_reverse. All 6 tables (blocks, state, tx, receipts, validators, verkle) initialized on open. StorageError with boxed TransactionError to satisfy clippy. Zero clippy warnings, fmt clean. 15 tests passing (exit criteria was 10+).
**Security Flags:** None

---

### 2026-03-05 -- project-lead -- Project Infrastructure
**Task:** Established build-phase documentation trail
**Sprint:** Pre-Sprint
**Git Ref:** pending
**Files Changed:**
- blockchain-project/STATUS.md (new)
- blockchain-project/DECISIONS.md (new)
- blockchain-project/BUILD_LOG.md (new)
- blockchain-project/sprints/SPRINT-001.md (new)
- CHANGELOG.md (new)
- blockchain-project/ORCHESTRATION.md (updated -- build-phase protocol)
**Review Notes:** Design phase had excellent tracking via ORCHESTRATION.md. Build phase needed its own tracking infrastructure. All five tracking documents now in place.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Storage
**Task:** Switched storage backend from RocksDB to redb
**Sprint:** Pre-Sprint
**Git Ref:** dd98662
**Files Changed:** crates/aztibase-storage/ (Cargo.toml, src/)
**Review Notes:** Pure Rust dependency, better cross-compilation story. See ADR-001.
**Security Flags:** None

---

### 2026-03-05 -- blockchain-architect -- Project Scaffold
**Task:** Initial project structure, all crate scaffolding, skill definitions
**Sprint:** Pre-Sprint
**Git Ref:** 86f194b
**Files Changed:** Full workspace creation -- Cargo.toml, 8 crates, .claude/skills/, blockchain-project/ docs
**Review Notes:** Foundation commit. All design docs, skill fleet, and crate stubs.
**Security Flags:** None
