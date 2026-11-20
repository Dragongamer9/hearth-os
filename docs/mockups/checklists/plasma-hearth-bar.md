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
- [x] Notification pop-ups, Mantel's, under the clock pill (on Plasma's own notification service; KDE's own pop-ups pause meanwhile) — hosts/linux/desktop/plasma-hearth/shim/NotificationServer.qml: "showAddedDuringInhibition"
- [x] Every look follows Hearth's settings live (desktop.json, desk.json, bar.json, the apps: a systemd .path unit, nothing polls) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel.path: "PathChanged=%h/.local/share/nimbus/desktop.json"
- [x] Bar layouts: one main bar on any edge, the dock on or off, auto-hide — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "SYNC_JS"
- [x] Settings › Bar and dock: Hearth's panel or KDE's panel (your panels saved first; back to KDE's default panel) — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Hearth's panel"
- [x] Settings › Bar and dock › App menu lists Hearth's five menus for KDE's own panel, in Kickoff's place — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Hearth's menu: {l}"
- [x] Your old panels exactly as they were (hearth-plasma-panel restore) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def save_panels"
- [x] Quick settings' backend shared with Firehold on Plasma — hosts/linux/desktop/plasma-hearth/hearth-quick: "hearth-quick qs"
- [x] Nothing at rest (plasmashell measured in CI: a few hundredths of a second a minute) — .github/scripts/plasma-hearth-shots.sh: "at rest"

- [x] The default on new Fedora installs and new accounts' first sign-in only, never for an account that already has a panel (the owner, Nov 19 22:11) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def first_login"
- [x] KDE's panel comes back by itself, and says so, when Hearth's bar doesn't start — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def guard"
- [x] Mantel's desktop: each look's own wallpaper or moving scene, the desktop clock, desktop icons — hosts/linux/desktop/plasma-hearth/package/DesktopLayer.qml: "Wallpaper.qml"
- [x] The desktop's right-click menu (Mantel's) — hosts/linux/desktop/plasma-hearth/package/DesktopLayer.qml: "backgroundMenu"
- [x] Mantel's power menu — hosts/linux/desktop/plasma-hearth/install.sh: "PowerMenu.qml"
- [x] Volume and brightness bars (Mantel's; KDE's own on-screen display off while Hearth's panel is in use) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "('plasmarc', 'OSD', 'Enabled', 'false')"
- [x] Mantel's window frames in every look (Aurorae, theme/hearth_aurorae.py) — hosts/linux/desktop/plasma-hearth/hearth-plasma-panel: "def frames"
- [x] Quick settings' weather row, the dashboard's weather tile and events, notifications with their apps' icons and buttons (pictured with Hearth Time's stand-in, as Mantel's CI) — .github/scripts/plasma-hearth-shots.sh: "--weather-summary"
- [x] Settings pages in the app menu's search (Mantel's settings-pages.js) — hosts/linux/desktop/plasma-hearth/shim/SettingsNav.qml: "settings-pages.js"
- [x] Keyboard layout button and copies and downloads (Plasma's keyboard layout and job progress) — hosts/linux/desktop/plasma-hearth/shim/PlasmaHub.qml: "KB.KeyboardLayout"
- [ ] Alt+Tab: Mantel's switcher (org.hearth.switcher, KWin's task switcher) — set, picture pending
- [ ] KDE's own pop-ups never show, critical ones too (paused while Hearth shows them; picture pending)

## Not built yet

- [ ] Glass's frosted blur behind the islands (the milky fill and edge are there)
- [ ] Tray icons of apps (Plasma's statusnotifieritem source is wired; not pictured, needs testing on a device)
- [ ] What's playing in the pill and the Now playing tile (Plasma's media players; needs testing on a device)
- [ ] Background apps, camera and microphone, printing, disks buttons (Mantel's own service hearth-notifyd streams; Plasma's equivalents to wire)
- [ ] Firehold's style for org.hearthos.quicksettings and org.hearthos.notifications (mockups firehold-plasma 07-10)
- [ ] The lock screen (Mantel's, hosts/linux/desktop/kwin-lock, in KDE's screen locker)
