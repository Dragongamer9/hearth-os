#!/bin/bash
# Builds one Hearth program for a Hearth Store part (quick-update.yml, 1.18: every Hearth part from the Store) inside the
# image's own base (Debian trixie), as base-peppermint.yml builds it, with its version set to the Hearth OS one as in the
# image. Usage: cargo-part-build.sh <crate folder under hosts/> <program> [webkit]; source /src (read only); out:
# /out/<program>. "webkit": the Tauri apps' libraries (Hearth's gaming apps).
set -eu
crate="${1:?crate}" prog="${2:?program}"
. "$(dirname "$0")/part-build-tools.sh"
if [ "${3:-}" = webkit ]; then
  part_tools libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libssl-dev libxdo-dev file
else
  part_tools
fi
mkdir -p /tmp/w/src
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
tar -C /src --exclude=.git --exclude='*/target' -cf - . | tar -C /tmp/w/src -xf -
cd "/tmp/w/src/hosts/$crate"
v=$(grep -oP '(?<=<Version>)[^<]+' /tmp/w/src/hosts/windows/Nimbus.Host/Nimbus.Host.csproj | head -1)
[ -n "$v" ] && sed -i "s/^version = \".*\"/version = \"$v\"/" Cargo.toml
if ! cargo build --release > /tmp/w/build.log 2>&1; then
  grep -A12 '^error' /tmp/w/build.log | head -60 | while read -r l; do echo "::error title=$prog build::$l"; done
  exit 1
fi
install -m755 "target/release/$prog" "/out/$prog"
