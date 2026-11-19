#!/usr/bin/env bash
# Restore a BrainDrop SQLite backup.
#
# Usage:
#   scripts/restore.sh <archive.tar.gz> [--yes]
#
# The current databases are moved aside, never deleted. The service is stopped
# before files are replaced and restarted afterwards when a service manager is
# available.
#
# Environment:
#   DATA_DIR        directory holding the databases (default: /data)
#   DB_PATH         application database (default: $DATA_DIR/braindrop.sqlite)
#   QUEUE_PATH      Loco queue database (default: $DATA_DIR/braindrop_queue.sqlite)
#   SERVICE         systemd/OpenRC service name (default: braindrop)
#   SERVICE_USER    owner for restored files (default: braindrop)
#   SUDO            privilege command (default: sudo; set empty when root)
#   NO_SERVICE=1    skip service stop/start (used by the backup test)
#   APP_BIN         binary to run migrations with (default: current/braindrop)
#   RUN_MIGRATIONS=1 run `APP_BIN db migrate` after restore
#   ENV_FILE        environment file to source before migrations
#   HEALTH_URL      URL checked after restart
set -euo pipefail

ARCHIVE="${1:-}"
ASSUME_YES=0
if [[ "${2:-}" == "--yes" || "${1:-}" == "--yes" ]]; then
  ASSUME_YES=1
fi
if [[ "$ARCHIVE" == "--yes" ]]; then
  ARCHIVE="${2:-}"
fi

die() {
  echo "error: $*" >&2
  exit 1
}

usage() {
  cat >&2 <<'EOF'
usage: scripts/restore.sh <archive.tar.gz> [--yes]

Restores the application and queue databases from a backup made by
scripts/backup.sh. Requires explicit confirmation unless --yes is given.
EOF
  exit 2
}

[[ -n "$ARCHIVE" ]] || usage
[[ -f "$ARCHIVE" ]] || die "backup archive not found: $ARCHIVE"
command -v sqlite3 >/dev/null 2>&1 || die "sqlite3 is required"
command -v tar >/dev/null 2>&1 || die "tar is required"

DATA_DIR="${DATA_DIR:-/data}"
DB_PATH="${DB_PATH:-$DATA_DIR/braindrop.sqlite}"
QUEUE_PATH="${QUEUE_PATH:-$DATA_DIR/braindrop_queue.sqlite}"
SERVICE="${SERVICE:-braindrop}"
SERVICE_USER="${SERVICE_USER:-braindrop}"
SUDO="${SUDO-sudo}"
NO_SERVICE="${NO_SERVICE:-0}"
RUN_MIGRATIONS="${RUN_MIGRATIONS:-0}"
ENV_FILE="${ENV_FILE:-}"
HEALTH_URL="${HEALTH_URL:-}"
APP_BIN="${APP_BIN:-}"

SUDO_CMD=()
if [[ -n "$SUDO" ]]; then
  read -r -a SUDO_CMD <<<"$SUDO"
fi
run_root() {
  # The +"..." form keeps macOS bash 3.2 happy when SUDO is empty under
  # `set -u` (a bare "${arr[@]}" of an empty array is an error there).
  "${SUDO_CMD[@]+"${SUDO_CMD[@]}"}" "$@"
}

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "sha256sum or shasum is required"
  fi
}

if [[ -f "$ARCHIVE.sha256" ]]; then
  EXPECTED="$(tr -d '[:space:]' <"$ARCHIVE.sha256")"
  ACTUAL="$(sha256_file "$ARCHIVE")"
  [[ "$EXPECTED" == "$ACTUAL" ]] || die "checksum mismatch for $ARCHIVE"
  echo "==> Checksum verified"
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/braindrop-restore.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
tar -xzf "$ARCHIVE" -C "$WORK"
[[ -f "$WORK/manifest.txt" ]] || die "archive has no manifest.txt"

APP_NAME="$(sed -n 's/^application_db=//p' "$WORK/manifest.txt")"
QUEUE_NAME="$(sed -n 's/^queue_db=//p' "$WORK/manifest.txt")"
[[ -n "$APP_NAME" ]] || die "archive manifest does not name an application database"
[[ -f "$WORK/$APP_NAME" ]] || die "archive is missing $APP_NAME"

