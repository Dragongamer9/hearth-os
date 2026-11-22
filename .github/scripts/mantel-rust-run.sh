#!/bin/bash
# Mantel in Rust next to the QML Mantel, on labwc on a virtual screen (1920x1080), for .github/workflows/mantel-rust.yml.
# For each look (Steam-style dark, Hearth dark, Hearth light) both Mantels are started with the same settings, the
# same apps and the same pins, and pictured: the desktop with a window open, then the app menu. The pictures are put
# side by side (QML on top, Rust below) in OUT/pair-*.jpg with how far apart they are, and each Mantel is measured
# for a minute at rest (CPU and memory: errors when the Rust one isn't quiet).
#   mantel-rust-run.sh OUT
set -u
OUT=${1:?out}; mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
SHARE=/usr/share/nimbus; SH=$SHARE/desktop/shell
FAIL=0
note() { echo "::notice title=mantel-rust::$*"; echo "$*" >>"$OUT/notes.txt"; }
err() { echo "::error title=mantel-rust::$*"; echo "ERROR $*" >>"$OUT/notes.txt"; FAIL=1; }
: >"$OUT/notes.txt"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 LC_ALL=C.UTF-8 NIMBUS_SHARE=$SHARE
export QT_QUICK_BACKEND=software
# The Rust one draws with wgpu, on Mesa's software Vulkan here (lavapipe), as a PC draws it on its graphics card.
export ICED_BACKEND=${ICED_BACKEND:-wgpu}

# a minute at rest: CPU (% of one) and memory (MB) of the processes matching $1
rest() {
  python3 - "$1" <<'PY'
import os, re, sys, time
pat = re.compile(sys.argv[1])
def procs():
    out = []
    for p in os.listdir('/proc'):
        if not p.isdigit(): continue
        try: cmd = open(f'/proc/{p}/cmdline', 'rb').read().replace(b'\0', b' ').decode(errors='replace')
        except Exception: continue
        if pat.search(cmd) and 'python3' not in cmd: out.append(p)
    return out
def ticks(p):
    try: f = open(f'/proc/{p}/stat').read().rsplit(')', 1)[1].split(); return int(f[11]) + int(f[12])
    except Exception: return 0
def rss(p):
    try: return int([l for l in open(f'/proc/{p}/status') if l.startswith('VmRSS')][0].split()[1])
    except Exception: return 0
ps = procs(); t0 = sum(ticks(p) for p in ps); time.sleep(60); t1 = sum(ticks(p) for p in ps)
print(round((t1 - t0) / 100 / 60 * 100, 2), round(sum(rss(p) for p in ps) / 1024), len(ps))
PY
}

shot() { grim "$OUT/$1.png" 2>/dev/null || echo "no picture $1" >>"$OUT/notes.txt"; }

