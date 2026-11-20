#!/bin/bash
# Hearth Games' desktop icons on the desktops (games-desktop-icons.yml, job "desktops", Fedora 44 as Anatase): the
# Desktop folder the "app" job left before Add to desktop, after it, and after uninstalling the game (state-*.tgz,
# with the launcher Hearth Games made and its picture), shown on
#   1. Mantel's desktop on labwc (Desktop.qml with hearth-desk), the same desktop Firehold's Desk draws;
#   2. KDE Plasma's own desktop (its folder view), on KDE's KWin;
#   3. Hearth's panel on Plasma (Mantel's Desktop.qml in a layer over Plasma's desktop).
# Checks: the new icon goes in a free spot and no other icon moves (Mantel: desk.json's saved places stay exactly as
# they were, the new one in the first free place; Plasma: only one icon-sized part of the screen changes), and it
# starts the game without asking (hearth-desk launch, as a double-click on Mantel's desktop; kioclient exec, KDE's own
# open, which asks first for a launcher it doesn't trust) through `nimbus --play-now gog:1207658924`.
#   games-desktop-icons-desktops.sh STATES OUT
set +e +o pipefail
STATES=$(cd "$1" && pwd); OUT=${2:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/step.log") 2>&1
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }
ok() { echo "ok: $1"; }
SRC=$PWD
export HOME=/tmp/home XDG_RUNTIME_DIR=/tmp/xdg USER=chris
mkdir -p "$HOME/.config/nimbus" "$HOME/.local/share/nimbus" "$HOME/Desktop" "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus"
export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
# The game starts through Hearth: a stand-in nimbus notes how it was started.
mkdir -p /tmp/fakebin; printf '#!/bin/sh\necho "$*" >> /tmp/home/nimbus-calls\n' > /tmp/fakebin/nimbus; chmod +x /tmp/fakebin/nimbus
export PATH="/tmp/fakebin:$PATH"

# A state from the app job, put in place the way files change on a desktop: the Desktop folder itself stays (the
# desktops watch it), files come and go in it.
apply() {
  python3 - "$STATES/state-$1.tgz" "$HOME" <<'PY'
import os, shutil, sys, tarfile, tempfile
tgz, home = sys.argv[1:3]
t = tempfile.mkdtemp()
with tarfile.open(tgz) as f:
    f.extractall(t)
src = os.path.join(t, home.lstrip('/'))
for sub in ('.local/share/nimbus/desktop-icons', 'art'):
    a = os.path.join(src, sub)
    if os.path.isdir(a):
        shutil.copytree(a, os.path.join(home, sub), dirs_exist_ok=True)
want, have = os.path.join(src, 'Desktop'), os.path.join(home, 'Desktop')
os.makedirs(have, exist_ok=True)
names = set(os.listdir(want))
for n in os.listdir(have):
    if n not in names:
        p = os.path.join(have, n)
        shutil.rmtree(p) if os.path.isdir(p) and not os.path.islink(p) else os.remove(p)
for n in sorted(names):
    a, b = os.path.join(want, n), os.path.join(have, n)
    if os.path.isdir(a):
        os.makedirs(b, exist_ok=True)
    elif not os.path.exists(b) or open(a, 'rb').read() != open(b, 'rb').read():
        shutil.copy2(a, b)
print('desktop:', sorted(os.listdir(have)))
PY
  # What Hearth Games sends when it adds or removes an icon (desktop_icons.rs tell_kde): KDE's KDirNotify, so Plasma's
  # desktop shows the change at once.
  [ -n "$TELL_KDE" ] && timeout 5 dbus-send --session --type=signal / org.kde.KDirNotify.FilesAdded "string:file://$HOME/Desktop"
  true
}
places() { python3 -c 'import json,sys; print(json.dumps(json.load(open(sys.argv[1])).get("icons", {}), sort_keys=True))' "$HOME/.config/nimbus/desk.json" 2>/dev/null; }
# Only one icon's worth of the screen changed between two pictures (above the bar at the bottom): nothing moved.
one_icon() {   # before after what
  python3 - "$OUT/$1.png" "$OUT/$2.png" <<'PY' > "$OUT/diff-$2.txt"
import sys
from PIL import Image, ImageChops
a, b = Image.open(sys.argv[1]).convert('RGB'), Image.open(sys.argv[2]).convert('RGB')
w, h = a.size
box = (0, 0, w, int(h * 0.88))
d = ImageChops.difference(a.crop(box), b.crop(box)).convert('L').point(lambda v: 255 if v > 40 else 0)
print(d.getbbox())
PY
  bb=$(cat "$OUT/diff-$2.txt")
  python3 -c 'import sys,ast; b=ast.literal_eval(sys.argv[1]); sys.exit(0 if b and b[2]-b[0] <= 260 and b[3]-b[1] <= 260 else 1)' "$bb" \
    && ok "$3: only the new icon appeared ($bb)" || fail "$3" "more than one icon's place changed on the desktop (changed area $bb): something moved, or nothing appeared"
}
launched() {   # what
  : > /tmp/home/nimbus-calls
  "$@" > "$OUT/launch.log" 2>&1 &
  for i in $(seq 1 20); do grep -q . /tmp/home/nimbus-calls 2>/dev/null && break; sleep 1; done
}
check_launch() {
  if grep -qx -- '--play-now gog:1207658924' /tmp/home/nimbus-calls; then ok "$1: the icon starts the game through Hearth"
  else fail "$1" "the icon didn't start nimbus --play-now gog:1207658924 (asked to trust it?): $(cat /tmp/home/nimbus-calls) | $(tail -c 400 "$OUT/launch.log" | tr '\n' '|')"; fi
}

