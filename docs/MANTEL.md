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
- **Discord** (`Discord.qml`, since 1.6; it replaced Hearth Friends on Matrix): the bar's chat button. The first press
  asks which Discord (the app, a web window or Dorion); then it toggles Discord's own window, small and placed by the bar
  on whichever edge the bar is (hidden by minimizing, never closed). Settings › Gaming › Discord has the rest. See
  [DISCORD](DISCORD.md).
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

## Bar layouts

Settings › Appearance › Bar and dock (`BarLayoutPicker.qml`):

- **Bar layout**: **Top bar + floating dock** (Mantel 2026's own, the default) or **One main bar** on the edge you pick:
  **Bottom**, **Top**, **Left** or **Right**. The one main bar (`Islands.qml` with `edge`) holds the Hearth button,
  the desktops, the window in front (across the top or bottom), the clock, the status island, notifications, Quick
  settings and the Game Mode button. Left and right stack the icons, the clock is compact (time over AM/PM), the Game
  Mode button is the controller icon in the accent, and menus, tips and panels (Quick settings, the dashboard,
  notifications, the app menus) open toward the screen.
- **Floating app menu** on / off: off puts the app menu button and your apps (`TaskStrip.qml`, every dock behaviour:
  click, middle-click, drag to reorder, right-click menu, Super+1–9, Show desktop) into the main bar, and there's no
  dock. With the default layout that's the top bar with your apps; with One main bar, everything is in one bar.
- **Auto-hide** (`dockMode`, the same switch as before): hides the dock, and in One main bar or with the apps in the bar
  the main bar too, on any edge (touch the edge to bring it back). Each edge reserves its own space (exclusiveZone on
  that edge, 0 while hidden) so maximized windows fill the rest; the bar waits for your settings and is put back on
  screen once when its space changes, the same Game Mode return fix the dock has.
- Changing any of these applies at once: `shell.qml` builds the bar and the dock again on the new edge (so nothing
  stays reserved on the old one). If the new bar can't load, the classic taskbar comes up as always.

## App menus

Settings › Appearance › Bar and dock › App menu (`AppMenuPicker.qml`, picture cards). The menu button is always
the Hearth flame.

| Menu | File | What it is |
|---|---|---|
| Hearth | `Launcher.qml` | Today's big centred panel with Apps and Games tabs |
| Centred | `MenuCentred.qml` | Search, Pinned (dock/bar pins first, then your apps) in pages of 18, All apps A–Z, Recommended (recent apps, games, Settings pages and files), you, Settings and Power |
| Tiles | `MenuTiles.qml` | From the flame's corner: a rail (you, Documents, Pictures, Files, Settings, Power), Most used and A–Z with letter jumps, live tiles in Play / Today / Home server / Apps |
| Start screen | `MenuStart.qml` | The whole screen: groups of live tiles that scroll sideways, All apps below (A–Z in columns), full-screen search |
| Classic | `MenuClassic.qml` | Two columns, glassy in Hearth's colours: frequent apps with jump lists, All apps in place, search; your places, Files, Settings, Devices, Help, Shut down with an arrow |

- All of them share `Launcher.qml`'s lists and actions and load by file name over its window; if one can't load,
  the Hearth menu shows instead.
- **Search** everywhere (`MenuSearch.qml`): apps, games, Settings pages and files in your home folder
  (`hearth-appmenu files`), a best match, filters All / Apps / Games / Settings / Files (LB / RB) and a preview.
- **Controller**: the D-pad moves (a focus ring on the item), A opens, B goes back or closes, X opens the options
  (jump list, Resize), Y brings the keyboard to search, LB / RB turn pages, change filters or switch side; hints show
  in every menu. All eight looks: the menus use the look's colours, fonts and corners (Classic's glass is tinted with
  your accent).
- **Live tiles** (`MenuTile.qml`): what's playing (MPRIS), the weather and your next events (Hearth Weather /
  Calendar), the Home server in short (`hearth-home-server quick`) and games with an update waiting (`hearth-appmenu
  tiles`: Hearth's games list marks `"update": true` when Steam's manifest says UpdateRequired or a paused update;
  other stores don't say, so their games never show there). A tile with no data is a plain icon tile. Right-click or
  X: Small, Medium, Wide, Large, Take off the menu or Move; apps can be added as tiles from the list.
- **Moving tiles** (Tiles and Start screen): drag one with the mouse onto the tile whose place it takes; with the
  controller, Y on a tile (or Move in its menu) picks it up, the D-pad moves it within its group, A drops it, B puts
  it back. Saved per user in `menuTiles` (`order`), only when you drop one somewhere new. A finger still scrolls the
  tiles: press and hold for the menu, then Move.
- **The Tiles menu's search field sits in the bar** next to the flame (Islands.qml, a bar across the top or bottom of
  a screen wider than 1440 px): click it, or just start typing with the menu open; what you type shows there and the
  results fill the menu (MenuSearch). Down a side, on Console or a small screen the field is at the top of the menu.
