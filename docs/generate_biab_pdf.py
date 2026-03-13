"""
Aztibase Network — Business in a Box
Professional PDF generator using reportlab with brand styling.
"""

from reportlab.lib.pagesizes import A4
from reportlab.lib.units import mm, cm
from reportlab.lib.colors import HexColor, Color
from reportlab.lib.styles import ParagraphStyle
from reportlab.lib.enums import TA_LEFT, TA_CENTER, TA_RIGHT, TA_JUSTIFY
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle,
    PageBreak, HRFlowable, KeepTogether
)
from reportlab.pdfgen import canvas
from reportlab.platypus.doctemplate import PageTemplate, BaseDocTemplate, Frame
from reportlab.lib import colors
import os

# ── Brand Colors (from AZTIBASE_BRAND_IDENTITY) ──────────────────────────

PEARL_WHITE     = HexColor("#FAF7F2")
PEARL_WARM      = HexColor("#F5F0EA")
DEEP_PURPLE     = HexColor("#4A2D73")
MID_PURPLE      = HexColor("#6B4D8A")
LIGHT_PURPLE    = HexColor("#E8DFF0")
SOFT_LAVENDER   = HexColor("#D4C5E2")
GOLD_ACCENT     = HexColor("#C9A84C")
GOLD_WARM       = HexColor("#D4B96A")
GOLD_MUTED      = HexColor("#E8D9A0")
CHARCOAL        = HexColor("#2D2A33")
DARK_GRAY       = HexColor("#4A4652")
MID_GRAY        = HexColor("#8A8590")
LIGHT_GRAY      = HexColor("#D4D0DA")
CORAL_SOFT      = HexColor("#D4907A")
SEAFOAM         = HexColor("#7ABDA0")
WHITE           = HexColor("#FFFFFF")
TABLE_ROW_ALT   = HexColor("#F3EFF8")
TABLE_HEADER_BG = HexColor("#4A2D73")
RULE_COLOR      = HexColor("#D4C5E2")

# ── Page Dimensions ───────────────────────────────────────────────────────

PAGE_W, PAGE_H = A4
MARGIN_LEFT   = 22 * mm
MARGIN_RIGHT  = 22 * mm
MARGIN_TOP    = 25 * mm
MARGIN_BOTTOM = 22 * mm

# ── Styles ────────────────────────────────────────────────────────────────

def make_styles():
    s = {}

    s["title"] = ParagraphStyle(
        "title", fontName="Helvetica-Bold", fontSize=28,
        textColor=DEEP_PURPLE, leading=34, spaceAfter=2 * mm,
        alignment=TA_LEFT
    )
    s["subtitle"] = ParagraphStyle(
        "subtitle", fontName="Helvetica", fontSize=13,
        textColor=MID_PURPLE, leading=18, spaceAfter=8 * mm,
        alignment=TA_LEFT
    )
    s["h1"] = ParagraphStyle(
        "h1", fontName="Helvetica-Bold", fontSize=18,
        textColor=DEEP_PURPLE, leading=24, spaceBefore=10 * mm,
        spaceAfter=4 * mm, alignment=TA_LEFT
    )
    s["h2"] = ParagraphStyle(
        "h2", fontName="Helvetica-Bold", fontSize=13,
        textColor=MID_PURPLE, leading=18, spaceBefore=6 * mm,
        spaceAfter=3 * mm, alignment=TA_LEFT
    )
    s["h3"] = ParagraphStyle(
        "h3", fontName="Helvetica-Bold", fontSize=11,
        textColor=DARK_GRAY, leading=15, spaceBefore=4 * mm,
        spaceAfter=2 * mm, alignment=TA_LEFT
    )
    s["body"] = ParagraphStyle(
        "body", fontName="Helvetica", fontSize=9.5,
        textColor=CHARCOAL, leading=14, spaceAfter=3 * mm,
        alignment=TA_JUSTIFY
    )
    s["body_bold"] = ParagraphStyle(
        "body_bold", fontName="Helvetica-Bold", fontSize=9.5,
        textColor=CHARCOAL, leading=14, spaceAfter=3 * mm,
        alignment=TA_LEFT
    )
    s["bullet"] = ParagraphStyle(
        "bullet", fontName="Helvetica", fontSize=9.5,
        textColor=CHARCOAL, leading=14, spaceAfter=1.5 * mm,
        leftIndent=8 * mm, bulletIndent=3 * mm,
        alignment=TA_LEFT
    )
    s["callout"] = ParagraphStyle(
        "callout", fontName="Helvetica-Oblique", fontSize=10,
        textColor=DEEP_PURPLE, leading=15, spaceAfter=4 * mm,
        leftIndent=5 * mm, rightIndent=5 * mm,
        alignment=TA_LEFT, backColor=LIGHT_PURPLE,
        borderPadding=(8, 10, 8, 10),
    )
    s["small"] = ParagraphStyle(
        "small", fontName="Helvetica", fontSize=8,
        textColor=MID_GRAY, leading=11, spaceAfter=2 * mm,
        alignment=TA_LEFT
    )
    s["footer"] = ParagraphStyle(
        "footer", fontName="Helvetica", fontSize=7.5,
        textColor=MID_GRAY, leading=10, alignment=TA_CENTER
    )
    s["toc_item"] = ParagraphStyle(
        "toc_item", fontName="Helvetica", fontSize=10,
        textColor=DARK_GRAY, leading=18, leftIndent=5 * mm,
        spaceAfter=1 * mm, alignment=TA_LEFT
    )
    s["toc_num"] = ParagraphStyle(
        "toc_num", fontName="Helvetica-Bold", fontSize=10,
        textColor=GOLD_ACCENT, leading=18, alignment=TA_RIGHT
    )
    s["stat_number"] = ParagraphStyle(
        "stat_number", fontName="Helvetica-Bold", fontSize=24,
        textColor=DEEP_PURPLE, leading=28, alignment=TA_CENTER
    )
    s["stat_label"] = ParagraphStyle(
        "stat_label", fontName="Helvetica", fontSize=8,
        textColor=MID_GRAY, leading=11, alignment=TA_CENTER
    )
    s["table_header"] = ParagraphStyle(
        "th", fontName="Helvetica-Bold", fontSize=8.5,
        textColor=WHITE, leading=12, alignment=TA_LEFT
    )
    s["table_cell"] = ParagraphStyle(
        "tc", fontName="Helvetica", fontSize=8.5,
        textColor=CHARCOAL, leading=12, alignment=TA_LEFT
    )
    s["table_cell_bold"] = ParagraphStyle(
        "tcb", fontName="Helvetica-Bold", fontSize=8.5,
        textColor=CHARCOAL, leading=12, alignment=TA_LEFT
    )
    return s

