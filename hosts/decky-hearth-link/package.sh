#!/bin/sh
# Packs Hearth Link the way Decky installs it (the decky-plugin-template's layout): OUT/decky-hearth-link/ with
# dist/index.js, main.py, link_sync.py, hearth_settings_io.py (Hearth OS's safe settings writer, docs/SETTINGS-PERSISTENCE.md),
# package.json, plugin.json and LICENSE. Then OUT/decky-hearth-link.zip.
# Run after `pnpm build` (CI: .github/workflows/decky-hearth-link.yml, and the image's app job). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/decky-hearth-link"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/link_sync.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
cp "$ROOT/LICENSE" "$P/LICENSE"
cp "$ROOT/hosts/linux/desktop/shell/hearth_settings_io.py" "$P/hearth_settings_io.py"
(cd "$OUT" && rm -f decky-hearth-link.zip && zip -qr decky-hearth-link.zip decky-hearth-link)
echo "packed $P ($(du -sh "$P" | cut -f1))"
