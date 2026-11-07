# Windows apps

Hearth OS 1.7 runs Windows programs (.exe and .msi) with Proton, offline: the newest GE-Proton we could check is built
into the image, so nothing has to be downloaded first.

## For you

- **Open a Windows program** from Hearth Files (double-click, or right-click › **Open with Hearth (Windows app)**), or
  from the app launcher's Open with. Each program gets its own space (a Wine prefix) in **Games › Windows apps ›
  <name>**, so one app can't break another.
- **Installers** (Setup.exe, *.msi) run first. When they finish, Hearth finds the program they installed and that
  becomes the app, with a menu entry and the program's own icon.
- **A program you open directly** (a portable .exe) stays where it is and gets a menu entry. Opening it again starts the
  same app.
- **Settings › Apps › Windows apps** lists them. For each one:
  - **Open**.
  - **Proton**: Automatic (the Proton detector picks), or one of the Protons it would try. One marked ↓ is downloaded
    first, after you say yes (about 500 MB, with a progress bar).
  - **DXVK** (DirectX 9 to 11 through Vulkan) and **VKD3D** (DirectX 12 through Vulkan): on unless a program draws
    wrongly with them.
  - **Windows version**: Automatic, 11, 10, 7 or XP.
  - **In Steam**: Hearth Sync adds it to Steam's library at its next sync, so it's in Steam Game Mode too.
  - **Remove**: deletes the app's space (everything installed in it), its menu entry and icon. The .exe you opened is
    never deleted.
- **When a program closes right away** (an error within 15 seconds), Hearth tells you and the app's card offers
  **Try with <the next Proton>**. The one that works is remembered.

## The Proton detector

When you haven't picked a Proton, Hearth tries, in this order:

1. The Proton the program (or Hearth Games game) last worked with on this device.
2. What Hearth's bundled table says for it (`/usr/share/hearth/proton-rules.json`, shipped with the image; nothing is
   looked up online). The table can also set the Windows version a program needs (Battle.net: Windows 10).
3. The GE-Proton built into Hearth OS (works offline).
4. Valve's newest Proton, if Steam has one installed.
5. Older GE-Protons: ones downloaded before, then ones Hearth can download (only after you say yes; each is checked
   against the SHA-512 in the table).

A Proton that closed right away for that program moves to the end of its list. Hearth Games uses the same table and
memory (`hosts/hearth-core/src/proton_pick.rs`, docs/HEARTH-GAMES.md); a Proton you pick yourself always wins.

## The built-in GE-Proton

- The image build downloads one pinned version (`GE_PROTON` and `GE_PROTON_SHA256` in `os-bases/peppermint/build.sh`)
  and stops if the download or checksum fails, so an image never quietly lacks it.
- It's unpacked to `/usr/share/hearth/proton/<version>` and linked into `/usr/share/steam/compatibilitytools.d`, Steam's
  system folder for compatibility tools, so Steam lists it too. Your Steam settings aren't changed.
- About 1.4 GB unpacked (about 530 MB to download as part of an update).
- Older GE-Protons Hearth downloads go to `~/.local/share/hearth/proton`, not into Steam's folders.
- **Updating it:** change `GE_PROTON` and `GE_PROTON_SHA256` in build.sh and `builtin` in the table together
  (`test_hearth_winapps.py` checks they match).

## For developers

- `hosts/linux/desktop/shell/hearth-winapps`: the helper (open, run, list, set, steam, remove, fetch). `/usr/bin/
  hearth-winapps` runs it; `hearth-winapps.desktop` makes it the handler for Windows programs.
- `hosts/linux/desktop/shell/hearth_proton.py`: the detector. Remembered results: `~/.local/share/nimbus/
  proton-worked.json` (keys `winapp:<id>` and `<store>:<id>`).
- Each app: `~/Games/Windows apps/<name>/` with `prefix/` (Proton's `pfx` points back at it, as umu-launcher does),
  `hearth-app.json` (its settings, written with hearth_settings_io: atomic, .bak, merge-on-write) and `state.json`
  (the last start).
- Proton starts with `proton waitforexitandrun` directly (no Steam Runtime download, so it works offline).
- Add to Steam: `~/.local/share/nimbus/winapps/steam-shortcuts.json`, read by Hearth Sync (`winapps_entries`).
- `SetWinApps.qml`: the Settings page. Tests: `hosts/linux/test/test_hearth_winapps.py`, `proton_pick` unit tests,
  Hearth Sync's `test_windows_apps_add_to_steam_entries`.
