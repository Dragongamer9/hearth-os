# Hearth Settings as its own pure-Rust app (layout A of the approved new Settings, for the Fedora edition)

Mockups: next-update/settings-A1-home-mouse.png, next-update/settings-A2-display-mouse.png
Release: next

The owner (Nov 18, 2026): Settings is its own app, not tied to the Game Mode app, in pure Rust like the new Task
Manager (hosts/hearth-ui/apps/settings). It follows the Global Theme picked in KDE, as every Hearth app must.

## The frame
- [x] left list: you (picture, name, Administrator · PC name) — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Administrator\" } else { \"Standard account\" }"
- [x] Search settings, finding pages by name and words — hosts/hearth-ui/apps/settings/src/frame.rs: "field(&p, \"Search settings\", &a.query, Msg::Query)"
- [x] the 13 categories with the picked one marked — hosts/hearth-ui/apps/settings/src/registry.rs: "assert_eq!(CATEGORIES.len(), 13);"
- [x] the page's path as its title (System › Display) — hosts/hearth-ui/apps/settings/src/frame.rs: "fn crumbs(a: &App)"
- [x] button hints: A Select, B Back, Y Search, LB/RB categories, Close — hosts/hearth-ui/apps/settings/src/frame.rs: "\"Previous / next category\""
- [x] follows the Global Theme picked in KDE's System Settings, Hearth's looks or any other — hosts/hearth-ui/src/theme.rs: "fn from_plasma() -> Option<Pal>"
- [ ] layout B (the controller look) and switching to it when a controller is used
- [ ] the controller moving between rows (A, B, LB/RB work as keys only: Esc, Page Up / Page Down)

## Home (settings-A1)
- [x] the PC card: picture, name, Hearth OS version · mode · graphics card · processor, Rename and About this PC — hosts/hearth-ui/apps/settings/src/cats/home.rs: "pill(&p, \"About this PC\""
- [x] four quick tiles: Wi-Fi, Bluetooth, Updates, Hearth Link with their state — hosts/hearth-ui/apps/settings/src/cats/home.rs: "tile(&p, G::Link, \"Hearth Link\""
- [x] Recommended settings: Night light (switch), Start in (Game Mode / Desktop Mode) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"What this PC opens when it turns on\", seg(&p"
- [ ] Recommended settings: Light mode while you play, Do not disturb
- [x] Storage: each drive with its bar and used of total, Manage — hosts/hearth-ui/apps/settings/src/cats/home.rs: "Some((\"Manage\", Msg::Go(\"storage\")))"
- [x] Changed lately: the last changes (what → to, who, when), each with Undo — hosts/hearth-ui/apps/settings/src/cats/home.rs: "btn(&p, \"Undo\""
- [ ] All changes (the full list page)
- [ ] Hearth Link's tile shows its devices

## A page (settings-A2, System › Display)
- [x] the displays drawn as you have them (1, 2), with the main one's name, resolution, refresh and graphics card — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"{} · main display\""
- [ ] Identify, Detect, Arrange, HDR
- [x] sections with headings (Brightness & colour, Scale & layout), each setting a row with its icon, name, line under it and its control — hosts/hearth-ui/apps/settings/src/cats/system.rs: "head(&p, \"Scale & layout\")"
- [x] Night light, Brightness, Scale, Resolution, Refresh rate — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Refresh rate\", \"How many pictures a second the display shows\""
- [x] X Keep these settings on a display change, and the old one back by itself if not kept — hosts/hearth-ui/apps/settings/src/cats/system.rs: "fn display_change("
- [ ] Related: Game Mode's resolution and frame limit

