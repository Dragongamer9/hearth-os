# Describe it in every Hearth app (hearth_ui::describe; desktop-layouts board 11, "every one of our apps needs this"):
# each app's own window bar drawn by describe::title_bar, with the Describe it box open at the bottom
# (HEARTH_DESCRIBE_OPEN=<app>). Sourced by hearth-apps-rust-test.sh inside its headless screen.
run describe-everywhere-tm hearth-tasks 10 env HEARTH_DESCRIBE_OPEN=task-manager "$BIN/hearth-task-manager"
run describe-everywhere-restore hearth-restore 0 env HEARTH_DESCRIBE_OPEN=system-restore "$BIN/hearth-system-restore"
run describe-everywhere-reliability hearth-reliability 0 env HEARTH_DESCRIBE_OPEN=reliability "$BIN/hearth-reliability-history"
run describe-everywhere-capture hearth-capture 0 env HEARTH_DESCRIBE_OPEN=capture "$BIN/hearth-capture-app"
