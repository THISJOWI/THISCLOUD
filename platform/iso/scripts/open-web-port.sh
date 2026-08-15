#!/usr/bin/env bash
# open-web-port.sh — Open the THISCLOUD web UI firewall port (80/tcp).
# Runs at boot via thiscloud-web-port.service and is also available
# as a standalone script for manual use.
#
# Supports firewalld, nftables, and iptables in order of preference.
set -euo pipefail

WEB_PORT="${THISCLOUD_WEB_PORT:-80}"

open_firewalld() {
  echo "==> firewalld detected"
  if ! systemctl is-active --quiet firewalld; then
    systemctl start firewalld
    systemctl enable firewalld 2>/dev/null || true
  fi

  if firewall-cmd --query-port="${WEB_PORT}/tcp" --permanent &>/dev/null; then
    echo "    ${WEB_PORT}/tcp already open"
  else
    firewall-cmd --permanent --add-port="${WEB_PORT}/tcp"
    firewall-cmd --reload
    echo "    Opened ${WEB_PORT}/tcp"
  fi
}

open_nftables() {
  echo "==> nftables detected"
  if command -v nft &>/dev/null; then
    # Check rule already present
    if nft list ruleset | grep -q "tcp dport ${WEB_PORT} accept"; then
      echo "    ${WEB_PORT}/tcp already open"
    else
      nft add rule inet filter input tcp dport ${WEB_PORT} accept 2>/dev/null || \
        nft add rule ip filter input tcp dport ${WEB_PORT} accept 2>/dev/null || \
        echo "    WARNING: could not add nft rule"
      echo "    Opened ${WEB_PORT}/tcp"
    fi
  fi
}

open_iptables() {
  echo "==> iptables detected"
  if ! iptables -C INPUT -p tcp --dport "${WEB_PORT}" -j ACCEPT 2>/dev/null; then
    iptables -I INPUT -p tcp --dport "${WEB_PORT}" -j ACCEPT
    echo "    Opened ${WEB_PORT}/tcp"
  else
    echo "    ${WEB_PORT}/tcp already open"
  fi
}

echo "==> Opening THISCLOUD web port ${WEB_PORT}/tcp"

if command -v firewall-cmd &>/dev/null; then
  open_firewalld
elif command -v nft &>/dev/null; then
  open_nftables
elif command -v iptables &>/dev/null; then
  open_iptables
else
  echo "    No firewall tool found (firewalld/nftables/iptables) — skipping"
fi

echo "==> Web port configured"