# Your saved places on Mantel's desktop (desk.json "icons", as dragging them leaves them): Projects at the top left,
# notes.txt one place lower down the gap, My Game in the fourth column. The first free place is the gap, (0, 1).
printf '{"icons": {"Projects": [0, 0], "notes.txt": [0, 2], "My Game.desktop": [3, 0]}}\n' > "$HOME/.config/nimbus/desk.json"
echo '{}' > "$HOME/.local/share/nimbus/desktop.json"
apply before
BEFORE=$(places)

# ---------------------------------------------------------------- 1. Mantel on labwc
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1
export QT_QUICK_BACKEND=software NIMBUS_SHARE="$SRC/hosts/linux" HEARTH_CHECKS=1
chmod +x "$SRC/hosts/linux/desktop/labwc/autostart" "$SRC/hosts/linux/desktop/theme/apply.sh"
labwc -C "$SRC/hosts/linux/desktop/labwc" > "$OUT/labwc.log" 2>&1 &
LABWC=$!
for i in $(seq 1 30); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 1; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 || true
SH="$NIMBUS_SHARE/desktop/shell"
q() { quickshell ipc -p "$SH" call "$@" 2>/dev/null | grep -v WARN | tail -n1; }
sleep 25
grim "$OUT/mantel-1-before.png" || fail mantel "no picture"
apply added; sleep 8
grim "$OUT/mantel-2-icon-added.png"
AFTER=$(places)
echo "::notice title=Mantel's saved places::before: $BEFORE | after Add to desktop: $AFTER"
python3 - "$BEFORE" "$AFTER" <<'PY' && ok "Mantel: your icons kept their places, the new one took the first free place" || fail mantel "icons moved, or the new one isn't in the first free place: $BEFORE → $AFTER"
import json, sys
b, a = json.loads(sys.argv[1]), json.loads(sys.argv[2])
assert all(a.get(k) == v for k, v in b.items()), 'a saved place changed'
assert a.get('Witcher Tales.desktop') == [0, 1], a.get('Witcher Tales.desktop')
PY
# A double-click on Mantel's desktop runs the launcher through hearth-desk launch.
launched python3 "$SH/hearth-desk" launch "$HOME/Desktop/Witcher Tales.desktop"
check_launch mantel
apply uninstalled; sleep 8
grim "$OUT/mantel-3-after-uninstall.png"
LAST=$(places)
python3 - "$BEFORE" "$LAST" <<'PY' && ok "Mantel: after uninstalling, your icons are where they were" || fail mantel "icons moved after uninstalling: $BEFORE → $LAST"
import json, sys
b, a = json.loads(sys.argv[1]), json.loads(sys.argv[2])
assert all(a.get(k) == v for k, v in b.items())
PY
# Firehold's Desk (unlocked the way the owner does: seven taps on the version), the same desktop under its bar and
# dock, with the icon added again (it goes back to the place it had).
apply added; sleep 6
for i in 1 2 3 4 5 6 7; do q settings tapversion >/dev/null; sleep 0.3; done
sleep 3; q firehold input mouse >/dev/null; sleep 5
grim "$OUT/mantel-4-firehold-desk-icon.png"
echo "::notice title=Firehold::$(q firehold state | head -c 300)"
cp "$HOME/.config/nimbus/desk.json" "$OUT/desk-after-mantel.json"
echo "::notice title=Mantel log::$(grep -ai 'desktop\.qml\|hearth-desk' "$XDG_RUNTIME_DIR/nimbus-shell.log" 2>/dev/null | tail -5 | cut -c1-200 | tr '\n' '|')"
pkill -f "quickshell" ; kill $LABWC; sleep 3; pkill -x labwc
unset WAYLAND_DISPLAY WLR_BACKENDS WLR_RENDERER WLR_LIBINPUT_NO_DEVICES WLR_HEADLESS_OUTPUTS QT_QUICK_BACKEND

