#!/usr/bin/env bash
# Build Calamares 3.3.14 + KPMcore 24.05.2 from source into a staging root
# that the live ISO (live.ks) pulls in as RPMs from iso/repo.
#
# MUST run on AlmaLinux 9 x86_64 with the build deps installed
# (see install-deps.sh additions). Run from platform/iso/calamares/.
#
#   ./scripts/build-calamares.sh /tmp/live-root
set -euo pipefail

STAGING="${1:?usage: build-calamares.sh /path/to/live-root}"
CALAMARES_VER="${CALAMARES_VER:-3.3.14}"
KPMCORE_VER="${KPMCORE_VER:-24.05.2}"
WORK="$(pwd)/.build"
SRC="$WORK/src"

echo "==> staging root: $STAGING"
mkdir -p "$WORK" "$SRC" "$STAGING"

# ── Fetch sources ────────────────────────────────────────────────────
echo "==> fetching calamares $CALAMARES_VER"
if [ ! -d "$SRC/calamares" ]; then
  curl -fsSL "https://github.com/calamares/calamares/archive/refs/tags/v${CALAMARES_VER}.tar.gz" -o "$WORK/calamares.tar.gz"
  tar -xzf "$WORK/calamares.tar.gz" -C "$SRC"
  mv "$SRC/calamares-${CALAMARES_VER}" "$SRC/calamares"
fi

echo "==> fetching kpmcore $KPMCORE_VER"
if [ ! -d "$SRC/kpmcore" ]; then
  curl -fsSL "https://github.com/KDE/kpmcore/archive/refs/tags/v${KPMCORE_VER}.tar.gz" -o "$WORK/kpmcore.tar.gz"
  tar -xzf "$WORK/kpmcore.tar.gz" -C "$SRC"
  mv "$SRC/kpmcore-v${KPMCORE_VER}" "$SRC/kpmcore"
fi

# ── Build KPMcore ────────────────────────────────────────────────────
echo "==> building kpmcore"
cmake -S "$SRC/kpmcore" -B "$WORK/kpmcore-build" \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_INSTALL_LIBDIR=/usr/lib64 \
  -DCMAKE_BUILD_TYPE=Release \
  -DBUILD_TESTING=OFF
cmake --build "$WORK/kpmcore-build" -j"$(nproc)"
cmake --install "$WORK/kpmcore-build" --prefix "$STAGING/usr"
DESTDIR="$STAGING" cmake --install "$WORK/kpmcore-build"

# ── Inject THISCLOUD module ─────────────────────────────────────────
echo "==> injecting thiscloudqml module"
THISCLOUD_MOD="$(pwd)/modules/thiscloudqml"
cp -r "$THISCLOUD_MOD" "$SRC/calamares/src/modules/thiscloudqml"

# ── Build Calamares ──────────────────────────────────────────────────
echo "==> configuring calamares"
cmake -S "$SRC/calamares" -B "$WORK/calamares-build" \
  -DCMAKE_INSTALL_PREFIX=/usr \
  -DCMAKE_INSTALL_LIBDIR=/usr/lib64 \
  -DCMAKE_BUILD_TYPE=Release \
  -DKPMCORE_DIR="$STAGING/usr/lib64/cmake/kpmcore" \
  -DWITH_QML=ON \
  -DWITH_PYTHON=ON \
  -DINSTALL_CONFIG=ON \
  -DSKIP_PEDANTIC=ON

echo "==> building calamares"
cmake --build "$WORK/calamares-build" -j"$(nproc)"
DESTDIR="$STAGING" cmake --install "$WORK/calamares-build"

# ── Install branding + settings + modules into staging ───────────────
echo "==> installing THISCLOUD branding/settings and module configs"
BRANDING_DIR="$(pwd)/branding/thiscloud"
install -d "$STAGING/etc/calamares/branding/thiscloud"
cp -r "$BRANDING_DIR"/. "$STAGING/etc/calamares/branding/thiscloud/"

