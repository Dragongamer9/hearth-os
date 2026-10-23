#!/bin/bash
# Hearth Recorder for real (CI: .github/workflows/capture-apps.yml runs it inside hosts/hearth-media/ci/session.sh,
# which gives it a screen, PipeWire, a tone on the speakers and a fake microphone). Every recording is checked
# with ffprobe: long enough, the right streams and size, and not silent.
#   RECORDER=path/to/hearth-recorder bash tests/e2e.sh
set -u
B=${RECORDER:-hearth-recorder}
export HOME=$(mktemp -d)
CFG=$HOME/.config/hearth-recorder
mkdir -p "$CFG"
OUT=${OUT:-/tmp/recorder-e2e}
mkdir -p "$OUT"
passed=0
fail() { echo "::error title=Recorder ($KIND) $1::$2"; tail -n 15 "${XDG_RUNTIME_DIR}/hearth-recorder/log.txt" 2>/dev/null; FAILED=1; }
note() { echo "ok: $*"; SUMMARY="$SUMMARY$*  ·  "; passed=$((passed + 1)); }
field() { python3 -c 'import json,sys
d=json.loads(sys.stdin.read() or "{}")
for k in sys.argv[1].split("."): d = d.get(k) if isinstance(d, dict) else None
print("" if d is None else d)' "$1"; }
dur() { ffprobe -v error -show_entries format=duration -of csv=p=0 "$1" 2>/dev/null | cut -d. -f1; }
streams() { ffprobe -v error -show_entries stream=codec_type,codec_name,width,height -of csv=p=0 "$1" 2>/dev/null | tr '\n' ' '; }
loud() { ffmpeg -nostdin -i "$1" -af volumedetect -f null - 2>&1 | grep -o 'mean_volume: [-0-9.]*' | grep -o '[-0-9.]*$'; }
settings() { echo "$1" > "$CFG/settings.json"; }
keep() { cp "$1" "$OUT/$2" 2>/dev/null; }
FAILED=0
SUMMARY=""

echo "--- what Recorder finds here"
$B --probe | tee "$OUT/probe-$KIND.json" | head -60

# 1. The whole screen with what's playing, 4 seconds.
settings '{"fps": 30, "system": true, "mic": false}'
r=$($B --start --json); echo "$r"
if [ "$(echo "$r" | field ok)" = True ]; then
  sleep 4
  r=$($B --stop --json); echo "$r"; f=$(echo "$r" | field file)
  s=$(streams "$f"); d=$(dur "$f"); v=$(loud "$f")
  echo "file: $f · $s · ${d}s · mean volume ${v} dB"
  if [ -s "$f" ] && [ "${d:-0}" -ge 3 ] && [[ "$s" == *video* ]] && [[ "$s" == *audio* ]] && awk "BEGIN{exit !(${v:--99} > -50)}"; then
    note "whole screen: $s ${d}s, sound at ${v} dB ($(echo "$r" | field kind))"; keep "$f" "screen-$KIND.mp4"
  else fail "whole screen" "$f: streams '$s', ${d}s, volume ${v}"; fi
else fail "whole screen" "didn't start: $(echo "$r" | field error)"; fi

# 2. An area, 640x480, no sound (Game Mode without gpu-screen-recorder records the whole screen only, and says so).
settings '{"fps": 30, "system": false, "mic": false}'
r=$($B --start --area="16,16 640x480" --json); echo "$r"
if [ "$KIND" = gamescope ] && [ "$(echo "$r" | field ok)" != True ]; then
  [[ "$(echo "$r" | field error)" == *whole\ screen* ]] && note "area in Game Mode: $(echo "$r" | field error)" || fail "area" "$(echo "$r" | field error)"
elif [ "$(echo "$r" | field ok)" = True ]; then
  sleep 3
  f=$($B --stop --json | field file); s=$(streams "$f")
  if [[ "$s" == *640,480* ]] && [[ "$s" != *audio* ]]; then note "area: $s"; keep "$f" "area-$KIND.mp4"; else fail "area" "$f: '$s'"; fi
else fail "area" "didn't start: $(echo "$r" | field error)"; fi

# 3. What's playing and the microphone together (a private mix).
settings '{"fps": 30, "system": true, "mic": true}'
r=$($B --toggle --json); echo "$r"
if [ "$(echo "$r" | field ok)" = True ]; then
  sleep 3
  r=$($B --toggle --json); echo "$r"; f=$(echo "$r" | field file); v=$(loud "$f")
  if [[ "$(streams "$f")" == *audio* ]] && awk "BEGIN{exit !(${v:--99} > -50)}"; then note "speakers + microphone mixed: ${v} dB"; else fail "mix" "$f: volume ${v}"; fi
  pactl list short modules | grep -q hearth_mix && fail "mix" "the mix was left behind"
else fail "mix" "didn't start: $(echo "$r" | field error)"; fi

# 4. Instant replay: 10 seconds kept, saved after 14.
settings '{"fps": 30, "system": true, "mic": false, "replaySecs": 10}'
r=$($B --replay --json); echo "$r"
if [ "$(echo "$r" | field ok)" = True ]; then
  sleep 14
  r=$($B --save-replay --json); echo "$r"; f=$(echo "$r" | field saved); d=$(dur "$f")
  if [ -s "$f" ] && [ "${d:-0}" -ge 5 ] && [ "${d:-99}" -le 13 ]; then note "instant replay saved ${d}s ($(streams "$f"))"; keep "$f" "replay-$KIND.mp4"; else fail "instant replay" "$f: ${d}s"; fi
  sleep 3
  r=$($B --save-replay --json); f2=$(echo "$r" | field saved)
  [ -s "$f2" ] && [ "$f2" != "$f" ] && note "saved a second replay" || fail "instant replay" "second save: $r"
  r=$($B --replay --json); echo "$r"
  [ "$(echo "$r" | field ok)" = True ] || fail "instant replay" "turning it off: $r"
else fail "instant replay" "didn't start: $(echo "$r" | field error)"; fi

# 5. The sound recorder: Opus, then MP3, with the level meter moving.
for fmt in opus mp3; do
  settings "{\"soundFormat\": \"$fmt\"}"
  r=$($B --sound-toggle --json); echo "$r"
  if [ "$(echo "$r" | field ok)" = True ]; then
    sleep 2; lvl=$($B --status | field session.level); sleep 1.5
    r=$($B --sound-toggle --json); echo "$r"; f=$(echo "$r" | field file); v=$(loud "$f")
    if [[ "$f" == *.$fmt ]] && [[ "$(streams "$f")" == *audio* ]] && awk "BEGIN{exit !(${v:--99} > -50 && ${lvl:-0} > 0.01)}"; then note "sound recorder ($fmt): ${v} dB, level ${lvl}"; keep "$f" "sound-$KIND.$fmt"; else fail "sound recorder ($fmt)" "$f: $(streams "$f") volume ${v} level ${lvl}"; fi
  else fail "sound recorder ($fmt)" "didn't start: $(echo "$r" | field error)"; fi
done

# 6. Asking to stop when nothing records says so; two at once is refused.
r=$($B --stop --json); [ "$(echo "$r" | field ok)" = False ] && note "stop with nothing recording: $(echo "$r" | field error)" || fail "idle stop" "$r"
$B --start --json >/dev/null && { r=$($B --sound-toggle --json); [ "$(echo "$r" | field ok)" = False ] && note "one at a time: $(echo "$r" | field error)" || fail "one at a time" "$r"; $B --stop --json >/dev/null; }

ls -la "$HOME/Videos/Recordings" "$HOME/Music/Recordings" 2>/dev/null
echo "passed: $passed"
echo "::notice title=Recorder ($KIND) passed::${SUMMARY:-nothing}" | cut -c1-4000
exit $FAILED