## Pages built (beyond the mockups' two)
- [x] Sound: volume, mute, speakers and microphone to use — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Recording from here\""
- [x] Power & battery: battery, power mode — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Power saver for a longer battery, Performance for games\""
- [x] Storage, Startup, Date & time (time zone, set by itself), About (with Rename) — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Find a time zone\""
- [x] Your desktop: KDE Plasma, Firehold (Rust, test), Firehold (Plasma, test), Switch now — hosts/hearth-ui/apps/settings/src/cats/system.rs: "\"Firehold (Plasma, test)\""
- [x] Wi-Fi: on/off, networks, connect with a password, disconnect, forget — hosts/hearth-ui/apps/settings/src/cats/net.rs: "\"The network's password:\""
- [x] Bluetooth: on/off, your devices (connect, forget), look for and pair new ones — hosts/hearth-ui/apps/settings/src/cats/bt.rs: "\"Look for devices\""
- [x] Personalization: the 8 looks, dark / light, the accent (KDE's Global Theme follows) — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Msg::Look(l) => return save(sh, json!({\"look\": l, \"subTheme\": \"\"}))"
- [x] Updates & recovery: check, update (a restore point first), restart, go back a version, System Restore, Reliability history — hosts/hearth-ui/apps/settings/src/cats/updates.rs: "\"Go back a version\""
- [ ] every other page of Mantel's Settings (listed on its category as "Coming next")

## Personalization, Accounts, Privacy & security, Accessibility (branch feature/settings-pers-acc)
Every page of these four categories, ported from Mantel's Set*.qml with the same helpers, for KDE Plasma on the Fedora
edition. Pictures: settings-pa-* (CI, .github/scripts/hearth-apps-rust-shots.d/settings-pers-acc.sh).

- [x] Personalization: KDE's Global Theme shown (Hearth's look or one picked in KDE), Use Hearth's look, Global Themes in KDE — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "KDE's Global Theme: {shown}"
- [x] Personalization: sub themes (palettes.json "subThemes" and the black set), Match sub theme — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "pub fn subs(look: &str, mode: &str) -> Vec<Sub>"
- [x] Personalization: Dark / Light / Auto (Sunset) with its two times — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "(\"sunsetLight\", \"Light from\", \"07:00\")"
- [x] Personalization: the wallpaper (the look's own, Hearth's, Pictures › Wallpapers, Choose a picture…), put on in Plasma — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "plasma-apply-wallpaperimage"
- [x] Personalization: accent presets, the look's own, From wallpaper (KDE's on Plasma), a hue, a typed colour — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Or type a colour, like #1a9fff, then Enter"
- [x] Personalization: keyboard lights follow the accent, desktop clock and weather (KDE's widgets), panel transparency, on-screen keyboard style — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Keyboard lights follow the accent"
- [x] Bar and dock on Plasma (KDE's panel): placement, auto-hide, bar style, width, alignment, thickness, network speed, Arrange the bar, add / remove a panel, the app launcher, right-click menu — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Network speed in the bar"
- [x] Bar and dock off Plasma (Mantel's keys): layout, edge, floating dock, auto-hide, bar style, app menu — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "Top bar + floating dock"
- [x] Fonts, cursor & icons: fonts and sizes, small and title fonts with their preview, smoothing, sub-pixel order, hinting, DPI, cursor, cursor size, icon theme (and KDE's own on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/fonts.rs: "Font smoothing and DPI"
- [x] Wallpaper per screen, slideshow, picture of the day (Bing, NASA, Wikimedia), light and dark, on Plasma's desktops — hosts/hearth-ui/apps/settings/src/cats/pers/desk.rs: "org.kde.potd"
- [x] My looks: save how it looks now, use, delete — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "Save as my look"
- [x] Window frames & sign-in splash: frame accent preview and slider, apps follow Hearth, the four splashes, Show it now (KDE's splash on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/looks.rs: "ksplashqml"
- [x] Window effects: every switch, choice and slider of SetKwinEffects.qml (hearth-kwin-settings, live on Plasma) — hosts/hearth-ui/apps/settings/src/cats/pers/kwin.rs: "Show paint"
- [x] Virtual desktops, Tiling (the layout drawn, presets, columns, rows, gap, quick tiling, snapping), Screen edges (the screen drawn, each corner and edge, touch edges), Window behaviour — hosts/hearth-ui/apps/settings/src/cats/pers/kwin.rs: "Where new windows open"
- [x] Accounts: the category page, your account, picture, password, other people (type, remove keeping or deleting files), add an account, finish setting up — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Remove, but keep their files"
- [ ] Accounts: cropping your picture in the circle (needs iced's image feature in hearth-ui: not in this branch)
- [x] Sign-in options: sign in automatically, Hearth's login screen (test), lock after sleep (KDE's lock on resume too), password, fingerprint (add, forget, where it's used), Moved — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Unlock the lock screen with a finger"
- [x] Online accounts: your accounts with what each is used for, Open in Files, remove; Nextcloud, Google, Microsoft (browser sign-in, Cancel), CalDAV/CardDAV, WebDAV, IMAP — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Sign in in the browser"
- [x] Login screen: the picture (Hearth's, your own, Choose a picture…, Hearth's fireside), Sign in automatically — hosts/hearth-ui/apps/settings/src/cats/accounts.rs: "Use Hearth's fireside"
- [x] Privacy & security: the category page, Player count, Firewall (on/off, incoming, Tailscale, rules, let a port in) — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "Let a port in"
- [x] Vaults, Recent files (with activities and left out), Firmware & device security, its history, Passwords — hosts/hearth-ui/apps/settings/src/cats/privacy.rs: "Hearth never shows or copies a saved password here."
- [x] Accessibility: magnifier, colour filters, invert one window, screen reader, sticky/bounce/slow/toggle/mouse keys, the bell, dwell click, text size, high contrast, large pointer, reduce motion, emoji — hosts/hearth-ui/apps/settings/src/cats/a11y.rs: "Click by resting the pointer"
- [ ] Pictures of wallpapers, avatars and the login screen's background drawn as the pictures themselves (these pages draw them as colour; needs iced's image feature)

What these four categories still need outside their own files (feature/settings-pers-acc):
* Pictures drawn as pictures (wallpapers, avatars, the login screen's background) and cropping your picture: iced's
  "image" feature in hearth-ui (a shared Cargo change, for the lead to decide).
* On KDE Plasma nothing yet changes light and dark at the Sunset times, and hearth-plasma-looks doesn't use
  frameTint, appsFollow or menuStyle: those three are saved (Mantel shares them) and their rows say what they reach.
  A light and a dark wallpaper are put on when the mode is changed here.
* Not carried over on purpose: Vaults' 10-second re-read (no timers while a page is open; it's read after each
  change), Slideshow's "Next picture now" (Plasma's scripting has no call for it: the page points to KDE's own).
