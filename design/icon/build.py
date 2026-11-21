#!/usr/bin/env python3
"""Draws every icon file of Quill, on any computer: python3 design/icon/build.py (needs Pillow).

The drawing is described once, below, and written out as design/icon/quill.svg and as the pictures:

  app/assets/icon.png          the window's icon on Windows and Linux (256 px)
  app/assets/icon-macos.png    the Dock icon of an app that runs outside its .app (512 px, Apple's grid)
  installer/Pencraft.icns      the icon of the Mac app
  installer/pencraft.ico       the icon of pencraft.exe, its shortcuts, design files and the installers

The icon: a quill drawing a line. build.sh draws the same files from quill.svg with a Mac's own renderer.
"""

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
UNIT = 1024  # the drawing's own square
OVER = 4  # every picture is drawn this many times larger and scaled down, for smooth edges

PLATE = ("#34343a", "#111113")  # top to bottom
FEATHER = ("#ffffff", "#e4e4e7")
INK = "#3b82f6"
DARK = "#1c1c1f"
RADIUS = 230

# The vane of the feather: from where it leaves the shaft, over its tip, and back.
VANE = [((300, 772), (318, 520), (540, 250), (836, 188)), ((836, 188), (836, 470), (600, 716), (300, 772))]
# The nib: a point where the line starts, widening to where the vane begins.
NIB = [(182, 880), (288, 759), (312, 785)]
# The shaft through the vane.
SHAFT = ((338, 736), (716, 316))
# The line the quill is drawing.
LINE = ((182, 880), (380, 968), (650, 936), (856, 772))
# Splits in the vane, each from its edge towards the shaft.
SPLITS = [((676, 660), (500, 604)), ((808, 484), (634, 440))]


def bezier(curve, steps=96):
    (ax, ay), (bx, by), (cx, cy), (dx, dy) = curve
    out = []
    for i in range(steps + 1):
        t = i / steps
        u = 1 - t
        out.append((u**3 * ax + 3 * u * u * t * bx + 3 * u * t * t * cx + t**3 * dx, u**3 * ay + 3 * u * u * t * by + 3 * u * t * t * cy + t**3 * dy))
    return out


def rgb(colour):
    return tuple(int(colour[i : i + 2], 16) for i in (1, 3, 5))


