#!/bin/sh
# Packs this plugin the way Decky installs it: OUT/<folder>/ with dist/index.js, main.py, package.json, plugin.json,
# LICENSE and the shared Hearth OS check (hearth_only.py), then OUT/<folder>.zip. Run after `pnpm build`
# (the plugins release, .github/workflows/decky-plugins-release.yml). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NAME="$(basename "$HERE")"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/$NAME"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
cp "$ROOT/LICENSE" "$P/LICENSE"
# Hearth OS only and the load mark for plugin updates (docs/PLUGIN-UPDATES.md), shared by every Hearth plugin.
cp "$ROOT/hosts/decky-shared/hearth_only.py" "$P/hearth_only.py"
(cd "$OUT" && rm -f "$NAME.zip" && zip -qr "$NAME.zip" "$NAME")
echo "packed $P ($(du -sh "$P" | cut -f1))"
