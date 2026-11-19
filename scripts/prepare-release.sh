#!/bin/sh
# Normalize an extracted runtime tree before it is installed on Alpine/Ubuntu.
# Archives can be built on macOS or extracted with different tar defaults.
set -eu

DIR="${1:-}"
[ -n "$DIR" ] && [ -d "$DIR" ] || {
  echo "usage: prepare-release.sh <extracted-runtime-dir>" >&2
  exit 2
}
[ "$(id -u)" = "0" ] || {
  echo "error: prepare-release.sh must run as root" >&2
  exit 1
}
[ -f "$DIR/braindrop" ] || {
  echo "error: runtime bundle is missing braindrop" >&2
  exit 1
}

chown -R root:root "$DIR"
find "$DIR" \( -name '._*' -o -name '.DS_Store' -o -name '__MACOSX' \) -exec rm -rf {} +
find "$DIR" -type d -exec chmod 755 {} \;
find "$DIR" -type f -exec chmod 644 {} \;
chmod 755 "$DIR/braindrop"
for script in backup.sh restore.sh deploy-init-alpine.sh deploy-init-ubuntu.sh \
  prepare-release.sh; do
  [ ! -f "$DIR/scripts/$script" ] || chmod 755 "$DIR/scripts/$script"
done
