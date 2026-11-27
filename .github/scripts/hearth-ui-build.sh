#!/bin/bash
# Builds Hearth's pure-Rust apps (hosts/hearth-ui: Task Manager, System Restore, Reliability history) inside the
# Fedora edition's base image (base-anatase.yml "hearth-ui"; docs/APPS-ANATASE.md), so they link exactly the libraries
# the image has. The source is /src (read only); out: /out/hearth-ui.tar.
set -eu
# The image's own base: Debian trixie for Hearth OS on Debian, Fedora's dnf for the archived edition (part-build-tools.sh).
. "$(dirname "$0")/part-build-tools.sh"
part_tools
echo "::notice title=Hearth's pure-Rust apps build::$(rustc --version) · $(. /etc/os-release; echo "$PRETTY_NAME")"
mkdir -p /tmp/w/hosts/linux/desktop/theme /tmp/w/cargo
# A bootc base's /root is a link to /var/roothome, which isn't there in a plain container: cargo's home goes to /tmp.
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
# hearth-ui includes hearth-core's settings_file.rs by path, and carries palettes.json as its built-in look.
cp -a /src/hosts/hearth-ui /src/hosts/hearth-core /src/hosts/hearth-layout /tmp/w/hosts/
cp /src/hosts/linux/desktop/theme/palettes.json /tmp/w/hosts/linux/desktop/theme/
# Settings' KDE pages (cats/kde.rs draws them from the same file hearth-kde-settings checks against).
mkdir -p /tmp/w/hosts/linux/desktop/shell && cp /src/hosts/linux/desktop/shell/kde-settings.json /tmp/w/hosts/linux/desktop/shell/
# hearth-layout carries Mantel's ready-made desktops (screen.rs READY: one copy for both Mantels).
cp -a /src/hosts/linux/desktop/shell/screens /tmp/w/hosts/linux/desktop/shell/
# Passwords, Cloud and Home carry the servers' helper and its user services (oss-kit server.rs), so they update with
# the apps from the Store.
cp /src/hosts/linux/desktop/shell/hearth-oss-server /tmp/w/hosts/linux/desktop/shell/
mkdir -p /tmp/w/os/system_files/usr/lib/systemd/user
cp /src/os/system_files/usr/lib/systemd/user/hearth-vaultwarden.* /src/os/system_files/usr/lib/systemd/user/hearth-nextcloud.* \
  /src/os/system_files/usr/lib/systemd/user/hearth-home-assistant.* /tmp/w/os/system_files/usr/lib/systemd/user/
cd /tmp/w/hosts/hearth-ui
if ! cargo build --release --workspace > /tmp/w/build.log 2>&1; then
  # The first errors as annotations (the build's log can't be read from here).
  awk '/^error/{if (b) print b; b=$0; n=0; next} b && n < 14 {b=b "|" $0; n++} END{if (b) print b}' /tmp/w/build.log | head -12 | while read -r l; do
    echo "::error title=Hearth's pure-Rust apps build::$(echo "$l" | cut -c1-3500)"
  done
  grep -q '^error' /tmp/w/build.log || echo "::error title=Hearth's pure-Rust apps build::$(tail -c 3000 /tmp/w/build.log | tr '\n' '|')"
fi
# Desktop layouts (hosts/hearth-layout): its command, which both Mantels' Settings use (Settings' Rust page uses its library)
if ! cargo build --release --manifest-path /tmp/w/hosts/hearth-layout/Cargo.toml > /tmp/w/build-layout.log 2>&1; then
  grep -A8 '^error' /tmp/w/build-layout.log | head -40 | tr '\n' '|' | cut -c1-3500 | while read -r l; do echo "::error title=hearth-layout build::$l"; done
fi
mkdir -p /tmp/w/out
cp /tmp/w/hosts/hearth-layout/target/release/hearth-layout /tmp/w/out/ || { echo "::error title=hearth-layout build::hearth-layout didn't build"; exit 1; }
for b in hearth-task-manager hearth-system-restore hearth-reliability-history hearth-settings-app hearth-files-app hearth-file-chooser hearth-capture-app hearth-pad-host hearth-g15 hearth-g15-app hearth-store-app hearth-server-app hearth-resmon-app hearth-family-app hearth-notes-app hearth-game-fixer hearth-proton-manager hearth-overlay hearth-bottles hearth-3d hearth-ideas-app hearth-retro-app hearth-retro-to-steam-app hearth-studio-app hearth-beat-maker-app hearth-lights-app hearth-remap-app hearth-mouse-app hearth-shield-app hearth-listen-app hearth-imagine-app hearth-passwords-app hearth-cloud-app hearth-home-app; do
  test -x "target/release/$b" || { echo "::error title=Hearth's pure-Rust apps build::$b didn't build"; exit 1; }
  cp "target/release/$b" /tmp/w/out/
done
tar -C /tmp/w/out -cf /out/hearth-ui.tar .
ls -la /tmp/w/out
