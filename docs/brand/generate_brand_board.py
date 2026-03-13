"""
Aztibase Network — Brand Identity Board
Pearlescent Signal design philosophy — pearl white palette
"""

import math
from PIL import Image, ImageDraw, ImageFont

# ─── Canvas ───────────────────────────────────────────────────────
W, H = 2400, 3400
bg = (248, 246, 243)  # warm pearl white
img = Image.new("RGB", (W, H), bg)
draw = ImageDraw.Draw(img)

# ─── Fonts ────────────────────────────────────────────────────────
FONT_DIR = r"C:\Users\Lenovo\.claude\plugins\cache\anthropic-agent-skills\example-skills\f23222824449\skills\canvas-design\canvas-fonts"

def font(name, size):
    try:
        return ImageFont.truetype(f"{FONT_DIR}/{name}", size)
    except:
        return ImageFont.load_default()

f_hero       = font("InstrumentSans-Bold.ttf", 96)
f_heading    = font("InstrumentSans-Bold.ttf", 38)
f_subhead    = font("InstrumentSans-Regular.ttf", 26)
f_body       = font("InstrumentSans-Regular.ttf", 19)
f_label      = font("GeistMono-Regular.ttf", 13)
f_label_lg   = font("GeistMono-Regular.ttf", 16)
f_mono       = font("GeistMono-Regular.ttf", 15)
f_mono_bold  = font("GeistMono-Bold.ttf", 16)
f_ticker     = font("GeistMono-Bold.ttf", 48)
f_section    = font("GeistMono-Bold.ttf", 12)
f_jura       = font("Jura-Medium.ttf", 28)
f_jura_lg    = font("Jura-Light.ttf", 42)
f_brand_sm   = font("InstrumentSans-Regular.ttf", 22)

# ─── Pearl Color Palette ──────────────────────────────────────────
PEARL_WHITE   = (248, 246, 243)  # #F8F6F3 — warm pearl base
PEARL_BLUSH   = (242, 235, 238)  # #F2EBEE — pink-pearl tint
PEARL_BLUE    = (234, 240, 248)  # #EAF0F8 — blue-pearl tint
PEARL_GOLD    = (248, 244, 232)  # #F8F4E8 — gold-pearl tint
DEEP_VIOLET   = (82, 52, 130)    # #523482 — primary brand
SOFT_VIOLET   = (138, 108, 186)  # #8A6CBA — lighter violet
MIST_VIOLET   = (198, 185, 218)  # #C6B9DA — very soft violet
SEAFOAM       = (128, 196, 188)  # #80C4BC — secondary cool
WARM_CORAL    = (228, 148, 128)  # #E49480 — secondary warm
MUTED_GOLD    = (198, 178, 128)  # #C6B280 — accent gold
INK           = (42, 38, 48)     # #2A2630 — text primary
GRAPHITE      = (92, 86, 98)     # #5C5662 — text secondary
SILVER        = (168, 164, 174)  # #A8A4AE — text tertiary
HAIRLINE      = (218, 214, 222)  # #DAD6DE — rules and dividers
LIGHT_WASH    = (238, 236, 242)  # #EEECF2 — subtle backgrounds

# ─── Helpers ──────────────────────────────────────────────────────
def hex_str(c):
    return f"#{c[0]:02X}{c[1]:02X}{c[2]:02X}"

def blend(c1, c2, t):
    return tuple(int(c1[i]*(1-t) + c2[i]*t) for i in range(3))

def draw_dendrite_element(draw, x, y, angle, length, depth, color, base_alpha=200):
    """Dendrite branching — the core visual element"""
    if depth <= 0 or length < 3:
        # Terminal bulb (synaptic bouton)
        if length >= 3:
            r = max(2, 5 - depth)
            c = blend(color, bg, 0.3)
            draw.ellipse([x-r, y-r, x+r, y+r], fill=c)
        return

    end_x = x + math.cos(angle) * length
    end_y = y + math.sin(angle) * length

    width = max(1, 5 - depth)
    fade = max(0.1, base_alpha/255 - depth * 0.12)
    c = blend(color, bg, 1 - fade)

    draw.line([(x, y), (end_x, end_y)], fill=c, width=width)

    # Junction node
    if depth <= 3:
        r = max(2, 4 - depth)
        draw.ellipse([end_x-r, end_y-r, end_x+r, end_y+r], fill=c)

    spread = 0.4 + depth * 0.1
    shrink = 0.6
    draw_dendrite_element(draw, end_x, end_y, angle - spread, length*shrink, depth-1, color, base_alpha*0.85)
    draw_dendrite_element(draw, end_x, end_y, angle + spread, length*shrink, depth-1, color, base_alpha*0.85)
    if depth > 2:
        draw_dendrite_element(draw, end_x, end_y, angle + spread*0.3, length*shrink*0.7, depth-2, color, base_alpha*0.7)


