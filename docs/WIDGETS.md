# Desktop widgets and Activities (1.14)

Mantel's own desktop widgets and Activities, written for Hearth (the owner, Nov 16: every KDE Plasma feature Mantel
lacks is Hearth's own code). They work the same under labwc and KWin and never need KDE's activity manager.

## Widgets

Right-click the desktop › **Add widgets…** (or Quick Access › Widgets, or `quickshell ipc … call widgets gallery`)
opens the gallery. Each widget goes on that screen's desktop at a free place, on the activity you're on. Hover a
widget for its handle (drag to move), the gear (its settings) and × (remove, after asking); drag the corner to resize.
**Arrange widgets** does all of that from the keyboard or the controller (D-pad moves, X switches to resize, LB / RB
picks, A opens its menu: Settings, Show on every activity, Remove…, B is done).

| Widget | What it shows | Its settings |
| --- | --- | --- |
| Clock | analog or digital, the date, your cities | look, seconds, cities (any time zone) |
| Calendar | the month and what's coming up (Hearth Time's calendar) | agenda on or off |
| Weather | now and the next days, from Open-Meteo (no key) | the place, by name; °C or °F |
| Sticky note | a coloured note you type into | colour, text size |
| System monitor | CPU, graphics, memory, disk, network graphs (Task Manager's own numbers, `hearth-tasks sample`) | which graphs |
| Battery | charge, charging, time left (UPower) | — |
| Media player | what's playing (MPRIS), previous / play-pause / next | — |
| Folder view | a folder's files (Downloads by default) | the folder, icons or list |
| Photo frame | Pictures, Hearth Photos' favourites or an album, or a folder | where from, how often, fill or fit |
| App tiles | big tiles for apps (your pinned apps until you pick) | the apps |
| News | an RSS or Atom feed's headlines | the feed |
| Timer & stopwatch | count down (notifies when done) or time something | — (the minutes you pick are kept) |
| Quick notes | a short list you tick off | — |

Clock, weather, system monitor, battery and timer can also go **in the bar** (Add to bar); right-click one there to
take it out.

Files: `hosts/linux/desktop/shell/Widgets.qml` (loaded by file name from shell.qml: a problem only loses widgets),
`WidgetWindow.qml` (one layer-shell surface per widget, on the Bottom layer: above the wallpaper and icons, under every
window), `WidgetGallery.qml`, `WidgetArrange.qml`, `BarWidgets.qml`, `Widget<Type>.qml` (each widget, on
`WidgetBase.qml`), `WidgetStore.qml` (the layout store), `widget-layout.js` (the rules), `hearth-widgets` (places,
weather, feeds, folders, photos, time zones; `HEARTH_WIDGETS_FIXTURES` answers for tests).

## Activities

Separate sets of windows, wallpaper, widgets and pinned apps (Main, Work, Games…). The switcher opens from the bar's
activity button (shown once there's more than one), Quick Access › Activity, the desktop's menu, and
`ipc call activities switcher`; LB / RB pick, A switches, X changes the picked one (name, icon, wallpaper, its own
pinned apps, delete), B closes. Switching hides the windows of the activity you leave and brings back the ones Hearth
hid on the one you go to; a window you minimized yourself stays minimized. Deleting an activity moves its windows and
widgets to Main.

Files: `Activities.qml` (the model), `ActivitySwitcher.qml`, `ActivityChip.qml` (the bar's button), `activities.js`.

## What's saved, and how (red flag: settings never change by themselves)

- `~/.config/nimbus/widgets.json`: one key per widget, `"w:<id>": {type, place, screen, x, y, w, h, activity, order,
  cfg}`. Only moving, resizing, adding, removing or changing a widget writes, and only that widget's key is merged
  (SettingsWriter → hearth_settings_io.py: lock, atomic, `.bak`). A screen that isn't plugged in keeps its widgets
  (shown on the main screen until it's back, never moved); a place pulled inside a smaller screen is only drawn there.
  An entry the shell doesn't understand is left as it is. A damaged file is put back from `.bak` (`repair`); the
  widgets already on screen stay.
- `~/.config/nimbus/activities.json`: `current`, `"act:<id>": {name, icon, wallpaper, pins, order}`,
  `"app:<app id>": "<id>"`. Main has no key until you change it: Hearth without the file is exactly as before.
- `$XDG_RUNTIME_DIR/hearth-activities-windows.json`: which open window is on which activity, and which ones Hearth
  hid, so a restart of the shell keeps them and still brings the hidden ones back. Runtime state, not a setting.
- Weather's °C / °F: until you pick, it follows your measurement system (°F for the US's: LC_MEASUREMENT or the
  locale). Only the default; nothing is written until you pick.

## Hooks for other parts of Mantel

- **Window lists** (taskbar, dock, TaskStrip, Alt+Tab, the window bridge): show a window only when
  `SettingsNav.activityShows(toplevel)` is true (null: show every window). Widgets.qml sets it and
  `SettingsNav.activityMoveItems(toplevel)` (PopMenu items for "Move to activity ›", [] with one activity) and sets
  both back to null if it unloads, so the bars never import Activities themselves.
- **The window bridge (WinList)**: Activities reads `WinList.toplevels` when the singleton exists (else
  `ToplevelManager.toplevels`). For lasting membership give each toplevel `kwinId` (KWin's internal id); minimizing
  uses `setMinimized(bool)` if it exists, else `minimized = …`.
- **A window's menu**: "Move to activity ›" uses `SettingsNav.activityMoveItems(toplevel)` (each item calls
  `Activities.moveWindow(toplevel, id)`).
- **Pinned apps**: nothing to add. DeskState shows an activity's own list (`DeskState.activityPins`) and pin / unpin /
  reorder change only that list while you're on it.
- **Wallpaper**: nothing to add. `Theme.wall` shows `Theme.activityWall` first (set by Widgets.qml).
- **Key bindings** (labwc rc.xml / KWin): Meta+Q (as KDE) → `quickshell ipc -p <shell> call activities switcher`;
  Meta+Ctrl+Q / Meta+Shift+Ctrl+Q → `activities next` / `activities previous`.
- **The desktop's right-click menu**: keep the "Add widgets…", "Arrange widgets" and "Activities…" items
  (Desktop.qml, through `SettingsNav.widgets`).

## Round 2 (1.14): more widgets, the bar's own items, more panels

Written for Hearth from KDE Plasma's list (docs/KDE-FEATURES-1.14.md), the same on KWin and labwc. CI:
`.github/workflows/notify-widgets.yml` (pictures of each), `hosts/linux/test/test_notify_widgets_114.py`.

- **New widgets** (widget-layout.js CATALOG): **Web page** (`WidgetWeb.qml`: the page drawn every few minutes by a
  browser with no window, in a profile of its own, `hearth-widgets webshot`; shown as a picture so a heavy page can't
  slow the desktop; click opens it), **Calculator** (`WidgetCalc.qml`, widget-tools.js `calc`: never eval),
  **Colour picker** (`WidgetColour.qml`: KWin's picker or slurp + grim, `hearth-widgets pick-colour`; hex, RGB, HSL),
  **Keyboard indicator** (`WidgetKeys.qml`: Caps / Num Lock and the layout), **Pager** (`WidgetPager.qml`: desktops
  with window outlines under KWin; labwc doesn't say which desktop a window is on), **Window list**
  (`WidgetWindowlist.qml`). The last four (and the colour picker) can also go in the bar.
- **Clocks**: binary and fuzzy styles (widget-tools.js `fuzzy`, `binaryColumns`). **Calendars** (calendars.js):
  Islamic (tabular), Hebrew, Persian, Julian and Indian dates under each day, and holidays (ten countries plus Islamic,
  Jewish and Persian days), in the calendar widget and the clock's panel; world clocks in the clock's panel
  (Settings › Date & time).
- **The bar's own items** (BarLayout.qml, BarArrange.qml; right-click the bar, or Settings › Bar and dock › Arrange the
  bar): every item at the start or the end, in any order, or hidden. Game Mode stays last; the flame can't be hidden.
- **More panels** (ExtraPanels.qml): extra bars on any edge of any screen, each with what you choose (app menu, your
  apps, window list, pager, clock, tray, status, notifications, layout, Game Mode, bar widgets); Clone the main bar.
- **Tray**: copies and downloads (KDE job progress), printing (lpstat), disks and devices (open, safely remove),
  camera / microphone in use, background apps (the portal's monitor, else Flatpak), the keyboard layout.
- **Quick Access**: a light / dark tile (turning Auto off says so; Settings › Appearance turns it back on).
- Files: `~/.config/nimbus/bar.json` (`items`, `p:<id>` per panel, `clockZones`, `clockAlt`, `clockHolidays`), written
  only when you change one (SettingsWriter: merged, atomic, `.bak`).
