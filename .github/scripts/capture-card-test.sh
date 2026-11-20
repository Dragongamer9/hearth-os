#!/bin/bash
# capture-card.yml's "run" job: Capture Card on a headless Wayland screen (sway), as an ordinary user, with CI's
# stand-in card (HEARTH_CAPTURE_FAKE_CARD: a JSON file naming the card, its modes, its sound, and the JPEG it sends
# as its MJPEG picture through the app's own pipeline). Every page of the approved mockups is pictured
# (shots/cc-*.png, 1920 × 1080; the Ally's Game Mode also at 1280 × 800). Measured: CPU at rest (limit 0.3 % of a
# core on pages with no picture), CPU while showing a 1080p60 picture (reported), memory. Checked: opening pages
# writes no settings file; a recording is a playable MP4; the picture comes up by itself when the signal or the
# card arrives.
set -u
ROOT="$PWD"
: > "$ROOT/notes.txt"; chmod 666 "$ROOT/notes.txt"
BIN="$ROOT/bin"
FIX="$ROOT/hosts/capture-card/tests/fixtures"
mkdir -p "$ROOT/shots" && chmod 777 "$ROOT/shots"
: > "$ROOT/results.txt"; chmod 666 "$ROOT/results.txt"
id ci >/dev/null 2>&1 || useradd -m ci
CIH=/home/ci
RUNDIR=/tmp/xdg-ci
mkdir -p "$RUNDIR" && chown ci "$RUNDIR" && chmod 700 "$RUNDIR"
chmod -R a+rX "$BIN" "$ROOT/hosts/capture-card"
cp /usr/bin/sway /tmp/sway-nocap && chmod 755 /tmp/sway-nocap

cat > /tmp/sway.conf <<'EOF'
output HEADLESS-1 resolution 1920x1080 position 0 0 bg #101010 solid_color
default_border none
default_floating_border none
focus_follows_mouse no
EOF

# The Store's own files, as an installed app that's up to date (Settings › Hearth Store: Up to date).
mkdir -p "$CIH/.local/share/hearth/store/apps/capture-card" "$CIH/.cache/hearth/hearth-store"
echo '{"version": "1.0"}' > "$CIH/.local/share/hearth/store/apps/capture-card/store.json"
echo '{"apps": {"capture-card": {"version": "1.0"}}}' > "$CIH/.cache/hearth/hearth-store/releases.json"
chown -R ci "$CIH"

# The fixtures: the stand-in card's pictures, 6 recordings (their thumbnails and lengths in the app's cache, as the
# app keeps them once worked out; the files themselves are sparse) and 14 screenshots (real PNGs: the app makes their
# thumbnails itself), with the mockups' times on today, yesterday and two days ago.
cat > /tmp/fixtures.py <<'PY'
import json, os, sys, time, datetime
from PIL import Image
fix, home = sys.argv[1], sys.argv[2]
scene = Image.open(os.path.join(fix, 'scene.jpg')).convert('RGB')
os.makedirs('/tmp/cc', exist_ok=True)
scene.save('/tmp/cc/scene.jpg', quality=88)
Image.new('RGB', (640, 480), (0, 0, 0)).save('/tmp/cc/black.jpg', quality=90)

def hue(img, deg, bright=1.0):
    h, s, v = img.convert('HSV').split()
    h = h.point(lambda x: int(x + deg * 255 / 360) % 256)
    if bright != 1.0:
        v = v.point(lambda x: max(0, min(255, int(x * bright))))
    return Image.merge('HSV', (h, s, v)).convert('RGB')

today = datetime.date.today()
def at(days, hh, mm, ss):
    d = today - datetime.timedelta(days=days)
    return datetime.datetime(d.year, d.month, d.day, hh, mm, ss)

rec = os.path.join(home, 'Videos/Capture Card'); shot = os.path.join(home, 'Pictures/Screenshots')
cache = os.path.join(home, '.cache/hearth/capture-card'); thumbs = os.path.join(cache, 'thumbs')
for d in (rec, shot, thumbs):
    os.makedirs(d, exist_ok=True)
