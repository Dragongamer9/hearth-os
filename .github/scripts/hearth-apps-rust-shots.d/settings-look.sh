# Hearth Settings' automatic look (the owner, Nov 19, on the Ally: "the settings keep switching between controller and
# mouse and keyboard mode"): the controller's own pointer (hearth-pad's, moved by the stick) and its clicks must not
# flip the controller look back. CI has no controller: --press stands in for one (a press every 250 ms for ~6 s)
# while wl-scroll.py moves the pointer like hearth-pad's does; Settings says each change of look on stderr. Then,
# with the controller idle, a pointer move is the mouse and goes back to the keyboard-and-mouse look.
lk_log=/tmp/settings-look.log
presses=$(for i in $(seq 1 12); do printf 'down up '; done)
"$BIN/hearth-settings-app" --page display --press "$presses" >"$lk_log" 2>&1 &
lk_pid=$!
for i in $(seq 1 400); do mapped hearth-settings && break; sleep 0.01; done
sleep 4.5
python3 "$ROOT/.github/scripts/wl-scroll.py" 2 move >/dev/null 2>&1   # the controller is still in use: no flip
grim "$ROOT/shots/settings-look-1-controller-pointer.png" 2>/dev/null || true
n1=$(grep -c 'look: keyboard and mouse' "$lk_log")
sleep 5                                                                     # the controller goes idle
python3 "$ROOT/.github/scripts/wl-scroll.py" 1 move >/dev/null 2>&1   # now a pointer move is the mouse
sleep 0.5
grim "$ROOT/shots/settings-look-2-mouse-again.png" 2>/dev/null || true
kill $lk_pid 2>/dev/null; wait $lk_pid 2>/dev/null
looks=$(grep 'look:' "$lk_log" | sed 's/.*look: //' | tr '\n' ',' )
# 1.18 (the owner, Nov 23: "get rid of the settings app controller mode as well, sitck with the normal settings app"):
# the controller moves through the normal look; nothing ever switches it to a controller look.
if grep -q 'look: controller' "$lk_log"; then
  echo "::error title=settings look::Settings switched to a controller look (it was removed in 1.18): $looks"
else
  echo "::notice title=settings look::controller presses and its pointer kept the normal look ($looks)"
fi
