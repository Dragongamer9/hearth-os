# Family and friends with Moonlight (hearth-link's moonlight.rs; Settings › Gaming › Family and friends and
# hearth-family-app): the switch and what turning it on allows, adding a person with a code (and its QR code), the
# people and their choices with who's connected now, the "wants to connect" question, the "is connected" notice with
# Stop sharing now, and the list of PCs on the connecting side; each in the mouse look (a) and the controller look (b).
# CI has no Hearth Link running: the answers come from fixtures (Settings: $HEARTH_SETTINGS_FIXTURES, the app:
# $HEARTH_FAMILY_FIXTURE), shaped as hearth-link answers them.
FAM=/tmp/family-fx
rm -rf "$FAM"; mkdir -p "$FAM"
NOWMS=$(( $(date +%s) * 1000 ))
UNTIL=$(( NOWMS + 42 * 60000 ))
DAD=3f9a6c0e1b2d4f5a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c
SAM=9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0f9e8d
famstatus() { # the "This PC" side: ON PEOPLE INVITE SESSION
  printf '{"on": %s, "readable": true, "people": %s, "invite": %s, "session": %s, "asking": false, "name": "Chris-Desktop", "last": null}\n' "$1" "$2" "$3" "$4" > "$FAM/hearth-link-call-moonlight.share.status.json"
}
FRIENDS='{"friends": [{"id": "'$DAD'", "name": "Dad'"'"'s PC", "online": true, "sharing": true, "what": "desktop", "ask": true, "limitMinutes": 0},
 {"id": "'$SAM'", "name": "Sam'"'"'s Deck", "online": false, "sharing": null, "what": "games", "ask": null, "limitMinutes": null}], "connected": null, "moonlight": true, "last": null}'
echo "$FRIENDS" > "$FAM/hearth-link-call-moonlight.friends.json"
PEOPLE='[{"id": "'$DAD'", "name": "Dad'"'"'s PC", "always": false, "what": "desktop", "limitMinutes": 60, "online": true, "relation": "family"},
 {"id": "'$SAM'", "name": "Sam'"'"'s Deck", "always": true, "what": "games", "limitMinutes": 0, "online": false, "relation": "friend"}]'
fam() { # NAME LAYOUT ARGS...: Settings' page
  n=$1; lay=$2; shift 2
  run "family-$n-$lay" hearth-settings 0 env HEARTH_SETTINGS_FIXTURES="$FAM" "$BIN/hearth-settings-app" --page family --layout "$lay" "$@"
}
FAM_BEFORE=$(ls -la "$HOME/.config/nimbus" 2>/dev/null | md5sum)
for lay in a b; do
  # 1. Off, and Turn on asked: what it allows, in plain words.
  famstatus false '[]' null null
  fam 01-turn-on-explained "$lay" --show family-confirm
  # 2. On, adding a person: the 6 digits, the code for anywhere and its QR code.
  famstatus true '[{"id": "'$DAD'", "name": "Dad'"'"'s PC", "always": false, "what": "desktop", "limitMinutes": 0, "online": true, "relation": "family"}]' \
    '{"code": "482913", "until": '$((NOWMS + 300000))', "netCode": "K7M-4QX-92HC", "netUntil": '$((NOWMS + 600000))', "triesLeft": 5}' null
  fam 02-add-a-person-code "$lay"
  # 3. The people (Dad's PC's choices open), Dad's PC connected now with Stop sharing now, and the other side's list.
  famstatus true "$PEOPLE" null '{"peer": "'$DAD'", "who": "Dad'"'"'s PC", "what": "desktop", "until": '$UNTIL', "started": '$((NOWMS - 18 * 60000))', "connected": true}'
  fam 03-people-and-connected "$lay"
done
# The connecting side's list in Settings (lower down the same page).
famstatus false '[]' null null
fam 04-their-pcs a
# hearth-family-app: the question, the notice, the list (a: mouse, b: controller).
FFX="$FAM/app.json"
python3 - "$FFX" "$FAM/hearth-link-call-moonlight.friends.json" <<'PY'
import json, sys
f = json.load(open(sys.argv[2]))
json.dump({"moonlight.friends": f}, open(sys.argv[1], "w"))
PY
for lay in a b; do
  rest=0; [ "$lay" = a ] && rest=20
  run "family-05-wants-to-connect-$lay" hearth-family-ask 0 env HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --ask --name "Chris's PC" --what desktop --limit 60 --layout "$lay"
  run "family-06-connected-notice-$lay" hearth-family-notice "$rest" env HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --notice --name "Chris's PC" --what desktop --until "$UNTIL" --layout "$lay"
  run "family-07-friends-list-$lay" hearth-family "$rest" env HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --page friends --layout "$lay"
done
# Connected to Dad's PC (the list's top row), and nobody yet.
python3 - "$FFX" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["moonlight.friends"]["connected"] = {"peer": d["moonlight.friends"]["friends"][0]["id"], "name": "Dad's PC", "started": 0, "until": 0, "relay": False}
json.dump(d, open(sys.argv[1], "w"))
PY
run family-08-friends-connected-b hearth-family 0 env HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --page friends --layout b
echo '{"moonlight.friends": {"friends": [], "connected": null, "moonlight": true}}' > "$FFX"
run family-09-friends-none-a hearth-family 0 env HEARTH_FAMILY_FIXTURE="$FFX" "$BIN/hearth-family-app" --page friends
# Opening them wrote nothing (red flag 9): no family-moonlight.json, nothing else in ~/.config/nimbus changed.
[ -e "$HOME/.config/nimbus/family-moonlight.json" ] && echo "::error title=family::opening Family and friends wrote family-moonlight.json"
[ "$(ls -la "$HOME/.config/nimbus" 2>/dev/null | md5sum)" = "$FAM_BEFORE" ] || echo "::warning title=family::~/.config/nimbus changed while Family and friends was pictured"
