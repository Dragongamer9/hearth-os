#!/bin/bash
# Firehold on labwc's approved screens (1.16; docs/mockups/firehold-plasma 01-12, checklist
# docs/mockups/checklists/firehold-labwc.md), pictured on labwc on a virtual screen with the mockups' sample games,
# devices and people (firehold-plasma-fixtures.py, the same stand-ins Firehold on Plasma's pictures use).
#   labwc-firehold-mockups.sh OUT
# Pictures OUT/lw-NN-*.png (named as the mockups), next to each one the mockup itself (OUT/mockup-NN-*.png).
# Fails (::error, exit 1) when the shared Couch doesn't load, or when its files print QML errors.
set -u
OUT=${1:?out folder}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SHARE=${NIMBUS_SHARE:-/usr/share/nimbus}
SH="$SHARE/desktop/shell"
FHP="$SHARE/desktop/firehold-plasma/hearth-firehold-plasma"
HERE=$(cd "$(dirname "$0")" && pwd)
MOCK=${MOCKUPS:-$HERE/../../docs/mockups/firehold-plasma}
fail=0
err() { echo "::error title=Firehold on labwc mockups::$*"; echo "FAIL: $*" >>"$OUT/mockups.md"; fail=1; }
note() { echo "::notice title=Firehold on labwc mockups::$*"; echo "$*" >>"$OUT/mockups.md"; }
: >"$OUT/mockups.md"

export XDG_RUNTIME_DIR=/tmp/xdg-mock HOME=/tmp/home-mock LC_ALL=C.UTF-8 NIMBUS_SHARE="$SHARE"
rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.cache/hearth" "$HOME/Desktop"; chmod 700 "$XDG_RUNTIME_DIR"
echo '{"screenOffMinutes": 0}' >"$HOME/.local/share/nimbus/desktop.json"
: >"$HOME/.local/share/nimbus/welcome-done"
# The mockups' stand-ins: the library (hearth-firehold's cache), each game's art, the home's cards, the bars' line.
FX=/tmp/fhp-fixtures-lw
python3 "$HERE/firehold-plasma-fixtures.py" "$HOME" "$FX" || err "the stand-ins couldn't be written"
export HEARTH_FIREHOLD_HOME_FIXTURE="$FX/home.json" HEARTH_FIREHOLD_SYS_FIXTURE="$FX/sys.json"
# Two screens, as the mockups have them: the TV (HEADLESS-1) and the monitor (HEADLESS-2), side by side.
export HEARTH_FIREHOLD_TV_SCREENS=HEADLESS-1 HEARTH_FIREHOLD_SCREENS="HEADLESS-1:tv:TCL 55″ TV:3840 × 2160 · 60 Hz · HDMI 2,HEADLESS-2::LG 27GP850:2560 × 1440 · 165 Hz · DisplayPort 1"
# Quick Settings answers as mockups 07 (the Nvidia PC) and 08 (the ROG Ally) show (hearth-quick's CI fixture), a
# history from yesterday for the list (mockup 10), and a time zone where it's afternoon (the quiet hours aren't on).
export HEARTH_QUICK_FIXTURE=/tmp/lw-quick.json
qs_fixture() {   # desk | couch
  python3 - "$HEARTH_QUICK_FIXTURE" "$1" <<'PY'
import json, sys
desk = {"device": "Nvidia PC", "line": "Nvidia PC · plugged in · up 3 days", "battery": -1, "user": "Chris",
 "wifi": {"on": True, "ssid": "Home-5G", "strength": "strong"}, "bt": {"on": True, "connected": 3, "names": ["Xbox Controller", "Arctis 7", "MX Master"]},
 "osk": False, "night": {"on": True, "until": "7 AM"}, "dnd": {"on": False, "byHand": False, "schedule": True, "from": "11 PM", "to": "7 AM", "games": True, "family": True},
 "sinks": [{"name": "headset", "word": "Headset", "default": True, "volume": 64}, {"name": "hdmi", "word": "TV (HDMI)", "default": False, "volume": 50}, {"name": "spk", "word": "Speakers", "default": False, "volume": 40}],
 "brightness": [{"id": "lg", "label": "LG monitor", "percent": 80, "max": 100}], "power": "balanced",
 "powerNames": {"power-saver": "Power saver", "balanced": "Balanced", "performance": "Performance"},
 "devices": [{"name": "Xbox Controller", "kind": "pad", "percent": 72, "player": 1, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True},
             {"name": "Arctis 7", "kind": "headset", "percent": 60, "low": False}, {"name": "MX Master", "kind": "mouse", "percent": 40, "low": False}],
 "gameModeOff": False, "modes": [{"name": "HEADLESS-1", "tv": True, "label": "TCL TV", "mode": "couch"}, {"name": "HEADLESS-2", "tv": False, "label": "LG monitor", "mode": "desk"}]}
couch = dict(desk, device="ROG Ally", line="ROG Ally · battery 64% · up 2 h", battery=64, batteryMore={"left": "about 2 h 40 min", "limit": 80},
 bt={"on": True, "connected": 1, "names": ["Earbuds"]}, night={"on": False, "until": "9 PM"},
 sinks=[{"name": "buds", "word": "Earbuds", "default": True, "volume": 45}], brightness=[{"id": "", "label": "This screen", "percent": 70, "max": 100}],
 powerNames={"power-saver": "Silent · 10 W", "balanced": "Balanced · 18 W", "performance": "Turbo · 25 W"},
 devices=[{"name": "Earbuds", "kind": "headset", "percent": 70, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True}],
 gameModeOff=True, modes=[{"name": "HEADLESS-1", "tv": True, "label": "Built-in", "mode": "couch"}])
json.dump(desk if sys.argv[2] == "desk" else couch, open(sys.argv[1], "w"))
PY
}
qs_fixture desk
python3 - "$HOME/.local/state/nimbus/notifications.json" <<'PY'
import json, os, sys, time
os.makedirs(os.path.dirname(sys.argv[1]), exist_ok=True)
now = int(time.time() * 1000); D = 86400000
def n(k, app, summary, body, t):
    return {"key": k, "app": app, "summary": summary, "body": body, "icon": "", "entry": "", "image": "", "time": t, "lock": False, "urgency": "normal", "category": "", "progress": -1}
