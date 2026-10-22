#!/bin/sh
# Opens the user's terminal, file manager, or takes a screenshot, using whatever is installed.
# The terminal: $TERMINAL if you set one, then Hearth Terminal (hosts/hearth-terminal), then foot and the others.
# Paths go to the commands through the environment ("$HOME", "$SHOT"), never pasted into them, so a folder or
# name with a quote or a space in it (O'Brien, "Mes images") works.
first() { for c in "$@"; do command -v "${c%% *}" >/dev/null 2>&1 && { exec sh -c "$c"; }; done; }
case "$1" in
  terminal)   first "$TERMINAL" hearth-terminal foot alacritty kitty wezterm konsole gnome-terminal xfce4-terminal xterm ;;
  files)      first "/usr/bin/hearth-filemanager" 'xdg-open "$HOME"' dolphin nautilus thunar pcmanfm-qt pcmanfm nemo ;;
  screenshot) dir="$(xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")/Screenshots"; mkdir -p "$dir"
              # Two in the same second get " (2)", " (3)"...: the name is taken (noclobber creates it only if it's
              # free) before grim writes, so the second one can't overwrite the first.
              base="$dir/Screenshot $(date '+%F %H-%M-%S')"; SHOT="$base.png"; n=1
              while ! (set -C; : >"$SHOT") 2>/dev/null && [ "$n" -lt 100 ]; do n=$((n + 1)); SHOT="$base ($n).png"; done
              export SHOT
              command -v grim >/dev/null 2>&1 || rm -f "$SHOT"   # spectacle names its own files
              first 'grim "$SHOT" || rm -f "$SHOT"' "spectacle -b -n" ;;
esac
