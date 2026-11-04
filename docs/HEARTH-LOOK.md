# Hearth Classic (the old Hearth Game Mode look on Steam Game Mode)

**Hearth Classic** is a Decky plugin (`hosts/decky-hearth-look`) that brings back the look of Hearth's own Game Mode
(the archived `ui/index.html`) on top of Steam's Game Mode. It's **off until you turn it on**: Quick Access ›
Hearth Classic › **Game Mode look**: **Hearth Classic** or **Steam**. The plugin never changes that choice by itself.

It's a Hearth plugin like Hearth Sync, Hearth Link and Hearth Music: Hearth OS only (anywhere else its panel is the
"Get Hearth OS" invitation and nothing is applied), and it arrives and updates through the plugins channel
(docs/PLUGIN-UPDATES.md); `decky-plugins-release.yml` finds it by itself.

Status: built and checked in CI against a stand-in of Steam's screens. **Needs testing on a device** (Steam's live UI
isn't available in CI; see "What only a device can show" below).

## What you get

With **Hearth Classic** picked:

- **The skin** (stage 1): the old Game Mode's look over Steam's screens, all through @decky/ui's class maps:
  - Hearth's colours (the dark blue-greys and the teal accent), corners and double-ring focus glow; Hearth's wallpaper
    behind the Library and the home (switch: **Hearth wallpaper**, on).
  - **Library**: Hearth's top bar on the tabs (pills, the picked one teal, its count badge), section headings like
    Hearth's (small, upper case, dim), Hearth's cards (10 px corners, the glow when focused).
  - **Game pages**: the hero fading into Hearth's background, Hearth's play bar with the teal Play button, the details on
    Hearth's cards.
  - **Quick Access**: Hearth's panel and tab rail (the picked tab teal), Hearth's section headings and rows.
  - **Dialogs, settings and menus**: Hearth's rounded rows, teal toggles and buttons, Hearth's context menus, footer and
    on-screen keyboard keys.
  - The font is Hearth's own font stack, the same one the old Game Mode used (Noto Sans on Hearth OS).