json.dump({"n:%d" % (now - D): n(now - D, "ROG Ally", "ROG Ally", "Saves backed up to this PC · 12 games", now - D - 3600000),
           "n:%d" % (now - D + 1): n(now - D + 1, "Controller 2", "Controller 2", "Battery low · 12%", now - D - 7200000)}, open(sys.argv[1], "w"), indent=2)
PY
export TZ=$(python3 -c 'import time; h=time.gmtime().tm_hour; print("Etc/GMT%+d" % -((14 - h + 12) % 24 - 12))')
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=2
export QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1
export HEARTH_WM=labwc HEARTH_FEDORA_LABWC=1 HEARTH_FIREHOLD_SESSION=1
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
eval "$(sed -n '/^# >>> data dirs/,/^# <<< data dirs/p' "${SESSION_FILE:-/usr/bin/nimbus-session}")" 2>/dev/null || true
python3 "$SH/hearth-browser" asked >/dev/null 2>&1 || true

sh -c 'HEARTH_DESKTOP_PID=$$; export HEARTH_DESKTOP_PID; exec labwc -C "$1"' sh "$SHARE/desktop/labwc" >"$OUT/labwc.log" 2>&1 &
LPID=$!
for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 --pos 0,0 >/dev/null 2>&1 || true
wlr-randr --output HEADLESS-2 --custom-mode 1920x1080 --pos 1920,0 >/dev/null 2>&1 || true
for i in $(seq 240); do [ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] && break; sleep 0.5; done
[ -e "$XDG_RUNTIME_DIR/hearth-shell-up" ] || { err "Mantel didn't come up: $(tail -c 1500 "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null | tr '\n' ' ')"; exit 1; }
SLOG="$XDG_RUNTIME_DIR/nimbus-shell.log"
python3 -c 'import gi, dbus' 2>/dev/null && { python3 "$FX/mpris.py" >"$OUT/mpris.log" 2>&1 & }
sleep 12

