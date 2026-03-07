# Sprint 019 -- Browser WASM Light Node & HD Wallet Derivation

**Sprint Goal:** Compile the light client verification logic to WASM for browser use, build a JS transport bridge and demo page, and add HD wallet derivation for multi-account support.

**Milestone:** M5 -- Light/Browser Nodes + Wallet
**Status:** COMPLETE
**Started:** 2026-03-07

---

## Rationale

Sprints 017-018 delivered the full light node backend: LightStore (redb), LightSyncProtocol state machine, verify_header_chain, P2P codec, peer scoring, and wallet transfer broadcast. The next M5 deliverable is making this accessible from a browser. A WASM-compiled light client lets any web page verify Aztibase state proofs without trusting an RPC provider -- a core requirement of the server-independence design constraint. HD derivation completes the wallet story by enabling multiple accounts from a single mnemonic.

---

## Phase 1: WASM Light Client Core (4 tasks)

Create an `aztibase-wasm` crate that compiles light client verification logic to `wasm32-unknown-unknown`.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Scaffold `aztibase-wasm` crate: `wasm-bindgen`, `wasm-pack` build, `wasm32-unknown-unknown` target, workspace member | blockchain-architect | DONE |
| 2 | Expose `verify_header_chain()` via wasm-bindgen: accept JSON-serialized headers + cert, return bool | p2p-network-engineer | DONE |
| 3 | Expose `verify_light_client_proof()` via wasm-bindgen: accept JSON-serialized proof + leaf, return bool | node-engineer | DONE |
| 4 | Tests: `wasm-pack test --node` for header chain verify and proof verify (roundtrip with known-good and known-bad inputs) | node-engineer | DONE |

**Exit Criteria:** `wasm-pack build` produces a working `.wasm` + JS glue. Header and proof verification callable from JS. 3+ tests.

---

## Phase 2: Browser Transport Bridge (4 tasks)

Build a JS/TS layer that connects the WASM light client to a full node via WebSocket, replacing libp2p (which doesn't run in browsers).

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | WebSocket transport adapter: JS class that frames `LightSyncMessage` over WebSocket (bincode or JSON, length-prefixed) | p2p-network-engineer | DONE |
| 6 | `LightClient` JS API: `connect(wsUrl)`, `sync()`, `getBalance(address)`, `verifyProof(key)` — wraps WASM verification | node-engineer | DONE |
| 7 | Header cache in IndexedDB: persist synced headers client-side, load on reconnect to avoid full re-sync | node-engineer | DONE |
| 8 | Demo HTML page: minimal browser light client — connect to full node WS, sync headers, display latest round + verify a proof | node-engineer | DONE |

**Exit Criteria:** Browser page connects to a full node, syncs headers, verifies at least one proof client-side. Headers persist across page reloads via IndexedDB.

---

## Phase 3: HD Wallet Derivation (4 tasks)

Add BIP-32-style hierarchical deterministic derivation so a single mnemonic can produce multiple accounts.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | `derive_child(parent_secret, index)` using BLAKE3 domain separation: `derive_key("aztibase child/{index}", parent)` — Aztibase-specific, not HMAC-SHA512 BIP-32 | node-engineer | DONE |
| 10 | `derive_account(mnemonic, account_index)` for path `m/44'/aztb'/N'/0/0` — derives keypair for account N | node-engineer | DONE |
| 11 | `aztibase wallet derive --index <N>` CLI command: derive child keypair by account index, display address, optionally save encrypted keyfile | node-engineer | DONE |
| 12 | Tests: derive index 0 matches existing single-account derivation, derive indices 0-9 produce unique keys, derive is deterministic across calls | node-engineer | DONE |

**Exit Criteria:** Multi-account derivation from single mnemonic. Index 0 backward-compatible with Sprint 017 derivation. 3+ tests.

---

## Phase 4: Security Review & Docs (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Security review: WASM attack surface (memory exposure in linear memory, JS interop data leaks, supply chain of wasm-bindgen), WebSocket transport (no auth, message tampering) | security-engineer | DONE |
| 14 | Security review: HD derivation (key material zeroize, child key independence, no parent key recovery from child) | security-engineer | DONE |
| 15 | `cargo clippy --workspace` zero warnings, `cargo fmt --check` clean, `cargo test --workspace` all passing, update test count | security-engineer | DONE |
| 16 | Update BUILD_LOG.md, STATUS.md, CHANGELOG.md, sprint plan, MEMORY.md | documentation-engineer | DONE |

**Exit Criteria:** 0 ELEVATED security findings. All docs updated. All tests pass. Clippy/fmt clean.

---

## Success Metrics

- `wasm-pack build` succeeds, `.wasm` under 2MB
- Browser demo page verifies headers and proofs without any server-side trust
- HD derivation produces deterministic, unique keys per index
- Index 0 backward-compatible with existing wallet derivation
- 25 new tests (21 WASM + 4 HD wallet), 0 regressions, 481 total
- 0 ELEVATED, 0 MEDIUM security findings (3 LOW)

---

## Dependencies

- Sprint 018: LightSyncCodec, P2P sync loop, wallet transfer broadcast
- Sprint 017: LightSyncProtocol, verify_header_chain, LightStore, BIP-39 wallet, Argon2id keyfiles
- Sprint 015: verify_light_client_proof, LightClientProof
- Sprint 014: StateCommitment trait, MerkleCommitment, VerkleCommitment
- aztibase-core: Hash, Address, Keypair, crypto primitives

---

## Notes

- libp2p does not work in browsers; the WebSocket bridge is a deliberate simplification for M5. WebRTC-based browser P2P (peer-to-peer without relay) is a future enhancement.
- BLAKE3 domain-separated HD derivation is Aztibase-specific and NOT BIP-32 compatible. This is intentional -- BIP-32 uses HMAC-SHA512 which would add a dependency and diverge from our BLAKE3-everywhere philosophy.
- Full Verkle IPA/KZG and Docker deployment are deferred to Sprint 020+.
- The `aztibase-wasm` crate does NOT include storage, networking, or consensus -- verification only. This avoids pulling wasmtime/revm/libp2p into the WASM build.
- `derive_account(phrase, 0)` produces the exact same keypair as `keypair_from_mnemonic()` from Sprint 017, ensuring backward compatibility.
