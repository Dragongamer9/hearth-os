#!/bin/bash
# Firehold on Plasma's screens and its switching, on KDE's own KWin on a virtual 1920x1080 screen (firehold-plasma.yml).
#   firehold-plasma-shots.sh OUT
# Pictures: OUT/fp-1-couch-home.png ... fp-8-fallback-plain-plasma.png. Checks (::error:: and OUT/FAILED):
#   * the Couch (org.hearthos.firehold) and the Desk (org.hearthos.firehold.desk) come up;
#   * Couch → Desk → Couch keeps every app and the game open (same process ids) and KWin running (the same one);
#   * a Couch that can't start goes to plain KDE Plasma by itself, the apps still open, and the choice is kept;
#   * no QML errors from Firehold's files in plasmashell's messages.
set +e +o pipefail
OUT=${1:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/step.log") 2>&1
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }

export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Pictures/Trips" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1
export HEARTH_FIREHOLD_PLASMA=1 FIREHOLD_PLASMA_SHELL_LOG="$OUT/plasmashell.log"
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
RUN="$XDG_RUNTIME_DIR/hearth-firehold-plasma"

# ---- the mockups' sample games, devices and people (firehold-plasma-fixtures.py; no Hearth app or Steam here).
FX=/tmp/fhp-fixtures
python3 "$(dirname "$0")/firehold-plasma-fixtures.py" "$HOME" "$FX" || fail fixtures "the stand-ins couldn't be written"
export HEARTH_FIREHOLD_HOME_FIXTURE="$FX/home.json" HEARTH_FIREHOLD_SYS_FIXTURE="$FX/sys.json" HEARTH_FIREHOLD_TV_SCREENS=Virtual-0,Virtual-1
key() { for k in "$@"; do hearth-firehold-plasma ask "key:$k"; sleep 1; done; }

# ---- a stand-in game window: "Rise of the Witch-king" (the library matches it by its title).
cat > /tmp/ember.qml <<'QML'
import QtQuick
import QtQuick.Window
Window {
    title: "Rise of the Witch-king"
    width: 1280; height: 720; visible: true
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: "#2b1a2e" }
            GradientStop { position: 0.5; color: "#5b2b1a" }
            GradientStop { position: 1.0; color: "#a3541e" }
        }
    }
    Text { anchors.centerIn: parent; text: "The game, full screen, drawn straight to the display"; color: "#c8f6ede6"; font.pixelSize: 30 }
}
QML
QML=$(command -v qml6 || command -v qml || ls /usr/lib64/qt6/bin/qml 2>/dev/null | head -1)

# ---- KWin's screenshot interface (KWIN_SCREENSHOT_NO_PERMISSION_CHECKS lets this use it).
cat > /tmp/shot.py <<'PY'
import dbus, os, sys
from PIL import Image
i = dbus.Interface(dbus.SessionBus().get_object('org.kde.KWin', '/org/kde/KWin/ScreenShot2'), 'org.kde.KWin.ScreenShot2')
r, w = os.pipe()
res = i.CaptureWorkspace({'native-resolution': True}, dbus.types.UnixFd(w), timeout=15)
os.close(w)
data = b''
while True:
    c = os.read(r, 1 << 20)
    if not c:
        break
    data += c
W, H, S = int(res['width']), int(res['height']), int(res['stride'])
# QImage's formats: 4-6 are ARGB32 (B, G, R, A in memory), 16-18 RGBX/RGBA8888 (R, G, B, A).
raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
im = Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB')
if len(sys.argv) > 2:   # one screen of several: X0 WIDTH (KWin's workspace is the screens side by side)
    x0, w = int(sys.argv[2]), int(sys.argv[3])
    im = im.crop((x0, 0, x0 + w, H))
im.save(sys.argv[1])
print('format', int(res.get('format', -1)), raw, file=sys.stderr)
PY
shot() { timeout 25 python3 /tmp/shot.py "$OUT/$1.png" "${@:2}" 2>>"$OUT/shot-errors.txt" || echo "no screenshot $1" >>"$OUT/shot-errors.txt"; }
# A one-off KWin script (only for these checks: putting the stand-in game in front, ending Show Desktop).
kn=0
kwin_js() {
  kn=$((kn + 1)); f=/tmp/ci-$kn.js; printf '%s\n' "$1" > "$f"
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript string:"$f" string:"ci$kn" >/dev/null
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.start >/dev/null
  sleep 1
  dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.unloadScript string:"ci$kn" >/dev/null
}
front() {   # front CAPTION FULLSCREEN(true|false)
  kwin_js "workspace.showingDesktop = false; var l = workspace.windowList(); for (var i = 0; i < l.length; i++) if (String(l[i].caption).indexOf('$1') >= 0) { workspace.activeWindow = l[i]; l[i].fullScreen = $2; }"
}
since=0
wait_ready() {   # wait_ready PACKAGE SECONDS: the package said it's up after $since
  for i in $(seq 1 "$2"); do
    f="$RUN/ready-$1"
    [ -f "$f" ] && [ "$(stat -c %Y "$f")" -ge "$since" ] && return 0
    sleep 1
  done
  return 1
}
apps_pids() { for p in dolphin konsole kate "$(basename "$QML")"; do pgrep -x "$p" | head -1; done | tr '\n' ' '; }

