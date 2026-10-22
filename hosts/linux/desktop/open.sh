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
              SHOT="$dir/Screenshot $(date '+%F %H-%M-%S').png"; export SHOT
              first 'grim "$SHOT"' "spectacle -b -n" ;;
esac
