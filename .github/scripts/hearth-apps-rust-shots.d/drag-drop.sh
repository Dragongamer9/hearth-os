# Drag and drop in every Hearth app (docs/mockups/drag-drop/drag-drop-3.png, its checklist's section 3; hearth_ui
# studio.rs): each app's window in editing (every part outlined and labelled, its blocks window, the toolbar), a block
# being dragged into a bar with the lit line, a part's edge being dragged with the guide and its note, and board 2's
# side panel on a part (Right-click any part › Customize). Nothing of the person's is touched: an empty config folder.
# Sourced by hearth-apps-rust-test.sh inside its headless screen; run() is its.
DC=/tmp/drag-drop-cfg
rm -rf "$DC"; mkdir -p "$DC/nimbus/apps"
shot() { # NAME APP_ID APP STAGE BINARY ARGS...
  n=$1; id=$2; app=$3; stage=$4; shift 4
  SHOT_WAIT=4 run "drag-drop-$n" "$id" 0 env XDG_CONFIG_HOME="$DC" HEARTH_ARRANGE_OPEN="$app" HEARTH_STUDIO_STAGE="$app:$stage" "$@"
}
shot files-1-editing hearth-files files:window none "$BIN/hearth-files-app" "$HOME/Downloads"
shot files-2-drag hearth-files files:window drag "$BIN/hearth-files-app" "$HOME/Downloads"
shot files-3-size hearth-files files:window size "$BIN/hearth-files-app" "$HOME/Downloads"
shot files-4-customize hearth-files files:window customize "$BIN/hearth-files-app" "$HOME/Downloads"