# ── Helpers ───────────────────────────────────────────────────────────────

def gold_rule():
    return HRFlowable(
        width="100%", thickness=0.75, color=GOLD_ACCENT,
        spaceBefore=1 * mm, spaceAfter=3 * mm
    )

def thin_rule():
    return HRFlowable(
        width="100%", thickness=0.4, color=RULE_COLOR,
        spaceBefore=2 * mm, spaceAfter=2 * mm
    )

def purple_rule():
    return HRFlowable(
        width="40%", thickness=2, color=DEEP_PURPLE,
        spaceBefore=0, spaceAfter=4 * mm, hAlign="LEFT"
    )

def section_header(styles, text):
    return [
        Spacer(1, 2 * mm),
        Paragraph(text, styles["h1"]),
        purple_rule(),
    ]

def make_table(headers, rows, col_widths, styles):
    s = styles
    data = [[Paragraph(h, s["table_header"]) for h in headers]]
    for row in rows:
        data.append([
            Paragraph(str(c), s["table_cell_bold"] if i == 0 else s["table_cell"])
            for i, c in enumerate(row)
        ])

    n_rows = len(data)
    row_colors = []
    for i in range(1, n_rows):
        bg = TABLE_ROW_ALT if i % 2 == 0 else WHITE
        row_colors.append(("BACKGROUND", (0, i), (-1, i), bg))

    t = Table(data, colWidths=col_widths, repeatRows=1)
    t.setStyle(TableStyle([
        ("BACKGROUND",    (0, 0), (-1, 0), TABLE_HEADER_BG),
        ("TEXTCOLOR",     (0, 0), (-1, 0), WHITE),
        ("FONTNAME",      (0, 0), (-1, 0), "Helvetica-Bold"),
        ("FONTSIZE",      (0, 0), (-1, 0), 8.5),
        ("BOTTOMPADDING", (0, 0), (-1, 0), 6),
        ("TOPPADDING",    (0, 0), (-1, 0), 6),
        ("LEFTPADDING",   (0, 0), (-1, -1), 6),
        ("RIGHTPADDING",  (0, 0), (-1, -1), 6),
        ("BOTTOMPADDING", (0, 1), (-1, -1), 5),
        ("TOPPADDING",    (0, 1), (-1, -1), 5),
        ("GRID",          (0, 0), (-1, -1), 0.4, RULE_COLOR),
        ("VALIGN",        (0, 0), (-1, -1), "TOP"),
        ("ROUNDEDCORNERS", [3, 3, 3, 3]),
    ] + row_colors))
    return t

def stat_box(styles, number, label):
    data = [[
        Paragraph(number, styles["stat_number"]),
    ], [
        Paragraph(label, styles["stat_label"]),
    ]]
    t = Table(data, colWidths=[38 * mm])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), LIGHT_PURPLE),
        ("ALIGN", (0, 0), (-1, -1), "CENTER"),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
        ("TOPPADDING", (0, 0), (-1, 0), 8),
        ("BOTTOMPADDING", (0, -1), (-1, -1), 8),
        ("LEFTPADDING", (0, 0), (-1, -1), 4),
        ("RIGHTPADDING", (0, 0), (-1, -1), 4),
        ("ROUNDEDCORNERS", [4, 4, 4, 4]),
    ]))
    return t

def stat_row(styles, stats):
    boxes = [stat_box(styles, n, l) for n, l in stats]
    t = Table([boxes], colWidths=[42 * mm] * len(stats))
    t.setStyle(TableStyle([
        ("ALIGN", (0, 0), (-1, -1), "CENTER"),
        ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
    ]))
    return t

def bullet(styles, text):
    return Paragraph(f"\u2022  {text}", styles["bullet"])

def callout_box(styles, text):
    data = [[Paragraph(text, styles["callout"])]]
    t = Table(data, colWidths=[PAGE_W - MARGIN_LEFT - MARGIN_RIGHT - 4 * mm])
    t.setStyle(TableStyle([
        ("BACKGROUND", (0, 0), (-1, -1), LIGHT_PURPLE),
        ("LEFTPADDING", (0, 0), (-1, -1), 12),
        ("RIGHTPADDING", (0, 0), (-1, -1), 12),
        ("TOPPADDING", (0, 0), (-1, -1), 10),
        ("BOTTOMPADDING", (0, 0), (-1, -1), 10),
        ("ROUNDEDCORNERS", [4, 4, 4, 4]),
    ]))
    return t

# ── Page Drawing (header/footer) ─────────────────────────────────────────

