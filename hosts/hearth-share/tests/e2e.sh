#!/bin/bash
# Hearth Share for real: shares the screen that hosts/hearth-media/ci/session.sh made (labwc headless or Xvfb) and
# checks what a browser would get: the viewer page, a picture, the live MJPEG stream and the MP3 sound.
# CI: .github/workflows/capture-apps.yml.
set -u
B=${SHARE:-hearth-share}
export HOME=$(mktemp -d)
OUT=${OUT:-/tmp/share-e2e}; mkdir -p "$OUT"
FAILED=0
fail() { echo "::error title=Share ($KIND) $1::$2"; tail -n 5 "$XDG_RUNTIME_DIR/hearth-share/capture.log" 2>/dev/null; FAILED=1; }
note() { echo "ok: $*"; SUMMARY="$SUMMARY$*  ·  "; }
field() { python3 -c 'import json,sys
d=json.loads(sys.stdin.read() or "{}"); v=d.get(sys.argv[1]); print("" if v is None else v)' "$1"; }

$B --serve 40 > "$OUT/serve.json" 2> "$OUT/serve.err" &
pid=$!
for _ in $(seq 60); do grep -q '"ok"' "$OUT/serve.json" 2>/dev/null && break; sleep 0.5; done
cat "$OUT/serve.json"
j=$(head -1 "$OUT/serve.json")
if [ "$(echo "$j" | field ok)" != True ]; then fail "start" "$(echo "$j" | field error) $(cat "$OUT/serve.err")"; exit 1; fi
port=$(echo "$j" | field port); token=$(echo "$j" | field token)
base="http://127.0.0.1:$port/$token"
note "sharing with $(echo "$j" | field backend), links: $(echo "$j" | field urls)"

code=$(curl -s -o "$OUT/page.html" -w '%{http_code}' "$base/")
[ "$code" = 200 ] && grep -q live.mjpg "$OUT/page.html" && note "viewer page" || fail "page" "HTTP $code"

curl -s -o "$OUT/frame-$KIND.jpg" "$base/frame.jpg"
s=$(ffprobe -v error -show_entries stream=codec_name,width,height -of csv=p=0 "$OUT/frame-$KIND.jpg" 2>/dev/null)
[[ "$s" == mjpeg,*,720 ]] && note "a picture: $s" || fail "picture" "'$s'"

curl -s --max-time 5 -o "$OUT/live.mjpg" "$base/live.mjpg"
n=$(grep -c -a -- '--hearthframe' "$OUT/live.mjpg")
[ "${n:-0}" -ge 15 ] && note "live stream: $n pictures in 5 seconds" || fail "live stream" "$n pictures in 5 seconds"

if [ "$(echo "$j" | field hasSound)" = True ]; then
  curl -s --max-time 5 -o "$OUT/live.mp3" "$base/live.mp3"
  a=$(ffprobe -v error -show_entries stream=codec_name -of csv=p=0 "$OUT/live.mp3" 2>/dev/null)
  v=$(ffmpeg -nostdin -i "$OUT/live.mp3" -af volumedetect -f null - 2>&1 | grep -o 'mean_volume: [-0-9.]*' | grep -o '[-0-9.]*$')
  [ "$a" = mp3 ] && awk "BEGIN{exit !(${v:--99} > -50)}" && note "sound: $a at ${v} dB" || fail "sound" "'$a' ${v} dB"
else fail "sound" "no sound stream"; fi

code=$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port/wrongtoken/live.mjpg")
[ "$code" = 404 ] && note "a wrong link gets nothing" || fail "token" "HTTP $code"

wait $pid; rc=$?
[ $rc = 0 ] && note "stopped cleanly" || fail "stop" "exit $rc: $(cat "$OUT/serve.err")"
echo "::notice title=Share ($KIND) passed::${SUMMARY:-nothing}" | cut -c1-4000
exit $FAILED
