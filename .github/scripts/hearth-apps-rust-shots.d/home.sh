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
grep -q '"room": "living_room"' "$HFX/config/nimbus/home.json" || echo "::error title=home settings::opening Hearth Home changed home.json"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
