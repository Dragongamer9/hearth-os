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
# A Flatpak app whose notifications take its icon from its desktop entry, as Mantel's CI makes it (desktop-test.yml:
# a green test icon in your Flatpak exports, which Fedora's sign-in puts in XDG_DATA_DIRS).
UF="$HOME/.local/share/flatpak/exports/share"
mkdir -p "$UF/applications" "$UF/icons/hicolor/scalable/apps"
printf '[Desktop Entry]\nType=Application\nName=Aa Flatpak (yours)\nExec=/usr/bin/flatpak run --branch=stable --arch=x86_64 org.hearth.IconTest\nIcon=org.hearth.IconTest\nStartupWMClass=icontest-window\n' > "$UF/applications/org.hearth.IconTest.desktop"
printf '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect x="4" y="4" width="56" height="56" rx="14" fill="#2f9e44"/><circle cx="32" cy="32" r="14" fill="#fff"/></svg>' > "$UF/icons/hicolor/scalable/apps/org.hearth.IconTest.svg"
export XDG_DATA_DIRS="$UF:/var/lib/flatpak/exports/share:/usr/local/share:/usr/share"
# Games, as Mantel's pictures have (hearth-games' copy from last time).
mkdir -p "$HOME/.cache/hearth"
printf '%s' '[{"id":"steam:570","title":"Dota 2","source":"steam","cover":"","wide":"","icon":""},{"id":"manual:1","title":"Celeste","source":"manual","cover":"","wide":"","icon":""},{"id":"beacon:bfme2","title":"The Battle for Middle-earth II","source":"beacon","cover":"","wide":"","icon":""}]' > "$HOME/.cache/hearth/desktop-games.json"
# Apps of your own (as Mantel's pictures have): yours come first in the menus, the system's in one folder.
mkdir -p "$HOME/.local/share/applications"
for a in "hearth-maps:Hearth Maps:applications-education-miscellaneous" "hearth-paint:Hearth Paint:applications-graphics" "aa-notes:Aa Notes:accessories-text-editor" "aa-music:Aa Music:multimedia-player"; do
  IFS=: read -r id name icon <<<"$a"
  printf '[Desktop Entry]\nType=Application\nName=%s\nIcon=%s\nExec=true\n' "$name" "$icon" > "$HOME/.local/share/applications/$id.desktop"
done
# Mantel's hearth-notifyd (the tray's background apps, camera, printing, disks) reads these instead of the PC, as
# Mantel's CI does (notify-widgets.yml): nothing of the kind going on, until the tray's own picture.
FX=/tmp/notifyd-fx; mkdir -p $FX; export HEARTH_NOTIFYD_FIXTURES=$FX
fx_none() { echo '{"disks": []}' > $FX/disks.json; echo '{"jobs": []}' > $FX/printers.json; echo '{"apps": []}' > $FX/camera.json; echo '{"apps": []}' > $FX/background.json; }
fx_none
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
export QT_LOGGING_RULES="kwin_scripting.debug=true;js.debug=true"
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
for f in plasmanotifyrc plasmarc kwinrc kscreenlockerrc; do echo "== $f"; cat "$HOME/.config/$f" 2>&1; done > "$OUT/kde-settings-after-apply.txt"
grep -q "CriticalInDndMode=false" "$HOME/.config/plasmanotifyrc" || fail kde-settings "KDE's critical pop-ups weren't paused (plasmanotifyrc)"
ls "$HOME/.local/share/aurorae/themes/" > "$OUT/aurorae-themes.txt" 2>&1
grep -q "Theme=org.hearthos.mantel-lock" "$HOME/.config/kscreenlockerrc" 2>/dev/null \
  || fail lock "Hearth's lock screen wasn't picked for KDE's screen locker: $(cat "$HOME/.config/kscreenlockerrc" 2>&1 | tr '\n' ' ' | cut -c1-200) $(grep -a "lock screen stays" "$OUT/login.json" | cut -c1-300)"
sleep 8
echo "::notice title=Panels::$(js 'print(panels().map(function(p){return p.location+" "+p.height+" "+p.hiding+" "+p.widgets().map(function(w){w.currentConfigGroup=["General"];return w.type+":"+w.readConfig("role","")}).join(",")}).join(" | "))' | tail -1 | cut -c1-600)"

