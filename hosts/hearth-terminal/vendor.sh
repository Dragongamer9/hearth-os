#!/bin/sh
# Puts the published xterm.js files (and the JetBrains Mono font) into ui/vendor, at the pinned versions below.
# They're downloaded from the npm registry as released (no npm, no build step) and checked against the
# registry's sha512. The terminal workflow runs this with "vendor" ticked and commits the result; nothing
# downloads anything when the app is built.
set -eu
cd "$(dirname "$0")"
out=ui/vendor
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# package  version  files to keep (paths inside the package)
PINS='
@xterm/xterm 5.5.0 lib/xterm.js css/xterm.css LICENSE
@xterm/addon-fit 0.10.0 lib/addon-fit.js
@xterm/addon-web-links 0.11.0 lib/addon-web-links.js
@xterm/addon-unicode11 0.8.0 lib/addon-unicode11.js
@xterm/addon-search 0.15.0 lib/addon-search.js
@xterm/addon-webgl 0.18.0 lib/addon-webgl.js
@xterm/addon-canvas 0.7.0 lib/addon-canvas.js
@fontsource/jetbrains-mono 5.1.1 files/jetbrains-mono-latin-400-normal.woff2 files/jetbrains-mono-latin-700-normal.woff2 files/jetbrains-mono-latin-ext-400-normal.woff2 files/jetbrains-mono-latin-ext-700-normal.woff2 LICENSE
'

mkdir -p "$out"
: > "$tmp/versions"
echo "$PINS" | while read -r pkg ver files; do
  [ -n "$pkg" ] || continue
  name=${pkg##*/}
  meta=$(curl -fsSL "https://registry.npmjs.org/$(echo "$pkg" | sed 's#/#%2f#')/$ver")
  url=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["tarball"])')
  sri=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["integrity"])')
  curl -fsSL "$url" -o "$tmp/$name.tgz"
  got="sha512-$(openssl dgst -sha512 -binary "$tmp/$name.tgz" | base64 -w0)"
  [ "$got" = "$sri" ] || { echo "$pkg@$ver: checksum doesn't match the registry's"; exit 1; }
  mkdir -p "$tmp/$name" && tar -xzf "$tmp/$name.tgz" -C "$tmp/$name"
  for f in $files; do
    base=$(basename "$f")
    [ "$base" = LICENSE ] && base="LICENSE-$name"
    cp "$tmp/$name/package/$f" "$out/$base"
  done
  echo "$pkg@$ver $sri" >> "$tmp/versions"
done
cp "$tmp/versions" "$out/VERSIONS"
ls -la "$out"
