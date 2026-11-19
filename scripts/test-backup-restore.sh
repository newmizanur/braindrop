#!/usr/bin/env bash
# Self-test for scripts/backup.sh and scripts/restore.sh.
#
# Creates disposable SQLite databases with real rows, backs them up, mutates
# the originals, restores the archive and verifies the rows and integrity.
# No service manager, Docker daemon or production database is involved.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

command -v sqlite3 >/dev/null 2>&1 || {
  echo "error: sqlite3 is required" >&2
  exit 1
}

WORK="$(mktemp -d "${TMPDIR:-/tmp}/braindrop-backup-test.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

DATA_DIR="$WORK/data"
BACKUP_DIR="$WORK/backups"
DB_PATH="$DATA_DIR/braindrop.sqlite"
QUEUE_PATH="$DATA_DIR/braindrop_queue.sqlite"
mkdir -p "$DATA_DIR" "$BACKUP_DIR"

sqlite3 "$DB_PATH" <<'SQL'
CREATE TABLE items (id INTEGER PRIMARY KEY, title TEXT NOT NULL);
INSERT INTO items (title) VALUES ('keep me'), ('and me');
CREATE TABLE user_settings (id INTEGER PRIMARY KEY, timezone TEXT NOT NULL);
INSERT INTO user_settings (timezone) VALUES ('Europe/Berlin');
SQL

sqlite3 "$QUEUE_PATH" <<'SQL'
CREATE TABLE jobs (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
INSERT INTO jobs (name) VALUES ('notification-delivery');
SQL

echo "==> Creating backup"
DATA_DIR="$DATA_DIR" DB_PATH="$DB_PATH" QUEUE_PATH="$QUEUE_PATH" \
  BACKUP_DIR="$BACKUP_DIR" RETENTION_DAYS=30 \
  ./scripts/backup.sh

ARCHIVE="$(find "$BACKUP_DIR" -name 'braindrop-backup-*.tar.gz' -type f | head -n 1)"
[[ -n "$ARCHIVE" ]] || {
  echo "error: backup archive was not created" >&2
  exit 1
}
[[ -f "$ARCHIVE.sha256" ]] || {
  echo "error: checksum sidecar was not created" >&2
  exit 1
}

echo "==> Mutating live databases"
sqlite3 "$DB_PATH" "DELETE FROM items; INSERT INTO items (title) VALUES ('mutated');"
sqlite3 "$QUEUE_PATH" "DELETE FROM jobs;"

echo "==> Restoring backup"
DATA_DIR="$DATA_DIR" DB_PATH="$DB_PATH" QUEUE_PATH="$QUEUE_PATH" \
  NO_SERVICE=1 SUDO= SERVICE_USER= \
  ./scripts/restore.sh "$ARCHIVE" --yes

COUNT="$(sqlite3 "$DB_PATH" 'SELECT COUNT(*) FROM items;')"
[[ "$COUNT" == "2" ]] || {
  echo "error: expected 2 restored items, found $COUNT" >&2
  exit 1
}
TITLE="$(sqlite3 "$DB_PATH" 'SELECT title FROM items ORDER BY id LIMIT 1;')"
[[ "$TITLE" == "keep me" ]] || {
  echo "error: restored row has unexpected title: $TITLE" >&2
  exit 1
}
TZ="$(sqlite3 "$DB_PATH" 'SELECT timezone FROM user_settings;')"
[[ "$TZ" == "Europe/Berlin" ]] || {
  echo "error: settings row was not restored" >&2
  exit 1
}
JOB="$(sqlite3 "$QUEUE_PATH" 'SELECT name FROM jobs;')"
[[ "$JOB" == "notification-delivery" ]] || {
  echo "error: queue database was not restored" >&2
  exit 1
}

for db in "$DB_PATH" "$QUEUE_PATH"; do
  INTEGRITY="$(sqlite3 "$db" 'PRAGMA integrity_check;')"
  [[ "$INTEGRITY" == "ok" ]] || {
    echo "error: integrity check failed for $db: $INTEGRITY" >&2
    exit 1
  }
done

echo "==> Backup and restore self-test passed"
