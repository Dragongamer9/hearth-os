#!/usr/bin/env bash
# Build an upgrade package installable on the original writable Mint Hearth ISO.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${1:-$ROOT/hosts/hearth-rs/target/release/hearth}"
OUT="${2:-$ROOT/dist}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/hosts/hearth-rs/Cargo.toml" | head -1)"
PACKAGE_VERSION="${VERSION}-7"
test -x "$BIN"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
ARCH="$(dpkg --print-architecture)"
test "$ARCH" = amd64 || { echo 'Only amd64 is supported for this first test.' >&2; exit 1; }
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
install -Dm755 "$BIN" "$STAGE/usr/bin/hearth"
install -Dm755 "$ROOT/scripts/hearth-upgrade.sh" "$STAGE/usr/bin/hearth-upgrade"
install -Dm755 "$ROOT/scripts/hearth-admin-center.py" "$STAGE/usr/bin/hearth-admin-center"
install -Dm755 "$ROOT/scripts/hearth-admin-report.py" "$STAGE/usr/bin/hearth-admin-report"
install -Dm755 "$ROOT/scripts/hearth-admin-branding.sh" "$STAGE/usr/bin/hearth-admin-branding"
install -Dm755 "$ROOT/scripts/hearth-admin-desktop.sh" "$STAGE/usr/bin/hearth-admin-desktop"
install -Dm755 "$ROOT/scripts/hearth-welcome.py" "$STAGE/usr/bin/hearth-welcome"
install -Dm755 "$ROOT/scripts/hearth-convert.sh" "$STAGE/usr/bin/hearth-convert"
install -Dm755 "$ROOT/scripts/hearth-session.sh" "$STAGE/usr/bin/hearth-session"
install -Dm644 "$ROOT/branding/hearth-admin-wallpaper.svg" "$STAGE/usr/share/backgrounds/hearth-admin-edition.svg"
install -d "$STAGE/etc"
printf "NAME=\"Hearth OS Admin Edition\"\\nVERSION=\"3.1 experimental\"\\nBASE=\"Linux Mint 22.3\"\\n" > "$STAGE/etc/hearth-admin-release"
install -d "$STAGE/usr/share/applications" "$STAGE/DEBIAN"
cat > "$STAGE/usr/share/applications/hearth.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth OS
Comment=Hearth gaming interface
Exec=/usr/bin/hearth --windowed
Icon=applications-games
Terminal=false
Categories=Game;
DESKTOP
cat > "$STAGE/usr/share/applications/hearth-admin-center.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth Admin Center
Comment=Read-only system information and recovery diagnostics
Exec=/usr/bin/hearth-admin-center
Icon=utilities-system-monitor
Terminal=false
Categories=System;Utility;
DESKTOP
cat > "$STAGE/usr/share/applications/hearth-admin-desktop.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth Admin Desktop Setup
Comment=Apply Hearth desktop wallpaper (reversible)
Exec=/usr/bin/hearth-admin-desktop apply
Icon=preferences-desktop-wallpaper
Terminal=true
Categories=Settings;DesktopSettings;
DESKTOP
cat > "$STAGE/usr/share/applications/hearth-welcome.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth OS Admin Edition
Comment=Hearth OS Admin Edition Welcome Center
Exec=/usr/bin/hearth-welcome
Icon=computer
Terminal=false
Categories=System;Utility;
DESKTOP
cat > "$STAGE/usr/share/applications/hearth-convert.desktop" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth OS Conversion Setup
Comment=Enable Hearth Welcome on login; keep your system writable
Exec=/usr/bin/hearth-convert enable
Icon=preferences-system
Terminal=true
Categories=System;Settings;
DESKTOP
install -d "$STAGE/usr/share/xsessions"
cat > "$STAGE/usr/share/xsessions/hearth-admin.desktop" <<'SESSION'
[Desktop Entry]
Name=Hearth OS Admin Edition (Cinnamon)
Comment=Writable Hearth desktop with Cinnamon fallback
Exec=/usr/bin/hearth-session
TryExec=/usr/bin/cinnamon-session
Type=Application
DesktopNames=X-Cinnamon;
SESSION
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: hearth-os-app
Version: $PACKAGE_VERSION
Section: games
Priority: optional
Architecture: amd64
Maintainer: Hearth OS community <noreply@github.com>
Depends: libwebkit2gtk-4.1-0, python3, python3-tk
Description: Hearth OS Admin Edition system tools and gaming interface
 Upgradeable native application for the writable Linux Mint Hearth OS test.
CONTROL
# The first ISO used /usr/local/bin/hearth and a launcher pointing to it.
# dpkg now owns /usr/bin/hearth and its launcher, so upgrades can replace them.
# The original /usr/local/bin/hearth is deliberately left untouched for rollback.
mkdir -p "$OUT"
dpkg-deb --build --root-owner-group "$STAGE" "$OUT/hearth-os-app_${PACKAGE_VERSION}_amd64.deb"
dpkg-deb --info "$OUT/hearth-os-app_${PACKAGE_VERSION}_amd64.deb"
