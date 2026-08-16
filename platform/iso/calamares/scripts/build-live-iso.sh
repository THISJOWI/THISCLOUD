#!/usr/bin/env bash
# Build the THISCLOUD live ISO hosting the Calamares installer.
# MUST run on AlmaLinux 9 x86_64 (livemedia-creator/lorax).
#
#   ALMAISO=/data/AlmaLinux-9-latest-x86_64-minimal.iso ./scripts/build-live-iso.sh
set -euo pipefail

ALMAISO="${ALMAISO:-/data/AlmaLinux-9-latest-x86_64-minimal.iso}"
OUT="${OUT:-/data/thiscloud-live-iso}"
VERSION="${VERSION:-0.1.0}"
LIVE_ROOT="${LIVE_ROOT:-/tmp/live-root}"
LOCAL_REPO="${LOCAL_REPO:-/data/thiscloud-repo}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CALAMAES_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ISO_REPO="$(cd "$CALAMAES_DIR/.." && pwd)/repo"   # existing THISCLOUD RPM repo

echo "==> building THISCLOUD live ISO"
mkdir -p "$OUT"

# 1. Build Calamares + KPMcore (from source) and package them as RPMs
#    into the local THISCLOUD repo (iso/repo).
if [ ! -f "$ISO_REPO/repodata/repomd.xml" ]; then
  echo "ERROR: $ISO_REPO is not a dnf repo (no repodata/). Run build-iso.sh step [1-4] first." >&2
  exit 1
fi
THISCLOUD_REPO_RPMS="$ISO_REPO" bash "$SCRIPT_DIR/build-calamares.sh" "$LIVE_ROOT"

# 2. Point live.ks at a host-visible repo. livemedia-creator runs with
#    --no-virt, so the file:// baseurl is reachable from the host.
mkdir -p "$(dirname "$LOCAL_REPO")"
if [ "$(readlink -f "$ISO_REPO")" != "$(readlink -f "$LOCAL_REPO")" ]; then
  rm -rf "$LOCAL_REPO"
  cp -a "$ISO_REPO"/. "$LOCAL_REPO"/
fi
# Builder-verified detail: if no-virt dnf can't reach the host file:// URL,
# serve it over http instead — `python3 -m http.server 8080 -d "$LOCAL_REPO"`
# and set `repo --name=thiscloud-local --baseurl=http://127.0.0.1:8080`
# in live.ks. Keep this line in sync with the repo URL in live.ks.

# 3. Assemble the live ISO. Package set (incl. calamares/kpmcore RPMs)
#    comes from %packages in live.ks, resolved against the local repo.
livemedia-creator --make-iso --no-virt --iso-only \
  --ks "$CALAMAES_DIR/live/live.ks" \
  --source "$ALMAISO" \
  --resultdir "$OUT" \
  --project "THISCLOUD" \
  --releasever 9 \
  --volid "THISCLOUD-${VERSION}"

echo "==> Done"
ls -lh "$OUT"/*.iso