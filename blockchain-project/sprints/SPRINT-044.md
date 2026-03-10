# Sprint 044 — Genesis Hash P2P Enforcement (M9-S5)

**Status:** COMPLETE
**Started:** 2026-03-10
**Completed:** 2026-03-10
**Engineer(s):** p2p-network-engineer, node-engineer, security-engineer

---

## Goal

Embed the genesis hash into all P2P protocol identifiers (gossipsub topics, Kademlia protocol, light-sync protocol). Nodes with different genesis configs will be on completely separate networks — no cross-chain message pollution, no consensus divergence from mismatched genesis.

## Scope

~4 files, focused changes. gossip.rs + discovery.rs + transport.rs + main.rs wiring.

---

## Phase 1: Chain-Scoped Protocol IDs

### Task 1.1 — Parameterize gossipsub topics with genesis hash
**File:** `crates/aztibase-network/src/gossip.rs`
- Replace hardcoded topic strings with function that takes genesis hash prefix
- Topic format: `/aztibase/blocks/1.0.0/{genesis_hex8}`
- `aztibase_topics(genesis_hex: &str)` replaces `aztibase_topics()`
- **Status:** DONE

### Task 1.2 — Parameterize Kademlia protocol ID
**File:** `crates/aztibase-network/src/discovery.rs`
- `kademlia_protocol()`, `kademlia_config_scoped()`, `kademlia_behaviour_scoped()` use `/aztibase/kad/1.0.0/{genesis_hex8}`
- **Status:** DONE

---

## Phase 2: Wire Genesis Hash into Network Layer

### Task 2.1 — Add genesis_hash to TransportConfig
**File:** `crates/aztibase-network/src/transport.rs`
- `genesis_hash` field in TransportConfig
- Wired through to gossipsub and kademlia
- **Status:** DONE

### Task 2.2 — Pass genesis hash from main.rs to transport
**File:** `crates/aztibase-node/src/main.rs`
- Genesis hash passed from genesis config to both full-node and light-node TransportConfig
- **Status:** DONE

---

## Phase 3: Tests

### Task 3.1 — Unit tests
- 6 gossip tests + 4 discovery tests = 10 new tests
- **Status:** DONE

### Task 3.2 — cargo clippy + fmt + test
- 0 clippy warnings, fmt clean, 796 tests total
- **Status:** DONE

---

## Phase 4: Documentation

### Task 4.1 — Doc updates
- BUILD_LOG, STATUS.md, CHANGELOG, sprint plan updated
- **Status:** DONE

---

## Exit Criteria

- [x] All gossipsub topics include genesis hash prefix
- [x] Kademlia protocol ID includes genesis hash prefix
- [x] Nodes with different genesis configs are on separate networks
- [x] Fallback behavior when no genesis configured
- [x] All tests pass
- [x] cargo clippy: 0 warnings, cargo fmt: clean

---

## Retrospective

### What went well
- Clean separation of concerns: gossip.rs handles topic scoping, discovery.rs handles kademlia scoping, transport.rs wires them together
- TOPIC_BASE_NAMES constant keeps topic definitions DRY
- genesis_hex_prefix() utility reusable across both gossipsub and kademlia

### What could improve
- Consider adding integration test for cross-genesis isolation (two nodes with different genesis hashes confirming they cannot communicate)

### Metrics
- **Tests added:** 10 (6 gossip + 4 discovery)
- **Tests total:** 796
- **Clippy warnings:** 0
- **Security findings:** 0 ELEVATED, 0 MEDIUM
