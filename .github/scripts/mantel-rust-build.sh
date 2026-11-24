#!/bin/bash
# Builds Mantel in Rust (hosts/mantel-rs, docs/MANTEL-RUST.md) inside the Fedora edition's base image, so it links
# exactly the libraries the image has (as hearth-ui-build.sh does for Hearth's apps). Source /src (read only);
# out: /out/hearth-mantel.
set -eu
# The image's own base: Debian trixie for Hearth OS on Debian, Fedora's dnf for the archived edition (part-build-tools.sh).
. "$(dirname "$0")/part-build-tools.sh"
part_tools
echo "::notice title=Mantel in Rust build::$(rustc --version) · $(. /etc/os-release; echo "$PRETTY_NAME")"
mkdir -p /tmp/w/hosts/linux/desktop/theme /tmp/w/hosts/linux/desktop/shell /tmp/w/cargo
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
cp -a /src/hosts/hearth-ui /src/hosts/hearth-core /src/hosts/hearth-layout /src/hosts/mantel-rs /tmp/w/hosts/
cp /src/hosts/linux/desktop/theme/palettes.json /tmp/w/hosts/linux/desktop/theme/
cp /src/hosts/linux/desktop/shell/kde-settings.json /tmp/w/hosts/linux/desktop/shell/
# hearth-layout's ready-made desktops (screen.rs READY, include_str!)
cp -a /src/hosts/linux/desktop/shell/screens /tmp/w/hosts/linux/desktop/shell/
cd /tmp/w/hosts/mantel-rs
if ! cargo build --release > /tmp/w/build.log 2>&1; then
  grep -A12 '^error' /tmp/w/build.log | head -60 | while read -r l; do echo "::error title=Mantel in Rust build::$l"; done
  exit 1
fi
install -m755 target/release/hearth-mantel /out/hearth-mantel
