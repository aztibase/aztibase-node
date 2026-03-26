import Link from "next/link";

const FEATURES = [
  {
    title: "DAG Consensus",
    desc: "Synaptic Consensus — blocks reference multiple parents. 400ms block time, sub-second finality, 10K+ TPS.",
    icon: (
      <svg viewBox="0 0 32 32" fill="none" className="w-7 h-7">
        <circle cx="16" cy="6" r="3" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="7" cy="18" r="3" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="25" cy="18" r="3" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="16" cy="28" r="3" stroke="currentColor" strokeWidth="1.5" />
        <path d="M14 8.5L9 15.5M18 8.5L23 15.5M9 21L14 25.5M23 21L18 25.5" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      </svg>
    ),
    color: "text-aztb-accent",
    glow: "group-hover:shadow-[0_0_30px_rgba(124,106,173,0.12)]",
  },
  {
    title: "AI-Native",
    desc: "On-chain model registry, compute market, attestation protocol. Proof of Useful Work for AI inference.",
    icon: (
      <svg viewBox="0 0 32 32" fill="none" className="w-7 h-7">
        <rect x="6" y="8" width="20" height="16" rx="3" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="12" cy="16" r="2" stroke="currentColor" strokeWidth="1.2" />
        <circle cx="20" cy="16" r="2" stroke="currentColor" strokeWidth="1.2" />
        <path d="M14 16h4" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
        <path d="M12 8V5M20 8V5M16 24v3" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
      </svg>
    ),
    color: "text-aztb-cyan",
    glow: "group-hover:shadow-[0_0_30px_rgba(106,196,200,0.12)]",
  },
  {
    title: "Dual VM",
    desc: "WASM + EVM side by side, with a cross-VM bridge. Deploy Solidity or Rust contracts. Interop both directions.",
    icon: (
      <svg viewBox="0 0 32 32" fill="none" className="w-7 h-7">
        <rect x="3" y="10" width="11" height="12" rx="2" stroke="currentColor" strokeWidth="1.5" />
        <rect x="18" y="10" width="11" height="12" rx="2" stroke="currentColor" strokeWidth="1.5" />
        <path d="M14 16h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        <path d="M6 14h5M6 18h5M21 14h5M21 18h5" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.5" />
      </svg>
    ),
    color: "text-aztb-green",
    glow: "group-hover:shadow-[0_0_30px_rgba(93,184,138,0.12)]",
  },
  {
    title: "Server-Independent",
    desc: "No DNS, no API gateways. QUIC + WebRTC transport, Kademlia DHT, gossipsub. Fully decentralized.",
    icon: (
      <svg viewBox="0 0 32 32" fill="none" className="w-7 h-7">
        <circle cx="16" cy="16" r="11" stroke="currentColor" strokeWidth="1.5" />
        <circle cx="16" cy="16" r="3" stroke="currentColor" strokeWidth="1.2" />
        <path d="M16 5v8M16 19v8M5 16h8M19 16h8" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.4" />
        <circle cx="8" cy="8" r="1.5" fill="currentColor" opacity="0.3" />
        <circle cx="24" cy="8" r="1.5" fill="currentColor" opacity="0.3" />
        <circle cx="8" cy="24" r="1.5" fill="currentColor" opacity="0.3" />
        <circle cx="24" cy="24" r="1.5" fill="currentColor" opacity="0.3" />
      </svg>
    ),
    color: "text-aztb-400",
    glow: "group-hover:shadow-[0_0_30px_rgba(136,120,166,0.12)]",
  },
];

const STATS = [
  { value: "400ms", label: "Block Time", accent: "text-aztb-accent" },
  { value: "<1.6s", label: "Finality", accent: "text-aztb-cyan" },
  { value: "10K+", label: "TPS Target", accent: "text-aztb-green" },
  { value: "1B", label: "Fixed Supply", accent: "text-aztb-400" },
];

