#!/bin/sh
# Puts the published, already-built library files Hearth Office uses into ui/vendor, at the pinned versions below.
# They're downloaded as released (no npm, no build step) and checked: npm packages against the registry's sha512,
# SheetJS (not on npm any more; its own CDN) against the sha256 pinned here. The office workflow runs this with
# "vendor" ticked and commits the result; nothing downloads anything when the app is built or run.
#   ONLY=npm sh vendor.sh     just the npm packages      ONLY=sheetjs sh vendor.sh     just SheetJS
set -eu
cd "$(dirname "$0")"
out=ui/vendor
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
ONLY=${ONLY:-all}

# package  version  files to keep (paths inside the package)      licence
#   mammoth       BSD-2-Clause   .docx → HTML (Write's fallback reader)
#   docx          MIT            writes .docx (Write)
#   hyperformula  GPL-3.0-only   the formula engine (Sheets); GPL-3.0 like Hearth
#   pptxgenjs     MIT            writes .pptx (Slides)
#   jszip         MIT            reads and writes the zip inside .docx/.odt/.xlsx/.pptx
#   dompurify     Apache-2.0     cleans HTML that comes from files (no scripts get into the page)
#   marked        MIT            Markdown → HTML;  turndown (MIT): HTML → Markdown
#   typo-js       BSD-3-Clause   spell checking with Hunspell dictionaries
#   dictionary-en MIT AND BSD    Hunspell en_US (SCOWL)
PINS='
mammoth 1.13.0 mammoth.browser.min.js LICENSE
docx 9.8.1 dist/index.iife.js LICENSE
hyperformula 3.4.0 dist/hyperformula.full.min.js LICENSE
pptxgenjs 4.0.1 dist/pptxgen.bundle.js LICENSE
jszip 3.10.1 dist/jszip.min.js LICENSE
dompurify 3.4.16 dist/purify.min.js LICENSE
marked 18.0.14 lib/marked.umd.js LICENSE
turndown 7.2.4 lib/turndown.browser.umd.js LICENSE
typo-js 1.3.2 typo.js LICENSE
dictionary-en 4.0.0 index.aff index.dic LICENSE
'

# SheetJS Community Edition (Apache-2.0): reads and writes .xlsx, .xls, .ods and .csv (Sheets).
SHEETJS_VER=0.20.3
SHEETJS_SHA256=${SHEETJS_SHA256:-}

: > "$tmp/versions"
[ -f "$out/VERSIONS" ] && cp "$out/VERSIONS" "$tmp/old"
if [ "$ONLY" = all ] || [ "$ONLY" = npm ]; then
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
    case "$base" in
      LICENSE*|license*) base="LICENSE-$name" ;;
      index.aff) base="en_US.aff" ;;
      index.dic) base="en_US.dic" ;;
      index.iife.js) base="docx.iife.js" ;;
    esac
    # Licence files: whichever the package has (their names vary between versions).
    case "$base" in LICENSE-*) lic=$(ls "$tmp/$name/package" | grep -i '^licen[cs]e' | head -1); [ -n "$lic" ] && cp "$tmp/$name/package/$lic" "$out/$base"; continue ;; esac
    cp "$tmp/$name/package/$f" "$out/$base"
  done
  echo "$pkg@$ver $sri" >> "$tmp/versions"
done
else
  [ -f "$tmp/old" ] && grep -v '^xlsx@' "$tmp/old" >> "$tmp/versions" || true
fi

if [ "$ONLY" = all ] || [ "$ONLY" = sheetjs ]; then
  curl -fsSL "https://cdn.sheetjs.com/xlsx-$SHEETJS_VER/xlsx-$SHEETJS_VER.tgz" -o "$tmp/xlsx.tgz"
  got=$(openssl dgst -sha256 -r "$tmp/xlsx.tgz" | cut -d' ' -f1)
  if [ -n "$SHEETJS_SHA256" ] && [ "$got" != "$SHEETJS_SHA256" ]; then echo "SheetJS $SHEETJS_VER: checksum $got isn't the pinned one"; exit 1; fi
  mkdir -p "$tmp/xlsx" && tar -xzf "$tmp/xlsx.tgz" -C "$tmp/xlsx"
  cp "$tmp/xlsx/package/dist/xlsx.full.min.js" "$out/xlsx.full.min.js"
  cp "$tmp/xlsx/package/LICENSE" "$out/LICENSE-sheetjs"
  echo "xlsx@$SHEETJS_VER sha256:$got (cdn.sheetjs.com)" >> "$tmp/versions"
else
  [ -f "$tmp/old" ] && grep '^xlsx@' "$tmp/old" >> "$tmp/versions" || true
fi
sort -o "$tmp/versions" "$tmp/versions"
cp "$tmp/versions" "$out/VERSIONS"
ls -la "$out"
