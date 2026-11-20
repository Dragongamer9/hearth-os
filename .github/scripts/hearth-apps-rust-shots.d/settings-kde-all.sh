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
# Batch 2: task switcher, KRunner, web search keywords, app updates, Trash, connection preferences, KWin scripts,
# KDE's background services, activities, your folders, device actions.
kde kde-20-tabbox --page kde-tabbox
kde kde-21-krunner --page kde-krunner
kde kde-22-webshortcuts --page kde-webshortcuts
kde kde-23-updates --page kde-updates
kde kde-24-trash --page kde-trash
kde kde-25-netpref --page kde-netpref
kde kde-26-kwin-scripts --page kde-kwin-scripts
kde kde-27-kded --page kde-kded
kde kde-28-activities --page kde-activities
kde kde-29-locations --page kde-locations
kde kde-30-device-actions --page kde-device-actions
kde kde-31-b-tabbox --layout b --page kde-tabbox --focus "Windows from"
kde kde-32-b-kded --layout b --page kde-kded --focus "Low space warning"
# Batch 3: day and night times, Plasma's on-screen keyboard, controllers in KDE, music CDs, remote desktop (RDP), the
# rest of the Info Center.
kde kde-40-nighttime --page kde-nighttime
kde kde-41-plasmakeyboard --page kde-plasmakeyboard
kde kde-42-gamecontroller --page kde-gamecontroller
kde kde-43-cddb --page kde-cddb
kde kde-44-krdp --page kde-krdp
kde kde-45-info --page kde-info
kde kde-46-b-nighttime --layout b --page kde-nighttime --focus "Night starts"
# Batch 4: KDE shortcuts (one app open, and a search), push services, colour profiles, font management.
kde kde-60-shortcuts --page kde-shortcuts
kde kde-61-shortcuts-kwin --page kde-shortcuts:kwin
kde kde-62-push --page kde-push
kde kde-63-colord --page kde-colord
kde kde-64-fontinst --page kde-fontinst
kde kde-65-b-shortcuts --layout b --page kde-shortcuts:kwin --focus "Toggle Overview"
# The lower part of the long pages (the focus on their last rows scrolls them there).
kde kde-50-workspace-end --page kde-workspace --focus "On the taskbar too"
kde kde-51-style-end --page kde-style --focus "GTK apps' theme"
kde kde-52-decorations-end --page kde-decorations --focus "Shadow strength"
kde kde-53-screenlock-end --page kde-screenlock --focus "Sign-in options"
kde kde-54-tabbox-end --page kde-tabbox --focus "Order"
kde kde-55-device-actions-end --page kde-device-actions --focus "Add this action"
kde kde-56-info-end --page kde-info --focus "Info Center"
kde kde-57-cddb-end --page kde-cddb --focus "MP3: write ID3 tags"
kde kde-58-plasmakeyboard-end --page kde-plasmakeyboard --focus "Hearth's on-screen keyboard"
kde kde-59-all-end --page kde-settings --focus "More system information"
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
