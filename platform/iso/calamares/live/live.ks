# THISCLOUD live installer kickstart.
# Builds the graphical live environment that hosts Calamares.
# The installed target system is produced BY Calamares at install time;
# this kickstart only describes the LIVE (host) environment.

# ── Boot behaviour ───────────────────────────────────────────────────
# No firstboot on the live host.
firstboot --disable

# ── Accounts ─────────────────────────────────────────────────────────
# Live host is root-only, autologin, no password (installer host only).
rootpw --lock
user --name=live --groups=wheel --iscrypted --password='$6$rounds=656000$x'

# ── Network ──────────────────────────────────────────────────────────
network --bootproto=dhcp --device=link --activate --onboot=yes

# ── Storage ──────────────────────────────────────────────────────────
# Live host does not persist; a tmpfs overlay is fine.
zerombr
clearpart --none --initlabel
part / --size=4096 --grow --fstype=ext4

# ── Source ───────────────────────────────────────────────────────────
# Live ISO built with livemedia-creator uses a different source model;
# packages come from the configured repos at build time.
#cdrom

# Local repo with THISCLOUD RPMs (thiscloud, thiscloudd, calamares,
# kpmcore — built by build-calamares.sh). Path is host-visible because
# the build runs with --no-virt.
repo --name=thiscloud-local --baseurl=file:///data/thiscloud-repo

# ── Packages ─────────────────────────────────────────────────────────
%packages
@core
# Graphical session: Xorg + lightweight WM + autologin
-xorg-x11-drv-vesa
xorg-x11-server-Xorg
xorg-x11-server-common
openbox
xterm
xsetroot
# Calamares + runtime deps (built by build-calamares.sh → RPMs in
# thiscloud-local repo)
calamares
kpmcore
python3
python3-pyqt6
qt6-qtbase
qt6-qtsvg
qt6-qtdeclarative
# THISCLOUD runtime bits (reused from existing repo)
thiscloud
thiscloudd
%end

# ── Post: wire autologin + autostart ─────────────────────────────────
%post --log=/root/live-post.log
echo "==> configuring live autologin + calamares autostart"

# openbox autostart for the 'live' user
mkdir -p /home/live/.config/openbox
cat > /home/live/.xinitrc <<'EOF'
# startx entry point — launch openbox-session (which runs autostart below)
exec openbox-session
EOF
cat > /home/live/.config/openbox/autostart <<'EOF'
# Launch the THISCLOUD installer once a display is up.
if [ -x /usr/bin/calamares ]; then
  /usr/bin/calamares --style "$(python3 -c 'print("thiscloud")')" &
fi
EOF
chown -R live:live /home/live/.config

# Xorg autologin (root on tty1 auto-starting X). Simplest reliable path:
# getty@tty1 spawns a login shell that starts X as the live user.
cat > /etc/systemd/system/x11-autologin@.service <<'EOF'
[Unit]
Description=X11 Autologin
After=systemd-user-sessions.service

[Service]
ExecStartPre=/usr/bin/install -d -o live -g live /tmp/.X11-unix
ExecStart=/usr/bin/startx -- -nolisten tcp vt1
Restart=no
User=live
Environment=HOME=/home/live

[Install]
WantedBy=multi-user.target
EOF

systemctl enable x11-autologin@tty1 2>/dev/null || true
echo "==> live post complete"
%end

# ── Kernel options ───────────────────────────────────────────────────
%addon com_redhat_kdump --disable
%end