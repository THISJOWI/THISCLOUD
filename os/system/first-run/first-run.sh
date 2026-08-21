#!/usr/bin/env bash
# THISCLOUD first-run agent.
# Runs once after first boot to configure the node.
# Replaces the Calamares thiscloud module logic.
#
# Prompts for (or reads from config):
#   - IP address
#   - Role (worker | controller)
#   - Cluster name (optional)
#   - Interface (auto-detected)
#
# Usage:
#   ./first-run.sh [--non-interactive] [--ip X.X.X.X] [--role worker]
set -euo pipefail

CONFIG_DIR="/etc/thiscloud"
CONFIG_FILE="$CONFIG_DIR/config.toml"
STATE_DIR="/var/lib/thpkg"
FIRST_RUN_DONE="$STATE_DIR/first-run-done"

NON_INTERACTIVE=0
IP=""
ROLE=""
CLUSTER=""
INTERFACE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --non-interactive) NON_INTERACTIVE=1; shift ;;
        --ip) IP="$2"; shift 2 ;;
        --role) ROLE="$2"; shift 2 ;;
        --cluster) CLUSTER="$2"; shift 2 ;;
        --interface) INTERFACE="$2"; shift 2 ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

# Skip if already done
if [ -f "$FIRST_RUN_DONE" ]; then
    echo "first-run: already completed, skipping"
    exit 0
fi

echo "============================================"
echo "  THISCLOUD First Run"
echo "============================================"
echo ""

# ── 1. Auto-detect interface ─────────────────────────────────────────

if [ -z "$INTERFACE" ]; then
    # Find the first non-loopback interface with a default route
    INTERFACE=$(ip route show default | awk '{print $5}' | head -1)
    if [ -z "$INTERFACE" ]; then
        # Fallback: first non-lo interface
        INTERFACE=$(ip -o link show | awk -F': ' '{print $2}' | grep -v lo | head -1)
    fi
    echo "Detected interface: ${INTERFACE:-none}"
fi

# ── 2. Get IP address ───────────────────────────────────────────────

if [ -z "$IP" ] && [ "$NON_INTERACTIVE" -eq 0 ]; then
    # Try to get current IP
    if [ -n "$INTERFACE" ]; then
        CURRENT_IP=$(ip -4 addr show "$INTERFACE" | grep -oP 'inet \K[0-9.]+' | head -1)
    fi
    echo ""
    echo "Current IP: ${CURRENT_IP:-not set}"
    read -p "Enter management IP [${CURRENT_IP:-192.168.1.100}]: " INPUT_IP
    IP="${INPUT_IP:-${CURRENT_IP:-192.168.1.100}}"
fi

if [ -z "$IP" ]; then
    echo "error: IP address is required"
    exit 1
fi

# ── 3. Get role ─────────────────────────────────────────────────────

if [ -z "$ROLE" ] && [ "$NON_INTERACTIVE" -eq 0 ]; then
    echo ""
    echo "Node roles:"
    echo "  1) worker   — runs VMs and workloads"
    echo "  2) controller — manages the cluster"
    read -p "Select role [1]: " ROLE_CHOICE
    case "${ROLE_CHOICE:-1}" in
        1) ROLE="worker" ;;
        2) ROLE="controller" ;;
        *) ROLE="worker" ;;
    esac
fi

if [ -z "$ROLE" ]; then
    ROLE="worker"
fi

# ── 4. Get cluster name (optional) ──────────────────────────────────

if [ -z "$CLUSTER" ] && [ "$NON_INTERACTIVE" -eq 0 ]; then
    read -p "Cluster name [thiscloud]: " INPUT_CLUSTER
    CLUSTER="${INPUT_CLUSTER:-thiscloud}"
fi

# ── 5. Write configuration ──────────────────────────────────────────

echo ""
echo "==> Writing configuration..."
mkdir -p "$CONFIG_DIR"

cat > "$CONFIG_FILE" << EOF
[node]
ip = "${IP}"
role = "${ROLE}"
interface = "${INTERFACE}"
cluster = "${CLUSTER}"

[storage]
driver = "local"

[network]
mode = "managed"
EOF

echo "    Config written to $CONFIG_FILE"

# ── 6. Set hostname ─────────────────────────────────────────────────

HOSTNAME="thiscloud-${ROLE}-$(echo "$IP" | tr '.' '-')"
hostnamectl set-hostname "$HOSTNAME" 2>/dev/null || true
echo "    Hostname: $HOSTNAME"

# ── 7. Configure static IP (if not DHCP) ────────────────────────────

if [ -n "$INTERFACE" ] && [ "$IP" != "dhcp" ]; then
    echo "    Configuring static IP on $INTERFACE..."
    cat > "/etc/NetworkManager/system-connections/thcloud-${INTERFACE}.nmconnection" << NMCON
[connection]
id=thcloud-${INTERFACE}
type=ethernet
interface-name=${INTERFACE}

[ipv4]
method=manual
address1=${IP}/24
gateway=
dns=8.8.8.8;
NMCON
    chmod 600 "/etc/NetworkManager/system-connections/thcloud-${INTERFACE}.nmconnection" 2>/dev/null || true
fi

# ── 8. Initialize THISCLOUD ─────────────────────────────────────────

echo ""
echo "==> Initializing THISCLOUD..."
if command -v thiscloud >/dev/null 2>&1; then
    thiscloud init --ip "$IP" --role "$ROLE" 2>&1 || {
        echo "    warning: thiscloud init returned non-zero (may be OK on first run)"
    }
else
    echo "    thiscloud CLI not found — skipping init"
    echo "    Run 'thiscloud init --ip $IP --role $ROLE' manually after boot"
fi

# ── 9. Generate session secret ──────────────────────────────────────

SECRET_FILE="$CONFIG_DIR/session-secret"
if [ ! -f "$SECRET_FILE" ]; then
    openssl rand -base64 32 > "$SECRET_FILE" 2>/dev/null || \
        head -c 64 /dev/urandom | base64 > "$SECRET_FILE"
    chmod 600 "$SECRET_FILE"
    echo "    Session secret generated"
fi

# ── 10. Mark first-run as done ──────────────────────────────────────

mkdir -p "$STATE_DIR"
date -u +"%Y-%m-%dT%H:%M:%SZ" > "$FIRST_RUN_DONE"

echo ""
echo "============================================"
echo "  First-run complete"
echo "============================================"
echo ""
echo "  IP:       $IP"
echo "  Role:     $ROLE"
echo "  Cluster:  ${CLUSTER:-thiscloud}"
echo "  Hostname: $HOSTNAME"
echo ""
echo "  Services will start on next boot."