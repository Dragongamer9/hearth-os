# Hearth Lights (hosts/hearth-ui/apps/lights; board docs/mockups/open-source-apps/09-lights.png, checklist
# docs/mockups/checklists/open-source-09-lights.md): the board's PC from a pretend OpenRGB SDK server
# (lights-fixture.py, protocol 3, answered as OpenRGB answers), so the app's own protocol code draws every device.
# 1 the board as it is (Ember wave on all 9, profile Gaming); 2 one part picked (the front fans) with Each light;
# 3 OpenRGB not on the PC (Get OpenRGB); 4 OpenRGB there but not running (Start OpenRGB); 5 Edit layout on the picture.
LFX="$ROOT/.github/scripts/hearth-apps-rust-shots.d/lights-fixture.py"
LLOG=/tmp/lights-sent.log; : > "$LLOG"
python3 "$LFX" 46742 "$LLOG" & LPID=$!
sleep 0.5
LCFG=/tmp/lights-cfg; rm -rf "$LCFG"; mkdir -p "$LCFG/hearth"
L="$BIN/hearth-lights-app"
SHOT_WAIT=3 run lights-1-board hearth-lights 0 env XDG_CONFIG_HOME="$LCFG" HEARTH_OPENRGB=127.0.0.1:46742 "$L"
# what the app sent the pretend server: Direct mode (1100) and every light (1050) for Ember wave
grep -q '^1050 ' "$LLOG" && echo "::notice title=lights::the lights were set through OpenRGB's protocol: $(cut -d' ' -f1 "$LLOG" | sort | uniq -c | tr '\n' ' ')" || echo "::error title=lights::nothing was sent to OpenRGB: $(head -c 300 "$LLOG" | tr '\n' ' ')"
[ -e "$LCFG/hearth/lights.json" ] && echo "::error title=lights::opening it wrote lights.json (red flag 9): $(head -c 300 "$LCFG/hearth/lights.json")"
printf '{"zoneMode": "light"}\n' > "$LCFG/hearth/lights.json"
SHOT_WAIT=3 run lights-2-picked hearth-lights 0 env XDG_CONFIG_HOME="$LCFG" HEARTH_OPENRGB=127.0.0.1:46742 "$L" --pick "Front fans@HID: /dev/hidraw3"
rm -f "$LCFG/hearth/lights.json"
run lights-3-not-installed hearth-lights 0 env XDG_CONFIG_HOME="$LCFG" HEARTH_OPENRGB=127.0.0.1:1 HEARTH_LIGHTS_INSTALLED=0 "$L"
run lights-4-not-running hearth-lights 0 env XDG_CONFIG_HOME="$LCFG" HEARTH_OPENRGB=127.0.0.1:1 HEARTH_LIGHTS_INSTALLED=1 "$L"
SHOT_WAIT=3 run lights-5-edit-layout hearth-lights 0 env XDG_CONFIG_HOME="$LCFG" HEARTH_OPENRGB=127.0.0.1:46742 HEARTH_ARRANGE_OPEN=lights:main HEARTH_ARRANGE_PICK=picture "$L"
kill $LPID 2>/dev/null
