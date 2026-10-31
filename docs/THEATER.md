# Hearth Theater

Hearth Theater (`hosts/hearth-theater`, Rust + Tauri) is your own wall of video sites. You add a site by pasting its
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
- **The viewer** (unchanged): left stick moves the pointer, A clicks, B goes back, right stick scrolls, X play/pause,
  Y fills the screen with the video, View returns to Theater.

## How a site's catalog is read

Theater reads a site the way a browser shows it, inside its own web view (WebKitGTK), with the same cookies and
sign-ins as the viewer: a hidden window loads the site's page with `src/reader.js`, and nothing else fetches the page.
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
- **DRM**: what plays is up to the site and WebKitGTK; DRM-protected streams may not play in the viewer, whatever
  Theater lists.
- Sites built entirely with scripts that load their lists late, or only after scrolling a lot, may list only part of
  what you see (Theater waits for the page and scrolls it a little; its browse pages are read without running their
  scripts). Bot checks ("checking your browser") and sites that answer only to their own apps list nothing; such a
  site still opens and works in the viewer as before.
- Search uses the site's own search page; sites whose search only works with scripts may show no results there (what
  they list is still searched).

## Tests

- `cargo test --lib`: catalog tidying, rows, search addresses, kept titles and files (`src/lib.rs`).
- `tests/pages.mjs` (Chromium, in `apps-create.yml`): the reader on four local test sites from `tests/fixtures`
  (served by `tests/serve.mjs`): JSON-LD, Open Graph cards, RSS with a sitemap, and plain repeated cards; checks the
  rows each gives, then shows them on Theater's home screen, moves around, makes a favorite, hides a row, searches,
  and picks a card (it must open that title's page). Its screenshots are posted as notices.
- The same workflow runs the real app with `--read-catalogs` against those test sites under Xvfb and checks the saved
  catalogs and posters, then screenshots its home screen.
- Never against real streaming sites, and never signed in to anything.
