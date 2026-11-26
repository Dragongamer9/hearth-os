# Hearth Studio (hosts/hearth-ui/apps/studio; board docs/mockups/open-source-apps/12-studio.png, checklist
# docs/mockups/checklists/open-source-12-studio.md): its pictures at the board's size (1920 x 1080, back to 1280 x 800
# after), in Hearth's own look, talking to a pretend OBS (oss-studio-obs.py: obs-websocket's real protocol, password
# and all, with the board's scenes, sources, mixer and outputs and a picture of the program). Its password is in the
# pretend home's OBS settings, as OBS keeps it. No real OBS, nothing goes live. Sourced by hearth-apps-rust-test.sh.
OH=/tmp/oss-studio-home
rm -rf "$OH"; mkdir -p "$OH/.config/obs-studio/plugin_config/obs-websocket" "$OH/.config/hearth" "$OH/Videos/Studio"
echo '{"server_enabled": true, "server_port": 4466, "auth_required": true, "server_password": "ci-pretend-pass"}' > "$OH/.config/obs-studio/plugin_config/obs-websocket/config.json"
echo '{"title": "Quad Mech devlog: legs that finally walk", "vertical": true, "upload": 40, "twitch": "pretend"}' > "$OH/.config/hearth/studio.json"
HOME="$OH" python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-studio-obs.py" 4466 ci-pretend-pass > /tmp/oss-studio-obs.log 2>&1 &
OBSPID=$!
sleep 2
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1
look hearth dark
mkdir -p "$OH/.local/share/nimbus" && cp "$HOME/.local/share/nimbus/desktop.json" "$OH/.local/share/nimbus/desktop.json"
ST="$BIN/hearth-studio-app"
studio() { # NAME ARGS...
  n=$1; shift
  SHOT_WAIT=5 run "oss-studio-$n" hearth-studio "${REST:-0}" env HOME="$OH" HEARTH_OBS_PORT=4466 HEARTH_STUDIO_VIEWERS=38 XDG_CONFIG_HOME="$OH/.config" "$@"
}
studio 1-board "$ST" --pick-item "Camera · round"
studio 2-mic-filters "$ST" --show filters
studio 3-add-to-the-scene "$ST" --show add
studio 4-where-you-go-live "$ST" --show where
studio 5-edit-layout env HEARTH_ARRANGE_OPEN=studio:window HEARTH_STUDIO_STAGE=studio:none "$ST"
studio 6-customize env HEARTH_ARRANGE_OPEN=studio:window HEARTH_STUDIO_STAGE=studio:customize "$ST"
# live with a pretend OBS: its CPU said, not limited (it's drawing OBS's meters and picture, as asked)
env HOME="$OH" HEARTH_OBS_PORT=4466 XDG_CONFIG_HOME="$OH/.config" "$ST" >/tmp/oss-studio-live.log 2>&1 &
LP=$!; sleep 6; a1=$(awk '{print $14 + $15}' /proc/$LP/stat 2>/dev/null || echo 0); sleep 10; a2=$(awk '{print $14 + $15}' /proc/$LP/stat 2>/dev/null || echo 0)
echo "::notice title=oss-studio live::connected to OBS with its meters and picture: $(python3 -c "print('%.1f' % (100.0 * ($a2 - $a1) / $(getconf CLK_TCK) / 10))")% of a core (software drawing, CI)"
kill $LP 2>/dev/null; wait $LP 2>/dev/null
kill $OBSPID 2>/dev/null
# without OBS: what Hearth Studio says and offers (here OBS "is installed", the pretend port is set, but nothing answers)
# at rest: OBS not answering (nothing live to show; the connection thread sleeps between tries). Live, it draws OBS's
# meters 8 times a second and its picture every other second, only while it's in front: that's measured apart below.
REST=20 studio 7-obs-not-answering "$ST"
grep -q "signed in" /tmp/oss-studio-obs.log && echo "::notice title=oss-studio::Hearth Studio signed in to the pretend OBS with its password ($(grep -c 'signed in' /tmp/oss-studio-obs.log) times)" || echo "::error title=oss-studio::Hearth Studio never signed in to the pretend OBS: $(tail -c 400 /tmp/oss-studio-obs.log | tr '\n' '|')"
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
look steam dark
