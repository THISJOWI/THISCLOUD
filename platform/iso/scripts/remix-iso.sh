#!/usr/bin/env bash
# Remix a base AlmaLinux ISO into a fully-branded THISCLOUD ISO.
# Extracts the ISO, edits boot configs to remove AlmaLinux branding,
# injects the THISCLOUD kickstart, repo, and product.img, then rebuilds.
#
# This replaces the previous mkksiso step and gives full control over
# the boot menu, installer branding, and ISO label.
#
# Usage:
#   INPUT_ISO=/path/to/AlmaLinux.iso OUTPUT_ISO=/path/to/out.iso \
#     ./remix-iso.sh
set -euo pipefail

# ── Resolve script directory ─────────────────────────────────────────
SOURCE="${BASH_SOURCE[0]}"
while [ -L "$SOURCE" ]; do
  DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ "$SOURCE" != /* ]] && SOURCE="$DIR/$SOURCE"
done
SCRIPT_DIR="$(cd "$(dirname "$SOURCE")" && pwd)"
ISO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
PLATFORM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PLATFORM_DIR"

# ── Configuration ───────────────────────────────────────────────────
INPUT_ISO="${INPUT_ISO:-/data/AlmaLinux-9-latest-x86_64-minimal.iso}"
OUTPUT_ISO="${OUTPUT_ISO:-/data/thiscloud-iso/ThisCloud-0.1.0-x86_64.iso}"
WORK_DIR="${WORK_DIR:-/tmp/remix-iso-work}"
KS_FILE="${KS_FILE:-iso/kickstart/thiscloud.ks}"
REPO_DIR="${REPO_DIR:-iso/repo}"
PRODUCT_IMG="${PRODUCT_IMG:-iso/product.img}"
VOLID="THISCLOUD"
VERSION="0.1.0"

# ── Preflight checks ────────────────────────────────────────────────
for tool in xorriso cpio gzip implantisomd5; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "error: missing required tool: $tool"
    exit 1
  fi
done

if [ ! -f "$INPUT_ISO" ]; then
  echo "error: input ISO not found: $INPUT_ISO"
  exit 1
fi

# ── Build product.img + boot assets if not present ──────────────────
if [ ! -f "$PRODUCT_IMG" ] || [ ! -f "iso/branding/boot/splash.png" ]; then
  echo "==> [1/11] Building product.img + boot assets"
  bash iso/scripts/make-product-img.sh
else
  echo "==> [1/11] product.img + boot assets already exist, skipping build"
fi

# ── Extract ISO ──────────────────────────────────────────────────────
echo "==> [2/11] Extracting ISO to $WORK_DIR"
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"
xorriso -osirrox on -indev "$INPUT_ISO" -extract / "$WORK_DIR" || {
  echo "error: ISO extraction failed with exit code $?"
  echo "       Check that the input ISO is intact: $INPUT_ISO"
  exit 1
}

# ── Detect boot structure ───────────────────────────────────────────
echo "==> [3/11] Detecting boot structure"
if [ -f "$WORK_DIR/isolinux/isolinux.bin" ]; then
  BOOT_BIOS="isolinux/isolinux.bin"
  echo "    BIOS boot: $BOOT_BIOS"
else
  echo "error: isolinux.bin not found — not a BIOS-bootable ISO"
  exit 1
fi

if [ -f "$WORK_DIR/images/efiboot.img" ]; then
  BOOT_EFI="images/efiboot.img"
elif [ -f "$WORK_DIR/images/eltorito.img" ]; then
  BOOT_EFI="images/eltorito.img"
else
  echo "error: no EFI boot image found in images/"
  exit 1
fi
echo "    EFI boot:  $BOOT_EFI"

# ── Extract boot config flags from the original ISO ─────────────────
echo "==> [4/11] Extracting original boot config flags"
ELTORITO_REPORT=$(xorriso -indev "$INPUT_ISO" -report_el_torito as_mkisofs 2>/dev/null || true)
echo "$ELTORITO_REPORT" > "$WORK_DIR/.el_torito_report"

# ── Edit BIOS boot config: isolinux/isolinux.cfg ────────────────────
echo "==> [5/11] Rebranding BIOS boot menu (isolinux)"
if [ -f "$WORK_DIR/isolinux/isolinux.cfg" ]; then
  # Replace AlmaLinux branding with THISCLOUD
  sed -i 's/AlmaLinux[[:space:]]*[0-9.]*/THISCLOUD '"$VERSION"'/g' \
    "$WORK_DIR/isolinux/isolinux.cfg"
  sed -i 's/Test this \^media & install/Test this media \& install/g' \
    "$WORK_DIR/isolinux/isolinux.cfg"
  # Rename non-versioned entries so no menu item references AlmaLinux
  sed -i 's/Rescue a broken system/Rescue THISCLOUD system/g' \
    "$WORK_DIR/isolinux/isolinux.cfg"
  # Set default timeout to 5 seconds
  sed -i 's/^timeout [0-9]*/timeout 50/' \
    "$WORK_DIR/isolinux/isolinux.cfg" 2>/dev/null || true
  # Update inst.stage2 label to match our VOLID
  sed -i "s/LABEL=[^ ]*/LABEL=$VOLID/g" \
    "$WORK_DIR/isolinux/isolinux.cfg"
  # Add inst.ks to default boot entry if not already present
  if ! grep -q "inst.ks=" "$WORK_DIR/isolinux/isolinux.cfg"; then
    sed -i "/^  append/s/quiet/quiet inst.ks=cdrom:\/kickstart.ks/" \
      "$WORK_DIR/isolinux/isolinux.cfg"
  fi
  # THISCLOUD menu color scheme (vesamenu uses the last value per attr)
  cat >> "$WORK_DIR/isolinux/isolinux.cfg" <<'MENUCOLORS'