echo "==> Verifying restored database integrity"
for name in "$APP_NAME" ${QUEUE_NAME:+"$QUEUE_NAME"}; do
  [[ -n "$name" ]] || continue
  [[ -f "$WORK/$name" ]] || die "archive is missing $name"
  integrity="$(sqlite3 "$WORK/$name" 'PRAGMA integrity_check;')"
  [[ "$integrity" == "ok" ]] || die "integrity check failed for $name: $integrity"
done

if [[ "$ASSUME_YES" != "1" ]]; then
  echo
  echo "This replaces:"
  echo "  $DB_PATH"
  [[ -n "$QUEUE_NAME" ]] && echo "  $QUEUE_PATH"
  echo "Current files are moved aside first, not deleted."
  read -r -p "Type RESTORE to continue: " answer
  [[ "$answer" == "RESTORE" ]] || die "restore cancelled"
fi

STOPPED=0
start_service() {
  [[ "$NO_SERVICE" == "1" || "$STOPPED" == "0" ]] && return 0
  if command -v systemctl >/dev/null 2>&1; then
    run_root systemctl start "$SERVICE"
  elif command -v rc-service >/dev/null 2>&1; then
    run_root rc-service "$SERVICE" start
  fi
}

if [[ "$NO_SERVICE" != "1" ]]; then
  echo "==> Stopping $SERVICE"
  if command -v systemctl >/dev/null 2>&1; then
    run_root systemctl stop "$SERVICE" && STOPPED=1
  elif command -v rc-service >/dev/null 2>&1; then
    run_root rc-service "$SERVICE" stop && STOPPED=1
  else
    die "no supported service manager found; stop the app manually and set NO_SERVICE=1"
  fi
fi

mkdir -p "$DATA_DIR"
PREVIOUS="$DATA_DIR/restore-previous-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$PREVIOUS"
if [[ -f "$DB_PATH" ]]; then
  mv "$DB_PATH" "$PREVIOUS/"
fi
if [[ -f "$DB_PATH-wal" ]]; then mv "$DB_PATH-wal" "$PREVIOUS/"; fi
if [[ -f "$DB_PATH-shm" ]]; then mv "$DB_PATH-shm" "$PREVIOUS/"; fi
if [[ -f "$QUEUE_PATH" ]]; then
  mv "$QUEUE_PATH" "$PREVIOUS/"
fi
if [[ -f "$QUEUE_PATH-wal" ]]; then mv "$QUEUE_PATH-wal" "$PREVIOUS/"; fi
if [[ -f "$QUEUE_PATH-shm" ]]; then mv "$QUEUE_PATH-shm" "$PREVIOUS/"; fi

echo "==> Restoring databases"
cp "$WORK/$APP_NAME" "$DB_PATH"
if [[ -n "$QUEUE_NAME" ]]; then
  cp "$WORK/$QUEUE_NAME" "$QUEUE_PATH"
fi
chmod 600 "$DB_PATH"
[[ -n "$QUEUE_NAME" ]] && chmod 600 "$QUEUE_PATH"

if [[ "$NO_SERVICE" != "1" ]] && id "$SERVICE_USER" >/dev/null 2>&1; then
  run_root chown "$SERVICE_USER:$SERVICE_USER" "$DB_PATH" "$PREVIOUS" || true
  [[ -n "$QUEUE_NAME" ]] && run_root chown "$SERVICE_USER:$SERVICE_USER" "$QUEUE_PATH" || true
fi

if [[ "$RUN_MIGRATIONS" == "1" ]]; then
  [[ -n "$APP_BIN" && -x "$APP_BIN" ]] || die "RUN_MIGRATIONS=1 requires an executable APP_BIN"
  echo "==> Running migrations"
  (
    set -a
    [[ -n "$ENV_FILE" && -f "$ENV_FILE" ]] && . "$ENV_FILE"
    set +a
    "$APP_BIN" db migrate
  )
fi

start_service

if [[ -n "$HEALTH_URL" ]]; then
  echo "==> Waiting for $HEALTH_URL"
  for _ in $(seq 1 30); do
    if curl -fsS "$HEALTH_URL" >/dev/null 2>&1; then
      echo "Health check passed"
      break
    fi
    sleep 2
  done
  curl -fsS "$HEALTH_URL" >/dev/null 2>&1 || die "health check failed after restore"
fi

echo
echo "Restore complete."
echo "Previous databases: $PREVIOUS"
echo "If the backup predates the current release, run migrations once."
