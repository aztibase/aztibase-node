#!/usr/bin/env python3
"""Build Aztibase Litepaper PDF with dark theme and professional layout."""

from reportlab.lib.pagesizes import A4
from reportlab.lib.units import mm, cm
from reportlab.lib.colors import HexColor, white, Color
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, KeepTogether, HRFlowable,
)
from reportlab.pdfgen import canvas
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
import os

# Colors
BG_DARK = HexColor("#0e0e16")
BG_CARD = HexColor("#16161f")
ACCENT = HexColor("#8878a6")
ACCENT_LIGHT = HexColor("#a898c6")
TEXT_PRIMARY = HexColor("#e0dce8")
TEXT_SECONDARY = HexColor("#9990aa")
TEXT_MUTED = HexColor("#666080")
GREEN = HexColor("#4ade80")
WHITE = white

W, H = A4

OUTPUT = os.path.join(os.path.dirname(__file__), "Aztibase-Litepaper-v1.0.pdf")


class DarkPageTemplate:
    def __init__(self, doc):
        self.doc = doc
        self.page_num = 0

    def on_page(self, canvas, doc):
        self.page_num += 1
        canvas.saveState()
        canvas.setFillColor(BG_DARK)
        canvas.rect(0, 0, W, H, fill=1, stroke=0)

        if self.page_num > 1:
            canvas.setFillColor(TEXT_MUTED)
            canvas.setFont("Helvetica", 7)
            canvas.drawString(doc.leftMargin, 15 * mm, "Aztibase Network — Litepaper v1.0")
            canvas.drawRightString(W - doc.rightMargin, 15 * mm, f"Page {self.page_num}")
            canvas.setStrokeColor(HexColor("#2a2a3a"))
            canvas.setLineWidth(0.5)
            canvas.line(doc.leftMargin, 20 * mm, W - doc.rightMargin, 20 * mm)
        canvas.restoreState()


def build_styles():
    s = {}
    s["title"] = ParagraphStyle(
        "Title", fontName="Helvetica-Bold", fontSize=36, leading=42,
        textColor=WHITE, alignment=TA_CENTER, spaceAfter=6 * mm,
    )
    s["subtitle"] = ParagraphStyle(
        "Subtitle", fontName="Helvetica", fontSize=16, leading=22,
        textColor=ACCENT_LIGHT, alignment=TA_CENTER, spaceAfter=4 * mm,
    )
    s["version"] = ParagraphStyle(
        "Version", fontName="Helvetica", fontSize=10, leading=14,
        textColor=TEXT_MUTED, alignment=TA_CENTER, spaceAfter=2 * mm,
    )
    s["h1"] = ParagraphStyle(
        "H1", fontName="Helvetica-Bold", fontSize=22, leading=28,
        textColor=WHITE, spaceBefore=10 * mm, spaceAfter=5 * mm,
    )
    s["h2"] = ParagraphStyle(
        "H2", fontName="Helvetica-Bold", fontSize=15, leading=20,
        textColor=ACCENT_LIGHT, spaceBefore=7 * mm, spaceAfter=3 * mm,
    )
    s["h3"] = ParagraphStyle(
        "H3", fontName="Helvetica-Bold", fontSize=12, leading=16,
        textColor=TEXT_PRIMARY, spaceBefore=5 * mm, spaceAfter=2 * mm,
    )
    s["body"] = ParagraphStyle(
        "Body", fontName="Helvetica", fontSize=10, leading=15,
        textColor=TEXT_PRIMARY, alignment=TA_JUSTIFY, spaceAfter=3 * mm,
    )
    s["bullet"] = ParagraphStyle(
        "Bullet", fontName="Helvetica", fontSize=10, leading=15,
        textColor=TEXT_PRIMARY, leftIndent=12 * mm, bulletIndent=5 * mm,
        spaceAfter=1.5 * mm,
    )
    s["code"] = ParagraphStyle(
        "Code", fontName="Courier", fontSize=7.5, leading=10.5,
        textColor=ACCENT_LIGHT, leftIndent=5 * mm, spaceAfter=3 * mm,
        backColor=BG_CARD,
    )
    s["tagline"] = ParagraphStyle(
        "Tagline", fontName="Helvetica-Bold", fontSize=13, leading=18,
        textColor=ACCENT, alignment=TA_CENTER, spaceBefore=8 * mm,
    )
    return s


def accent_line():
    return HRFlowable(width="100%", thickness=1, color=ACCENT, spaceAfter=4 * mm, spaceBefore=2 * mm)


