# Steam Game Mode

**Hearth OS is Debian + Mantel (Desktop Mode) + Steam Game Mode.** Steam Game Mode is Hearth OS's Game Mode: **Valve's
own Steam gamepad screens** (Steam Big Picture in gamescope, the way SteamOS, Bazzite and ChimeraOS run it), with
**Hearth brought into Steam** through Decky Loader and the **Hearth Decky plugin**. The device starts in it, and every
"Game Mode" button (Mantel's taskbar, power menu, launcher, Quick Access, Steam's own Game Mode switch) opens it.

Hearth's own Game Mode (Hearth fullscreen in gamescope) is **archived**: it's gone from the menus and Settings, and
runs only for first-time setup on a fresh install (once), as the live USB's installer, and as the very last resort when
nothing else can run. Its code stays in the repo; `HEARTH_LEGACY_GAME_MODE=1` in `~/.config/nimbus/session.conf` brings
it back as "Game Mode" (a developer switch, off by default). **Multi-seat and Couch Co-op are dropped for now** (hidden,
code kept; `HEARTH_LEGACY_MULTIPLAYER=1` brings them back for developers): the sections about them below describe the
kept code.

## Where the device goes

| Situation | Where the PC goes |
| --- | --- |
| Turning on / signing in | Steam Game Mode |
| Fresh install, first sign-in | First-time setup in Hearth's app (Wi-Fi, store accounts, controller, games), once; Steam Game Mode when it's done or skipped |
| Any "Game Mode" request (`nimbus-mode game`, Mantel's Game Mode buttons, Steam's Game Mode switch, `NIMBUS_START_MODE`, `hearth.mode=game`) | Steam Game Mode |
| Steam's **Power › Switch to Desktop**, Quick Access › Hearth › **Desktop Mode** | Desktop Mode (Mantel, or the desktop picked in Settings) |
| `hearth.mode=desktop` on the kernel command line, **B or Guide held** as the session starts, Recovery's **Desktop Mode this time**, Settings › Sign-in › Start in › Desktop Mode | Desktop Mode (no "didn't start" notice: you asked) |
| Steam or gamescope not installed | Desktop Mode, with the reason and **Try Steam Game Mode again** |
| Safe graphics (Recovery's **Safe graphics this time**) | Desktop Mode (Hearth Classic), with the reason |
| Steam ends with an error within 20 seconds | Desktop Mode, with the reason |
| Steam closes by itself after a while | Steam Game Mode again (as SteamOS does) |
| Two failed runs within 10 minutes (errors, or closing by itself within 2 minutes, or starts that hung) | Desktop Mode, and Steam Game Mode is **paused**: every start opens Desktop Mode until you try again |
| Desktop Mode can't start either | Steam Game Mode if it hasn't just failed, else Hearth's archived Game Mode as the last resort |
| Live USB / installer | Hearth's app (the installer), as before |

## When Steam Game Mode can't start

The session writes why in `$XDG_RUNTIME_DIR/hearth-steam-mode-failed` (and its log, `~/.local/share/nimbus/session.log`)
and opens Desktop Mode. **Mantel** (Hearth Classic) shows **"Steam Game Mode didn't start"** with the reason and two
choices, **Try Steam Game Mode again** (`nimbus-mode steam`) and **Stay in Desktop Mode**, with a controller (Up/Down,
A, B) or a mouse (`hosts/linux/desktop/shell/SteamFallback.qml`). (Mantel is the only desktop; the notification Hearth
Plasma, GNOME and COSMIC got instead is gone with them.) It's shown once per failure (`$XDG_RUNTIME_DIR/hearth-steam-notice-shown`), and Settings
› System › Steam Game Mode shows the last reason too.

It never loops: the session doesn't go back and forth between Steam Game Mode and Desktop Mode by itself. Every Steam
Game Mode run counts as a failure until it has run 2 minutes or you switched on purpose (so a start that hangs or takes
the device down counts too); one error within 20 seconds opens Desktop Mode for this time; two failures within 10
minutes pause Steam Game Mode (`~/.local/state/nimbus/steam-default-off`, the count in `steam-default-fails`), and the
device then starts in Desktop Mode until you try again. A try you ask for always runs, and a good run ends the pause.
Desktop Mode's own watchdog (labwc autostart) doesn't send you back to Game Mode while Steam's failure is fresh.

## First-time setup and Hearth Recovery

- **First-time setup** is Hearth's app (Settings' Wi-Fi, store sign-ins, the controller, importing games), shown once
  on a fresh install. Until it's done or skipped (`setupDone` in `~/.local/share/nimbus/library.json`), the session
  runs it in Hearth's app instead of Steam Game Mode (mode "setup", the archived Game Mode's code, used only for this).
  When it finishes, Hearth asks the session for Game Mode and the device goes to Steam Game Mode (or Desktop Mode with
  the reason, when Steam can't start, e.g. not installed or no internet for Steam's first update). Store sign-ins that
  go through Desktop Mode come back to setup. If Hearth's app closes or crashes during setup, the device goes to Steam
  Game Mode, and setup is offered again at the next sign-in. A Hearth that isn't Hearth OS's never shows it.
