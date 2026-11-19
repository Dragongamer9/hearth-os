#!/bin/bash
# Builds Hearth's pure-Rust apps (hosts/hearth-ui: Task Manager, System Restore, Reliability history) inside the
# Fedora edition's base image (base-anatase.yml "hearth-ui"; docs/APPS-ANATASE.md), so they link exactly the libraries
# the image has. The source is /src (read only); out: /out/hearth-ui.tar.
set -eu
DNF=$(command -v dnf5 || command -v dnf)
$DNF -y install gcc pkgconf-pkg-config rust cargo libxkbcommon-devel wayland-devel fontconfig-devel freetype-devel >/dev/null
echo "::notice title=Hearth's pure-Rust apps build::$(rustc --version) · $(. /etc/os-release; echo "$PRETTY_NAME")"
mkdir -p /tmp/w/hosts/linux/desktop/theme /tmp/w/cargo
# A bootc base's /root is a link to /var/roothome, which isn't there in a plain container: cargo's home goes to /tmp.
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
# hearth-ui includes hearth-core's settings_file.rs by path, and carries palettes.json as its built-in look.
cp -a /src/hosts/hearth-ui /src/hosts/hearth-core /tmp/w/hosts/
cp /src/hosts/linux/desktop/theme/palettes.json /tmp/w/hosts/linux/desktop/theme/
cd /tmp/w/hosts/hearth-ui
if ! cargo build --release --workspace > /tmp/w/build.log 2>&1; then
  # The first errors as annotations (the build's log can't be read from here).
  awk '/^error/{if (b) print b; b=$0; n=0; next} b && n < 14 {b=b "|" $0; n++} END{if (b) print b}' /tmp/w/build.log | head -12 | while read -r l; do
    echo "::error title=Hearth's pure-Rust apps build::$(echo "$l" | cut -c1-3500)"
  done
  grep -q '^error' /tmp/w/build.log || echo "::error title=Hearth's pure-Rust apps build::$(tail -c 3000 /tmp/w/build.log | tr '\n' '|')"
fi
mkdir -p /tmp/w/out
for b in hearth-task-manager hearth-system-restore hearth-reliability-history hearth-settings-app hearth-files-app hearth-file-chooser hearth-capture-app hearth-g15 hearth-g15-app; do
  test -x "target/release/$b" || { echo "::error title=Hearth's pure-Rust apps build::$b didn't build"; exit 1; }
  cp "target/release/$b" /tmp/w/out/
done
tar -C /tmp/w/out -cf /out/hearth-ui.tar .
ls -la /tmp/w/out
