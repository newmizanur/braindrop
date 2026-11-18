#!/usr/bin/env bash
# Start an isolated BrainDrop server for local Playwright runs.
#
# The database files are dedicated to e2e and are removed on every start. Never
# point this script at a development or production database.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

export LOCO_ENV=development
export PORT=5199
export BINDING=127.0.0.1
export DATABASE_URL="sqlite://braindrop_e2e.sqlite?mode=rwc"
export QUEUE_URL="sqlite://loco_e2e.sqlite?mode=rwc"
export ALLOW_REGISTRATION=false
export REQUIRE_EMAIL_VERIFICATION=false
export E2E_EMAIL="${E2E_EMAIL:-e2e@example.com}"
export E2E_PASSWORD="${E2E_PASSWORD:-e2e-password}"
# Playwright starts an OpenRouter-compatible mock on this port. The app uses it
# only during e2e runs, so no real provider credentials are needed.
export AI_ENABLED=true
export OPENROUTER_API_KEY=sk-e2e-mock
export OPENROUTER_MODEL=e2e/mock
export OPENROUTER_BASE_URL="http://127.0.0.1:${MOCK_OPENROUTER_PORT:-8099}"

rm -f braindrop_e2e.sqlite braindrop_e2e.sqlite-shm braindrop_e2e.sqlite-wal
rm -f loco_e2e.sqlite loco_e2e.sqlite-shm loco_e2e.sqlite-wal

# The project pins pnpm 10.12.4 in package.json; corepack honours that even
# when a newer global pnpm is installed.
corepack pnpm@10.12.4 --dir frontend build

cargo loco db migrate
cargo loco task admin:create \
  "email:${E2E_EMAIL}" \
  "name:E2E User" \
  "password:${E2E_PASSWORD}" \
  verified:true \
  force:true

exec cargo loco start
