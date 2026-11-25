# Every page of Capture, the G15 app, Hearth Ideas, Hearth Notes, Reliability history, Resource Monitor, Server mode,
# System Restore and Task Manager made of its real parts (hearth_ui blocks.rs / arrange.rs; hosts/linux/test/
# test_app_blocks_b.py checks each page names its blocks). Each page twice: as it is (nothing customised: exactly the
# app's own look) and in Edit layout (HEARTH_ARRANGE_OPEN=<app>:<page>, the picked block from HEARTH_ARRANGE_PICK):
# the live page with each block's soft outline, the picked one's name, ✕ Hide, its edges and corner, and the bar with
# Undo, Back to the original, Describe it… and Done. Then a few pages laid out from a kept file with the new block ids.
# Sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its. Fixtures as each app's own shots file
# makes them (in folders of their own here, so this file runs alone with only=app-blocks-b).
BB=/tmp/blocks-b
rm -rf "$BB"; mkdir -p "$BB/empty/nimbus/apps" "$BB/kept/nimbus/apps"
E="$BB/empty"

# NAME APPID APP:PAGE PICK COMMAND…: the page as it is, then in Edit layout
both() {
  local name=$1 appid=$2 open=$3 pick=$4
  shift 4
  run "app-blocks-b-$name" "$appid" 0 env XDG_CONFIG_HOME="${CFG:-$E}" "$@"
  run "app-blocks-b-$name-edit" "$appid" 0 env XDG_CONFIG_HOME="${CFG:-$E}" HEARTH_ARRANGE_OPEN="$open" HEARTH_ARRANGE_PICK="$pick" "$@"
}

# ---- Task Manager: the window (the rail and the page) and every page
TM="$BIN/hearth-task-manager"
both tm-window hearth-tasks task-manager:window rail "$TM" --page processes
both tm-processes hearth-tasks task-manager:processes buttons "$TM" --page processes
both tm-performance hearth-tasks task-manager:performance device "$TM" --page performance
for pg in history startup users details services tasks settings; do
  both "tm-$pg" hearth-tasks "task-manager:$pg" table "$TM" --page "$pg"
done

# ---- System Restore and Reliability history
both restore hearth-restore system-restore:main changes "$BIN/hearth-system-restore"
both reliability hearth-reliability reliability:main marks "$BIN/hearth-reliability-history"

# ---- Resource Monitor: the window and every tab (resmon.sh's fixture helper and test PC)
export HEARTH_RESMON_HELPER="$ROOT/.github/scripts/hearth-apps-rust-shots.d/resmon-fixture.py" HEARTH_RESMON_PC_FIXTURE="$ROOT/hosts/linux/test/fixtures/resmon/atx-desktop.json"
RM="$BIN/hearth-resmon-app"
both resmon-window hearth-resmon resmon:window tabs "$RM" --page overview
both resmon-pc hearth-resmon resmon:pc details "$RM" --page pc
for pg in overview cpu memory disk network gpu; do
  both "resmon-$pg" hearth-resmon "resmon:$pg" programs "$RM" --page "$pg"
done
unset HEARTH_RESMON_HELPER HEARTH_RESMON_PC_FIXTURE

# ---- Server mode's dashboard (server.sh's fixture)
cp "$ROOT/hosts/linux/test/fixtures/server-dash.json" "$BB/dash.json"
rm -rf "$XDG_RUNTIME_DIR/hearth-server"
both server hearth-server server:dashboard alerts env HEARTH_SERVER_NO_PAD=1 HEARTH_SERVER_DASH_FIXTURE="$BB/dash.json" HEARTH_NIMBUS_MODE=/bin/true "$BIN/hearth-server-app"
rm -rf "$XDG_RUNTIME_DIR/hearth-server"