def draw_soft_circle(draw, cx, cy, r, color, border=None):
    """Circle with soft edge"""
    # Outer glow
    for i in range(6, 0, -1):
        gc = blend(color, bg, 0.7 + i*0.05)
        draw.ellipse([cx-r-i, cy-r-i, cx+r+i, cy+r+i], fill=gc)
    draw.ellipse([cx-r, cy-r, cx+r, cy+r], fill=color)
    if border:
        draw.ellipse([cx-r, cy-r, cx+r, cy+r], outline=border, width=2)


# ═══════════════════════════════════════════════════════════════════
# Subtle background texture — faint dendrite watermark
# ═══════════════════════════════════════════════════════════════════
for bx, by, ba, bl in [
    (180, 300, -0.3, 200), (2250, 500, 2.7, 180),
    (200, 2200, -0.6, 160), (2200, 2600, 2.4, 140),
]:
    draw_dendrite_element(draw, bx, by, ba, bl, 5, HAIRLINE, 80)


# ═══════════════════════════════════════════════════════════════════
# HEADER
# ═══════════════════════════════════════════════════════════════════
draw.line([(100, 90), (W-100, 90)], fill=HAIRLINE, width=1)
draw.text((100, 48), "BRAND IDENTITY SYSTEM", fill=SILVER, font=f_section)
draw.text((W-100, 48), "AZTIBASE NETWORK  /  2026", fill=SILVER, font=f_section, anchor="ra")

draw.text((100, 115), "Aztibase", fill=INK, font=f_hero)
draw.text((100, 225), "NETWORK", fill=DEEP_VIOLET, font=f_jura_lg)
draw.text((100, 280), "Layer-1  AI-Native  Server-Independent  Blockchain", fill=GRAPHITE, font=f_body)

# AZTB badge — pearl style
bx, by = W - 240, 130
draw.rounded_rectangle([bx, by, bx+140, by+60], radius=8, fill=DEEP_VIOLET)
draw.text((bx+70, by+30), "AZTB", fill=PEARL_WHITE, font=f_ticker, anchor="mm")

draw.line([(100, 330), (W-100, 330)], fill=HAIRLINE, width=1)


# ═══════════════════════════════════════════════════════════════════
# 01 — COLOR SYSTEM (Pearl Palette)
# ═══════════════════════════════════════════════════════════════════
sy = 365
draw.text((100, sy), "01", fill=SILVER, font=f_label)
draw.text((130, sy), "COLOR SYSTEM", fill=GRAPHITE, font=f_section)

palette = [
    ("Pearl White",    PEARL_WHITE,  "Base",        "Primary backgrounds"),
    ("Pearl Blush",    PEARL_BLUSH,  "Warm Surface","Cards, hover states"),
    ("Pearl Blue",     PEARL_BLUE,   "Cool Surface","Panels, code blocks"),
    ("Deep Violet",    DEEP_VIOLET,  "Primary",     "Brand, headings, CTAs"),
    ("Soft Violet",    SOFT_VIOLET,  "Secondary",   "Links, accents"),
    ("Mist Violet",    MIST_VIOLET,  "Tertiary",    "Tags, borders"),
    ("Seafoam",        SEAFOAM,      "Cool Accent", "Success, data, nodes"),
    ("Warm Coral",     WARM_CORAL,   "Warm Accent", "Alerts, highlights"),
    ("Muted Gold",     MUTED_GOLD,   "Gold Accent", "Rewards, premium"),
    ("Ink",            INK,          "Text Primary","Headings, body"),
    ("Graphite",       GRAPHITE,     "Text Subtle", "Captions, meta"),
    ("Silver",         SILVER,       "Text Muted",  "Placeholders, disabled"),
]

