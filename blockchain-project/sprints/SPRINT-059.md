# Sprint 059 — Friends Testnet: Genesis Ceremony & Multi-Party Setup

**Status:** DONE
**Started:** 2026-03-12
**Completed:** 2026-03-12
**Milestone:** M9 — Mainnet Prep

## Goal

Enable friends to each generate their own validator keys and join a shared private testnet, without sharing private keys or modifying code.

## Phases

### Phase 1: Wallet --validator Flag ✅
- Added `generate_validator_key()` to wallet.rs — produces Ed25519 + BLS12-381 keypair
- Added `--validator` flag to `aztibase wallet generate`
- 1 test: roundtrip generate + load + verify BLS present

### Phase 2: Genesis Ceremony CLI ✅
- Refactored `Command::Genesis` to `GenesisAction` subcommand enum
- `genesis init` — creates empty scaffold (chain_id, timestamp, no validators)
- `genesis add-validator` — from keyfile OR from public info (address + public_key + bls_public_key)
- `genesis add-account` — adds pre-funded account
- `genesis validate` — runs validation, prints errors
- `genesis show` — prints summary with genesis hash
- Added `save_genesis()` helper
- 8 tests: scaffold, keyfile add, public-info add, duplicate rejection, account add, overlap rejection, show, full ceremony flow
- **Breaking**: `aztibase genesis --validators 4` → `aztibase genesis generate --validators 4`

### Phase 3: Setup Script Custom Network ✅
- Added `--boot-nodes <comma-separated-multiaddrs>` flag
- Added `--genesis <path>` flag (copies to node config dir)
- Added `custom` network option (requires --genesis, warns if no --boot-nodes)
- Updated systemd ExecStart to inject custom genesis and boot nodes

### Phase 4: Friends Testnet Guide ✅
- Created `docs/FRIENDS_TESTNET_GUIDE.md`
- Covers: key generation, coordinator workflow, genesis ceremony, seed node setup, verification, troubleshooting, security reminders

### Phase 5: Coordinator Quick-Start Script ✅
- Created `scripts/setup-friends-testnet.sh`
- Interactive prompt-driven flow: init → add validators → add accounts → validate → show → print instructions

### Phase 6: Tests & Doc Sync ✅
- All tests pass across 9 crates
- Clippy: 0 warnings
- Fmt: clean
- BUILD_LOG, CHANGELOG, STATUS updated

## Also Added
- `--boot-node` CLI flag on main node binary (was missing — boot nodes could only come from config TOML)
- Fixed 3 test sites in main.rs that manually construct Cli struct (missing new `boot_node` field)

## Files Changed
- `crates/aztibase-node/src/wallet.rs`
- `crates/aztibase-node/src/main.rs`
- `crates/aztibase-node/src/genesis.rs`
- `deploy/setup-validator.sh`
- `docs/FRIENDS_TESTNET_GUIDE.md` (new)
- `scripts/setup-friends-testnet.sh` (new)
- `blockchain-project/BUILD_LOG.md`
- `blockchain-project/STATUS.md`
- `CHANGELOG.md`
- `blockchain-project/sprints/SPRINT-059.md` (this file)
