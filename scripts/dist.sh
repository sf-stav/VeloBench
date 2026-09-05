#!/usr/bin/env bash
# ============================================================================
#  Package a VeloBenchmark release artifact for a target platform.
#
#  Produces:  dist/velobenchmark-<os>-<arch>.tar.gz   (+ .sha256)
#             containing the single `velobench` binary — exactly the name the
#             one-line installer (install.sh) looks for on GitHub Releases:
#
#    https://github.com/sf-stav/VeloBench/releases/latest/download/
#        velobenchmark-<os>-<arch>.tar.gz
#
#  Usage:
#    bash scripts/dist.sh                          # native build, package
#    bash scripts/dist.sh --skip-fe                # reuse the current assets/ build
#    bash scripts/dist.sh aarch64-unknown-linux-gnu
#                                                  # cross-build (needs
#                                                  # gcc-aarch64-linux-gnu +
#                                                  # rustup target add aarch64-unknown-linux-gnu)
#
#  Supported targets:
#    (native)                           -> linux-<arch> / macos-<arch> per host
#    x86_64-unknown-linux-musl          -> linux-x86_64   STATIC (needs cargo-zigbuild + zig)
#    aarch64-unknown-linux-musl         -> linux-arm64    STATIC (needs cargo-zigbuild + zig)
#    x86_64-unknown-linux-gnu           -> linux-x86_64   (cross: gcc-x86-64-linux-gnu)
#    aarch64-unknown-linux-gnu          -> linux-arm64    (cross: gcc-aarch64-linux-gnu)
#
#  The musl builds are fully static — they run on any Linux (glibc or musl),
#  which makes them the recommended release artifacts. zig setup, no root:
#    pip install --user ziglang
#    printf '#!/bin/sh\\nexec python3 -m ziglang "$@"\\n' > ~/.local/bin/zig && chmod +x ~/.local/bin/zig
#    cargo install cargo-zigbuild
#    rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl
#
#  macOS builds must run on macOS (Apple SDK licensing) — use a Mac or the
#  GitHub Actions release workflow (.github/workflows/release.yml), which
#  produces all four artifacts from a tag push.
# ============================================================================
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

TARGET=""
SKIP_FE=0
for arg in "$@"; do
  case "$arg" in
    --skip-fe) SKIP_FE=1 ;;
    *) TARGET="$arg" ;;
  esac
done

# --- resolve host tag (used when no target given) ---------------------------
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in Linux) HOST_TAG="linux" ;; Darwin) HOST_TAG="macos" ;; *) echo "unsupported OS: $OS" >&2; exit 1 ;; esac
case "$ARCH" in x86_64|amd64) HOST_ARCH="x86_64" ;; aarch64|arm64) HOST_ARCH="arm64" ;; *) echo "unsupported arch: $ARCH" >&2; exit 1 ;; esac

# --- map target triple -> release tag + binary path -------------------------
TRIPLE_ARGS=()
CARGO_SUB="build"
BIN_PATH="$ROOT/target/release/velobench"
STRIP="strip"
if [[ -n "$TARGET" ]]; then
  case "$TARGET" in
    x86_64-unknown-linux-musl)  TAG="linux-x86_64" ;;
    aarch64-unknown-linux-musl) TAG="linux-arm64" ;;
    x86_64-unknown-linux-gnu)   TAG="linux-x86_64" ;;
    aarch64-unknown-linux-gnu)  TAG="linux-arm64" ;;
    x86_64-apple-darwin)        TAG="macos-x86_64" ;;
    aarch64-apple-darwin)       TAG="macos-arm64" ;;
    *) echo "unsupported target: $TARGET" >&2; exit 1 ;;
  esac
  BIN_PATH="$ROOT/target/$TARGET/release/velobench"
  if [[ "$TARGET" == *"-musl" ]]; then
    # fully static builds via zig as the cross C compiler: cargo zigbuild
    export PATH="$HOME/.local/bin:$PATH"
    CARGO_SUB="zigbuild"
    TRIPLE_ARGS=(--target "$TARGET")
  else
    CARGO_SUB="build"
    TRIPLE_ARGS=(--target "$TARGET")
    if [[ "$TARGET" == "aarch64-unknown-linux-gnu" ]]; then
      export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
      export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
      export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
      STRIP="aarch64-linux-gnu-strip"
    fi
  fi
else
  TAG="$HOST_TAG-$HOST_ARCH"
fi

if [[ $SKIP_FE -eq 0 ]]; then
  echo "==> building frontend"
  bash scripts/build-frontend.sh
fi

echo "==> cargo $CARGO_SUB --release ${TRIPLE_ARGS[*]:-} (velobench)"
cargo "$CARGO_SUB" --release ${TRIPLE_ARGS[@]+"${TRIPLE_ARGS[@]}"} --bin velobench

# strip to shrink the artifact (best effort — keep going if strip is absent)
if command -v "$STRIP" >/dev/null 2>&1; then
  "$STRIP" "$BIN_PATH" || true
fi

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
cp "$BIN_PATH" "$STAGE/velobench"

mkdir -p "$ROOT/dist"
OUT="$ROOT/dist/velobenchmark-$TAG.tar.gz"
tar -C "$STAGE" -czf "$OUT" velobench
if command -v shasum >/dev/null 2>&1; then
  ( cd "$ROOT/dist" && shasum -a 256 "$(basename "$OUT")" > "$(basename "$OUT").sha256" )
else
  ( cd "$ROOT/dist" && sha256sum "$(basename "$OUT")" > "$(basename "$OUT").sha256" )
fi

echo "==> packaged: $OUT"
ls -la "$OUT" "$OUT.sha256"
echo
echo "upload to a GitHub release (e.g. with gh):"
echo "  gh release create v<version> dist/velobenchmark-*.tar.gz dist/*.sha256 --title \"v<version>\" --notes \"...\""
