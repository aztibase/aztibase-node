# Dendrite Network - Project Genesis

## Project Overview
This is the Dendrite Network blockchain project. A Layer-1 AI-native, server-independent blockchain built in Rust.

## Key Documents
- Master Plan: `blockchain-project/GENESIS_CHAIN_MASTER_PLAN.md`
- Full Design: `blockchain-project/MASTER_DESIGN.md`
- Research: `blockchain-project/RESEARCH_BRIEF.md`
- Legal: `blockchain-project/LEGAL_LANDSCAPE.md`, `blockchain-project/LEGAL_CLEARANCE_REPORT.md`
- Naming: `blockchain-project/NAMING_REPORT.md`
- Orchestration: `blockchain-project/ORCHESTRATION.md`
- Visual Explainer: `blockchain-project/dendrite-visual-explainer.html`

## Branch Strategy
- **main**: Protected. Only receives merges from `dev` via PR. Never commit directly to main.
- **dev**: Active development branch. All work happens here. Push to `dev`, PR to `main`.
- Always ensure you are on the `dev` branch before making changes.

## Skill Fleet
This project uses a fleet of 13 specialist skills located in `.claude/skills/`.
Each skill represents a team member with a specific role, authority, and output responsibilities.

### How to invoke skills
Use `/project-lead` to coordinate the team or get status.
Use `/blockchain-architect`, `/consensus-engineer`, etc. to invoke a specific engineer.

### Team Structure
```
                 /project-lead
                      |
         +------------+------+------------+
         |            |      |            |
  /blockchain-   /research-  /docs-   /legal-ip-
    architect      analyst   engineer    counsel
         |
    +----+----+----+----+----+----+
    |    |    |    |    |    |    |
  cons  node  p2p  sec  ai  smart token
```

### Full Roster (13 skills)
| Skill | Role | Authority |
|-------|------|-----------|
| `/project-lead` | Orchestrator, strategic decisions, sprint planning | COMMANDS ALL |
| `/blockchain-architect` | Lead architect, final technical authority | FINAL SAY (technical) |
| `/consensus-engineer` | Synaptic Consensus (SynBFT + PoUW) | Consensus domain |
| `/node-engineer` | 5 node types, storage, sync | Node domain |
| `/p2p-network-engineer` | Networking, server-independence | P2P domain |
| `/security-engineer` | Threat modeling, crypto, auditing | ELEVATED PRIORITY |
| `/ai-integration-engineer` | 3-layer AI, compute market | AI domain |
| `/smart-contract-engineer` | Dual VM, contracts, gas model | Execution domain |
| `/tokenomics-engineer` | DNDR economics | Economics domain |
| `/documentation-engineer` | Dev docs, API refs, guides, docs site | Documentation |
| `/research-analyst` | Web research, competitive intel | Research |
| `/legal-ip-counsel` | IP, trademarks, regulatory | VETO on naming/IP |
| `/naming-council` | Branding, naming | Naming (legal-gated) |

### Chain of Command
- `/project-lead` commands all, resolves strategic disputes
- `/blockchain-architect` has FINAL SAY on technical disputes
- `/security-engineer` flags are ELEVATED priority — must be addressed before shipping
- `/legal-ip-counsel` has absolute VETO on naming/branding/IP — even project-lead cannot override

### Build Dependency Order
1. blockchain-architect (architecture framework)
2. consensus-engineer (needs architect)
3. node-engineer (needs consensus)
4. p2p-network-engineer (needs nodes)
5. security + ai-integration + smart-contract engineers (parallel)
6. tokenomics-engineer (needs all above)
7. legal-ip-counsel (gates naming)
8. naming-council (blocked by legal)
9. research-analyst (runs first in new research sprints)
10. documentation-engineer (continuous — documents each phase as it completes)

## Clean Code Rules (MANDATORY -- zero tolerance)

These rules are non-negotiable. Every line of code must look like it was written by a skilled human developer.

