#!/bin/bash
# Hearth Games' desktop icons (games-desktop-icons.yml, job "app"): the real hearth-gamestore window under Xvfb on a
# home with one GOG game installed through Hearth (Witcher Tales, a stand-in: its goggame-<id>.info, library.json's
# gogInstalls and the GOG account's cached list with its cover). Driven like a person: the library card's menu,
# Add to desktop, the menu again (Remove from desktop), then Uninstall. Checks the launcher it made (name, the
# game's art kept in Hearth's folder, Exec=nimbus --play-now <id>, executable) and that uninstalling took off only
# Hearth's icon: your own files and your copy of the icon stay. The Desktop folder before, after Add and after
# Uninstall goes to the "desktops" job (state-*.tgz) to be shown on Mantel's and Plasma's desktops.
#   games-desktop-icons-app.sh BIN OUT
set +e +o pipefail
BIN=$(cd "$(dirname "$1")" && pwd)/$(basename "$1"); OUT=${2:-shots}
mkdir -p "$OUT"; OUT=$(cd "$OUT" && pwd)
exec > >(tee "$OUT/step.log") 2>&1
fail() { echo "::error title=$1::$2"; touch "$OUT/FAILED"; }
ok() { echo "ok: $1"; }

export HOME=/tmp/home XDG_RUNTIME_DIR=/tmp/xdg-games
unset XDG_CONFIG_HOME XDG_DATA_HOME XDG_DESKTOP_DIR
rm -rf "$HOME" "$XDG_RUNTIME_DIR"; mkdir -p "$HOME" "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
N="$HOME/.local/share/nimbus"; G="$HOME/Games/GOG/Witcher Tales"; D="$HOME/Desktop"
mkdir -p "$N/cache" "$G" "$D/Projects" "$HOME/art" "$HOME/.config/nimbus"
# The game's cover (a stand-in picture) and its files as gogdl leaves them.
convert -size 300x450 gradient:'#d8742c-#2a1206' -gravity south -font DejaVu-Sans-Bold -pointsize 30 -fill white -annotate +0+46 'Witcher Tales' "$HOME/art/witcher.png" 2>/dev/null \
  || convert -size 300x450 xc:'#b5521f' "$HOME/art/witcher.png"
convert -size 300x450 gradient:'#3a7bd5-#0b1a33' -gravity south -pointsize 30 -fill white -annotate +0+46 'Pixel Keep' "$HOME/art/pixel.png" 2>/dev/null || cp "$HOME/art/witcher.png" "$HOME/art/pixel.png"
printf '{"gameId": "1207658924", "name": "Witcher Tales", "playTasks": [{"isPrimary": true, "type": "FileTask", "path": "witcher.exe", "name": "Witcher Tales"}]}\n' > "$G/goggame-1207658924.info"
: > "$G/witcher.exe"
printf '{"gogInstalls": {"1207658924": "%s"}}\n' "$G" > "$N/library.json"
now=$(date +%s000)
printf '{"fetched": %s, "games": [{"id": "1207658924", "title": "Witcher Tales", "art": {"cover": "%s"}, "added": 0, "lastPlayed": 0, "note": null}, {"id": "1207658925", "title": "Pixel Keep", "art": {"cover": "%s"}, "added": 0, "lastPlayed": 0, "note": null}]}\n' \
  "$now" "$HOME/art/witcher.png" "$HOME/art/pixel.png" > "$N/cache/gog-owned.json"
# Your own things on the desktop: a folder, a note and a launcher you made.
printf 'shopping list\n' > "$D/notes.txt"
printf '[Desktop Entry]\nType=Application\nName=My Game\nExec=/usr/bin/true\nIcon=applications-games\n' > "$D/My Game.desktop"; chmod 755 "$D/My Game.desktop"
state() { tar -C / -czf "$OUT/state-$1.tgz" "${HOME#/}/Desktop" "${HOME#/}/.local/share/nimbus/desktop-icons" "${HOME#/}/art" 2>/dev/null || tar -C / -czf "$OUT/state-$1.tgz" "${HOME#/}/Desktop" "${HOME#/}/art"; }
listing() { (cd "$D" && find . -mindepth 1 -printf '%P %m\n' | sort) > "$OUT/desktop-$1.txt"; }
state before; listing before

export DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 NO_AT_BRIDGE=1
Xvfb :99 -screen 0 1280x800x24 >/dev/null 2>&1 &
sleep 2
shot() { sleep "${2:-2}"; import -window root "$OUT/$1.png" || echo "no picture $1"; }
front() { w=$(xdotool search --name "^Hearth Games\$" | tail -1); [ -n "$w" ] && { xdotool windowactivate --sync "$w" 2>/dev/null || xdotool windowfocus --sync "$w" 2>/dev/null; }; true; }
key() { xdotool key --delay 120 "$@"; sleep 1; }
# A stand-in nimbus so the window's Play is offered (not used here).
mkdir -p /tmp/fakebin; printf '#!/bin/sh\necho "$*" >> "$HOME/nimbus-calls"\n' > /tmp/fakebin/nimbus; chmod +x /tmp/fakebin/nimbus
export NIMBUS_APP=/tmp/fakebin/nimbus PATH="/tmp/fakebin:$PATH"

