#!/bin/bash
# Desktop layouts on labwc on a virtual screen (1920x1080), for .github/workflows/hearth-layout.yml: each ready-made
# layout put in place with hearth-layout (as Settings does), then the QML Mantel and Mantel in Rust started in turn and
# pictured (the desktop with a window open; the app menu; GNOME-style's overview) under its approved mockup
# (docs/mockups/desktop-layouts): OUT/pair-*.jpg, the mockup, the QML Mantel, Mantel in Rust. Their logs are checked
# for errors from the layouts' parts.
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
  local which=$1 id=$2 look=${3:-hearth}
  export XDG_RUNTIME_DIR=/tmp/xdg-$which-$id HOME=/tmp/home-$which-$id
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
  if [ "$which" = qml ]; then
    printf 'quickshell -p %s >%s/shell.log 2>&1 &\n' "$SH" "$XDG_RUNTIME_DIR" >/tmp/rc-$id/autostart
  else
    printf 'hearth-mantel session >%s/shell.log 2>&1 &\n' "$XDG_RUNTIME_DIR" >/tmp/rc-$id/autostart
  fi
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  labwc -C /tmp/rc-$id >"$OUT/labwc-$id.log" 2>&1 &
  local lw=$!
  for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  sleep $([ "$which" = qml ] && echo 25 || echo 10)
  foot -a foot -T "Home" >/dev/null 2>&1 & sleep 3
  shot "$which-$id-1-desktop"
  if [ "$which" = qml ]; then ipc launcher toggle; else hearth-mantel menu; fi; sleep 4; shot "$which-$id-2-menu"
  if [ "$which" = qml ]; then ipc launcher toggle; else hearth-mantel menu; fi; sleep 2
  if [ "$id" = gnome ]; then
    if [ "$which" = qml ]; then ipc overview toggle; else hearth-mantel overview; fi; sleep 4; shot "$which-$id-3-overview"
    if [ "$which" = qml ]; then ipc overview toggle; else hearth-mantel overview; fi; sleep 1
  fi
  [ "$which" = rust ] && { pgrep -f 'hearth-mantel session' >/dev/null || err "$id: Mantel in Rust isn't running: $(grep -v MESA "$XDG_RUNTIME_DIR/shell.log" | tail -c 700 | tr '\n' ' ')"; }
  # undo puts back what was there (the look stays), then the layout again
  hearth-layout undo >/dev/null 2>&1 && [ "$(hearth-layout current)" = "" ] || err "$id: Undo didn't put the desktop back"
  grep -v -e MESA -e '^$' "$XDG_RUNTIME_DIR/shell.log" >"$OUT/$which-$id-shell.log" 2>/dev/null
  local bad; bad=$(grep -E 'LayoutBlock|Overview|Islands|ExtraPanels|widget-tools|panicked' "$OUT/$which-$id-shell.log" | grep -iE 'error|TypeError|ReferenceError|is not defined|Cannot|Unable|failed|not a type|panicked' | sort -u | head -6 | tr '\n' '|' | cut -c1-1500)
  [ -n "$bad" ] && err "$which $id: the log: $bad"
  kill "$lw" 2>/dev/null; sleep 1; pkill -x quickshell 2>/dev/null; pkill -f 'hearth-mantel session' 2>/dev/null; pkill -f dbus-daemon 2>/dev/null; pkill -x foot 2>/dev/null; sleep 1
}

for id in plasma gnome windows11 macos steam hearth; do
  command -v quickshell >/dev/null && run_one qml "$id"
  command -v hearth-mantel >/dev/null && run_one rust "$id"
done

# next to the approved mockups: mockup above, Mantel below
python3 - "$OUT" "$MOCK" <<'PY' >>"$OUT/notes.txt"
import os, sys
from PIL import Image, ImageDraw
o, m = sys.argv[1:3]
mock = {'plasma-1-desktop': 'layouts-1-plasma-style.png', 'gnome-1-desktop': 'layouts-2-gnome-style.png',
        'gnome-3-overview': 'layouts-3-gnome-style-overview.png', 'windows11-2-menu': 'layouts-4-windows-11-style.png',
        'macos-1-desktop': 'layouts-5-macos-style.png'}
W, H = 1280, 720
names = sorted({f[4:-4] if f.startswith('qml-') else f[5:-4] for f in os.listdir(o) if f.endswith('.png') and (f.startswith('qml-') or f.startswith('rust-'))})
for name in names:
    rows = []
    if name in mock and os.path.exists(os.path.join(m, mock[name])):
        rows.append(('the approved mockup', Image.open(os.path.join(m, mock[name])).convert('RGB')))
    for which, label in (('qml', 'the QML Mantel'), ('rust', 'Mantel in Rust')):
        f = os.path.join(o, which + '-' + name + '.png')
        if os.path.exists(f):
            R = Image.open(f).convert('RGB'); rows.append((label, R))
            R.resize((W, H)).save(os.path.join(o, which + '-' + name + '.jpg'), quality=85)
    if len(rows) > 1:
        out = Image.new('RGB', (W, (H + 30) * len(rows)), (20, 20, 20))
        d = ImageDraw.Draw(out)
        for i, (label, im) in enumerate(rows):
            d.text((10, i * (H + 30) + 8), label + '   ' + name, fill=(255, 255, 255))
            out.paste(im.resize((W, H)), (0, i * (H + 30) + 30))
        out.save(os.path.join(o, 'pair-' + name + '.jpg'), quality=82)
        print('pair ' + name)
PY
rm -f "$OUT"/*.png
exit $FAIL
