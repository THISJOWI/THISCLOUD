# THISCLOUD — AlmaLinux 9 kickstart
# Auto-provisions THISCLOUD with all dependencies: cloud-hypervisor,
# the Go API server, the Rust daemon, the Next.js web UI, OVN/OVS,
# DRBD/Linstor, and etcd.
# This file is embedded into the ISO by remix-iso.sh.

# ── Installer behaviour ──────────────────────────────────────────────
# Remove 'text' mode so Anaconda uses graphical if available.
# This lets the user configure ALL spokes (lang, keyboard, timezone,
# root password, storage, network) during installation.
# If graphical isn't available (headless/minimal ISO), Anaconda
# falls back to text mode automatically.

# Show ALL configuration screens on first boot.
firstboot --reconfig

# Language default (user can change during Anaconda install).
lang en_US.UTF-8
# Do NOT set keyboard here — let Anaconda show the Keyboard spoke so the
# user can pick their layout.  When kickstart provides a value Anaconda
# may skip or override the spoke, losing the user's choice.

# ── Accounts ─────────────────────────────────────────────────────────
# Do NOT pre-set root password — let the user choose during installation.
# Anaconda locks the root-password spoke when kickstart provides a value.

# ── Network ──────────────────────────────────────────────────────────
network --bootproto=dhcp --device=link --activate --onboot=yes

# ── Storage ──────────────────────────────────────────────────────────
zerombr
clearpart --all --initlabel
autopart --type=lvm

# ── Source ───────────────────────────────────────────────────────────
cdrom

# ── Anaconda security policies ──────────────────────────────────────
%anaconda
pwpolicy root --minlen=6
%end

# ── Packages ─────────────────────────────────────────────────────────
%packages
@core
@standard
%end

# ── Post-install: copy files from install media (HOST environment) ──
# This runs OUTSIDE the chroot. We need to find where Anaconda mounted
# the ISO and copy THISCLOUD files to the target before the chroot %post.
# Logs to /root/ks-post-nochroot.log (on the target).
%post --nochroot --log=/mnt/sysimage/root/ks-post-nochroot.log

echo "==> THISCLOUD nochroot: starting media copy"
echo "    Date: $(date)"

# Find the install media. Anaconda's mount point varies by version:
#   - /run/install/repo/       (RHEL 8+, EL9 standard)
#   - /mnt/source/             (some Anaconda versions)
#   - /mnt/install/repo/       (older Anaconda)
#   - /tmp/anaconda-repo/      (rare fallback)
#
# Our ISO layout puts THISCLOUD files under repo/ on the ISO root:
#   repo/thiscloud/            RPMs + repodata
#   repo/cloud-hypervisor      binary
#   repo/thiscloud-api         binary
#   repo/web-ui/               Next.js standalone
#   repo/systemd/              service units
#
# So after Anaconda mounts the ISO, the files are at <mount>/repo/.

MEDIA_BASES="/run/install/repo /mnt/source /mnt/install/repo /tmp/anaconda-repo"
SOURCE_DIR=""
MOUNTED_DIR=""

for BASE in $MEDIA_BASES; do
  if [ -d "$BASE/repo" ]; then
    echo "==> Found install media at $BASE"
    SOURCE_DIR="$BASE/repo"
    break
  fi
done

# Fallback: use findmnt to locate any mount of /dev/sr0 or /dev/cdrom
if [ -z "$SOURCE_DIR" ]; then
  echo "==> Standard paths missed, trying findmnt"
  for DEV in /dev/sr0 /dev/cdrom; do
    if [ -e "$DEV" ]; then
      MNT=$(findmnt -n -o TARGET "$DEV" 2>/dev/null || true)
      if [ -n "$MNT" ] && [ -d "$MNT/repo" ]; then
        echo "==> Found media via findmnt: $MNT"
        SOURCE_DIR="$MNT/repo"
        break
      fi
    fi
  done
fi

# Last resort: try to mount /dev/sr0 (may fail if busy)
if [ -z "$SOURCE_DIR" ]; then
  echo "==> Last resort: trying to mount /dev/sr0"
  mkdir -p /mnt/source
  if mount /dev/sr0 /mnt/source 2>/dev/null; then
    MOUNTED_DIR="/mnt/source"
    if [ -d /mnt/source/repo ]; then
      SOURCE_DIR="/mnt/source/repo"
    fi
  fi