menu color border 30;44 #2a2f3a #00000000
menu color title 1;37 #3b82f6 #00000000 std
menu color sel 30;37 #ffffff #3b82f6
menu color unsel 0;37 #8b93a3 #00000000
MENUCOLORS
  echo "    isolinux.cfg updated"
fi

# Also update isolinux/grub.conf if it exists (some versions have it)
if [ -f "$WORK_DIR/isolinux/grub.conf" ]; then
  sed -i 's/AlmaLinux[[:space:]]*[0-9.]*/THISCLOUD '"$VERSION"'/g' \
    "$WORK_DIR/isolinux/grub.conf"
  sed -i "s/LABEL=[^ ]*/LABEL=$VOLID/g" \
    "$WORK_DIR/isolinux/grub.conf"
  echo "    isolinux/grub.conf updated"
fi

# ── Edit UEFI boot config: EFI/BOOT/grub.cfg ───────────────────────
echo "==> [6/11] Rebranding UEFI boot menu (grub2)"
if [ -f "$WORK_DIR/EFI/BOOT/grub.cfg" ]; then
  sed -i 's/AlmaLinux[[:space:]]*[0-9.]*/THISCLOUD '"$VERSION"'/g' \
    "$WORK_DIR/EFI/BOOT/grub.cfg"
  sed -i "s/LABEL=[^ ]*/LABEL=$VOLID/g" \
    "$WORK_DIR/EFI/BOOT/grub.cfg"
  # Add inst.ks if not present
  if ! grep -q "inst.ks=" "$WORK_DIR/EFI/BOOT/grub.cfg"; then
    sed -i '/inst.stage2/s/quiet/quiet inst.ks=cdrom:\/kickstart.ks/' \
      "$WORK_DIR/EFI/BOOT/grub.cfg"
  fi
  # THISCLOUD gfxmenu colors + branded background. background_image must
  # come after the terminal output is gfxterm; fall back to after
  # load_video (or top of file) when the line differs.
  GRUB_CFG="$WORK_DIR/EFI/BOOT/grub.cfg"
  if ! grep -q "color_highlight" "$GRUB_CFG"; then
    cp -f iso/branding/boot/grub-background.png \
      "$WORK_DIR/EFI/BOOT/background.png"
    if grep -q "^terminal_output" "$GRUB_CFG"; then
      sed -i "/^terminal_output/a\\
set color_normal=\"white/black\"\\
set color_highlight=\"#3b82f6/black\"\\
background_image /EFI/BOOT/background.png" "$GRUB_CFG"
    elif grep -q "^load_video" "$GRUB_CFG"; then
      sed -i "/^load_video/a\\
set color_normal=\"white/black\"\\
set color_highlight=\"#3b82f6/black\"\\
background_image /EFI/BOOT/background.png" "$GRUB_CFG"
    else
      sed -i "1i\\
set color_normal=\"white/black\"\\
set color_highlight=\"#3b82f6/black\"\\
background_image /EFI/BOOT/background.png" "$GRUB_CFG"
    fi
    echo "    grub.cfg theme applied"
  fi
  echo "    grub.cfg updated"
fi

# Also check for grub2 config (some versions use boot/grub2/)
if [ -f "$WORK_DIR/boot/grub2/grub.cfg" ]; then
  sed -i 's/AlmaLinux[[:space:]]*[0-9.]*/THISCLOUD '"$VERSION"'/g' \
    "$WORK_DIR/boot/grub2/grub.cfg"
  sed -i "s/LABEL=[^ ]*/LABEL=$VOLID/g" \
    "$WORK_DIR/boot/grub2/grub.cfg"
  echo "    boot/grub2/grub.cfg updated"
fi

# ── Replace splash images with THISCLOUD placeholders ───────────────
echo "==> [7/11] Replacing boot splash images"
# Create a simple dark blue splash for BIOS boot
if [ -d "$WORK_DIR/isolinux" ]; then
  # The splash image is loaded by isolinux — use the branded 640x480 asset
  if [ -f "$WORK_DIR/isolinux/splash.png" ]; then
    cp -f "iso/branding/boot/splash.png" \
      "$WORK_DIR/isolinux/splash.png" 2>/dev/null || true
    echo "    BIOS splash replaced"
  fi
fi