dbus-run-session -- "$BIN" > "$OUT/gamestore.log" 2>&1 &
sleep 14; front
shot app-1-library 1
# The first card (installed games come first): the Library tab again puts the focus on the store chips; Down is the
# first card; then the Menu key (as a right-click or Start on a controller).
firstcard() { key Next; sleep 1; key Prior; sleep 1; key Down; }
firstcard
key Menu
shot app-2-card-menu 1
# Add to desktop is the second item (after Play).
key Down
key Return
shot app-3-added 2
F="$D/Witcher Tales.desktop"
if [ -f "$F" ]; then
  ok "the launcher is on the desktop"
  cp "$F" "$OUT/witcher-tales.desktop"
  grep -qx 'Exec=nimbus --play-now gog:1207658924' "$F" || fail launcher "its Exec line isn't nimbus --play-now gog:1207658924: $(grep ^Exec "$F")"
  grep -qx 'Name=Witcher Tales' "$F" || fail launcher "its name isn't the game's: $(grep ^Name "$F")"
  grep -qx 'Type=Application' "$F" || fail launcher "not an application launcher"
  grep -qx 'X-Hearth-Game=gog:1207658924' "$F" || fail launcher "no X-Hearth-Game mark"
  icon=$(sed -n 's/^Icon=//p' "$F")
  case "$icon" in "$N/desktop-icons/"*) [ -s "$icon" ] && ok "its picture is kept in Hearth's folder ($icon)" || fail launcher "its picture $icon isn't there" ;;
    *) fail launcher "its picture isn't the game's art kept in Hearth's folder: Icon=$icon" ;; esac
  cmp -s "$icon" "$HOME/art/witcher.png" || fail launcher "its picture isn't the game's cover"
  [ "$(stat -c %a "$F")" = 755 ] && ok "executable (trusted)" || fail launcher "not executable: $(stat -c %a "$F")"
  python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); g=d["games"]["gog:1207658924"]; assert g["file"]==sys.argv[2], g; assert len(g["sha256"])==64' "$N/desktop-icons/made.json" "$F" \
    && ok "Hearth noted it as its own" || fail launcher "made.json doesn't note it: $(cat "$N/desktop-icons/made.json" 2>&1 | head -c 400)"
  echo "::notice title=The launcher::$(tr '\n' '|' < "$F")"
else
  fail add "Add to desktop made no launcher: $(ls -la "$D" | tr '\n' '|') | $(tail -c 800 "$OUT/gamestore.log" | tr '\n' '|')"
fi
[ -e "$HOME/.config/nimbus/hearth-games.json" ] && fail settings "hearth-games.json was written without the tick box being changed"
listing added; state added
# The card's menu again: now it offers Remove from desktop.
firstcard
key Menu
shot app-4-card-menu-remove 1
key Escape
# You copy Hearth's icon and rename the copy: a file of yours now, which uninstalling must leave alone.
if [ -f "$F" ]; then sed 's/^Name=Witcher Tales$/Name=Witcher (my copy)/' "$F" > "$D/Witcher (my copy).desktop"; chmod 755 "$D/Witcher (my copy).desktop"; fi
# Uninstall from the card's menu: Play, Remove from desktop, Game details, Uninstall; then the confirmation's Uninstall.
firstcard
key Menu
key Down Down Down
shot app-5-menu-uninstall 1
key Return
shot app-6-confirm-uninstall 1
key Right Right
key Return
shot app-7-after-uninstall 4
[ -e "$F" ] && fail uninstall "Hearth's icon is still on the desktop after uninstalling"
[ -e "$G" ] && fail uninstall "the game wasn't uninstalled (its folder is there): $(tail -c 600 "$OUT/gamestore.log" | tr '\n' '|')"
for keep in notes.txt Projects "My Game.desktop" "Witcher (my copy).desktop"; do
  [ -e "$D/$keep" ] && ok "$keep stayed" || fail uninstall "uninstalling removed $keep, a file of yours"
done
python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); assert "gog:1207658924" not in d.get("games", {})' "$N/desktop-icons/made.json" 2>/dev/null \
  || fail uninstall "made.json still notes the icon: $(cat "$N/desktop-icons/made.json" 2>&1 | head -c 300)"
ls "$N/desktop-icons/" > "$OUT/desktop-icons-folder-after.txt" 2>&1
grep -q '^gog_1207658924\.' "$OUT/desktop-icons-folder-after.txt" && fail uninstall "Hearth's copy of the picture is still kept"
listing uninstalled; state uninstalled
# Its page now, not installed: Install with the "Add a desktop icon" tick box (off: never ticked).
firstcard
key Return
shot app-8-page-install-tickbox 2
key Escape
[ -e "$HOME/.config/nimbus/hearth-games.json" ] && fail settings "hearth-games.json was written by opening a page: $(cat "$HOME/.config/nimbus/hearth-games.json")"
pkill -f "$BIN" || true
echo "::notice title=Desktop folder::before: $(tr '\n' ',' < "$OUT/desktop-before.txt") | added: $(tr '\n' ',' < "$OUT/desktop-added.txt") | after uninstall: $(tr '\n' ',' < "$OUT/desktop-uninstalled.txt")"
echo "::notice title=Hearth Games log::$(grep -a 'desktop icons\|uninstall' "$N/../../state/nimbus/"*.log 2>/dev/null | tail -6 | cut -c1-200 | tr '\n' '|') $(tail -c 500 "$OUT/gamestore.log" | tr '\n' '|')"
out=""; for f in "$OUT"/*.png; do out="$out $(basename "$f" .png): $(identify -format '%wx%h spread %[fx:standard_deviation]' "$f") |"; done
echo "::notice title=Pictures::$out"
grep -c '^ok:' "$OUT/step.log" | sed 's/^/::notice title=Checks passed::/'
exit 0
