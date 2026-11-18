#!/bin/bash
# Builds firehold-comp and firehold-shell inside the Anatase base image (base-anatase.yml "firehold"; docs/FIREHOLD-RUST.md),
# so they link exactly the libraries the image has. The source is /src (read only); out: /out/hearth-firehold.tar.
set -eu
DNF=$(command -v dnf5 || command -v dnf)
$DNF -y install gcc gcc-c++ pkgconf-pkg-config rust cargo systemd-devel libseat-devel libinput-devel mesa-libgbm-devel \
  libdrm-devel libxkbcommon-devel wayland-devel mesa-libEGL-devel pixman-devel fontconfig-devel freetype-devel >/dev/null
echo "::notice title=Firehold (Rust, test) build::$(rustc --version) · $(. /etc/os-release; echo "$PRETTY_NAME")"
mkdir -p /tmp/w/hosts
# firehold-shell includes hearth-core's settings_file.rs by path (one helper for every Rust writer).
cp -a /src/hosts/firehold-comp /src/hosts/firehold-shell /src/hosts/hearth-core /tmp/w/hosts/
cd /tmp/w
for c in firehold-comp firehold-shell; do
  (cd "hosts/$c" && cargo build --release 2>&1 | grep -E '^(error|warning: unused)' | head -20; test -x "target/release/$c") \
    || { echo "::error title=Firehold (Rust, test) build::$c didn't build"; exit 1; }
done
mkdir -p /tmp/w/out
cp hosts/firehold-comp/target/release/firehold-comp hosts/firehold-shell/target/release/firehold-shell /tmp/w/out/
strip /tmp/w/out/* 2>/dev/null || true
tar -C /tmp/w/out -cf /out/hearth-firehold.tar firehold-comp firehold-shell
ls -la /tmp/w/out
