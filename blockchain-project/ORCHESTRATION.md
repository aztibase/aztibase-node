# ORCHESTRATION PROTOCOL - Genesis Chain Skill Fleet

## Execution Dependency Graph

```
Phase 3 (Parallel):
  research-analyst ──────────────┐
  legal-ip-counsel ──────────────┤
                                 v
Phase 4 (Sequential with dependencies):
  blockchain-architect ──────────┐ (reads RESEARCH_BRIEF + LEGAL_LANDSCAPE)
       │                         │
       v                         │
  consensus-engineer ────────────┤ (needs architect's initial framework)
       │                         │
       v                         │
  node-engineer ─────────────────┤ (needs consensus design)
       │                         │
       v                         │
  p2p-network-engineer ──────────┤ (needs node types defined)
       │                         │
  [PARALLEL BATCH]               │
  security-engineer ─────────────┤ (reviews all above)
  ai-integration-engineer ───────┤ (needs consensus + security)
  smart-contract-engineer ───────┤ (needs architect + security)
       │                         │
       v                         │
  tokenomics-engineer ───────────┘ (needs consensus + node + security)

Phase 5 (Legal-Gated Sequence):
  Step 5a: tokenomics-engineer + blockchain-architect → 10 raw names
  Step 5b: legal-ip-counsel → LEGAL_CLEARANCE_REPORT.md
  Step 5c: naming-council → NAMING_REPORT.md (only GREEN names)

Phase 6:
  blockchain-architect → GENESIS_CHAIN_MASTER_PLAN.md
```

## Output Sharing Protocol

All skills share outputs via MASTER_DESIGN.md sections:

| Skill | Writes To | Reads From |
|-------|-----------|------------|
| research-analyst | RESEARCH_BRIEF.md | - |
| legal-ip-counsel | LEGAL_LANDSCAPE.md, LEGAL_CLEARANCE_REPORT.md | All sections (for patent/license review) |
| blockchain-architect | Section 0 (rulings), Section 1 | RESEARCH_BRIEF, LEGAL_LANDSCAPE, all sections |
| consensus-engineer | Section 2 | RESEARCH_BRIEF, Section 0 |
| tokenomics-engineer | Section 3 | RESEARCH_BRIEF, LEGAL_LANDSCAPE, Section 2 |
| node-engineer | Section 4 | RESEARCH_BRIEF, Section 0, Section 2 |
| security-engineer | Section 5 | RESEARCH_BRIEF, ALL sections |
| ai-integration-engineer | Section 6 | RESEARCH_BRIEF, Section 0, Section 2, Section 5 |
| smart-contract-engineer | Section 7 | RESEARCH_BRIEF, Section 0, Section 5, Section 6 |
| p2p-network-engineer | Section 8 | RESEARCH_BRIEF, Section 0, Section 4 |
| naming-council | Section 9 + NAMING_REPORT.md | LEGAL_CLEARANCE_REPORT, MASTER_DESIGN |

## Conflict Resolution Protocol

1. Any skill may raise a **CONFLICT** or **CHALLENGE** against another skill's output
2. Conflicts are flagged in MASTER_DESIGN.md with the format:
   ```
   > CONFLICT [raising-skill → target-skill]: [description]
   > Priority: [STANDARD / SECURITY-ELEVATED]
   ```
3. **blockchain-architect** has FINAL SAY on all technical conflicts
4. **security-engineer** conflicts are SECURITY-ELEVATED and must be addressed before proceeding
5. **legal-ip-counsel** has VETO POWER on naming/branding/IP conflicts
6. Stack challenges follow the same protocol but use the STACK RULING format

## Stack Challenge Protocol

1. Any skill may CHALLENGE a stack decision within their domain of authority
2. Challenges must include:
   - What is being challenged
   - Proposed alternative
   - Technical justification (benchmarks, research, or experience)
3. blockchain-architect rules on challenges using the STACK RULING format
4. Rulings are appended to Section 0 of MASTER_DESIGN.md

## Communication Format

Each skill appends to MASTER_DESIGN.md with a signed section:

```
---
### [SECTION TITLE]
**Contributed by:** [skill-name]
**Date:** [date]
**Status:** DRAFT / REVIEW / FINAL
**Dependencies met:** [list of prerequisites consumed]

[Content]

**Stack challenges raised:** [none / list]
**Conflicts raised:** [none / list]
**Sign-off:** [skill-name] ✓
---
```

