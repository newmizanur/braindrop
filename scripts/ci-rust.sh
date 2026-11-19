#!/usr/bin/env bash
# Rust checks for the Docker-first local CI pipeline.
#
# Runs against the source copied into the image. Test databases are created
# inside the container and are never the development or production database.
set -euo pipefail

echo "==> cargo fmt"
cargo fmt --all -- --check

echo "==> cargo clippy"
cargo clippy --all-targets -- -D warnings

echo "==> cargo test"
cargo test --workspace

echo "==> SQLite migrations"
rm -f /tmp/braindrop_ci.sqlite /tmp/braindrop_ci.sqlite-shm /tmp/braindrop_ci.sqlite-wal
rm -f /tmp/braindrop_ci_queue.sqlite /tmp/braindrop_ci_queue.sqlite-shm /tmp/braindrop_ci_queue.sqlite-wal
DATABASE_URL="sqlite:///tmp/braindrop_ci.sqlite?mode=rwc" \
QUEUE_URL="sqlite:///tmp/braindrop_ci_queue.sqlite?mode=rwc" \
cargo run --quiet --bin braindrop-cli -- db migrate
rm -f /tmp/braindrop_ci.sqlite /tmp/braindrop_ci.sqlite-shm /tmp/braindrop_ci.sqlite-wal
rm -f /tmp/braindrop_ci_queue.sqlite /tmp/braindrop_ci_queue.sqlite-shm /tmp/braindrop_ci_queue.sqlite-wal

echo "==> Backup and restore self-test"
./scripts/test-backup-restore.sh

echo "==> Deployment argument validation"
SSH_HOST=ci.example.com SSH_USER=root OS=alpine ARCH=amd64 ./scripts/deploy.sh validate
SSH_HOST=ci.example.com SSH_USER=deploy OS=ubuntu ARCH=arm64 ./scripts/deploy.sh validate

echo "==> Service script syntax"
bash -n scripts/deploy.sh
sh -n scripts/deploy-init-alpine.sh
sh -n scripts/deploy-init-ubuntu.sh
bash -n scripts/backup.sh
bash -n scripts/restore.sh
bash -n scripts/package.sh

echo "==> Distribution package layout check"
PKG_CHECK="$(mktemp -d)"
mkdir -p "$PKG_CHECK/config" "$PKG_CHECK/frontend/dist" "$PKG_CHECK/scripts"
printf '#!/bin/sh\nexit 0\n' >"$PKG_CHECK/braindrop"
chmod 755 "$PKG_CHECK/braindrop"
: >"$PKG_CHECK/config/production.yaml"
: >"$PKG_CHECK/frontend/dist/index.html"
: >"$PKG_CHECK/scripts/backup.sh"
: >"$PKG_CHECK/scripts/restore.sh"
: >"$PKG_CHECK/.env.example"
: >"$PKG_CHECK/README.md"
: >"$PKG_CHECK/LICENSE"
: >"$PKG_CHECK/RELEASE"
./scripts/package.sh --check "$PKG_CHECK"
rm -rf "$PKG_CHECK"

echo "==> Rust checks passed"
