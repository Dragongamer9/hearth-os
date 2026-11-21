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
- Plasma has its own menu copies (`/usr/share/hearth/app-kinds-plasma`), made at build time in a staging tree with
  each kind's KDE list reordered and Hearth Classic's "own apps" rule (which hides Hearth's apps under KDE) off there
  only; `nimbus-session`'s `run_plasma` puts them first for Plasma, and only when the build's check passed (without
  them Plasma's menu is as before: Hearth's and KDE's apps both). Hearth Classic's copies, the shared table and
  `/etc/hearth/desktops.json` are never changed.
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
| Task Manager | QML (Mantel only) → **pure Rust** (`hearth-task-manager`) | yes, when built | yes: Plasma's task manager (before System Monitor) | | ported; the device settings tabs are still open (checklist) |
| System Restore | QML (Mantel only) → **pure Rust** (`hearth-system-restore`) | yes, when built | yes | | ported |
| Reliability history | QML (Mantel only) → **pure Rust** (`hearth-reliability-history`) | yes, when built | yes | | ported |
| Capture (screenshots) | QML (Mantel only) | entry only | no (Hearth's Spectacle build, else Spectacle) | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Hearth Files (classic) | QML (Mantel only, FileManager.qml) | entry only | no (Dolphin) | `OnlyShowIn=Nimbus`; needs Mantel | **pure Rust port** |
| Resource Monitor | QML (Mantel only) | entry only | no | `OnlyShowIn=Nimbus`; needs Mantel | pure Rust port (with Task Manager) |
| Quick Settings | QML (Mantel only) | no | no (Plasma's own tray) | part of Mantel's bar | **pure Rust port** (tray or app) |
| On-screen keyboard | QML (Mantel only) | no | no (Plasma's own keyboard) | part of Mantel | **pure Rust port** |
| File chooser | QML (Mantel only, hearth-portal) | no | no (KDE's portal) | part of Mantel | **pure Rust port** (xdg-desktop-portal FileChooser) |
| Notifications | QML (Mantel only) | no | no (Plasma's own) | part of Mantel | **pure Rust port** |
| Windows apps (`hearth-winapps`) | shell | yes | hidden (NoDisplay: opened from files) | | stays |

## Pure Rust apps (hosts/hearth-ui)

No Tauri and no web view: each app is one small iced program (iced 0.14, the same toolkit and version as Firehold's
Rust shell on feature/firehold-rust), drawn in software with tiny-skia, so no graphics driver is loaded and it looks
the same on every PC. libcosmic wasn't used: it isn't on crates.io (a git dependency with its own iced fork), and
Hearth's look isn't COSMIC's (docs/FIREHOLD-RUST.md has the same reasoning).

- `hosts/hearth-ui` (the shared crate): the person's look (palettes.json: every look, dark and light, sub themes,
  the accent; desktop.json is read, never written, and followed while open through inotify), text and buttons,
  cards, chips, the window's own title bar and edges (as the mockups draw it), dialogs, line icons and graphs, and
  settings_file.rs for anything an app saves (red flag 9: atomic, .bak, merged, only what you change).
- `apps/task-manager` (`hearth-task-manager`): its own sampler, in Rust, from /proc and /sys, only while it's open
  and only what the page on show needs (the process list only on Processes, Details only on Details); the same app
  groups, protected parts and tags as Mantel's (hearth_tasks_procs.py's rules). Changing anything goes through
  `hearth-tasks` (only your own processes, never a part of Hearth). Its choices are `~/.config/nimbus/taskmgr.json`,
  the same keys as Mantel's. What of its mockups is in: docs/mockups/checklists/task-manager-rust.md.
- `apps/system-restore` (`hearth-system-restore`) and `apps/reliability` (`hearth-reliability-history`): Mantel's
  pages ported part for part; their numbers come from `hearth_restore.py` and `hearth_reliability.py`, which run once
  when the window opens (and for each action), never in the background.
- CI: `.github/workflows/hearth-apps-rust.yml` builds them in Fedora 45, starts each on a headless Wayland screen and
  fails on a slow first frame, high memory or CPU at rest, and posts pictures of every page and dialog.
- The Fedora edition builds them inside its own base (`base-anatase.yml`, job `hearth-ui`) and installs them in the
  "Hearth's apps in Plasma" block of `os-bases/anatase/build.sh`, where they take Task Manager's, System Restore's and
  Reliability history's menu entries and commands. Not built: the image is made as before.

Still to port, in this order: Capture, Hearth Files (classic), Quick Settings, the on-screen keyboard, Hearth's file
chooser (an xdg-desktop-portal FileChooser backend), notifications. The Tauri apps above become pure Rust after them.

## Status of the 1.16 fixes (Nov 19, paused for usage)

Done, green in CI (Hearth apps (pure Rust) run 44522638901 on d2a488793): the graphics card renderer (wgpu, tiny-skia as
the fallback; hearth-pad-host stays on tiny-skia), icons kept between frames, the scroll measurement; one owner per
controller button on Plasma (hearth-pad PLASMA_PAD_READERS / PLASMA_TAB_APPS); Settings' look no longer flipped by the
controller's own pointer, and Accessibility › Controller look; the account picture in Settings' corner; Files: This PC
first with the C: view on, each favourite once, no Gallery or separate Home server; Plasma's launcher swap in place and
panel transparency; one firewall (firewalld for new installs, ufw kept where it ran); Logitech G15 and
feature/game-mode-switch merged; Firehold (Rust) archived.
In progress: the Fedora image check (base-anatase.yml, publish=false, run 44522656094).
Next: read that run's result (and its smoothness check); then rebase onto the 1.15 release commit when it's given.
Not pictured in CI (no Plasma there): the launcher's place and panel transparency (checked by
hosts/linux/test/test_plasma_panel_js.py); try them on a device.
