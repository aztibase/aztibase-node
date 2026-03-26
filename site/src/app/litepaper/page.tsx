import type { Metadata } from "next";
import Link from "next/link";

export const metadata: Metadata = { title: "Litepaper" };

const Section = ({ title, children }: { title: string; children: React.ReactNode }) => (
  <section className="mb-12">
    <h2 className="text-xl font-heading font-bold text-aztb-200 mb-4 pb-2 border-b border-aztb-500/10">
      {title}
    </h2>
    {children}
  </section>
);

const Table = ({ headers, rows }: { headers: string[]; rows: string[][] }) => (
  <div className="overflow-x-auto mb-4">
    <table className="w-full text-sm">
      <thead>
        <tr className="border-b border-aztb-500/15">
          {headers.map((h) => (
            <th key={h} className="text-left py-2 px-3 text-aztb-400 font-semibold text-xs uppercase tracking-wider">{h}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {rows.map((row, i) => (
          <tr key={i} className="border-b border-aztb-500/5">
            {row.map((cell, j) => (
              <td key={j} className="py-2 px-3 text-aztb-300">{cell}</td>
            ))}
          </tr>
        ))}
      </tbody>
    </table>
  </div>
);

export default function LitepaperPage() {
  return (
    <div className="max-w-3xl mx-auto px-4 py-12">
      {/* Header */}
      <div className="text-center mb-12">
        <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-aztb-accent/10 border border-aztb-accent/20 text-aztb-accent text-xs font-mono mb-4">
          Version 1.0 — March 2026
        </div>
        <h1 className="text-3xl md:text-4xl font-heading font-bold text-aztb-200 mb-3">
          Aztibase Network
        </h1>
        <p className="text-lg text-aztb-400 mb-6">
          The Chain Where AI Agents Live
        </p>
        <div className="flex justify-center gap-3">
          <a
            href="/litepaper.pdf"
            download="Aztibase-Litepaper-v1.0.pdf"
            className="btn-secondary text-sm px-5 py-2"
          >
            Download PDF
          </a>
          <Link href="/docs" className="btn-secondary text-sm px-5 py-2">
            Read the Docs
          </Link>
        </div>
      </div>

      {/* Content */}
      <div className="card p-8 md:p-10">
        <Section title="The Problem">
          <p className="text-aztb-300 leading-relaxed mb-4">
            AI agents need a blockchain that understands them. Current chains treat AI as an afterthought — inference happens off-chain, verification is impossible, and 12-second block times make per-inference micropayments impractical.
          </p>
          <p className="text-aztb-300 leading-relaxed">
            Aztibase is the first Layer 1 blockchain built from the ground up for AI agent economies.
          </p>
        </Section>

        <Section title="What Makes Aztibase Different">
          <div className="space-y-6">
            <div>
              <h3 className="text-base font-semibold text-aztb-200 mb-2">AI-Native at the Protocol Level</h3>
              <p className="text-sm text-aztb-300 mb-3">
                Every Aztibase node runs an ONNX inference engine. AI isn&apos;t a smart contract bolted on top — it&apos;s part of the node software itself.
              </p>
              <ul className="space-y-1.5 text-sm text-aztb-300">
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">Model Registry</strong>: Register AI models on-chain with metadata, pricing, and version tracking</span></li>
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">Inference Execution</strong>: Nodes execute AI inference tasks and produce verifiable results</span></li>
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">AI Sentinel</strong>: Autonomous health monitor using ML to detect anomalies and take corrective action</span></li>
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">Agent Identity</strong>: On-chain accounts for autonomous AI agents with capability policies</span></li>
              </ul>
            </div>

            <div>
              <h3 className="text-base font-semibold text-aztb-200 mb-2">Dual Virtual Machine</h3>
              <p className="text-sm text-aztb-300 mb-3">
                EVM and WASM smart contracts side by side, with a cross-VM bridge enabling calls between them.
              </p>
              <ul className="space-y-1.5 text-sm text-aztb-300">
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">EVM (revm)</strong>: Deploy Solidity contracts. Compatible with existing Ethereum tooling</span></li>
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">WASM (wasmtime)</strong>: Deploy Rust, C, or AssemblyScript for maximum performance</span></li>
                <li className="flex gap-2"><span className="text-aztb-accent shrink-0">—</span> <span><strong className="text-aztb-200">Cross-VM Bridge</strong>: Bidirectional calls between WASM and EVM contracts</span></li>
              </ul>
            </div>

            <div>
              <h3 className="text-base font-semibold text-aztb-200 mb-2">DAG-Based Consensus</h3>
              <p className="text-sm text-aztb-300 mb-3">
                Directed Acyclic Graph where multiple blocks are proposed simultaneously — no leader bottleneck.
              </p>
              <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                {[
                  ["400ms", "Block Time"],
                  ["<1.6s", "Finality"],
                  ["10K+", "TPS Target"],
                  ["f/3", "Fault Tolerance"],
                ].map(([val, label]) => (
                  <div key={label} className="bg-aztb-950 rounded-lg p-3 text-center">
                    <div className="text-lg font-heading font-bold text-aztb-200">{val}</div>
                    <div className="text-[0.6rem] text-aztb-500 font-mono uppercase">{label}</div>
                  </div>
                ))}
              </div>
            </div>

            <div>
              <h3 className="text-base font-semibold text-aztb-200 mb-2">Server Independence</h3>
              <p className="text-sm text-aztb-300">
                Pure P2P via libp2p — Kademlia DHT, gossipsub, QUIC + WebRTC. No DNS, no API gateways, no bootstrap server dependency. Browser nodes run via WASM + WebRTC.
              </p>
            </div>
          </div>
        </Section>

        <Section title="Architecture">
          <pre className="bg-aztb-950 border border-aztb-500/10 rounded-lg p-4 text-xs font-mono text-aztb-400 overflow-x-auto mb-6 leading-relaxed">{`                    Aztibase Node
    ┌──────────────────────────────────────┐
    │  Consensus (DAG-BFT)                 │
    │  ├── Vertex proposal + voting        │
    │  ├── Finality certificates           │
    │  └── Equivocation detection          │
    │                                      │
    │  Execution Pipeline                  │
    │  ├── 27+ transaction types           │
    │  ├── EVM (revm) + WASM (wasmtime)    │
    │  ├── Block-STM parallel execution    │
    │  └── EIP-1559 dynamic gas pricing    │
    │                                      │
    │  Storage (redb, pure Rust)           │
    │  ├── Verkle tree state commitments   │
    │  └── Account + contract storage      │
    │                                      │
    │  Networking (libp2p)                 │
    │  ├── QUIC + WebRTC transport         │
    │  ├── Gossipsub propagation           │
    │  └── Pipelined block sync            │
    │                                      │
    │  AI Runtime (tract / ONNX)           │
    │  ├── Model registry + marketplace    │
    │  ├── Anomaly detection scorer        │
    │  └── Sentinel autonomous monitor     │
    │                                      │
    │  JSON-RPC + WebSocket API            │
    │  └── 57+ methods, Prometheus metrics │
    └──────────────────────────────────────┘`}</pre>

          <h3 className="text-base font-semibold text-aztb-200 mb-3">What&apos;s Built and Running</h3>
          <Table
            headers={["Component", "Status", "Details"]}
            rows={[
              ["DAG Consensus", "Live", "Sub-second finality on public testnet"],
              ["EVM Contracts", "Live", "Deploy, call, read via aztb_call"],
              ["WASM Contracts", "Live", "Full deploy/call with fuel metering"],
              ["Cross-VM Bridge", "Live", "Bidirectional EVM↔WASM calls"],
              ["AI Sentinel", "Live", "3-tier autonomous action engine"],
              ["DEX (AMM)", "Live", "WASZTB/tUSDC pair with liquidity"],
              ["TypeScript SDK", "Published", "@aztibase/sdk"],
              ["Chrome Wallet", "Published", "Extension with staking UI"],
              ["Explorer + Indexer", "Live", "SQLite-backed, REST API"],
            ]}
          />
        </Section>

        <Section title="Tokenomics">
          <h3 className="text-base font-semibold text-aztb-200 mb-3">AZTB Token</h3>
          <Table
            headers={["Parameter", "Value"]}
            rows={[
              ["Total Supply", "1,000,000,000 AZTB"],
              ["Genesis Mint", "400,000,000 (40%)"],
              ["Emission Pool", "600,000,000 (60%) over ~10 years"],
              ["Block Time", "400ms"],
              ["Consensus", "DAG-BFT (Synaptic Consensus)"],
            ]}
          />

          <h3 className="text-base font-semibold text-aztb-200 mb-3 mt-6">Genesis Allocation (400M)</h3>
          <Table
            headers={["Category", "Amount", "Vesting"]}
            rows={[
              ["Protocol Treasury", "100M (25%)", "Immediate, governance-controlled"],
              ["Ecosystem Development", "80M (20%)", "6-month cliff, 4-year linear"],
              ["Core Team", "60M (15%)", "12-month cliff, 4-year linear"],
              ["Foundation Reserve", "40M (10%)", "2-year lock, 3-year linear"],
              ["Community Airdrop", "40M (10%)", "Immediate"],
              ["Validator Bootstrap", "40M (10%)", "6-month linear"],
              ["AI Ecosystem Fund", "20M (5%)", "6-month cliff, 3-year linear"],
              ["Liquidity Provision", "20M (5%)", "Immediate"],
            ]}
          />

          <h3 className="text-base font-semibold text-aztb-200 mb-3 mt-6">Emission Distribution</h3>
          <Table
            headers={["Recipient", "Share"]}
            rows={[
              ["Validator Rewards", "70%"],
              ["AI Compute (PoUW)", "15%"],
              ["Protocol Treasury", "10%"],
              ["Staking Insurance", "5%"],
            ]}
          />

          <h3 className="text-base font-semibold text-aztb-200 mb-3 mt-6">Staking Parameters</h3>
          <Table
            headers={["Parameter", "Value"]}
            rows={[
              ["Minimum Stake", "50,000 AZTB"],
              ["Maximum Stake", "50,000,000 AZTB (5% of supply)"],
              ["Unbonding Period", "21 days"],
              ["Target APY", "3–12%"],
              ["Equivocation Slash", "10%"],
              ["Downtime Slash", "0.5%"],
            ]}
          />
        </Section>

        <Section title="AI Integration">
          <div className="space-y-4">
            <div className="bg-aztb-950 rounded-lg p-4">
              <h4 className="text-sm font-semibold text-aztb-200 mb-1">Layer 1 — Anomaly Detection</h4>
              <p className="text-xs text-aztb-400">Every transaction is scored by an ONNX autoencoder for anomalous behavior. High scores flag potential attacks.</p>
            </div>
            <div className="bg-aztb-950 rounded-lg p-4">
              <h4 className="text-sm font-semibold text-aztb-200 mb-1">Layer 2 — Sentinel Health Monitor</h4>
              <p className="text-xs text-aztb-400">Aggregates network metrics and produces a chain health score. Three tiers: observe → score → act autonomously.</p>
            </div>
            <div className="bg-aztb-950 rounded-lg p-4">
              <h4 className="text-sm font-semibold text-aztb-200 mb-1">Layer 3 — Compute Marketplace</h4>
              <p className="text-xs text-aztb-400">Model creators register on-chain. Users post inference tasks. Compute providers execute and submit attestations. Chain verifies results.</p>
            </div>
          </div>
        </Section>

        <Section title="Technology">
          <Table
            headers={["Component", "Choice", "Why"]}
            rows={[
              ["Language", "Rust", "No GC, memory safety, WASM target"],
              ["Async", "Tokio", "Production-grade async I/O"],
              ["P2P", "libp2p", "NAT traversal, QUIC, WebRTC"],
              ["EVM", "revm", "Production Ethereum execution"],
              ["WASM VM", "wasmtime", "Fastest WASM runtime"],
              ["Storage", "redb", "Pure Rust, ACID, no C deps"],
              ["State", "Verkle trees", "Constant-size proofs"],
              ["Hashing", "BLAKE3", "Fastest cryptographic hash"],
              ["Signatures", "Ed25519", "Industry standard"],
              ["AI Runtime", "tract (ONNX)", "Pure Rust inference"],
              ["Serialization", "postcard", "Compact binary, no-std"],
            ]}
          />
          <div className="bg-aztb-950 rounded-lg p-4 mt-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-center">
              {[
                ["9", "Rust Crates"],
                ["1,040+", "Tests"],
                ["34", "ADRs"],
                ["0", "Clippy Warnings"],
              ].map(([val, label]) => (
                <div key={label}>
                  <div className="text-lg font-heading font-bold text-aztb-200">{val}</div>
                  <div className="text-[0.6rem] text-aztb-500 font-mono uppercase">{label}</div>
                </div>
              ))}
            </div>
          </div>
        </Section>

        <Section title="Roadmap">
          <div className="space-y-4">
            <div>
              <h3 className="text-sm font-semibold text-aztb-green mb-2">Completed</h3>
              <ul className="text-sm text-aztb-300 space-y-1">
                {[
                  "Layer 1 blockchain (consensus, execution, networking, storage)",
                  "Public testnet with monitoring (Grafana + VictoriaMetrics)",
                  "Dual VM (EVM + WASM) with cross-VM bridge",
                  "AI Sentinel with ONNX inference",
                  "On-chain governance (proposals, voting, parameter changes)",
                  "L2 bridge primitives",
                  "AMM DEX with liquidity pools and oracle",
                  "TypeScript SDK, Chrome wallet extension",
                  "Block explorer and indexer",
                ].map((item) => (
                  <li key={item} className="flex gap-2"><span className="text-aztb-green">✓</span>{item}</li>
                ))}
              </ul>
            </div>
            <div>
              <h3 className="text-sm font-semibold text-aztb-accent mb-2">Next</h3>
              <ul className="text-sm text-aztb-300 space-y-1">
                {[
                  "Security audit",
                  "Mainnet launch with 100+ validators",
                  "AI Model Marketplace",
                  "Agent Identity Registry (on-chain DID for AI agents)",
                  "Content Provenance Registry (C2PA-style)",
                  "Name Service (.aztb domains)",
                  "Community Task Chain (L2 for AI compute)",
                ].map((item) => (
                  <li key={item} className="flex gap-2"><span className="text-aztb-accent">→</span>{item}</li>
                ))}
              </ul>
            </div>
          </div>
        </Section>

        <Section title="How to Participate">
          <div className="grid md:grid-cols-2 gap-4">
            <div className="bg-aztb-950 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-aztb-200 mb-2">Run a Validator</h3>
              <p className="text-xs text-aztb-400 mb-3">
                Download the binary, stake AZTB, start earning rewards. Minimum hardware: 2 vCPU, 4 GB RAM (~$4/month).
              </p>
              <Link href="/docs/run-validator" className="text-xs text-aztb-accent hover:underline">
                Validator Guide →
              </Link>
            </div>
            <div className="bg-aztb-950 rounded-lg p-4">
              <h3 className="text-sm font-semibold text-aztb-200 mb-2">Build on Aztibase</h3>
              <p className="text-xs text-aztb-400 mb-3">
                Deploy EVM (Solidity) or WASM (Rust) contracts. Full TypeScript SDK available.
              </p>
              <Link href="/docs/quickstart" className="text-xs text-aztb-accent hover:underline">
                Quick Start →
              </Link>
            </div>
          </div>
          <div className="mt-6 text-center space-y-2 text-sm">
            <div className="text-aztb-400">
              <a href="https://aztibase.com" className="text-aztb-accent hover:underline">aztibase.com</a>
              {" · "}
              <a href="https://x.com/aztibase" className="text-aztb-accent hover:underline">@aztibase</a>
              {" · "}
              <a href="https://github.com/aztibase/aztibase-node" className="text-aztb-accent hover:underline">GitHub</a>
            </div>
          </div>
        </Section>

        <div className="text-center pt-4 border-t border-aztb-500/10">
          <p className="text-sm text-aztb-400 mb-1">
            Built by a solo engineer in South Africa. Every line — consensus, networking, execution, AI, wallet, explorer, SDK — written from scratch in Rust.
          </p>
          <p className="text-xs text-aztb-500">
            Aztibase (Pty) Ltd — Registered with the South African CIPC
          </p>
          <p className="text-xs text-aztb-500 mt-1">MIT / Apache-2.0 dual license</p>
        </div>
      </div>
    </div>
  );
}
