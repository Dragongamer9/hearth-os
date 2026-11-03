#!/bin/sh
# Packs Hearth Sync the way Decky installs it (the decky-plugin-template's layout): OUT/decky-hearth-sync/ with
# dist/index.js, main.py, package.json, plugin.json and LICENSE. Then OUT/decky-hearth-sync.zip.
# Run after `pnpm build` (CI: .github/workflows/decky-hearth-sync.yml, and the image's app job). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/decky-hearth-sync"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
# The Steam import (docs/HEARTH-SYNC.md): the system's hearth-steam-import, as a module next to main.py.
cp "$ROOT/os/system_files/usr/bin/hearth-steam-import" "$P/steam_import.py"
cp "$ROOT/LICENSE" "$P/LICENSE"
# Hearth OS only and the load mark for plugin updates (docs/PLUGIN-UPDATES.md), shared by the three plugins.
cp "$ROOT/hosts/decky-shared/hearth_only.py" "$P/hearth_only.py"
(cd "$OUT" && rm -f decky-hearth-sync.zip && zip -qr decky-hearth-sync.zip decky-hearth-sync)
echo "packed $P ($(du -sh "$P" | cut -f1))"
