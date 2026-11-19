#!/usr/bin/env bash
# Build one release binary for a supported Rust target.
#
# Usage: scripts/build-native.sh <target>
#
# Targets:
#   x86_64-unknown-linux-musl    Alpine AMD64
#   aarch64-unknown-linux-musl   Alpine ARM64
#   x86_64-unknown-linux-gnu     Ubuntu AMD64
#   aarch64-unknown-linux-gnu    Ubuntu ARM64
#   aarch64-apple-darwin         macOS Apple Silicon
#   x86_64-apple-darwin          macOS Intel
#
# Linux builds run in Docker so no Linux toolchain is required on the host.
# The result is copied to target/native/<target>/braindrop.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

TARGET="${1:-}"
if [[ -z "$TARGET" ]]; then
  echo "usage: scripts/build-native.sh <rust-target>" >&2
  exit 2
fi

OUT_DIR="${OUT_DIR:-target/native}"
BIN_NAME="braindrop-cli"
REGISTRY_VOLUME="${REGISTRY_VOLUME:-braindrop-cargo-registry}"
BUILD_SHA="${BUILD_SHA:-$(git rev-parse --short HEAD 2>/dev/null || echo unknown)}"
export BUILD_SHA

case "$TARGET" in
  x86_64-unknown-linux-musl | aarch64-unknown-linux-musl | \
  x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu | \
  aarch64-apple-darwin | x86_64-apple-darwin) ;;
  *)
    echo "error: unsupported target: $TARGET" >&2
    exit 2
    ;;
esac

if [[ "${SKIP_FRONTEND:-0}" != "1" ]]; then
  ./scripts/build-frontend.sh
fi

mkdir -p "$OUT_DIR/$TARGET"

copy_out() {
  local volume=$1
  local target_dir=$2
  docker run --rm \
    -v "$volume":/target \
    -v "$(cd "$OUT_DIR/$TARGET" && pwd)":/out \
    alpine:3.20 \
    sh -c "cp '/target/$target_dir/release/$BIN_NAME' /out/braindrop && chmod 755 /out/braindrop"
}

build_musl() {
  local arch=$1
  local image="messense/rust-musl-cross:${arch}-musl"
  local volume="braindrop-target-${TARGET}"

  echo "==> Building $TARGET with $image"
  docker run --rm \
    -v "$PWD":/home/rust/src \
    -w /home/rust/src \
    -e "BUILD_SHA=$BUILD_SHA" \
    -v "$REGISTRY_VOLUME":/root/.cargo/registry \
    -v "$volume":/home/rust/src/target \
    "$image" \
    cargo build --release --target "$TARGET" --bin "$BIN_NAME"
  copy_out "$volume" "$TARGET"
}

build_gnu() {
  local platform=$1
  local slug=${platform#linux/}
  local image="braindrop-native-glibc:bullseye-$slug"
  local volume="braindrop-target-${TARGET}"

  echo "==> Building glibc toolchain image ($platform)"
  docker build --platform "$platform" \
    -f docker/native/glibc.Dockerfile \
    -t "$image" \
    docker/native

  echo "==> Building $TARGET in $image"
  docker run --rm --platform "$platform" \
    -v "$PWD":/app \
    -w /app \
    -e "BUILD_SHA=$BUILD_SHA" \
    -v "$REGISTRY_VOLUME":/usr/local/cargo/registry \
    -v "$volume":/app/target \
    "$image" \
    cargo build --release --target "$TARGET" --bin "$BIN_NAME"
  copy_out "$volume" "$TARGET"
}

build_macos() {
  echo "==> Building $TARGET natively"
  rustup target add "$TARGET" >/dev/null 2>&1 || true
  cargo build --release --target "$TARGET" --bin "$BIN_NAME"
  cp "target/$TARGET/release/$BIN_NAME" "$OUT_DIR/$TARGET/braindrop"
  chmod 755 "$OUT_DIR/$TARGET/braindrop"
}

case "$TARGET" in
  x86_64-unknown-linux-musl) build_musl x86_64 ;;
  aarch64-unknown-linux-musl) build_musl aarch64 ;;
  x86_64-unknown-linux-gnu) build_gnu linux/amd64 ;;
  aarch64-unknown-linux-gnu) build_gnu linux/arm64 ;;
  aarch64-apple-darwin | x86_64-apple-darwin) build_macos ;;
esac

BINARY="$OUT_DIR/$TARGET/braindrop"
[[ -x "$BINARY" ]] || {
  echo "error: build did not produce $BINARY" >&2
  exit 1
}
echo "==> $TARGET: $BINARY ($(wc -c <"$BINARY" | tr -d ' ') bytes)"
