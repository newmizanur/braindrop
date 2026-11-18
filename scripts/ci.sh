#!/usr/bin/env bash
# Reproducible, Docker-first local CI.
#
# This is the single entry point for validation. GitHub Actions runs the same
# script through a manual `workflow_dispatch`, so local and hosted checks do
# not drift apart.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is required for local CI" >&2
  exit 1
fi

if ! docker info >/dev/null 2>&1; then
  echo "error: the Docker daemon is not running" >&2
  exit 1
fi

export COMPOSE_PROJECT_NAME=braindrop-ci
COMPOSE=(docker compose -f docker-compose.ci.yml)

cleanup() {
  "${COMPOSE[@]}" down --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

echo "==> Building CI and production images"
"${COMPOSE[@]}" build rust frontend e2e app

echo "==> Rust: formatting, clippy, tests and SQLite migrations"
"${COMPOSE[@]}" run --rm rust

echo "==> Frontend: typecheck, lint, unit tests and production build"
"${COMPOSE[@]}" run --rm frontend

echo "==> Starting the production image and waiting for /_health"
"${COMPOSE[@]}" up -d --wait app
"${COMPOSE[@]}" exec -T app curl -fsS http://127.0.0.1:5150/_health >/dev/null
echo "health check passed"

# The banner is the supported Loco signal that all three runtime modes were
# booted. A server-only start would leave notifications and scheduled scans
# silently dead, so CI fails loudly if any mode is missing.
APP_LOGS="$("${COMPOSE[@]}" logs --no-color app)"
grep -q "worker is online" <<<"$APP_LOGS" || {
  echo "error: the production image did not start the Loco worker" >&2
  exit 1
}
grep -q "scheduler is running" <<<"$APP_LOGS" || {
  echo "error: the production image did not start the Loco scheduler" >&2
  exit 1
}
echo "worker and scheduler are running"

echo "==> Playwright: browser smoke tests against the production image"
"${COMPOSE[@]}" run --rm e2e

echo
echo "All Docker CI checks passed."
