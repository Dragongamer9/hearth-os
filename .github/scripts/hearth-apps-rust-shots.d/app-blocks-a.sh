# Every page of Settings, the Store, Files and Family and friends made of its real parts as blocks (the owner, Nov 24:
# every Hearth app modular; hearth_ui blocks.rs / arrange.rs, Settings' sections secs.rs): each page pictured as it
# always looks (nothing kept: exactly as before) and with Edit layout open on it (HEARTH_ARRANGE_OPEN=<app>:<page>: the
# live page with its grid, each part's bar and corner, the picked part's Look, Undo, Back to the original, Keep this),
# and a few laid out from a kept file. Each app's kept file lives in a config folder of its own here, so nothing of the
# person's (or the other pictures') is touched. Sourced by hearth-apps-rust-test.sh inside its headless screen.
BA=/tmp/blocks-a
rm -rf "$BA"; mkdir -p "$BA/empty" "$BA/kept/nimbus/apps"

# ------------------------------------------------------------------ Hearth Store (its pretend world, store.sh's)
SFB=/tmp/store-fx-blocks
rm -rf "$SFB"
python3 "$ROOT/.github/scripts/hearth-store-fixture.py" "$SFB" "$ROOT/os/system_files/usr/share/hearth/store/catalog.json" || echo "::error title=store fixture::it couldn't be made"
sb() { # NAME CONFIG ARGS...: the Store in the pretend world with home-a's apps, its own config folder
  n=$1; cfg=$2; shift 2
  run "app-blocks-a-$n" hearth-store 0 env HOME="$SFB/home-a" XDG_DATA_HOME="$SFB/home-a/data" XDG_CONFIG_HOME="$cfg" XDG_CACHE_HOME="$SFB/home-a/cache" \
    HEARTH_STORE_FIXTURE="$SFB/." HEARTH_STORE_CATALOG="$SFB/catalog.json" "$BIN/hearth-store-app" "$@"
}
# page id in the Store's BLOCKS, then the Store's own --page for it
for pg in "home:home" "apps:apps" "library:library" "parts:built-in" "system:system" "tests:tests" "settings:settings"; do
  id=${pg%%:*}; arg=${pg#*:}
  sb "store-$id" "$BA/empty" --page "$arg"
  HEARTH_ARRANGE_OPEN="store:$id" sb "store-$id-edit" "$BA/empty" --page "$arg"
done
sb store-app "$BA/empty" --page apps --app command-center
HEARTH_ARRANGE_OPEN=store:app HEARTH_ARRANGE_PICK=shots sb store-app-edit "$BA/empty" --page apps --app command-center
HEARTH_ARRANGE_OPEN=store:window HEARTH_ARRANGE_PICK=rail sb store-window-edit "$BA/empty"
# Laid out: the rail on the right, Home's own apps over the featured one, the picks hidden.
mkdir -p "$BA/store/nimbus/apps"
cat > "$BA/store/nimbus/apps/store.json" <<'J'
{"format": 1, "layout": {"blocks": {
  "window": {"title": {"x": 0, "y": 0, "w": 12, "h": 1}, "page": {"x": 0, "y": 1, "w": 11, "h": 11}, "rail": {"x": 11, "y": 1, "w": 1, "h": 11, "corners": 16}},
  "home": {"head": {"x": 0, "y": 0, "w": 12, "h": 1}, "yours": {"x": 0, "y": 1, "w": 12, "h": 4, "corners": 16, "accent": "#5b8cff"}, "featured": {"x": 0, "y": 5, "w": 12, "h": 7}, "picks": {"hidden": true}}}},
 "features": []}
J
sb store-kept-layout "$BA/store"

# ------------------------------------------------------------------ Hearth Files
fl() { # NAME CONFIG ARGS...
  n=$1; cfg=$2; shift 2
  run "app-blocks-a-$n" hearth-files 0 env XDG_CONFIG_HOME="$cfg" "$BIN/hearth-files-app" "$@"
}
fl files "$BA/empty" "$HOME" --view details --show pane
HEARTH_ARRANGE_OPEN=files:bars HEARTH_ARRANGE_PICK=toolbar fl files-bars-edit "$BA/empty" "$HOME" --view details --show pane
HEARTH_ARRANGE_OPEN=files:window HEARTH_ARRANGE_PICK=preview fl files-panes-edit "$BA/empty" "$HOME" --view details --show pane
# Laid out: the status bar at the top under the title bar, the toolbar at the bottom, the address bar rounded.
mkdir -p "$BA/files/nimbus/apps"
cat > "$BA/files/nimbus/apps/files.json" <<'J'
{"format": 1, "layout": {"blocks": {"bars": {
  "tabs": {"x": 0, "y": 0, "w": 12, "h": 1}, "status": {"x": 0, "y": 1, "w": 12, "h": 1},
  "address": {"x": 0, "y": 2, "w": 12, "h": 1, "corners": 16}, "panes": {"x": 0, "y": 3, "w": 12, "h": 8}, "toolbar": {"x": 0, "y": 11, "w": 12, "h": 1}}}},
 "features": []}
J
fl files-kept-layout "$BA/files" "$HOME" --view details

# ------------------------------------------------------------------ Family and friends (its three windows)
FB="$BA/family"; mkdir -p "$FB"
cat > "$FB/app.json" <<'J'
{"moonlight.friends": {"friends": [{"id": "3f9a6c0e1b2d4f5a6b7c8d9e0f1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c", "name": "Dad's PC", "online": true, "sharing": true, "what": "desktop", "ask": true, "limitMinutes": 0},
 {"id": "9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d3c2b1a0f9e8d", "name": "Sam's Deck", "online": false, "sharing": null, "what": "games", "ask": null, "limitMinutes": null}], "connected": null, "moonlight": true, "last": null}}
J
UNTIL_B=$(( $(date +%s) * 1000 + 42 * 60000 ))
fm() { # NAME APPID ARGS...
  n=$1; id=$2; shift 2
  run "app-blocks-a-$n" "$id" 0 env XDG_CONFIG_HOME="$BA/empty" HEARTH_FAMILY_FIXTURE="$FB/app.json" "$BIN/hearth-family-app" "$@"
}
fm family hearth-family --page friends
HEARTH_ARRANGE_OPEN=family:friends HEARTH_ARRANGE_PICK=people fm family-edit hearth-family --page friends
fm family-ask hearth-family-ask --ask --name "Dad's PC" --what desktop --limit 60
HEARTH_ARRANGE_OPEN=family:ask HEARTH_ARRANGE_PICK=buttons fm family-ask-edit hearth-family-ask --ask --name "Dad's PC" --what desktop --limit 60
fm family-notice hearth-family-notice --notice --name "Dad's PC" --what desktop --until "$UNTIL_B"
HEARTH_ARRANGE_OPEN=family:notice HEARTH_ARRANGE_PICK=stop fm family-notice-edit hearth-family-notice --notice --name "Dad's PC" --what desktop --until "$UNTIL_B"

# ------------------------------------------------------------------ Hearth Settings (the window, Home, each page's sections)
st() { # NAME CONFIG ARGS...
  n=$1; cfg=$2; shift 2
  run "app-blocks-a-$n" hearth-settings 0 env XDG_CONFIG_HOME="$cfg" "$BIN/hearth-settings-app" "$@"
}
HEARTH_ARRANGE_OPEN=settings:window HEARTH_ARRANGE_PICK=cats st settings-window-edit "$BA/empty" --page home
HEARTH_ARRANGE_OPEN=settings:home st settings-home-edit "$BA/empty" --page home
for page in display sound power notifications datetime languages region accessibility users signin fonts bluetooth wifi drives; do
  st "settings-$page" "$BA/empty" --page "$page"
  HEARTH_ARRANGE_OPEN="settings:$page" st "settings-$page-edit" "$BA/empty" --page "$page"
done
# The kits' pages (secs.rs Plain / SecVec / Rows: drawn block by block while laid out or arranged), the section front
# pages and Quick's tiles (parts_dyn), and the AI page: with Edit layout open.
for page in privacy firewall kde-style gaming steam controller apps autostart services hs-overview hs-shares link updates \
            sec-system sec-gaming quick keyboard mouse printers bluetooth-devices usb-devices vpn hotspot net-details \
            kwin-effects desktop-layout sec-ai; do
  HEARTH_ARRANGE_OPEN="settings:$page" st "settings-$page-edit" "$BA/empty" --page "$page"
done
# Laid out: the categories on the right, the title over the whole width; Date & time's Clock section rounded in blue.
mkdir -p "$BA/settings/nimbus/apps"
cat > "$BA/settings/nimbus/apps/settings.json" <<'J'
{"format": 1, "layout": {"blocks": {
  "window": {"title": {"x": 0, "y": 0, "w": 12, "h": 1}, "brand": {"x": 9, "y": 1, "w": 3, "h": 1}, "search": {"x": 9, "y": 2, "w": 3, "h": 1},
             "cats": {"x": 9, "y": 3, "w": 3, "h": 9, "corners": 16, "seeThrough": 15}, "page": {"x": 0, "y": 1, "w": 9, "h": 11}},
  "datetime": {"clock": {"x": 0, "y": 0, "w": 12, "h": 4, "corners": 16, "accent": "#5b8cff"}}}},
 "features": []}
J
st settings-kept-layout "$BA/settings" --page datetime