- **Hearth Recovery** (on the text screen before the session, `hearth-recovery`) doesn't depend on any Game Mode:
  **Desktop Mode this time** opens Desktop Mode, **Safe graphics this time** opens Hearth Classic with a "safe graphics"
  notice (Steam Game Mode doesn't run with safe graphics), and going back to the previous version is ostree's, as before.

## What you get

- **Steam's own Game Mode**: Steam's home, library, store, Quick Access menu, overlay and keyboard, full screen in
  gamescope, the way a Steam Deck works.
- **Hearth's look over Steam** (on by default, one switch to turn off): Hearth's colours, corners, font, focus rings
  (Hearth's double ring and glow), Hearth's wallpaper behind the library and home, Hearth's style for Steam's tabs,
  play bar and Play button, Quick Access menu, main menu, dialogs, footer and on-screen keyboard, and Hearth's logo
  where Steam's main menu shows its own.
- **The Hearth takeover** (on by default, see below): Steam's Game Mode turns into Hearth as far as Decky allows.
- **Quick Access › Hearth** (the Decky menu):
  - **Desktop Mode**: Steam closes and Desktop Mode (Mantel) opens. (The old **Switch to Hearth Game Mode** is gone
    with Hearth's own Game Mode.)
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

Each shortcut runs a tiny wrapper first (`/bin/sh -c …`, the same for every shortcut): Steam starts shortcuts with its
own runtime's libraries and its overlay preloaded, which broke Hearth's program in 0.6.60 (the games didn't start).
The wrapper takes those out before Hearth starts, and decides at every launch which Steam is running: inside Flathub's
Steam it starts Hearth on the system through `flatpak-spawn --host`. Hearth then cleans the rest of Steam's settings
for its own games (the Steam Runtime's and pressure-vessel's variables, Steam's compatibility-tool paths, Steam's
runtime folders in PATH) and gives Steam's overlay library back to the game only, so the overlay still works over
it. Every start and failure is in Hearth's log (`play-now:` lines, with the reason). Shortcuts synced before are
brought up to date by themselves each time the plugin starts.

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

Nothing listens on the network: there's no server or port. Hearth answers only the plugin's backend (running as
you), and only questions from a fixed list: through Hearth's core service while Steam Game Mode runs (below), or one
question at a time on Hearth's command line when it isn't running. It reads; the only things it changes are what
Hearth's own Quick Access changes (the power mode, a game's Hearth Input layout, its Proton version, a screenshot,
recording) and the library's own bookkeeping when Downloads is open.

## Hearth inside Steam's menus, Quick Access and in-game overlay (Experimental)

The next step of moving Hearth into Steam's Game Mode: Hearth's places, power options, Quick Access and the settings
for the game you're playing now sit inside Steam's own menus. Each has its own switch in **Quick Access › Hearth**
(under **Hearth takeover**), and each turns itself off on its own if Steam changes the screen under it.

### Hearth's core in Steam Game Mode

When Steam Game Mode starts, Hearth's session also starts **Hearth's core without a window** (`nimbus --core-service`):
it gives the plugin live answers (the menus and Quick Access no longer start Hearth's program for every question) and
tells **Hearth Input** which game is in front, so a game's Hearth controller layout works in Steam Game Mode too. It
stops with Steam (and by itself when Steam Game Mode ends). It's guarded: if it doesn't start, Steam Game Mode carries
on and the plugin asks Hearth's command line as before.

What it never does: no window, nothing about gamescope's focus (it only reads which game gamescope shows), no pad mouse,
no keeping Steam ready or closing Steam, no InputPlumber or Handheld Daemon takeover, no controllers of its own. Hearth
Input holds the controller **only** while a game with a Hearth Input layout is in front: the plugin says every 2 seconds
which game Steam runs, gamescope says whether it's the one on screen, Hearth's Quick Access tabs pause it while they're
open, and if the plugin stops saying anything for 6 seconds Hearth Input lets go by itself.

The socket is `$XDG_RUNTIME_DIR/hearth/core.sock` (`/run/user/<you>/hearth`: the folder only you can open, the socket
0600), and both ends check that the other is you (SO_PEERCRED). Only a fixed list of questions is answered
(`hosts/hearth-core/src/core_service.rs`, `COMMANDS`); anything else is refused. Put `HEARTH_STEAM_CORE=0` in
`~/.config/nimbus/session.conf` to leave it (and Hearth Input) out of Steam Game Mode.

### Steam's main menu (the Steam / Xbox button)

Below Steam's own entries (switch: **Hearth in Steam's menu**), in Hearth's look:

- **Now playing** (while a game runs, switch: **Hearth for the game you're playing**): the game, opening its Hearth
  section (below).
- **Hearth**: Hearth Home, Hearth Store, Hearth Library, Theater, Hearth Mods, Storage, Downloads, Hearth Friends,
  Hearth Link, Couch Co-op. Hearth Home is Steam's Home when Hearth's home screen has it, else a page of its own.
- **Hearth power**, beside Steam's own Power: **Switch to Hearth Game Mode** and **Desktop Mode** (asks first while a
  game runs, since Steam closes).

### Hearth's tabs in Quick Access (the … button)

Decky Loader has no way for a plugin to add its own top-level Quick Access tabs: Decky adds its single plug tab to
Steam's tab list itself. The plugin adds Hearth's tabs **through Decky's own tab list** (the same list Decky's plug tab
is in), so they sit beside Steam's tabs (switch: **Hearth tabs in Quick Access**). If Decky changes that list, the part
turns itself off and the same sections stay in Decky's Hearth tab.

| Tab | What's in it |
| --- | --- |
| **Hearth** | **This game** (below), **Quick Resume** and **Game streaming** (below), **Hearth tools** (Keyboard: Steam's on-screen keyboard, also Steam + X; Screenshot: Hearth's, to Pictures › Screenshots › Hearth; Record: Hearth Recorder), **Switch to Hearth Game Mode**, **Hearth OS updates** |
| **Hearth Performance** | Hearth's **power mode** (Saver, Balanced, Performance, through hearth-power) and Hearth's **power limit**, read only. The frame limit, refresh rate and TDP stay Steam's own Performance tab's in Steam Game Mode, so the two never fight. |
| **Hearth Players** | Couch co-op: the controllers connected, as **P1** to **P4** in Hearth's player order, **Move up** / **Move down** and **Rumble** (to tell which controller is which), and **Split-screen games** (below). **Seats**: multi-seat beside Steam Game Mode (below). |
| **Hearth Controller** | What Hearth Input is doing now, **Hearth Input in Steam Game Mode** (on/off), the running game's **Hearth Input layout** and **Edit layouts in Hearth** (Hearth's editor in a window). Steam's own layouts stay in Steam's Controller tab: for a game with a Hearth layout, set Steam's to Gamepad or none, or the game gets both. |

### The game you're playing

While a game runs, Steam's main menu (**Now playing**), Quick Access › Hearth (**This game**) and its own page show
Hearth's section for that game. The game is found from Steam's running app: a Hearth game played through its Steam
shortcut is that Hearth game, a Steam game is `steam:<app id>`.

- its **Hearth Input layout**: Off, one of Hearth's templates, your own or your layout for every game;
- its **Proton** for **Play through Hearth** (games Hearth starts with Proton: Epic, GOG, ones you added); Steam's games
  keep Steam's own Compatibility setting;
- its **mods in Hearth Mods** (how many, how many on; **Open Hearth Mods**). Shown when Hearth Mods' worker is running;
- Hearth's facts (ProtonDB rating, achievements) and a page with all of it (**Hearth's page for this game**; on that
  page the button goes on to the game's Hearth page, or Steam's page for a Steam game).

