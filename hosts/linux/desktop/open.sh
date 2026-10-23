#!/bin/sh
# Opens the user's terminal, file manager, or takes a screenshot, using whatever is installed.
# The terminal: $TERMINAL if you set one, then Hearth Terminal (hosts/hearth-terminal), then foot and the others.
# Paths go to the commands through the environment ("$HOME", "$SHOT"), never pasted into them, so a folder or
# name with a quote or a space in it (O'Brien, "Mes images") works.
#   open.sh terminal | files
#   open.sh screenshot             the whole screen (every screen)
#   open.sh screenshot-region      drag a box around what you want (slurp); Esc cancels
#   open.sh screenshot-clipboard   the whole screen, to the clipboard only (no file)
#   open.sh record | record-area | replay-save   Hearth Recorder's shortcuts (see the end)
# Screenshots are saved in Pictures/Screenshots, copied to the clipboard (wl-copy), and a notification says so
# (click it to open the picture).
first() { for c in "$@"; do command -v "${c%% *}" >/dev/null 2>&1 && { exec sh -c "$c"; }; done; }
have() { command -v "$1" >/dev/null 2>&1; }

# A free file name in Pictures/Screenshots: two in the same second get " (2)", " (3)"...: the name is taken
# (noclobber creates it only if it's free) before grim writes, so the second one can't overwrite the first.
new_shot() {
  dir="$(xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")/Screenshots"; mkdir -p "$dir"
  base="$dir/Screenshot $(date '+%F %H-%M-%S')"; SHOT="$base.png"; n=1
  while ! (set -C; : >"$SHOT") 2>/dev/null && [ "$n" -lt 100 ]; do n=$((n + 1)); SHOT="$base ($n).png"; done
  export SHOT
}

# Copy the picture and say where it went. notify-send -A waits for the click (in the background), so the
# notification opens the picture; an older notify-send without -A just shows it.
done_shot() {
  have wl-copy && wl-copy --type image/png <"$SHOT"
  name=$(basename "$SHOT")
  (
    act=$(notify-send -a Screenshot -i "$SHOT" -A default=Open -A folder="Show in folder" \
      "Screenshot saved" "$name, also copied to the clipboard" 2>/dev/null) \
      || notify-send -a Screenshot -i "$SHOT" "Screenshot saved" "$name, also copied to the clipboard" 2>/dev/null
    case "$act" in
      default) xdg-open "$SHOT" ;;
      folder) sh "$0" files-at "$SHOT" ;;
    esac
  ) >/dev/null 2>&1 &
}

fail_shot() { rm -f "$SHOT"; [ -n "$1" ] && notify-send -a Screenshot "Screenshot" "$1" 2>/dev/null; exit 1; }

case "$1" in
  terminal)   first "$TERMINAL" hearth-terminal foot alacritty kitty wezterm konsole gnome-terminal xfce4-terminal xterm ;;
  files)      first "/usr/bin/hearth-filemanager" 'xdg-open "$HOME"' dolphin nautilus thunar pcmanfm-qt pcmanfm nemo ;;
  # A file's folder in the file manager (the screenshot notification's "Show in folder").
  files-at)   export FILE="$2"; DIR=$(dirname "$2"); export DIR
              first 'quickshell ipc -p "$NIMBUS_SHARE/desktop/shell" call files open "$FILE"' \
                    'dbus-send --session --dest=org.freedesktop.FileManager1 --type=method_call /org/freedesktop/FileManager1 org.freedesktop.FileManager1.ShowItems array:string:"file://$FILE" string:""' \
                    'xdg-open "$DIR"' ;;
  screenshot)
              if ! have grim; then first "spectacle -b -n"; exit 1; fi    # spectacle names its own files
              new_shot
              grim "$SHOT" || fail_shot "Couldn't take a screenshot"
              done_shot ;;
  screenshot-region)
              have grim || { first "spectacle -b -n -r"; exit 1; }
              have slurp || { notify-send -a Screenshot "Screenshot" "Choosing an area needs slurp, which isn't installed. Print Screen takes the whole screen." 2>/dev/null; exit 1; }
              # Choose first, then make the file (Esc leaves nothing behind).
              geo=$(slurp -d -b '#00000066' -c '#ff8a3d' -s '#ff8a3d22' -w 2 2>/dev/null) || exit 0
              [ -n "$geo" ] || exit 0
              new_shot
              grim -g "$geo" "$SHOT" || fail_shot "Couldn't take a screenshot"
              done_shot ;;
  screenshot-clipboard)
              have grim && have wl-copy || exit 1
              grim - | wl-copy --type image/png
              notify-send -a Screenshot -t 2500 "Screenshot copied" "Paste it into any app" 2>/dev/null ;;
  # ---- Hearth Recorder (hosts/hearth-recorder), from rc.xml's Super+Alt+R / Super+Alt+Shift+R / Super+Alt+G: it says
  # what happened with its own notifications (Recording, Recording saved with Open and Show in folder, Replay saved).
  record|record-area|replay-save)
              if ! have hearth-recorder; then notify-send -a Recorder "Recorder" "Hearth Recorder isn't installed" 2>/dev/null; exit 1; fi
              case "$1" in
                record)      exec hearth-recorder --toggle ;;
                record-area) exec hearth-recorder --toggle --area ;;
                replay-save) exec hearth-recorder --save-replay ;;
              esac ;;
  # ---- end of Hearth Recorder
esac