- **Jump lists** (Classic's arrow, Tiles' right-click / X): the app's recent files from the freedesktop recently-used
  list (`hearth-appmenu recent`), otherwise its own actions from its .desktop file.
- **Picking Tiles or Classic** asks **Match the menu** (One main bar at the bottom, the floating app menu off) or
  **Keep mine** (nothing else changes). Nothing changes without that click.
- **New handheld installs start with the Start screen**: when `appMenu` isn't set, the default is Start screen on a
  handheld (by name, the same list as Hearth Link's device kind) whose `/etc/nimbus/installed.json` says the installer
  had the five menus (`"appMenus": 1`); everyone else gets Hearth. It's worked out when read and never written, so
  existing devices keep what they have.

## Right-click menu

Settings › Personalization › Bar and dock › **Right-click menu** (`MenuStylePicker.qml`, picture cards; search finds
"Right-click menu") picks how every right-click menu looks: the desktop and its icons, the bar and the dock (`Bar.qml`,
`Islands.qml`, `TaskStrip.qml`, `Dock.qml`), the app menus and launcher (`Launcher.qml`, `shell.qml`), all through the
one shared `PopMenu.qml`, and Hearth Files' actions menu (its own overlay, drawn the same way). `Settings.menuStyle`
(`""` = Today, never written until you pick; `Theme.menuStyle`):

- **Today** (`today`): `PopMenu.qml`'s own 268 px menu with 40 px rows, the accent filling the row; submenus replace
  the list. Hearth Files keeps its own 320 px menu in this look.
- **Windows 11 style** (`windows`): a row of quick icons on top, then 30 px rows with icons and shortcuts on the right;
  a soft tint and a small accent bar show where you are; a submenu is a flyout beside the menu (on the left near the
  screen's edge).
- **Compact list** (`compact`): 212 px, 27 px rows, 14 px icons, a soft accent tint with an outline, dim shortcuts,
  danger items in red, thin separators.
- **Tiles and groups** (`tiles`): the quick items as a 4-tile grid (icon and name), then titled groups (small caps) of
  29 px rows with the accent filling the row.

The three new looks are `MenuStyles.qml`, loaded by file name by PopMenu and by Hearth Files; if it can't load (a
warning in the shell's log), today's menu shows, so a problem never loses the menu. The API stays
`menu.popup(screen, x, y, opts, items)`; items may add `quick: true` (the quick icons / tiles; a normal row in Today
and Compact), `shortcut: "Ctrl+C"` (the old `hint` still works) and `group: "Open"` (Tiles and groups' headings; `short: "Trash"` names a tile;
ungrouped items go under "More"). Files marks Cut, Copy, Rename and Move to Trash as quick (Files has no Share yet);
the desktop, the bar's and dock's app menus and the launcher mark their main actions. Keys and controller: Up / Down,
Left / Right in the quick row or the tiles, A / Enter chooses, Right opens a submenu, B / Esc / Left goes back. While
a controller is in use (`DeskState.padSeen`, as the hint strip; in Files, a menu opened with X / the Menu key) a small
**A Choose · B Back** row shows under every look. `PopMenu.cardWidth` gives the menu's width to callers that centre it.

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
  their own landing pages; Network & internet (Wi-Fi), Hearth Link, Personalization, Accessibility and Updates &
  recovery open their page, with the category's other pages as cards at the end (under `MORE`'s heading when it has
  one: "Reach your devices from anywhere" over Tailscale).
- **Updates & recovery** (`SetUpdates.qml`): a status card (up to date / ready to download / ready, restart) whose
  button is the one next step, Update anyway, **Decky plugins** as one expander (`updates:decky` opens it), "If
  something broke" (Go back, and "The previous version always stays startable"), the writable system, then Backup and
  help. The same hearth-admin, hearth-decky and hearth-unlock calls as before; only the layout changed.
- **Wi-Fi** (`SetWifi.qml`): the network you're on as a card at the top (name, signal, band, address; Properties shows
  security, gateway, DNS, IPv6 and MAC; Disconnect; Forget asks once more and removes the saved connection), the
  Wi-Fi switch, Networks nearby, and **Add a hidden network** (name and password, `nmcli ... hidden yes`). Every
  change is NetworkManager's and only on your click; nothing writes Hearth's settings.
- **Titled card groups** (`SetCardGroup`): Sound, Notifications, Power & battery, Date & time, Mouse & touchpad,
  Printers, Apps, Default apps, Controller and Handheld (each Handheld Daemon section) show their cards in titled
  groups, each card with an icon, as in the mockups. Only the layout changed: the same helpers and settings keys.
- **Your devices** (`SetDevices.qml`, docs/HEARTH-LINK.md "Your devices"): under Hearth Link, with a card on Home;
  stand-in devices for checks with `HEARTH_DEVICES_SAMPLE` (desktop-test.yml).
- **AI devices** (`SetHomeAIDevices.qml`, docs/AI-SHARE.md): Home server › AI devices, a card under "On this server"
  on the Home server page, and linked from Services › AI chat and Hearth Link.