# ── Inject THISCLOUD assets into extracted ISO ──────────────────────
echo "==> [8/11] Injecting THISCLOUD assets"

# Copy kickstart to ISO root
mkdir -p "$WORK_DIR"
cp -f "$KS_FILE" "$WORK_DIR/kickstart.ks"
echo "    Kickstart: kickstart.ks"

# Copy product.img into images/ (replaces AlmaLinux's product.img)
mkdir -p "$WORK_DIR/images"
cp -f "$PRODUCT_IMG" "$WORK_DIR/images/product.img"
echo "    Product image: images/product.img"

# Copy the THISCLOUD repo
mkdir -p "$WORK_DIR/repo"
if [ -d "$REPO_DIR" ]; then
  cp -a "$REPO_DIR"/* "$WORK_DIR/repo/" 2>/dev/null || true
  echo "    Repo: repo/ ($(ls "$WORK_DIR/repo/" 2>/dev/null | wc -l | tr -d ' ') entries)"
else
  echo "    WARNING: repo directory not found at $REPO_DIR"
fi

# Update .discinfo if present
if [ -f "$WORK_DIR/.discinfo" ]; then
  sed -i 's/AlmaLinux/THISCLOUD/g' "$WORK_DIR/.discinfo" 2>/dev/null || true
  echo "    .discinfo updated"
fi

# Update .treeinfo if present — the installer reads it to identify the
# product (family / name / variants); leaving "AlmaLinux" makes the
# installer source still show AlmaLinux.
if [ -f "$WORK_DIR/.treeinfo" ]; then
  sed -i 's/AlmaLinux/THISCLOUD/g' "$WORK_DIR/.treeinfo" 2>/dev/null || true
  echo "    .treeinfo updated"
fi

# ── Rebuild ISO ──────────────────────────────────────────────────────
echo "==> [9/11] Rebuilding ISO with xorriso"
mkdir -p "$(dirname "$OUTPUT_ISO")"
rm -f "$OUTPUT_ISO"

if command -v free >/dev/null 2>&1; then
  echo "    free memory before rebuild: $(free -m | awk '/Mem:/{print $7 " MB available"}')"
fi

# Use xorriso to rebuild the ISO from the extracted tree.
# Source dir is passed as positional arg (mkisofs-style); no -map.
xorriso -as mkisofs \
  -o "$OUTPUT_ISO" \
  -b "$BOOT_BIOS" \
  -c isolinux/boot.cat \
  --no-emul-boot \
  --boot-load-size 4 \
  --boot-info-table \
  -eltorito-alt-boot \
  -e "$BOOT_EFI" \
  --no-emul-boot \
  -J \
  -R \
  -l \
  -V "$VOLID" \
  "$WORK_DIR" \
  2>&1 | tail -20

XORRISO_RC=${PIPESTATUS[0]}
if [ "$XORRISO_RC" -ne 0 ]; then
  echo "error: xorriso failed with exit code $XORRISO_RC"
  echo "       Check the output above for details."
  if [ "$XORRISO_RC" -ge 128 ]; then
    echo ""
    echo "       The xorriso process was killed by signal $((XORRISO_RC - 128))."
    echo "       This is usually the OOM killer (out of memory)."
    if command -v free >/dev/null 2>&1; then
      echo "       --- free -m ---"
      free -m
    fi
    if command -v dmesg >/dev/null 2>&1; then
      echo "       --- recent OOM evidence ---"
      dmesg 2>/dev/null | grep -iE "killed process|out of memory" | tail -10 || true
    fi
    echo ""
    echo "       Fix: the builder needs >=4 GB free RAM during the ISO rebuild,"
    echo "       or enlarge the builder VM memory."
  fi
  exit 1
fi

if [ ! -f "$OUTPUT_ISO" ]; then
  echo "error: ISO file was not created at $OUTPUT_ISO"
  exit 1
fi

echo "    ISO rebuilt: $(ls -lh "$OUTPUT_ISO" | awk '{print $5}')"

# ── Implant MD5 checksum ────────────────────────────────────────────
echo "==> [10/11] Implanting MD5 checksum"
implantisomd5 "$OUTPUT_ISO" 2>/dev/null || echo "    WARNING: implantisomd5 failed"

# ── Verify ───────────────────────────────────────────────────────────
echo "==> [11/11] Verifying ISO"
if [ -f "$OUTPUT_ISO" ]; then
  ISO_SIZE=$(ls -lh "$OUTPUT_ISO" | awk '{print $5}')
  ISO_LABEL=$(xorriso -indev "$OUTPUT_ISO" -pvd_info 2>/dev/null | grep "Volume id" | awk '{print $3}' || echo "unknown")
  echo "    File: $OUTPUT_ISO"
  echo "    Size: $ISO_SIZE"
  echo "    Label: $ISO_LABEL"
  echo ""
  echo "==> THISCLOUD ISO ready!"
else
  echo "    ERROR: ISO was not created"
  exit 1
fi

# Clean up working directory
echo "==> Cleaning up $WORK_DIR"
rm -rf "$WORK_DIR"
