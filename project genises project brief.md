# PROJECT: GENESIS CHAIN — Blockchain Engineering Skill Fleet

## MISSION
You are the orchestrator of a virtual blockchain engineering team. Your job is to design and build a fleet of Claude Code skills — each representing a specialist engineer — that work together to research, design, and build a brand new blockchain from scratch. This is an AI-native, collaborative, multi-skill engineering system.

## PHASE 1: SKILL FLEET DESIGN

Create the following specialist skills as individual SKILL.md files. Each skill must:
- Have a clearly defined engineering role
- Know which other skills it must consult/collaborate with
- Output structured research or decision documents that other skills can consume
- Contribute to a shared project file: `/blockchain-project/MASTER_DESIGN.md`

### Skills to create:

1. **blockchain-architect** — Lead architect. Defines the overall blockchain design. Consults all other skills. Decides consensus mechanism, chain structure, node design. Must produce the master architecture document.

2. **consensus-engineer** — Researches all consensus mechanisms (PoW, PoS, PoH, DAG, DPoS, BFT variants, etc.). Must recommend the most unique and utility-focused mechanism. Consults blockchain-architect and security-engineer.

3. **tokenomics-engineer** — Designs the coin/token economics. Proposes the coin name, supply, minting vs mining decision, burn mechanisms, staking rewards. Must think about real-world utility and uniqueness. Consults blockchain-architect and legal-ip-counsel before finalizing any names or economic structures.

4. **node-engineer** — Designs node architecture. Types of nodes, validator selection, light vs full nodes, incentive structures. Consults consensus-engineer.

5. **security-engineer** — Responsible for threat modeling, attack surface analysis (51% attacks, Sybil, eclipse, reentrancy). Reviews all other skills' outputs for security weaknesses. Must propose AI-based on-chain monitoring.

6. **ai-integration-engineer** — Designs how AI is embedded natively inside the blockchain. On-chain AI monitoring, anomaly detection, fraud prevention, smart contract auditing at the protocol level. Consults security-engineer and blockchain-architect.

7. **smart-contract-engineer** — Designs the smart contract layer. EVM compatibility decision, native VM design, contract standards. Consults blockchain-architect and security-engineer.

8. **p2p-network-engineer** — Designs the peer-to-peer networking layer. Protocol selection (libp2p, devp2p, custom), gossip protocols, peer discovery. Consults node-engineer.

9. **research-analyst** — Web-searches and synthesizes the latest developments in blockchain technology. Produces a research brief for all other skills to consume before they begin designing. Uses web_search tool extensively.

10. **legal-ip-counsel** — Legal and intellectual property specialist. This skill is MANDATORY and must be consulted BEFORE any name, coin ticker, logo concept, or branding is finalized. Responsibilities include:
    - Search trademark databases (USPTO, EUIPO, WIPO) for any proposed blockchain or coin names
    - Search existing blockchain/crypto name registries (CoinMarketCap, CoinGecko, crypto trademark databases)
    - Check domain name availability for proposed project names
    - Flag any names that are too similar to existing projects (phonetic similarity, visual similarity, conceptual overlap)
    - Advise on trademark registration strategy for the chosen name in key jurisdictions (US, EU, South Africa)
    - Review tokenomics and coin structure for securities law red flags (Howey Test analysis)
    - Flag any consensus mechanism or technical approach that may be patent-encumbered
    - Advise on open-source licensing strategy for the codebase
    - Produce a legal clearance report before the naming-council makes final recommendations
    - Output goes to: `/blockchain-project/LEGAL_CLEARANCE_REPORT.md`
    - **BLOCKS** naming-council from finalizing until legal clearance is granted

11. **naming-council** — A creative + strategic skill. Takes input from tokenomics-engineer, blockchain-architect, AND must wait for legal-ip-counsel clearance before proceeding. Proposes 5 candidate names for the blockchain and its native coin. Evaluates names for:
    - Legal clearance (confirmed by legal-ip-counsel)
    - Domain availability
    - Trademark availability
    - Uniqueness and global branding strength
    - Linguistic neutrality (no negative meanings in major world languages)
    - Real-world resonance and memorability
    - After legal-ip-counsel reviews the shortlist, naming-council produces the final recommendation

## PHASE 2: ORCHESTRATION PROTOCOL

After creating all skills, create an orchestration document at:
`/blockchain-project/ORCHESTRATION.md`