ipc() { timeout 20 quickshell ipc -p "$SH" call firehold "$@" >/dev/null 2>&1 || echo "ipc $* failed" >>"$OUT/ipc.txt"; sleep "${WAIT:-2}"; }
press() { for b in "$@"; do ipc press "$b"; done; }
shot() { if [ "${3:-}" = all ]; then grim "$OUT/$1.png"; else grim -o "${3:-HEADLESS-1}" "$OUT/$1.png"; fi 2>/dev/null || echo "no picture $1" >>"$OUT/ipc.txt"; cp "$MOCK/$2" "$OUT/mockup-$2" 2>/dev/null || true; }
fhp() { python3 "$FHP" "$@"; }

grep -q "the shared Couch didn't load" "$SLOG" && err "the approved Couch didn't load: $(grep -iE 'fhcouch|Couch.qml' "$SLOG" | head -6 | tr '\n' '|' | cut -c1-900)"
note "state: $(timeout 10 quickshell ipc -p "$SH" call firehold state 2>/dev/null | cut -c1-300)"

# ---- 01 Couch home, 02 further down (the low controller's notice), 03 game options from the Library
ipc couch; ipc ask cards; sleep 3
shot lw-01-couch-home 01-couch-home.png
press down down down down right
python3 "$HERE/firehold-plasma-fixtures.py" --low "$FX"; ipc ask cards; sleep 4
note "after the presses: $(timeout 10 quickshell ipc -p "$SH" call firehold state 2>/dev/null | cut -c1-200)"
shot lw-02-couch-home-rows 02-couch-home-rows.png
press b; ipc tab library; sleep 2
press right x down; sleep 2
shot lw-03-couch-game-options 03-couch-game-options.png
press b; ipc tab home; sleep 2

# ---- 06 Away (this screen Away: hearth-firehold-plasma's live modes; never a setting), then back with the controller
cp "$HOME/.config/nimbus/firehold.json" /tmp/fh-before.json 2>/dev/null
fhp mode all away --why pick; sleep 5
shot lw-06-away 06-away.png
press a; sleep 4
fhp modes >"$OUT/modes-after.json" 2>&1
grep -q '"mode": "away"' "$OUT/modes-after.json" && err "the controller didn't bring the screen back from Away: $(head -c 300 "$OUT/modes-after.json")"
python3 -c 'import json,sys; sys.exit(json.load(open(sys.argv[1])) != json.load(open(sys.argv[2])))' "$HOME/.config/nimbus/firehold.json" /tmp/fh-before.json || err "switching to Away and back wrote Firehold's settings (red flag 9): $(head -c 300 "$HOME/.config/nimbus/firehold.json")"
shot lw-06b-back-from-away 01-couch-home.png

# ---- 04 Modes for each screen: the TV Couch, the monitor Desk (the mouse is there), the Modes card from its pill
fhp mode HEADLESS-2 desk --why mouse; sleep 5
note "modes: $(fhp modes 2>/dev/null | head -c 300)"
ipc modes true; sleep 3
shot lw-04-modes-per-screen 04-modes-per-screen.png HEADLESS-2
grim "$OUT/lw-04b-both-screens.png" 2>/dev/null
ipc modes false

# ---- 07 Quick Settings on the Desk (the monitor), 08 with the controller on the Couch (the Ally's answers)
ipc quick; sleep 3
note "panels: $(timeout 10 quickshell ipc -p "$SH" call firehold panels 2>/dev/null)"
shot lw-07-quick-settings-desk 07-quick-settings-desk.png HEADLESS-2
ipc quick; sleep 1
qs_fixture couch
fhp mode all couch --why pad; sleep 4
press menu; sleep 3
press down down down down down down down down; sleep 1
shot lw-08-quick-settings-couch 08-quick-settings-couch.png HEADLESS-1
press menu; sleep 1
qs_fixture desk

