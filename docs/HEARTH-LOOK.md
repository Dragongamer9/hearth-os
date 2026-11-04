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

- **The skin** (stage 1): Hearth's colours (the dark blue-greys and the teal accent), its corners, its double-ring focus
  glow, Hearth's top bar on the Library's tabs (and the teal selected tab), Hearth's side menu (the Steam button's menu,
  with Hearth's logo), Hearth's dialogs, rows, buttons, Play button, footer and on-screen keyboard keys, and Hearth's
  wallpaper behind the Library and the home. The font is Hearth's own font stack, the same one the old Game Mode used
  (it bundled no font: on Hearth OS that's the system's Noto Sans). Switch: **Hearth wallpaper** (on).
- **The home screen** (stage 2): Steam's **Home** becomes the old Hearth home: Hearth's wallpaper and logo, **Jump back
  in** with your last game (**Play**, **Details**), **Hearth's places** (Library, Store, Downloads, Media, Friends,
  Settings), **Continue playing**, **Favorites**, and a row per store: **Steam**, then Hearth Sync's collections (Epic,
  GOG, Battle.net, Xbox, Beacon, Recomps, Emulators, Added by hand, ...). Moved through with the controller like the
  rest of Steam. Every game opens **Steam's own game page** (Steam's games, and the shortcuts Hearth Sync made for your
  other games). Switch: **Hearth home screen** (on).
- **Hearth sounds** (off by default): Hearth's soft move and select sounds on the home screen.

**Steam** puts everything back to Steam's own look at once.

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
- If the home screen throws while drawing, Steam's own Home is shown at once and Hearth Classic pauses the same way.
- Every change to Steam (the skin in each of Steam's windows, the Home route patch, the sounds) is kept in one list
  with its undo. Picking **Steam**, a pause, or Decky unloading or updating the plugin undoes all of it.

## Settings

`settings.json` in the plugin's Decky settings folder (`~/homebrew/settings/decky-hearth-look/`):
`look` (`steam` or `classic`; `steam` by default), `home`, `wallpaper`, `sounds`. Saved the Hearth way
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
- the Play button (`SteamClient.Apps.RunGame`), the Store and Downloads places, and the sounds;
- that the pause and Try again behave on a real Steam update.