# KWin's own fake input (org_kde_kwin_fake_input) for presses and clicks in the pictures: firehold-fakeinput.c, test
# tooling, built here; KWin offers it only to a program whose desktop file asks for it.
FAKE=/usr/local/bin/hearth-ci-fakeinput
FIP=$(ls /usr/share/plasma-wayland-protocols/fake-input.xml 2>/dev/null | head -1)
if command -v gcc >/dev/null && command -v wayland-scanner >/dev/null && [ -n "$FIP" ]; then
  wayland-scanner client-header "$FIP" /tmp/fake-input-client-protocol.h && wayland-scanner private-code "$FIP" /tmp/fake-input-protocol.c \
    && gcc -O1 -I/tmp -o "$FAKE" "$(dirname "$0")/firehold-fakeinput.c" /tmp/fake-input-protocol.c -lwayland-client -lm >"$OUT/fakeinput-build.txt" 2>&1
  printf '[Desktop Entry]\nType=Application\nName=Hearth CI fake input\nExec=%s\nNoDisplay=true\nX-KDE-Wayland-Interfaces=org_kde_kwin_fake_input\n' "$FAKE" \
    > /usr/share/applications/org.hearthos.ci-fakeinput.desktop
fi
[ -x "$FAKE" ] || echo "fake input not built: gcc $(command -v gcc) scanner $(command -v wayland-scanner) xml $FIP" >> "$OUT/fakeinput-build.txt"

# ---- KWin (KDE's own, unchanged) on a virtual screen, PipeWire for window pictures, Plasma's activities.
for p in pipewire wireplumber; do command -v $p >/dev/null && { $p >"$OUT/$p.log" 2>&1 & }; done
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin.log" 2>&1 &
KWIN=$!
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start: $(tail -5 "$OUT/kwin.log" | tr '\n' '|' | cut -c1-600)"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$OUT/kamd.log" 2>&1 & break; }; done
# KDE's global shortcuts service (the Plasma session starts it; the KWin script's shortcuts go through it).
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"$OUT/kglobalaccel.log" 2>&1 & break; }; done
sleep 3

# Hearth's shared Quick Settings over the Couch answers as mockup 08 shows (a ROG Ally: its battery, earbuds, its own
# power steps, Game Mode switched off); the screen in Couch.
export HEARTH_QUICK_FIXTURE=/tmp/fhp-quick-couch.json
python3 - "$HEARTH_QUICK_FIXTURE" <<'PY'
import json, sys
json.dump({"device": "ROG Ally", "line": "ROG Ally · battery 64% · up 2 h", "battery": 64, "user": "Chris", "batteryMore": {"left": "about 2 h 40 min", "limit": 80},
 "wifi": {"on": True, "ssid": "Home-5G", "strength": "strong"}, "bt": {"on": True, "connected": 1, "names": ["Earbuds"]},
 "osk": False, "night": {"on": False, "until": "9 PM"}, "dnd": {"on": False, "byHand": False, "schedule": True, "from": "11 PM", "to": "7 AM", "games": True, "family": True},
 "sinks": [{"name": "buds", "word": "Earbuds", "default": True, "volume": 45}], "brightness": [{"id": "", "label": "This screen", "percent": 70, "max": 100}],
 "power": "balanced", "powerNames": {"power-saver": "Silent · 10 W", "balanced": "Balanced · 18 W", "performance": "Turbo · 25 W"},
 "devices": [{"name": "Earbuds", "kind": "headset", "percent": 70, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True}],
 "gameModeOff": True, "modes": [{"name": "Virtual-0", "tv": False, "label": "Built-in", "mode": "couch"}, {"name": "Virtual-1", "tv": False, "label": "Built-in", "mode": "couch"}]},
 open(sys.argv[1], "w"))
PY

