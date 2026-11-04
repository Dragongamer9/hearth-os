# Hearth Eras (home screens inspired by past console eras, on Steam Game Mode)

**Hearth Eras** is a Decky plugin (`hosts/decky-hearth-eras`) that changes Steam Game Mode's look to a home screen and
skin inspired by a past console era. Every era is **Hearth's own design**: the layout, colours, motion and sounds
evoke an era, but no console maker's names, logos, fonts, sounds or screens are used or copied, and nothing is taken
from other community themes.

It arrives through the plugin channel (docs/PLUGIN-UPDATES.md, "New from Hearth"), not in the OS image, and works only
on Hearth OS (elsewhere the panel is the "Get Hearth OS" invitation and nothing is applied).

Status: built and checked in CI against a stand-in of Steam's screens. **Needs testing on a device.**

## Using it

Quick Access › **Hearth Eras** › **Eras**: a gallery with a small picture of each era. **Steam** (Steam's own look) is
the default, and the default is never written to the settings file. Picking an era applies it at once. Eras with more
than one colour preset show **Colours**.

| Era | Evokes | What you see |
|---|---|---|
| **Channels** | a 2006-style channel grid | 12 rounded "channel" frames with your games on soft silver, the focused one lifts with a glow, its name below; a tray with a big clock, the date and Steam's places |
| **Blades** | mid-2000s sideways panels | glossy slanted panels side by side (Recent, Favorites, a row per store, Places); the one you move onto opens wide, the others stay as labelled edges |
| **Tiles** | a 2020-style big-tile dashboard | a hero banner from the focused game, a strip with one big tile and tall tiles, flat place tiles, a strip per row |
| **Crossbar** | a 2000s category bar | category icons across the screen, the games of the picked one listed under it, slow waves drifting behind, a clock |
| **Pocket** | a handheld-era icon row | your wallpaper, a frosted card for the focused game, a row of round icons, place buttons along the bottom |
| **Hearth Classic** | the old Hearth Game Mode | Hearth Classic's own home and skin (hosts/decky-hearth-look, reused unchanged; its menu bars are never used) |

Every era's home shows your games from Hearth's library (or Steam's library and Hearth Sync's collections when Hearth's
program doesn't answer). **Every game opens Steam's own game page**, so Play is always Steam's. The places (Library,
Store, Downloads, Media, Friends, Settings) open Steam's own pages through Steam's navigator, the way Hearth Classic did
on a device. With nothing to show, **Steam's own Home stays**. Everything is made of Steam's own focusable parts, so the
controller moves through it like through Steam.

**Extras** (each off until you turn it on):

- **Era sounds**: each era's soft move/select sounds on its home, made from plain waves by Hearth as they play (no
  recordings).
- **Quick switching**: hold **L1** and press **D-pad left/right** to change the era (Steam's look is one of them), or
  **L2/R2** to change its colours; a small notification says what's on. Not while a game runs. Saved like a change in
  the panel.
- **Loading screen style**: Steam's loading spinner in the era's colours.
- **On-screen keyboard skin**: the keyboard's panel and focused key in the era's colours.
- **Styled Library pages**: the era's background and panels on the Library, game details and search.

**Diagnostics**: "Show what Hearth Eras found" (state, era, Steam's windows, class names found and missing, quick
switching) and **Send Hearth Eras log** (through Hearth's Send logs service, by hand only; personal details are taken
out first). Each change of state is also written to `~/.local/share/nimbus/hearth-eras.log` (Hearth's Logs app).

## How it works

- **The skin** (`src/skin.ts`, `defaults/eras/*.css`): one `<style>` in every Steam UI window (the main window and its
  popups: Quick Access, the menu, the keyboard), refreshed for windows Steam opens later. Steam's class names come from
  @decky/ui's class maps or findClassModule, never hard-coded; a line naming a class Steam doesn't have is left out.
  The colours are the preset's CSS variables, then `base.css`, then the era's own file. The Hearth Classic era uses
  `hearth-classic.css` without its menu sections.
- **The home** (`src/homes.tsx`): Decky's route patch on `/library/home` swaps in the era's home inside an error
  boundary; any error puts Steam's Home back and pauses Eras.
- **Pausing**: before anything is applied, the class names an era can't do without and Decky's route patching are
  checked (Hearth Classic's REQUIRED list). If one is missing: nothing is applied, Steam's look is on, the panel says
  "Hearth Eras paused: Steam changed its layout" with the reason, and **Try again**. Your choice stays saved.
- **Not attempted, on purpose**: Steam's main menu window is never moved, resized or turned into bars. Hearth Classic
  proved on a device that Steam keeps that window at 241×644 and ignores ResizeTo/MoveTo, and that anything drawn in the
  main window instead isn't visible over games or web pages.
- **Settings** (`settings.json` in the plugin's Decky settings folder): atomic writes with `settings.json.bak`,
  merge-on-write (keys a newer version wrote are kept; one era's preset never resets another's), a damaged file is put
  back from `.bak` or kept as `.damaged` and never overwritten with defaults, the defaults are never written. Only the
  person changes them (the panel, or the quick-switch buttons once they turned those on).
- **Quick switching** listens to Steam's controller state (`SteamClient.Input.RegisterForControllerStateChanges`, the
  `ulButtons` bits) only while it's switched on, and stops on unload.

## Tests (CI: `.github/workflows/decky-hearth-eras.yml`)

- `tests/test_main.py`: settings (defaults, remembered, refused values, merge-on-write, `.bak`, damaged files, atomic),
  the Hearth OS gate, the load mark, the skins and the packed layout, the home data, pictures only from Hearth's and
  Steam's folders, the log and sending it; and the rules: the Classic era's code is Hearth Classic's unchanged, no
  console maker's or console's names, no recordings or fonts shipped, nothing that touches Steam's menu window.
- `tests/eras.test.mjs`: the eras and presets, settings, deciding, the skins (with and without Steam's names, extras),
  quick switching, the sounds' recipes, the homes' data, diagnostics.
- `tests/render.mjs`: the real plugin over a stand-in of Steam's screens: the gallery, every era's home and Library skin,
  a preset, the extras on and off, sounds on and off, quick switching (and not in a game), the fallback to Steam's Home,
  paused, not Hearth OS, unloading; pictures on the `ci/hearth-eras-shots` branch.

## What only a device can show

How each home and skin sits in the real Steam (sizes, Steam's own header over the Home, fonts), controller focus moving
through the homes (Blades' panels and Crossbar's categories open as focus reaches them), the real class names for the
loading spinner and search box (the diagnostics list any that are missing), the quick-switch buttons' bits on each
device's controller, and the sounds' volume.