def table_style():
    return TableStyle([
        ("BACKGROUND", (0, 0), (-1, 0), ACCENT),
        ("TEXTCOLOR", (0, 0), (-1, 0), WHITE),
        ("FONTNAME", (0, 0), (-1, 0), "Helvetica-Bold"),
        ("FONTSIZE", (0, 0), (-1, 0), 9),
        ("FONTNAME", (0, 1), (-1, -1), "Helvetica"),
        ("FONTSIZE", (0, 1), (-1, -1), 8.5),
        ("TEXTCOLOR", (0, 1), (-1, -1), TEXT_PRIMARY),
        ("BACKGROUND", (0, 1), (-1, -1), BG_CARD),
        ("ROWBACKGROUNDS", (0, 1), (-1, -1), [BG_CARD, HexColor("#1a1a25")]),
        ("GRID", (0, 0), (-1, -1), 0.4, HexColor("#2a2a3a")),
        ("TOPPADDING", (0, 0), (-1, -1), 4),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 4),
        ("LEFTPADDING", (0, 0), (-1, -1), 6),
        ("RIGHTPADDING", (0, 0), (-1, -1), 6),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
    ])


def make_table(headers, rows, col_widths=None):
    data = [headers] + rows
    t = Table(data, colWidths=col_widths, repeatRows=1)
    t.setStyle(table_style())
    return t


