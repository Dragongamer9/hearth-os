#!/bin/sh
# Puts the published pdf.js (PDF), epub.js and JSZip (EPUB) files into ui/vendor, at the pinned versions below.
# They're downloaded from the npm registry as released (no npm, no build step) and checked against the registry's
# sha512. The documents workflow runs this when ui/vendor/VERSIONS doesn't match these pins and commits the result;
# nothing downloads anything when the app is built or run.
set -eu
cd "$(dirname "$0")"
out=ui/vendor
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# package version (pdf.js's "legacy" build runs on older web views too)
PINS='
pdfjs-dist 4.10.38
epubjs 0.3.93
jszip 3.10.1
'

rm -rf "$out"
mkdir -p "$out/pdfjs" "$out/epub"
: > "$tmp/versions"
echo "$PINS" | while read -r pkg ver; do
  [ -n "$pkg" ] || continue
  meta=$(curl -fsSL "https://registry.npmjs.org/$pkg/$ver")
  url=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["tarball"])')
  sri=$(printf '%s' "$meta" | python3 -c 'import json,sys; print(json.load(sys.stdin)["dist"]["integrity"])')
  curl -fsSL "$url" -o "$tmp/$pkg.tgz"
  got="sha512-$(openssl dgst -sha512 -binary "$tmp/$pkg.tgz" | base64 -w0)"
  [ "$got" = "$sri" ] || { echo "$pkg@$ver: checksum doesn't match the registry's"; exit 1; }
  mkdir -p "$tmp/$pkg" && tar -xzf "$tmp/$pkg.tgz" -C "$tmp/$pkg"
  P="$tmp/$pkg/package"
  case "$pkg" in
    pdfjs-dist)
      cp "$P/legacy/build/pdf.min.mjs" "$P/legacy/build/pdf.worker.min.mjs" "$out/pdfjs/"
      cp "$P/legacy/web/pdf_viewer.mjs" "$P/legacy/web/pdf_viewer.css" "$out/pdfjs/"
      if [ -d "$P/legacy/web/images" ]; then cp -r "$P/legacy/web/images" "$out/pdfjs/images"; elif [ -d "$P/web/images" ]; then cp -r "$P/web/images" "$out/pdfjs/images"; fi
      cp -r "$P/cmaps" "$out/pdfjs/cmaps"
      cp -r "$P/standard_fonts" "$out/pdfjs/standard_fonts"
      [ -d "$P/wasm" ] && cp -r "$P/wasm" "$out/pdfjs/wasm" || true
      cp "$P/LICENSE" "$out/pdfjs/LICENSE" ;;
    epubjs)
      cp "$P/dist/epub.min.js" "$out/epub/"
      cp "$P/LICENSE" "$out/epub/LICENSE-epubjs" 2>/dev/null || cp "$P/license" "$out/epub/LICENSE-epubjs" 2>/dev/null || true ;;
    jszip)
      cp "$P/dist/jszip.min.js" "$out/epub/"
      cp "$P/LICENSE.markdown" "$out/epub/LICENSE-jszip" 2>/dev/null || true ;;
  esac
  echo "$pkg@$ver $sri" >> "$tmp/versions"
done
cp "$tmp/versions" "$out/VERSIONS"
find "$out" -maxdepth 2 | sort | head -80
du -sh "$out"
