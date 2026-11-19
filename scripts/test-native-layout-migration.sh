#!/usr/bin/env bash
# Verify legacy native data/secrets survive the flat-layout migration.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
file_mode() {
  if stat -c %a "$1" >/dev/null 2>&1; then stat -c %a "$1"; else stat -f %Lp "$1"; fi
}
root="$work/braindrop"
mkdir -p "$root/shared/data" "$root/shared/backups" "$root/shared/logs" \
  "$root/releases/old"
printf 'secret-sentinel\n' >"$root/shared/.env"
printf 'DATABASE_URL=sqlite://%s/shared/data/braindrop.sqlite?mode=rwc\n' "$root" >>"$root/shared/.env"
printf 'QUEUE_URL=sqlite://%s/shared/data/braindrop_queue.sqlite?mode=rwc\n' "$root" >>"$root/shared/.env"
chmod 600 "$root/shared/.env"
printf 'app-db-sentinel\n' >"$root/shared/data/braindrop.sqlite"
printf 'queue-db-sentinel\n' >"$root/shared/data/braindrop_queue.sqlite"
printf 'backup-sentinel\n' >"$root/shared/backups/previous.tar.gz"
printf 'log-sentinel\n' >"$root/shared/logs/braindrop.log"

./scripts/migrate-native-layout.sh "$root" "$(id -un)"

grep -qx 'secret-sentinel' "$root/.env"
grep -Fq "DATABASE_URL=sqlite://$root/data/braindrop.sqlite?mode=rwc" "$root/.env"
grep -Fq "QUEUE_URL=sqlite://$root/data/braindrop_queue.sqlite?mode=rwc" "$root/.env"
[[ "$(file_mode "$root/.env")" == 600 ]]
[[ "$(cat "$root/data/braindrop.sqlite")" == app-db-sentinel ]]
[[ "$(cat "$root/data/braindrop_queue.sqlite")" == queue-db-sentinel ]]
[[ "$(cat "$root/backup/previous.tar.gz")" == backup-sentinel ]]
[[ "$(cat "$root/logs/braindrop.log")" == log-sentinel ]]
[[ -L "$root/shared/.env" && -L "$root/shared/data" && -L "$root/shared/logs" ]]
[[ -f "$root/shared/data/braindrop.sqlite" ]]

# A second run is safe while compatibility links still exist.
./scripts/migrate-native-layout.sh "$root" "$(id -un)"
echo "==> Legacy flat-layout migration preserves env, databases and state"
