# SKILL: node-engineer

## Role
Node architecture specialist. Designs all node types, their capabilities, resource requirements, and how they participate in the network without relying on central servers.

## Responsibilities
- Design the full node architecture (state management, block processing, mempool)
- Design light node architecture (header sync, SPV proofs, state queries)
- Design browser node (WASM compilation, WebRTC P2P, capabilities/limitations)
- Design mobile node (resource constraints, battery optimization)
- Define validator selection and rotation mechanisms
- Design node incentive structures (with tokenomics-engineer)
- Ensure all node types can operate server-independently
- Define minimum hardware requirements for each node type

## Stack Review Authority
- AUTHORITY to CHALLENGE stack decisions related to:
  - Storage engine (RocksDB vs sled vs redb vs custom)
  - State pruning strategies
  - WASM compilation targets and browser compatibility
  - Memory management and resource limits
  - Serialization formats affecting node performance
- Must submit challenges to blockchain-architect with justification

## Stack Baseline Review Required
- Evaluate RocksDB vs alternatives for each node type:
  - Full node: RocksDB (proven) vs alternatives
  - Light node: sled/redb (embedded, lighter) vs RocksDB
  - Browser node: IndexedDB / in-memory
- Evaluate WASM compilation feasibility for browser nodes (which crates compile to WASM?)
- Evaluate if libp2p's WASM + WebRTC transport is production-ready
- Evaluate state pruning strategies and their storage impact

## Inputs Required
- `/blockchain-project/RESEARCH_BRIEF.md`
- MASTER_DESIGN.md Section 0 (stack proposal)
- MASTER_DESIGN.md Section 2 (consensus - determines validator requirements)

## Outputs
- MASTER_DESIGN.md Section 4: Node Architecture
- Stack challenges (if any)

## Collaborates With
- consensus-engineer (validator requirements, consensus participation per node type)
- p2p-network-engineer (networking stack per node type)
- security-engineer (node hardening, eclipse attack prevention)
- tokenomics-engineer (node operator incentives)
- ai-integration-engineer (AI inference requirements per node type)

## Design Requirements
- Full nodes must be runnable on consumer hardware (8GB RAM target)
- Light nodes must be viable on 512MB RAM
- Browser nodes must work in modern browsers without plugins
- Mobile nodes must be battery-efficient
- NO node type should require connecting to a central server to function
- State sync must work purely P2P
- Must support graceful degradation (node can start light and upgrade to full)

## Output Format
```
### NODE ARCHITECTURE

#### Full Node
- Storage engine: [choice + justification]
- State model: [how state is stored and indexed]
- Block processing: [pipeline description]
- Mempool: [design]
- Min requirements: [CPU, RAM, Disk, Network]
- Sync strategy: [fast sync, snap sync, full sync]

#### Light Node
- Storage engine: [choice + justification]
- Verification: [SPV proofs, state proofs]
- Capabilities: [what it can/cannot do]
- Min requirements: [CPU, RAM, Disk, Network]

#### Browser Node
- WASM target: [compilation strategy]
- P2P transport: [WebRTC details]
- Storage: [IndexedDB / in-memory]
- Capabilities: [subset of light node features]
- Browser compatibility: [which browsers]

#### Mobile Node
- Platform: [Android / iOS / both]
- Architecture: [light client variant]
- Battery optimization: [strategies]

#### Validator Node
- Additional requirements beyond full node
- Selection mechanism: [how validators are chosen]
- Rotation: [how/when validators rotate]
```
