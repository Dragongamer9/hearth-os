# Hearth Store (approved Nov 19, 2026; the plan and the owner's decisions: hearth-workspace hearth/mockups/hearth-store/README.md)

Mockups: hearth-store/store-1-home.png, hearth-store/store-2-app-page.png, hearth-store/store-3-updates.png, hearth-store/store-4-installing.png, hearth-store/store-5-repair.png, hearth-store/store-6-uninstall.png, hearth-store/store-7-controller.png, hearth-store/store-8-offline-empty.png, hearth-store/store-9-settings.png
Release: 1.16

The app is hosts/hearth-ui/apps/store (hearth-store-app; docs/HEARTH-STORE.md). Fedora edition only, built into the
image (os-bases/anatase/build.sh, its files in os-bases/anatase/system_files). Pictures: hearth-apps-rust.yml
(.github/scripts/hearth-apps-rust-shots.d/store.sh, store-1…13).

Status (Nov 19): built green in CI (hearth-apps-rust 44526860375, then 44535986019 with the picture fixes: store-1-home
at rest 0.00 % of a core, 26.5 MB). Every page pictured (store-1…13) and opened next to its mockup. Then merged with
feature/1.15-rust-apps (d7b558459) and pictured again (44666623022, 44672203235): store-2 and store-6 open the app's page
from Your apps, as their mockups, and say "‹ Your apps". Every part is built.

Wording changed from the mockups by the owner's decisions (Nov 19):
- "Not offering anything Debian": the "On older Debian PCs · System package (.deb)" line (store-2) and the "Older Debian
  PCs · System packages (.deb)" line (store-9) are left out; the Store is Fedora only.
- "Built into Hearth OS": the list of apps is the one in the image; Store settings' "List of apps" says "Built into
  Hearth OS 1.16" (or "hearthos.fun/store · signature checked" once a signed newer list is in use), and "Look for
  updates" says what's really asked for: "A few small files from GitHub and hearthos.fun, a few KB".
- store-4's "the signed list from hearthos.fun" is "the release's SHA256SUMS.txt" (what an app's download is checked
  against: Command Center's and Hearth AI's releases; "Hearth OS's list" when the list pins a version).
- The file names in What's wrong (store-5) are the real ones: the AppImage is kept under a name without a version
  (Hearth-AI-Command-Center.AppImage) so Command Center's own updater replaces it in place (a red flag there), and the
  menu entry is hearth-store-command-center.desktop.
- Texts that come from the apps' releases (what's new) and the Store's list (taglines, about) show what those say.

## Every page (the frame)
- [x] Title bar: the orange S, "Hearth Store", "· Hearth", minimize, maximize, close — hosts/hearth-ui/apps/store/src/view.rs: "ts(\"Hearth Store\", f(18.0), tn.text), gap(z(10.0), 1.0), t(\"· Hearth\""
- [x] Sidebar search "Search apps" with the Y key cap — hosts/hearth-ui/apps/store/src/view.rs: "text_input(\"Search apps\", &a.search)"
- [x] Sidebar pages with their icons: Home, Your apps, Updates (the orange count), Built into Hearth, Store settings; the open one lit with the orange mark — hosts/hearth-ui/apps/store/src/view.rs: "(Page::BuiltIn, \"▣\", \"Built into Hearth\"), (Page::Settings, \"⚙\", \"Store settings\")"
- [x] Sidebar foot: "This PC: Hearth OS 1.16 · Fedora edition", "Apps made for Hearth by Dragongamer9." (one line of text, not bold: the bold word wrapped apart at this size) — hosts/hearth-ui/apps/store/src/view.rs: "t(\"Apps made for Hearth by Dragongamer9.\", f(14.0), tn.muted)"
- [x] The controller's bar at the bottom with each page's buttons, and LB RB Previous / next page — hosts/hearth-ui/apps/store/src/view.rs: "hint2(&p, \"LB\", \"RB\", \"Previous / next page\")"
- [x] Mouse, keyboard and controller: arrows / D-pad move the orange focus ring, Enter / A, Esc / B, Ctrl+F / Y, X Update all, Ctrl+Page Up / Down and LB / RB pages — hosts/hearth-ui/apps/store/src/main.rs: "P::Rb => page_step(a, 1),"
- [x] Settings › Apps › App store: a "Hearth's own apps" row that opens the Hearth Store — hosts/hearth-ui/apps/settings/src/cats/apps/store.rs: "group(p, \"Hearth's own apps\""
- [x] In the Fedora image's menu (Plasma): "Hearth Store", with its icon — os-bases/anatase/system_files/usr/share/applications/hearth-store.desktop: "Name=Hearth Store"

