# Hearth Theater

Hearth Theater (`hosts/hearth-theater`: a Rust core, its windows drawn by Chromium) is your own wall of video sites. You add a site by pasting its
link; Theater then lists that site's shows, movies and videos on its own home screen, as rows of cards you browse with
the controller. Choosing a card opens that title's page full screen in Theater's viewer, where it plays on the site
itself. Game Mode opens it from its menu (Theater), full screen (`hosts/hearth-core/src/theater.rs`).

## Using it

- **Add a site**: Menu (or the Add site button), paste or type the link, pick a shelf (Anime, TV, Movies, Other). The
  name, picture and icon come from the site's page. Theater then reads what the site lists (a row of grey cards and
  "Reading the site…" shows meanwhile).
- **Home (All)**: Continue (titles and sites you opened, newest first), Favorites, then each site's own rows, named
  after its sections ("Popular", "New episodes", genre headings...) or "From <site>" when it has none, and last Your
  sites (the site tiles). Each card shows the poster, the name, and the episode count or year when the site gives them.
- **Shelves**: LB/RB moves between All, Anime, TV, Movies, Other, Favorites and Search. A shelf shows the rows of the
  sites on it, then its site tiles.
- **Cards**: A watches (the title's page opens in the viewer), X makes it a favorite, Y has options (watch, favorite,
  open the site, the site's options).
- **A site's options** (Y on its tile, or from a card): Refresh now (read its titles again), Show or hide its rows,
  edit its name/link/shelf, get its picture again, Remove from Theater (its catalog, posters and kept titles go too).
- **Search** (View, or the Search tab): type with the Hearth on-screen keyboard (A on the field). What your sites list
  is searched at once ("In your sites"); when typing stops (or on Search), each site that has its own search page is
  asked too, and its results show as "Results from <site>".
- **The viewer**: left stick moves the pointer, A clicks (hold A to drag), B goes back a page (or leaves the page's
  own full screen), right stick or D-pad up/down scrolls, D-pad left/right skips the video 10 s back/ahead, X
  play/pause, Y puts the video full screen (again: back), LB/RB back/forward, RT the on-screen keyboard (it docks
  under the page and types into the field you clicked; B or Done closes it), Menu shows these hints, View returns to
  Theater.

## How it's built

- `/usr/bin/hearth-theater` (what Game Mode and the app menu start) starts the window app: Hearth's own Chromium,
  `/usr/lib/hearth-electron/hearth-electron --hearth-theater` (the Electron build that draws Game Mode on Nvidia;
  `hosts/electron/src/start.js` picks Theater's `hosts/hearth-theater/electron/main.js`, carried in its
  `resources/theater`). The window app starts `hearth-theater --core-stdio`, the core (`src/main.rs`): sites,
  catalogs, posters and the pointer, one JSON object a line over stdin/stdout (the lines are listed at the top of
  `src/main.rs`).
- **Why Chromium, not WebKitGTK** (what Theater used until now, through Tauri): on Nvidia RTX 50 cards WebKitGTK's
  drawing fails (WebKit bug 303811, the same reason Game Mode draws with Chromium there, docs/RENDERERS.md), so
  Theater's window stayed empty; bot checks (Cloudflare Turnstile, hCaptcha, reCAPTCHA) often loop on WebKitGTK; and
  its pages could only be given pretend clicks. Chromium is already in both images, so nothing new is downloaded.
  Game Mode only shows Theater when both programs are there (`hosts/hearth-core/src/theater.rs`).
- **The main window** is Theater's home screen (`ui/index.html`, with `electron/preload.js` giving it the same calls
  it had from Tauri). **The viewer** shows the site full screen as a view in the same window, with nothing of
  Theater's put into the site. The controller is read by Theater's own page underneath (`ui/viewer-pad.js`); the
  pointer it moves is a **real mouse** made by the core (`src/pointer.rs`): a uinput device, "Hearth Theater
  pointer" (as Game Mode's controller mouse), or XTest on an X11 display without /dev/uinput. So every click is the
  system's own and trusted, wherever it lands: frames from other sites, video players, popups and captchas.
  A hint bar and messages show in small views on top. Play/pause, skipping and full screen run a short script in the
  frame with the biggest video only when you press X, Y or ◀ ▶, and never in a bot check's or captcha's frame.