# A window and notifications, as Mantel's pictures have. Mantel's CI sends them over its run (desktop-test.yml): the
# older ones before a restart of the desktop (in the dashboard they're the history Hearth kept, with "Open <app>"
# instead of their buttons), then Mira 2 s before the looks (gone from the corner after 6 s, Mantel's timing) and
# Hearth's "Download finished" (Hearth's own warning: it stays until you close it). Sent as notify-send sends them.
dolphin "$HOME/Documents" >"$OUT/dolphin.log" 2>&1 &
sleep 6
cat > /tmp/notify.py <<'PY'
import dbus, sys
n = dbus.Interface(dbus.SessionBus().get_object('org.freedesktop.Notifications', '/org/freedesktop/Notifications'), 'org.freedesktop.Notifications')
a = sys.argv + [''] * 12
acts = [x for x in a[6:10] if x != '']
hints = {'urgency': dbus.Byte(int(a[5]) if a[5] else 1)}
if a[10]: hints['desktop-entry'] = a[10]
print(n.Notify(a[1], 0, a[4], a[2], a[3], acts, hints, -1))
PY
note() { timeout 10 python3 /tmp/notify.py "$@" >>"$OUT/notify.log" 2>&1 || echo "notify failed: $*" >>"$OUT/notify.log"; }
note "Printers" "2 cartridges are low" "Black toner (12 %), Magenta (8 %) · HP LaserJet M209"; sleep 1
note "Test App" "Hello from a test" "Notifications pop up in the corner" "" 1 default Open later "Remind me"; sleep 1
note "Other App" "Second one" "For the count on the bell"; sleep 1
note "Flatpak test" "From a Flatpak app" "Its icon comes from its desktop entry" "" 1 "" "" "" "" org.hearth.IconTest
sleep 8
# The desktop restarted (plasmashell, as a sign-in does): Hearth's history comes back from its file.
cp "$HOME/.local/state/nimbus/notifications.json" "$OUT/history-before-restart.json" 2>/dev/null
kquitapp6 plasmashell >/dev/null 2>&1 || pkill -x plasmashell
for i in $(seq 1 20); do pgrep -x plasmashell >/dev/null || break; sleep 1; done
plasmashell >>"$OUT/plasmashell.log" 2>&1 &
for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && break; sleep 1; done
sleep 10
grep -q "Flatpak test" "$OUT/history-before-restart.json" 2>/dev/null || fail history "Hearth's notification history wasn't kept in its file (~/.local/state/nimbus/notifications.json)"
note "Hearth" "Download finished" "Celeste is ready to play"; sleep 1
note "Friends" "Mira" "Ready for co-op tonight?"
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

# ---- A critical notification: Mantel's pop-up only (it stays until you close it); KDE's own never shows while
# Hearth's notifications are in use (KWin's list of windows: no notification window of KDE's).
wins() {   # KWin's windows now, one line each, in OUT/$1.txt
  local id
  id=$(dbus-send --session --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript "string:/tmp/wins.js" "string:hearthwins$RANDOM" 2>/dev/null | awk '/int32/{print $2}')
  [ -n "$id" ] || { echo "no KWin script" > "$OUT/$1.txt"; return; }
  dbus-send --session --print-reply --dest=org.kde.KWin "/Scripting/Script$id" org.kde.kwin.Script.run >/dev/null 2>&1; sleep 2
  grep -a "HEARTHWIN" "$OUT/kwin.log" | sed 's/.*HEARTHWIN //' | tail -40 > "$OUT/$1.txt"
  dbus-send --session --print-reply --dest=org.kde.KWin "/Scripting/Script$id" org.kde.kwin.Script.stop >/dev/null 2>&1
  : > /dev/null
}
cat > /tmp/wins.js <<'JS'
var ws = workspace.stackingOrder;
print("HEARTHWIN ---- " + ws.length + " windows");
for (var i = 0; i < ws.length; i++) {
  var w = ws[i], g = w.frameGeometry;
  print("HEARTHWIN " + w.resourceClass + " | " + w.caption + " | notification=" + w.notification + " critical=" + w.criticalNotification
        + " osd=" + w.onScreenDisplay + " | " + Math.round(g.x) + "," + Math.round(g.y) + " " + Math.round(g.width) + "x" + Math.round(g.height));
}
JS
# Plasma's own pop-up code and the setting it reads (what the next fix would need, if one were needed)
{ for f in $(grep -rlE "criticalPopupsInDoNotDisturbMode" /usr/share/plasma/plasmoids/org.kde.plasma.notifications /usr/lib64/qt6/plugins/plasma/applets 2>/dev/null | head -4); do
    echo "== $f"; grep -an -B6 -A4 "criticalPopupsInDoNotDisturbMode" "$f" 2>/dev/null | cut -c1-200 | head -40
    strings -n 6 "$f" 2>/dev/null | grep -n -B8 -A4 "criticalPopupsInDoNotDisturbMode" | cut -c1-200 | head -60; done
  grep -A3 "Notifications" "$HOME/.config/plasmanotifyrc"; } > "$OUT/kde-popups-code.txt" 2>&1
