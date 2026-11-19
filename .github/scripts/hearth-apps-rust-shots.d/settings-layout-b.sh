# Hearth Settings' two looks (the approved new Settings: settings-A1/A2 keyboard and mouse, settings-B1/B2 the
# controller look), pictured with --layout so they don't depend on a controller being there. CI has no displays,
# Wi-Fi, Bluetooth or PipeWire: small stand-ins for kscreen-doctor, nmcli, bluetoothctl, wpctl/pactl, brightnessctl,
# powerprofilesctl and the update check (only on PATH for these pictures) give the pages something real to show.
FK=/tmp/hs-fake
mkdir -p "$FK"
cat > "$FK/kscreen-doctor" <<'EOF'
#!/bin/sh
[ "$1" = "-j" ] || exit 0
cat <<'JSON'
{"outputs":[{"name":"DP-1","connected":true,"enabled":true,"currentModeId":"3","priority":1,"scale":1.25,"preferredModes":["3"],"modes":[{"id":"1","refreshRate":60.0,"size":{"width":1920,"height":1080}},{"id":"2","refreshRate":144.0,"size":{"width":2560,"height":1440}},{"id":"3","refreshRate":165.0,"size":{"width":2560,"height":1440}},{"id":"4","refreshRate":60.0,"size":{"width":2560,"height":1440}}]},{"name":"HDMI-A-1","connected":true,"enabled":true,"currentModeId":"5","priority":2,"scale":1.0,"preferredModes":["5"],"modes":[{"id":"5","refreshRate":60.0,"size":{"width":1920,"height":1080}}]}]}
JSON
EOF
cat > "$FK/brightnessctl" <<'EOF'
#!/bin/sh
case "$*" in *-m*) echo "intel_backlight,backlight,5600,70%,8000" ;; esac
EOF
cat > "$FK/kreadconfig6" <<'EOF'
#!/bin/sh
echo "${HS_NIGHT:-false}"
EOF
cat > "$FK/nmcli" <<'EOF'
#!/bin/sh
case "$*" in
  "-t radio wifi") echo enabled ;;
  "-t -f TYPE,STATE,CONNECTION dev") echo "wifi:connected:Home-5G" ;;
  *"dev wifi list"*) printf '*:Home-5G:82:WPA2\n :Cafe:40:\n' ;;
  *"con show"*) echo "Home-5G:802-11-wireless" ;;
esac
EOF
cat > "$FK/bluetoothctl" <<'EOF'
#!/bin/sh
case "$*" in
  *show*) printf 'Controller 11:22:33:44:55:66 (public)\n\tPowered: yes\n' ;;
  *devices*) printf 'Device AA:BB:CC:DD:EE:01 Xbox controller\nDevice AA:BB:CC:DD:EE:02 AirPods\n' ;;
esac
EOF
cat > "$FK/wpctl" <<'EOF'
#!/bin/sh
case "$*" in
  *SINK*) echo "Volume: 0.45" ;;
  *SOURCE*) echo "Volume: 0.80" ;;
esac
EOF
cat > "$FK/pactl" <<'EOF'
#!/bin/sh
case "$*" in
  get-default-sink) echo alsa_output.speakers ;;
  get-default-source) echo alsa_input.mic ;;
  *"list sinks"*) echo '[{"name":"alsa_output.speakers","description":"Speakers"}]' ;;
  *"list sources"*) echo '[{"name":"alsa_input.mic","description":"Microphone"}]' ;;
esac
EOF
cat > "$FK/powerprofilesctl" <<'EOF'
#!/bin/sh
case "$1" in
  get) echo balanced ;;
  list) printf '  performance:\n    CpuDriver:\tintel_pstate\n\n* balanced:\n    CpuDriver:\tintel_pstate\n\n  power-saver:\n' ;;
esac
EOF
cat > "$FK/pkexec" <<'EOF'
#!/bin/sh
[ "$2" = update-status ] && { echo '{"booted":{"version":"1.13.0"}}'; exit 0; }
exit 1
EOF
cat > "$FK/home-server.py" <<'EOF'
import json, sys
if sys.argv[1:2] == ["status"]:
    print(json.dumps({"settings": {"lightMode": True}}))
EOF
chmod 755 "$FK"/*
# A change in the history (Night light, turned on by you 8 days ago), for the help panel's "Changed by you".
mkdir -p /tmp/hs-state/nimbus
printf '{"changes": [{"id": 1, "file": "desktop.json", "key": "nightLight", "from": false, "to": true, "who": "you", "at": %s}], "next": 2}\n' "$(( $(date +%s) - 8 * 86400 ))" > /tmp/hs-state/nimbus/settings-history.json
SET="env PATH=$FK:$PATH HEARTH_HOME_SERVER=$FK/home-server.py $BIN/hearth-settings-app"

# B1: Quick, the focused tile lifted (measured at rest: the controller is read by a thread that sleeps in the kernel).
run settings-b1-quick hearth-settings 20 $SET --layout b --page quick --focus "Light mode while you play"
# B2: System › Display, big rows (slider with its value, ‹ value ›, switches), the help panel for Resolution.
run settings-b2-display hearth-settings 0 $SET --layout b --page display --focus Resolution
# B: Sound (two sliders with their values), the list on the left focused (its help: the page itself).
run settings-b3-sound hearth-settings 0 $SET --layout b --page sound --focus Volume
run settings-b4-list hearth-settings 0 $SET --layout b --page display
# B: X Pin to Quick, and Y search.
run settings-b5-pin hearth-settings 0 $SET --layout b --page quick:pin --focus Wi-Fi
run settings-b6-search hearth-settings 0 $SET --layout b --page display --find night
# A with the keyboard's focus on a row (settings-A2's outlined Resolution), and Home with its tiles.
run settings-a2-display-focus hearth-settings 0 $SET --layout a --page display --focus Resolution
run settings-a1-home-tiles hearth-settings 0 $SET --layout a --page home
# The controller really moving and changing things (--press: presses as if read from the controller, one at a time).
# Quick: right, right (Wi-Fi, Bluetooth), down (Brightness), right, right (70 % to 80 %).
run settings-b7-quick-moved hearth-settings 0 $SET --layout b --page quick --press "right right down right right"
# The help panel with the change history: Night light, changed by you.
run settings-b8-history hearth-settings 0 env XDG_STATE_HOME=/tmp/hs-state $SET --layout b --page display --focus "Night light"
# X Default on a setting that isn't at its default (Night light on): asked first.
run settings-b9-default hearth-settings 0 env HS_NIGHT=true $SET --layout b --page display --focus "Night light" --press x
# The keyboard-and-mouse look moved with the arrows / D-pad: into the page, down twice (Scale outlined).
run settings-a3-display-moved hearth-settings 0 $SET --layout a --page display --press "right down down"
