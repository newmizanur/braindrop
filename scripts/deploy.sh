#!/usr/bin/env bash
# BrainDrop SSH deployment.
#
# Called by the Makefile deploy-* targets. Uses plain SSH and SCP; the remote
# host needs no Docker, Rust, Node.js or rsync.
#
# Flat runtime layout: application/config/frontend at REMOTE_DIR; persistent
# state in .env, data/, logs/ and backup/. One previous code archive provides
# rollback without timestamped release directories.
#
# A routine deploy never uploads the local .env. The first deploy (`init`)
# creates a minimal starter .env, generates secrets and leaves it for review.
# Later deploys preserve it. Public distribution archives are not uploaded.
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
RELEASE_ID="${RELEASE_ID:-$(date -u +%Y%m%dT%H%M%SZ)-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)}"
UPLOAD_PATH="${UPLOAD_PATH:-}"
RUNTIME_ARCHIVE=""
KEEP_STAGING="${KEEP_STAGING:-0}"
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

cleanup_local() {
  [[ -z "$RUNTIME_ARCHIVE" ]] || rm -f "$RUNTIME_ARCHIVE"
}
trap cleanup_local EXIT

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
  validate_value RELEASE_ID "$RELEASE_ID" '^[A-Za-z0-9][A-Za-z0-9._-]*$'
  [[ "$RELEASE_ID" != *".."* ]] || die "RELEASE_ID must not contain '..'"
  validate_value LINES "$LINES" '^[0-9]+$'
  [[ "$OS" == "alpine" || "$OS" == "ubuntu" ]] || die "OS must be alpine or ubuntu"
  [[ "$ARCH" == "amd64" || "$ARCH" == "arm64" ]] || die "ARCH must be amd64 or arm64"
  [[ "$SSH_PORT" -ge 1 && "$SSH_PORT" -le 65535 ]] || die "SSH_PORT is out of range"
  [[ "$APP_PORT" -ge 1 && "$APP_PORT" -le 65535 ]] || die "APP_PORT is out of range"
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
    "REMOTE_DIR='$REMOTE_DIR' SERVICE='$SERVICE' SERVICE_USER='$SERVICE_USER' APP_PORT='$APP_PORT' SUDO='$REMOTE_SUDO' RELEASE_ID='$RELEASE_ID' GENERATE_SECRETS='$GENERATE_SECRETS' UPLOAD_PATH='$UPLOAD_PATH' KEEP_STAGING='$KEEP_STAGING' OS='$OS' sh -s"
}

ensure_package() {
  local target platform distribution
  target="$(target_for)"
  platform="$(platform_for)"
  ARCHIVE="$DIST/braindrop-$platform.tar.gz"

  if [[ "$SKIP_BUILD" != "1" ]]; then
    ./scripts/build-native.sh "$target"
    ./scripts/package.sh "$target"
  fi
  distribution="$DIST/braindrop-$platform"
  [[ -f "$ARCHIVE" && -d "$distribution" ]] || die "distribution package is missing; build/package $target first"
  RUNTIME_ARCHIVE="$(mktemp "${TMPDIR:-/tmp}/braindrop-runtime.XXXXXX")"
  ./scripts/create-runtime-archive.sh "$distribution" "$RUNTIME_ARCHIVE"
  echo "==> Using runtime-only bundle derived from $ARCHIVE"
}

remote_dirs() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
as_root mkdir -p "$REMOTE_DIR"
as_root chown root:root "$REMOTE_DIR"
as_root chmod 755 "$REMOTE_DIR"
SCRIPT
}