crit=$(timeout 10 python3 /tmp/notify.py "Battery" "Battery low" "10 % left: plug in soon" "" 2 | tail -1)
sleep 3
shot ph-25-critical-notification
wins kwin-windows-critical
if grep -qE "notification=true|critical=true" "$OUT/kwin-windows-critical.txt"; then
  fail kde-popups "KDE's own pop-up showed for a critical notification while Hearth's notifications are in use: $(grep -E "notification=true|critical=true" "$OUT/kwin-windows-critical.txt" | head -3 | tr '\n' ' ' | cut -c1-400)"
fi
grep -q "HEARTHWIN\|windows" "$OUT/kwin-windows-critical.txt" || echo "::warning title=KWin windows::couldn't list KWin's windows: $(head -c 300 "$OUT/kwin-windows-critical.txt")"
[ -n "$crit" ] && timeout 5 dbus-send --session --dest=org.freedesktop.Notifications /org/freedesktop/Notifications org.freedesktop.Notifications.CloseNotification "uint32:$crit" 2>/dev/null
sleep 2

# ---- The tray's extras (Mantel's hearth-notifyd, through hearth-plasma-panel's go-between), as Mantel's CI
# pictures them (notify-widgets.yml 02-bar-and-dock): a USB stick, two print jobs, the camera in use, background apps.
echo '{"disks": [{"path": "/dev/sda1", "name": "USB STICK", "size": "29.8G", "mount": "/run/media/you/USB STICK", "fstype": "vfat", "drive": "/dev/sda"}]}' > $FX/disks.json
echo '{"jobs": [{"job": "Office-12", "printer": "Office", "user": "you", "size": "20480"}, {"job": "Office-13", "printer": "Office", "user": "you", "size": "4096"}]}' > $FX/printers.json
echo '{"apps": [{"name": "Hearth Call", "pid": 4242}]}' > $FX/camera.json
echo '{"apps": [{"app": "com.valvesoftware.Steam", "message": "Downloading 2 games", "instance": "1"}, {"app": "org.kde.kdeconnect.daemon", "message": "", "instance": "2"}]}' > $FX/background.json
sleep 22
shot ph-26-tray-extras
python3 -c 'import sys; from PIL import Image; Image.open(sys.argv[1]).crop((960, 0, 1920, 90)).save(sys.argv[2])' "$OUT/ph-26-tray-extras.png" "$OUT/ph-26b-tray-extras-close.png" 2>/dev/null
pgrep -fa "notifyd" > "$OUT/notifyd-processes.txt" 2>&1
fx_none; sleep 22

# ---- Mantel's lock screen in KDE's screen locker (its own test run, on the screen: what locking shows)
G=$(ls /usr/libexec/kscreenlocker_greet /usr/lib64/libexec/kscreenlocker_greet 2>/dev/null | head -1)
if [ -n "$G" ]; then
  "$G" --testing > "$OUT/lock-greeter.log" 2>&1 & gp=$!
  sleep 6; shot ph-27-lock-screen; kill $gp 2>/dev/null; sleep 1
  grep -q "hearth-lock-screen: loaded" "$OUT/lock-greeter.log" || fail lock "KDE's screen locker didn't show Hearth's lock screen: $(grep -v 'notify signal' "$OUT/lock-greeter.log" | tail -4 | tr '\n' '|' | cut -c1-400)"
else
  fail lock "KDE's screen locker (kscreenlocker_greet) isn't installed"
fi

