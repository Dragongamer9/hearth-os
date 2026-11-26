# Hearth Imagine (hosts/hearth-ui/apps/imagine; checklist docs/mockups/checklists/open-source-06-imagine.md): board 06
# against a pretend ComfyUI (oss-fixtures.py comfy: its own API answers and /ws, pictures 1 and 2 done, 3 on step 18 of
# 28, two others waiting; nothing is made here), at the board's size (1920 x 1080). Sourced by hearth-apps-rust-test.sh
# inside its headless screen; run() pictures each one.
IFX2=/tmp/imagine-fx
rm -rf "$IFX2"; mkdir -p "$IFX2"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" imagine "$IFX2" || echo "::error title=imagine fixture::it couldn't be made"
IM="$BIN/hearth-imagine-app"
IENV="XDG_DATA_HOME=$IFX2/data XDG_CONFIG_HOME=$IFX2/config HEARTH_IMAGINE_COMFY_DIR=$IFX2/ComfyUI"
# at rest with nothing being made (ComfyUI answering, its queue empty): measured on the usual screen
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" comfy 8188 idle >/tmp/comfy-idle.log 2>&1 &
cm=$!
sleep 1
run imagine-0-at-rest hearth-imagine 20 env $IENV "$IM"
kill $cm 2>/dev/null; wait $cm 2>/dev/null
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" comfy 8188 >/tmp/comfy.log 2>&1 &
cm=$!
sleep 1
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
SHOT_WAIT=4 run imagine-1-board hearth-imagine 0 env $IENV "$IM"
SHOT_WAIT=4 run imagine-2-type-only hearth-imagine 0 env $IENV "$IM" --view type
SHOT_WAIT=4 run imagine-3-graph-only hearth-imagine 0 env $IENV "$IM" --view graph
SHOT_WAIT=4 run imagine-4-edit-window hearth-imagine 0 env $IENV HEARTH_ARRANGE_OPEN=imagine:window HEARTH_ARRANGE_PICK=results "$IM"
SHOT_WAIT=4 run imagine-5-edit-type hearth-imagine 0 env $IENV HEARTH_ARRANGE_OPEN=imagine:type HEARTH_ARRANGE_PICK=look "$IM"
kill $cm 2>/dev/null; wait $cm 2>/dev/null
# ComfyUI not there: what to do about it
SHOT_WAIT=4 run imagine-6-no-comfyui hearth-imagine 0 env $IENV "$IM"
grep -q '"steps": 28' "$IFX2/config/nimbus/imagine.json" || echo "::error title=imagine settings::opening Hearth Imagine changed imagine.json: $(cat "$IFX2/config/nimbus/imagine.json")"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
