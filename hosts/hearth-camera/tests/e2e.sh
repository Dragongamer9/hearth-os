#!/bin/bash
# Hearth Camera for real, with ffmpeg's test picture as the camera (HEARTH_CAMERA_TEST=1) and the fake microphone
# that hosts/hearth-media/ci/session.sh sets up. CI: .github/workflows/capture-apps.yml.
set -u
B=${CAMERA:-hearth-camera}
export HOME=$(mktemp -d) HEARTH_CAMERA_TEST=1
OUT=${OUT:-/tmp/camera-e2e}; mkdir -p "$OUT"
FAILED=0
fail() { echo "::error title=Camera ($KIND) $1::$2"; tail -n 5 "$XDG_RUNTIME_DIR/hearth-camera/ffmpeg.log" 2>/dev/null; FAILED=1; }
note() { echo "ok: $*"; SUMMARY="$SUMMARY$*  ·  "; }
streams() { ffprobe -v error -show_entries stream=codec_type,codec_name,width,height -of csv=p=0 "$1" 2>/dev/null | tr '\n' ' '; }
dur() { ffprobe -v error -show_entries format=duration -of csv=p=0 "$1" 2>/dev/null | cut -d. -f1; }

echo "--- cameras"; $B --list | tee "$OUT/cameras-$KIND.json"
grep -q '"test"' "$OUT/cameras-$KIND.json" && note "lists the cameras (and the test camera)" || fail "list" "no test camera"

p=$($B --photo --device test 2>&1 | tail -1); s=$(streams "$p")
if [[ "$s" == *mjpeg*1280,720* ]]; then note "photo: $s"; cp "$p" "$OUT/photo-$KIND.jpg"; else fail "photo" "$p: $s"; fi

v=$($B --video 4 --device test 2>&1 | tail -1); s=$(streams "$v"); d=$(dur "$v")
if [[ "$s" == *video* ]] && [[ "$s" == *audio* ]] && [ "${d:-0}" -ge 3 ]; then note "video with sound: $s ${d}s"; cp "$v" "$OUT/video-$KIND.mp4"; else fail "video" "$v: $s ${d}s"; fi

v=$($B --video 2 --device test --no-sound 2>&1 | tail -1); s=$(streams "$v")
if [[ "$s" == *video* ]] && [[ "$s" != *audio* ]]; then note "video without sound: $s"; else fail "silent video" "$v: $s"; fi

r=$($B --photo --device /dev/video99 2>&1); [ $? -ne 0 ] && note "a missing camera says: $r" || fail "missing camera" "$r"
echo "::notice title=Camera ($KIND) passed::${SUMMARY:-nothing}" | cut -c1-4000
exit $FAILED