# ---- Mantel's power menu, the desktop's right-click menu, the volume and brightness bars (Hearth dark)
setlook hearth dark
open_ power; shot ph-20-power-menu; open_ close
open_ deskmenu; shot ph-21-desktop-menu; open_ close
open_ osd; sleep 0; shot ph-22-volume-bar; sleep 2
# Alt+Tab (KWin's task switcher, Mantel's own layout org.hearth.switcher), with two windows open and nothing of
# Hearth's open over them
open_ close
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
for f in plasmanotifyrc plasmarc kwinrc kscreenlockerrc; do echo "== $f"; cat "$HOME/.config/$f" 2>&1; done > "$OUT/kde-settings-after-undo.txt"
grep -q "CriticalInDndMode=false" "$HOME/.config/plasmanotifyrc" && fail undo "KDE's critical pop-ups setting wasn't put back"
grep -q "Theme=org.hearthos.mantel-lock" "$HOME/.config/kscreenlockerrc" 2>/dev/null && fail undo "KDE's lock screen setting wasn't put back"
shot ph-50-back-to-kde-panel
left=$(js 'var n=0;var ps=panels();for(var i=0;i<ps.length;i++)n+=ps[i].widgets("org.hearthos.panel").length;print(n+"/"+ps.length)' | tail -1)
case "$left" in *'"0/'[1-9]*) ;; *) fail undo "Hearth's panel didn't come out, or no KDE panel came back: $left" ;; esac

