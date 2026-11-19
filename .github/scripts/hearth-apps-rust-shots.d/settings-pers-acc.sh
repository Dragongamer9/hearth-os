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
# Real pictures (Hearth's own stills from the repo), so the wallpaper tiles, your account picture and the login
# screen's background are drawn as the pictures themselves (pics.rs).
W="$HEARTH_SHELL_DIR/images/wallpapers"
cp "$W/slate-horizon.jpg" "$HOME/Pictures/Wallpapers/mountains.jpg"; cp "$W/slate-nebula.jpg" "$HOME/Pictures/Wallpapers/night.jpg"
cp "$W/slate-ridges.jpg" "$HOME/Pictures/Wallpapers/forest-path.jpg"; cp "$W/slate-facets.jpg" "$HOME/.face"
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
# What some helpers would answer on a real PC (HEARTH_JOB_FIXTURES: <helper>-<args>.json), for fuller pictures.
J="$FX/jobs"; mkdir -p "$J"
cat > "$J/hearth-system-users.json" <<'JSON'
{"users": [{"name": "chris", "fullname": "Chris", "me": true, "admin": true, "player": false, "picture": "@FACE@", "home": "/home/chris"},
           {"name": "dad", "fullname": "Dad", "me": false, "admin": false, "player": false, "picture": "", "home": "/home/dad"},
           {"name": "player2", "fullname": "Player 2", "me": false, "admin": false, "player": true, "picture": "", "home": "/home/player2"}],
 "me": "chris", "admin": true, "canAdmin": true, "oemSetup": false}
JSON
sed -i "s|@FACE@|$HOME/.face|" "$J/hearth-system-users.json"
cat > "$J/hearth-loginscreen-status.json" <<JSON
{"background": "$HOME/Pictures/Wallpapers/night.jpg", "autologin": false,
 "pictures": ["$HOME/Pictures/Wallpapers/mountains.jpg", "$HOME/Pictures/Wallpapers/night.jpg", "$HOME/Pictures/Wallpapers/forest-path.jpg"]}
JSON
cat > "$J/hearth-fingerprint-status.json" <<'JSON'
{"fprintd": true, "reader": true, "device": "Goodix MOC Fingerprint Sensor", "fingers": ["right-index-finger"], "places": {"lock": true, "polkit": false}, "pam": {"lock": true, "polkit": true}}
JSON
cat > "$J/hearth_accounts.py-list.json" <<'JSON'
{"accounts": [{"id": "nc1", "kind": "nextcloud", "kindLabel": "Nextcloud", "name": "cloud.example.com", "user": "chris", "can": {"calendar": true, "contacts": true, "files": true, "mail": false}, "use": {"calendar": true, "contacts": false, "files": true, "mail": false}}]}
JSON
cat > "$J/hearth_accounts.py-providers.json" <<'JSON'
{"secretService": true, "google": {"ready": false, "label": "Google", "why": "Google's sign-in isn't set up on this device yet."}, "microsoft": {"ready": true, "label": "Microsoft"}}
JSON
cat > "$J/hearth-firewall-status.json" <<'JSON'
{"backend": "firewalld", "on": true, "incoming": "deny", "zone": "public", "tailscale": true, "ssh": false, "needsAdmin": false,
 "rules": [{"id": "r1", "label": "Minecraft", "port": "25565", "proto": "tcp", "from": "local", "hearth": false},
           {"id": "r2", "label": "Hearth Link", "port": "47990-48010", "proto": "udp", "from": "local", "hearth": true}]}
JSON
cat > "$J/hearth-vaults-list.json" <<'JSON'
{"vaults": [{"id": "private", "name": "Private", "open": true, "mount": "/home/ci/Vaults/Private", "backend": "gocryptfs", "idle": 15, "lockOnLock": true, "data": "/home/ci/.local/share/hearth-vaults/private"},
            {"id": "taxes", "name": "Taxes", "open": false, "mount": "/home/ci/Vaults/Taxes", "backend": "cryfs", "idle": 0, "lockOnLock": true, "data": "/home/ci/.local/share/hearth-vaults/taxes"}],
 "backends": ["gocryptfs", "cryfs"], "folder": "~/Vaults"}
JSON
cat > "$J/hearth-firmware-status.json" <<'JSON'
{"secureBoot": "on", "uefi": true, "tpm": {"present": true, "version": "2.0"}, "fwupd": true}
JSON
cat > "$J/hearth-firmware-devices.json" <<'JSON'
{"devices": [{"id": "abc", "name": "UEFI Device Firmware", "vendor": "ASUSTeK", "version": "312", "system": true, "updatable": true, "installable": false, "update": {"version": "318", "summary": "Firmware for the ROG Ally", "description": "Improves battery life and fixes waking from sleep."}},
             {"id": "def", "name": "Logitech G305", "vendor": "Logitech", "version": "RQR12.11", "system": false, "updatable": true, "installable": true, "update": {"version": "RQR12.12", "summary": "", "description": "Better wireless range."}},
             {"id": "ghi", "name": "Samsung SSD 980", "vendor": "Samsung", "version": "3B4QFXO7", "system": false, "updatable": true, "installable": true}]}
