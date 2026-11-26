#!/bin/bash
# Builds gamescope for the Hearth Store part "gamescope" (quick-update.yml; docs/STORE-SYSTEM-PARTS.md "Starting up"),
# inside Debian trixie, the image's own base. It's Debian's own gamescope source package, the version line the image
# ships (build.sh installs gamescope from trixie-backports) or newer (GAMESCOPE_SUITE, a Debian suite that has a newer
# one), built with Debian's own options (dpkg-buildpackage): the image's gamescope rebuilt, not another program.
# Every library it needs is checked with ldd here: none missing, all from the system's own library folders (what
# the image has); the list goes with it (NEEDED.txt), and hearth-part's trial checks the same on the device before it's
# ever started. Run in the base image with the source at /src (read only); out: /out/gamescope/ with gamescope,
# gamescopereaper (when the package has it), NEEDED.txt and SOURCE.txt.
set -euo pipefail
export DEBIAN_FRONTEND=noninteractive
SUITE="${GAMESCOPE_SUITE:-trixie-backports}"
ann() { echo "::$1 title=gamescope part::$(echo "$2" | tr '\n' '|' | cut -c1-3000)"; }
cat > /etc/apt/sources.list.d/hearth-part-build.sources <<EOF
Types: deb deb-src
URIs: http://deb.debian.org/debian
Suites: trixie trixie-updates trixie-backports
Components: main contrib non-free non-free-firmware
Signed-By: /usr/share/keyrings/debian-archive-keyring.gpg
EOF
apt-get update -qq >/dev/null
apt-get install -y -qq --no-install-recommends build-essential dpkg-dev devscripts fakeroot ca-certificates file >/dev/null
mkdir -p /tmp/g && cd /tmp/g
# The source of the version the image would install from that suite (what `apt-get install -t SUITE gamescope` picks).
apt-get source -qq -t "$SUITE" gamescope >/dev/null 2>/tmp/g/source.log || { ann error "no gamescope source in $SUITE: $(tail -5 /tmp/g/source.log)"; exit 1; }
dir=$(find /tmp/g -maxdepth 1 -mindepth 1 -type d -name 'gamescope-*' | head -1)
[ -n "$dir" ] || { ann error "the source package didn't unpack"; exit 1; }
ver=$(cd "$dir" && dpkg-parsechangelog -S Version)
apt-get build-dep -y -qq -t "$SUITE" "$dir" >/tmp/g/deps.log 2>&1 || { ann error "build dependencies: $(tail -8 /tmp/g/deps.log)"; exit 1; }
(cd "$dir" && DEB_BUILD_OPTIONS="nocheck parallel=$(nproc)" dpkg-buildpackage -b -uc -us >/tmp/g/build.log 2>&1) \
  || { ann error "build of $ver: $(grep -iE 'error|failed' /tmp/g/build.log | tail -12)"; exit 1; }
deb=$(ls /tmp/g/gamescope_*.deb | head -1)
mkdir -p /tmp/g/x && dpkg-deb -x "$deb" /tmp/g/x
mkdir -p /out/gamescope
for b in gamescope gamescopereaper; do
  f=$(find /tmp/g/x/usr -type f -name "$b" -perm -u+x | head -1)
  if [ -n "$f" ]; then install -m755 "$f" "/out/gamescope/$b"; fi
done
test -x /out/gamescope/gamescope || { ann error "the package has no gamescope program"; exit 1; }
# The libraries: every one found, every one from the system's own folders (never a stray copy from the build).
bad=""
: > /out/gamescope/NEEDED.txt
for b in /out/gamescope/gamescope /out/gamescope/gamescopereaper; do
  [ -x "$b" ] || continue
  file "$b" | grep -q ELF || { ann error "$(basename "$b") isn't a program"; exit 1; }
  out=$(ldd "$b")
  if echo "$out" | grep -q 'not found'; then bad="$bad $(basename "$b"): $(echo "$out" | grep 'not found' | tr -s ' ' | tr '\n' ';')"; fi
  echo "$out" | awk '/=>/ {print $3}' | while read -r lib; do
    case "$lib" in /lib/*|/usr/lib/*|/lib64/*) ;; *) echo "OUTSIDE $lib" ;; esac
  done > /tmp/g/outside.txt
  [ -s /tmp/g/outside.txt ] && bad="$bad $(basename "$b"): $(tr '\n' ' ' < /tmp/g/outside.txt)"
  readelf -d "$b" | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p' | sed "s#^#$(basename "$b") #" >> /out/gamescope/NEEDED.txt
done
[ -z "$bad" ] || { ann error "libraries:$bad"; exit 1; }
v=$(/out/gamescope/gamescope --version 2>&1 | head -2 | tr '\n' ' ' || true)
printf 'gamescope %s (Debian source package, %s), built with Debian'"'"'s own options in Debian trixie.\nBSD-2-Clause and others: https://github.com/ValveSoftware/gamescope\n%s\n' "$ver" "$SUITE" "$v" > /out/gamescope/SOURCE.txt
ann notice "built gamescope $ver from $SUITE: $(wc -l < /out/gamescope/NEEDED.txt) libraries, all found ($v)"
