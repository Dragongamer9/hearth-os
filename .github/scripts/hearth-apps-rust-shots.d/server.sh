# Server mode's dashboard on the Fedora edition (apps/server, hearth-server-app; docs/SERVER-MODE.md), sourced by
# hearth-apps-rust-test.sh inside its headless screen (run() is its). The data is the same fixture Mantel's
# dashboard is pictured with in desktop-test.yml (hearth-server-mode reads HEARTH_SERVER_DASH_FIXTURE), so the two
# can be compared. Then the real Fedora path: cage on its own headless screen running desktop/server/cage/run.
SV=/tmp/server-fix
mkdir -p "$SV"
cp "$ROOT/hosts/linux/test/fixtures/server-dash.json" "$SV/dash.json"
python3 - "$SV/dash.json" "$SV/down.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
for t in d['tiles']:
    if t['id'] == 'media':
        t.update({'state': 'stopped', 'chip': 'Stopped', 'big': '0', 'sub': "Jellyfin stopped: X restarts it"})
d['status'] = {'level': 'warn', 'title': 'Needs a look', 'sub': d['status']['sub'], 'problems': ['Media']}
d['alerts'].insert(0, {'level': 'warn', 'text': 'Media stopped', 'sub': 'Jellyfin ended by itself', 'at': '3:58 AM'})
for p in d['pills']:
    if p['name'] == 'Media':
        p['state'] = 'stopped'
json.dump(d, open(sys.argv[2], 'w'))
PY
export HEARTH_SERVER_NO_PAD=1 HEARTH_SERVER_DASH_FIXTURE="$SV/dash.json" HEARTH_NIMBUS_MODE=/bin/true
SB="$BIN/hearth-server-app"
rm -rf "$XDG_RUNTIME_DIR/hearth-server"
run server-1-dashboard hearth-server 20 "$SB"
run server-2-ai-opened hearth-server 0 "$SB" --show detail:1 --sel 1
run server-3-guide-countdown hearth-server 0 "$SB" --show switch
run server-4-menu hearth-server 0 "$SB" --show menu
run server-5-back-from-game-mode hearth-server 0 "$SB" --show back
run server-6-sleep-after-idle-dim hearth-server 0 "$SB" --show dim
mkdir -p "$XDG_RUNTIME_DIR/hearth-server"
python3 -c 'import json,time,sys; json.dump({"action": {"kind": "mode", "to": "game"}, "to": "game", "what": "mode", "deadline": time.time() + 9, "total": 10, "from": "web", "by": "Work laptop"}, open(sys.argv[1], "w"))' "$XDG_RUNTIME_DIR/hearth-server/countdown.json"
run server-7-web-page-countdown hearth-server 0 "$SB"
echo '{}' > "$XDG_RUNTIME_DIR/hearth-server/countdown.json"
HEARTH_SERVER_DASH_FIXTURE="$SV/down.json" run server-8-a-service-down hearth-server 0 "$SB" --sel 4
HEARTH_SERVER_DASH_FIXTURE="$SV/down.json" run server-9-service-down-opened hearth-server 0 "$SB" --show detail:4 --sel 4
# The real thing: cage (its own headless screen) with the watcher, as nimbus-session runs it on the Fedora edition.
if command -v cage >/dev/null 2>&1; then
  cp "$(command -v cage)" /tmp/cage-nocap 2>/dev/null && chmod 755 /tmp/cage-nocap
  rm -f "$XDG_RUNTIME_DIR/hearth-server-dash-up" "$XDG_RUNTIME_DIR/hearth-server-dash-failed"
  before=$(ls "$XDG_RUNTIME_DIR" | grep -E '^wayland-[0-9]+$' | sort)
  ( unset WAYLAND_DISPLAY SWAYSOCK; WLR_BACKENDS=headless WLR_RENDERER=pixman HEARTH_SERVER_APP="$SB" HEARTH_SHELL_DIR="$HEARTH_SHELL_DIR" \
      exec /tmp/cage-nocap -s -- sh "$ROOT/hosts/linux/desktop/server/cage/run" ) >/tmp/cage.log 2>&1 &
  cpid=$!
  for i in $(seq 1 60); do [ -e "$XDG_RUNTIME_DIR/hearth-server-dash-up" ] && break; sleep 0.5; done
  sock=$(ls "$XDG_RUNTIME_DIR" | grep -E '^wayland-[0-9]+$' | sort | comm -13 <(echo "$before") - | head -1)
  if [ -e "$XDG_RUNTIME_DIR/hearth-server-dash-up" ] && [ -n "$sock" ]; then
    sleep 4
    WAYLAND_DISPLAY="$sock" grim "$ROOT/shots/server-10-in-cage.png" 2>/tmp/grim-cage.log \
      || echo "::warning title=server in cage::grim: $(cat /tmp/grim-cage.log)"
    echo "::notice title=server in cage::cage $(rpm -q cage 2>/dev/null) ran the dashboard (it said it's up) on $sock"
  else
    echo "::error title=server in cage::the dashboard never said it's up in cage: $(tail -c 800 /tmp/cage.log | tr '\n' '|') $(tail -c 600 "$HOME/.local/share/nimbus/server-dash.log" 2>/dev/null | tr '\n' '|')"
  fi
  kill "$cpid" 2>/dev/null; sleep 1; kill -9 "$cpid" 2>/dev/null; pkill -f hearth-server-app 2>/dev/null
  rm -f "$XDG_RUNTIME_DIR/hearth-server-dash-up"
else
  echo "::warning title=server in cage::cage isn't installed in Fedora 45 here"
fi
unset HEARTH_SERVER_NO_PAD HEARTH_SERVER_DASH_FIXTURE HEARTH_NIMBUS_MODE
