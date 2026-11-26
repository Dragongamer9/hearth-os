# Hearth Cloud (hosts/hearth-ui/apps/cloud; checklist docs/mockups/checklists/open-source-13-cloud.md): board 13 on
# made-up answers from Nextcloud (HEARTH_CLOUD_FIXTURE; the WebDAV / CalDAV / CardDAV reading is proved by cargo test,
# dav.rs), at the board's size, and the sign-in as it really starts. Sourced by hearth-apps-rust-test.sh.
CFX=/tmp/cloud-fx
rm -rf "$CFX"; mkdir -p "$CFX"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" cloud "$CFX" || echo "::error title=cloud fixture::it couldn't be made"
CL="$BIN/hearth-cloud-app"
run cloud-0-sign-in hearth-cloud 20 env XDG_CONFIG_HOME="$CFX/empty" "$CL"
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
run cloud-1-board hearth-cloud 0 env XDG_CONFIG_HOME="$CFX/config" HEARTH_CLOUD_FIXTURE="$CFX/cloud.json" "$CL"
run cloud-2-edit-window hearth-cloud 0 env XDG_CONFIG_HOME="$CFX/config" HEARTH_CLOUD_FIXTURE="$CFX/cloud.json" HEARTH_ARRANGE_OPEN=cloud:window HEARTH_ARRANGE_PICK=side "$CL"
run cloud-3-edit-home hearth-cloud 0 env XDG_CONFIG_HOME="$CFX/config" HEARTH_CLOUD_FIXTURE="$CFX/cloud.json" HEARTH_ARRANGE_OPEN=cloud:home HEARTH_ARRANGE_PICK=today "$CL"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
