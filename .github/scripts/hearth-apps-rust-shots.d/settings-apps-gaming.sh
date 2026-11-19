# Hearth Settings › Apps and Gaming (hosts/hearth-ui/apps/settings/src/cats/apps*, gaming*): every page as CI has it
# (no Flatpak, InputPlumber, HHD or Steam here: the pages say so), then a few with $HEARTH_SETTINGS_FIXTURES answers
# (cats/apps/kit.rs fixture()) to picture them as they look on a device. Sourced by hearth-apps-rust-test.sh.
SAG_LIBEXEC="$ROOT/os/system_files/usr/libexec/nimbus"
SAG_MOVIES=/tmp/sag-movies
mkdir -p "$SAG_MOVIES"
for m in flame sweep reveal embers aurora starlight orbit; do : > "$SAG_MOVIES/hearth-$m-16x9.webm"; done
sag() { # NAME REST PAGE [ENV=...]
  n=$1; rest=$2; pg=$3; shift 3
  run "settings-$n" hearth-settings "$rest" env HEARTH_LIBEXEC_DIR="$SAG_LIBEXEC" HEARTH_MOVIES="$SAG_MOVIES" "$@" "$BIN/hearth-settings-app" --page "$pg"
}
sag_sum() { cat "$HOME/.config/nimbus/session.conf" "$HOME/.config/nimbus/store.json" "$HOME/.config/mimeapps.list" 2>/dev/null | md5sum; }
look steam dark
SAG_BEFORE=$(sag_sum)
sag apps-01-installed 0 apps
sag apps-02-store 0 store
sag apps-03-store-updates 0 store:updates
sag apps-04-store-sources 0 store:sources
sag apps-05-store-drivers 0 store:drivers
sag apps-06-winapps 0 winapps
sag apps-07-defaults 0 defaults
sag apps-08-file-types 0 file-types
sag apps-09-autostart 0 autostart
sag apps-10-menu-editor 0 menu-editor
sag apps-11-permissions 0 app-permissions
sag apps-12-services 0 services
sag apps-13-window-rules 0 kwin-rules
sag apps-14-browser 0 browser
sag apps-15-claude 0 claude
sag gaming-01-gaming 20 gaming
sag gaming-02-saved-games 0 savedgames
sag gaming-03-performance 0 gameperf
sag gaming-04-steam 0 steam
sag gaming-05-startup-animation 0 startup-animation
sag gaming-06-animation-preview 0 startup-animation:preview-aurora
sag gaming-07-games 0 games
sag gaming-08-discord 0 discord
sag gaming-09-remote 0 remote
sag gaming-10-controller 0 controller
sag gaming-11-kwin-games 0 kwin-games
sag gaming-12-handheld 0 handheld

# ---- as on a device: answers a real PC gives (an ROG Ally with HHD, a Windows app, Sunshine on), from files
FX=/tmp/sag-fixtures
mkdir -p "$FX"
cat > "$FX/hearth-hhd-page.json" <<'JSON'
{"running": true, "version": "3.17.2", "controllers": ["ROG Ally X"], "sections": [
 {"title": "Controller", "steamNote": "In Steam Game Mode, Steam's own controller settings come first.", "items": [
  {"key": "controllers.rog_ally.controller_mode.mode", "kind": "choice", "title": "Controller emulation", "hint": "What games see", "value": "dualsense",
   "options": [{"id": "uinput", "label": "Xbox"}, {"id": "dualsense", "label": "DualSense"}, {"id": "hori_steam", "label": "Steam"}], "depth": 0},
  {"key": "controllers.rog_ally.gyro", "kind": "bool", "title": "Gyro", "hint": "Motion controls in games that use them", "value": true, "depth": 1},
  {"key": "controllers.rog_ally.swap_legion", "kind": "bool", "title": "Swap the back buttons", "hint": "", "value": false, "depth": 1},
  {"key": "controllers.rog_ally.vibration", "kind": "choice", "title": "Rumble", "hint": "", "value": "medium",
   "options": [{"id": "off", "label": "Off"}, {"id": "low", "label": "Low"}, {"id": "medium", "label": "Medium"}, {"id": "high", "label": "High"}, {"id": "max", "label": "Strongest"}], "depth": 1}]},
 {"title": "Power", "steamNote": "Steam's Performance panel sets the power limit while Steam Game Mode runs.", "items": [
  {"key": "tdp.unified.tdp.custom.tdp", "kind": "number", "title": "Power limit", "hint": "", "value": 17, "min": 7, "max": 30, "step": 1, "int": true, "unit": "W", "depth": 0},
  {"key": "tdp.unified.fan.mode", "kind": "choice", "title": "Fan", "hint": "", "value": "auto", "options": [{"id": "auto", "label": "Automatic"}, {"id": "manual", "label": "Your curve"}], "depth": 0},
  {"key": "tdp.unified.charge_limit", "kind": "bool", "title": "Charge to 80%", "hint": "Kinder to the battery when it's plugged in all day", "value": true, "depth": 0, "advanced": true}]},
 {"title": "Lights", "steamNote": "", "items": [
  {"key": "rgb.unified.mode", "kind": "choice", "title": "Lights", "hint": "", "value": "solid", "options": [{"id": "off", "label": "Off"}, {"id": "solid", "label": "Solid"}, {"id": "pulse", "label": "Pulse"}], "depth": 0},
  {"key": "rgb.unified.color", "kind": "color", "title": "Colour", "hint": "", "value": "#e8743b", "depth": 1}]}]}
