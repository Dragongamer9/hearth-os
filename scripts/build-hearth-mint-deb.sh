#!/usr/bin/env bash
# Build an upgrade package installable on the original writable Mint Hearth ISO.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${1:-$ROOT/hosts/hearth-rs/target/release/hearth}"
OUT="${2:-$ROOT/dist}"
VERSION="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/hosts/hearth-rs/Cargo.toml" | head -1)"
test -x "$BIN"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]
ARCH="$(dpkg --print-architecture)"
test "$ARCH" = amd64 || { echo 'Only amd64 is supported for this first test.' >&2; exit 1; }
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT
install -Dm755 "$BIN" "$STAGE/usr/bin/hearth"
install -Dm755 "$ROOT/scripts/hearth-upgrade.sh" "$STAGE/usr/bin/hearth-upgrade"
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
cat > "$STAGE/DEBIAN/control" <<CONTROL
Package: hearth-os-app
Version: $VERSION
Section: games
Priority: optional
Architecture: amd64
Maintainer: Hearth OS community <noreply@github.com>
Depends: libwebkit2gtk-4.1-0
Description: Hearth OS gaming interface
 Upgradeable native application for the writable Linux Mint Hearth OS test.
CONTROL
# The first ISO used /usr/local/bin/hearth and a launcher pointing to it.
# dpkg now owns /usr/bin/hearth and its launcher, so upgrades can replace them.
# The original /usr/local/bin/hearth is deliberately left untouched for rollback.
mkdir -p "$OUT"
dpkg-deb --build --root-owner-group "$STAGE" "$OUT/hearth-os-app_${VERSION}_amd64.deb"
dpkg-deb --info "$OUT/hearth-os-app_${VERSION}_amd64.deb"
