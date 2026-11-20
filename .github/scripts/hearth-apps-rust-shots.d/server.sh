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
run server-1-dashboard hearth-server 0 "$SB"
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
# 1.16: the tiles' own pages (as Mantel's): the AI team, Ithil (a stand-in ~/HearthAI, as desktop-test.yml makes it),
# the game download cache, the ad blocker and quiet hours & rules, with the mockups' numbers.
python3 "$ROOT/hosts/linux/test/fixtures/ithil-home.py" "$HOME/HearthAI"
python3 - "$ROOT" "$SV/dash.json" "$SV/full.json" "$SV/quiet.json" 2>/tmp/svgen.err >/tmp/svgen.out <<'PY2'
import importlib.machinery, json, sys, time
root = sys.argv[1]
sys.path.insert(0, root + '/hosts/linux/desktop/shell')
import importlib.util
_l = importlib.machinery.SourceFileLoader('sm', root + '/hosts/linux/desktop/shell/hearth-server-mode')
sm = importlib.util.module_from_spec(importlib.util.spec_from_loader('sm', _l))
_l.exec_module(sm)
import hearth_ithil
d = json.load(open(sys.argv[2]))
now = time.time()
today = {'day': time.strftime('%Y-%m-%d', time.localtime(now)), 'total': 54, 'here': 48, 'sent': {'rog-ally|voice': 6},
         'senders': {'Work laptop': {'n': 15, 'last': 'Asked qwen3:30b-a3b', 'at': now - 10, 'kind': 'chat'},
                     'ROG Ally': {'n': 9, 'last': 'Spoke a reply', 'at': now - 20, 'kind': 'voice'},
                     'Claude': {'n': 10, 'last': 'Asked qwen3', 'at': now - 3600, 'kind': 'chat', 'via': 'Command Center'},
                     'Codex': {'n': 4, 'last': 'Asked qwen3', 'at': now - 7200, 'kind': 'chat', 'via': 'Command Center'}},
         'lastEvent': {'who': 'Claude', 'via': 'Command Center', 'kind': 'chat', 'model': 'qwen3:30b', 'at': now - 30}}
engines = [{'id': 'chat', 'name': 'AI chat', 'icon': 'chat', 'model': 'qwen3:30b-a3b', 'state': 'working', 'status': 'qwen3:30b-a3b · 2 asking'},
           {'id': 'pictures', 'name': 'Pictures', 'icon': 'image', 'engine': 'ComfyUI', 'state': 'working', 'status': 'making 1 of 3'},
           {'id': 'voices', 'name': 'Voices', 'icon': 'vol', 'engine': 'Voice Studio', 'state': 'ready', 'status': 'Voice Studio'},
           {'id': 'stt', 'name': 'Speech to text', 'icon': 'mic', 'engine': 'whisper', 'state': 'ready', 'status': 'whisper · ready'}]
team = sm.team_view(today, [{'name': 'Living room TV', 'what': ['Media']}], engines, {'total': 3, 'making': True}, d['pauses'], True, 'chris-hearth', now)
d['team'] = team
d['tiles'].insert(0, {'id': 'team', 'title': 'AI team', 'engine': 'One AI address · 4 engines', 'icon': 'sparkle', 'state': 'running', 'chip': team['chip'],
                      'new': True, 'mesh': True, 'actions': ['restart', 'pause']})
try:
    v = hearth_ithil.view() or {}
except Exception as e:  # noqa: BLE001
    import traceback; traceback.print_exc()
    v = {}
v.update({'running': True, 'state': 'awake', 'chip': 'Awake', 'version': '0.6', 'brainLoaded': True})
d['tiles'].insert(1, {'id': 'ithil', 'title': 'Ithil', 'engine': 'Her own mind · Hearth AI 0.6', 'icon': 'moon', 'state': 'running', 'chip': 'Awake',
                      'new': True, 'wide': True, 'ithil': v, 'actions': []})
d['ithil'] = v
d['alerts'].insert(0, {'level': 'team', 'text': team['event'], 'sub': '', 'at': ''})
G = 1e9
day = lambda i: time.strftime('%a', time.localtime(now - (6 - i) * 86400))
dns = {'cache': {'on': True, 'running': True, 'stores': {'steam': True, 'epic': True, 'blizzard': True, 'xbox': False}, 'sizeGB': 1000, 'dir': '/var/mnt/games',
                 'storeNames': {'steam': 'Steam', 'epic': 'Epic Games', 'blizzard': 'Battle.net', 'xbox': 'Xbox / Microsoft Store'},
                 'view': {'savedWeek': 412 * G, 'internetWeek': 186 * G, 'devicesWeek': 598 * G, 'used': 784 * G, 'drive': 'Games drive',
                          'bars': [{'day': day(i), 'saved': x * G} for i, x in enumerate([44, 73, 22, 100, 62, 33, 84])],
                          'now': [{'device': 'ROG Ally', 'what': "Baldur's Gate 3 update", 'from': 'from the cache', 'rate': 94e6},
                                  {'device': 'Living room PC', 'what': 'Halo Infinite', 'from': 'from the internet once', 'rate': 31e6}]}},
       'ads': {'on': True, 'running': True, 'pausedUntil': 0, 'allow': [], 'devices': {},
               'view': {'today': {'percent': 18, 'blocked': 4812, 'ms': 9}, 'pausedUntil': 0,
                        'top': [{'name': 'doubleclick.net', 'count': 1204}, {'name': 'app-measurement.com', 'count': 811}, {'name': 'ads.youtube.com', 'count': 402}, {'name': 'telemetry.example-tv.com', 'count': 377}],
                        'devices': [{'id': 'chris-hearth', 'name': 'chris-hearth (this PC)', 'on': True, 'state': 'Using it'},
                                    {'id': 'rog-ally', 'name': 'ROG Ally · at work, over Tailscale', 'on': True, 'state': 'Using it'},
                                    {'id': 'work-laptop', 'name': 'Work laptop', 'on': False, 'state': 'Paused 1 h'},
                                    {'id': 'lan', 'name': 'Everything else on your Wi-Fi', 'on': True, 'state': 'Through the router'}]}}}
