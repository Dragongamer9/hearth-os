#!/usr/bin/env bash
# Optional per-user Hearth Admin Edition desktop identity for Cinnamon.
# Never changes Linux Mint package repositories or /etc/os-release.
set -euo pipefail
case "${1:-status}" in
  status)
    cat /etc/hearth-admin-release 2>/dev/null || echo 'Hearth Admin Edition package not installed'
    command -v gsettings >/dev/null || exit 0
    gsettings get org.cinnamon.desktop.background picture-uri 2>/dev/null || true
    ;;
  apply)
    if ! command -v gsettings >/dev/null; then echo "Cinnamon settings unavailable" >&2; exit 1; fi
    if [ "${EUID}" -eq 0 ]; then echo "Run this as your normal desktop user, not with sudo." >&2; exit 1; fi
    target=/usr/share/backgrounds/hearth-admin-edition.svg
    test -f "$target"
    gsettings set org.cinnamon.desktop.background picture-uri "file://$target"
    echo 'Hearth Admin Edition wallpaper enabled for your user.'
    ;;
  *)
    echo "Usage: hearth-admin-branding [status|apply]" >&2
    exit 2
    ;;
esac