## 1. Home (store-1)
- [x] "Hearth Store" and "Checked for updates 2 min ago" — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"Checked for updates {}\", ago(a, a.seen.checked))"
- [x] The featured app, outlined: "NEW VERSION · UPDATE READY", its icon, "Hearth AI 0.6", its line, its text, Update and See what's new — hosts/hearth-ui/apps/store/src/view.rs: "(\"NEW VERSION · UPDATE READY\""
- [x] The featured picture on the right (points of light joined in an oval) — hosts/hearth-ui/apps/store/src/look.rs: "pub struct Brain;"
- [x] "Your apps" with "See all ›" — hosts/hearth-ui/apps/store/src/view.rs: "look::link(\"See all ›\""
- [x] Command Center's card: "3.13.0 · updates itself", "Up to date", Open — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"{} · updates itself\", i.version)"
- [x] Hearth AI's card: "0.5.0 → 0.6.0 · about 120 MB", "Update ready", Update — hosts/hearth-ui/apps/store/src/view.rs: "chip(&p, \"Update ready\", Chip::Up)"
- [x] "Every app": eight tiles (four a row) with icon, name, line and Installed / Update / Built in / Coming — hosts/hearth-ui/apps/store/src/view.rs: "c = c.push(grid(tiles, 4, z(16.0)));"
- [x] The apps' icons: CC, ☾, G15, B, USB, ☺, ⬡, T in their colours — hosts/hearth-ui/apps/store/src/look.rs: "\"moon\" | \"smile\" | \"hex\""
- [x] Coming in a dashed chip — hosts/hearth-ui/apps/store/src/view.rs: "look::dashed(chip(&p, \"Coming\", Chip::Coming)"
- [x] Hearth OS CAD and Trains 2010 only on the owner's devices (his private list), never in the public list — hosts/hearth-ui/apps/store/src/catalog.rs: "pub fn private_apps(path: &Path)"
- [x] "Looking for other apps?" (dashed): "Flathub's apps are in Settings › Apps › App store", "Open App store ›" — hosts/hearth-ui/apps/store/src/view.rs: "\"Flathub's apps are in Settings › Apps › App store\""
- [x] Bar: A Select, B Back, Y Search, X Update all; LB RB Previous / next page — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"Y\", \"Search\"), hint(&p, \"X\", \"Update all\")"

