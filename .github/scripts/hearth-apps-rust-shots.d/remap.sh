# Hearth Remap (hosts/hearth-ui/apps/remap; board docs/mockups/open-source-apps/10-remap.png, checklist
# docs/mockups/checklists/open-source-10-remap.md): the board's devices and profiles as Input Remapper keeps them (its
# own preset files, made by remap-fixture.py in input-remapper's format) and Hearth's remap.json, read by the app's own
# code (ir.rs). 1 the board (Battle for Middle-earth II on, its macro); 2 a change being edited; 3 a macro step picked;
# 4 Input Remapper off (Turn it on); 5 not on the PC; 6 Edit layout on the controller.
# the boards are 1920 × 1080: the screen is that size for these pictures, and back to CI's own after
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1; sleep 0.5
RCFG=/tmp/remap-cfg; RFXF=/tmp/remap-fx.json
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/remap-fixture.py" "$RCFG" "$RFXF" || echo "::error title=remap fixture::it couldn't be made"
R="$BIN/hearth-remap-app"
RON="Battle for Middle-earth II"
SHOT_WAIT=3 run remap-1-board hearth-remap 0 env XDG_CONFIG_HOME="$RCFG" HEARTH_REMAP_FIXTURE="$RFXF" HEARTH_REMAP_ON="$RON" "$R" --profile "$RON"
run remap-2-everywhere hearth-remap 0 env XDG_CONFIG_HOME="$RCFG" HEARTH_REMAP_FIXTURE="$RFXF" HEARTH_REMAP_ON="$RON" "$R" --profile Everywhere
run remap-3-off hearth-remap 0 env XDG_CONFIG_HOME="$RCFG" HEARTH_REMAP_FIXTURE="$RFXF" HEARTH_REMAP_RUNNING=0 "$R" --profile "$RON"
run remap-4-not-installed hearth-remap 0 env XDG_CONFIG_HOME="$RCFG" HEARTH_REMAP_FIXTURE="$RFXF" HEARTH_REMAP_INSTALLED=0 "$R" --profile "$RON"
SHOT_WAIT=3 run remap-5-edit-layout hearth-remap 0 env XDG_CONFIG_HOME="$RCFG" HEARTH_REMAP_FIXTURE="$RFXF" HEARTH_REMAP_ON="$RON" HEARTH_ARRANGE_OPEN=remap:main HEARTH_ARRANGE_PICK=picture "$R" --profile "$RON"
# opening it wrote nothing (red flag 9): the fixture's files are as they were made
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/remap-fixture.py" --check "$RCFG" || echo "::error title=remap::opening Hearth Remap changed its files"
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1; sleep 0.3
