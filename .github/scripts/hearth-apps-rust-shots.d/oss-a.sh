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
oss oss-01-game-fixer hearth-game-fixer "$GF" --game bfme2
oss oss-01-game-fixer-install hearth-game-fixer "$GF" --game bfme2 --show install
oss oss-01-game-fixer-exigo hearth-game-fixer "$GF" --game exigo
SHOT_WAIT=4.5 oss oss-01-game-fixer-editing hearth-game-fixer env HEARTH_ARRANGE_OPEN=game-fixer:window HEARTH_STUDIO_STAGE=game-fixer:none "$GF" --game bfme2
SHOT_WAIT=4.5 oss oss-01-game-fixer-customize hearth-game-fixer env HEARTH_ARRANGE_OPEN=game-fixer:game HEARTH_STUDIO_STAGE=game-fixer:customize "$GF" --game bfme2
# Opening it wrote nothing of Lutris' (red flag 9: only what you change is written)
if [ -n "$(find "$OH/.config/lutris" -newer "$OA/game-fixer/lutris-games.json" -type f 2>/dev/null)" ]; then
  echo "::error title=oss-01 game fixer::opening it changed a Lutris settings file"
fi
