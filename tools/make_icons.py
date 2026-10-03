"""Draws the inventory icons in assets/icons/.

The tilesets have no art for the items, so the icons are drawn here with simple
shapes. Run with: python3 tools/make_icons.py (requires Pillow).
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 64
SCALE = 8  # Drawn larger and scaled down for smooth edges.
S = SIZE * SCALE
OUT = Path(__file__).resolve().parent.parent / "assets" / "icons"

OUTLINE = (25, 18, 14, 255)
W = 3 * SCALE  # outline width


def canvas():
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    return img, ImageDraw.Draw(img)


def p(*points):
    """Points given in 0..64 icon coordinates."""
    return [(x * SCALE, y * SCALE) for x, y in points]


def save(img, name):
    # Soft shadow below the icon to separate it from any background.
    alpha = img.getchannel("A").filter(ImageFilter.GaussianBlur(2 * SCALE))
    shadow = Image.new("RGBA", img.size, (0, 0, 0, 0))
    shadow.putalpha(alpha.point(lambda a: a * 0.5))
    result = Image.new("RGBA", img.size, (0, 0, 0, 0))
    result.alpha_composite(shadow, (SCALE, 2 * SCALE))
    result.alpha_composite(img)
    result.resize((SIZE, SIZE), Image.LANCZOS).save(OUT / f"{name}.png")


def knife():
    img, d = canvas()
    # Blade: a broad kitchen knife from the handle (bottom-left) to the tip.
    d.line(p((22, 46), (48, 16)), fill=OUTLINE, width=14 * SCALE)
    d.polygon(p((42.7, 11.4), (56.5, 5.5), (53.3, 20.6)), fill=OUTLINE)
    d.line(p((23, 45), (48, 16)), fill=(200, 208, 218), width=9 * SCALE)
    d.polygon(p((44.6, 13.1), (54.2, 8.6), (51.4, 18.9)), fill=(200, 208, 218))
    d.line(p((22, 42), (48, 13)), fill=(245, 248, 250), width=2 * SCALE)
    # Handle with rivets
    d.line(p((24, 48), (8, 60)), fill=OUTLINE, width=12 * SCALE)
    d.line(p((23, 49), (9, 59)), fill=(120, 75, 40), width=7 * SCALE)
    for x, y in ((19, 52), (13, 56)):
        d.ellipse(p((x - 1.5, y - 1.5), (x + 1.5, y + 1.5)), fill=(220, 200, 150))
    return img


def candle(burning):
    img, d = canvas()
    d.ellipse(p((14, 50), (50, 60)), fill=(140, 110, 60), outline=OUTLINE, width=W)  # holder
    d.rectangle(p((24, 22), (40, 54)), fill=(235, 225, 200), outline=OUTLINE, width=W)
    d.ellipse(p((24, 18), (40, 26)), fill=(250, 245, 225), outline=OUTLINE, width=W)
    d.line(p((32, 22), (32, 14)), fill=OUTLINE, width=2 * SCALE)  # wick
    if burning:
        glow = Image.new("RGBA", img.size, (0, 0, 0, 0))
        ImageDraw.Draw(glow).ellipse(p((18, 4), (46, 26)), fill=(255, 190, 80, 110))
        glow = glow.filter(ImageFilter.GaussianBlur(3 * SCALE))
        img = Image.alpha_composite(glow, img)
        d = ImageDraw.Draw(img)
        d.polygon(p((32, 2), (39, 12), (36, 18), (28, 18), (25, 12)), fill=(255, 170, 40), outline=(160, 70, 10, 255), width=2 * SCALE)
        d.polygon(p((32, 8), (35, 13), (33, 17), (31, 17), (29, 13)), fill=(255, 245, 170))
    return img


def stink(d, x, top, bottom):
    """A wavy vertical line rising from the cheese."""
    points = [(x + 2.5 * math.sin((y - top) / 3), y) for y in range(top, bottom + 1)]
    d.line(p(*points), fill=(150, 180, 80), width=2 * SCALE, joint="curve")


def cheese(melted):
    img, d = canvas()
    if melted:
        d.polygon(p((6, 46), (18, 34), (40, 30), (58, 40), (54, 52), (40, 50), (34, 58), (26, 52), (12, 54)), fill=(240, 200, 70), outline=OUTLINE, width=W)
        for x, y, r in ((22, 44, 3), (40, 40, 2.5), (46, 46, 2)):
            d.ellipse(p((x - r, y - r), (x + r, y + r)), fill=(205, 160, 40))
        # Stink lines
        for x in (20, 32, 44):
            stink(d, x, 10, 26)
    else:
        d.polygon(p((8, 40), (56, 26), (56, 48), (8, 56)), fill=(245, 205, 80), outline=OUTLINE, width=W)
        d.polygon(p((8, 40), (40, 14), (56, 26)), fill=(255, 225, 120), outline=OUTLINE, width=W)
        for x, y, r in ((22, 47, 4), (40, 42, 3), (48, 36, 2.5), (32, 30, 2.5)):
            d.ellipse(p((x - r, y - r), (x + r, y + r)), fill=(215, 170, 50))
        for x in (16, 26):
            stink(d, x, 2, 18)
    return img


def feather(color, creased):
    img, d = canvas()
    # Black feathers get a light outline, so they are visible on dark backgrounds.
    outline = (150, 150, 170, 255) if sum(color[:3]) < 300 else OUTLINE
    dark = tuple(max(0, c - 60) for c in color[:3]) if outline == OUTLINE else (110, 110, 130)
    # Vane as a leaf shape along the diagonal from bottom-left to top-right.
    pts = []
    for i in range(21):
        t = i / 20
        x, y = 8 + 48 * t, 56 - 48 * t
        w = 10 * math.sin(math.pi * min(1.0, t * 1.1)) * (0.6 if creased and 0.4 < t < 0.6 else 1)
        pts.append((x - w * 0.7, y - w * 0.7))
    for i in range(20, -1, -1):
        t = i / 20
        x, y = 8 + 48 * t, 56 - 48 * t
        w = 9 * math.sin(math.pi * min(1.0, t * 1.1)) * (0.5 if creased and 0.35 < t < 0.65 else 1)
        pts.append((x + w * 0.7, y + w * 0.7))
    d.polygon(p(*pts), fill=color, outline=outline, width=W)
    # Barbs
    for t in (0.35, 0.5, 0.65, 0.8):
        x, y = 8 + 48 * t, 56 - 48 * t
        d.line(p((x, y), (x - 6, y - 2)), fill=dark, width=SCALE)
        d.line(p((x, y), (x + 2, y + 6)), fill=dark, width=SCALE)
    d.line(p((4, 60), (54, 10)), fill=outline, width=2 * SCALE)  # quill
    if creased:
        d.line(p((26, 26), (34, 30), (30, 38)), fill=outline, width=2 * SCALE)
    return img


def wine():
    img, d = canvas()
    d.polygon(p((16, 26), (48, 26), (56, 50), (46, 60), (18, 60), (8, 50)), fill=(130, 30, 50), outline=OUTLINE, width=W)
    d.polygon(p((24, 26), (40, 26), (36, 14), (28, 14)), fill=(150, 110, 70), outline=OUTLINE, width=W)  # neck
    d.line(p((26, 18), (38, 18)), fill=(200, 170, 90), width=3 * SCALE)  # string
    d.ellipse(p((16, 34), (26, 46)), fill=(185, 70, 90))  # highlight
    return img


def herring():
    img, d = canvas()
    d.ellipse(p((8, 22), (50, 44)), fill=(205, 45, 40), outline=OUTLINE, width=W)
    d.polygon(p((46, 33), (60, 20), (58, 33), (60, 46)), fill=(180, 35, 30), outline=OUTLINE, width=W)  # tail
    d.ellipse(p((14, 28), (20, 34)), fill=(255, 255, 255), outline=OUTLINE, width=SCALE)  # eye
    d.arc(p((20, 24), (34, 42)), 300, 60, fill=OUTLINE, width=2 * SCALE)  # gill
    d.line(p((22, 38), (44, 36)), fill=(240, 120, 110), width=2 * SCALE)
    return img


ICONS = {
    "knife": knife(),
    "candle": candle(False),
    "burning_candle": candle(True),
    "stinky_cheese": cheese(False),
    "melted_cheese": cheese(True),
    "creased_feather": feather((235, 232, 220, 255), True),
    "perfect_feather": feather((250, 250, 245, 255), False),
    "creased_raven_feather": feather((45, 45, 55, 255), True),
    "perfect_raven_feather": feather((40, 40, 50, 255), False),
    "wine": wine(),
    "red_herring": herring(),
}

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, img in ICONS.items():
        save(img, name)
    print(f"Wrote {len(ICONS)} icons to {OUT}")
