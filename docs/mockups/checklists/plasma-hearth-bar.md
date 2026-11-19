# Hearth's panel on KDE Plasma (stage 1 of "Hearth Classic on Plasma")

Mockups: plasma-hearth/mantel-hearth-dark-1-desktop.jpg, plasma-hearth/mantel-hearth-dark-3-launcher.jpg
Release: 1.16

The approved look is Mantel's own bar, dock and app menus on Debian, in every Hearth look. Mantel's CI pictures of
every look are on the branch ci/desktop-shots (looks/mantel/<look>-<mode>-1-desktop.png, -3-launcher.png, and
looks/mantel/bars/ for the bar edges and the five app menus); two are copied here, the rest get added as each look is
pictured on Plasma and compared side by side.

## Status (paused Nov 19 for the owner's usage limit)

Done: Mantel's bar, dock, app menus, theme and settings code read; the design below decided; nothing built yet.
In progress: nothing. Next step: write hosts/linux/desktop/plasma-hearth/ (below), then the CI workflow.

## How it's built (decided)

- One copy of the code: hosts/linux/desktop/plasma-hearth/install.sh builds the Plasma packages from Mantel's own
  files (Islands.qml, Dock.qml, Launcher.qml, MenuCentred/Tiles/Start/Classic.qml, MenuSearch, MenuTile, PopMenu,
  Frame, Icon, Theme, Settings, SettingsWriter, DeskState, BarLayout...), dropping only the Quickshell imports and
  the layer-shell lines, so the bar looks exactly like Mantel's in every look, sub theme, mode, accent and frame.
- Small stand-ins (shim/) do what Quickshell did, with Plasma's own pieces: commands through Plasma's executable
  source, windows and desktops from org.kde.taskmanager, sound from plasma-pa, battery from powermanagement,
  network from plasma-nm, Bluetooth from BluezQt, notifications' count from Plasma's notification server.
- Package org.hearthos.panel (roles bar / dock) in two Plasma panels (top bar, bottom dock; the panel's own
  background hidden, so only Mantel's islands show); org.hearthos.appmenu is the same menu as a launcher for KDE's
  own panel (Settings › App launcher). The menus and right-click menus open as full-screen layer windows, as Mantel.
- hearth-plasma-panel (helper): apply (saves your panels first, then puts Hearth's in), undo (KDE's default panel),
  sync (after a Hearth setting changes: the panels follow the look, bar edge and auto-hide; event driven by a
  systemd .path unit, nothing running at rest). Only run when you pick it: an update never rearranges your panel.

## Parts

- [ ] Top bar: start island (Hearth flame, desktops, the window in front)
- [ ] Top bar: clock pill (time, date, newest notification, what's playing)
- [ ] Top bar: status island (tray, Discord, keyboard, network, Bluetooth, volume, battery, notifications)
- [ ] Top bar: Game Mode button in every look (hidden when Game Mode is off)
- [ ] Bar styles: floating islands, one full-width bar, Console's shelf, thin (Zen)
- [ ] Bar layout: top bar + dock, one main bar on any edge, auto-hide, network speed
- [ ] Dock: flame, pinned and running apps grouped, running dots, right-click menu, Show desktop
- [ ] App menus: Hearth, Centred, Tiles, Start screen, Classic, with search
- [ ] Every look pictured on Plasma next to Mantel's: Hearth, Steam, Gaming, Zen, Glass, Synthwave, Cabin, Console (dark), Hearth and Glass (light)
- [ ] Settings › Personalization › Bar and dock: "Hearth's panel" (apply / back to KDE's panel)