sx, sy = 100, sy + 35
sw, sh = 170, 100
cols = 6
for i, (name, color, role, usage) in enumerate(palette):
    col = i % cols
    row = i // cols
    x = sx + col * (sw + 18)
    y = sy + row * (sh + 62)

    # Swatch with subtle shadow
    shadow = blend(color, (180,180,190), 0.3)
    draw.rounded_rectangle([x+2, y+2, x+sw+2, y+sh+2], radius=6, fill=shadow)
    draw.rounded_rectangle([x, y, x+sw, y+sh], radius=6, fill=color)

    # Border for light swatches
    lum = color[0]*0.299 + color[1]*0.587 + color[2]*0.114
    if lum > 200:
        draw.rounded_rectangle([x, y, x+sw, y+sh], radius=6, outline=HAIRLINE, width=1)
        draw.text((x+10, y+sh-22), hex_str(color), fill=GRAPHITE, font=f_label)
    else:
        draw.text((x+10, y+sh-22), hex_str(color), fill=PEARL_WHITE, font=f_label)

    # Labels
    draw.text((x, y+sh+8), name.upper(), fill=INK, font=f_label)
    draw.text((x, y+sh+24), role, fill=GRAPHITE, font=f_label)

# Iridescent gradient bar
gy = sy + 2*(sh+62) + 20
pearl_colors = [PEARL_BLUSH, PEARL_WHITE, PEARL_BLUE, PEARL_GOLD, PEARL_WHITE, PEARL_BLUSH]
bar_w = W - 200
for px in range(bar_w):
    t = px / bar_w
    seg = t * (len(pearl_colors) - 1)
    idx = min(int(seg), len(pearl_colors) - 2)
    lt = seg - idx
    c = blend(pearl_colors[idx], pearl_colors[idx+1], lt)
    draw.line([(100+px, gy), (100+px, gy+14)], fill=c)
draw.rounded_rectangle([100, gy, 100+bar_w, gy+14], radius=7, outline=HAIRLINE, width=1)
draw.text((100, gy+20), "PEARL IRIDESCENCE  —  BLUSH → WHITE → BLUE → GOLD → WHITE → BLUSH", fill=SILVER, font=f_label)

sep1 = gy + 52
draw.line([(100, sep1), (W-100, sep1)], fill=HAIRLINE, width=1)


# ═══════════════════════════════════════════════════════════════════
# 02 — VISUAL ELEMENT / MASCOT CONCEPTS
# ═══════════════════════════════════════════════════════════════════
ey = sep1 + 30
draw.text((100, ey), "02", fill=SILVER, font=f_label)
draw.text((130, ey), "VISUAL ELEMENT  /  MASCOT CONCEPTS", fill=GRAPHITE, font=f_section)
ey += 45

# --- Element A: The Dendrite Specimen ---
# A detailed dendrite branch rendered like a botanical/anatomical illustration
ea_cx, ea_cy = 340, ey + 200

# Specimen plate background
plate_r = 170
draw.ellipse([ea_cx-plate_r, ea_cy-plate_r, ea_cx+plate_r, ea_cy+plate_r], fill=PEARL_BLUE, outline=HAIRLINE, width=1)

# Central soma (cell body)
soma_r = 18
draw_soft_circle(draw, ea_cx, ea_cy, soma_r, DEEP_VIOLET)

# Main dendrite branches radiating outward — anatomical style
branch_configs = [
    (-1.5, 120, 5),   # up-left
    (-1.1, 110, 5),   # up
    (-0.5, 130, 5),   # up-right
    (0.3, 100, 4),    # right
    (-2.3, 90, 4),    # left
    (2.0, 80, 4),     # down-left
    (1.2, 85, 4),     # down
]
for angle, length, depth in branch_configs:
    draw_dendrite_element(draw, ea_cx, ea_cy, angle, length, depth, DEEP_VIOLET, 220)

# Secondary finer branches
for angle in [0.8, -0.2, -2.8, 2.5]:
    draw_dendrite_element(draw, ea_cx, ea_cy, angle, 60, 3, SOFT_VIOLET, 150)

# Annotation lines (scientific illustration style)
draw.line([(ea_cx+plate_r+10, ea_cy-40), (ea_cx+plate_r+60, ea_cy-40)], fill=SILVER, width=1)
draw.text((ea_cx+plate_r+65, ea_cy-47), "dendrite", fill=GRAPHITE, font=f_label)

draw.line([(ea_cx, ea_cy+soma_r+2), (ea_cx+plate_r+10, ea_cy+30), (ea_cx+plate_r+60, ea_cy+30)], fill=SILVER, width=1)
draw.text((ea_cx+plate_r+65, ea_cy+23), "soma", fill=GRAPHITE, font=f_label)