1. **No AI fingerprints**: Zero comments like "// AI-generated", "// Claude", "// TODO: Claude", "// Helper function for...", or any phrasing that reveals AI assistance. No boilerplate explanations disguised as comments. Comments exist ONLY to explain non-obvious *why*, never obvious *what*.
2. **No over-commenting**: Do not add comments to self-explanatory code. `let total = a + b; // add a and b` is a violation. If the code reads clearly, it needs no comment.
3. **No placeholder or filler code**: Every function does real work. No `todo!()` in shipped code unless explicitly marked as a future-phase stub in the sprint plan.
4. **Type errors are tracked**: When a type error occurs during development, Claude MUST log it in memory and never repeat the same mistake. Type errors are bugs in understanding -- fix the understanding, not just the code.
5. **Clippy + fmt are law**: `cargo clippy` zero warnings, `cargo fmt --check` clean. No exceptions. Run both before declaring any task done.
6. **Idiomatic Rust**: Use standard patterns (Result/Option, iterators, trait impls). No Java-style getters/setters, no unnecessary clones, no `.unwrap()` in library code.
7. **Dead code is deleted code**: No commented-out code blocks, no unused imports, no dead functions. If it's not used, it doesn't exist.

## Tech Stack
- Language: Rust
- Async: tokio
- P2P: rust-libp2p (QUIC + WebRTC)
- Consensus: Synaptic Consensus (SynBFT + PoUW)
- VM: wasmtime (WASM) + revm (EVM)
- Storage: redb (all node types -- ADR-001, pure Rust, no native deps)
- State: Verkle trees
- Crypto: BLAKE3, Ed25519, BLS12-381
- AI Runtime: tract (primary), candle (secondary)
- License: Dual MIT / Apache-2.0

## Reference Repos (in /references/)
Study these BEFORE writing implementation code for the corresponding domain:
- `references/mysticeti/` — DAG-BFT consensus (MystiCeti by MystenLabs). Key: `mysticeti-core/src/core.rs`, `types.rs`, `consensus/`
- `references/sui/` — Object model, parallel execution, DAG consensus. Key: `consensus/core/src/`, `crates/sui-core/src/`
- `references/lighthouse/` — Production Rust node architecture (Ethereum). Key: `consensus/src/`, `beacon_node/`
- `references/rust-libp2p/` — P2P networking patterns. Key: `examples/`, `protocols/gossipsub/src/`
- `references/redb/` — Storage engine internals. Key: `src/db.rs`, `src/transactions.rs`

## Critical Design Constraints (from MASTER_DESIGN.md)
These are non-negotiable. Violating any of these is a bug:
1. **Server-independence**: Every design must work if all centralized servers go offline
2. **DAG consensus**: Blocks have multiple parents (Vec<BlockHash>), NOT linear chain
3. **400ms target block time**, <1s finality, 10k+ TPS at launch
4. **Verkle trees** for state (NOT Merkle Patricia Tries) — but design binary Merkle + SNARK escape hatch
5. **Hybrid account + object model** (not pure account like Ethereum, not pure object like Sui)
6. **AI inference is OFF-CHAIN** with ON-CHAIN verification only
7. **Privacy is protocol-level** with selective disclosure (not full anonymity)
8. **nChain patent risk**: FTO analysis required before finalizing consensus design (1,308 patents)

## Cross-Skill Review Protocol
After completing any major component, run a security review:
1. Write code in the domain skill (e.g., `/consensus-engineer`)
2. Review it as `/security-engineer` — check for threat vectors, crypto misuse, DoS surfaces
3. If security flags are raised, they MUST be addressed before merging
4. For consensus/crypto code: also review as `/blockchain-architect` for architectural alignment

## Web Research Capability
The `/research-analyst` skill has access to `WebSearch` and `WebFetch` tools.
Use these to get current data before major design decisions. Example triggers:
- "Research latest DAG consensus developments" → WebSearch + summarize
- "Check rust-libp2p release status" → WebFetch the GitHub releases page
- "Compare our approach to [competitor]" → WebSearch for recent benchmarks

## Build-Phase Rules (MANDATORY -- enforced on every change)
These are hard requirements. Violating any of these blocks shipping.

