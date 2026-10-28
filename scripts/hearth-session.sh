#!/usr/bin/env bash
# Hearth desktop session: Cinnamon remains available as fallback.
set -euo pipefail
if ! command -v cinnamon-session >/dev/null 2>&1; then
  echo 'Cinnamon session missing; select the original Cinnamon session at login.' >&2
  exit 1
fi
# Autostart Hearth welcome in this session only; never change the user's default login.
export XDG_CURRENT_DESKTOP="X-Cinnamon"
export DESKTOP_SESSION="hearth-admin"
( sleep 8; /usr/bin/hearth-welcome >/dev/null 2>&1 ) &
exec cinnamon-session
