#!/usr/bin/env bash
# BrainDrop SSH deployment.
#
# Called by the Makefile deploy-* targets. Uses plain SSH and SCP; the remote
# host needs no Docker, Rust, Node.js or rsync.
#
# Remote layout:
#   /opt/braindrop/current -> releases/<release-id>
#   /opt/braindrop/releases/<release-id>/
#   /opt/braindrop/shared/.env
#   /opt/braindrop/shared/data/        (application + queue SQLite)
#   /opt/braindrop/shared/backups/
#   /opt/braindrop/shared/logs/
#
# A routine deploy never uploads the local .env. The first deploy (`init`)
# creates a starter .env from .env.example, generates secrets, and leaves the
# file in place for the operator to review. Later deploys preserve it.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

ACTION="${1:-help}"

SSH_HOST="${SSH_HOST:-}"
SSH_PORT="${SSH_PORT:-22}"
SSH_USER="${SSH_USER:-root}"
SSH_IDENTITY="${SSH_IDENTITY:-}"
REMOTE_DIR="${REMOTE_DIR:-/opt/braindrop}"
OS="${OS:-alpine}"
ARCH="${ARCH:-amd64}"
SERVICE="${SERVICE:-braindrop}"
SERVICE_USER="${SERVICE_USER:-braindrop}"
APP_PORT="${APP_PORT:-5150}"
KEEP_RELEASES="${KEEP_RELEASES:-3}"
RELEASE_ID="${RELEASE_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)}"
ROLLBACK_RELEASE="${ROLLBACK_RELEASE:-}"
PREVIOUS_RELEASE=""
LINES="${LINES:-200}"
DIST="${DIST:-dist}"
SKIP_BUILD="${SKIP_BUILD:-0}"
AUTO_ROLLBACK="${AUTO_ROLLBACK:-1}"
GENERATE_SECRETS="${GENERATE_SECRETS:-1}"

if [[ -z "${REMOTE_SUDO+x}" ]]; then
  if [[ "$SSH_USER" == "root" ]]; then
    REMOTE_SUDO=""
  else
    REMOTE_SUDO="sudo"
  fi
fi

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  cat <<'EOF'
usage: scripts/deploy.sh <action>

Actions: files | init | deploy | restart | status | logs | stop | backup |
         rollback | validate

Environment:
  SSH_HOST        required; hostname or IP (host key verification stays on)
  SSH_PORT        default 22
  SSH_USER        default root
  SSH_IDENTITY    optional private key path
  REMOTE_DIR      default /opt/braindrop
  OS              alpine | ubuntu (default alpine)
  ARCH            amd64 | arm64 (default amd64)
  SERVICE         default braindrop
  SERVICE_USER    default braindrop
  APP_PORT        default 5150
  KEEP_RELEASES   default 3
  REMOTE_SUDO     default sudo for non-root, empty for root
  SKIP_BUILD=1    reuse an existing archive in dist/
  AUTO_ROLLBACK=1 restore the previous release when a health check fails
  GENERATE_SECRETS=1 (init only) generate JWT and encryption keys

Examples:
  SSH_HOST=server.example.com SSH_USER=root OS=alpine ./scripts/deploy.sh init
  SSH_HOST=server.example.com SSH_USER=deploy OS=ubuntu ARCH=arm64 ./scripts/deploy.sh deploy
EOF
}

validate_value() {
  local name=$1 value=$2 pattern=$3
  [[ -n "$value" ]] || die "$name must not be empty"
  [[ "$value" =~ $pattern ]] || die "$name contains unsupported characters: $value"
}

