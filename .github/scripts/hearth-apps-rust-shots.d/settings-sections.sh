# The Settings redesign (docs/mockups/settings-cleanup, approved Nov 23, 2026): Home and every section's front page
# (cats/sections.rs, cats/ai.rs), with the values this CI PC has (hearth-settings-values reads them; none are made up),
# to put next to the mockups. Sourced by hearth-apps-rust-test.sh.
look steam dark
SEC_ENV="HEARTH_SHELL_DIR=$ROOT/hosts/linux/desktop/shell"
run settings-sec-01-home hearth-settings 0 env "$SEC_ENV" "$BIN/hearth-settings-app" --page home
n=2
for s in system net bt link pers apps gaming ai accounts hs privacy a11y updates; do
  run "settings-sec-$(printf %02d $n)-$s" hearth-settings 0 env "$SEC_ENV" "$BIN/hearth-settings-app" --page "sec-$s"
  n=$((n + 1))
done
