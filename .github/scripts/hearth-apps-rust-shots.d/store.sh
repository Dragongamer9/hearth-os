# The Hearth Store (hosts/hearth-ui/apps/store; checklist docs/mockups/checklists/hearth-store.md): every page of the
# approved mockups (docs/mockups/hearth-store/store-1…9) on a pretend world (hearth-store-fixture.py: the image's list
# of apps, pretend releases, home folders with apps installed, changed, or none, and offline). Sourced by
# hearth-apps-rust-test.sh inside its headless screen; run() pictures each one.
SF=/tmp/store-fixture
rm -rf "$SF"
python3 "$ROOT/.github/scripts/hearth-store-fixture.py" "$SF" "$ROOT/os-bases/anatase/system_files/usr/share/hearth/store/catalog.json" || echo "::error title=store fixture::it couldn't be made"
STORE_LOOK="$HOME/.local/share/nimbus/desktop.json"
store_home() { # NAME [WORLD]
  export HOME_BEFORE_STORE="${HOME_BEFORE_STORE:-$HOME}"
  export HOME="$SF/$1" XDG_DATA_HOME="$SF/$1/data" XDG_CONFIG_HOME="$SF/$1/config" XDG_CACHE_HOME="$SF/$1/cache"
  export HEARTH_DESKTOP_JSON="$STORE_LOOK" HEARTH_STORE_FIXTURE="$SF/${2:-.}" HEARTH_STORE_CATALOG="$SF/catalog.json"
}
store_done() {
  export HOME="$HOME_BEFORE_STORE"
  unset XDG_DATA_HOME XDG_CONFIG_HOME XDG_CACHE_HOME HEARTH_DESKTOP_JSON HEARTH_STORE_FIXTURE HEARTH_STORE_CATALOG HOME_BEFORE_STORE
}

store_home home-a
run store-1-home hearth-store 20 "$BIN/hearth-store-app"
# store-2 as the mockup has it: Command Center opened from Your apps (the same step as pressing its row there), so
# its back says "‹ Your apps".
run store-2-app-page hearth-store 0 "$BIN/hearth-store-app" --page yours --app command-center --show menu
run store-3-updates hearth-store 0 "$BIN/hearth-store-app" --page updates
run store-4-installing hearth-store 0 "$BIN/hearth-store-app" --demo-progress-app hearth-ai --demo-progress 62
run store-6-uninstall hearth-store 0 "$BIN/hearth-store-app" --page app:hearth-ai --show uninstall:hearth-ai
run store-7-controller hearth-store 0 "$BIN/hearth-store-app" --controller
run store-9-settings hearth-store 0 "$BIN/hearth-store-app" --page settings --focus s-test
run store-10-app-hearth-ai hearth-store 0 "$BIN/hearth-store-app" --page app:hearth-ai
run store-11-your-apps hearth-store 0 "$BIN/hearth-store-app" --page yours
run store-12-built-in hearth-store 0 "$BIN/hearth-store-app" --page built-in
run store-13-controller-updates hearth-store 0 "$BIN/hearth-store-app" --controller --page updates
# Opening it wrote none of your settings (red flag 9), and the daily check runs without a window.
[ -e "$XDG_CONFIG_HOME/nimbus/hearth-store.json" ] && echo "::error title=store::opening the Store wrote hearth-store.json: $(cat "$XDG_CONFIG_HOME/nimbus/hearth-store.json")"
"$BIN/hearth-store-app" --check > /tmp/store-check.log 2>&1 || echo "::error title=store --check::$(tail -c 800 /tmp/store-check.log | tr '\n' '|')"
echo "::notice title=store --check::$(tail -c 300 /tmp/store-check.log | tr '\n' ' ')"
store_done

store_home home-b
run store-5-repair hearth-store 0 "$BIN/hearth-store-app" --page repair:command-center
store_done

store_home home-c offline-world
run store-8-offline-empty hearth-store 0 "$BIN/hearth-store-app"
store_done
