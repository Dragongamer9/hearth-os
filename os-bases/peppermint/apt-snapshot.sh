#!/bin/sh
# Smaller updates (1.13): Debian's repositories as they were at one moment (snapshot.debian.org), so two builds in
# the same week get byte-for-byte the same packages. Run in the Containerfile's base stage. The repositories as
# they are live are kept in /usr/share/hearth/apt-live; build.sh puts them back at the end, so apt on a device
# (Writable system) gets today's packages and security fixes, never the frozen ones.
#   apt-snapshot.sh YYYYMMDDTHHMMSSZ   ("" leaves the live repositories)
# If the snapshot can't be reached, the build goes on with the live repositories and says so (updates are then
# bigger, nothing else changes).
set -eu
D=${1:-}
L=/usr/share/hearth/apt-live
S=/etc/apt/sources.list.d
mkdir -p "$L"
cp "$S/debian.sources" "$S/debian-backports.sources" "$L/"
if [ -z "$D" ]; then echo "apt-snapshot: Debian's live repositories (no freeze)"; exit 0; fi
case "$D" in
  [0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]T[0-9][0-9][0-9][0-9][0-9][0-9]Z) ;;
  *) echo "apt-snapshot: not a snapshot date: $D" >&2; exit 1 ;;
esac
C='main contrib non-free non-free-firmware'
K=/usr/share/keyrings/debian-archive-keyring.gpg
cat > "$S/debian.sources" <<SRC
Types: deb
URIs: http://snapshot.debian.org/archive/debian/$D/
Suites: trixie trixie-updates
Components: $C
Signed-By: $K
Check-Valid-Until: no

Types: deb
URIs: http://snapshot.debian.org/archive/debian-security/$D/
Suites: trixie-security
Components: $C
Signed-By: $K
Check-Valid-Until: no
SRC
cat > "$S/debian-backports.sources" <<SRC
Types: deb
URIs: http://snapshot.debian.org/archive/debian/$D/
Suites: trixie-backports
Components: $C
Signed-By: $K
Check-Valid-Until: no
SRC
# snapshot.debian.org is slower than the mirrors and sometimes busy: more patience, more retries.
printf 'Acquire::Retries "8";\nAcquire::http::Timeout "120";\nAcquire::Queue-Mode "access";\n' > /etc/apt/apt.conf.d/80-hearth-snapshot
if apt-get update -y; then
  echo "$D" > /usr/share/hearth/debian-snapshot
  echo "apt-snapshot: Debian frozen at $D"
else
  echo "HEARTH-SNAPSHOT-FALLBACK: snapshot.debian.org didn't answer; building from the live repositories"
  cp "$L/debian.sources" "$L/debian-backports.sources" "$S/"
  rm -f /etc/apt/apt.conf.d/80-hearth-snapshot
  apt-get update -y
fi
