# Sprint 028 — M8 Sprint 3: Networking Hardening (Phase 1)

**Goal:** Harden the P2P networking layer for public testnet resilience — persistent peer reputation, inbound connection filtering, AutoNAT detection, and subnet-aware peer diversity.

**Started:** 2026-03-08
**Status:** COMPLETE

---

## Phase 1: Peer Reputation & Persistent Ban List (Tasks 1–4)

Gossipsub peer scoring exists (Sprint 010) but resets on restart. Attackers get infinite retries. This phase adds persistent reputation tracking and a ban list that survives node restarts.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `PeerReputation` struct: `peer_id`, `score: f64`, `last_seen: u64`, `banned_until: Option<u64>`, `offenses: u16` — stored in redb table `peer_reputation` | p2p-network-engineer | DONE |
| 2 | `PeerReputationStore`: `record_offense(peer, severity)` decays score, `is_banned(peer)` checks ban expiry, `decay_scores(now)` applies time-based recovery, `prune_stale(cutoff)` evicts unseen peers | p2p-network-engineer | DONE |
| 3 | Wire `PeerReputationStore` into `Libp2pTransport`: on `PeerConnected` check ban list → reject if banned, on gossipsub `InvalidMessage` → `record_offense(peer, HIGH)`, on connection timeout → `record_offense(peer, LOW)` | p2p-network-engineer | DONE |
| 4 | Ban thresholds: score < -100 → ban 1 hour, score < -200 → ban 24 hours, score < -500 → ban 7 days. Configurable via `TransportConfig`. Tests: banned peer rejected on connect, offense accumulation, score decay over time, ban expiry (4 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Peer reputation persists across restarts, banned peers rejected on reconnect, 4+ new tests pass ✅

---

## Phase 2: Inbound Connection Filtering (Tasks 5–8)

Currently any IP can open up to 50 connections with no per-IP tracking. This phase adds per-IP limits, connection rate limiting, and subnet-level awareness.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `ConnectionFilter` struct: `HashMap<IpAddr, ConnectionInfo>` tracking active count + last connect timestamp per IP. `try_accept(ip) → bool` enforces `MAX_CONNS_PER_IP` (default 3) and `MIN_CONNECT_INTERVAL_MS` (default 1000ms) | p2p-network-engineer | DONE |
| 6 | `SubnetTracker`: extract `/16` subnet from peer IP, enforce `MAX_PEERS_PER_SUBNET` (default 5). Prevents a single data center or ISP from dominating the peer table | p2p-network-engineer | DONE |
| 7 | Wire `ConnectionFilter` + `SubnetTracker` into swarm event loop: on `IncomingConnection` check both filters before accepting. Log rejected connections at WARN level with reason (ip-limit / rate-limit / subnet-limit) | p2p-network-engineer | DONE |
| 8 | Tests: per-IP limit enforcement, rate limit rejection, subnet cap enforcement, legitimate peer accepted after cooldown (4 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Per-IP and per-subnet connection limits enforced, rate limiting prevents rapid reconnection, 4+ new tests pass ✅

---

## Phase 3: AutoNAT Detection & Relay Client (Tasks 9–13)

30-50% of nodes on a public testnet will be behind NAT. Without AutoNAT detection and relay fallback, these nodes cannot participate as full peers. This phase adds NAT awareness and relay-based connectivity.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Add `libp2p::autonat` to `AztibaseBehaviour` composite. Configure with 3 boot nodes as AutoNAT servers, 30s probe interval, 2 min confidence threshold | p2p-network-engineer | DONE |
| 10 | `NatStatus` enum (`Public`, `Private`, `Unknown`) tracked in `Libp2pTransport`. Update on `autonat::Event::StatusChanged`. Log NAT status at INFO level on change | p2p-network-engineer | DONE |
| 11 | Add `libp2p::relay::client` to `AztibaseBehaviour`. When `NatStatus::Private` detected, dial boot nodes as relay servers and listen on relay circuit addresses | p2p-network-engineer | DONE |
| 12 | `TransportConfig` additions: `enable_autonat: bool` (default true), `relay_servers: Vec<Multiaddr>` (defaults to boot_nodes), `autonat_probe_interval_secs: u64` (default 30) | p2p-network-engineer | DONE |
| 13 | Tests: AutoNAT behaviour initializes, NatStatus transitions, relay client config validates, TransportConfig new fields parse correctly (4 tests) | p2p-network-engineer | DONE |

**Exit criteria:** Nodes detect their own NAT status, private nodes connect through relay servers, 4+ new tests pass ✅

---

## Phase 4: Security Review & Documentation (Tasks 14–16)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 14 | Security review as `/security-engineer`: audit new PeerReputationStore for DoS via score manipulation, ConnectionFilter bypass vectors, relay trust assumptions. Flag any ELEVATED/MEDIUM findings | security-engineer | DONE |
| 15 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, `cargo test --workspace` all pass. Run `cargo deny check` for new dependencies | security-engineer | DONE |
| 16 | Update BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan status. ADR if any non-obvious design choices were made (e.g., ban duration formula, subnet bucket size) | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED findings unresolved, clippy/fmt/test clean, all docs updated ✅

---

## Dependencies

- Phase 2 depends on Phase 1 (ConnectionFilter references PeerReputationStore for ban checks)
- Phase 3 is independent of Phase 1-2 (AutoNAT is a separate libp2p behaviour)
- Phase 4 depends on all prior phases

## New Dependencies

| Crate | Version | Pure Rust | Purpose |
|-------|---------|-----------|---------|
| libp2p autonat | via libp2p 0.54 feature | YES | NAT type detection |
| libp2p relay (client) | via libp2p 0.54 feature | YES | Relay circuit connectivity for NAT'd nodes |

No new external crates — `autonat` and `relay` are libp2p sub-protocols enabled via Cargo features on the existing `libp2p` dependency.

## Scope Notes

This is the **first of 3-4 networking hardening sprints**. Scope deliberately limited to:
- Peer reputation persistence (not full reputation propagation between peers)
- Basic subnet diversity (not AS-level or geographic diversity)
- AutoNAT detection + relay client (not relay server — boot nodes serve as relays)
- No gossipsub parameter retuning (Sprint 029)
- No WebRTC swarm integration (Sprint 030+)
- No hole punching / DCUtR (Sprint 030+)
