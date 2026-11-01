# Steam Game Mode (Experimental)

Steam Game Mode is an **optional, experimental** way to play on Hearth OS with **Valve's own Steam gamepad screens**
(Steam Big Picture in gamescope, the way SteamOS, Bazzite and ChimeraOS run it), with **Hearth brought into Steam**
through Decky Loader and the **Hearth Decky plugin**. Hearth's own Game Mode is still the default and hasn't changed:
Steam Game Mode is something you switch to for a while, never what the device starts in.

## What you get

- **Steam's own Game Mode**: Steam's home, library, store, Quick Access menu, overlay and keyboard, full screen in
  gamescope, the way a Steam Deck works.
- **Hearth's look over Steam** (on by default, one switch to turn off): Hearth's colours, corners, font, focus rings
  (Hearth's double ring and glow), Hearth's wallpaper behind the library and home, Hearth's style for Steam's tabs,
  play bar and Play button, Quick Access menu, main menu, dialogs, footer and on-screen keyboard, and Hearth's logo
  where Steam's main menu shows its own.
- **A Hearth row on Steam's home**: your games from Hearth's other stores at the top of Home, with Hearth's
  wallpaper and logo, like Hearth's home. Press one to play it. (Its own switch.)
- **Quick Access › Hearth** (the Decky menu):
  - **Switch to Hearth Game Mode**: Steam closes and Hearth's Game Mode comes back.
  - **Hearth look** and **Hearth row on Steam's home** switches.
  - **Hearth's apps**: Hearth Store, Hearth Mods and Hearth Theater (each opens as a Steam shortcut, so it gets the
    screen).
  - **Your Hearth games**: your installed games from Epic, GOG, Battle.net, Xbox PC (Xodus), Beacon, emulators and the
    ones you added yourself, each with **Play**.
  - **Sync to Steam library**: adds them all to Steam's library as non-Steam games, with Hearth's cover and banner
    art. Hearth's shortcuts for games you've uninstalled are removed; nothing else in Steam's library is touched.
    **Remove Hearth's shortcuts** takes them all out again.

Games added this way **play through Hearth**: the shortcut runs `nimbus --play-now <game>`, which starts the game the
way Hearth's Play button does (Proton, Epic, GOG, Beacon's mods, Battle.net through Faugus, emulators, your launch
options) and waits until the game has closed, so Steam shows it as running, gives it the screen and its overlay, and
counts the play time. Steam's own games simply play from Steam.

## How to try it

1. On a Hearth OS version that has it, open **Settings › System › Steam Game Mode (Experimental)**.
2. Optional, once: **Decky Loader and the Hearth plugin › Install**. It asks for your administrator password (it adds a
   service that runs as root, see Security below), downloads Decky Loader's newest stable release from its GitHub
   page, adds the Hearth plugin that comes with Hearth OS, and turns on Steam's remote debugging, which Decky needs.
3. **Switch to Steam Game Mode** (also in the power menu). Steam starts in its gamepad screens. If Steam isn't signed
   in yet, Steam asks you to sign in on its own screen; Hearth never sees or keeps your Steam password.
4. With Decky installed, press the **…** (Quick Access) button and open the **plug** tab, then **Hearth**. Press
   **Sync to Steam library** to see your Hearth games in Steam's library.
5. To go back: Quick Access › Hearth › **Switch to Hearth Game Mode**, or Steam's own **Power › Switch to Desktop**
   (Desktop Mode), or just restart: the device **always starts in Hearth's Game Mode**.

## How it's kept safe

- **This time only.** The switch is never saved. Whatever happens in Steam Game Mode, the session comes back to
  Hearth's Game Mode (or Desktop Mode, when you asked for it), and a restart always starts Hearth's. Hearth tried a
  saved "Steam's Game Mode" choice in 0.2.8 and it could leave you stuck; this one can't.
- **Falls back by itself.** No Steam or gamescope, safe graphics (Hearth Recovery), an extra multi-seat player, or
  Steam ending with an error within 20 seconds: you're back in Hearth's Game Mode straight away, and Settings says why.
- **Never touches booting.** Nothing in the boot menu, the login screen or the start mode changes. Decky Loader's
  service is not enabled at boot: the session starts it when Steam Game Mode starts and stops it when it ends, and if
  it fails, Steam Game Mode carries on without Decky.
- **Steam's Switch to Desktop** asks Hearth's session (Hearth's own `steamos-session-select` comes first on Steam's
  path), so it never changes the login screen's session.

