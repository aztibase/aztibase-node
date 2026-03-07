# Sprint 020 -- WebSocket Gateway, RPC Subscriptions & Deployment

**Sprint Goal:** Add a WebSocket gateway to full nodes so browser light clients can connect, implement event subscriptions for real-time header/finality streaming, add wallet management commands, and create Docker deployment infrastructure. Complete M5.

**Milestone:** M5 -- Light/Browser Nodes + Wallet (COMPLETION)
**Status:** COMPLETE
**Start Date:** 2026-03-07

---

## Rationale

Sprint 019 delivered a browser light client (WASM + JS) that expects a WebSocket connection to a full node. However, the full node RPC server is HTTP-only (axum POST handler). This sprint closes the gap by adding a WebSocket upgrade path, implementing subscription methods for real-time streaming, completing wallet CLI operations, and creating Docker deployment infrastructure. Completing these items satisfies all M5 exit criteria.

---

## Phase 1: WebSocket Gateway on Full Node (4 tasks)

Add WebSocket support to the axum-based RPC server so browser light clients can connect.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Add axum WebSocket upgrade handler at `/ws` route; accept connections, maintain client set via `Arc<RwLock<HashMap<u64, mpsc::Sender>>>`, enforce max 256 concurrent connections | node-engineer | DONE |
| 2 | WebSocket JSON-RPC dispatch: accept JSON-RPC requests over WS text frames, route through existing `dispatch()`, send responses back on same socket | node-engineer | DONE |
| 3 | Light sync over WebSocket: handle `LightSyncMessage` (RequestHeaders/RequestProof) from browser clients, serve headers and proofs from storage (same logic as P2P handler, WS transport) | node-engineer | DONE |
| 4 | Tests: WS upgrade handshake, JSON-RPC request/response over WS, light sync header request over WS, concurrent connection limit enforcement | node-engineer | DONE |

**Exit Criteria:** Browser light client connects to `ws://host:port/ws`, sends JSON-RPC calls, receives light sync responses. 4+ tests.

---

## Phase 2: Event Subscriptions (4 tasks)

Implement JSON-RPC subscription methods so clients receive real-time events without polling.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | `aztb_subscribe` method: topic parameter (`"newHeads"`, `"finality"`), returns subscription ID; WebSocket-only (HTTP callers get clear error) | node-engineer | DONE |
| 6 | `aztb_unsubscribe` method: cancel subscription by ID, clean up broadcast channel slot | node-engineer | DONE |
| 7 | Subscription broadcast infrastructure: `tokio::sync::broadcast` channel per topic, execution pipeline publishes new headers/certs, WS handler forwards to subscribed clients | node-engineer | DONE |
| 8 | Tests: subscribe to newHeads + receive header event, subscribe to finality + receive cert event, unsubscribe stops delivery, subscribe over HTTP returns error | node-engineer | DONE |

**Exit Criteria:** WS clients subscribe to `newHeads`/`finality`, receive push notifications, unsubscribe cleanly. HTTP subscribe returns error. 4+ tests.

---

## Phase 3: Wallet Management & Export (4 tasks)

Complete wallet CLI with list, balance, and portable export/import.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | `aztibase wallet list`: scan keyfile directory (default `{data_dir}/keys/`), display address + account index + encrypted status per keyfile | node-engineer | DONE |
| 10 | `aztibase wallet balance --address <addr> --rpc <url>`: query `aztb_getBalance` + `aztb_getNonce` via JSON-RPC, display formatted output | node-engineer | DONE |
| 11 | `aztibase wallet export --key <keyfile> --format json` + `aztibase wallet import --file <path>`: portable encrypted keyfile JSON export/import with validation | node-engineer | DONE |
| 12 | Tests: wallet list discovers keyfiles, balance query against mock RPC, export/import roundtrip preserves keypair, import rejects tampered file | node-engineer | DONE |

**Exit Criteria:** Users can list wallets, check balance via RPC, export/import encrypted keyfiles. 4+ tests.

---

## Phase 4: Docker Deployment & Security Review (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Dockerfile: multi-stage build (rust builder + debian:bookworm-slim runtime), compile `aztibase` binary, expose 8545 (RPC/WS) + 9000 (P2P), configurable via ENV | node-engineer | DONE |
| 14 | `docker-compose.yml`: 3-node local testnet (validator1-3) with pre-generated genesis + keys, shared network, health checks | node-engineer | DONE |
| 15 | Security review: WS attack surface (connection flooding, slowloris, frame size limits), subscription memory exhaustion (max subs per client, broadcast bounds), wallet export format (no secret leakage) | security-engineer | DONE |
| 16 | Clippy zero warnings, fmt clean, all tests passing, BUILD_LOG.md, STATUS.md, CHANGELOG.md, DECISIONS.md (ADR-009: WebSocket gateway), MEMORY.md updated | documentation-engineer | DONE |

**Exit Criteria:** `docker compose up` starts 3-node testnet. Browser demo connects to Dockerized node via WS. 0 ELEVATED security findings. All docs updated. M5 marked COMPLETE.

---

## Dependencies

- Sprint 019: aztibase-wasm, browser demo, LightClient JS, HD wallet derivation
- Sprint 018: LightSyncCodec, P2P light sync handler, wallet broadcast
- Sprint 017: LightStore, LightSyncProtocol, BIP-39 wallet, Argon2id keyfiles
- Sprint 006: axum JSON-RPC server (RpcServer, dispatch)
- axum WebSocket: built-in (`axum::extract::ws`), no new dependency

## Key Files

- `crates/aztibase-rpc/src/server.rs` -- add WS upgrade, subscriptions, WS dispatch
- `crates/aztibase-node/src/main.rs` -- wire broadcast channels from pipeline to RPC
- `crates/aztibase-node/src/wallet.rs` -- add list, balance, export, import
- `crates/aztibase-wasm/web/light-client.js` -- reference: browser-side WS protocol

## Notes

- Full Verkle IPA/KZG deferred to Sprint 021+ (multi-sprint, new crypto dep). BLAKE3 placeholder works correctly for all proof flows.
- Docker uses Debian slim (not Alpine) because blst (ADR-004) requires glibc.
- WebSocket reuses existing axum router; no new HTTP framework dependency.
- This sprint completes M5. Sprint 021 begins M6 (AI compute market + PoUW).

## Success Metrics

- Browser demo connects to real full node via WS and syncs headers
- Event subscriptions deliver headers within 100ms of creation
- `docker compose up` starts functional 3-validator testnet
- Wallet CLI covers full lifecycle: generate, recover, derive, list, balance, transfer, export, import
- 16+ new tests, 0 regressions, target ~497 total
- 0 ELEVATED, 0 MEDIUM security findings
- M5 milestone marked COMPLETE
