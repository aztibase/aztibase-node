# Sprint 027 — M8 Sprint 2: Docker Testnet Bootstrap & Genesis Tooling

**Goal:** Make `docker compose up` produce a working 3-validator testnet with peer discovery, genesis state, and monitoring — zero manual steps after setup.

**Started:** 2026-03-08
**Status:** COMPLETE

---

## Phase 1: Docker Genesis Setup Script (Tasks 1–4)

Generate the full data directory layout that docker-compose.yml expects.

| # | Task | Status |
|---|------|--------|
| 1 | Create `scripts/setup-docker-testnet.sh` — runs `aztibase genesis --docker`, outputs Docker-ready layout | DONE |
| 2 | Add `write_docker_configs()` to genesis.rs — generates Docker-aware node configs with `/dns4/` boot_nodes and container-internal paths | DONE |
| 3 | Add `--docker` flag to `aztibase genesis` CLI subcommand — outputs Docker-ready layout directly | DONE |
| 4 | Verify directory structure matches docker-compose volume mounts | DONE |

**Exit criteria:** MET — `data/` directory layout matches docker-compose volumes.

---

## Phase 2: RPC Additions + Chain Identity (Tasks 5–8)

Add chain identity RPC methods for testnet verification.

| # | Task | Status |
|---|------|--------|
| 5 | Add `aztb_chainId` RPC method — returns chain_id from RpcState | DONE |
| 6 | Add `aztb_genesisHash` RPC — returns BLAKE3 hash of serialized genesis config | DONE |
| 7 | Store genesis hash in RpcState at startup (computed once from genesis TOML) | DONE |
| 8 | Tests pass (existing 53 RPC tests + 148 node tests) | DONE |

**Exit criteria:** MET — `aztb_chainId` returns `0xa27b`, `aztb_genesisHash` returns deterministic BLAKE3 hash.

---

## Phase 3: Testnet Lifecycle Tooling (Tasks 9–12)

Developer ergonomics for testnet management.

| # | Task | Status |
|---|------|--------|
| 9 | Create `Makefile` with targets: `setup`, `start`, `stop`, `reset`, `logs`, `status`, `build`, `test`, `clean` | DONE |
| 10 | Create `scripts/reset-testnet.sh` — wipes node DBs but preserves keys + genesis | DONE |
| 11 | Update docker-compose.yml — simplified to read from TOML config (no CLI overrides needed) | DONE |
| 12 | Update Dockerfile: add `curl` for healthchecks | DONE |

**Exit criteria:** MET — `make setup && make start` boots testnet, `make reset` cleans state, `make stop` tears down.

---

## Phase 4: Security Review + Docs (Tasks 13–16)

| # | Task | Status |
|---|------|--------|
| 13 | Security review: genesis key storage (plaintext JSON), Docker volume permissions, boot_node DNS trust | DONE |
| 14 | clippy 0 warnings + fmt clean | DONE |
| 15 | 201 tests pass (148 node + 53 RPC), 0 failures | DONE |
| 16 | Doc updates: BUILD_LOG, STATUS, CHANGELOG, MEMORY | DONE |

**Exit criteria:** MET — 0 clippy warnings, fmt clean, all tests pass, docs updated.

---

## Security Notes

- **Genesis keys stored as plaintext JSON** in `data/node{N}/keys/` — acceptable for testnet, not for mainnet. Mainnet keys must use encrypted keyfiles (Argon2id + ChaCha20-Poly1305, already implemented in wallet module).
- **Boot nodes use Docker DNS** (`/dns4/validator{N}/tcp/30333`) — trust is implicit within the Docker bridge network. Production deployments should use explicit peer IDs in multiaddrs.
- **Faucet already gated** by chain_id == 0xA27B (Sprint 026) — cannot be exploited on mainnet.
- **No new security findings** (0 ELEVATED, 0 MEDIUM, 0 LOW).

---

## Retrospective

### What went well
- `write_docker_configs()` cleanly separates Docker layout from local-testnet layout
- `--docker` flag on `aztibase genesis` makes one-command setup possible
- Makefile provides developer-friendly workflow (`make setup && make start`)
- All 201 tests pass after changes

### What could improve
- Windows disk space continues to be a constraint for full workspace builds — cargo clean required between test runs
- Should add integration test that runs `aztibase genesis --docker` and validates output structure

### Metrics
- 201 tests (148 node + 53 RPC), 0 failures
- 0 clippy warnings, fmt clean
- Sprint duration: ~1 session
