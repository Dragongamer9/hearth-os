# Logitech G15 (hearth-g15-app, docs/G15.md): every page of the approved mockups (docs/mockups/g15) on a pretend G15
# (hearth-g15 make-fixture) with a g15.json like the mockups' (hearth-g15-sample.json), then the keyboard's screen pages.
G15FX=/tmp/g15-fixture
rm -rf "$G15FX"; "$BIN/hearth-g15" make-fixture "$G15FX"
cp "$ROOT/.github/scripts/hearth-g15-sample.json" "$G15FX/g15.json"
export HEARTH_G15_FIXTURE="$G15FX" HEARTH_G15_CONFIG="$G15FX/g15.json"
cp "$G15FX/g15.json" /tmp/g15-before.json
run g15-1-overview hearth-g15-app 20 "$BIN/hearth-g15-app" --page overview
run g15-2-g-keys hearth-g15-app 0 "$BIN/hearth-g15-app" --page g-keys --key 5
run g15-3-keyboard-screen hearth-g15-app 0 "$BIN/hearth-g15-app" --page screen
run g15-4-lighting hearth-g15-app 0 "$BIN/hearth-g15-app" --page lighting
run g15-6-make-your-own hearth-g15-app 0 "$BIN/hearth-g15-app" --page editor:dads-screen --element 2
run g15-7-hearth-actions hearth-g15-app 0 "$BIN/hearth-g15-app" --page g-keys --key 5 --show hearth
run g15-8-games hearth-g15-app 0 "$BIN/hearth-g15-app" --page overview --show games
# Opening the app wrote nothing (red flag 9).
cmp -s "$G15FX/g15.json" /tmp/g15-before.json || echo "::error title=g15::opening the app changed g15.json"
for p in clock system music alerts alert rec set dads-screen inverted; do
  "$BIN/hearth-g15" frame "$p" "$ROOT/shots/g15-lcd-$p.png" --scale 4 || echo "::error title=g15 frame::$p"
done
unset HEARTH_G15_FIXTURE HEARTH_G15_CONFIG
run settings-g15 hearth-settings 0 "$BIN/hearth-settings-app" --page g15
# The editor (g15-6) with other elements picked, and a picture added the way Add › Picture does it (a made-up
# photo: a shaded ball and a grey ramp, so the dithering shows), on a copy of the sample so g15.json above stays.
G15ED=/tmp/g15-editor
rm -rf "$G15ED"; mkdir -p "$G15ED"
python3 - "$ROOT/.github/scripts/hearth-g15-sample.json" "$G15ED" <<'PY'
import json, math, struct, sys, zlib
d = json.load(open(sys.argv[1]))
d["screens"].append({"id": "photo", "name": "Photo", "show": "hidden", "elements": [{"type": "text", "x": 96, "y": 4, "size": 1, "text": "Hello Dad"}, {"type": "clock", "x": 96, "y": 16, "size": 2, "format": "24h"}]})
json.dump(d, open(sys.argv[2] + "/g15.json", "w"), indent=1)
W, H = 180, 86
rows = b""
for y in range(H):
    row = bytearray([0])
    for x in range(W):
        dx, dy = x - 50, y - 43
        r = math.hypot(dx, dy)
        if r < 40:
            light = max(0.0, (-(dx + 15) * 0.6 - (dy + 15) * 0.6) / 40 + 0.5)
            v = int(40 + 200 * min(1.0, light))
        elif x > 100:
            v = int(255 * (y / (H - 1)))
        else:
            v = 255
        row.append(max(0, min(255, v)))
    rows += bytes(row)
def chunk(k, b):
    return struct.pack(">I", len(b)) + k + b + struct.pack(">I", zlib.crc32(k + b) & 0xffffffff)
png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 0, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b"")
open(sys.argv[2] + "/photo.png", "wb").write(png)
PY
export HEARTH_G15_FIXTURE="$G15FX" HEARTH_G15_CONFIG="$G15ED/g15.json"
run g15-6b-editor-value hearth-g15-app 0 "$BIN/hearth-g15-app" --page editor:dads-screen --element 0
run g15-6c-editor-clock hearth-g15-app 0 "$BIN/hearth-g15-app" --page editor:dads-screen --element 3
run g15-6d-editor-picture hearth-g15-app 0 "$BIN/hearth-g15-app" --page editor:photo --add-picture "$G15ED/photo.png"
grep -q '"bits"' "$G15ED/g15.json" || echo "::error title=g15 editor::adding a picture didn't save its dots"
# Keyboard screen › Dots: Light on dark (inverted), the mockups g15-7 and g15-8.
G15FX=/tmp/g15-fixture
export HEARTH_G15_FIXTURE="$G15FX" HEARTH_G15_CONFIG="$G15FX/g15.json"
python3 - "$G15FX/g15.json" <<'PY'
import json, sys
p = sys.argv[1]; c = json.load(open(p)); c["lcdInvert"] = True; open(p, "w").write(json.dumps(c, indent=1))
PY
run g15-9-keyboard-screen-inverted hearth-g15-app 0 "$BIN/hearth-g15-app" --page screen
mkdir -p /tmp/g15inv
for p in clock system music alert rec set dads-screen; do "$BIN/hearth-g15" frame "$p" "/tmp/g15inv/$p.png" --scale 3 || echo "::error title=g15 frame::inverted $p"; done
python3 - "$ROOT/shots/g15-lcd-inverted-pages.png" <<'PY'
import sys
from PIL import Image
names = ["clock", "system", "music", "alert", "rec", "set", "dads-screen"]
ims = [Image.open("/tmp/g15inv/%s.png" % n).convert("RGB") for n in names]
w, h = ims[0].width, ims[0].height
sheet = Image.new("RGB", (w * 2 + 30, (h + 10) * 4 + 10), (24, 26, 31))
for i, im in enumerate(ims):
    sheet.paste(im, (10 + (i % 2) * (w + 10), 10 + (i // 2) * (h + 10)))
sheet.save(sys.argv[1])
PY
unset HEARTH_G15_FIXTURE HEARTH_G15_CONFIG
