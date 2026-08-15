#!/usr/bin/env bash
# session-secret.sh — Generate a persistent SESSION_SECRET for the web UI.
# Called by thiscloud-webui.service via ExecStartPre.
# Writes to /etc/thiscloud/web-ui.env if SESSION_SECRET is not already set.
set -euo pipefail

ENV_FILE="/etc/thiscloud/web-ui.env"

# Ensure directory exists
mkdir -p "$(dirname "$ENV_FILE")"

# If env file exists and already has SESSION_SECRET, nothing to do
if [ -f "$ENV_FILE" ] && grep -q '^SESSION_SECRET=' "$ENV_FILE" 2>/dev/null; then
  echo "==> SESSION_SECRET already configured"
  exit 0
fi

# Generate a random 64-byte hex secret
SECRET=$(head -c 64 /dev/urandom | od -An -tx1 | tr -d ' \n')

# Append or create
if [ -f "$ENV_FILE" ]; then
  # Remove old entry if present (race guard)
  grep -v '^SESSION_SECRET=' "$ENV_FILE" > "${ENV_FILE}.tmp" || true
  echo "SESSION_SECRET=${SECRET}" >> "${ENV_FILE}.tmp"
  mv "${ENV_FILE}.tmp" "$ENV_FILE"
else
  echo "SESSION_SECRET=${SECRET}" > "$ENV_FILE"
fi

chmod 600 "$ENV_FILE"
echo "==> SESSION_SECRET generated and written to $ENV_FILE"
