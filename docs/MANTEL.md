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
- **Settings** (the redesign, approved from the mockups; see "Settings" below): 12 categories plus Home, setting
  cards in titled groups, breadcrumbs, a search that finds single settings, and the controller's hints.
- **Settings › Personalization** (was Appearance, `SetAppearance.qml`): the **Look** as eight picture cards; then
  **Your style**: **Mode** Dark / Light / **Sunset**, the **accent** (presets on the card; From wallpaper, a hue and a
  typed colour in "More accent colours"), the **wallpaper**; then **Desktop**: **Bar and dock** (its own page,
  `SetBarDock.qml`), **Desktop clock and weather**, **Panel transparency**, a way to the on-screen keyboard's style.
  Night light moved to System › Display, Reduce motion to Accessibility (the same settings).
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

## Settings (the redesign)

Hearth Settings (`SettingsApp.qml`) opens on **Home** every time. On the left: your account, **search** (Y on the
controller, Ctrl+F, or start typing on the list) and the categories: Home, System, Network & internet, Bluetooth &
devices, Hearth Link, Personalization, Apps, Gaming, Accounts, Home server, Privacy & security, Accessibility,
Updates & recovery. On the right the page under a **breadcrumb title** ("Home server › Services"; click a part to go
back there). At the bottom what the controller does: **A** select, **B** back (a subpage goes up to its category, a
category to the list, the list closes Settings), **Y** search, **LB / RB** previous / next category, **☰** close
(hearth-pad sends Ctrl+F, Ctrl+PgUp / PgDn and Ctrl+W to the Settings window). A big ring shows the controller's place.

- **The registry** (`settings-pages.js`): the categories and every page, each with its one home, label, icon, a
  line for its card, search words and its old name. Adding a page is one entry. Every page id from before still
  opens (deep links, notifications, the launcher, `hearth-settings <page>`); `page:part` opens at a part of a page
  (`keyboard:osk` opens the on-screen keyboard's expander; `appearance:bar` opens Bar and dock).
- **Home** (`SetHome.qml`): this PC (its wallpaper, name and **Rename**, the device name's home now), Wi-Fi /
  Bluetooth / Updates / Hearth Link tiles, Recommended settings (Night light, Light mode while you play, Do not
  disturb, Start in), Bluetooth devices, Storage, Home server, Your look. On the live USB, an Install Hearth banner.
- **Category pages** (`SetCategory.qml`): their pages as cards with a live status where it's cheap to read, then link
  cards to settings whose real home is elsewhere. Gaming (`SetGaming.qml`) and Home server (`SetHomeServer.qml`) have
  their own landing pages; Hearth Link, Personalization, Accessibility and Updates & recovery open their page, with
  the category's other pages as cards at the end.
- **Cards** (`SetRow` / `SetCard`, `SetSwitch`, `SetChoice`, `SetSlider`, `SetCardGroup`, `SetExpander`, `SetChip`,
  `SetBanner`, `SetLinkCard`, `SetButton`, sizes and the good / warn colours in `SetStyle.qml`): icon, title, one
  line, the control or a status chip on the right, a chevron only when it opens a page. Every colour is a Theme token,
  so all eight looks and light mode work. A wide card puts its choices on the right; a narrow one under the title.
- **Search** (`settings-index.js`, made by `tools/build-settings-index.py` from every page's cards): single settings
  with their path ("Go to sleep after · System › Power & battery"), pages, and the pages' old names ("Appearance").
  Opening a result puts the focus on that card (opening its expander if it's folded away).
- **Where things went** (the IA table): Tailscale → Network & internet (a link card stays in Home server); Remote play
  → Gaming (listed in Home server › Services too); Home server › AI → Home server › Services › AI chat; Boot &
  startup → System › Startup (the splash) and Gaming › Startup animation; About's Privacy → Privacy & security ›
  Player count, its Help and logs and Backup → Updates & recovery › Backup and help (one file, `page.part`); Start in
  → Gaming (`StartInChoice.qml`, also on Home); Users → Accounts; Keyboard's on-screen keyboard is an expander.
- **Always opens**: the frame (`SettingsFrame.qml`) is loaded by file name; if it can't load, Settings shows the plain
  list of every page (`SettingsSimple.qml`). A page that can't load says so and the rest keeps working.
- **New installs** start Steam-style with the **Sky** accent: `/etc/skel`'s `desktop.json` says `"defaults": 2`, and
  Theme.qml and hearth-theme give Steam-style's own accent as Sky for such a file. Existing users (no `defaults` key)
  keep Steam blue. Nothing is written for you: it's a default worked out when the file is read.
- **The handheld layout** (a window under 1300 px wide, like 1280×800): a narrower list and tighter cards, the same
  labels.

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
- **Opening a Settings page writes nothing.** The Desktop Mode test opens every page in `settings-pages.js` and checks
  that no settings file changed; the frame, the cards and the category pages never change a setting
  (`test_settings_redesign.py`). Moved switches keep their keys (Night light is still `nightLight`).

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
hearth-stats), `test_settings_redesign.py` (Settings' categories, every old page's home, the search index, the
frame's fallback, the controller, opening writes nothing, the Sky default) and the Desktop Mode test (`desktop-test.yml`), which renders every look with Quick settings, the
launcher, the dashboard and Settings (Personalization and Home) into `looks/mantel/`, and the redesign's pages,
search, controller and handheld layout into `settings-redesign/`, on the `ci/desktop-shots-*` branch.
