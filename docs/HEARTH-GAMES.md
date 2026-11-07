# Hearth Games

Hearth Games (`hearth-gamestore`, in the app menu as **Hearth Games**) is Hearth OS's own app for your **GOG**, **Epic
Games** and **Battle.net** games. You sign in, see your library, install a game on the drive you pick, watch the
download, play it and uninstall it. It's controller-first, like Hearth's other apps, and it uses Hearth's look.

It isn't a fork of Heroic. It uses the same open-source engines Heroic does, run by Hearth's own store engine
(hearth-core): **gogdl** for GOG, **legendary** for Epic, and Blizzard's own **Battle.net** app set up through
**Faugus Launcher**. Everything it installs goes into Hearth's library. Hearth Sync puts those games in Steam's library,
so they show up in **Steam Game Mode** and start there the same way they start here.

Settings › Games & stores opens it. The Epic, GOG and Battle.net rows there open it on that store, and Downloads opens
it too.

## What you see

- **Library**: covers of every game on your GOG and Epic accounts, plus your Battle.net games. Chips filter by store or
  show only Installed. **Y** searches, with the on-screen keyboard when you're using a controller.
  - Choose a game to open its page: its picture, the store, where it's installed, its size, play time and last played.
  - The page's buttons are **Play**, **Install**, **Pause** / **Carry on** / **Cancel** while it downloads, and
    **Uninstall**.
- **Downloads**: what's downloading, with progress, speed, Pause, Carry on and Cancel, plus Pause all and Carry on
  with all. Below that are the games installed lately.
- **Stores**: a card for each store.
  - **GOG** and **Epic**: who's signed in, how many games are on the account, Look for new games, Open the store and
    Sign out.
  - **Battle.net**: Set up Battle.net, or Open Battle.net and the shop once it's set up.
  - **Where new games go**: pick the drive for new installs. It shows which Proton Windows games start with.

**Moving around:** **A** chooses, **B** goes back, **X** looks for new games, and **LB** / **RB** switch tabs.
Keyboard, mouse and touch work as usual.

## Signing in

- **GOG and Epic:** Sign in opens the store's own sign-in page in its own window. Hearth never sees your password: the
  page hands Hearth a one-time code, and gogdl or legendary keep the sign-in in your own folder (`~/.local/share/nimbus`),
  the same place Hearth's library has always kept it.
  - The window uses the store's browser profile, shared with the rest of Hearth, so you only sign in once.
  - **Sign out** keeps a private copy of the sign-in on this device. Hearth never puts back a sign-in you signed out of.
- **Battle.net:** you sign in inside Blizzard's own app. Hearth never sees it.
- **Steam:** stays with Steam. Hearth Games doesn't show or touch Steam games.

## Installing and playing

- **Install** asks which drive when there's more than one. The choice is saved as Hearth's install folder (the same
  setting as Settings › Storage), merged into `library.json` and nothing else touched.
- Downloads use Hearth's download queue, so they carry on after a restart.
- **Closing while games download** asks you to choose:
  - **Keep downloading**: the window hides and the app closes by itself once the downloads are done.
  - **Close now**: the downloads carry on the next time you open Hearth Games.
- **Play** runs `nimbus --play-now <id>`, the exact start a game's Steam Game Mode shortcut uses (it includes 1.6's
  GOG launch fix). It runs in its own session, so closing Hearth Games doesn't end the game.
- **Battle.net games:** install them in Battle.net, and they show up here and in Steam Game Mode. Play starts them
  through Battle.net, so its sign-in and the game's licence work as on Windows.

## Proton

A Windows game whose options say **Default** starts with the GE-Proton built into Hearth OS, if the image has one
(`/usr/share/steam/compatibilitytools.d/GE-Proton*`), so it starts offline. Otherwise umu-launcher picks UMU-Proton,
downloaded when needed, as before. A Proton you pick in a game's options always wins.

The code is `hosts/hearth-core/src/proton_pick.rs`. `auto_for` is where the Proton detector (planned for 1.7) will
choose per game.

## The engines in the image

The image carries gogdl and legendary in `/usr/lib/hearth/tools`, so the first install downloads nothing but the game.
They're the same pinned releases as `hosts/hearth-core/src/tools.rs`, checked against the same SHA-256 when the image
is built. `test_hearth_gamestore.py` keeps them equal. An image without them works as before: Hearth downloads them the
first time, with the same checksum check.

## For developers

- **The window and its page:** `hosts/hearth-gaming/src/apps/gamestore.rs` is the window, and `ui/games.html` plus
  `ui/games.js` are the page.
  - The page can make only the hearth-core calls in `hosts/hearth-gaming/src/gamestore.rs` `CALLS`, only for
    `gog:`, `epic:` and `battlenet:` games, and only with their own arguments.
  - Opened in a plain browser, the page shows sample games (`KIT.demo`).
- **One window at a time:** a second start shows the open window, through `$XDG_RUNTIME_DIR/hearth/gamestore.sock`.
- **Tests:**
  - `hosts/linux/test/test_hearth_gamestore.py`.
  - The `gamestore` and `proton_pick` unit tests.
  - `gaming.yml`: the build, plus screenshots of Hearth Games on an empty home.
- **Next stage:** cloud saves (gogdl's and legendary's save sync), DLC, and Amazon Prime Gaming through nile.
