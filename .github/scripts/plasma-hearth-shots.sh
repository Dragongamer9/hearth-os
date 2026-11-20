#!/bin/bash
# Hearth's panel on KDE Plasma (plasma-hearth.yml): plain KDE Plasma on KDE's own KWin on a virtual 1920x1080
# screen, Hearth's panel put in (hearth-plasma-panel apply), then every Hearth look pictured as Mantel's CI pictures it
# (desktop-test.yml looks/mantel/<look>-<mode>-1-desktop, -2-quick-settings, -3-launcher, -4-notifications-dashboard)
# and the five app menus. Checks (::error:: and OUT/FAILED): Hearth's panel goes in and comes out again (undo), and
# plasmashell says nothing about Hearth's files. Pictures and logs in OUT.
#   plasma-hearth-shots.sh OUT
set +e +o pipefail
OUT=${1:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/step.log") 2>&1
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }

export XDG_RUNTIME_DIR=/tmp/xdg HOME=/tmp/home USER=chris
mkdir -p "$XDG_RUNTIME_DIR" "$HOME/Documents/Recipes" "$HOME/Documents/Reports" "$HOME/Documents/School" "$HOME/Desktop" \
  "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" && chmod 700 "$XDG_RUNTIME_DIR"
printf 'shopping list\n' > "$HOME/Documents/notes.txt"; cp "$HOME/Documents/notes.txt" "$HOME/Desktop/notes.txt"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1 HEARTH_PLASMA_PANEL_CI=1
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
D="$HOME/.local/share/nimbus/desktop.json"
printf '{"pins": ["hearth-maps", "hearth-paint", "org.kde.konsole"], "recent": ["org.kde.dolphin", "org.kde.konsole", "org.kde.kate"]}\n' > "$HOME/.config/nimbus/desk.json"
# Hearth Time's stand-in (hosts/hearth-time), as Mantel's CI: an event for the calendar, a weather line.
mkdir -p /usr/local/bin
printf '%s\n' '#!/bin/sh' 'case "$1" in' \
  '  --events) d=$(date +%F); printf "{\"events\":[{\"title\":\"Dentist\",\"date\":\"%s\",\"lastDate\":\"%s\",\"allDay\":false,\"start\":%s000,\"end\":%s000,\"location\":\"Smile Clinic\",\"color\":\"\"}]}\n" "$d" "$d" "$(date -d "+2 hours" +%s)" "$(date -d "+3 hours" +%s)" ;;' \
  '  --weather-summary) echo "{\"place\":\"Lisbon\",\"line\":\"18° Partly cloudy\",\"day\":true}" ;;' 'esac' > /usr/local/bin/hearth-time
