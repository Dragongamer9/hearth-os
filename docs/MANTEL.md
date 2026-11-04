# Mantel 2026: the desktop's facelift

Mantel is Hearth OS's desktop (labwc + the Quickshell QML shell in `hosts/linux/desktop/shell/`). The 2026 facelift
replaces the single bottom taskbar with **floating top islands** and a **floating dock**, redraws Quick settings, the
launcher, the clock's panel and Settings, and adds six new looks next to Hearth and Steam-style.

## What you see

- **Top islands** (`Islands.qml`): the Hearth mark (opens the launcher), your desktops (the one you're on is a long
  accent pill), and the window in front; the **centre pill** with the clock, which grows to show what's playing
  (MPRIS) and, for a few seconds, the newest notification; the **status island** (tray, Friends, keyboard, Wi-Fi,
  Bluetooth, volume, battery, notifications: click opens Quick settings, right-click menus, middle-click mutes,
  scroll changes the volume); and the **Game Mode button**, the biggest target in the bar, in every look.
- **Dock** (`Dock.qml`): pinned and open apps with every behaviour the taskbar had (click / middle-click / drag to
  reorder / right-click jump list, Pin, Move to desktop, Maximize, Close, Super+1–9), the launcher button and the
  Show desktop strip. Running apps have a dot, the one in front a long accent dot.
- **Quick settings** (`QuickAccess.qml`): your battery and time left, Settings / Lock / Power (Sleep is in Power),
  a big **Return to Game Mode**, tiles for Wi-Fi and Bluetooth (with a chevron to their lists), Night light,
  Controller mouse, Do not disturb and Keyboard, pill sliders for volume (its chevron opens the output in Settings ›
  Sound) and brightness, the power profile, Handheld settings and the weather, and controller hints.
- **Launcher** (`Launcher.qml`): a large centred panel. Search finds apps, **Settings pages and games**; Recent can
  hold all three. Apps / Games tabs, the System apps folder, right-click actions and the power row (Game Mode first)
  are as before; icons are bigger.
- **Notification centre + dashboard** (`ClockPanel.qml`, from the centre pill or the bell): notifications with Do not
  disturb and Clear all on the left; the month with Hearth Calendar's events, what's playing, the weather and the
  system (CPU, GPU, memory, temperature; read only while it's open) on the right.
- **Settings** (`SettingsApp.qml`): a search field over the page list (type anywhere on the list to search), the
  pages in groups (Connections, Look and sound, Devices, System). Every page is still there.
- **Settings › Appearance** (`SetAppearance.qml`): the **Look** as eight picture cards; **Mode** Dark / Light /
  **Sunset**; **Bar and dock** (the look's own, floating islands or a full-width bar; the dock always shown or
  auto-hide); the **accent** (presets, **From wallpaper**, a hue, a typed colour); the wallpaper; **Desktop clock
  and weather**; **Panel transparency**; a way to the on-screen keyboard's style; Night light and Reduce motion.
- **Handheld screens** (1440 px wide or less, like 1280×800): the bar keeps the essentials (the mark, desktops, clock,
  Wi-Fi, volume, battery, Game Mode); Friends is in Quick settings' header; the dock's icons are bigger. After you
  use the controller in Hearth's menus, a **button-hint strip** shows for a while (`HintStrip.qml`).

## The looks

All eight are entries in `theme/palettes.json` with the colour tokens hearth-theme already uses (so window frames
and Qt / GTK / Hearth apps follow) plus style tokens only the shell reads:

| Look | islandStyle | frame | Its own touches |
|---|---|---|---|
| Hearth | floating | plain | Warm firelit darks; the fireside wallpaper |
| Steam-style | bar | plain | One flat full-width bar, 4 px corners, Inter, blue |
| Gaming | floating | chamfer | Cut corners, neon edges along the screen, a performance island (FPS only while MangoHud logs it), drawn neon wallpaper |
| Zen | bar (thin) | plain | One 36 px bar, sage accent, the dock auto-hides, a brush-circle wallpaper |
| Glass | floating | frost | Faux frost: each island draws a blurred copy of the wallpaper (labwc has no real blur), aurora wallpaper, white Game Mode button |
| Synthwave | floating | neon | Glowing edges, scanlines, a chrome clock, the sunset-grid wallpaper |
| Cabin | floating | wood | Wood-grain islands, an ember-glow Game Mode button, a wooden clock plaque, the Night Cabin wallpaper |
| Console | shelf | plain | A 10-foot layout: a big status row, a shelf of big app tiles with status lines, the desktops and Show desktop under it, controller hints always shown |

Tokens: `islandStyle` (floating | bar | shelf), `frame` (plain | chamfer | neon | wood | frost), `thin`,
`dockAutoHide`, `deskClock`, `perfIsland`, `edgeGlow`, `scanlines`, `hints`, `desc`, `order`, and the usual
`font`, `corner`, `windowCorner`, `wallpaper`.

## Settings never change by themselves (red flag 9)

- New options are new keys in `desktop.json`, saved one at a time through `SettingsWriter.qml` (merge on write,
  atomic, `.bak`); a key you never changed is never written. `""` means "the look's own".
  `barStyle`, `dockMode`, `deskClock`, `panelGlass`, `sunset`, `sunsetLight`, `sunsetDark`, `accentFrom`, `wallSwatches`.
- **Choosing a look changes only `look`.** The look's wallpaper, bar, dock, clock and transparency are used only
  while yours is `""`.
- **Sunset is off** unless you choose it. When on, light from `sunsetLight` (07:00) to `sunsetDark` (19:30); your
  Dark / Light choice is kept underneath and never rewritten. Theme.qml and hearth-theme work it out the same way.
- **From wallpaper** works the colour out once, when you press it or pick a wallpaper in Appearance with it on (a
  32×32 copy of the picture and a small k-means; the drawn and moving wallpapers have a fixed set). It's stored as
  your accent; a wallpaper set from somewhere else doesn't recompute it.
- Existing users keep their look, accent, mode, wallpaper, pinned apps and dock order: the pins are the same
  `desk.json`; nothing is migrated.

## Desktop Mode always starts (red flag 2)

`shell.qml` loads `Islands.qml` and `Dock.qml` by file name. If either can't load (or the screen is upright), the
classic taskbar (`Bar.qml`: the launcher, every app, the Game Mode button) comes up instead. The look-specific parts
(frames, frost, drawn wallpapers, the media pill, the performance island, the dashboard's cards, From wallpaper)
are loaded the same way, so one bad part only loses itself. Nothing here touches booting, sessions or services.

## What Quickshell 0.3.2 / labwc can't do, and what's used instead

- **Real background blur**: not on labwc (no `ext-background-effect-v1`). Glass draws its own blurred copy of the
  wallpaper (QtQuick.Effects, only with a GPU; with software rendering the copy is shown unblurred under a milky fill).
- **FPS of another app**: no frame-timing protocol; shown only from a fresh MangoHud CSV log (`hearth-stats`).
- **Dock "hide when a window touches it"**: no window positions; it's plain auto-hide (touch the bottom edge).
- **Live window thumbnails** (Console): need screencopy per window; Console shows icons and status lines.
- **Upright screens** keep the classic side taskbar.

## Checks

`hosts/linux/test/test_mantel_2026.py` (the looks, settings, Sunset, Game Mode in every look, the fallback,
hearth-stats) and the Desktop Mode test (`desktop-test.yml`), which renders every look with Quick settings, the
launcher, the dashboard and Settings › Appearance into `looks/mantel/` on the `ci/desktop-shots-*` branch.
