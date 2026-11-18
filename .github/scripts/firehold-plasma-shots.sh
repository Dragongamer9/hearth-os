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

# ---- the games Firehold shows: the mockups' sample games, as hearth-firehold's cache (no Hearth app here).
python3 - "$HOME/.cache/hearth/firehold-library.json" <<'PY'
import json, os, sys, time
now = int(time.time() * 1000)
m = 60000
g = lambda i, t, store, kind, **k: dict({'id': i, 'title': t, 'store': store, 'kind': kind, 'installed': True, 'lastPlayed': 0}, **k)
games = [
    g('steam:1', 'Ember Trails', 'Steam', 'steam', lastPlayed=now - 41 * m, cloudSaves=True, chapter='Chapter 4'),
    g('steam:2', 'Skyforge', 'Steam', 'steam', lastPlayed=now - 3 * 60 * m),
    g('gog:1', 'Night Harbor', 'GOG', 'gog', lastPlayed=now - 5 * 60 * m),
    g('emu:1', 'Pixel Rally', 'Emulator', 'emu', lastPlayed=now - 8 * 60 * m),
    g('epic:1', 'Hollow Deep', 'Epic', 'epic', lastPlayed=now - 26 * 60 * m, update=True),
    g('steam:3', 'Tiny Farm', 'Steam', 'steam', lastPlayed=now - 50 * 60 * m),
    g('winapp:1', 'Old Office', 'Windows app', 'winapp', lastPlayed=now - 80 * 60 * m),
    g('manual:1', 'Quad Mech', 'Your game', 'yours', lastPlayed=now - 90 * 60 * m, devBuild=True),
    g('gog:2', 'Star Courier', 'GOG', 'gog', installed=False),
    g('epic:2', 'Deep Root', 'Epic', 'epic', installed=False),
    g('steam:4', 'Cloud Lanes', 'Steam', 'steam', installed=False, cloudSaves=True),
    g('steam:5', 'Frost Keep', 'Steam', 'steam', installed=False),
]
counts = {'all': 412, 'installed': 96, 'steam': 288, 'gog': 41, 'epic': 57, 'winapp': 9, 'emu': 14, 'yours': 3}
os.makedirs(os.path.dirname(sys.argv[1]), exist_ok=True)
json.dump({'games': games, 'counts': counts}, open(sys.argv[1], 'w'))
PY

# ---- a stand-in game window: "Ember Trails" (the library matches it by its title).
cat > /tmp/ember.qml <<'QML'
import QtQuick
import QtQuick.Window
Window {
    title: "Ember Trails"
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
Image.frombuffer('RGBA', (W, H), data, 'raw', 'BGRA', S, 1).convert('RGB').save(sys.argv[1])
PY
shot() { timeout 25 python3 /tmp/shot.py "$OUT/$1.png" 2>>"$OUT/shot-errors.txt" || echo "no screenshot $1" >>"$OUT/shot-errors.txt"; }
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

# ---- KWin (KDE's own, unchanged) on a virtual screen, PipeWire for window pictures, Plasma's activities.
for p in pipewire wireplumber; do command -v $p >/dev/null && { $p >"$OUT/$p.log" 2>&1 & }; done
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin.log" 2>&1 &
KWIN=$!
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start: $(tail -5 "$OUT/kwin.log" | tr '\n' '|' | cut -c1-600)"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$OUT/kamd.log" 2>&1 & break; }; done
sleep 3

# ---- the Couch, as nimbus-session starts it.
hearth-firehold-plasma begin org.hearthos.firehold
since=$(date +%s)
PLASMA_DEFAULT_SHELL=org.hearthos.firehold plasmashell -p org.hearthos.firehold >>"$OUT/plasmashell.log" 2>&1 &
PLASMA_DEFAULT_SHELL=org.hearthos.firehold hearth-firehold-plasma session >"$OUT/session.log" 2>&1 &
wait_ready org.hearthos.firehold 90 && echo "::notice title=Couch::up in $(( $(date +%s) - since )) s" \
  || fail Couch "org.hearthos.firehold didn't say it's up: $(grep -iE 'firehold|error' "$OUT/plasmashell.log" | head -8 | tr '\n' '|' | cut -c1-900)"
sleep 4
shot fp-0-couch-first

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
front "Ember Trails" true; sleep 3
hearth-firehold-plasma ask guide; sleep 5
shot fp-3-guide
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
wait_ready org.hearthos.firehold.desk 90 && echo "::notice title=Desk::up in $(( $(date +%s) - since )) s" \
  || fail Desk "org.hearthos.firehold.desk didn't say it's up: $(grep -iE 'error' "$OUT/plasmashell.log" | tail -8 | tr '\n' '|' | cut -c1-900)"
sleep 3
front "Ember Trails" false; sleep 2
front "Pictures" false; sleep 4
shot fp-2-desk
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

pkill -f "hearth-firehold-plasma session"; pkill plasmashell; pkill dolphin; pkill konsole; pkill kate; pkill -f ember.qml
pkill -f /tmp/kwin_wayland; pkill -f kactivitymanagerd; pkill pipewire; pkill wireplumber; pkill dbus-daemon
sleep 2; pkill -9 plasmashell; pkill -9 -f /tmp/kwin_wayland
exit 0
