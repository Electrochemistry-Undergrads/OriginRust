#!/usr/bin/env python3
"""Build Sheaf's application icon from the owner's logo, docs/brand/sheaf.png.

    python3 tools/make-icon.py                 # writes crates/app/assets/icon/ (rust tile, the chosen look)
    python3 tools/make-icon.py --style paper   # the logo's rust S on a paper-white tile instead

The logo (the bronze S) is kept as drawn: its outline comes from the logo's alpha channel, cleaned of
the faint specks around it and smoothed. It is set in paper white on a rounded tile in the logo's rust
(the owner chose this look on 2026-11-14), in the macOS icon grid (an 824 of 1024 body with a shadow),
over a faint graph-paper grid, since Sheaf is a plotting program. Sizes
of 32 pixels and less use a plainer tile with a larger S and no grid, so the letter stays readable.

Outputs (all in crates/app/assets/icon/):
  sheaf-1024.png   master image
  sheaf-256.png    the window and Dock icon the app embeds (crates/app/src/main.rs)
  sheaf.ico        Windows icon, 16 to 256 pixels
  Sheaf.icns       macOS icon (only when `iconutil` exists, that is on macOS)

Needs Pillow and NumPy.
"""

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
LOGO = ROOT / "docs" / "brand" / "sheaf.png"
OUT = ROOT / "crates" / "app" / "assets" / "icon"

SS = 2048  # working size; everything is drawn at twice 1024 and scaled down
BODY = 1648  # 824 of 1024: the body of a macOS icon, leaving room for its shadow
RADIUS = 370  # corner radius of the body (185 of 1024)

# Two looks. "paper": the logo's rust bronze S (lighter at the top; the logo's mean is #A9461B) on warm
# paper white. "rust": the same rust as the tile, with the S in paper white.
STYLES = {
    "paper": dict(
        s_top=(196, 88, 44), s_bottom=(160, 62, 22),
        tile_top=(255, 252, 246), tile_bottom=(240, 231, 218),
        grid=(226, 214, 198), grid_major=(214, 199, 180),
    ),
    "rust": dict(
        s_top=(255, 250, 242), s_bottom=(244, 233, 218),
        tile_top=(200, 90, 46), tile_bottom=(150, 56, 20),
        grid=(186, 82, 41), grid_major=(176, 74, 36),
    ),
}
STYLE = STYLES["rust"]


def logo_mask() -> Image.Image:
    """The S as an 8-bit mask cropped to its bounds: opaque logo pixels, specks removed, edges smoothed."""
    alpha = np.array(Image.open(LOGO).convert("RGBA"))[..., 3]
    mask = Image.fromarray(np.where(alpha > 128, 255, 0).astype(np.uint8))
    # The faint red specks along the edges are below the threshold; a median removes the few that pass.
    mask = mask.filter(ImageFilter.MedianFilter(7))
    # Smooth the stair-stepped outline: blur, then make the edge steep again.
    mask = mask.filter(ImageFilter.GaussianBlur(2.5))
    m = np.asarray(mask).astype(np.float32)
    m = np.clip((m - 90.0) / (165.0 - 90.0), 0.0, 1.0) * 255.0
    mask = Image.fromarray(m.astype(np.uint8))
    return mask.crop(mask.getbbox())


def vertical_gradient(size: tuple[int, int], top: tuple, bottom: tuple) -> Image.Image:
    w, h = size
    t = np.linspace(0.0, 1.0, h, dtype=np.float32)[:, None, None]
    rgb = np.array(top, np.float32) * (1 - t) + np.array(bottom, np.float32) * t
    return Image.fromarray(np.repeat(rgb, w, axis=1).astype(np.uint8), "RGB").convert("RGBA")


def rounded(size: int, box: tuple, radius: int) -> Image.Image:
    m = Image.new("L", (size, size), 0)
    ImageDraw.Draw(m).rounded_rectangle(box, radius=radius, fill=255)
    return m


