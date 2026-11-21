#!/bin/bash
# CI only: small tools for checks on a KWin with a virtual screen (hearth-apps-rust-kwin.sh, plasma-hearth-shots.sh).
#   bash ci-kwin-tools.sh install     (as root, once) builds ci-fake-input (KWin's fake input: keys and the pointer)
#                                     and a copy of Python, ci-kwin-shot-py, that KWin lets take screenshots: each gets
#                                     its own .desktop file naming it, the way KWin allows any program (by its path).
#                                     KWin's own permission checks stay on, so whatever else asks KWin is checked.
#   source ci-kwin-tools.sh           kshot FILE [screen NAME|window UUID] (a PNG from KWin), kwins FILE (KWin's
#                                     windows now, one line each: class | caption | x,y wxh | fullscreen | keepAbove |
#                                     excludeFromCapture | screen | uuid; KWin must run with QT_LOGGING_RULES=js.debug=true
#                                     and its output in $KWIN_LOG), fake CMD... (ci-fake-input)
if [ "${1:-}" = install ]; then
  set -e
  here=$(cd "$(dirname "$0")" && pwd)
  gen=$(mktemp -d)
  xml=$(ls /usr/share/plasma-wayland-protocols/fake-input.xml 2>/dev/null | head -1)
  [ -n "$xml" ] || { echo "::error title=ci tools::no fake-input.xml (plasma-wayland-protocols)"; exit 1; }
  wayland-scanner client-header "$xml" "$gen/fake-input-client-protocol.h"
  wayland-scanner private-code "$xml" "$gen/fake-input-protocol.c"
  cc -O2 -Wall -I"$gen" -o /usr/local/bin/ci-fake-input "$here/ci-fake-input.c" "$gen/fake-input-protocol.c" $(pkg-config --cflags --libs wayland-client)
  py=$(readlink -f /usr/bin/python3)
  cp "$py" /usr/bin/ci-kwin-shot-py
  mkdir -p /usr/share/applications
  printf '[Desktop Entry]\nType=Application\nName=CI fake input\nExec=/usr/local/bin/ci-fake-input\nNoDisplay=true\nX-KDE-Wayland-Interfaces=org_kde_kwin_fake_input\n' > /usr/share/applications/ci-fake-input.desktop
  printf '[Desktop Entry]\nType=Application\nName=CI screenshots\nExec=/usr/bin/ci-kwin-shot-py\nNoDisplay=true\nX-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2\n' > /usr/share/applications/ci-kwin-shot.desktop
  cat > /usr/local/share/ci-kwin-shot.py <<'PY'
import dbus, os, sys
from PIL import Image
out, kind = sys.argv[1], (sys.argv[2] if len(sys.argv) > 2 else 'workspace')
i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
o = {'native-resolution': True}
if kind == 'screen':
    res = i.CaptureScreen(sys.argv[3], o, dbus.types.UnixFd(w), timeout=15)
elif kind == 'window':
    res = i.CaptureWindow(sys.argv[3], dict(o, **{'include-decoration': True}), dbus.types.UnixFd(w), timeout=15)
else:
    res = i.CaptureWorkspace(o, dbus.types.UnixFd(w), timeout=15)
os.close(w)
data = b''
while True:
    c = os.read(r, 1 << 20)
    if not c:
        break
    data += c
W, H, S = int(res['width']), int(res['height']), int(res['stride'])
raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(out)
PY
  command -v kbuildsycoca6 >/dev/null && kbuildsycoca6 --noincremental >/dev/null 2>&1
  rm -rf "$gen"
  echo "::notice title=ci tools::ci-fake-input and ci-kwin-shot-py are in"
  exit 0
fi

kshot() { timeout 25 /usr/bin/ci-kwin-shot-py /usr/local/share/ci-kwin-shot.py "$@" 2>>"${OUT:-/tmp}/kshot-errors.txt" || echo "no screenshot $1" >>"${OUT:-/tmp}/kshot-errors.txt"; }
fake() { timeout 60 /usr/local/bin/ci-fake-input "$@" 2>>"${OUT:-/tmp}/fake-input.txt"; }
cat > /tmp/ci-kwins.js <<'JS'
var ws = workspace.stackingOrder;
print("CIWIN ---- " + ws.length);
for (var i = 0; i < ws.length; i++) {
  var w = ws[i], g = w.frameGeometry;
  print("CIWIN " + w.resourceClass + " | " + w.caption + " | " + Math.round(g.x) + "," + Math.round(g.y) + " " + Math.round(g.width) + "x" + Math.round(g.height)
        + " | fullscreen=" + w.fullScreen + " | keepAbove=" + w.keepAbove + " | excludeFromCapture=" + (('excludeFromCapture' in w) ? w.excludeFromCapture : 'n/a')
        + " | screen=" + (w.output ? w.output.name : '?') + " | " + w.internalId);
}
print("CIWIN ==== end");
JS
kwins() {   # kwins FILE
  local id
  id=$(dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript "string:/tmp/ci-kwins.js" "string:ciwins$RANDOM" 2>/dev/null | awk '/int32/{print $2}')
  [ -n "$id" ] || { echo "no KWin script" > "$1"; return; }
  dbus-send --session --print-reply --dest=org.kde.KWin "/Scripting/Script$id" org.kde.kwin.Script.run >/dev/null 2>&1; sleep 1.5
  # Only the last listing.
  grep -a "CIWIN" "${KWIN_LOG:-/dev/null}" | sed 's/.*CIWIN //' | awk '/^---- /{buf=""} {buf=buf $0 "\n"} END{printf "%s", buf}' > "$1"
  dbus-send --session --print-reply --dest=org.kde.KWin "/Scripting/Script$id" org.kde.kwin.Script.stop >/dev/null 2>&1
}