draw.text((ea_cx, ey+400), "ELEMENT A", fill=INK, font=f_label_lg, anchor="ma")
draw.text((ea_cx, ey+420), "The Dendrite Specimen", fill=GRAPHITE, font=f_label, anchor="ma")
draw.text((ea_cx, ey+436), "Anatomical illustration style — works as", fill=SILVER, font=f_label, anchor="ma")
draw.text((ea_cx, ey+450), "watermark, icon, pattern element, merch", fill=SILVER, font=f_label, anchor="ma")


# --- Element B: The Axolotl (Mascot) ---
# Axolotls regenerate neurons/dendrites — perfect fit
eb_cx, eb_cy = 920, ey + 200

# Soft circular frame
plate_r2 = 170
draw.ellipse([eb_cx-plate_r2, eb_cy-plate_r2, eb_cx+plate_r2, eb_cy+plate_r2], fill=PEARL_BLUSH, outline=HAIRLINE, width=1)

# Stylized axolotl — geometric/minimal
# Body (rounded)
body_w, body_h = 80, 50
draw.ellipse([eb_cx-body_w, eb_cy-body_h+20, eb_cx+body_w, eb_cy+body_h+20], fill=MIST_VIOLET)

# Head (larger circle)
head_cy = eb_cy - 30
head_r = 55
draw.ellipse([eb_cx-head_r, head_cy-head_r, eb_cx+head_r, head_cy+head_r], fill=SOFT_VIOLET)

# Eyes
eye_r = 8
for ex_off in [-22, 22]:
    # White of eye
    draw.ellipse([eb_cx+ex_off-eye_r-3, head_cy-8-eye_r-3, eb_cx+ex_off+eye_r+3, head_cy-8+eye_r+3], fill=PEARL_WHITE)
    # Iris
    draw.ellipse([eb_cx+ex_off-eye_r, head_cy-8-eye_r, eb_cx+ex_off+eye_r, head_cy-8+eye_r], fill=INK)
    # Highlight
    draw.ellipse([eb_cx+ex_off-3, head_cy-12, eb_cx+ex_off+3, head_cy-6], fill=PEARL_WHITE)

# Smile
draw.arc([eb_cx-15, head_cy+2, eb_cx+15, head_cy+20], start=0, end=180, fill=INK, width=2)

# Gills (the iconic axolotl feature — rendered as dendrite branches!)
gill_configs = [
    (eb_cx-50, head_cy-20, -2.0, 55, 4, DEEP_VIOLET),
    (eb_cx-45, head_cy-35, -1.6, 50, 4, DEEP_VIOLET),
    (eb_cx-35, head_cy-48, -1.2, 45, 3, WARM_CORAL),
    (eb_cx+50, head_cy-20, -1.1, 55, 4, DEEP_VIOLET),
    (eb_cx+45, head_cy-35, -1.5, 50, 4, DEEP_VIOLET),
    (eb_cx+35, head_cy-48, -1.9, 45, 3, WARM_CORAL),
]
for gx, gy_pos, ga, gl, gd, gc in gill_configs:
    draw_dendrite_element(draw, gx, gy_pos, ga, gl, gd, gc, 200)

# Tiny limbs
for lx_off, ly_off in [(-60, 30), (60, 30), (-50, 60), (50, 60)]:
    lx = eb_cx + lx_off
    ly = eb_cy + ly_off
    draw.ellipse([lx-8, ly-6, lx+8, ly+6], fill=SOFT_VIOLET)
    # Tiny fingers
    for fa in range(3):
        fx = lx + math.cos(-1.2 + fa*0.4 + (0.8 if lx_off > 0 else 0)) * 10
        fy = ly + math.sin(-1.2 + fa*0.4 + (0.8 if lx_off > 0 else 0)) * 10
        draw.line([(lx, ly), (fx, fy)], fill=MIST_VIOLET, width=2)

# Tail
tail_pts = [(eb_cx+body_w-10, eb_cy+20)]
for t in range(20):
    tx = eb_cx + body_w - 10 + t * 4
    ty = eb_cy + 20 + math.sin(t*0.4) * 15
    tail_pts.append((tx, ty))
