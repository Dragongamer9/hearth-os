# The open-source apps for the Hearth Store (docs/mockups/open-source-apps; checklists
# docs/mockups/checklists/open-source-NN-*.md): Hearth Game Fixer (Lutris), each picture next to its board. Each app on a
# pretend world of its own (oss-a-fixture.py: a home folder with what the project keeps there, and stand-ins for its
# commands on PATH), so it shows the board through its real code. Then the app in Edit layout (its parts outlined,
# its own blocks window) and a part's Customize. Sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its.
OA=/tmp/oss-a
rm -rf "$OA"; mkdir -p "$OA"
oss_world() { # APP: its home and stand-ins
  OB=$(python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-a-fixture.py" "$OA" "$1" | tail -1) || echo "::error title=oss-a fixture::$1's world couldn't be made"
  OH="$OA/$1/home"
}
oss() { # NAME APP_ID COMMAND…: pictured in that world, Hearth's own dark look
  local n=$1 id=$2
  shift 2
  run "$n" "$id" 0 env HOME="$OH" XDG_CONFIG_HOME="$OH/.config" XDG_DATA_HOME="$OH/.local/share" HEARTH_DESKTOP_JSON="$OH/.local/share/nimbus/desktop.json" PATH="$OB:$PATH" "$@"
}

# ---- 01 Hearth Game Fixer (Lutris)
oss_world game-fixer
GF="$BIN/hearth-game-fixer"
oss oss-a-01-game-fixer hearth-game-fixer "$GF" --game bfme2
oss oss-a-01-game-fixer-install hearth-game-fixer "$GF" --game bfme2 --show install
oss oss-a-01-game-fixer-exigo hearth-game-fixer "$GF" --game exigo
SHOT_WAIT=4.5 oss oss-a-01-game-fixer-editing hearth-game-fixer env HEARTH_ARRANGE_OPEN=game-fixer:window HEARTH_STUDIO_STAGE=game-fixer:none "$GF" --game bfme2
SHOT_WAIT=4.5 oss oss-a-01-game-fixer-customize hearth-game-fixer env HEARTH_ARRANGE_OPEN=game-fixer:game HEARTH_STUDIO_STAGE=game-fixer:customize "$GF" --game bfme2
# Opening it wrote nothing of Lutris' (red flag 9: only what you change is written)
if [ -n "$(find "$OH/.config/lutris" -newer "$OA/game-fixer/lutris-games.json" -type f 2>/dev/null)" ]; then
  echo "::error title=oss-01 game fixer::opening it changed a Lutris settings file"
fi

# ---- 02 Proton Manager (ProtonUp-Qt): Settings › Gaming › Proton and Wine
oss_world proton-manager
PM="$BIN/hearth-proton-manager"
export HEARTH_BUILTIN_PROTON_DIR="$OA/proton-manager/builtin" HEARTH_PM_WHO=Dragongamer9 HEARTH_PM_PC="Nvidia PC"
before=$(md5sum "$OH/.local/share/Steam/config/config.vdf")
SHOT_WAIT=4 oss oss-a-02-proton-manager hearth-proton-manager "$PM"
SHOT_WAIT=4.5 oss oss-a-02-proton-manager-editing hearth-proton-manager env HEARTH_ARRANGE_OPEN=proton-manager:page HEARTH_STUDIO_STAGE=proton-manager:none "$PM"
[ "$before" = "$(md5sum "$OH/.local/share/Steam/config/config.vdf")" ] || echo "::error title=oss-02 proton manager::opening it changed Steam's config.vdf"
unset HEARTH_BUILTIN_PROTON_DIR HEARTH_PM_WHO HEARTH_PM_PC

# ---- 03 Hearth Overlay (MangoHud + GOverlay): its settings beside a game with MangoHud drawing over it (vkcube on
# CI's software Vulkan stands in for the game, when MangoHud and vkcube are here)
oss_world overlay
OV="$BIN/hearth-overlay"
vk=
if command -v vkcube >/dev/null 2>&1; then
  env HOME="$OH" XDG_CONFIG_HOME="$OH/.config" MANGOHUD=1 MANGOHUD_CONFIGFILE="$OH/.config/MangoHud/MangoHud.conf" vkcube --wsi wayland >/tmp/vkcube.log 2>&1 &
  vk=$!
  sleep 2
  kill -0 "$vk" 2>/dev/null || { echo "::notice title=oss-03 overlay::no game stand-in: $(tail -c 400 /tmp/vkcube.log | tr '\n' '|')"; vk=; }
else
  echo "::notice title=oss-03 overlay::vkcube isn't here, so the picture has no game under MangoHud"
fi
swaymsg 'for_window [app_id="hearth-overlay"] floating enable, resize set 660 770, move position 610 15' >/dev/null 2>&1
before=$(md5sum "$OH/.config/MangoHud/MangoHud.conf")
SHOT_WAIT=3.5 oss oss-a-03-overlay hearth-overlay "$OV"
SHOT_WAIT=3.5 oss oss-a-03-overlay-forza hearth-overlay "$OV" --game wine-ForzaHorizon5.conf
SHOT_WAIT=4.5 oss oss-a-03-overlay-editing hearth-overlay env HEARTH_ARRANGE_OPEN=overlay:settings HEARTH_STUDIO_STAGE=overlay:none "$OV"
[ -n "$vk" ] && { kill "$vk" 2>/dev/null; wait "$vk" 2>/dev/null; }
[ "$before" = "$(md5sum "$OH/.config/MangoHud/MangoHud.conf")" ] || echo "::error title=oss-03 overlay::opening it changed MangoHud.conf"

# ---- 08 Hearth Bottles (Bottles): two programs running in Office and tools (stand-ins with its WINEPREFIX)
oss_world bottles
BO="$BIN/hearth-bottles"
sh "$OA/bottles/run-office.sh"
sleep 1.5
SHOT_WAIT=4 oss oss-a-08-bottles hearth-bottles "$BO" --bottle "Office and tools"
SHOT_WAIT=4.5 oss oss-a-08-bottles-editing hearth-bottles env HEARTH_ARRANGE_OPEN=bottles:bottle HEARTH_STUDIO_STAGE=bottles:none "$BO" --bottle "Office and tools"
pkill -f "import time; x = bytearray" 2>/dev/null

# ---- 17 Hearth 3D (Blender): the quad mech (Blender's answer stands in: oss-a-fixture.py's scene)
oss_world hearth-3d
H3="$BIN/hearth-3d"
SHOT_WAIT=4 oss oss-a-17-hearth-3d hearth-3d "$H3" "$OH/Models/quad-mech.blend" --pick Body
SHOT_WAIT=4 oss oss-a-17-hearth-3d-wire hearth-3d "$H3" "$OH/Models/quad-mech.blend" --pick Gun --shade wire
SHOT_WAIT=4.5 oss oss-a-17-hearth-3d-editing hearth-3d env HEARTH_ARRANGE_OPEN=hearth-3d:studio HEARTH_STUDIO_STAGE=hearth-3d:none "$H3" "$OH/Models/quad-mech.blend" --pick Body
