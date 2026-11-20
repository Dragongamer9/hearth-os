#!/bin/bash
# Hearth Player with the fixture discs (player.yml, inside a D-Bus session on a headless screen): each disc's screen as
# a picture, and the DVD, the Video CD and the music CD played for real (WebKitGTK and GStreamer, the position read
# over MPRIS), a DVD seek, the locked DVD's note and its "Add DVD decoding" question (not answered), the Blu-ray
# note, the controller buttons, a data disc's folder, and the player's CPU at rest on the disc screen.
#   player-disc-drive.sh FIXTURES SHOTS BINARY      (results: RESULT lines; exit 1 when a disc didn't play)
d="$1"; shots="$2"; bin="$3"
mkdir -p "$shots"
pulseaudio --start --exit-idle-time=-1 --daemonize=yes 2>/dev/null || true
fails=0
shot() { sleep "${2:-1}"; import -window root "$shots/$1.png" 2>/dev/null || echo "no picture $1"; }
front() { local w; w=$(xdotool search --name 'Hearth Player' 2>/dev/null | tail -1); [ -n "$w" ] && xdotool windowactivate --sync "$w" 2>/dev/null; true; }
start() {
  tag="$1"; shift
  "$bin" "$@" > "run-$tag.log" 2>&1 &
  pid=$!
  name=""
  for i in $(seq 40); do name=$(busctl --user list --no-legend 2>/dev/null | awk '{print $1}' | grep -m1 '^org.mpris.MediaPlayer2.HearthPlayer'); [ -n "$name" ] && break; sleep 1; done
  sleep 4; front; sleep 1
}
stop() { kill "$pid" 2>/dev/null; sleep 1; kill -9 "$pid" 2>/dev/null; sleep 1; }
pos() { busctl --user get-property "$name" /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player Position 2>/dev/null | awk '{print $2}'; }
wait_pos() { local p; for i in $(seq 45); do p=$(pos); [ "${p:-0}" -gt "$1" ] 2>/dev/null && { echo "$p"; return 0; }; sleep 1; done; echo "${p:-0}"; return 1; }
result() { echo "RESULT $1: $2"; }
key() { front; xdotool key "$@"; }

# ---- DVD: its own menu first (libdvdnav: move, press "Play the film"), then playing, a seek, sound tracks, the
# titles list over the film; then the titles list on its own (B from the menu).
cpu() { local s=0 x; for x in $pid $(pgrep -P "$pid"); do s=$((s + $(awk '{print $14 + $15}' "/proc/$x/stat" 2>/dev/null || echo 0))); done; echo $s; }
rest() { local a b; sleep 3; a=$(cpu); sleep 10; b=$(cpu); awk -v a="$a" -v b="$b" -v hz="$(getconf CLK_TCK)" 'BEGIN { printf "%.2f %% of a core", (b - a) / hz / 10 * 100 }'; }
if [ -f "$d/hearth-test.iso" ]; then
  start dvd --disc "$d/hearth-test.iso"
  shot disc-dvd-own-menu 4
  result "CPU at rest (the DVD's own menu, a still picture, 10 s)" "$(rest)"
  # The menu's sound: its own ffmpeg (Opus in Ogg) while the menu is up, gone once the film starts.
  if pgrep -f 'ogg -flush_packets' >/dev/null; then result "DVD menu sound" "playing while the menu is up"; else result "DVD menu sound" "NO sound stream with the menu up"; fails=$((fails + 1)); fi
  key Down; shot disc-dvd-own-menu-down 2
  key Up; sleep 1
  key Return
  sleep 4
  if pgrep -f 'ogg -flush_packets' >/dev/null; then result "DVD menu sound after the film started" "STILL RUNNING"; fails=$((fails + 1)); else result "DVD menu sound after the film started" "stopped"; fi
  if p=$(wait_pos 2000000); then result "DVD menu: Play the film" "plays (at ${p} µs)"; else result "DVD menu: Play the film" "NO (position $p)"; fails=$((fails + 1)); tail -30 run-dvd.log; fi
  shot disc-dvd-playing 2
  key l
  # Past 14.5 s means the stream started again near 13 s and played on (the time shown while it starts doesn't count).
  if p=$(wait_pos 14500000); then result "DVD seek" "yes (to ${p} µs)"; else result "DVD seek" "NO (position $p)"; fails=$((fails + 1)); fi
  key a; shot disc-dvd-sound 2
  key Escape; sleep 1
  key d; shot disc-dvd-menu 2
  stop
  start dvd2 --disc "$d/hearth-test.iso"
  sleep 2; key Escape; shot disc-dvd 2
  stop
