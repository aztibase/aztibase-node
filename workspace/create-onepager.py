from reportlab.lib.pagesizes import A4, landscape
from reportlab.lib.units import inch, mm
from reportlab.lib.colors import HexColor
from reportlab.pdfgen import canvas
from reportlab.lib.enums import TA_LEFT, TA_CENTER
from reportlab.platypus import Paragraph
from reportlab.lib.styles import ParagraphStyle
import os

# Brand colors
VIOLET_DARK = HexColor("#3A2D7A")
VIOLET = HexColor("#4A3B8F")
VIOLET_LIGHT = HexColor("#6B5CA5")
VIOLET_MUTED = HexColor("#8B7FB0")
VIOLET_PALE = HexColor("#C4B8E0")
VIOLET_BG = HexColor("#F5F2FA")
GOLD = HexColor("#D4A574")
TEXT_DARK = HexColor("#3A2D7A")
TEXT_BODY = HexColor("#5A5070")
TEXT_LIGHT = HexColor("#8B7FB0")
BG = HexColor("#FDFCFD")
WHITE = HexColor("#FFFFFF")

out_path = os.path.join(os.path.dirname(__file__), "..", "docs", "AZTIBASE_ONE_PAGER.pdf")
W, H = landscape(A4)

c = canvas.Canvas(out_path, pagesize=landscape(A4))
c.setTitle("Aztibase Network — Investor One-Pager")
c.setAuthor("Aztibase Network")

# Background
c.setFillColor(BG)
c.rect(0, 0, W, H, fill=1, stroke=0)

# Top accent bar
c.setFillColor(VIOLET)
c.rect(0, H - 4*mm, W * 0.6, 4*mm, fill=1, stroke=0)
c.setFillColor(GOLD)
c.rect(W * 0.6, H - 4*mm, W * 0.4, 4*mm, fill=1, stroke=0)

# Left accent stripe
c.setFillColor(VIOLET)
c.rect(0, 0, 3*mm, H - 4*mm, fill=1, stroke=0)

# Header area
y = H - 22*mm
c.setFillColor(VIOLET_DARK)
c.setFont("Helvetica", 22)
c.drawString(18*mm, y, "AZTIBASE NETWORK")
y -= 7*mm
c.setFillColor(VIOLET_LIGHT)
c.setFont("Helvetica", 10)
c.drawString(18*mm, y, "The AI-Native Blockchain — Where mining does useful AI work")

# Thin separator
y -= 4*mm
c.setStrokeColor(VIOLET_PALE)
c.setLineWidth(0.5)
c.line(18*mm, y, W - 18*mm, y)

# Column dimensions
left_x = 18*mm
left_w = (W - 36*mm) * 0.52
right_x = left_x + left_w + 8*mm
right_w = (W - 36*mm) * 0.48 - 8*mm
col_top = y - 5*mm

def section_header(c, x, y, text):
    c.setFillColor(VIOLET_DARK)
    c.setFont("Helvetica-Bold", 9)
    c.drawString(x, y, text.upper())
    y -= 2*mm
    c.setStrokeColor(GOLD)
    c.setLineWidth(1.5)
    c.line(x, y, x + 30*mm, y)
    return y - 4*mm

def body_text(c, x, y, text, width, size=7.5):
    style = ParagraphStyle('body', fontName='Helvetica', fontSize=size, leading=size + 2.5,
                           textColor=TEXT_BODY, alignment=TA_LEFT)
    p = Paragraph(text, style)
    w_val, h_val = p.wrap(width, 200*mm)
    p.drawOn(c, x, y - h_val)
    return y - h_val - 2*mm

def bullet_item(c, x, y, bold_part, rest, width, size=7.5):
    style = ParagraphStyle('bullet', fontName='Helvetica', fontSize=size, leading=size + 2.5,
                           textColor=TEXT_BODY, alignment=TA_LEFT, leftIndent=8, firstLineIndent=-8)
    text = f'<bullet>&bull;</bullet> <b><font color="#4A3B8F">{bold_part}</font></b> {rest}'
    p = Paragraph(text, style)
    w_val, h_val = p.wrap(width - 4*mm, 200*mm)
    p.drawOn(c, x + 2*mm, y - h_val)
    return y - h_val - 0.5*mm

# ==================== LEFT COLUMN ====================
y = col_top

# WHAT WE BUILD
y = section_header(c, left_x, y, "What We Build")
y = body_text(c, left_x, y,
    "Layer-1 blockchain where validators earn rewards by running <b>AI inference</b> instead of solving "
    "meaningless puzzles. <b>Proof of Useful Work (PoUW)</b> turns every block into productive computation. "
    "AI inference is a native transaction type, verified on-chain through deterministic re-execution "
    "and quorum attestation.",
    left_w)
y -= 1*mm
y = body_text(c, left_x, y,
    "<b>Dual VM</b> (WASM + EVM) &mdash; write contracts in Rust, C, or Solidity. "
    "<b>DAG consensus</b> with 10,000+ TPS and sub-second finality. "
    "<b>Server-independent</b> via WebRTC browser nodes.",
    left_w)
y -= 2*mm

