# Hearth Settings › Personalization, Accounts, Privacy & security and Accessibility (cats/pers*, accounts.rs,
# privacy.rs, a11y.rs): every page, the lower parts of long pages (PAGE:from:N) and their lists (PAGE:id).
# KDE Plasma isn't in CI: the Plasma pages read a fixture (HEARTH_PLASMA_FIXTURE: what plasmashell's scripting
# would print, never sent anything), KWin's pages hearth-kwin-settings' preview (HEARTH_KWIN_FIXTURE), and the
# helpers in /usr/libexec/nimbus are the repo's (HEARTH_LIBEXEC). Sourced by hearth-apps-rust-test.sh (run()).
FX=/tmp/pers-acc-fixtures
mkdir -p "$FX" "$HOME/Pictures/Wallpapers" "$HOME/.config/nimbus"
cat > "$FX/plasma.json" <<'JSON'
{"panels": [{"id": 2, "location": "bottom", "height": 44, "hiding": "none", "floating": true, "lengthMode": "fill", "alignment": "center",
             "opacity": "adaptive", "screen": 0, "widgets": ["org.kde.plasma.kickoff", "org.kde.plasma.pager", "org.kde.plasma.icontasks",
             "org.kde.plasma.marginsseparator", "org.kde.plasma.systemtray", "org.kde.plasma.digitalclock", "org.kde.plasma.showdesktop"]},
            {"id": 27, "location": "top", "height": 32, "hiding": "autohide", "floating": false, "lengthMode": "fit", "alignment": "right",
             "opacity": "translucent", "screen": 0, "widgets": ["org.kde.plasma.digitalclock"]}],
 "desktops": [{"screen": 0, "plugin": "org.kde.image", "x": 0, "y": 0, "width": 1920, "height": 1080, "image": "file:///usr/share/wallpapers/Hearth-ridge"},
              {"screen": 1, "plugin": "org.kde.image", "x": 1920, "y": 0, "width": 2560, "height": 1440, "image": "file:///home/ci/Pictures/Wallpapers/mountains.jpg"}],
 "plugins": ["org.kde.image", "org.kde.slideshow", "org.kde.potd", "org.kde.color"],
 "ks": {"outputs": [{"name": "eDP-1", "pos": {"x": 0, "y": 0}, "enabled": true}, {"name": "DP-2", "pos": {"x": 1920, "y": 0}, "enabled": true}]}}
JSON
cat > "$FX/plasma-looks.py" <<'PY'
import json
print(json.dumps({"kde": {"lnf": "org.hearthos.steam", "scheme": "HearthOS-steam-crimson-Dark", "style": "hearth-steam", "colours": True}, "state": None, "plasma": True}))
PY
python3 - "$ROOT/hosts/linux/test/fixtures/kwin-preview.json" "$FX/kwin.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
d["desktops"] = 4
json.dump(d, open(sys.argv[2], "w"))
PY
for f in mountains night forest-path; do : > "$HOME/Pictures/Wallpapers/$f.jpg"; done
cat > "$HOME/.config/nimbus/my-looks.json" <<'JSON'
{"look:cosy-evening": {"name": "Cosy evening", "saved": 1763400000000, "desktop": {"look": "hearth", "mode": "dark", "frameTint": 30}, "fonts": {}, "accentShown": "#ff8a3d", "lookLabel": "Hearth"},
 "look:bright-desk": {"name": "Bright desk", "saved": 1763300000000, "desktop": {"look": "steam", "mode": "light"}, "fonts": {}, "accentShown": "#1a9fff", "lookLabel": "Steam-style"}}
JSON
cat > "$HOME/.local/share/nimbus/desktop.json" <<'JSON'
{"look": "steam", "mode": "dark", "subTheme": "crimson", "wallMatch": true, "deskClock": "on", "panelGlass": "frosted",
 "wallShow": {"on": false, "folder": "/home/ci/Pictures/Wallpapers", "minutes": 30, "shuffle": true}, "wallPotd": {"on": true, "source": "apod"},
 "wallPair": {"on": true, "light": "", "dark": "/home/ci/Pictures/Wallpapers/night.jpg"}, "wallScreens": {"DP-2": "/home/ci/Pictures/Wallpapers/mountains.jpg"},
 "frameTint": 40, "signinSplash": "flame", "countMe": true}
