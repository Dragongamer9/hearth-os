# Hearth Home (hosts/hearth-ui/apps/home; checklist docs/mockups/checklists/open-source-14-home.md): board 14 on
# made-up Home Assistant states in its own format (HEARTH_HOME_FIXTURE; nothing is sent), at the board's size, and the
# sign-in as it really starts. Sourced by hearth-apps-rust-test.sh.
HFX=/tmp/home-fx
rm -rf "$HFX"; mkdir -p "$HFX"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" home "$HFX" || echo "::error title=home fixture::it couldn't be made"
HM="$BIN/hearth-home-app"
run home-0-sign-in hearth-home 20 env XDG_CONFIG_HOME="$HFX/empty" "$HM"
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
run home-1-board hearth-home 0 env XDG_CONFIG_HOME="$HFX/config" HEARTH_HOME_FIXTURE="$HFX/home.json" "$HM"
run home-2-edit-board hearth-home 0 env XDG_CONFIG_HOME="$HFX/config" HEARTH_HOME_FIXTURE="$HFX/home.json" HEARTH_ARRANGE_OPEN=home:board HEARTH_ARRANGE_PICK=lights "$HM"
# The cameras as moving video (mjpeg.rs): a stand-in for Home Assistant's camera_proxy_stream (MJPEG), two pictures a
# second apart: the sweeping bar and the time on each camera moved between them.
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/home-mjpeg.py" 18123 "$HFX" >/tmp/home-mjpeg.log 2>&1 &
mj=$!
sleep 1
env XDG_CONFIG_HOME="$HFX/config" HEARTH_HOME_FIXTURE="$HFX/home.json" HEARTH_HOME_STREAM=http://127.0.0.1:18123 "$HM" >/tmp/home-moving.log 2>&1 &
hp=$!
for i in $(seq 1 400); do mapped hearth-home && break; sleep 0.02; done
sleep 3
grim "$ROOT/shots/home-3-moving-a.png" 2>/dev/null
sleep 1.2
grim "$ROOT/shots/home-3-moving-b.png" 2>/dev/null
kill $hp 2>/dev/null; wait $hp 2>/dev/null
kill $mj 2>/dev/null; wait $mj 2>/dev/null
python3 - "$ROOT/shots/home-3-moving-a.png" "$ROOT/shots/home-3-moving-b.png" <<'PY' || echo "::error title=home cameras::the camera tiles didn't move"
import sys
from PIL import Image, ImageChops
a, b = (Image.open(x).convert('RGB') for x in sys.argv[1:3])
# the cameras' tile: the right third of the board's top half
box = (a.width * 2 // 3, 0, a.width, a.height // 2)
d = ImageChops.difference(a.crop(box), b.crop(box)).getbbox()
print('::notice title=home cameras::changed between the two pictures: %s' % (d,))
sys.exit(0 if d else 1)
PY
grep -q '"room": "living_room"' "$HFX/config/nimbus/home.json" || echo "::error title=home settings::opening Hearth Home changed home.json"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
