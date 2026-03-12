# Changelog -- Aztibase Network

All notable changes to this project are documented here, organized by milestone.
Format follows [Keep a Changelog](https://keepachangelog.com/).

---

## Pre-Mainnet Audit (2026-03-12)

### Added
- Full pre-mainnet audit report (`blockchain-project/AUDIT_REPORT_2026-03-12.md`) covering all 9 crates + architecture alignment

### Audit Results
- **15 CRITICAL**: `.unwrap()` panics on untrusted data, phantom parent attacks, gossipsub message suppression, WASM key exposure, APY formula mismatch, VerkleTree leaf count bug
- **25 HIGH**: No DagBlock signatures (top blocker), silent committed batch drops, faucet abuse vectors, empty runtime stubs, CORS defaults
- **3 constraint violations**: `blst` C dep (ADR-001), Merkle-not-Verkle state root, zero privacy primitives
- **8 architecture gaps**: Object model, Block-STM integration, WASM execution, privacy, DAS, relay incentives, runtime upgrades
- **Code quality**: PASS — zero AI fingerprints, zero `unsafe`, zero `todo!()`, 931 tests

---

## Consensus-Based FaucetDrip & Testnet Liveness Fixes (2026-03-12)

### Changed
- **Faucet is now consensus-based**: FaucetDrip (0x1B) is a signed transaction that goes through mempool→gossipsub→consensus→pipeline execution. All nodes reach consistent state. Replaces the prior node-local faucet.

### Fixed
- **FaucetDrip nonce not incremented**: Pipeline execution set recipient balance but never called `increment_nonce(validator)`. Second+ faucet drips failed nonce validation (expected=0, got=1).
- **P2P mesh incomplete on local testnet**: Nodes bound to `127.0.0.1` couldn't be reached when peers discovered them via LAN IPs (172.x, 192.x) through libp2p address observation. Changed to `0.0.0.0`.
- **Testnet startup race condition**: All 3 nodes started simultaneously, causing boot_node connections to fail before listeners were ready. Added 2s staggered starts.

### Validated (Live 3-Node Testnet)
- FaucetDrip: 2x drips credited correctly (nonce 0 and 1), balance 2,000,000 on all 3 nodes
- Transfers: 500 + 1,000 AZTB with correct gas accounting (21,000 gas each)
- Cross-node consistency: all 3 nodes show identical sender/recipient balances
- Block production: ~15 batches/sec continuous, full P2P mesh (2 peers per node)

---

## Open Risk Resolution & Testnet Validation (2026-03-12)

### Security
- **quinn-proto DoS fix**: Bumped 0.11.13 → 0.11.14 (RUSTSEC-2026-0037, severity 8.7 HIGH). Eliminates DoS on QUIC P2P transport.
- **nChain FTO analysis (ADR-028)**: Overall LOW-MEDIUM risk. 3,900 patents reviewed. PoUW attestation mechanism confirmed architecturally distinct from nChain's ZK-based verifiable AI. No DAG consensus or Verkle tree overlap.
- **wasmtime v42 Windows**: Trap handling confirmed clean — setjmp/longjmp rewrite shipped in v38-v40, fully baked by v42.

### Fixed
- **Staking metrics in /metrics/json**: `active_validators` and `total_staked` counters were always 0. Now updated from shared staking store after each batch commit.

### Added
- **Testnet validation script**: `test-testnet.sh` — 12-phase automated validation (health, chain ID, block production, faucet, balances, metrics, cross-node consistency, stress test).

### Validated (Live 3-Node Testnet)
- 2226+ batches committed, 0 equivocations, 3.2ms avg commit latency
- All 49 RPC methods responsive, 3 validators active (1M stake each)
- Prometheus + JSON metrics endpoints confirmed
- Rate limiter, faucet, cross-node genesis hash consistency confirmed
- 20/20 rapid faucet drips succeeded under stress

---

## Admin Dashboard & Testnet Hardening (2026-03-12)

### Added
- **Internal admin dashboard**: `admin/` — multi-file vanilla JS dashboard for Aztibase team. 4 pages: Nodes (multi-node health overview with live metrics), Validators (active set + stake lookup), Operations (faucet, chain params, governance, emission), Accounts (address/tx/batch lookup). Served on port 8081.
- **Grafana dashboard enhancements**: Added Active Validators, Total Staked, Slashing Events, and Block Production Rate panels to node-health dashboard.

### Fixed
- **TX lookup returning null**: `getTransactionByHash` now returns data for all transactions (including nonce-rejected ones), not just successfully executed txs.
- **Nonce increment on failed escrow**: Transactions that fail gas-price or escrow checks no longer consume a nonce slot. Matches standard behavior: if you can't pay, no nonce consumed.
- **Faucet documented as node-local**: Added rustdoc comment (now superseded — faucet is consensus-based as of fc4de4e).

---

## Sprint 057 — Validator Business-in-a-Box & Cloud Monitoring (M9-S18) (2026-03-11)

### Added
- **One-click validator setup**: `deploy/setup-validator.sh` — interactive wizard for non-technical operators. Auto-detects OS/arch, installs deps, builds binary, generates keys, configures systemd, opens firewall, runs health check. Supports `--dry-run`, `--uninstall`, `--yes` (non-interactive).
- **Grafana Cloud monitoring**: Built-in Grafana Alloy integration pushes metrics to free Grafana Cloud tier (10k series). Setup guide at `docs/GRAFANA_CLOUD_SETUP.md`.
- **VPS recommendations**: Hardware requirements, provider comparison table (6 providers), cost estimates starting at €3.99/mo, step-by-step walkthrough at `docs/VPS_GUIDE.md`.

---

## Sprint 056 — Full State Snapshots & Mainnet Genesis (M9-S17) (2026-03-11)

### Added
- **Full state snapshots**: `ProtocolStoreBundle` captures all 9 protocol stores + base_fee alongside AccountState. SNAPSHOT_VERSION bumped to 3.
- **Snapshot file I/O**: `write_snapshot_file` / `read_snapshot_file` with BLAKE3 integrity verification (corruption detected before deserialization)
- **Snapshot CLI**: `aztibase snapshot export --output <path>` and `aztibase --snapshot <path>` for node bootstrap from file
- **Mainnet genesis ceremony**: `mainnet_genesis()` allocates 400M AZTB across 8 categories (team 15%, investors 10%, ecosystem 25%, community 20%, treasury 15%, validators 5%, advisors 5%, reserve 5%)
- **ADR-027**: Full state snapshots & mainnet genesis design

---

## Sprint 055 — Public Testnet Launch Infrastructure (M9-S16) (2026-03-11)

### Added
- **Canonical testnet genesis**: Pre-generated `testnet/genesis/` with 3 validators, 1 faucet account, node configs
- **Faucet web UI**: `faucet/index.html` — dark theme, address validation, drip button, 60s cooldown, configurable RPC via `?rpc=`
- **Testnet landing page**: `testnet/index.html` — network details, developer guide (4 steps), validator guide (6 steps), seed node table, RPC summary
- **Cloud seed node configs**: `deploy/seed-nodes/seed-{1,2,3}.toml` for testnet{1,2,3}.aztibase.com
- **Systemd service**: `deploy/systemd/aztibase.service` for Linux service management
- **Bootstrap script**: `deploy/bootstrap.sh` for Ubuntu 22.04+ (install deps, build, configure, start)
- **ADR-026**: Public testnet launch infrastructure

---

## Sprint 054 — Mainnet Operational Hardening (M9-S15) (2026-03-11)

### Added
- **Network profiles**: `NetworkProfile` enum (Dev/Testnet/Mainnet) drives CORS, faucet, and startup behavior
- **`--mainnet` CLI flag**: mutually exclusive with `--testnet`, sets profile to Mainnet
- **Per-IP RPC rate limiter**: token bucket (default 100 req/s, configurable via `rate_limit_per_ip`)
- **Profile-driven CORS**: Mainnet restricts to explicit origin whitelist; Dev/Testnet allow any origin
- **Faucet gating**: `aztb_faucetDrip` disabled when profile = Mainnet
- **DB sentinel**: write/clear sentinel on startup/shutdown; dirty-start detection warns operator
- **Graceful shutdown**: SIGINT handler flushes all protocol stores and clears sentinel before exit
- **Validator key rotation**: `TxKind::RotateValidatorKey` (0x1A) atomically re-keys validator + delegations + unbonding queue
- **Request body size limit**: configurable (default 1 MiB) to prevent oversized RPC payloads
- **ADR-025**: Network profiles & mainnet operational hardening
- 19 new tests (7 config + 3 sentinel + 4 staking + 1 routing + 4 RPC), 925 total

---

## Sprint 053 — Protocol Store Persistence (M9-S14) (2026-03-11)

### Added
- **Protocol store persistence**: 8 in-memory stores now survive node restart via redb STATE_TABLE serialization (2026-03-11)
- Persisted stores: StakingStore, GovernanceStore, EmissionTracker, ChainParams, AgentPolicyStore, L2Registry, L2AnchorStore, BridgeEscrow, BridgeWithdrawProofs
- Generic `flush_serializable`/`load_serializable` helpers using postcard binary serialization (2026-03-11)
- Pipeline startup loads all stores from disk; batch commit flushes all stores atomically (2026-03-11)
- ADR-024: Protocol store persistence design (STATE_TABLE prefixed keys, postcard, flush-after-batch) (2026-03-11)
- 9 new persistence roundtrip tests (staking, governance, emission, chain_params, agent_policies, bridge, empty_db, crash_recovery, full_protocol) (2026-03-11)

---

## Sprint 052 — L1 Bridge Primitives for Sovereign Rollups (M9-S13) (2026-03-11)

### Added
- 4 new TxKind variants: `RegisterL2` (0x19), `AnchorL2State` (0x16), `BridgeDeposit` (0x17), `BridgeWithdraw` (0x18) (2026-03-11)
- `L2Registry`: governance-gated L2 chain registration with sequencer_set authorization (2026-03-11)
- `L2AnchorStore`: anchored L2 state roots with 100-batch finality window, last-10 history (2026-03-11)
- `BridgeEscrow`: lock/unlock AZTB per (l2_chain_id, depositor) for cross-layer transfers (2026-03-11)
- `BridgeWithdrawProofs`: proof hash dedup table preventing double-spend withdrawals (2026-03-11)
- 4 new RPC endpoints: `aztb_getL2State`, `aztb_listL2s`, `aztb_getBridgeBalance`, `aztb_getBridgeProofStatus` (2026-03-11)
- ADR-023: L2 bridge design (challenge window, proof format, governance gate, escrow consistency) (2026-03-11)
- 23 new tests (10 l2_bridge unit + 6 routing roundtrip + 4 RPC + 3 e2e integration) (2026-03-11)

### Fixed
- Bridge escrow now correctly decrements on successful withdraw (found in security review) (2026-03-11)

---

## Sprint 051 — WASM Tx Signing & Block Explorer (M9-S12) (2026-03-11)

### Added
- WASM `signTransfer()`: construct + Ed25519-sign Transfer transactions in the browser, returns hex envelope (2026-03-11)
- WASM `generateKeypair()` + `addressFromSecret()`: key management exports (2026-03-11)
- WASM RPC request builders: `buildSendTxRequest`, `buildGetNonceRequest`, `buildGetBalanceRequest`, `buildEstimateGasRequest` (2026-03-11)
- Block explorer (`explorer/index.html`): vanilla HTML/JS, dark theme, responsive, latest blocks view, block/tx/account detail, `?rpc=` endpoint override (2026-03-11)
- `aztb_getBlockTransactionCount` RPC: returns tx count for a block by number (2026-03-11)
- `aztb_sendRawTransaction` RPC: alias for `aztb_sendTransaction` (browser wallet compat) (2026-03-11)
- CORS headers on all RPC routes via `tower-http` CorsLayer (`Access-Control-Allow-Origin: *`) (2026-03-11)
- 11 new tests: 8 WASM tx signing + 3 RPC (block tx count, sendRaw alias, CORS preflight) (2026-03-11)

---

## Sprint 050 — CI Pipeline & Public Testnet Infrastructure (M9-S11) (2026-03-10)

### Added
- CI pipeline: `dev` branch trigger, `CARGO_INCREMENTAL=0`, release build gate, security audit job (2026-03-10)
- RPC API reference (`docs/rpc-api.md`): all 43 methods documented with params, return types, curl examples (2026-03-10)
- `--testnet` CLI flag: loads bundled deterministic genesis config + DNS boot nodes (testnet1/2/3.aztibase.com) (2026-03-10)
- `BlsKeypair::from_ikm()`: deterministic BLS key derivation using proper HKDF (not raw scalar bytes) (2026-03-10)
- `testnet_genesis()`: 3-validator testnet genesis from fixed BLAKE3 seeds + faucet (100M tokens) (2026-03-10)
- Criterion benchmarks: `verify_and_route_single_tx`, `verify_tx_batch` (10/100/500) (2026-03-10)
- `scripts/deploy-validator.sh`: VPS/cloud deployment with systemd service, key generation, `--testnet` support (2026-03-10)
- 3 new tests: testnet genesis validity, determinism, boot node format (2026-03-10)

### Performance (release-mode benchmarks)
- **Consensus**: vertex 1.35µs, commit_rule 17.6ms, BLS cert 3.4ms (21 val) / 8.0ms (100 val)
- **Execution**: single transfer 1.99µs, verify_and_route 54.7µs (**18.3K TPS single-threaded**)
- **Batched**: verify_tx_batch/100 5.84ms (17.1K TPS), /500 27.8ms (18.0K TPS)
- **State**: state_root/1K accounts 839µs, /10K accounts 8.4ms
- **Proofs**: merkle 2.04µs, verkle 7.14µs (10K leaves)
- **Verdict**: 10K TPS target achievable — Ed25519 sig verify is bottleneck, scales with rayon cores

---

## Sprint 049 — Protocol Hardening & Multi-Node Stability (M9-S10) (2026-03-10)

### Added
- `latest_batch_index()` — recovers last persisted batch number from disk via reverse range scan (2026-03-10)
- Crash recovery in `ExecutionPipeline::with_storage()` — batch_count restored from BATCH_INDEX_TABLE on restart (2026-03-10)
- Shutdown logging — pipeline logs last persisted batch number on exit (2026-03-10)
- `PROTOCOL_VERSION` (v1), `agent_version()`, `parse_agent_version()` — protocol version infrastructure (2026-03-10)
- `identify::Behaviour` in libp2p swarm — peers with mismatched protocol versions disconnected automatically (2026-03-10)
- `--epoch-length` CLI flag — override epoch length for testnet/testing (2026-03-10)
- 17 new tests: 3 crash recovery, 4 epoch boundary, 2 throughput, 5 protocol version, 3 persist (2026-03-10)
- ADR-021: No mempool persistence — re-gossip from peers on restart (2026-03-10)
- ADR-022: Protocol version negotiation via libp2p identify (2026-03-10)

### Changed
- `batch_count.fetch_add(1)` moved from `run()` into `execute_batch()` for correct behavior in direct calls (2026-03-10)

### Performance
- Baseline TPS: 86 TPS (debug mode, 50 txs across 10 batches with redb persistence) (2026-03-10)
- Empty batch throughput: ~39,000 batches/s (debug mode) (2026-03-10)

---

## Sprint 048 — Security Flag Resolution (M9-S9) (2026-03-10)

### Security (9 ELEVATED flags resolved — ALL cleared)
- **S4-3**: Checkpoint P2P distribution — RequestCheckpoint/ResponseCheckpoint in light-sync protocol, checkpoint-announce gossipsub topic (2026-03-10)
- **S1-3**: Agent spending pre-execution — mempool rejects AgentExecute txs violating policy before consensus (2026-03-10)
- **S3-1**: Flash-loan resistant governance — vote weights from balance snapshot at proposal creation, not live balances (2026-03-10)
- **S2-1**: Equivocation window hardening — detection window 20→100 rounds, persistent proofs in redb, formal bounds in ADR-015 (2026-03-10)
- **S1-1**: MEV mitigation Phase 1 — MAX_TX_SIZE (256 KiB) enforced at mempool boundary, ADR-016 (2026-03-10)
- **S1-2**: Quantum migration drill — Verkle↔Merkle StateCommitment swap proven via migration tests, ADR-017 (2026-03-10)
- **S2-2**: VRF last-revealer bias fix — RANDAO-style seed accumulation from causal DAG history, ADR-018 (2026-03-10)
- **S4-1**: Browser wallet spending limits — per-tx cap (100 AZTB), session cap (10K AZTB), balance warnings via WASM exports, ADR-019 (2026-03-10)
- **S8-1**: DHT poisoning resistance — quorum-signed records (≥2 sigs), round-based freshness (10K rounds), ADR-020 (2026-03-10)

### Added
- `CheckpointAnnounce` struct for gossip-based checkpoint notification (2026-03-10)
- `TOPIC_CHECKPOINT_ANNOUNCE` gossipsub topic (7th topic, weight 1.5) (2026-03-10)
- `AgentPolicyStore::check_spend()` — read-only policy validation for mempool gate (2026-03-10)
- `Mempool::insert_checked_with_policy()` — pre-execution agent tx validation (2026-03-10)
- `snapshot_balances` field on `Proposal` — captures balances at proposal creation (2026-03-10)
- `EQUIVOCATION_PROOFS_TABLE` — persistent equivocation evidence in redb (14th table) (2026-03-10)
- `store_equivocation_proof()` / `get_equivocation_proof()` — evidence persistence API (2026-03-10)
- `MAX_TX_SIZE` constant (256 KiB) — consensus-layer tx size limit (2026-03-10)
- `accumulate_vrf_seed()` — RANDAO-style VRF seed mixing from DAG causal history (2026-03-10)
- `browser_wallet` module in aztibase-wasm — spending limits and WASM-exported safety checks (2026-03-10)
- `SignedDhtRecord` + `validate_dht_record()` — quorum-signed DHT records with freshness validation (2026-03-10)
- ADR-015: Equivocation detection window formal analysis (2026-03-10)
- ADR-016 through ADR-020: MEV mitigation, quantum migration, VRF seed, browser wallet, DHT poisoning (2026-03-10)
- 31 new tests across 6 crates (2026-03-10)

---

## Sprint 047 — Testnet Validation (M9-S8) (2026-03-10)
- First successful 3-node testnet: all validators committing blocks
- Peer-aware proposal gating: engine waits for peers before first proposal
- Relaxed DAG insert: accept vertices with missing parents (enables async delivery)
- Fixed causal_order panic when parent blocks are missing from local DAG
- Verified RPC: nodeInfo, chainId, getBalance, gasPrice, health endpoints working
- Tests: 113 consensus + 194 node = 307 pass (was 306 before, +1 new test)

---

## [M9-S7] -- Weak Subjectivity Checkpoints (Sprint 046)

### Added
- `Checkpoint` struct with optional `FinalityCertificate`, batch_index, state_root, timestamp (2026-03-10)
- `CHECKPOINT_INTERVAL` constant: checkpoints emitted every 1000 batches (2026-03-10)
- `CHECKPOINTS_TABLE` in storage: persistent checkpoint storage (2026-03-10)
- Raw checkpoint persistence API: `store_checkpoint_raw`, `get_checkpoint_raw`, `latest_checkpoint_raw` (2026-03-10)
- Pipeline: automatic checkpoint emission after every 1000th batch with state root and timestamp (2026-03-10)
- `--checkpoint <batch_index:state_root_hex>` CLI flag: trusted checkpoint validation on startup (2026-03-10)
- `aztb_getCheckpoint(batch_index)` RPC: query checkpoint at specific batch (2026-03-10)
- `aztb_latestCheckpoint` RPC: query most recent checkpoint (2026-03-10)
- 4 new tests (2 consensus + 2 execution persist), 811 total (2026-03-10)

### Security
- Weak subjectivity checkpoints prevent long-range attacks on syncing/recovering nodes (2026-03-10)
- Startup validation aborts if stored checkpoint doesn't match trusted value (2026-03-10)

---

## [M9-S6] -- Ed25519 Strict Verification & Attestation Domain Separation (Sprint 045)

### Changed
- `PublicKey::verify()` now uses `verify_strict()` — rejects malleable signatures (S >= L) and weak/small-order keys (2026-03-10)
- Removed `Verifier` trait import from crypto.rs (no longer needed) (2026-03-10)

### Security
- **S0-1 RESOLVED**: Ed25519 strict verification enforced across all signature paths (2026-03-10)
- Attestation domain separation confirmed already present (`AZTB_ATTESTATION_V1\0` prefix in `attestation_hash()`) (2026-03-10)
- 4 new tests: strict_verify_rejects_short_signature, strict_verify_rejects_tampered_signature, strict_verify_rejects_all_zeros_signature, strict_verify_rejects_malleable_s (2026-03-10)
- 804 tests pass, 0 clippy warnings, fmt clean (2026-03-10)

---

## [M9-S5] -- Genesis Hash P2P Enforcement (Sprint 044)

### Added
- `genesis_hex_prefix()`: computes 8-char hex prefix from first 4 bytes of genesis hash (2026-03-10)
- `chain_scoped_topics()`, `aztibase_topics_scoped()`: gossipsub topics parameterized by genesis hash prefix (2026-03-10)
- `peer_score_params_scoped()`: per-topic scoring with chain-scoped topic names (2026-03-10)
- `TOPIC_BASE_NAMES`: constant array of base topic names for DRY topic generation (2026-03-10)
- `kademlia_protocol()`, `kademlia_config_scoped()`, `kademlia_behaviour_scoped()`: Kademlia protocol ID includes genesis hash prefix (2026-03-10)
- `genesis_hash` field in `TransportConfig`: wired through to gossipsub and kademlia (2026-03-10)
- main.rs: genesis hash passed from genesis config to both full-node and light-node TransportConfig (2026-03-10)
- 10 new tests (6 gossip + 4 discovery), 796 total (2026-03-10)

### Changed
- Nodes with different genesis configs are now on completely separate P2P networks (2026-03-10)
- Topic format: `/aztibase/{topic}/1.0.0/{genesis_hex8}` (2026-03-10)
- Kademlia protocol format: `/aztibase/kad/1.0.0/{genesis_hex8}` (2026-03-10)

---

## [M9-S4] -- Autonomous AI Agent Transactions (Sprint 043)

### Added
- `AgentPolicy` store: per-agent spending constraints (per_tx_limit, per_epoch_limit, allowed_tx_kinds, expiry_epoch)
- `AgentSpendTracker`: epoch-based cumulative spend tracking with automatic epoch reset
- `TxKind::SetAgentPolicy` (0x14): owner-only policy configuration for AI agents
- `TxKind::AgentExecute` (0x15): autonomous agent transactions validated against policy constraints
- Pipeline: SetAgentPolicy execution (owner auth, AIAgent type check), AgentExecute (policy validation, balance check, transfer)
- `aztb_getAgentPolicy` RPC method: query agent policy by address
- 8 new unit tests (agent module), 786 total

### Changed
- `estimate_gas` expanded to 21 TxKind variants (added 0x14=60K, 0x15=80K)
- `extract_tx_features` captures AgentExecute value for anomaly scoring

---

## [M9-S3] -- Genesis Validation & Network Identity (Sprint 042)

### Added
- `GenesisValidationError` enum with 10 structured error variants (2026-03-10)
- `validate_genesis()`: comprehensive genesis config validation — duplicates, bounds, supply cap, address format, BLS key length (2026-03-10)
- `validate_genesis_with_min_stake()`: configurable minimum stake threshold (2026-03-10)
- Startup: validate genesis config before consensus initialization, abort on error (2026-03-10)
- Startup: log genesis hash for operator verification on successful validation (2026-03-10)
- 12 new genesis validation tests (2026-03-10)
- 778 tests pass, 0 clippy warnings, fmt clean (2026-03-10)

---

## [M9-S2] -- Gas Price Enforcement & Transaction Validation (Sprint 041)

### Fixed
- **SECURITY**: Closed fee-free execution vulnerability — txs with gas_price=0 previously bypassed escrow entirely (2026-03-10)

### Changed
- Pipeline: all transactions now validated against current base_fee before escrow (2026-03-10)
- Txs with gas_price < base_fee rejected with "gas price too low" error receipt (2026-03-10)
- `estimate_gas()` expanded from 7 to all 19 TxKind variants (2026-03-10)
- All ~120+ test gas_price values updated from 0 to 1, test balances adjusted for gas escrow (2026-03-10)
- 766 tests pass, 0 clippy warnings, fmt clean (2026-03-10)

---

## [M9-S1] -- u64→u128 Balance Migration (Sprint 040)

### Changed
- **BREAKING**: All monetary values migrated from u64 to u128 across entire codebase (~20 files, 6 crates) (2026-03-10)
- `Account.balance`, `Transaction.value`, `FeeEscrow.max_fee` → u128 (2026-03-10)
- `ValidatorStake`, `Delegation`, `UnbondingEntry`, `SlashRecord` monetary fields → u128 (2026-03-10)
- `ValidatorInfo.stake`, `ValidatorRecord.stake`, `ValidatorSet.total_stake` → u128 (2026-03-10)
- `InferenceTask.reward`, `ComputeCommitment.committed_stake` → u128 (2026-03-10)
- `ModelRegistry` compute_cost/min_stake, `GovernanceStore` vote weights → u128 (2026-03-10)
- `StateValue::Balance(u128)`, `read_balance()` → u128 in Block-STM (2026-03-10)
- `AccountRecord` persist.rs: 16-byte balance, shifted byte offsets (min record 24 bytes) (2026-03-10)
- VRF leader selection: 16-byte hash slice for u128 stake-weighted position (2026-03-10)
- Genesis TOML: `serde_u128_as_string` module for u128 stake/balance fields (2026-03-10)
- Prometheus: saturating i64 cast for u128 total_staked metric (2026-03-10)
- 766 tests pass, 0 clippy warnings, fmt clean (2026-03-10)

---

## [M8-S14] -- Full-Node Integration Wiring & M8 Close (Sprint 039)

### Added
- `ConsensusOutput::EquivocationDetected`: consensus emits equivocation events on outbox (2026-03-10)
- `ConsensusInput::UpdateValidatorSet`: pipeline sends updated validator set to consensus at epoch boundaries (2026-03-10)
- main.rs: equivocation events bridged to pipeline SlashEvent channel (2026-03-10)
- main.rs: StakingStore wired to RPC server via `with_staking_store()` (2026-03-10)
- main.rs: consensus_tx passed to pipeline for validator set sync (2026-03-10)
- `ExecutionPipeline::bootstrap_genesis_validators()`: registers genesis validators in StakingStore at startup (2026-03-10)
- `ExecutionPipeline::set_consensus_tx()`: attaches consensus input channel for epoch boundary updates (2026-03-10)
- `epoch_length` governance-controllable ChainParam (bounds: 1,000-100,000, default: 10,000) (2026-03-10)
- Staking Prometheus metrics: `active_validators`, `total_staked`, `slashes_applied` gauges (2026-03-10)
- `NodeMetrics::update_staking()` and `inc_slashes()` for Prometheus export (2026-03-10)
- 7+ new tests: equivocation output, validator set update, genesis bootstrap, epoch_length param, consensus_tx wiring, staking metrics (2026-03-10)

### Changed
- `ConsensusEngine`: handles `UpdateValidatorSet` input to replace live validator set (2026-03-10)
- Pipeline: epoch boundary reads `epoch_length` from ChainParams instead of EmissionTracker (2026-03-10)
- Removed `#[allow(dead_code)]` from `shared_staking_store()` and `slash_sender()` (2026-03-10)
- Integration tests: all ConsensusOutput match arms updated for EquivocationDetected (2026-03-10)

---

## [M8-S13] -- Validator Staking, Delegation & Slashing (Sprint 038)

### Added
- `staking` module in aztibase-execution: StakingStore, ValidatorStake, Delegation, UnbondingEntry, SlashRecord, OffenseType, StakingError (2026-03-10)
- StakingStore methods: register_validator, add_stake, begin_unstake, delegate, begin_undelegate, process_unbonding, slash_validator, distribute_epoch_rewards, active_set_snapshot, active_validators, pending_unbonding, validator_slash_history (2026-03-10)
- `TxKind::Stake` (0x10), `TxKind::Unstake` (0x11), `TxKind::Delegate` (0x12), `TxKind::Undelegate` (0x13) — 19 total TxKind variants (2026-03-10)
- Pipeline: full execution handlers for all 4 staking tx types with nonce/balance/cap validation (2026-03-10)
- Pipeline: unbonding queue processing after batch persistence — matured entries credited to account balances (2026-03-10)
- Pipeline: epoch boundary processing — EmissionTracker.advance_epoch() → distribute_epoch_rewards() → downtime slashing → validator set rebuild (2026-03-10)
- Pipeline: SlashEvent mpsc channel for consensus→pipeline equivocation reporting (2026-03-10)
- Pipeline: epoch_participation tracking (HashSet of active senders) for downtime detection (2026-03-10)
- `aztb_getValidatorStake(validator_id)` RPC: self_stake, total_delegated, effective_stake, active, slash_history (2026-03-10)
- `aztb_getDelegation(address)` RPC: validator_id, amount, round_delegated (or null) (2026-03-10)
- `aztb_getActiveValidators()` RPC: sorted by effective_stake descending (2026-03-10)
- `aztb_getUnbondingStatus(address)` RPC: pending unbonding entries with amounts + available_round (2026-03-10)
- 3 new governance-controllable staking params: min_validator_stake, max_stake_cap, validator_commission_bps (2026-03-10)
- ~34 new tests: 24 staking unit + 4 RPC + 1 chain_params + pipeline tests (2026-03-10)

### Changed
- `ExecutionPipeline`: added staking_store, slash_rx/slash_tx, epoch_participation fields (2026-03-10)
- `RpcState`: added `staking_store: Option<Arc<RwLock<StakingStore>>>` field (2026-03-10)
- `ChainParams`: 12 params total (9 existing + 3 staking governance) (2026-03-10)
- `TxKind` routing: 19 variants (15 existing + 4 staking) (2026-03-10)

---

## [M8-S12] -- Token Supply, Emission & Vesting (Sprint 037)

### Added
- `tokenomics` module in aztibase-execution: token supply constants, emission schedule, vesting, genesis allocations, epoch reward distribution (2026-03-09)
- `EmissionSchedule`: 2-year halving curve (120M→7.5M over 10 years, 3.75M/year tail), `emission_for_year()`, `emission_per_epoch()`, `cumulative_emission()` (2026-03-09)
- `EmissionDistribution`: 4-pool split — 70% validators, 15% PoUW, 10% treasury, 5% insurance (2026-03-09)
- `StakingAPY`: piecewise linear curve (12% max at <20% staking → 3% floor at >70%), `calculate_apy_bps()` (2026-03-09)
- `VestingSchedule`: cliff + linear unlock, `vested_at()`, `locked_at()`, `cliff_end_round()`, `end_round()` (2026-03-09)
- `GenesisAllocation`: 8 categories (Treasury 100M, EcoDev 80M, Team 60M, Foundation 40M, Airdrop 40M, Validator Bootstrap 40M, AI Fund 20M, Liquidity 20M) totaling 400M genesis mint (2026-03-09)
- `EmissionTracker`: runtime state tracking current_epoch, total_emitted, treasury/insurance balances, hard cap enforcement (2026-03-09)
- `validator_epoch_reward()`: per-validator reward based on stake × participation ratio (2026-03-09)
- `aztb_getEmissionInfo` RPC: current epoch, total emitted/remaining, hard cap, treasury/insurance balances (2026-03-09)
- `aztb_getVestingStatus(category)` RPC: total/vested/locked amounts, cliff/end rounds for any genesis allocation (2026-03-09)
- ADR-014: u128 tokenomics with deferred u64→u128 balance migration strategy (2026-03-09)
- 39 new tests: 36 tokenomics unit + 3 RPC (2026-03-09)

### Changed
- `ExecutionPipeline`: added `emission_tracker` field, `shared_emission_tracker()` method (2026-03-09)
- `RpcState`: added `emission_tracker: Option<Arc<RwLock<EmissionTracker>>>` field (2026-03-09)

---

## [M8-S11] -- Governance Execution & Chain Parameters (Sprint 036)

### Added
- `ChainParams` registry: runtime-mutable parameter store with typed `ParamValue` (U64/Bool/Str), bounds validation, get/set/list, 9 governance-controllable parameters (2026-03-09)
- `ParamDef` registry: static table of known param keys with type, default, min/max bounds, description (2026-03-09)
- `GovernanceStore::passed_unexecuted()`: returns Passed proposals not yet Executed (2026-03-09)
- `BaseFeeCalculator::update_with_params()`: accepts governance-controlled fee parameters (2026-03-09)
- Pipeline: auto-executes Passed governance proposals — applies param_key/param_value to ChainParams, calls mark_executed() (2026-03-09)
- Pipeline: failed proposal execution (invalid key/value) still marks Executed to prevent retry loops (2026-03-09)
- `aztb_getChainParam(key)` RPC: returns current value, type, and description for a chain parameter (2026-03-09)
- `aztb_listChainParams()` RPC: returns all chain parameters sorted by key (2026-03-09)
- 13 new tests: 7 ChainParams unit + 4 governance execution pipeline + 2 RPC (2026-03-09)

### Changed
- `BaseFeeCalculator`: fee update now reads from ChainParams (target_gas, change_denom, fee_floor, fee_ceiling) (2026-03-09)
- Pipeline eviction: reads max_stored_txs, max_stored_batch_roots, max_stored_receipts from ChainParams (2026-03-09)
- `RpcState`: added `chain_params: Option<Arc<RwLock<ChainParams>>>` field (2026-03-09)
- `ExecutionPipeline`: added `chain_params` field, `shared_chain_params()` method (2026-03-09)

---

## [M8-S10] -- On-Chain Governance Foundations (Sprint 035)

### Added
- `GovernanceStore`: in-memory proposal store with stake-weighted voting, quorum threshold (>50% approve, ≥2 voters), and automatic finalization (2026-03-09)
- `Proposal`, `Vote`, `VoteTally`, `ProposalStatus` types in `aztibase-execution::governance` (2026-03-09)
- `TxKind::CreateProposal` (0x0E): proposer, description, param_key, param_value, voting_period, nonce, gas_price (2026-03-09)
- `TxKind::CastVote` (0x0F): voter, proposal_id, approve, nonce, gas_price (2026-03-09)
- Pipeline: CreateProposal validation (description length, param bounds, voting period), CastVote uses voter balance as stake weight (2026-03-09)
- Pipeline: automatic proposal finalization at `end_round` after each batch execution (2026-03-09)
- `aztb_getProposal(proposal_id)` RPC: returns proposal details + vote tally (2026-03-09)
- `aztb_listProposals(status_filter?)` RPC: returns proposals filtered by Active/Passed/Rejected/Executed (2026-03-09)
- 8 new tests: 6 governance unit tests + 2 routing roundtrip tests — 674 total (2026-03-09)

### Changed
- `RpcState`: added `governance: Option<Arc<RwLock<GovernanceStore>>>` field (2026-03-09)
- `ExecutionPipeline`: added `governance` field, `shared_governance()` method (2026-03-09)

---

## [M8-S9] -- Archive Node & Historical Queries (Sprint 034)

### Added
- `--archive` CLI flag and `archive: bool` config field: disables all eviction and DAG pruning for full history retention (2026-03-09)
- `BATCH_INDEX_TABLE`: maps batch number (u64 BE) → anchor hash for sequential block lookups (2026-03-09)
- `BATCH_TXS_TABLE`: maps anchor hash → postcard(Vec<[u8; 32]>) for batch→transaction associations (2026-03-09)
- 7 new JSON-RPC methods: `aztb_getBlockByNumber`, `aztb_getBlockByHash`, `aztb_getTransactionByHash`, `aztb_getBatchRoot`, `aztb_getBlockRange`, `aztb_getTransactionsByBatch`, `aztb_getReceiptsByBatch` (2026-03-09)
- `MAX_BLOCK_RANGE = 100` pagination limit on `aztb_getBlockRange` to prevent unbounded reads (2026-03-09)
- 6 new persist functions: `store_batch_index`, `get_batch_by_number`, `store_batch_txs`, `get_batch_txs`, `store_transaction`, `get_batch_range` (2026-03-09)
- 4 new persist tests: batch index roundtrip, batch txs roundtrip, transaction store/retrieve, batch range query (2026-03-09)

### Changed
- `ConsensusConfig`: added `archive: bool` field, gates `prune_before()` and state pruning (2026-03-09)
- `ExecutionPipeline`: added `archive` field and `set_archive()`, gates eviction of receipts/txs/batch_roots (2026-03-09)
- Pipeline batch persistence: now stores batch index, batch txs, and individual transactions in TX_TABLE (2026-03-09)
- `ALL_TABLES` array in storage: expanded from 10 to 12 tables (2026-03-09)

---

## [M8-S8] -- State Pruning & Bounded Growth (Sprint 033)

### Added
- `DagStore::prune_before(committed_round)`: round-based pruning removes old blocks from in-memory index + on-disk BLOCKS_TABLE, with `DAG_RETENTION_BUFFER = 16` rounds (2026-03-09)
- `StateStore::delete_batch()`: atomic multi-key deletion in a single write transaction (2026-03-09)
- `evict_old_transactions()`: count-based TX_TABLE eviction capped at 500K entries (2026-03-09)
- `evict_old_batch_roots()`: count-based BATCH_ROOTS_TABLE eviction capped at 100K entries (2026-03-09)
- 8 new unit tests: 5 DagStore pruning + 3 disk eviction (2026-03-09)

### Changed
- `ConsensusEngine::evaluate_commits()`: auto-prunes DAG after successful commit wave (2026-03-09)
- `executed_anchors`: converted from unbounded `HashSet` to bounded VecDeque+HashSet ring buffer (`MAX_EXECUTED_ANCHORS = 10_000`) with FIFO eviction (2026-03-09)
- `attestation_buffer`: per-task cap (`MAX_ATTESTATIONS_PER_TASK = 32`) and total cap (`MAX_ATTESTATION_BUFFER_TASKS = 2048`) with oldest-task eviction (2026-03-09)
- Pipeline batch persistence: now triggers best-effort eviction of receipts, transactions, and batch roots tables (2026-03-09)

---

## [M8-S7] -- Adversarial Consensus Testing (Sprint 032)

### Added
- `FaultRouter`: configurable fault injection harness with drop rate, network partitions, and message reordering (2026-03-09)
- `AdversarialTestbed`: multi-node consensus test runner using FaultRouter for injected-fault scenarios (2026-03-09)
- 14 new adversarial consensus tests — 175 total node tests (2026-03-09)
- Tests: fault_router_drop_rate, fault_router_partition_isolation, fault_router_no_partition_reaches_all, fault_router_reorder_changes_delivery_order (2026-03-09)
- Tests: leader_equivocation_rejected, multi_byzantine_below_threshold, conflicting_vertex_flood_capped, invalid_parent_hash_rejected, duplicate_vertex_ignored (2026-03-09)
- Tests: network_partition_and_heal, consensus_stall_minority_online, finality_cert_forgery_rejected, buffer_exhaustion_graceful, message_reordering_convergence (2026-03-09)

### Changed
- `ConsensusEngine`: `state`, `insert_genesis()`, `handle_input()`, `handle_received_vertex()` made public for external test access (2026-03-09)

---

## [M8-S6] -- End-to-End Smoke Tests & Integration Testing (Sprint 031)

### Added
- 13 new e2e integration tests covering previously untested paths (2026-03-09)
- `verify_and_route_with_pubkey()`: returns routed TxKind + raw Ed25519 pubkey from signed envelope (2026-03-09)
- `verify_and_route_batch_with_pubkeys()`: batch variant returning sender→pubkey map (2026-03-09)
- `BatchWithPubkeys` type alias for complex return type (2026-03-09)
- Tests: fee_market_transfer_e2e, multi_transfer_batch_stress, nonce_gap_rejection_e2e, insufficient_balance_receipt_e2e (2026-03-09)
- Tests: register_model_e2e, commit_compute_e2e, post_task_e2e, submit_attestation_e2e (2026-03-09)
- Tests: full_ai_lifecycle_e2e (RegisterModel → 2x CommitCompute → PostTask → 2x SubmitAttestation → Settlement) (2026-03-09)
- Tests: deregister_compute_refund_e2e, duplicate_model_registration_e2e, deregister_model_with_pending_tasks_blocked, batch_mixed_tx_types_e2e (2026-03-09)

### Fixed
- **CRITICAL BUG**: Attestation signature verification was using BLAKE3-hashed address as Ed25519 public key, making all attestation signatures invalid through the signed tx flow. Fixed by threading raw pubkeys from envelope through batch routing (SEC-ATT-PUBKEY-001) (2026-03-09)

---

## [M8-S5] -- WebRTC Direct Transport & DCUtR Hole Punching (Sprint 030)

### Added
- `dcutr::Behaviour` integrated into `AztibaseBehaviour` swarm for NAT hole punching (2026-03-08)
- `NatTraversalStats`: tracks DCUtR upgrade attempts, successes, failures (2026-03-08)
- `build_libp2p_transport()`: real libp2p-webrtc transport behind `webrtc` feature flag (2026-03-08)
- `enable_webrtc` and `webrtc_listen_port` in `TransportConfig` and `NetworkConfig` (2026-03-08)
- `--webrtc` CLI flag for enabling WebRTC direct transport (2026-03-08)
- 4 new tests (DCUtR behaviour, stats default/clone, WebRTC config propagation) — 60 total network tests (2026-03-08)

### Changed
- Dockerfile: EXPOSE now includes 30333 (P2P), 9944 (RPC), 9000 (WebRTC) (2026-03-08)
- `dcutr` added to workspace libp2p features (2026-03-08)
- Full-node and light-node TransportConfig construction wired with WebRTC config from CLI + TOML (2026-03-08)

---

## [M8-S4] -- Gossipsub Hardening & Persistent Peer Discovery (Sprint 029)

### Added
- `PeerStore`: redb-backed persistent peer address storage with merge/dedup, list_recent, prune_stale, evict_excess (2026-03-08)
- `validate_gossip_message()`: structural validation — reject empty, oversized, per-topic minimum sizes (blocks ≥ 64B, tx ≥ 32B, consensus ≥ 32B) (2026-03-08)
- `MessageAcceptance` enum: Accept/Reject/Ignore for gossip message validation (2026-03-08)
- `load_cached_peers()`: on startup, dial up to 50 most-recent peers from PeerStore, skip banned (2026-03-08)
- Kademlia bootstrap: triggered on first PeerConnected, routing updates persisted to PeerStore (2026-03-08)
- 9 new tests (2 scoring, 4 peer store, 3 message validation) — 56 total network crate tests (2026-03-08)

### Changed
- Gossipsub per-topic weights: consensus=2.0, blocks=1.5, validator-announce=1.5, tx=1.0, state-sync=0.5, ai-proofs=0.5 (2026-03-08)
- `invalid_message_deliveries_weight`: -10 → -50, decay 0.3 → 0.1 (2026-03-08)
- `mesh_message_deliveries_threshold`: 20 → 50, cap 100 → 500 (2026-03-08)
- `publish_threshold`: -50 → -30, `graylist_threshold`: -80 → -60 (2026-03-08)
- Gossipsub event handler: validates messages before processing, records Medium offense on rejection (2026-03-08)
- `KademliaEvent::RoutingUpdated`: persists peer addresses to PeerStore (2026-03-08)

---

## [M8-S3] -- Networking Hardening (Sprint 028)

### Added
- `PeerReputationStore`: redb-backed persistent peer reputation with tiered bans (1hr/24hr/7day), score decay (+1/hr), offense tracking, stale pruning, excess eviction (2026-03-08)
- `ConnectionFilter`: per-IP connection limits (MAX=3), rate limiting (1000ms cooldown), per-subnet /16 caps (MAX=5) with IPv4/IPv6 support (2026-03-08)
- `NatStatus` enum: AutoNAT-driven NAT type detection (Public/Private/Unknown), logged on status change (2026-03-08)
- Relay client integration: NAT'd nodes automatically listen on relay circuit addresses when Private detected (2026-03-08)
- `TransportConfig` extended: `reputation_store`, `enable_autonat`, `relay_servers`, `autonat_probe_interval_secs` (2026-03-08)
- 18 new network tests (7 reputation, 7 connection filter, 4 autonat/relay) — 47 total network crate tests (2026-03-08)

### Changed
- `AztibaseBehaviour`: added `autonat::Behaviour` and `relay::client::Behaviour` fields (2026-03-08)
- `SwarmBuilder`: uses `.with_relay_client()` for noise+yamux relay transport (2026-03-08)
- `ConnectionEstablished` handler: ban check → connection filter → peer IP tracking (2026-03-08)
- `ConnectionClosed` handler: releases IP from connection filter (2026-03-08)
- `OutgoingConnectionError`: records Low offense in reputation store (2026-03-08)
- libp2p features: added `autonat` and `relay` to workspace Cargo.toml (2026-03-08)

---

## [M8-S2] -- Docker Testnet Bootstrap & Genesis Tooling (Sprint 027)

### Added
- `aztibase genesis --docker` flag: generates Docker-ready data layout (genesis.toml, per-node configs with boot_nodes, validator keys) (2026-03-08)
- `write_docker_configs()` in genesis.rs: Docker-aware layout with `/dns4/` boot_nodes for container peer discovery (2026-03-08)
- `aztb_chainId` RPC: returns chain ID as hex string (2026-03-08)
- `aztb_genesisHash` RPC: returns BLAKE3 hash of genesis config for network identity verification (2026-03-08)
- `genesis_hash()` function: deterministic BLAKE3 hash of serialized genesis TOML (2026-03-08)
- `Makefile` with testnet lifecycle targets: setup, start, stop, reset, logs, status (2026-03-08)
- `scripts/setup-docker-testnet.sh`: one-command Docker testnet genesis generation (2026-03-08)
- `scripts/reset-testnet.sh`: wipes node databases while preserving keys and genesis (2026-03-08)

### Changed
- `docker-compose.yml`: simplified validator commands to use TOML config only (no CLI overrides) (2026-03-08)
- `Dockerfile`: added `curl` to runtime image for container healthchecks (2026-03-08)
- `write_keyfile()`: refactored to accept filename parameter (decoupled from hex address) (2026-03-08)

---

## [M8-S1] -- Prometheus Metrics + Testnet Infrastructure (Sprint 026)

### Added
- `prometheus-client` v0.23 (pure Rust) metrics registry with 12 typed counters/gauges (2026-03-08)
- `GET /metrics` endpoint: Prometheus text format (text/plain; version=0.0.4) for Grafana/Prometheus scraping (2026-03-08)
- `GET /metrics/json` endpoint: backward-compatible JSON metrics (ADR-008) (2026-03-08)
- `GET /health` endpoint: returns `{"status":"ok","blockHeight":N}` for Docker/LB healthchecks (2026-03-08)
- `aztb_faucetDrip` RPC: testnet-only (chain_id=0xA27B), 10 AZTB per drip, 60s rate limit per address (2026-03-08)
- `aztb_nodeInfo` RPC: returns version, chain_id, block_height, protocol_version (2026-03-08)
- Prometheus scrape config for 3-validator Docker setup (monitoring/prometheus.yml) (2026-03-08)
- Grafana provisioned datasource + 2 auto-loaded dashboards: Node Health and Consensus (2026-03-08)
- Docker Compose: Prometheus (port 9090) and Grafana (port 3000) services with persistent volumes (2026-03-08)

### Changed
- Metrics backend: `Arc<RwLock<serde_json::Value>>` replaced by `NodeMetrics` (typed prometheus-client registry) (2026-03-08)
- Docker healthcheck: uses `GET /health` instead of JSON-RPC POST (simpler, faster) (2026-03-08)

---

## [M7-S2] -- Dependency Upgrades, Crypto Audit, Verkle Foundations (Sprint 025)

### Security
- wasmtime upgraded v28→v42: resolves 4 CVEs (2026-03-08)
- bincode→postcard migration: resolves RUSTSEC-2025-0141 (unmaintained crate) (2026-03-08)
- Ed25519 transaction signing: `AZTB_TX_V1` domain prefix prevents cross-protocol replay (2026-03-08)
- Attestation hashing: `AZTB_ATTESTATION_V1` domain prefix added (2026-03-08)
- Merkle tree: `LEAF_DOMAIN`/`NODE_DOMAIN` prefixes prevent leaf-node collision attacks (2026-03-08)
- Verkle proof verification: replaced trivially-forgeable placeholder with domain-separated BLAKE3 commitment verification (2026-03-08)
- BLS PoP enforcement confirmed at CommitCompute registration (2026-03-08)

### Added
- 8 known-answer cryptographic test vectors: BLAKE3 (3), Ed25519 (2), BLS12-381 (3) — pins exact hex values (2026-03-08)
- cargo-fuzz targets: 3 for aztibase-core (Transaction/BlockHeader deser, hash+pubkey), 3 for aztibase-consensus (Vertex/FinalityCert deser, SignerBitmap ops) (2026-03-08)
- ADR-013: Verkle BLAKE3 placeholder retained — no mature pure-Rust IPA crate exists (2026-03-08)

### Changed
- `VerkleProof` struct: now carries `stem`, `value_hash`, and `levels` (self-contained proof) (2026-03-08)
- `VerkleTree::verify_proof()`: bottom-up commitment recomputation with 256-width inner nodes (2026-03-08)
- WASM `JsVerkleProof`: updated to match new proof structure (2026-03-08)

---

## [M7] -- Security Audit + Hardening (COMPLETE)

### Security
- Cross-VM reentrancy guard: `ReentrancyGuard` prevents recursive calls between WASM and EVM VMs (SEC-BRIDGE-003 MEDIUM) (2026-03-08)
- EVM revert reason truncation at 1024 bytes: prevents unbounded error strings (SEC-EVM-008) (2026-03-08)
- AI inference timeout enforcement: 30s default with elapsed check + 16MB input size limit (SEC-AI-002) (2026-03-08)
- Receipt store eviction: automatic pruning at 100K entries via `evict_old_receipts()` (SEC-RCPT-002) (2026-03-08)
- Per-IP WebSocket rate limiting: `IpConnectionTracker` with MAX_WS_PER_IP=8, returns 429 (SEC-WS-002) (2026-03-08)
- Proof cache bounded eviction: MAX_PROOF_CACHE_ENTRIES=4096, round-sorted removal (SEC-CACHE-001) (2026-03-08)
- STUN URI validation: scheme/host/port parsing on WebRTC transport construction (SEC-WEBRTC-001) (2026-03-08)
- Multi-peer header validation: `MultiPeerValidator` requires majority consensus for eclipse attack resistance (SEC-P2P-001) (2026-03-08)
- Finality certificate structural validation: reject zero batch_hash, zero signers, empty bitmap (SEC-LC-001) (2026-03-08)

### Changed
- `FinalityCertificate.signer_bitmap`: replaced `Vec<bool>` with packed `SignerBitmap` (Vec<u8>), 8x bandwidth savings (SEC-BLS-009) (2026-03-08)
- `StateStore.evict_oldest()`: generic count-based eviction method for any redb table (2026-03-08)

### Added
- `deny.toml`: cargo-deny configuration for license/advisory/ban/source policy enforcement (2026-03-08)
- EVM gas cost verification tests: deploy and call gas bounds assertions (SEC-EVM-002) (2026-03-08)

---

## [M6] -- AI Compute Market + PoUW (COMPLETE)

### Added
- `ModelRegistry` in aztibase-execution: on-chain model metadata with register/query/deregister/list_active, backed by AccountState storage at `MODEL_REGISTRY_ADDRESS` (2026-03-07)
- `TxKind::RegisterModel` (0x08): register AI models on-chain with owner, fingerprint, compute_cost, min_stake (2026-03-07)
- `TxKind::PostTask` (0x09): post inference tasks with model_id, input_hash, reward escrow, deadline_round (2026-03-07)
- `ComputeCommitment` + `ComputeCommitmentStore`: validator opt-in to provide AI compute for specific models (2026-03-07)
- `SlidingWindowPoUWScore`: multi-metric PoUW scoring — 0.4 accuracy + 0.3 latency + 0.3 availability, replaces `StubPoUWScore` (2026-03-07)
- `AttestationAggregator`: quorum-based attestation consensus (≥ 2 matching result_hash, validator dedup) (2026-03-07)
- `ValidatorWorkHistory`: sliding window attestation tracking with accuracy/latency/availability calculations and pruning (2026-03-07)
- `TaskPool`: pending inference task storage with model index, 1024 capacity limit, expiry eviction (2026-03-07)
- `TaskAssigner`: select best validator by PoUW score from committed candidates (2026-03-07)
- `TaskSettlement`: settle completed tasks — verify attestation quorum, distribute reward with remainder handling (2026-03-07)
- `aztb_getModelInfo` RPC: query model metadata by ID (2026-03-07)
- `aztb_listModels` RPC: list all active registered models (2026-03-07)
- `aztb_getTaskStatus` RPC: query pending inference task by task_id (2026-03-07)
- Pipeline execution for RegisterModel and PostTask transactions with nonce checks, fee escrow, and receipts (2026-03-07)
- ADR-010: PoUW scoring formula and attestation quorum design (2026-03-07)
- `TxKind::SubmitAttestation` (0x0A): validators submit signed attestations with Ed25519 signature verification (2026-03-07)
- `TxKind::CommitCompute` (0x0B): validators register as compute providers with stake bond (2026-03-07)
- TaskPool wired into ExecutionPipeline: PostTask inserts into live pool, expired tasks auto-refunded (2026-03-07)
- Attestation → quorum → settlement flow: `TaskSettlement::settle()` distributes rewards on quorum (≥ 2 matching) (2026-03-07)
- `aztb_pendingTaskCount` RPC: query count of pending inference tasks (2026-03-07)
- `aztb_getComputeCommitment` RPC: query validator compute commitment by ID (2026-03-07)
- `aztb_listComputeProviders` RPC: list validators committed to a specific model (2026-03-07)
- Attestation buffer cleanup on task expiry to prevent unbounded memory growth (2026-03-07)
- ADR-011: Attestation signature scheme and settlement flow design (2026-03-07)

- `TxKind::DeregisterCompute` (0x0C): validators deregister from compute market with stake refund (2026-03-07)
- `TxKind::DeregisterModel` (0x0D): model owners deregister models (blocked if pending tasks exist) (2026-03-07)
- TaskAssigner pipeline wiring: PostTask automatically assigns best validator via PoUW score from ComputeCommitmentStore candidates (2026-03-07)
- `InferenceTask.assigned_validator`: tracks which validator is assigned to execute the task (2026-03-07)
- SubmitAttestation enforcement: rejects attestations from validators not matching assigned_validator (2026-03-07)
- CommitCompute model validation: validates all supported_models exist in ModelRegistry before accepting commitment (2026-03-07)
- CommitCompute overwrite refund: previous stake automatically refunded when validator updates commitment (2026-03-07)
- `aztb_getTaskStatus` now includes `assignedValidator` field in response (2026-03-07)

### Fixed
- SEC-COMMIT-OVERWRITE (LOW): CommitCompute now refunds prior stake on overwrite, preventing locked funds (2026-03-07)
- SEC-SYBIL-001: `AttestationAggregator` now deduplicates attestations by validator_id, preventing single-validator quorum faking (2026-03-07)
- SEC-ATT-LEAK-001 (LOW): Attestation buffer entries now cleaned up when tasks expire, preventing memory leak (2026-03-07)

---

## [M5] -- Light/Browser Nodes + Wallet (COMPLETE)

### Added
- WebSocket gateway on full node: `/ws` upgrade handler on axum RPC server, JSON-RPC dispatch over WebSocket text frames, max 256 concurrent connections (2026-03-07)
- Light sync over WebSocket: `RequestHeaders`, `RequestBalance`, `RequestProof` message handling from browser light clients (2026-03-07)
- `aztb_subscribe` method: WebSocket-only, supports `"newHeads"` and `"finality"` topics, returns subscription ID (2026-03-07)
- `aztb_unsubscribe` method: cancel subscription by ID, aborts forwarding task (2026-03-07)
- `EventBus`: `tokio::sync::broadcast` channels for `newHeads` and `finality` events, published on batch commit (2026-03-07)
- HTTP subscribe returns clear error: "Subscriptions are only available over WebSocket" (2026-03-07)
- `aztibase wallet list`: scan keyfile directory, display address + encrypted status per keyfile (2026-03-07)
- `aztibase wallet balance --address <addr> --rpc <url>`: query `aztb_getBalance` + `aztb_getNonce` via JSON-RPC (2026-03-07)
- `aztibase wallet export --key <path>`: export encrypted keyfile as portable JSON (2026-03-07)
- `aztibase wallet import --file <path>`: import keyfile JSON with format validation (2026-03-07)
- `Dockerfile`: multi-stage build (rust:1.88 + debian:bookworm-slim), exposes 9944 (RPC/WS) + 9000 (P2P) (2026-03-07)
- `docker-compose.yml`: 3-node local testnet with health checks (2026-03-07)
- `aztibase-wasm` crate: browser-compatible WASM light client (cdylib + rlib), wasm-bindgen exports for trustless verification (2026-03-07)
- WASM header chain verification: `verifyHeaderChain(headersJson, certJson, expectedStart)` — validates sequential rounds, quorum, signatures (2026-03-07)
- WASM Merkle proof verification: `verifyMerkleProof(proofJson, rootHex, leafHex)` — sibling-path reconstruction (2026-03-07)
- WASM Verkle proof verification: `verifyVerkleProof(proofJson, rootHex)` — path commitment check (2026-03-07)
- WASM light client proof verification: `verifyLightClientProof(proofJson, leafHex)` — inner Merkle/Verkle dispatch with finality cert check (2026-03-07)
- WASM utility: `blake3Hash(data)` returns hex-encoded BLAKE3 hash, `buildHeaderRequest(fromRound, count)` builds sync request JSON (2026-03-07)
- `LightClient` JS class: `connect(wsUrl)`, `sync()`, `getBalance(address)`, `verifyProof(proofJson, leafHex)`, WebSocket transport bridge (2026-03-07)
- `HeaderCache` JS class: IndexedDB-backed header persistence (`headers` + `meta` stores), survives page reloads (2026-03-07)
- Browser demo page: dark-themed UI with connect/sync/disconnect controls, proof verification panel, event log (2026-03-07)
- HD wallet derivation: `derive_account(phrase, account_index)` for path `m/44'/aztb'/N'/0/0` using BLAKE3 domain separation (2026-03-07)
- `aztibase wallet derive` CLI: `--phrase`, `--index`, optional `--output` + `--passphrase` for encrypted keyfile export (2026-03-07)
- Index 0 backward-compatible: `derive_account(phrase, 0)` produces same keypair as existing `keypair_from_mnemonic` (2026-03-07)
- BIP-39 mnemonic wallet: `generate_mnemonic()` (24-word), `recover_from_mnemonic()` (12/24-word), BLAKE3 domain-separated key derivation `m/44'/aztb'/0'/0/0` (2026-03-07)
- Argon2id encrypted keyfiles: `encrypt_keyfile()` / `decrypt_keyfile()` with ChaCha20-Poly1305 AEAD, 256MB memory cost, 3 iterations (2026-03-07)
- `EncryptedKeyFile` JSON format: salt, nonce, Argon2id params, ciphertext — passphrase-protected secret key storage (2026-03-07)
- `--mnemonic` flag on `aztibase wallet generate`: produces 24-word seed + encrypted keyfile (2026-03-07)
- `aztibase wallet recover`: restores keypair from 12/24-word mnemonic phrase (2026-03-07)
- `--passphrase` flag on `aztibase wallet show`: decrypts encrypted keyfiles for display (2026-03-07)
- `LightStore` in `aztibase-storage/src/light.rs`: dedicated redb instance with 5 tables (headers, finality_certs, proof_cache, wallet_state, peer_cache) (2026-03-07)
- Light node header CRUD: `store_header()`, `get_header()`, `latest_header_round()`, `store_headers_batch()` with u64-keyed rounds (2026-03-07)
- Light node finality cert CRUD: `store_finality_cert()`, `get_finality_cert()`, `latest_finality_round()` (2026-03-07)
- Proof cache with TTL eviction: `cache_proof()`, `get_cached_proof()`, `evict_stale_proofs(max_age_rounds)` (2026-03-07)
- Local wallet state storage: `store_wallet_state()`, `get_wallet_state()` keyed by address (2026-03-07)
- `LightSyncMessage` enum: `RequestHeaders`, `ResponseHeaders`, `RequestProof`, `ResponseProof` with version-tagged bincode encoding (2026-03-07)
- `LightSyncProtocol` state machine: tracks `last_synced_round` / `target_round`, generates batched sync requests (max 100 headers) (2026-03-07)
- `verify_header_chain()`: validates sequential rounds, finality cert anchor, BLS quorum >= 2/3, non-empty signatures (2026-03-07)
- `--light` CLI flag: starts node in light mode (LightStore, header sync only, no execution/consensus) (2026-03-07)
- `/aztibase/light-sync/1` request-response protocol: `LightSyncCodec` (bincode framing, 1MB max), full-node header serving (2026-03-07)
- Light node P2P sync loop: async header sync with peer scoring (failure tracking, latency preference), chain verification, LightStore persistence (2026-03-07)
- `sign_transfer_encrypted()`: sign transfers from Argon2id-encrypted keyfiles (2026-03-07)
- `broadcast_transaction()`: submit signed tx to JSON-RPC endpoint via reqwest, returns tx hash (2026-03-07)
- `--passphrase` flag on `aztibase wallet transfer`: decrypt encrypted keyfile for signing (2026-03-07)
- `--rpc` flag on `aztibase wallet transfer`: broadcast signed tx to a full node RPC endpoint (2026-03-07)
- `PeerId` re-exported from `aztibase-network` for cross-crate use (2026-03-07)

### Security
- Sprint 020 security review: 0 ELEVATED, 0 MEDIUM, 3 LOW (2026-03-07)
  - SEC-WS-002 (LOW): WebSocket connections limited to 256 but no per-IP rate limiting; relies on reverse proxy for production
  - SEC-SUB-001 (LOW): Broadcast channels bounded at 256; slow consumers get lagged errors, not unbounded memory growth
  - SEC-EXPORT-001 (LOW): Wallet export outputs encrypted keyfile JSON only; no plaintext secret keys in export format
- Sprint 019 security review: 0 ELEVATED, 0 MEDIUM, 3 LOW (2026-03-07)
  - SEC-WASM-001 (LOW): WASM linear memory may expose data to JS host; mitigated by verification-only design (no secrets in WASM module)
  - SEC-WS-001 (LOW): WebSocket transport has no built-in auth; relies on wss:// for confidentiality
  - SEC-HD-001 (LOW): Non-standard HD derivation (BLAKE3, not HMAC-SHA512 BIP-32); documented, Aztibase-specific by design
- Sprint 018 security review: 0 ELEVATED, 0 MEDIUM, 2 LOW (2026-03-07)
  - SEC-P2P-001 (LOW): Single-peer sync per cycle; eclipse mitigated by scoring but not multi-peer cross-validation
  - SEC-RPC-001 (LOW): Wallet broadcast doesn't enforce HTTPS; user responsibility to use secure endpoint
- Sprint 017 security review: 0 ELEVATED, 0 MEDIUM, 3 LOW (2026-03-07)
  - SEC-WALLET-001 (LOW): Mnemonic displayed to stdout — user responsibility to secure; zeroize applied to seed bytes
  - SEC-LIGHT-001 (LOW): Light node trusts BLS quorum — by design, documented trust assumption
  - SEC-SYNC-003 (LOW): Header sync from single peer — future: multi-peer cross-validation

### Testing
- 456 tests total (10 new in Sprint 018): codec encode/decode request, codec encode/decode response, frame roundtrip, oversized frame rejection, light sync protocol advance, gap rejection in sync, light store persistence, sign_transfer_encrypted envelope, broadcast to mock RPC, broadcast RPC error propagation (2026-03-07)
- 446 tests total (21 new in Sprint 017): wallet mnemonic roundtrip, 12-word recovery, encrypt/decrypt roundtrip, wrong passphrase rejection, mnemonic+encrypted keyfile e2e, light store open, header CRUD, batch insert, finality cert CRUD, proof cache + eviction, wallet state CRUD, message serde roundtrip, version mismatch, request cap, valid header chain, gap rejection, cert outside batch, insufficient quorum, empty signature, sync protocol state machine, proof request/response (2026-03-07)

---

## [M4] -- Integration Testing + AI + Testnet (COMPLETE)

### Security
- Sprint 016 security review: 0 ELEVATED, 1 MEDIUM (SEC-BRIDGE-003: cross-VM reentrancy), 4 LOW, 1 INFO (2026-03-07)
- Sprint 015 security review: 0 ELEVATED, 0 MEDIUM, 3 LOW (SEC-METRIC-001, SEC-WEBRTC-001, SEC-LC-001) (2026-03-07)
- Block-STM parallel validation race FIXED: `finish_validation()` now checks all prior txs are Validated before accepting (2026-03-07)
- Sprint 014 security review: 0 ELEVATED, 0 MEDIUM, 4 LOW, 1 INFO (SEC-EQUI-001, SEC-BUF-001, SEC-VERKLE-001/002, SEC-METRICS-001) (2026-03-07)
- Sprint 012 security review: 0 ELEVATED, 0 MEDIUM, 4 LOW (SEC-KEY-001/002, SEC-FEE-004, SEC-BASE-001) (2026-03-07)
- SEC-FEE-002 CLOSED: Pre-execution fee escrow prevents underfunded execution (2026-03-07)
- SEC-FEE-003 CLOSED: BaseFeeCalculator persists to redb, loads on startup, drives real gas prices (2026-03-07)
- SEC-SIG-002 CLOSED: Pipeline validates nonces at execution time, not just mempool (2026-03-07)
- Sprint 011 security review: 0 ELEVATED, 2 MEDIUM documented (SEC-SIG-002, SEC-FEE-002), 6 LOW (2026-03-07)
- SEC-AA-001 CLOSED: Fee mechanism implemented — gas_price on all TxKind variants, post-execution fee deduction (2026-03-07)
- SEC-AA-002 CLOSED: Ed25519 signed transaction envelopes — every tx verified before execution (2026-03-07)
- SEC-AA-003 CLOSED: Nonce enforcement at mempool + nonce-ordered batch execution (2026-03-07)
- Sprint 010 security review: 0 ELEVATED, 0 MEDIUM, 6 LOW documented (SEC-STM-001/002, SEC-GS-001, SEC-AA-001/002/003) (2026-03-07)
- SEC-SYNC-001 FIXED: `SnapshotAssembler::new()` now validates `total_chunks` (max 64) to prevent OOM DoS (2026-03-07)
- SEC-SYNC-002 FIXED: `add_chunk()` validates chunk data size (max CHUNK_SIZE + 1024) to prevent memory exhaustion (2026-03-07)
- Sprint 009 security review: 2 ELEVATED fixed, 8 MEDIUM (2 fixed, 6 documented), 5 LOW documented (2026-03-07)

### Added
- `AnomalyScorer` with deterministic heuristic scoring for transaction anomaly detection (0.0-1.0) (2026-03-07)
- `anomaly_score` field on `ExecutionReceipt` with `#[serde(default)]` backward compat (2026-03-07)
- Anomaly scoring wired into execution pipeline: every tx receipt carries an advisory score (2026-03-07)
- Cross-VM bridge: `CrossVmCall`, `wasm_to_evm()`, `evm_to_wasm()` with depth limit of 4 (2026-03-07)
- `InferenceTask` type for PoUW: task_id, model_id, input_hash, requester, reward, deadline_round (2026-03-07)
- `InferenceAttestation` type for PoUW: validator attestation of completed inference work (2026-03-07)
- `PoUWScore` trait with `StubPoUWScore` returning 0.0 (stub for M6) (2026-03-07)
- `GET /metrics` HTTP endpoint with consensus + execution JSON counters, gated by `--metrics` CLI flag (2026-03-07)
- `MetricsConfig` in `NodeConfig` for metrics endpoint configuration (2026-03-07)
- `LightClientProof` variant in `StateProof`: wraps inner proof + state root + height + finality certificate (2026-03-07)
- `verify_light_client_proof()`: validates inner Merkle/Verkle proof, rejects empty certs and nested proofs (2026-03-07)
- `StateSnapshot` gains `height` and `finality_certificate` fields for canonical verification (2026-03-07)
- `snapshot_header_hash()` for compact snapshot identification (2026-03-07)
- Criterion benchmarks: Merkle/Verkle proof verification (100/1k/10k leaves), BLS cert verification (21/100 validators) (2026-03-07)
- WebRTC transport scaffold: `WebRtcTransport`, `WebRtcConfig`, STUN server config, feature-gated `webrtc` (2026-03-07)
- `stun_servers` field in `NetworkConfig` with Google STUN defaults (2026-03-07)
- Equivocation detection in consensus engine: tracks `(round, author)` pairs, rejects conflicting vertices (2026-03-07)
- Vertex buffering for missing parents: bounded buffer (MAX=64) with drain-on-arrival (2026-03-07)
- Byzantine fault injection integration test: 4-validator network with 1 equivocating validator (2026-03-07)
- Validator crash recovery integration test: 3/4 honest validators continue committing (2026-03-07)
- Criterion benchmark harness: consensus (vertex creation, DAG insert, commit eval) + execution (state root, transfers, latency) (2026-03-07)
- `ConsensusMetrics` struct with atomic counters for vertices, commits, equivocations, latency (2026-03-07)
- `StateCommitment` trait in aztibase-core: pluggable commitment backends with `commit()`, `prove()`, `verify()` (2026-03-07)
- `StateProof` enum: `Merkle(MerkleProof)` and `Verkle(VerkleProof)` variants (2026-03-07)
- `MerkleCommitment` implementing `StateCommitment`: sibling-path proofs with verify-by-reconstruction (2026-03-07)
- `VerkleTree` prototype: width-256 inner nodes, BLAKE3 placeholder commitments, insert/prove/verify (2026-03-07)
- `VerkleCommitment` implementing `StateCommitment`: Verkle tree wrapped as pluggable backend (2026-03-07)
- ADR-007: StateCommitment trait with enum-based proofs (2026-03-07)
- Genesis-driven validator bootstrap: `ValidatorSet` built from `GenesisConfig` instead of hardcoded placeholders (2026-03-07)
- `--validator-key <path>` CLI arg: node identity from Ed25519 key file with genesis validation (2026-03-07)
- BLS keypair generation in genesis: Ed25519 + BLS12-381 keypairs per validator (2026-03-07)
- `BlsKeypair::from_secret_bytes()` / `secret_bytes()` for BLS key file persistence (2026-03-07)
- `ValidatorSet` carries optional BLS public keys via `add_with_bls()` / `bls_keys_ordered()` (2026-03-07)
- `build_certificate_from_set()` / `verify_certificate_from_set()`: finality cert API pulling BLS keys from ValidatorSet (2026-03-07)
- `load_keyfile_full()`: returns `(Keypair, Address, Option<BlsKeypair>)` for BLS-aware key loading (2026-03-07)
- Per-validator TOML node configs generated by `aztibase genesis` subcommand (2026-03-07)
- `NodeConfig` gains `genesis_path` and `validator_key` fields, CLI flags override config file (2026-03-07)
- Integration test: full genesis → consensus → execution → BLS finality cert → verify flow (2026-03-07)
- `GenesisConfig` struct with TOML serialization for network bootstrapping (2026-03-07)
- `aztibase genesis` CLI: generates genesis.toml + Ed25519 key files for validators and funded accounts (2026-03-07)
- `aztibase wallet generate/show/transfer` CLI: key management and transaction signing (2026-03-07)
- `--genesis <path>` flag on node startup: applies genesis config to empty state (2026-03-07)
- `Keypair::from_secret_bytes()` / `Keypair::secret_bytes()` for key file persistence (2026-03-07)
- Pre-execution fee escrow: `escrow_fee()` wired into pipeline, `collect_fees()` removed, `refund_unused()` after execution (2026-03-07)
- `PipelineResult.total_fees_burned`: tracks total fees burned per batch (2026-03-07)
- Persistent base fee: `store_base_fee()` / `load_base_fee()` in redb STATE_TABLE (2026-03-07)
- `ExecutionPipeline` owns `BaseFeeCalculator` + `Arc<AtomicU64>`, updates after each batch (2026-03-07)
- Mempool gas price validation: `insert_checked()` rejects txs below current base fee (2026-03-07)
- Pipeline nonce validation: sequential nonce chain validated per-sender at execution time (2026-03-07)
- `aztb_gasPrice` returns live base fee from shared `Arc<AtomicU64>` (2026-03-07)
- `SignedTx` type: Ed25519 signed transaction envelope with wire format `[0xAA][len][payload][pubkey][sig]` (2026-03-07)
- `verify_and_route()` / `verify_and_route_batch()`: signature verification + sender address cross-check before execution (2026-03-07)
- `address_from_pubkey()`: BLAKE3 hash of Ed25519 public key for deterministic address derivation (2026-03-07)
- Mempool nonce validation: `insert_checked()` with MAX_NONCE_GAP=16, rejects stale and far-future nonces (2026-03-07)
- Nonce-ordered batch execution: pipeline sorts by (sender, nonce) before processing (2026-03-07)
- `gas_price: u64` field on all 7 TxKind variants with `gas_price()` and `gas_limit()` accessors (2026-03-07)
- `collect_fees()`: post-execution fee deduction (`gas_used * gas_price`) from sender balance (2026-03-07)
- `BaseFeeCalculator`: EIP-1559-style dynamic base fee (target 15M gas/batch, min 1, max 1B) (2026-03-07)
- `escrow_fee()` / `refund_unused()`: pre-execution fee escrow utilities (2026-03-07)
- `aztb_gasPrice` RPC method: returns current base fee (2026-03-07)
- `aztb_estimateGas` RPC method: returns gas estimates per transaction type (2026-03-07)
- Account abstraction: `AccountType` enum (EOA, Contract, AIAgent) with state-level type tracking (2026-03-07)
- `TxKind::CreateAgent` (0x07): creates AI agent accounts with model_id binding (2026-03-07)
- `aztb_getAccountType` RPC method: query account type by address (2026-03-07)
- Auto-promotion: `set_code()` promotes EOA → Contract automatically (2026-03-07)
- State root now includes account_type discriminant + model_id hash for integrity (2026-03-07)
- Snapshot format v2: includes account_type and model_id fields (backward-incompatible with v1) (2026-03-07)
- Persistence format extended with account_type + model_id (backward-compatible read of v1 records) (2026-03-07)
- Gossipsub protocol hardening: duplicate cache 2min, max transmit 2MiB, heartbeat 500ms, max 100 msgs/RPC (2026-03-07)
- Gossipsub peer scoring: per-topic scoring for all 6 topics, behaviour penalties, IP colocation penalty (2026-03-07)
- Connection limits: max 50 established connections, max 2 per peer, warnings on denied connections (2026-03-07)
- Block-STM parallel execution via rayon: thread-safe MVMemory, Arc-wrapped scheduler, rayon::scope worker pool (2026-03-07)
- Parallel threshold: batches >= 4 txs execute in parallel, smaller batches sequential (2026-03-07)
- SchedulerTask::Wait variant for multi-threaded worker coordination (2026-03-07)
- EVM precompiles: ecrecover (0x01), SHA-256 (0x02), RIPEMD-160 (0x03), identity (0x04), modexp (0x05) via revm built-in pure-Rust implementations (2026-03-07)
- Precompile verification: 12 tests confirm all 5 precompiles work through deployed-contract STATICCALL path (2026-03-07)
- bn128/KZG precompiles excluded (feature-gated C deps) — deferred to Sprint 010 (2026-03-07)
- State sync protocol: `StateSnapshot` with bincode serialization, 64 MiB limit, version-tagged format (2026-03-07)
- `apply_snapshot()`: restores `AccountState` from snapshot with state root verification (2026-03-07)
- `SyncMessage` enum: `SnapshotRequest` / `SnapshotResponse` protocol on TOPIC_STATE_SYNC (2026-03-07)
- Snapshot chunking: 1 MiB chunks with per-chunk BLAKE3 integrity hash (2026-03-07)
- `SnapshotAssembler`: reassembles chunked responses with ordering, hash, and state root validation (2026-03-07)
- `bootstrap_from_snapshot()`: applies snapshot to in-memory state + optional redb persistence (2026-03-07)
- Node bootstrap: empty-state nodes request snapshots from peers on `StateRootAnnounce` receipt (2026-03-07)
- Vertex wire format: version-prefixed encoding with size limits, hash integrity, validator check, round proximity (2026-03-06)
- `encode_vertex()` / `decode_vertex()` in aztibase-consensus/wire.rs (2026-03-06)
- `StateRootAnnounce`: broadcast state roots on TOPIC_STATE_SYNC after batch execution (2026-03-06)
- Execution pipeline result channel: `PipelineResult` sent back to main loop for state root broadcasting (2026-03-06)
- Multi-node consensus convergence test: 3 engines, shared genesis, async vertex routing, state root verification (2026-03-06)
- Local testnet launch script: `scripts/local-testnet.sh` starts 3 nodes on localhost (2026-03-06)
- Transaction receipt store: ExecutionReceipt persisted to redb RECEIPTS_TABLE with atomic batch writes (2026-03-06)
- `aztb_getTransactionReceipt` RPC method: query receipts by tx hash (2026-03-06)
- EVM execution via revm v36: `evm_deploy()` and `evm_call()` with CacheDB state adapter (2026-03-06)
- Dual VM architecture: WASM (wasmtime) + EVM (revm) coexisting in same AccountState (2026-03-06)
- TxKind::EvmDeploy (0x04) and TxKind::EvmCall (0x05) wire format variants (2026-03-06)
- TractRuntime: AIRuntime implementation with real ONNX model loading via tract-onnx (2026-03-06)
- Model registry: register/unregister ONNX models with thread-safe RwLock storage (2026-03-06)
- AI inference pipeline: f32 input → tract tensor → model run → f32 output serialization (2026-03-06)
- InferenceReceipt: deterministic BLAKE3 hash of (model_id || input || output) for verification (2026-03-06)
- `verify_inference()` stub: re-runs inference and compares deterministic hash (2026-03-06)
- `TxKind::AiInfer` (0x06): first-class AI inference transaction type with model_id, input, max_compute_units (2026-03-06)
- AI inference in execution pipeline: AiInfer transactions routed to TractRuntime via `dyn AIRuntime` trait (2026-03-06)
- `ExecutionReceipt.inference_hash`: deterministic hash field for AI inference verification (2026-03-06)
- `inferenceHash` field in `aztb_getTransactionReceipt` RPC response for AI transactions (2026-03-06)
- `NodeConfig.ai` section: model preload from TOML config with model_id + ONNX path (2026-03-06)
- Mempool priority ordering: `BTreeMap<(Reverse<priority>, TxHash)>` drains highest-priority first (2026-03-06)
- Mempool eviction: full pool evicts lowest-priority entry for higher-priority newcomer (2026-03-06)
- Mempool BLAKE3 hash dedup: `seen` set persists after removal to prevent tx replay (2026-03-06)
- Block-STM parallel execution engine: MVMemory, Scheduler, BlockSTMExecutor with optimistic execution + read-set validation (2026-03-06)
- `execute_full()`: returns both TxOutput and WriteSet for pipeline state application (2026-03-06)
- `apply_block_stm_to_state()`: applies Block-STM write sets to AccountState in tx order (2026-03-06)
- MVView: read-set-tracking wrapper over MVMemory with base state fallback (2026-03-06)

### Changed
- ExecutionPipeline now uses Block-STM (`BlockSTMExecutor::execute_full`) for transfer batches instead of sequential `execute_transfers` (2026-03-06)
- MSRV bumped from 1.85 to 1.88 for revm v36 compatibility (2026-03-06)
- Pipeline now collects receipts from transfers, WASM contracts, and EVM operations (2026-03-06)

### Security
- SEC-AI-001: 64 MiB model size cap prevents memory exhaustion via oversized ONNX models (2026-03-06)
- SEC-EVM-004: revm with `default-features = false, features = ["std"]` — no C dependencies (2026-03-06)
- EVM nonce validation and chain ID enforcement (0xDE0D) on both deploy and call (2026-03-06)
- AI input size validation rejects mismatched input before model execution (2026-03-06)
- SEC-MEM-001 FIXED: Mempool `seen` set bounded to max_size×10 with FIFO eviction — prevents unbounded memory growth (2026-03-06)
- SEC-AI-001 FIXED: Model path traversal blocked — paths must be under data_dir (canonicalized check) (2026-03-06)
- Sprint 008 security review: 2 MEDIUM (both fixed), 4 LOW — zero ELEVATED flags (2026-03-06)
- cargo-audit: 6 transitive vulns (ring, wasmtime ×3, tracing-subscriber), 7 warnings — all transitive, no action needed (2026-03-06)

### Testing
- 264 tests total (73 new): wire format (9), multi-node convergence (1), receipt store (5), receipt RPC (4), receipt e2e (1), EVM engine (3), EVM routing (2), EVM pipeline (3), tract runtime (13), AI routing (2), AI pipeline (3), mempool (11), Block-STM (16) (2026-03-06)

---

## [M3] -- Execution Layer (COMPLETE)

### Added
- JSON-RPC 2.0 server: axum-based HTTP server with 6 methods (aztb_getBalance, aztb_getNonce, aztb_getCode, aztb_sendTransaction, aztb_blockNumber, aztb_getStateRoot) (2026-03-06)
- RPC server wired into node binary with shared state via Arc<RwLock<AccountState>> (2026-03-06)
- Transaction submission via RPC flows to mempool and consensus engine (2026-03-06)
- ExecutionPipeline: consensus-to-execution wiring via tokio::mpsc channel (2026-03-06)
- TxKind routing: prefix-byte wire format (0x01=Transfer, 0x02=Deploy, 0x03=Call) (2026-03-06)
- State persistence: flush/load AccountState to redb across 4 tables (2026-03-06)
- Batch root storage: state root per committed batch, queryable by anchor hash (2026-03-06)
- Startup recovery: node loads persisted state from redb on launch (2026-03-06)
- BLS12-381 primitives: BlsKeypair, BlsPublicKey, BlsSignature via blst crate (2026-03-06)
- BLS signature aggregation and verification (2026-03-06)
- FinalityCertificate: batch hash + state root + aggregated BLS sig + signer bitmap (2026-03-06)
- Certificate creation (build_certificate) and verification (verify_certificate) (2026-03-06)

### Changed
- ConsensusOutput::BatchCommitted now carries full CommittedBatch instead of BlockHash (2026-03-06)
- Separate redb databases for consensus DAG and execution state (2026-03-06)

### Security
- ADR-004: blst C dependency justified exception for BLS12-381 (2026-03-06)
- Security review: consensus-execution wiring, state persistence, BLS certificates, tx routing (2026-03-06)
- BLS Proof-of-Possession: prevents rogue-key attacks on aggregate signatures (2026-03-06)
- BLS DST switched from NUL to POP ciphersuite for safe aggregation (2026-03-06)
- Finality message domain separator (AZTIBASE_FINALITY_V1) prevents cross-protocol replay (2026-03-06)
- Atomic state flush: single batch_put_multi() transaction for crash safety (2026-03-06)
- Safe deserialization: all unwrap() in persistence replaced with graceful error handling (2026-03-06)
- Duplicate signer deduplication in build_certificate (2026-03-06)
- cargo-audit: transitive advisories (ring, wasmtime WASI, lru, bincode) — no new critical issues (2026-03-06)
- SEC-WIRE-005: HashSet for committed blocks — O(1) dedup in extract_committed_batch (2026-03-06)
- SEC-WIRE-003: Double-execution guard — anchor_hash dedup prevents re-executing batches (2026-03-06)
- SEC-WIRE-004: Fatal flush handling — pipeline halts on persistence failure (2026-03-06)
- SEC-BLS-002: Unique BLS keys enforcement — duplicate keys rejected in certificate operations (2026-03-06)
- SEC-BLS-008: Quorum derived from ValidatorSet — prevents caller-supplied quorum manipulation (2026-03-06)
- SEC-ROUTE-006: func_name validation — rejects injection via malformed function names (2026-03-06)
- SEC-ROUTE-005: Strict bincode decoding — trailing bytes rejected (2026-03-06)

### Testing
- Integration test: transfer end-to-end — balances, nonces, redb persistence, batch root (2026-03-06)
- Integration test: contract deploy + call end-to-end — WASM deploy, storage_set call, state root change (2026-03-06)
- Integration test: finality certificate — BLS sign, aggregate, build + verify certificate (2026-03-06)
- Integration test: startup recovery — flush to redb, reload, verify state, execute more batches (2026-03-06)

### Milestone
- **M3 COMPLETE**: Execution layer feature-complete — WASM VM, state management, persistence, RPC, security hardened, 191 tests (2026-03-06)
- SEC-RPC-001: 1MB request body size limit on RPC server via axum DefaultBodyLimit (2026-03-06)
- ADR-005: axum chosen for JSON-RPC server (tokio-native, minimal, auditable) (2026-03-06)

---

## [M2] -- P2P Networking + Basic Consensus (Complete)

### Added
- ConsensusEngine: async round loop at 400ms with vertex proposal/reception (2026-03-06)
- VRF-based leader election via BLAKE3 PRF with stake-weighted selection (2026-03-06)
- CommittedBatch: deterministic topological ordering of committed vertices (2026-03-06)
- AccountState: in-memory account store with balance, nonce, code, contract storage (2026-03-06)
- SimpleTransfer execution: native token transfers with nonce/balance validation (2026-03-06)
- WASM contract deployment and execution with persisted storage (2026-03-06)
- BLAKE3 Merkle state root computation over sorted account state (2026-03-06)
- Mempool: bounded priority queue with dedup (2026-03-06)
- Round pruning: bounded vertices_by_round with 2-round safety buffer (2026-03-06)

### Security
- Security review: 12-item checklist passed, zero ELEVATED flags (Sprint 004) (2026-03-06)
- VRF seed derived from committed anchor hash -- not manipulable by proposer (2026-03-06)
- Balance arithmetic: checked subtraction + saturating_add -- no overflow/underflow (2026-03-06)
- cargo-audit: transitive advisories documented, no new issues affecting our code (2026-03-06)

---

## [M1] -- Core Primitives (Complete)

### Added
- CryptoProvider trait for quantum-ready crypto abstraction (2026-03-05)
- AIRuntime trait with Passthrough mode for non-AI nodes (2026-03-05)
- Node startup wiring: storage, network, TOML config, graceful shutdown (2026-03-05)
- Build-phase tracking infrastructure: STATUS.md, DECISIONS.md, CHANGELOG.md, BUILD_LOG.md (2026-03-05)

### Changed
- Switched storage backend from RocksDB to redb (ADR-001) (2026-03-05)

### Security
- Fixed WASM host function bounds checking (negative ptr, overflow, out-of-bounds) (2026-03-05)
- Security review: 13-item checklist passed, zero ELEVATED flags (2026-03-05)
- cargo-audit: 5 transitive advisories documented, none exploitable in our code (2026-03-05)

---

## [M0] -- Design Phase (Complete)

### Added
- Full design phase deliverables (2026-03-05):
  - GENESIS_CHAIN_MASTER_PLAN.md -- definitive blueprint
  - MASTER_DESIGN.md -- full technical design (9 sections, all engineers)
  - RESEARCH_BRIEF.md -- competitive analysis and market research
  - LEGAL_LANDSCAPE.md -- IP and regulatory landscape
  - LEGAL_CLEARANCE_REPORT.md -- name/ticker clearance
  - NAMING_REPORT.md -- final naming decision (Aztibase Network / AZTB)
  - ORCHESTRATION.md -- team protocol and dependency graph
  - aztibase-visual-explainer.html -- interactive visual explainer
- 12 specialist skills defined in .claude/skills/
- Workspace scaffolded: 8 Rust crates in Cargo workspace
- Initial crate structure with module stubs for all domains
