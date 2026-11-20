# Every KDE System Settings module as a Hearth Settings page (cats/kde.rs, hearth-kde-settings, kde-settings.json;
# docs/mockups/checklists/settings-kde-all.md). KDE's files come from settings-kde-all/config (a Plasma user's),
# what KDE's programs would answer (installed themes, on Plasma or not) from settings-kde-all/*.json. Opening a page
# writes nothing: checked below.
S="$BIN/hearth-settings-app"
KF="$ROOT/.github/scripts/hearth-apps-rust-shots.d/settings-kde-all"
KC=/tmp/kde-cfg
rm -rf "$KC"; mkdir -p "$KC"; cp -a "$KF/config/." "$KC/"
( cd "$KC" && find . -type f | sort | xargs sha256sum ) > /tmp/kde-cfg.before
kde() { # NAME ARGS...: a picture with KDE's files and answers
  name=$1; shift
  run "$name" hearth-settings 0 env XDG_CONFIG_HOME="$KC" HEARTH_KDE_FIXTURE="$KF" XDG_CURRENT_DESKTOP=KDE "$S" "$@"
}
# Batch 1: workspace and looks, screen locking, session, renderer, feedback.
kde kde-00-all --page kde-settings
kde kde-01-workspace --page kde-workspace
kde kde-02-style --page kde-style
kde kde-03-plasma-style --page kde-plasma-style
kde kde-04-colors --page kde-colors
kde kde-05-global-theme --page kde-global-theme
kde kde-06-decorations --page kde-decorations
kde kde-07-screenlock --page kde-screenlock
kde kde-08-session --page kde-session
kde kde-09-renderer --page kde-renderer
kde kde-10-feedback --page kde-feedback
# Long lists open over the page (A / a click on the name).
kde kde-11-colors-list --page kde-colors:pick
kde kde-12-decorations-list --page kde-decorations:pick
# The controller look: the focused row, ‹ name › for lists.
kde kde-13-b-decorations --layout b --page kde-decorations --focus "Title bar buttons"
kde kde-14-b-colors --layout b --page kde-colors --focus "Colour scheme"
kde kde-15-b-workspace --layout b --page kde-workspace --focus "Double-click speed"
# The controller moving and changing a choice (only in a copy of the files, put back after).
# On labwc (not KDE Plasma): the pages say so, and still save.
mkdir -p /tmp/kde-labwc; cp "$KF"/*.json /tmp/kde-labwc/; echo '{"plasma": false, "desktop": "Hearth"}' > /tmp/kde-labwc/status.json
run kde-16-labwc-screenlock hearth-settings 0 env XDG_CONFIG_HOME="$KC" HEARTH_KDE_FIXTURE=/tmp/kde-labwc XDG_CURRENT_DESKTOP=Hearth "$S" --page kde-screenlock
run kde-17-labwc-workspace hearth-settings 0 env XDG_CONFIG_HOME="$KC" HEARTH_KDE_FIXTURE=/tmp/kde-labwc XDG_CURRENT_DESKTOP=Hearth "$S" --page kde-workspace
( cd "$KC" && find . -type f | sort | xargs sha256sum ) > /tmp/kde-cfg.after
cmp -s /tmp/kde-cfg.before /tmp/kde-cfg.after && echo "::notice title=KDE pages::opening every page wrote nothing" || echo "::error title=KDE pages::opening a page changed KDE's files: $(diff /tmp/kde-cfg.before /tmp/kde-cfg.after | tr '\n' '|' | cut -c1-600)"
# A file that can't be read is never written over: the page says so and its rows can't be changed.
printf '[org.kde.kdecoration2\nlibrary=org.kde.breeze\0' > "$KC/kwinrc"
kde kde-18-bad-file --page kde-decorations
