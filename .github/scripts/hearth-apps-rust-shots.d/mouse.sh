# Hearth Mouse (hosts/hearth-ui/apps/mouse; board docs/mockups/open-source-apps/11-mouse.png, checklist
# docs/mockups/checklists/open-source-11-mouse.md): the board's mouse as libratbag's ratbagd describes it on D-Bus
# (mouse-fixture.py), read by the app's own code (rb.rs). 1 the board (button 3 picked); 2 a button's choices open;
# 3 libratbag not on the PC; 4 Edit layout on the DPI steps.
# the boards are 1920 × 1080: the screen is that size for these pictures, and back to CI's own after
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1; sleep 0.5
MFX=/tmp/mouse-fx.json
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/mouse-fixture.py" "$MFX" || echo "::error title=mouse fixture::it couldn't be made"
M="$BIN/hearth-mouse-app"
MCFG=/tmp/mouse-cfg; rm -rf "$MCFG"; mkdir -p "$MCFG"
SHOT_WAIT=3 run mouse-1-board hearth-mouse 0 env XDG_CONFIG_HOME="$MCFG" HEARTH_MOUSE_FIXTURE="$MFX" "$M" --pin 3
run mouse-2-not-installed hearth-mouse 0 env XDG_CONFIG_HOME="$MCFG" HEARTH_MOUSE_FIXTURE=/nonexistent HEARTH_MOUSE_INSTALLED=0 "$M"
SHOT_WAIT=3 run mouse-3-edit-layout hearth-mouse 0 env XDG_CONFIG_HOME="$MCFG" HEARTH_MOUSE_FIXTURE="$MFX" HEARTH_ARRANGE_OPEN=mouse:main HEARTH_ARRANGE_PICK=dpi "$M" --pin 3
[ -n "$(ls -A "$MCFG" 2>/dev/null)" ] && echo "::error title=mouse::opening Hearth Mouse wrote settings (red flag 9): $(ls -R "$MCFG" | head -c 300)"
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1; sleep 0.3