## Legal Gates

- **GATE 1**: No name, ticker, or brand asset may be proposed publicly without legal-ip-counsel review
- **GATE 2**: naming-council is BLOCKED until LEGAL_CLEARANCE_REPORT.md is produced
- **GATE 3**: Only GREEN-status names proceed to naming-council
- **GATE 4**: legal-ip-counsel must be re-consulted if any of the following change after initial review:
  - Consensus mechanism (patent implications)
  - Token economic model (securities law implications)
  - Any proposed name or branding
  - Open-source license choice
  - Major dependency additions

## Phase Execution Triggers

| Phase | Trigger | Blocking Condition |
|-------|---------|-------------------|
| Phase 3 | Manual start | None |
| Phase 4 | RESEARCH_BRIEF.md + LEGAL_LANDSCAPE.md complete | Phase 3 completion |
| Phase 5a | All Phase 4 sections in MASTER_DESIGN.md | Phase 4 completion |
| Phase 5b | 10 raw names submitted | Step 5a completion |
| Phase 5c | LEGAL_CLEARANCE_REPORT.md complete with GREEN names | Step 5b completion |
| Phase 6 | NAMING_REPORT.md complete | Phase 5 completion |

---

## Build-Phase Tracking Protocol

The design phase (Phases 3-6) is complete. The following protocol governs all build activity from M1 onward.

### Tracking Documents

| Document | Location | Purpose | Updated By |
|----------|----------|---------|------------|
| STATUS.md | blockchain-project/STATUS.md | Living project status, milestones, risks | project-lead |
| DECISIONS.md | blockchain-project/DECISIONS.md | Architecture Decision Records (ADRs) | Any engineer (reviewed by blockchain-architect) |
| CHANGELOG.md | CHANGELOG.md | Per-milestone change log | Engineer who made the change |
| BUILD_LOG.md | blockchain-project/BUILD_LOG.md | Running build diary (who, what, when) | Engineer who did the work |
| Sprint plans | blockchain-project/sprints/SPRINT-NNN.md | Per-sprint task breakdown | project-lead |

### Build-Phase Rules

1. **Every code change must be traceable.** The chain is: Design (MASTER_DESIGN.md) -> Decision (DECISIONS.md) -> Sprint (sprints/) -> Code (git commit) -> Log (BUILD_LOG.md) -> Status (STATUS.md).

2. **ADRs are mandatory** for any non-obvious technical choice. "Non-obvious" means: a reasonable engineer could have chosen differently. If in doubt, write the ADR.

3. **BUILD_LOG entries are mandatory** for every meaningful code session. A "meaningful session" is any work that results in a git commit. Format is defined in BUILD_LOG.md.

4. **CHANGELOG entries are mandatory** for every user-facing or architecture-affecting change. Internal refactors that don't change behavior can skip the changelog.

5. **STATUS.md is updated** at sprint boundaries and whenever a milestone status changes. project-lead owns this document.

6. **Sprint plans are written before work begins.** No engineer starts M1+ build work without an active sprint plan assigning their tasks.

7. **Security flags propagate immediately.** If security-engineer raises a flag during build, it is logged in BUILD_LOG.md with `Security Flags:` and added to STATUS.md open risks. The flag blocks the affected code from shipping until resolved.

8. **Code ownership follows crate boundaries.** Each crate has a primary owner (see STATUS.md crate table). Cross-crate changes require review from both owners.

### Engineer Build Responsibilities

When an engineer completes a task, they must:
1. Commit the code with a clear commit message
2. Add a BUILD_LOG.md entry (prepended, newest first)
3. Update CHANGELOG.md if the change is user/architecture-facing
4. Write an ADR in DECISIONS.md if a non-obvious decision was made
5. Update their task status in the active sprint plan
6. Flag any security concerns with `Security Flags:` in their build log entry

### Sprint Lifecycle

```
1. project-lead creates sprints/SPRINT-NNN.md
2. Tasks assigned to engineers
3. Engineers work tasks, updating BUILD_LOG as they go
4. project-lead updates STATUS.md at sprint boundary
5. Sprint retrospective fills the retro section
6. Next sprint plan created
```