# ---- the Couch, as nimbus-session starts it.
hearth-firehold-plasma begin org.hearthos.firehold
since=$(date +%s)
PLASMA_DEFAULT_SHELL=org.hearthos.firehold plasmashell -p org.hearthos.firehold >>"$OUT/plasmashell.log" 2>&1 &
PLASMA_DEFAULT_SHELL=org.hearthos.firehold hearth-firehold-plasma session >"$OUT/session.log" 2>&1 &
wait_ready org.hearthos.firehold 90 && echo "::notice title=Couch::up in $(( $(date +%s) - since )) s" \
  || fail Couch "org.hearthos.firehold didn't say it's up: $(grep -iE 'firehold|error' "$OUT/plasmashell.log" | head -8 | tr '\n' '|' | cut -c1-900)"
sleep 4
shot fp-0-couch-first
# ---- the Couch home (mockups firehold-plasma 01-03), before anything else is open.
command -v python3 >/dev/null && python3 -c 'import gi, dbus' 2>/dev/null && { python3 "$FX/mpris.py" >"$OUT/mpris.log" 2>&1 & }
sleep 3
hearth-firehold-plasma ask cards; sleep 4
echo "::notice title=Screen::$(QT_QPA_PLATFORM=wayland timeout 5 kscreen-doctor -o 2>/dev/null | head -3 | tr '\n' ' ')"
shot fp-01-couch-home
key down down down down right; sleep 1
python3 "$(dirname "$0")/firehold-plasma-fixtures.py" --low "$FX"; hearth-firehold-plasma ask cards; sleep 4
shot fp-02-couch-home-rows
key b
# Quick Settings with the controller over the Couch (mockup 08): the Couch's ≡ (the Menu key); B (Escape) closes it.
key up up up up; sleep 1
echo "added: $(hearth-firehold-plasma ready org.hearthos.firehold 2>&1 | tail -2)" > "$OUT/couch-quick.txt"; sleep 4
echo "Couch's widgets: $(dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript string:'print(desktops().map(function(d){return d.type + ": " + d.widgets().map(function(w){return w.type;}).join(",");}).join(" | "))' 2>&1 | tail -1)" >> "$OUT/couch-quick.txt"
command -v wtype >/dev/null && timeout 5 wtype -k Menu || hearth-firehold-plasma do open-applet org.hearthos.quicksettings >>"$OUT/couch-quick.txt"
sleep 4
shot fp-08-quick-settings-couch
command -v wtype >/dev/null && timeout 5 wtype -k Escape || hearth-firehold-plasma do open-applet org.hearthos.quicksettings >/dev/null
sleep 2
shot fp-08b-quick-settings-closed
hearth-firehold-plasma ask tab:library; sleep 3
key right x down; sleep 2
shot fp-03-couch-game-options
key b; hearth-firehold-plasma ask tab:home; sleep 2

# ---- open apps and the game, then the Couch in front again (KWin's Show Desktop, as Home does).
dolphin "$HOME/Pictures" >"$OUT/dolphin.log" 2>&1 &
konsole >/dev/null 2>&1 &
kate >/dev/null 2>&1 &
[ -n "$QML" ] && { "$QML" /tmp/ember.qml >"$OUT/game.log" 2>&1 & }
sleep 15
PIDS=$(apps_pids)

echo "::notice title=Open::apps and game: $PIDS"
hearth-firehold-plasma do home; sleep 4
shot fp-1-couch-home
hearth-firehold-plasma ask tab:library; sleep 4
shot fp-5-library
hearth-firehold-plasma ask open; sleep 6
shot fp-4-everything-open
hearth-firehold-plasma ask open; sleep 1
# The game full screen in front, then the Guide over it.
front "Rise of the Witch-king" true; sleep 3
hearth-firehold-plasma ask guide; sleep 5
shot fp-3-guide
echo "::notice title=listener::$(ls -la "$RUN" | tr '\n' ' ' | cut -c1-900); $(pgrep -af 'hearth-firehold-plasma listen' | tr '\n' ' ')"
grep -q guide "$RUN/front" 2>/dev/null && echo "::notice title=Guide::in front of the game" || echo "::warning title=Guide::the Couch says $(cat "$RUN/front" 2>/dev/null)"
hearth-firehold-plasma ask guide; sleep 1
hearth-firehold-plasma ask settings; sleep 4
shot fp-6-settings

