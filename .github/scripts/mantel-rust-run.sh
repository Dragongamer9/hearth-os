#!/bin/bash
# Mantel in Rust on labwc on a virtual screen (software drawing), for .github/workflows/mantel-rust.yml: the session
# started as labwc's autostart would, pictures of the desktop (Hearth and Steam-style looks), the app menu open, then
# a minute at rest: its CPU and memory as notices (errors when it isn't quiet). Pictures go out as SHOT notices.
#   mantel-rust-run.sh OUT
set -u
OUT=${1:?out}; mkdir -p "$OUT"
export XDG_RUNTIME_DIR=/tmp/xdg; mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1
export HOME=/tmp/home; mkdir -p "$HOME/.local/share/nimbus" "$HOME/.config/nimbus"
export ICED_BACKEND=tiny-skia
printf '{"pins":["foot","org.hearth.files"]}\n' > "$HOME/.config/nimbus/desk.json"
printf '{"look":"hearth","mode":"dark"}\n' > "$HOME/.local/share/nimbus/desktop.json"
mkdir -p /tmp/rc && cat > /tmp/rc/rc.xml <<'XML'
<?xml version="1.0"?>
<labwc_config><desktops number="4"><names><name>1</name><name>2</name><name>3</name><name>4</name></names></desktops></labwc_config>
XML
printf 'hearth-mantel session >/tmp/out-session.log 2>&1 &\n' > /tmp/rc/autostart
eval "$(dbus-launch --sh-syntax 2>/dev/null)" || true
labwc -C /tmp/rc >"$OUT/labwc.log" 2>&1 &
LW=$!
for i in $(seq 1 40); do ls "$XDG_RUNTIME_DIR"/wayland-* >/dev/null 2>&1 && break; sleep 0.5; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-? | head -1)")
sleep 8
note() { echo "::notice title=mantel-rust::$*"; }
err() { echo "::error title=mantel-rust::$*"; FAIL=1; }
FAIL=0
shot() { grim "$OUT/$1.png" 2>/dev/null || { err "no picture $1"; return; }
  python3 - "$OUT/$1.png" "$OUT/$1.jpg" <<'PY'
import sys
from PIL import Image
Image.open(sys.argv[1]).convert('RGB').resize((960, 540)).save(sys.argv[2], quality=82)
PY
  base64 -w0 "$OUT/$1.jpg" | fold -w 3800 | nl -ba -w2 -s' ' | head -12 | while read -r i part; do echo "::notice title=SHOT $1 $i::$part"; done; }
pgrep -af 'hearth-mantel' | sed 's/^/  /' > "$OUT/procs.txt"
for p in bar dock wallpaper; do pgrep -f "hearth-mantel $p" >/dev/null || err "the $p isn't running: $(tail -c 600 /tmp/out-session.log | tr '\n' ' ')"; done
foot >/dev/null 2>&1 & sleep 3
shot rs-01-desktop-hearth
hearth-mantel menu >/dev/null 2>&1 & sleep 4
shot rs-02-menu
pkill -f 'hearth-mantel menu'; sleep 1
printf '{"look":"steam","mode":"dark"}\n' > "$HOME/.local/share/nimbus/desktop.json"; sleep 3
shot rs-03-desktop-steam
printf '{"look":"hearth","mode":"light"}\n' > "$HOME/.local/share/nimbus/desktop.json"; sleep 3
shot rs-04-desktop-hearth-light
printf '{"look":"hearth","mode":"dark"}\n' > "$HOME/.local/share/nimbus/desktop.json"; sleep 2
# ---- a minute at rest: CPU (ticks) and memory of every hearth-mantel process
pids=$(pgrep -f 'hearth-mantel (bar|dock|wallpaper|session)' | tr '\n' ' ')
t0=$(for p in $pids; do awk '{print $14+$15}' /proc/$p/stat 2>/dev/null; done | paste -sd+ | bc 2>/dev/null || echo 0)
sleep 60
t1=$(for p in $pids; do awk '{print $14+$15}' /proc/$p/stat 2>/dev/null; done | paste -sd+ | bc 2>/dev/null || echo 0)
rss=$(for p in $pids; do awk '/VmRSS/{print $2}' /proc/$p/status 2>/dev/null; done | paste -sd+ | bc 2>/dev/null || echo 0)
cpu=$(python3 -c "print(round(($t1-$t0)/100/60*100, 2))")
note "at rest for 60 s: ${cpu}% of one CPU, ${rss} kB in memory (bar, dock, wallpaper and the session together)"
python3 -c "import sys; sys.exit(0 if $cpu < 3 else 1)" || err "not quiet at rest: ${cpu}% CPU"
cp /tmp/out-session.log "$OUT/session.log" 2>/dev/null
grep -iE "panicked|error" /tmp/out-session.log | head -5 | while read -r l; do echo "::warning title=session log::$l"; done
kill $LW 2>/dev/null
exit $FAIL
