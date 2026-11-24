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
    printf 'HEARTH_DESCRIBE_FAKE=/tmp/describe-answer.json quickshell -p %s >%s/shell.log 2>&1 &\n' "$SH" "$XDG_RUNTIME_DIR" >/tmp/rc-$id/autostart
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
  # Describe it (QML Mantel): a layout, a new feature and something that needs new work, with the AI's answers
  # given here (HEARTH_DESCRIBE_FAKE: the AI isn't in CI)
  if [ "$which" = qml ] && [ "$id" = plasma ]; then
    for k in layout feature needs; do
      cp "$PWD/.github/scripts/describe-$k.json" /tmp/describe-answer.json
      timeout 20 quickshell ipc -p "$SH" call describe ask "$( [ $k = needs ] && echo quickaccess || echo bar )" "$(python3 -c "import json;print(json.load(open('/tmp/describe-answer.json'))['_asked'])")" >/dev/null 2>&1
      sleep 6; shot "$which-describe-$k"; timeout 20 quickshell ipc -p "$SH" call describe close >/dev/null 2>&1; sleep 1
    done
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

# Build anything (docs/mockups/checklists/build-anything.md): the QML Mantel with pretend games, each ready-made
# whole-screen desktop put in place as Settings › Your desktop does (quickshell ipc call screen use ID), the first-start
# choice, and back to Hearth OS Mantel (the bar and dock must come back).
run_screens() {
  export XDG_RUNTIME_DIR=/tmp/xdg-screens HOME=/tmp/home-screens XDG_CACHE_HOME=/tmp/home-screens/.cache
  rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$HOME/.local/share/applications" "$XDG_CACHE_HOME/hearth"; chmod 700 "$XDG_RUNTIME_DIR"
  : >"$HOME/.local/share/nimbus/welcome-done"
  printf '{"look":"hearth","mode":"dark","screenOffMinutes":0,"defaults":2}\n' >"$HOME/.local/share/nimbus/desktop.json"
  python3 - "$XDG_CACHE_HOME/hearth" <<'PY'
import json, os, sys
from PIL import Image, ImageDraw
d = sys.argv[1]
games = [("ember-trails", "Ember Trails", (40, 70, 110), (140, 70, 40)), ("quad-mech", "Quad Mech", (20, 90, 40), (10, 40, 30)),
         ("gravesleep", "Gravesleep", (130, 45, 20), (40, 10, 10)), ("pixel-rally", "Pixel Rally", (60, 30, 140), (20, 90, 140)),
         ("trains-2010", "Trains 2010", (150, 100, 20), (60, 40, 10)), ("tributary", "Tributary", (150, 20, 60), (60, 10, 30))]
out = []
for gid, title, a, b in games:
    for kind, size in (("cover", (600, 900)), ("wide", (1920, 620))):
        im = Image.new("RGB", size); dr = ImageDraw.Draw(im)
        for y in range(size[1]):
            t = y / size[1]; dr.line([(0, y), (size[0], y)], fill=tuple(int(a[i] * (1 - t) + b[i] * t) for i in range(3)))
        im.save(os.path.join(d, gid + "-" + kind + ".png"))
    out.append({"id": "steam:" + gid, "title": title, "source": "steam", "cover": "file://" + os.path.join(d, gid + "-cover.png"), "wide": "file://" + os.path.join(d, gid + "-wide.png"), "icon": ""})
json.dump(out, open(os.path.join(d, "desktop-games.json"), "w"))
PY
  printf '{"pins":["foot"],"recent":["game:steam:pixel-rally"]}\n' >"$HOME/.config/nimbus/desk.json"
  python3 "$SH/hearth-browser" asked >/dev/null 2>&1 || true   # the 1.12 "Pick your browser" was answered long ago
  mkdir -p /tmp/rc-screens && printf '<?xml version="1.0"?>\n<labwc_config><desktops number="4"/></labwc_config>\n' >/tmp/rc-screens/rc.xml
  # a pretend AI's answer for "Or say it" (board 5) and what the pretend printer at 192.168.1.40 says
  mkdir -p /tmp/printer && printf '{"job":{"file":{"display":"Quad Mech base plate"}},"progress":{"completion":62,"printTimeLeft":6480},"temps":{"nozzle":205,"bed":60}}' >/tmp/printer/api_job
  python3 - "$OUT" <<'PY'
import json, sys
scr = json.load(open("hosts/linux/desktop/shell/screens/console.json"))
scr["blocks"].append({"id": "printer", "block": "own:printer-camera", "x": 18, "y": 9, "w": 6, "h": 4, "style": {"corners": 10}})
made = {"format": 1, "id": "printer-camera", "name": "Printer camera", "asked": "",
        "reach": {"kind": "address", "value": "192.168.1.40", "what": "your printer"}, "path": "/api/job",
        "shows": {"title": "{job.file.display} · {progress.completion}%", "lines": ["{progress.printTimeLeft} s left · nozzle {temps.nozzle}° · bed {temps.bed}°"],
                  "progress": "progress.completion", "picture": "/webcam/?action=snapshot", "chip": "Live · Anycubic Chiron"}, "every": 10}
json.dump({"kind": "screen", "screen": scr, "make": [made], "changes": ["Your printer's camera and progress, in the corner"]}, open("/tmp/describe-screen.json", "w"))
PY
  printf 'HEARTH_DESCRIBE_FAKE=/tmp/describe-screen.json HEARTH_BLOCK_FAKE=/tmp/printer quickshell -p %s >%s/shell.log 2>&1 &\n' "$SH" "$XDG_RUNTIME_DIR" >/tmp/rc-screens/autostart
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  labwc -C /tmp/rc-screens >"$OUT/labwc-screens.log" 2>&1 &
  local lw=$!
  for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  sleep 25
  # a new install: the first-start choice comes up by itself
  shot "qml-anything-7-first-start"
  timeout 20 quickshell ipc -p "$SH" call screen mantel >/dev/null 2>&1; sleep 3
  for id in console iconrow cockpit cabin notebook retro; do
    timeout 20 quickshell ipc -p "$SH" call screen use "$id" >/dev/null 2>&1; sleep 6
    shot "qml-anything-$id"
    st=$(timeout 20 quickshell ipc -p "$SH" call screen state 2>/dev/null)
    note "screen $id: $st"
    echo "$st" | grep -q '"drawn":0' && err "screen $id: on, but not drawn ($st)"
    echo "$st" | grep -q '"on":true' || err "screen $id: didn't come on ($st)"
    echo "$st" | grep -q '"failed":true' && err "screen $id: Screen.qml couldn't be drawn"
  done
  timeout 20 quickshell ipc -p "$SH" call screen use console >/dev/null 2>&1; sleep 4
  timeout 20 quickshell ipc -p "$SH" call screen guide >/dev/null 2>&1; sleep 2; shot "qml-anything-console-guide"
  # the block editor (board 4) on the console dashboard, the hero tile picked
  timeout 20 quickshell ipc -p "$SH" call screen editReady console >/dev/null 2>&1; sleep 4
  timeout 20 quickshell ipc -p "$SH" call screen editorPick 2 >/dev/null 2>&1; sleep 2
  shot "qml-anything-4-block-editor"
  # board 5: your AI makes a block Hearth hasn't got (a pretend AI and a pretend printer), it asks first, then it's on the desktop
  timeout 20 quickshell ipc -p "$SH" call screen editorSay "Add my 3D printer's camera and how far the print is, in the corner" >/dev/null 2>&1; sleep 8
  # answered Allow (as its ask's Allow does): the block goes live in the editor
  hearth-layout block-allow printer-camera always >/dev/null 2>&1; sleep 7
  shot "qml-anything-5-ai-made-block"
  timeout 20 quickshell ipc -p "$SH" call screen editorState 2>/dev/null | grep -q '"made":true' || err "board 5: the made block didn't come up ($(timeout 20 quickshell ipc -p "$SH" call screen editorState 2>/dev/null))"
  hearth-layout block-allow printer-camera always >/dev/null 2>&1
  timeout 20 quickshell ipc -p "$SH" call screen mantel >/dev/null 2>&1; sleep 6
  shot "qml-anything-back-to-mantel"
  timeout 20 quickshell ipc -p "$SH" call settings page your-desktop >/dev/null 2>&1; sleep 5; shot "qml-anything-settings"
  python3 -c "import json,os;d=json.load(open(os.path.expanduser('~/.local/share/nimbus/screen.json')));assert d.get('on') is False and d.get('chosen') is True, d" 2>>"$OUT/notes.txt" || err "screen: Back to Hearth OS Mantel wasn't kept"
  # The lock screen made of blocks (docs/mockups/checklists/lock-screen-blocks.md): its editor (board 3), then locked for
  # real with Hearth's own blocks (board 1) and in the 1995 look (board 2). A wrong password must keep it locked; the
  # right one (this run's own, set here) unlocks through PAM as always.
  ipc screen editLock; sleep 5; shot "qml-lock-3-the-editor"
  note "lock editor: $(timeout 20 quickshell ipc -p "$SH" call screen lockState 2>/dev/null)"
  timeout 20 quickshell ipc -p "$SH" call screen lockState 2>/dev/null | grep -q '"lockMode":true' || err "lock editor: didn't open in lock mode"
  ipc screen editorClose; sleep 2
  LOCKPW=hearthtest42
  echo "$(id -un):$LOCKPW" | chpasswd 2>>"$OUT/notes.txt" || note "lock: couldn't set this run's password"
  python3 "$SH/hearth-lockscreen" match >/dev/null 2>&1 || err "hearth-lockscreen match failed"
  lock_shot() {
    ipc lock lock; sleep 6; shot "$1"
    timeout 20 quickshell ipc -p "$SH" call lock state 2>/dev/null | grep -q '"locked":true' || err "$1: it didn't lock ($(timeout 20 quickshell ipc -p "$SH" call lock state 2>&1 | tail -1))"
    wtype "not-the-password" 2>/dev/null; wtype -k Return 2>/dev/null; sleep 5
    timeout 20 quickshell ipc -p "$SH" call lock state 2>/dev/null | grep -q '"locked":false' && err "$1: a wrong password unlocked it"
    wtype "$LOCKPW" 2>/dev/null; wtype -k Return 2>/dev/null; sleep 6
    if timeout 20 quickshell ipc -p "$SH" call lock state 2>/dev/null | grep -q '"locked":false'; then note "$1: the right password unlocked it"
    else note "$1: still locked after the right password (typing into the lock screen may not reach it here); restarting Mantel"; fi
  }
  lock_shot "qml-lock-1-made-of-blocks"
  ipc screen use retro; sleep 5
  lock_shot "qml-lock-2-in-the-1995-look"
  grep -v -e MESA -e '^$' "$XDG_RUNTIME_DIR/shell.log" >"$OUT/qml-screens-shell.log" 2>/dev/null
  note "screens log: $(grep -E 'Mantel:|shell.qml|Screen.qml' "$OUT/qml-screens-shell.log" | sort -u | head -12 | tr '\n' '|' | cut -c1-1800)"
  local bad; bad=$(grep -E 'Screen.qml|shell.qml|FirstStart|ScreenLayout|SetYourDesktop|ScreenEditor|OwnBlock|Lock.qml|LockBlocks|LockPassword' "$OUT/qml-screens-shell.log" | grep -v -e 'File does not exist' -e 'QML Image at' | grep -iE 'error|TypeError|ReferenceError|is not defined|Cannot|Unable|failed|not a type' | sort -u | head -8 | tr '\n' '|' | cut -c1-1500)
  [ -n "$bad" ] && err "screens: the log: $bad"
  kill "$lw" 2>/dev/null; sleep 1; pkill -x quickshell 2>/dev/null; pkill -f dbus-daemon 2>/dev/null; sleep 1
  unset XDG_CACHE_HOME
}
command -v quickshell >/dev/null && run_screens

