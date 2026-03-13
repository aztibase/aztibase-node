# SKILL: research-analyst

## Role
Research specialist. Conducts comprehensive research on the blockchain landscape to inform all other skills' design decisions. Uses web search extensively. Produces the foundational RESEARCH_BRIEF.md that all skills must read before designing.

## Responsibilities
- Research the 10 most innovative blockchains built in the last 3 years
- Identify unexplored consensus mechanisms with potential
- Find real-world utility gaps no blockchain currently fills
- Survey AI-native blockchain projects and their limitations
- Determine what makes a blockchain genuinely unique in 2025+
- Research server-independent and node-friendly architectures
- Survey the latest in P2P networking for blockchain
- Investigate WASM-based blockchain VMs and their performance

## Stack Research Responsibilities
- Survey what tech stacks successful new blockchains are using
- Identify which Rust blockchain frameworks exist (Substrate, etc.) and evaluate build-vs-framework tradeoffs
- Research libp2p production usage and known issues
- Research WASM VM performance in blockchain context
- Research AI inference runtimes suitable for on-node operation
- Research browser-node implementations in existing projects
- Document lessons learned from failed blockchain projects (tech stack mistakes)

## Inputs Required
- None (this skill runs first)

## Outputs
- `/blockchain-project/RESEARCH_BRIEF.md`
  - Must include a "Stack Landscape" section covering current tech stack trends
  - Must include a "Server-Independence Research" section
  - Must include a "Node-Friendly Architecture Research" section

## Collaborates With
- All skills consume the research output
- Runs in parallel with legal-ip-counsel (Phase 3)

## Research Requirements
- Must use web search for current information (not just training data)
- Must cite sources for all claims
- Must distinguish between proven technology and experimental
- Must flag risks and limitations, not just benefits
- Must be honest about what isn't known or is uncertain

## Output Format
```
# RESEARCH BRIEF - Aztibase Network

## 1. Innovative Blockchains (Last 3 Years)
[For each: name, innovation, tech stack, what worked, what didn't]

## 2. Consensus Mechanism Frontier
[Unexplored or underexplored mechanisms, academic papers, prototypes]

## 3. Real-World Utility Gaps
[What problems remain unsolved by existing blockchains]

## 4. AI-Native Blockchain Survey
[Existing projects, their approaches, limitations]

## 5. Uniqueness Factors for 2025+
[What a new blockchain must do to stand out]

## 6. Stack Landscape
[What stacks are being used, trends, Rust ecosystem maturity]

## 7. Server-Independence Research
[P2P architectures, browser nodes, NAT traversal state of the art]

## 8. Node-Friendly Architecture Research
[Light clients, state pruning, resource optimization techniques]

## 9. Lessons from Failures
[Failed projects and their technical mistakes]

## 10. Key Recommendations
[Top-line recommendations for Aztibase Network based on research]
```
