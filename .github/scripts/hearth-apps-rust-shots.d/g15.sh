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