def gradient(size, colours, across=False):
    """A square fading from the first colour to the second, downwards (or towards the lower right)."""
    first, second = rgb(colours[0]), rgb(colours[1])
    ramp = Image.new("RGB", (256, 1))
    ramp.putdata([tuple(round(a + (b - a) * i / 255) for a, b in zip(first, second)) for i in range(256)])
    if across:
        return ramp.resize((size * 2, 1)).resize((size * 2, size * 2)).rotate(-45, resample=Image.BICUBIC).crop((size // 2, size // 2, size // 2 + size, size // 2 + size))
    return ramp.rotate(-90, expand=True).resize((size, size))


def stroke(draw, points, width, fill):
    """A line with round ends: a disc set down every little way along it."""
    for (ax, ay), (bx, by) in zip(points, points[1:]):
        steps = max(1, int(max(abs(bx - ax), abs(by - ay))))
        for i in range(steps + 1):
            x, y = ax + (bx - ax) * i / steps, ay + (by - ay) * i / steps
            draw.ellipse((x - width / 2, y - width / 2, x + width / 2, y + width / 2), fill=fill)


def plate(size):
    """The icon on its plate, `size` pixels square, the plate filling the square."""
    big = size * OVER
    k = big / UNIT

    def at(points):
        return [(x * k, y * k) for x, y in points]

    picture = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    shape = Image.new("L", (big, big), 0)
    ImageDraw.Draw(shape).rounded_rectangle((0, 0, big - 1, big - 1), radius=RADIUS * k, fill=255)
    picture.paste(gradient(big, PLATE), (0, 0), shape)
    # The rim: a light edge, strongest at the top.
    inner = Image.new("L", (big, big), 0)
    ImageDraw.Draw(inner).rounded_rectangle((10 * k, 10 * k, big - 1 - 10 * k, big - 1 - 10 * k), radius=(RADIUS - 10) * k, fill=255)
    edge = ImageChops.subtract(shape, inner)
    fade = Image.new("L", (1, 256))
    fade.putdata([round(255 * (0.22 - 0.36 * i / 255 if i < 128 else 0.04 + 0.08 * (i - 128) / 127)) for i in range(256)])
    edge = ImageChops.multiply(edge, fade.resize((big, big)))
    picture.paste(Image.new("RGBA", (big, big), (255, 255, 255, 255)), (0, 0), edge)

    draw = ImageDraw.Draw(picture)
    stroke(draw, at(bezier(LINE)), 46 * k, INK)

    vane = Image.new("L", (big, big), 0)
    mask = ImageDraw.Draw(vane)
    mask.polygon(at(bezier(VANE[0]) + bezier(VANE[1])), fill=255)
    mask.polygon(at(NIB), fill=255)
    for split in SPLITS:
        stroke(mask, at(split), 18 * k, 0)
    stroke(mask, at(SHAFT), 18 * k, 0)
    picture.paste(gradient(big, FEATHER, across=True), (0, 0), vane)
    return picture.resize((size, size), Image.LANCZOS)


def mac(size):
    """The icon on Apple's grid: the plate 824 of 1024 wide, with a soft shadow under it."""
    inner = round(size * 824 / 1024)
    icon = plate(inner)
    picture = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    corner = ((size - inner) // 2, (size - inner) // 2)
    shadow = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    shadow.paste(Image.new("RGBA", (inner, inner), (0, 0, 0, 77)), (corner[0], corner[1] + round(size * 10 / 1024)), icon.getchannel("A"))
    picture = Image.alpha_composite(picture, shadow.filter(ImageFilter.GaussianBlur(size * 10.5 / 1024)))
    picture.alpha_composite(icon, corner)
    return picture


def path(curves):
    first = curves[0][0]
    return f"M {first[0]} {first[1]} " + " ".join(f"C {b[0]} {b[1]} {c[0]} {c[1]} {d[0]} {d[1]}" for _, b, c, d in curves)


def svg():
    splits = " ".join(f"M {a[0]} {a[1]} L {b[0]} {b[1]}" for a, b in SPLITS)
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {UNIT} {UNIT}">
  <!-- The Quill icon: a quill drawing a line. Written by build.py, which also draws every icon file. -->
  <defs>
    <linearGradient id="plate" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="{PLATE[0]}"/>
      <stop offset="1" stop-color="{PLATE[1]}"/>
    </linearGradient>
    <linearGradient id="rim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color="#ffffff" stop-opacity="0.22"/>
      <stop offset="0.5" stop-color="#ffffff" stop-opacity="0.04"/>
      <stop offset="1" stop-color="#ffffff" stop-opacity="0.08"/>
    </linearGradient>
    <linearGradient id="feather" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="{FEATHER[0]}"/>
      <stop offset="1" stop-color="{FEATHER[1]}"/>
    </linearGradient>
    <mask id="vane" maskUnits="userSpaceOnUse" x="0" y="0" width="{UNIT}" height="{UNIT}">
      <path d="{path(VANE)} Z" fill="#ffffff"/>
      <path d="M {NIB[0][0]} {NIB[0][1]} L {NIB[1][0]} {NIB[1][1]} L {NIB[2][0]} {NIB[2][1]} Z" fill="#ffffff"/>
      <path d="{splits}" stroke="#000000" stroke-width="18" stroke-linecap="round"/>
      <path d="M {SHAFT[0][0]} {SHAFT[0][1]} L {SHAFT[1][0]} {SHAFT[1][1]}" stroke="#000000" stroke-width="18" stroke-linecap="round"/>
    </mask>
  </defs>
  <rect width="{UNIT}" height="{UNIT}" rx="{RADIUS}" fill="url(#plate)"/>
  <rect x="5" y="5" width="{UNIT - 10}" height="{UNIT - 10}" rx="{RADIUS - 5}" fill="none" stroke="url(#rim)" stroke-width="10"/>
  <!-- The line the quill is drawing. -->
  <path d="{path([LINE])}" fill="none" stroke="{INK}" stroke-width="46" stroke-linecap="round"/>
  <!-- The quill: its vane with two splits, the shaft through it, and the nib at the start of the line. -->
  <rect width="{UNIT}" height="{UNIT}" fill="url(#feather)" mask="url(#vane)"/>
</svg>
"""


def main():
    (HERE / "quill.svg").write_text(svg(), encoding="utf-8", newline="\n")
    assets = ROOT / "app" / "assets"
    assets.mkdir(parents=True, exist_ok=True)
    plate(256).save(assets / "icon.png")
    mac(512).save(assets / "icon-macos.png")
    # Windows: each size is drawn at its own size, not scaled down from a big one. Plain bitmaps, not PNG:
    # the resource compiler of older Windows SDKs refuses PNG entries.
    sizes = [16, 20, 24, 32, 40, 48, 64, 96, 128, 256]
    images = [plate(size) for size in sizes]
    images[-1].save(ROOT / "installer" / "pencraft.ico", format="ICO", bitmap_format="bmp", sizes=[(s, s) for s in sizes], append_images=images[:-1])
    # macOS: every size the Finder and the Dock ask for, each drawn at its own size.
    wanted = [16, 32, 64, 128, 256, 512, 1024]
    drawn = [mac(size) for size in wanted]
    drawn[-1].save(ROOT / "installer" / "Pencraft.icns", format="ICNS", append_images=drawn[:-1])


if __name__ == "__main__":
    main()