fi

if [ -z "$SOURCE_DIR" ]; then
  echo "FATAL: No install media found."
  echo "       THISCLOUD components will not be installed."
  echo "       The system will boot but services will be missing."
  echo "       Check /root/ks-post-nochroot.log for details."
  exit 0
fi

echo "==> Source directory: $SOURCE_DIR"
echo "    Contents:"
ls -la "$SOURCE_DIR/" 2>/dev/null || echo "    (listing failed)"

# ── Copy THISCLOUD repo (RPMs + repodata) ────────────────────────────
echo "==> Copying THISCLOUD repo"
mkdir -p /mnt/sysimage/tmp/thiscloud-repo
if [ -d "$SOURCE_DIR/thiscloud" ]; then
  cp -a "$SOURCE_DIR/thiscloud" /mnt/sysimage/tmp/thiscloud-repo/
  RPM_COUNT=$(ls /mnt/sysimage/tmp/thiscloud-repo/thiscloud/*.rpm 2>/dev/null | wc -l)
  echo "    Repo copied: $RPM_COUNT RPMs"
else
  echo "ERROR: $SOURCE_DIR/thiscloud not found — no RPMs will be installed"
fi

# ── Copy binary artifacts ────────────────────────────────────────────
echo "==> Copying binary artifacts"
# Also map each binary to its final install destination so we can
# install directly from nochroot, bypassing any staging path issues.
# Copy open-ports script
if [ -f "$SOURCE_DIR/thiscloud-open-ports" ]; then
  cp -f "$SOURCE_DIR/thiscloud-open-ports" /mnt/sysimage/tmp/
  chmod 755 /mnt/sysimage/tmp/thiscloud-open-ports
  echo "    thiscloud-open-ports copied"
fi

# Copy web-port script
if [ -f "$SOURCE_DIR/thiscloud-open-web-port" ]; then
  cp -f "$SOURCE_DIR/thiscloud-open-web-port" /mnt/sysimage/tmp/
  chmod 755 /mnt/sysimage/tmp/thiscloud-open-web-port
  echo "    thiscloud-open-web-port copied"
fi

# Copy session-secret generator script
if [ -f "$SOURCE_DIR/thiscloud-session-secret" ]; then
  cp -f "$SOURCE_DIR/thiscloud-session-secret" /mnt/sysimage/tmp/
  chmod 755 /mnt/sysimage/tmp/thiscloud-session-secret
  echo "    thiscloud-session-secret copied"
fi

for BIN in cloud-hypervisor thiscloud-api thiscloudd thiscloud-cli; do
  if [ -f "$SOURCE_DIR/$BIN" ]; then
    # Stage to /tmp (used by chroot fallback)
    cp -f "$SOURCE_DIR/$BIN" /mnt/sysimage/tmp/
    chmod 755 /mnt/sysimage/tmp/$BIN
    # Install directly to final destination from nochroot
    case "$BIN" in
      cloud-hypervisor) DEST="/mnt/sysimage/usr/local/bin/$BIN" ;;
      thiscloud-api)    DEST="/mnt/sysimage/usr/local/bin/$BIN" ;;
      thiscloudd)       DEST="/mnt/sysimage/usr/sbin/$BIN" ;;
      thiscloud-cli)    DEST="/mnt/sysimage/usr/bin/thiscloud" ;;
    esac
    mkdir -p "$(dirname "$DEST")"
    cp -f "$SOURCE_DIR/$BIN" "$DEST"
    chmod 755 "$DEST"
    echo "    $BIN copied -> $DEST ($(stat -c%s "$DEST" 2>/dev/null || echo ?) bytes)"
  else
    echo "ERROR: $SOURCE_DIR/$BIN not found — listing repo dir:"
    ls -la "$SOURCE_DIR/" 2>/dev/null || echo "    (cannot list)"
  fi
done

# ── Copy web-ui directory ────────────────────────────────────────────
echo "==> Copying web-ui"
mkdir -p /mnt/sysimage/tmp/thiscloud-web-ui
if [ -d "$SOURCE_DIR/web-ui" ]; then
  cp -a "$SOURCE_DIR/web-ui"/. /mnt/sysimage/tmp/thiscloud-web-ui/ 2>/dev/null
  FILE_COUNT=$(find /mnt/sysimage/tmp/thiscloud-web-ui/ -maxdepth 1 | wc -l)
  echo "    web-ui copied: $FILE_COUNT items (including hidden dirs)"
else
  echo "ERROR: $SOURCE_DIR/web-ui not found"
fi

# ── Copy systemd unit files ──────────────────────────────────────────
echo "==> Copying systemd units"
mkdir -p /mnt/sysimage/tmp/thiscloud-systemd
if [ -d "$SOURCE_DIR/systemd" ]; then
  cp -f "$SOURCE_DIR/systemd"/*.service /mnt/sysimage/tmp/thiscloud-systemd/ 2>/dev/null
  UNIT_COUNT=$(ls /mnt/sysimage/tmp/thiscloud-systemd/*.service 2>/dev/null | wc -l)
  echo "    systemd units copied: $UNIT_COUNT files"
else
  echo "ERROR: $SOURCE_DIR/systemd not found"
fi

# ── Summary ──────────────────────────────────────────────────────────
echo "==> Nochroot copy summary:"
echo "    Repo:      $(ls /mnt/sysimage/tmp/thiscloud-repo/thiscloud/*.rpm 2>/dev/null | wc -l) RPMs"
echo "    Binaries:  cloud-hypervisor=$(test -f /mnt/sysimage/tmp/cloud-hypervisor && echo OK || echo MISSING)"
echo "               thiscloud-api=$(test -f /mnt/sysimage/tmp/thiscloud-api && echo OK || echo MISSING)"
echo "               thiscloudd=$(test -f /mnt/sysimage/tmp/thiscloudd && echo OK || echo MISSING)"
echo "               thiscloud=$(test -f /mnt/sysimage/tmp/thiscloud && echo OK || echo MISSING)"
echo "    Web UI:    $(ls /mnt/sysimage/tmp/thiscloud-web-ui/ 2>/dev/null | wc -l) files"
echo "    Systemd:   $(ls /mnt/sysimage/tmp/thiscloud-systemd/*.service 2>/dev/null | wc -l) units"
echo "    Ports:     thiscloud-open-ports=$(test -f /mnt/sysimage/tmp/thiscloud-open-ports && echo OK || echo MISSING)"

# Cleanup mount if we created it
if [ -n "$MOUNTED_DIR" ] && mountpoint -q "$MOUNTED_DIR" 2>/dev/null; then
  umount "$MOUNTED_DIR" 2>/dev/null || true
fi

echo "==> Nochroot media copy complete"
%end

# ── Post-install: configure the target system (CHROOT) ──────────────
# This runs INSIDE the chroot (target filesystem).
%post --log=/root/ks-post.log

echo "==> THISCLOUD post-install starting"
echo "    Date: $(date)"
echo "    Hostname: $(hostname)"
echo "    Target: $(df -h / 2>/dev/null | tail -1)"

# ── Verify staging files from nochroot ───────────────────────────────
echo "==> Verifying staging files"
RPM_COUNT=$(ls /tmp/thiscloud-repo/thiscloud/*.rpm 2>/dev/null | wc -l)
echo "    RPMs in staging: $RPM_COUNT"
echo "    cloud-hypervisor: $(test -f /tmp/cloud-hypervisor && echo "OK ($(stat -c%s /tmp/cloud-hypervisor) bytes)" || echo "MISSING")"
echo "    thiscloud-api:    $(test -f /tmp/thiscloud-api && echo "OK ($(stat -c%s /tmp/thiscloud-api) bytes)" || echo "MISSING")"
echo "    thiscloudd:       $(test -f /tmp/thiscloudd && echo "OK ($(stat -c%s /tmp/thiscloudd) bytes)" || echo "MISSING")"
echo "    thiscloud-cli:    $(test -f /tmp/thiscloud-cli && echo "OK ($(stat -c%s /tmp/thiscloud-cli) bytes)" || echo "MISSING")"
echo "    web-ui files:     $(ls /tmp/thiscloud-web-ui/ 2>/dev/null | wc -l)"
echo "    systemd units:    $(ls /tmp/thiscloud-systemd/*.service 2>/dev/null | wc -l)"

if [ "$RPM_COUNT" -eq 0 ]; then
  echo "WARNING: No RPMs were staged. THISCLOUD packages will not be installed via RPM."
  echo "         Will fall back to manual binary installation from staging."
fi

# ── Point DNF at the embedded repo ──────────────────────────────────
cat > /etc/yum.repos.d/thiscloud.repo <<'REPOEOF'
[thiscloud]
name=THISCLOUD Local Repo
baseurl=file:///tmp/thiscloud-repo/thiscloud
enabled=1
gpgcheck=0
REPOEOF

# ── Refresh repos and install base packages ─────────────────────────
echo "==> Refreshing DNF repos"
dnf makecache 2>/dev/null || true

# ── Install system utilities from base repos ────────────────────────
echo "==> Installing system utilities"
dnf install -y epel-release 2>/dev/null || echo "WARNING: epel-release not available"
dnf install -y htop tmux wget curl firewalld 2>/dev/null || echo "WARNING: Some utilities failed"

# ── Install nginx + qemu-kvm from base repos ───────────────────────
echo "==> Installing nginx + qemu-kvm"
dnf install -y nginx 2>/dev/null || echo "WARNING: nginx install failed"
dnf install -y qemu-kvm 2>/dev/null || echo "WARNING: qemu-kvm install failed"

# ── Install Node.js via NodeSource ──────────────────────────────────
echo "==> Installing Node.js"
if ! command -v node >/dev/null 2>&1; then
  curl -fsSL https://rpm.nodesource.com/setup_20.x | bash - 2>/dev/null || echo "WARNING: NodeSource setup failed"
  dnf install -y nodejs 2>/dev/null || echo "WARNING: nodejs install failed"
else
  echo "    Node.js already installed: $(node --version)"
fi

# ── Install THISCLOUD packages from the local repo ─────────────────
echo "==> Installing THISCLOUD packages"
dnf install -y --enablerepo=thiscloud thiscloud thiscloudd etcd 2>&1 | tee /tmp/dnf-thiscloud.log | tail -20
DNF_OK=0
if [ "${PIPESTATUS[0]}" -ne 0 ]; then
  DNF_OK=1
  echo "WARNING: THISCLOUD RPM packages failed to install (see /tmp/dnf-thiscloud.log)"
fi
# Always verify critical binaries exist — RPM install can "succeed" but skip
# files due to conflicts or missing deps.
NEED_FALLBACK=0
for BIN_PAIR in "thiscloudd:/usr/sbin/thiscloudd" "thiscloud-cli:/usr/bin/thiscloud"; do
  NAME="${BIN_PAIR%%:*}"
  DEST="${BIN_PAIR##*:}"
  if [ ! -f "$DEST" ] || [ ! -x "$DEST" ]; then
    NEED_FALLBACK=1
    echo "WARNING: $DEST missing or not executable after dnf install"
  fi
done
if [ "$NEED_FALLBACK" -eq 1 ]; then
  echo "==> Falling back to manual binary installation from staging..."
  for BIN_PAIR in "thiscloudd:/usr/sbin/thiscloudd" "thiscloud-cli:/usr/bin/thiscloud"; do
    NAME="${BIN_PAIR%%:*}"
    DEST="${BIN_PAIR##*:}"
    if [ -f "/tmp/$NAME" ]; then
      install -m 0755 "/tmp/$NAME" "$DEST"
      echo "    $NAME installed to $DEST (fallback)"
    else
      echo "WARNING: /tmp/$NAME not found — $NAME NOT installed"
    fi
  done
  # Install systemd units from staging
  if [ -d /tmp/thiscloud-systemd ]; then
    cp -f /tmp/thiscloud-systemd/*.service /etc/systemd/system/ 2>/dev/null
    echo "    systemd units installed from staging"
  fi
fi

# ── Install OVN/OVS, DRBD, Linstor (if available) ──────────────────
echo "==> Installing optional packages"
dnf install -y --enablerepo=thiscloud openvswitch ovn ovn-central 2>/dev/null || echo "INFO: OVN/OVS not available"
dnf install -y --enablerepo=thiscloud drbd-utils 2>/dev/null || echo "INFO: DRBD not available"
dnf install -y --enablerepo=thiscloud linstor linstor-client linstor-common 2>/dev/null || echo "INFO: Linstor not available"

# ── cloud-hypervisor binary ─────────────────────────────────────────
echo "==> Installing cloud-hypervisor"
if [ -f /tmp/cloud-hypervisor ]; then
  install -m 0755 /tmp/cloud-hypervisor /usr/local/bin/cloud-hypervisor
  /usr/local/bin/cloud-hypervisor --version 2>&1 | head -1 || echo "WARNING: cloud-hypervisor binary check failed"
else
  echo "WARNING: cloud-hypervisor binary not found in staging"
fi

# ── THISCLOUD Go API binary ─────────────────────────────────────────
echo "==> Installing thiscloud-api"
if [ -f /tmp/thiscloud-api ]; then
  install -m 0755 /tmp/thiscloud-api /usr/local/bin/thiscloud-api
else
  echo "WARNING: thiscloud-api binary not found in staging"
fi

# ── THISCLOUD CLI binary ────────────────────────────────────────────
echo "==> Installing thiscloud CLI"
if [ -f /tmp/thiscloud-cli ]; then
  install -m 0755 /tmp/thiscloud-cli /usr/bin/thiscloud
  echo "    thiscloud CLI installed to /usr/bin/thiscloud"
else
  echo "WARNING: thiscloud-cli binary not found in staging"
fi

# ── THISCLOUD Web UI (Next.js standalone) ───────────────────────────
echo "==> Installing web-ui"
mkdir -p /usr/share/thiscloud/web-ui
if [ -d /tmp/thiscloud-web-ui ] && [ "$(ls -A /tmp/thiscloud-web-ui 2>/dev/null)" ]; then
  cp -a /tmp/thiscloud-web-ui/. /usr/share/thiscloud/web-ui/
  chmod 755 /usr/share/thiscloud/web-ui/server.js 2>/dev/null || true
  echo "    web-ui installed: $(find /usr/share/thiscloud/web-ui/ -maxdepth 1 | wc -l) items (including hidden dirs)"
else
  echo "WARNING: web-ui not found in staging"
fi

# ── THISCLOUD Web UI secrets ────────────────────────────────────────
echo "==> Configuring web-ui secrets"
mkdir -p /etc/thiscloud
if [ ! -f /etc/thiscloud/web-ui.env ]; then
  SESSION_SECRET="$(openssl rand -hex 32 2>/dev/null || head -c 32 /dev/urandom | base64 | tr -d '\n')"
  umask 077
  cat > /etc/thiscloud/web-ui.env <<EOF
SESSION_SECRET=${SESSION_SECRET}
EOF
  umask 022
fi
chmod 600 /etc/thiscloud/web-ui.env

# ── Systemd services ────────────────────────────────────────────────
echo "==> Installing systemd services"
if [ -d /tmp/thiscloud-systemd ]; then
  cp -f /tmp/thiscloud-systemd/*.service /etc/systemd/system/ 2>/dev/null
  echo "    systemd units installed: $(ls /tmp/thiscloud-systemd/*.service 2>/dev/null | wc -l) files"
else
  echo "WARNING: systemd units not found in staging"
fi

# ── Nginx reverse-proxy for the Web UI ──────────────────────────────
echo "==> Configuring nginx"
cat > /etc/nginx/conf.d/thiscloud-ui.conf <<'NGINXEOF'
server {
    listen 80 default_server;
    server_name _;

    # Proxy everything to the Next.js server on port 3000
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_cache_bypass $http_upgrade;
    }
}
NGINXEOF

# Remove the default nginx server block to avoid port 80 conflicts.
rm -f /etc/nginx/conf.d/default.conf 2>/dev/null || true
sed -i '/^\s*server\s*{/,/^\s*}/d' /etc/nginx/nginx.conf 2>/dev/null || true

# ── Firewall ────────────────────────────────────────────────────────
echo "==> Configuring firewall"
systemctl enable firewalld 2>/dev/null || true
# Install the port-opening script (runs at boot via systemd)
if [ -f /tmp/thiscloud-open-ports ]; then
  install -m 0755 /tmp/thiscloud-open-ports /usr/local/bin/thiscloud-open-ports
  echo "    thiscloud-open-ports installed"
else
  echo "WARNING: thiscloud-open-ports script not found in staging"
fi

# Install the dedicated web port script (opens port 80 for the web UI)
if [ -f /tmp/thiscloud-open-web-port ]; then
  install -m 0755 /tmp/thiscloud-open-web-port /usr/local/bin/thiscloud-open-web-port
  echo "    thiscloud-open-web-port installed"
else
  echo "WARNING: thiscloud-open-web-port script not found in staging"
fi

# Install the session-secret generator (creates signing key for web UI sessions)
if [ -f /tmp/thiscloud-session-secret ]; then
  install -m 0755 /tmp/thiscloud-session-secret /usr/local/bin/thiscloud-session-secret
  echo "    thiscloud-session-secret installed"
else
  echo "WARNING: thiscloud-session-secret script not found in staging"
fi

# ── Hostname ────────────────────────────────────────────────────────
echo "==> Setting hostname"
hostnamectl set-hostname thiscloud

# ── Create state directory for go-api ───────────────────────────────
mkdir -p /var/lib/thiscloud

# ── Installed system branding ──────────────────────────────────────
echo "==> Applying THISCLOUD branding"

cat > /etc/os-release <<'OSRELEASE'
NAME="THISCLOUD"
VERSION="0.1.0 (Nucleus)"
ID="thiscloud"
ID_LIKE="rhel fedora"
VERSION_ID="0.1.0"
PLATFORM_ID="platform:el9"
PRETTY_NAME="THISCLOUD 0.1.0 (Nucleus)"
ANSI_COLOR="0;31"
CPE_NAME="cpe:/o:thiscloud:thiscloud:0.1.0"
HOME_URL="https://github.com/THISJOWI/THISCLOUD"
BUG_REPORT_URL="https://github.com/THISJOWI/THISCLOUD/issues"
OSRELEASE

cat > /etc/almalinux-release <<'ALMARELEASE'
THISCLOUD release 0.1.0 (Nucleus)
ALMARELEASE

cat > /etc/system-release <<'SYSRELEASE'
THISCLOUD release 0.1.0 (Nucleus)
SYSRELEASE

cat > /etc/redhat-release <<'REDHATRELEASE'
THISCLOUD release 0.1.0 (Nucleus)
REDHATRELEASE

cat > /etc/issue <<'ISSUE'

THISCLOUD 0.1.0 (Nucleus)
Kernel \r on an \m

ISSUE

cat > /etc/motd <<'MOTD'

  _____ _   _ ______   _____ _  ______   _____              _
 |_   _| | | |  ____| |_   _| |/ / __ \ / ____|            | |
   | | | |_| | |__      | | | ' / |  | | (___  _ __   __ _| | _____
   | | |  _  |  __|     | | |  <| |  | |\___ \| '_ \ / _` | |/ / _ \
  _| |_| | | | |        | | | . \ |__| |____) | | | | (_| |   <  __/
 |_____|_| |_|_|        |_| |_|\_\____/|_____/|_| |_|\__,_|_|\_\___|

 Services are already running via systemd.
 Use the CLI (not the daemon/API binaries directly):
   thiscloud vm list
   thiscloud vm create --name <name> --cpus 2 --memory 2048
   thiscloud vm create --name <name> --cpus 2 --memory 4096 --disk /path/to/disk.qcow2 --network mynet
   thiscloud network list
   thiscloud network create --name mynet --cidr 10.0.0.0/24
MOTD

# GRUB: THISCLOUD branding — title, colors, and custom menu entry
if [ -f /etc/default/grub ]; then
  sed -i 's/^GRUB_CMDLINE_LINUX=.*/GRUB_CMDLINE_LINUX="crashkernel=auto quiet"/' /etc/default/grub
  # Replace AlmaLinux distributor name with THISCLOUD
  sed -i 's/^GRUB_DISTRIBUTOR=.*/GRUB_DISTRIBUTOR="THISCLOUD"/' /etc/default/grub
  # Brand colors (matching web-ui #0f1115 bg / #3b82f6 accent)
  grep -q '^GRUB_COLOR_NORMAL=' /etc/default/grub || \
    echo 'GRUB_COLOR_NORMAL="white/black"' >> /etc/default/grub
  grep -q '^GRUB_COLOR_HIGHLIGHT=' /etc/default/grub || \
    echo 'GRUB_COLOR_HIGHLIGHT="#3b82f6/black"' >> /etc/default/grub
  # Regenerate GRUB config
  grub2-mkconfig -o /boot/grub2/grub.cfg 2>/dev/null || true
fi

# Custom GRUB menu entry: replace the default AlmaLinux entry with THISCLOUD
if [ -f /boot/grub2/grub.cfg ]; then
  sed -i 's/AlmaLinux/THISCLOUD/g' /boot/grub2/grub.cfg
  sed -i 's/almalinux/thiscloud/g' /boot/grub2/grub.cfg
fi

# Rename AlmaLinux repos to THISCLOUD (keep baseurl intact)
for f in /etc/yum.repos.d/almalinux-*.repo; do
  if [ -f "$f" ]; then
    NEWNAME=$(echo "$f" | sed 's/almalinux-/thiscloud-/')
    sed -i 's/name=AlmaLinux/name=THISCLOUD/g' "$f"
    mv "$f" "$NEWNAME" 2>/dev/null || true
  fi
done

# ── Clean up staging files ──────────────────────────────────────────
echo "==> Cleaning up staging files"
rm -rf /tmp/thiscloud-repo /tmp/thiscloud-web-ui /tmp/thiscloud-systemd
rm -f /tmp/cloud-hypervisor /tmp/thiscloud-api /tmp/thiscloud-open-ports /tmp/thiscloud-open-web-port /tmp/thiscloud-session-secret

# ── Enable services ─────────────────────────────────────────────────
echo "==> Enabling services"
systemctl daemon-reload 2>/dev/null || true
systemctl enable etcd 2>/dev/null || true
systemctl enable openvswitch 2>/dev/null || true
systemctl enable ovn-controller 2>/dev/null || true
systemctl enable linstor-controller 2>/dev/null || true
systemctl enable nginx 2>/dev/null || true
systemctl enable thiscloudd.service 2>/dev/null || true
systemctl enable thiscloud-api.service 2>/dev/null || true
systemctl enable thiscloud-webui.service 2>/dev/null || true
systemctl enable thiscloud-ports.service 2>/dev/null || true
systemctl enable thiscloud-web-port.service 2>/dev/null || true

# ── THISCLOUD config ────────────────────────────────────────────────
echo "==> Initializing THISCLOUD config"
mkdir -p /etc/thiscloud
if [ ! -f /etc/thiscloud/config.toml ]; then
  /usr/bin/thiscloud init --ip "${LASTIP:-127.0.0.1}" --role master 2>/dev/null || echo "WARNING: thiscloud init failed (CLI may not be installed)"
fi

echo "==> THISCLOUD post-install complete"
echo "    Installed components:"
echo "      - thiscloud CLI: $(which thiscloud 2>/dev/null || echo 'NOT FOUND') ($(test -f /usr/bin/thiscloud && stat -c%s /usr/bin/thiscloud || echo 0) bytes)"
echo "      - thiscloudd: $(which thiscloudd 2>/dev/null || echo 'NOT FOUND')"
echo "      - thiscloud-api: $(which thiscloud-api 2>/dev/null || echo 'NOT FOUND')"
echo "      - cloud-hypervisor: $(which cloud-hypervisor 2>/dev/null || echo 'NOT FOUND')"
echo "      - nginx: $(which nginx 2>/dev/null || echo 'NOT FOUND')"
echo "      - node: $(which node 2>/dev/null || echo 'NOT FOUND')"
echo ""
echo "    Services run via systemd (do NOT start them manually):"
echo "      systemctl status thiscloudd thiscloud-api thiscloud-webui"
echo ""
echo "    Use the CLI to manage resources:"
echo "      thiscloud vm list"
echo "      thiscloud vm create --name test --cpus 1 --memory 1024"
%end

# ── Kernel options ───────────────────────────────────────────────────
%addon com_redhat_kdump --disable
%end
