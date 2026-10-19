#!/bin/bash
# Hearth on Ubuntu 26.04 LTS: the builder stage of os-bases/ubuntu/Containerfile. Compiles what Ubuntu doesn't
# package (or packages too old, or in a form bootc can't use) into /output, which the image stage copies over /.
# Nothing from here (compilers, -dev packages, Rust) ends up in the image. The same recipe as
# os-bases/peppermint/builder.sh (proven in CI on Debian 13), on Ubuntu's archive.
#
#   ostree   built from the release tarball, pinned: bootc needs libostree 2025.3 or newer, and one known
#            ostree (with its dracut module, 50ostree) for every Debian-family Hearth image
#   bootc    not in Ubuntu (updates, rollbacks, `bootc status`, what hearth-admin drives)
#   bootupd  not in Ubuntu (what Hearth's installer uses to put the boot loader on the EFI partition)
#   Quickshell  built at the same tag as the other Hearth images (Desktop Mode's taskbar, launcher, settings)
#   RyzenAdj    not in Ubuntu (power limit on AMD handhelds without ASUS firmware attributes; hearth-admin)
#   Handheld Daemon  not in Ubuntu: built from its tagged source into its own venv at /usr/lib/hhd
#
# It also writes:
#   /debs/*.deb   placeholder packages telling dpkg that libostree-1-1, ostree and ostree-boot are provided by
#                 the ostree built here (Ubuntu's flatpak depends on libostree-1-1)
#   /output/usr/share/hearth/builder-runtime-deps.txt   the Ubuntu packages the compiled programs link against
#                 (build.sh installs them)
#
# Versions are pinned so a rebuild doesn't silently pick up something untested. Bump them here (and in
# os-bases/peppermint/builder.sh, so the Debian-family images stay alike).
set -ouex pipefail

OSTREE_VER="${OSTREE_VER:-2026.4}"
BOOTC_REF="${BOOTC_REF:-v1.17.1}"
BOOTUPD_REF="${BOOTUPD_REF:-v0.3.2}"
QUICKSHELL_REF="${QUICKSHELL_REF:-v0.3.2}"
RYZENADJ_REF="${RYZENADJ_REF:-v0.19.0}"
HHD_REF="${HHD_REF:-v4.1.12}"
MULTIARCH=$(dpkg-architecture -qDEB_HOST_MULTIARCH 2>/dev/null || echo x86_64-linux-gnu)

export DEBIAN_FRONTEND=noninteractive
apt-get update -y
apt-get install -y --no-install-recommends \
  ca-certificates curl git xz-utils file make build-essential pkgconf cmake ninja-build go-md2man \
  autoconf automake libtool bison flex dpkg-dev equivs \
  libglib2.0-dev liblzma-dev zlib1g-dev libext2fs-dev libcurl4-openssl-dev libssl-dev libgpgme-dev \
  libarchive-dev libmount-dev libsystemd-dev systemd-dev libfuse3-dev libzstd-dev libcap-dev \
  libpci-dev \
  qt6-base-dev qt6-base-private-dev qt6-declarative-dev qt6-declarative-private-dev \
  qt6-wayland-dev qt6-shadertools-dev qt6-svg-dev spirv-tools \
  libcli11-dev libjemalloc-dev libpipewire-0.3-dev libpam0g-dev libpolkit-agent-1-dev libpolkit-gobject-1-dev \
  libdrm-dev libgbm-dev libegl-dev libvulkan-dev wayland-protocols libwayland-dev libwayland-bin libxcb1-dev \
  extra-cmake-modules meson \
  python3 python3-venv python3-pip python3-setuptools python3-wheel \
  clang libclang-dev
# (clang: bootc's "make install" builds its man pages with default features, whose SELinux bindings need it)
# Names that may differ between releases: each on its own, a missing one isn't fatal here (the build that needs
# it says so).
# libcomposefs-dev: ostree uses composefs when it's there (the image's root is mounted through it if so).
# qt6-wayland-private-dev: Quickshell's Wayland parts (some releases ship these headers in qt6-wayland-dev).
for p in libcomposefs-dev qt6-wayland-private-dev vulkan-headers spirv-headers; do
  apt-get install -y --no-install-recommends "$p" || echo "builder: $p not available"
done

