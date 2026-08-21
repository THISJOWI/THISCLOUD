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
# The dnf repo root is the thiscloud/ subdir: createrepo_c runs on it in
# build-iso.sh step [7] (RPM_DIR=iso/repo/thiscloud), so that's where the
# RPMs + repodata/ live. The repo/ root holds binaries (cloud-hypervisor,
# thiscloud-api, etc.) and web-ui/, not RPM metadata.
REPO_DIR="$ISO_REPO/thiscloud"

echo "==> building THISCLOUD live ISO"
mkdir -p "$OUT"

# 1. Build Calamares + KPMcore (from source) and package them as RPMs
#    into the local THISCLOUD repo (iso/repo/thiscloud).
if [ ! -f "$REPO_DIR/repodata/repomd.xml" ]; then
  echo "ERROR: $REPO_DIR is not a dnf repo (no repodata/). Run build-iso.sh step [7] first (createrepo_c on iso/repo/thiscloud)." >&2
  exit 1
fi
THISCLOUD_REPO_RPMS="$REPO_DIR" bash "$SCRIPT_DIR/build-calamares.sh" "$LIVE_ROOT"

# 2. Point live.ks at a host-visible repo. livemedia-creator runs with
#    --no-virt, so the file:// baseurl is reachable from the host.
#    live.ks baseurl is file:///data/thiscloud-repo — copy the dnf repo
#    root (thiscloud/) so dnf sees repodata/ + RPMs at the URL root.
mkdir -p "$(dirname "$LOCAL_REPO")"
if [ "$(readlink -f "$REPO_DIR")" != "$(readlink -f "$LOCAL_REPO")" ]; then
  rm -rf "$LOCAL_REPO"
  cp -a "$REPO_DIR"/. "$LOCAL_REPO"/
fi
# Builder-verified detail: if no-virt dnf can't reach the host file:// URL,
# serve it over http instead — `python3 -m http.server 8080 -d "$LOCAL_REPO"`
# and set `repo --name=thiscloud-local --baseurl=http://127.0.0.1:8080`
# in live.ks. Keep this line in sync with the repo URL in live.ks.

# 3. Assemble the live ISO. Package set (incl. the calamares RPM that
#    bundles KPMcore) comes from %packages in live.ks, resolved against the
#    local repo. livemedia-creator --iso-only writes the ISO to a temp dir
#    named `--iso-name`; --resultdir copies it into $OUT afterwards.
livemedia-creator --make-iso --no-virt --iso-only \
  --ks "$CALAMAES_DIR/live/live.ks" \
  --source "$ALMAISO" \
  --resultdir "$OUT" \
  --project "THISCLOUD" \
  --releasever 9 \
  --volid "THISCLOUD-${VERSION}" \
  --iso-name "ThisCloud-${VERSION}-x86_64.iso"

echo "==> Done"
ls -lh "$OUT"/*.iso