class BrandDocTemplate(BaseDocTemplate):
    def __init__(self, filename, **kwargs):
        super().__init__(filename, **kwargs)
        frame = Frame(
            MARGIN_LEFT, MARGIN_BOTTOM,
            PAGE_W - MARGIN_LEFT - MARGIN_RIGHT,
            PAGE_H - MARGIN_TOP - MARGIN_BOTTOM,
            id="main"
        )
        self.addPageTemplates([
            PageTemplate(id="cover", frames=[frame], onPage=self.draw_cover),
            PageTemplate(id="normal", frames=[frame], onPage=self.draw_normal),
        ])

    def draw_cover(self, canvas, doc):
        canvas.saveState()
        # Full page subtle gradient background
        canvas.setFillColor(PEARL_WHITE)
        canvas.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)

        # Top accent bar
        canvas.setFillColor(DEEP_PURPLE)
        canvas.rect(0, PAGE_H - 8 * mm, PAGE_W, 8 * mm, fill=1, stroke=0)

        # Gold accent line under purple bar
        canvas.setStrokeColor(GOLD_ACCENT)
        canvas.setLineWidth(1.5)
        canvas.line(0, PAGE_H - 8 * mm, PAGE_W, PAGE_H - 8 * mm)

        # Bottom bar
        canvas.setFillColor(DEEP_PURPLE)
        canvas.rect(0, 0, PAGE_W, 12 * mm, fill=1, stroke=0)

        # Bottom text
        canvas.setFillColor(HexColor("#C4B8D6"))
        canvas.setFont("Helvetica", 7.5)
        canvas.drawString(MARGIN_LEFT, 4.5 * mm, "aztibase.com  |  @aztibase  |  Aztibase (Pty) Ltd")
        canvas.drawRightString(PAGE_W - MARGIN_RIGHT, 4.5 * mm, "Confidential")

        canvas.restoreState()

    def draw_normal(self, canvas, doc):
        canvas.saveState()
        # Subtle background
        canvas.setFillColor(PEARL_WHITE)
        canvas.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)

        # Top thin accent line
        canvas.setStrokeColor(DEEP_PURPLE)
        canvas.setLineWidth(2)
        canvas.line(0, PAGE_H - 3 * mm, PAGE_W, PAGE_H - 3 * mm)

        canvas.setStrokeColor(GOLD_ACCENT)
        canvas.setLineWidth(0.5)
        canvas.line(0, PAGE_H - 4 * mm, PAGE_W, PAGE_H - 4 * mm)

        # Header text
        canvas.setFillColor(MID_GRAY)
        canvas.setFont("Helvetica", 7)
        canvas.drawString(MARGIN_LEFT, PAGE_H - 9 * mm, "AZTIBASE NETWORK")
        canvas.drawRightString(PAGE_W - MARGIN_RIGHT, PAGE_H - 9 * mm, "Business in a Box — Product Strategy")

        # Footer
        canvas.setStrokeColor(RULE_COLOR)
        canvas.setLineWidth(0.4)
        canvas.line(MARGIN_LEFT, 14 * mm, PAGE_W - MARGIN_RIGHT, 14 * mm)

        canvas.setFillColor(MID_GRAY)
        canvas.setFont("Helvetica", 7)
        canvas.drawString(MARGIN_LEFT, 9 * mm, "Confidential — Aztibase (Pty) Ltd")
        canvas.drawRightString(PAGE_W - MARGIN_RIGHT, 9 * mm, f"Page {doc.page}")

        canvas.restoreState()


# ── Document Content ──────────────────────────────────────────────────────

