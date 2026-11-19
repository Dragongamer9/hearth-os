# The Fedora edition's Plasma menu entries that open Hearth's apps through a small helper
# (os-bases/anatase/system_files/usr/libexec/hearth): each started as its menu entry starts it, and pictured.
#   Windows apps (hearth-winapps-open, no file): Settings › Apps › Windows apps, the pure-Rust Settings
#   Join a friend's game (hearth-join, no link): KDE's dialog asks for the invite link; a link that isn't an invite
#   says so (hearth-discord join, the same as Debian's)
FX="$ROOT/os-bases/anatase/system_files/usr/libexec/hearth"
grep -q '^Exec=/usr/libexec/hearth/hearth-winapps-open %f' "$ROOT/os-bases/anatase/system_files/usr/share/applications/hearth-winapps.desktop" \
  && ! grep -q '^NoDisplay=true' "$ROOT/os-bases/anatase/system_files/usr/share/applications/hearth-winapps.desktop" \
  || echo "::error title=fedora menu::hearth-winapps.desktop (Fedora's) isn't the shown entry that opens hearth-winapps-open"
PATH="$BIN:$PATH" run winapps-1-from-the-menu hearth-settings 0 sh "$FX/hearth-winapps-open"

# kdialog's window (its app id is its own: org.kde.kdialog or kdialog), started like run() does, then pictured.
dialog_shot() {
  local name=$1; shift
  local pid i ff=-1 t0
  t0=$(now_ms)
  QT_QPA_PLATFORM=wayland "$@" >"/tmp/$name.log" 2>&1 &
  pid=$!
  for i in $(seq 1 800); do
    if swaymsg -t get_tree 2>/dev/null | grep -q '"app_id": "[^"]*kdialog'; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.01
  done
  sleep 1.5
  grim "$ROOT/shots/$name.png" 2>/dev/null || true
  echo "SHOT $name firstframe=$ff" >> "$ROOT/results.txt"
  [ "$ff" -ge 0 ] || echo "::error title=$name::its dialog never came up: $(tail -c 800 /tmp/$name.log | tr '\n' '|')"
  pkill -f kdialog 2>/dev/null; kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.3
}
if command -v kdialog >/dev/null; then
  dialog_shot join-1-from-the-menu sh "$FX/hearth-join"
  dialog_shot join-2-not-an-invite sh "$FX/hearth-join" "https://hearthos.fun/join#not-a-real-invite"
else
  echo "::error title=fedora menu::kdialog isn't installed here, so Join's dialog couldn't be tried"
fi
