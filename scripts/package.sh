#!/usr/bin/env bash
# Assemble a self-contained BrainDrop distribution directory and archive.
#
# Usage: scripts/package.sh [--check <dir>] <rust-target>
#
# Example:
#   scripts/build-native.sh x86_64-unknown-linux-musl
#   scripts/package.sh x86_64-unknown-linux-musl
#
# Produces:
#   dist/braindrop-linux-amd64-musl/
#   dist/braindrop-linux-amd64-musl.tar.gz
#   dist/braindrop-linux-amd64-musl.tar.gz.sha256
#
# The archive contains the release binary, production config, built SPA,
# operational scripts, .env.example, README and LICENSE. It never contains a
# database, a real .env, node_modules, source or build output.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

DIST="${DIST:-dist}"
TARGET=""
CHECK_DIR=""

if [[ "${1:-}" == "--check" ]]; then
  CHECK_DIR="${2:-}"
  [[ -n "$CHECK_DIR" ]] || {
    echo "usage: scripts/package.sh --check <distribution-dir>" >&2
    exit 2
  }
  TARGET="${3:-}"
else
  TARGET="${1:-}"
fi

platform_name() {
  case "$1" in
    x86_64-unknown-linux-musl) echo "linux-amd64-musl" ;;
    aarch64-unknown-linux-musl) echo "linux-arm64-musl" ;;
    x86_64-unknown-linux-gnu) echo "linux-amd64-glibc" ;;
    aarch64-unknown-linux-gnu) echo "linux-arm64-glibc" ;;
    aarch64-apple-darwin) echo "macos-arm64" ;;
    x86_64-apple-darwin) echo "macos-amd64" ;;
    *) return 1 ;;
  esac
}

check_layout() {
  local dir=$1
  local missing=0
  for path in \
    braindrop \
    config/production.yaml \
    frontend/dist/index.html \
    scripts/backup.sh \
    scripts/restore.sh \
    .env.example \
    README.md \
    LICENSE \
    RELEASE; do
    if [[ ! -e "$dir/$path" ]]; then
      echo "error: package is missing $path" >&2
      missing=1
    fi
  done
  [[ -x "$dir/braindrop" ]] || {
    echo "error: package binary is not executable" >&2
    missing=1
  }
  if find "$dir" -name '*.sqlite' -o -name '.env' | grep -q .; then
    echo "error: package contains a database or a real .env" >&2
    missing=1
  fi
  [[ "$missing" == "0" ]] || exit 1
  echo "==> Package layout is complete: $dir"
}

if [[ -n "$CHECK_DIR" ]]; then
  check_layout "$CHECK_DIR"
  exit 0
fi

[[ -n "$TARGET" ]] || {
  echo "usage: scripts/package.sh <rust-target>" >&2
  exit 2
}
NAME="$(platform_name "$TARGET")" || {
  echo "error: unsupported target: $TARGET" >&2
  exit 2
}

BINARY="${BINARY:-target/native/$TARGET/braindrop}"
if [[ ! -x "$BINARY" && -x "target/$TARGET/release/braindrop-cli" ]]; then
  BINARY="target/$TARGET/release/braindrop-cli"
fi
[[ -x "$BINARY" ]] || {
  echo "error: release binary not found for $TARGET; run scripts/build-native.sh $TARGET" >&2
  exit 1
}

if [[ ! -f frontend/dist/index.html ]]; then
  ./scripts/build-frontend.sh
fi

STAGE="$DIST/braindrop-$NAME"
rm -rf "$STAGE"
mkdir -p "$STAGE/config" "$STAGE/frontend" "$STAGE/scripts"

cp "$BINARY" "$STAGE/braindrop"
chmod 755 "$STAGE/braindrop"
cp config/production.yaml "$STAGE/config/production.yaml"
cp -R frontend/dist "$STAGE/frontend/dist"
for script in backup.sh restore.sh test-backup-restore.sh \
  deploy-init-alpine.sh deploy-init-ubuntu.sh; do
  cp "scripts/$script" "$STAGE/scripts/$script"
  chmod 755 "$STAGE/scripts/$script"
done
cp .env.example "$STAGE/.env.example"
cp README.md LICENSE "$STAGE/"
cp docs/DEPLOYMENT.md "$STAGE/DEPLOYMENT.md"

GIT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo unknown)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
cat >"$STAGE/RELEASE" <<EOF
version=$VERSION
target=$TARGET
platform=$NAME
git_sha=$GIT_SHA
built_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)
EOF

check_layout "$STAGE"

ARCHIVE="$DIST/braindrop-$NAME.tar.gz"
rm -f "$ARCHIVE" "$ARCHIVE.sha256"
tar -czf "$ARCHIVE" -C "$DIST" "braindrop-$NAME"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$DIST" && sha256sum "braindrop-$NAME.tar.gz" >"braindrop-$NAME.tar.gz.sha256")
else
  (cd "$DIST" && shasum -a 256 "braindrop-$NAME.tar.gz" >"braindrop-$NAME.tar.gz.sha256")
fi

echo
echo "Distribution: $STAGE"
echo "Binary: $(wc -c <"$STAGE/braindrop" | tr -d ' ') bytes"
echo "Archive: $ARCHIVE ($(wc -c <"$ARCHIVE" | tr -d ' ') bytes)"
echo "Checksum: $(cat "$ARCHIVE.sha256")"
