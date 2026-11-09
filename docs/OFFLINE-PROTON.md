# Proton offline and without Steam

The owner's red flag: **Hearth OS's built-in Proton must work without Steam and 100% offline.** Every Windows game or
program Hearth starts outside Steam must start on a PC that has never been online, with only what's in the image.

## How

Hearth OS has GE-Proton built in (`/usr/share/hearth/proton/GE-Proton*`, linked into
`/usr/share/steam/compatibilitytools.d`; `GE_PROTON` in `os-bases/peppermint/build.sh`). Two ways start it, and neither
uses umu-launcher, the Steam Linux Runtime, a Steam folder or the internet:

- **hearth-winapps** (Windows apps, docs/WINDOWS-APPS.md) runs Proton's `proton` script itself.
- **hearth-proton-run** (`/usr/bin/hearth-proton-run`, `hosts/linux/desktop/shell/hearth_proton_run.py`) is for
  everything else. It takes umu-run's command line and variables (`<exe> [args]`, `winetricks <verbs>`, WINEPREFIX,
  PROTONPATH, GAMEID, STORE, PROTON_VERB), so prefixes umu made before carry on unchanged, but it only uses Protons that
  are on the disk. With nothing picked it uses the built-in GE-Proton.

Both run Proton the way umu-launcher does outside Steam (`UMU_ID`, an empty `STEAM_COMPAT_CLIENT_INSTALL_PATH`, so
GE-Proton starts the program through its `umu.exe`, not Steam's `steam.exe`). Both also switch the network off for
Proton's own Python: GE-Proton's protonfixes would otherwise look for AMD's FSR4 files online at every start and fetch
winetricks parts for some game fixes. With the network off it behaves as it does on a PC with no internet: it skips
those and carries on. The game itself isn't limited.

## Every launch path

| Path | Code | How it starts Proton now |
|---|---|---|
| Hearth Games: GOG, Epic (legendary) | `hearth-core/src/stores.rs` `proton_launch` | hearth-proton-run |
| Steam-engine direct starts (DRM-free Steam games) | `hearth-core/src/steam_engine.rs` | hearth-proton-run |
| Games you added yourself (.exe) | `hearth-core/src/manual.rs` | hearth-proton-run (Wine only if there's no Proton) |
| Recomps' Windows builds (menu entries) | `hearth-core/src/recomps.rs` | `Exec=hearth-proton-run …` |
| Battle.net, Hearth's own setup (`run.sh`) | `hearth-core/src/battlenet.rs` | hearth-proton-run; an older `run.sh` naming umu-run is rewritten |
| Hearth Mods: Play modded, Mod Organizer 2, Wabbajack lists | `hearth-gaming/src/launch.rs`, `organizer/vfs.rs`, `wabbajack.rs` | hearth-proton-run |
| Couch Co-op (archived) | `os/system_files/usr/libexec/nimbus/hearth-coop` | hearth-proton-run, else umu-run |
| Hearth's older Electron host | `hosts/linux/src/stores/proton.js`, `library/manual.js` | hearth-proton-run, else umu-run |
| Windows apps | `hosts/linux/desktop/shell/hearth-winapps` | Proton directly |
| Steam's own games | Valve's Steam | Steam's Proton (not Hearth's) |

umu-run (the system's, or Hearth's downloaded copy from `tools.rs`) is only used on a system that has no
hearth-proton-run, such as another distro running Hearth's apps.

Not covered by this, by nature: **Faugus Launcher** (when Battle.net is set up through it) runs its own umu-run and
Proton, as Faugus does, and Hearth leaves Faugus's settings alone. Battle.net's installer and sign-in, store downloads,
and Hearth Mods' **Repair** (`hearth-proton-run winetricks`, which downloads the Windows parts you asked for) need the
internet. Starting Proton never does.

## Your Proton choices

A game's saved Proton is a setting and never changes by itself. If the saved one isn't on this device (say, a GE-Proton
you downloaded on another PC), hearth-proton-run starts that run with the built-in one, Hearth shows a message saying
so, and the saved choice stays as it was. It's used again once it's back. A newer Proton you download in Settings is
picked up when you choose it ("GE-Proton (newest)" means the newest one on this device). Nothing is downloaded when a
game starts.

## Tests

- `hosts/linux/test/test_hearth_proton_run.py` (winapps.yml): umu-run's command line, the built-in pick, a missing pick,
  no Steam folder, no network use on the start path, protonfixes kept offline, every Hearth call site wired.
- winapps.yml `offline` job: set up like the image, then with `podman run --network=none` it starts 64- and 32-bit
  Windows test programs through hearth-proton-run (a store game, your own game, a missing saved Proton, a Steam-style
  id) and through hearth-winapps. A fake umu-run on the PATH fails the check if anything calls it
  (`tools/winapps-e2e/e2e.sh offline`).
- base-peppermint.yml runs the same check inside the built image with the network off (`e2e.sh offline-image`), and
  the build stops if hearth-proton-run doesn't pick the built-in GE-Proton.
