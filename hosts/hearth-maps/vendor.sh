#!/bin/sh
# Puts MapLibre GL JS's published build (maplibre-gl.js, maplibre-gl.css and its licence) into ui/vendor, at the
# pinned version below. Downloaded from the npm registry as released (no npm, no build step) and checked against
# the registry's sha512. The maps workflow runs this with "vendor" ticked and commits the result; nothing is
# downloaded when the app is built.
set -eu
cd "$(dirname "$0")"
out=ui/vendor
pkg=maplibre-gl
ver=4.7.1
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
meta=$(curl -fsSL "https://registry.npmjs.org/$pkg/$ver")
url=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["tarball"])')
sri=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["integrity"])')
curl -fsSL "$url" -o "$tmp/p.tgz"
got="sha512-$(openssl dgst -sha512 -binary "$tmp/p.tgz" | base64 -w0)"
[ "$got" = "$sri" ] || { echo "$pkg@$ver: checksum doesn't match the registry's"; exit 1; }
tar -xzf "$tmp/p.tgz" -C "$tmp"
mkdir -p "$out"
cp "$tmp/package/dist/maplibre-gl.js" "$tmp/package/dist/maplibre-gl.css" "$out/"
for l in LICENSE.txt LICENSE; do [ -f "$tmp/package/$l" ] && cp "$tmp/package/$l" "$out/LICENSE-maplibre-gl.txt" && break; done
echo "$pkg@$ver $sri" > "$out/VERSIONS"
ls -la "$out"
