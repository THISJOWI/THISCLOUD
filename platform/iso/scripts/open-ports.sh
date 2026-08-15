#!/usr/bin/env bash
# open-ports.sh — Open firewall ports for THISCLOUD services.
# Runs at boot via thiscloud-ports.service (Type=oneshot).
set -euo pipefail

# Ports:
#   80    — nginx (web UI)
#   3000  — Next.js web UI (direct access)
#   8080  — thiscloudd daemon
#   8081  — Go API
#   2379  — etcd client
#   2380  — etcd peer

PORTS="80/tcp 3000/tcp 8080/tcp 8081/tcp 2379/tcp 2380/tcp"

echo "==> Opening THISCLOUD firewall ports"

# Ensure firewalld is running
if ! systemctl is-active --quiet firewalld; then
  systemctl start firewalld
fi

for PORT in $PORTS; do
  if ! firewall-cmd --query-port="$PORT" --permanent &>/dev/null; then
    firewall-cmd --permanent --add-port="$PORT"
    echo "    Opened $PORT"
  else
    echo "    $PORT already open"
  fi
done

firewall-cmd --reload
echo "==> Firewall ports configured"