# The same in Mantel in Rust: the first start, each ready-made desktop (put in screen.json as Settings › Your desktop
# does: this home is the run's own), the block editor, and back to Hearth OS Mantel.
run_screens_rust() {
  export XDG_RUNTIME_DIR=/tmp/xdg-screens-rs HOME=/tmp/home-screens-rs XDG_CACHE_HOME=/tmp/home-screens-rs/.cache
  rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$XDG_CACHE_HOME/hearth"; chmod 700 "$XDG_RUNTIME_DIR"
  cp -a /tmp/home-screens/.cache/hearth/. "$XDG_CACHE_HOME/hearth/" 2>/dev/null
  python3 -c "import json,os,sys;d=json.load(open(sys.argv[1]));[g.update({k:g[k].replace('/tmp/home-screens/','/tmp/home-screens-rs/')}) for g in d for k in ('cover','wide')];json.dump(d,open(sys.argv[1],'w'))" "$XDG_CACHE_HOME/hearth/desktop-games.json" 2>/dev/null
  : >"$HOME/.local/share/nimbus/welcome-done"
  printf '{"look":"hearth","mode":"dark","screenOffMinutes":0,"defaults":2}\n' >"$HOME/.local/share/nimbus/desktop.json"
  printf '{"pins":["foot"],"recent":["game:steam:pixel-rally"]}\n' >"$HOME/.config/nimbus/desk.json"
  mkdir -p /tmp/rc-screens-rs && printf '<?xml version="1.0"?>\n<labwc_config><desktops number="4"/></labwc_config>\n' >/tmp/rc-screens-rs/rc.xml
  printf 'hearth-mantel session >%s/shell.log 2>&1 &\n' "$XDG_RUNTIME_DIR" >/tmp/rc-screens-rs/autostart
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  labwc -C /tmp/rc-screens-rs >"$OUT/labwc-screens-rs.log" 2>&1 &
  local lw=$!
  for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  sleep 10
  note "rust: your games as Mantel reads them: $(python3 "$SH/hearth-games" cached 2>&1 | tail -c 300)"
  shot "rust-anything-7-first-start"
  for id in console iconrow cockpit cabin notebook retro; do
    python3 - "$id" <<'PY'
import json, os, sys
f = os.path.expanduser("~/.local/share/nimbus/screen.json")
d = json.load(open(f)) if os.path.exists(f) else {}
scr = json.load(open("hosts/linux/desktop/shell/screens/%s.json" % sys.argv[1]))
d.update({"on": True, "chosen": True, "from": sys.argv[1], "screen": scr, "said": scr.get("said", "")})
json.dump(d, open(f + ".tmp", "w")); os.replace(f + ".tmp", f)
PY
    sleep 4; shot "rust-anything-$id"
  done
  hearth-mantel guide >/dev/null 2>&1; sleep 2; shot "rust-anything-console-guide"
  hearth-mantel guide >/dev/null 2>&1
  hearth-mantel edit >/dev/null 2>&1; sleep 3; shot "rust-anything-4-block-editor"
  pgrep -f 'hearth-mantel session' >/dev/null || err "Mantel in Rust with a whole-screen desktop isn't running: $(grep -v MESA "$XDG_RUNTIME_DIR/shell.log" | tail -c 700 | tr '\n' ' ')"
  grep -v -e MESA -e '^$' "$XDG_RUNTIME_DIR/shell.log" >"$OUT/rust-screens-shell.log" 2>/dev/null
  grep -q panicked "$OUT/rust-screens-shell.log" && err "Mantel in Rust: $(grep -m1 -A2 panicked "$OUT/rust-screens-shell.log" | tr '\n' ' ' | cut -c1-600)"
  kill "$lw" 2>/dev/null; sleep 1; pkill -f 'hearth-mantel session' 2>/dev/null; pkill -f dbus-daemon 2>/dev/null; sleep 1
  unset XDG_CACHE_HOME
}
command -v hearth-mantel >/dev/null && run_screens_rust