- **The home screen** (stage 2): Steam's **Home** becomes the old Hearth home (ui/index.html): a soft banner from the
  focused game's art with Hearth's logo and "Home", **Recent games** (the newest one big and wide, the rest portrait,
  store badges top right, the focused game named under the row), **Hearth's places**, **Favorites**, and a row per store
  (Steam, then Hearth Sync's collections). Every game opens **Steam's own game page** (Play is there). Switch:
  **Hearth home screen** (on).
- **Hearth bottom bar menu** (on): the Steam button's menu becomes the old Game Mode's **Bottom Dock**: a floating bar
  along the bottom, Hearth's icons alone, the focused entry big, teal and named. The entries are Steam's own and do what
  they always do.
- **In a game: Hearth top bar** (on, under the bottom bar switch): while a game runs, the same bar at the top of the
  screen instead (the old in-game top dock), a little smaller. Back to the bottom bar when the game closes.
- **Hearth sounds** (off by default): Hearth's soft move and select sounds on the home screen.
- **Hearth Music on the home**: focusing a game's tile on the Hearth home plays its Hearth Music theme, the same way
  highlighting it in Steam's library does (Hearth Music's own player, settings and songs: with "Where" set to the
  library, its volume, and nothing when Hearth Music isn't installed or is off). Each tile says which game it is
  (`data-hearth-appid`), which Hearth Music's focus watch reads; nothing is matched or played by Hearth Classic itself.

**Steam** puts everything back to Steam's own look at once.

## How the menu bars find Steam's menu (round 3)

Round 2's bars didn't show on a device: Steam draws its main menu (and Quick Access) in **its own window**, a popup of
the main window that CSS Loader calls "MainMenu" ("QuickAccess"), and the skin only went into the main window. Round 3:

- The skin goes into every one of Steam's windows: the main window, `Router.WindowStore`'s windows, every popup in
  `g_PopupManager`, and the window of every gamepad navigation tree.
- The menu is found by the structure CSS Loader themes style on current Steam: `#MainNavMenuContainer` (the menu's
  Focusable, its `navID`) `> .Panel > * > .Panel.Focusable` (the entries). The bar's rules only apply under
  `html.hearth-bar`, set on that window only when this structure is there with at least three labelled entries.
- On a device Steam's menu window is a narrow popup at the left (241 px on the owner's, round 4), so the bar is made to
  fit in one of three ways, in this order (Diagnostics says which, with the window's size before and after):
  1. **Resized**: the menu's own window is made as wide as the screen and bar-high, at the bottom, or at the top while a
     game runs (also when the menu opens over a game), with that window's `SteamClient.Window` `ResizeTo`/`MoveTo`
     (or `window.resizeTo`/`moveTo`). Its size is read back; its old bounds are put back when the bar goes off,
     pauses or the plugin unloads. If Steam narrows it again, it's resized again when the menu opens.
  2. **Mirror**: if the window can't be resized, Hearth's bar is drawn in Steam's main window while the menu is open,
     and Steam's menu panel is made see-through but stays where it is, focused and working: up and down in Steam's
     menu move the bar's highlight, A presses Steam's real entry, the Steam button still opens and closes it.
  3. **Hearth's colours on Steam's own menu**, when neither works. A bar is never squeezed into a narrow window.
- Round 5 (from the owner's device on 1.2.3, where the bottom bar works):
  - Only the menu that's open (focused) is resized, never a menu window Steam keeps but isn't showing.
  - After resizing, a **self-check** makes sure every entry is really visible in the window (inside it, not hidden, not
    covered). If not, the window goes back where it was and the mirror takes over; in a game, Steam's menu in Hearth's
    colours (the top dock showed a blank strip on the device). Never a blank bar.
  - Steam's main window keeps an empty place for its side menu while the menu is open (the blank menu behind the bar):
    it's hidden while Hearth's bar is up and back as soon as it isn't.
  - On web pages (the Store), which Steam draws above its other windows, the menu stays in Steam's own place in
    Hearth's colours, and the bar comes back when you leave the page.
  - Diagnostics also show the page, the self-check and the main window's menu place.
- When the menu is on screen, the bar is checked for real: across the window, along the bottom (the top in a game),
  the entries in one row and the first one reachable. If not, it comes off at once (the next way above).
- If the structure isn't there, nothing touches the menu and the panel says why.
- A running game is read from Steam's `Router.MainRunningApp`.
- The controller: Steam's menu keeps its own navigation (up and down), which now moves along the bar.

**Diagnostics** (Quick Access › Hearth Classic › Diagnostics): "Show what Hearth Classic found" lists Steam's windows
(their titles), the class names found and missing, whether the menu was found (its entries), the bar's state and why,
and whether a game runs. "Write a Hearth Classic log" saves it to Decky's log and to
`~/.local/share/nimbus/hearth-classic.log`, which Hearth's Logs app lists; every change of the bar's state is written
there too.

## Sending the diagnostics to Hearth (round 6)

Quick Access › Hearth Classic › Diagnostics:

- **Send diagnostics to Hearth automatically** (off by default; only you turn it on): when the bar pauses or falls
  back, the self-check fails, the mirror is used or Steam refuses a resize, and once at the menu's first opening after
  each start, Hearth Classic sends its diagnostics through Hearth's own **Send logs** service (the same one as
  Settings › System › Send logs, `hearth-core` `logs.rs` `send_report`: the counter worker in `counter/` keeps it for
  14 days under a code and files it as an issue in the private repo `Dragongamer9/hearth-logs`). At most one send per
  reason per start and ten a day.
- **Send Hearth Classic log now**: the same, by hand. **Last sent: <time> (code XXXX-XXXX-XX)** shows the newest.
- Only Hearth Classic's own diagnostics and `hearth-classic.log` go: the home folder, the user's and Steam's account and
  persona names, web addresses' queries, Steam ids, e-mail addresses and anything that looks like a token are taken out
  first.
- Every send includes the **deep diagnostics**, taken while the menu is open: the windows (which one has the focus),
  `Router.MainRunningApp`, the menu window's bounds before and after, what `MainNavMenuAnchor` matched, an outline of
  the main window's elements over the side menu's place or named like a menu, nav, sidebar, overlay, backdrop or modal
  (tag, id, classes, box, position, opacity, visibility, display, z-index, background, transform), the menu window's
  size, background, `#MainNavMenuContainer` with its parents, its panel and entries, the menu window's other content,
  and the self-check per entry.

Reading them (the owner, with `gh`):

```
gh api 'repos/Dragongamer9/hearth-logs/issues?state=all&per_page=30' -q '.[] | select(.title | test("Hearth Classic")) | "\(.number) \(.created_at) \(.title)"'
gh api repos/Dragongamer9/hearth-logs/contents/logs/<date>/<code>.txt -H 'Accept: application/vnd.github.raw'
```

(`<date>` is the day it was sent, YYYY-MM-DD; the issue links the file.)

## The places row

Each place opens Steam's own page through Steam's window navigator (the calls Steam's own menu makes, from @decky/ui's
SteamUIStore types), trying the next way when one isn't there, and the Library at worst, never an empty page:

| Place | Opens | If that's missing |
| --- | --- | --- |
| Library | Steam's Library (`/library`) | |
| Store | Steam's store (`Navigator.SteamWeb`, store.steampowered.com) | the window's `NavigateToSteamWeb`, then `/steamweb` (the route Steam's own menu uses) |
| Downloads | Steam's downloads (`Navigator.Downloads`) | Settings › Downloads, then the Library |
| Media | Steam's media, recent screenshots and clips (`Navigator.Media.Grid`) | the list view, then the Library |
| Friends | Steam's Friends & Chat (`Navigator.Chat`) | Settings › Friends, then the Library |
| Settings | Steam's Settings (`Navigator.Settings`) | `/settings` |

1.2's places used the routes `/store`, `/downloads`, `/media` and `/friends`, which Steam doesn't have: those showed
empty pages (fixed in round 2).

## Where the home's games come from

Light: no second Hearth runs behind it. Once a minute at most, the plugin's backend asks Hearth's program for its home
layout (`nimbus --steam-mode-data home`, `hosts/hearth-core/src/steam_takeover.rs`, the same answer the archived
takeover used) and reads Hearth Sync's `shortcuts.json` (which Steam shortcut belongs to which Hearth game). Only games
Steam can open are shown: a game Hearth Sync hasn't put in Steam (or whose store is switched off there) is left out.
If Hearth's program doesn't answer, the home is made from Steam's own library instead: your recent games, Steam's
Favorites, your Steam games and Hearth Sync's collections. With nothing to show, Steam's own Home stays.

Pictures are Steam's own (what Steam shows for the game, including the art Hearth Sync gave its shortcuts), then
Hearth's; a picture on the device is only read from Hearth's library and Steam's own picture folders.

## When Steam changes (the safety net)

Steam updates can rename or move the parts of its screens a skin styles. Hearth Classic checks Steam first, every time
it starts and every time you change something:

- The class names are never hard-coded: they come from `@decky/ui`'s class maps (`gamepadTabbedPageClasses`,
  `gamepadDialogClasses`, `quickAccessMenuClasses`, `footerClasses`) or `findClassModule`.
- If the names the look needs most (`src/model.ts` `REQUIRED`), or Decky's route patching for the home, are missing,
  **nothing is applied**: Steam keeps its normal look, Quick Access shows **"Hearth Classic paused: Steam changed its
  layout"** with the reason and a **Try again** button, and the reason goes to Decky's log. Your choice stays saved.
- A less important name that's missing only leaves its one line of the skin out (the look gets plainer, never broken).
- The menu bars go in whole or not at all: if Steam's main menu names (`gamepadUIClasses.MainNavMenuAnchor`,
  `mainMenuAppRunningClasses.NavigationColumn` and `NavigationMenuItem`; in a game also `AppColumn`) are missing,
  Steam's own menu stays, the rest of Hearth Classic goes on, and the panel says "Hearth bottom bar menu paused: Steam
  changed its menu".
