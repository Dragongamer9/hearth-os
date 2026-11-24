# Describe it in every Hearth app (hearth_ui::describe; desktop-layouts board 11, "every one of our apps needs this"):
# each app's own window bar drawn by describe::title_bar, with the Describe it box open at the bottom
# (HEARTH_DESCRIBE_OPEN=<app>). Sourced by hearth-apps-rust-test.sh inside its headless screen.
run describe-everywhere-tm hearth-tasks 10 env HEARTH_DESCRIBE_OPEN=task-manager "$BIN/hearth-task-manager"
run describe-everywhere-restore hearth-restore 0 env HEARTH_DESCRIBE_OPEN=system-restore "$BIN/hearth-system-restore"
run describe-everywhere-reliability hearth-reliability 0 env HEARTH_DESCRIBE_OPEN=reliability "$BIN/hearth-reliability-history"
run describe-everywhere-capture hearth-capture 0 env HEARTH_DESCRIBE_OPEN=capture "$BIN/hearth-capture-app"
# Task Manager as Describe it left it (its own pieces, the owner Nov 22: "Modular app menues and right clicks as well"):
# icons-only side with Performance first, End task first on the bar and Efficiency mode / Open file location hidden
# (on the bar and in the right-click menu), Go to details at the top of the menu.
TMC=/tmp/describe-tm-cfg
rm -rf "$TMC"; mkdir -p "$TMC/nimbus/apps"
cat > "$TMC/nimbus/apps/task-manager.json" <<'J'
{"format": 1, "layout": {"rail": "icons", "winbuttons": "dots", "zones": {
  "rail": ["fold", "performance", "processes", "details", "spacer", "settings", "history", "startup", "users", "services", "tasks"],
  "bar": ["end-task", "run-new-task"], "hidden": ["efficiency-mode", "open-file-location"],
  "menu:row": ["go-to-details", "end-task", "end-process-tree"]}}, "features": [], "undo": []}
J
run describe-everywhere-tm-kept hearth-tasks 0 env XDG_CONFIG_HOME="$TMC" "$BIN/hearth-task-manager" --show menu
