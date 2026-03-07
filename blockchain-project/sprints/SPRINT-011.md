# Sprint 011 — Transaction Signatures + Nonce Enforcement + Fee Market

**Status:** COMPLETE
**Start Date:** 2026-03-07
**Completion Date:** 2026-03-07
**Goal:** Close the three biggest security gaps: unsigned transactions (SEC-AA-002), missing nonce enforcement at submission (SEC-AA-003), and absent fee mechanism (SEC-AA-001). After this sprint, every transaction on the network carries an Ed25519 signature, replays are rejected, and gas fees are deducted from sender balances.

---

## Phase 1: Signed Transaction Envelope (Tasks 1-4)

### Task 1: SignedTx type + signing helper — DONE
- [x] Created `SignedTx` in `aztibase-execution/src/tx.rs`: `{ payload: Vec<u8>, public_key: [u8; 32], signature: [u8; 64] }`
- [x] `SignedTx::new(payload, keypair)` — signs the raw TxKind bytes with Ed25519
- [x] `SignedTx::verify(&self) -> bool` — verifies signature against public_key
- [x] `SignedTx::sender_address(&self) -> Address` — BLAKE3(public_key) → 32-byte address
- [x] Wire format: `[0xAA][payload_len: u32 LE][payload][pubkey: 32][sig: 64]`
- [x] Tests: sign_verify_roundtrip, tampered_payload_rejected, sender_address_deterministic (3 tests)

### Task 2: Envelope encode/decode — DONE
- [x] `SignedTx::encode()` serializes wire format
- [x] `SignedTx::decode(data)` deserializes + validates size limits
- [x] Maximum signed TX size: 1 MiB (MAX_ENVELOPE_SIZE)
- [x] Tests: encode_decode_roundtrip, oversized_rejected, truncated_rejected, wrong_magic_rejected (4 tests)

### Task 3: Pipeline integration — verify before routing — DONE
- [x] Pipeline calls `verify_and_route_batch()` which decodes + verifies + routes
- [x] Signature failure → tx rejected with `InvalidSignature` status (not executed)
- [x] Sender address extracted from pubkey, cross-checked against TxKind.from field
- [x] `verify_and_route()` / `verify_and_route_batch()` public API
- [x] Tests: pipeline_rejects_unsigned_tx, pipeline_rejects_wrong_signer, verify_and_route_valid_transfer, verify_and_route_bad_signature, verify_and_route_sender_mismatch, verify_and_route_batch_mixed (6 tests)

### Task 4: RPC submission accepts signed envelopes — DONE
- [x] `aztb_sendRawTransaction` accepts hex-encoded SignedTx envelope (pre-existing)
- [x] Server forwards to mempool channel for insertion
- [x] Tests: send_transaction_valid, send_transaction_malformed (existing tests cover)

**Phase 1 Exit Criteria:**
- [x] Every transaction entering the pipeline is signature-verified
- [x] Sender address is derived from public key, not trusted from payload
- [x] All existing tests adapted to use signed envelopes
- [x] 13 new tests

---

## Phase 2: Nonce Enforcement + Address Derivation (Tasks 5-8)

### Task 5: Address derivation from public key — DONE
- [x] `address_from_pubkey(pubkey: &[u8; 32]) -> [u8; 32]` in aztibase-core
- [x] Address = BLAKE3(pubkey) full 32-byte hash
- [x] Used by `SignedTx::sender_address()` and `verify_and_route()`
- [x] Tests: sender_address_deterministic (in tx.rs tests)

### Task 6: Mempool nonce validation — DONE
- [x] `Mempool::insert_checked()` validates nonce on insertion
- [x] Rejects if nonce < current account nonce (stale)
- [x] Rejects if nonce > current + MAX_NONCE_GAP (16)
- [x] Accepts next expected nonce and future within gap
- [x] `decode_sender_nonce()` extracts sender + nonce from signed envelope without sig verification
- [x] Tests: insert_checked_accepts_matching_nonce, insert_checked_accepts_future_within_gap, insert_checked_rejects_stale_nonce, insert_checked_rejects_far_future_nonce, insert_checked_rejects_malformed (5 tests)

### Task 7: Duplicate tx hash rejection — DONE
- [x] Tx hash = BLAKE3(raw bytes) — deduplicated via mempool `seen` set (Sprint 008)
- [x] Mempool `seen` set persists after removal for replay prevention
- [x] Tests: dedup_by_hash_persists_after_removal, dedup_uses_blake3_hash (existing tests cover)

