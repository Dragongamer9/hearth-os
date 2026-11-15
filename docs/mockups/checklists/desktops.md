# KDE Plasma and GNOME, downloaded when picked (approved Nov 14, 2026)

Mockups: next-update/desktops-1-your-desktop-settings.png, next-update/desktops-2-kde-plasma.png, next-update/desktops-3-gnome.png
Release: 1.13

The owner's choices: KDE and GNOME are downloaded only when picked (not in the image); Hearth lives inside their own
look; each desktop shows only its own apps plus the ones you installed (Hearth's own apps stay in Mantel); Mantel is
always built in and is where Hearth goes back to. No KDE or GNOME logos are drawn. Settings opens the page as
System › Your desktop (Settings' categories are fixed at 12; the mockup's caption says "Settings › Desktop").

## Settings › Desktop › Your desktop (desktops-1)
- [x] Mantel (Hearth Classic): picked, "built in · using now", its line — hosts/linux/desktop/shell/SetYourDesktop.qml: "\"built in\" + (page.st && page.st.now === \"classic\" ? \" · using now\" : \"\")"
- [x] KDE Plasma: its line, "downloaded" tag, Remove and "Use KDE Plasma" — hosts/linux/desktop/shell/SetYourDesktop.qml: "SetButton { visible: card.here && !card.chosen; text: \"Remove\""
- [x] GNOME: its line, "downloading" tag, a progress bar with the percent, "made for Hearth OS <version>", "the rest keeps working while it downloads", Cancel — hosts/linux/desktop/shell/SetYourDesktop.qml: "% · made for Hearth OS "
- [x] Downloading for real: apt into the writable system, progress from apt, cancel while downloading, remove — os/system_files/usr/libexec/nimbus/hearth-desktops-helper: "APT::Status-Fd=%d"
- [x] Only show each desktop's own apps: a switch (on), its line — hosts/linux/desktop/shell/SetYourDesktop.qml: "Only show each desktop's own apps"
- [x] ... and it really does: Mantel Hearth's apps, KDE KDE's, GNOME GNOME's, yours everywhere — os/system_files/usr/libexec/hearth-app-kinds: "def own_apps(root, table, raw, installed, chosen, seen, odir, written):"
- [x] Pick at the login screen too: its line, a "This one ▾" drop-down — hosts/linux/desktop/shell/SetYourDesktop.qml: "{ value: \"save\", label: \"This one\" }"
- [x] ... the login screen lists them once downloaded — hosts/linux/session/hearth-kde.desktop: "TryExec=/usr/bin/startplasma-wayland"
- [x] Safe start: its line, "always on" (twice failing to start opens Mantel and says why; the choice stays saved) — hosts/linux/desktop/shell/SetYourDesktop.qml: "chip: \"always on\""

## KDE Plasma with Hearth inside (desktops-2)
- [x] KDE's own panel at the bottom with its menu button, pinned apps, tray and clock — os/system_files/usr/share/hearth-kde/plasma/layout-templates/org.kde.plasma.desktop.defaultPanel/contents/layout.js: "panel.addWidget(\"org.hearth.kickoff\")"
- [x] The start menu lists only KDE's apps (tagged KDE) and the apps you installed (tagged installed) — os/system_files/usr/share/plasma/plasmoids/org.hearth.kickoff/contents/ui/main.qml: "PlasmaComponents.Label { text: modelData.tag"
- [x] Start menu foot: Shut down, Restart, Switch desktop…, Return to Game Mode — os/system_files/usr/share/plasma/plasmoids/org.hearth.kickoff/contents/ui/main.qml: "text: \"⇄ Switch desktop…\""
- [x] A Hearth item in the tray (orange square) — os/system_files/usr/share/plasma/plasmoids/org.hearth.tray/metadata.json: "\"X-Plasma-NotificationArea\": \"true\""
- [x] Its popup: "Hearth" title, a big "Return to Game Mode" button — os/system_files/usr/share/plasma/plasmoids/org.hearth.tray/contents/ui/main.qml: "text: \"🎮  Return to Game Mode\""
- [x] Controller (pad and mouse mode), Keyboard (shows when you type), Hearth notices (in KDE's notifications), Server at home, Hearth update, Switch desktop (Mantel · KDE · GNOME) — os/system_files/usr/share/plasma/plasmoids/org.hearth.tray/contents/ui/main.qml: "{ a: \"switch\", icon: \"⇄\", t: \"Switch desktop\""
- [x] The controller works in KDE (Hearth's pad service, started once KDE is up) — hosts/linux/session/nimbus-session: "desk_companion \"$dname\""

## GNOME with Hearth inside (desktops-3)
- [x] GNOME's own top bar (activities, clock, status icons with the Hearth square) — os/system_files/usr/share/gnome-shell/extensions/hearth-quick@hearthos.fun/extension.js: "// The Hearth square among the status icons."
- [x] The overview's dash and app grid list only GNOME's apps and the apps you installed — os/system_files/usr/share/hearth/app-kinds.json: "org.gnome.Loupe.desktop"
- [x] GNOME's quick settings with a Hearth section: big orange "Return to Game Mode" — os/system_files/usr/share/gnome-shell/extensions/hearth-quick@hearthos.fun/extension.js: "label: '🎮  Return to Game Mode'"
- [x] Controller, On-screen keyboard, Hearth notices, Server at home, Hearth update, Switch desktop › — os/system_files/usr/share/gnome-shell/extensions/hearth-quick@hearthos.fun/extension.js: "['keyboard', '⌨', 'On-screen keyboard'],"
- [x] On by default once GNOME is downloaded — os/system_files/usr/share/glib-2.0/schemas/90_hearth-desktops.gschema.override: "enabled-extensions=['hearth-quick@hearthos.fun', 'hearth-pad@hearthos.fun']"
