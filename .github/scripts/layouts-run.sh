#!/bin/bash
# Desktop layouts on labwc on a virtual screen (1920x1080), for .github/workflows/hearth-layout.yml: each ready-made
# layout put in place with hearth-layout (as Settings does), then the QML Mantel started and pictured (the desktop with
# a window open; the app menu; GNOME-style's overview) next to its approved mockup (docs/mockups/desktop-layouts):
# OUT/pair-*.jpg, mockup above, Mantel below. Mantel's log is checked for errors from the layouts' parts.
#   layouts-run.sh OUT
set -u
OUT=${1:?out}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SHARE=/usr/share/nimbus; SH=$SHARE/desktop/shell
MOCK=$PWD/docs/mockups/desktop-layouts
FAIL=0
note() { echo "::notice title=layouts::$*"; echo "$*" >>"$OUT/notes.txt"; }
err() { echo "::error title=layouts::$*"; echo "ERROR $*" >>"$OUT/notes.txt"; FAIL=1; }
: >"$OUT/notes.txt"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 LC_ALL=C.UTF-8 NIMBUS_SHARE=$SHARE
export QT_QUICK_BACKEND=software
shot() { grim "$OUT/$1.png" 2>/dev/null || echo "no picture $1" >>"$OUT/notes.txt"; }
ipc() { timeout 20 quickshell ipc -p "$SH" call "$@" >/dev/null 2>&1; }

run_one() {
  local id=$1 look=${2:-hearth}
  export XDG_RUNTIME_DIR=/tmp/xdg-$id HOME=/tmp/home-$id
  rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$HOME/.local/share/applications"; chmod 700 "$XDG_RUNTIME_DIR"
  : >"$HOME/.local/share/nimbus/welcome-done"
  printf '{"look":"%s","mode":"dark","screenOffMinutes":0}\n' "$look" >"$HOME/.local/share/nimbus/desktop.json"
  printf '{"pins":["hearth-test-files","hearth-test-web","hearth-test-notes","foot"]}\n' >"$HOME/.config/nimbus/desk.json"
  printf '[Desktop Entry]\nType=Application\nName=Hearth Files\nExec=foot\nIcon=system-file-manager\n' >"$HOME/.local/share/applications/hearth-test-files.desktop"
  printf '[Desktop Entry]\nType=Application\nName=Web Browser\nExec=foot\nIcon=applications-internet\n' >"$HOME/.local/share/applications/hearth-test-web.desktop"
  printf '[Desktop Entry]\nType=Application\nName=Hearth Notes\nExec=foot\nIcon=accessories-text-editor\n' >"$HOME/.local/share/applications/hearth-test-notes.desktop"
  hearth-layout apply "$id" >>"$OUT/notes.txt" 2>&1 || err "$id: hearth-layout apply failed"
  local tb; tb=$(python3 -c 'import json,os;print(json.load(open(os.path.expanduser("~/.local/share/nimbus/desktop.json"))).get("windowButtons","icon:iconify,max,close"))')
  mkdir -p /tmp/rc-$id && cat >/tmp/rc-$id/rc.xml <<XML
<?xml version="1.0"?>
<labwc_config><theme><titlebar><layout>$tb</layout></titlebar></theme><desktops number="4"><names><name>1</name><name>2</name><name>3</name><name>4</name></names></desktops></labwc_config>
XML
  printf 'quickshell -p %s >%s/shell.log 2>&1 &\n' "$SH" "$XDG_RUNTIME_DIR" >/tmp/rc-$id/autostart
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  labwc -C /tmp/rc-$id >"$OUT/labwc-$id.log" 2>&1 &
  local lw=$!
  for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  sleep 25
  foot -a foot -T "Home" >/dev/null 2>&1 & sleep 3
  shot "$id-1-desktop"
  ipc launcher toggle; sleep 4; shot "$id-2-menu"; ipc launcher toggle; sleep 2
  if [ "$id" = gnome ]; then ipc overview toggle; sleep 4; shot "$id-3-overview"; ipc overview toggle; sleep 1; fi
  # undo puts back what was there (the look stays), then the layout again
  hearth-layout undo >/dev/null 2>&1 && [ "$(hearth-layout current)" = "" ] || err "$id: Undo didn't put the desktop back"
  grep -v -e MESA -e '^$' "$XDG_RUNTIME_DIR/shell.log" >"$OUT/$id-shell.log" 2>/dev/null
  local bad; bad=$(grep -E 'LayoutBlock|Overview|Islands|ExtraPanels|widget-tools' "$OUT/$id-shell.log" | grep -iE 'error|TypeError|ReferenceError|is not defined|Cannot|Unable|failed|not a type' | sort -u | head -6 | tr '\n' '|' | cut -c1-1500)
  [ -n "$bad" ] && err "$id: Mantel's log: $bad"
  kill "$lw" 2>/dev/null; sleep 1; pkill -x quickshell 2>/dev/null; pkill -f dbus-daemon 2>/dev/null; pkill -x foot 2>/dev/null; sleep 1
}

for id in plasma gnome windows11 macos steam hearth; do run_one "$id"; done

# next to the approved mockups: mockup above, Mantel below
python3 - "$OUT" "$MOCK" <<'PY' >>"$OUT/notes.txt"
import os, sys
from PIL import Image, ImageDraw
o, m = sys.argv[1:3]
mock = {'plasma-1-desktop': 'layouts-1-plasma-style.png', 'gnome-1-desktop': 'layouts-2-gnome-style.png',
        'gnome-3-overview': 'layouts-3-gnome-style-overview.png', 'windows11-2-menu': 'layouts-4-windows-11-style.png',
        'macos-1-desktop': 'layouts-5-macos-style.png'}
for f in sorted(os.listdir(o)):
    if not f.endswith('.png'): continue
    name = f[:-4]; R = Image.open(os.path.join(o, f)).convert('RGB')
    W, H = 1280, 720
    if name in mock and os.path.exists(os.path.join(m, mock[name])):
        Q = Image.open(os.path.join(m, mock[name])).convert('RGB')
        out = Image.new('RGB', (W, H * 2 + 40), (20, 20, 20))
        out.paste(Q.resize((W, H)), (0, 0)); out.paste(R.resize((W, H)), (0, H + 40))
        ImageDraw.Draw(out).text((10, H + 10), 'above: the approved mockup   below: Mantel   ' + name, fill=(255, 255, 255))
        out.save(os.path.join(o, 'pair-' + name + '.jpg'), quality=85)
        print('pair ' + name)
    R.resize((W, H)).save(os.path.join(o, name + '.jpg'), quality=85)
PY
rm -f "$OUT"/*.png
exit $FAIL
