# Logitech G15 (hearth-g15-app, docs/G15.md): every page of the approved mockups (docs/mockups/g15) on a pretend G15
# (hearth-g15 make-fixture) with a g15.json like the mockups' (hearth-g15-sample.json), then the keyboard's screen pages.
G15FX=/tmp/g15-fixture
rm -rf "$G15FX"; "$BIN/hearth-g15" make-fixture "$G15FX"
cp "$ROOT/.github/scripts/hearth-g15-sample.json" "$G15FX/g15.json"
export HEARTH_G15_FIXTURE="$G15FX" HEARTH_G15_CONFIG="$G15FX/g15.json"
cp "$G15FX/g15.json" /tmp/g15-before.json
run g15-1-overview hearth-g15-app 20 "$BIN/hearth-g15-app" --page overview
run g15-2-g-keys hearth-g15-app 0 "$BIN/hearth-g15-app" --page g-keys --key 5
run g15-3-keyboard-screen hearth-g15-app 0 "$BIN/hearth-g15-app" --page screen
run g15-4-lighting hearth-g15-app 0 "$BIN/hearth-g15-app" --page lighting
run g15-6-make-your-own hearth-g15-app 0 "$BIN/hearth-g15-app" --page editor:dads-screen --element 2
run g15-7-hearth-actions hearth-g15-app 0 "$BIN/hearth-g15-app" --page g-keys --key 5 --show hearth
run g15-8-games hearth-g15-app 0 "$BIN/hearth-g15-app" --page overview --show games
# Opening the app wrote nothing (red flag 9).
cmp -s "$G15FX/g15.json" /tmp/g15-before.json || echo "::error title=g15::opening the app changed g15.json"
for p in clock system music alerts alert rec set dads-screen; do
  "$BIN/hearth-g15" frame "$p" "$ROOT/shots/g15-lcd-$p.png" --scale 4 || echo "::error title=g15 frame::$p"
done
unset HEARTH_G15_FIXTURE HEARTH_G15_CONFIG
