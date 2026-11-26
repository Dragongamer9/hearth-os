# Hearth Listen (hosts/hearth-ui/apps/listen; checklist docs/mockups/checklists/open-source-07-listen.md): board 07 on
# the board's own recordings and words (oss-fixtures.py listen: made-up recordings, sparse model files, a pretend
# whisper-cli and nvidia-smi; nothing is transcribed here). The screen is the board's size (1920 x 1080) for these.
# Sourced by hearth-apps-rust-test.sh inside its headless screen; run() pictures each one.
LFX=/tmp/listen-fx
rm -rf "$LFX"; mkdir -p "$LFX"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" listen "$LFX" || echo "::error title=listen fixture::it couldn't be made"
LCFG=/tmp/listen-cfg; rm -rf "$LCFG"; mkdir -p "$LCFG/nimbus"
# the board's choices: Tell voices apart, Commas and capitals and Captions on, Translate off, Bottom, 28 px, What's playing
printf '%s\n' '{"captions": true, "captionsWhere": "bottom", "captionSize": 28, "captionsFrom": "playing"}' > "$LCFG/nimbus/listen.json"
LI="$BIN/hearth-listen-app"
LENV="HEARTH_LISTEN_DATA=$LFX XDG_CONFIG_HOME=$LCFG PATH=$LFX/fakes:$PATH"
# measured (first frame, memory, CPU at rest) on the usual screen
run listen-0-at-rest hearth-listen 20 env $LENV "$LI" --open take3 --at 108.9
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
run listen-1-board hearth-listen 0 env $LENV "$LI" --open take3 --at 108.9
run listen-2-export hearth-listen 0 env $LENV "$LI" --open take3 --at 108.9 --show export
run listen-3-model hearth-listen 0 env $LENV "$LI" --open take3 --show model
run listen-4-search hearth-listen 0 env $LENV "$LI" --open take3 --search "Hearth Store"
# the live captions bar (its own window, on top; floated here as a desktop would show it)
env $LENV HEARTH_LISTEN_CAPTION="…and every Hearth app can now be laid out|exactly the way you want it, just drag|Firefox video" "$LI" --captions >/tmp/listen-captions.log 2>&1 &
cp=$!
for i in $(seq 1 200); do mapped hearth-listen-captions && break; sleep 0.05; done
swaymsg '[app_id="hearth-listen-captions"] floating enable, resize set 1240 84, move position 340 930' >/dev/null 2>&1 || true
sleep 2
grim "$ROOT/shots/listen-5-captions.png" 2>/dev/null || echo "::warning title=listen-5-captions::grim failed"
kill -0 $cp 2>/dev/null || echo "::error title=listen-5-captions::it ended by itself: $(tail -c 800 /tmp/listen-captions.log | tr '\n' '|')"
kill $cp 2>/dev/null; wait $cp 2>/dev/null
# whisper.cpp not on this PC yet: the Install card
rm -f "$LFX/bin/whisper-cli"
run listen-6-install hearth-listen 0 env $LENV "$LI" --open take3
# made of blocks: Edit layout on the window and on How it listens
run listen-7-edit-window hearth-listen 0 env $LENV HEARTH_ARRANGE_OPEN=listen:window HEARTH_ARRANGE_PICK=how "$LI" --open take3
run listen-8-edit-how hearth-listen 0 env $LENV HEARTH_ARRANGE_OPEN=listen:how HEARTH_ARRANGE_PICK=live "$LI" --open take3
# opening it wrote nothing of its own (red flag 9): listen.json as it was
grep -q '"captionSize": 28' "$LCFG/nimbus/listen.json" || echo "::error title=listen settings::opening Hearth Listen changed listen.json: $(cat "$LCFG/nimbus/listen.json")"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