### Task 8: Nonce-ordered execution within batch — DONE
- [x] Pipeline sorts routed txs by `(sender, nonce)` before execution
- [x] `routed.sort_by(|a, b| a.sender().cmp(b.sender()).then(a.nonce().cmp(&b.nonce())))`
- [x] Transactions from same sender execute in nonce order regardless of batch position
- [x] Tests: pipeline_executes_transfers (verifies nonce-ordered execution)

**Phase 2 Exit Criteria:**
- [x] Stale and far-future nonces rejected at mempool
- [x] Duplicate transactions rejected at mempool layer
- [x] Batch execution respects nonce ordering per sender
- [x] 5+ new tests

---

## Phase 3: Fee Market Basics (Tasks 9-12)

### Task 9: Gas price in TxKind — DONE
- [x] Added `gas_price: u64` field to all 7 TxKind variants
- [x] `TxKind::gas_price()` accessor returns the gas price for any variant
- [x] `TxKind::gas_limit()` accessor: fixed 21_000 for Transfer, 53_000 for CreateAgent, field value for others
- [x] Wire format updated (backward-incompatible — acceptable pre-mainnet)
- [x] All test TxKind constructions updated with `gas_price: 0`
- [x] Tests: all existing routing roundtrip tests validate gas_price serialization

### Task 10: Fee deduction in execution — DONE
- [x] `collect_fees()` deducts `gas_used * gas_price` from sender balance post-execution
- [x] Fee deduction uses `saturating_mul` and `saturating_sub` for overflow safety
- [x] Zero gas_price skipped (no fee for gas_price == 0)
- [x] Tests: pipeline_deducts_fees, pipeline_zero_gas_price_no_fee (2 tests)

### Task 11: Base fee calculation (EIP-1559 style) — DONE
- [x] `BaseFeeCalculator` in `aztibase-execution/src/fee.rs`
- [x] Base fee adjusts per batch: above target → increase, below → decrease
- [x] Minimum base fee: 1 (MIN_BASE_FEE), maximum: 1B (MAX_BASE_FEE)
- [x] Target gas per batch: 15M, max: 30M, change denominator: 8
- [x] `validate_gas_price()` checks against current base fee
- [x] `escrow_fee()` / `refund_unused()` for pre-execution fee escrow pattern
- [x] Tests: 14 tests covering escrow, refund, base fee adjustment, overflow protection

### Task 12: RPC fee queries — DONE
- [x] `aztb_gasPrice` RPC method — returns current base fee as hex
- [x] `aztb_estimateGas` RPC method — returns fixed estimates per tx type prefix
- [x] Tests: gas_price_returns_base_fee, estimate_gas_transfer, estimate_gas_create_agent (3 tests)

**Phase 3 Exit Criteria:**
- [x] Fee collection implemented, deducted from sender balance after execution
- [x] Base fee calculator with EIP-1559 dynamics
- [x] RPC exposes gas price and estimation
- [x] 19+ new tests

---

## Phase 4: Security Review + Documentation (Tasks 13-15)

### Task 13: Security review — DONE
- [x] Review SignedTx for signature malleability (Ed25519 canonical checks)
- [x] Review nonce logic for gaps, overflows, and off-by-one errors
- [x] Review fee logic for integer overflow (gas_limit * gas_price)
- [x] Review address derivation for collision resistance
- [x] Review mempool for DoS vectors (nonce gap abuse, gas price spam)
- [x] Classify all findings (ELEVATED / MEDIUM / LOW)

**Security Findings:**

| ID | Severity | Area | Finding | Status |
|----|----------|------|---------|--------|
| SEC-SIG-001 | LOW | SignedTx | Ed25519 via ed25519-dalek uses strict verification (rejects non-canonical signatures). No malleability risk. | VERIFIED |
| SEC-SIG-002 | MEDIUM | SignedTx | `verify_and_route()` does not check nonce at pipeline level — nonce validation only at mempool. Batch-level same-nonce conflicts theoretically possible. Mitigated by mempool dedup. | DOCUMENTED |
| SEC-FEE-001 | LOW | Fee | `collect_fees()` uses `saturating_mul`/`saturating_sub` — cannot overflow or underflow. | VERIFIED |
| SEC-FEE-002 | MEDIUM | Fee | Post-execution fee deduction: sender with insufficient balance still has tx executed, fee capped at available balance via `saturating_sub`. Pre-execution escrow (`escrow_fee`) exists but not yet wired into pipeline. | DOCUMENTED |
| SEC-FEE-003 | LOW | Fee | `BaseFeeCalculator` is stateless in RPC — always returns base fee of 1. Needs pipeline state integration for dynamic adjustment. | DOCUMENTED |
| SEC-NONCE-001 | LOW | Nonce | MAX_NONCE_GAP=16 hardcoded. Future nonces within gap accepted but may never execute if intermediates missing. Bounded by mempool max_size. | DOCUMENTED |
| SEC-NONCE-002 | LOW | Nonce | `decode_sender_nonce()` skips signature verification for performance. Forged envelopes pass mempool but fail at pipeline verification. No execution risk. | VERIFIED |
| SEC-ADDR-001 | LOW | Address | BLAKE3 hash of Ed25519 pubkey: 256-bit collision resistance. Address derivation is sound. | VERIFIED |

