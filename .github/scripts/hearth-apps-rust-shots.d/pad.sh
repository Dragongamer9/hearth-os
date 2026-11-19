# hearth-pad-host (hosts/hearth-ui/apps/pad): the controller's overlays in Plasma (the Hearth menu, the on-screen
# keyboard, the controls card, the hints, a message), pictured over Hearth Settings as they'd be over any app.
# Layer-shell overlays aren't in sway's window list, so each is pictured after a fixed wait; no controller helper runs
# for them (--show), except pad-8, where a stand-in hearth-pad opens the menu through the real link.
padshot() {
  name=$1; shift
  "$BIN/hearth-pad-host" "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  sleep 2.5
  grim "$ROOT/shots/$name.png" 2>/tmp/grim.log || echo "::warning title=$name::grim: $(cat /tmp/grim.log)"
  kill -0 $pid 2>/dev/null || echo "::error title=$name::it ended by itself: $(tail -c 1500 /tmp/$name.log | tr '\n' '|')"
  [ -s "/tmp/$name.log" ] && echo "::notice title=$name log::$(tail -c 900 /tmp/$name.log | tr '\n' '|')"
  echo "SHOT $name firstframe=overlay" >> "$ROOT/results.txt"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.3
}

"$BIN/hearth-settings-app" >/tmp/pad-backdrop.log 2>&1 &
backdrop=$!
for i in $(seq 1 300); do mapped hearth-settings && break; sleep 0.02; done
sleep 1

padshot pad-1-menu --show menu
padshot pad-2-keyboard --show keyboard
padshot pad-3-accents --show accents
padshot pad-4-wheel --show wheel
padshot pad-5-controls --show controls
padshot pad-6-hints --show hints
padshot pad-7-toast --show toast

# The real link: a stand-in for hearth-pad --plasma says it's ready and that Guide opened the menu on Close window.
cat > /tmp/fake-hearth-pad.py <<'PY'
import json, sys, time
print(json.dumps({'t': 'ready'}), flush=True)
time.sleep(0.5)
print(json.dumps({'t': 'radial', 'open': True, 'sel': 3}), flush=True)
for line in sys.stdin:
    with open('/tmp/fake-hearth-pad.in', 'a') as f:
        f.write(line)
PY
rm -f /tmp/fake-hearth-pad.in
HEARTH_PAD_HELPER=/tmp/fake-hearth-pad.py padshot pad-8-from-helper
grep -q '"cmd":"mouse"' /tmp/fake-hearth-pad.in 2>/dev/null \
  && echo "::notice title=pad link::the host answered hearth-pad's ready: $(head -c 300 /tmp/fake-hearth-pad.in | tr '\n' '|')" \
  || echo "::error title=pad link::the host sent hearth-pad nothing after ready"

# A Global Theme picked in KDE (Breeze, light): the keyboard's Steam-style look follows it.
mkdir -p "$HOME/.config/hearth"
cat > "$HOME/.config/hearth/theme.json" <<'JSON'
{"look": "kde", "mode": "light", "source": "plasma", "scheme": "BreezeLight", "radius": 6, "corner": 6,
 "colors": {"bg": "#eff0f1", "bg2": "#e3e5e7", "panel": "#ffffff", "panel2": "#fcfcfc", "panel3": "#e3e5e7", "line": "#c9cbcd",
            "text": "#232629", "muted": "#707d8a", "danger": "#da4453", "ember": "#3daee9", "flame": "#f67400", "ink": "#ffffff"}}
JSON
HEARTH_APPS_THEME="$HOME/.config/hearth/theme.json" padshot pad-9-keyboard-breeze --show keyboard
rm -f "$HOME/.config/hearth/theme.json"
kill $backdrop 2>/dev/null; wait $backdrop 2>/dev/null

# At rest (nothing open, no helper): nothing may run. Same limits as the other apps (0.3 % of a core, 60 MB).
"$BIN/hearth-pad-host" --no-helper >/tmp/pad-rest.log 2>&1 &
pid=$!
sleep 3
if kill -0 $pid 2>/dev/null; then
  a=$(ticks $pid); sleep 20; b=$(ticks $pid)
  cpu=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $(getconf CLK_TCK) / 20))")
  rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status)
  echo "::notice title=pad-rest::at rest ${cpu}% of a core · memory ${rss} MB resident · $(ls /proc/$pid/task | wc -l) threads"
  python3 -c "import sys; sys.exit(0 if float('$cpu') <= 0.3 else 1)" || echo "::error title=pad-rest::${cpu}% of a core at rest (limit 0.3%)"
  python3 -c "import sys; sys.exit(0 if float('$rss') <= 60 else 1)" || echo "::error title=pad-rest::${rss} MB resident at rest (limit about 60)"
else
  echo "::error title=pad-rest::it ended by itself: $(tail -c 1500 /tmp/pad-rest.log | tr '\n' '|')"
fi
kill $pid 2>/dev/null; wait $pid 2>/dev/null

# Without layer-shell (here: no Wayland at all) it says so with exit 4, so the launcher falls back to the plain controller.
WAYLAND_DISPLAY=wayland-none "$BIN/hearth-pad-host" --no-helper >/tmp/pad-nols.log 2>&1
rc=$?
[ "$rc" = 4 ] && echo "::notice title=pad fallback::no layer-shell: exit 4 as the launcher expects" \
  || echo "::error title=pad fallback::no layer-shell gave exit $rc, not 4: $(tail -c 500 /tmp/pad-nols.log | tr '\n' '|')"
