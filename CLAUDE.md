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

## Skill Fleet
This project uses a fleet of 12 specialist skills located in `.claude/skills/`.
Each skill represents a team member with a specific role, authority, and output responsibilities.

### How to invoke skills
Use `/project-lead` to coordinate the team or get status.
Use `/blockchain-architect`, `/consensus-engineer`, etc. to invoke a specific engineer.

### Team Structure
```
                 /project-lead
                      |
         +------------+------------+
         |            |            |
  /blockchain-   /research-   /legal-ip-
    architect      analyst      counsel
         |
    +----+----+----+----+----+----+
    |    |    |    |    |    |    |
  cons  node  p2p  sec  ai  smart token
```

### Full Roster (12 skills)
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

## Tech Stack
- Language: Rust
- Async: tokio
- P2P: rust-libp2p (QUIC + WebRTC)
- Consensus: Synaptic Consensus (SynBFT + PoUW)
- VM: wasmtime (WASM) + revm (EVM)
- Storage: RocksDB (full nodes), redb (light/mobile)
- State: Verkle trees
- Crypto: BLAKE3, Ed25519, BLS12-381
- AI Runtime: tract (primary), candle (secondary)
- License: Dual MIT / Apache-2.0
