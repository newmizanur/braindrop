#!/bin/sh
# First-time Alpine Linux / OpenRC setup for BrainDrop.
#
# Run as root on the target host. `scripts/deploy.sh init` runs this after
# installing the runtime and provisioning .env. It is safe to re-run: existing
# secrets and data are preserved.
#
# Required Alpine packages:
#   apk add ca-certificates curl openssl sqlite tzdata
set -eu

REMOTE_DIR="${REMOTE_DIR:-/opt/braindrop}"
SERVICE="${SERVICE:-braindrop}"
SERVICE_USER="${SERVICE_USER:-braindrop}"
APP_PORT="${APP_PORT:-5150}"
ENV_FILE="$REMOTE_DIR/.env"
LOG_DIR="$REMOTE_DIR/logs"

die() {
  echo "error: $*" >&2
  exit 1
}

[ "$(id -u)" = "0" ] || die "run this script as root"
command -v rc-service >/dev/null 2>&1 || die "OpenRC is required (apk add openrc)"
[ -f "$REMOTE_DIR/braindrop" ] || die "no installed binary at $REMOTE_DIR/braindrop"
[ -f "$ENV_FILE" ] || die "missing $ENV_FILE; run deploy.sh init"

if ! id "$SERVICE_USER" >/dev/null 2>&1; then
  addgroup -S "$SERVICE_USER" 2>/dev/null || true
  adduser -S -D -H -G "$SERVICE_USER" -h "$REMOTE_DIR" -s /sbin/nologin "$SERVICE_USER"
fi

mkdir -p "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$LOG_DIR"
chown -R "$SERVICE_USER:$SERVICE_USER" "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$LOG_DIR"
chmod 750 "$REMOTE_DIR/data" "$REMOTE_DIR/backup" "$LOG_DIR"
chmod 600 "$ENV_FILE"
chown "$SERVICE_USER:$SERVICE_USER" "$ENV_FILE"
chown root:root "$REMOTE_DIR" "$REMOTE_DIR/braindrop"
chmod 755 "$REMOTE_DIR" "$REMOTE_DIR/braindrop"

cat >"/etc/init.d/$SERVICE" <<EOF
#!/sbin/openrc-run
name="BrainDrop"
description="BrainDrop personal inbox, tasks and reminders"

command="$REMOTE_DIR/braindrop"
command_args="start --all"
command_user="$SERVICE_USER:$SERVICE_USER"
directory="$REMOTE_DIR"
pidfile="/run/$SERVICE.pid"
supervisor="supervise-daemon"
respawn_delay=5
respawn_max=0
supervise_daemon_args="--stdout $LOG_DIR/braindrop.log --stderr $LOG_DIR/braindrop.err.log"

if [ -f "$ENV_FILE" ]; then
    set -a
    . "$ENV_FILE"
    set +a
fi

depend() {
    need net
    after firewall
}

start_pre() {
    checkpath --directory --owner "$SERVICE_USER:$SERVICE_USER" "$LOG_DIR"
}
EOF
chmod 755 "/etc/init.d/$SERVICE"

rc-update add "$SERVICE" default >/dev/null
rc-service "$SERVICE" stop >/dev/null 2>&1 || true
rc-service "$SERVICE" start

echo "==> Installed OpenRC service $SERVICE"
echo "    rc-service $SERVICE status"
echo "    rc-service $SERVICE restart"
echo "    tail -f $LOG_DIR/braindrop.log"
