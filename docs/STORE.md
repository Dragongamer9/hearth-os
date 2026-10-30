# Game Mode › Store

Game Mode has a **Store** page (in the menu, the top bar and the layouts' navigation, next to Library). It has a
section for each store: **Steam**, **GOG**, **Epic Games**, **Xbox** (Microsoft Store / PC Game Pass) and
**Battle.net**. LB and RB switch between them. Each section shows a featured game, shelves (deals, top sellers, new releases and so on), a
search, and a page for each game. The pages work with a controller, a keyboard and mouse, or touch.

## What each section shows, and where it comes from

Hearth reads each store's **public** pages. Browsing needs no sign-in, and Hearth sends no account details
to read them. The code is in `hosts/hearth-core/src/storefront/`.

| Store | Shelves | Search | Game page |
|---|---|---|---|
| Steam | Featured, Deals, Top sellers (`store.steampowered.com/api/featured`, `featuredcategories`), Popular new releases (Steam's search page) | `api/storesearch` | `api/appdetails` (description, screenshots, price, Metacritic score), plus ProtonDB's rating for **On Linux** (the same one the library shows) |
| GOG | Trending, Bestsellers, Deals, New on GOG (`catalog.gog.com`) | `catalog.gog.com` | `api.gog.com/products` (description, screenshots), with the price and rating from the catalog |
| Epic | Free now and Free next (Epic's free-games feed), On sale, New releases, Coming soon (`store.epicgames.com/graphql`) | the same GraphQL | the offer's page and its players' rating from the same GraphQL |
| Xbox | Most popular on PC, Deals, New releases (xbox.com's browse lists), Popular with PC Game Pass, New to PC Game Pass (`catalog.gamepass.com`) | the Microsoft Store's search (`storeedgefd.dsx.mp.microsoft.com`) | the Microsoft Store's product page |

Store names, prices and pictures are shown as the stores give them, labelled with the store's name, and nothing
else. Hearth keeps each answer for a while: store pages for 30 minutes, searches for 15 minutes and game pages for
12 hours (in `~/.local/share/nimbus/cache/storefront`). If a store can't be reached, the last answer is shown,
with a note that it's from earlier. Hearth sends one request at a time to each store, with a pause between them, so
the stores' rate limits aren't hit. Steam's game pages have a longer pause (1.5 s).

Sometimes Epic's store asks for a browser check (Cloudflare) instead of answering. It's more likely from data
centres than from home connections. Hearth doesn't try to get around the check. Epic's free games still show,
because they come from a feed without the check, and the page offers Epic's own site in the store window instead.
That window is a real browser, so the check passes there. Every section except Steam also has a button for the
store's own site (GOG.com, Epic Games Store, Xbox.com), and a failed search offers the same search on that site.

Games already in your library show **In your library** or **Installed**, and their page offers **Install** or
**Play**. Install is the library's own install: Steam games install quietly through Steam, and GOG and Epic games
install through Hearth. Xbox PC games show **Open Xodus Nexus**, or **Set up Xbox PC games** if Nexus isn't added
yet.

## Buying: one sign-in for the library and the store

Hearth never handles store passwords, cards or tokens for buying. Purchases always happen on the store's own page.

**Steam.** Buy, wishlist and gift happen in the **Steam client's own store**. Hearth opens `steam://store/<appid>` and
shows Steam's window over Game Mode, the same way it shows Steam's other windows. The Steam client is already signed in. Hearth
never signs in to Steam, never uses Steam's accounts or tokens, and never loads `steamclient.so`. If Steam isn't
signed in, the button says **Sign in to Steam** and opens Steam's own sign-in, the same one Library › Stores uses.
Steam purchases show in the library at its next refresh. A game's page also offers
**Bought it? Refresh my Steam games**.

**GOG, Epic, Xbox.** The store's page opens in Hearth's **store window**. It shares one browser profile with that
store's sign-in window in Library › Stores & accounts:

- Library › Stores' sign-in window used to start empty each time (a private window). It now keeps that store's own
  browser profile in `~/.local/share/nimbus/accounts/web/<store>-<id>/` (`webkit/` for Hearth's usual window,
  `chromium/` for the Chromium one on Nvidia). The store window opens with the same folder.
- So when you sign in to a store in Library › Stores (on the store's own sign-in page), the store window is signed
  in too, and the reverse is true as well. One sign-in covers both.
- If a store isn't signed in, Buy says **Sign in to GOG** (or Epic Games, or Xbox). It runs the library's usual
  sign-in, so one sign-in covers both. In Game Mode that sign-in still goes through Desktop Mode, as it already did.
- In Game Mode the store window opens full screen with the controller helper from the sign-in window. The stick
  moves a pointer, A clicks, Y opens a keyboard and B comes back to Hearth. On the desktop it's an ordinary window.
- When you close the store window, Hearth fetches that store's games again, so a game you just bought shows in the
  library.
- **Sign out** in Library › Stores also deletes that store's browser profile (cookies and all). The next sign-in
  starts fresh in a new folder.
- The folder is under `accounts/`, which moves with each person's profile when people switch profiles, the same
  as the rest of their sign-ins.

The only things Hearth keeps are the stores' own sign-ins (the library's, as before) and the store's own web
session (its cookies), and both stay in the person's own `accounts/` folder. Hearth never sees the
password, and nothing is sent anywhere else.

Things to know:

- WebKit (the usual window) and Chromium (the Nvidia window) keep separate profiles. If you change the drawing
  engine (Settings › Display), the store window asks you to sign in once more.
- Epic's checkout opens Epic's own purchase page (`store.epicgames.com/purchase?offers=…`). GOG and Xbox open the
  game's page on the store, where its own Buy button starts the checkout. Wishlist (★) opens the game's store page.
- Payment pop-ups (PayPal and the like) open as a small window over the store window in the Chromium window. In
  the WebKit window, links that would open a new window open in the same window instead.

## Battle.net

Blizzard sells and installs its games only in its own Battle.net app, and there's no open-source downloader for it
(like legendary for Epic or gogdl for GOG). So Hearth runs **the real Battle.net app** with Proton, the way it runs
Epic and GOG games (`hosts/hearth-core/src/battlenet.rs`):

- **One press:** Library › Stores › Battle.net › **Set up Battle.net** (or the Store's Battle.net section). Hearth gets
  umu-launcher if it isn't there yet, downloads Blizzard's installer from Blizzard
  (`downloader.battle.net/…gameProgram=BATTLENET_APP`), and installs Battle.net into a Proton prefix of its own,
  `~/.local/share/nimbus/battlenet/prefix`, in the system's language when Blizzard has it. When the installer has
  finished (Battle.net opens by itself at the end), Hearth closes it and turns off Battle.net's hardware acceleration,
  which draws a black window in Proton, unless you've chosen otherwise. The installer's output is in
  `battlenet/setup.log`.
- **Sign-in, buying, installing and updates all happen in Battle.net's own window.** Hearth never sees the Blizzard
  password or payment details and never uses the account. **Sign in** on the card opens Battle.net in Desktop Mode,
  where the controller types, like Xodus Nexus.
- **Library › Battle.net:** Battle.net itself, and each Blizzard game it installed. A game is found by the
  `.build.info` file Battle.net puts in every game folder (its product, or the folder's name), under Program Files,
  Program Files (x86) or C:\Games in the prefix. Overwatch 2, World of Warcraft, Diablo II: Resurrected, III, IV and
  Immortal, Hearthstone, Heroes of the Storm, StarCraft, StarCraft II and Warcraft III are known. **Find games** reads
  them again.
- **Play** starts the game through Battle.net (`Battle.net.exe --exec="launch <code>"`), so its sign-in and the game's
  licence work as on Windows. The game's number (`HEARTH_GAME_ID`) is in Battle.net's environment, and the game's
  folder finds its processes.
- **Controller:** Battle.net's own window starts with the **Mouse only** layout (it's made for a mouse); the games
  start Off, since they have controller support of their own.
- **Remove** deletes the prefix, with every Blizzard game installed in it.
- The Store's Battle.net section has no shelves: Hearth reads no Blizzard pages. It opens Battle.net, or sets it up.

## Xodus Nexus (Xbox PC games)

Xbox PC games show **Open Xodus Nexus**. Nexus is integrated the way Beacon is (`hosts/hearth-core/src/xodus.rs`):

- **Preinstalled:** each Hearth OS edition's build puts the latest release's AppImage in `/usr/lib/xodus-nexus` (with
  its version), `/usr/bin/xodus-nexus` to start it and `xodus-nexus.desktop`. If there's no release to download, the
  build warns and carries on. Settings › Games › Library still offers a newer release, downloaded into your own data
  folder (that copy wins; Remove goes back to the image's). Elsewhere, **Add Xodus Nexus** downloads it.
- **Library › Xbox PC:** Nexus itself, and a tile for each game you made a shortcut for in Nexus (a game's menu ›
  Create shortcuts). Nexus doesn't document its shortcuts, so Hearth takes `.desktop` files in your applications folder
  or on the desktop whose Exec names Xodus, and Play starts the shortcut as Nexus wrote it.
- **Library › Stores:** Nexus's card, with Open Xodus Nexus, Sign in (in Desktop Mode) and Show games.
- **Controller:** Nexus's window starts with **Mouse only**; Xbox games start Off.
- **Desktop Mode:** in Mantel's launcher, and pinned to the taskbar (for a new user, and once on an existing taskbar
  the first time it's installed, like Bazaar).

## Checks

- Rust tests for each store's reader use recorded answers from the real stores
  (`hosts/hearth-core/src/storefront/fixtures/`). These are trimmed copies of what `tools/store-live-check.py`
  saved.
- `tools/ui-check.js` checks the Store page with the preview's data: the store tabs, LB/RB, the shelves, a game's
  page offering Play, Sign in, Buy in Steam or Buy on Xbox as it should, search, and B going back. It also checks the
  Battle.net section and the Xodus Nexus and Battle.net cards in Library › Stores. It also takes
  two screenshots, which release.yml posts as notices (`SHOT store-home`, `SHOT store-game`).
- The CI job `store-live` (rust.yml, warn-only) asks every endpoint above once. It also runs Hearth's own store
  code against the real stores (`cargo test --lib storefront::live -- --ignored`) and keeps the answers on the
  `ci/store-live` branch, so the test samples can be refreshed when a store changes its format.