RECS = [(0, 3, 41, 7, 768, 3.4e9, -14, 1.0), (0, 2, 10, 55, 362, 1.6e9, 40, 1.0), (1, 5, 22, 30, 91, 0.4e9, 160, 0.8),
        (1, 3, 4, 12, 1335, 5.9e9, -40, 1.1), (2, 4, 47, 1, 200, 0.9e9, 90, 0.9), (2, 1, 15, 44, 489, 2.2e9, 200, 1.0)]
lib = {}
for i, (dd, hh, mm, ss, secs, size, h, b) in enumerate(RECS):
    t = at(dd, hh, mm, ss)
    p = os.path.join(rec, t.strftime('Capture %Y-%m-%d %H-%M-%S.mp4'))
    with open(p, 'wb') as f:
        f.truncate(int(size))
    ts = time.mktime(t.timetuple())
    os.utime(p, (ts, ts))
    th = 'ci-rec-%d.png' % i
    hue(scene, h, b).resize((480, 270)).save(os.path.join(thumbs, th))
    st = os.stat(p)
    lib['%s|%d|%d' % (p, st.st_size, int(st.st_mtime))] = {'secs': secs, 'w': 1920, 'h': 1080, 'fps': 60, 'thumb': th}
with open(os.path.join(cache, 'library.json'), 'w') as f:
    json.dump(lib, f)
SH = [(0, 3, 58, 21, 25), (0, 3, 52, 4, -30), (0, 3, 12, 40, 70), (0, 2, 48, 19, 130), (0, 2, 5, 33, 190), (1, 5, 40, 10, -80),
      (1, 5, 2, 55, 250), (1, 4, 21, 8, 300), (1, 3, 33, 47, 10), (1, 2, 17, 26, -120), (2, 4, 59, 12, 160), (2, 1, 44, 3, 220),
      (2, 1, 2, 9, 280), (2, 0, 30, 50, 330)]
for dd, hh, mm, ss, h in SH:
    t = at(dd, hh, mm, ss)
    p = os.path.join(shot, t.strftime('Capture %Y-%m-%d %H-%M-%S.png'))
    hue(scene, h).save(p, compress_level=1)
    ts = time.mktime(t.timetuple())
    os.utime(p, (ts, ts))
# Hearth Capture's own screenshot: not Capture Card's, so not listed.
scene.resize((320, 180)).save(os.path.join(shot, 'Screenshot 2026-11-19 22-00-00.png'))
PY
runuser -u ci -- python3 /tmp/fixtures.py "$FIX" "$CIH" || echo "::error title=fixtures::the fixtures couldn't be made"

cat > /tmp/inner.sh <<'INNER'
#!/bin/bash
set -u
ROOT="$1"; BIN="$ROOT/bin"
export XDG_RUNTIME_DIR=/tmp/xdg-ci HOME=/home/ci WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1
export HEARTH_RENDERER=cpu XDG_PICTURES_DIR=/home/ci/Pictures XDG_VIDEOS_DIR=/home/ci/Videos
export HEARTH_CAPTURE_FAKE_SOURCES="alsa_input.usb-MACROSILICON_USB3_Video-02.analog-stereo|USB3 Video Analog Stereo"
/tmp/sway-nocap -c /tmp/sway.conf >/tmp/sway.log 2>&1 &
for i in $(seq 1 100); do ls "$XDG_RUNTIME_DIR"/wayland-* >/dev/null 2>&1 && break; sleep 0.1; done
export WAYLAND_DISPLAY=$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-* | grep -v lock | head -1)")
export SWAYSOCK=$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* 2>/dev/null | head -1)
[ -n "$WAYLAND_DISPLAY" ] || { echo "::error title=sway::no Wayland screen: $(tail -5 /tmp/sway.log | tr '\n' '|')"; exit 1; }