### Store, Mods, Storage and Recomps that work (Experimental, needs testing on a device)

These pages change things, not only show them. Every change goes through Hearth's core service (`call`, `mods-call`,
`games-disk` in `hosts/hearth-core/src/core_service.rs`, from `page_call` in the plugin's `main.py`), which keeps one
Hearth library for them (a `Bridge`, made the first time a page asks) and runs only the calls on its lists
(`BRIDGE_CALLS`, `MODS_CALLS`), with only their own arguments. When the core isn't running, the page says so and does
nothing; the pages are in `src/manage.tsx` and the Store's details in `src/pages.tsx`.

- **Store** (`/hearth/store`): a game opens its details. **Buy** / **Get it** opens the store's own page in Steam's
  browser (Steam's store for Steam games), signed in there: Hearth never sees payment details, passwords or tokens.
  A game you own: **Install** (Epic and GOG through Hearth's queue; Steam games on Steam's own page, with Steam's
  dialog), then its download's progress, **Cancel download**, **Play**. Hearth's downloads also show at the top.
- **Mods** (`/hearth/mods`): a game's mods in Hearth Mods: **Turn on / off**, **Uninstall** (press twice), and mods
  you downloaded: **Install** (into the game, turned on; one with installer choices is finished in Hearth Mods).
  Finding new mods (Nexus Mods, signed in there) stays in Hearth Mods' window.
- **Storage** (`/hearth/storage`, docs/STORAGE.md): drives, games by size, **Move to <drive>** with progress and
  **Stop**, **Uninstall**, **Clean up** (find, choose, delete with a second press) and **Use this disk for games**
  (Hearth's install folder for new Epic and GOG games on that drive).
- **Recomps** (`/hearth/recomps`): install a port with progress, **Cancel**, **Play** (through Hearth, like every
  Hearth game), **Uninstall** (saves backed up). A port that needs your copy of the game says to choose it in Hearth
  (there's no file picker inside Steam).

Steam's own games are never installed, moved or uninstalled by Hearth here: Hearth's way closes Steam for that, and
Steam runs this Game Mode. Their buttons open Steam's page or Steam's Settings › Storage instead (the core refuses
them too). A download Hearth started here stops if you leave Steam Game Mode before it's done; Hearth carries on with
its queue the next time it runs.

### Couch co-op in Steam Game Mode (Experimental, needs testing on a device)

Hearth's Couch Co-op (docs/archive/COUCH-COOP.md) inside Steam's Game Mode, with the same engine (`hearth-coop`):

- **Quick Access › Hearth Players** (also a **Players** section in Decky's Hearth tab): the controllers connected, in
  **Hearth's player order**. Players 1 to 4 get the first four; more wait. **Move up** / **Move down** change it,
  **Rumble** gives one a short rumble. The order is kept by the controller (its serial or Bluetooth address, else its
  make and name), not by the port it's in, in `~/.local/share/nimbus/coop/player-order.json`. Steam Input's own virtual
  pads aren't listed (they are copies of real controllers).
- **Split-screen games** (`/hearth/coop`): how many play (everyone connected, or 2, 3, 4), how the screen is shared
  (Auto, Side by side, Top and bottom, 2 × 2), then the games Couch Co-op can play (and why others can't, such as
  anti-cheat). **Play split-screen** or **Add to Steam library** makes **one Steam shortcut** for the game
  ("<game> (Split-screen)"), which runs `hearth-coop play --game <id>`: Hearth's split screen, a nested labwc that
  fills Steam's gamescope window, a nested gamescope per player in it, each player's copy seeing only their own
  controller, given by the player order at that moment. It stays until everyone has quit, so Steam shows it running;
  Steam's **Exit game** stops every copy cleanly. **Sync to Steam library** leaves these shortcuts alone; **Remove
  Hearth's shortcuts** removes them too.
- When it can't start (fewer than two controllers, a game that can't, something missing), the shortcut ends at once
  and the reason is under Quick Access › Hearth Players (**Last split-screen start**) and in
  `~/.local/share/nimbus/multiplay.log`.

The list, order and rumble go through Hearth's core service (`coop`, only `players`, `order`, `identify`, `games`,
`check`, with event nodes as controllers), or `hearth-coop` itself when the core isn't running. Nothing here can start
a split screen except the game's own Steam shortcut.

Not checked on a device yet (CI can't run Steam): that Steam's gamescope gives the split screen its window (gamescope
finds it from labwc's process under Steam's launcher; STEAM_GAME is also set on labwc's window when `xdotool` and
`xprop` are installed); Steam Input on the shortcut (set it off in the game's controller settings, or Steam may also
move a mouse in the split screen); Flathub's Steam (the shortcut hands the display and Steam's app id to
`flatpak-spawn --host`, but labwc then isn't under Steam's launcher, so focus relies on STEAM_GAME); rumble while Steam
holds the controllers; two identical controllers without serial numbers keep their plug-in order only.

### Multi-seat beside Steam Game Mode (Experimental, needs testing on a device)

With multi-seat (docs/archive/MULTI-SEAT.md) set up, **player 1 can play in Steam's Game Mode while the other players keep their own
Hearth Game Mode**, each on their own account, screen (or part of one, or a stream) and controllers. Nothing changes when
multi-seat isn't set up or is off: Steam Game Mode starts exactly as before.

- **The other players aren't touched.** Their sessions are their own system services (`hearth-seat@hearth-pN`); Steam Game
  Mode never starts, stops or waits for them, and switching between Hearth's and Steam's Game Mode doesn't end them (except
  as below for screens player 1's Game Mode hosts). Extra players themselves always get Hearth's Game Mode, never Steam's.
- **A graphics card per player:** Steam's gamescope stays on player 1's card (`hearth-seats gamescope-args`, as Hearth's
  Game Mode does). The other cards and their HDMI sound were already taken out of player 1's reach (udev rules and ACLs).
- **Another screen of player 1's card, or a split screen:** those screens need player 1's Game Mode to share them out, so
  Steam's gamescope runs inside the same host as Hearth's (`hearth-seats host`: labwc over every screen, Steam's gamescope
  as player 1's window, nested). Steam then can't use gamescope's direct screen control (VRR, HDR, its own refresh-rate
  switching) on player 1's screen. If the host doesn't start (or paused itself after failing twice), Steam Game Mode
  starts as usual without it and the hosted players wait for their screen. `HEARTH_STEAM_SEATS_HOST=0` in
  `~/.config/nimbus/session.conf` leaves the host out of Steam Game Mode.
- **Streamed players** run on their own virtual screen and aren't affected at all.
- **Controllers, graphics cards, sound:** Steam (player 1's) can only open what multi-seat leaves to player 1: the other
  players' controllers, hidraw nodes and cards are outside player 1's seat with ACLs for their own account, and Steam
  Input's virtual controllers go to the active session (player 1) only. On shared screens everyone's sound goes through
  player 1's PipeWire on their own screen's output, as in Hearth's Game Mode; Steam's sound settings can still move player
  1's own output. No performance overlay (mangoapp) while multi-seat is on, as in Hearth's Game Mode.
- **Per-seat safe:** Hearth's core service, its socket and Hearth Input are per account (`$XDG_RUNTIME_DIR`) and only start
  for player 1; the `steamos-session-select` and `steamos-update` stand-ins are only on player 1's Steam's PATH for that
  run. Updating Hearth OS from Steam only stages the update and never restarts. Restarting or turning off from Steam's
  Power menu is a normal restart, which ends the other players' sessions too (logind usually asks for an administrator's
  password first while other people are signed in, depending on the base system's polkit rules).
- **Quick Access › Hearth Players › Seats** (also in Decky's Hearth tab): each player, how they play (their own card,
  another screen, part of this screen, streamed), their account and whether they're playing; **Start Player N** / **Stop
  Player N** for each extra player (only that player: `hearth-seats start|stop --seat N` → `hearth-admin seats-start|-stop
  --seat N`), and **Assign controllers**: each connected controller with **P1** to **P4** to give it to another player
  (it reconnects to them; not to streamed players, whose controllers come from their device). A player on a shared screen
  started while there's no host gets their screen the next time Steam Game Mode starts, and the section says so. These go
  through Hearth's core service (`seats`: only `status`, `pads`, `start`/`stop` for players 2-4, `assign` with a pad id
  hearth-admin takes); without it the section can still show the players but changes go to Hearth's Settings. Setting
  multi-seat up, changing it and removing it stay in Hearth's Game Mode (Settings › Games › Multi-seat).

**A Linux user and their own Steam account per seat** is how multi-seat already works: every extra player is their own
Linux account (`hearth-p2` … `hearth-p4`, created by multi-seat's set-up, password locked, hidden from the login screen),
with their own home, Hearth profile, saves and their own Steam (signed in on that player's own screen, by them, in Steam's
own sign-in). The Seats section shows each player's account. Hearth never sees, stores or moves a Steam sign-in, password
or token: there's no "choose a Steam account for this seat" in Hearth, and there won't be. Player 1's account is the one
signed in at the device.

Not checked on a device yet (CI can't run Steam or several screens): Steam's gamepad screens inside the nested host
(Steam's own focus and overlay in a nested gamescope, the Quick Access and Steam buttons), Steam's sound and
brightness settings with the host, two Steams of different accounts running at once on one PC (Steam supports it; Decky
attaches only to player 1's, the only one with remote debugging on), and starting or stopping a player from the Seats
section while Steam holds the controllers (moving a controller reconnects it, which Steam should take in its stride).

### The rest of Hearth's Game Mode (Experimental, needs testing on a device)

Each goes through Hearth's core service (`aside`, `friends`, `link`, `remote` in `core_service.rs`, from `rest_call` in
the plugin's `main.py`; `src/rest.tsx`), and says so when the core isn't running.

- **Quick Resume** (Quick Access › Hearth, also in Decky's Hearth tab): **Put <game> aside** pauses the game Steam runs
  where it is (its processes stopped, kept in memory: no processor or graphics use) and goes to Steam's library. It
  carries on by itself when it's back in front (Steam's **Resume**, with Quick Access closed), or with **Resume** there;
  **Close** (press twice) closes it. Steam runs one game at a time, so only that game can be put aside: starting another
  asks Steam to close it, and the core lets it go. Only processes carrying the game's number are paused (Steam's reaper
  for its app id, Hearth's own HEARTH_GAME_ID, or Steam's SteamAppId), never Steam's own; anything still paused carries
  on when Steam Game Mode ends.
- **Game streaming** (Quick Access › Hearth): Sunshine on or off, the address and the Moonlight devices paired, and **Pair
  a device** with a number pad for Moonlight's 4-digit PIN (Hearth's Settings › Remote play does the same; the PIN is
  never logged). Moonlight to play another PC stays in Hearth.
- **Hearth Friends** (`/hearth/friends`, in Steam's main menu): your Hearth Friends (Matrix) friends, online first, and
  what they're playing. Read only: chats, requests, invites and signing in stay in Hearth's Friends (**Chats and requests
  in Hearth** opens Hearth's window). Steam's friends are Steam's own Friends & Chat.
- **Hearth Link** (`/hearth/link`, in Steam's main menu): this device's name, the paired devices (online or not, **Sync
  now**), copies in progress, games offered, and **Hearth Storage** devices (free space, **Wake it** when it's asleep and
  this device knows how). Pairing, settings and getting games from another device open Hearth's Settings in a window.
  The page never shows this device's long link.
- **On-screen keyboard**: Steam's own (Steam + X; Quick Access › Hearth › Hearth tools › Keyboard) everywhere in Steam.
  Hearth's keyboard only exists inside Hearth's own windows (Store, Settings, Hearth): opened from Steam Game Mode, it
  waits behind its prompts' **On-screen keyboard** button (the prompt says "Steam + X: Steam's keyboard"), so the two
  never come up together.

Put aside used to find nothing to pause for **Flathub Steam's games** (pressure-vessel has the Flatpak portal start the
game, outside Steam's reaper): Hearth now also knows a game's processes by Steam's SteamAppId (`procs.rs`), in Hearth's
Game Mode too.

| Hearth Game Mode feature | In Steam Game Mode |
| --- | --- |
| Home, Store, Library, Downloads, Storage, Mods, Recomps, game pages | Ported (above) |
| Couch co-op, multi-seat | Ported (player order, split-screen shortcuts, seats; set-up stays in Hearth) |
| Hearth menu (Guide button): Home, Library, Store, Friends, Downloads, Hearth Link, Couch Co-op, Theater | Ported: Steam's main menu › Hearth and Quick Access › Hearth |
| Hearth menu: Search, Discover, Apps (Bazaar), profiles | Not: Steam's own search; Discover, Apps and profiles need Hearth's window |
| Hearth menu: Settings, Power | Partially: Hearth's Settings opens in a window; Hearth power (back to Hearth, Desktop Mode) beside Steam's Power |
| Hearth menu over a game: screenshot, record, performance | Ported (Hearth tools, Hearth Performance) |
| Quick Resume | Partially: the game Steam runs (put aside, resume, close); several games at once can't (Steam runs one) |
| Pause (without leaving the game) | Not: Put aside does it with Steam's screen in front |
| On-screen keyboard | Steam's is used; Hearth's only in Hearth's windows, on request |
| Hearth Friends | Partially: read only (who's online, what they play); chats and requests in Hearth |
| Hearth Link, Hearth Storage | Partially: status, Sync now, Wake; pairing, settings, copies started from Hearth |
| Sunshine (game streaming) | Ported: on/off, status, pairing with the PIN; Sunshine's own login set-up in Hearth |
| Hearth Input | Ported (choosing a layout; the editor opens in Hearth's window) |
| Per-game power settings, frame limit, TDP | Not: Steam's own Performance tab (so the two never fight) |
| Parental controls, favourites, collections, Discord status | Not: Hearth's window |

### Still Hearth-only

Hearth's own Quick Access over a game (Pause, per-game power settings and the frame limit), Hearth's Settings and
Hearth Input's editor (they open in Hearth's windows), and the rest of "What only Hearth's Game Mode has" below.

### When Steam updates

Every change the takeover makes to Steam's screens is wrapped: if Steam's screens aren't shaped the way that part
expects any more, that part **turns itself off**, says why in Decky's log and under **Quick Access › Hearth › Turned
off by itself**, and Steam's own screen shows instead. Hearth's home also falls back to Steam's Home whenever Hearth
doesn't answer or anything in it fails. Parts that turned off get another chance the next time the plugin starts. The
switches turn any part off by hand, and **Hearth takeover** turns them all off at once.

## How to use it

1. Nothing to turn on: the device starts in Steam Game Mode. If Steam isn't signed in yet, Steam asks you to sign in on
   its own screen; Hearth never sees or keeps your Steam password.
2. Nothing to install: **Decky Loader is built into Hearth OS** (a pinned stable release, checked against its SHA-256
   when the image is built, in `/usr/lib/hearth-decky`; it's updated with Hearth OS). When Steam Game Mode starts, the
   session turns on Steam's remote debugging (Decky needs it), makes `~/homebrew` and starts
   `hearth-decky@<you>.service` (as root, as Decky needs) without a password prompt (`49-hearth-decky.rules`: only your
   own instance, only from the session at the device, for every account); the service puts Hearth Sync in place
   and removes an earlier downloaded install (`hearth-decky.service`, `/var/lib/hearth-decky`). It stops when Steam
   Game Mode ends. A Decky you installed with Decky's own installer (`plugin_loader.service`) is left alone, and the
   built-in one doesn't start next to it. Decky's own "update" button doesn't apply (it works in a temporary folder).
3. With Decky installed, press the **…** (Quick Access) button and open the **plug** tab, then **Hearth**. Press
   **Sync to Steam library** to see your Hearth games in Steam's library.
4. Desktop Mode: Steam's own **Power › Switch to Desktop**, or Quick Access › Hearth › **Desktop Mode**. Back from
   Desktop Mode: any **Game Mode** button.

## How it's kept safe

- **Always somewhere usable.** Whatever happens, the session ends up in Steam Game Mode or Desktop Mode (and, if
  neither can run, Hearth's archived Game Mode), never on a black screen of its own making. See "Where the device goes"
  above.
- **Falls back by itself, says why, never loops.** See "When Steam Game Mode can't start" above.
- **Never touches booting.** Nothing in the boot menu, the login screen, the kernel or ostree changes: this is all the
  session (`nimbus-session`). The previous OS version stays startable through ostree's boot menu entry. Decky Loader's
  service is not enabled at boot: the session starts it when Steam Game Mode starts and stops it when it ends, and if
  it fails, Steam Game Mode carries on without Decky.
- **Steam's Switch to Desktop** asks Hearth's session (Hearth's own `steamos-session-select` comes first on Steam's
  path), so it never changes the login screen's session.

## Ways out with only a controller

- **Steam's Power › Switch to Desktop**, or **Quick Access › Hearth › Desktop Mode** (needs Decky): Desktop Mode.
- **Hold B or Guide while the device starts**: Desktop Mode for that start. Hearth checks the controller's buttons for
  about 2.5 seconds as the session starts.
- **Hearth Recovery** (hold Volume Down, or Select + Start on a controller, while the device starts): **Desktop Mode
  this time** or **Safe graphics this time**.
- With a keyboard: add **`hearth.mode=desktop`** to the kernel command line in the boot menu (in GRUB: `e` on the entry,
  add it to the `linux` line, then Ctrl+X or F10): Desktop Mode for that start. Hearth doesn't add a boot menu entry
  for this (the boot menu isn't changed at all). (`hearth.mode=game` is simply Steam Game Mode.)
- **Settings › Sign-in › Start in › Desktop Mode**: start in Desktop Mode every time.

The old per-device **Start in Steam Game Mode** setting (`HEARTH_STEAM_DEFAULT`) is gone from Settings and Quick
Access: starting in Steam Game Mode is always on. The session ignores the old setting.

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

## Updating Hearth OS from Steam Game Mode

Steam's **Settings › System › Check for updates** updates Hearth OS: Hearth puts a `steamos-update` on Steam Game
Mode's Steam's way (only for that run, in the session's own folder; nothing system-wide), which runs Hearth's updater
(`/usr/libexec/nimbus/hearth-steamos-update`, Hearth's normal update through `hearth-admin`: bootc, or rpm-ostree on a
system with extra packages, then the system's Flatpak apps). It follows Steam's contract the way SteamOS, Bazzite and
ChimeraOS do: `check` answers 0 when an update is available (or downloaded and waiting for a restart) and 7 when Hearth
OS is up to date; the update prints `NN%` lines for Steam's progress bar and ends with 0 when it's ready for a restart.

**Quick Access › Hearth › Hearth OS updates** does the same with **Check for Hearth updates**, then **Update Hearth OS**
with a progress bar.

Neither ever restarts the device. Restart from Steam's power menu (or Hearth's) when you're ready: it's a normal
restart into the new version, and the version before stays installed (Settings › Updates › Go back, or the previous
entry in the boot menu). If Steam's own Restart button after an update doesn't do anything, use the power menu's
Restart.

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

## What only Hearth's app has (archived Game Mode)

In Steam Game Mode you have Steam's features instead of these Hearth ones: Hearth's Quick Access (Hearth's
performance and controller pages, the power limit, Hearth Recorder, Pause, Quick Resume for more than the game Steam
runs; the power mode is in Quick Access › Hearth), favourites, collections and
game settings, Hearth Friends' chats and requests, Couch Co-op's own setup (player names, choosing controllers by pressing A; Steam Game
Mode has the player order and split-screen shortcuts, above) and setting multi-seat up (Steam Game Mode starts, stops and
gives controllers to the players already set up, above), Hearth Link's device health and pairing, editing Hearth Input
layouts (choosing one is in Quick Access › Hearth Controller), parental controls and per-game power settings, Hearth's
on-screen keyboard, Discord status, and
Hearth's Settings (open Hearth in Desktop Mode for those). Xbox Cloud Gaming games aren't added to Steam (they need Hearth's stream
window).

## Screen modes

gamescope starts at the screen's own mode unless Settings › Display chose another (`-W`/`-H`/`-r` in
`~/.config/nimbus/session.conf`). Before gamescope starts, in Game Mode and in Steam Game Mode, the session asks
`/usr/libexec/nimbus/hearth-screen check` whether the connected screen lists that mode (the kernel's own list, with
refresh rates: the list gamescope picks from):

- **Listed**, or it can't be told (several screens, the built-in screen's refresh rates): gamescope gets it, as before.
- **Not listed** (a TV that has 2560 × 1440 only at 60 Hz, another screen plugged in): this start uses the screen's own
  mode. session.conf isn't changed, so the screen that has the mode gets it again; Game Mode says so and offers
  **Try again**, which really tries it (on trial, switched back after 20 seconds unless kept).
- gamescope ending within seconds with a chosen mode: Steam Game Mode starts once more at the screen's own mode (the
  mode is kept in `display-reverted.json` for Try again) instead of counting as Steam failing.

**ROG Ally built-in screen.** gamescope ships a display script for the ROG Ally / Ally X panels; a panel it doesn't know
(the ROG Xbox Ally's) gets `~/.config/gamescope/scripts/hearth-builtin-panel.lua`, made at every start from the panel's
own EDID and timings: refresh rates 48 Hz up to the panel's own (120 Hz), made like gamescope's Ally script (same pixel
clock, longer vertical front porch). It matches only that panel, and gamescope's own script wins when it knows it.
Send logs › "Screen" shows each screen's modes with refresh rates, its EDID, and gamescope's own messages.

## Limits (Experimental)

- **Valve's updates can break Decky** (Decky patches Steam's screens), and with it the Hearth plugin and look. Steam
  itself keeps working: turn **Hearth look** off, update Decky from its own settings, or remove it in Hearth's.
- The look uses Steam's own colour variables and focus class (stable), plus Steam's component names looked up at run
  time the way CSS Loader themes do. When Steam renames a component, that part simply stays Steam's style.
- Hearth's startup movie: the plugin puts one in Steam's custom boot video folder
  (`~/.steam/root/config/uioverrides/movies/deck_startup.webm`) only if one ships with it, and never replaces yours.
  Hearth OS itself now ships seven startup animations (`tools/make-startup-movies.py`: flame, sweep, reveal, embers,
  aurora, starlight, orbit), picked and previewed in Settings › Boot & startup (`hearth-boot-look` copies the chosen
  one there; the previews are frame sheets from `tools/make-boot-previews.py`). Hearth's sounds are made by Hearth's page itself (no
  sound files), so the plugin plays the same sounds on its own pages, only when **Hearth sounds** is on.
- The takeover's parts that change Steam's own screens (the main menu entries and the Library tabs most of all) are
  found by looking at Steam's screens while it runs; CI can't check them against a real Steam, so some may turn
  themselves off on your Steam version until they're adjusted.
- Handheld Daemon's Quick Access button may open Hearth's overlay rather than Steam's; use Steam's **…** button.
- Steam's own system settings that need SteamOS (OS updates, some power options) may be missing or do nothing.
- Needs Steam installed (both editions have it) and signed in for the library and store. Steam's first start
  downloads its own update: without internet it ends right away, and Desktop Mode opens with that reason (connect to
  Wi-Fi there, or in first-time setup, and try again).
- **Needs testing on a device.** CI builds and tests the plugin, the session switch, the Decky helper and Hearth's
  answers for the pages, draws the theme over a static stand-in for Steam's screens and the takeover's pages with
  made-up games, but no real Steam runs in CI.

## For developers

| Part | Where |
| --- | --- |
| Steam Game Mode and its fallback | `hosts/linux/session/nimbus-session` (`run_steam_mode`, `steam_mode_failed`, `steam_back`, `resolve_mode`, `setup_pending`, the main loop), `nimbus-mode steam` / `game` |
| Starting in it, the two-failures pause | `nimbus-session` (`steam_start_mode`, `steam_auto_before` / `_after`, `steam_pause`), `os/system_files/usr/libexec/nimbus/hearth-pad-held` (B / Guide held) |
| The "didn't start" notice | `hosts/linux/desktop/shell/SteamFallback.qml` (Mantel), `nimbus-session` `steam_notice_other` (Plasma, GNOME, COSMIC), `hosts/linux/desktop/labwc/autostart` (no back and forth) |
| Archived parts | Hearth's Game Mode (`run_game`, `HEARTH_LEGACY_GAME_MODE`), multi-seat and Couch Co-op (`HEARTH_LEGACY_MULTIPLAYER`, `multiplay.rs`, the plugin's `MULTIPLAYER`), the old Start in Steam Game Mode setting (`steam_mode.rs` `set_default`, the plugin's `get_steam_default` / `set_steam_default`, kept but not offered) |
| Settings, power menu, status | `hosts/hearth-core/src/steam_mode.rs`, `power.rs` ("steam"), `ui/index.html` |
| Playing a Hearth game from Steam | the shortcut's wrapper (`main.py` `WRAPPER`), then `hearth --play-now <id>` (`hosts/hearth-core/src/desktop_games.rs`, `steam_env_scrub`) |
| Game pictures | `main.py` (`art_bytes`, `picture_url`, `read_art`, `steam_art_file`), `src/model.ts` (`artCandidates`), `src/api.ts` (`useArt`), `src/ui.tsx` (`CoverArt`), `src/row.tsx` |
| OS updates from Steam | `os/system_files/usr/libexec/nimbus/hearth-steamos-update` (Steam's `steamos-update`, put on Steam's PATH by `run_steam_mode`), `main.py` `update_*` |
| Send logs' section | `hosts/hearth-core/src/logs.rs` (`decky_report`) |
| Decky install, start/stop (root) | `os/system_files/usr/libexec/nimbus/hearth-decky` through `hearth-admin decky-*` |
| The plugin | `archive/hosts/decky-hearth` (`src/index.tsx` panel and start, `src/shortcuts.ts`, `src/theme.ts`, `main.py`) |
| The takeover | `src/patches.tsx` (Steam's screens, each guarded), `src/pages.tsx` (Hearth's pages), `src/model.ts` (switches, guard, pure helpers, unit-tested), `src/api.ts`, `src/ui.tsx` |
| Hearth's answers for the pages | `hearth --steam-mode-data home\|library\|store <s>\|downloads\|storage\|info <id>\|power\|power-mode <m>` (`hosts/hearth-core/src/steam_takeover.rs`) |
| Hearth's core in Steam Game Mode | `nimbus --core-service --while <running mark>` (`hosts/hearth-core/src/core_service.rs`: socket, `COMMANDS`, `front` for Hearth Input), started by `nimbus-session` (`steam_core_start` / `steam_core_stop`, `steam_input_start`), the plugin's `core_call` (`main.py`) |
| Couch co-op in Steam Game Mode | `src/coop.tsx` (Players tab, `/hearth/coop`), `main.py` (`coop_call`, `coop_shortcut`, `COOP_WRAPPER`), `core_service.rs` (`coop`, `COOP_OPS`), `hearth-coop` (`players`, `order-set`, `identify`, `play`) |
| Multi-seat beside Steam Game Mode | `nimbus-session` (`steam_seats_on`, the host and `gamescope-args` in `run_steam_mode`), `src/coop.tsx` (`SeatsBody`), `src/model.ts` (`seatState`, `seatAction`, `seatTargets`, `seatsLine`, `seatDone`), `main.py` (`seats_call`, `SEATS_OPS`), `core_service.rs` (`seats`, `SEATS_OPS`, `seat_pad_ok`), `hearth-seats start\|stop --seat N`, `hearth-admin seats-start\|seats-stop --seat N` |
| Steam's main menu, Quick Access tabs, in-game section | `src/patches.tsx` (`HearthMenuItems`), `src/qam.tsx` (`syncQamTabs` through Decky's tab list, the tabs, `ThisGame`), `src/ingame.tsx` (`/hearth/now`, `/hearth/home`), `src/core.ts` (the running game, the 2-second `front`), `src/model.ts` (`menuPlaces`, `runningGame`, `frontMessage`, `qamPlan`) |
| Quick Resume, Sunshine, Hearth Friends, Hearth Link | `src/rest.tsx`, `main.py` (`rest_call`, `ASIDE_OPS`, `LINK_OPS`, `REMOTE_OPS`), `core_service.rs` (`aside`, `aside_follow`, `friends`, `link`, `remote`), `procs.rs` (SteamAppId) |
| The look | `archive/hosts/decky-hearth/defaults/hearth-theme.css` (also packed as a CSS Loader theme, stable part only) |
| CI | `.github/workflows/decky-hearth.yml` (zip, theme and takeover pictures, and pictures of Steam's main menu, Hearth's Quick Access tabs and the in-game section over static stand-ins, as downloads and on the `ci/decky-hearth-shots` branch); images ship it in `/usr/share/hearth/decky-hearth` |

To try a build of the plugin without a new image: download `decky-hearth` from a run of the Hearth Decky plugin
workflow, and in Decky's settings turn on Developer mode, then **Install plugin from ZIP**.
