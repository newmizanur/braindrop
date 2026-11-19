#!/bin/sh
# First-time Ubuntu Server / systemd setup for BrainDrop.
#
# Run as root on the target host. `scripts/deploy.sh init` runs this after
# uploading a release and provisioning shared/.env. It is safe to re-run: the
# service account, directories and existing .env are preserved.
#
# Required Ubuntu packages:
#   apt-get install -y ca-certificates curl openssl sqlite3 tzdata
set -eu

REMOTE_DIR="${REMOTE_DIR:-/opt/braindrop}"
SERVICE="${SERVICE:-braindrop}"
SERVICE_USER="${SERVICE_USER:-braindrop}"
APP_PORT="${APP_PORT:-5150}"
ENV_FILE="$REMOTE_DIR/shared/.env"
CURRENT="$REMOTE_DIR/current"

die() {
  echo "error: $*" >&2
  exit 1
}

[ "$(id -u)" = "0" ] || die "run this script as root"
command -v systemctl >/dev/null 2>&1 || die "systemd is required"
[ -f "$CURRENT/braindrop" ] || die "no active release at $CURRENT; run deploy.sh init"
[ -f "$ENV_FILE" ] || die "missing $ENV_FILE; run deploy.sh init"

if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  useradd --system --home-dir "$REMOTE_DIR" --shell /usr/sbin/nologin "$SERVICE_USER"
fi

mkdir -p "$REMOTE_DIR/shared/data" "$REMOTE_DIR/shared/backups" "$REMOTE_DIR/shared/logs"
chown -R "$SERVICE_USER:$SERVICE_USER" "$REMOTE_DIR/shared"
chmod 750 "$REMOTE_DIR/shared" "$REMOTE_DIR/shared/data"
chmod 600 "$ENV_FILE"
chown "$SERVICE_USER:$SERVICE_USER" "$ENV_FILE"

cat >"/etc/systemd/system/$SERVICE.service" <<EOF
[Unit]
Description=BrainDrop personal inbox, tasks and reminders
Documentation=file://$REMOTE_DIR/current/DEPLOYMENT.md
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$CURRENT
Environment=LOCO_ENV=production
EnvironmentFile=$ENV_FILE
ExecStart=$CURRENT/braindrop start --all
Restart=on-failure
RestartSec=5
TimeoutStopSec=30
KillSignal=SIGTERM

# Hardening. The app only writes under shared/ (databases, backups, logs).
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=true
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectControlGroups=true
RestrictSUIDSGID=true
LockPersonality=true
ReadWritePaths=$REMOTE_DIR/shared
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
