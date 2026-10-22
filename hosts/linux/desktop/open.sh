#!/bin/sh
# Opens the user's terminal, file manager, or takes a screenshot, using whatever is installed.
# The terminal: $TERMINAL if you set one, then Hearth Terminal (hosts/hearth-terminal), then foot and the others.
first() { for c in "$@"; do command -v "${c%% *}" >/dev/null 2>&1 && { exec sh -c "$c"; }; done; }
case "$1" in
  terminal)   first "$TERMINAL" hearth-terminal foot alacritty kitty wezterm konsole gnome-terminal xfce4-terminal xterm ;;
  files)      first "/usr/bin/hearth-filemanager" "xdg-open $HOME" dolphin nautilus thunar pcmanfm-qt pcmanfm nemo ;;
  screenshot) dir="$(xdg-user-dir PICTURES 2>/dev/null || echo "$HOME/Pictures")/Screenshots"; mkdir -p "$dir"
              first "grim '$dir/Screenshot $(date '+%F %H-%M-%S').png'" "spectacle -b -n" ;;
esac
