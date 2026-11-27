# Hearth Forge (hosts/hearth-ui/apps/forge; checklist docs/mockups/checklists/hearth-forge.md): boards 01-make and
# 02-models against hearth-forge.py's own service (serve, on 127.0.0.1:8191) over forge-fixture.py's made-up folder:
# the lantern made (its preview by the helper's own code), the board's other made models, two waiting (the service
# waits as it does while you play: a pretend "gamescope" runs), the four installed models, a pretend RTX 5070 and a
# pretend Tripo3D with 2,275 credits. Nothing is downloaded or made here. Sourced by hearth-apps-rust-test.sh inside
# its headless screen; run() pictures each one.
FFX=/tmp/forge-fx
FDIR="$ROOT/.github/scripts/hearth-apps-rust-shots.d"
FH="$ROOT/hosts/hearth-ui/apps/forge/helper/hearth-forge.py"
# its helper's own tests first (hosts/linux/test/test_hearth_forge.py: every model's runner with fakes, Get, the
# runtimes, splats, the make flow): a notice when they pass, each failure as an error
if (cd "$ROOT" && python3 -W ignore -m unittest -v hosts/linux/test/test_hearth_forge.py) >/tmp/forge-tests.log 2>&1; then
  echo "::notice title=forge tests::$(grep -c ' ... ok$' /tmp/forge-tests.log) passed · $(tail -n 3 /tmp/forge-tests.log | tr '\n' ' ')"
else
  grep -E '^(FAIL|ERROR):' /tmp/forge-tests.log | head -20 | while read -r l; do echo "::error title=forge tests::$l"; done
  echo "::error title=forge tests::$(grep -E -B2 -A12 '^(FAIL|ERROR):' /tmp/forge-tests.log | tail -c 3500 | tr '\n' '|')"
fi
python3 "$FDIR/forge-fixture.py" make "$FFX" || echo "::error title=forge fixture::it couldn't be made"
FA="$BIN/hearth-forge-app"
FENV="XDG_DATA_HOME=$FFX/data XDG_CONFIG_HOME=$FFX/config HEARTH_FORGE_DIR=$FFX/Forge HEARTH_FORGE_TRIPO=http://127.0.0.1:8199 PATH=$FFX/bin:$PATH"
"$FFX/bin/gamescope" 3600 &
fg=$!
python3 "$FDIR/forge-fixture.py" tripo 8199 >/tmp/forge-tripo.log 2>&1 &
ft=$!
env PATH="$FFX/bin:$PATH" HEARTH_FORGE_DIR="$FFX/Forge" XDG_STATE_HOME="$FFX/state" HEARTH_FORGE_DATA="$FFX/data/hearth/forge" python3 "$FH" serve >/tmp/forge-serve.log 2>&1 &
fs=$!
sleep 2
python3 -c "import urllib.request; print(urllib.request.urlopen('http://127.0.0.1:8191/status', timeout=5).read()[:200])" >/tmp/forge-status.json 2>&1 || echo "::error title=forge service::it doesn't answer: $(tail -c 800 /tmp/forge-serve.log | tr '\n' '|')"
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
SHOT_WAIT=5 run forge-1-make hearth-forge 0 env $FENV "$FA"
SHOT_WAIT=4 run forge-2-models hearth-forge 0 env $FENV "$FA" --page models
SHOT_WAIT=4 run forge-3-made hearth-forge 0 env $FENV "$FA" --page made
SHOT_WAIT=4 run forge-4-waiting hearth-forge 0 env $FENV "$FA" --page waiting
SHOT_WAIT=4 run forge-5-settings hearth-forge 0 env $FENV "$FA" --page settings
SHOT_WAIT=5 run forge-6-edit-window hearth-forge 0 env $FENV HEARTH_ARRANGE_OPEN=forge:window HEARTH_ARRANGE_PICK=view "$FA"
kill $fs $fg 2>/dev/null; wait $fs $fg 2>/dev/null
# the Nvidia PC not reachable: what to do about it
SHOT_WAIT=4 run forge-7-no-service hearth-forge 0 env $FENV "$FA"
kill $ft 2>/dev/null; wait $ft 2>/dev/null
[ -e "$FFX/config/nimbus/forge.json" ] && echo "::error title=forge settings::opening Hearth Forge wrote forge.json by itself: $(cat "$FFX/config/nimbus/forge.json")"
[ -s "$FFX/data/hearth/forge/hearth-forge.py" ] || echo "::error title=forge helper::the app didn't write its helper next to it"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
# at rest with nothing being made (no service: nothing asks again), measured on the usual screen
run forge-0-at-rest hearth-forge 20 env $FENV "$FA"
