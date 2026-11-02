#!/bin/sh
# Packs the built plugin the way Decky installs it (the decky-plugin-template's layout): OUT/decky-hearth/ with
# dist/index.js, main.py, package.json, plugin.json, LICENSE, the files from defaults/ at its top (the theme), Hearth's
# logo and wallpaper in assets/, and a CSS Loader theme (the stable part of the look). Then OUT/decky-hearth.zip.
# Run after `pnpm build` (CI: .github/workflows/decky-hearth.yml, and the images' app job). Usage: package.sh OUT
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OUT="${1:?usage: package.sh OUT}"
P="$OUT/decky-hearth"
test -s "$HERE/dist/index.js"
rm -rf "$P" && mkdir -p "$P/dist" "$P/assets"
cp "$HERE/dist/index.js" "$P/dist/"
cp "$HERE/main.py" "$HERE/package.json" "$HERE/plugin.json" "$P/"
cp "$ROOT/LICENSE" "$P/LICENSE"
cp -r "$HERE/defaults/." "$P/"
cp "$ROOT/site/img/logo.svg" "$P/assets/logo.svg"
cp "$ROOT/os/branding/wallpapers/hearth-embers.jpg" "$P/assets/wallpaper.jpg"
# CSS Loader (if you use it instead of the plugin's switch): layers 1 and 2 only, which need no class names.
mkdir -p "$P/css-loader/Hearth"
grep -v '{{' "$HERE/defaults/hearth-theme.css" | grep -v '%WALLPAPER%\|%LOGO%' > "$P/css-loader/Hearth/shared.css"
cp "$HERE/defaults/css-loader/Hearth/theme.json" "$P/css-loader/Hearth/theme.json"
rm -rf "$P/css-loader/Hearth/.keep" "$P/css-loader/css-loader" 2>/dev/null || true
(cd "$OUT" && rm -f decky-hearth.zip && zip -qr decky-hearth.zip decky-hearth)
echo "packed $P ($(du -sh "$P" | cut -f1))"