JSON
cat > "$J/hearth-firmware-security.json" <<'JSON'
{"hsi": "HSI:2", "level": 2, "items": [{"name": "Secure Boot", "ok": true, "level": 1, "result": "enabled"}, {"name": "TPM v2.0", "ok": true, "level": 1, "result": "found"}, {"name": "Intel BootGuard", "ok": false, "level": 2, "result": "not-enabled"}]}
JSON
cat > "$J/hearth-firmware-history.json" <<'JSON'
{"history": [{"name": "Logitech G305", "vendor": "Logitech", "from": "RQR12.10", "to": "RQR12.11", "when": 1762000000, "state": "done", "summary": "Better battery life"},
             {"name": "Samsung SSD 980", "vendor": "Samsung", "from": "3B4QFXO6", "to": "3B4QFXO7", "when": 1761000000, "state": "failed", "error": "The device went away during the update"}]}
JSON
cat > "$J/hearth-secrets-status.json" <<'JSON'
{"running": true, "provider": "KDE Wallet (Secret Service)", "keyrings": [{"id": "kdewallet", "label": "kdewallet", "locked": false, "count": 3, "protected": true, "default": true}, {"id": "old", "label": "Old keyring", "locked": true, "count": 1, "protected": false}], "pam": {"auth": true}, "prompter": true}
JSON
cat > "$J/hearth-secrets-list.json" <<'JSON'
{"items": [{"id": "1", "label": "Chrome Safe Storage", "app": "Chromium", "account": "", "where": "", "created": 1760000000000, "keyring": "kdewallet"},
           {"id": "2", "label": "Discord", "app": "Discord", "account": "chris", "where": "", "created": 1761000000000, "keyring": "kdewallet"},
           {"id": "3", "label": "NAS share", "app": "Hearth Files", "account": "chris", "where": "smb://nas/home", "created": 1762000000000, "keyring": "kdewallet"}], "lockedKeyrings": ["Old keyring"]}
JSON
S="$BIN/hearth-settings-app"
KDE="env XDG_CURRENT_DESKTOP=KDE HEARTH_PLASMA_FIXTURE=$FX/plasma.json HEARTH_PLASMA_LOOKS=$FX/plasma-looks.py HEARTH_KWIN_FIXTURE=$FX/kwin.json HEARTH_LIBEXEC=$ROOT/os/system_files/usr/libexec/nimbus HEARTH_JOB_FIXTURES=$J"
# KWin's pages where KWin isn't (greyed, with why), before the preview is on.
run settings-pa-00-effects-no-kwin hearth-settings 0 "$S" --page kwin-effects
run settings-pa-00-bar-mantel hearth-settings 0 "$S" --page bar-dock
touch "$XDG_RUNTIME_DIR/hearth-kwin-preview"
# Personalization
run settings-pa-01-appearance hearth-settings 0 $KDE "$S" --page appearance
run settings-pa-02-appearance-wallpaper hearth-settings 0 $KDE "$S" --page appearance:wallpaper,from:8
run settings-pa-03-appearance-sub hearth-settings 0 $KDE "$S" --page appearance:sub
run settings-pa-04-appearance-desktop hearth-settings 0 $KDE "$S" --page appearance:from:9
run settings-pa-05-bar-dock hearth-settings 0 $KDE "$S" --page bar-dock
run settings-pa-06-bar-dock-menu hearth-settings 0 $KDE "$S" --page bar-dock:from:10
run settings-pa-07-fonts hearth-settings 0 $KDE "$S" --page fonts:all
run settings-pa-08-fonts-list hearth-settings 0 $KDE "$S" --page fonts:font
run settings-pa-09-fonts-cursor hearth-settings 0 $KDE "$S" --page fonts:all,from:6
run settings-pa-09b-fonts-cursor hearth-settings 0 $KDE "$S" --page fonts:all,from:8
run settings-pa-10-wallpaper-more hearth-settings 0 $KDE "$S" --page wallpaper-more
run settings-pa-11-wallpaper-more-2 hearth-settings 0 $KDE "$S" --page wallpaper-more:from:10
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
run settings-pa-31b-users-dad hearth-settings 0 $KDE "$S" --page users:dad,from:3
run settings-pa-31c-users-add hearth-settings 0 $KDE "$S" --page users:from:9
run settings-pa-32-signin hearth-settings 0 $KDE "$S" --page signin
run settings-pa-32b-signin-fingerprint hearth-settings 0 $KDE "$S" --page signin:from:10
run settings-pa-32c-signin-finger-list hearth-settings 0 $KDE "$S" --page signin:finger
run settings-pa-33-online-accounts hearth-settings 0 $KDE "$S" --page online-accounts
run settings-pa-34-online-imap hearth-settings 0 $KDE "$S" --page online-accounts:add-imap,from:10
run settings-pa-35-login-screen hearth-settings 0 $KDE "$S" --page login-screen
# Privacy & security
run settings-pa-36-privacy hearth-settings 0 $KDE "$S" --page privacy
run settings-pa-37-player-count hearth-settings 0 $KDE "$S" --page player-count
run settings-pa-38-firewall hearth-settings 0 $KDE "$S" --page firewall
run settings-pa-38b-firewall-2 hearth-settings 0 $KDE "$S" --page firewall:from:5
run settings-pa-39-vaults hearth-settings 0 $KDE "$S" --page vaults:taxes
run settings-pa-39b-vaults-new hearth-settings 0 $KDE "$S" --page vaults:new,from:4
run settings-pa-40-recent-files hearth-settings 0 $KDE "$S" --page recent-files
run settings-pa-41-firmware hearth-settings 0 $KDE "$S" --page firmware
run settings-pa-41b-firmware-2 hearth-settings 0 $KDE "$S" --page firmware:from:5
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
