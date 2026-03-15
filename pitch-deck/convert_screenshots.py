"""Convert full-page website screenshots into multi-page PDFs.

Uses a custom page width matching the screenshot at 72 DPI (1px = 1pt),
sliced into portrait-height pages for readable full-width rendering.
"""

from pathlib import Path
from reportlab.lib.units import mm
from reportlab.pdfgen import canvas
from reportlab.lib.pagesizes import A4
from PIL import Image

SCREENSHOTS_DIR = Path("D:/aztibase/archives/pitch-deck/screenshots")
OUTPUT_DIR = Path("pitch-deck")

PEARL = (0.98, 0.98, 0.97)
INK = (0.13, 0.12, 0.14)
GOLD = (0.72, 0.65, 0.50)

PAGE_W = 1440
PAGE_H = 1080
MARGIN = 20


def screenshot_to_pdf(image_path: Path, output_path: Path, title: str):
    img = Image.open(image_path)
    img_w, img_h = img.size

    usable_w = PAGE_W - 2 * MARGIN
    usable_h = PAGE_H - MARGIN - 40

    scale = usable_w / img_w
    slice_h_px = int(usable_h / scale)

    total_pages = -(-img_h // slice_h_px)

    c = canvas.Canvas(str(output_path), pagesize=(PAGE_W, PAGE_H))

    y_px = 0
    page_num = 0

    while y_px < img_h:
        remaining = img_h - y_px
        current_slice_px = min(slice_h_px, remaining)

        crop = img.crop((0, y_px, img_w, y_px + current_slice_px))
        temp = output_path.with_suffix(f".tmp_{page_num}.png")
        crop.save(str(temp), optimize=True)

        rendered_h = current_slice_px * scale

        c.setFillColorRGB(*PEARL)
        c.rect(0, 0, PAGE_W, PAGE_H, fill=1, stroke=0)

        c.setStrokeColorRGB(*GOLD)
        c.setLineWidth(0.75)
        c.line(MARGIN, PAGE_H - 28, PAGE_W - MARGIN, PAGE_H - 28)

        c.setFont("Helvetica-Bold", 9)
        c.setFillColorRGB(*INK)
        c.drawString(MARGIN, PAGE_H - 20, f"AZTIBASE NETWORK")
        c.setFont("Helvetica", 9)
        c.drawString(MARGIN + 115, PAGE_H - 20, f"—  {title}")
        c.drawRightString(PAGE_W - MARGIN, PAGE_H - 20,
                          f"{page_num + 1} / {total_pages}")

        c.drawImage(str(temp), MARGIN, PAGE_H - 35 - rendered_h,
                    width=usable_w, height=rendered_h)

        temp.unlink()
        c.showPage()
        y_px += current_slice_px
        page_num += 1

    c.save()
    print(f"  {output_path.name}: {page_num} pages ({img_w}x{img_h} @ {scale:.2f}x)")


def wallet_to_pdf(image_path: Path, output_path: Path):
    img = Image.open(image_path)
    img_w, img_h = img.size

    A4_W, A4_H = A4
    c = canvas.Canvas(str(output_path), pagesize=A4)

    c.setFillColorRGB(*PEARL)
    c.rect(0, 0, A4_W, A4_H, fill=1, stroke=0)

    c.setStrokeColorRGB(*GOLD)
    c.setLineWidth(0.75)
    c.line(40, A4_H - 28, A4_W - 40, A4_H - 28)

    c.setFont("Helvetica-Bold", 9)
    c.setFillColorRGB(*INK)
    c.drawString(40, A4_H - 20, "AZTIBASE NETWORK")
    c.setFont("Helvetica", 9)
    c.drawString(155, A4_H - 20, "—  Wallet Extension")

    max_h = A4_H - 80
    max_w = A4_W - 80
    scale = min(max_w / img_w, max_h / img_h, 2.5)
    rendered_w = img_w * scale
    rendered_h = img_h * scale

    x = (A4_W - rendered_w) / 2
    y = (A4_H - rendered_h) / 2 - 10

    c.setStrokeColorRGB(0.82, 0.82, 0.82)
    c.setLineWidth(0.5)
    c.roundRect(x - 6, y - 6, rendered_w + 12, rendered_h + 12, 10, fill=0, stroke=1)

    c.drawImage(str(image_path), x, y, width=rendered_w, height=rendered_h)

    c.setFont("Helvetica", 8)
    c.setFillColorRGB(0.45, 0.45, 0.45)
    c.drawCentredString(A4_W / 2, y - 24,
                        "Chrome MV3  |  AES-256-GCM  |  TOTP + WebAuthn 2FA  |  WASM Staking")

    c.showPage()
    c.save()
    print(f"  {output_path.name}: 1 page ({img_w}x{img_h} @ {scale:.2f}x)")


if __name__ == "__main__":
    OUTPUT_DIR.mkdir(exist_ok=True)
    print("Converting screenshots to full-width PDFs...")

    screenshot_to_pdf(
        SCREENSHOTS_DIR / "website-light.png",
        OUTPUT_DIR / "WEBSITE_LIGHT.pdf",
        "Website — Light Mode"
    )
    screenshot_to_pdf(
        SCREENSHOTS_DIR / "website-dark.png",
        OUTPUT_DIR / "WEBSITE_DARK.pdf",
        "Website — Dark Mode"
    )
    wallet_to_pdf(
        SCREENSHOTS_DIR / "wallet-welcome.png",
        OUTPUT_DIR / "WALLET_EXTENSION.pdf"
    )
    print("Done.")