# ---- Couch → Desk: only plasmashell restarts.
same() {   # same WHAT: every app, the game and KWin still the same processes
  now=$(apps_pids)
  if [ "$now" = "$PIDS" ] && kill -0 "$KWIN" 2>/dev/null; then echo "::notice title=$1::everything stayed open ($now), KWin the same ($KWIN)"
  else fail "$1" "open before: $PIDS, now: $now; KWin $(kill -0 "$KWIN" 2>/dev/null && echo running || echo GONE)"; fi
}
since=$(date +%s)
hearth-firehold-plasma switch desk --why mouse
if ! wait_ready org.hearthos.firehold.desk 40 && command -v gdb >/dev/null; then
  # It didn't come up: again under gdb, so a crash leaves its backtrace (KDE_DEBUG: no crash handler).
  echo "::warning title=Desk::not up in 40 s; again under gdb"
  pkill plasmashell; sleep 2
  KDE_DEBUG=1 timeout 120 gdb -batch -ex run -ex "thread apply all bt 25" --args plasmashell -p org.hearthos.firehold.desk >"$OUT/desk-gdb.txt" 2>&1 &
  sleep 60
  echo "::warning title=Desk under gdb::$(grep -E '^#|signal|Thread .* \(|qFatal|ASSERT|terminate' "$OUT/desk-gdb.txt" | head -40 | cut -c1-160 | tr '\n' '|' | cut -c1-3500)"
fi
wait_ready org.hearthos.firehold.desk 30 && echo "::notice title=Desk::up in $(( $(date +%s) - since )) s" \
  || fail Desk "org.hearthos.firehold.desk didn't say it's up: $(grep -iE 'error' "$OUT/plasmashell.log" | tail -8 | tr '\n' '|' | cut -c1-900)"
sleep 3
front "Rise of the Witch-king" false; sleep 2
front "Pictures" false; sleep 4
shot fp-2-desk
echo "::notice title=Desk panels::$(timeout 10 dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript string:'print(panels().map(function(p){return p.location + " " + p.lengthMode + " " + p.widgets().map(function(w){return w.type}).join(",")}).join(" | "))' 2>&1 | tr '\n' ' ' | cut -c1-900)"
same "Couch → Desk"
echo "plasmashell now: $(pgrep -a plasmashell)"

# ---- Desk → Couch.
since=$(date +%s)
hearth-firehold-plasma switch couch home
wait_ready org.hearthos.firehold 90 || fail "Desk → Couch" "the Couch didn't come back"
sleep 3
hearth-firehold-plasma do home; sleep 3
shot fp-7-couch-again
same "Desk → Couch"
logsize=$(stat -c %s "$OUT/plasmashell.log")

# ---- a Couch that can't start: plain KDE Plasma by itself, everything still open, the choice kept.
hearth-firehold-plasma switch desk; since=$(date +%s); wait_ready org.hearthos.firehold.desk 90; sleep 2
mv /usr/share/plasma/plasmoids/org.hearthos.firehold.couch /tmp/couch-away
since=$(date +%s)
FIREHOLD_PLASMA_READY_SECS=25 hearth-firehold-plasma switch couch
sleep 45
if pgrep -af plasmashell | grep -q org.kde.plasma.desktop && [ -s "$RUN/failed" ]; then
  echo "::notice title=Fallback::the Couch didn't start, so plain KDE Plasma did ($(pgrep -af plasmashell | tr '\n' ' '))"
else
  fail Fallback "plasmashell: $(pgrep -af plasmashell | tr '\n' ' '); failed marker: $(cat "$RUN/failed" 2>/dev/null)"
fi
sleep 5
shot fp-8-fallback-plain-plasma
same "Fallback"
mv /tmp/couch-away /usr/share/plasma/plasmoids/org.hearthos.firehold.couch

# ---- Firehold's QML must load without errors (before the fallback check, which removes the Couch on purpose).
head -c "$logsize" "$OUT/plasmashell.log" > "$OUT/plasmashell-firehold.log"
bad=$(grep -E "org\.hearthos|firehold" "$OUT/plasmashell-firehold.log" | grep -iE "error|TypeError|ReferenceError|is not a type|is not installed|Cannot assign|not defined|failed|Unable to assign" | sort -u | head -15)
if [ -n "$bad" ]; then
  fail "Firehold QML" "$(printf '%s' "$bad" | cut -c1-300 | sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g')"