validate() {
  validate_value SSH_HOST "$SSH_HOST" '^[A-Za-z0-9._:-]+$'
  validate_value SSH_USER "$SSH_USER" '^[A-Za-z0-9._-]+$'
  validate_value SSH_PORT "$SSH_PORT" '^[0-9]+$'
  validate_value REMOTE_DIR "$REMOTE_DIR" '^/[A-Za-z0-9._/-]+$'
  validate_value SERVICE "$SERVICE" '^[A-Za-z0-9._-]+$'
  validate_value SERVICE_USER "$SERVICE_USER" '^[A-Za-z0-9._-]+$'
  validate_value APP_PORT "$APP_PORT" '^[0-9]+$'
  validate_value KEEP_RELEASES "$KEEP_RELEASES" '^[0-9]+$'
  validate_value RELEASE_ID "$RELEASE_ID" '^[A-Za-z0-9][A-Za-z0-9._-]*$'
  [[ "$RELEASE_ID" != *".."* ]] || die "RELEASE_ID must not contain '..'"
  if [[ -n "$ROLLBACK_RELEASE" ]]; then
    validate_value ROLLBACK_RELEASE "$ROLLBACK_RELEASE" '^[A-Za-z0-9][A-Za-z0-9._-]*$'
    [[ "$ROLLBACK_RELEASE" != *".."* ]] || die "ROLLBACK_RELEASE must not contain '..'"
  fi
  validate_value LINES "$LINES" '^[0-9]+$'
  [[ "$OS" == "alpine" || "$OS" == "ubuntu" ]] || die "OS must be alpine or ubuntu"
  [[ "$ARCH" == "amd64" || "$ARCH" == "arm64" ]] || die "ARCH must be amd64 or arm64"
  [[ "$SSH_PORT" -ge 1 && "$SSH_PORT" -le 65535 ]] || die "SSH_PORT is out of range"
  [[ "$APP_PORT" -ge 1 && "$APP_PORT" -le 65535 ]] || die "APP_PORT is out of range"
  [[ "$KEEP_RELEASES" -ge 1 ]] || die "KEEP_RELEASES must be at least 1"
  if [[ -n "$REMOTE_SUDO" && ! "$REMOTE_SUDO" =~ ^[A-Za-z0-9._\ -]+$ ]]; then
    die "REMOTE_SUDO contains unsupported characters"
  fi
}

target_for() {
  case "$OS:$ARCH" in
    alpine:amd64) echo "x86_64-unknown-linux-musl" ;;
    alpine:arm64) echo "aarch64-unknown-linux-musl" ;;
    ubuntu:amd64) echo "x86_64-unknown-linux-gnu" ;;
    ubuntu:arm64) echo "aarch64-unknown-linux-gnu" ;;
    *) die "unsupported OS/ARCH combination: $OS/$ARCH" ;;
  esac
}

platform_for() {
  case "$OS:$ARCH" in
    alpine:amd64) echo "linux-amd64-musl" ;;
    alpine:arm64) echo "linux-arm64-musl" ;;
    ubuntu:amd64) echo "linux-amd64-glibc" ;;
    ubuntu:arm64) echo "linux-arm64-glibc" ;;
  esac
}

SSH_OPTS=(-p "$SSH_PORT" -o BatchMode=yes -o ConnectTimeout=15)
SCP_OPTS=(-P "$SSH_PORT" -o BatchMode=yes -o ConnectTimeout=15)
if [[ -n "$SSH_IDENTITY" ]]; then
  SSH_OPTS+=(-i "$SSH_IDENTITY")
  SCP_OPTS+=(-i "$SSH_IDENTITY")
fi

remote() {
  ssh "${SSH_OPTS[@]}" "$SSH_USER@$SSH_HOST" "$@"
}

remote_script() {
  ssh "${SSH_OPTS[@]}" "$SSH_USER@$SSH_HOST" \
    "REMOTE_DIR='$REMOTE_DIR' SERVICE='$SERVICE' SERVICE_USER='$SERVICE_USER' APP_PORT='$APP_PORT' KEEP_RELEASES='$KEEP_RELEASES' SUDO='$REMOTE_SUDO' RELEASE_ID='$RELEASE_ID' GENERATE_SECRETS='$GENERATE_SECRETS' ROLLBACK_RELEASE='$ROLLBACK_RELEASE' PREVIOUS_RELEASE='$PREVIOUS_RELEASE' OS='$OS' sh -s"
}