const TECH = [
  { label: "Language", value: "Rust", mono: true },
  { label: "Async", value: "Tokio", mono: true },
  { label: "P2P", value: "libp2p (QUIC + WebRTC)", mono: false },
  { label: "Consensus", value: "Synaptic (SynBFT + PoUW)", mono: false },
  { label: "VM", value: "wasmtime + revm", mono: true },
  { label: "Storage", value: "redb (pure Rust)", mono: false },
  { label: "State", value: "Verkle Trees", mono: false },
  { label: "Crypto", value: "BLAKE3 + Ed25519 + BLS", mono: false },
  { label: "AI Runtime", value: "tract (ONNX)", mono: true },
];

export default function Home() {
  return (
    <div className="relative">
      {/* Background effects */}
      <div className="absolute inset-0 hero-grid pointer-events-none" />
      <div className="absolute inset-0 hero-radial pointer-events-none" />
      <div className="absolute inset-0 hero-radial-cyan pointer-events-none" />

      <div className="relative max-w-5xl mx-auto px-4">
        {/* ── Hero ──────────────────────────────────── */}
        <section className="pt-24 pb-20 text-center">
          <div className="badge mb-6 animate-fade-in-up">
            <span className="w-1.5 h-1.5 rounded-full bg-aztb-green animate-pulse" />
            Testnet Live
          </div>

          <h1 className="text-4xl sm:text-5xl md:text-6xl font-heading font-bold text-aztb-200 mb-5 leading-[1.1] animate-fade-in-up delay-100">
            The Chain Where
            <br />
            <span className="text-gradient">AI Agents Live</span>
          </h1>

          <p className="text-base sm:text-lg text-aztb-400 max-w-2xl mx-auto mb-10 leading-relaxed animate-fade-in-up delay-200">
            AI-native Layer 1 blockchain built in Rust. Server-independent architecture,
            DAG consensus, dual VM execution, and protocol-level AI integration.
          </p>

          <div className="flex items-center justify-center gap-3 flex-wrap animate-fade-in-up delay-300">
            <Link href="/docs/quickstart" className="btn-primary px-7 py-3 text-sm">
              Get Started
            </Link>
            <Link href="/docs/run-validator" className="btn-secondary px-7 py-3 text-sm">
              Run a Validator
            </Link>
            <Link href="/litepaper" className="btn-ghost px-7 py-3 text-sm">
              Litepaper
            </Link>
          </div>
        </section>

        {/* ── Stats ─────────────────────────────────── */}
        <section className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-20 animate-fade-in-up delay-400">
          {STATS.map((s) => (
            <div key={s.label} className="card-glass p-5 text-center group hover:border-aztb-500/20 transition-all duration-300">
              <div className={`text-2xl md:text-3xl font-heading font-bold mb-1 ${s.accent}`}>
                {s.value}
              </div>
              <div className="text-[0.65rem] text-aztb-500 font-mono uppercase tracking-widest">
                {s.label}
              </div>
            </div>
          ))}
        </section>

        {/* ── Features ──────────────────────────────── */}
        <section className="mb-20">
          <div className="text-center mb-10">
            <h2 className="section-heading mb-3">Built Different</h2>
            <p className="text-sm text-aztb-400 max-w-lg mx-auto">
              Not another EVM fork. Every component — consensus, networking, execution, AI — written from scratch in Rust.
            </p>
          </div>

          <div className="grid md:grid-cols-2 gap-4">
            {FEATURES.map((f) => (
              <div key={f.title} className={`group card-glow p-6 ${f.glow}`}>
                <div className={`${f.color} mb-4`}>{f.icon}</div>
                <h3 className="text-lg font-heading font-bold text-aztb-200 mb-2">{f.title}</h3>
                <p className="text-sm text-aztb-400 leading-relaxed">{f.desc}</p>
              </div>
            ))}
          </div>
        </section>

        {/* ── Tech Stack ────────────────────────────── */}
        <section className="mb-20">
          <div className="card p-8">
            <div className="flex items-center gap-3 mb-6">
              <div className="w-8 h-8 rounded-lg bg-aztb-accent/10 border border-aztb-accent/20 flex items-center justify-center">
                <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4 text-aztb-accent">
                  <path d="M10 2L2 6v8l8 4 8-4V6l-8-4z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
                  <path d="M2 6l8 4m0 0l8-4m-8 4v8" stroke="currentColor" strokeWidth="1.2" opacity="0.5" />
                </svg>
              </div>
              <h2 className="section-heading">Tech Stack</h2>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-x-8 gap-y-3">
              {TECH.map((t) => (
                <div key={t.label} className="flex items-baseline gap-2 py-1.5 border-b border-aztb-500/5 last:border-0">
                  <span className="text-xs text-aztb-500 uppercase tracking-wider shrink-0 w-20">{t.label}</span>
                  <span className={`text-sm text-aztb-300 ${t.mono ? "font-mono text-xs" : ""}`}>{t.value}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ── Architecture Preview ──────────────────── */}
        <section className="mb-20">
          <div className="card-glass p-8 md:p-10">
            <h2 className="section-heading mb-2">9 Rust Crates. Zero C Dependencies.</h2>
            <p className="text-sm text-aztb-400 mb-8">
              Monorepo architecture with clean separation of concerns. Every crate is pure Rust.
            </p>

            <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-5 gap-3">
              {[
                { name: "consensus", desc: "DAG-BFT + PoUW", color: "border-aztb-accent/25 hover:border-aztb-accent/50" },
                { name: "execution", desc: "EVM + WASM + fees", color: "border-aztb-green/25 hover:border-aztb-green/50" },
                { name: "network", desc: "libp2p transport", color: "border-aztb-cyan/25 hover:border-aztb-cyan/50" },
                { name: "storage", desc: "redb + Verkle", color: "border-aztb-accent/25 hover:border-aztb-accent/50" },
                { name: "ai", desc: "ONNX + Sentinel", color: "border-aztb-cyan/25 hover:border-aztb-cyan/50" },
                { name: "node", desc: "Orchestrator", color: "border-aztb-500/25 hover:border-aztb-500/50" },
                { name: "rpc", desc: "57+ methods", color: "border-aztb-500/25 hover:border-aztb-500/50" },
                { name: "types", desc: "Shared primitives", color: "border-aztb-500/25 hover:border-aztb-500/50" },
                { name: "wasm", desc: "Light client", color: "border-aztb-500/25 hover:border-aztb-500/50" },
                { name: "1,040+", desc: "Tests passing", color: "border-aztb-green/25 hover:border-aztb-green/50" },
              ].map((c) => (
                <div
                  key={c.name}
                  className={`bg-aztb-950/60 border rounded-lg px-3 py-3 transition-all duration-300 ${c.color}`}
                >
                  <div className="text-sm font-mono font-semibold text-aztb-200">{c.name}</div>
                  <div className="text-[0.6rem] text-aztb-500 mt-0.5">{c.desc}</div>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ── CTA ───────────────────────────────────── */}
        <section className="text-center pb-20">
          <h2 className="section-heading mb-3">Start Building</h2>
          <p className="text-sm text-aztb-400 mb-8 max-w-md mx-auto">
            Build from source, request testnet tokens, and deploy your first contract.
          </p>
          <div className="flex items-center justify-center gap-3 flex-wrap">
            <Link href="/swap" className="btn-primary px-7 py-3 text-sm">
              Swap Tokens
            </Link>
            <Link href="/faucet" className="btn-secondary px-7 py-3 text-sm">
              Get Testnet AZTB
            </Link>
            <Link href="/docs/rpc" className="btn-ghost px-7 py-3 text-sm">
              RPC API Reference
            </Link>
          </div>
        </section>
      </div>
    </div>
  );
}
