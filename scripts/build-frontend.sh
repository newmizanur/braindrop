#!/usr/bin/env bash
# Build the React SPA with the project's pinned pnpm version.
#
# Loco serves `frontend/dist` from disk; it must exist before the Rust binary
# starts and before a distribution archive is assembled.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

PNPM_VERSION="${PNPM_VERSION:-10.12.4}"
if ! command -v node >/dev/null 2>&1; then
  echo "error: node is required to build the frontend" >&2
  exit 1
fi

if [[ "${SKIP_INSTALL:-0}" != "1" && ! -d frontend/node_modules ]]; then
  echo "==> Installing frontend dependencies"
  corepack "pnpm@$PNPM_VERSION" --dir frontend install --frozen-lockfile
fi

echo "==> Building frontend"
corepack "pnpm@$PNPM_VERSION" --dir frontend build

[[ -f frontend/dist/index.html ]] || {
  echo "error: frontend build did not produce frontend/dist/index.html" >&2
  exit 1
}