def build_pdf():
    sty = build_styles()
    doc = SimpleDocTemplate(
        OUTPUT, pagesize=A4,
        leftMargin=22 * mm, rightMargin=22 * mm,
        topMargin=25 * mm, bottomMargin=25 * mm,
    )
    tmpl = DarkPageTemplate(doc)
    story = []
    usable_w = W - 44 * mm

    # ── Title Page ──
    story.append(Spacer(1, 50 * mm))
    story.append(Paragraph("AZTIBASE", sty["title"]))
    story.append(Paragraph("NETWORK", sty["title"]))
    story.append(Spacer(1, 8 * mm))
    story.append(HRFlowable(width="40%", thickness=2, color=ACCENT, spaceAfter=8 * mm))
    story.append(Paragraph("The Chain Where AI Agents Live", sty["subtitle"]))
    story.append(Spacer(1, 12 * mm))
    story.append(Paragraph("Litepaper v1.0 — March 2026", sty["version"]))
    story.append(Paragraph("aztibase.com | @aztibase", sty["version"]))
    story.append(PageBreak())

    # ── The Problem ──
    story.append(Paragraph("The Problem", sty["h1"]))
    story.append(accent_line())
    story.append(Paragraph(
        "AI agents need a blockchain that understands them. Current chains treat AI as an afterthought — "
        "inference happens off-chain, verification is impossible, and 12-second block times make "
        "per-inference micropayments impractical.", sty["body"]
    ))
    story.append(Paragraph(
        "Aztibase is the first Layer 1 blockchain built from the ground up for AI agent economies.",
        sty["body"]
    ))

    # ── What Makes Aztibase Different ──
    story.append(Paragraph("What Makes Aztibase Different", sty["h1"]))
    story.append(accent_line())

    story.append(Paragraph("AI-Native at the Protocol Level", sty["h2"]))
    story.append(Paragraph(
        "Every Aztibase node runs an ONNX inference engine. AI isn't a smart contract bolted on top — "
        "it's part of the node software itself.", sty["body"]
    ))
    for item in [
        "<b>Model Registry</b> — Register AI models on-chain with metadata, pricing, and version tracking",
        "<b>Inference Execution</b> — Nodes execute AI inference tasks and produce verifiable results",
        "<b>AI Sentinel</b> — Autonomous health monitor using machine learning, detecting anomalies and taking corrective action",
        "<b>Agent Identity</b> — On-chain accounts for AI agents with capability policies and spending limits",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    story.append(Paragraph("Dual Virtual Machine", sty["h2"]))
    story.append(Paragraph(
        "Aztibase runs both EVM and WASM smart contracts side by side, with a cross-VM bridge:", sty["body"]
    ))
    for item in [
        "<b>EVM (revm)</b> — Deploy Solidity contracts, compatible with Ethereum tooling",
        "<b>WASM (wasmtime)</b> — Deploy Rust, C, or AssemblyScript for maximum performance",
        "<b>Cross-VM Bridge</b> — WASM contracts call EVM contracts and vice versa",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    story.append(Paragraph("DAG-Based Consensus (Synaptic Consensus)", sty["h2"]))
    story.append(Paragraph(
        "Unlike linear blockchains, Aztibase uses a Directed Acyclic Graph where multiple blocks "
        "are proposed simultaneously:", sty["body"]
    ))
    for item in [
        "<b>400ms block time</b> (vs 12s Ethereum)",
        "<b>Sub-1.6s finality</b> (4 consensus rounds)",
        "<b>No leader bottleneck</b> — all validators propose concurrently",
        "<b>Byzantine fault tolerant</b> — tolerates up to 1/3 malicious validators",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    story.append(Paragraph("Server Independence", sty["h2"]))
    for item in [
        "Pure P2P via libp2p (Kademlia DHT, gossipsub, QUIC + WebRTC)",
        "Every node is its own RPC endpoint",
        "No bootstrap server dependency after initial peer discovery",
        "Browser nodes via WASM + WebRTC (no server relay)",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    story.append(PageBreak())

    # ── Architecture ──
    story.append(Paragraph("Architecture", sty["h1"]))
    story.append(accent_line())

    arch_lines = [
        "Aztibase Node",
        "─────────────────────────────────────",
        "Consensus (DAG-BFT)",
        "  Vertex proposal + voting",
        "  Finality certificates",
        "  Equivocation detection",
        "",
        "Execution Pipeline",
        "  27+ transaction types",
        "  EVM (revm) + WASM (wasmtime)",
        "  Block-STM parallel execution",
        "  EIP-1559 dynamic gas pricing",
        "  AI inference routing",
        "",
        "Storage (redb) — Pure Rust, ACID",
        "  Verkle tree state commitments",
        "",
        "Networking (libp2p)",
        "  QUIC + WebRTC transport",
        "  Gossipsub propagation",
        "  Pipelined block sync",
        "",
        "AI Runtime (tract/ONNX)",
        "  Model registry + marketplace",
        "  Anomaly scorer + Sentinel",
        "",
        "JSON-RPC + WebSocket API",
        "  57+ methods, Prometheus metrics",
    ]
    for line in arch_lines:
        story.append(Paragraph(line, sty["code"]))

    story.append(Paragraph("What's Built and Running", sty["h2"]))
    story.append(make_table(
        ["Component", "Status", "Details"],
        [
            ["DAG Consensus", "Live", "Sub-second finality on public testnet"],
            ["EVM Contracts", "Live", "Deploy, call, read via aztb_call/eth_call"],
            ["WASM Contracts", "Live", "Full deploy/call with fuel metering"],
            ["Cross-VM Bridge", "Live", "Bidirectional EVM-WASM calls"],
            ["Block-STM", "Live", "Concurrent transfer execution"],
            ["AI Sentinel", "Live", "3-tier autonomous action engine"],
            ["Dynamic Gas", "Live", "EIP-1559 adaptive base fee"],
            ["DEX (AMM)", "Live", "WASZTB/tUSDC pair with liquidity"],
            ["Oracle", "Live", "CoinGecko price feed, on-chain updates"],
            ["Block Sync", "Live", "5x pipelined catch-up"],
            ["TypeScript SDK", "Published", "@aztibase/sdk, 21 tests"],
            ["Chrome Wallet", "Published", "Extension with staking UI"],
            ["Indexer + Explorer", "Live", "SQLite-backed, REST API"],
        ],
        col_widths=[usable_w * 0.22, usable_w * 0.12, usable_w * 0.66],
    ))

    story.append(PageBreak())

    # ── Tokenomics ──
    story.append(Paragraph("Tokenomics", sty["h1"]))
    story.append(accent_line())

    story.append(Paragraph("AZTB Token", sty["h2"]))
    story.append(make_table(
        ["Parameter", "Value"],
        [
            ["Total Supply", "1,000,000,000 AZTB"],
            ["Genesis Mint", "400,000,000 (40%)"],
            ["Emission Pool", "600,000,000 (60%) over ~10 years"],
            ["Block Time", "400ms"],
            ["Consensus", "DAG-BFT (Synaptic Consensus)"],
        ],
        col_widths=[usable_w * 0.35, usable_w * 0.65],
    ))

    story.append(Paragraph("Genesis Allocation (400M)", sty["h2"]))
    story.append(make_table(
        ["Category", "Amount", "Vesting"],
        [
            ["Protocol Treasury", "100M (25%)", "Immediate, governance-controlled"],
            ["Ecosystem Development", "80M (20%)", "6-month cliff, 4-year linear"],
            ["Core Team", "60M (15%)", "12-month cliff, 4-year linear"],
            ["Foundation Reserve", "40M (10%)", "2-year lock, 3-year linear"],
            ["Community Airdrop", "40M (10%)", "Immediate"],
            ["Validator Bootstrap", "40M (10%)", "6-month linear"],
            ["AI Ecosystem Fund", "20M (5%)", "6-month cliff, 3-year linear"],
            ["Liquidity Provision", "20M (5%)", "Immediate"],
        ],
        col_widths=[usable_w * 0.30, usable_w * 0.18, usable_w * 0.52],
    ))

    story.append(Paragraph("Emission Schedule", sty["h2"]))
    story.append(Paragraph(
        "Validator rewards, AI compute incentives, and treasury funding come from the 600M emission pool:",
        sty["body"]
    ))
    story.append(make_table(
        ["Years", "Annual Emission", "Cumulative"],
        [
            ["1-2", "120M/year", "240M"],
            ["3-4", "60M/year", "360M"],
            ["5-6", "30M/year", "420M"],
            ["7-8", "15M/year", "450M"],
            ["9-10", "7.5M/year", "465M"],
            ["11+", "3.75M/year (tail)", "Approaches 600M"],
        ],
        col_widths=[usable_w * 0.25, usable_w * 0.38, usable_w * 0.37],
    ))

    story.append(Paragraph("Emission Distribution", sty["h2"]))
    story.append(make_table(
        ["Recipient", "Share"],
        [
            ["Validator Rewards", "70%"],
            ["AI Compute (PoUW)", "15%"],
            ["Protocol Treasury", "10%"],
            ["Staking Insurance", "5%"],
        ],
        col_widths=[usable_w * 0.50, usable_w * 0.50],
    ))

    story.append(Paragraph("Staking", sty["h2"]))
    story.append(make_table(
        ["Parameter", "Value"],
        [
            ["Minimum Stake", "50,000 AZTB (governance range: 10K-500K)"],
            ["Maximum Stake", "50,000,000 AZTB (5% of supply)"],
            ["Unbonding Period", "21 days"],
            ["Target APY", "3-12%"],
            ["Equivocation Slash", "10%"],
            ["Downtime Slash", "0.5%"],
        ],
        col_widths=[usable_w * 0.35, usable_w * 0.65],
    ))

    story.append(PageBreak())

    # ── AI Integration ──
    story.append(Paragraph("AI Integration", sty["h1"]))
    story.append(accent_line())

    story.append(Paragraph("Three-Layer AI Stack", sty["h2"]))

    story.append(Paragraph("Layer 1 — Anomaly Detection (per-transaction)", sty["h3"]))
    story.append(Paragraph(
        "Every transaction is scored for anomalous behavior using an ONNX autoencoder model "
        "running inside the node. High scores flag potential attacks or unusual patterns.",
        sty["body"]
    ))

    story.append(Paragraph("Layer 2 — Sentinel Health Monitor (per-epoch)", sty["h3"]))
    story.append(Paragraph(
        "The AI Sentinel aggregates network metrics (commit latency, TPS, peer count, equivocations) "
        "and produces a chain health score. It operates in three tiers:",
        sty["body"]
    ))
    for item in [
        "Tier 1: Observe and report",
        "Tier 2: ONNX model-based scoring with confidence levels",
        "Tier 3: Autonomous action (throttle gas, alert validators, trigger emergency responses)",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    story.append(Paragraph("Layer 3 — Compute Marketplace", sty["h3"]))
    story.append(Paragraph(
        "Model creators register models on-chain. Users post inference tasks with reward amounts. "
        "Compute providers execute tasks, submit attestations, and receive payment. "
        "The chain verifies results.",
        sty["body"]
    ))

    story.append(Paragraph("Why This Matters", sty["h2"]))
    story.append(Paragraph("AI agents generating autonomous transactions need a chain that can:", sty["body"]))
    for i, item in enumerate([
        "Verify inference results (not just trust an oracle)",
        "Settle micropayments per-inference (400ms finality enables this)",
        "Manage agent identity and spending policies on-chain",
        "Detect and respond to anomalous agent behavior automatically",
    ], 1):
        story.append(Paragraph(f"{i}. {item}", sty["bullet"]))
    story.append(Paragraph(
        "<b>No other L1 has this stack built into the protocol layer.</b>", sty["body"]
    ))

    # ── Technology ──
    story.append(Paragraph("Technology", sty["h1"]))
    story.append(accent_line())
    story.append(make_table(
        ["Component", "Choice", "Why"],
        [
            ["Language", "Rust", "No GC, memory safety, WASM target"],
            ["Async Runtime", "Tokio", "Production-grade async I/O"],
            ["P2P", "libp2p", "NAT traversal, QUIC, WebRTC"],
            ["EVM", "revm", "Production Ethereum execution engine"],
            ["WASM VM", "wasmtime", "Fastest WASM runtime"],
            ["Storage", "redb", "Pure Rust, ACID, no C deps"],
            ["State", "Verkle trees", "Constant-size proofs"],
            ["Hashing", "BLAKE3", "Fastest cryptographic hash"],
            ["Signatures", "Ed25519", "Industry standard"],
            ["AI Runtime", "tract (ONNX)", "Pure Rust inference, no Python"],
            ["Serialization", "postcard", "Compact binary, no-std"],
        ],
        col_widths=[usable_w * 0.22, usable_w * 0.23, usable_w * 0.55],
    ))
    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph(
        "9 Rust crates in monorepo. 1,040+ tests, zero flaky. 34 Architecture Decision Records. "
        "MIT / Apache-2.0 dual license.",
        sty["body"]
    ))

    story.append(PageBreak())

    # ── Roadmap ──
    story.append(Paragraph("Roadmap", sty["h1"]))
    story.append(accent_line())

    story.append(Paragraph("Completed", sty["h2"]))
    completed = [
        "Layer 1 blockchain (consensus, execution, networking, storage)",
        "Public testnet with monitoring (Grafana + VictoriaMetrics)",
        "Dual VM (EVM + WASM) with cross-VM bridge",
        "AI Sentinel with ONNX inference",
        "On-chain governance (proposals, voting, parameter changes)",
        "L2 bridge primitives (RegisterL2, AnchorL2State, BridgeDeposit, BridgeWithdraw)",
        "AMM DEX with liquidity pools",
        "Oracle with live price feeds",
        "TypeScript SDK, Chrome wallet extension",
        "Block explorer and indexer",
    ]
    for item in completed:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2713"))

    story.append(Paragraph("Next", sty["h2"]))
    upcoming = [
        "Security audit",
        "Mainnet launch with 100+ validators",
        "AI Model Marketplace (register, price, and trade AI models)",
        "Agent Identity Registry (on-chain DID for AI agents)",
        "Content Provenance Registry (C2PA-style proof of AI-generated content)",
        "Name Service (.aztb domains)",
        "Community Task Chain (L2 for AI compute tasks)",
    ]
    for item in upcoming:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2192"))

    # ── Participate ──
    story.append(Paragraph("How to Participate", sty["h1"]))
    story.append(accent_line())

    story.append(Paragraph("Run a Validator", sty["h2"]))
    story.append(Paragraph(
        "Download the binary from GitHub, stake AZTB, and start earning rewards. "
        "Minimum hardware: 2 vCPU, 4 GB RAM, 50 GB SSD (~$4/month on Hetzner).",
        sty["body"]
    ))
    story.append(Paragraph("Build on Aztibase", sty["h2"]))
    story.append(Paragraph(
        "Deploy EVM (Solidity) or WASM (Rust) contracts. Full TypeScript SDK available.",
        sty["body"]
    ))
    story.append(Paragraph("Join the Community", sty["h2"]))
    for item in [
        "Website: aztibase.com",
        "Twitter: @aztibase",
        "GitHub: github.com/aztibase/aztibase-node",
    ]:
        story.append(Paragraph(item, sty["bullet"], bulletText="\u2022"))

    # ── Team ──
    story.append(Paragraph("Team", sty["h1"]))
    story.append(accent_line())
    story.append(Paragraph(
        "Built by a solo engineer in South Africa. Every line of code — consensus, networking, "
        "execution, AI, wallet, explorer, SDK — written from scratch in Rust.",
        sty["body"]
    ))
    story.append(Paragraph("Aztibase (Pty) Ltd is registered with the South African CIPC.", sty["body"]))

    story.append(Spacer(1, 15 * mm))
    story.append(Paragraph("Aztibase Network — The Chain Where AI Agents Live", sty["tagline"]))

    doc.build(story, onFirstPage=tmpl.on_page, onLaterPages=tmpl.on_page)
    print(f"PDF written to {OUTPUT}")


if __name__ == "__main__":
    build_pdf()
