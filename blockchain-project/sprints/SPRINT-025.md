# Sprint 025 — M7 Continued: Dependency Upgrades, Crypto Audit, Verkle Foundations

**Milestone:** M7 (Security Audit + Hardening) — Sprint 2 of 3
**Start Date:** 2026-03-08
**Status:** COMPLETE

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

**Exit Criteria:** Zero advisory ignores in deny.toml for wasmtime/bincode. cargo-deny advisories clean. All 570+ tests pass. ✅

---

## Phase 2: Cryptographic Audit (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 5 | Audit Ed25519 usage: Added TX_DOMAIN (b"AZTB_TX_V1") to SignedTx signing. Added AZTB_ATTESTATION_V1 prefix to attestation_hash. No key reuse found. | security-engineer | DONE |
| 6 | Audit BLAKE3 usage: Added LEAF_DOMAIN/NODE_DOMAIN prefixes to Merkle tree. Wallet/finality already had proper domain separation. | security-engineer | DONE |
| 7 | Audit BLS12-381 usage: DSTs correct. PoP implemented AND enforced at CommitCompute registration (pipeline.rs lines 1023-1052). No gap found. | security-engineer | DONE |
| 8 | Add cryptographic test vectors: 8 known-answer tests pinning BLAKE3 (3), Ed25519 (2), BLS12-381 (3) exact hex values. | security-engineer | DONE |

**Exit Criteria:** All crypto paths documented with domain separation analysis. Known-answer test vectors added. ✅

---

## Phase 3: Verkle Polynomial Commitments (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 9 | Research Verkle commitment libraries: verkle-trie, ipa-multipoint evaluated. No mature pure-Rust IPA crate exists. Decision: retain domain-separated BLAKE3 (ADR-013). | blockchain-architect | DONE |
| 10 | Rewrite VerkleTree with proper verifiable proofs: domain-separated leaf/inner commitments (AZTB_VERKLE_LEAF, AZTB_VERKLE_INNER), width-256 inner nodes, bottom-up verification. | blockchain-architect | DONE |
| 11 | Update VerkleProof/VerkleCommitment: self-contained proofs (stem + value_hash + levels), verify_proof recomputes commitments bottom-up. WASM proof types updated. | blockchain-architect | DONE |
| 12 | Verkle proof tests: 12 tests (roundtrip, wrong root/value/siblings rejection, nonexistent key, single-leaf, domain separation, trait impl, all-leaves). | blockchain-architect | DONE |

**Exit Criteria:** VerkleTree uses domain-separated BLAKE3 commitments. Proofs are properly verifiable. 12 Verkle tests + 20 WASM tests pass. ✅

---

## Phase 4: Fuzz Testing + Sprint Close (4 tasks)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 13 | Set up cargo-fuzz for aztibase-core: 3 fuzz targets (Transaction deser, BlockHeader deser, hash+pubkey). Requires nightly+libFuzzer (Linux CI). | security-engineer | DONE |
| 14 | Set up cargo-fuzz for aztibase-consensus: 3 fuzz targets (Vertex deser, FinalityCertificate deser, SignerBitmap ops). Requires nightly+libFuzzer (Linux CI). | security-engineer | DONE |
| 15 | Security review: clippy 0 warnings, fmt clean, cargo-deny clean (advisories ok, bans ok, licenses ok, sources ok). No new findings. | security-engineer | DONE |
| 16 | Doc updates: BUILD_LOG, STATUS, CHANGELOG, DECISIONS (ADR-013), sprint retrospective. | documentation-engineer | DONE |

**Exit Criteria:** Fuzz targets defined and ready for CI. Zero clippy warnings. All tests pass. Docs updated. ✅

---

## Dependencies

- Phase 1 must complete before Phase 3 (cargo-deny must be clean before adding new deps)
- Phase 2 is independent and can run in parallel with Phase 1
- Phase 4 depends on all prior phases

---

## Sprint Retrospective

### What went well
- Dependency upgrades (wasmtime v42, postcard) were smooth — API breakages were contained to 2-3 files each
- Crypto audit found that most paths already had proper domain separation; only tx signing and Merkle tree needed fixes
- BLS PoP enforcement was already complete (pipeline.rs) — no gap existed despite initial concern
- Verkle proof rewrite preserved the tree structure while making proofs properly verifiable

### What could improve
- Verkle research revealed no mature pure-Rust IPA crate exists — this blocks the eventual polynomial commitment upgrade
- Fuzz targets can't run on Windows (libFuzzer requires nightly + Linux) — need CI pipeline for fuzzing
- Full workspace test suite is slow on Windows due to PDB linker limits — `cargo clean` needed periodically

### Key decisions
- ADR-013: Retain BLAKE3 domain-separated commitments for Verkle tree (no IPA) — swap commitment functions when a pure-Rust crate matures
- Known-answer test vectors pin exact hex values — any crypto backend change will be caught immediately

### Metrics
- 16/16 tasks complete
- 0 new security findings
- 8 new crypto test vectors
- 12 new Verkle tests, 20 WASM tests passing
- 6 fuzz targets defined
- clippy 0 warnings, fmt clean, cargo-deny clean
