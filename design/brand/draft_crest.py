"""Draft Castellan crest, used as the app icon until the G1 logo is chosen.

Run from the repository root:

    python design/brand/draft_crest.py

It writes design/brand/crest-draft-1024.png. Feed that to `pnpm tauri icon`
in apps/desktop to regenerate the icon set. Needs Pillow.
"""

import math
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

SIZE = 1024
AMETHYST = (176, 148, 255)
PLASMA = (98, 204, 218)
VOID = (13, 16, 22)
OUT = Path(__file__).with_name("crest-draft-1024.png")


def gradient(size: int) -> Image.Image:
    """Diagonal amethyst-to-plasma gradient."""
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            t = (x + (size - y)) / (2 * size)
            px[x, y] = tuple(int(a + (b - a) * t) for a, b in zip(AMETHYST, PLASMA))
    return img


def hexagon(cx: float, cy: float, r: float) -> list[tuple[float, float]]:
    """Pointy-top hexagon."""
    return [
        (cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
        for a in range(-90, 270, 60)
    ]


def main() -> None:
    c = SIZE / 2
    # Rounded dark tile so the icon reads on any taskbar.
    tile = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(tile).rounded_rectangle(
        (40, 40, SIZE - 40, SIZE - 40), radius=210, fill=VOID + (255,)
    )

    mask = Image.new("L", (SIZE, SIZE), 0)
    d = ImageDraw.Draw(mask)
    outer = hexagon(c, c + 10, 360)
    # Hexagon outline with a gap at the upper-right vertex for the watch light.
    stroke = 46
    pts = outer + [outer[0]]
    for i in range(6):
        a, b = pts[i], pts[i + 1]
        if i == 0:
            # Shorten the first edge so it stops before the upper-right vertex.
            b = (a[0] + (b[0] - a[0]) * 0.62, a[1] + (b[1] - a[1]) * 0.62)
        if i == 1:
            a = (a[0] + (b[0] - a[0]) * 0.38, a[1] + (b[1] - a[1]) * 0.38)
        d.line([a, b], fill=255, width=stroke, joint="curve")
    for p in outer[2:] + [outer[0]]:
        d.ellipse((p[0] - stroke / 2, p[1] - stroke / 2, p[0] + stroke / 2, p[1] + stroke / 2), fill=255)

    # The tower: three bars with crenellations, the middle one tallest.
    base = c + 210
    width = 92
    gap = 34
    heights = [250, 380, 250]
    for i, h in enumerate(heights):
        x0 = c - width * 1.5 - gap + i * (width + gap)
        top = base - h
        d.rectangle((x0, top, x0 + width, base), fill=255)
        # Crenellation notch.
        d.rectangle((x0 + width * 0.36, top, x0 + width * 0.64, top + 30), fill=0)
    # Keyhole notch at the foot of the middle bar.
    kx = c - width * 1.5 - gap + (width + gap) + width / 2
    d.ellipse((kx - 18, base - 96, kx + 18, base - 60), fill=0)
    d.polygon([(kx - 12, base - 72), (kx + 12, base - 72), (kx + 6, base - 30), (kx - 6, base - 30)], fill=0)

    crest = gradient(SIZE).convert("RGBA")
    crest.putalpha(mask)

    # Watch light with a soft glow, in plasma.
    corner = outer[1]
    glow = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(glow).ellipse(
        (corner[0] - 70, corner[1] - 70, corner[0] + 70, corner[1] + 70), fill=PLASMA + (150,)
    )
    glow = glow.filter(ImageFilter.GaussianBlur(28))
    dot = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    ImageDraw.Draw(dot).ellipse(
        (corner[0] - 34, corner[1] - 34, corner[0] + 34, corner[1] + 34), fill=PLASMA + (255,)
    )

    out = Image.alpha_composite(tile, glow)
    out = Image.alpha_composite(out, crest)
    out = Image.alpha_composite(out, dot)
    out.save(OUT)
    print(f"wrote {OUT}")


if __name__ == "__main__":
    main()