**Summary:** 0 ELEVATED, 2 MEDIUM (both documented, acceptable pre-mainnet), 6 LOW. No blockers.

### Task 14: cargo clippy + fmt + test — DONE
- [x] `cargo clippy --workspace` — zero warnings
- [x] `cargo fmt --check` — clean
- [x] `cargo test --workspace` — 353 tests passing
- [ ] `cargo audit` — skipped (not installed; prior advisories documented)

### Task 15: Documentation updates — DONE
- [x] BUILD_LOG.md entries for all 4 phases
- [x] STATUS.md updated (sprint 011 complete, 353 tests)
- [x] CHANGELOG.md entries for signatures, nonces, fees
- [x] Sprint 011 retrospective written
- [x] DECISIONS.md: ADR-006 for post-execution fee collection

**Phase 4 Exit Criteria:**
- [x] Zero ELEVATED security findings open
- [x] All docs updated per Doc Sync Rules
- [x] Sprint 011 fully complete

---

## Definition of Done (Sprint 011)

- [x] All 15 tasks completed
- [x] `cargo check --workspace` passes
- [x] `cargo test --workspace` passes (353 tests)
- [x] `cargo clippy --workspace` zero warnings
- [x] `cargo fmt --check` clean
- [x] All code has BUILD_LOG entries
- [x] Security review complete (0 ELEVATED, 2 MEDIUM documented)
- [x] STATUS.md updated

---

## Sprint 011 Retrospective

**Delivered:**
- Phase 1: Ed25519 signed transaction envelopes — `SignedTx` type, wire format, `verify_and_route()` pipeline integration
- Phase 2: Nonce enforcement — mempool `insert_checked()` with MAX_NONCE_GAP, nonce-ordered batch execution, address derivation
- Phase 3: Fee market — `gas_price` on all 7 TxKind variants, `collect_fees()` deduction, `BaseFeeCalculator` EIP-1559 dynamics, `escrow_fee/refund_unused`, `aztb_gasPrice` + `aztb_estimateGas` RPC
- Phase 4: Security review (2 MEDIUM, 6 LOW), all docs updated

**Metrics:**
- Tests: 315 → 353 (+38 new tests)
- Security gaps closed: SEC-AA-001 (fees), SEC-AA-002 (signatures), SEC-AA-003 (nonces)
- New files: `fee.rs`
- Modified: routing.rs, tx.rs, mempool.rs, pipeline.rs, server.rs, lib.rs, integration.rs, main.rs

**Lessons Learned:**
1. Post-execution fee collection is simpler but weaker than pre-execution escrow — wiring `escrow_fee` into the pipeline is Sprint 012 priority
2. Adding a field to all TxKind variants requires updating ~50+ test sites — parallel subagent updates essential
3. Mempool nonce validation without sig verification is a pragmatic tradeoff — forged envelopes fail at pipeline

**What Went Well:**
- All three Sprint 010 security gaps closed in a single sprint
- `SignedTx` wire format is clean and extensible
- Fee system has both simple and advanced patterns ready

**Risks/Debt:**
- SEC-FEE-002: Post-execution fees don't prevent underfunded execution
- SEC-FEE-003: BaseFeeCalculator not persistent across batches
- SEC-SIG-002: Same-nonce batch conflicts theoretically possible

---

## Candidates for Next Sprint (012)

1. Pre-execution fee escrow (wire `escrow_fee` into pipeline, close SEC-FEE-002)
2. Persistent `BaseFeeCalculator` state across batches (close SEC-FEE-003)
3. WebRTC transport for browser nodes (libp2p-webrtc)
4. Light client protocol (header sync, state proofs)
5. bn128 precompiles (pure Rust via ark-bn254)
6. Verkle tree state commitment (replace BLAKE3 Merkle)
7. Block-STM for contract/EVM txs (read/write tracking in VM)
