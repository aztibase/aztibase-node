# Sprint 029 — M8 Sprint 4: Gossipsub Hardening & Persistent Peer Discovery

**Goal:** Tighten gossipsub scoring for adversarial conditions, add persistent peer storage, implement Kademlia bootstrap protocol, and add protocol-level message validation.

**Started:** 2026-03-08
**Status:** COMPLETE

---

## Phase 1: Gossipsub Scoring Retune (Tasks 1–4)

Current gossipsub params are Sprint 010 defaults — adequate for local testnet but too permissive for public exposure. Invalid message penalties are weak (-10 weight) and mesh delivery thresholds are low.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Tighten `invalid_message_deliveries_weight` from -10.0 to -50.0, `decay` from 0.3 to 0.1. Severely penalize peers sending malformed data | p2p-network-engineer | DONE |
| 2 | Increase `mesh_message_deliveries_threshold` from 20 to 50 and `mesh_message_deliveries_cap` from 100 to 500. Raise the bar for peers staying in mesh | p2p-network-engineer | DONE |
| 3 | Per-topic weight differentiation: consensus=2.0, blocks=1.5, transactions=1.0, state-sync=0.5, ai-proofs=0.5, validator-announce=1.5. Critical topics penalize harder | p2p-network-engineer | DONE |
| 4 | Lower `graylist_threshold` from -80 to -60, `publish_threshold` from -50 to -30. Faster isolation of misbehaving peers. Tests: verify new thresholds build, scoring params valid (2 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Gossipsub params hardened for adversarial conditions, all existing tests pass with new values ✅

---

## Phase 2: Persistent Peer Store (Tasks 5–8)

Kademlia uses MemoryStore — all discovered peers lost on restart. Nodes waste time re-discovering the same peers every boot. This phase adds persistent peer storage.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `PeerStore` struct: redb-backed `peer_addresses` table mapping `PeerId → Vec<Multiaddr>` with `last_seen: u64`. Methods: `insert`, `get`, `remove`, `list_recent(limit)`, `prune_stale(max_age)` | p2p-network-engineer | DONE |
| 6 | On `PeerConnected`: persist peer address to PeerStore. On `ConnectionClosed`: update last_seen. On `MdnsDiscovered` / `KademliaRoutingUpdated`: insert new addresses | p2p-network-engineer | DONE |
| 7 | On startup: load up to 50 most-recent peers from PeerStore, dial them before boot nodes. Skip banned peers (check PeerReputationStore). Fall back to boot nodes if fewer than 3 stored peers | p2p-network-engineer | DONE |
| 8 | Tests: peer persists across store reopen, stale peers pruned, banned peers skipped on load, max limit respected (4 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Peers persist across restarts, startup dials cached peers first, 4+ new tests pass ✅

---

## Phase 3: Kademlia Bootstrap & Message Validation (Tasks 9–12)

Kademlia bootstrap isn't triggered — DHT never populates beyond directly connected peers. Gossipsub accepts all messages without structural validation.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Trigger `kademlia.bootstrap()` after first peer connects. Re-bootstrap every 5 minutes via tokio interval. Log routing table size at DEBUG level | p2p-network-engineer | DONE |
| 10 | Handle `KademliaEvent::RoutingUpdated`: insert new peer addresses into PeerStore. Handle `KademliaEvent::OutboundQueryProgressed` for bootstrap completion logging | p2p-network-engineer | DONE |
| 11 | `validate_gossip_message(topic, data) → MessageAcceptance`: basic structural validation — reject empty messages, messages > 2MB, check topic-specific minimum sizes (blocks ≥ 64 bytes, transactions ≥ 32 bytes). Return Accept/Reject/Ignore | p2p-network-engineer | DONE |
| 12 | Wire message validation into gossipsub event handler: validate before processing, record Medium offense on Reject. Tests: empty message rejected, oversized rejected, valid accepted (3 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Kademlia DHT bootstraps and maintains routing table, messages structurally validated before processing, 3+ new tests pass ✅

---

## Phase 4: Security Review & Documentation (Tasks 13–15)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: audit new scoring params for over-penalization risk, PeerStore for data leakage, message validation for bypass vectors | security-engineer | DONE |
| 14 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, `cargo test -p aztibase-network` all pass | security-engineer | DONE |
| 15 | Update BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED findings, clippy/fmt/test clean, all docs updated ✅

---

## Dependencies

- Phase 2 builds on Sprint 028's PeerReputationStore (ban check on peer load)
- Phase 3 depends on Phase 2 (PeerStore for Kademlia routing updates)
- Phase 4 depends on all prior phases

## New Dependencies

None — all features use existing libp2p + redb dependencies.
