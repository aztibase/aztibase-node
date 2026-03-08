# Sprint 025 — M7 Continued: Dependency Upgrades, Crypto Audit, Verkle Foundations

**Milestone:** M7 (Security Audit + Hardening) — Sprint 2 of 3
**Start Date:** 2026-03-08
**Status:** IN PROGRESS

---

## Goal

Resolve mainnet-blocking dependency vulnerabilities (wasmtime CVEs, bincode unmaintained), audit all cryptographic usage for correctness, replace the BLAKE3 Verkle placeholder with polynomial commitment foundations, and add fuzz testing for critical serialization paths.

---

## Phase 1: Dependency Upgrades — Wasmtime + Bincode (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | Upgrade wasmtime from v28 to v42: update Cargo.toml, fix API breakages in vm.rs (wasmtime::Error not anyhow). Resolves 4 CVEs. | smart-contract-engineer | DONE |
| 2 | Migrate bincode → postcard for all internal serialization (16 source files, 8 Cargo.toml). Resolves RUSTSEC-2025-0141. | node-engineer | DONE |
| 3 | Update deny.toml: removed wasmtime/bincode ignores, added 5 transitive dep ignores. cargo-deny passes clean. | security-engineer | DONE |
| 4 | Verify all tests pass after dependency upgrades, fix any API breakage. Fixed: merkle prove/verify not using merkle_parent() (2 test failures). Fixed: postcard trailing bytes (take_from_bytes). | security-engineer | DONE |

**Exit Criteria:** Zero advisory ignores in deny.toml for wasmtime/bincode. cargo-deny advisories clean. All 570+ tests pass.

---

## Phase 2: Cryptographic Audit (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Audit Ed25519 usage: DONE. Added TX_DOMAIN (b"AZTB_TX_V1") to SignedTx signing. Added AZTB_ATTESTATION_V1 prefix to attestation_hash. No key reuse found. | security-engineer | DONE |
| 6 | Audit BLAKE3 usage: DONE. Added LEAF_DOMAIN/NODE_DOMAIN prefixes to Merkle tree. Wallet/finality already had proper domain separation. | security-engineer | DONE |
| 7 | Audit BLS12-381 usage: audited. DSTs correct. PoP implemented but NOT enforced at CommitCompute registration — must add bls_pop field + verify. | security-engineer | PENDING |
| 8 | Add cryptographic test vectors: known-answer tests for Ed25519 sign/verify, BLAKE3 domain separation, BLS aggregate verify. Ensures no regression if crypto backends change. | security-engineer | PENDING |

**Exit Criteria:** All crypto paths documented with domain separation analysis. Known-answer test vectors added. Any gaps documented as security findings.

---

## Phase 3: Verkle Polynomial Commitments (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Research and select Verkle commitment library: evaluate verkle-trie crate, ipa-multipoint, or hand-roll Pedersen/IPA commitments. Must be pure Rust (ADR-001). | blockchain-architect | PENDING |
| 10 | Implement VerkleNode with polynomial commitment: replace BLAKE3 hash-based VerkleTree with actual IPA commitment structure. Internal nodes store commitments, leaves store values. | blockchain-architect | PENDING |
| 11 | Update StateCommitment trait: VerkleCommitment generates real polynomial proofs instead of BLAKE3 placeholder. Proof verification uses IPA check. | blockchain-architect | PENDING |
| 12 | Verkle proof tests: opening proof generation + verification, multi-leaf batch proofs, invalid proof rejection, proof size benchmarks. | blockchain-architect | PENDING |

**Exit Criteria:** VerkleTree uses polynomial commitments. Proofs are verifiable. Benchmark shows proof size < 2KB for 256-key tree.

---

## Phase 4: Fuzz Testing + Sprint Close (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Set up cargo-fuzz for aztibase-core: fuzz targets for Transaction deserialization, SignedTx verification, BLAKE3 hash roundtrip. | security-engineer | PENDING |
| 14 | Set up cargo-fuzz for aztibase-consensus: fuzz targets for Vertex deserialization, FinalityCertificate verification, SignerBitmap operations. | security-engineer | PENDING |
| 15 | Security review: run full test suite, clippy, fmt, cargo-deny. Document any new findings. | security-engineer | PENDING |
| 16 | Update BUILD_LOG, STATUS, CHANGELOG, DECISIONS. Sprint retrospective + Sprint 026 scope. | documentation-engineer | PENDING |

**Exit Criteria:** Fuzz targets defined and runnable. Zero clippy warnings. All tests pass. Docs updated.

---

## Dependencies

- Phase 1 must complete before Phase 3 (cargo-deny must be clean before adding new deps)
- Phase 2 is independent and can run in parallel with Phase 1
- Phase 4 depends on all prior phases