install -d "$STAGING/etc/calamares"
cp -f "$(pwd)/settings.conf" "$STAGING/etc/calamares/settings.conf"

install -d "$STAGING/etc/calamares/modules"

# Install custom thiscloud job module
install -d "$STAGING/etc/calamares/modules/thiscloud"
cp -f "$(pwd)/modules/thiscloud/main.py" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/thiscloud_logic.py" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/module.desc" "$STAGING/etc/calamares/modules/thiscloud/"
cp -f "$(pwd)/modules/thiscloud/thiscloud.conf" "$STAGING/etc/calamares/modules/thiscloud/"

# Install Proxmox-like customized module configurations
for mod in welcome partition locale keyboard users finished; do
  if [ -f "$(pwd)/modules/$mod/$mod.conf" ]; then
    install -d "$STAGING/etc/calamares/modules/$mod"
    cp -f "$(pwd)/modules/$mod/$mod.conf" "$STAGING/etc/calamares/modules/$mod/"
    cp -f "$(pwd)/modules/$mod/$mod.conf" "$STAGING/etc/calamares/modules/$mod.conf"
  fi
done

echo "==> sanity checks"
test -x "$STAGING/usr/bin/calamares" && echo "  calamares: OK"
ls "$STAGING/usr/lib64/calamares/modules/" | grep -q thiscloudqml && echo "  thiscloudqml plugin: OK"
test -f "$STAGING/etc/calamares/branding/thiscloud/branding.desc" && echo "  branding: OK"
test -f "$STAGING/etc/calamares/settings.conf" && echo "  settings: OK"
test -f "$STAGING/etc/calamares/modules/welcome.conf" && echo "  welcome.conf: OK"
test -f "$STAGING/etc/calamares/modules/partition.conf" && echo "  partition.conf: OK"

# ── Package the staging root into RPMs for live.ks %packages ─────────
echo "==> packaging staging root into RPMs"
RPMROOT="$WORK/rpm"
mkdir -p "$RPMROOT"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
REPO_RPMS="${THISCLOUD_REPO_RPMS:-$(cd "$(pwd)/.." && pwd)/repo/thiscloud}"  # platform/iso/repo/thiscloud — createrepo'd dnf repo

rpmbuild_spec() { # $1=name $2=version $3=summary
  cat > "$RPMROOT/SPECS/$1.spec" <<EOF
%define _topdir $RPMROOT
Name: $1
Version: $2
Release: 1
Summary: $3
License: GPL-3.0-or-later
BuildArch: $(uname -m)
BuildRoot: %{_tmppath}/%{name}-%{version}-root

%description
$3. Compiled from source for AlmaLinux 9 (no EPEL9 package).

%install
rm -rf %{buildroot}
cp -a "$STAGING"/. %{buildroot}/

%files
EOF
  # Enumerate every staged file (path relative to root, leading /).
  ( cd "$STAGING" && find . -type f -o -type l | sort | sed 's|^\.|/|' ) \
    >> "$RPMROOT/SPECS/$1.spec"
}

rpmbuild_spec calamares "${CALAMARES_VER}" "Calamares installer + KPMcore for THISCLOUD"
rpmbuild --define "_topdir $RPMROOT" -bb "$RPMROOT/SPECS/calamares.spec" \
  || { echo "ERROR: rpmbuild failed (see $RPMROOT/rpms-build.log)"; exit 1; }
mkdir -p "$REPO_RPMS"
cp "$RPMROOT"/RPMS/*/*.rpm "$REPO_RPMS/" 2>/dev/null || cp "$RPMROOT"/RPMS/*.rpm "$REPO_RPMS/" 2>/dev/null || true
echo "  rpm output: $(ls "$REPO_RPMS")"

echo "==> regenerating repo metadata"
if command -v createrepo_c >/dev/null 2>&1; then
  createrepo_c --update "$REPO_RPMS"
else
  echo "WARNING: createrepo_c not found — run: dnf install -y createrepo_c; createrepo_c $REPO_RPMS"
fi

echo "DONE"