mkdir -p /output /src /debs
fetch() { git clone --depth 1 --branch "$2" "https://github.com/$1" "/src/$3"; }

# ------------------------------------------------------------------ ostree
# The release tarball (no autogen needed). Installed twice: into / (bootc builds against it) and into /output
# (the image). Same library directory as Ubuntu's own package, so nothing else needs to know.
curl -fsSL --retry 3 -o /src/ostree.tar.xz \
  "https://github.com/ostreedev/ostree/releases/download/v${OSTREE_VER}/libostree-${OSTREE_VER}.tar.xz"
tar -C /src -xf /src/ostree.tar.xz
cd "/src/libostree-${OSTREE_VER}"
./configure --prefix=/usr --libdir="/usr/lib/${MULTIARCH}" --libexecdir=/usr/libexec \
  --sysconfdir=/etc --localstatedir=/var \
  --with-crypto=openssl --with-curl --without-soup --with-libarchive --with-libmount --with-libsystemd \
  --without-selinux --without-avahi --with-dracut=yes \
  --with-systemdsystemunitdir=/usr/lib/systemd/system \
  --with-systemdsystemgeneratordir=/usr/lib/systemd/system-generators \
  --disable-gtk-doc --disable-man --disable-introspection --disable-static --disable-installed-tests \
  --disable-rofiles-fuse
make -j"$(nproc)"
make install
make install DESTDIR=/output
ldconfig
cd /
if ! out=$(/usr/bin/ostree --version 2>&1); then
  echo "ostree --version failed: $out"
  ls -la /usr/bin/ostree || true
  ldd /usr/bin/ostree 2>&1 | grep -E "not found|ostree" || true
  exit 1
fi
printf "%s\n" "$out" | sed -n 1,3p
test -x /output/usr/lib/ostree/ostree-prepare-root || test -x /output/usr/libexec/ostree-prepare-root || \
  find /output -name 'ostree-prepare-root*' | grep . >/dev/null