# ---- Capture: the window, Take, Edit (on a picture made here) and Recent
python3 -c "from PIL import Image, ImageDraw; im = Image.new('RGB', (1280, 720), '#e8eef6'); d = ImageDraw.Draw(im); d.rectangle([0, 0, 1280, 64], fill='#5b8cff'); d.rectangle([80, 140, 700, 420], fill='#ffffff', outline='#c9cfd6'); im.save('$BB/shot.png')" || echo "::warning title=blocks-b::no picture for Capture's Edit"
CA="$BIN/hearth-capture-app"
export HEARTH_CAPTURE_HELPER="$HEARTH_SHELL_DIR/hearth-capture-helper"
both capture-window hearth-capture capture:window header "$CA"
both capture-take hearth-capture capture:take wait "$CA"
both capture-edit hearth-capture capture:edit side "$CA" --edit "$BB/shot.png" --sample
both capture-recent hearth-capture capture:recent grid "$CA" --page recent

# ---- Logitech G15 (g15.sh's pretend G15)
G15B="$BB/g15"
"$BIN/hearth-g15" make-fixture "$G15B" && cp "$ROOT/.github/scripts/hearth-g15-sample.json" "$G15B/g15.json"
export HEARTH_G15_FIXTURE="$G15B" HEARTH_G15_CONFIG="$G15B/g15.json"
G15="$BIN/hearth-g15-app"
both g15-window hearth-g15-app g15:window side "$G15" --page overview
both g15-overview hearth-g15-app g15:overview sets "$G15" --page overview
both g15-gkeys hearth-g15-app g15:gkeys key "$G15" --page g-keys --key 5
both g15-screen hearth-g15-app g15:screen pages "$G15" --page screen
both g15-lighting hearth-g15-app g15:lighting keys "$G15" --page lighting
both g15-editor hearth-g15-app g15:editor props "$G15" --page editor:dads-screen --element 2
unset HEARTH_G15_FIXTURE HEARTH_G15_CONFIG

# ---- Hearth Ideas (ideas.sh's pretend GitHub answers: its fixture part, before its first picture)
eval "$(sed '/^run /,$d' "$ROOT/.github/scripts/hearth-apps-rust-shots.d/ideas.sh")"
ID="$BIN/hearth-ideas-app"
both ideas-window hearth-ideas ideas:window side env HEARTH_IDEAS_FIXTURE="$IFX/new.json" "$ID"
both ideas-list hearth-ideas ideas:list ideas env HEARTH_IDEAS_FIXTURE="$IFX/new.json" "$ID"
both ideas-idea hearth-ideas ideas:idea build env HEARTH_IDEAS_FIXTURE="$IFX/new.json" "$ID" --open 812
both ideas-built hearth-ideas ideas:built steps env HEARTH_IDEAS_FIXTURE="$IFX/built.json" "$ID" --open 812 --page built

