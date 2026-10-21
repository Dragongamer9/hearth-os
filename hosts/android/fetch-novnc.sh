#!/usr/bin/env bash
# Hearth Linux stage 2: puts noVNC (the VNC client the Linux desktop is shown with, inside Hearth's page) into the
# app's assets at app/src/main/assets/ui/novnc/{core,vendor/pako} (served as /ui/novnc/… next to Hearth's page), from
# a pinned release checked twice: the tag must be the pinned commit, and the files taken must match one SHA-256 over
# all of them. CI runs it before building (.github/workflows/android.yml); run it yourself before a local build. The
# files are never kept in git.
#
# noVNC: https://github.com/novnc/noVNC, MPL-2.0 (pako, in vendor/pako: MIT). Licenses: app/src/main/assets/licenses/
# (noVNC-LICENSE.txt, MPL-2.0.txt, pako-LICENSE.txt) and docs/ANDROID-LINUX.md.
set -euo pipefail

TAG=v1.7.0
COMMIT=63107bd06d9e1f6136ff21aeda8cd62cbf0d433e
# sha256sum of every file taken (core/**, vendor/pako/**), sorted by path, then sha256sum of that list.
TREE_SHA256=0e61b932064ec6ecffa0a4e4443fc2f447e7f1d1452ec2070f35c029f60bc334

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/app/src/main/assets/ui/novnc"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

fail() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error title=noVNC for Hearth Linux::$1"; else echo "error: $1" >&2; fi
  exit 1
}

git -c advice.detachedHead=false clone -q --depth 1 --branch "$TAG" https://github.com/novnc/noVNC.git "$work/src" \
  || fail "couldn't download noVNC $TAG"
got="$(git -C "$work/src" rev-parse HEAD)"
[ "$got" = "$COMMIT" ] || fail "noVNC $TAG is commit $got, expected $COMMIT. The tag moved: don't build with it."

stage="$work/novnc"
mkdir -p "$stage/vendor"
cp -R "$work/src/core" "$stage/core"
cp -R "$work/src/vendor/pako" "$stage/vendor/pako"
sum="$(cd "$stage" && find core vendor/pako -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d' ' -f1)"
[ "$sum" = "$TREE_SHA256" ] || fail "noVNC $TAG's files: SHA-256 is $sum, expected $TREE_SHA256"
[ -f "$stage/core/rfb.js" ] || fail "noVNC $TAG has no core/rfb.js"

rm -rf "$out"
mkdir -p "$(dirname "$out")"
cp -R "$stage" "$out"
echo "noVNC $TAG ($COMMIT): $(find "$out" -type f | wc -l) files in $out"
