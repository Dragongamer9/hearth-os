#!/bin/sh
# Adds (or with --remove, removes) the "Nimbus" session on the login screen.
# Run as root; Nimbus runs it through pkexec from Settings › Shell.
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ "$1" = "--remove" ]; then
  rm -f /usr/share/wayland-sessions/nimbus.desktop /usr/local/bin/nimbus-session
  rm -rf /usr/local/lib/nimbus
  echo "Nimbus session removed"
  exit 0
fi

install -Dm755 "$HERE/nimbus-session" /usr/local/bin/nimbus-session
install -Dm755 "$HERE/nimbus-watchdog" /usr/local/lib/nimbus/nimbus-watchdog
install -Dm644 "$HERE/nimbus.desktop" /usr/share/wayland-sessions/nimbus.desktop
echo "Nimbus session installed. Pick \"Nimbus\" on the login screen."
