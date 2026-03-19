# Sprint 061 — Progressive Decentralization + EmergencyAction

**Sprint Letter:** F
**Start Date:** TBD
**Status:** DONE
**Owner:** project-lead + blockchain-architect + security-engineer

---

## Goal

Ship the mainnet protection layer: permissioned validator registration with progressive opening timeline, and a foundation emergency key with hard-coded 1-year sunset. These two mechanisms protect the chain during the vulnerable bootstrap phase (first 10-20 validators, low market cap).

**Headline:** "Aztibase launches with training wheels that self-destruct."

---

## Phase 1: ValidatorRegistrationMode ChainParam

**Owner:** blockchain-architect + consensus-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 1.1 | Add `ValidatorRegistrationMode` enum: `Permissioned`, `StakeGated`, `Open` | DONE |
| 1.2 | Add `validator_registration_mode` to ChainParams registry with default `Permissioned` | DONE |
| 1.3 | Add `approved_validators` list to genesis config (foundation-approved pubkeys) | DONE |
| 1.4 | Gate `RegisterValidator` (0x1C) execution on current mode: Permissioned checks approved list, StakeGated enforces 500K minimum, Open uses existing 50K minimum | DONE |
| 1.5 | Add governance proposal type to change registration mode | DONE |
| 1.6 | Unit tests: permissioned reject/accept, stake-gated threshold, mode transitions | DONE |

### Exit Criteria
- [x] Unapproved validator registration rejected in Permissioned mode
- [x] 500K stake enforced in StakeGated mode
- [x] Existing behavior preserved in Open mode
- [x] Mode changeable via governance proposal

---

## Phase 2: EmergencyAction TxKind (0x1D)

**Owner:** blockchain-architect + security-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 2.1 | Define `EmergencyAction` TxKind (0x1D) with variants: `Pause`, `Unpause`, `ForceParam`, `RemoveValidator` | DONE |
| 2.2 | Add `emergency_key` field to genesis config (Ed25519 pubkey) | DONE |
| 2.3 | Add `EMERGENCY_KEY_SUNSET_EPOCH` constant (~365 days worth of epochs) | DONE |
| 2.4 | Implement execution: verify sender == emergency_key AND current_epoch < sunset_epoch | DONE |
| 2.5 | Implement `Pause`/`Unpause`: set `chain_paused` flag, pipeline skips tx execution when paused (still produces empty blocks for liveness) | DONE |
| 2.6 | Implement `ForceParam`: update ChainParam without governance vote | DONE |
| 2.7 | Implement `RemoveValidator`: deregister + unstake target validator | DONE |
| 2.8 | Add RPC: `aztb_getEmergencyKeyStatus` (active/expired, sunset epoch, key address) | DONE |
| 2.9 | Unit tests: valid emergency action, expired key rejected, wrong sender rejected, pause/unpause lifecycle, each action variant | DONE |

### Exit Criteria
- [x] Emergency key can pause/unpause chain
- [x] Emergency key can force parameter changes
- [x] Emergency key can remove validators
- [x] Key is dead code after sunset epoch (hardcoded, ungovernable)
- [x] RPC exposes key status for transparency

---

## Phase 3: Security Review

**Owner:** security-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 3.1 | Review EmergencyAction for privilege escalation — can the key do anything beyond the 4 defined actions? | DONE |
| 3.2 | Review sunset logic — can it be extended, bypassed, or reset? | DONE |
| 3.3 | Review ValidatorRegistrationMode — can an attacker force a mode change without governance? | DONE |
| 3.4 | Review pause mechanism — can a paused chain still produce blocks (liveness)? | DONE |
| 3.5 | Clippy + fmt + cargo test (zero warnings, all pass) | DONE |

### Exit Criteria
- [x] 0 ELEVATED flags
- [x] Sunset is provably ungovernable (no code path can extend it)
- [x] Pause preserves liveness (empty blocks continue)

---

## Phase 4: Documentation

**Owner:** documentation-engineer

### Tasks

| # | Task | Status |
|---|------|--------|
| 4.1 | ADR-033: Progressive Decentralization (ValidatorRegistrationMode + EmergencyAction + timeline) | DONE |
| 4.2 | BUILD_LOG entry | DONE |
| 4.3 | CHANGELOG entry | DONE |
| 4.4 | STATUS.md update | DONE |
| 4.5 | Update VALIDATOR_ONBOARDING.md with permissioned registration process | DONE |
| 4.6 | Public transparency statement for community (draft) | DONE |

---

## Dependencies

- None — all infrastructure exists (ChainParams, Governance, TxKind routing, genesis config)

## Risks

| Risk | Mitigation |
|------|-----------|
| Emergency key compromise | Store offline, use only in emergencies, sunset limits blast radius |
| Community pushback on centralization | Transparent communication + hard-coded sunset = trust |
| Sunset too short (1 year) | Can be longer, but 1 year aligns with progressive decentralization timeline |

## Next Sprint

Sprint 062: Sentinel Tier 3 — Autonomous Action Engine (uses EmergencyAction from this sprint)
