#!/bin/bash
# Builds Hearth's two small KWin helpers (docs/KWIN-INPUT.md) into OUT (default ./out):
#   hearth-kwin-im     the input method KWin starts (typing any character into apps' text fields)
#   hearth-kwin-cast   screen-sharing streams and screenshots from KWin, for Hearth's portal backend
# Needs a C compiler, pkg-config, wayland-scanner, libwayland-client, GLib/GIO, wayland-protocols (the
# input-method XML) and plasma-wayland-protocols (KWin's screencast XML). Used by os-bases/peppermint/builder.sh
# and .github/workflows/kwin-input.yml; never run on the owner's machine.
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:-$HERE/out}
GEN=$(mktemp -d)
trap 'rm -rf "$GEN"' EXIT
mkdir -p "$OUT"
find_xml() {   # the first of these that exists
  for f in "$@"; do [ -f "$f" ] && { echo "$f"; return 0; }; done
  echo "build.sh: none of $* is installed" >&2; return 1
}
WP=$(pkg-config --variable=pkgdatadir wayland-protocols 2>/dev/null || echo /usr/share/wayland-protocols)
IM_XML=$(find_xml "$WP/unstable/input-method/input-method-unstable-v1.xml" /usr/share/wayland-protocols/unstable/input-method/input-method-unstable-v1.xml)
PWP=$(pkg-config --variable=pkgdatadir plasma-wayland-protocols 2>/dev/null || true)
CAST_XML=$(find_xml "${PWP:-/nonexistent}/screencast.xml" /usr/share/plasma-wayland-protocols/screencast.xml)
for x in "$IM_XML:input-method-unstable-v1" "$CAST_XML:screencast"; do
  xml=${x%%:*}; base=${x##*:}
  wayland-scanner client-header "$xml" "$GEN/$base-client-protocol.h"
  wayland-scanner private-code "$xml" "$GEN/$base-protocol.c"
done
CFLAGS="-O2 -g -Wall -Wextra -Werror=implicit-function-declaration -Werror=incompatible-pointer-types -D_FORTIFY_SOURCE=2 -fstack-protector-strong"
cc $CFLAGS -I"$GEN" -o "$OUT/hearth-kwin-im" "$HERE/hearth-kwin-im.c" "$GEN/input-method-unstable-v1-protocol.c" \
  $(pkg-config --cflags --libs wayland-client)
cc $CFLAGS -I"$GEN" -o "$OUT/hearth-kwin-cast" "$HERE/hearth-kwin-cast.c" "$GEN/screencast-protocol.c" \
  $(pkg-config --cflags --libs wayland-client gio-2.0 gio-unix-2.0) -lpthread
ls -l "$OUT/hearth-kwin-im" "$OUT/hearth-kwin-cast"
