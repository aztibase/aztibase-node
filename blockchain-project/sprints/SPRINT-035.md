# Sprint 035 — M8 Sprint 10: On-Chain Governance Foundations

**Goal:** Add on-chain governance primitives allowing validators to create proposals, cast stake-weighted votes, and execute parameter changes via consensus. This is a prerequisite for mainnet decentralized governance.

**Started:** 2026-03-09
**Completed:** 2026-03-09
**Status:** COMPLETE

---

## Phase 1: Governance Types & Proposal Store (Tasks 1–5)

Core governance data structures, proposal lifecycle, and vote tallying with stake-weighted quorum.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 1 | `Proposal` struct: id, proposer, description, param_key, param_value, start_round, end_round, status | smart-contract-engineer | DONE |
| 2 | `Vote` struct: voter address, proposal_id, approve (bool), weight (stake) | smart-contract-engineer | DONE |
| 3 | `ProposalStatus` enum: Active, Passed, Rejected, Executed | smart-contract-engineer | DONE |
| 4 | `GovernanceStore`: in-memory proposal storage with insert/get/list, vote tracking, tally logic, `CreateProposalParams` struct | smart-contract-engineer | DONE |
| 5 | 6 unit tests: proposal lifecycle, vote tallying, quorum threshold, double-vote rejection, expired proposal finalization, empty store | smart-contract-engineer | DONE |

**Exit criteria:** GovernanceStore compiles with full proposal lifecycle, 6 tests pass ✓

---

## Phase 2: Governance TxKinds & Pipeline Routing (Tasks 6–10)

Two new transaction types wired through the routing and execution pipeline.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 6 | `TxKind::CreateProposal` (0x0E): proposer, description, param_key, param_value, voting_period, nonce, gas_price | smart-contract-engineer | DONE |
| 7 | `TxKind::CastVote` (0x0F): voter, proposal_id, approve, nonce, gas_price | smart-contract-engineer | DONE |
| 8 | Wire routing: PREFIX constants, encode/decode, nonce/gas_price/sender/gas_limit match arms, empty description rejection | smart-contract-engineer | DONE |
| 9 | Pipeline execution: CreateProposal validation (description length, param bounds, voting period), CastVote validation (proposal exists, active, no double vote, balance as stake weight) | node-engineer | DONE |
| 10 | 2 unit tests: CreateProposal roundtrip, CastVote roundtrip (pipeline e2e tests covered by governance unit tests) | node-engineer | DONE |

**Exit criteria:** 2 new TxKinds route correctly, pipeline creates proposals and records votes, 2 tests pass ✓

---

## Phase 3: Proposal Finalization & RPC (Tasks 11–15)

Auto-finalize proposals at end_round, expose governance via RPC.

| # | Task | Owner | Status |
|---|------|-------|--------|
| 11 | `GovernanceStore::finalize_expired()`: check proposals past end_round, tally votes, update status to Passed/Rejected based on quorum (>50% of voting stake, min 2 voters) | smart-contract-engineer | DONE |
| 12 | Pipeline: call `finalize_expired()` after each batch execution, log finalized proposals | node-engineer | DONE |
| 13 | `aztb_getProposal(proposal_id)` RPC: return proposal details + vote tally (approveWeight, rejectWeight, voterCount) | node-engineer | DONE |
| 14 | `aztb_listProposals(status_filter)` RPC: return all proposals matching optional status filter (Active/Passed/Rejected/Executed) | node-engineer | DONE |
| 15 | Governance wired into main.rs: `.with_governance()` on RPC server, `shared_governance()` on pipeline | node-engineer | DONE |

**Exit criteria:** Proposals auto-finalize, 2 RPC methods work, governance wired end-to-end ✓

---

## Phase 4: Security Review & Documentation (Tasks 16–18)

| # | Task | Owner | Status |
|---|------|-------|--------|
| 16 | Security review: double voting prevented, quorum gaming mitigated (≥2 voters), proposal spam capped (MAX_ACTIVE_PROPOSALS=64), voting period bounded (10-10000), description/param length-capped, zero-stake votes rejected | security-engineer | DONE |
| 17 | `cargo clippy` zero warnings, `cargo fmt --check` clean, 674 tests pass (8 new) | security-engineer | DONE |
| 18 | Update BUILD_LOG, STATUS, CHANGELOG, sprint plan | documentation-engineer | DONE |

**Exit criteria:** 0 ELEVATED, 0 MEDIUM, clippy clean, fmt clean, all tests pass, docs updated ✓