ensure_package() {
  local target platform
  target="$(target_for)"
  platform="$(platform_for)"
  ARCHIVE="$DIST/braindrop-$platform.tar.gz"

  if [[ "$SKIP_BUILD" != "1" ]]; then
    ./scripts/build-native.sh "$target"
    ./scripts/package.sh "$target"
  fi
  [[ -f "$ARCHIVE" ]] || die "release archive not found: $ARCHIVE (run without SKIP_BUILD=1)"
  echo "==> Using $ARCHIVE"
}

remote_dirs() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
as_root mkdir -p "$REMOTE_DIR/releases" "$REMOTE_DIR/shared/data" \
  "$REMOTE_DIR/shared/backups" "$REMOTE_DIR/shared/logs"
if id "$SERVICE_USER" >/dev/null 2>&1; then
  as_root chown -R "$SERVICE_USER:$SERVICE_USER" "$REMOTE_DIR/shared" "$REMOTE_DIR/releases"
fi
SCRIPT
}

upload_release() {
  local tmp="/tmp/braindrop-$RELEASE_ID.tar.gz"
  echo "==> Uploading release $RELEASE_ID"
  scp "${SCP_OPTS[@]}" "$ARCHIVE" "$SSH_USER@$SSH_HOST:$tmp"
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
tmp="/tmp/braindrop-$RELEASE_ID.tar.gz"
staging="$REMOTE_DIR/releases/.staging-$RELEASE_ID"
as_root mkdir -p "$REMOTE_DIR/releases"
as_root rm -rf "$staging"
as_root mkdir -p "$staging"
as_root tar -xzf "$tmp" -C "$staging" --strip-components=1
as_root rm -f "$tmp"
if id "$SERVICE_USER" >/dev/null 2>&1; then
  as_root chown -R "$SERVICE_USER:$SERVICE_USER" "$staging"
fi
as_root rm -rf "$REMOTE_DIR/releases/$RELEASE_ID"
as_root mv "$staging" "$REMOTE_DIR/releases/$RELEASE_ID"
echo "Release stored at $REMOTE_DIR/releases/$RELEASE_ID"
SCRIPT
}

provision_env() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
env_file="$REMOTE_DIR/shared/.env"
example="$REMOTE_DIR/releases/$RELEASE_ID/.env.example"
if [ -f "$env_file" ]; then
  echo "==> Preserving existing $env_file"
  exit 0
fi
[ -f "$example" ] || { echo "error: missing $example" >&2; exit 1; }
as_root cp "$example" "$env_file"
# Native installs keep data under shared/data instead of the Docker /data path.
as_root sed -i "s#/data/#$REMOTE_DIR/shared/data/#g" "$env_file"
if [ "$GENERATE_SECRETS" = "1" ]; then
  command -v openssl >/dev/null 2>&1 || { echo "error: openssl is required to generate secrets" >&2; exit 1; }
  for key in JWT_SECRET AI_ENCRYPTION_KEY NOTIFICATION_ENCRYPTION_KEY; do
    value="$(openssl rand -base64 32)"
    as_root sed -i "s#^$key=.*#$key=$value#" "$env_file"
  done
else
  echo "warning: GENERATE_SECRETS=0; edit $env_file before starting" >&2
fi
as_root chmod 600 "$env_file"
if id "$SERVICE_USER" >/dev/null 2>&1; then
  as_root chown "$SERVICE_USER:$SERVICE_USER" "$env_file"
fi
echo "==> Created $env_file"
echo "    Review HOST and SMTP settings before exposing the service."
SCRIPT
}

activate_release() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
as_root ln -sfn "$REMOTE_DIR/releases/$RELEASE_ID" "$REMOTE_DIR/current"
echo "==> Activated $REMOTE_DIR/releases/$RELEASE_ID"
SCRIPT
}