for i in range(len(tail_pts)-1):
    w = max(1, 8 - i//3)
    c = blend(MIST_VIOLET, bg, i/len(tail_pts)*0.5)
    draw.line([tail_pts[i], tail_pts[i+1]], fill=c, width=w)

# Annotation
draw.text((eb_cx, ey+400), "ELEMENT B", fill=INK, font=f_label_lg, anchor="ma")
draw.text((eb_cx, ey+420), 'The Axolotl  "Azi"', fill=GRAPHITE, font=f_label, anchor="ma")
draw.text((eb_cx, ey+436), "Regenerates neurons — gills ARE dendrites", fill=SILVER, font=f_label, anchor="ma")
draw.text((eb_cx, ey+450), "Friendly, memorable, unique in crypto", fill=SILVER, font=f_label, anchor="ma")


# --- Element C: The Coral Node ---
ec_cx, ec_cy = 1500, ey + 200

plate_r3 = 170
draw.ellipse([ec_cx-plate_r3, ec_cy-plate_r3, ec_cx+plate_r3, ec_cy+plate_r3], fill=PEARL_GOLD, outline=HAIRLINE, width=1)

# Coral structure — branching upward like an underwater tree
# Multiple coral branches from a base
base_y = ec_cy + 80
draw.ellipse([ec_cx-40, base_y-8, ec_cx+40, base_y+8], fill=MUTED_GOLD)

coral_branches = [
    (ec_cx-20, base_y, -1.6, 100, 5, WARM_CORAL),
    (ec_cx, base_y, -1.5, 120, 5, DEEP_VIOLET),
    (ec_cx+10, base_y, -1.3, 110, 5, SOFT_VIOLET),
    (ec_cx-35, base_y, -1.8, 80, 4, WARM_CORAL),
    (ec_cx+30, base_y, -1.1, 85, 4, SEAFOAM),
]
for cx, cy, ca, cl, cd, cc in coral_branches:
    draw_dendrite_element(draw, cx, cy, ca, cl, cd, cc, 210)

# Small fish/particles around coral
particles = [
    (ec_cx-90, ec_cy-60, 4), (ec_cx+100, ec_cy-80, 3),
    (ec_cx-110, ec_cy+20, 3), (ec_cx+80, ec_cy-20, 5),
    (ec_cx-60, ec_cy-110, 3), (ec_cx+120, ec_cy+10, 4),
]
for px, py, pr in particles:
    draw.ellipse([px-pr, py-pr, px+pr, py+pr], fill=blend(SEAFOAM, bg, 0.4))

draw.text((ec_cx, ey+400), "ELEMENT C", fill=INK, font=f_label_lg, anchor="ma")
draw.text((ec_cx, ey+420), "The Coral Network", fill=GRAPHITE, font=f_label, anchor="ma")
draw.text((ec_cx, ey+436), "Distributed branching colony — each branch", fill=SILVER, font=f_label, anchor="ma")
draw.text((ec_cx, ey+450), "a node, the whole a living consensus", fill=SILVER, font=f_label, anchor="ma")


# --- Element D: Recommendation callout ---
rec_x, rec_y = 1850, ey + 20
draw.rounded_rectangle([rec_x, rec_y, W-100, rec_y+440], radius=12, fill=LIGHT_WASH, outline=HAIRLINE, width=1)
draw.text((rec_x+20, rec_y+18), "RECOMMENDATION", fill=DEEP_VIOLET, font=f_section)

rec_lines = [
    ("Best mascot:", INK, f_label_lg),
    ('The Axolotl "Azi"', DEEP_VIOLET, font("InstrumentSans-Bold.ttf", 20)),
    ("", None, None),
    ("Why it works:", INK, f_label_lg),
    ("• Axolotls regenerate", GRAPHITE, f_label),
    ("  neurons (dendrites!)", GRAPHITE, f_label),
    ("• Gills = branching", GRAPHITE, f_label),
    ("  dendrite structures", GRAPHITE, f_label),
    ("• Name starts with A", GRAPHITE, f_label),
    ("• Friendly + memorable", GRAPHITE, f_label),
    ("• Zero crypto mascot", GRAPHITE, f_label),
    ("  collision", GRAPHITE, f_label),
    ("• Axolotls are server-", GRAPHITE, f_label),
    ("  independent (survive", GRAPHITE, f_label),
    ("  alone or in groups)", GRAPHITE, f_label),
    ("", None, None),
    ("Best pattern element:", INK, f_label_lg),
    ("Dendrite Specimen", DEEP_VIOLET, font("InstrumentSans-Bold.ttf", 18)),
    ("For watermarks, bg", GRAPHITE, f_label),
    ("patterns, loading", GRAPHITE, f_label),
    ("states, 404 pages", GRAPHITE, f_label),
]

rl_y = rec_y + 42
for text, color, f in rec_lines:
    if text:
        draw.text((rec_x+20, rl_y), text, fill=color, font=f)
    rl_y += 18

sep2 = ey + 480
draw.line([(100, sep2), (W-100, sep2)], fill=HAIRLINE, width=1)


# ═══════════════════════════════════════════════════════════════════
# 03 — TYPOGRAPHY
# ═══════════════════════════════════════════════════════════════════
ty_y = sep2 + 30
draw.text((100, ty_y), "03", fill=SILVER, font=f_label)
draw.text((130, ty_y), "TYPOGRAPHY", fill=GRAPHITE, font=f_section)
ty_y += 45

# Heading typeface
draw.text((100, ty_y), "Instrument Sans", fill=INK, font=f_heading)
draw.text((580, ty_y+8), "HEADINGS  /  DISPLAY  /  UI", fill=SILVER, font=f_label)
ty_y += 48
draw.text((100, ty_y), "Aa Bb Cc Dd Ee Ff Gg Hh Ii  0123456789", fill=GRAPHITE, font=f_subhead)
ty_y += 42

# Mono typeface
draw.text((100, ty_y), "Geist Mono", fill=INK, font=f_heading)
draw.text((580, ty_y+8), "CODE  /  DATA  /  LABELS  /  CLI", fill=SILVER, font=f_label)
ty_y += 48
draw.text((100, ty_y), "Aa Bb Cc Dd Ee Ff Gg Hh Ii  0123456789", fill=GRAPHITE, font=f_mono_bold)
ty_y += 36

# Accent typeface
draw.text((100, ty_y), "Jura", fill=INK, font=f_heading)
draw.text((580, ty_y+8), "ACCENT  /  TAGLINES  /  TOKEN", fill=SILVER, font=f_label)
ty_y += 48
draw.text((100, ty_y), "Aztibase Network  —  AZTB  —  0xA27B", fill=GRAPHITE, font=f_jura)
ty_y += 50

# Type scale
draw.text((100, ty_y), "TYPE SCALE", fill=SILVER, font=f_label)
ty_y += 22
scales = [
    ("Display / 48px", font("InstrumentSans-Bold.ttf", 48)),
    ("H1 / 36px", font("InstrumentSans-Bold.ttf", 36)),
    ("H2 / 26px", f_subhead),
    ("Body / 19px", f_body),
    ("Label / 13px", f_label),
]
for label, f in scales:
    bbox = draw.textbbox((0,0), "Aztibase", font=f)
    line_h = bbox[3] - bbox[1] + 12
    draw.text((100, ty_y), label, fill=SILVER, font=f_label)
    draw.text((300, ty_y), "Aztibase", fill=INK, font=f)
    ty_y += line_h

ty_y += 10
sep3 = ty_y
draw.line([(100, sep3), (W-100, sep3)], fill=HAIRLINE, width=1)


# ═══════════════════════════════════════════════════════════════════
# 04 — BRAND ATTRIBUTES
# ═══════════════════════════════════════════════════════════════════
kw_y = sep3 + 30
draw.text((100, kw_y), "04", fill=SILVER, font=f_label)
draw.text((130, kw_y), "BRAND ATTRIBUTES", fill=GRAPHITE, font=f_section)
kw_y += 40

keywords = [
    ("NEURAL", DEEP_VIOLET),
    ("LUMINOUS", SOFT_VIOLET),
    ("SOVEREIGN", SEAFOAM),
    ("PRECISE", INK),
    ("CONVERGENT", DEEP_VIOLET),
    ("RESILIENT", WARM_CORAL),
    ("DISTRIBUTED", SOFT_VIOLET),
    ("LIVING", SEAFOAM),
]

kx = 100
for kw, kc in keywords:
    bbox = draw.textbbox((0,0), kw, font=f_label_lg)
    tw = bbox[2] - bbox[0] + 28
    if kx + tw > W - 100:
        kx = 100
        kw_y += 40
    draw.rounded_rectangle([kx, kw_y, kx+tw, kw_y+30], radius=15, outline=kc, width=1)
    draw.text((kx + tw//2, kw_y+15), kw, fill=kc, font=f_label_lg, anchor="mm")
    kx += tw + 10

kw_y += 50
draw.text((100, kw_y), '"Infrastructure for distributed intelligence."', fill=DEEP_VIOLET, font=f_subhead)
kw_y += 32
draw.text((100, kw_y), '"Where signals converge, consensus emerges."', fill=GRAPHITE, font=f_body)

kw_y += 45
draw.line([(100, kw_y), (W-100, kw_y)], fill=HAIRLINE, width=1)


# ═══════════════════════════════════════════════════════════════════
# 05 — APPLICATIONS
# ═══════════════════════════════════════════════════════════════════
app_y = kw_y + 30
draw.text((100, app_y), "05", fill=SILVER, font=f_label)
draw.text((130, app_y), "APPLICATIONS", fill=GRAPHITE, font=f_section)
app_y += 45

# --- Terminal mockup (pearl-themed dark terminal) ---
term_x, term_y = 100, app_y
term_w, term_h = 720, 260
draw.rounded_rectangle([term_x, term_y, term_x+term_w, term_y+term_h], radius=10, fill=(38, 34, 48))

# Chrome bar
draw.rounded_rectangle([term_x, term_y, term_x+term_w, term_y+30], radius=10, fill=(52, 48, 62))
draw.rectangle([term_x, term_y+20, term_x+term_w, term_y+30], fill=(52, 48, 62))
for i, c in enumerate([(255,95,86), (255,189,46), (39,201,63)]):
    draw.ellipse([term_x+14+i*18, term_y+9, term_x+24+i*18, term_y+19], fill=c)
draw.text((term_x+term_w//2, term_y+14), "aztibase", fill=(168,164,174), font=f_label, anchor="mm")

cli_y = term_y + 44
cli_lines = [
    ((128,196,188), "  ╔═══════════════════════════════════════════╗"),
    ((128,196,188), "  ║        AZTIBASE NETWORK  v0.1.0          ║"),
    ((128,196,188), "  ║      AI-Native · Server-Independent      ║"),
    ((128,196,188), "  ╚═══════════════════════════════════════════╝"),
    ((92,86,98), ""),
    ((198,178,128), "  Chain ID: 0xA27B  |  Network: friends-testnet"),
    ((240,238,234), "  $ aztibase --validator --network friends"),
    ((138,108,186), "  [2026-03-13 08:41:22] Syncing...  █████░ 84%"),
]
for color, line in cli_lines:
    if line:
        draw.text((term_x+14, cli_y), line, fill=color, font=f_mono)
    cli_y += 22

draw.text((term_x+term_w//2, term_y+term_h+12), "CLI / Terminal", fill=SILVER, font=f_label, anchor="ma")


# --- Web header mockup (pearl light theme) ---
web_x, web_y = 880, app_y
web_w, web_h = 720, 260
draw.rounded_rectangle([web_x+2, web_y+2, web_x+web_w+2, web_y+web_h+2], radius=10, fill=blend(HAIRLINE, bg, 0.5))
draw.rounded_rectangle([web_x, web_y, web_x+web_w, web_y+web_h], radius=10, fill=PEARL_WHITE, outline=HAIRLINE, width=1)

# Browser chrome
draw.rounded_rectangle([web_x, web_y, web_x+web_w, web_y+30], radius=10, fill=LIGHT_WASH)
draw.rectangle([web_x, web_y+20, web_x+web_w, web_y+30], fill=LIGHT_WASH)
draw.rounded_rectangle([web_x+80, web_y+7, web_x+380, web_y+23], radius=8, fill=PEARL_WHITE, outline=HAIRLINE, width=1)
draw.text((web_x+230, web_y+15), "aztibase.com", fill=SILVER, font=f_label, anchor="mm")

# Nav
nav_y = web_y + 44
draw.text((web_x+24, nav_y), "AZTIBASE", fill=INK, font=font("InstrumentSans-Bold.ttf", 22))
for i, item in enumerate(["Docs", "Network", "Validators"]):
    draw.text((web_x+280+i*90, nav_y+3), item, fill=GRAPHITE, font=f_body)

draw.rounded_rectangle([web_x+web_w-150, nav_y-3, web_x+web_w-24, nav_y+27], radius=6, fill=DEEP_VIOLET)
draw.text((web_x+web_w-87, nav_y+12), "Launch App", fill=PEARL_WHITE, font=f_label_lg, anchor="mm")

# Hero
hero_y = nav_y + 50
draw.text((web_x+24, hero_y), "Infrastructure for", fill=INK, font=font("InstrumentSans-Bold.ttf", 30))
draw.text((web_x+24, hero_y+36), "distributed intelligence", fill=DEEP_VIOLET, font=font("InstrumentSans-Bold.ttf", 30))

# Stats
stat_y = hero_y + 88
stats_data = [("400ms", "Block Time"), ("<1s", "Finality"), ("10k+", "TPS")]
for i, (val, label) in enumerate(stats_data):
    sx = web_x + 24 + i * 170
    draw.text((sx, stat_y), val, fill=DEEP_VIOLET, font=font("GeistMono-Bold.ttf", 22))
    draw.text((sx, stat_y+26), label, fill=SILVER, font=f_label)

draw.text((web_x+web_w//2, web_y+web_h+12), "Website Header", fill=SILVER, font=f_label, anchor="ma")


# --- Token symbol (pearl style) ---
tok_x, tok_y = 1660, app_y

# Token circle with pearl gradient
tok_cx, tok_cy = tok_x + 100, tok_y + 100
tok_r = 65

for ri in range(tok_r+8, tok_r, -1):
    a = (ri - tok_r) / 8
    gc = blend(MIST_VIOLET, bg, a * 0.7)
    draw.ellipse([tok_cx-ri, tok_cy-ri, tok_cx+ri, tok_cy+ri], fill=gc)

for ri in range(tok_r, 0, -1):
    t = ri / tok_r
    c = blend(SOFT_VIOLET, DEEP_VIOLET, t * 0.5 + 0.3)
    draw.ellipse([tok_cx-ri, tok_cy-ri, tok_cx+ri, tok_cy+ri], fill=c)

draw.text((tok_cx, tok_cy-2), "A", fill=PEARL_WHITE, font=font("InstrumentSans-Bold.ttf", 56), anchor="mm")

draw.text((tok_cx+100, tok_cy-24), "AZTB", fill=INK, font=f_ticker)
draw.text((tok_cx+100, tok_cy+24), "Aztibase Network", fill=GRAPHITE, font=f_body)
draw.text((tok_cx+100, tok_cy+48), "Chain ID: 0xA27B", fill=SILVER, font=f_mono)

# Dark variant
dark_tok_cy = tok_cy + 170
draw.rounded_rectangle([tok_cx-80, dark_tok_cy-50, tok_cx+280, dark_tok_cy+50], radius=8, fill=(38,34,48))
# Small token circle
for ri in range(25, 0, -1):
    t = ri / 25
    c = blend(SOFT_VIOLET, DEEP_VIOLET, t*0.5+0.3)
    draw.ellipse([tok_cx-ri, dark_tok_cy-ri, tok_cx+ri, dark_tok_cy+ri], fill=c)
draw.text((tok_cx, dark_tok_cy), "A", fill=PEARL_WHITE, font=font("InstrumentSans-Bold.ttf", 28), anchor="mm")
draw.text((tok_cx+40, dark_tok_cy-10), "AZTB", fill=(232,230,236), font=f_mono_bold)
draw.text((tok_cx+40, dark_tok_cy+10), "On dark surfaces", fill=(120,116,128), font=f_label)

draw.text((tok_x+150, tok_y+term_h+12), "Token Symbol", fill=SILVER, font=f_label, anchor="ma")


# ═══════════════════════════════════════════════════════════════════
# FOOTER
# ═══════════════════════════════════════════════════════════════════
foot_y = app_y + term_h + 45
draw.line([(100, foot_y), (W-100, foot_y)], fill=HAIRLINE, width=1)

draw.text((100, foot_y+14), "AZTIBASE NETWORK", fill=GRAPHITE, font=f_label_lg)
draw.text((100, foot_y+36), "Brand Identity System v2.0  —  Pearlescent Signal  —  2026", fill=SILVER, font=f_label)
draw.text((W-100, foot_y+14), "aztibase.com", fill=DEEP_VIOLET, font=f_label_lg, anchor="ra")
draw.text((W-100, foot_y+36), "@aztibase", fill=SILVER, font=f_label, anchor="ra")


# ═══════════════════════════════════════════════════════════════════
# SAVE — crop to content height
# ═══════════════════════════════════════════════════════════════════
final_h = foot_y + 65
img_cropped = img.crop((0, 0, W, final_h))
out_path = r"c:\Users\Lenovo\Documents\Project Genises\docs\brand\AZTIBASE_BRAND_IDENTITY.png"
img_cropped.save(out_path, "PNG", quality=100)
print(f"Saved: {out_path}")
print(f"Size: {W}x{final_h}")
