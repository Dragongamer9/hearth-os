#!/usr/bin/env bash
# Hearth OS Admin Edition desktop integration. Run as normal Cinnamon user.
# No system-wide settings, privilege escalation or automatic changes.
set -euo pipefail
SCHEMA=org.cinnamon.desktop.background
KEY=picture-uri
WALLPAPER=file:///usr/share/backgrounds/hearth-admin-edition.svg
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/hearth-admin-edition"
BACKUP="$CONFIG/desktop-before-hearth.txt"
usage() {
  echo "Usage: hearth-admin-desktop {status|apply|restore|launch}"
  echo "  apply  - save current wallpaper and set Hearth wallpaper (opt-in)"
  echo "  restore - restore saved wallpaper"
  echo "  launch - open Hearth Admin Center"
}
check_user() {
  if [ "${EUID}" -eq 0 ]; then echo 'Run as desktop user, not sudo.' >&2; exit 1; fi
  if ! command -v gsettings >/dev/null || ! gsettings list-keys "$SCHEMA" 2>/dev/null | grep -Fxq "$KEY"; then
    echo 'Cinnamon desktop settings are unavailable.' >&2; exit 1
  fi
}
case "${1:-status}" in
  status)
    if [ -r /etc/hearth-admin-release ]; then cat /etc/hearth-admin-release; fi
    if command -v gsettings >/dev/null; then gsettings get "$SCHEMA" "$KEY" 2>/dev/null || true; fi
    if [ -f "$BACKUP" ]; then echo "Wallpaper backup: $BACKUP"; fi
    ;;
  apply)
    check_user
    test -f /usr/share/backgrounds/hearth-admin-edition.svg || { echo 'Hearth wallpaper missing' >&2; exit 1; }
    mkdir -p -m 700 "$CONFIG"
    if [ ! -e "$BACKUP" ]; then
      umask 077
      gsettings get "$SCHEMA" "$KEY" > "$BACKUP"
    fi
    gsettings set "$SCHEMA" "$KEY" "$WALLPAPER"
    echo 'Hearth Admin Edition wallpaper applied. Run hearth-admin-desktop restore to undo.'
    ;;
  restore)
    check_user
    test -f "$BACKUP" || { echo 'No saved desktop setting to restore.' >&2; exit 1; }
    saved="$(cat "$BACKUP")"
    # gsettings get returns a quoted GVariant string. Reject non-string content.
    if [[ ! "$saved" =~ ^\'.*\'$ ]]; then echo 'Invalid saved wallpaper setting.' >&2; exit 1; fi
    gsettings set "$SCHEMA" "$KEY" "${saved:1:${#saved}-2}"
    rm -- "$BACKUP"
    echo 'Previous wallpaper restored.'
    ;;
  launch) exec /usr/bin/hearth-admin-center ;;
  *) usage; exit 2 ;;
esac