previous_release() {
  remote "readlink -f '$REMOTE_DIR/current' 2>/dev/null || true"
}

require_env() {
  remote "test -f '$REMOTE_DIR/shared/.env'" || die "$REMOTE_DIR/shared/.env is missing; run the init action first"
}

run_migrations() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
env_file="$REMOTE_DIR/shared/.env"
release="$REMOTE_DIR/releases/$RELEASE_ID"
command="set -a; . '$env_file'; set +a; cd '$release'; ./braindrop db migrate"
if [ "$(id -u)" = "0" ]; then
  if [ -n "$SUDO" ]; then
    $SUDO -u "$SERVICE_USER" sh -c "$command"
  else
    su -s /bin/sh -c "$command" "$SERVICE_USER"
  fi
else
  sh -c "$command"
fi
echo "==> Migrations applied"
SCRIPT
}

restart_service() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
if command -v systemctl >/dev/null 2>&1; then
  as_root systemctl restart "$SERVICE"
elif command -v rc-service >/dev/null 2>&1; then
  as_root rc-service "$SERVICE" restart
else
  echo "error: neither systemctl nor rc-service is available" >&2
  exit 1
fi
echo "==> Service $SERVICE restarted"
SCRIPT
}

stop_service() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
if command -v systemctl >/dev/null 2>&1; then
  as_root systemctl stop "$SERVICE" || true
elif command -v rc-service >/dev/null 2>&1; then
  as_root rc-service "$SERVICE" stop || true
fi
echo "==> Service $SERVICE stopped"
SCRIPT
}

health_check() {
  remote_script <<'SCRIPT'
set -eu
i=0
while [ "$i" -lt 45 ]; do
  if curl -fsS "http://127.0.0.1:$APP_PORT/_health" >/dev/null 2>&1; then
    echo "==> Health check passed"
    exit 0
  fi
  i=$((i + 1))
  sleep 2
done
echo "error: health check failed on port $APP_PORT" >&2
exit 1
SCRIPT
}

