#!/usr/bin/env bash
# Consistent SQLite backup for BrainDrop.
#
# Uses SQLite's online backup API (`.backup`) so a live database with WAL
# writes is captured consistently. Never copies a live database file with cp.
#
# Environment:
#   DATA_DIR        directory holding the databases (default: /data)
#   DB_PATH         application database (default: $DATA_DIR/braindrop.sqlite)
#   QUEUE_PATH      Loco queue database (default: $DATA_DIR/braindrop_queue.sqlite)
#   BACKUP_DIR      destination (default: $DATA_DIR/backups)
#   RETENTION_DAYS  delete archives older than this (default: 30)
#   INCLUDE_ENV=1   include ENV_FILE in the archive (contains secrets)
#   ENV_FILE        environment file to include when INCLUDE_ENV=1
#   PREFIX          archive prefix (default: braindrop)
#
# The archive is a gzipped tar with the database copies, a manifest and an
# optional environment file. A `.sha256` sidecar is written next to it.
set -euo pipefail

DATA_DIR="${DATA_DIR:-/data}"
DB_PATH="${DB_PATH:-$DATA_DIR/braindrop.sqlite}"
QUEUE_PATH="${QUEUE_PATH:-$DATA_DIR/braindrop_queue.sqlite}"
BACKUP_DIR="${BACKUP_DIR:-$DATA_DIR/backups}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
INCLUDE_ENV="${INCLUDE_ENV:-0}"
ENV_FILE="${ENV_FILE:-}"
PREFIX="${PREFIX:-braindrop}"

die() {
  echo "error: $*" >&2
  exit 1
}

command -v sqlite3 >/dev/null 2>&1 || die "sqlite3 is required (install the sqlite package)"
[[ -f "$DB_PATH" ]] || die "application database not found: $DB_PATH"

if [[ ! "$RETENTION_DAYS" =~ ^[0-9]+$ ]]; then
  die "RETENTION_DAYS must be a non-negative integer"
fi

mkdir -p "$BACKUP_DIR"
BACKUP_DIR="$(cd "$BACKUP_DIR" && pwd)"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/braindrop-backup.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "sha256sum or shasum is required"
  fi
}

backup_one() {
  local source=$1
  local name
  name="$(basename "$source")"
  local target="$WORK/$name"

  echo "==> Backing up $source"
  sqlite3 "$source" ".backup '$target'"
  local integrity
  integrity="$(sqlite3 "$target" 'PRAGMA integrity_check;')"
  [[ "$integrity" == "ok" ]] || die "integrity check failed for $name: $integrity"
  [[ -s "$target" ]] || die "backup file is empty: $name"
}

QUEUE_INCLUDED=0
if [[ -f "$QUEUE_PATH" ]]; then
  # Queue first, then app DB. Scheduling persists the delivery row before
  # enqueueing its job; this ordering plus the recovery scan means a restored
  # pair cannot contain a queued job whose delivery row is absent. These are
  # still sequential online snapshots, not an atomic cross-database snapshot.
  backup_one "$QUEUE_PATH"
  QUEUE_INCLUDED=1
else
  echo "warning: queue database not found at $QUEUE_PATH; skipping it" >&2
fi

backup_one "$DB_PATH"

{
  echo "format=1"
  echo "created_at=$STAMP"
  echo "application_db=$(basename "$DB_PATH")"
  if [[ "$QUEUE_INCLUDED" == "1" ]]; then
    echo "queue_db=$(basename "$QUEUE_PATH")"
  fi
  echo "application_sha256=$(sha256_file "$WORK/$(basename "$DB_PATH")")"
  if [[ "$QUEUE_INCLUDED" == "1" ]]; then
    echo "queue_sha256=$(sha256_file "$WORK/$(basename "$QUEUE_PATH")")"
  fi
} >"$WORK/manifest.txt"

if [[ "$INCLUDE_ENV" == "1" ]]; then
  [[ -n "$ENV_FILE" && -f "$ENV_FILE" ]] || die "INCLUDE_ENV=1 requires ENV_FILE to point at an existing file"
  cp "$ENV_FILE" "$WORK/.env"
  chmod 600 "$WORK/.env"
  echo "warning: the archive contains $ENV_FILE (secrets). Encrypt it at rest." >&2
fi

ARCHIVE="$BACKUP_DIR/$PREFIX-backup-$STAMP.tar.gz"
tar -czf "$ARCHIVE" -C "$WORK" .
chmod 600 "$ARCHIVE"
sha256_file "$ARCHIVE" >"$ARCHIVE.sha256"
chmod 600 "$ARCHIVE.sha256"

if [[ "$RETENTION_DAYS" -gt 0 ]]; then
  find "$BACKUP_DIR" -maxdepth 1 -type f \
    \( -name "$PREFIX-backup-*.tar.gz" -o -name "$PREFIX-backup-*.tar.gz.sha256" \) \
    -mtime +"$RETENTION_DAYS" -print -delete
fi

echo
echo "Backup written: $ARCHIVE"
echo "Size: $(wc -c <"$ARCHIVE" | tr -d ' ') bytes"
echo "Checksum: $(cat "$ARCHIVE.sha256")"