1. **Traceability**: Every code change must have a BUILD_LOG entry with git ref, files changed, and review notes
2. **ADRs**: Non-obvious technical choices require an Architecture Decision Record in `blockchain-project/DECISIONS.md`
3. **BUILD_LOG**: Entry in `blockchain-project/BUILD_LOG.md` mandatory for every commit
4. **CHANGELOG**: Entry in `CHANGELOG.md` for architecture-level or user-facing changes
5. **STATUS.md**: Updated at every sprint boundary in `blockchain-project/STATUS.md`
6. **Sprint plans**: Written in `blockchain-project/sprints/SPRINT-XXX.md` BEFORE work begins
7. **Security flags**: Propagate immediately. Security-engineer flags are ELEVATED priority and BLOCK shipping until resolved

### Development Workflow
- Run `cargo check` after every code change (fast compilation check)
- Run `cargo test` after every change
- Run `cargo clippy` before committing (zero warnings policy)
- Run `cargo fmt --check` before committing (consistent formatting)
- Reference repos are for reading only -- never modify them
- All new code needs tests (minimum: happy path + one failure case)
- No `unsafe` blocks without security-engineer review and justification comment
- All dependencies must be pure Rust (no C/C++ build deps -- ADR-001)
- Security-engineer review mandatory before merging any code

## Session Handoff Protocol (MANDATORY -- enforced on EVERY new conversation)

This is a solo-developer project. Context WILL be lost between conversations.
This protocol ensures zero drift between sessions.

### When the user starts a new conversation (says "lead", "continue", "status", or similar):

Claude MUST do the following BEFORE any other work:

1. **Read these files silently** (do NOT dump their contents to the user):
   - `blockchain-project/STATUS.md`
   - `blockchain-project/BUILD_LOG.md` (last 5 entries)
   - Current sprint plan in `blockchain-project/sprints/`
   - `blockchain-project/DECISIONS.md` (last 3 ADRs)
   - `CHANGELOG.md` (last section)

2. **Run these checks silently**:
   - `git branch` (confirm on `dev`)
   - `git status` (check for uncommitted work)
   - `git log --oneline -3` (last 3 commits)
   - `cargo check --workspace` (does it compile?)

3. **Respond with a SHORT sync report** (max 15 lines):
   ```
   SYNC REPORT -- Dendrite Network
   Branch: dev (clean / dirty)
   Last commit: <hash> <message>
   Sprint: <number> -- <name>
   Phase: <current phase> | Task: <current/next task>
   Tests: <pass count> | Warnings: <count>
   Blockers: <none or list>
   Next action: <what to do next>
   ```

4. **Wait for the user's go-ahead** before doing any work.

### Why this exists:
- Single developer = no one else to catch drift
- Long conversations lose context = errors compound
- This 30-second check prevents hours of rework

## Doc Sync Rules (MANDATORY -- enforced on every task completion)

Documentation is NOT optional. It is part of the Definition of Done for EVERY task.

### After completing ANY task:
1. **BUILD_LOG.md**: Add entry with git ref, files changed, what was done
2. **Sprint plan**: Update task status from PENDING to DONE
3. **STATUS.md**: Update if a phase boundary was crossed
4. **CHANGELOG.md**: Add entry if the change is architecture-level or user-facing
5. **DECISIONS.md**: Add ADR if a non-obvious technical choice was made
6. **Code docs**: If a new public trait/type/function was added, ensure it has rustdoc comments

### After completing ANY phase:
1. All of the above, PLUS:
2. **STATUS.md**: Full update (crate depth table, completed list, next up)
3. **Sprint plan**: Mark phase exit criteria as met
4. **CHANGELOG.md**: Phase completion entry

### After completing ANY sprint:
1. All of the above, PLUS:
2. **Sprint retrospective**: Written in the sprint plan file
3. **STATUS.md**: Full refresh with metrics
4. **Next sprint plan**: Drafted in `blockchain-project/sprints/SPRINT-XXX.md`

### Enforcement:
- Claude MUST NOT say "task complete" until ALL doc updates are done
- If Claude forgets, the user should say "docs?" and Claude must immediately do the updates
- No commit is valid without its BUILD_LOG entry
