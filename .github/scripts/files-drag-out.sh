#!/bin/bash
# Dragging a file out of Hearth Files (the pure-Rust app, hosts/hearth-ui/apps/files, dragout.rs) into another app's
# window, on KDE's own KWin on a virtual 1920x1080 screen (hearth-apps-rust.yml's drag-out job):
#   files-drag-out.sh BIN OUT
# Files on the left with one file, a Qt (QML) app's drop target on the right; KWin's fake input presses on the file,
# moves to the target and lets go. Checks (::error::): the target got file:///…/drop-me.txt, and Files stays at rest
# (under 0.3 % of a core) before and after. Pictures: OUT/files-36-drag-out-start.png, -37-dragging, -38-dropped.
set +e +o pipefail
BIN=$1; OUT=${2:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
ROOT=$PWD
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }
export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Outbox" && chmod 700 "$XDG_RUNTIME_DIR"
printf 'A file that goes to another app\n' > "$HOME/Outbox/drop-me.txt"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 KWIN_WAYLAND_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1 LANG=C.UTF-8
export HEARTH_SHELL_DIR="$ROOT/hosts/linux/desktop/shell" HEARTH_PALETTES="$ROOT/hosts/linux/desktop/theme/palettes.json"
mkdir -p "$HOME/.local/share/nimbus" && printf '{"look": "steam", "mode": "dark"}\n' > "$HOME/.local/share/nimbus/desktop.json"

# KWin's fake input for the pointer (files-fakeinput.c, test tooling built here).
FAKE=/usr/local/bin/hearth-ci-files-fakeinput
FIP=$(ls /usr/share/plasma-wayland-protocols/fake-input.xml 2>/dev/null | head -1)
wayland-scanner client-header "$FIP" /tmp/fake-input-client-protocol.h && wayland-scanner private-code "$FIP" /tmp/fake-input-protocol.c \
  && gcc -O1 -I/tmp -o "$FAKE" "$ROOT/.github/scripts/files-fakeinput.c" /tmp/fake-input-protocol.c -lwayland-client -lm >"$OUT/fakeinput-build.txt" 2>&1
[ -x "$FAKE" ] || { fail "fake input" "not built: $(tail -c 600 "$OUT/fakeinput-build.txt" | tr '\n' '|') xml=$FIP"; exit 0; }
printf '[Desktop Entry]\nType=Application\nName=Hearth CI fake input\nExec=%s\nNoDisplay=true\nX-KDE-Wayland-Interfaces=org_kde_kwin_fake_input\n' "$FAKE" \
  > /usr/share/applications/org.hearthos.ci-files-fakeinput.desktop

cat > /tmp/shot.py <<'PY'
import dbus, os, sys
from PIL import Image
i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
res = i.CaptureWorkspace({'native-resolution': True, 'include-cursor': True}, dbus.types.UnixFd(w), timeout=15)
os.close(w)
data = b''
while True:
    c = os.read(r, 1 << 20)
    if not c:
        break
    data += c
W, H, S = int(res['width']), int(res['height']), int(res['stride'])
raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(sys.argv[1])
PY
shot() { timeout 25 python3 /tmp/shot.py "$OUT/$1.png" 2>>"$OUT/shot-errors.txt" || echo "no screenshot $1" >>"$OUT/shot-errors.txt"; }
kn=0
kwin_js() {
  kn=$((kn + 1)); f=/tmp/ci-$kn.js; printf '%s\n' "$1" > "$f"
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript string:"$f" string:"ci$kn" >/dev/null
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.start >/dev/null
  sleep 1
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.unloadScript string:"ci$kn" >/dev/null
}
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
rest() {   # rest PID SECONDS: % of a core
  a=$(ticks $1); sleep $2; b=$(ticks $1)
  python3 -c "print('%.2f' % (100.0 * ($b - $a) / $(getconf CLK_TCK) / $2))"
}

cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin.log" 2>&1 &
KW=$!
for i in $(seq 1 90); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start (running: $(kill -0 $KW 2>/dev/null && echo yes || echo no); sockets: $(ls $XDG_RUNTIME_DIR | tr '\n' ' ')): $(tail -c 1500 "$OUT/kwin.log" | tr '\n' '|')"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
sleep 2
# The pointer exists once the fake input device does (KWin adds the seat's pointer then): a first move.
"$FAKE" click 1900 1060 >>"$OUT/fakeinput.log" 2>&1

"$BIN/hearth-files-app" "$HOME/Outbox" --view xlarge >"$OUT/files.log" 2>&1 &
FILES=$!
cat > /tmp/target.qml <<'QML'
import QtQuick
import QtQuick.Window
Window {
    id: w
    title: "Drop target"
    width: 960; height: 1080; visible: true
    color: "#1d2128"
    property string got: ""
    property bool over: false
    Rectangle {
        anchors.fill: parent; anchors.margins: 40; radius: 16
        color: w.over ? "#24476b" : "#252b34"; border.color: "#5aa7e6"; border.width: 2
        Text {
            anchors.centerIn: parent; width: parent.width - 60
            wrapMode: Text.WrapAnywhere; horizontalAlignment: Text.AlignHCenter
            color: "white"; font.pixelSize: 26
            text: w.got === "" ? (w.over ? "Let go to drop it here" : "Another app's window (Qt)\nDrop files here") : "Dropped here:\n\n" + w.got
        }
    }
    DropArea {
        anchors.fill: parent
        onEntered: drag => { w.over = true; drag.accept(Qt.CopyAction); }
        onExited: w.over = false
        onDropped: drop => { w.over = false; w.got = drop.urls.map(u => String(u)).join("\n"); console.log("DROPPED " + w.got); drop.accept(Qt.CopyAction); }
    }
}
QML
QML=$(command -v qml6 || command -v qml || ls /usr/lib64/qt6/bin/qml 2>/dev/null | head -1)
"$QML" /tmp/target.qml >"$OUT/target.log" 2>&1 &
sleep 5
kwin_js "var l = workspace.windowList(); for (var i = 0; i < l.length; i++) { var c = String(l[i].caption); if (c.indexOf('Hearth Files') >= 0) l[i].frameGeometry = {x: 0, y: 0, width: 960, height: 1080}; if (c.indexOf('Drop target') >= 0) l[i].frameGeometry = {x: 960, y: 0, width: 960, height: 1080}; }"
sleep 2
kill -0 $FILES 2>/dev/null || { fail files "Hearth Files ended: $(tail -c 800 "$OUT/files.log" | tr '\n' '|')"; exit 0; }
cpu0=$(rest $FILES 15)
shot files-36-drag-out-start
# Press on drop-me.txt (its extra large icon), move across to the other app's window, hold, let go.
"$FAKE" drag 296 240 1440 540 40 2500 >>"$OUT/fakeinput.log" 2>&1 &
DRAG=$!
sleep 3.4
shot files-37-dragging
wait $DRAG
sleep 2
shot files-38-dropped
cpu1=$(rest $FILES 15)
got=$(grep -o 'DROPPED .*' "$OUT/target.log" | head -1)
echo "::notice title=drag out::Files at rest ${cpu0}% before and ${cpu1}% of a core after; the target got: ${got:-nothing}; Files said: $(grep -i drag "$OUT/files.log" | head -3 | tr '\n' '|')"
case "$got" in
  *file:///tmp/home/Outbox/drop-me.txt*) ;;
  *) fail "drag out" "the other app didn't get file:///tmp/home/Outbox/drop-me.txt (got: ${got:-nothing}); fake input: $(tail -c 300 "$OUT/fakeinput.log" | tr '\n' '|'); target: $(tail -c 400 "$OUT/target.log" | tr '\n' '|')" ;;
esac
for c in "$cpu0" "$cpu1"; do python3 -c "import sys; sys.exit(0 if float('$c') < 0.3 else 1)" || fail "drag out" "Hearth Files used ${c}% of a core at rest"; done
kill $FILES 2>/dev/null
exit 0