upload_release() {
  local upload_path
  upload_path="$(remote 'umask 077; mktemp "${HOME}/.braindrop-upload.XXXXXX"')"
  [[ "$upload_path" == /* && "$upload_path" != *"'"* ]] || die "unexpected remote upload path"
  echo "==> Uploading runtime bundle $RELEASE_ID"
  scp "${SCP_OPTS[@]}" "$RUNTIME_ARCHIVE" "$SSH_USER@$SSH_HOST:$upload_path"
  UPLOAD_PATH="$upload_path" remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
tmp="$UPLOAD_PATH"
staging="$REMOTE_DIR/.staging-$RELEASE_ID"
[ -f "$tmp" ] || { echo "error: uploaded archive is missing" >&2; exit 1; }
cleanup() {
  as_root rm -f "$tmp" 2>/dev/null || true
  [ "$KEEP_STAGING" = "1" ] || as_root rm -rf "$staging" 2>/dev/null || true
}
trap cleanup EXIT
as_root rm -rf "$staging"
as_root mkdir -p "$staging"
as_root chmod 700 "$staging"
as_root tar -xzf "$tmp" -C "$staging"
as_root sh "$staging/scripts/prepare-release.sh" "$staging"
echo "Runtime staged at $staging"
SCRIPT
}

provision_env() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
env_file="$REMOTE_DIR/.env"
if [ -f "$env_file" ]; then
  echo "==> Preserving existing $env_file"
  exit 0
fi
if [ -f "$REMOTE_DIR/shared/.env" ]; then
  echo "error: legacy environment must be migrated before provisioning" >&2
  exit 1
fi
[ "$GENERATE_SECRETS" = "1" ] || { echo "error: secret generation is required for init" >&2; exit 1; }
command -v openssl >/dev/null 2>&1 || { echo "error: openssl is required to generate secrets" >&2; exit 1; }
jwt_secret="$(openssl rand -base64 32)"
ai_key="$(openssl rand -base64 32)"
as_root sh -c 'umask 077; cat > "$1"' sh "$env_file" <<EOF
LOCO_ENV=production
PORT=$APP_PORT
BINDING=0.0.0.0
HOST=http://localhost:$APP_PORT
LOG_LEVEL=info
DATABASE_URL=sqlite://$REMOTE_DIR/data/braindrop.sqlite?mode=rwc
QUEUE_URL=sqlite://$REMOTE_DIR/data/braindrop_queue.sqlite?mode=rwc
DB_AUTO_MIGRATE=true
DB_MAX_CONNECTIONS=4
JWT_SECRET=$jwt_secret
ALLOW_REGISTRATION=false
REQUIRE_EMAIL_VERIFICATION=false
REMOTE_IP_SOURCE=ConnectInfo
AI_ENABLED=false
AI_ENCRYPTION_KEY=$ai_key
NOTIFICATION_ENCRYPTION_KEY=
EOF
as_root chmod 600 "$env_file"
if id "$SERVICE_USER" >/dev/null 2>&1; then
  as_root chown "$SERVICE_USER:$SERVICE_USER" "$env_file"
fi
echo "==> Created protected $env_file with fresh persistent secrets"
SCRIPT
}

migrate_native_layout() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
staging="$REMOTE_DIR/.staging-$RELEASE_ID"
as_root sh "$staging/scripts/migrate-native-layout.sh" "$REMOTE_DIR" "$SERVICE_USER"
SCRIPT
}

require_env() {
  remote "test -f '$REMOTE_DIR/.env' -o -f '$REMOTE_DIR/shared/.env'" || die "$REMOTE_DIR/.env is missing; run the init action first"
}

run_migrations() {
  remote_script <<'SCRIPT'
set -eu
command="set -a; . '$REMOTE_DIR/.env'; set +a; cd '$REMOTE_DIR/.staging-$RELEASE_ID'; ./braindrop db migrate"
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

snapshot_previous_runtime() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
source="$REMOTE_DIR"
if [ -L "$REMOTE_DIR/current" ]; then source="$(readlink -f "$REMOTE_DIR/current")"; fi
if [ ! -f "$source/braindrop" ]; then
  echo "==> No previous runtime to snapshot"
  exit 0
fi
as_root mkdir -p "$REMOTE_DIR/backup"
tmp="$REMOTE_DIR/backup/.previous-runtime-$RELEASE_ID.tar.gz"
as_root tar -czf "$tmp" --exclude='._*' --exclude='.DS_Store' --exclude='__MACOSX' \
  -C "$source" braindrop config/production.yaml frontend/dist scripts/backup.sh scripts/restore.sh
as_root chmod 600 "$tmp"
as_root mv -f "$tmp" "$REMOTE_DIR/backup/previous-runtime.tar.gz"
echo "==> Previous runtime saved for rollback"
SCRIPT
}

backup_before_upgrade() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
data="$REMOTE_DIR/data"
if [ ! -f "$data/braindrop.sqlite" ]; then
  echo "==> No application database exists yet; skipping pre-upgrade backup"
  exit 0
fi
source="$REMOTE_DIR"
if [ -L "$REMOTE_DIR/current" ]; then source="$(readlink -f "$REMOTE_DIR/current")"; fi
script="$source/scripts/backup.sh"
[ -x "$script" ] || { echo "error: previous runtime has no backup script" >&2; exit 1; }
command="DATA_DIR='$data' DB_PATH='$data/braindrop.sqlite' QUEUE_PATH='$data/braindrop_queue.sqlite' BACKUP_DIR='$REMOTE_DIR/backup' '$script'"
if [ "$(id -u)" = "0" ]; then
  if [ -n "$SUDO" ]; then $SUDO -u "$SERVICE_USER" sh -c "$command"; else su -s /bin/sh -c "$command" "$SERVICE_USER"; fi
else
  sh -c "$command"
fi
SCRIPT
}

activate_runtime() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
staging="$REMOTE_DIR/.staging-$RELEASE_ID"
for path in braindrop config frontend scripts; do
  as_root rm -rf "$REMOTE_DIR/$path"
done
as_root mkdir -p "$REMOTE_DIR/scripts"
as_root cp -R "$staging/braindrop" "$REMOTE_DIR/"
as_root cp -R "$staging/config" "$REMOTE_DIR/"
as_root cp -R "$staging/frontend" "$REMOTE_DIR/"
as_root cp -R "$staging/scripts/backup.sh" "$staging/scripts/restore.sh" "$REMOTE_DIR/scripts/"
as_root chown -R root:root "$REMOTE_DIR/braindrop" "$REMOTE_DIR/config" "$REMOTE_DIR/frontend" "$REMOTE_DIR/scripts"
as_root chmod 755 "$REMOTE_DIR/braindrop"
as_root find "$REMOTE_DIR/config" "$REMOTE_DIR/frontend" "$REMOTE_DIR/scripts" -type d -exec chmod 755 {} \;
as_root find "$REMOTE_DIR/config" "$REMOTE_DIR/frontend" "$REMOTE_DIR/scripts" -type f -exec chmod 644 {} \;
for script in backup.sh restore.sh; do as_root chmod 755 "$REMOTE_DIR/scripts/$script"; done
echo "==> Installed root-owned flat runtime"
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

install_service() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
init_script="$REMOTE_DIR/.staging-$RELEASE_ID/scripts/deploy-init-$OS.sh"
test -f "$init_script" || { echo "error: missing service installer in staged runtime" >&2; exit 1; }
as_root env REMOTE_DIR="$REMOTE_DIR" SERVICE="$SERVICE" SERVICE_USER="$SERVICE_USER" \
  APP_PORT="$APP_PORT" sh "$init_script"
SCRIPT
}

cleanup_legacy_layout() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
# The old release tree is code-only. Refuse to prune it if a prior/manual
# layout put environment, database, or runtime state files beneath it.
if [ -d "$REMOTE_DIR/releases" ]; then
  if find "$REMOTE_DIR/releases" \( -name '.env' -o -name '*.sqlite' -o -name '*.sqlite-*' \) -print | grep -q .; then
    echo "error: refusing to remove legacy releases containing secrets or SQLite state" >&2
    exit 1
  fi
  if find "$REMOTE_DIR/releases" -type d \( -name data -o -name backup -o -name backups -o -name logs \) -print | grep -q .; then
    echo "error: refusing to remove legacy releases containing runtime state directories" >&2
    exit 1
  fi
fi
as_root rm -f "$REMOTE_DIR/shared/.env" "$REMOTE_DIR/shared/data" \
  "$REMOTE_DIR/shared/backups" "$REMOTE_DIR/shared/logs"
as_root rmdir "$REMOTE_DIR/shared" 2>/dev/null || true
as_root rm -f "$REMOTE_DIR/current"
as_root rm -rf "$REMOTE_DIR/releases"
as_root rm -rf "$REMOTE_DIR/.staging-$RELEASE_ID"
SCRIPT
}

restore_previous_runtime() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
archive="$REMOTE_DIR/backup/previous-runtime.tar.gz"
[ -s "$archive" ] || { echo "error: previous runtime archive is missing" >&2; exit 1; }
staging="$REMOTE_DIR/.rollback-$RELEASE_ID"
as_root rm -rf "$staging"
as_root mkdir -p "$staging"
as_root tar -xzf "$archive" -C "$staging"
as_root chown -R root:root "$staging"
as_root find "$staging" -type d -exec chmod 755 {} \;
as_root find "$staging" -type f -exec chmod 644 {} \;
as_root chmod 755 "$staging/braindrop" "$staging/scripts/backup.sh" "$staging/scripts/restore.sh"
for path in braindrop config frontend scripts; do as_root rm -rf "$REMOTE_DIR/$path"; done
for path in braindrop config frontend scripts; do as_root cp -R "$staging/$path" "$REMOTE_DIR/$path"; done
as_root chown -R root:root "$REMOTE_DIR/braindrop" "$REMOTE_DIR/config" "$REMOTE_DIR/frontend" "$REMOTE_DIR/scripts"
as_root rm -rf "$staging"
echo "==> Previous runtime restored"
SCRIPT
}

restore_previous_runtime_after_failure() {
  stop_service || true
  restore_previous_runtime || return 1
  restart_service || return 1
  health_check
}

action_files() {
  ensure_package
  remote_dirs
  KEEP_STAGING=1
  upload_release
}

action_init() {
  ensure_package
  remote_dirs
  KEEP_STAGING=1
  upload_release
  migrate_native_layout
  provision_env
  activate_runtime
  install_service
  if ! health_check; then
    stop_service || true
    die "initial deployment health check failed"
  fi
  cleanup_legacy_layout
  echo "==> Initial flat runtime installed; review $REMOTE_DIR/.env before public exposure"
}

action_deploy() {
  ensure_package
  require_env
  remote_dirs
  KEEP_STAGING=1
  upload_release
  stop_service
  if ! migrate_native_layout; then restart_service || true; die "could not migrate legacy runtime paths"; fi
  if ! snapshot_previous_runtime; then restart_service || true; die "could not save previous runtime for rollback"; fi
  if ! backup_before_upgrade; then restart_service || true; die "pre-upgrade database backup failed"; fi
  if ! run_migrations; then restart_service || true; die "database migration failed; previous runtime remains installed"; fi
  if ! activate_runtime; then
    restore_previous_runtime_after_failure || true
    die "runtime activation failed; rollback was attempted"
  fi
  if ! install_service || ! health_check; then
    if [[ "$AUTO_ROLLBACK" == "1" ]]; then
      echo "error: deployment health failed; restoring previous runtime" >&2
      if restore_previous_runtime_after_failure; then
        cleanup_legacy_layout || true
        die "deployment failed; previous runtime restored; database remains at migrated schema"
      fi
    fi
    die "deployment failed and service did not become healthy; inspect service and rollback archive"
  fi
  cleanup_legacy_layout
  echo "==> Flat deployment $RELEASE_ID complete"
}

action_restart() {
  require_env
  restart_service
  health_check
}

action_status() {
  remote_script <<'SCRIPT'
set -eu
if [ -f "$REMOTE_DIR/braindrop" ]; then echo "layout: flat"; else echo "layout: not-installed"; fi
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
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
if command -v journalctl >/dev/null 2>&1; then
  as_root journalctl -u "$SERVICE" -n "$LINES" -f
else
  as_root tail -n "$LINES" "$REMOTE_DIR/logs/braindrop.log" "$REMOTE_DIR/logs/braindrop.err.log"
fi
SCRIPT
}

action_stop() {
  stop_service
}

action_backup() {
  remote_script <<'SCRIPT'
set -eu
if [ ! -f "$REMOTE_DIR/data/braindrop.sqlite" ]; then
  echo "error: application database does not exist yet" >&2
  exit 1
fi
command="DATA_DIR='$REMOTE_DIR/data' DB_PATH='$REMOTE_DIR/data/braindrop.sqlite' QUEUE_PATH='$REMOTE_DIR/data/braindrop_queue.sqlite' BACKUP_DIR='$REMOTE_DIR/backup' '$REMOTE_DIR/scripts/backup.sh'"
if [ "$(id -u)" = "0" ]; then
  if [ -n "$SUDO" ]; then $SUDO -u "$SERVICE_USER" sh -c "$command"; else su -s /bin/sh -c "$command" "$SERVICE_USER"; fi
else
  sh -c "$command"
fi
SCRIPT
}

action_rollback() {
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
archive="$REMOTE_DIR/backup/previous-runtime.tar.gz"
[ -s "$archive" ] || { echo "error: no previous runtime is available" >&2; exit 1; }
SCRIPT
  stop_service
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
archive="$REMOTE_DIR/backup/previous-runtime.tar.gz"
target="$REMOTE_DIR/backup/.rollback-target-$RELEASE_ID.tar.gz"
as_root mv "$archive" "$target"
SCRIPT
  if ! snapshot_previous_runtime; then
    remote "mv '$REMOTE_DIR/backup/.rollback-target-$RELEASE_ID.tar.gz' '$REMOTE_DIR/backup/previous-runtime.tar.gz'"
    restart_service || true
    die "could not preserve the active runtime before rollback"
  fi
  remote_script <<'SCRIPT'
set -eu
as_root() { if [ -n "$SUDO" ]; then $SUDO "$@"; else "$@"; fi; }
target="$REMOTE_DIR/backup/.rollback-target-$RELEASE_ID.tar.gz"
staging="$REMOTE_DIR/.rollback-$RELEASE_ID"
as_root mkdir -p "$staging"
as_root tar -xzf "$target" -C "$staging"
as_root chown -R root:root "$staging"
as_root find "$staging" -type d -exec chmod 755 {} \;
as_root find "$staging" -type f -exec chmod 644 {} \;
as_root chmod 755 "$staging/braindrop" "$staging/scripts/backup.sh" "$staging/scripts/restore.sh"
for path in braindrop config frontend scripts; do as_root rm -rf "$REMOTE_DIR/$path"; done
for path in braindrop config frontend scripts; do as_root cp -R "$staging/$path" "$REMOTE_DIR/$path"; done
as_root chown -R root:root "$REMOTE_DIR/braindrop" "$REMOTE_DIR/config" "$REMOTE_DIR/frontend" "$REMOTE_DIR/scripts"
as_root rm -rf "$staging" "$target"
SCRIPT
  if ! restart_service || ! health_check; then
    restore_previous_runtime_after_failure || die "rollback health failed and the prior runtime could not be restored"
    die "rollback health failed; the active runtime was restored"
  fi
  echo "warning: database schema was not rolled back; restore the matching backup if required."
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