JSON
cat > "$FX/hearth-gamemode-display-get.json" <<'JSON'
{"w": 1920, "h": 1080, "r": 120, "limit": 60, "hdr": false, "args": "-W 1920 -H 1080 -r 120 --framerate-limit 60",
 "screen": {"name": "eDP-1", "model": "ROG Ally X screen", "modes": [{"w": 1920, "h": 1080, "rates": [120, 60]}, {"w": 1600, "h": 900, "rates": [60]}, {"w": 1280, "h": 720, "rates": [120, 60]}], "hdr": false},
 "has": {"limit": true, "hdr": true}}
JSON
cat > "$FX/hearth-winapps-list.json" <<'JSON'
{"builtin": "GE-Proton10-25", "valve": "Proton 10.0", "apps": [
 {"id": "nexus-mods", "name": "Vortex Mod Manager", "exe": "Vortex.exe", "args": [], "proton": "", "steam": true, "dxvk": true, "vkd3d": true, "winver": "",
  "dir": "/home/ci/Games/Windows apps/Vortex", "log": "/home/ci/Games/Windows apps/Vortex/logs/last.log", "detailedNext": false, "detailedLog": "",
  "candidates": [{"name": "GE-Proton10-25"}, {"name": "GE-Proton9-27", "download": true}], "dlls": [], "fixNames": {"corefonts": "Windows fonts", "vcrun": "Visual C++ runtime"}, "fixes": ["vcrun"],
  "last": {"at": 1763400000, "what": "started", "proton": "GE-Proton10-25", "seconds": 812.4, "code": 0}},
 {"id": "old-game", "name": "Old Game Installer", "exe": "setup.exe", "args": [], "proton": "", "steam": false, "dxvk": true, "vkd3d": true, "winver": "winxp",
  "dir": "/home/ci/Games/Windows apps/Old Game", "log": "", "candidates": [], "dlls": [], "fixNames": {}, "fixes": [],
  "last": {"at": 1763390000, "what": "started", "proton": "GE-Proton10-25", "seconds": 1.2, "code": 1, "crashed": true, "next": {"name": "GE-Proton9-27", "download": true}}}]}
JSON
cat > "$FX/hearth-controller-status.json" <<'JSON'
{"available": true, "backend": "hhd", "devices": [{"name": "ROG Ally X controller"}], "config": {"target": "", "map": {"left_paddle": "a", "right_paddle": "key_f12"}},
 "types": [{"id": "", "label": "Automatic", "sub": "What the controller is"}, {"id": "xbox-series", "label": "Xbox Series", "sub": ""}, {"id": "ds5", "label": "DualSense", "sub": ""}],
 "buttons": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}, {"id": "x", "label": "X"}, {"id": "y", "label": "Y"}, {"id": "left_paddle", "label": "Left back button"}, {"id": "right_paddle", "label": "Right back button"}],
 "targets": [{"id": "a", "label": "A", "kind": "button"}, {"id": "b", "label": "B", "kind": "button"}, {"id": "key_f12", "label": "F12", "kind": "key"}]}
