# Sprint 062 — Sentinel Tier 3: Autonomous Action Engine

**Sprint Letter:** G
**Start Date:** 2026-03-19
**Status:** DONE
**Owner:** ai-integration-engineer + security-engineer + blockchain-architect

---

## Goal

Turn the AI Sentinel from a passive observer (Tiers 1 & 2) into an autonomous safety system that responds to chain anomalies faster than a human operator. Actions are gated by confidence tiers with escalating authority and safety guardrails.

**Headline:** "The chain watches itself — and acts when something goes wrong."

---

## Phase 1: Action Engine Core (sentinel.rs)

**Owner:** ai-integration-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 1.1 | Define `SentinelAction` enum: `Alert`, `AdjustBaseFee`, `ProposeGovernance`, `EmergencyPause` | DONE |
| 1.2 | Define `ConfidenceTier` enum (Low/Medium/High/Critical) with threshold constants | DONE |
| 1.3 | Implement `ActionEngine` struct: confidence evaluation, cooldown tracking, sustained anomaly detection (N consecutive ticks above threshold before acting) | DONE |
| 1.4 | Implement rate limiting: minimum 1000 batches between on-chain actions, 100 batches between alerts | DONE |
| 1.5 | Implement dry-run mode: log what action would be taken without executing | DONE |
| 1.6 | Action log: ring buffer of recent actions (last 100) queryable via RPC | DONE |
| 1.7 | Unit tests: each confidence tier triggers correct action, cooldown prevents rapid-fire, sustained anomaly requirement, dry-run logs without acting | DONE |

### Exit Criteria
- [x] Score 0.3-0.5 sustained → Alert action
- [x] Score 0.5-0.8 sustained → AdjustBaseFee action (if emergency key available)
- [x] Score 0.8+ sustained → ProposeGovernance action (if validator)
- [x] Score 0.95+ sustained → EmergencyPause action (if emergency key available, before sunset)
- [x] Dry-run mode logs all actions without side effects
- [x] Cooldown enforced between actions

---

## Phase 2: Node Wiring + On-Chain Execution

**Owner:** ai-integration-engineer + node-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 2.1 | Integrate ActionEngine into `run_sentinel` loop: evaluate after each score, dispatch actions | DONE |
| 2.2 | `SentinelExecutor`: signs and submits EmergencyAction/Governance txs using node keyfile, tracks nonce | DONE |
| 2.3 | Wire emergency key into sentinel config (loaded from genesis or CLI) | DONE |
| 2.4 | Wire mempool handle for tx injection | DONE |
| 2.5 | CLI flags: `--sentinel-dry-run` (default true for safety), `--sentinel-auto-pause` (enable CRITICAL actions) | DONE |
| 2.6 | Degrade gracefully: nodes without emergency key → log recommendations only | DONE |

### Exit Criteria
- [x] Sentinel can auto-submit EmergencyAction::Pause when CRITICAL + auto-pause enabled + emergency key present
- [x] Sentinel can auto-adjust base fee via EmergencyAction::ForceParam when MEDIUM + emergency key present
- [x] Non-key nodes log recommendations without submitting
- [x] Default is dry-run (opt-in to live actions)

---

## Phase 3: RPC + Observability

**Owner:** node-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 3.1 | `aztb_getSentinelActions` RPC: returns recent actions (type, score, batch, timestamp, dry_run flag) | DONE |
| 3.2 | Extend `chainHealth` WebSocket topic with action events | DONE |
| 3.3 | Sentinel action counter in Prometheus metrics | DONE |

### Exit Criteria
- [x] RPC returns action history
- [x] WebSocket pushes action events to subscribers

---

## Phase 4: Security Review

**Owner:** security-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 4.1 | Review action authority: can sentinel do anything beyond defined 4 action types? | DONE |
| 4.2 | Review false positive safety: sustained anomaly + cooldown sufficient? | DONE |
| 4.3 | Review privilege: sentinel with emergency key has same power as manual EmergencyAction — acceptable during foundation period? | DONE |
| 4.4 | Verify dry-run default: no accidental live actions on first run | DONE |
| 4.5 | Clippy + fmt + cargo test (zero warnings, all pass) | DONE |

### Exit Criteria
- [x] 0 ELEVATED flags
- [x] Dry-run default verified
- [x] False positive rate acceptable (sustained anomaly requirement)

---

## Phase 5: Documentation

**Owner:** documentation-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 5.1 | ADR-034: Sentinel Tier 3 Autonomous Action Engine | DONE |
| 5.2 | BUILD_LOG entry | DONE |
| 5.3 | CHANGELOG entry | DONE |
| 5.4 | STATUS.md update | DONE |

---

## Dependencies

- Sprint 061: EmergencyAction TxKind (0x1D) — DONE
- Sprint 060: Sentinel Tiers 1 & 2 — DONE
- Existing: ChainParams, GovernanceStore, mempool

## Risks

| Risk | Mitigation |
|------|-----------|
| False positive triggers emergency pause | Sustained anomaly requirement (3+ consecutive ticks), CRITICAL threshold at 0.95, dry-run default |
| Sentinel amplifies oscillation (adjusts fee → score changes → adjusts again) | Cooldown of 1000 batches between on-chain actions, ±5% max adjustment per action |
| Emergency key compromise via sentinel code path | Same key security as manual EmergencyAction — if key is compromised, sentinel path is no worse |

## Next Sprint

Sprint 062b: ONNX Tx Anomaly Scorer Upgrade (shares TractRuntime)
