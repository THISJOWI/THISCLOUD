#!/usr/bin/env bash
# Build a custom product.img for THISCLOUD Anaconda branding.
# Generates brand-palette gradient backgrounds (no logo — text-only
# product name, rendered natively by Anaconda) and assembles them into
# product.img using cpio+gzip. The product.img is loaded by Anaconda at
# boot time to replace AlmaLinux branding with THISCLOUD branding.
#
# Also generates the boot-loader background assets (isolinux splash.png
# and grub2 background.png) into iso/branding/boot/ for remix-iso.sh.
#
# Uses only Python stdlib (struct + zlib) — no PIL/Pillow needed.
#
# Usage: ./make-product-img.sh [OUTPUT_DIR]
#   OUTPUT_DIR defaults to the script's parent directory.
set -euo pipefail

# Resolve script directory
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
SCRIPT_DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
ISO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

OUTPUT_DIR="${1:-$ISO_DIR}"
BRANDING_DIR="$ISO_DIR/branding/product"

echo "==> Building THISCLOUD product.img"

# ── Generate brand assets using Python ──────────────────────────────
echo "    Generating brand pixmaps + boot assets..."

python3 - "$BRANDING_DIR" <<'PYTHON_EOF'
import struct, zlib, sys, os

BRANDING_DIR = sys.argv[1]
PIXMAPS = os.path.join(BRANDING_DIR, "usr", "share", "anaconda", "pixmaps")
BOOT = os.path.normpath(os.path.join(BRANDING_DIR, os.pardir, "boot"))
os.makedirs(PIXMAPS, exist_ok=True)
os.makedirs(BOOT, exist_ok=True)

# ── Brand palette (matches web-ui/src/app/globals.css) ──────────────
BG     = (0x0f, 0x11, 0x15)   # --bg
CARD   = (0x17, 0x1a, 0x21)   # --card
ACCENT = (0x3b, 0x82, 0xf6)   # --accent
FG     = (0xe6, 0xe9, 0xef)   # --fg

def _chunk(tag, data):
    """PNG chunk: length + tag + data + crc."""
    return (struct.pack('>I', len(data)) + tag + data +
            struct.pack('>I', zlib.crc32(tag + data) & 0xffffffff))

def write_png(path, width, height, pixel_at):
    """Write an 8-bit RGBA PNG where pixel_at(x, y) -> (r,g,b,a)."""
    sig = b'\x89PNG\r\n\x1a\n'
    ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)
    raw = bytearray()
    for y in range(height):
        raw.append(0)  # filter byte: none
        for x in range(width):
            raw.extend(pixel_at(x, y))
    png = (sig + _chunk(b'IHDR', ihdr) +
           _chunk(b'IDAT', zlib.compress(bytes(raw))) +
           _chunk(b'IEND', b''))
    with open(path, 'wb') as f:
        f.write(png)

def gradient_pix(width, height, top, bottom, accent_line=0):
    """Vertical gradient top→bottom with optional accent strip at the
    bottom edge. Precomputes one color per row, then maps x→color."""
    rows = []
    for y in range(height):
        t = (y / (height - 1)) if height > 1 else 0.0
        r = round(top[0] + (bottom[0] - top[0]) * t)
        g = round(top[1] + (bottom[1] - top[1]) * t)
        b = round(top[2] + (bottom[2] - top[2]) * t)
        rows.append((r, g, b))
    def pix(x, y):
        if accent_line and y >= height - accent_line:
            return ACCENT + (255,)
        return rows[y] + (255,)
    return pix

# Anaconda sidebar background — dark vertical gradient.
write_png(os.path.join(PIXMAPS, "sidebar-bg.png"), 240, 800,
          gradient_pix(240, 800, BG, CARD))

# Anaconda top bar (spoke navigation) — subtle gradient + accent strip.
write_png(os.path.join(PIXMAPS, "topbar-bg.png"), 1920, 60,
          gradient_pix(1920, 60, BG, CARD, accent_line=3))

# Sidebar logo — transparent placeholder (no logo image; the product
# name is rendered as text by Anaconda from product_name).
write_png(os.path.join(PIXMAPS, "sidebar-logo.png"), 200, 100,
          lambda x, y: (0, 0, 0, 0))

# Sidebar right-arrow icon — white ">" chevron on transparent bg.
def chevron_pix(x, y):
    if x > 4 + abs(y - 7.5) * 0.9:
        return FG + (255,)
    return (0, 0, 0, 0)
write_png(os.path.join(PIXMAPS, "right-arrow-icon.png"), 16, 16, chevron_pix)

# Boot-loader backgrounds (consumed by remix-iso.sh).
# isolinux splash must be 640x480; grub2 background 1024x768.
write_png(os.path.join(BOOT, "splash.png"), 640, 480,
          gradient_pix(640, 480, BG, CARD, accent_line=6))
write_png(os.path.join(BOOT, "grub-background.png"), 1024, 768,
          gradient_pix(1024, 768, BG, CARD, accent_line=8))

print(f"    pixmaps: {len(os.listdir(PIXMAPS))} files")
print(f"    boot:    {sorted(os.listdir(BOOT))}")
PYTHON_EOF

# ── Build product.img archive ───────────────────────────────────────
echo "    Assembling product.img with cpio+gzip..."
(cd "$BRANDING_DIR" && find . | cpio -c -o 2>/dev/null | gzip -9 > "$OUTPUT_DIR/product.img")

echo "    product.img built: $(ls -lh "$OUTPUT_DIR/product.img" | awk '{print $5}')"
echo "==> Done"