rules = {'quiet': [{'id': '1', 'name': 'Weeknights', 'from': '23:00', 'to': '07:00', 'days': [0, 1, 2, 3, 4], 'daysText': 'Mon–Fri', 'on': True},
                   {'id': '2', 'name': 'Sunday lie-in', 'from': '00:00', 'to': '10:00', 'days': [6], 'daysText': 'Sun', 'on': False}],
         'while': {'fans': 40, 'powerW': 150, 'jobs': True, 'downloads': True},
         'rules': [{'id': '1', 'when': 'jellyfin', 'do': 'pictures', 'on': True, 'whenText': 'someone is watching Jellyfin', 'doText': 'AI pictures wait', 'paused': False},
                   {'id': '2', 'when': 'game', 'do': 'downloads', 'on': True, 'whenText': 'a game runs on this PC', 'doText': 'downloads at half speed', 'paused': False},
                   {'id': '3', 'when': 'drive:50', 'do': 'backups', 'on': False, 'whenText': 'the drive is over 50 °C', 'doText': 'backups wait', 'paused': True}],
         'now': {'at': int(now), 'quiet': 'Weeknights', 'quietUntil': int(now) + 5 * 3600, 'wait': ['pictures', 'voices'], 'fanSpeed': 34, 'why': ['Weeknights'], 'game': False}}
d['tiles'] += sm.dns_tiles(dns) + [sm.rules_tile(rules)]
json.dump(d, open(sys.argv[3], 'w'))
# Nothing working (for the light-at-rest measurement): every core ready, no mesh, no Ithil.
q = json.load(open(sys.argv[2]))
for c in q['cores']:
    c['state'] = 'ready'
json.dump(q, open(sys.argv[4], 'w'))
print('tiles: ' + ' '.join('%d=%s' % (i + 1, t['id']) for i, t in enumerate(d['tiles'])))
PY2
[ -s "$SV/full.json" ] && echo "::notice title=server fixture::$(cat /tmp/svgen.out)" \
  || echo "::error title=server fixture::the pages' test data wasn't made: $(tail -c 1500 /tmp/svgen.err | tr '\n' '|')"
HEARTH_SERVER_DASH_FIXTURE="$SV/full.json" timeout 30 python3 "$HEARTH_SHELL_DIR/hearth-server-mode" dash >/tmp/svdash.out 2>/tmp/svdash.err \
  || echo "::error title=server fixture::dash with it failed: $(tail -c 1500 /tmp/svdash.err | tr '\n' '|')"
export HEARTH_SERVER_DASH_FIXTURE="$SV/full.json"
nrules=$(python3 -c "import json,sys; print(len(json.load(open(sys.argv[1]))['tiles']))" "$SV/full.json")
run server-live-dashboard hearth-server 20 "$SB"
run server-11-team-and-ithil-tiles hearth-server 0 "$SB" --sel 1
run server-12-ai-team-opened hearth-server 0 "$SB" --show page:team --sel 1
run server-13-ithil-opened hearth-server 0 "$SB" --show page:ithil
run server-13b-ithil-last-24-hours hearth-server 0 "$SB" --show page:ithil --ithil-win 24
run server-14-cache-ads-rules-tiles hearth-server 0 "$SB" --sel "$nrules"
run server-15-cache-opened hearth-server 0 "$SB" --show page:cache
run server-16-adblock-opened hearth-server 0 "$SB" --show page:ads
run server-17-rules-opened hearth-server 0 "$SB" --show page:rules
run server-17b-rules-new-rule hearth-server 0 "$SB" --show new-rule
HEARTH_SERVER_DASH_FIXTURE="$SV/full.json" run server-quiet-dim hearth-server 20 "$SB" --show dim
HEARTH_SERVER_DASH_FIXTURE="$SV/quiet.json" run server-quiet-nothing-working hearth-server 20 "$SB"
export HEARTH_SERVER_DASH_FIXTURE="$SV/dash.json"
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
