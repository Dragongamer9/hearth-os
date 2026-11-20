# System › Couch, Desk and Away (Firehold on Plasma, mockup firehold-plasma/05), in the controller look as the mockup
# draws it, and its While Away page. A stand-in hearth-firehold-plasma answers `get` and `modes` with the mockup's two
# screens (the TV in Couch, the monitor in Desk); nothing is written.
FHP_STUB=$(mktemp -d)
cat > "$FHP_STUB/hearth-firehold-plasma" <<'SH'
#!/bin/sh
case "$1" in
  get) echo '{"awayAfter": 20, "autoPad": true, "autoPadScope": "tv", "autoMouse": true, "games": "follow"}' ;;
  modes) echo '{"screens": [{"name": "HDMI-A-2", "tv": true, "label": "TCL 55″ TV", "spec": "3840 × 2160 · 60 Hz · HDMI 2 on the RTX 5070", "mode": "couch", "why": "pad"}, {"name": "DP-1", "tv": false, "label": "LG 27GP850", "spec": "2560 × 1440 · 165 Hz · DisplayPort 1", "mode": "desk", "why": "mouse"}], "autoPad": true, "autoMouse": true, "awayAfter": 20}' ;;
  *) exit 0 ;;
esac
SH
chmod +x "$FHP_STUB/hearth-firehold-plasma"
run settings-couch-1-modes hearth-settings 0 env PATH="$FHP_STUB:$PATH" "$BIN/hearth-settings-app" --page couch-desk-away --layout b
run settings-couch-3-starts-in hearth-settings 0 env PATH="$FHP_STUB:$PATH" "$BIN/hearth-settings-app" --page couch-desk-away --layout b --focus "Sound in Couch"
run settings-couch-4-switching hearth-settings 0 env PATH="$FHP_STUB:$PATH" "$BIN/hearth-settings-app" --page couch-desk-away --layout b --focus "Mouse or keyboard → Desk"
run settings-couch-5-games hearth-settings 0 env PATH="$FHP_STUB:$PATH" "$BIN/hearth-settings-app" --page couch-desk-away --layout b --focus "Games when you switch"
run settings-couch-2-away hearth-settings 0 env PATH="$FHP_STUB:$PATH" "$BIN/hearth-settings-app" --page couch-away --layout b
rm -rf "$FHP_STUB"
