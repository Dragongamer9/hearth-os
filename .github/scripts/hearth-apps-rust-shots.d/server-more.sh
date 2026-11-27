# The Home server helper's Game servers, Game cache, Remote play, Media and PCs (hosts/hearth-ui/apps/server-more;
# boards docs/mockups/server-expansion/05, 07, 08, 09, 10; checklists docs/mockups/checklists/server-*.md). Pretend
# servers: server-hub-fixture.py writes what each source answers on the boards' night (Podman's game servers and their
# players, the cache's numbers, Sunshine's stream, Jellyfin, the other Hearth PCs) and Mantel's own hearth-server-hub
# builds each page from them, as on a PC. 1-5 the five boards; 9 Edit layout on Game servers; w1, w4 in the Home server helper's own window.
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1; sleep 0.5
HFX=/tmp/server-hub-fx.json; HCFG=/tmp/server-hub-cfg; rm -rf "$HCFG"
TZ=America/Chicago python3 "$ROOT/.github/scripts/hearth-apps-rust-shots.d/server-hub-fixture.py" "$HFX" "$HCFG" || echo "::error title=server-more fixture::it couldn't be made"
for v in games cache play media pcs; do
  out=$(TZ=America/Chicago XDG_CONFIG_HOME="$HCFG" XDG_STATE_HOME=/tmp/server-hub-state HEARTH_SERVER_HUB_FIXTURE="$HFX" python3 "$ROOT/hosts/linux/desktop/shell/hearth-server-hub" view "$v" 2>&1 | tail -c 300)
  case "$out" in *'"ok": true'*) ;; *) echo "::error title=hearth-server-hub view $v::$out" ;; esac
done
SM="$BIN/hearth-server-more-app"
srv() { n=$1; shift; run "server-more-$n" hearth-server-more 0 env TZ=America/Chicago XDG_CONFIG_HOME="$HCFG" XDG_STATE_HOME=/tmp/server-hub-state HEARTH_SERVER_HUB_FIXTURE="$HFX" HEARTH_SHELL_DIR="$ROOT/hosts/linux/desktop/shell" HEARTH_SERVER_NAME="Nvidia PC · RTX 5070 12 GB" HEARTH_SERVER_UP_DAYS=9 "$@"; }
SHOT_WAIT=4 srv 1-game-servers "$SM" --page games
SHOT_WAIT=4 srv 2-game-cache "$SM" --page cache
SHOT_WAIT=4 srv 3-remote-play "$SM" --page play
SHOT_WAIT=4 srv 4-media "$SM" --page media
SHOT_WAIT=4 srv 5-pcs "$SM" --page pcs
SHOT_WAIT=4 srv 9-edit-layout env HEARTH_ARRANGE_OPEN=home-server:srv-games HEARTH_ARRANGE_PICK=cards "$SM" --page games
# The same pages in the Home server helper's own window (apps/home-server, pages/more.rs), its sidebar with them
hsw() { n=$1; shift; run "server-more-w$n" hearth-home-server 0 env TZ=America/Chicago XDG_CONFIG_HOME="$HCFG" XDG_STATE_HOME=/tmp/server-hub-state HEARTH_SERVER_HUB_FIXTURE="$HFX" HEARTH_SHELL_DIR="$ROOT/hosts/linux/desktop/shell" "$BIN/hearth-home-server-app" "$@"; }
SHOT_WAIT=4 hsw 1-game-servers --page games
SHOT_WAIT=4 hsw 4-media --page media
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1; sleep 0.3
