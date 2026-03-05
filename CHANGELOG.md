# Changelog -- Dendrite Network

All notable changes to this project are documented here, organized by milestone.
Format follows [Keep a Changelog](https://keepachangelog.com/).

---

## [M1] -- Core Primitives (Complete)

### Added
- CryptoProvider trait for quantum-ready crypto abstraction (2026-03-05)
- AIRuntime trait with Passthrough mode for non-AI nodes (2026-03-05)
- Node startup wiring: storage, network, TOML config, graceful shutdown (2026-03-05)
- Build-phase tracking infrastructure: STATUS.md, DECISIONS.md, CHANGELOG.md, BUILD_LOG.md (2026-03-05)

### Changed
- Switched storage backend from RocksDB to redb (ADR-001) (2026-03-05)

### Security
- Fixed WASM host function bounds checking (negative ptr, overflow, out-of-bounds) (2026-03-05)
- Security review: 13-item checklist passed, zero ELEVATED flags (2026-03-05)
- cargo-audit: 5 transitive advisories documented, none exploitable in our code (2026-03-05)

---

## [M0] -- Design Phase (Complete)

### Added
- Full design phase deliverables (2026-03-05):
  - GENESIS_CHAIN_MASTER_PLAN.md -- definitive blueprint
  - MASTER_DESIGN.md -- full technical design (9 sections, all engineers)
  - RESEARCH_BRIEF.md -- competitive analysis and market research
  - LEGAL_LANDSCAPE.md -- IP and regulatory landscape
  - LEGAL_CLEARANCE_REPORT.md -- name/ticker clearance
  - NAMING_REPORT.md -- final naming decision (Dendrite Network / DNDR)
  - ORCHESTRATION.md -- team protocol and dependency graph
  - dendrite-visual-explainer.html -- interactive visual explainer
- 12 specialist skills defined in .claude/skills/
- Workspace scaffolded: 8 Rust crates in Cargo workspace
- Initial crate structure with module stubs for all domains