- **Bot checks**: Theater says it is the Chromium it is (a plain Chrome user agent of the version drawing, without
  Electron's name; no automation switches, so `navigator.webdriver` is false), keeps its own cookies and sign-ins
  between runs (`~/.local/share/hearth-theater/chromium`, never Hearth's own window's profile), opens sign-in and
  check popups as real windows with the same cookies (other new tabs, mostly ads, are refused; links to the same site
  open in the viewer), and never runs its scripts on challenge pages ("Just a moment…", Cloudflare's, DDoS-Guard's)
  or in captcha frames (Cloudflare, hCaptcha, reCAPTCHA, Arkose, DataDome...; `electron/rules.js`). Sites may use full
  screen and the pointer lock; nothing else (no camera, microphone, location or notifications).
- **Its log**: `~/.local/share/hearth-theater/theater.log` (kept under 2 MB), and the "Hearth Theater" section of Send
  logs: when it started and how, the renderer and graphics card Chromium got, pages loaded or failed (addresses
  without their query), bot checks seen, popups, the pointer (which kind, or why there is none) and the core.
  A Theater that can't start or crashes stays in its own log: it's a separate program and never affects Game Mode.

## How a site's catalog is read

Theater reads a site the way a browser shows it, in its own Chromium, with the same cookies and sign-ins as the
viewer: a hidden window loads the site's page and runs `src/reader.js` in an isolated world (the page can't see it),
and nothing else fetches the page. On a bot check it waits for the check to pass, without running anything on it;
when it doesn't pass by itself, the catalog says to open the site in the viewer once, then Refresh now.
There is no server and no separate scraper. The reader looks for titles in this order:

1. **schema.org JSON-LD** on the page (Movie, TVSeries, TVSeason, TVEpisode, VideoObject; ItemList names become rows);
2. **Open Graph / Twitter cards** of the pages the site links to (for links without a picture), and **oEmbed** when a
   page has no picture of its own;
3. the **RSS/Atom feeds** the site advertises (the feed's title is the row's name);
4. the site's **sitemap** (robots.txt's Sitemap lines or /sitemap.xml), when the rest found fewer than 12 titles;
5. **repeated cards**: a link with a picture and a name, grouped under the section heading they sit under.

It also reads up to five of the site's own browse pages from its menu (Popular, New, Movies, Genres...) and "See all"
links, with the same session. It finds the site's search from its **OpenSearch** description, or its search form.
What it hands back is checked in `src/lib.rs` (`sanitize`): web addresses only, names tidied, rows of the same name
joined, each title once per row, at most 24 rows, 60 titles a row and 400 a site.

- **Kept**: `~/.local/share/hearth-theater/catalog/<site>.json` per site; posters in `art/c-<site>/` (up to 160 a
  site, saved once and kept while the site still lists them; until then a card loads the poster from the site);
  titles you opened or made favorites in `titles.json`.
- **Refreshed**: in the background when Theater starts, for catalogs older than six hours (one site at a time); right
  after a site is added or its link changes; and with Refresh now.
- `hearth-theater --read-catalogs` reads every site's catalog without a window, saves it and quits (CI uses it).

## What it doesn't do, and sites that won't list well

- Theater only lists the links a site's pages show and opens those pages to watch, as a browser does. It never
  downloads or rips video, and has no site-specific scrapers: it doesn't get past sign-ins, paywalls, DRM or ads.
- **Sign-ins**: sign in on the site in the viewer, then choose Refresh now. When a site shows a sign-in form and lists
  (almost) nothing, Theater says so.
- **DRM**: what plays is up to the site and Chromium (Electron has no Widevine); DRM-protected streams may not play
  in the viewer, whatever Theater lists.
- Sites built entirely with scripts that load their lists late, or only after scrolling a lot, may list only part of
  what you see (Theater waits for the page and scrolls it a little; its browse pages are read without running their
  scripts). Sites that answer only to their own apps list nothing; such a site still opens and works in the viewer.
  Bot checks that need a click are passed in the viewer with the pointer, as in any browser.
- Search uses the site's own search page; sites whose search only works with scripts may show no results there (what
  they list is still searched).

## Tests

- `cargo test --lib`: catalog tidying, rows, search addresses, kept titles and files (`src/lib.rs`).
- `cargo test --lib` also covers the window app's lookup, the log and the pointer's commands.
- `node --test electron/rules.test.js`: the user agent, bot-check and captcha addresses, the popup rules, log
  addresses and permissions.
- `tests/pages.mjs` (Chromium, in `apps-create.yml`): the reader on four local test sites from `tests/fixtures`
  (served by `tests/serve.mjs`): JSON-LD, Open Graph cards, RSS with a sitemap, and plain repeated cards; checks the
  rows each gives, then shows them on Theater's home screen, moves around, makes a favorite, hides a row, searches,
  and picks a card (it must open that title's page). Then the viewer's controller (`ui/viewer-pad.js`) with a stand-in
  controller: the pointer, A (button down and up), scrolling, every button, the keyboard, and the home screen leaving
  the controller alone meanwhile. Its screenshots are posted as notices.
- The same workflow runs the real app (its core and its windows in Electron 38, as Hearth's build runs them) under
  Xvfb: `--read-catalogs` against those test sites (the saved catalogs and posters are checked), its home screen (a
  screenshot), and **the real pointer**: `tests/pointer.mjs` serves a page with a frame from another site holding a
  button that only takes real clicks (`isTrusted`); Theater opens it in the viewer (`--open`), its pointer (XTest
  there) moves onto the button and A is pressed the way the controller does; the page's title must say a real click
  arrived.
- Never against real streaming sites, and never signed in to anything.
