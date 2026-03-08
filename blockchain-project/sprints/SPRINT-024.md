# Sprint 024 — M7 Kickoff: Security Hardening & Code Audit

**Milestone:** M7 (Security Audit + Hardening) — Sprint 1 of 3
**Start Date:** 2026-03-08
**Status:** COMPLETE

---

## Goal

Begin M7 by resolving all documented MEDIUM-severity security findings, hardening high-risk subsystems (EVM, cross-VM bridge, WebSocket), and adding defensive bounds where production deployments require them. This sprint focuses on code-level security fixes — cryptographic audit and formal verification are deferred to Sprint 025.

---

## Phase 1: Cross-VM Reentrancy Guard + EVM Hardening (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Full reentrancy guard for cross-VM bridge: track call stack per-transaction, reject reentrant calls to the same contract within a single execution context (resolves SEC-BRIDGE-003 MEDIUM) | security-engineer | DONE |
| 2 | EVM revert reason truncation: cap revert data at 1024 bytes in ExecutionReceipt, add test for oversized revert (resolves SEC-EVM-008) | smart-contract-engineer | DONE |
| 3 | EVM gas cost verification tests: add test cases that assert expected gas consumption for deploy, call, and precompile ops (resolves SEC-EVM-002) | smart-contract-engineer | DONE |
| 4 | AI inference timeout enforcement: add configurable timeout (default 30s) to TractRuntime::infer, return error on expiry (resolves SEC-AI-002) | ai-integration-engineer | DONE |

**Exit Criteria:** SEC-BRIDGE-003 resolved. EVM gas and revert hardened with tests. AI inference bounded by timeout.

---

## Phase 2: Resource Bounds & Rate Limiting (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Receipt store eviction: prune receipts older than configurable round window (default 10_000 rounds), add test (resolves SEC-RCPT-002) | node-engineer | DONE |
| 6 | Per-IP WebSocket rate limiting: track connections per IP, reject new connections beyond limit (default 8 per IP), add test (resolves SEC-WS-002) | p2p-network-engineer | DONE |
| 7 | Proof cache bounded eviction: add max entry count (default 4096) to LightStore proof cache alongside round-based eviction, add test (resolves SEC-CACHE-001) | node-engineer | DONE |
| 8 | STUN server URL validation: parse and validate STUN/TURN URIs in WebRTC transport config, reject malformed URLs, add test (resolves SEC-WEBRTC-001) | p2p-network-engineer | DONE |

**Exit Criteria:** All resource-consuming subsystems have bounded growth. Per-IP rate limiting active on WebSocket gateway.

---

## Phase 3: Light Client & P2P Hardening (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Light sync multi-peer validation: request headers from N peers (default 3), accept only if majority agree, add test (resolves SEC-P2P-001) | p2p-network-engineer | DONE |
| 10 | BLS signer bitmap optimization: replace `Vec<bool>` with `bitvec` bitfield for compact finality certificate encoding, add roundtrip test (resolves SEC-BLS-009) | consensus-engineer | DONE |
| 11 | Finality certificate structural validation: reject certs with zero signers, mismatched rounds, or empty batch hash; add 3 negative-case tests (resolves SEC-LC-001) | consensus-engineer | DONE |
| 12 | `cargo-deny` setup: configure deny.toml for license audit + advisory DB checks, integrate into CI-ready check script | security-engineer | DONE |

**Exit Criteria:** Light sync is eclipse-resistant. Finality certificates fully validated. Dependency audit tooling in place.

---

## Phase 4: Security Review + M7-Sprint-1 Close (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Full security review of all Phase 1-3 changes as security-engineer: threat model each fix, verify no regressions | security-engineer | DONE |
| 14 | `cargo clippy` zero warnings + `cargo fmt --check` clean + all tests pass | node-engineer | DONE |
| 15 | Update BUILD_LOG.md, STATUS.md, CHANGELOG.md, DECISIONS.md (if ADRs needed) | documentation-engineer | DONE |
| 16 | Sprint 024 retrospective + draft Sprint 025 scope (crypto audit + Verkle commitments) | project-lead | DONE |

**Exit Criteria:** All MEDIUM findings resolved. All LOW findings from resource bounds resolved. Zero new ELEVATED/MEDIUM. Docs updated.

---

## Security Findings Targeted

| Finding | Severity | Sprint Introduced | Resolution |
|---------|----------|-------------------|------------|
| SEC-BRIDGE-003 | MEDIUM | 016 | Task 1: Full reentrancy guard |
| SEC-EVM-008 | LOW | 009 | Task 2: Revert reason truncation |
| SEC-EVM-002 | LOW | 009 | Task 3: Gas cost test coverage |
| SEC-AI-002 | LOW | 007 | Task 4: Inference timeout |
| SEC-RCPT-002 | LOW | 007 | Task 5: Receipt eviction |
| SEC-WS-002 | LOW | 020 | Task 6: Per-IP rate limit |
| SEC-CACHE-001 | LOW | 017 | Task 7: Proof cache bound |
| SEC-WEBRTC-001 | LOW | 015 | Task 8: STUN URL validation |
| SEC-P2P-001 | LOW | 018 | Task 9: Multi-peer sync |
| SEC-BLS-009 | INFO | 005 | Task 10: Bitmap optimization |
| SEC-LC-001 | LOW | 016 | Task 11: Cert validation |

---

## M7 Roadmap (3 sprints)

| Sprint | Focus | Status |
|--------|-------|--------|
| 024 | Security hardening + code audit (this sprint) | COMPLETE |
| 025 | Cryptographic audit + Verkle polynomial commitments + cargo-deny run | NEXT |
| 026 | Dependency audit + pipeline refactor + M7 close | NOT PLANNED |

---

## Sprint 024 Retrospective

### What went well
- All 11 documented security findings resolved in a single sprint
- Zero regressions: test count increased from 555 to 570
- SignerBitmap achieved 8x bandwidth savings without adding dependencies
- MultiPeerValidator and IpConnectionTracker are clean, testable abstractions

### What could improve
- AI inference timeout is a post-execution check (tract is synchronous); true async cancellation would be better but requires upstream tract changes
- cargo-deny is set up but hasn't been run yet (install was background); should be validated in Sprint 025
- Some light client types (SyncFinalityCert, LightFinalityCert) still use Vec<bool>; could be unified later

### Sprint 025 scope (draft)
1. Run cargo-deny and resolve any license/advisory findings
2. Verkle polynomial commitment integration (replace BLAKE3 placeholder)
3. Cryptographic audit: review all signature schemes, hash usage, key derivation
4. Formal threat model document for consensus + P2P layers
5. Fuzz testing setup for transaction routing and serialization
6. Performance benchmarks: block production, state transitions, BLS aggregation
