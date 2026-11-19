#!/bin/sh
# One-time move from the former current/releases/shared tree to the flat runtime.
set -eu

ROOT="${1:-}"
SERVICE_USER="${2:-braindrop}"
[ -n "$ROOT" ] && [ -d "$ROOT" ] || {
  echo "usage: migrate-native-layout.sh <application-root> [service-user]" >&2
  exit 2
}

legacy="$ROOT/shared"
env_source="$ROOT/.env"
if [ -f "$ROOT/.env" ] && [ -f "$legacy/.env" ] && ! cmp -s "$ROOT/.env" "$legacy/.env"; then
  echo "error: both flat and legacy environment files differ; refusing to discard either" >&2
  exit 1
fi
if [ ! -f "$env_source" ] && [ -f "$legacy/.env" ]; then
  env_source="$legacy/.env"
fi
if [ -f "$env_source" ]; then
  tmp="$ROOT/.env.migrate.$$"
  cp -p "$env_source" "$tmp"
  sed "s|sqlite://$ROOT/shared/data/|sqlite://$ROOT/data/|g" "$tmp" >"$tmp.next"
  mv -f "$tmp.next" "$tmp"
  chown "$SERVICE_USER:$SERVICE_USER" "$tmp" 2>/dev/null || true
  chmod 600 "$tmp"
  mv -f "$tmp" "$ROOT/.env"
  if [ "$env_source" != "$ROOT/.env" ]; then
    rm -f "$env_source"
  fi
fi

move_state_dir() {
  old="$legacy/$1"
  new="$ROOT/$2"
  [ ! -L "$old" ] || return 0
  [ -d "$old" ] || return 0
  if [ -d "$new" ]; then
    if [ -n "$(ls -A "$new" 2>/dev/null)" ]; then
      echo "error: both $old and $new contain data; refusing to merge" >&2
      exit 1
    fi
    rmdir "$new"
  fi
  mv "$old" "$new"
}

move_state_dir data data
move_state_dir backups backup
move_state_dir logs logs

mkdir -p "$ROOT/data" "$ROOT/logs" "$ROOT/backup"
chown -R "$SERVICE_USER:$SERVICE_USER" "$ROOT/data" "$ROOT/logs" "$ROOT/backup" 2>/dev/null || true
chmod 750 "$ROOT/data" "$ROOT/logs" "$ROOT/backup"
[ ! -f "$ROOT/.env" ] || chmod 600 "$ROOT/.env"

# Keep the old service definition working until the flat service is installed.
# deploy.sh removes this compatibility shim after health passes.
mkdir -p "$legacy"
[ -e "$legacy/.env" ] || [ -L "$legacy/.env" ] || ln -s ../.env "$legacy/.env"
[ -e "$legacy/data" ] || [ -L "$legacy/data" ] || ln -s ../data "$legacy/data"
[ -e "$legacy/backups" ] || [ -L "$legacy/backups" ] || ln -s ../backup "$legacy/backups"
[ -e "$legacy/logs" ] || [ -L "$legacy/logs" ] || ln -s ../logs "$legacy/logs"
