# Sprint 017 — Wallet Hardening, Light Node Storage & Header Sync

**Sprint Number:** 017
**Start Date:** 2026-03-07
**End Date:** 2026-03-07
**Status:** COMPLETE
**Milestone:** M5 — Light/Browser Nodes + Wallet

---

## Objective

Begin M5 by hardening the wallet with BIP-39 mnemonics and Argon2id-encrypted keyfiles, then build the light node storage schema (redb) and header sync protocol. These are the foundational pieces that all subsequent M5 work (browser nodes, mobile SDK, full Verkle) depends on.

**Scope rationale:** Full Verkle IPA/KZG is a multi-sprint effort; browser WASM + mobile SDK are downstream. This sprint delivers the two highest-value, lowest-dependency M5 foundations: secure wallet keys and light client infrastructure.

---

## Dependencies (already built)

- Light client proofs: `crates/aztibase-execution/src/light_client.rs` (Sprint 015)
- Verkle tree placeholder: `crates/aztibase-execution/src/verkle.rs` (Sprint 014)
- BLS finality certificates: `crates/aztibase-core/src/bls.rs` (Sprint 005/013)
- CLI wallet: `crates/aztibase-node/src/wallet.rs` (Sprint 012)
- State snapshots: `crates/aztibase-execution/src/snapshot.rs` (Sprint 009/015)
- redb storage: `crates/aztibase-storage/` (Sprint 002+)

---

## Phases & Tasks

### Phase 1: Wallet Hardening — BIP-39 + Argon2id (Tasks 1-4)

Upgrade the CLI wallet from raw key files to mnemonic-based key derivation with encrypted storage.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add `bip39` + `argon2` crates to `aztibase-node/Cargo.toml`; verify pure-Rust (ADR-001 compliant) | node-engineer | DONE |
| 2 | `generate_mnemonic()`: 24-word BIP-39 seed, derive Ed25519 keypair via HMAC-SHA512 + BIP-44 path `m/44'/aztb'/0'/0/0`, display mnemonic to user | node-engineer | DONE |
| 3 | `recover_from_mnemonic()`: accept 12/24-word mnemonic string, re-derive same keypair deterministically, verify round-trip | node-engineer | DONE |
| 4 | `encrypt_keyfile()` / `decrypt_keyfile()`: Argon2id (256MB memory, 3 iterations, 32-byte salt) wraps secret key bytes; `EncryptedKeyFile` JSON format with salt, params, ciphertext | node-engineer | DONE |

**Exit Criteria:**
- `aztibase wallet generate --mnemonic` produces 24-word seed + encrypted keyfile
- `aztibase wallet recover` restores identical keypair from mnemonic
- Encrypted keyfile cannot be read without passphrase
- 4+ tests: mnemonic round-trip, encryption round-trip, wrong passphrase rejection, 12-word compat

---

### Phase 2: Light Node Storage Schema (Tasks 5-9)

Build the redb storage layer for light nodes: headers, finality certs, Verkle proof cache, and wallet state.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `LightStore` struct in `aztibase-storage/src/light.rs`: opens dedicated redb instance, defines 5 tables (headers, finality_certs, proof_cache, wallet_state, peer_cache) | node-engineer | DONE |
| 6 | `store_header()` / `get_header()` / `latest_header()`: GenesisBlockHeader by round number (u64 key), with batch insert for sync | node-engineer | DONE |
| 7 | `store_finality_cert()` / `get_finality_cert()` / `latest_finality_cert()`: FinalityCertificate by anchor round | consensus-engineer | DONE |
| 8 | `cache_proof()` / `get_cached_proof()` / `evict_stale_proofs(max_age_rounds)`: Verkle proof cache with TTL-based eviction (>1000 rounds old) | node-engineer | DONE |
| 9 | `store_wallet_state()` / `get_wallet_state()`: local wallet state (balance, nonce, pending tx hashes) keyed by address | node-engineer | DONE |

**Exit Criteria:**
- LightStore opens/closes cleanly, tables created on first use
- CRUD operations for all 5 tables with serialization round-trips
- Proof cache eviction removes entries older than threshold
- 5+ tests: header CRUD, cert CRUD, proof cache + eviction, wallet state, batch insert

---

### Phase 3: Light Node Header Sync Protocol (Tasks 10-14)

Define the sync protocol types and implement header sync logic for light nodes to follow the chain.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 10 | `LightSyncMessage` enum in `aztibase-network`: `RequestHeaders { from_round, count }`, `ResponseHeaders { headers, finality_certs }`, `RequestProof { state_key, at_round }`, `ResponseProof { proof }` | p2p-network-engineer | DONE |
| 11 | `LightSyncProtocol` struct: manages header sync state (last_synced_round, target_round), validates incoming headers against finality certs (BLS aggregate verify) | consensus-engineer | DONE |
| 12 | `sync_headers()`: request missing headers from peers in batches (max 100 per request), validate each batch's finality cert, store in LightStore | node-engineer | DONE |
| 13 | `verify_header_chain()`: given a batch of headers + finality cert, verify: (a) cert signs the anchor hash, (b) quorum stake >= 2/3, (c) round numbers are sequential | consensus-engineer | DONE |
| 14 | `--light` CLI flag on `aztibase` node: starts in light mode (LightStore instead of full StateManager, no execution, no vertex proposal, header sync only) | node-engineer | DONE |

