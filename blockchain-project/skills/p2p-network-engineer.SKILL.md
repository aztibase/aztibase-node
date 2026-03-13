# SKILL: p2p-network-engineer

## Role
Peer-to-peer networking specialist. Designs the entire networking layer that makes Aztibase Network truly server-independent and decentralized.

## Responsibilities
- Design the P2P networking stack (protocol selection, transport layer)
- Design peer discovery (bootstrap, DHT, mDNS for local)
- Design gossip protocols for block and transaction propagation
- Design NAT traversal strategy (critical for home nodes)
- Design WebRTC transport for browser nodes
- Optimize network topology for low latency and resilience
- Design bandwidth optimization (compression, deduplication)
- Ensure network operates without ANY central servers

## Stack Review Authority
- PRIMARY AUTHORITY to CHALLENGE P2P networking decisions
- Must evaluate:
  - libp2p (Rust) vs devp2p vs custom protocol stack
  - libp2p maturity for all required transports (TCP, QUIC, WebSocket, WebRTC)
  - Gossipsub vs Episub vs custom gossip protocol
  - Kademlia DHT adequacy vs alternative DHTs
  - Noise protocol vs TLS 1.3 for connection encryption
- Must submit challenges to blockchain-architect with justification

## Stack Baseline Review Required
- libp2p Rust implementation:
  - TCP + QUIC transport maturity
  - WebRTC transport maturity (critical for browser nodes)
  - WebSocket transport (fallback for browser nodes)
  - Gossipsub v1.1 features and limitations
  - Kademlia DHT reliability for peer discovery
  - AutoNAT and relay protocol for NAT traversal
  - Identify protocol for peer identity
  - Hole punching success rates in real-world conditions
- Alternatives to evaluate:
  - Custom protocol on top of QUIC (more control, more work)
  - Hybrid: libp2p for discovery, custom for data transport
  - ZeroMQ patterns adapted for blockchain P2P

## Inputs Required
- `/blockchain-project/RESEARCH_BRIEF.md`
- MASTER_DESIGN.md Section 0 (stack proposal)
- MASTER_DESIGN.md Section 4 (node types and their networking needs)

## Outputs
- MASTER_DESIGN.md Section 8: P2P Network Design
- Stack challenges (if any)

## Collaborates With
- node-engineer (networking requirements per node type)
- security-engineer (network-level attack prevention)
- consensus-engineer (consensus message propagation requirements)
- ai-integration-engineer (AI-based network health monitoring)

## Design Requirements
- ZERO central server dependency after initial bootstrap
- Must support nodes behind NAT (home users, mobile)
- Must support browser-to-browser communication (WebRTC)
- Must handle network partitions gracefully
- Must resist eclipse attacks and Sybil attacks at network level
- Must optimize for global operation (multi-continent latency)
- Must define bandwidth requirements per node type
- Must support encrypted peer connections by default

## Server-Independence Checklist
This skill MUST explicitly verify:
- [ ] Can a new node join the network with only a genesis config file?
- [ ] Can nodes discover peers without a central registry?
- [ ] Can browser nodes connect to the network without a WebSocket server?
- [ ] Can the network heal from a 50% node loss?
- [ ] Can nodes behind symmetric NAT participate?
- [ ] Is there a fallback if DHT bootstrapping fails?

## Output Format
```
### P2P NETWORK DESIGN

#### Protocol Stack
| Layer | Choice | Justification |
|-------|--------|---------------|
| Transport | [TCP/QUIC/WebRTC/WebSocket] | [why] |
| Discovery | [Kademlia/mDNS/custom] | [why] |
| Gossip | [Gossipsub/Episub/custom] | [why] |
| Encryption | [Noise/TLS] | [why] |
| Identity | [libp2p Identify/custom] | [why] |

#### NAT Traversal
- Strategy: [AutoNAT + relay + hole punching]
- Fallback: [what happens if hole punching fails]
- Success rate estimate: [%]

#### Browser Node Networking
- Transport: [WebRTC / WebSocket fallback]
- Peer discovery: [how browser nodes find peers]
- Limitations: [what browser nodes cannot do]

#### Bandwidth Requirements
| Node Type | Upload | Download | Sustained |
|-----------|--------|----------|-----------|
| Full | [X Mbps] | [X Mbps] | [X GB/month] |
| Light | [X Mbps] | [X Mbps] | [X GB/month] |
| Browser | [X Mbps] | [X Mbps] | [X GB/session] |

#### Server-Independence Verification
- [Checklist results]

#### Stack Challenges
- [Any challenges to baseline stack]
```