chmod +x /usr/local/bin/hearth-time
# Desktop icons, as Mantel's pictures have.
mkdir -p "$HOME/Desktop/Projects"
# A Flatpak-style app whose notifications take its icon from its desktop entry.
mkdir -p "$HOME/.local/share/applications"
printf '[Desktop Entry]\nType=Application\nName=Flatpak test\nIcon=applications-games\nExec=true\n' > "$HOME/.local/share/applications/org.hearth.IconTest.desktop"
# Games, as Mantel's pictures have (hearth-games' copy from last time).
mkdir -p "$HOME/.cache/hearth"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""},{"id":"beacon:bfme2","title":"The Battle for Middle-earth II","source":"beacon","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
# Apps of your own (as Mantel's pictures have): yours come first in the menus, the system's in one folder.
mkdir -p "$HOME/.local/share/applications"
for a in "hearth-maps:Hearth Maps:applications-education-miscellaneous" "hearth-paint:Hearth Paint:applications-graphics" "aa-notes:Aa Notes:accessories-text-editor" "aa-music:Aa Music:multimedia-player"; do
  IFS=: read -r id name icon <<<"$a"
  printf '[Desktop Entry]\nType=Application\nName=%s\nIcon=%s\nExec=true\n' "$name" "$icon" > "$HOME/.local/share/applications/$id.desktop"
done
# Four desktops, as Mantel's pictures have.
mkdir -p "$HOME/.config"; printf '[Desktops]\nNumber=4\nRows=1\n' > "$HOME/.config/kwinrc"

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
raw = 'RGBA' if int(res.get('format', 5)) in (16, 17, 18) else 'BGRA'
Image.frombuffer('RGBA', (W, H), data, 'raw', raw, S, 1).convert('RGB').save(sys.argv[1])
PY
shot() { timeout 25 python3 /tmp/shot.py "$OUT/$1.png" 2>>"$OUT/shot-errors.txt" || echo "no screenshot $1" >>"$OUT/shot-errors.txt"; }
js() { timeout 15 dbus-send --session --print-reply --dest=org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.evaluateScript "string:$1" 2>&1; }
setlook() {   # look mode [key=value ...]: desktop.json as Settings writes it, then the panel follows
  python3 - "$D" "$@" <<'PY'
import json, sys
p, look, mode = sys.argv[1:4]
try: d = json.load(open(p))
except Exception: d = {}
d.update(look=look, mode=mode, accent="", subTheme="", wallMatch=True)
for kv in sys.argv[4:]:
    k, v = kv.split("=", 1)
    try: d[k] = json.loads(v)
    except ValueError: d[k] = v
open(p, "w").write(json.dumps(d, indent=2) + "\n")
PY
  [ -x /usr/libexec/nimbus/hearth-plasma-looks ] && timeout 60 python3 /usr/libexec/nimbus/hearth-plasma-looks sync >>"$OUT/looks.log" 2>&1
  hearth-plasma-panel sync >>"$OUT/sync.log" 2>&1
  sleep 4
}
open_() { hearth-plasma-panel open "$1" >>"$OUT/sync.log" 2>&1; sleep 3; }

# ---- KWin and plain KDE Plasma
for p in pipewire wireplumber; do command -v $p >/dev/null && { $p >"$OUT/$p.log" 2>&1 & }; done
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth >"$OUT/kwin.log" 2>&1 &
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start: $(tail -5 "$OUT/kwin.log" | tr '\n' '|' | cut -c1-600)"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >"$OUT/kamd.log" 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >"$OUT/kglobalaccel.log" 2>&1 & break; }; done
setlook hearth dark
plasmashell >"$OUT/plasmashell.log" 2>&1 &
for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && break; sleep 1; done
sleep 6
js 'var d=desktops();for(var i=0;i<d.length;i++){d[i].wallpaperPlugin="org.kde.image";d[i].currentConfigGroup=["Wallpaper","org.kde.image","General"];d[i].writeConfig("Image","file:///usr/share/backgrounds/hearth/hearth-fireside.jpg");}' >/dev/null
sleep 2
shot ph-00-plain-kde-plasma

# ---- Hearth's panel in (the person's pick: Settings › Bar and dock › Hearth's panel)
# A new account's first sign-in (the owner, Nov 19: the default on new installs): /etc/skel's mark, then the sign-in
# step Plasma's autostart runs. It puts Hearth's panel in and checks it came up.
mkdir -p "$HOME/.config/hearth" && cp /etc/skel/.config/hearth/plasma-panel-new "$HOME/.config/hearth/" 2>/dev/null \
  || fail install "no first sign-in mark in /etc/skel"
timeout 120 hearth-plasma-panel login > "$OUT/login.json" 2>&1
grep -q '"first": "applied"' "$OUT/login.json" || fail login "a new account's first sign-in didn't get Hearth's panel: $(tr '\n' ' ' <"$OUT/login.json" | cut -c1-600)"
grep -q '"guard": "up"' "$OUT/login.json" || fail login "Hearth's bar didn't say it's up: $(tr '\n' ' ' <"$OUT/login.json" | cut -c1-600)"
[ -e "$HOME/.config/hearth/plasma-panel-new" ] && fail login "the first sign-in mark is still there (it must happen once)"
ls "$HOME/.local/share/hearth/plasma-panel/" >"$OUT/saved-panels.txt" 2>&1
for f in plasmanotifyrc plasmarc kwinrc; do echo "== $f"; cat "$HOME/.config/$f" 2>&1; done > "$OUT/kde-settings-after-apply.txt"
grep -q "CriticalInDndMode=false" "$HOME/.config/plasmanotifyrc" || fail kde-settings "KDE's critical pop-ups weren't paused (plasmanotifyrc)"
ls "$HOME/.local/share/aurorae/themes/" > "$OUT/aurorae-themes.txt" 2>&1
sleep 8
echo "::notice title=Panels::$(js 'print(panels().map(function(p){return p.location+" "+p.height+" "+p.hiding+" "+p.widgets().map(function(w){w.currentConfigGroup=["General"];return w.type+":"+w.readConfig("role","")}).join(",")}).join(" | "))' | tail -1 | cut -c1-600)"

# A window and a notification, as Mantel's pictures have.
dolphin "$HOME/Documents" >"$OUT/dolphin.log" 2>&1 &
sleep 6
cat > /tmp/notify.py <<'PY'
import dbus, sys
n = dbus.Interface(dbus.SessionBus().get_object('org.freedesktop.Notifications', '/org/freedesktop/Notifications'), 'org.freedesktop.Notifications')
a = sys.argv + [''] * 12
acts = [x for x in a[6:10] if x != '']
hints = {'urgency': dbus.Byte(int(a[5]) if a[5] else 1)}
if a[10]: hints['desktop-entry'] = a[10]
print(n.Notify(a[1], 0, a[4], a[2], a[3], acts, hints, 30000))
PY
note() { timeout 10 python3 /tmp/notify.py "$@" >>"$OUT/notify.log" 2>&1 || echo "notify failed: $*" >>"$OUT/notify.log"; }
note "Printers" "2 cartridges are low" "Black toner (12 %), Magenta (8 %) · HP LaserJet M209"; sleep 1
note "Other App" "Second one" "For the count on the bell"; sleep 1
note "Test App" "Hello from a test" "Notifications pop up in the corner" "" 1 default Open later "Remind me"; sleep 1
note "Flatpak test" "From a Flatpak app" "Its icon comes from its desktop entry" "" 1 "" "" "" "" org.hearth.IconTest; sleep 1
note "Friends" "Mira" "Ready for co-op tonight?"; sleep 1
note "Hearth" "Download finished" "Celeste is ready to play" "" 2
sleep 2
shot ph-01-hearth-dark-desktop

# ---- every look, as Mantel's CI: desktop, Quick settings, the app menu, the dashboard
n=10
for lm in "hearth dark" "steam dark" "gaming dark" "zen dark" "glass dark" "synthwave dark" "cabin dark" "console dark" "hearth light" "glass light"; do
  set -- $lm
  setlook "$1" "$2"
  shot "ph-$n-$1-$2-1-desktop"
  open_ quick; shot "ph-$n-$1-$2-2-quick-settings"; open_ quick
  open_ launcher; shot "ph-$n-$1-$2-3-launcher"; open_ launcher
  open_ calendar; shot "ph-$n-$1-$2-4-notifications-dashboard"; open_ calendar
  n=$((n + 1))
done

# ---- Mantel's power menu, the desktop's right-click menu, the volume and brightness bars (Hearth dark)
setlook hearth dark
open_ power; shot ph-20-power-menu; hearth-plasma-panel open power >/dev/null 2>&1; timeout 5 wtype -k Escape 2>/dev/null; sleep 2
open_ deskmenu; shot ph-21-desktop-menu; timeout 5 wtype -k Escape 2>/dev/null; sleep 2
open_ osd; sleep 0; shot ph-22-volume-bar; sleep 2
# Alt+Tab (KWin's task switcher, Mantel's own layout org.hearth.switcher), with two windows open
konsole >/dev/null 2>&1 & sleep 5
timeout 5 dbus-send --session --dest=org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.invokeShortcut "string:Walk Through Windows" 2>/dev/null; sleep 1.5; shot ph-24-alt-tab; timeout 5 wtype -k Escape 2>/dev/null; sleep 1
open_ osd-bright; shot ph-23-brightness-bar; sleep 2

# ---- the five app menus (Settings › Bar and dock › App menu), Hearth dark
for st in hearth centred tiles start classic; do
  setlook hearth dark "appMenu=$st"
  open_ launcher; shot "ph-30-menu-$st"; open_ launcher
done
# Searching (the Centred menu): typing lands in its field.
setlook hearth dark "appMenu=centred"
open_ launcher
command -v wtype >/dev/null && { wtype "term"; sleep 2; shot ph-31-menu-search-centred; wtype -k Escape; }
open_ launcher
setlook hearth dark "appMenu=hearth"

# ---- Bar layouts: one main bar at the bottom, auto-hide
setlook steam dark 'barLayout=one' 'barEdge=bottom'
shot ph-40-steam-one-bar-bottom
setlook hearth dark 'barLayout=split' 'dockMode=autohide'
shot ph-41-hearth-dock-autohide
setlook hearth dark 'dockMode=shown'

# ---- at rest: plasmashell's processor time over a minute with Hearth's panel up
t0=$(awk '{print $14+$15}' /proc/$(pgrep -x plasmashell | head -1)/stat 2>/dev/null); sleep 60
t1=$(awk '{print $14+$15}' /proc/$(pgrep -x plasmashell | head -1)/stat 2>/dev/null)
echo "::notice title=At rest::plasmashell used $(( (t1 - t0) )) ticks (1/100 s) in 60 s with Hearth's panel up"

# ---- back to KDE's panel (Settings › Bar and dock › KDE's panel)
hearth-plasma-panel undo > "$OUT/undo.json" 2>&1; sleep 6
for f in plasmanotifyrc plasmarc kwinrc; do echo "== $f"; cat "$HOME/.config/$f" 2>&1; done > "$OUT/kde-settings-after-undo.txt"
grep -q "CriticalInDndMode=false" "$HOME/.config/plasmanotifyrc" && fail undo "KDE's critical pop-ups setting wasn't put back"
shot ph-50-back-to-kde-panel
left=$(js 'var n=0;var ps=panels();for(var i=0;i<ps.length;i++)n+=ps[i].widgets("org.hearthos.panel").length;print(n+"/"+ps.length)' | tail -1)
case "$left" in *'"0/'[1-9]*) ;; *) fail undo "Hearth's panel didn't come out, or no KDE panel came back: $left" ;; esac

# ---- what plasmashell said about Hearth's files
bad=$(grep -E "org\.hearthos" "$OUT/plasmashell.log" | grep -iE "error|TypeError|ReferenceError|is not a type|is not installed|Cannot assign|not defined|Unable to assign|Cannot read" | sort -u | head -25)
[ -n "$bad" ] && echo "::warning title=plasmashell about Hearth's panel::$(printf '%s' "$bad" | sed 's#file:///usr/share/plasma/plasmoids/##' | cut -c1-230 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
grep -E "org\.hearthos|plasma-hearth" "$OUT/plasmashell.log" | sort | uniq -c | sort -rn | head -80 > "$OUT/hearth-messages.txt"
pkill dolphin; pkill plasmashell; sleep 2; pkill -9 -f /tmp/kwin_wayland
ls "$OUT"/*.png | wc -l > "$OUT/count.txt"
exit 0