**Exit Criteria:**
- LightSyncMessage serializes/deserializes via bincode
- sync_headers fetches and validates a batch of headers from a mock peer
- verify_header_chain rejects invalid certs, wrong quorum, gaps
- --light flag starts node in light mode (no panic, stores headers)
- 5+ tests: message serde, valid sync, invalid cert rejection, quorum check, gap detection

---

### Phase 4: Security Review (Tasks 15-16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 15 | Security review of Phase 1-3: key derivation path correctness, Argon2id parameter adequacy, mnemonic handling (zeroize after use), light sync trust assumptions, header validation completeness | security-engineer | DONE |
| 16 | `cargo clippy` zero warnings, `cargo fmt --check` clean, `cargo test` all pass, update BUILD_LOG + STATUS + CHANGELOG | project-lead | DONE |

**Exit Criteria:**
- 0 ELEVATED security flags
- All MEDIUM flags documented with mitigation plan
- clippy/fmt/test clean
- All docs updated

---

## New Dependencies (to add)

| Crate | Version | Purpose | Pure Rust? |
|-------|---------|---------|------------|
| `bip39` | `2.x` | BIP-39 mnemonic generation/recovery | Yes |
| `argon2` | `0.5` | Argon2id password-based key encryption | Yes (RustCrypto) |
| `chacha20poly1305` | `0.10` | AEAD cipher for keyfile encryption (pairs with Argon2id) | Yes (RustCrypto) |
| `zeroize` | `1.x` | Secure memory zeroing for secrets | Yes |

All pure Rust — ADR-001 compliant.

---

## Files to Create/Modify

### New Files
- `crates/aztibase-storage/src/light.rs` — LightStore (redb schema + CRUD)
- `crates/aztibase-network/src/light_sync.rs` — LightSyncMessage, LightSyncProtocol

### Modified Files
- `crates/aztibase-node/src/wallet.rs` — BIP-39 mnemonic + Argon2id encryption
- `crates/aztibase-node/Cargo.toml` — bip39, argon2, chacha20poly1305, zeroize deps
- `crates/aztibase-storage/src/lib.rs` — light module + exports
- `crates/aztibase-storage/Cargo.toml` — (if new deps needed)
- `crates/aztibase-network/src/lib.rs` — light_sync module + exports
- `crates/aztibase-node/src/main.rs` — --light CLI flag
- `crates/aztibase-node/src/config.rs` — LightNodeConfig (optional fields)

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| BIP-44 path derivation for non-standard curve | LOW | Use HMAC-SHA512 seed → Ed25519; document deviation from BIP-32 |
| Argon2id 256MB param too slow on low-end devices | LOW | Configurable params; 256MB is NIST minimum for high-security |
| Light sync header gap after long offline period | MEDIUM | Batch sync in chunks of 100; checkpoint sync for >10k gap (future) |
| Trust assumption: light node trusts BLS quorum | LOW | By design — BLS aggregate is the trust anchor |

---

## Success Metrics

- Total tests: 425 + 21 new = 446
- New crate deps: 4 (all pure Rust)
- New files: 2 (light.rs, light_sync.rs)
- Modified files: 8 (wallet.rs, main.rs, 2x Cargo.toml, 2x lib.rs, workspace Cargo.toml, network Cargo.toml)
- M5 progress: Phase 1 (wallet hardening) + Phase 1B (light node foundation)

---

## Security Findings (Phase 4)

| ID | Severity | Description | Status |
|----|----------|-------------|--------|
| SEC-WALLET-001 | LOW | Mnemonic displayed to stdout; user must secure it; seed bytes zeroized after derivation | DOCUMENTED |
| SEC-LIGHT-001 | LOW | Light node trusts BLS quorum — by design, BLS aggregate is the trust anchor | DOCUMENTED |
| SEC-SYNC-003 | LOW | Header sync from single peer; future: multi-peer cross-validation for Byzantine resistance | DOCUMENTED |

**Summary:** 0 ELEVATED, 0 MEDIUM, 3 LOW. All LOW findings are by-design or deferred to future sprints.

---

## Retrospective

### What went well
- BIP-39 + Argon2id integration was clean; all pure-Rust crates worked as expected
- BLAKE3 domain-separated key derivation is simpler and more Aztibase-idiomatic than HMAC-SHA512 BIP-32
- redb u64-keyed tables for headers/certs enable efficient range queries by round number
- LightSyncProtocol state machine is well-isolated and easily testable
- 21 new tests with zero flaky behavior

### What could improve
- bip39 v2 API differs significantly from v1 docs; caused initial compile errors (from_entropy vs generate)
- argon2::Error doesn't impl std::Error, requiring manual anyhow wrapping
- Adding CLI flags required updating all test Cli struct literals (3 instances) — consider builder pattern for test helpers

### Decisions made
- Used BLAKE3 derive_key instead of HMAC-SHA512 for BIP-44-like derivation (Aztibase-specific, non-standard)
- Light node gets separate redb instance (light.redb) rather than sharing full node's database
- u64 keys for header/cert tables vs &[u8] keys for proof/wallet tables (different access patterns)
- LightSyncMessage uses version field (LIGHT_SYNC_VERSION=1) for future protocol upgrades
