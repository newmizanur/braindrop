#!/bin/sh
# First-time Ubuntu Server / systemd setup for BrainDrop.
#
# Run as root on the target host. `scripts/deploy.sh init` runs this after
# installing the runtime and provisioning .env. It is safe to re-run: existing
# secrets and data are preserved.
#
# Required Ubuntu packages:
#   apt-get install -y ca-certificates curl openssl sqlite3 tzdata
set -eu

REMOTE_DIR="${REMOTE_DIR:-/opt/braindrop}"
SERVICE="${SERVICE:-braindrop}"
SERVICE_USER="${SERVICE_USER:-braindrop}"
APP_PORT="${APP_PORT:-5150}"
ENV_FILE="$REMOTE_DIR/.env"

die() {
  echo "error: $*" >&2
  exit 1
}

[ "$(id -u)" = "0" ] || die "run this script as root"
command -v systemctl >/dev/null 2>&1 || die "systemd is required"
[ -f "$REMOTE_DIR/braindrop" ] || die "no installed binary at $REMOTE_DIR/braindrop"
[ -f "$ENV_FILE" ] || die "missing $ENV_FILE; run deploy.sh init"

if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  useradd --system --home-dir "$REMOTE_DIR" --shell /usr/sbin/nologin "$SERVICE_USER"
fi

mkdir -p "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$REMOTE_DIR/logs"
chown -R "$SERVICE_USER:$SERVICE_USER" "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$REMOTE_DIR/logs"
chmod 750 "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$REMOTE_DIR/logs"
chmod 600 "$ENV_FILE"
chown "$SERVICE_USER:$SERVICE_USER" "$ENV_FILE"
chown root:root "$REMOTE_DIR" "$REMOTE_DIR/braindrop"
chmod 755 "$REMOTE_DIR" "$REMOTE_DIR/braindrop"

cat >"/etc/systemd/system/$SERVICE.service" <<EOF
[Unit]
Description=BrainDrop personal inbox, tasks and reminders
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$REMOTE_DIR
Environment=LOCO_ENV=production
EnvironmentFile=$ENV_FILE
ExecStart=$REMOTE_DIR/braindrop start --all
Restart=on-failure
RestartSec=5
TimeoutStopSec=30
KillSignal=SIGTERM

# Hardening. The app only writes under data/, backup/ and logs/.
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=true
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictSUIDSGID=true
LockPersonality=true
ReadWritePaths=$REMOTE_DIR/data $REMOTE_DIR/backup $REMOTE_DIR/logs
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
EOF
chmod 644 "/etc/systemd/system/$SERVICE.service"

systemctl daemon-reload
systemctl enable "$SERVICE" >/dev/null
systemctl restart "$SERVICE"

echo "==> Installed systemd service $SERVICE"
echo "    systemctl status $SERVICE"
echo "    systemctl restart $SERVICE"
echo "    journalctl -u $SERVICE -f"
