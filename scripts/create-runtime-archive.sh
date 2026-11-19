#!/usr/bin/env bash
# Build a minimal SSH deployment bundle from a public distribution directory.
set -euo pipefail

SOURCE="${1:-}"
OUTPUT="${2:-}"
if [[ -z "$SOURCE" || -z "$OUTPUT" || ! -d "$SOURCE" ]]; then
  echo "usage: create-runtime-archive.sh <distribution-dir> <output-archive>" >&2
  exit 2
fi

for path in braindrop config/production.yaml frontend/dist/index.html \
  scripts/backup.sh scripts/restore.sh scripts/deploy-init-alpine.sh \
  scripts/deploy-init-ubuntu.sh scripts/prepare-release.sh \
  scripts/migrate-native-layout.sh; do
  [[ -f "$SOURCE/$path" ]] || {
    echo "error: distribution is missing $path" >&2
    exit 1
  }
done

COPYFILE_DISABLE=1 tar -czf "$OUTPUT" --exclude='._*' --exclude='.DS_Store' --exclude='__MACOSX' \
  -C "$SOURCE" \
  braindrop config/production.yaml frontend/dist \
  scripts/backup.sh scripts/restore.sh scripts/deploy-init-alpine.sh \
  scripts/deploy-init-ubuntu.sh scripts/prepare-release.sh \
  scripts/migrate-native-layout.sh
