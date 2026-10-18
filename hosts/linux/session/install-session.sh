#!/bin/sh
# Adds (or with --remove, removes) the "Nimbus" session on the login screen.
# Run as root; Nimbus runs it through pkexec from Settings › Shell.
#   /usr/local/bin/nimbus-session        the session: Game Mode and Desktop Mode
#   /usr/local/bin/nimbus-mode           switches between them
#   /usr/local/lib/nimbus/nimbus-watchdog keeps Game Mode's Nimbus running
#   /usr/local/share/nimbus/desktop/     Desktop Mode: labwc config and the Quickshell taskbar
set -e
HERE="$(cd "$(dirname "$0")" && pwd)"

if [ "$1" = "--remove" ]; then
  rm -f /usr/share/wayland-sessions/nimbus.desktop /usr/local/bin/nimbus-session /usr/local/bin/nimbus-mode
  rm -rf /usr/local/lib/nimbus /usr/local/share/nimbus
  echo "Hearth session removed"
  exit 0
fi

install -Dm755 "$HERE/nimbus-session" /usr/local/bin/nimbus-session
install -Dm755 "$HERE/nimbus-mode" /usr/local/bin/nimbus-mode
install -Dm755 "$HERE/nimbus-watchdog" /usr/local/lib/nimbus/nimbus-watchdog
install -Dm644 "$HERE/nimbus.desktop" /usr/share/wayland-sessions/nimbus.desktop
if [ -d "$HERE/desktop" ]; then
  rm -rf /usr/local/share/nimbus/desktop
  mkdir -p /usr/local/share/nimbus
  cp -r "$HERE/desktop" /usr/local/share/nimbus/desktop
  chmod -R a+rX /usr/local/share/nimbus
  find /usr/local/share/nimbus/desktop -name '*.sh' -exec chmod 755 {} +
  chmod 755 /usr/local/share/nimbus/desktop/labwc/autostart
fi
echo "Hearth session installed. Pick \"Hearth\" on the login screen."