# ---- 09 notification cards on the Desk: an urgent one stays, a picture (a recorded clip), a message with a reply
# box, a game that's ready, "2 more"; 10 the list from the clock
fhp mode HEADLESS-2 desk --why mouse; sleep 4
python3 - "$HERE/../../docs/mockups/firehold-plasma/06-away.png" /tmp/lw-clip.png <<'PY'
import sys
try:
    from PIL import Image
    Image.open(sys.argv[1]).convert('RGB').resize((640, 360)).save(sys.argv[2])
except Exception:
    pass
PY
cat > /tmp/lw-notify.py <<'PY'
import dbus, json, sys
n = dbus.Interface(dbus.SessionBus().get_object('org.freedesktop.Notifications', '/org/freedesktop/Notifications'), 'org.freedesktop.Notifications')
o = json.loads(sys.argv[1])
hints = {'urgency': dbus.Byte(o.get('urgency', 1))}
if o.get('image'): hints['image-path'] = o['image']
if o.get('reply'): hints['x-kde-reply-placeholder-text'] = o['reply']
print(n.Notify(o['app'], 0, '', o['summary'], o.get('body', ''), o.get('actions', []), hints, o.get('timeout', 30000)))
PY
nfy() { timeout 10 python3 /tmp/lw-notify.py "$1" >>"$OUT/notify.log" 2>&1 || echo "notify failed: $1" >>"$OUT/notify.log"; sleep 1; }
nfy '{"app": "Firefox", "summary": "Firefox download finished", "body": "hollow-pines.tar.zst · 2.9 GB"}'
nfy '{"app": "Hearth OS", "summary": "Hearth OS update ready", "body": "Nothing changes until you restart"}'
nfy '{"app": "Discord", "summary": "Ryder in #bfme-nights", "body": "“Edain game at 11? I will host.”", "actions": ["inline-reply", "Reply", "read", "Mark read"], "reply": "Reply to Ryder"}'
nfy '{"app": "Hearth Recorder", "summary": "Saved the last 30 seconds", "body": "Rise of the Witch-king · 1440p · 96 MB", "image": "/tmp/lw-clip.png", "actions": ["open", "Open", "share", "Copy a share link"]}'
nfy '{"app": "Downloads", "summary": "Helldivers 2 is ready", "body": "The 6.0 GB update finished (Steam).", "actions": ["play", "Play", "tv", "Play on the TV"]}'
nfy '{"app": "Hearth Link", "summary": "Dad\u2019s PC asks for help", "body": "“Trains 2010 won\u2019t start after the update.” He\u2019s at his PC now.", "urgency": 2, "actions": ["connect", "Connect (Remote desktop)", "later", "Call him back later"], "timeout": 0}'
sleep 2
shot lw-09-notification-cards 09-notification-cards.png HEADLESS-2
ipc list; sleep 3
shot lw-10-notification-history 10-notification-history.png HEADLESS-2
ipc list

# ---- 11 the Desk bar with the app's own menus (Hearth's menu source on labwc: hearth-globalmenu), File open, and the
# bar's first-run tip; 12 Hearth's standard menus for an app with none (Window › Move to screen), the controller hint.
# The apps are stand-ins: a Qt Widgets "Hearth Files" with the mockup's menus (it puts them in the menu registrar as
# KDE's apps do) and a "Trains 2010" window with no menus.
fhp mode HEADLESS-2 desk --why mouse; sleep 3
mkdir -p "$HOME/.local/share/applications"
printf '[Desktop Entry]\nType=Application\nName=Hearth Files\nExec=hearth-files\nIcon=system-file-manager\n' >"$HOME/.local/share/applications/hearth-files.desktop"
cat > /tmp/lw-files.py <<'PY'
import sys
from PyQt6.QtWidgets import QApplication, QMainWindow, QLabel
from PyQt6.QtGui import QAction, QKeySequence
from PyQt6.QtCore import Qt
app = QApplication(sys.argv)
app.setDesktopFileName('hearth-files')
app.setApplicationName('Hearth Files')
w = QMainWindow(); w.setWindowTitle('Hearth Files — Pictures'); w.resize(1260, 760)
w.setCentralWidget(QLabel('Pictures'))
mb = w.menuBar()
def menu(name, rows):
    m = mb.addMenu(name)
    for r in rows:
        if r is None: m.addSeparator(); continue
        if isinstance(r, tuple) and isinstance(r[1], list):
            s = m.addMenu(r[0])
            for x in r[1]: s.addAction(x)
            continue
        a = QAction(r[0], w)
        if r[1]: a.setShortcut(QKeySequence(r[1]))
        m.addAction(a)
