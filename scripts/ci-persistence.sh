#!/usr/bin/env bash
# Deployment persistence and static-asset checks against the running CI image.
#
# Creates a real item through the API, restarts the container, and verifies the
# item is still there. Also checks that the SPA and hashed assets are served,
# and that the in-container backup script can read the live database.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

COMPOSE=(docker compose -f docker-compose.ci.yml)
APP_URL="http://127.0.0.1:5150"

die() {
  echo "error: $*" >&2
  exit 1
}

app_curl() {
  "${COMPOSE[@]}" exec -T app curl -fsS "$@"
}

echo "==> Static assets and SPA fallback"
INDEX="$(app_curl "$APP_URL/")"
grep -qi '<div id="root"' <<<"$INDEX" || die "SPA index did not contain the React root"
DASHBOARD="$(app_curl "$APP_URL/dashboard")"
grep -qi '<div id="root"' <<<"$DASHBOARD" || die "SPA deep link did not fall back to index.html"
ASSET="$(grep -o '/assets/[^"]*\.js' <<<"$INDEX" | head -n 1)"
[[ -n "$ASSET" ]] || die "could not find a hashed asset in index.html"
app_curl -o /dev/null "$APP_URL$ASSET" || die "hashed asset $ASSET was not served"
echo "    index, deep link and $ASSET served"

echo "==> Creating an item through the API"
LOGIN="$(app_curl -X POST "$APP_URL/api/auth/login" \
  -H 'content-type: application/json' \
  -d '{"email":"ci@example.com","password":"ci-password"}')"
TOKEN="$(sed -n 's/.*"token":"\([^"]*\)".*/\1/p' <<<"$LOGIN")"
[[ -n "$TOKEN" ]] || die "login did not return a token"

STAMP="$(date -u +%s)"
TITLE="CI persistence probe $STAMP"
CREATE="$(app_curl -X POST "$APP_URL/api/items" \
  -H "authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d "{\"title\":\"$TITLE\"}")"
PID="$(sed -n 's/.*"pid":"\([^"]*\)".*/\1/p' <<<"$CREATE")"
[[ -n "$PID" ]] || die "item creation did not return a pid"

echo "==> Restarting the application container"
"${COMPOSE[@]}" restart app >/dev/null

echo "==> Waiting for health after restart"
HEALTHY=0
for _ in $(seq 1 60); do
  if "${COMPOSE[@]}" exec -T app curl -fsS "$APP_URL/_health" >/dev/null 2>&1; then
    HEALTHY=1
    break
  fi
  sleep 2
done
[[ "$HEALTHY" == "1" ]] || die "app did not become healthy after restart"

FETCHED="$(app_curl -H "authorization: Bearer $TOKEN" "$APP_URL/api/items/$PID")"
grep -q "$TITLE" <<<"$FETCHED" || die "item did not survive the container restart"
echo "    item survived the restart"

app_curl -X DELETE -H "authorization: Bearer $TOKEN" "$APP_URL/api/items/$PID" >/dev/null
echo "    probe item removed"

echo "==> In-container SQLite backup"
"${COMPOSE[@]}" exec -T app sh -c \
  'DATA_DIR=/data BACKUP_DIR=/data/backups /app/scripts/backup.sh' >/dev/null
"${COMPOSE[@]}" exec -T app sh -c \
  'ls /data/backups/braindrop-backup-*.tar.gz >/dev/null'

echo "==> Persistence, static assets and backup checks passed"