# ---- Firehold on Plasma's style of the shared Quick settings and notifications (style "firehold", mockups
# docs/mockups/firehold-plasma/07-10), on plain KDE Plasma: both in a top panel, as Firehold's Desk bar has them,
# opened the way Firehold's bar opens them (Plasma's "activate widget" shortcut). Quick settings reads the same
# stand-in hearth-quick answers as the mockups show (HEARTH_QUICK_FIXTURE).
QF=/tmp/hearth-quick-fixture.json
qs_fixture() {   # desk | couch
  python3 - "$QF" "$1" <<'PY'
import json, sys
desk = {"device": "Nvidia PC", "line": "Nvidia PC · plugged in · up 3 days", "battery": -1, "user": "Chris",
 "wifi": {"on": True, "ssid": "Home-5G", "strength": "strong"}, "bt": {"on": True, "connected": 3, "names": ["Xbox Controller", "Arctis 7", "MX Master"]},
 "osk": False, "night": {"on": True, "until": "7 AM"}, "dnd": {"on": False, "byHand": False, "schedule": True, "from": "11 PM", "to": "7 AM", "games": True, "family": True},
 "sinks": [{"name": "headset", "word": "Headset", "default": True, "volume": 64}, {"name": "hdmi", "word": "TV (HDMI)", "default": False, "volume": 50}, {"name": "spk", "word": "Speakers", "default": False, "volume": 40}],
 "brightness": [{"id": "lg", "label": "LG monitor", "percent": 80, "max": 100}], "power": "balanced",
 "powerNames": {"power-saver": "Power saver", "balanced": "Balanced", "performance": "Performance"},
 "devices": [{"name": "Xbox Controller", "kind": "pad", "percent": 72, "player": 1, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True},
             {"name": "Arctis 7", "kind": "headset", "percent": 60, "low": False}, {"name": "MX Master", "kind": "mouse", "percent": 40, "low": False}],
 "gameModeOff": False, "modes": [{"name": "HDMI-A-1", "tv": True, "label": "TCL TV", "mode": "couch"}, {"name": "Virtual-1", "tv": False, "label": "LG monitor", "mode": "desk"}]}
couch = dict(desk, device="ROG Ally", line="ROG Ally · battery 64% · up 2 h", battery=64, batteryMore={"left": "about 2 h 40 min", "limit": 80},
 bt={"on": True, "connected": 1, "names": ["Earbuds"]}, night={"on": False, "until": "9 PM"},
 sinks=[{"name": "buds", "word": "Earbuds", "default": True, "volume": 45}], brightness=[{"id": "", "label": "This screen", "percent": 70, "max": 100}],
 powerNames={"power-saver": "Silent · 10 W", "balanced": "Balanced · 18 W", "performance": "Turbo · 25 W"},
 devices=[{"name": "Earbuds", "kind": "headset", "percent": 70, "low": False}, {"name": "8BitDo", "kind": "pad", "percent": 18, "player": 2, "low": True}],
 gameModeOff=True, modes=[{"name": "Virtual-1", "tv": False, "label": "Built-in", "mode": "couch"}])
json.dump(desk if sys.argv[2] == "desk" else couch, open(sys.argv[1], "w"))
PY
}
qs_fixture desk
# Plasma reads the fixture's path from its own environment: Plasma again, with it (as the sign-in would start it).
kwriteconfig6 --file plasmanotifyrc --group Notifications --key CriticalInDndMode --notify false 2>/dev/null
kquitapp6 plasmashell >/dev/null 2>&1 || pkill -x plasmashell
for i in $(seq 1 20); do pgrep -x plasmashell >/dev/null || break; sleep 1; done
HEARTH_QUICK_FIXTURE=$QF plasmashell >>"$OUT/plasmashell.log" 2>&1 &
for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && break; sleep 1; done
sleep 6
ids=$(js 'var p=new Panel("org.kde.panel");p.location="top";p.height=40;var a=p.addWidget("org.hearthos.quicksettings");a.currentConfigGroup=["General"];a.writeConfig("style","firehold");var b=p.addWidget("org.hearthos.notifications");b.currentConfigGroup=["General"];b.writeConfig("style","firehold");print(a.id+","+b.id)' | grep -o '"[0-9]*,[0-9]*"' | tr -d '"')
echo "::notice title=Firehold style::the shared plasmoids in a top panel: $ids"
qid=${ids%,*}; nid=${ids#*,}
act() { timeout 5 dbus-send --session --dest=org.kde.kglobalaccel /component/plasmashell org.kde.kglobalaccel.Component.invokeShortcut "string:activate widget $1" 2>/dev/null; sleep 3; }
sleep 6
if [ -n "$qid" ] && [ -n "$nid" ]; then
  act "$qid"; shot ph-60-firehold-quick-settings-desk; act "$qid"
  qs_fixture couch
  act "$qid"; timeout 5 wtype -k Down -k Down -k Down -k Down -k Down -k Down -k Down -k Down 2>/dev/null; sleep 1; shot ph-61-firehold-quick-settings-couch; act "$qid"
  # The cards (09): an urgent one stays, a picture, a message with a reply box, "2 more"
  note "Firefox" "Firefox download finished" "hollow-pines.tar.zst · 2.9 GB" ; sleep 1
  note "Hearth OS" "Hearth OS update ready" "Nothing changes until you restart" ; sleep 1
  note "Discord" "Ryder in #bfme-nights" "“Edain game at 11? I'll host.”" "" 1 read "Mark read"; sleep 1
  note "Hearth Recorder" "Saved the last 30 seconds" "Rise of the Witch-king · 1440p · 96 MB" "" 1 open Open share "Copy a share link"; sleep 1
  note "Downloads" "Helldivers 2 is ready" "The 6.0 GB update finished (Steam)." "" 1 play Play tv "Play on the TV"; sleep 1
  note "Hearth Link" "Dad's PC asks for help" "“Trains 2010 won't start after the update.” He's at his PC now." "" 2 connect "Connect (Remote desktop)" later "Call him back later"
  sleep 2
  shot ph-62-firehold-notification-cards
  wins kwin-windows-firehold
  act "$nid"; shot ph-63-firehold-notification-list; act "$nid"
else
  fail firehold "the shared plasmoids didn't go into a panel: $(js 'print(panels().length)' | tail -1)"
fi
kwriteconfig6 --file plasmanotifyrc --group Notifications --key CriticalInDndMode --delete --notify 2>/dev/null

# ---- what plasmashell said about Hearth's files
bad=$(grep -E "org\.hearthos" "$OUT/plasmashell.log" | grep -iE "error|TypeError|ReferenceError|is not a type|is not installed|Cannot assign|not defined|Unable to assign|Cannot read" | sort -u | head -25)
[ -n "$bad" ] && echo "::warning title=plasmashell about Hearth's panel::$(printf '%s' "$bad" | sed 's#file:///usr/share/plasma/plasmoids/##' | cut -c1-230 | sed -e ':a;N;$!ba;s/\n/%0A/g')"
grep -E "org\.hearthos|plasma-hearth" "$OUT/plasmashell.log" | sort | uniq -c | sort -rn | head -80 > "$OUT/hearth-messages.txt"
pkill dolphin; pkill plasmashell; sleep 2; pkill -9 -f /tmp/kwin_wayland
ls "$OUT"/*.png | wc -l > "$OUT/count.txt"
exit 0
