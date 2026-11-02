#!/bin/sh
# Packs Hearth Music the way Decky installs it (the decky-plugin-template's layout): OUT/decky-hearth-music/ with
# dist/index.js, main.py, package.json, plugin.json and LICENSE. Then OUT/decky-hearth-music.zip.
# Run after `pnpm build` (CI: .github/workflows/decky-hearth-music.yml, and the image's app job). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/decky-hearth-music"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
cp "$ROOT/LICENSE" "$P/LICENSE"
(cd "$OUT" && rm -f decky-hearth-music.zip && zip -qr decky-hearth-music.zip decky-hearth-music)
echo "packed $P ($(du -sh "$P" | cut -f1))"
