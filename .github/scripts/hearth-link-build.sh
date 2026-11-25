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
# It uses hearth-core, which reads a few files from beside it (include_str!): those come along at their own paths.
cp -a /src/hosts/hearth-core /tmp/w/hosts/
mkdir -p /tmp/w/hosts/linux /tmp/w/os/system_files/usr/share/hearth
for d in desktop session test; do cp -a "/src/hosts/linux/$d" /tmp/w/hosts/linux/; done
cp -a /src/os/system_files/usr/share/hearth/recomps.json /tmp/w/os/system_files/usr/share/hearth/
rm -rf /tmp/w/hosts/hearth-core/target /tmp/w/hosts/hearth-link/target
cd /tmp/w/hosts/hearth-link
if ! cargo build --release > /tmp/w/build.log 2>&1; then
  grep -A12 '^error' /tmp/w/build.log | head -60 | while read -r l; do echo "::error title=Hearth Link build::$l"; done
  exit 1
fi
install -m755 target/release/hearth-link /out/hearth-link
