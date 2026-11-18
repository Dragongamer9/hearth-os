# Hearth's apps on Hearth OS, Fedora edition

The owner, Nov 18, 2026: every Hearth app is in Hearth OS, Fedora edition (internally `anatase`, `os-bases/anatase`),
shown in KDE Plasma's menu, and Hearth's own apps are the defaults there; and, since Hearth is moving towards a pure
Rust desktop, every app becomes pure Rust ("goal is light and fast").

How Plasma shows them (`os-bases/anatase/build.sh`, the block "Hearth's apps in Plasma"):

- Each kind of app (files, terminal, text editor, calculator, reader, screenshots, pictures...) lists Hearth's own app
  first for Plasma, then Hearth's KDE builds, then KDE's own as the fallback (`/usr/share/hearth/app-kinds.json`,
  docs/APP-KINDS.md). KDE's apps stay installed; where Hearth has an app of that kind, KDE's is hidden from the menu and
  never opens those files instead.
- Hearth's choice becomes Plasma's default for each file type (`/etc/xdg/kde-mimeapps.list`). A default you chose
  yourself (Plasma's Default Applications, Open With › Remember, or Hearth Settings › Default apps, all kept in
  `~/.config/mimeapps.list`) always comes first and is never written by the image (red flag 9).
- Plasma uses Hearth's menu copies only when the build's check passed (`/usr/share/hearth/app-kinds/plasma-ready`,
  read by `nimbus-session`'s `run_plasma`). Without it Plasma's menu is as before (Hearth's and KDE's apps both).
- The image check (`.github/scripts/anatase-image-check.sh`) lists every Hearth entry as shown or hidden in Plasma, with
  the reason, and warns for each app the owner named that isn't shown. The build names every Hearth entry whose program
  is missing or can't start (a missing library).
- Apps that run only inside Mantel (Quickshell isn't in this edition) are never put in Plasma's menu until their pure
  Rust version is in (below).

## The apps

"In the image": installed by the Fedora edition's build (from the app list of the last build, run 44296110901).
"Shown in Plasma": by the desktop-entry rules (NoDisplay, OnlyShowIn, NotShowIn, TryExec). The owner saw several of
these missing on his laptop (test-1.15); the next build's "Plasma menu" notices say for each one whether it's shown and
why not.

| App | Written in | In the image | Shown in Plasma | Why, if not | Plan |
|---|---|---|---|---|---|
| Hearth Settings (`hearth-settings`) | Rust + Tauri (hosts/hearth-rs, page in ui/) | yes | yes | | convert to pure Rust later |
| Game Mode (Back to Game Mode) | shell (nimbus-mode) | yes | yes | | stays |
| Logs, Send logs | Rust + Tauri (hearth-tools), shell | yes | yes | | convert to pure Rust later |
| Browser | Firefox with Hearth's settings | yes | yes | | stays |
| Text Editor (`hearth-text`) | Rust + Tauri | yes | yes (now Plasma's text editor) | | convert to pure Rust later |
| Disks | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Photos | Rust + Tauri | yes | yes (now Plasma's picture viewer, before Gwenview) | | convert to pure Rust later |
| Player (videos, music) | Rust + Tauri | yes | yes | | convert to pure Rust later |
| System Info, Disk Usage, Network Tools, Passwords & Keys, Printers & Scanner, Benchmark, Welcome | Rust + Tauri (hearth-tools) | yes | yes | | convert to pure Rust later |
| Calculator (`hearth-calculator`) | Rust + Tauri | yes | was hidden by KCalc; now Plasma's calculator | KDE's list put KCalc first | convert to pure Rust later |
| Terminal (`hearth-terminal`) | Rust + Tauri | yes | was hidden by Konsole; now Plasma's terminal | KDE's list put Konsole first | convert to pure Rust later |
| Documents (PDF, e-books, comics) | Rust + Tauri | yes | was hidden by Okular; now Plasma's reader | KDE's list put Okular first | convert to pure Rust later |
| Camera, Recorder, Share Screen | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Clock, Calendar, Notes, Weather (Hearth Time) | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Contacts, Maps | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Write, Sheets, Slides (Hearth Office) | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Paint, Emoji, Fonts, Theater | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Nearby Share, Hearth Connect, Join | Rust + Tauri | yes | yes | | convert to pure Rust later |
| Controller Lab, Saves, Mods | Rust + Tauri (hearth-gaming) | yes | yes | | convert to pure Rust later |
| Accessibility, USB maker, Get Claude | Rust + Tauri / shell | yes | yes | | convert to pure Rust later |
| Hearth Files (KDE build, `hearth-files-kde`) | C++ (Dolphin, hosts/hearth-kde) | no | no | its build doesn't match the image's KDE libraries, so it's dropped and Fedora's Dolphin opens folders | the pure Rust Hearth Files replaces it |
| Terminal, Text Editor, Reader, Capture, Calculator (KDE builds) | C++ (hosts/hearth-kde) | yes | only where Hearth's own isn't there | Hearth's own come first | kept as fallbacks; dropped once the pure Rust apps are in |
| Task Manager | QML (Mantel only: TaskManagerView.qml + hearth-tasks) | no | no (Plasma's System Monitor is used) | needs Mantel (Quickshell) | **pure Rust port** (hosts/hearth-ui) |
| System Restore | QML (Mantel only) | entry only | no | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Reliability history | QML (Mantel only) | entry only | no | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Capture (screenshots) | QML (Mantel only) | entry only | no (Hearth's Spectacle build, else Spectacle) | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Hearth Files (classic) | QML (Mantel only, FileManager.qml) | entry only | no (Dolphin) | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Resource Monitor | QML (Mantel only) | entry only | no | `OnlyShowIn=Nimbus`; needs Mantel | pure Rust port (with Task Manager) |
| Quick Settings | QML (Mantel only) | no | no (Plasma's own tray) | part of Mantel's bar | **pure Rust port** (tray or app) |
| On-screen keyboard | QML (Mantel only) | no | no (Plasma's own keyboard) | part of Mantel | **pure Rust port** |
| File chooser | QML (Mantel only, hearth-portal) | no | no (KDE's portal) | part of Mantel | **pure Rust port** (xdg-desktop-portal FileChooser) |
| Notifications | QML (Mantel only) | no | no (Plasma's own) | part of Mantel | **pure Rust port** |
| Windows apps (`hearth-winapps`) | shell | yes | hidden (NoDisplay: opened from files) | | stays |

What's ported, and how light and fast each one is, is in the sections below as each port lands.
