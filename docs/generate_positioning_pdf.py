"""
Generate the Aztibase Network Web3 Positioning Brief as a branded PDF.
Uses the Pearlescent Signal design system from docs/brand/.
Embeds the actual symbol.png and logo-light-render.png from docs/brand/.
"""

import os
from reportlab.lib.pagesizes import A4
from reportlab.lib.units import mm
from reportlab.lib.colors import HexColor, Color, white
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    Image
)
from reportlab.platypus.flowables import Flowable

BRAND_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "brand")
SYMBOL_PNG = os.path.join(BRAND_DIR, "symbol.png")
LOGO_PNG   = os.path.join(BRAND_DIR, "logo-light-render.png")

# ---------------------------------------------------------------------------
# Brand Colors — Pearlescent Signal (docs/brand/DESIGN_PHILOSOPHY.md)
# ---------------------------------------------------------------------------
DEEP_VIOLET   = HexColor("#523482")
SOFT_VIOLET   = HexColor("#6B5B8A")
MIST_VIOLET   = HexColor("#C6B9DA")

GOLD          = HexColor("#B8A67E")
WARM_CORAL    = HexColor("#E49480")
SEAFOAM       = HexColor("#80C4BC")

PEARL_WHITE   = HexColor("#F8F6F3")
LIGHT_WASH    = HexColor("#EEECF2")

INK           = HexColor("#2A2630")
GRAPHITE      = HexColor("#5C5662")
SILVER        = HexColor("#A8A4AE")
HAIRLINE      = HexColor("#DAD6DE")

BLUE          = HexColor("#0052FF")
VIOLET        = HexColor("#9B7DFF")
PINK          = HexColor("#FF6B9D")

PAGE_W, PAGE_H = A4
MARGIN_L = 22 * mm
MARGIN_R = 22 * mm
MARGIN_T = 25 * mm
MARGIN_B = 22 * mm
CONTENT_W = PAGE_W - MARGIN_L - MARGIN_R


def _gradient_color(t):
    if t < 0.4:
        f = t / 0.4
        return Color(
            BLUE.red   + (VIOLET.red   - BLUE.red)   * f,
            BLUE.green + (VIOLET.green - BLUE.green) * f,
            BLUE.blue  + (VIOLET.blue  - BLUE.blue)  * f,
        )
    f = (t - 0.4) / 0.6
    return Color(
        VIOLET.red   + (PINK.red   - VIOLET.red)   * f,
        VIOLET.green + (PINK.green - VIOLET.green) * f,
        VIOLET.blue  + (PINK.blue  - VIOLET.blue)  * f,
    )


def _draw_gradient_bar(c, x, y, w, h, steps=100):
    sw = w / steps
    for i in range(steps):
        c.setFillColor(_gradient_color(i / steps))
        c.rect(x + i * sw, y, sw + 0.5, h, fill=1, stroke=0)


# ---------------------------------------------------------------------------
# Flowables
# ---------------------------------------------------------------------------
class GradientBar(Flowable):
    def __init__(self, width, height=2.5 * mm):
        super().__init__()
        self.width = width
        self.height = height

    def draw(self):
        _draw_gradient_bar(self.canv, 0, 0, self.width, self.height)


class SectionDivider(Flowable):
    def __init__(self, width):
        super().__init__()
        self.width = width
        self.height = 6 * mm

    def draw(self):
        c = self.canv
        c.setStrokeColor(GOLD)
        c.setLineWidth(0.5)
        c.line(0, self.height / 2, self.width, self.height / 2)


class PullQuote(Flowable):
    def __init__(self, text, width):
        super().__init__()
        self.text = text
        self.width = width
        self._para = Paragraph(
            f"<i>{text}</i>",
            ParagraphStyle(
                "PQ", fontName="Helvetica-Oblique", fontSize=10.5, leading=15.5,
                textColor=SOFT_VIOLET, leftIndent=10 * mm, rightIndent=6 * mm,
            )
        )
        _, h = self._para.wrap(width - 4 * mm, 999)
        self.height = h + 6 * mm

    def draw(self):
        c = self.canv
        c.setFillColor(MIST_VIOLET)
        c.rect(0, 0, 2.5, self.height, fill=1, stroke=0)
        c.setFillColor(VIOLET)
        c.circle(1.25, self.height - 3 * mm, 1.25, fill=1, stroke=0)
        self._para.drawOn(c, 2 * mm, 2 * mm)


