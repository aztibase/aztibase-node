# Changelog -- Aztibase Network

All notable changes to this project are documented here, organized by milestone.
Format follows [Keep a Changelog](https://keepachangelog.com/).

---

## [M6] -- AI Compute Market + PoUW (IN PROGRESS)

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

### Fixed
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