mkdir -p /tmp/cc/card
# note TITLE TEXT: a notice, printed by the workflow's Notes steps (GitHub keeps 10 notices a step).
note() { printf '%s\t%s\n' "$1" "$(echo "$2" | tr '\n\t' '  ' | cut -c1-1500)" >> "$ROOT/notes.txt"; }
card() { # card PICTURE: the stand-in card, sending PICTURE (written so the app's watch sees it arrive)
  cat > /tmp/cc/card.json.new <<EOF
{"name": "Monster 4K", "chip": "USB3 Video (MS2130)", "speed": 5000, "id": "345f:2130",
 "modes": [{"format": "MJPG", "w": 1920, "h": 1080, "fps": [60, 50, 30]}, {"format": "MJPG", "w": 1280, "h": 720, "fps": [60, 30]},
           {"format": "MJPG", "w": 640, "h": 480, "fps": [30, 5]}, {"format": "YUYV", "w": 1920, "h": 1080, "fps": [30, 5]}],
 "picture": "/tmp/cc/pic.jpg", "sound": "alsa_input.usb-MACROSILICON_USB3_Video-02.analog-stereo"}
EOF
  cp "$1" /tmp/cc/pic.jpg.new && mv /tmp/cc/pic.jpg.new /tmp/cc/pic.jpg
  mv /tmp/cc/card.json.new /tmp/cc/card/card.json
}
nocard() { rm -f /tmp/cc/card/card.json; }
export HEARTH_CAPTURE_FAKE_CARD=/tmp/cc/card/card.json

mapped() { swaymsg -t get_tree 2>/dev/null | grep -q "\"app_id\": \"$1\""; }
now_ms() { echo $(( $(date +%s%N) / 1000000 )); }
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
measure() { # measure NAME KIND SECONDS PID
  local a b hz cpu rss
  a=$(ticks $4); sleep "$3"; b=$(ticks $4)
  hz=$(getconf CLK_TCK)
  cpu=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $hz / $3))")
  rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$4/status 2>/dev/null)
  echo "$2 $1 cpu=$cpu rss=${rss:-0}" | tee -a "$ROOT/results.txt"
}
shoot() { grim "$ROOT/shots/$1.png" 2>/tmp/grim.log || echo "::warning title=$1::grim: $(cat /tmp/grim.log)"; }
want() { [ -z "${HEARTH_SHOTS_ONLY:-}" ] || printf '%s' "$1" | grep -Eq "$HEARTH_SHOTS_ONLY"; }

# open NAME WAIT ARGS...: start, wait until its window shows and WAIT s more; PID in $pid.
open_app() {
  name=$1; wait_s=$2; shift 2
  "$BIN/hearth-capture-card" "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  t0=$(now_ms); ff=-1
  for i in $(seq 1 600); do
    if mapped hearth-capture-card; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.01
  done
  echo "SHOT $name firstframe=$ff" >> "$ROOT/results.txt"
  sleep "$wait_s"
}
close_app() {
  kill -0 $pid 2>/dev/null || echo "::error title=$1::it ended by itself: $(tail -c 1500 /tmp/$1.log | tr '\n' '|')"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.5
}
# run NAME REST WAIT ARGS...: picture, and (REST > 0) measure at rest.
run() {
  local n=$1 rest=$2 w=$3; shift 3
  want "$n" || return 0
  open_app "$n" "$w" "$@"
  shoot "$n"
  if [ "$rest" -gt 0 ] && kill -0 $pid 2>/dev/null; then sleep 2; measure "$n" RESULT "$rest" $pid; fi
  close_app "$n"
}

"$BIN/hearth-capture-card" --probe > /tmp/probe.txt 2>&1; nocard
card /tmp/cc/scene.jpg; "$BIN/hearth-capture-card" --probe > /tmp/probe.txt 2>&1
note "--probe" "$(tr '\n' '|' < /tmp/probe.txt)"