# ---- Hearth Notes (notes.sh's pretend worlds); its config folder is each world's own (nothing kept there)
NF=/tmp/notes-fixture-b
rm -rf "$NF"
NOTES_NOW=$(python3 "$ROOT/.github/scripts/hearth-notes-fixture.py" "$NF" "$ROOT" | tail -1) || echo "::error title=blocks-b notes fixture::it couldn't be made"
printf '{"look": "steam", "mode": "dark", "accent": "#ff7a2f"}\n' > "$NF/desktop.json"
# WORLD NAME APP:PAGE PICK ARGS…
notes_both() {
  local world=$1 name=$2 open=$3 pick=$4
  shift 4
  local w="$NF/$world/home"
  local envs=(env HOME="$w" XDG_DATA_HOME="$w/.local/share" XDG_CONFIG_HOME="$w/.config" XDG_CACHE_HOME="$w/.cache" HEARTH_DESKTOP_JSON="$NF/desktop.json"
    HEARTH_NOTES_NOW="$NOTES_NOW" HEARTH_NOTES_THIS_PC="Hearth OS 1.16 · Fedora edition" HEARTH_NOTES_PC_NAME="Nvidia PC" PATH="$NF/bin:$PATH")
  run "app-blocks-b-$name" hearth-notes 0 "${envs[@]}" "$BIN/hearth-notes-app" "$@" --no-auto-sync
  run "app-blocks-b-$name-edit" hearth-notes 0 "${envs[@]}" HEARTH_ARRANGE_OPEN="$open" HEARTH_ARRANGE_PICK="$pick" "$BIN/hearth-notes-app" "$@" --no-auto-sync
}
SN=hearth/mockups/hearth-store/README.md
notes_both main notes-window notes:window side --note "$SN" --open-folders hearth,other
notes_both main notes-files notes:files graph --note "$SN" --open-folders hearth,other
notes_both main notes-search notes:search doc --page search --note hearth/notes/1.16-fedora-release-channel/README.md --search "Fedora USB" --search-folder hearth/notes,log --search-days 7 --search-open 1
notes_both today notes-today notes:today dashboard --page today --open-folders atlas,log
notes_both main notes-board notes:board columns --page board --open-folders hearth,other
notes_both main notes-graph notes:graph graph --page graph --graph-mode classic --select "$SN" --still --open-folders hearth,other
notes_both main notes-modes notes:modes grid --page modes --graph-mode classic --still --open-folders hearth,other
notes_both sync notes-sync notes:sync changes --page sync --show sync --open-folders hearth,other
notes_both conflict notes-conflict notes:conflict panes --page conflict --show conflict --open-folders hearth,other
notes_both main notes-settings notes:settings github --page settings --open-folders hearth,other

# ---- Laid out from a kept file, with the new block ids (the rest of each page at its own place)
K="$BB/kept/nimbus/apps"
cat > "$K/reliability.json" <<'J'
{"format": 1, "layout": {"blocks": {"main": {
  "heading": {"x": 0, "y": 0, "w": 12, "h": 1}, "toggle": {"hidden": true},
  "events": {"x": 0, "y": 1, "w": 6, "h": 11, "corners": 16},
  "stability": {"x": 6, "y": 1, "w": 6, "h": 3, "corners": 16, "accent": "#5b8cff"},
  "marks": {"x": 6, "y": 4, "w": 6, "h": 6}, "days": {"x": 6, "y": 10, "w": 6, "h": 2}}}},
 "features": []}
J
cat > "$K/system-restore.json" <<'J'
{"format": 1, "layout": {"blocks": {"main": {
  "make": {"x": 0, "y": 0, "w": 4, "h": 1}, "status": {"x": 4, "y": 0, "w": 4, "h": 1}, "go": {"x": 8, "y": 0, "w": 4, "h": 1},
  "header": {"hidden": true}, "see": {"hidden": true},
  "changes": {"x": 0, "y": 1, "w": 4, "h": 11, "corners": 20, "accent": "#2f9e5a"},
  "heads": {"x": 4, "y": 1, "w": 8, "h": 1}, "points": {"x": 4, "y": 2, "w": 8, "h": 9}, "older": {"x": 4, "y": 11, "w": 8, "h": 1}}}},
 "features": []}
J
cat > "$K/task-manager.json" <<'J'
{"format": 1, "layout": {"blocks": {
  "processes": {"buttons": {"x": 0, "y": 0, "w": 6, "h": 1, "corners": 12, "seeThrough": 15}, "title": {"x": 6, "y": 0, "w": 3, "h": 1}, "tabs": {"x": 9, "y": 0, "w": 3, "h": 1}}}},
 "features": []}
J
run app-blocks-b-kept-reliability hearth-reliability 0 env XDG_CONFIG_HOME="$BB/kept" "$BIN/hearth-reliability-history"
run app-blocks-b-kept-restore hearth-restore 0 env XDG_CONFIG_HOME="$BB/kept" "$BIN/hearth-system-restore"
run app-blocks-b-kept-tm-processes hearth-tasks 0 env XDG_CONFIG_HOME="$BB/kept" "$TM" --page processes
