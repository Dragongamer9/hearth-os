# Hearth Classic on Plasma, stage 1: Hearth's panel (Mantel's bar, dock, app menus, Quick settings, notifications)

Mockups: plasma-hearth/mantel-hearth-dark-1-desktop.jpg, plasma-hearth/mantel-hearth-dark-3-launcher.jpg
Release: 1.16

The approved look is Mantel's own (Debian's Desktop Mode) in every Hearth look. Mantel's CI pictures of every look
are on the branch ci/desktop-shots (looks/mantel/<look>-<mode>-1-desktop, -2-quick-settings, -3-launcher,
-4-notifications-dashboard, and looks/mantel/bars/ for the five app menus); Plasma's are on
ci/plasma-hearth-shots-feature-plasma-hearth-menu (.github/workflows/plasma-hearth.yml), compared side by side.

How it's built: hosts/linux/desktop/plasma-hearth/install.sh makes Plasma packages out of Mantel's own files
(Islands.qml, Dock.qml, Launcher.qml and its four styles, QuickAccess.qml, ClockPanel.qml, Notifications.qml, Theme.qml,
Settings.qml, DeskState.qml...), taking out only the Quickshell lines (transform.py), so every look, sub theme, mode,
accent, frame and panel transparency is Mantel's to the pixel; shim/ does what Quickshell did with Plasma's own pieces.
Packages: org.hearthos.panel (the bar and the dock, in two Plasma panels whose own background is hidden),
org.hearthos.appmenu, org.hearthos.quicksettings, org.hearthos.notifications (for KDE's own panel). Quick settings'
backend shared with Firehold on Plasma: /usr/libexec/hearth/hearth-quick (qs / qs-do, the same JSON).

## Built and pictured (all 10 looks: Hearth, Steam, Gaming, Zen, Glass, Synthwave, Cabin, Console dark, Hearth and Glass light)

- [x] The bar is Mantel's own islands bar, in every look — hosts/linux/desktop/plasma-hearth/install.sh: "Islands.qml"
- [x] Start island: the Hearth flame, the desktops (KWin's, through Plasma's task manager), the window in front — hosts/linux/desktop/plasma-hearth/shim/WinList.qml: "TaskManager.VirtualDesktopInfo"
- [x] Clock pill with the date, and the newest notification in it — hosts/linux/desktop/plasma-hearth/package/main.qml: "Notifications.qml"
- [x] Status island: Discord, keyboard, network, Bluetooth, volume, battery, notifications with the count — hosts/linux/desktop/plasma-hearth/shim/Pipewire.qml: "PaSink.qml"
- [x] Game Mode button in every look, hidden when Game Mode is off — hosts/linux/desktop/plasma-hearth/install.sh: "Islands.qml"
- [x] Gaming's performance island (CPU, RAM) — hosts/linux/desktop/plasma-hearth/install.sh: "PerfIsland.qml"
- [x] Bar styles: floating islands, Steam's full-width bar, Zen's thin bar, Console's shelf with its hints — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "function bandOf(sw)"
- [x] Plasma's own panel background hidden, so only the islands show — hosts/linux/desktop/plasma-hearth/package/main.qml: "panel-background"
- [x] Dock: the flame, Task view, pinned apps (desk.json, Mantel's), running apps grouped with their dots, Show desktop — hosts/linux/desktop/plasma-hearth/install.sh: "Dock.qml"
- [x] App menus: Hearth, Centred, Tiles (with its search field in the bar), Start screen, Classic — hosts/linux/desktop/plasma-hearth/install.sh: "MenuCentred.qml"
- [x] The app menu with Super (Plasma's launcher key) and as a launcher for KDE's own panel — hosts/linux/desktop/plasma-hearth/package/metadata.json: "org.kde.plasma.launchermenu"
- [x] Quick settings, Mantel's, in every look — hosts/linux/desktop/plasma-hearth/install.sh: "QuickAccess.qml"
- [x] The dashboard: notifications, calendar, system — hosts/linux/desktop/plasma-hearth/install.sh: "ClockPanel.qml"
- [x] Notification pop-ups, Mantel's, under the clock pill, one at a time as Mantel's timing has them (on Plasma's own notification service; KDE's own pop-ups pause meanwhile) — hosts/linux/desktop/plasma-hearth/shim/NotificationServer.qml: "showAddedDuringInhibition"
- [x] The dashboard's list as Mantel's: a Flatpak app's own icon, "Open <app>" on notifications kept from before a restart — .github/scripts/plasma-hearth-shots.sh: "org.hearth.IconTest.svg"
- [x] Every look follows Hearth's settings live (desktop.json, desk.json, bar.json, the apps: a systemd .path unit, nothing polls) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel.path: "PathChanged=%h/.local/share/nimbus/desktop.json"
- [x] Bar layouts: one main bar on any edge, the dock on or off, auto-hide — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "SYNC_JS"
- [x] Settings › Bar and dock: Hearth's panel or KDE's panel (your panels saved first; back to KDE's default panel) — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Hearth's panel"
- [x] Settings › Bar and dock › App menu lists Hearth's five menus for KDE's own panel, in Kickoff's place — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Hearth's menu: {l}"
- [x] Your old panels exactly as they were (hearth-plasma-panel restore) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def save_panels"
- [x] Quick settings' backend shared with Firehold on Plasma — hosts/linux/desktop/plasma-hearth/hearth-quick: "hearth-quick qs"
- [x] Nothing heavy at rest: away from the PC or locked, Mantel's moving wallpapers hold still (plasmashell measured in CI: 0.04 s in a minute; with a moving wallpaper drawn by the processor, as CI has no graphics card, it's busy) — hosts/linux/desktop/plasma-hearth/package/IdleLink.qml: "idle-wait"

- [x] The default on new Fedora installs and new accounts' first sign-in only, never for an account that already has a panel (the owner, Nov 19 22:11) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def first_login"
- [x] KDE's panel comes back by itself, and says so, when Hearth's bar doesn't start — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def guard"
- [x] Mantel's desktop: each look's own wallpaper or moving scene, the desktop clock, desktop icons with their folder and file pictures — hosts/linux/desktop/plasma-hearth/shim/IconImage.qml: "name|fallback"
- [x] Desktop icons stay where you put them: off and on, a restart, signing out and in, a smaller screen (the owner, Nov 19 23:34; Mantel's hearth-desk never changes a saved place by itself; CI ph-28a/ph-28b: moved, off and on, restarted, same place) — hosts/linux/desktop/shell/hearth-desk: "Only the icons that moved"
- [x] The desktop's right-click menu (Mantel's) — hosts/linux/desktop/plasma-hearth/package/DesktopLayer.qml: "backgroundMenu"
- [x] Mantel's power menu — hosts/linux/desktop/plasma-hearth/install.sh: "PowerMenu.qml"
- [x] Volume and brightness bars (Mantel's; KDE's own on-screen display off while Hearth's panel is in use) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "('plasmarc', 'OSD', 'Enabled', 'false')"
- [x] Mantel's window frames in every look (Aurorae, theme/hearth_aurorae.py) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def frames"
- [x] Quick settings' weather row, the dashboard's weather tile and events, notifications with their apps' icons and buttons (pictured with Hearth Time's stand-in, as Mantel's CI) — .github/scripts/plasma-hearth-shots.sh: "--weather-summary"
- [x] Settings pages in the app menu's search (Mantel's settings-pages.js) — hosts/linux/desktop/plasma-hearth/shim/SettingsNav.qml: "settings-pages.js"
- [x] Keyboard layout button and copies and downloads (Plasma's keyboard layout and job progress) — hosts/linux/desktop/plasma-hearth/shim/PlasmaHub.qml: "KB.KeyboardLayout"
- [x] Alt+Tab: Mantel's switcher (org.hearth.switcher, KWin's task switcher), pictured with Alt held down through KWin's fake input (CI extras ph-73: the card in the middle, each window's live picture, the one you'll switch to lit in the accent) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "('kwinrc', 'TabBox', 'LayoutName', 'org.hearth.switcher')"
- [x] KDE's own pop-ups never show, critical ones too (paused while Hearth shows them; KWin's window list in CI has no KDE notification window) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "'--notify'"

## Built in 1.16's last round (CI extras: plasma-hearth.yml's extras job, KWin drawing with OpenGL)

- [x] Glass's frosted blur behind the islands: Mantel's frost copy drawn small and stretched back (blurred on any PC, graphics card or not, next to no cost), cut to each island's round corners on the graphics card; under the milky fill and edge (CI ph-70, ph-71, ph-72/72b: a sharp striped wallpaper is smooth behind the clock pill) — hosts/linux/desktop/shell/Frost.qml: "layer.textureSize"
- [x] Tray icons of apps: Plasma 6's tray offers them to no other widget, so Hearth's own small tray host (tells KDE's watcher there's a tray; icons, titles, tooltips; clicks, scrolls and the app's own menu in Mantel's pop-up menu; asleep until an icon changes) (CI ph-74/74b: two apps' icons in the status island) — hosts/linux/desktop/plasma-hearth/hearth-tray-host: "RegisterStatusNotifierHost"
- [x] What's playing in the pill and the Now playing tile (Plasma's media players): the clock pill's cover, title, artist, level meter and play/pause; the Tiles and Start screen menus' Now playing tile; the dashboard's media card with its position, length and controls (CI ph-74b, ph-75, ph-76) — hosts/linux/desktop/plasma-hearth/shim/MprisPlayers.qml: "positionChanged: function"

## Built after the first stage

- [x] Background apps, camera and microphone, printing, disks buttons (Mantel's hearth-notifyd through a go-between; the microphone from Plasma's sound service) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def notifyd_bridge"
- [x] Firehold's style for org.hearthos.quicksettings (mockups firehold-plasma 07 and 08: Desk and Couch) — hosts/linux/desktop/plasma-hearth/package/FireholdQuick.qml: "Couch (08)"
- [x] Firehold's style for org.hearthos.notifications (mockups firehold-plasma 09 cards and 10 list) — hosts/linux/desktop/plasma-hearth/package/FireholdCards.qml: "Stays until you answer"
- [x] The lock screen (Mantel's, hosts/linux/desktop/kwin-lock, in KDE's screen locker: KWin's own environment names it, from the next sign-in, only after the locker's test run shows it; CI ph-27; needs testing on a device) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "plasma-kwin_wayland.service.d"
