#!/usr/bin/env bash
# Exercise public archive filtering, runtime bundle contents and root-owned
# permission normalization. Run only in the disposable Docker CI container.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
[[ "$(id -u)" == 0 ]] || {
  echo "error: package permission test requires root (run through Docker CI)" >&2
  exit 1
}

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod 755 "$work"
mkdir -p "$work/bin" "$work/frontend-dist/__MACOSX"
cat >"$work/bin/braindrop" <<'EOF'
#!/bin/sh
[ "$(id -u)" -ne 0 ] || exit 7
echo non-root-execution-ok
EOF
chmod 700 "$work/bin/braindrop"
printf 'frontend test asset\n' >"$work/frontend-dist/index.html"
printf 'mac metadata\n' >"$work/frontend-dist/._index.html"
printf 'mac metadata\n' >"$work/frontend-dist/.DS_Store"
printf 'mac metadata\n' >"$work/frontend-dist/__MACOSX/._index.html"
printf 'deployment docs fixture\n' >"$work/DEPLOYMENT.md"

DIST="$work/dist" BINARY="$work/bin/braindrop" FRONTEND_DIST="$work/frontend-dist" \
DEPLOYMENT_DOC="$work/DEPLOYMENT.md" \
  ./scripts/package.sh x86_64-unknown-linux-musl >/dev/null
distribution="$work/dist/braindrop-linux-amd64-musl"
public_archive="$work/dist/braindrop-linux-amd64-musl.tar.gz"

for path in README.md LICENSE DEPLOYMENT.md .env.example; do
  [[ -f "$distribution/$path" ]] || { echo "error: public package missing $path" >&2; exit 1; }
done
[[ ! -e "$distribution/scripts/test-backup-restore.sh" ]] || {
  echo "error: developer test script shipped in public archive" >&2; exit 1;
}
public_members="$(tar -tzf "$public_archive")"
if grep -E '(^|/)(\._[^/]*|\.DS_Store|__MACOSX)(/|$)' <<<"$public_members" >/dev/null; then
  echo "error: public archive contains macOS metadata" >&2
  exit 1
fi
grep -q '/LICENSE$' <<<"$public_members"
grep -q '/DEPLOYMENT.md$' <<<"$public_members"

# Simulate a macOS-built bundle with foreign numeric owners and restrictive
# directory/file modes, then verify remote normalization restores safe modes.
cp -R "$distribution" "$work/foreign"
printf 'mac metadata\n' >"$work/foreign/._root"
mkdir -p "$work/foreign/__MACOSX"
printf 'mac metadata\n' >"$work/foreign/__MACOSX/.DS_Store"
find "$work/foreign" -type d -exec chmod 700 {} \;
find "$work/foreign" -type f -exec chmod 600 {} \;
chmod 700 "$work/foreign/braindrop"
chown -R 65534:65534 "$work/foreign"
runtime_archive="$work/runtime.tar.gz"
./scripts/create-runtime-archive.sh "$work/foreign" "$runtime_archive"

members="$(tar -tzf "$runtime_archive")"
for expected in braindrop config/production.yaml frontend/dist/index.html \
  scripts/backup.sh scripts/restore.sh; do
  grep -qx "$expected" <<<"$members" || {
    echo "error: runtime archive missing $expected" >&2; exit 1;
  }
done
for forbidden in README.md LICENSE DEPLOYMENT.md .env.example RELEASE \
  scripts/test-backup-restore.sh; do
  if grep -q "$forbidden" <<<"$members"; then
    echo "error: unnecessary file included in runtime archive: $forbidden" >&2
    exit 1
  fi
done
if grep -E '(^|/)(\._[^/]*|\.DS_Store|__MACOSX)(/|$)' <<<"$members" >/dev/null; then
  echo "error: runtime archive contains macOS metadata" >&2
  exit 1
fi

mkdir "$work/extracted"
chmod 755 "$work/extracted"
tar -xzf "$runtime_archive" -C "$work/extracted"
sh "$work/extracted/scripts/prepare-release.sh" "$work/extracted"
[[ "$(stat -c %u "$work/extracted/braindrop")" == 0 ]]
[[ "$(stat -c %a "$work/extracted")" == 755 ]]
[[ "$(stat -c %a "$work/extracted/braindrop")" == 755 ]]
[[ "$(stat -c %a "$work/extracted/config")" == 755 ]]
[[ "$(stat -c %a "$work/extracted/config/production.yaml")" == 644 ]]
[[ "$(stat -c %a "$work/extracted/frontend/dist/index.html")" == 644 ]]
[[ "$(stat -c %a "$work/extracted/scripts/backup.sh")" == 755 ]]
[[ "$(runuser -u nobody -- "$work/extracted/braindrop")" == "non-root-execution-ok" ]]
runuser -u nobody -- test -r "$work/extracted/config/production.yaml"
runuser -u nobody -- test -r "$work/extracted/frontend/dist/index.html"

echo "==> Public distribution/runtime archive and non-root permission tests passed"
