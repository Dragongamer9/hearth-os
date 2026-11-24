# Sourced by the Hearth Store part builds (hearth-ui-build.sh, mantel-rust-build.sh, hearth-link-build.sh,
# hearth-rs-part-build.sh): the compilers and libraries in the image's own base, so a part links exactly what the
# Hearth OS it's for has. Debian trixie (Hearth OS on Debian, 1.18): Debian's packages and Rust from rustup (trixie's
# own Rust is too old for some crates, as base-peppermint.yml found); Fedora (the archived edition, and the Fedora
# pictures workflow): dnf as before. part_tools [extra apt packages...]
part_tools() {
  mkdir -p /tmp/w
  if command -v apt-get >/dev/null 2>&1; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -qq >/dev/null
    apt-get install -y -qq --no-install-recommends ca-certificates curl gcc libc6-dev pkg-config \
      libxkbcommon-dev libwayland-dev libfontconfig-dev libfreetype-dev "$@" >/dev/null
    export RUSTUP_HOME=/tmp/w/rustup CARGO_HOME=/tmp/w/cargo
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path >/dev/null 2>&1
    export PATH="/tmp/w/cargo/bin:$PATH"
  else
    DNF=$(command -v dnf5 || command -v dnf)
    $DNF -y install gcc pkgconf-pkg-config rust cargo libxkbcommon-devel wayland-devel fontconfig-devel freetype-devel >/dev/null
  fi
}
