#!/bin/sh
# Packs Hearth Classic the way Decky installs it: OUT/decky-hearth-look/ with dist/index.js, main.py, package.json,
# plugin.json, LICENSE, the shared Hearth OS check (hearth_only.py), the skin (hearth-classic.css) and Hearth's logo and
# wallpaper in assets/, then OUT/decky-hearth-look.zip. Run after `pnpm build` (decky-hearth-look.yml and the plugins
# release, .github/workflows/decky-plugins-release.yml). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
NAME="$(basename "$HERE")"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/$NAME"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist" "$P/assets"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
cp "$ROOT/LICENSE" "$P/LICENSE"
# Hearth OS only and the load mark for plugin updates (docs/PLUGIN-UPDATES.md), shared by every Hearth plugin.
cp "$ROOT/hosts/decky-shared/hearth_only.py" "$P/hearth_only.py"
# The skin and Hearth's own pictures (the old Hearth Game Mode's logo and its default wallpaper).
cp "$HERE/defaults/hearth-classic.css" "$P/hearth-classic.css"
cp "$ROOT/site/img/logo.svg" "$P/assets/logo.svg"
cp "$ROOT/os/branding/wallpapers/hearth-embers.jpg" "$P/assets/wallpaper.jpg"
(cd "$OUT" && rm -f "$NAME.zip" && zip -qr "$NAME.zip" "$NAME")
echo "packed $P ($(du -sh "$P" | cut -f1))"
