#!/bin/bash
# Builds Hearth's Game Mode host (hosts/hearth-rs, the `hearth` program the image installs as /usr/lib/nimbus/nimbus)
# for a Hearth Store part (quick-update.yml, 1.18: every Hearth part from the Store), inside Debian trixie as
# base-peppermint.yml builds it, so it links the WebKitGTK the image has. Its version is the Hearth OS one, as in the
# image. Source /src (read only); out: /out/nimbus.
set -eu
. "$(dirname "$0")/part-build-tools.sh"
part_tools libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev libssl-dev libxdo-dev file
mkdir -p /tmp/w/src
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
# hearth-rs includes the UI (ui/) and files from hosts/hearth-core, hosts/linux and os/ by path: the whole tree.
tar -C /src --exclude=.git --exclude='*/target' -cf - . | tar -C /tmp/w/src -xf -
cd /tmp/w/src/hosts/hearth-rs
v=$(grep -oP '(?<=<Version>)[^<]+' ../windows/Nimbus.Host/Nimbus.Host.csproj | head -1)
[ -n "$v" ] && sed -i "s/^version = \".*\"/version = \"$v\"/" Cargo.toml
if ! cargo build --release > /tmp/w/build.log 2>&1; then
  grep -A12 '^error' /tmp/w/build.log | head -60 | while read -r l; do echo "::error title=Game Mode host build::$l"; done
  exit 1
fi
install -m755 target/release/hearth /out/nimbus