- If the home screen throws while drawing, Steam's own Home is shown at once and Hearth Classic pauses the same way.
- Every change to Steam (the skin in each of Steam's windows, the Home route patch, the sounds) is kept in one list
  with its undo. Picking **Steam**, a pause, or Decky unloading or updating the plugin undoes all of it.

## Settings

`settings.json` in the plugin's Decky settings folder (`~/homebrew/settings/decky-hearth-look/`):
`look` (`steam` or `classic`; `steam` by default), `home`, `wallpaper`, `menu`, `menuTop`, `sounds`. Only what
you change is written (the defaults never are). Saved the Hearth way
(docs/SETTINGS-PERSISTENCE.md): atomic writes, the last good copy as `settings.json.bak`, merge-on-write (keys a newer
version wrote are kept), a damaged file put back from the `.bak`, and never replaced with defaults: if neither can be
read, the plugin shows the defaults (off) but leaves the file alone until you change something, and keeps the damaged
file as `settings.json.damaged`. Starting the plugin never writes settings.

## What's Hearth's own design

Only Hearth's: the colours, corners, focus ring, top bar, side menu, tiles, store badges, sounds, logo and wallpaper all
come from the old Hearth Game Mode (`ui/index.html`, `ui/wallpapers`, `site/img/logo.svg`) and the archived Hearth
takeover (`archive/hosts/decky-hearth`), which are the owner's own. No other console's or launcher's look.

## Tests and pictures

`.github/workflows/decky-hearth-look.yml`:

- **Backend** (`tests/test_main.py`): settings off by default, remembered, atomic with `.bak`, merge-on-write, a damaged
  file never overwritten with defaults; the Hearth OS gate; the load mark; the home data (Hearth's layout, Hearth
  Sync's shortcuts, at most once a minute, errors); pictures only from the allowed folders.
- **Pure parts** (`tests/model.test.mjs`): settings, the fallback when Steam's modules are missing or renamed, the skin
  template (lines left out, never an open comment), patches all removed on unload, the home data mapping (from
  Hearth's layout and from Steam's library), pictures.
- **Pictures and checks** (`tests/render.mjs`): the real plugin over a stand-in of Steam's Game Mode (`tests/mock/`):
  the home, the skin on a Library page before and after, the Quick Access panel (Steam look, picking Hearth Classic,
  on, paused after a Steam change, not Hearth OS), unloading (the skin and the route patch are gone, Steam's Home is
  back), and the old Hearth Game Mode home (`site/img/gm-home.webp`) beside the new one. The pictures go to the
  `ci/hearth-look-shots` branch and the run's artifacts.
- `decky-plugins-release.yml` and `tools/decky_plugins.py check` (the shared Hearth OS check and the load mark).

## What only a device can show

CI never runs Steam, so these need a Hearth OS device:

- that Steam's real class maps have the names the skin uses, and how the skin looks on Steam's real screens (the
  stand-in only approximates them);
- that Steam's Home route takes the home screen (Decky's `routerHook.addPatch("/library/home")`) and that controller
  focus moves through it as expected (Steam's focus system isn't in the stand-in);
- that Steam's own pictures (`appStore` capsules and heroes) show on the tiles;
- the places: that Steam's navigator opens the Store, Downloads, Media and Friends pages (the calls are Steam's own,
  but this Steam's names need a device);
- the bottom bar and top bar: how Steam's real menu sits in them (the stand-in only approximates its layout), that its
  entries get their icons (English labels; other languages keep only their labels), and the controller: Steam's menu
  still moves with up and down (its own navigation), now along the bar;
- Hearth Music on the home: that Steam's focus reaches the tiles as it does in the library;
- the Play button (`SteamClient.Apps.RunGame`), the Store and Downloads places, and the sounds;
- that the pause and Try again behave on a real Steam update.