# ---------------------------------------------------------------------------
# Page templates
# ---------------------------------------------------------------------------
def _draw_header_footer(canvas_obj, doc, is_first=False):
    canvas_obj.saveState()
    bar_h = 4 * mm if is_first else 3 * mm
    _draw_gradient_bar(canvas_obj, 0, PAGE_H - bar_h, PAGE_W, bar_h)

    if not is_first and doc.page > 1:
        # Small symbol in header
        symbol_size = 6 * mm
        canvas_obj.drawImage(
            SYMBOL_PNG,
            MARGIN_L, PAGE_H - bar_h - 11 * mm,
            width=symbol_size, height=symbol_size,
            preserveAspectRatio=True, mask='auto'
        )
        canvas_obj.setFont("Helvetica-Bold", 7.5)
        canvas_obj.setFillColor(GRAPHITE)
        canvas_obj.drawString(MARGIN_L + 8 * mm, PAGE_H - bar_h - 9 * mm,
                              "AZTIBASE NETWORK")
        canvas_obj.setFont("Helvetica", 7.5)
        canvas_obj.setFillColor(SILVER)
        canvas_obj.drawRightString(PAGE_W - MARGIN_R, PAGE_H - bar_h - 9 * mm,
                                   "Web3 Positioning Brief")

    # Footer
    canvas_obj.setStrokeColor(GOLD)
    canvas_obj.setLineWidth(0.4)
    canvas_obj.line(MARGIN_L, MARGIN_B - 4 * mm, PAGE_W - MARGIN_R, MARGIN_B - 4 * mm)
    canvas_obj.setFont("Helvetica", 7)
    canvas_obj.setFillColor(SILVER)
    canvas_obj.drawString(MARGIN_L, MARGIN_B - 8 * mm,
                          "Aztibase (Pty) Ltd  \u00b7  aztibase.com  \u00b7  Confidential")
    if not is_first:
        canvas_obj.drawRightString(PAGE_W - MARGIN_R, MARGIN_B - 8 * mm,
                                   f"{doc.page}")
    canvas_obj.restoreState()


def first_page_template(canvas_obj, doc):
    _draw_header_footer(canvas_obj, doc, is_first=True)


def later_page_template(canvas_obj, doc):
    _draw_header_footer(canvas_obj, doc, is_first=False)


# ---------------------------------------------------------------------------
# Styles
# ---------------------------------------------------------------------------
def make_styles():
    s = {}
    s["subtitle"] = ParagraphStyle(
        "Subtitle", fontName="Helvetica", fontSize=15, leading=19,
        textColor=SOFT_VIOLET, spaceAfter=3 * mm
    )
    s["h1"] = ParagraphStyle(
        "H1", fontName="Helvetica-Bold", fontSize=17, leading=21,
        textColor=DEEP_VIOLET, spaceBefore=7 * mm, spaceAfter=3.5 * mm
    )
    s["h2"] = ParagraphStyle(
        "H2", fontName="Helvetica-Bold", fontSize=12.5, leading=16,
        textColor=SOFT_VIOLET, spaceBefore=5 * mm, spaceAfter=2.5 * mm
    )
    s["body"] = ParagraphStyle(
        "Body", fontName="Helvetica", fontSize=9.5, leading=14,
        textColor=INK, alignment=TA_JUSTIFY, spaceAfter=3 * mm
    )
    s["body_bold"] = ParagraphStyle(
        "BodyBold", fontName="Helvetica-Bold", fontSize=9.5, leading=14,
        textColor=INK, alignment=TA_JUSTIFY, spaceAfter=3 * mm
    )
    s["bullet"] = ParagraphStyle(
        "Bullet", fontName="Helvetica", fontSize=9.5, leading=14,
        textColor=INK, leftIndent=8 * mm, bulletIndent=3 * mm,
        spaceAfter=1.5 * mm, alignment=TA_LEFT
    )
    s["caption"] = ParagraphStyle(
        "Caption", fontName="Helvetica", fontSize=8, leading=11,
        textColor=SILVER, alignment=TA_LEFT, spaceAfter=2 * mm
    )
    s["th"] = ParagraphStyle(
        "TH", fontName="Helvetica-Bold", fontSize=8.5, leading=11,
        textColor=white, alignment=TA_LEFT
    )
    s["td"] = ParagraphStyle(
        "TD", fontName="Helvetica", fontSize=8.5, leading=11.5,
        textColor=INK, alignment=TA_LEFT
    )
    s["td_b"] = ParagraphStyle(
        "TDB", fontName="Helvetica-Bold", fontSize=8.5, leading=11.5,
        textColor=INK, alignment=TA_LEFT
    )
    s["footer"] = ParagraphStyle(
        "Footer", fontName="Helvetica", fontSize=9, leading=13,
        textColor=GRAPHITE, alignment=TA_CENTER, spaceAfter=1.5 * mm
    )
    return s


