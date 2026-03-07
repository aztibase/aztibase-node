# Sprint 018 -- Light Node P2P Integration & Wallet Transfers

**Sprint Goal:** Wire the light node sync protocol into the live P2P network, enable real header sync between full and light nodes, and implement end-to-end wallet transfer signing and broadcast.

**Milestone:** M5 -- Light/Browser Nodes + Wallet
**Status:** COMPLETE
**Started:** 2026-03-07

---

## Rationale

Sprint 017 built the light node storage layer (LightStore), the sync protocol state machine (LightSyncProtocol), and wallet hardening (BIP-39, Argon2id keyfiles). But these components are not yet wired into the live P2P network -- the sync protocol generates messages but doesn't send them over libp2p, and the wallet can sign transactions but can't broadcast them to the mempool. This sprint closes both gaps.

---

## Phase 1: Light Sync P2P Handler (4 tasks)

Wire LightSyncProtocol into the libp2p request-response framework so light nodes can actually sync headers from full nodes.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Define `/aztibase/light-sync/1` request-response protocol using libp2p `request_response::Behaviour` with `LightSyncMessage` as codec | p2p-network-engineer | DONE |
| 2 | Implement `LightSyncCodec` (bincode encode/decode over async read/write, max 1MB frame) | p2p-network-engineer | DONE |
| 3 | Add light sync handler to `AztibaseNetwork`: on incoming `RequestHeaders` → read from storage, reply with `ResponseHeaders`; on incoming `RequestProof` → reply with `ResponseProof` | p2p-network-engineer | DONE |
| 4 | Tests: codec roundtrip, request-response mock (full node serves headers to light node), oversized frame rejection | p2p-network-engineer | DONE |

**Exit Criteria:** Light sync messages flow over libp2p request-response. Full nodes serve headers to light nodes. 3+ tests.

---

## Phase 2: Light Node Sync Loop (4 tasks)

Build the light node's main sync loop that discovers peers, requests headers, verifies chains, and persists to LightStore.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Implement `run_light_sync()` async loop: connect to peers, call `LightSyncProtocol::next_request()`, send via P2P, apply responses, persist to LightStore | node-engineer | DONE |
| 6 | Add peer scoring for light sync: track response latency and failure rate per peer, prefer fast responders, disconnect peers that serve invalid headers | node-engineer | DONE |
| 7 | Wire `run_light_sync()` into `run_light_node()` in main.rs (replace placeholder), pass `LightStore` + `AztibaseNetwork` handles | node-engineer | DONE |
| 8 | Tests: sync loop advances round on valid response, rejects invalid chain (gap, bad quorum), retries on peer failure | node-engineer | DONE |

**Exit Criteria:** Light node syncs headers from full node peers over P2P, verifies chain, persists to LightStore. 3+ tests.

---

## Phase 3: Wallet Transaction Broadcast (4 tasks)

Enable the CLI wallet to sign and broadcast transactions to the network, completing the user-facing transfer flow.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Implement `sign_transfer()` in wallet.rs: build `TxKind::Transfer`, wrap in `SignedTx`, sign with loaded keypair | node-engineer | DONE |
| 10 | Add `broadcast_transaction()`: connect to a full node RPC endpoint, submit signed tx via `aztb_sendTransaction` | node-engineer | DONE |
| 11 | Wire wallet transfer CLI: `aztibase wallet transfer --to <addr> --amount <u64> --key <keyfile> [--passphrase] --rpc <url>` builds, signs, broadcasts, prints tx hash | node-engineer | DONE |
| 12 | Tests: sign_transfer produces valid SignedTx, broadcast mock (assert correct RPC call format), encrypted keyfile transfer roundtrip | node-engineer | DONE |

**Exit Criteria:** User can sign and broadcast a transfer from CLI. 3+ tests.

---

## Phase 4: Security Review & Docs (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: P2P light sync attack surfaces (eclipse, slow-drip, invalid header flooding), wallet signing (key exposure, nonce reuse) | security-engineer | DONE |
| 14 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean | security-engineer | DONE |
| 15 | `cargo test --workspace` all passing, update test count | security-engineer | DONE |
| 16 | Update BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan, MEMORY.md | documentation-engineer | DONE |

**Exit Criteria:** 0 ELEVATED security findings. All docs updated. All tests pass. Clippy/fmt clean.

---

## Success Metrics

- Light node syncs headers from full node over actual libp2p P2P
- Header chain verification runs on every sync batch
- CLI wallet signs and broadcasts transfers end-to-end
- 12+ new tests, 0 regressions
- 0 ELEVATED, 0 MEDIUM security findings

---

## Dependencies

- Sprint 017: LightStore, LightSyncProtocol, LightSyncMessage, verify_header_chain
- Sprint 017: BIP-39 wallet, encrypted keyfiles
- Sprint 011: SignedTx, verify_and_route
- Sprint 006: JSON-RPC server (aztb_sendTransaction)
- aztibase-network: AztibaseNetwork, libp2p request-response

---

## Notes

- Browser WASM node (wasm32 compilation, IndexedDB, WebRTC-only transport) is deferred to Sprint 019 -- requires the P2P light sync to be working first
- Mobile SDK is post-M5
- HD wallet derivation paths (multiple accounts) deferred to future sprint
