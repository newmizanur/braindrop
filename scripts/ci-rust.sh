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

echo "==> Rust checks passed"