## Security: what installing Decky means

- **Decky Loader runs as root** while Steam Game Mode runs, and loads plugins from `~/homebrew/plugins`. Hearth keeps
  Decky's program in `/var/lib/hearth-decky` (owned by root, so nothing running as you can replace what runs as root;
  Decky's own installer keeps it in your home folder). The plugins folder is root's too, as Decky makes it. Plugins you
  add from Decky's store can run code as root if they ask for it, so install only ones you trust. The Hearth plugin
  runs as you.
- **Steam's remote debugging is turned on** (`~/.steam/steam/.cef-enable-remote-debugging`): Steam's screens listen on
  a local port so Decky can add its menu. Any program running on this device can then control Steam's screens through
  that port (including the store, where your account is signed in). It's local only, not reachable from the network.
  Removing Decky from Settings turns it off again (unless you had turned it on yourself).
- Decky Loader itself listens on `127.0.0.1:1337` with its own access token.
- Hearth downloads Decky from its official GitHub releases over HTTPS, the same file Decky's own installer uses; there
  is no published checksum to compare it with. Hearth checks that it's a program and the right size class.

**Remove** in Settings takes away the service, the program, the Hearth plugin and the remote debugging file Hearth
made. Your other Decky plugins and their settings stay in `~/homebrew`. If you installed Decky yourself with Decky's
installer, Hearth leaves that Decky alone and only adds the Hearth plugin.

## What only Hearth's Game Mode has

In Steam Game Mode you have Steam's features instead of these Hearth ones: Hearth's Quick Access (Hearth's
performance, power and controller pages, Hearth Recorder, Quick Resume, Pause), Hearth's library and its store pages
(open **Hearth Store** for those), Hearth Friends, Couch Co-op and multi-seat, Hearth Link's device health, Hearth Input
controller layouts, parental controls and per-game power settings, Hearth's on-screen keyboard, Discord status, and
Hearth's Settings (switch back for those). Xbox Cloud Gaming games aren't added to Steam (they need Hearth's stream
window).

## Limits (Experimental)

- **Valve's updates can break Decky** (Decky patches Steam's screens), and with it the Hearth plugin and look. Steam
  itself keeps working: turn **Hearth look** off, update Decky from its own settings, or remove it in Hearth's.
- The look uses Steam's own colour variables and focus class (stable), plus Steam's component names looked up at run
  time the way CSS Loader themes do. When Steam renames a component, that part simply stays Steam's style.
- Hearth's startup movie: the plugin puts one in Steam's custom boot video folder
  (`~/.steam/root/config/uioverrides/movies/deck_startup.webm`) only if one ships with it, and never replaces yours.
  Hearth doesn't have a startup movie yet, so Steam's own plays.
- Handheld Daemon's Quick Access button may open Hearth's overlay rather than Steam's; use Steam's **…** button.
- Steam's own system settings that need SteamOS (OS updates, some power options) may be missing or do nothing.
- Needs Steam installed (both editions have it) and signed in for the library and store.
- **Needs testing on a device.** CI builds and tests the plugin, the session switch, the Decky helper and draws the
  theme over a static stand-in for Steam's screens, but no real Steam runs in CI.

## For developers

| Part | Where |
| --- | --- |
| The switch (one time, falls back) | `hosts/linux/session/nimbus-session` (`run_steam_mode`), `nimbus-mode steam` |
| Settings, power menu, status | `hosts/hearth-core/src/steam_mode.rs`, `power.rs` ("steam"), `ui/index.html` |
| Playing a Hearth game from Steam | `hearth --play-now <id>` (`hosts/hearth-core/src/desktop_games.rs`) |
| Decky install, start/stop (root) | `os/system_files/usr/libexec/nimbus/hearth-decky` through `hearth-admin decky-*` |
| The plugin | `hosts/decky-hearth` (`src/index.tsx` panel and home row, `src/shortcuts.ts`, `src/theme.ts`, `main.py`) |
| The look | `hosts/decky-hearth/defaults/hearth-theme.css` (also packed as a CSS Loader theme, stable part only) |
| CI | `.github/workflows/decky-hearth.yml` (zip and theme pictures as downloads); images ship it in `/usr/share/hearth/decky-hearth` |

To try a build of the plugin without a new image: download `decky-hearth` from a run of the Hearth Decky plugin
workflow, and in Decky's settings turn on Developer mode, then **Install plugin from ZIP**.