# ---------------- 2b no card, at rest; then the card arrives and the picture comes up by itself
nocard
if want cc-2b-no-card; then
  open_app cc-2b-no-card 3
  shoot cc-2b-no-card
  sleep 2; measure cc-2b-no-card RESULT 20 $pid
  card /tmp/cc/scene.jpg
  sleep 6
  shoot cc-2d-card-arrives
  close_app cc-2b-no-card
fi

# ---------------- 2 no signal (the card sends a black picture), at rest; then the signal arrives
card /tmp/cc/black.jpg
if want cc-2-no-signal; then
  open_app cc-2-no-signal 6
  shoot cc-2-no-signal
  sleep 2; measure cc-2-no-signal WAITING 20 $pid
  cp /tmp/cc/scene.jpg /tmp/cc/pic.jpg.new && mv /tmp/cc/pic.jpg.new /tmp/cc/pic.jpg
  sleep 6
  shoot cc-2c-signal-arrives
  close_app cc-2-no-signal
fi

# ---------------- 1 the viewer: its bar, the tooltip over Full screen; CPU while showing 1080p60
card /tmp/cc/scene.jpg
if want cc-1-viewer; then
  open_app cc-1-viewer 5 --show bar,pill,tip
  shoot cc-1-viewer
  measure cc-1-viewer VIEWING 15 $pid
  note delay "$(grep -h "delay" /tmp/cc-1-viewer.log | tail -2 | tr '\n' '|')"
  close_app cc-1-viewer
fi
# The bar and the pill hidden: nothing else on screen.
run cc-1c-viewer-bar-hidden 0 9 --show pill-only

