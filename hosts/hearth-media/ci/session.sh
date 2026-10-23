#!/bin/bash
# For CI (.github/workflows/capture-apps.yml): starts what a Hearth session has, inside a container, then runs
# "$@" in it:
#  - a D-Bus session, PipeWire with WirePlumber and its PulseAudio side
#  - "speakers" (the default output) playing a 440 Hz tone, and a fake microphone (the default input) hearing 880 Hz
#  - a screen: labwc headless with something moving on it (KIND=wayland, like Mantel), or Xvfb (KIND=x11)
# Run it under dbus-run-session.
set -u
KIND=${KIND:-x11}
export XDG_RUNTIME_DIR=/tmp/xdg-$(id -u)
mkdir -p -m 700 "$XDG_RUNTIME_DIR"
L=${LOGS:-/tmp/session-logs}; mkdir -p "$L"

pipewire >"$L/pipewire.log" 2>&1 &
sleep 1
wireplumber >"$L/wireplumber.log" 2>&1 &
sleep 1
pipewire-pulse >"$L/pipewire-pulse.log" 2>&1 &
for _ in $(seq 40); do pactl info >/dev/null 2>&1 && break; sleep 0.5; done
pactl info | grep -E 'Server Name|Default' || { echo "::error::PipeWire's PulseAudio side didn't start"; tail -20 "$L"/*.log; }
pactl load-module module-null-sink sink_name=speakers sink_properties=device.description=Speakers >/dev/null
pactl load-module module-null-sink sink_name=micfeed sink_properties=device.description=Micfeed >/dev/null
pactl load-module module-remap-source master=micfeed.monitor source_name=fakemic source_properties=device.description=Fake-microphone >/dev/null
sleep 1
pactl set-default-sink speakers
pactl set-default-source fakemic
echo "default sink: $(pactl get-default-sink), default source: $(pactl get-default-source)"
ffmpeg -nostdin -loglevel error -re -f lavfi -i "sine=frequency=440:sample_rate=48000" -f pulse -device speakers "tone" >"$L/tone.log" 2>&1 &
ffmpeg -nostdin -loglevel error -re -f lavfi -i "sine=frequency=880:sample_rate=48000" -f pulse -device micfeed "voice" >"$L/voice.log" 2>&1 &

if [ "$KIND" = wayland ]; then
  mkdir -p /tmp/labwc-cfg
  WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 WLR_HEADLESS_OUTPUTS=1 labwc -C /tmp/labwc-cfg >"$L/labwc.log" 2>&1 &
  for _ in $(seq 40); do [ -S "$XDG_RUNTIME_DIR/wayland-0" ] && break; sleep 0.25; done
  export WAYLAND_DISPLAY=wayland-0
  unset DISPLAY
  command -v wlr-randr >/dev/null && wlr-randr --output HEADLESS-1 --custom-mode 1280x720 2>/dev/null
  # Something that changes, so the screen has new pictures to capture.
  foot -e top -d 0.3 >"$L/foot.log" 2>&1 &
else
  Xvfb :99 -screen 0 1280x720x24 >"$L/xvfb.log" 2>&1 &
  export DISPLAY=:99
  unset WAYLAND_DISPLAY
  sleep 2
  command -v xterm >/dev/null && xterm -geometry 120x40+0+0 -e top -d 0.3 >/dev/null 2>&1 &
fi
sleep 2
"$@"