JSON
cat > "$FX/hearth-saves-status.json" <<'JSON'
{"ludusavi": true, "folder": "~/Saved Games/Backups", "last": "Today, 2:14 AM", "games": [
 {"name": "Elden Ring", "bytes": 31457280, "files": 4, "backedUp": true}, {"name": "Hades II", "bytes": 2097152, "files": 12, "backedUp": false},
 {"name": "Stardew Valley", "bytes": 524288, "files": 6, "backedUp": true, "here": false}]}
JSON
cat > "$FX/hearth-remote-desktop-status.json" <<JSON
{"installed": true, "on": true, "running": true, "reach": "tailscale", "keepAwake": true, "tailscale": {"running": true},
 "address": {"name": "hearth-pc", "ip": "100.64.12.7"}, "clients": [{"name": "Work laptop", "uuid": "1"}],
 "stream": {"on": true, "who": "Work laptop", "width": 1920, "height": 1080, "fps": 60, "mbps": 20, "path": "direct", "since": $(( $(date +%s) - 2700 )), "peerIp": "100.64.12.9"}, "awake": true}
JSON
cat > "$FX/nimbus---settings-call-remote.status.json" <<'JSON'
{"ok": true, "value": {"installed": true, "on": true, "clients": [], "needsLogin": false, "addresses": ["192.168.1.20"], "moonlight": false}}
JSON
cat > "$FX/nimbus---settings-call-steamMode.status.json" <<'JSON'
{"ok": true, "value": {"available": true, "steam": true, "gamescope": true, "decky": {"installed": true, "via": "hearth", "plugin": true, "shipped": true}, "lastFailure": ""}}
JSON
chmod -R a+rX "$FX"
sag gaming-21-handheld-ally 0 handheld HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-22-performance-ally 0 gameperf HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-23-controller-ally 0 controller HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-24-saved-games-device 0 savedgames HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-25-remote-on 0 remote HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-26-steam-decky 0 steam HEARTH_SETTINGS_FIXTURES="$FX"
sag gaming-27-gaming-device 0 gaming HEARTH_SETTINGS_FIXTURES="$FX"
sag apps-21-winapps-device 0 winapps HEARTH_SETTINGS_FIXTURES="$FX"
sag apps-22-winapps-app 0 winapps:old-game HEARTH_SETTINGS_FIXTURES="$FX"
cat > "$FX/hearth-kwin-settings-status.json" <<'JSON'
{"live": true, "running": true, "version": "6.5.3", "effects": [], "defaults": {}}
JSON
cat > "$FX/hearth-kwin-settings-rules.json" <<'JSON'
{"rules": [{"id": "a1b2", "values": {"Description": "Steam windows", "wmclass": "steam", "wmclassmatch": "1", "above": "true", "aboverule": "2",
  "size": "1280,800", "sizerule": "3", "tearing": "true", "tearingrule": "2", "opacityinactive": "85", "opacityinactiverule": "2"}, "other": []},
 {"id": "c3d4", "values": {"Description": "Firefox on desktop 2", "wmclass": "firefox", "wmclassmatch": "1", "desktops": "d2", "desktopsrule": "3"}, "other": ["placementrule"]}]}
JSON
cat > "$FX/hearth-kwin-settings-desktops.json" <<'JSON'
{"desktops": [{"id": "d1", "name": "Desktop 1"}, {"id": "d2", "name": "Desktop 2"}]}
JSON
sag apps-23-window-rules-open 0 kwin-rules:steam HEARTH_SETTINGS_FIXTURES="$FX"
# Opening these pages wrote nothing of Hearth's own settings (red flag 9).
grep -q '"look": "steam"' "$HOME/.local/share/nimbus/desktop.json" || echo "::error title=settings-apps-gaming::desktop.json changed: $(cat "$HOME/.local/share/nimbus/desktop.json")"
[ "$(sag_sum)" = "$SAG_BEFORE" ] || echo "::error title=settings-apps-gaming::opening a page changed session.conf, store.json or mimeapps.list"