menu('&File', [('New window', 'Ctrl+N'), ('New tab', 'Ctrl+T'), ('New folder…', 'Ctrl+Shift+N'), ('New file', ['Text file', 'Spreadsheet']), None,
               ('Open with', ['Hearth Photos', 'Hearth Paint']), ('Open in Terminal', 'Shift+F4'), ('Send to', ["Dad's PC", 'ROG Ally']), None,
               ('Copy location', 'Ctrl+Alt+C'), ('Properties', 'Alt+Return'), None, ('Close tab', 'Ctrl+W'), ('Quit', 'Ctrl+Q')])
for n in ('&Edit', '&View', '&Go', '&Bookmarks', '&Help'):
    menu(n, [('Something', '')])
w.show()
sys.exit(app.exec())
PY
cat > /tmp/lw-trains.py <<'PY'
import sys
from PyQt6.QtWidgets import QApplication, QWidget
app = QApplication(sys.argv)
app.setDesktopFileName('trains2010')
w = QWidget(); w.setWindowTitle('Trains 2010'); w.resize(1280, 800); w.setStyleSheet('background: #8a6a3a'); w.show()
sys.exit(app.exec())
PY
if python3 -c 'import PyQt6.QtWidgets' 2>/dev/null; then
  # (as an app from the menu starts on labwc: Mantel's Qt look, qt6ct, which puts its menus in the registrar)
  QT_LOGGING_RULES="qt.qpa.menus*=true;qt.qpa.theme*=true" QT_QPA_PLATFORMTHEME=qt6ct GIO_LAUNCHED_DESKTOP_FILE="$HOME/.local/share/applications/hearth-files.desktop" \
    python3 /tmp/lw-files.py >"$OUT/files-standin.log" 2>&1 & FPID=$!
  sleep 6
  on_monitor() {   # the window on the monitor (labwc's Move to output, Super+Shift+→), in front
    timeout 10 quickshell ipc -p "$SH" call firehold focus "$1" >/dev/null 2>&1; sleep 1
    [ "$(timeout 10 quickshell ipc -p "$SH" call firehold screenOf "$1" 2>/dev/null)" = HEADLESS-2 ] || { wtype -M logo -M shift -k Right -m shift -m logo; sleep 2; }
    timeout 10 quickshell ipc -p "$SH" call firehold focus "$1" >/dev/null 2>&1; sleep 2
  }
  on_monitor "Hearth Files — Pictures"
  note "the Desk bar: $(timeout 10 quickshell ipc -p "$SH" call firehold bar 2>/dev/null) · registered: $(cat "$XDG_RUNTIME_DIR/hearth-globalmenu.json" 2>/dev/null | head -c 300) · its menus: $(python3 "$SH/hearth-globalmenu" layout hearth-files 2>&1 | head -c 200) · Qt: $(grep -iE 'menu|theme|dbus' "$OUT/files-standin.log" | head -4 | cut -c1-120 | tr '\n' '|')"
  ipc barmenu File; sleep 2
  ipc bartip true; sleep 2
  shot lw-11-desk-bar-app-menus 11-desk-bar-app-menus.png HEADLESS-2
  ipc barmenu ""; ipc bartip false
  python3 /tmp/lw-trains.py >"$OUT/trains-standin.log" 2>&1 & TPID=$!
  sleep 5; on_monitor "Trains 2010"
  # a controller button while every screen is Desk: the hint; then the TV back to Couch, as the mockup has it
  fhp mode all desk --why mouse; sleep 3
  press x; sleep 1
  fhp mode HEADLESS-1 couch --why pad; sleep 2
  ipc barmenu move; sleep 1
  shot lw-12-desk-bar-standard-menus 12-desk-bar-standard-menus.png HEADLESS-2
  note "the Desk bar (no menus of its own): $(timeout 10 quickshell ipc -p "$SH" call firehold bar 2>/dev/null)"
  ipc barmenu help; sleep 2
  shot lw-12b-desk-bar-help 12-desk-bar-standard-menus.png HEADLESS-2
  ipc barmenu ""
  kill "$FPID" "$TPID" 2>/dev/null