def build_document():
    output_path = os.path.join(os.path.dirname(__file__), "AZTIBASE_BUSINESS_IN_A_BOX.pdf")
    doc = BrandDocTemplate(output_path, pagesize=A4)
    s = make_styles()
    story = []
    W = PAGE_W - MARGIN_LEFT - MARGIN_RIGHT

    # ══════════════════════════════════════════════════════════════════════
    # COVER PAGE
    # ══════════════════════════════════════════════════════════════════════
    story.append(Spacer(1, 35 * mm))
    story.append(Paragraph("AZTIBASE NETWORK", s["title"]))
    story.append(Spacer(1, 2 * mm))

    cover_sub = ParagraphStyle(
        "cover_sub", parent=s["subtitle"], fontSize=11,
        textColor=GOLD_ACCENT, fontName="Helvetica-Bold",
        spaceAfter=3 * mm
    )
    story.append(Paragraph("The AI-Native Blockchain", cover_sub))
    story.append(gold_rule())
    story.append(Spacer(1, 8 * mm))

    cover_title = ParagraphStyle(
        "cover_title", parent=s["title"], fontSize=32,
        textColor=DEEP_PURPLE, leading=38, spaceAfter=6 * mm
    )
    story.append(Paragraph("Business in a Box", cover_title))

    cover_desc = ParagraphStyle(
        "cover_desc", parent=s["body"], fontSize=12,
        textColor=DARK_GRAY, leading=18, spaceAfter=4 * mm
    )
    story.append(Paragraph(
        "Product strategy for plug-and-play validator hardware — "
        "turning a blockchain node into an appliance anyone can operate.",
        cover_desc
    ))

    story.append(Spacer(1, 12 * mm))

    # Key stats row
    story.append(stat_row(s, [
        ("$699", "Standard Node"),
        ("$1,500", "AI Compute Node"),
        ("$200+", "Margin / Unit"),
        ("7 min", "Unbox to Earning"),
    ]))

    story.append(Spacer(1, 15 * mm))

    cover_meta = ParagraphStyle(
        "cover_meta", parent=s["small"], fontSize=9,
        textColor=MID_GRAY, leading=14
    )
    story.append(Paragraph("Prepared by: Aztibase (Pty) Ltd  |  March 2026", cover_meta))
    story.append(Paragraph("Status: Strategic Planning  |  Version 1.0", cover_meta))
    story.append(Paragraph("Classification: Confidential", cover_meta))

    story.append(PageBreak())

    # Switch to normal page template
    from reportlab.platypus import NextPageTemplate
    story.insert(-1, NextPageTemplate("normal"))

    # ══════════════════════════════════════════════════════════════════════
    # TABLE OF CONTENTS
    # ══════════════════════════════════════════════════════════════════════
    story.append(Spacer(1, 5 * mm))
    story.append(Paragraph("CONTENTS", s["h1"]))
    story.append(purple_rule())

    toc_items = [
        ("01", "Executive Summary"),
        ("02", "Product Definition — Two SKUs"),
        ("03", "Unit Economics & Bill of Materials"),
        ("04", "Customer Journey — Unbox to Earning"),
        ("05", "Technical Architecture — The 4-Layer Stack"),
        ("06", "Fulfillment Model — Phase 1 & Phase 2"),
        ("07", "Revenue Model — Stacked Income Streams"),
        ("08", "Go / No-Go Checklist"),
        ("09", "Risk Matrix"),
        ("10", "Timeline & Milestones"),
    ]

    for num, title in toc_items:
        row = Table(
            [[Paragraph(f'<font color="#{GOLD_ACCENT.hexval()[2:]}">{num}</font>', s["toc_num"]),
              Paragraph(title, s["toc_item"])]],
            colWidths=[12 * mm, W - 12 * mm]
        )
        row.setStyle(TableStyle([
            ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
            ("LEFTPADDING", (0, 0), (-1, -1), 0),
            ("RIGHTPADDING", (0, 0), (-1, -1), 0),
            ("BOTTOMPADDING", (0, 0), (-1, -1), 2),
            ("TOPPADDING", (0, 0), (-1, -1), 2),
        ]))
        story.append(row)

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 01 — EXECUTIVE SUMMARY
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "01  Executive Summary"))

    story.append(Paragraph(
        "Aztibase Network is a Layer-1 AI-native blockchain where validators earn rewards by running "
        "useful AI inference instead of solving meaningless hash puzzles. The protocol is production-ready: "
        "44,900 lines of Rust across 9 crates, 940+ passing tests, and a live 3-node testnet producing "
        "400ms blocks with sub-second finality.",
        s["body"]
    ))
    story.append(Paragraph(
        "The <b>Business in a Box</b> model transforms this technology into a physical product: a pre-configured "
        "mini PC that arrives ready to validate. The operator plugs it in, connects to WiFi, enters an invite "
        "code in a browser-based setup wizard, and begins earning AZTB tokens within minutes. No terminal, "
        "no compilation, no DevOps knowledge required.",
        s["body"]
    ))
    story.append(Paragraph(
        "This document defines two hardware SKUs, their unit economics, the customer journey from "
        "unboxing to earning, the technical architecture that enables plug-and-play operation, and the "
        "phased fulfillment strategy from white-label to custom OEM.",
        s["body"]
    ))

    story.append(Spacer(1, 3 * mm))
    story.append(callout_box(s,
        "<b>Core thesis:</b> The validator hardware business generates immediate fiat revenue ($200+ margin per unit), "
        "seeds the network with real validators, and creates a self-reinforcing flywheel — more validators increase "
        "network security, which increases token value, which increases hardware demand."
    ))

    # ══════════════════════════════════════════════════════════════════════
    # 02 — PRODUCT DEFINITION
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "02  Product Definition — Two SKUs"))

    story.append(Paragraph(
        "Two distinct products serve two distinct buyer profiles. The Standard Node targets the "
        "crypto-curious operator who wants passive income from staking. The AI Compute Node targets "
        "the technically ambitious operator who wants to earn additional PoUW inference rewards.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))
    story.append(Paragraph("SKU Comparison", s["h2"]))

    story.append(make_table(
        ["", "Standard Node", "AI Compute Node"],
        [
            ["Target Buyer", "Passive staker, non-technical", "Power user, AI-curious operator"],
            ["Base Hardware", "Beelink SER5 (Ryzen 5, 16GB)", "Beelink SER7 (Ryzen 7, 32GB)"],
            ["CPU", "6-core / 12-thread", "8-core / 16-thread"],
            ["RAM", "16 GB DDR4", "32 GB DDR5"],
            ["Storage", "500 GB NVMe", "1 TB NVMe"],
            ["Network", "1 GbE + WiFi 6", "2.5 GbE + WiFi 6E"],
            ["AI Capability", "None (PassthroughRuntime)", "64 MiB ONNX inference via tract"],
            ["Earnings", "Staking rewards (70% of emission)", "Staking + PoUW rewards (70% + 15%)"],
            ["Retail Price", "$699", "$1,499"],
            ["Hardware Cost", "~$280", "~$520"],
            ["Gross Margin", "~$419 (60%)", "~$979 (65%)"],
        ],
        [W * 0.18, W * 0.41, W * 0.41],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("What's in the Box", s["h2"]))

    story.append(make_table(
        ["Item", "Standard Node", "AI Compute Node"],
        [
            ["Pre-configured mini PC", "\u2713", "\u2713"],
            ["Pre-flashed Ubuntu + Aztibase image", "\u2713", "\u2713"],
            ["Power adapter + ethernet cable", "\u2713", "\u2713"],
            ["Quick-start card with invite code", "\u2713", "\u2713"],
            ["Aztibase logo sticker / branded sleeve", "\u2713", "\u2713"],
            ["Genesis validator slot", "\u2713", "\u2713"],
            ["3 reference ONNX models pre-loaded", "\u2014", "\u2713"],
            ["Extended setup support (Discord priority)", "\u2014", "\u2713"],
        ],
        [W * 0.45, W * 0.275, W * 0.275],
        s
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 03 — UNIT ECONOMICS
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "03  Unit Economics & Bill of Materials"))

    story.append(Paragraph("Standard Node — BOM at 50 Units", s["h2"]))

    story.append(make_table(
        ["Component", "Unit Cost", "Notes"],
        [
            ["Beelink SER5 5560U (16GB / 500GB)", "$230", "Bulk pricing via AliExpress / direct"],
            ["NVMe pre-flash (Ubuntu + Aztibase)", "$5", "SD card cloner + labor"],
            ["Logo sticker + branded sleeve", "$3", "Print run of 100+"],
            ["Quick-start card (printed)", "$1", "Double-sided card stock"],
            ["Ethernet cable (1m Cat6)", "$2", "Bulk pack"],
            ["Packaging (kraft box + foam insert)", "$8", "Custom cut foam"],
            ["QA testing + burn-in (30 min/unit)", "$15", "Labor cost"],
            ["Shipping to customer (domestic avg)", "$15", "Courier, insured"],
            ["<b>Total COGS</b>", "<b>$279</b>", ""],
            ["<b>Retail Price</b>", "<b>$699</b>", ""],
            ["<b>Gross Margin</b>", "<b>$420 (60%)</b>", ""],
        ],
        [W * 0.45, W * 0.2, W * 0.35],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("AI Compute Node — BOM at 50 Units", s["h2"]))

    story.append(make_table(
        ["Component", "Unit Cost", "Notes"],
        [
            ["Beelink SER7 7840HS (32GB / 1TB)", "$500", "Direct or Amazon bulk"],
            ["NVMe pre-flash + ONNX model bundle", "$5", "Includes 3 reference models"],
            ["Logo sticker + branded sleeve", "$3", "Same print run"],
            ["Quick-start card (printed)", "$1", ""],
            ["Ethernet cable (1m Cat6)", "$2", ""],
            ["Packaging (kraft box + foam insert)", "$10", "Slightly larger box"],
            ["QA testing + burn-in (30 min/unit)", "$15", ""],
            ["Shipping to customer (domestic avg)", "$15", ""],
            ["<b>Total COGS</b>", "<b>$551</b>", ""],
            ["<b>Retail Price</b>", "<b>$1,499</b>", ""],
            ["<b>Gross Margin</b>", "<b>$948 (63%)</b>", ""],
        ],
        [W * 0.45, W * 0.2, W * 0.35],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("Margin at Scale", s["h2"]))

    story.append(make_table(
        ["Volume", "Std Node COGS", "Std Margin", "AI Node COGS", "AI Margin"],
        [
            ["50 units", "$279", "$420 (60%)", "$551", "$948 (63%)"],
            ["100 units", "$255", "$444 (64%)", "$510", "$989 (66%)"],
            ["500 units", "$220", "$479 (69%)", "$450", "$1,049 (70%)"],
            ["1,000 units (OEM)", "$180", "$519 (74%)", "$380", "$1,119 (75%)"],
        ],
        [W * 0.16, W * 0.21, W * 0.21, W * 0.21, W * 0.21],
        s
    ))

    story.append(Spacer(1, 3 * mm))
    story.append(callout_box(s,
        "<b>Break-even analysis:</b> At 50 units of Standard Node ($699 each), total revenue is $34,950 "
        "with $20,950 gross profit. Fixed costs (OS image development, web UI, packaging design) are estimated "
        "at ~$2,000. The hardware business is profitable from the first batch."
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 04 — CUSTOMER JOURNEY
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "04  Customer Journey — Unbox to Earning"))

    story.append(Paragraph(
        "The entire experience is designed around a single constraint: the operator should never "
        "open a terminal. Every interaction happens through the physical hardware and a browser.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))

    journey_steps = [
        ("1. UNBOX", "Open box. Remove mini PC, power adapter, ethernet cable, and quick-start card."),
        ("2. CONNECT", "Plug in power + ethernet (or WiFi via WPS button). The node boots automatically."),
        ("3. DISCOVER", 'Open any browser on the same network. Navigate to <b>http://aztibase.local</b> (mDNS). The setup wizard greets you.'),
        ("4. NAME", '"Name your node" — enter a friendly name (e.g., "My Validator"). This sets the node identity on the network.'),
        ("5. SECURE", 'Set a 6-digit PIN. Behind the scenes: Ed25519 + BLS validator keys are generated and encrypted with the PIN. Optional: write down 24-word recovery phrase.'),
        ("6. JOIN", 'Enter the network invite code from the quick-start card (e.g., <b>AZTB-7K3M-QXPW</b>). The code decodes to genesis hash + boot node addresses. The node downloads the genesis config and connects to peers automatically.'),
        ("7. SYNC", 'Progress screen: "Finding peers... Syncing blocks... Ready!" — typically under 60 seconds on a fresh network.'),
        ("8. EARN", 'Dashboard appears: node status, block height, peer count, balance, and earnings. The operator is now a validator, earning AZTB every epoch (~7 minutes).'),
    ]

    for step_title, step_desc in journey_steps:
        story.append(KeepTogether([
            Paragraph(f'<font color="#{GOLD_ACCENT.hexval()[2:]}"><b>{step_title}</b></font>', s["h3"]),
            Paragraph(step_desc, s["body"]),
        ]))

    story.append(Spacer(1, 4 * mm))
    story.append(callout_box(s,
        "<b>Total time from opening the box to seeing the first reward: under 7 minutes.</b> "
        "No Rust toolchain. No git clone. No CLI flags. No SSH. No cloud accounts. "
        "This is the DappNode experience but lighter, cheaper, and AI-native."
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 05 — TECHNICAL ARCHITECTURE
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "05  Technical Architecture — The 4-Layer Stack"))

    story.append(Paragraph(
        "The plug-and-play experience requires four software layers, each eliminating a category "
        "of technical friction. All layers ship with the hardware — no external services required.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))

    story.append(make_table(
        ["Layer", "What It Eliminates", "Technology", "Effort"],
        [
            ["1. Pre-Flashed OS Image", "Compile, install, configure Linux", "Ubuntu Server minimal + systemd + avahi (mDNS)", "1\u20132 days"],
            ["2. Web Setup Wizard", "CLI key generation, TOML editing, flag memorization", "Rust axum server, htmx + vanilla JS, 5 screens", "3\u20135 days"],
            ["3. Built-In Dashboard", "curl, jq, Prometheus, Grafana", "Same axum server, real-time via SSE, RPC-backed panels", "2\u20133 days"],
            ["4. Auto-Updater", "SSH maintenance, manual binary swaps", "Systemd timer, Ed25519-signed binaries, rollback support", "1\u20132 days"],
        ],
        [W * 0.18, W * 0.28, W * 0.34, W * 0.11],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("Layer 1 — Pre-Flashed OS Image", s["h2"]))
    story.append(Paragraph(
        "Each unit ships with an NVMe drive pre-loaded with Ubuntu Server 24.04 LTS minimal. "
        "The Aztibase binary is pre-compiled and installed at /usr/local/bin/aztibase. A systemd "
        "service (aztibase-validator.service) auto-starts the node on boot. Avahi daemon enables "
        "mDNS discovery so the box is reachable at aztibase.local from any device on the LAN. "
        "No terminal access is needed — ever.",
        s["body"]
    ))

    story.append(Paragraph("Layer 2 — Web Setup Wizard", s["h2"]))
    story.append(Paragraph(
        "The existing axum HTTP server (which already serves the JSON-RPC API on port 9944) gains "
        "a set of HTML routes for the setup wizard. This means zero additional processes, zero Docker, "
        "zero nginx. The wizard is 5 screens: Welcome, Network (invite code), Security (PIN + optional "
        "mnemonic backup), Connecting (progress), and Dashboard (redirect). All key generation happens "
        "server-side in Rust using the existing wallet module.",
        s["body"]
    ))

    story.append(Paragraph("Layer 3 — Built-In Dashboard", s["h2"]))

    story.append(make_table(
        ["Panel", "Data Source (RPC)", "Update Frequency"],
        [
            ["Node Status (online / syncing / error)", "aztb_nodeInfo", "5 seconds"],
            ["Block Height", "aztb_blockHeight", "Per block (~400ms)"],
            ["Peer Count", "aztb_nodeInfo \u2192 peer_count", "10 seconds"],
            ["Your Balance", "aztb_getBalance", "Per epoch (~7 min)"],
            ["Earnings Over Time", "Local balance delta tracking", "Per epoch"],
            ["Network Validators", "aztb_getActiveValidators", "Per epoch"],
            ["Node Uptime", "Local timestamp", "Continuous"],
        ],
        [W * 0.35, W * 0.35, W * 0.30],
        s
    ))

    story.append(Spacer(1, 3 * mm))
    story.append(Paragraph("Layer 4 — Auto-Updater", s["h2"]))
    story.append(Paragraph(
        "A systemd timer checks for new releases once daily from a configured update endpoint. "
        "The update payload is an Ed25519-signed binary. The updater verifies the signature, "
        "swaps the binary atomically (rename, not overwrite), and restarts the service. If the "
        "new binary fails health checks within 60 seconds, automatic rollback to the previous version. "
        "The operator sees an 'Update available' banner on the dashboard, or auto-applies if configured.",
        s["body"]
    ))

    story.append(Spacer(1, 3 * mm))
    story.append(Paragraph("The Invite Code System", s["h2"]))
    story.append(Paragraph(
        "Instead of sharing genesis.toml files and multiaddr strings, the network coordinator generates "
        "a short, human-readable invite code:",
        s["body"]
    ))

    code_style = ParagraphStyle(
        "code", fontName="Courier-Bold", fontSize=16,
        textColor=DEEP_PURPLE, leading=22, alignment=TA_CENTER,
        spaceBefore=3 * mm, spaceAfter=3 * mm
    )
    story.append(Paragraph("AZTB-7K3M-QXPW", code_style))

    story.append(Paragraph(
        "This encodes (via base32): the genesis block hash (network identity), one or more boot node "
        "multiaddrs, and an optional network display name. The new validator enters this single code "
        "in the web wizard. The box resolves everything automatically — zero copy-paste of hex strings, "
        "zero TOML editing, zero chance of joining the wrong network.",
        s["body"]
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 06 — FULFILLMENT MODEL
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "06  Fulfillment Model — Phase 1 & Phase 2"))

    story.append(Paragraph("Phase 1 — White-Label (0\u2013100 units)", s["h2"]))
    story.append(Paragraph(
        "Buy off-the-shelf Beelink mini PCs, flash them in-house, add branded packaging. "
        "This requires zero upfront tooling investment and can begin as soon as the web UI ships.",
        s["body"]
    ))

    story.append(make_table(
        ["Step", "Action", "Cost", "Time"],
        [
            ["1", "Order 10\u201350 Beelink units (AliExpress / Amazon)", "$2,300\u2013$14,500", "1\u20132 weeks"],
            ["2", "Create master OS image (Ubuntu + Aztibase + wizard)", "$0 (dev time)", "2\u20133 days"],
            ["3", "Flash NVMe via USB cloner (10\u201315 min/unit)", "$200 (cloner hardware)", "1\u20132 days"],
            ["4", "QA test: boot, wizard, join testnet, verify dashboard", "$0 (labor)", "30 min/unit"],
            ["5", "Package: kraft box, foam, quick-start card, sticker", "$12/unit", "Batch"],
            ["6", "Ship to customer", "$15/unit avg", "2\u20135 days"],
        ],
        [W * 0.06, W * 0.46, W * 0.22, W * 0.17],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("Phase 2 — Custom OEM (100+ units)", s["h2"]))
    story.append(Paragraph(
        "Once 100+ white-label units are sold and the product-market fit is validated, move to "
        "custom-branded hardware from Shenzhen OEM manufacturers. This improves margins, strengthens "
        "brand identity, and enables custom enclosure design.",
        s["body"]
    ))

    story.append(make_table(
        ["Aspect", "Details"],
        [
            ["Manufacturers", "Jinghong (odmminipc.com), OAI PC, INCTEL, Shenzhen JIALAIBAO"],
            ["Minimum Order", "100 units (some accept 50)"],
            ["Customization", "Branded enclosure, logo, custom BIOS splash, pre-installed OS image, packaging"],
            ["Per-Unit Cost (100)", "$200\u2013$350 (vs $280 white-label)"],
            ["Per-Unit Cost (500)", "$150\u2013$250"],
            ["Lead Time", "45\u201390 days from order to delivery"],
            ["Upfront Investment", "$20K\u2013$35K for 100 units"],
            ["Enclosure Options", "CNC machined ($20\u201350/unit, $2K tooling) or injection molded ($2\u20135/unit, $15K tooling)"],
        ],
        [W * 0.25, W * 0.75],
        s
    ))

    story.append(Spacer(1, 3 * mm))
    story.append(callout_box(s,
        "<b>Phase 1 validates demand with zero tooling risk.</b> Phase 2 only triggers after "
        "selling 100+ units proves the market. Presale revenue from Phase 1 can fund the Phase 2 OEM order."
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 07 — REVENUE MODEL
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "07  Revenue Model — Stacked Income Streams"))

    story.append(Paragraph(
        "The Business in a Box model generates revenue from four complementary streams. "
        "Hardware sales provide immediate fiat revenue. Protocol fees generate recurring token income. "
        "The combination creates a self-reinforcing flywheel.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))

    story.append(make_table(
        ["Stream", "Revenue Type", "Margin", "When"],
        [
            ["Hardware Sales", "Fiat (one-time)", "60\u201375%", "Immediate"],
            ["Validator NFTs", "Crypto (one-time)", "~95% (digital)", "Pre-mainnet"],
            ["Protocol Fees", "AZTB (recurring)", "5% of all PoUW inference fees", "Post-mainnet"],
            ["VaaS Commission", "AZTB (recurring)", "5\u201315% of delegator rewards", "Post-mainnet"],
        ],
        [W * 0.22, W * 0.25, W * 0.28, W * 0.25],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("Year 1 Revenue Projection (Conservative)", s["h2"]))

    story.append(make_table(
        ["Product", "Units / Events", "Price", "Revenue"],
        [
            ["Standard Node", "50\u2013100 units", "$699", "$35K\u2013$70K"],
            ["AI Compute Node", "10\u201320 units", "$1,499", "$15K\u2013$30K"],
            ["Validator NFTs (limited 1,000)", "200\u2013500 mints", "$200\u2013$1,000", "$100K\u2013$250K"],
            ["Token Sale (Fjord Foundry LBP)", "1 event", "\u2014", "$100K\u2013$300K"],
            ["<b>Total Year 1</b>", "", "", "<b>$250K\u2013$650K</b>"],
        ],
        [W * 0.30, W * 0.20, W * 0.20, W * 0.30],
        s
    ))

    story.append(Spacer(1, 4 * mm))
    story.append(Paragraph("The Flywheel Effect", s["h2"]))

    story.append(Paragraph(
        "More hardware sold \u2192 more validators \u2192 more network security \u2192 higher token value \u2192 "
        "more attractive staking rewards \u2192 more hardware demand. Each unit sold strengthens the network, "
        "which makes the next unit easier to sell. This is the same dynamic that powered Helium's growth "
        "to 1M+ hotspots.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))
    story.append(Paragraph("Validator Earnings Model", s["h2"]))
    story.append(Paragraph(
        "Year 1\u20132 emission: 120M AZTB/year. Validator share: 70% = 84M AZTB. "
        "PoUW share: 15% = 18M AZTB. At various network sizes:",
        s["body"]
    ))

    story.append(make_table(
        ["Validators", "Staking Reward / Validator / Year", "+ PoUW (AI Nodes)", "Total / Validator"],
        [
            ["10", "8,400,000 AZTB", "1,800,000 AZTB", "10,200,000 AZTB"],
            ["50", "1,680,000 AZTB", "360,000 AZTB", "2,040,000 AZTB"],
            ["100", "840,000 AZTB", "180,000 AZTB", "1,020,000 AZTB"],
            ["500", "168,000 AZTB", "36,000 AZTB", "204,000 AZTB"],
        ],
        [W * 0.18, W * 0.32, W * 0.25, W * 0.25],
        s
    ))

    story.append(Spacer(1, 2 * mm))
    story.append(Paragraph(
        "<i>Note: Actual USD value depends on AZTB market price. At $0.01/AZTB, a validator in a "
        "100-node network earns ~$10,200/year. At $0.10/AZTB, that becomes ~$102,000/year. "
        "The hardware pays for itself quickly in most scenarios.</i>",
        s["small"]
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 08 — GO / NO-GO CHECKLIST
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "08  Go / No-Go Checklist"))

    story.append(Paragraph(
        "All items must be checked before ordering the first hardware batch. "
        "These are hard prerequisites — shipping hardware without them creates support debt and reputational risk.",
        s["body"]
    ))

    story.append(Spacer(1, 2 * mm))

    story.append(make_table(
        ["#", "Prerequisite", "Status", "Blocks"],
        [
            ["1", "Web setup wizard (5 screens) functional on testnet", "Not started", "Hardware v1"],
            ["2", "Built-in dashboard showing status, balance, peers, earnings", "Not started", "Hardware v1"],
            ["3", "Invite code system (encode/decode/resolve)", "Not started", "Hardware v1"],
            ["4", "Pre-flash OS image tested on target hardware (Beelink SER5)", "Not started", "Hardware v1"],
            ["5", "Auto-updater with signed binary verification", "Not started", "Hardware v1.1"],
            ["6", "Friends testnet stable with 5+ validators for 7+ days", "In progress", "Presale launch"],
            ["7", "Security audit (at least consensus + wallet crates)", "Planned", "Mainnet"],
            ["8", "Legal opinion from SA crypto attorney (CASP classification)", "Not started", "Token sale"],
            ["9", "Presale page live on aztibase.com with payment processing", "Not started", "Presale launch"],
            ["10", "Support channel (Discord) with FAQ and troubleshooting guide", "Partial", "Hardware v1"],
        ],
        [W * 0.05, W * 0.50, W * 0.20, W * 0.20],
        s
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 09 — RISK MATRIX
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "09  Risk Matrix"))

    story.append(make_table(
        ["Risk", "Likelihood", "Impact", "Mitigation"],
        [
            ["Hardware defects / DOA units", "Medium", "High",
             "QA burn-in test every unit. Keep 5% buffer stock. Beelink has 1-year warranty."],
            ["Low demand (< 20 units sold)", "Medium", "Medium",
             "Phase 1 is white-label \u2014 no upfront tooling. Unsold units become team/testnet nodes."],
            ["Token price near zero at launch", "Medium", "High",
             "Hardware margin provides fiat revenue regardless of token price. Business survives at $0 AZTB."],
            ["Regulatory action (SA CASP)", "Low", "High",
             "Engage attorney pre-launch. Hardware sales are not crypto services. Token entity offshore if needed."],
            ["Solo developer risk", "High", "High",
             "Recruit 2\u20133 advisors. Document everything (59 sprints, full build log). Code is the proof."],
            ["Supply chain disruption", "Low", "Medium",
             "Multiple OEM suppliers identified. Beelink available from Amazon, AliExpress, direct."],
            ["Competitor launches similar product", "Low", "Low",
             "44,900 lines of production code is a deep moat. No other L1 has AI at consensus level."],
            ["Support burden overwhelms solo dev", "Medium", "Medium",
             "Web UI eliminates 90% of support queries. FAQ + Discord community handles the rest."],
        ],
        [W * 0.22, W * 0.10, W * 0.10, W * 0.53],
        s
    ))

    story.append(PageBreak())

    # ══════════════════════════════════════════════════════════════════════
    # 10 — TIMELINE
    # ══════════════════════════════════════════════════════════════════════
    story.extend(section_header(s, "10  Timeline & Milestones"))

    story.append(make_table(
        ["Phase", "Timeline", "Deliverables", "Revenue Event"],
        [
            ["Friends Testnet", "Now \u2013 Month 1",
             "5+ validators running, stability proven, feedback collected", "\u2014"],
            ["Web UI Development", "Month 1\u20132",
             "Setup wizard, dashboard, invite codes, auto-updater", "\u2014"],
            ["Hardware Prototype", "Month 2",
             "5 units flashed + tested on Beelink SER5, dogfooded internally", "\u2014"],
            ["Presale Launch", "Month 3",
             "aztibase.com presale page, Early Bird tier (50 units @ $599)", "$30K\u2013$50K"],
            ["First Batch Ships", "Month 4",
             "50 Standard Nodes delivered, onboarding support", "$35K"],
            ["Validator NFT Mint", "Month 4\u20135",
             "Limited 1,000 NFTs granting mainnet validator slots", "$100K\u2013$250K"],
            ["Mainnet Genesis", "Month 6\u20137",
             "Genesis ceremony with hardware validators, token goes live", "\u2014"],
            ["Token Sale (LBP)", "Month 7\u20138",
             "Fjord Foundry LBP, DEX liquidity seeded", "$100K\u2013$300K"],
            ["AI Compute Node Launch", "Month 8\u20139",
             "Second SKU ships, PoUW inference tasks live on mainnet", "$15K\u2013$30K"],
            ["OEM Transition", "Month 10\u201312",
             "Custom-branded hardware, improved margins, scale to 500+ units", "Ongoing"],
        ],
        [W * 0.18, W * 0.14, W * 0.43, W * 0.18],
        s
    ))

    story.append(Spacer(1, 6 * mm))

    # Final callout
    story.append(callout_box(s,
        "<b>The Business in a Box model turns Aztibase from a software project into a hardware product company "
        "with immediate fiat revenue.</b> The validator hardware generates $200\u2013$950 margin per unit, "
        "seeds the network with real participants, and creates a flywheel where every unit sold makes the "
        "network more valuable. Combined with Validator NFTs and a token launch, the conservative Year 1 "
        "projection is $250K\u2013$650K — more than enough to fund engineering, legal, and infrastructure "
        "for the next phase of growth."
    ))

    story.append(Spacer(1, 10 * mm))

    # Closing
    closing = ParagraphStyle(
        "closing", parent=s["body"], fontSize=10,
        textColor=MID_PURPLE, alignment=TA_CENTER, leading=16
    )
    story.append(Paragraph(
        "<b>Aztibase (Pty) Ltd</b>  |  aztibase.com  |  @aztibase",
        closing
    ))
    story.append(Paragraph(
        "Infrastructure for distributed intelligence.",
        ParagraphStyle("tagline", parent=closing, fontName="Helvetica-Oblique",
                       textColor=GOLD_ACCENT, fontSize=9)
    ))

    # Build
    doc.build(story)
    return output_path


if __name__ == "__main__":
    path = build_document()
    print(f"Generated: {path}")