fi

# ---- The locked DVD: the note, then "Add DVD decoding" asks first (the demo stands in for the real install).
if [ -f "$d/hearth-locked.iso" ]; then
  export HEARTH_PLAYER_DVD_DECODING_DEMO=1
  start locked --disc "$d/hearth-locked.iso"
  shot disc-locked 2
  key Return; shot disc-locked-confirm 2
  key Escape; sleep 1
  stop
  unset HEARTH_PLAYER_DVD_DECODING_DEMO
fi

# ---- Music CD: its songs (CD-Text names), then playing.
if [ -f "$d/cd.cue" ]; then
  start cd --disc "$d/cd.cue"
  shot disc-audio 2
  key Return
  if p=$(wait_pos 2000000); then result "Music CD plays" "yes (at ${p} µs)"; else result "Music CD plays" "NO (position $p)"; fails=$((fails + 1)); tail -30 run-cd.log; fi
  shot disc-audio-playing 1
  stop
fi

# ---- Video CD.
if [ -f "$d/vcd.cue" ]; then
  start vcd --disc "$d/vcd.cue"
  shot disc-vcd 1
  key Return
  if p=$(wait_pos 2000000); then result "Video CD plays" "yes (at ${p} µs)"; else result "Video CD plays" "NO (position $p)"; fails=$((fails + 1)); tail -30 run-vcd.log; fi
  shot disc-vcd-playing 2
  stop
fi

# ---- Blu-ray: the note; and the CPU at rest on a disc screen (nothing playing: the disc watcher sleeps on udev).
if [ -d "$d/bd" ]; then
  start bd --disc "$d/bd"
  shot disc-bluray 2
  sleep 3
  # The player and its web view's processes (its children).
  t() { local s=0 x; for x in $pid $(pgrep -P "$pid"); do s=$((s + $(awk '{print $14 + $15}' "/proc/$x/stat" 2>/dev/null || echo 0))); done; echo $s; }
  a=$(t); sleep 10; b=$(t)
  hz=$(getconf CLK_TCK)
  result "CPU at rest (disc screen, 10 s)" "$(awk -v a="$a" -v b="$b" -v hz="$hz" 'BEGIN { printf "%.2f %% of a core", (b - a) / hz / 10 * 100 }')"
  stop
fi

# ---- For comparison: the player's own start screen at rest, with no disc (what it costs without the disc part).
start empty
sleep 3
t() { local s=0 x; for x in $pid $(pgrep -P "$pid"); do s=$((s + $(awk '{print $14 + $15}' "/proc/$x/stat" 2>/dev/null || echo 0))); done; echo $s; }
a=$(t); sleep 10; b=$(t)
result "CPU at rest (start screen, no disc, 10 s)" "$(awk -v a="$a" -v b="$b" -v hz="$(getconf CLK_TCK)" 'BEGIN { printf "%.2f %% of a core", (b - a) / hz / 10 * 100 }')"
stop

# ---- The controller and remote buttons.
if [ -f "$d/hearth-test.iso" ]; then
  start pad --disc "$d/hearth-test.iso" --show controller
  shot disc-controller 2
  stop
fi

# ---- A data disc: its folder opens.
if [ -d "$d/files" ]; then
  start data --disc "$d/files"
  shot disc-data 2
  stop
fi

ls "$shots"
exit $(( fails > 0 ))