def place_s(canvas: Image.Image, mask: Image.Image, height: int, centre: tuple, shadow: bool) -> None:
    """Draw the S `height` pixels high, centred at `centre`, in the style's gradient."""
    w = round(mask.width * height / mask.height)
    m = mask.resize((w, height), Image.LANCZOS)
    x, y = round(centre[0] - w / 2), round(centre[1] - height / 2)
    if shadow:
        sh = Image.new("L", canvas.size, 0)
        sh.paste(m, (x, y + height // 60))
        sh = sh.filter(ImageFilter.GaussianBlur(height / 70)).point(lambda v: v * 0.22)
        canvas.alpha_composite(Image.merge("RGBA", (*[Image.new("L", canvas.size, 40)] * 3, sh)))
    fill = vertical_gradient((w, height), STYLE["s_top"], STYLE["s_bottom"])
    fill.putalpha(m)
    canvas.alpha_composite(fill, (x, y))


def tile(mask: Image.Image, small: bool) -> Image.Image:
    """The icon at SS pixels. `small` is the plain version for 16 and 32 pixels."""
    canvas = Image.new("RGBA", (SS, SS), (0, 0, 0, 0))
    if small:
        # Small sizes fill more of the square and have no shadow, which would be a blur at 16 pixels.
        inset, radius = 96, 420
    else:
        inset, radius = (SS - BODY) // 2, RADIUS
    box = (inset, inset, SS - inset - 1, SS - inset - 1)
    body = rounded(SS, box, radius)

    if not small:
        # The shadow macOS icons carry in their artwork: soft, a little below the body.
        sh = Image.new("L", (SS, SS), 0)
        sh.paste(body, (0, 24))
        sh = sh.filter(ImageFilter.GaussianBlur(36)).point(lambda v: v * 0.30)
        canvas.alpha_composite(Image.merge("RGBA", (*[Image.new("L", (SS, SS), 0)] * 3, sh)))

    paper = vertical_gradient((SS, SS), STYLE["tile_top"], STYLE["tile_bottom"])
    if not small:
        # Graph paper: ten squares across the body, every fifth line a little darker.
        d = ImageDraw.Draw(paper)
        step = (box[2] - box[0]) / 10
        for i in range(1, 10):
            c, wdt = (STYLE["grid_major"], 5) if i == 5 else (STYLE["grid"], 3)
            p = round(box[0] + i * step)
            d.line([(p, box[1]), (p, box[3])], fill=c, width=wdt)
            d.line([(box[0], p), (box[2], p)], fill=c, width=wdt)
    paper.putalpha(ImageChops.multiply(body, paper.getchannel("A")))
    canvas.alpha_composite(paper)

    if not small:
        # A thin darker rim so the light tile keeps its edge on a white Dock or desktop.
        rim = rounded(SS, box, radius)
        inner = rounded(SS, (box[0] + 4, box[1] + 4, box[2] - 4, box[3] - 4), radius - 4)
        edge = ImageChops.subtract(rim, inner).point(lambda v: v * 0.35)
        canvas.alpha_composite(Image.merge("RGBA", (*[Image.new("L", (SS, SS), 120)] * 3, edge)))

    body_h = box[3] - box[1]
    centre = (SS / 2, SS / 2)
    place_s(canvas, mask, round(body_h * (0.78 if small else 0.64)), centre, shadow=not small)
    if small:
        # Keep the S inside the rounded body.
        canvas.putalpha(ImageChops.multiply(canvas.getchannel("A"), body))
    return canvas


def sized(img: Image.Image, px: int) -> Image.Image:
    return img.resize((px, px), Image.LANCZOS)


def main() -> int:
    global STYLE
    args = sys.argv[1:]
    if args[:1] == ["--style"] and len(args) == 2 and args[1] in STYLES:
        STYLE = STYLES[args[1]]
    elif args:
        print(f"usage: make-icon.py [--style {'|'.join(STYLES)}]", file=sys.stderr)
        return 2
    OUT.mkdir(parents=True, exist_ok=True)
    mask = logo_mask()
    big, small = tile(mask, small=False), tile(mask, small=True)
    pick = lambda px: sized(small if px <= 32 else big, px)  # noqa: E731

    pick(1024).save(OUT / "sheaf-1024.png", optimize=True)
    pick(256).save(OUT / "sheaf-256.png", optimize=True)

    ico_sizes = [16, 24, 32, 48, 64, 128, 256]
    imgs = [pick(px) for px in ico_sizes]
    imgs[-1].save(OUT / "sheaf.ico", sizes=[(s, s) for s in ico_sizes], append_images=imgs[:-1])

    if shutil.which("iconutil"):
        with tempfile.TemporaryDirectory() as tmp:
            iconset = Path(tmp) / "Sheaf.iconset"
            iconset.mkdir()
            for pt in (16, 32, 128, 256, 512):
                pick(pt).save(iconset / f"icon_{pt}x{pt}.png")
                pick(pt * 2).save(iconset / f"icon_{pt}x{pt}@2x.png")
            subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(OUT / "Sheaf.icns")], check=True)
    else:
        print("iconutil not found (not macOS): Sheaf.icns left as it was", file=sys.stderr)

    for f in sorted(OUT.iterdir()):
        print(f"{f.relative_to(ROOT)}  {f.stat().st_size // 1024} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
