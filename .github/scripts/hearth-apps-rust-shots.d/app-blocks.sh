# Every Hearth app made of blocks (the owner, Nov 24: "Modular for all Hearth apps is required as well";
# hearth_ui blocks.rs and arrange.rs): apps laid out from a kept file (a fixture's own config folder, so nothing of the
# person's is touched), and Arrange (Describe it › Move and resize blocks) open with its grid, each block's bar and
# corner, the picked block's Look. Sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its.
BC=/tmp/blocks-cfg
rm -rf "$BC"; mkdir -p "$BC/nimbus/apps"

# Task Manager: the side wider on rounded cards with the orange edge; on Processes the title and buttons at the bottom,
# see-through, in Noto Serif.
cat > "$BC/nimbus/apps/task-manager.json" <<'J'
{"format": 1, "layout": {"blocks": {
  "window": {"rail": {"x": 0, "y": 0, "w": 3, "h": 12, "corners": 16, "accent": "#ff7a2f"}, "page": {"x": 3, "y": 0, "w": 9, "h": 12}},
  "processes": {"table": {"x": 0, "y": 0, "w": 12, "h": 10}, "head": {"x": 0, "y": 10, "w": 12, "h": 2, "corners": 12, "seeThrough": 35, "font": "Noto Serif"}}}},
 "features": []}
J
# Reliability history: the day's events on the left, the chart on the right in blue, the heading over both.
cat > "$BC/nimbus/apps/reliability.json" <<'J'
{"format": 1, "layout": {"blocks": {"main": {
  "header": {"x": 0, "y": 0, "w": 12, "h": 2},
  "events": {"x": 0, "y": 2, "w": 7, "h": 10, "corners": 16},
  "chart": {"x": 7, "y": 2, "w": 5, "h": 10, "corners": 16, "accent": "#5b8cff", "size": "smaller"}}}},
 "features": []}
J
# System Restore: its buttons at the top, no heading, What will change on the left.
cat > "$BC/nimbus/apps/system-restore.json" <<'J'
{"format": 1, "layout": {"blocks": {"main": {
  "footer": {"x": 0, "y": 0, "w": 12, "h": 1},
  "header": {"hidden": true},
  "aside": {"x": 0, "y": 1, "w": 4, "h": 11, "corners": 20, "accent": "#2f9e5a"},
  "list": {"x": 4, "y": 1, "w": 8, "h": 11}}}},
 "features": []}
J
# Hearth Store: the side on the right, the bar at the bottom moved to the top.
cat > "$BC/nimbus/apps/store.json" <<'J'
{"format": 1, "layout": {"blocks": {"window": {
  "bottom": {"x": 0, "y": 0, "w": 12, "h": 1},
  "page": {"x": 0, "y": 1, "w": 9, "h": 11},
  "side": {"x": 9, "y": 1, "w": 3, "h": 11, "corners": 16, "seeThrough": 15}}}},
 "features": []}
J
# A file that can't be read: the app opens as it always looks (never a broken app).
echo '{"format": 1, "layout": {"blocks": {"window": {"side": {"x": 40}}}}' > "$BC/nimbus/apps/g15.json"

run blocks-01-task-manager hearth-tasks 10 env XDG_CONFIG_HOME="$BC" "$BIN/hearth-task-manager"
run blocks-02-reliability hearth-reliability 0 env XDG_CONFIG_HOME="$BC" "$BIN/hearth-reliability-history"
run blocks-03-system-restore hearth-restore 0 env XDG_CONFIG_HOME="$BC" "$BIN/hearth-system-restore"
run blocks-04-store hearth-store 0 env XDG_CONFIG_HOME="$BC" "$BIN/hearth-store-app"
run blocks-05-g15-broken-file-is-the-original hearth-g15-app 0 env XDG_CONFIG_HOME="$BC" "$BIN/hearth-g15-app"

# Arrange open: the grid, each block's bar (drag it to move, ✕ hides) and corner (drag it to size), the picked block's
# Look (corners, see-through, font, text size, accent), Undo, Back to the original, Done, Keep this.
run blocks-06-arrange-task-manager hearth-tasks 0 env XDG_CONFIG_HOME="$BC" HEARTH_ARRANGE_OPEN=task-manager:processes HEARTH_ARRANGE_PICK=head "$BIN/hearth-task-manager"
run blocks-07-arrange-restore-hidden-heading hearth-restore 0 env XDG_CONFIG_HOME="$BC" HEARTH_ARRANGE_OPEN=system-restore:main HEARTH_ARRANGE_PICK=aside "$BIN/hearth-system-restore"
run blocks-08-arrange-files hearth-files 0 env XDG_CONFIG_HOME=/tmp/blocks-empty HEARTH_ARRANGE_OPEN=files:window HEARTH_ARRANGE_PICK=folder "$BIN/hearth-files-app" "$HOME"
run blocks-09-arrange-settings-home hearth-settings 0 env XDG_CONFIG_HOME=/tmp/blocks-empty HEARTH_ARRANGE_OPEN=settings:home HEARTH_ARRANGE_PICK=storage "$BIN/hearth-settings-app"
run blocks-10-arrange-notes hearth-notes 0 env XDG_CONFIG_HOME=/tmp/blocks-empty HEARTH_ARRANGE_OPEN=notes:window "$BIN/hearth-notes-app"
# Describe it's box with Move and resize blocks beside Done
run blocks-11-describe-box hearth-capture 0 env XDG_CONFIG_HOME=/tmp/blocks-empty HEARTH_DESCRIBE_OPEN=capture "$BIN/hearth-capture-app"
