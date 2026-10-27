#!/usr/bin/env bash
# Hearth OS Admin Edition V3: user-controlled, fully writable conversion.
set -euo pipefail
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/hearth-admin-edition"
AUTOSTART="${XDG_CONFIG_HOME:-$HOME/.config}/autostart/hearth-welcome.desktop"
BACKUP="$CONFIG/autostart-before-v3.desktop"
SOURCE=/usr/share/applications/hearth-welcome.desktop
usage() { echo 'Usage: hearth-convert {status|enable|disable|verify}'; }
check_user() { if [ "$EUID" -eq 0 ]; then echo 'Run as your desktop user (no sudo).' >&2; exit 1; fi; }
verify() {
  echo 'Hearth OS Admin Edition V3 — writable-system verification'
  if command -v ostree >/dev/null 2>&1 && { test -d /ostree || test -e /run/ostree-booted; }; then
    echo 'WARNING: OSTree system detected. V3 does not support converting OSTree systems.' >&2; return 1
  fi
  if command -v dpkg >/dev/null && command -v apt-get >/dev/null; then echo 'APT/dpkg: available'; else echo 'APT/dpkg: missing'; return 1; fi
  if test -e /run/ostree-booted; then echo 'OSTree boot detected'; return 1; fi
  if test -r /etc/os-release; then . /etc/os-release; echo "Underlying distro: ${PRETTY_NAME:-unknown}"; fi
  for tool in hearth hearth-welcome hearth-admin-center hearth-admin-report hearth-upgrade; do
    if command -v "$tool" >/dev/null; then echo "$tool: installed"; else echo "$tool: missing"; fi
  done
  echo 'No immutable filesystem, snapshot manager, kernel or package source changes are made.'
}
case "${1:-status}" in
  verify) verify ;;
  status)
    verify
    if test -f "$AUTOSTART"; then echo 'Hearth Welcome autostart: enabled'; else echo 'Hearth Welcome autostart: disabled'; fi
    echo "User configuration: $CONFIG"
    ;;
  enable)
    check_user
    verify
    test -f "$SOURCE" || { echo 'Welcome Center desktop entry missing' >&2; exit 1; }
    mkdir -p -m 700 "$CONFIG" "$(dirname "$AUTOSTART")"
    if test -e "$AUTOSTART" && ! test -e "$BACKUP"; then cp -p -- "$AUTOSTART" "$BACKUP"; fi
    cp -- "$SOURCE" "$AUTOSTART"
    chmod 600 "$AUTOSTART"
    echo 'Hearth Welcome Center will launch on next desktop login. Mint and APT remain fully writable.'
    ;;
  disable)
    check_user
    if test -e "$BACKUP"; then mv -- "$BACKUP" "$AUTOSTART"; else rm -f -- "$AUTOSTART"; fi
    echo 'Hearth Welcome autostart disabled; prior launcher restored if one existed.'
    ;;
  *) usage; exit 2 ;;
esac
