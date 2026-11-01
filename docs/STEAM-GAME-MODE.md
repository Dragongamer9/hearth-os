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
- **The Hearth takeover** (on by default, see below): Steam's Game Mode turns into Hearth as far as Decky allows.
- **Quick Access › Hearth** (the Decky menu):
  - **Switch to Hearth Game Mode**: Steam closes and Hearth's Game Mode comes back.
  - **Hearth takeover** and a switch for each of its parts, **Hearth look**, and (with Hearth's home screen off) the
    older **Hearth row on Steam's home**.
  - **Quick settings**: the **power mode** (Saver, Balanced, Performance: Hearth's, through hearth-power, on devices
    that have one) and **Controller layouts** (Steam's are in Quick Access › Controller; Hearth Input's open in Hearth's
    Settings).
  - **Hearth's places**: Hearth home, Hearth Store, Hearth Library, Downloads, Storage, Hearth Mods and Theater.
  - **Your Hearth games**: your installed games from Epic, GOG, Battle.net, Xbox PC (Xodus), Beacon, emulators and the
    ones you added yourself, each with **Play**.
  - **Sync to Steam library**: adds them all to Steam's library as non-Steam games, with Hearth's cover and banner
    art. Hearth's shortcuts for games you've uninstalled are removed; nothing else in Steam's library is touched.
    **Remove Hearth's shortcuts** takes them all out again.

Games added this way **play through Hearth**: the shortcut runs `nimbus --play-now <game>`, which starts the game the
way Hearth's Play button does (Proton, Epic, GOG, Beacon's mods, Battle.net through Faugus, emulators, your launch
options) and waits until the game has closed, so Steam shows it as running, gives it the screen and its overlay, and
counts the play time. Steam's own games simply play from Steam.

## The Hearth takeover

With the **Hearth takeover** switch on (Quick Access › Hearth; on by default), Hearth takes these over:

| Part (its own switch) | What you see |
| --- | --- |
| **Hearth home screen** | Steam's **Home** is Hearth's home: Hearth's wallpaper and logo, **Jump back in** with your last game (Play, Details), a row of Hearth's places (Store, Library, Downloads, Theater, Mods, Storage, Steam Library, Steam Store), then **Continue playing**, **Favorites**, a row per store (Steam, Epic, GOG, Battle.net, Xbox, Beacon, Recomps, Emulators, Added) and **Recently added**, like Hearth's home. Steam's games open Steam's own game page; Hearth's open Hearth's game page. Moved through with the controller like the rest of Steam. |
| (always, with the takeover) **Hearth's pages** | **Hearth Store**: Steam, Epic Games, GOG, Free games, Xbox, Battle.net and Recomps, the same store pages Hearth's Store shows; pressing a game opens Hearth Store to buy, claim or install it. **Hearth Library**: your Hearth library with a tab per store. **Downloads**: Hearth's queue (Epic, GOG, Hearth's Steam installs and Hearth's own Steam client's), with a button to Steam's own Downloads. **Storage**: each drive and how full it is, with a button to Hearth's Settings to move games and clean up. A game's **Hearth page**: its store, play time, size, ProtonDB rating and achievements, **Play through Hearth**. **Theater** and **Hearth Mods** open Hearth's own apps. |
| **Hearth in Steam's menu** | Hearth's places at the bottom of Steam's main menu (the Steam button). |
| **Hearth tabs in the Library** | Epic, GOG, Battle.net, Xbox, Beacon, Recomps, Emulators and Added as tabs in Steam's own Library, after Steam's tabs. |
| **Hearth on game pages** | On a game's page in Steam: "Plays through Hearth" on the games the plugin added, and Hearth's ProtonDB rating and your achievements count (Steam's games too). |
| **Hearth sounds** (off by default) | Hearth's own sounds when moving on Hearth's pages (Steam's sounds stay). |

**What stays Steam's**: Steam's library, game pages and Play button, Steam's store, downloads, friends and chat,
the overlay and Quick Access menu, the on-screen keyboard, Steam's settings and power menu. The takeover adds to
them; it doesn't replace them.

**What's Hearth-only** (switch to Hearth's Game Mode for these): see "What only Hearth's Game Mode has" below.
Buying, installing and uninstalling games from Hearth's stores, Hearth Mods' work and Hearth's Settings open Hearth's
own windows (as Steam shortcuts, so they get the screen and the controller works in them).

### How the pages are made

Hearth's pages in Steam are drawn by the plugin with Decky's own controls (the same focusable buttons Steam's screens
use), in Hearth's look, from Hearth's own answers: the plugin's backend asks `nimbus --steam-mode-data …`
(`hosts/hearth-core/src/steam_takeover.rs`), which gives the same library, store pages, downloads and drives Hearth's
screens show. Showing Hearth's own page (`ui/index.html`) inside Steam was looked at and not used: Hearth's page needs
Hearth running behind it (a second Hearth would also take the controllers and Game Mode's services), and Steam's
window doesn't pass the controller into a page inside it (Steam keeps the controller for its own navigation), so it
couldn't be used with a controller. For the heavy work (buying, installing, Hearth Mods) the plugin opens Hearth's own
windows instead, where everything works.

Nothing new listens on the device: there's no server or port. Hearth answers only the plugin's backend (running as
you), one question at a time, and only questions from a fixed list. It reads; the only things it changes are the power
mode (as Hearth's Quick Access does) and the library's own bookkeeping when Downloads is open.

### When Steam updates

Every change the takeover makes to Steam's screens is wrapped: if Steam's screens aren't shaped the way that part
expects any more, that part **turns itself off**, says why in Decky's log and under **Quick Access › Hearth › Turned
off by itself**, and Steam's own screen shows instead. Hearth's home also falls back to Steam's Home whenever Hearth
doesn't answer or anything in it fails. Parts that turned off get another chance the next time the plugin starts. The
switches turn any part off by hand, and **Hearth takeover** turns them all off at once.

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

## Game pictures in Steam Game Mode

The Hearth row on Steam's home, Hearth's home and pages and the Library tabs show each game's picture with its
**title always at the bottom**. Pictures are tried in order: the portrait cover, then (for Steam's games) Steam's own
picture from its library cache on this device, then the wide art, cropped. If none loads, the card shows the title on
Hearth's panel colour; a card is never blank.

Every kind of picture Hearth keeps is shown: web pictures (from Hearth's picture cache when it has them, so nothing is
downloaded again), Hearth's cache addresses (`nimbus-art://cache/…`, `nimbus-art://file/…`), your own pictures (in
your home folder, `~/…`, `file://…`), in PNG, JPEG, WebP, AVIF or GIF. The plugin reads only pictures (checked from
their first bytes), only from your home folder, `/usr/share`, `/opt` and Flatpak's folder; links pointing elsewhere are
refused. Steam's window can't open files, so the plugin hands these to the page as `data:` addresses of the right type.

**Sync to Steam library** gives each shortcut Hearth's cover and banner. Steam only takes PNG and JPEG there, so WebP,
AVIF and GIF pictures are turned into PNG first (with Pillow if Decky's Python has it, otherwise ImageMagick or ffmpeg
if they're installed); when nothing can convert one, that shortcut simply keeps Steam's own look and the plugin's log
says so.

Pictures that couldn't be shown are counted in the plugin's log by reason (never the picture's path or address), for
example `art: not shown (page art: not in the art cache), 3 so far`. **Send logs** (Hearth's Settings) has a
**Steam Game Mode (Decky plugin)** section: whether Decky and its service are installed and running, whether the Hearth
plugin is there, the newest lines of its log (`~/homebrew/logs/…`) and the parts that turned themselves off.

0.6.60 showed blank cards in the Hearth row: the covers were loaded, but Hearth's look over Steam paints every Steam
dialog button's background (with `!important`), and the row drew each cover as that button's background. Covers are now
pictures inside the card, which the look doesn't touch.

## What only Hearth's Game Mode has

In Steam Game Mode you have Steam's features instead of these Hearth ones: Hearth's Quick Access (Hearth's
performance and controller pages, the power limit, Hearth Recorder, Quick Resume, Pause; the power mode is in Quick
Access › Hearth), buying and installing from Hearth's stores (Hearth Store opens for that), favourites, collections and
game settings, Hearth Friends, Couch Co-op and multi-seat, Hearth Link's device health, Hearth Input
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
  Hearth doesn't have a startup movie yet, so Steam's own plays. Hearth's sounds are made by Hearth's page itself (no
  sound files), so the plugin plays the same sounds on its own pages, only when **Hearth sounds** is on.
- The takeover's parts that change Steam's own screens (the main menu entries and the Library tabs most of all) are
  found by looking at Steam's screens while it runs; CI can't check them against a real Steam, so some may turn
  themselves off on your Steam version until they're adjusted.
- Handheld Daemon's Quick Access button may open Hearth's overlay rather than Steam's; use Steam's **…** button.
- Steam's own system settings that need SteamOS (OS updates, some power options) may be missing or do nothing.
- Needs Steam installed (both editions have it) and signed in for the library and store.
- **Needs testing on a device.** CI builds and tests the plugin, the session switch, the Decky helper and Hearth's
  answers for the pages, draws the theme over a static stand-in for Steam's screens and the takeover's pages with
  made-up games, but no real Steam runs in CI.

## For developers

| Part | Where |
| --- | --- |
| The switch (one time, falls back) | `hosts/linux/session/nimbus-session` (`run_steam_mode`), `nimbus-mode steam` |
| Settings, power menu, status | `hosts/hearth-core/src/steam_mode.rs`, `power.rs` ("steam"), `ui/index.html` |
| Playing a Hearth game from Steam | `hearth --play-now <id>` (`hosts/hearth-core/src/desktop_games.rs`) |
| Decky install, start/stop (root) | `os/system_files/usr/libexec/nimbus/hearth-decky` through `hearth-admin decky-*` |
| The plugin | `hosts/decky-hearth` (`src/index.tsx` panel and start, `src/shortcuts.ts`, `src/theme.ts`, `main.py`) |
| The takeover | `src/patches.tsx` (Steam's screens, each guarded), `src/pages.tsx` (Hearth's pages), `src/model.ts` (switches, guard, pure helpers, unit-tested), `src/api.ts`, `src/ui.tsx` |
| Hearth's answers for the pages | `hearth --steam-mode-data home\|library\|store <s>\|downloads\|storage\|info <id>\|power\|power-mode <m>` (`hosts/hearth-core/src/steam_takeover.rs`) |
| The look | `hosts/decky-hearth/defaults/hearth-theme.css` (also packed as a CSS Loader theme, stable part only) |
| CI | `.github/workflows/decky-hearth.yml` (zip, theme and takeover pictures as downloads and on the `ci/decky-hearth-shots` branch); images ship it in `/usr/share/hearth/decky-hearth` |

To try a build of the plugin without a new image: download `decky-hearth` from a run of the Hearth Decky plugin
workflow, and in Decky's settings turn on Developer mode, then **Install plugin from ZIP**.