## 2. An app's page (store-2)
- [x] "‹ Your apps" back (store-2 is pictured opened from Your apps, the same step as pressing its row there) — .github/scripts/hearth-apps-rust-shots.d/store.sh: "--page yours --app command-center --show menu"
- [x] The big icon, "Hearth AI Command Center", "Dragongamer9 · Your AI team: …" — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"{} · {}\", e.author, e.tagline)"
- [x] Chips: "Installed · 3.13.0" (green), "Updates itself", "Stable" — hosts/hearth-ui/apps/store/src/view.rs: "chip(&p, format!(\"Installed · {}\", i.version), Chip::Ok)"
- [x] Open (orange, big) and ⋯ — hosts/hearth-ui/apps/store/src/view.rs: "btn(&p, \"⋯\", B::Normal, true"
- [x] The ⋯ menu: Check and repair files (lit), Show its files, Release page on GitHub, a line, Uninstall… in red — hosts/hearth-ui/apps/store/src/view.rs: "(\"↗\", \"Release page on GitHub\""
- [x] Three pictures of the app — hosts/hearth-ui/apps/store/src/view.rs: "let w = if i == 0 { 125 } else { 100 };"
- [x] "What's new in 3.13" with the release's points, "About" and its text — hosts/hearth-ui/apps/store/src/view.rs: "ts(\"About\", f(21.0), tn.text)"
- [x] Facts: Version (3.13.0 · released Nov 15, 2026), Download (129 MB), Installs as (AppImage in your home folder; This PC: Fedora edition · no password, no restart), Updates (By itself, from the same release), Checked with (The release's SHA-256 list) — hosts/hearth-ui/apps/store/src/view.rs: "\"By itself, from the same release\""
- [x] Bar: A Select, B Back, ≡ More; LB RB Previous / next app — hosts/hearth-ui/apps/store/src/view.rs: "hint2(&p, \"LB\", \"RB\", \"Previous / next app\")"

## 3. Updates (store-3)
- [x] "Updates" with "Update all (1)" — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"Update all ({n})\")"
- [x] "Checked 2 minutes ago. The Store looks again when you open it, and at most once a day." — hosts/hearth-ui/apps/store/src/view.rs: "\"The Store looks again when you open it, and at most once a day.\""
- [x] Hearth AI's update, outlined: "0.5.0 → 0.6.0 · about 120 MB · AppImage in your home folder", Hide what's new ▴, Update — hosts/hearth-ui/apps/store/src/view.rs: "\"Hide what's new ▴\""
- [x] "What's new in 0.6" with its points, and "Her memories and settings stay as they are. Hearth AI closes for the update and opens again after." — os-bases/anatase/system_files/usr/share/hearth/store/catalog.json: "Her memories and settings stay as they are. Hearth AI closes for the update and opens again after."
- [x] Command Center: "3.13.0 · this app updates itself. When it has a new version, the button here is "Open to update"." and Up to date — hosts/hearth-ui/apps/store/src/view.rs: "this app updates itself. When it has a new version, the button here is"
- [x] "Built into Hearth": built in 1.16; in 1.17 the approved quick-updates mockup puts "Hearth's own apps and desktops" there instead (checklist quick-updates.md) — hosts/hearth-ui/apps/store/src/view.rs: "Hearth's own apps and desktops"
- [x] "Test versions": "Test versions are off on this PC", its line, "Store settings ›" — hosts/hearth-ui/apps/store/src/view.rs: "\"Test versions are off on this PC\""
- [x] Bar: A Update, B Back, X Update all, Y What's new — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"Y\", \"What's new\")"

## 4. Installing / updating (store-4)
- [x] "‹ Updates", the icon, "Updating Hearth AI", "0.5.0 → 0.6.0 · AppImage in your home folder · from the Hearth-AI release on GitHub", Cancel — hosts/hearth-ui/apps/store/src/view.rs: "AppImage in your home folder · from the {repo} release on GitHub"
- [x] "Downloading... 62%", "74 of 120 MB · about 10 seconds left", the bar, "Cancel works until the new version is put in place. That last step is a single swap, …" — hosts/hearth-ui/apps/store/src/view.rs: "Cancel works until the new version is put in place. That last step is a single swap"
- [x] The steps: 1 Download (its file, "next to the old one, which keeps working meanwhile", its bar, 62%), 2 Check the download, 3 Close Hearth AI (She saves her memories first), 4 Put it in place (0.5.0 kept until 0.6.0 has started fine), 5 Update the menu entry and open her again; Waiting — hosts/hearth-ui/apps/store/src/view.rs: "\"next to the old one, which keeps working meanwhile\""
- [x] "Everything happens in your home folder: no password, no restart, and nothing that starts your PC is changed." — hosts/hearth-ui/apps/store/src/view.rs: "Everything happens in your home folder: no password, no restart, and nothing that starts your PC is changed."
- [x] Really done that way: downloaded beside, SHA-256 checked (a mismatch deleted), one rename, the old kept until the new started — hosts/hearth-ui/apps/store/src/files.rs: "fn an_update_swaps_in_one_rename_and_keeps_the_old_one"
- [x] Bar: B Back (it carries on); ≡ Show details — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"B\", \"Back (it carries on)\")"

## 5. Check and repair (store-5)
- [x] "‹ Hearth AI Command Center", the icon, "Check and repair", "Hearth AI Command Center 3.13.0 · AppImage in your home folder" — hosts/hearth-ui/apps/store/src/view.rs: "tb(\"Check and repair\", f(38.0), tn.text)"
- [x] Steps with green ticks: Check the app (Doesn't match), Check the menu entry and icon (The menu entry is missing · 1 to fix), Check it can start (AppImage support (FUSE) is there · Fine), 4 Fix what's wrong (Download 3.13.0 again (129 MB), check its SHA-256, swap it in and write the menu entry again) — hosts/hearth-ui/apps/store/src/view.rs: "check its SHA-256, swap it in and write the menu entry again"
- [x] Repair now (outlined by the focus) and Not now — hosts/hearth-ui/apps/store/src/view.rs: "btn(&p, \"Not now\", B::Normal, true, lit(a, \"not-now\")"
- [x] "What's wrong (3)": the AppImage Changed (red), the same AppImage Not allowed to run, the .desktop Missing (yellow), in a mono font, and the note under them — hosts/hearth-ui/apps/store/src/view.rs: "chip(&p, \"Not allowed to run\", Chip::Warn)"
- [x] "The same version, put back": "A repair never updates or changes versions. The app's own updater keeps working afterwards." — hosts/hearth-ui/apps/store/src/view.rs: "\"A repair never updates or changes versions. The app's own updater keeps working afterwards.\""
- [x] Really checked and fixed: SHA-256, may run, FUSE, menu entry and icon; only what's wrong, the same version — hosts/hearth-ui/apps/store/src/files.rs: "fn repair_finds_what_is_wrong_and_fixes_it"
- [x] Bar: A Repair now, B Back; Y Show every file — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"Y\", \"Show every file\")"

## 6. Uninstall (store-6)
- [x] The app's page dimmed behind (the sidebar as it is) — hosts/hearth-ui/apps/store/src/view.rs: "Some(_) => ui::dialog(area, uninstall_dialog(a), Some(Msg::DlgCancel)),"
- [x] The icon and "Uninstall Hearth AI?" — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"Uninstall {}?\", e.name)"
- [x] "Keep my settings and data" (outlined by the focus, on by default) with "Her memories, settings and chats stay in your home folder (2.3 GB) and are there again if you install her later" — os-bases/anatase/system_files/usr/share/hearth/store/catalog.json: "Her memories, settings and chats stay in your home folder ({size}) and are there again if you install her later"
- [x] "What goes": The app (Hearth AI 0.6.0 · AppImage in your home folder · 120 MB), Its menu entry and icon (Yes), Settings and data (Kept (turn the switch off to move them to the Trash)) — hosts/hearth-ui/apps/store/src/view.rs: "\"Kept (turn the switch off to move them to the Trash)\""
- [x] Cancel and Uninstall (red) — hosts/hearth-ui/apps/store/src/view.rs: "btn(&p, \"Uninstall\", B::Danger"
- [x] Really: kept by default, else to the Trash (never deleted) — hosts/hearth-ui/apps/store/src/files.rs: "fn uninstall_keeps_data_or_moves_it_to_the_trash"
- [x] Bar: A Toggle, B Cancel; X Uninstall — hosts/hearth-ui/apps/store/src/view.rs: "(vec![hint(&p, \"A\", \"Toggle\"), hint(&p, \"B\", \"Cancel\")], vec![hint(&p, \"X\", \"Uninstall\")])"

## 7. With a controller (store-7)
- [x] The controller look when a controller is used (read as Settings reads it) — hosts/hearth-ui/apps/store/src/main.rs: "Subscription::run(pad::presses).map(Msg::Pad)"
- [x] The top: the S, "Hearth Store", LB, Home (lit) · Your apps · Updates (1) · Built in · Settings, RB, the clock — hosts/hearth-ui/apps/store/src/view.rs: "(Page::BuiltIn, \"Built in\"), (Page::Settings, \"Settings\")"
- [x] The featured app's big tile, outlined: its icon, "Hearth AI", "0.5.0 → 0.6.0 · Colibri is her brain now", A Update, Y What's new — os-bases/anatase/system_files/usr/share/hearth/store/catalog.json: "\"heroShort\": \"Colibri is her brain now\""
- [x] Three tall tiles: AI Command Center (3.13.0 · Installed), Logitech G15 (Built in), Beacon (Built in), tinted by their colours — hosts/hearth-ui/apps/store/src/view.rs: "format!(\"{} · Installed\", i.version)"
- [x] "What's new in Hearth AI 0.6" (two points) beside Installs as / Download / Her memories and settings: Stay as they are — hosts/hearth-ui/apps/store/src/view.rs: "\"Stay as they are\""
- [x] "More apps": Hearth Buddy, Hearth OS CAD, Trains 2010 (Coming), Hearth USB maker (Built in) — hosts/hearth-ui/apps/store/src/view.rs: "ts(\"More apps\", f(24.0), tn.text)"
- [x] Bar: A Update, Y What's new, ≡ More (repair, uninstall), X Update all, B Back; LB RB Pages, ⧉ Search — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"≡\", \"More (repair, uninstall)\")"
- [x] On KDE Plasma the controller's buttons are the Store's alone while it's in front — hosts/linux/desktop/shell/hearth-pad: "'hearth-store': ('a', 'b', 'x', 'y', 'lb', 'rb', 'up', 'down', 'left', 'right', 'menu'),"

## 8. Offline, and nothing installed (store-8)
- [x] "You're offline" banner with its sign: "This is the list the Store saw today at 9:12 AM. Installing and updates wait until you're back online.", Try again (outlined) — hosts/hearth-ui/apps/store/src/view.rs: "Installing and updates wait until you're back online."
- [x] "Nothing installed from the Store yet" (dashed), with its line — hosts/hearth-ui/apps/store/src/view.rs: "\"Nothing installed from the Store yet\""
- [x] "Every app · from today at 9:12 AM": AppImage · 129 MB / about 120 MB with Install greyed, Comes with Hearth OS / Built in, Not out yet / Coming — hosts/hearth-ui/apps/store/src/view.rs: "State::Coming => \"Not out yet\".into(),"
- [x] "Pictures you haven't seen yet aren't shown while offline. Apps you've installed keep working offline; only installs and updates wait." — hosts/hearth-ui/apps/store/src/view.rs: "Pictures you haven't seen yet aren't shown while offline."
- [x] Bar: A Try again, B Back — hosts/hearth-ui/apps/store/src/view.rs: "hint(&p, \"A\", \"Try again\")"

## 9. Store settings (store-9)
- [x] "Store settings", "Updates" — hosts/hearth-ui/apps/store/src/view.rs: "h1(&p, \"Store settings\", None)"
- [x] Look for updates: "When the Store opens and once a day" (orange) / "Only when the Store opens" — hosts/hearth-ui/apps/store/src/view.rs: "(\"When the Store opens and once a day\".into(), a.prefs.daily, Msg::SetDaily(true))"
- [x] Tell me when an update is ready (the bell; on by default): One notice, like "Hearth AI 0.6.0 is ready" — hosts/hearth-ui/apps/store/src/prefs.rs: "Prefs { daily: true, notify: true, metered: false, test: false, unreadable: false }"
- [x] Download on metered connections (off): Phone hotspots and other connections marked metered — hosts/hearth-ui/apps/store/src/view.rs: "\"Phone hotspots and other connections marked metered\""
- [x] Get test versions (off; outlined by the focus): "New versions reach this PC before everyone else, marked TEST. For trying them on your own devices first." — hosts/hearth-ui/apps/store/src/view.rs: "For trying them on your own devices first."
- [x] "Where apps go on this PC": This PC, Apps install as (AppImages in your home folder (~/.local/share/hearth/store/apps), no password or restart), List of apps with its green dot and time — hosts/hearth-ui/apps/store/src/view.rs: "AppImages in your home folder ({}), no password or restart"
- [x] "Related": App store (Flathub) · Settings › Apps; Your Store apps' folder · Open in Files; Hearth OS updates · Settings › Updates & recovery — hosts/hearth-ui/apps/store/src/view.rs: "\"Your Store apps' folder\", \"Open in Files\""
- [x] The switches remembered and never changed by themselves (settings_file.rs; opening writes nothing) — hosts/hearth-ui/apps/store/src/prefs.rs: "settings_file::merge_json(&crate::paths::prefs(), &m)"
- [x] Bar: A Toggle, B Back — hosts/hearth-ui/apps/store/src/view.rs: "Page::Settings => (vec![hint(&p, \"A\", \"Toggle\"), hint(&p, \"B\", \"Back\")], vec![pages()]),"

## Behind the pages (the plan)
- [x] The list of apps built into the image; a refresh from hearthos.fun used only when signed by the key built in — hosts/hearth-ui/apps/store/src/catalog.rs: "fn only_the_owners_signature_counts"
- [x] The website signs the list with the repository secret HEARTH_STORE_SIGNING_KEY, and publishes nothing without it — .github/workflows/site.yml: "KEY: ${{ secrets.HEARTH_STORE_SIGNING_KEY }}"
- [x] Command Center: installed from its GitHub release (checked against SHA256SUMS.txt), then "Updates itself · Open to update", its files never replaced — hosts/hearth-ui/apps/store/src/view.rs: "State::Ready(..) if e.self_updates => Some(Msg::Launch(e.id.clone())),"
- [x] Hearth AI: read from its latest GitHub release (Dragongamer9/Hearth-AI, release workflow on its ci/releases branch) — os-bases/anatase/system_files/usr/share/hearth/store/catalog.json: "\"github\": \"Dragongamer9/Hearth-AI\""
- [x] Looks for updates only when it opens and at most once a day (a user timer), not while a game runs or on a metered connection — hosts/hearth-ui/apps/store/src/check.rs: "println!(\"hearth-store: a game is running; looking later\");"
- [x] In the Fedora image: built, installed, its timer on only when it's there — os-bases/anatase/build.sh: "ln -sf ../hearth-store-check.timer /usr/lib/systemd/user/timers.target.wants/hearth-store-check.timer"