ls -d /output/usr/lib/dracut/modules.d/*ostree >/dev/null   # 50ostree since ostree 2024; 98ostree before

# Placeholders so dpkg knows these are already provided (flatpak needs libostree-1-1). Held in build.sh, so
# nothing replaces the ostree built here with the archive's.
for pkg in libostree-1-1 ostree ostree-boot; do
  cat > "/src/$pkg.ctl" <<EOF
Section: misc
Priority: optional
Standards-Version: 4.6.2
Package: $pkg
Version: ${OSTREE_VER}-0hearth1
Maintainer: Hearth OS <noreply@github.com>
Architecture: $(dpkg --print-architecture)
Description: ostree ${OSTREE_VER}, built from source by Hearth (os-bases/ubuntu/builder.sh)
 Placeholder: the files are in the image, installed outside dpkg.
EOF
  (cd /debs && equivs-build "/src/$pkg.ctl")
done
ls -la /debs

# ------------------------------------------------------------------ Rust (rustup: the same toolchain as the other images)
export CARGO_HOME=/tmp/rust RUSTUP_HOME=/tmp/rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --profile minimal -y
. /tmp/rust/env

# ------------------------------------------------------------------ bootc (no SELinux on Ubuntu: built without it, which bootc
# supports; pre-6.15 only adds a fallback for older kernels, as in the Peppermint and CachyOS builds)
fetch bootc-dev/bootc "$BOOTC_REF" bootc
make -C /src/bootc bin CARGO_OPTIONS='--no-default-features --features "install-to-disk pre-6.15"'
make -C /src/bootc install install-ostree-hooks DESTDIR=/output
test -x /output/usr/bin/bootc

# ------------------------------------------------------------------ bootupd
fetch coreos/bootupd "$BOOTUPD_REF" bootupd
make -C /src/bootupd
make -C /src/bootupd install install-grub-static DESTDIR=/output
test -x /output/usr/bin/bootupctl

# ------------------------------------------------------------------ Quickshell
# Wayland protocol files at a known version for the build only (XML files; nothing ships in the image), so
# Quickshell's protocols (ext-background-effect, ...) are there whatever the archive carries.
git clone --depth 1 --branch 1.46 https://gitlab.freedesktop.org/wayland/wayland-protocols.git /src/wayland-protocols
meson setup /src/wayland-protocols/build /src/wayland-protocols --prefix=/usr -Dtests=false
ninja -C /src/wayland-protocols/build install
fetch quickshell-mirror/quickshell "$QUICKSHELL_REF" quickshell
cmake -S /src/quickshell -B /src/quickshell/build -G Ninja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=/usr \
  -DDISTRIBUTOR="Hearth OS (Ubuntu edition)" -DCRASH_HANDLER=OFF -DHYPRLAND=OFF -DI3=OFF
cmake --build /src/quickshell/build
DESTDIR=/output cmake --install /src/quickshell/build
test -x /output/usr/bin/quickshell

# ------------------------------------------------------------------ RyzenAdj
fetch FlyGoat/RyzenAdj "$RYZENADJ_REF" ryzenadj
cmake -S /src/ryzenadj -B /src/ryzenadj/build -DCMAKE_BUILD_TYPE=Release
cmake --build /src/ryzenadj/build
install -Dm755 /src/ryzenadj/build/ryzenadj /output/usr/bin/ryzenadj

# ------------------------------------------------------------------ Handheld Daemon
# Its own venv (--system-site-packages: its Python dependencies come from Ubuntu's packages, which build.sh
# installs: python3-evdev, -yaml, -rich, -xlib, -serial, -pyroute2, -gi, -dbus, -setuptools). Same Python as
# the image (both stages start from the same Ubuntu), at the same path, so the venv works there unchanged.
fetch hhd-dev/hhd "$HHD_REF" hhd
python3 -m venv --system-site-packages /usr/lib/hhd
/usr/lib/hhd/bin/pip install --no-cache-dir --no-deps --no-build-isolation /src/hhd
test -x /usr/lib/hhd/bin/hhd
mkdir -p /output/usr/lib && cp -a /usr/lib/hhd /output/usr/lib/hhd
# Its system files: udev rules (controller access), uinput, the services, the power-profiles D-Bus policy.
# (Each loop ends in an if, not a bare `[ … ] &&`: a false test as a loop's last command would stop set -e.)
h=/src/hhd/usr
for f in "$h"/lib/udev/rules.d/*.rules; do install -Dm644 "$f" "/output/usr/lib/udev/rules.d/$(basename "$f")"; done
for f in "$h"/lib/udev/hwdb.d/*.hwdb; do if [ -f "$f" ]; then install -Dm644 "$f" "/output/usr/lib/udev/hwdb.d/$(basename "$f")"; fi; done
for f in "$h"/lib/modules-load.d/*.conf; do if [ -f "$f" ]; then install -Dm644 "$f" "/output/usr/lib/modules-load.d/$(basename "$f")"; fi; done
for f in "$h"/lib/systemd/system/*.service; do install -Dm644 "$f" "/output/usr/lib/systemd/system/$(basename "$f")"; done
for f in "$h"/share/dbus-1/system.d/*.conf; do if [ -f "$f" ]; then install -Dm644 "$f" "/output/usr/share/dbus-1/system.d/$(basename "$f")"; fi; done
mkdir -p /output/usr/bin
ln -sf ../lib/hhd/bin/hhd /output/usr/bin/hhd
ln -sf ../lib/hhd/bin/hhdctl /output/usr/bin/hhdctl
mkdir -p /output/usr/share && echo "${HHD_REF#v}" > /output/usr/share/hearth-hhd-version

# ------------------------------------------------------------------ the Ubuntu packages all of the above link against
mkdir -p /output/usr/share/hearth
owner() {   # the package that owns a library (merged /usr: try both spellings of the path)
  local l="$1" p
  for p in "$l" "$(readlink -f "$l")" "${l#/usr}" "/usr${l}"; do
    dpkg -S "$p" 2>/dev/null | sed -n 1p | cut -d: -f1 && return 0
  done
  return 0
}
find /output -type f \( -perm -u+x -o -name '*.so*' \) | while read -r f; do
  file -b "$f" 2>/dev/null | grep ELF >/dev/null || continue
  ldd "$f" 2>/dev/null | awk '/=> \//{print $3}'
done | sort -u | while read -r lib; do owner "$lib"; done | grep -v '^$' | sort -u \
  > /output/usr/share/hearth/builder-runtime-deps.txt
cat /output/usr/share/hearth/builder-runtime-deps.txt
test -s /output/usr/share/hearth/builder-runtime-deps.txt
