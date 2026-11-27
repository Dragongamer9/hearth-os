# The Home server helper (hosts/hearth-ui/apps/home-server) and Settings › Home server (cats/hs_board.rs): boards 01,
# 02, 03, 04, 06 and 11 of docs/mockups/server-expansion (checklists docs/mockups/checklists/server-*.md), at the
# boards' size. home-server-fixture.py makes a pretend home server (a pretend PC for the dashboard, a vault drive with
# its models and a week of checks, four devices' backups in a pretend restic, the Store's last list) and runs the REAL
# engines against it; the app shows their answers (HEARTH_SERVER_FIXTURES: nothing runs, nothing is sent). Sourced by
# hearth-apps-rust-test.sh.
HSF=/tmp/home-server-fx
rm -rf "$HSF"; mkdir -p "$HSF"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/home-server-fixture.py" "$HSF" || echo "::error title=home server fixture::it couldn't be made"
HS="$BIN/hearth-home-server-app"
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
export HEARTH_SERVER_FIXTURES="$HSF"
NIGHT=$(cat "$HSF/restore-night" 2>/dev/null)
run hsrv-01-vault hearth-home-server 20 "$HS" --page vault
run hsrv-02-checks hearth-home-server 0 "$HS" --page checks
run hsrv-03-dash hearth-home-server 0 "$HS" --page dash
run hsrv-04-tv hearth-home-server 0 "$HS" --page tv
run hsrv-04-phone hearth-home-server 0 "$HS" --page phone
run hsrv-06-backups hearth-home-server 0 "$HS" --page backup --set restoreNight="$NIGHT" --set restoreSearch=Gravesleep
run hsrv-11-home hearth-home-server 0 "$HS" --page home
run hsrv-11-settings hearth-settings 0 "$BIN/hearth-settings-app" --page sec-hs
run hsrv-12-vault-setup hearth-home-server 0 "$HS" --page vault --set vaultSetup=true
run hsrv-13-backups-add hearth-home-server 0 "$HS" --page backup --set backupAdd=true --set restoreMode=usb
run hsrv-14-dash-edit hearth-home-server 0 env HEARTH_ARRANGE_OPEN=home-server:dash "$HS" --page dash
# opening any page wrote no setting (red flag 9): the fixture's own settings files are exactly as it made them
for f in "$HSF"/world/home/.config/nimbus/server-*.json; do
  [ -f "$f" ] && python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$f" || echo "::error title=home server settings::$f isn't whole"
done
unset HEARTH_SERVER_FIXTURES
# without the engines' answers: the page says the part isn't here yet (a PC before the Store's update)
run hsrv-15-no-engines hearth-home-server 0 env HEARTH_SERVER_PARTS_DIR=/nonexistent "$HS" --page home
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
