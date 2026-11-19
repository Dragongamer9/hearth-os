# Hearth Settings › Home, Hearth Link, Home server and Updates & recovery (cats/home.rs, link.rs, hs/, updates.rs).
# The helpers answer from settings-hs-home.json here (HEARTH_SETTINGS_FIXTURES: believable answers in the same JSON
# the helpers print, so the pictures show filled pages; a container has no Samba, Tailscale or paired devices).
# PAGE:a=b,c opens a page with its own parts open (a share's settings, an expander).
S="$BIN/hearth-settings-app"
export HEARTH_SETTINGS_FIXTURES="$ROOT/.github/scripts/hearth-apps-rust-shots.d/settings-hs-home.json"
# A few changes in the change history (only in CI's own home), for Changed lately and All changes.
mkdir -p "$HOME/.local/state/nimbus"
now=$(date +%s)
cat > "$HOME/.local/state/nimbus/settings-history.json" <<JSON
{"next": 5, "changes": [
 {"id": 1, "file": "desktop.json", "key": "lightMode", "from": false, "to": true, "who": "you", "at": $((now - 6 * 86400))},
 {"id": 2, "file": "desktop.json", "key": "sleepMinutes", "from": 30, "to": 0, "who": "you", "at": $((now - 86400))},
 {"id": 3, "file": "desktop.json", "key": "nightLight", "from": false, "to": true, "who": "you", "at": $((now - 3 * 3600))},
 {"id": 4, "file": "desktop.json", "key": "accent", "from": "", "to": "Ember orange", "who": "you", "at": $((now - 120))}]}
JSON
run hshome-01-home hearth-settings 0 "$S"
run hshome-02-quick hearth-settings 0 "$S" --page quick
run hshome-03-changes hearth-settings 0 "$S" --page changes
run hshome-04-link hearth-settings 0 "$S" --page link:peer=a3
run hshome-05-devices hearth-settings 0 "$S" --page devices
run hshome-06-device hearth-settings 0 "$S" --page devices:detail=a1
run hshome-07-updates hearth-settings 0 "$S" --page updates
run hshome-08-updates-decky hearth-settings 0 "$S" --page updates:x:decky
run hshome-09-backup hearth-settings 0 "$S" --page backup
run hshome-10-hs-overview hearth-settings 0 "$S" --page hs-overview
run hshome-11-hs-setup hearth-settings 0 "$S" --page hs-setup:step=2
run hshome-12-hs-server hearth-settings 0 "$S" --page hs-server
run hshome-13-hs-aiengine hearth-settings 0 "$S" --page hs-aiengine:fam:qwen3-30b-abl
run hshome-14-hs-shares hearth-settings 0 "$S" --page hs-shares:sel=s1
run hshome-15-hs-shares-add hearth-settings 0 "$S" --page hs-shares:adding
run hshome-16-hs-files hearth-settings 0 "$S" --page hs-files
run hshome-17-hs-people hearth-settings 0 "$S" --page hs-people:sel=mia
run hshome-18-hs-family hearth-settings 0 "$S" --page hs-family
run hshome-19-hs-services hearth-settings 0 "$S" --page hs-services:x:media,x:voices
run hshome-20-hs-ai hearth-settings 0 "$S" --page hs-ai
run hshome-21-hs-aidevices hearth-settings 0 "$S" --page hs-aidevices
run hshome-22-hs-apps hearth-settings 0 "$S" --page hs-apps:x:minecraft
run hshome-23-hs-phone hearth-settings 0 "$S" --page hs-phone
run hshome-24-hs-gaming hearth-settings 0 "$S" --page hs-gaming
run hshome-25-hs-drives hearth-settings 0 "$S" --page hs-drives
run hshome-26-hs-backups hearth-settings 0 "$S" --page hs-backups:sel=b1
run hshome-27-hs-safety hearth-settings 0 "$S" --page hs-safety
run hshome-28-hs-alerts hearth-settings 0 "$S" --page hs-alerts
run hshome-29-hs-server-pin hearth-settings 0 "$S" --page hs-server:pinOpen
unset HEARTH_SETTINGS_FIXTURES
# Without the helpers' answers: the empty and "couldn't read" states.
run hshome-30-hs-overview-empty hearth-settings 0 "$S" --page hs-overview
run hshome-31-link-empty hearth-settings 0 "$S" --page link
rm -f "$HOME/.local/state/nimbus/settings-history.json"