# one Mantel (qml | rust) in one look: pictures, then (Steam-style only) a minute at rest
run_one() {
  local which=$1 look=$2 mode=$3
  export XDG_RUNTIME_DIR=/tmp/xdg-$which-$look-$mode HOME=/tmp/home-$which-$look-$mode
  rm -rf "$XDG_RUNTIME_DIR" "$HOME"; mkdir -p "$XDG_RUNTIME_DIR" "$HOME/.local/share/nimbus" "$HOME/.config/nimbus" "$HOME/.local/share/applications"; chmod 700 "$XDG_RUNTIME_DIR"
  : >"$HOME/.local/share/nimbus/welcome-done"
  printf '{"look":"%s","mode":"%s","screenOffMinutes":0}\n' "$look" "$mode" >"$HOME/.local/share/nimbus/desktop.json"
  printf '{"pins":["hearth-test-files","hearth-test-web","foot"]}\n' >"$HOME/.config/nimbus/desk.json"
  printf '[Desktop Entry]\nType=Application\nName=Hearth Files\nExec=foot\nIcon=system-file-manager\n' >"$HOME/.local/share/applications/hearth-test-files.desktop"
  printf '[Desktop Entry]\nType=Application\nName=Web Browser\nExec=foot\nIcon=applications-internet\n' >"$HOME/.local/share/applications/hearth-test-web.desktop"
  printf '[Desktop Entry]\nType=Application\nName=Hearth Notes\nExec=foot\nIcon=accessories-text-editor\n' >"$HOME/.local/share/applications/hearth-test-notes.desktop"
  mkdir -p /tmp/rc-$which && cat >/tmp/rc-$which/rc.xml <<'XML'
<?xml version="1.0"?>
<labwc_config><desktops number="4"><names><name>1</name><name>2</name><name>3</name><name>4</name></names></desktops></labwc_config>
XML
  if [ "$which" = qml ]; then
    printf 'quickshell -p %s >%s/shell.log 2>&1 &\n' "$SH" "$XDG_RUNTIME_DIR" >/tmp/rc-$which/autostart
  else
    printf 'hearth-mantel session >%s/shell.log 2>&1 &\n' "$XDG_RUNTIME_DIR" >/tmp/rc-$which/autostart
  fi
  dbus-daemon --session --fork --address="unix:path=$XDG_RUNTIME_DIR/bus" && export DBUS_SESSION_BUS_ADDRESS="unix:path=$XDG_RUNTIME_DIR/bus"
  labwc -C /tmp/rc-$which >"$OUT/labwc-$which.log" 2>&1 &
  local lw=$!
  for i in $(seq 60); do ls "$XDG_RUNTIME_DIR"/wayland-? >/dev/null 2>&1 && break; sleep 0.5; done
  export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? 2>/dev/null | head -1)")
  wlr-randr --output HEADLESS-1 --custom-mode 1920x1080 >/dev/null 2>&1 || true
  sleep $([ "$which" = qml ] && echo 25 || echo 10)
  local tag="$look-$mode"
  foot -a foot -T "Terminal" >/dev/null 2>&1 & sleep 3
  shot "$which-$tag-1-desktop"
  if [ "$which" = qml ]; then timeout 20 quickshell ipc -p "$SH" call launcher toggle >/dev/null 2>&1; else hearth-mantel menu; fi
  sleep 4
  shot "$which-$tag-2-menu"
  if [ "$which" = qml ]; then timeout 20 quickshell ipc -p "$SH" call launcher toggle >/dev/null 2>&1; else hearth-mantel menu; fi
  sleep 2
  if [ "$look" = steam ]; then
    pkill -x foot; sleep 2
    local pat; pat=$([ "$which" = qml ] && echo '^quickshell' || echo 'hearth-mantel session')
    read -r cpu mb n < <(rest "$pat")
    note "$which Mantel at rest for 60 s: ${cpu}% of one CPU, ${mb} MB in memory ($n processes)"
    echo "$which $cpu $mb" >>"$OUT/rest.txt"
    if [ "$which" = rust ]; then python3 -c "import sys; sys.exit(0 if $cpu < 3 else 1)" || err "Rust Mantel isn't quiet at rest: ${cpu}% CPU"; fi
  fi
  if [ "$which" = rust ]; then
    pgrep -f 'hearth-mantel session' >/dev/null || err "Rust Mantel ($tag) isn't running: $(grep -v MESA "$XDG_RUNTIME_DIR/shell.log" | tail -c 800 | tr '\n' ' ')"
  fi
  grep -v -e MESA -e '^$' "$XDG_RUNTIME_DIR/shell.log" | tail -n 40 >"$OUT/$which-$tag-shell.log" 2>/dev/null
  kill "$lw" 2>/dev/null; sleep 1; pkill -f 'hearth-mantel session' 2>/dev/null; pkill -x quickshell 2>/dev/null; pkill -f dbus-daemon 2>/dev/null; pkill -x foot 2>/dev/null; sleep 1
}

for lm in "steam dark" "hearth dark" "hearth light"; do
  set -- $lm
  command -v quickshell >/dev/null && run_one qml "$1" "$2"
  run_one rust "$1" "$2"
done

# side by side: QML on top, Rust below; how far apart (0 = the same picture)
python3 - "$OUT" <<'PY' >>"$OUT/notes.txt"
import os, sys
from PIL import Image, ImageChops, ImageDraw, ImageStat
o = sys.argv[1]
for f in sorted(os.listdir(o)):
    if not (f.startswith('rust-') and f.endswith('.png')): continue
    name = f[5:-4]; q = os.path.join(o, 'qml-' + name + '.png'); r = os.path.join(o, f)
    R = Image.open(r).convert('RGB')
    if os.path.exists(q):
        Q = Image.open(q).convert('RGB').resize(R.size)
        diff = sum(ImageStat.Stat(ImageChops.difference(Q, R)).mean) / 3 / 255 * 100
        W, H = 1280, 720
        out = Image.new('RGB', (W, H * 2 + 40), (20, 20, 20))
        out.paste(Q.resize((W, H)), (0, 0)); out.paste(R.resize((W, H)), (0, H + 40))
        d = ImageDraw.Draw(out); d.text((10, H + 10), 'above: QML Mantel   below: Rust Mantel   %s   difference %.1f%%' % (name, diff), fill=(255, 255, 255))
        out.save(os.path.join(o, 'pair-' + name + '.jpg'), quality=85)
        print('%s: %.1f%% different' % (name, diff))
    R.resize((1280, 720)).save(os.path.join(o, 'rust-' + name + '.jpg'), quality=85)
PY
grep "different" "$OUT/notes.txt" | while read -r l; do echo "::notice title=QML vs Rust::$l"; done
rm -f "$OUT"/*.png
exit $FAIL
