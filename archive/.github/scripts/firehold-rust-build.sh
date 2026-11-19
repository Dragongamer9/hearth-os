#!/bin/bash
# Builds firehold-comp and firehold-shell inside the Anatase base image (base-anatase.yml "firehold"; docs/FIREHOLD-RUST.md),
# so they link exactly the libraries the image has. The source is /src (read only); out: /out/hearth-firehold.tar.
set -eu
DNF=$(command -v dnf5 || command -v dnf)
$DNF -y install gcc gcc-c++ pkgconf-pkg-config rust cargo systemd-devel libseat-devel libinput-devel mesa-libgbm-devel \
  libdrm-devel libxkbcommon-devel wayland-devel mesa-libEGL-devel pixman-devel fontconfig-devel freetype-devel >/dev/null
echo "::notice title=Firehold (Rust, test) build::$(rustc --version) · $(. /etc/os-release; echo "$PRETTY_NAME")"
mkdir -p /tmp/w/hosts/linux/desktop/theme /tmp/w/cargo
# A bootc base's /root is a link to /var/roothome, which isn't there in a plain container: cargo's home goes to /tmp
# (without this cargo stops at once with "failed to create directory `/root/.cargo`: File exists").
export HOME=/tmp/w CARGO_HOME=/tmp/w/cargo
# firehold-shell includes hearth-core's settings_file.rs by path (one helper for every Rust writer) and takes Hearth's
# look from hearth-ui, which carries palettes.json as its built-in copy.
cp -a /src/hosts/firehold-comp /src/hosts/firehold-shell /src/hosts/hearth-core /src/hosts/hearth-ui /tmp/w/hosts/
cp /src/hosts/linux/desktop/theme/palettes.json /tmp/w/hosts/linux/desktop/theme/
cd /tmp/w
for c in firehold-comp firehold-shell; do
  if ! (cd "hosts/$c" && cargo build --release > "/tmp/w/build-$c.log" 2>&1); then
    # The first errors as annotations (the build's log can't be read from here), else cargo's own last words.
    awk '/^error/{if (b) print b; b=$0; n=0; next} b && n < 14 {b=b "|" $0; n++} END{if (b) print b}' "/tmp/w/build-$c.log" | head -12 | while read -r l; do
      echo "::error title=Firehold (Rust, test) build ($c)::$(echo "$l" | cut -c1-3500)"
    done
    grep -q '^error' "/tmp/w/build-$c.log" || echo "::error title=Firehold (Rust, test) build ($c)::$(tail -c 3000 "/tmp/w/build-$c.log" | tr '\n' '|')"
  fi
  test -x "hosts/$c/target/release/$c" || { echo "::error title=Firehold (Rust, test) build::$c didn't build"; exit 1; }
done
mkdir -p /tmp/w/out
cp hosts/firehold-comp/target/release/firehold-comp hosts/firehold-shell/target/release/firehold-shell /tmp/w/out/
strip /tmp/w/out/* 2>/dev/null || true
tar -C /tmp/w/out -cf /out/hearth-firehold.tar firehold-comp firehold-shell
ls -la /tmp/w/out