# next to the approved mockups: mockup above, Mantel below
python3 - "$OUT" "$MOCK" <<'PY' >>"$OUT/notes.txt"
import os, sys
from PIL import Image, ImageDraw
o, m = sys.argv[1:3]
mock = {'plasma-1-desktop': 'layouts-1-plasma-style.png', 'gnome-1-desktop': 'layouts-2-gnome-style.png',
        'gnome-3-overview': 'layouts-3-gnome-style-overview.png', 'windows11-2-menu': 'layouts-4-windows-11-style.png',
        'macos-1-desktop': 'layouts-5-macos-style.png', 'describe-layout': 'layouts-7-describe-it.png',
        'describe-feature': 'layouts-8-describe-it-new-feature.png', 'describe-needs': 'layouts-9-describe-it-send-idea.png',
        'anything-7-first-start': '../build-anything/anything-7-first-start.png', 'anything-console-guide': '../build-anything/anything-2-console-dashboard.png',
        'anything-iconrow': '../build-anything/anything-3-icon-row-home.png', 'anything-4-block-editor': '../build-anything/anything-4-block-editor.png',
        'anything-5-ai-made-block': '../build-anything/anything-5-ai-made-block.png', 'anything-console': '../build-anything/anything-1-no-two-alike.png',
        'lock-1-made-of-blocks': '../lock-screen-blocks/lock-1-made-of-blocks.png', 'lock-2-in-the-1995-look': '../lock-screen-blocks/lock-2-in-the-1995-look.png',
        'lock-3-the-editor': '../lock-screen-blocks/lock-3-the-editor.png'}
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
