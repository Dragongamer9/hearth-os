# Hearth Passwords (hosts/hearth-ui/apps/passwords; checklist docs/mockups/checklists/open-source-16-passwords.md): board
# 16 on a made-up vault already opened (HEARTH_PASSWORDS_FIXTURE: nothing is sent, nothing written; the real unlocking,
# keys and encryption are proved by cargo test: bw.rs, vault.rs, kp.rs), at the board's size (1920 x 1080), and the
# lock screen as it really starts. Sourced by hearth-apps-rust-test.sh inside its headless screen.
PFX=/tmp/passwords-fx
rm -rf "$PFX"; mkdir -p "$PFX"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/oss-fixtures.py" passwords "$PFX" || echo "::error title=passwords fixture::it couldn't be made"
PW="$BIN/hearth-passwords-app"
PENV="XDG_DATA_HOME=$PFX/data XDG_CONFIG_HOME=$PFX/config"
# locked (as it opens on a PC), measured at rest on the usual screen
run passwords-0-locked hearth-passwords 20 env $PENV "$PW"
swaymsg output HEADLESS-1 mode 1920x1080 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1 || true
run passwords-1-board hearth-passwords 0 env $PENV HEARTH_PASSWORDS_FIXTURE="$PFX/vault.json" "$PW" --open gh --show maker
run passwords-2-item hearth-passwords 0 env $PENV HEARTH_PASSWORDS_FIXTURE="$PFX/vault.json" "$PW" --open gh
run passwords-3-card hearth-passwords 0 env $PENV HEARTH_PASSWORDS_FIXTURE="$PFX/vault.json" "$PW" --open card
run passwords-4-locked-big hearth-passwords 0 env $PENV "$PW"
run passwords-5-edit-window hearth-passwords 0 env $PENV HEARTH_PASSWORDS_FIXTURE="$PFX/vault.json" HEARTH_ARRANGE_OPEN=passwords:window HEARTH_ARRANGE_PICK=list "$PW" --open gh
run passwords-6-edit-item hearth-passwords 0 env $PENV HEARTH_PASSWORDS_FIXTURE="$PFX/vault.json" HEARTH_ARRANGE_OPEN=passwords:item HEARTH_ARRANGE_PICK=security "$PW" --open gh
grep -q '"makerLength": 24' "$PFX/config/nimbus/passwords.json" || echo "::error title=passwords settings::opening Hearth Passwords changed passwords.json"
# nothing of the vault in the open on disk
grep -rl "r7#Vq4mLp" "$PFX/data" "$PFX/config" 2>/dev/null && echo "::error title=passwords::a password was written to disk in the open"
swaymsg output HEADLESS-1 mode 1280x800 >/dev/null 2>&1 || swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1 || true