- **Cards** (`SetRow` / `SetCard`, `SetSwitch`, `SetChoice`, `SetSlider`, `SetCardGroup`, `SetExpander`, `SetChip`,
  `SetBanner`, `SetLinkCard`, `SetButton`, sizes and the good / warn colours in `SetStyle.qml`): icon, title, one
  line, the control or a status chip on the right, a chevron only when it opens a page. Every colour is a Theme token,
  so all eight looks and light mode work. A wide card puts its choices on the right; a narrow one under the title.
- **Search** (`settings-index.js`, made by `tools/build-settings-index.py` from every page's cards): single settings
  with their path ("Go to sleep after · System › Power & battery"), pages, and the pages' old names ("Appearance").
  Opening a result puts the focus on that card (opening its expander if it's folded away).
- **Where things went** (the IA table): Tailscale → Network & internet (a link card stays in Home server); Remote play
  → Gaming (listed in Home server › Services too); Bar layouts and app menus → Personalization › Bar and dock
  (`appearance:bar` opens it); ComfyUI and Kokoro → Home server › Services › AI on this PC; Home server › AI → Home server › Services › AI chat; Boot &
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
  `barStyle`, `dockMode`, `deskClock`, `panelGlass`, `sunset`, `sunsetLight`, `sunsetDark`, `accentFrom`, `wallSwatches`,
  and for the bar layouts and app menus `barLayout`, `barEdge`, `dockFloat`, `appMenu`, `menuTiles`, and the right-click
  menu's look `menuStyle`.
- **Each control writes only its own key.** The one exception is **Match the menu** (after picking Tiles or Classic),
  which you click, and which sets `barLayout`, `barEdge` and `dockFloat` together.
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

## Server mode (Hearth 1.6)

Server mode (docs/SERVER-MODE.md) is its own light session, not part of Desktop Mode: nimbus-session starts labwc with its
own folder (`desktop/server/labwc`) and one Quickshell app, `shell/server-mode.qml`, built from Mantel's parts (Theme,
Icon, SetStyle, and the same Hearth Settings, `SettingsApp.qml`, opened from its menu). No bar, dock, launcher or desktop.
Settings has two new pages: Home server › **Server mode** (`SetServerMode.qml`) and System › **Startup**
(`SetStartup.qml`: Start in, with Server mode once it's on, and its optional schedule); the boot splash page is now
System › **Boot splash**. Start in (`StartInChoice.qml`) now saves through hearth_settings_io's `conf-set` (only that line,
atomic, `.bak`, under the session's lock) instead of `sed -i`.

## Desktop Mode always starts (red flag 2)

`shell.qml` loads `Islands.qml` and (with the floating app menu on) `Dock.qml` by file name, once your settings are
read (at most a few seconds) and again when the bar layout changes. If either can't load (or the screen is upright),
the classic taskbar (`Bar.qml`: the launcher, every app, the Game Mode button) comes up instead. Your apps in the bar
(`TaskStrip.qml`), the pickers and the app menus load by file name too; a menu that can't load falls back to Hearth's. The look-specific parts
(frames, frost, drawn wallpapers, the media pill, the performance island, the dashboard's cards, From wallpaper)
are loaded the same way, so one bad part only loses itself. Nothing here touches booting, sessions or services.

## What Quickshell 0.3.2 / labwc can't do, and what's used instead

- **Real background blur**: not on labwc (no `ext-background-effect-v1`). Glass draws its own blurred copy of the
  wallpaper (QtQuick.Effects, only with a GPU; with software rendering the copy is shown unblurred under a milky fill).
- **FPS of another app**: no frame-timing protocol; shown only from a fresh MangoHud CSV log (`hearth-stats`).
- **Dock "hide when a window touches it"**: no window positions; it's plain auto-hide (touch the edge).
- **Live window previews in the menus**: no per-window screencopy here either; the menus show icons and names.
- **Live window thumbnails** (Console): need screencopy per window; Console shows icons and status lines.
- **Upright screens** keep the classic side taskbar.

## Checks

`hosts/linux/test/test_mantel_2026.py` (the looks, settings, Sunset, Game Mode in every look, the fallback,
hearth-stats), `test_settings_redesign.py` (Settings' categories, every old page's home, the search index, the
frame's fallback, the controller, opening writes nothing, the Sky default), `hosts/linux/test/test_mantel_bars_menus.py`
(bar edges, reserved space and auto-hide, the rebuild and fallback, the five menus, settings only written when you
change them, the handheld default, hearth-appmenu), `test_mantel_menu_styles.py` (the right-click menu's looks: Today by
default and never written by itself, the fallback, the item fields) and the Desktop Mode test (`desktop-test.yml`), which renders every
look with Quick settings, the launcher, the dashboard and Settings (Personalization and Home) into `looks/mantel/`,
each bar edge, the dock off, auto-hide, each app menu, search, the picker and other looks into `looks/mantel/bars/`,
the redesign's pages, search, controller and handheld layout into `settings-redesign/`, and each right-click menu
look (a file on the desktop, the desktop, Hearth Files, the flyout, the controller hints, light mode, the Hearth look
and the picker) into `menu-styles/`, on the
`ci/desktop-shots-*` branch.
