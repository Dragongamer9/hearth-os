#!/bin/bash
# Screenshots of Hearth's looks in Plasma (Anatase edition), in the image itself: KDE Plasma on kwin_wayland --virtual
# (software drawing), as a new account that hasn't chosen a look, through hearth-plasma-looks sync (what the session
# runs). Hearth in full (desktop, app menu, Dolphin, System Settings › Global Theme), then one per look picked the way
# Hearth's Settings picks it (desktop.json "look").
#   anatase-look-shots.sh IMAGE OUTDIR
# Warn-only: it never fails the build; every shot it couldn't take is a ::warning.
set -u
IMG=${1:?image}; OUT=${2:?out dir}
mkdir -p "$OUT"
NAME=looks-$$
sudo podman run -d --name "$NAME" --privileged --entrypoint sleep "$IMG" infinity >/dev/null || { echo "::warning title=looks::couldn't start the image"; exit 0; }
trap 'sudo podman rm -f "$NAME" >/dev/null 2>&1' EXIT
cat > /tmp/looks-in.sh <<'IN'
set -u
export XDG_RUNTIME_DIR=/tmp/xdg-ci HOME=/home/ci LC_ALL=C.UTF-8 QT_QUICK_BACKEND=software LIBGL_ALWAYS_SOFTWARE=1
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
export KWIN_SCREENSHOT_NO_PERMISSION_CHECKS=1
mkdir -p "$XDG_RUNTIME_DIR" /tmp/shots; chmod 700 "$XDG_RUNTIME_DIR"
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu | head -1)" applications.menu)
kwin_wayland --virtual --width 1280 --height 800 --socket hearth-looks >/tmp/shots/kwin.log 2>&1 &
for i in $(seq 120); do [ -S "$XDG_RUNTIME_DIR/hearth-looks" ] && break; sleep 0.5; done
export WAYLAND_DISPLAY=hearth-looks QT_QPA_PLATFORM=wayland
kbuildsycoca6 >/dev/null 2>&1
plasmashell --no-respawn >/tmp/shots/plasmashell.log 2>&1 &
up() { dbus-send --session --print-reply --dest=org.freedesktop.DBus / org.freedesktop.DBus.NameHasOwner string:org.kde.plasmashell 2>/dev/null | grep -q 'boolean true'; }
for i in $(seq 180); do up && break; sleep 1; done
up || { echo "WARN plasmashell didn't start"; tail -20 /tmp/shots/plasmashell.log; exit 0; }
sleep 15
shot() {   # shot NAME: the whole screen
  rm -f "/tmp/shots/$1.png"
  timeout 60 spectacle --background --nonotify --fullscreen --output "/tmp/shots/$1.png" >>/tmp/shots/spectacle.log 2>&1
  [ -s "/tmp/shots/$1.png" ] && echo "SHOT $1" || echo "WARN no screenshot for $1"
}
qd() { for q in qdbus6 qdbus-qt6 qdbus; do command -v $q >/dev/null && { $q "$@"; return; }; done; }
pick() {   # pick LOOK: as Hearth's Settings does (desktop.json), then what the session runs
  mkdir -p "$HOME/.local/share/nimbus"
  python3 - "$1" <<'PY'
import json, os, sys
p = os.path.expanduser('~/.local/share/nimbus/desktop.json')
try:
    d = json.load(open(p))
except Exception:
    d = {}
d['look'] = sys.argv[1]
json.dump(d, open(p, 'w'), indent=2)
PY
  timeout 120 /usr/libexec/nimbus/hearth-plasma-looks sync >>/tmp/shots/sync.log 2>&1
  grep -q "LookAndFeelPackage=org.hearthos.$1" "$HOME/.config/kdeglobals" 2>/dev/null && echo "OK $1 applied" || echo "WARN $1: kdeglobals doesn't name org.hearthos.$1"
  sleep 8
}
# Hearth in full: the first start (nothing chosen) applies Hearth's look by itself.
timeout 120 /usr/libexec/nimbus/hearth-plasma-looks sync >>/tmp/shots/sync.log 2>&1
grep -q 'LookAndFeelPackage=org.hearthos.hearth' "$HOME/.config/kdeglobals" 2>/dev/null && echo "OK first start: Hearth's look" || echo "WARN first start: Hearth's look wasn't applied"
sleep 8
shot hearth-desktop
qd org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu >/dev/null 2>&1; sleep 4
shot hearth-app-menu
qd org.kde.plasmashell /PlasmaShell org.kde.PlasmaShell.activateLauncherMenu >/dev/null 2>&1; sleep 2
dolphin --new-window /usr/share/wallpapers >/tmp/shots/dolphin.log 2>&1 & DP=$!; sleep 10
shot hearth-dolphin
kill $DP 2>/dev/null; sleep 2
systemsettings kcm_lookandfeel >/tmp/shots/systemsettings.log 2>&1 & SP=$!; sleep 15
shot hearth-system-settings
kill $SP 2>/dev/null; sleep 2
# One per look, with Dolphin open so the window colours show.
dolphin --new-window /usr/share/wallpapers >>/tmp/shots/dolphin.log 2>&1 & DP=$!; sleep 8
for l in $(python3 -c 'import json; d=json.load(open("/usr/share/nimbus/desktop/theme/palettes.json")); print(" ".join(k for k,v in sorted(d.items(), key=lambda kv: (kv[1].get("order", 99) if isinstance(kv[1], dict) else 99, kv[0])) if not k.startswith("_") and isinstance(v,dict) and isinstance(v.get("dark"),dict)))'); do
  pick "$l"
  shot "look-$l"
done
kill $DP 2>/dev/null
cp "$HOME/.config/kdeglobals" /tmp/shots/kdeglobals.txt 2>/dev/null
cp "$HOME/.local/state/hearth/plasma-look.json" /tmp/shots/plasma-look.json 2>/dev/null
IN
sudo podman exec "$NAME" bash -c 'id ci >/dev/null 2>&1 || useradd -m -u 1000 ci 2>/dev/null || useradd -m ci'
sudo podman cp /tmp/looks-in.sh "$NAME:/tmp/looks-in.sh"
sudo podman exec "$NAME" bash -c 'chmod 644 /tmp/looks-in.sh; runuser -u ci -- dbus-run-session -- bash /tmp/looks-in.sh' 2>&1 | tee /tmp/looks.log | grep -E '^(SHOT|WARN|OK) ' || true
sudo podman exec "$NAME" bash -c 'chmod -R a+r /tmp/shots 2>/dev/null; ls /tmp/shots' >/dev/null 2>&1
sudo podman cp "$NAME:/tmp/shots/." "$OUT/" 2>/dev/null || true
sudo chown -R "$(id -u):$(id -g)" "$OUT" 2>/dev/null || true
grep '^WARN ' /tmp/looks.log | while read -r _ w; do echo "::warning title=looks::$w"; done
n=$(ls "$OUT"/*.png 2>/dev/null | wc -l)
echo "::notice title=looks::$n screenshots of Hearth's looks in Plasma: $(cd "$OUT" && ls *.png 2>/dev/null | tr '\n' ' ')"
for f in sync.log plasmashell.log; do
  [ -s "$OUT/$f" ] && echo "::notice title=looks $f::$(grep -iE 'hearth-plasma-looks|error|warn' "$OUT/$f" | head -8 | tr '\n' ' ' | cut -c1-900)"
done
exit 0
