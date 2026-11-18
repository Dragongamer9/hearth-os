#!/bin/bash
# Hearth's looks in Plasma (Hearth OS on Anatase), in the image itself: KDE Plasma on kwin_wayland --virtual (software
# drawing) as a new account that hasn't chosen a look, through hearth-plasma-looks sync (what the session runs).
#   anatase-look-shots.sh IMAGE OUTDIR [REPO]
# REPO: put this checkout's hearth-plasma-looks into the image first and generate the looks again (anatase-looks.yml
# tries a fix on the published image without building one).
#
# First start (REQUIRED, OUTDIR/first-start.txt "OK ..." or "FAIL ..."): Hearth's Global Theme is applied and recorded
# (KDE's settings name org.hearthos.hearth, its colours and Plasma style; the state file written). The workflow fails
# on anything but OK.
# Pictures (warn-only): Hearth in full (desktop, app menu, the Hearth apps found by KRunner, Dolphin, System Settings ›
# Global Theme and › Colors), each look picked as Hearth's Settings picks it, a sheet of every Hearth colour scheme, and
# Firehold on Plasma (Couch, Guide, Open now, Desk) when the image has it.
set -u
IMG=${1:?image}; OUT=${2:?out dir}; REPO=${3:-}
HERE=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
echo "FAIL the first start never ran" > "$OUT/first-start.txt"
NAME=looks-$$
P="sudo podman"
$P run -d --name "$NAME" --privileged --entrypoint sleep "$IMG" infinity >/dev/null \
  || { echo "FAIL couldn't start the image" > "$OUT/first-start.txt"; echo "::error title=looks::couldn't start the image"; exit 0; }
trap '$P rm -f "$NAME" >/dev/null 2>&1' EXIT

# Only for the pictures, in this throwaway container (never in the image): KWin's screenshot through dbus-python, the
# sheet through qml. (python3-gobject, already in the image, is the fallback for the screenshots.)
$P exec "$NAME" bash -c 'timeout 300 dnf5 -y -q install --skip-unavailable python3-dbus qt6-qtdeclarative >/dev/null 2>&1' \
  || echo "::notice title=looks::the screenshot helpers didn't install; PyGObject is used"
if [ -n "$REPO" ]; then
  $P cp "$REPO/os-bases/anatase/system_files/usr/libexec/nimbus/hearth-plasma-looks" "$NAME:/usr/libexec/nimbus/hearth-plasma-looks"
  $P exec "$NAME" bash -c 'chmod 755 /usr/libexec/nimbus/hearth-plasma-looks &&
    python3 /usr/libexec/nimbus/hearth-plasma-looks build --logo /usr/share/icons/hicolor/scalable/apps/hearth.svg >/dev/null' \
    && echo "::notice title=looks::this branch's hearth-plasma-looks, looks generated again" \
    || echo "::error title=looks::this branch's hearth-plasma-looks couldn't generate the looks"
fi
$P exec "$NAME" bash -c 'id ci >/dev/null 2>&1 || useradd -m -u 1000 ci 2>/dev/null || useradd -m ci'
$P cp "$HERE/anatase-look-shot.py" "$NAME:/tmp/shot.py"

