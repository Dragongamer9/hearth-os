#!/bin/sh
# Launch the experimental Hearth AI Command Center from its source directory.
set -eu
SELF=$(readlink -f "$0")
HERE=$(dirname "$SELF")
if ! command -v python3 >/dev/null 2>&1; then
  printf '%s\n' 'Hearth AI requires Python 3.' >&2
  exit 1
fi
cd "$HERE"
exec python3 "$HERE/server.py" "$@"
