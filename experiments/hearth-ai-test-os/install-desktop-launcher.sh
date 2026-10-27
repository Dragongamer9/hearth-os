#!/bin/sh
# Explicit user-level launcher install. No root/system files changed.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APPDIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
TARGET="$APPDIR/hearth-ai-experimental.desktop"
mkdir -p "$APPDIR"
case "${1:-install}" in
  --remove)
    rm -f "$TARGET"
    printf 'Removed launcher: %s\n' "$TARGET"
    ;;
  install)
    python3 - "$HERE" "$TARGET" <<'PY'
import sys
from pathlib import Path
here, target=sys.argv[1:]
def escape(value):
    # Desktop Entry Specification: escape backslashes, line feeds and reserved chars.
    return value.replace("\\","\\\\").replace("\n"," ").replace("\r"," ")
script=Path(here)/"launch.sh"
if not script.exists():
    raise SystemExit("launch.sh not found")
text="""[Desktop Entry]
Type=Application
Name=Hearth AI Command Center (Experimental)
Comment=Test-only local AI dashboard for Hearth OS
Exec={command}
Path={workdir}
Icon=utilities-terminal
Terminal=true
Categories=Utility;Development;
StartupNotify=false
""".format(command=escape(str(script)),workdir=escape(here))
Path(target).write_text(text,encoding="utf-8")
PY
    chmod 755 "$HERE/launch.sh"
    printf 'Installed launcher: %s\n' "$TARGET"
    ;;
  *) printf 'Usage: %s [install|--remove]\n' "$0" >&2; exit 2 ;;
esac
