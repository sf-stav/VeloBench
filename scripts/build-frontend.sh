#!/usr/bin/env bash
# Build the Angular frontend and copy its output into the embedded assets/ dir.
# Run from anywhere; resolves paths relative to the repo root. After this,
# `cargo build --release` produces the full single binary.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/frontend"

echo "==> generating protobuf client types from proto/velobench.proto"
npx pbjs -t static-module -w es6 -o src/app/proto/velobench.js ../proto/velobench.proto
npx pbts -o src/app/proto/velobench.d.ts src/app/proto/velobench.js

echo "==> building Angular frontend (production)"
npx ng build --configuration production

SRC="dist/velobench/browser"
DEST="$ROOT/assets"

if [[ ! -d "$SRC" ]]; then
  echo "error: build output not found at $SRC" >&2
  exit 1
fi

echo "==> copying build output -> $DEST"
mkdir -p "$DEST"
# Remove stale frontend build artifacts so the embedded binary doesn't bloat
# with accumulated hashed bundles. Preserve non-build assets (e.g. payloads/).
rm -f "$DEST"/main-*.js "$DEST"/polyfills-*.js "$DEST"/chunk-*.js \
      "$DEST"/styles-*.css "$DEST"/3rdpartylicenses.txt \
      "$DEST"/prerendered-routes.json
# Copy the build output, but never clobber a user-provided favicon in assets/.
tar -C "$SRC" --exclude='./favicon.ico' -cf - . | tar -C "$DEST" -xf -
[ -f "$DEST/favicon.ico" ] || cp "$SRC/favicon.ico" "$DEST/favicon.ico" 2>/dev/null || true

# The embedded assets/ dir also holds static payloads (payloads/lorem.txt) that
# live outside the Angular bundle; cp preserves them.
echo "==> done. assets/ now contains:"
ls -1 "$DEST"