else
  err "11-12: PyQt6 isn't here for the stand-in apps"
fi

note "the clip's notification in the history: $(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print([v.get("image") for v in d.values() if v.get("app") == "Hearth Recorder"])' "$HOME/.local/state/nimbus/notifications.json" 2>&1 | cut -c1-200)"

# ---- 05 Settings › Couch, Desk and Away (Hearth's Settings, the Rust app: in the image, not in this container), on
# labwc, in the controller look the mockup draws; the stand-in helper answers with the mockup's two screens.
if command -v hearth-settings-app >/dev/null 2>&1; then
  fhp mode all desk --why pick; sleep 4
  STUB=$(mktemp -d)
  cat > "$STUB/hearth-firehold-plasma" <<'SH'
#!/bin/sh
case "$1" in
  get) echo '{"awayAfter": 20, "autoPad": true, "autoPadScope": "tv", "autoMouse": true, "games": "follow"}' ;;
  modes) echo '{"screens": [{"name": "HEADLESS-1", "tv": true, "label": "TCL 55″ TV", "spec": "3840 × 2160 · 60 Hz · HDMI 2 on the RTX 5070", "mode": "couch", "why": "pad"}, {"name": "HEADLESS-2", "tv": false, "label": "LG 27GP850", "spec": "2560 × 1440 · 165 Hz · DisplayPort 1", "mode": "desk", "why": "mouse"}], "autoPad": true, "autoMouse": true, "awayAfter": 20}' ;;
  *) exit 0 ;;
esac
SH
  chmod +x "$STUB/hearth-firehold-plasma"
  for v in "1-modes:" "2-switching:right down down down down down down down down down" "3-games:right down down down down down down down down down down down down down down down down"; do
    n=${v%%:*}; pr=${v#*:}
    if [ -n "$pr" ]; then PATH="$STUB:$PATH" hearth-settings-app --page couch-desk-away --layout b --press "$pr" >"$OUT/settings-$n.log" 2>&1 &
    else PATH="$STUB:$PATH" hearth-settings-app --page couch-desk-away --layout b >"$OUT/settings-$n.log" 2>&1 & fi
    sp=$!; sleep 7
    shot lw-05-settings-modes-$n 05-settings-modes.png all
    kill "$sp" 2>/dev/null; sleep 1
  done
  rm -rf "$STUB"
else
  note "05: Hearth's Settings app isn't in this container (pictured in the image run)"
fi

# ---- QML errors from the shared Couch's files or the labwc host
errs=$(grep -E 'fhcouch/|FhpCouch|FhCouchHost|FhMedia|FhIcon' "$SLOG" | grep -iE 'error|TypeError|ReferenceError|is not defined|Cannot|Unable|failed' | sort | uniq -c | sort -rn | head -12)
[ -z "$errs" ] || err "QML messages from the approved Couch: $(echo "$errs" | cut -c1-200 | tr '\n' '|')"
note "presses: $(grep -E 'firehold: (press|Couch on)' "$SLOG" | sed 's/\x1b\[[0-9;]*m//g' | head -6 | cut -c1-160 | tr '\n' '|')"
grep -v -E 'Frame rendered|Frame prepared' "$SLOG" | tail -c 200000 >"$OUT/shell.log" 2>/dev/null
note "pictures: $(ls "$OUT"/lw-*.png 2>/dev/null | wc -l)"
kill "$LPID" 2>/dev/null; sleep 2; pkill -x quickshell 2>/dev/null
exit "$fail"