fi
warn=$(grep -E "org\.hearthos|firehold" "$OUT/plasmashell-firehold.log" | sort -u | head -12)
[ -n "$warn" ] && echo "::notice title=plasmashell about Firehold::$(printf '%s' "$warn" | cut -c1-250 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
echo "::notice title=session helper::$(tail -c 800 "$HOME/.local/state/nimbus/firehold-plasma.log" 2>/dev/null | tr '\n' '|')"
ls "$OUT"/*.png >/dev/null 2>&1 || fail screenshots "none were taken: $(head -c 600 "$OUT/shot-errors.txt" 2>/dev/null | tr '\n' '|')"
[ -s "$OUT/shot-errors.txt" ] && echo "::warning title=screenshot messages::$(head -c 800 "$OUT/shot-errors.txt" | tr '\n' '|')"
cp "$HOME/.local/state/nimbus/firehold-plasma.log" "$OUT/firehold-plasma.txt" 2>/dev/null

# ---- two screens (mockups firehold-plasma 04 and 06): the TV in Couch, the monitor in Desk, on a KWin with two
# outputs. The Desk shell with the Couch as a layer over the TV, the Modes popup from the monitor's mode pill, Away.
pkill -f "hearth-firehold-plasma session"; pkill plasmashell; pkill dolphin; pkill konsole; pkill kate; pkill -f ember.qml; pkill -f mpris.py
sleep 2; pkill -f /tmp/kwin_wayland; sleep 3
QT_LOGGING_RULES="kwin_scripting.debug=true" /tmp/kwin_wayland --virtual --output-count 2 --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin2.log" 2>&1 &
KWIN=$!
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
sleep 3
# Plasma's activities and global shortcuts went with the first KWin (they're its clients): again on this one.
pkill -f kactivitymanagerd; pkill -f kglobalacceld; sleep 1
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$OUT/kamd2.log" 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"$OUT/kglobalaccel2.log" 2>&1 & break; }; done
# KDE's daemon (its app menu module: KDE's global menu in the bar, mockup 11).
command -v kded6 >/dev/null && { kded6 >"$OUT/kded2.log" 2>&1 & }
sleep 3
export HEARTH_FIREHOLD_SCREENS="Virtual-0::LG 27GP850:2560 × 1440 · 165 Hz · DisplayPort 1,Virtual-1:tv:TCL 55″ TV:3840 × 2160 · 60 Hz · HDMI 2" HEARTH_FIREHOLD_TV_SCREENS=Virtual-1
# Hearth's shared Quick Settings answers as mockup 07 shows (HEARTH_QUICK_FIXTURE), a history from yesterday (mockup 10,
# in the file Hearth keeps it in), and a time zone where it's afternoon (the quiet hours, 11 PM - 7 AM, not on).
export HEARTH_QUICK_FIXTURE=/tmp/fhp-quick.json
python3 - "$HEARTH_QUICK_FIXTURE" "$HOME/.local/state/nimbus/notifications.json" <<'PY'
import json, os, sys, time
json.dump({"device": "Nvidia PC", "line": "Nvidia PC · plugged in · up 3 days", "battery": -1, "user": "Chris",
 "wifi": {"on": True, "ssid": "Home-5G", "strength": "strong"}, "bt": {"on": True, "connected": 3, "names": ["Xbox Controller", "Arctis 7", "MX Master"]},
 "osk": False, "night": {"on": True, "until": "7 AM"}, "dnd": {"on": False, "byHand": False, "schedule": True, "from": "11 PM", "to": "7 AM", "games": True, "family": True},
 "sinks": [{"name": "headset", "word": "Headset", "default": True, "volume": 64}, {"name": "hdmi", "word": "TV (HDMI)", "default": False, "volume": 50}, {"name": "spk", "word": "Speakers", "default": False, "volume": 40}],
 "brightness": [{"id": "lg", "label": "LG monitor", "percent": 80, "max": 100}], "power": "balanced",
 "powerNames": {"power-saver": "Power saver", "balanced": "Balanced", "performance": "Performance"},
 "devices": [{"name": "Xbox Controller", "kind": "pad", "percent": 72, "player": 1, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True},
             {"name": "Arctis 7", "kind": "headset", "percent": 60, "low": False}, {"name": "MX Master", "kind": "mouse", "percent": 40, "low": False}],
 "gameModeOff": False, "modes": [{"name": "Virtual-1", "tv": True, "label": "TCL TV", "mode": "couch"}, {"name": "Virtual-0", "tv": False, "label": "LG monitor", "mode": "desk"}]},
 open(sys.argv[1], "w"))
os.makedirs(os.path.dirname(sys.argv[2]), exist_ok=True)
now = int(time.time() * 1000); D = 86400000
def n(k, app, summary, body, t):
    return {"key": k, "app": app, "summary": summary, "body": body, "icon": "", "entry": "", "image": "", "time": t, "lock": False, "urgency": "normal", "category": "", "progress": -1}
json.dump({"n:%d" % (now - D): n(now - D, "ROG Ally", "ROG Ally", "Saves backed up to this PC · 12 games", now - D - 3600000),
           "n:%d" % (now - D + 1): n(now - D + 1, "Controller 2", "Controller 2", "Battery low · 12%", now - D - 7200000)}, open(sys.argv[2], "w"), indent=2)
PY
export TZ=$(python3 -c 'import time; h=time.gmtime().tm_hour; print("Etc/GMT%+d" % -((14 - h + 12) % 24 - 12))')
cat > /tmp/fhp-notify.py <<'PY'
import dbus, json, sys
n = dbus.Interface(dbus.SessionBus().get_object('org.freedesktop.Notifications', '/org/freedesktop/Notifications'), 'org.freedesktop.Notifications')
o = json.loads(sys.argv[1])
hints = {'urgency': dbus.Byte(o.get('urgency', 1))}
if o.get('image'): hints['image-path'] = o['image']
if o.get('urls'): hints['x-kde-urls'] = dbus.Array(o['urls'], signature='s')
if o.get('reply'): hints['x-kde-reply-placeholder-text'] = o['reply']
print(n.Notify(o['app'], 0, '', o['summary'], o.get('body', ''), o.get('actions', []), hints, o.get('timeout', -1)))
PY
note() { timeout 10 python3 /tmp/fhp-notify.py "$1" >>"$OUT/notify.log" 2>&1 || echo "notify failed: $1" >>"$OUT/notify.log"; }
hearth-firehold-plasma begin org.hearthos.firehold.desk
now=$(($(date +%s) * 1000))
printf '{"screens": {"Virtual-1": {"mode": "couch", "why": "pad", "at": %s}, "Virtual-0": {"mode": "desk", "why": "mouse", "at": %s}}}' "$now" "$now" > "$RUN/modes.json"
since=$(date +%s)
# Under gdb from the start (the two-screen Desk has crashed now and then at start since round 20, and not again under
# gdb when started a second time): a crash leaves its backtrace in desk2-gdb.txt.
if command -v gdb >/dev/null; then
  HEARTH_FIREHOLD_TIP=always KDE_DEBUG=1 PLASMA_DEFAULT_SHELL=org.hearthos.firehold.desk timeout 1500 gdb -batch -ex run -ex "thread apply all bt 30" --args plasmashell -p org.hearthos.firehold.desk >"$OUT/desk2-gdb.txt" 2>&1 &
else
  HEARTH_FIREHOLD_TIP=always PLASMA_DEFAULT_SHELL=org.hearthos.firehold.desk plasmashell -p org.hearthos.firehold.desk >>"$OUT/plasmashell2.log" 2>&1 &
fi
PLASMA_DEFAULT_SHELL=org.hearthos.firehold.desk hearth-firehold-plasma session >"$OUT/session2.log" 2>&1 &
wait_ready org.hearthos.firehold.desk 90 && echo "::notice title=Two screens::the Desk shell is up" || fail "Two screens" "the Desk didn't come up"
python3 "$FX/mpris.py" >"$OUT/mpris2.log" 2>&1 &
dolphin "$HOME/Pictures" >/dev/null 2>&1 &
sleep 12
echo "::notice title=Two screens::$(hearth-firehold-plasma modes | cut -c1-600)"
shot fp-10-two-screens
shot fp-11-desk-bar-tip 0 1920
shot fp-12-couch-layer-on-tv 1920 1920
kwin_js 'var l = workspace.stackingOrder; for (var i = 0; i < l.length; i++) { var w = l[i]; console.warn("FHWIN " + w.caption + " | " + w.resourceClass + " | " + (w.output ? w.output.name : "") + " | " + w.frameGeometry.x + "," + w.frameGeometry.y + " " + w.frameGeometry.width + "x" + w.frameGeometry.height + " | layer " + w.layer + " | hidden " + w.hidden + " | min " + w.minimized); }'
sleep 1
echo "::notice title=KWin's windows (two screens)::$(grep -h FHWIN "$OUT/kwin2.log" "$OUT/plasmashell2.log" | tail -20 | cut -c1-160 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
# Game options with two screens: Play on the TV · Couch / the monitor · Desk.
hearth-firehold-plasma ask tab:library; sleep 3
key right x down; sleep 3
shot fp-03-two-screens-game-options 1920 1920
key b; hearth-firehold-plasma ask tab:home; sleep 2
# An app with no menus of its own in front on the monitor: Hearth's standard menus, Window open (mockup 12).
sed 's/Rise of the Witch-king/Trains 2010/' /tmp/ember.qml > /tmp/trains.qml
[ -n "$QML" ] && { "$QML" /tmp/trains.qml >/dev/null 2>&1 & }
sleep 6; front "Trains 2010" false
# On the monitor, in a window (mockup 12 draws it there).
kwin_js "var o = workspace.screens.filter(function (s) { return s.name === 'Virtual-0'; })[0]; workspace.windowList().forEach(function (w) { if (String(w.caption).indexOf('Trains 2010') >= 0) { workspace.sendClientToScreen(w, o); w.frameGeometry = { x: 70, y: 90, width: 1300, height: 820 }; workspace.activeWindow = w; } });"
sleep 2
hearth-firehold-plasma ping menu-move; sleep 3
shot fp-13-desk-standard-menus 0 1920
hearth-firehold-plasma ping menu-off; sleep 1
hearth-firehold-plasma ping menu-app; sleep 3
shot fp-13b-desk-app-menu 0 1920
hearth-firehold-plasma ping menu-off; sleep 1
hearth-firehold-plasma ping menu-help; sleep 3
shot fp-13c-desk-help-menu 0 1920
hearth-firehold-plasma ping menu-off; sleep 1
# A controller button pressed on the Desk (no controller here: the same call the controller's button makes).
hearth-firehold-plasma pad-button a; sleep 3
shot fp-14-controller-hint 0 1920
{ echo "notify-send: $(command -v notify-send || echo none)"; echo "posture: $(cat "$RUN/posture" 2>/dev/null)"
  echo "in Hearth's history: $(grep -c 'controller is in your hand' "$HOME/.local/state/nimbus/notifications.json" 2>/dev/null)"
  notify-send -a Check check check 2>&1; echo "notify-send exit $?"; } > "$OUT/controller-hint.txt" 2>&1
# The app's own menus (mockup 11): Kate in front, its menus in the bar from KDE's global menu, File open (Alt+F).
printf 'Age of the Ring 8.4 patch notes\n' > /tmp/patch-notes.txt
QT_QPA_PLATFORMTHEME=kde kate /tmp/patch-notes.txt >/dev/null 2>&1 &
sleep 8; front "patch-notes" false; sleep 3
# File open: KDE's app menu daemon's showMenu with no place (-1, -1: as a shortcut asks), so the bar's global menu
# opens Kate's first menu, File, under its button.
cat > /tmp/fhp-file-menu.py <<'PY'
import dbus, re, sys
bus = dbus.SessionBus()
pid = int(sys.argv[1])
dbo = bus.get_object('org.freedesktop.DBus', '/org/freedesktop/DBus')
def paths(n, p):
    # the dbusmenu objects under /MenuBar (Qt puts them at /MenuBar/1 and so on)
    xml = bus.get_object(n, p).Introspect(dbus_interface='org.freedesktop.DBus.Introspectable')
    if 'com.canonical.dbusmenu' in xml:
        yield p
    for c in re.findall(r'<node name="([^"]+)"', xml):
        yield from paths(n, p.rstrip('/') + '/' + c)
for n in dbo.ListNames(dbus_interface='org.freedesktop.DBus'):
    if not n.startswith(':'):
        continue
    try:
        if int(dbo.GetConnectionUnixProcessID(n, dbus_interface='org.freedesktop.DBus')) != pid:
            continue
        for p in paths(n, '/MenuBar'):
            lay = bus.get_object(n, p).GetLayout(0, 1, dbus.Array([], signature='s'), dbus_interface='com.canonical.dbusmenu')
            first = int(lay[1][2][0][0])
            print('menu', n, p, first)
            bus.get_object('org.kde.kappmenu', '/KAppMenu').showMenu(-1, -1, n, dbus.ObjectPath(p), first, dbus_interface='org.kde.kappmenu')   # -1: the bar's own menu shows it
            sys.exit(0)
    except Exception as e:
        print('no menu on', n, e)
print('no menus found for', pid)
PY
if [ -x "$FAKE" ]; then
  # A click on File in the bar (Kate's own menus, KDE's global menu), as a person does.
  "$FAKE" click 124 31 >> "$OUT/controller-hint.txt" 2>&1; echo "fake input click: $?" >> "$OUT/controller-hint.txt"; sleep 3
else
  timeout 15 python3 /tmp/fhp-file-menu.py "$(pgrep -x kate | head -1)" >> "$OUT/controller-hint.txt" 2>&1; sleep 3
fi
shot fp-11-desk-bar-app-menus 0 1920
[ -x "$FAKE" ] && "$FAKE" keys 1; sleep 1
front "patch-notes" false; sleep 1
echo "kate's menus: $(hearth-firehold-plasma menus "$(pgrep -x kate | head -1)")" >> "$OUT/controller-hint.txt"
front "Trains 2010" false; sleep 2
# While you played (mockup 10): notifications that come while a game is full screen are held back.
front "Trains 2010" true; sleep 3
note '{"app": "Friends", "summary": "Mira", "body": "Ready for co-op tonight?"}'; sleep 1
note '{"app": "Steam", "summary": "Satisfactory installed", "body": "Ready to play"}'; sleep 3
front "Trains 2010" false; sleep 2
front "Pictures" false; sleep 2
# Quick Settings on the Desk (mockup 07) and the notification history (10): Hearth's shared widgets, Firehold's look.
if command -v hearth-plasma-panel >/dev/null; then
  # The cards (mockup 09): urgent, a game ready, a clip with its picture, a message with a reply box, then "2 more".
  note '{"app": "Firefox", "summary": "Firefox download finished", "body": "hollow-pines.tar.zst · 2.9 GB", "timeout": 30000}'; sleep 1
  note '{"app": "Hearth OS", "summary": "Hearth OS update ready", "body": "Nothing changes until you restart", "timeout": 30000}'; sleep 1
  note '{"app": "Discord", "summary": "Ryder in #bfme-nights", "body": "“Edain game at 11? I'"'"'ll host.”", "actions": ["inline-reply", "Reply", "read", "Mark read"], "reply": "Reply...", "timeout": 30000}'; sleep 1
  note '{"app": "Hearth Recorder", "summary": "Saved the last 30 seconds", "body": "Rise of the Witch-king · 1440p · 96 MB", "urls": ["file://'"$HOME"'/.cache/hearth/art/beacon_rotwk.jpg"], "actions": ["open", "Open", "share", "Copy a share link", "delete", "Delete"], "timeout": 30000}'; sleep 1
  note '{"app": "Downloads", "summary": "Helldivers 2 is ready", "body": "The 6.0 GB update finished (Steam).", "actions": ["play", "Play", "tv", "Play on the TV"], "timeout": 30000}'; sleep 1
  note '{"app": "Hearth Link", "summary": "Dad'"'"'s PC asks for help", "body": "“Trains 2010 won'"'"'t start after the update.” He'"'"'s at his PC now.", "urgency": 2, "actions": ["connect", "Connect (Remote desktop)", "later", "Call him back later", "notnow", "Not now"]}'; sleep 3
  shot fp-09-notification-cards 0 1920
  hearth-plasma-panel open quick; sleep 4; shot fp-07-quick-settings-desk 0 1920; hearth-plasma-panel open close; sleep 1
  hearth-plasma-panel open calendar; sleep 4; shot fp-10-notification-history 0 1920; hearth-plasma-panel open close; sleep 1
fi
hearth-firehold-plasma ping popup Virtual-0; sleep 4
shot fp-04-modes-per-screen 0 1920
hearth-firehold-plasma ping popup-off Virtual-0; sleep 1
hearth-firehold-plasma mode Virtual-1 away --why pick; sleep 6
shot fp-06-away 1920 1920
cat "$OUT/desk2-gdb.txt" >> "$OUT/plasmashell2.log" 2>/dev/null
grep -E "org\.hearthos|firehold" "$OUT/plasmashell2.log" | grep -iE "error|TypeError|ReferenceError|is not a type|is not installed|Cannot assign|not defined|failed|Unable to assign" | sort -u | head -12 > "$OUT/qml2-errors.txt"
echo "::notice title=plasmashell about Firehold (two screens)::$(grep -E "firehold|org\.hearthos" "$OUT/plasmashell2.log" | sort -u | head -12 | cut -c1-250 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
[ -s "$OUT/qml2-errors.txt" ] && fail "Firehold QML (two screens)" "$(cut -c1-300 "$OUT/qml2-errors.txt" | sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g')"

pkill -f "hearth-firehold-plasma session"; pkill plasmashell; pkill dolphin; pkill konsole; pkill kate; pkill -f ember.qml
pkill -f /tmp/kwin_wayland; pkill -f kactivitymanagerd; pkill pipewire; pkill wireplumber; pkill dbus-daemon
sleep 2; pkill -9 plasmashell; pkill -9 -f /tmp/kwin_wayland
exit 0
