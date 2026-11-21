#!/bin/bash
# Quickshell for Mantel on the Fedora edition (1.16: Hearth Classic and Firehold on labwc, chosen in Settings › Your
# desktop; docs/LABWC-FEDORA.md). The same pinned release the Debian edition builds (os-bases/peppermint/builder.sh):
# Fedora 45 only has an older git snapshot (0.2.1^git20260209), and Mantel is written and tested against v0.3.2.
# Quickshell uses Qt's private API, so it must be built against the very Qt the image has: os-bases/anatase/Containerfile
# runs this in a stage made from the same base image, and build.sh refuses a Quickshell built against another Qt.
#   quickshell-build.sh OUT    installs into OUT (OUT/usr/bin/quickshell) and writes OUT/usr/share/hearth/quickshell-qt
# Used by .github/workflows/labwc-fedora.yml too (the same build, in a Fedora 45 container).
set -euxo pipefail
OUT=${1:?output folder}
QUICKSHELL_REF="${QUICKSHELL_REF:-v0.3.2}"
dnf5 -y install --setopt=install_weak_deps=False \
  git cmake ninja-build gcc-c++ pkgconf-pkg-config \
  qt6-qtbase-devel qt6-qtbase-private-devel qt6-qtdeclarative-devel qt6-qtwayland-devel qt6-qtshadertools-devel \
  qt6-qtsvg-devel spirv-tools cli11-devel jemalloc-devel pipewire-devel pam-devel polkit-devel glib2-devel \
  libdrm-devel mesa-libgbm-devel libglvnd-devel vulkan-headers wayland-devel wayland-protocols-devel libxcb-devel
# Private headers that some Fedora releases split out (each may be missing: the build says what it lacks).
dnf5 -y install --skip-unavailable --setopt=install_weak_deps=False qt6-qtdeclarative-private-devel qt6-qtwayland-private-devel || true
src=$(mktemp -d)
git clone --depth 1 --branch "$QUICKSHELL_REF" https://github.com/quickshell-mirror/quickshell "$src/quickshell"
cmake -S "$src/quickshell" -B "$src/build" -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=/usr \
  -DDISTRIBUTOR="Hearth OS (Fedora edition)" -DCRASH_HANDLER=OFF -DHYPRLAND=OFF -DI3=OFF
cmake --build "$src/build"
DESTDIR="$OUT" cmake --install "$src/build"
test -x "$OUT/usr/bin/quickshell"
mkdir -p "$OUT/usr/share/hearth"
rpm -q --qf '%{VERSION}-%{RELEASE}\n' qt6-qtbase > "$OUT/usr/share/hearth/quickshell-qt"
echo "$QUICKSHELL_REF" > "$OUT/usr/share/hearth/quickshell-ref"
rm -rf "$src"
echo "Quickshell $QUICKSHELL_REF built against Qt $(cat "$OUT/usr/share/hearth/quickshell-qt")"
