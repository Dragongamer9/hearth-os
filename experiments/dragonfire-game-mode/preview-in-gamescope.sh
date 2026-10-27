#!/bin/sh
# Run a *browser preview* of Dragonfire inside a nested Gamescope window.
# Does not replace Hearth Game Mode or modify login/session/boot files.
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd -P)
if ! command -v gamescope >/dev/null 2>&1; then
  echo "gamescope not installed; preview not started" >&2
  exit 2
fi
browser=""
for candidate in chromium chromium-browser google-chrome firefox; do
  if command -v "$candidate" >/dev/null 2>&1; then browser=$candidate; break; fi
done
if [ -z "$browser" ]; then
  echo "No supported browser available (Chromium or Firefox)" >&2
  exit 2
fi
echo "DRAGONFIRE PREVIEW ONLY: this will NOT launch your games or replace Hearth."
echo "Press Alt+F4 to close the browser and return to your desktop."
# Desktop nested mode only. The real Hearth host bridge is unavailable here.
url=$(python3 - "$HERE/index.html" <<'PY'
from pathlib import Path
import sys
print(Path(sys.argv[1]).resolve().as_uri())
PY
)
case "$browser" in
  firefox) exec gamescope -W 1280 -H 800 -- firefox --new-window "$url" ;;
  *) exec gamescope -W 1280 -H 800 -- "$browser" --user-data-dir="${XDG_RUNTIME_DIR:-/tmp}/dragonfire-preview-$(id -u)" --no-first-run --new-window "$url" ;;
esac
