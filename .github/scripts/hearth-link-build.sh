#!/bin/bash
# Builds Hearth Link (hosts/hearth-link) inside the Fedora edition's base image, for a Hearth Store part (quick-update.yml;
# the owner, Nov 23: everything Hearth makes updatable from the Store). Source /src (read only); out: /out/hearth-link.
set -eu
# The image's own base: Debian trixie for Hearth OS on Debian, Fedora's dnf for the archived edition (part-build-tools.sh).
. "$(dirname "$0")/part-build-tools.sh"
part_tools
mkdir -p /tmp/w/hosts /tmp/w/cargo
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
cp -a /src/hosts/hearth-link /tmp/w/hosts/
cd /tmp/w/hosts/hearth-link
if ! cargo build --release > /tmp/w/build.log 2>&1; then
  grep -A12 '^error' /tmp/w/build.log | head -60 | while read -r l; do echo "::error title=Hearth Link build::$l"; done
  exit 1
fi
install -m755 target/release/hearth-link /out/hearth-link
