#!/bin/sh
# Optional per-user desktop integration; never writes into the OS image.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
APPDIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
TARGET="$APPDIR/hearth-ai-experimental.desktop"
case "${1:-install}" in
  --remove)
    rm -f -- "$TARGET"
    printf 'Removed launcher: %s\n' "$TARGET"
    ;;
  install)
    command -v python3 >/dev/null 2>&1 || { echo 'Python 3 required' >&2; exit 1; }
    mkdir -p -- "$APPDIR"
    python3 - "$HERE" "$TARGET" <<'PY'
import sys
from pathlib import Path

here, target = map(Path, sys.argv[1:])
script = (here / "launch.sh").resolve(strict=True)
icon = (here / "hearth-ai.svg").resolve(strict=True)
if not script.is_file():
    raise SystemExit("Hearth AI launch script is not a regular file")

def desktop_argument(path):
    # Desktop Entry spec: quote any path; escape backslashes, quotes, dollars, and backticks.
    # Percent is escaped separately because '%' denotes field codes in Exec entries.
    value = str(path).replace("%", "%%")
    value = value.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$").replace("`", "\\`")
    return '"' + value + '"'

def desktop_value(path):
    return str(path).replace("\\", "\\\\").replace("\n", "").replace("\r", "")

entry = """[Desktop Entry]
Type=Application
Name=Hearth AI Command Center (Experimental)
Comment=Test-only local AI dashboard for Hearth OS
Exec={exec_command}
Path={working_dir}
Icon={icon_path}
Terminal=true
Categories=Utility;Development;
StartupNotify=false
""".format(exec_command=desktop_argument(script), working_dir=desktop_value(here),icon_path=desktop_value(icon))
target.write_text(entry, encoding="utf-8")
PY
    chmod u+x "$HERE/launch.sh"
    printf 'Installed launcher: %s\n' "$TARGET"
    ;;
  *) printf 'Usage: %s [install|--remove]\n' "$0" >&2; exit 2 ;;
esac