prune_releases() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
current="$(readlink -f "$REMOTE_DIR/current" 2>/dev/null || true)"
i=0
for dir in $(ls -1dt "$REMOTE_DIR/releases"/* 2>/dev/null); do
  i=$((i + 1))
  [ "$i" -le "$KEEP_RELEASES" ] && continue
  [ "$dir" = "$current" ] && continue
  as_root rm -rf "$dir"
done
SCRIPT
}

install_service() {
  local init_script="$REMOTE_DIR/releases/$RELEASE_ID/scripts/deploy-init-$OS.sh"
  remote_script <<SCRIPT
set -eu
as_root() { if [ -n "\$SUDO" ]; then \$SUDO "\$@"; else "\$@"; fi; }
test -f "$init_script" || { echo "error: missing $init_script" >&2; exit 1; }
# Pass the settings explicitly: sudo resets the environment by default, so
# relying on inherited variables would silently fall back to the defaults.
as_root env REMOTE_DIR="$REMOTE_DIR" SERVICE="$SERVICE" SERVICE_USER="$SERVICE_USER" \
  APP_PORT="$APP_PORT" sh "$init_script"
SCRIPT
}

action_files() {
  ensure_package
  remote_dirs
  upload_release
}

action_init() {
  ensure_package
  remote_dirs
  upload_release
  provision_env
  activate_release
  install_service
  health_check
  echo "==> Initial deployment complete. Review $REMOTE_DIR/shared/.env, then restart the service."
}

action_deploy() {
  ensure_package
  require_env
  remote_dirs
  PREVIOUS_RELEASE="$(previous_release)"
  upload_release
  run_migrations
  activate_release
  if ! restart_service; then
    die "service restart failed"
  fi
  if ! health_check; then
    if [[ "$AUTO_ROLLBACK" == "1" && -n "$PREVIOUS_RELEASE" ]]; then
      echo "error: health check failed; rolling back to $PREVIOUS_RELEASE" >&2
      remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
if [ -n "$PREVIOUS_RELEASE" ]; then
  as_root ln -sfn "$PREVIOUS_RELEASE" "$REMOTE_DIR/current"
fi
SCRIPT
      restart_service || true
      health_check || true
      die "deployment failed and was rolled back; the database was not rolled back"
    fi
    die "deployment failed; the service did not become healthy"
  fi
  prune_releases
  echo "==> Deployment $RELEASE_ID complete"
}

action_restart() {
  require_env
  restart_service
  health_check
}

action_status() {
  remote_script <<'SCRIPT'
set -eu
echo "current: $(readlink -f "$REMOTE_DIR/current" 2>/dev/null || echo none)"
if command -v systemctl >/dev/null 2>&1; then
  systemctl is-active "$SERVICE" || true
elif command -v rc-service >/dev/null 2>&1; then
  rc-service "$SERVICE" status || true
fi
if curl -fsS "http://127.0.0.1:$APP_PORT/_health" >/dev/null 2>&1; then
  echo "health: ok"
else
  echo "health: failed"
fi
SCRIPT
}

action_logs() {
  remote_script <<SCRIPT
set -eu
as_root() { if [ -n "\$SUDO" ]; then \$SUDO "\$@"; else "\$@"; fi; }
if command -v journalctl >/dev/null 2>&1; then
  as_root journalctl -u "$SERVICE" -n "$LINES" -f
else
  as_root tail -n "$LINES" -f "$REMOTE_DIR/shared/logs/braindrop.log"
fi
SCRIPT
}

action_stop() {
  stop_service
}

action_backup() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
data="$REMOTE_DIR/shared/data"
command="cd '$REMOTE_DIR/current'; DATA_DIR='$data' BACKUP_DIR='$data/../backups' DB_PATH='$data/braindrop.sqlite' QUEUE_PATH='$data/braindrop_queue.sqlite' ./scripts/backup.sh"
if [ "$(id -u)" = "0" ]; then
  if [ -n "$SUDO" ]; then
    $SUDO -u "$SERVICE_USER" sh -c "$command"
  else
    su -s /bin/sh -c "$command" "$SERVICE_USER"
  fi
else
  sh -c "$command"
fi
SCRIPT
}

action_rollback() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
current="$(readlink -f "$REMOTE_DIR/current" 2>/dev/null || true)"
target=""
if [ -n "${ROLLBACK_RELEASE:-}" ]; then
  target="$REMOTE_DIR/releases/$ROLLBACK_RELEASE"
else
  for dir in $(ls -1dt "$REMOTE_DIR/releases"/* 2>/dev/null); do
    if [ "$dir" != "$current" ]; then
      target="$dir"
      break
    fi
  done
fi
[ -n "$target" ] && [ -d "$target" ] || { echo "error: no previous release available" >&2; exit 1; }
as_root ln -sfn "$target" "$REMOTE_DIR/current"
echo "==> Rolled back to $target"
SCRIPT
  restart_service
  health_check
  echo "warning: the database schema was not rolled back; restore a matching backup if needed."
}

if [[ "$ACTION" == "help" || "$ACTION" == "--help" || "$ACTION" == "-h" ]]; then
  usage
  exit 0
fi

if [[ "$ACTION" == "validate" ]]; then
  validate
  echo "configuration is valid"
  echo "target: $(target_for)"
  echo "package: $DIST/braindrop-$(platform_for).tar.gz"
  exit 0
fi

validate

case "$ACTION" in
  files) action_files ;;
  init) action_init ;;
  deploy) action_deploy ;;
  restart) action_restart ;;
  status) action_status ;;
  logs) action_logs ;;
  stop) action_stop ;;
  backup) action_backup ;;
  rollback) action_rollback ;;
  *)
    usage
    die "unknown action: $ACTION"
    ;;
esac