# TRACTION
y = section_header(c, left_x, y, "Traction & Proof")
y = bullet_item(c, left_x, y, "44,900", "lines of production Rust across 9 crates", left_w)
y = bullet_item(c, left_x, y, "940+", "passing tests, 0 clippy warnings", left_w)
y = bullet_item(c, left_x, y, "18.3K TPS/core", "benchmarked (Criterion)", left_w)
y = bullet_item(c, left_x, y, "Live testnet:", "3 nodes, 400ms blocks, sub-second finality", left_w)
y = bullet_item(c, left_x, y, "59 sprints,", "103 commits, full build traceability", left_w)
y = bullet_item(c, left_x, y, "Pre-mainnet audit:", "71 findings, 70%+ resolved", left_w)
y = bullet_item(c, left_x, y, "48 RPC methods", "— all functional, documented", left_w)
y = bullet_item(c, left_x, y, "One-click", "validator deployment (setup-validator.sh)", left_w)
y -= 2*mm

# TOKENOMICS
y = section_header(c, left_x, y, "Tokenomics")
y = bullet_item(c, left_x, y, "AZTB", "| Hard cap: 1 billion | Halving: every 2 years", left_w)
y = bullet_item(c, left_x, y, "Year 1:", "120M emission — 70% validators, 15% PoUW, 10% treasury, 5% insurance", left_w)
y = bullet_item(c, left_x, y, "EIP-1559", "fee burning creates deflationary pressure", left_w)

# ==================== RIGHT COLUMN ====================
y = col_top

# MARKET
y = section_header(c, right_x, y, "Market Opportunity")
y = bullet_item(c, right_x, y, "$150B+", "AI infrastructure market by 2027", right_w)
y = bullet_item(c, right_x, y, "$7B+", "combined FDV of decentralized AI compute projects", right_w)
y = body_text(c, right_x + 2*mm, y,
    '<font color="#4A3B8F"><b>Bittensor</b> ($3B+), <b>Render</b> ($3B+), <b>Akash</b> ($1B+)</font> '
    '— all bolt compute onto existing chains. None have AI integrated INTO consensus.',
    right_w - 4*mm)
y -= 1*mm

# Box highlight
c.setFillColor(VIOLET_BG)
c.roundRect(right_x, y - 12*mm, right_w, 12*mm, 2*mm, fill=1, stroke=0)
style = ParagraphStyle('highlight', fontName='Helvetica-Bold', fontSize=7.5, leading=10,
                       textColor=VIOLET, alignment=TA_CENTER)
p = Paragraph("Aztibase is the first blockchain with AI at the consensus level", style)
pw, ph = p.wrap(right_w - 8*mm, 20*mm)
p.drawOn(c, right_x + 4*mm, y - 8*mm)
y -= 16*mm

# REVENUE
y = section_header(c, right_x, y, "Revenue Model")
y = bullet_item(c, right_x, y, "Validator hardware", "— Standard ($699) and AI Compute ($1,500) nodes", right_w)
y = bullet_item(c, right_x, y, "Validator NFTs", "— limited-supply mainnet slots ($200–$1,000)", right_w)
y = bullet_item(c, right_x, y, "Token sale", "— Fjord Foundry LBP (fair price discovery)", right_w)
y = bullet_item(c, right_x, y, "Protocol fees", "— 5% cut on all PoUW inference tasks", right_w)
y = bullet_item(c, right_x, y, "Year 1 projection:", "$250K–$850K (conservative)", right_w)
y -= 2*mm

# THE ASK
y = section_header(c, right_x, y, "The Ask")
# Ask box
box_h = 28*mm
c.setFillColor(VIOLET_BG)
c.roundRect(right_x, y - box_h, right_w, box_h, 2*mm, fill=1, stroke=0)
c.setStrokeColor(VIOLET)
c.setLineWidth(1)
c.line(right_x + 2*mm, y - box_h, right_x + 2*mm, y)

ay = y - 4*mm
c.setFillColor(VIOLET_DARK)
c.setFont("Helvetica-Bold", 11)
c.drawString(right_x + 6*mm, ay, "[TO BE CONFIRMED]")
ay -= 4*mm
c.setFont("Helvetica", 7.5)
c.setFillColor(TEXT_BODY)
c.drawString(right_x + 6*mm, ay, "Pre-seed / Seed — SAFE or Token Warrant")
ay -= 5*mm
c.setFont("Helvetica-Bold", 7)
c.setFillColor(VIOLET)
c.drawString(right_x + 6*mm, ay, "USE OF FUNDS")
ay -= 3.5*mm
c.setFont("Helvetica", 7)
c.setFillColor(TEXT_BODY)
c.drawString(right_x + 6*mm, ay, "Engineering 40%  |  Legal 25%  |  Infrastructure 20%  |  Marketing 15%")
ay -= 4*mm
c.setFont("Helvetica", 7)
c.drawString(right_x + 6*mm, ay, "Entity: Aztibase (Pty) Ltd (South Africa)  |  Team: [TO BE CONFIRMED]")

y -= box_h + 2*mm

# ==================== FOOTER ====================
footer_y = 8*mm

# Footer separator
c.setStrokeColor(VIOLET_PALE)
c.setLineWidth(0.5)
c.line(18*mm, footer_y + 6*mm, W - 18*mm, footer_y + 6*mm)

c.setFillColor(TEXT_LIGHT)
c.setFont("Helvetica", 6.5)
c.drawString(18*mm, footer_y, "aztibase.com  |  @aztibase  |  Code available for technical due diligence")

c.setFillColor(VIOLET_MUTED)
c.setFont("Courier", 6.5)
c.drawRightString(W - 18*mm, footer_y, "Run our testnet:  bash start-testnet.sh")

c.save()
print(f"One-pager saved to {out_path}")
