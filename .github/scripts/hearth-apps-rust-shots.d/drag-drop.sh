# Drag and drop in every Hearth app (docs/mockups/drag-drop/drag-drop-3.png, its checklist's section 3; hearth_ui
# studio.rs): each app's window in editing (every part outlined and labelled, its blocks window, the toolbar), a block
# being dragged into a bar with the lit line, a part's (or pane's) edge being dragged with the guide and its note, and
# board 2's side panel on a part (Right-click any part › Customize); for Hearth Files also the right-click menu and a
# part moved off its side. Nothing of the person's is touched: config folders of their own. Sourced by
# hearth-apps-rust-test.sh inside its headless screen; run() is its.
DC=/tmp/drag-drop-cfg
rm -rf "$DC"; mkdir -p "$DC/nimbus/apps"
DE=/tmp/drag-drop-empty
rm -rf "$DE"; mkdir -p "$DE/nimbus/apps"

# Hearth Files in Downloads as board 3 has it: three folders, a picture, a zip; a Preview pane kept on the right (the
# folder beside it), so the picture is the mockup's window.
D="$HOME/Downloads"
mkdir -p "$D/Installers" "$D/Pictures" "$D/Videos"
head -c 2400 /dev/zero > "$D/mod.zip"
python3 - "$D/console.png" <<'PY' || echo "::warning title=drag-drop::the picture wasn't made"
import sys
from PIL import Image
small = Image.new("RGB", (16, 9))
px = small.load()
for y in range(9):
    for x in range(16):
        px[x, y] = (int(120 - 4 * y + 2 * x), int(70 - 3 * y), int(40 - 2 * y))
small.resize((3840, 2160), Image.BILINEAR).save(sys.argv[1], "PNG")
PY
cat > "$DC/nimbus/apps/files.json" <<'J'
{"format": 1, "layout": {"studio": {"pages": {"window": {
  "parts": {"sidebar": {"at": [0, 0, 216, 0], "from": "ssse"}, "folder": {"at": [216, 0, 528, 0], "from": "ssee"}, "preview": {"hidden": true}},
  "adds": [{"uid": "b1", "block": "pane:preview", "title": "Preview", "at": [520, 8, 8, 8], "from": "esee"}]}}, "own": []}},
 "features": []}
J

shot() { # NAME APP_ID PAGE STAGE CONFIG BINARY ARGS...
  n=$1; id=$2; pg=$3; stage=$4; cfg=$5; shift 5
  SHOT_WAIT=4.5 run "drag-drop-$n" "$id" 0 env XDG_CONFIG_HOME="$cfg" HEARTH_ARRANGE_OPEN="$pg" HEARTH_STUDIO_STAGE="${pg%%:*}:$stage" "$@"
}
F="$BIN/hearth-files-app"
shot files-1-editing hearth-files files:window none "$DC" "$F" "$D"
shot files-2-drag-into-the-command-bar hearth-files files:window drag "$DC" "$F" "$D"
shot files-3-size-the-preview hearth-files files:window size "$DC" "$F" "$D"
shot files-4-customize hearth-files files:window customize "$DC" "$F" "$D"
shot files-5-right-click-a-part hearth-files files:window menu "$DC" "$F" "$D"
shot files-6-a-part-moved-anywhere hearth-files files:window move "$DC" "$F" "$D"

# every other Rust app: editing, a block dragged in, an edge sized, a part customized
four() { # APP APP_ID PAGE ENV... -- BINARY ARGS...
  app=$1; id=$2; pg=$3; shift 3
  for st in editing drag size customize; do
    stage=$st; [ "$st" = editing ] && stage=none
    SHOT_WAIT=4.5 run "drag-drop-$app-$st" "$id" 0 env XDG_CONFIG_HOME="$DE" HEARTH_ARRANGE_OPEN="$pg" HEARTH_STUDIO_STAGE="${pg%%:*}:$stage" "$@"
  done
}
four settings hearth-settings settings:window "$BIN/hearth-settings-app"
four task-manager hearth-tasks task-manager:window "$BIN/hearth-task-manager"
four capture hearth-capture capture:window "$BIN/hearth-capture-app"
four notes hearth-notes notes:window "$BIN/hearth-notes-app"
four store hearth-store store:window "$BIN/hearth-store-app"
four g15 hearth-g15-app g15:window "$BIN/hearth-g15-app"
FFX=/tmp/drag-drop-family
rm -rf "$FFX"; mkdir -p "$FFX"
echo '{"friends": [{"id": "a1", "name": "Dad'"'"'s PC", "online": true, "sharing": true, "what": "desktop", "ask": true}, {"id": "b2", "name": "Sam'"'"'s Deck", "online": false}], "connected": null, "moonlight": true}' > "$FFX/hearth-link-call-moonlight.friends.json"
four family hearth-family family:friends HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --page friends
four server hearth-server server:dashboard HEARTH_SERVER_NO_PAD=1 HEARTH_NIMBUS_MODE=/bin/true HEARTH_SERVER_DASH_FIXTURE="$ROOT/hosts/linux/test/fixtures/server-dash.json" "$BIN/hearth-server-app"
four resmon hearth-resmon resmon:window HEARTH_RESMON_HELPER="$ROOT/.github/scripts/hearth-apps-rust-shots.d/resmon-fixture.py" HEARTH_RESMON_PC_FIXTURE="$ROOT/hosts/linux/test/fixtures/resmon/atx-desktop.json" "$BIN/hearth-resmon-app" --page overview
four system-restore hearth-restore system-restore:main "$BIN/hearth-system-restore"
four reliability hearth-reliability reliability:main "$BIN/hearth-reliability-history"
four ideas hearth-ideas ideas:window "$BIN/hearth-ideas-app"