# ---------------------------------------------------------------- 2. KDE Plasma's own desktop
apply before
TELL_KDE=1
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export QT_FORCE_STDERR_LOGGING=1 KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1 LIBGL_ALWAYS_SOFTWARE=1 HEARTH_PLASMA_PANEL_CI=1
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu 2>/dev/null | head -1)" applications.menu)
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
cp "$(command -v kwin_wayland)" /tmp/kwin_wayland
/tmp/kwin_wayland --virtual --width 1920 --height 1080 --no-lockscreen --socket wayland-hearth > "$OUT/kwin.log" 2>&1 &
for i in $(seq 1 40); do [ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] && break; sleep 1; done
[ -S "$XDG_RUNTIME_DIR/wayland-hearth" ] || { fail KWin "didn't start: $(tail -5 "$OUT/kwin.log" | tr '\n' '|' | cut -c1-600)"; exit 0; }
export WAYLAND_DISPLAY=wayland-hearth QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" > "$OUT/kamd.log" 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" > "$OUT/kglobalaccel.log" 2>&1 & break; }; done
plasmashell > "$OUT/plasmashell.log" 2>&1 &
for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && break; sleep 1; done
sleep 6
# A still wallpaper (so the pictures only differ where icons are), and the icons arranged by hand (Plasma keeps each
# where it is and puts new ones in a free spot, as for someone who has placed theirs).
js 'var d=desktops();for(var i=0;i<d.length;i++){d[i].wallpaperPlugin="org.kde.image";d[i].currentConfigGroup=["Wallpaper","org.kde.image","General"];d[i].writeConfig("Image","file:///usr/share/backgrounds/hearth/hearth-fireside.jpg");d[i].currentConfigGroup=["General"];d[i].writeConfig("sortMode",-1);}print(d.map(function(x){return x.type}).join(","))' > "$OUT/plasma-desktops.txt"
echo "::notice title=Plasma's desktop::$(tail -1 "$OUT/plasma-desktops.txt" | cut -c1-200)"
sleep 6
shot plasma-1-before
apply added; sleep 8
shot plasma-2-icon-added
if grep -q None "$OUT/diff-plasma-2-icon-added.txt" 2>/dev/null; then
  # Plasma's folder watch hasn't noticed the new file yet (CI's container): Plasma started again, as a sign-in does.
  echo "::warning title=plasma::the new icon didn't show within 8 s; Plasma restarted to show it (see plasma-2a-after-restart)"
  kquitapp6 plasmashell >/dev/null 2>&1 || pkill -x plasmashell
  for i in $(seq 1 20); do pgrep -x plasmashell >/dev/null || break; sleep 1; done
  plasmashell >> "$OUT/plasmashell.log" 2>&1 &
  for i in $(seq 1 60); do js 'print("up")' | grep -q '"up"' && break; sleep 1; done
  sleep 12
  shot plasma-2a-after-restart
  one_icon plasma-1-before plasma-2a-after-restart plasma
else
  one_icon plasma-1-before plasma-2-icon-added plasma
fi
# KDE's own way to open it (what a double-click on Plasma's desktop does): it asks first for a launcher it doesn't trust.
launched timeout 30 kioclient exec "$HOME/Desktop/Witcher Tales.desktop"
check_launch plasma
sleep 2; shot plasma-2b-after-open

# ---------------------------------------------------------------- 3. Hearth's panel on Plasma (Mantel's desktop over Plasma's)
mkdir -p "$HOME/.config/hearth" && cp /etc/skel/.config/hearth/plasma-panel-new "$HOME/.config/hearth/" 2>/dev/null
timeout 120 hearth-plasma-panel login > "$OUT/login.json" 2>&1
grep -q '"first": "applied"' "$OUT/login.json" || echo "::warning title=Hearth's panel::$(tr '\n' ' ' < "$OUT/login.json" | cut -c1-400)"
sleep 12
shot plasma-3-hearth-panel-icon-added
P3=$(places)
python3 - "$BEFORE" "$P3" <<'PY' && ok "Hearth's panel on Plasma: your icons kept their places" || fail plasma-hearth "icons moved with Hearth's panel: $BEFORE → $P3"
import json, sys
b, a = json.loads(sys.argv[1]), json.loads(sys.argv[2])
assert all(a.get(k) == v for k, v in b.items())
assert a.get('Witcher Tales.desktop') == [0, 1], a.get('Witcher Tales.desktop')
PY
launched python3 /usr/share/nimbus/desktop/shell/hearth-desk launch "$HOME/Desktop/Witcher Tales.desktop"
check_launch plasma-hearth
apply uninstalled; sleep 8
shot plasma-4-hearth-panel-after-uninstall
hearth-plasma-panel undo > "$OUT/undo.json" 2>&1; sleep 8
shot plasma-5-kde-after-uninstall
cp "$HOME/.config/nimbus/desk.json" "$OUT/desk-after-plasma.json"
out=""; for f in "$OUT"/*.png; do out="$out $(basename "$f" .png): $(python3 -c 'import sys; from PIL import Image, ImageStat; i=Image.open(sys.argv[1]); print("%dx%d spread %.1f" % (i.size[0], i.size[1], sum(ImageStat.Stat(i.convert("L")).stddev)))' "$f") |"; done
echo "::notice title=Pictures::$out"
echo "::notice title=Checks passed::$(grep -c '^ok:' "$OUT/step.log")"
exit 0