JSON
S="$BIN/hearth-settings-app"
KDE="env XDG_CURRENT_DESKTOP=KDE HEARTH_PLASMA_FIXTURE=$FX/plasma.json HEARTH_PLASMA_LOOKS=$FX/plasma-looks.py HEARTH_KWIN_FIXTURE=$FX/kwin.json HEARTH_LIBEXEC=$ROOT/os/system_files/usr/libexec/nimbus"
# KWin's pages where KWin isn't (greyed, with why), before the preview is on.
run settings-pa-00-effects-no-kwin hearth-settings 0 "$S" --page kwin-effects
run settings-pa-00-bar-mantel hearth-settings 0 "$S" --page bar-dock
touch "$XDG_RUNTIME_DIR/hearth-kwin-preview"
# Personalization
run settings-pa-01-appearance hearth-settings 0 $KDE "$S" --page appearance
run settings-pa-02-appearance-wallpaper hearth-settings 0 $KDE "$S" --page appearance:wallpaper,from:8
run settings-pa-03-appearance-sub hearth-settings 0 $KDE "$S" --page appearance:sub
run settings-pa-04-appearance-desktop hearth-settings 0 $KDE "$S" --page appearance:from:12
run settings-pa-05-bar-dock hearth-settings 0 $KDE "$S" --page bar-dock
run settings-pa-06-bar-dock-menu hearth-settings 0 $KDE "$S" --page bar-dock:from:10
run settings-pa-07-fonts hearth-settings 0 $KDE "$S" --page fonts:all
run settings-pa-08-fonts-list hearth-settings 0 $KDE "$S" --page fonts:font
run settings-pa-09-fonts-cursor hearth-settings 0 $KDE "$S" --page fonts:all,from:13
run settings-pa-10-wallpaper-more hearth-settings 0 $KDE "$S" --page wallpaper-more
run settings-pa-11-wallpaper-more-2 hearth-settings 0 $KDE "$S" --page wallpaper-more:from:5
run settings-pa-12-my-looks hearth-settings 0 $KDE "$S" --page my-looks
run settings-pa-13-look-more hearth-settings 0 $KDE "$S" --page look-more
run settings-pa-14-look-more-2 hearth-settings 0 $KDE "$S" --page look-more:from:4
run settings-pa-15-effects hearth-settings 0 $KDE "$S" --page kwin-effects
run settings-pa-16-effects-2 hearth-settings 0 $KDE "$S" --page kwin-effects:from:17
run settings-pa-17-effects-3 hearth-settings 0 $KDE "$S" --page kwin-effects:from:33
run settings-pa-18-effects-4 hearth-settings 0 $KDE "$S" --page kwin-effects:from:48
run settings-pa-19-effects-list hearth-settings 0 $KDE "$S" --page kwin-effects:tabbox
run settings-pa-20-desktops hearth-settings 0 $KDE "$S" --page kwin-desktops
run settings-pa-21-desktops-2 hearth-settings 0 $KDE "$S" --page kwin-desktops:from:8
run settings-pa-22-tiling hearth-settings 0 $KDE "$S" --page kwin-tiling
run settings-pa-23-tiling-2 hearth-settings 0 $KDE "$S" --page kwin-tiling:from:12
run settings-pa-24-edges hearth-settings 0 $KDE "$S" --page kwin-edges
run settings-pa-25-edges-2 hearth-settings 0 $KDE "$S" --page kwin-edges:from:11
run settings-pa-26-edges-list hearth-settings 0 $KDE "$S" --page kwin-edges:edge:7
run settings-pa-27-behaviour hearth-settings 0 $KDE "$S" --page kwin-behaviour
run settings-pa-28-behaviour-2 hearth-settings 0 $KDE "$S" --page kwin-behaviour:from:11
run settings-pa-29-behaviour-list hearth-settings 0 $KDE "$S" --page kwin-behaviour:focus
# Accounts
run settings-pa-30-accounts hearth-settings 0 $KDE "$S" --page accounts
run settings-pa-31-users hearth-settings 0 $KDE "$S" --page users
run settings-pa-32-signin hearth-settings 0 $KDE "$S" --page signin
run settings-pa-33-online-accounts hearth-settings 0 $KDE "$S" --page online-accounts
run settings-pa-34-online-imap hearth-settings 0 $KDE "$S" --page online-accounts:add-imap
run settings-pa-35-login-screen hearth-settings 0 $KDE "$S" --page login-screen
# Privacy & security
run settings-pa-36-privacy hearth-settings 0 $KDE "$S" --page privacy
run settings-pa-37-player-count hearth-settings 0 $KDE "$S" --page player-count
run settings-pa-38-firewall hearth-settings 0 $KDE "$S" --page firewall
run settings-pa-39-vaults hearth-settings 0 $KDE "$S" --page vaults:new
run settings-pa-40-recent-files hearth-settings 0 $KDE "$S" --page recent-files
run settings-pa-41-firmware hearth-settings 0 $KDE "$S" --page firmware
run settings-pa-42-firmware-history hearth-settings 0 $KDE "$S" --page firmware-history
run settings-pa-43-passwords hearth-settings 0 $KDE "$S" --page passwords
# Accessibility
run settings-pa-44-accessibility hearth-settings 0 $KDE "$S" --page accessibility
run settings-pa-45-accessibility-2 hearth-settings 0 $KDE "$S" --page accessibility:from:12
run settings-pa-46-accessibility-3 hearth-settings 0 $KDE "$S" --page accessibility:from:24
run settings-pa-47-accessibility-4 hearth-settings 0 $KDE "$S" --page accessibility:from:32
rm -f "$XDG_RUNTIME_DIR/hearth-kwin-preview" "$HOME/.config/nimbus/my-looks.json"
rm -rf "$FX"
look steam dark
