#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

MODE="${1:-check}"
case "$MODE" in
  generate)
    TS_RS_EXPORT_DIR=frontend/src/bindings cargo test --lib export_bindings_
    ;;
  check)
    export_dir="$(mktemp -d)"
    trap 'rm -rf "$export_dir"' EXIT
    TS_RS_EXPORT_DIR="$export_dir" cargo test --lib export_bindings_ --quiet
    diff -ru frontend/src/bindings "$export_dir"
    ;;
  *)
    echo "usage: $0 [generate|check]" >&2
    exit 2
    ;;
esac
