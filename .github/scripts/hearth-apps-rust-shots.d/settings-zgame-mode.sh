# Game Mode on this PC (Settings › System › Startup; cats/system.rs startup, cats/home.rs Start in; the owner, Nov 19):
# the switch On and Off, Home's Start in with it off, and the Guide menu (hearth-pad-host) with its Game Mode slot
# empty. After settings-system.sh (its examples folder, on "KDE Plasma"). desktop.json holds the setting
# (gameModeOff); opening the pages writes nothing (checked below).
S="$BIN/hearth-settings-app"
[ -d /tmp/settings-system-fixture ] && export HEARTH_SYSTEM_FIXTURE=/tmp/settings-system-fixture
export XDG_CURRENT_DESKTOP=KDE
D="$HOME/.local/share/nimbus/desktop.json"
printf '{"look": "hearth", "mode": "dark"}\n' > "$D"
run gm-1-startup-on hearth-settings 0 "$S" --page startup
printf '{"look": "hearth", "mode": "dark", "gameModeOff": true}\n' > "$D"
cp "$D" /tmp/gm-desktop.before
run gm-2-startup-off hearth-settings 0 "$S" --page startup
[ -f "$ROOT/.github/scripts/hearth-apps-rust-shots.d/settings-hs-home.json" ] && export HEARTH_SETTINGS_FIXTURES="$ROOT/.github/scripts/hearth-apps-rust-shots.d/settings-hs-home.json"
run gm-3-home-off hearth-settings 0 "$S"
unset HEARTH_SETTINGS_FIXTURES HEARTH_SYSTEM_FIXTURE XDG_CURRENT_DESKTOP
cmp -s "$D" /tmp/gm-desktop.before || echo "::error title=game mode switch::opening Startup or Home changed desktop.json: $(cat "$D")"
# The Guide menu with Game Mode off: its slot empty (the label says so when it's picked).
if command -v padshot >/dev/null 2>&1 && [ -x "$BIN/hearth-pad-host" ]; then
  padshot gm-4-guide-menu-off --show menu
fi
printf '{"look": "hearth", "mode": "dark"}\n' > "$D"
rm -f /tmp/gm-desktop.before
