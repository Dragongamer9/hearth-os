# Hearth Shield (hosts/hearth-ui/apps/shield; board docs/mockups/open-source-apps/15-shield.png, checklist
# docs/mockups/checklists/open-source-15-shield.md): the board's day as AdGuard Home's API answers it and as
# hearth-server-dns keeps Hearth's side (shield-fixture.py), read by the app's own code (data.rs). 1 Today (the board);
# 2 7 days; 3 a device's page; 4 What was asked; 5 Block lists; 6 Always allow; 7 Family rules; 8 Settings;
# 9 Shield off (Turn it on); 10 Edit layout on Today.
# the boards are 1920 × 1080: the screen is that size for these pictures, and back to CI's own after
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1; sleep 0.5
SFX=/tmp/shield-fx.json; SCFG=/tmp/shield-cfg; rm -rf "$SCFG"
python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/shield-fixture.py" "$SFX" "$SCFG" || echo "::error title=shield fixture::it couldn't be made"
S="$BIN/hearth-shield-app"
shield() { n=$1; shift; run "shield-$n" hearth-shield 0 env XDG_CONFIG_HOME="$SCFG" HEARTH_SHIELD_FIXTURE="$SFX" HEARTH_SHIELD_SERVER="Nvidia PC" HEARTH_SHIELD_HOUR=23 "$@"; }
SHOT_WAIT=3 shield 1-today "$S"
shield 2-seven-days "$S" --range 7
shield 3-device "$S" --page device --device 192.168.1.20
shield 4-what-was-asked "$S" --page log
shield 5-block-lists "$S" --page lists
shield 6-always-allow "$S" --page allow
shield 7-family-rules "$S" --page family
shield 8-settings "$S" --page settings
python3 - "$SFX" <<'PY'
import json, sys
f = json.load(open(sys.argv[1])); f.pop('/control/stats'); f['status']['ads']['on'] = False; f['status']['ads']['running'] = False
json.dump(f, open(sys.argv[1] + '.off', 'w'))
PY
run shield-9-off hearth-shield 0 env XDG_CONFIG_HOME="$SCFG" HEARTH_SHIELD_FIXTURE="$SFX.off" HEARTH_SHIELD_SERVER="Nvidia PC" "$S"
SHOT_WAIT=3 shield 10-edit-layout env HEARTH_ARRANGE_OPEN=shield:today HEARTH_ARRANGE_PICK=chart "$S"
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1; sleep 0.3