cat > /tmp/looks-in.sh <<'IN'
set -u
S=/tmp/shots
mkdir -p "$S"
export XDG_RUNTIME_DIR=/tmp/xdg-ci HOME=/home/ci LC_ALL=C.UTF-8
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
say() { echo "$*"; echo "$*" >> "$S/events.txt"; }
# KWin without its file capabilities (a copy), so it keeps the screenshot switch from the environment.
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$S/kwin.log" 2>&1 &
KWIN=$!
for i in $(seq 60); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { say "FIRSTFAIL KWin didn't start: $(tail -5 "$S/kwin.log" | tr '\n' '|' | cut -c1-500)"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$S/kamd.log" 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"$S/kglobalaccel.log" 2>&1 & break; }; done
kbuildsycoca6 >/dev/null 2>&1
up() { dbus-send --session --print-reply --dest=org.freedesktop.DBus / org.freedesktop.DBus.NameHasOwner string:org.kde.plasmashell 2>/dev/null | grep -q 'boolean true'; }
SHELLPKG=org.kde.plasma.desktop
start_shell() {
  echo "---- plasmashell -p $SHELLPKG ($(date +%T))" >> "$S/plasmashell.log"
  plasmashell --no-respawn -p "$SHELLPKG" >>"$S/plasmashell.log" 2>&1 &
  SHELL_PID=$!
  for i in $(seq 120); do up && return 0; sleep 1; done
  return 1
}
keep_shell() {   # keep_shell WHEN: plasmashell ended? say so (with its last words) and start it again
  if ! kill -0 "$SHELL_PID" 2>/dev/null || ! up; then
    wait "$SHELL_PID" 2>/dev/null; code=$?
    say "WARN plasmashell had ended ($1, exit $code): $(grep -v '^$' "$S/plasmashell.log" | tail -6 | tr '\n' '|' | cut -c1-700)"
    start_shell && sleep 10
  fi
}
shot() {
  rm -f "$S/$1.png"
  timeout 40 python3 /tmp/shot.py "$S/$1.png" 2>>"$S/shot-errors.txt"
  if [ ! -s "$S/$1.png" ]; then   # KWin's interface didn't answer: Spectacle
    timeout 40 spectacle --background --nonotify --fullscreen --output "$S/$1.png" >>"$S/shot-errors.txt" 2>&1
  fi
  [ -s "$S/$1.png" ] && say "SHOT $1" || say "WARN no screenshot for $1"
}
status() { /usr/libexec/nimbus/hearth-plasma-looks status 2>&1 | tail -1; }
pick() {   # pick LOOK: as Hearth's Settings does (desktop.json), then what the path unit runs
  mkdir -p "$HOME/.local/share/nimbus"
  python3 - "$1" <<'PY'
import json, os, sys
p = os.path.expanduser('~/.local/share/nimbus/desktop.json')
try:
    d = json.load(open(p))
except Exception:
    d = {}
d['look'] = sys.argv[1]
tmp = p + '.tmp'
json.dump(d, open(tmp, 'w'), indent=2)
os.replace(tmp, p)
PY
  timeout 180 /usr/libexec/nimbus/hearth-plasma-looks sync >>"$S/sync.log" 2>&1
  st=$(status)
  echo "$st" | grep -q "\"lnf\": \"org.hearthos.$1\"" && say "OK $1 applied" || say "WARN $1 not applied: $st"
  sleep 6
}

start_shell || { say "FIRSTFAIL plasmashell didn't start: $(tail -8 "$S/plasmashell.log" | tr '\n' '|' | cut -c1-700)"; exit 0; }
sleep 12
# ---- the first start: nothing chosen, Hearth's look applied by itself (the autostart's command).
echo "---- first start" >> "$S/sync.log"
timeout 240 /usr/libexec/nimbus/hearth-plasma-looks sync --wait >>"$S/sync.log" 2>&1; rc=$?
st=$(status)
echo "$st" > "$S/first-start-status.json"
if echo "$st" | grep -q '"lnf": "org.hearthos.hearth"' && echo "$st" | grep -q '"scheme": "HearthOS-hearth-' \
   && echo "$st" | grep -q '"style": "hearth-hearth"' && echo "$st" | grep -q '"applied": {"look": "hearth"' \
   && grep -q 'LookAndFeelPackage=org.hearthos.hearth' "$HOME/.config/kdeglobals" 2>/dev/null; then
  say "FIRSTOK Hearth's look applied and recorded (sync exit $rc): $st"
else
  say "FIRSTFAIL sync exit $rc; $st; $(grep -v '^$' "$S/sync.log" | tail -8 | tr '\n' '|' | cut -c1-900)"
fi
sleep 8
keep_shell "after the first start"
# ---- Hearth in full.
shot hearth-desktop
dbus-send --session --type=method_call --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu; sleep 5
shot hearth-app-menu
dbus-send --session --type=method_call --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu; sleep 2
keep_shell "after the app menu"
dbus-send --session --type=method_call --dest=org.kde.krunner /App org.kde.krunner.App.query string:"Hearth" >/dev/null 2>&1; sleep 8
shot hearth-apps-krunner
pkill -u "$(id -u)" -x krunner; sleep 1
# Every Hearth app KDE lists (the menu's own source: the .desktop files KDE shows).
python3 - > "$S/hearth-apps.txt" <<'PY'
import configparser, glob, os
seen = {}
for d in ('/usr/share/applications', '/usr/local/share/applications', os.path.expanduser('~/.local/share/applications')):
    for f in sorted(glob.glob(d + '/*.desktop')):
        c = configparser.RawConfigParser(strict=False, interpolation=None)
        try:
            c.read(f, encoding='utf-8')
            e = c['Desktop Entry']
        except Exception:
            continue
        if e.get('NoDisplay', 'false') == 'true' or e.get('Hidden', 'false') == 'true':
            continue
        if 'KDE' in e.get('NotShowIn', '').split(';') or (e.get('OnlyShowIn') and 'KDE' not in e.get('OnlyShowIn').split(';')):
            continue
        n = e.get('Name', '')
        if 'hearth' in os.path.basename(f).lower() or n.startswith('Hearth'):
            seen[os.path.basename(f)] = n
for k, v in sorted(seen.items(), key=lambda kv: kv[1].lower()):
    print('%-40s %s' % (v, k))
PY
say "APPS $(wc -l < "$S/hearth-apps.txt") Hearth apps in KDE's menu"
dolphin --new-window /usr/share/wallpapers >"$S/dolphin.log" 2>&1 & DP=$!; sleep 10
shot hearth-dolphin
kill $DP 2>/dev/null; sleep 2
systemsettings kcm_lookandfeel >"$S/systemsettings.log" 2>&1 & SP=$!; sleep 15
shot hearth-system-settings-global-theme
kill $SP 2>/dev/null; sleep 3
systemsettings kcm_colors >>"$S/systemsettings.log" 2>&1 & SP=$!; sleep 15
shot hearth-system-settings-colours
kill $SP 2>/dev/null; sleep 3
keep_shell "after System Settings"
# ---- each look, picked the way Hearth's Settings picks it, with Dolphin open for the window colours.
dolphin --new-window /usr/share/wallpapers >>"$S/dolphin.log" 2>&1 & DP=$!; sleep 8
for l in $(python3 -c 'import json; d=json.load(open("/usr/share/nimbus/desktop/theme/palettes.json")); print(" ".join(k for k,v in sorted(d.items(), key=lambda kv: (kv[1].get("order", 99) if isinstance(kv[1], dict) else 99, kv[0])) if not k.startswith("_") and isinstance(v,dict) and isinstance(v.get("dark"),dict)))'); do
  keep_shell "before $l"
  pick "$l"
  shot "look-$l"
done
kill $DP 2>/dev/null
pick hearth
# ---- a sheet of every Hearth colour scheme (the looks' own and each sub theme's), drawn by Qt.
python3 - > /tmp/sheet.qml <<'PY'
import glob, json, os, re
def grp(text, g):
    m = re.search(r'(?ms)^\[%s\]\n(.*?)(?=^\[|\Z)' % re.escape(g), text)
    return dict(re.findall(r'(?m)^(\w+)=(.*)$', m.group(1))) if m else {}
def hexc(v):
    try:
        return '#%02x%02x%02x' % tuple(int(x) for x in v.split(',')[:3])
    except Exception:
        return '#ff00ff'
items = []
for f in sorted(glob.glob('/usr/share/color-schemes/HearthOS-*.colors')):
    t = open(f).read()
    name = re.search(r'(?m)^Name=(.*)$', t).group(1).replace('Hearth OS · ', '')
    w, v, b, s, wm = grp(t, 'Colors:Window'), grp(t, 'Colors:View'), grp(t, 'Colors:Button'), grp(t, 'Colors:Selection'), grp(t, 'WM')
    items.append({'n': name, 'c': [hexc(wm.get('activeBackground', '0,0,0')), hexc(w.get('BackgroundNormal', '0,0,0')),
                                   hexc(v.get('BackgroundNormal', '0,0,0')), hexc(b.get('BackgroundNormal', '0,0,0')),
                                   hexc(s.get('BackgroundNormal', '0,0,0')), hexc(w.get('ForegroundNormal', '0,0,0'))]})
print('''import QtQuick
import QtQuick.Window
Window {
    title: "Hearth colour schemes"; visible: true; visibility: Window.FullScreen; color: "#101010"
    property var items: %s
    Text { x: 16; y: 8; text: "Hearth OS colour schemes (" + items.length + "): title bar, window, view, button, selection, text"; color: "#f0f0f0"; font.pixelSize: 18 }
    Grid {
        x: 12; y: 40; columns: 5; columnSpacing: 10; rowSpacing: 6
        Repeater {
            model: items
            Rectangle {
                width: 370; height: 54; radius: 6; color: modelData.c[1]; border.color: "#444"
                Row { x: 8; y: 6; spacing: 4; Repeater { model: modelData.c; Rectangle { width: 22; height: 16; radius: 3; color: modelData; border.color: "#80808080" } } }
                Text { x: 8; y: 28; width: 354; elide: Text.ElideRight; text: modelData.n; color: modelData.c[5]; font.pixelSize: 15 }
            }
        }
    }
}''' % json.dumps(items))
PY
QML=$(command -v qml6 || command -v qml || ls /usr/lib64/qt6/bin/qml 2>/dev/null | head -1)
if [ -n "$QML" ]; then
  "$QML" /tmp/sheet.qml >"$S/sheet.log" 2>&1 & QP=$!; sleep 10
  shot colour-schemes-sheet
  kill $QP 2>/dev/null
else
  say "WARN no qml program for the colour scheme sheet"
fi
# ---- Firehold on Plasma (an opt-in choice on this edition): Couch, Guide, Open now, Desk.
if [ -f /usr/share/plasma/shells/org.hearthos.firehold/metadata.json ] && command -v hearth-firehold-plasma >/dev/null; then
  export HEARTH_FIREHOLD_PLASMA=1 FIREHOLD_PLASMA_SHELL_LOG="$S/plasmashell.log"
  RUN="$XDG_RUNTIME_DIR/hearth-firehold-plasma"
  ready() { for i in $(seq "$2"); do [ -f "$RUN/ready-$1" ] && [ "$(stat -c %Y "$RUN/ready-$1")" -ge "$since" ] && return 0; sleep 1; done; return 1; }
  kill "$SHELL_PID" 2>/dev/null; sleep 3; pkill -u "$(id -u)" -x plasmashell; sleep 2
  hearth-firehold-plasma begin org.hearthos.firehold >>"$S/firehold.log" 2>&1
  since=$(date +%s)
  SHELLPKG=org.hearthos.firehold
  PLASMA_DEFAULT_SHELL=org.hearthos.firehold plasmashell -p org.hearthos.firehold >>"$S/plasmashell.log" 2>&1 & SHELL_PID=$!
  PLASMA_DEFAULT_SHELL=org.hearthos.firehold hearth-firehold-plasma session >>"$S/firehold.log" 2>&1 &
  if ready org.hearthos.firehold 90; then
    sleep 5
    hearth-firehold-plasma do home >>"$S/firehold.log" 2>&1; sleep 4
    shot firehold-couch
    hearth-firehold-plasma ask guide >>"$S/firehold.log" 2>&1; sleep 5
    shot firehold-guide
    hearth-firehold-plasma ask guide >>"$S/firehold.log" 2>&1; sleep 2
    hearth-firehold-plasma ask open >>"$S/firehold.log" 2>&1; sleep 5
    shot firehold-open-now
    hearth-firehold-plasma ask open >>"$S/firehold.log" 2>&1; sleep 2
    since=$(date +%s)
    hearth-firehold-plasma switch desk --why mouse >>"$S/firehold.log" 2>&1
    ready org.hearthos.firehold.desk 90 && { sleep 6; shot firehold-desk; } || say "WARN the Firehold Desk didn't say it's up"
  else
    say "WARN Firehold's Couch didn't say it's up: $(grep -iE 'firehold|error' "$S/plasmashell.log" | tail -6 | tr '\n' '|' | cut -c1-700)"
  fi
else
  say "NOTE Firehold on Plasma isn't in this image"
fi
cp "$HOME/.config/kdeglobals" "$S/kdeglobals.txt" 2>/dev/null
cp "$HOME/.local/state/hearth/plasma-look.json" "$S/plasma-look.json" 2>/dev/null
pkill -u "$(id -u)" plasmashell; kill "$KWIN" 2>/dev/null
IN
$P cp /tmp/looks-in.sh "$NAME:/tmp/looks-in.sh"
$P exec "$NAME" bash -c 'chmod 644 /tmp/looks-in.sh /tmp/shot.py; runuser -u ci -- bash /tmp/looks-in.sh' > /tmp/looks.log 2>&1
$P exec "$NAME" bash -c 'chmod -R a+r /tmp/shots' >/dev/null 2>&1
$P cp "$NAME:/tmp/shots/." "$OUT/" 2>/dev/null || true
sudo chown -R "$(id -u):$(id -g)" "$OUT" 2>/dev/null || true

# ---- results (every line is in the step's log; the important ones are annotations too)
first=$(grep -E '^FIRST(OK|FAIL) ' "$OUT/events.txt" 2>/dev/null | tail -1)
case "$first" in
  FIRSTOK*) echo "OK ${first#FIRSTOK }" > "$OUT/first-start.txt"; echo "::notice title=first start::${first#FIRSTOK }" ;;
  FIRSTFAIL*) echo "FAIL ${first#FIRSTFAIL }" > "$OUT/first-start.txt"; echo "::error title=first start::${first#FIRSTFAIL }" ;;
  *) echo "FAIL no result: $(tail -5 /tmp/looks.log | tr '\n' '|' | cut -c1-600)" > "$OUT/first-start.txt"; echo "::error title=first start::$(cat "$OUT/first-start.txt")" ;;
esac
grep -E '^(WARN) ' "$OUT/events.txt" 2>/dev/null | while read -r _ w; do echo "::warning title=looks::$(echo "$w" | cut -c1-900)"; done
n=$(ls "$OUT"/*.png 2>/dev/null | wc -l)
echo "::notice title=looks::$n screenshots: $(cd "$OUT" && ls *.png 2>/dev/null | tr '\n' ' ')"
for f in events.txt sync.log first-start-status.json shot-errors.txt hearth-apps.txt firehold.log looks; do
  src="$OUT/$f"; [ "$f" = looks ] && src=/tmp/looks.log
  [ -s "$src" ] || continue
  echo "==== $f"; tail -c 20000 "$src"
done
echo "==== plasmashell.log (tail)"; tail -c 12000 "$OUT/plasmashell.log" 2>/dev/null
echo "==== kwin.log (tail)"; tail -c 4000 "$OUT/kwin.log" 2>/dev/null
exit 0
