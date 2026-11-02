#!/usr/bin/env python3
"""Generira ikone klijenta iz logo-izvor.png (512x512, prozirna pozadina). Treba Pillow.

Pokretanje (iz korijena repozitorija):  python3 -P wips/branding/generiraj-ikone.py
Rezultat ide u wips/branding/generirano/ i commita se; CI ga samo kopira (apply-branding.sh).

Male veličine (do 48 px) koriste samo crveni krug s monitorom: cijeli logo (fotografija zeca)
na 16-32 px postaje tamna mrlja i nestaje na tamnoj traci zadataka. Velike veličine koriste
cijeli logo s tankim bijelim obrubom, da se vidi i na tamnoj pozadini.
"""
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

DIR = Path(__file__).resolve().parent
SRC = DIR / "logo-izvor.png"
OUT = DIR / "generirano"

SMALL = [16, 20, 24, 32, 40, 48]
LARGE = [64, 96, 128, 256]


def badge_from(im):
    """Crveni krug (s bijelim rubom) iz donjeg desnog kuta logotipa, kao kvadratna slika."""
    px = im.load()
    pts = [(x, y) for y in range(im.height) for x in range(im.width)
           if px[x, y][3] > 200 and px[x, y][0] > 200 and px[x, y][1] < 90 and px[x, y][2] < 90]
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    cx, cy = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    pad = max(max(xs) - min(xs), max(ys) - min(ys)) / 2 * 1.12
    badge = im.crop((round(cx - pad), round(cy - pad), round(cx + pad), round(cy + pad)))
    mask = Image.new("L", badge.size, 0)
    ImageDraw.Draw(mask).ellipse((0, 0, badge.width - 1, badge.height - 1), fill=255)
    badge.putalpha(Image.composite(badge.getchannel("A"), Image.new("L", badge.size, 0), mask))
    return badge


def square(im):
    im = im.crop(im.getchannel("A").getbbox())
    side = max(im.size)
    sq = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    sq.paste(im, ((side - im.width) // 2, (side - im.height) // 2))
    return sq


def outlined(im, width):
    """Bijeli obrub oko neprozirnog dijela."""
    margin = width + 2
    big = Image.new("RGBA", (im.width + 2 * margin, im.height + 2 * margin), (0, 0, 0, 0))
    big.paste(im, (margin, margin))
    alpha = big.getchannel("A").point(lambda a: 255 if a > 40 else 0)
    ring = alpha.filter(ImageFilter.MaxFilter(2 * width + 1)).filter(ImageFilter.GaussianBlur(1))
    white = Image.new("RGBA", big.size, (255, 255, 255, 0))
    white.putalpha(ring)
    return Image.alpha_composite(white, big)


def save_ico(path, frames):
    """frames: {veličina: slika}. Sve sličice kao BMP, radi najšire kompatibilnosti (rc.exe, tray)."""
    sizes = sorted(frames)
    base = frames[sizes[-1]]
    base.save(path, format="ICO", sizes=[(s, s) for s in sizes],
              append_images=[frames[s] for s in sizes[:-1]], bitmap_format="bmp")


def main():
    src = Image.open(SRC).convert("RGBA")
    badge = badge_from(src)
    full = square(src)
    full_outlined = outlined(full, width=7)
    OUT.mkdir(exist_ok=True)

    def rs(im, s):
        return im.resize((s, s), Image.LANCZOS)

    app = {s: rs(badge, s) for s in SMALL} | {s: rs(full_outlined, s) for s in LARGE}
    save_ico(OUT / "app.ico", app)
    save_ico(OUT / "tray.ico", {s: rs(badge, s) for s in SMALL + [64]})
    rs(badge, 256).save(OUT / "icon.png")
    rs(full, 512).save(OUT / "logo.png")
    rs(full_outlined, 512).save(OUT / "logo_dark.png")
    print("generirano:", ", ".join(sorted(p.name for p in OUT.iterdir())))


if __name__ == "__main__":
    main()