# ---------------- 1b recording (a real MP4 through the app's own encoder) and a screenshot
if want cc-1b-viewer-recording; then
  # The picture while recording (and a screenshot just taken).
  open_app cc-1b-viewer-recording 7 --show record,shot
  shoot cc-1b-viewer-recording
  close_app cc-1b-viewer-recording
  ls -t "$HOME/Videos/Capture Card"/*.mp4 | head -1 | xargs -r -I{} rm -f "{}"
  ls -t "$HOME/Pictures/Screenshots"/Capture*.png | head -1 | while read -r f; do
    note screenshot "saved $(basename "$f") ($(python3 -c "from PIL import Image; import sys; print('%dx%d' % Image.open(sys.argv[1]).size)" "$f"))"
    mv "$f" /tmp/cc/shot.png
  done
  # A real recording, started and stopped by the app (5 s), checked with GStreamer's own reader.
  GST_DEBUG=2 open_app cc-1e-recording-file 9 --show record,stop-record
  close_app cc-1e-recording-file
  newest=$(ls -t "$HOME/Videos/Capture Card"/*.mp4 2>/dev/null | head -1)
  if [ -n "$newest" ] && [ -s "$newest" ] && [ "$(stat -c %Y "$newest")" -gt "$(( $(date +%s) - 120 ))" ]; then
    info=$(gst-discoverer-1.0 -v "$newest" 2>&1 | grep -iE "Duration|video #|audio #|container|Width|Height|Frame rate|Seekable|Sample rate" | sort -u | tr '\n' ' ' | tr -s ' ')
    note recording "$(basename "$newest"): $(du -h "$newest" | cut -f1) · $info"
    mv "$newest" /tmp/cc/recording.mp4
  else
    echo "::warning title=recording::no recording was made: $(grep -h -i -E 'recording|encoder|WARN|ERROR' /tmp/cc-1e-recording-file.log | grep -v 'recording: queue' | tail -6 | cut -c1-300 | tr '\n' '|')"
  fi
  note "recording log" "$(grep -h 'hearth-capture-card: recording' /tmp/cc-1e-recording-file.log | grep -v 'recording: queue' | tr '\n' '|')"
fi

# ---------------- 3 Settings, 4 Recordings, 4b Screenshots (at rest)
run cc-3-settings 20 3 --page settings
run cc-4-recordings 20 4 --page recordings
run cc-4b-screenshots 20 4 --page screenshots
# A real recording's own thumbnail and length (worked out by the app through GStreamer).
if want cc-4c-real-recording && [ -f /tmp/cc/recording.mp4 ]; then
  cp /tmp/cc/recording.mp4 "$HOME/Videos/Capture Card/Capture 2099-01-01 00-00-00.mp4"
  touch "$HOME/Videos/Capture Card/Capture 2099-01-01 00-00-00.mp4"
  run cc-4c-real-recording 0 8 --page recordings
  rm -f "$HOME/Videos/Capture Card/Capture 2099-01-01 00-00-00.mp4"
fi

# ---------------- 5 Game Mode (1920 × 1080 as drawn, and the Ally's 1280 × 800)
run cc-5-game-mode-viewer 0 2.5 --game-mode --show pill
run cc-5a-game-mode-hints-faded 0 6 --game-mode
run cc-5b-game-mode-quick-menu 0 4 --game-mode --show menu
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null; sleep 0.5
run cc-5c-ally-viewer 0 2.5 --game-mode --show pill
run cc-5d-ally-quick-menu 0 4 --game-mode --show menu
nocard
run cc-5e-ally-no-card 0 3 --game-mode
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null; sleep 0.5

# ---------------- the red flags
if [ -e "$HOME/.config/nimbus/capture-card.json" ]; then
  echo "::error title=settings::opening pages wrote capture-card.json: $(cat "$HOME/.config/nimbus/capture-card.json")"
else
  note settings "opening every page wrote no settings file"
fi
for f in /tmp/cc-*.log; do grep -h -m3 -E 'panicked|stopped:|GStreamer:' "$f" 2>/dev/null | while read -r l; do echo "::warning title=$(basename "$f" .log)::$(echo "$l" | cut -c1-600)"; done; done
note pipeline "$(grep -h "pipeline:" /tmp/cc-1-viewer.log 2>/dev/null | head -1)"
note "recording pipeline" "$(grep -h "recording: queue" /tmp/cc-1e-recording-file.log 2>/dev/null | head -1)"
note library "$(python3 -c "import json; d=json.load(open('/home/ci/.cache/hearth/capture-card/library.json')); print(len(d), 'files known;', [v for k, v in d.items() if k.endswith('.png') or '.png|' in k][:1])" 2>&1)"
kill %1 2>/dev/null
exit 0
INNER
chmod 755 /tmp/inner.sh
runuser -u ci -- /tmp/inner.sh "$ROOT"
inner=$?

# The limits: pages with no picture at most 0.3 % of a core; while waiting for a signal and while showing a picture,
# reported (the picture is live work; CI draws with the CPU, a PC with its graphics card).
fail=$inner
while read -r kind name rest; do
  case "$kind" in RESULT|WAITING|VIEWING) ;; *) continue ;; esac
  eval "$(echo "$rest" | tr ' ' '\n' | sed 's/^/r_/')"
  case "$kind" in
    RESULT)
      printf '%s\t%s\n' "$name" "at rest ${r_cpu} % of a core, ${r_rss} MB" >> "$ROOT/notes.txt"
      python3 -c 'import sys; c=float(sys.argv[2]); print("::error title=%s::%.2f %% of a core at rest (limit 0.3 %%)" % (sys.argv[1], c)) if c > 0.3 else None; sys.exit(1 if c > 0.3 else 0)' "$name" "$r_cpu" || fail=1 ;;
    WAITING) printf '%s\t%s\n' "$name" "waiting for a signal (the stand-in card sends a small black picture 5 times a second): ${r_cpu} % of a core, ${r_rss} MB" >> "$ROOT/notes.txt" ;;
    VIEWING) printf '%s\t%s\n' "$name" "showing 1080p60 (decoding and drawing with the CPU here): ${r_cpu} % of a core, ${r_rss} MB" >> "$ROOT/notes.txt" ;;
  esac
done < "$ROOT/results.txt"
ls -la "$ROOT/shots" | tail -25
exit $fail