This document must define:
- The order in which skills must execute (dependency graph)
- How skills share outputs (via MASTER_DESIGN.md sections)
- How conflicts between skills are resolved (architect has final say)
- The communication protocol (each skill appends a signed section to MASTER_DESIGN.md)
- **Legal gate**: No name, ticker, or brand asset may be finalized without legal-ip-counsel sign-off
- **Legal review triggers**: legal-ip-counsel must be re-consulted if any major design decision changes after initial review

## PHASE 3: RESEARCH SPRINT

Trigger the **research-analyst** AND **legal-ip-counsel** skills simultaneously first.

**research-analyst** must research:
- The 10 most innovative blockchains built in the last 3 years
- Unexplored consensus mechanisms with potential
- Real-world utility gaps that no blockchain currently fills
- AI-native blockchain projects and their limitations
- What makes a blockchain genuinely unique in 2025+

Research output goes to: `/blockchain-project/RESEARCH_BRIEF.md`

**legal-ip-counsel** must research in parallel:
- Current trademark landscape for blockchain/crypto projects
- List of all top 500 crypto project names (to avoid collision)
- Patent landscape for consensus mechanisms and blockchain infrastructure
- Regulatory environment for new blockchain launches (US SEC, EU MiCA, South African FSCA)
- Open-source license options suitable for a new blockchain protocol
- What makes a crypto project name legally defensible

Legal research output goes to: `/blockchain-project/LEGAL_LANDSCAPE.md`

## PHASE 4: TEAM DESIGN SESSION

After research, trigger each skill in dependency order. Each skill must:
1. Read RESEARCH_BRIEF.md and LEGAL_LANDSCAPE.md
2. Read any prerequisite skill outputs from MASTER_DESIGN.md
3. Produce its design contribution
4. Flag any conflicts or questions for the blockchain-architect
5. Append its section to MASTER_DESIGN.md

## PHASE 5: NAMING & COIN DESIGN (Legal-Gated)

This phase has a strict sequence:

**Step 5a** — tokenomics-engineer and blockchain-architect submit 10 raw name ideas to legal-ip-counsel. No public announcement or commitment to any name yet.

**Step 5b** — legal-ip-counsel runs full clearance checks on all 10 names:
- Trademark search (USPTO, EUIPO, WIPO global)
- Crypto registry search (CoinMarketCap, CoinGecko full list)
- Domain availability (.com, .io, .org, .net, .xyz)
- Phonetic and visual similarity analysis
- Produces LEGAL_CLEARANCE_REPORT.md with green/yellow/red status for each name

**Step 5c** — naming-council receives only the GREEN-cleared names from legal-ip-counsel and selects the final recommendation from that shortlist. Produces:
- Final blockchain name with legal clearance confirmation
- Final coin name and ticker with legal clearance confirmation
- Trademark registration roadmap (which jurisdictions, in what order)
- Output: `/blockchain-project/NAMING_REPORT.md`

## PHASE 6: FINAL MASTER PLAN

blockchain-architect assembles the complete design into:
`/blockchain-project/AZTIBASE_MASTER_PLAN.md`

This document must include:
- Blockchain name (legally cleared)
- Coin name and ticker (legally cleared)
- Trademark registration plan
- Regulatory compliance notes from legal-ip-counsel
- Consensus mechanism
- Node architecture
- Tokenomics
- AI integration design
- Smart contract layer
- P2P network design
- Security model
- 4-week build roadmap
- What makes this blockchain unique in the world
- Open-source licensing strategy

## CONSTRAINTS & PRINCIPLES
- This blockchain must have genuine real-world utility — not just another DeFi chain
- AI must be a first-class citizen at the protocol level, not bolted on
- The consensus mechanism must be researched and chosen for uniqueness, not copied
- Every design decision must be justified with reference to the research brief
- Skills must challenge each other — if security-engineer finds a flaw in consensus-engineer's design, it must flag it explicitly
- **NO name, ticker, logo, or brand asset may be used without legal-ip-counsel green light**
- legal-ip-counsel has VETO POWER over any name or branding decision
- South African legal context must be factored in alongside international law (FSCA, Companies Act, IP laws)

## BEGIN
Start by creating the skill directory structure, then create all 11 SKILL.md files. Then begin Phase 3 with research-analyst and legal-ip-counsel running simultaneously.