def branded_table(headers, rows, col_widths, st):
    data = [[Paragraph(h, st["th"]) for h in headers]]
    for row in rows:
        data.append([Paragraph(c, st["td"]) for c in row])
    t = Table(data, colWidths=col_widths, repeatRows=1)
    t.setStyle(TableStyle([
        ("BACKGROUND",    (0, 0), (-1,  0), DEEP_VIOLET),
        ("TEXTCOLOR",     (0, 0), (-1,  0), white),
        ("BOTTOMPADDING", (0, 0), (-1,  0), 3.5 * mm),
        ("TOPPADDING",    (0, 0), (-1,  0), 3 * mm),
        ("ROWBACKGROUNDS",(0, 1), (-1, -1), [PEARL_WHITE, white]),
        ("TOPPADDING",    (0, 1), (-1, -1), 2.5 * mm),
        ("BOTTOMPADDING", (0, 1), (-1, -1), 2.5 * mm),
        ("LEFTPADDING",   (0, 0), (-1, -1), 3 * mm),
        ("RIGHTPADDING",  (0, 0), (-1, -1), 3 * mm),
        ("LINEBELOW",     (0, 0), (-1,  0), 0.8, GOLD),
        ("LINEBELOW",     (0, 1), (-1, -2), 0.25, HAIRLINE),
        ("LINEBELOW",     (0,-1), (-1, -1), 0.5, GOLD),
        ("VALIGN",        (0, 0), (-1, -1), "TOP"),
    ]))
    return t


# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
def build_pdf():
    base_dir = os.path.dirname(os.path.abspath(__file__))
    out_name = "WEB3_POSITIONING.pdf"
    out_path = os.path.join(base_dir, out_name)
    # If file is locked (open in a viewer), write to a temp name
    if os.path.exists(out_path):
        try:
            with open(out_path, 'ab'):
                pass
        except PermissionError:
            out_path = os.path.join(base_dir, "WEB3_POSITIONING_new.pdf")

    doc = SimpleDocTemplate(
        out_path, pagesize=A4,
        leftMargin=MARGIN_L, rightMargin=MARGIN_R,
        topMargin=MARGIN_T + 10 * mm, bottomMargin=MARGIN_B + 4 * mm,
        title="Aztibase Network \u2014 Web3 Positioning Brief",
        author="Aztibase (Pty) Ltd",
        subject="Web3 Positioning Brief",
        creator="Aztibase Network"
    )

    st = make_styles()
    story = []

    # ==================================================================
    # COVER — actual logo PNG from docs/brand/
    # ==================================================================
    story.append(Spacer(1, 8 * mm))

    # Full logo: hex symbol + "aztiBase" wordmark (rendered from logo.svg)
    logo_w = 100 * mm
    logo_h = logo_w * (192 / 840)  # preserve aspect ratio
    story.append(Image(LOGO_PNG, width=logo_w, height=logo_h))

    story.append(Spacer(1, 6 * mm))
    story.append(GradientBar(CONTENT_W, height=2.5 * mm))
    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("Web3 Positioning Brief", st["subtitle"]))
    story.append(Paragraph("March 2026  \u00b7  Version 1.0", st["caption"]))
    story.append(Spacer(1, 5 * mm))

    story.append(PullQuote(
        "Aztibase Network is the first Layer-1 blockchain where AI inference is a protocol "
        "primitive, not an afterthought \u2014 built for a world where centralized servers are a "
        "liability, not a convenience.",
        CONTENT_W
    ))
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 1. THE LANDSCAPE
    # ==================================================================
    story.append(Paragraph("The Landscape", st["h1"]))
    story.append(Paragraph(
        "Web3 is the architectural thesis that users should own their data, identity, and assets "
        "through cryptographic keys and decentralized protocols, not platform terms of service. "
        "Layer-1 blockchains are the infrastructure layer that makes this possible \u2014 they replace "
        "centralized servers with validator networks, databases with on-chain state, and corporate "
        "logins with self-sovereign wallets.",
        st["body"]
    ))
    story.append(Paragraph(
        "As of March 2026, the L1 space has fragmented into specializations: DeFi chains optimizing "
        "for financial throughput, privacy protocols pursuing anonymity, DePIN networks coordinating "
        "physical hardware, and a growing cluster of projects claiming AI integration. The "
        "AI-infrastructure market exceeds <b>$150 billion</b> and is growing at 40%+ annually. "
        "Decentralized compute projects collectively hold <b>$7 billion+</b> in market cap.",
        st["body"]
    ))
    story.append(Paragraph(
        "The convergence of AI and blockchain is inevitable. But integration so far has been "
        "shallow \u2014 middleware layers, compute-only networks, or branding exercises stapled onto "
        "existing architectures. No production chain combines AI-native consensus, general-purpose "
        "smart contracts, protocol-level privacy, and genuine server-independence into a single "
        "coherent design.",
        st["body"]
    ))
    story.append(Paragraph("<b>Aztibase Network occupies this gap.</b>", st["body_bold"]))
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 2. WHAT AZTIBASE IS
    # ==================================================================
    story.append(Paragraph("What Aztibase Is", st["h1"]))
    story.append(Paragraph(
        "Aztibase is a Layer-1, AI-native, server-independent blockchain built entirely in Rust. "
        "It uses DAG-based consensus (Synaptic Consensus) to achieve sub-second finality at "
        "10,000+ TPS, with no central points of failure.",
        st["body"]
    ))
    story.append(Paragraph(
        "AI lives inside the consensus mechanism, the state model, the execution layer, and the "
        "economic design. It is not a sidecar, a subnet, or a middleware integration. Removing AI "
        "from Aztibase would require redesigning the consensus, the state model, the execution "
        "layer, and the tokenomics simultaneously. That is what \u201cfirst-class citizen\u201d means.",
        st["body"]
    ))
    story.append(Paragraph(
        "The name encodes the architecture. Dendrites are the branching input structures of "
        "biological neurons that receive, integrate, and process signals from many sources. The "
        "DAG topology of Synaptic Consensus is a dendritic structure. The AI-native design mirrors "
        "how dendrites gather distributed intelligence. The name teaches what the chain does.",
        st["body"]
    ))

    ci = [
        [Paragraph("<b>Token</b>", st["td_b"]),
         Paragraph("AZTB", st["td"]),
         Paragraph("<b>Chain ID</b>", st["td_b"]),
         Paragraph("0xA27B", st["td"]),
         Paragraph("<b>License</b>", st["td_b"]),
         Paragraph("Dual MIT / Apache-2.0", st["td"])]
    ]
    ci_t = Table(ci, colWidths=[15*mm, 16*mm, 20*mm, 18*mm, 18*mm, None])
    ci_t.setStyle(TableStyle([
        ("BACKGROUND",    (0, 0), (-1, -1), LIGHT_WASH),
        ("TOPPADDING",    (0, 0), (-1, -1), 2.5*mm),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 2.5*mm),
        ("LEFTPADDING",   (0, 0), (-1, -1), 3*mm),
        ("LINEBELOW",     (0, 0), (-1, -1), 0.4, GOLD),
        ("LINEABOVE",     (0, 0), (-1,  0), 0.4, GOLD),
        ("VALIGN",        (0, 0), (-1, -1), "MIDDLE"),
    ]))
    story.append(Spacer(1, 2*mm))
    story.append(ci_t)
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 3. THREE PILLARS
    # ==================================================================
    story.append(Paragraph("Three Pillars", st["h1"]))

    story.append(Paragraph("1. AI-Native Consensus", st["h2"]))
    story.append(Paragraph(
        "Every \u201cAI blockchain\u201d in 2024\u20132026 falls into one of three categories: "
        "AI-only networks with no general smart contracts (Bittensor), middleware that bridges AI "
        "to existing chains (Ritual), or token mergers that rebrand separate architectures under "
        "one ticker (ASI Alliance). None of them make the chain itself intelligent.",
        st["body"]
    ))
    story.append(Paragraph(
        "Aztibase takes a different path. Proof of Useful Work (PoUW) validators earn rewards for "
        "performing verified AI inference \u2014 real computation, not hash puzzles. Validator scoring "
        "weights accuracy (40%), latency (30%), and availability (30%), preventing the "
        "garbage-output problem that plagues speed-only evaluation. The state model includes "
        "AIAgent accounts, ModelRegistry entries, InferenceRequest objects, and "
        "InferenceAttestation records as protocol-level primitives. Smart contracts call "
        "<font face='Courier' size='8.5'>ai_inference()</font> as a built-in operation. AI "
        "inference fees create deflationary pressure through a 5% burn mechanism.",
        st["body"]
    ))
    story.append(Paragraph("<b>Mining on Aztibase produces real value.</b>", st["body_bold"]))

    story.append(Paragraph("2. Server-Independence", st["h2"]))
    story.append(Paragraph(
        "Most blockchains claim decentralization but depend on centralized infrastructure for "
        "practical use. Over 70% of Ethereum transactions route through three RPC providers "
        "(Infura, Alchemy, QuickNode). Solana validators require datacenter hardware. Browser "
        "users interact through centralized APIs, never touching the P2P network directly.",
        st["body"]
    ))
    story.append(Paragraph(
        "Aztibase enforces server-independence as a hard architectural constraint. Every full node "
        "is its own RPC endpoint. Browser users run WASM-compiled light clients connecting via "
        "WebRTC directly to the P2P mesh. Peer discovery uses Kademlia DHT, gossipsub, mDNS, "
        "and hardcoded bootstraps. Verkle tree state proofs are constant-size, enabling mobile "
        "and browser verification without downloading full state. BLS aggregate signatures "
        "compress finality certificates from 4.3KB to 96 bytes.",
        st["body"]
    ))
    story.append(Paragraph(
        "<b>If every centralized server on the internet goes offline, Aztibase keeps running.</b>",
        st["body_bold"]
    ))

    story.append(Paragraph("3. Privacy as Protocol", st["h2"]))
    story.append(Paragraph(
        "Following the thesis that privacy creates irreversible network effects \u2014 bridging "
        "tokens between chains is trivial, but bridging secrets is impossible \u2014 Aztibase "
        "implements privacy as selective disclosure at the protocol layer. Once users commit "
        "private state, that privacy guarantee cannot follow them to another chain. This creates "
        "organic lock-in that is architecturally, not just economically, defensible.",
        st["body"]
    ))
    story.append(Paragraph(
        "Privacy is not full anonymity. It is user-controlled selective disclosure with regulatory "
        "compliance hooks: prove you are KYC-verified without revealing your identity. This "
        "positions the chain for enterprise and institutional adoption while preserving individual "
        "sovereignty.",
        st["body"]
    ))

    story.append(Paragraph("The Intersection Creates the Moat", st["h2"]))
    story.append(Paragraph(
        "Each pillar alone is achievable. Bittensor does AI. Bitcoin does server-independence. "
        "Zcash does privacy. No chain does all three, because the requirements conflict: PoUW "
        "pushes toward datacenter hardware, ZK proofs are compute-heavy, and full self-verification "
        "conflicts with trust assumptions that simplify AI and privacy.",
        st["body"]
    ))
    story.append(Paragraph(
        "Aztibase resolves these through deliberate architectural choices: PoUW is optional (the "
        "chain runs without it), Verkle proofs enable lightweight verification, and privacy is "
        "selective (public by default, private when chosen). Each feature reinforces the others.",
        st["body"]
    ))
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 4. TECHNICAL FOUNDATION
    # ==================================================================
    story.append(Paragraph("Technical Foundation", st["h1"]))
    story.append(branded_table(
        ["Component", "Choice"],
        [
            ["Consensus",     "Synaptic Consensus (SynBFT + PoUW), DAG-based"],
            ["Block Time",    "400ms target, &lt;1.6s finality (4 rounds)"],
            ["Throughput",    "10,000+ TPS at launch"],
            ["VM",            "WASM (wasmtime) + EVM (revm) \u2014 Rust, AssemblyScript, Solidity"],
            ["State Model",   "Hybrid account + object (parallel execution via Block-STM)"],
            ["State Proofs",  "Verkle trees (100\u00d7 smaller than Merkle Patricia Tries)"],
            ["Storage",       "redb (pure Rust embedded DB)"],
            ["Hashing",       "BLAKE3"],
            ["Signatures",    "Ed25519 (transactions), BLS12-381 (finality)"],
            ["P2P",           "rust-libp2p (QUIC + WebRTC)"],
            ["Language",      "Rust \u2014 pure, zero C/C++ dependencies"],
            ["Architecture",  "9 crates, modular monolith"],
            ["License",       "Dual MIT / Apache-2.0"],
            ["Token Supply",  "1 billion hard cap, EIP-1559 burn, halving every 2 years"],
        ],
        col_widths=[35*mm, CONTENT_W - 35*mm],
        st=st
    ))
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 5. COMPETITIVE LANDSCAPE
    # ==================================================================
    story.append(Paragraph("Competitive Landscape", st["h1"]))
    story.append(branded_table(
        ["Category", "Project", "What They Do", "What They Lack"],
        [
            ["AI-only network",      "<b>Bittensor</b> (TAO)",
             "Compute marketplace, subnet architecture",
             "No general smart contracts; speed-only scoring produces garbage outputs"],
            ["AI bridge layer",      "<b>Ritual</b>",
             "Middleware connecting AI to existing chains",
             "Adds latency and trust assumptions; the chain itself is not intelligent"],
            ["AI token merger",      "<b>ASI Alliance</b>",
             "Unified token across three projects",
             "Cosmetic integration; three separate architectures underneath"],
            ["Decentralized compute","<b>Render, Akash</b>",
             "GPU rental networks",
             "No consensus integration; no on-chain verification of inference"],
            ["General L1",           "<b>Solana, Sui, Aptos</b>",
             "High-TPS smart contract platforms",
             "AI is an application, not a protocol primitive"],
        ],
        col_widths=[28*mm, 30*mm, 38*mm, CONTENT_W - 96*mm],
        st=st
    ))
    story.append(Spacer(1, 3*mm))
    story.append(PullQuote(
        "Aztibase is the only project where removing AI would require redesigning the "
        "consensus, state model, execution layer, and tokenomics simultaneously.",
        CONTENT_W
    ))
    story.append(SectionDivider(CONTENT_W))

    # ==================================================================
    # 6. PROOF OF WORK DONE
    # ==================================================================
    story.append(Paragraph("Proof of Work Done", st["h1"]))
    story.append(Paragraph("<b>This is not a whitepaper. The code exists.</b>", st["body_bold"]))
    for p in [
        "<b>44,900+ lines</b> of production Rust across 9 crates",
        "<b>983 unit tests</b> passing, zero flaky, zero Clippy warnings",
        "<b>Live 4-node testnet</b> (3 validators + 1 full node) with public RPC endpoints",
        "<b>48 JSON-RPC methods</b> + WebSocket subscriptions, fully documented",
        "<b>WASM smart contract lifecycle</b> validated on testnet (deploy, initialize, execute, query)",
        "<b>Chrome wallet extension</b> (Manifest V3, 2FA via TOTP + WebAuthn, WASM staking)",
        "<b>AI Sentinel Tier 1</b> shipped (15-feature heuristic health scorer, real-time monitoring)",
        "<b>Epoch rewards validated</b> on live testnet (emission math, per-validator splits, consistency)",
        "<b>Dynamic staking validated</b> (stake, unstake with unbonding, delegate \u2014 all via CLI + RPC)",
        "<b>Block sync validated</b> (full node catches up 0 \u2192 434 blocks in ~45 seconds)",
        "<b>32 Architecture Decision Records</b> documenting every non-trivial technical choice",
    ]:
        story.append(Paragraph(f"\u2022  {p}", st["bullet"]))
    story.append(SectionDivider(CONTENT_W))

    story.append(Spacer(1, 8*mm))
    story.append(GradientBar(CONTENT_W, height=1.5*mm))
    story.append(Spacer(1, 5*mm))
    story.append(Paragraph(
        "Aztibase (Pty) Ltd, South Africa  \u00b7  aztibase.com  \u00b7  @aztibase",
        st["footer"]
    ))

    doc.build(story, onFirstPage=first_page_template, onLaterPages=later_page_template)
    return out_path


if __name__ == "__main__":
    path = build_pdf()
    print(f"PDF generated: {path}")
