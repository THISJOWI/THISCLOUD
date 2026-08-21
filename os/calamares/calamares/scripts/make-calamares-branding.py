#!/usr/bin/env python3
"""Generate Calamares branding pixmaps for THISCLOUD (stdlib only).

Usage: python3 make-calamares-branding.py [OUTPUT_DIR]
Writes productIcon.png, productLogo.png, productWelcome.png, wallpaper.png,
sidebar-bg.png, and slides/slide-1..4.png into OUTPUT_DIR (default:
<repo>/calamares/branding/thiscloud).
"""
import argparse
import os
import struct
import zlib

BG = (0x0F, 0x11, 0x15)      # --bg
CARD = (0x17, 0x1A, 0x21)     # --card
ACCENT = (0x3B, 0x82, 0xF6)   # --accent
FG = (0xE6, 0xE9, 0xEF)       # --fg


def _chunk(tag, data):
    return (struct.pack(">I", len(data)) + tag + data +
            struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF))


def write_png(path, width, height, pixel_at):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(pixel_at(x, y, width, height))
    png = (sig + _chunk(b"IHDR", ihdr) +
           _chunk(b"IDAT", zlib.compress(bytes(raw))) +
           _chunk(b"IEND", b""))
    with open(path, "wb") as f:
        f.write(png)


def gradient(top, bottom, accent_line=0):
    def pix(x, y, w, h):
        t = (y / (h - 1)) if h > 1 else 0.0
        r = round(top[0] + (bottom[0] - top[0]) * t)
        g = round(top[1] + (bottom[1] - top[1]) * t)
        b = round(top[2] + (bottom[2] - top[2]) * t)
        if accent_line and y >= h - accent_line:
            return ACCENT + (255,)
        return (r, g, b) + (255,)
    return pix


def solid(color):
    def pix(x, y, w, h):
        return color + (255,)
    return pix


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("output", nargs="?", default=None,
                    help="output dir (default: branding/thiscloud next to script)")
    args = ap.parse_args()

    if args.output:
        out = os.path.abspath(args.output)
    else:
        here = os.path.dirname(os.path.abspath(__file__))
        out = os.path.abspath(os.path.join(here, os.pardir, "branding", "thiscloud"))
    os.makedirs(out, exist_ok=True)

    # 128x128 square product icon — accent fill with dark "T" glyph via punch-out.
    def icon_pix(x, y, w, h):
        # Punch a simple "T" using the FG color over accent.
        bar = 30 <= y <= 46 and 40 <= x <= 88
        stem = 40 <= x <= 48 and 46 <= y <= 92
        if bar or stem:
            return FG + (255,)
        return ACCENT + (255,)
    write_png(os.path.join(out, "productIcon.png"), 128, 128, icon_pix)

    # 80x80 sidebar logo — same "T" glyph, accent on transparent rounded square.
    def logo_pix(x, y, w, h):
        if 18 <= y <= 62 and 26 <= x <= 54:
            return FG + (255,)
        if 26 <= x <= 34 and 34 <= y <= 62:
            return FG + (255,)
        return (0, 0, 0, 0)
    write_png(os.path.join(out, "productLogo.png"), 80, 80, logo_pix)

    # Welcome banner 320x150 — vertical gradient with accent strip.
    write_png(os.path.join(out, "productWelcome.png"), 320, 150,
              gradient(BG, CARD, accent_line=6))

    # Window wallpaper 800x520.
    write_png(os.path.join(out, "wallpaper.png"), 800, 520,
              gradient(BG, CARD))

    # Sidebar 240x800.
    write_png(os.path.join(out, "sidebar-bg.png"), 240, 800,
              gradient(BG, CARD, accent_line=8))

    # Slides 800x480: gradient + accent bottom strip + slide number blob.
    for i in range(1, 5):
        def slide_pix(x, y, w=800, h=480, _i=i):
            c = gradient(BG, CARD, accent_line=8)(x, y, w, h)
            # Title bar band near top, distinct per-slide x offset.
            if 60 <= y <= 100:
                if x % 60 < 40:
                    return FG + (255,)
            return c
        write_png(os.path.join(out, "slides", f"slide-{i}.png"), 800, 480, slide_pix)

    print(f"wrote {len(os.listdir(out))} top-level files to {out}")
    print("slides:", sorted(os.listdir(os.path.join(out, "slides"))))


if __name__ == "__main__":
    main()