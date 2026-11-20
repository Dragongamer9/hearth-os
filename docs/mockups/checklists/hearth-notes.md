# Hearth Notes (approved Nov 20, 2026, 03:20; the plan and the owner's decisions: hearth-workspace hearth/mockups/hearth-notes/README.md)

Mockups: hearth-notes/notes-1-main.png, hearth-notes/notes-2-today.png, hearth-notes/notes-2b-board.png, hearth-notes/notes-3-graph-classic.png, hearth-notes/notes-3b-graph-modes.png, hearth-notes/notes-3c-galaxy.png, hearth-notes/notes-3d-solar-system.png, hearth-notes/notes-3e-black-hole.png, hearth-notes/notes-3f-customise-graph.png, hearth-notes/notes-3g-ember.png, hearth-notes/notes-3h-railway-map.png, hearth-notes/notes-4-search.png, hearth-notes/notes-5-sync.png, hearth-notes/notes-6-conflict.png, hearth-notes/notes-7-new-item.png, hearth-notes/notes-8-settings.png, hearth-notes/notes-9-controller.png
Release: next

The release is to be picked by the owner ("next" until he gives it). The app is hosts/hearth-ui/apps/notes
(hearth-notes-app; docs/HEARTH-NOTES.md), built into the Fedora image and listed in the Hearth Store as Built in.
Pictures: hearth-apps-rust.yml (.github/scripts/hearth-apps-rust-shots.d/notes.sh, notes-1…13), each opened next to
its mockup before a part is ticked.

Status (Nov 20): built and tested green in CI (hearth-apps-rust 44718850634, only=notes: cargo test, every page
pictured, both pages at rest 0.00 % of a core, 49 MB). Every page opened next to its mockup; fixes from that in the next
commit. Needs testing on a device: the graph on a real graphics card (CI draws with the CPU), the clone with the owner's
own Git sign-in, Sync by itself, and the controller on the Ally. Where the app differs from the picture on purpose:
real counts (Black hole's made-up page counts are the vault's own; empty projects say "no notes yet"); the conflict's
other side is "another PC" (Git doesn't know which PC pushed); Railway map's station names are level (iced's canvas
can't turn text); New item's releases are the vault's own release tags.

## 1 Main window
- [x] Thin icon rail (files, search, today, board, graph, sync, new item, settings, help) — hosts/hearth-ui/apps/notes/src/side.rs: "icon(G::Asterisk, on_page(Page::Graph)"
- [x] Folder tree project > kind > item, with numbers (00 Atlas, 10 hearth, 20 other, 90 log, 99 tools) and counts — hosts/hearth-ui/apps/notes/src/side.rs: "\"log\" => \"90\".into(),"
- [x] Atlas of maps of content (Home = INDEX.md, hearth map, other map, daily logs map) — hosts/hearth-ui/apps/notes/src/side.rs: "\"Daily logs · map\""
- [x] Tabs across the top of each pane (note, log, Board; Graph view, Local graph) — hosts/hearth-ui/apps/notes/src/gview.rs: "\"✳ Local graph\""
- [x] Live preview Markdown: headings, checklist, a `[[link]]` shown raw on the cursor's line, an embedded picture, a code block — hosts/hearth-ui/apps/notes/src/editor.rs: "The block the cursor is in, raw"
- [x] `[[links]]` with autocomplete while typing — hosts/hearth-ui/apps/notes/src/editor.rs: "pub fn autocomplete(a: &App)"
- [x] Source and Read modes (Live / Source / Read) — hosts/hearth-ui/apps/notes/src/editor.rs: "(\"Read\".into(), m == Mode::Read"
- [x] Saves right away and atomically, with a .bak (outside the workspace) — hosts/hearth-ui/apps/notes/src/save.rs: "fn saves_atomically_with_a_bak_outside_the_vault"
- [x] Live graph pane in the middle, following the open note — hosts/hearth-ui/apps/notes/src/gview.rs: "Follows the open note"
- [x] Right column: month calendar with dots on days with a log and Today, outline, backlinks, tags, modified today — hosts/hearth-ui/apps/notes/src/editor.rs: "pub fn calendar(a: &App)"
- [x] Status bar: backlinks, properties, words, characters, saved — hosts/hearth-ui/apps/notes/src/editor.rs: "fn status_bar(a: &App)"
- [x] The vault: any folder, defaulting to the hearth-workspace clone; a first-run step clones it with the PC's own Git sign-in, and says how to sign in when Git isn't — hosts/hearth-ui/apps/notes/src/side.rs: "\"Clone hearth-workspace\""
- [x] Hearth's looks and accent followed — hosts/hearth-ui/apps/notes/src/main.rs: "fn themed(base: Pal, theme: &str)"

## 2 Today
- [x] Action buttons: Home, New note, Today's log, This week, Atlas, Close — hosts/hearth-ui/apps/notes/src/today.rs: "action_btn(a, \"◷ Today's log\""
- [x] Date title, "Friday, November 20, 2026", « Yesterday | Tomorrow », US Central time — hosts/hearth-ui/apps/notes/src/today.rs: "tm(day::long(a.day)"
- [x] The day starts at 6:00 AM (a setting, shown in Settings) — hosts/hearth-ui/apps/notes/src/day.rs: "fn the_log_day_starts_at_6_am"
- [x] Productivity bar (0–10) and optional habit tracker (Writing, Workout, Reading), kept apart from the log — hosts/hearth-ui/apps/notes/src/trackers.rs: "fn trackers_are_kept_apart_from_the_log"
- [x] Quick add: Made / Changed / Decided / Stored at / Waiting, with the line it will add — hosts/hearth-ui/apps/notes/src/today.rs: "pub fn quick_line("
- [x] Sections Made, Decided, Stored at, Waiting, What happened today, New tasks — hosts/hearth-ui/apps/notes/src/today.rs: "fn the_log_sorted_into_its_sections"
- [x] Right column: month calendar, sections of the page, still waiting from the last 5 logs, modified today — hosts/hearth-ui/apps/notes/src/today.rs: "\"Still waiting\""

## 2b Board
- [x] Columns Waiting, To do, Doing, Done this week, made from `- [ ]` and "Waiting:" lines — hosts/hearth-ui/apps/notes/src/board.rs: "fn cards_from_checklists_and_waiting_lines"
- [x] Each card names its note and tag, coloured by project; group by status, project, tag or date — hosts/hearth-ui/apps/notes/src/boardv.rs: "Group::Project =>"
- [x] Moving a card ticks its box or edits its line in the note — hosts/hearth-ui/apps/notes/src/board.rs: "fn moving_a_card_ticks_its_box_or_edits_its_line"

## 3 Graph, Classic
- [x] Nodes coloured by project (all 8 projects plus daily logs), filters by project and tag, colour by, link distance — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Link distance\""
- [x] Hover card with Open note and Show only its links; legend — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Show only its links\""
- [x] Drawn on the graphics card with a light CPU fallback; animates only while visible; Reduce motion still; ~0 % CPU at rest — hosts/hearth-ui/apps/notes/src/gview.rs: "fn glow(fr: &mut Frame"
- [x] Real counts from the vault: a project with no notes says "no notes yet" — hosts/hearth-ui/apps/notes/src/gview.rs: "\"no notes yet\""

## 3b Graph modes
- [x] 10 modes with live previews: Galaxy, Solar system, Black hole, Ember, Railway map, Night sky, Neural, City at night, Timeline river, Classic — hosts/hearth-ui/apps/notes/src/gview.rs: "(\"river\", \"Timeline river\""
- [x] Reduce motion, Customise, Stays light — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Stays light\""

## 3c Galaxy
- [x] Starfield, notes as stars, links as faint light trails, a nebula per project — hosts/hearth-ui/apps/notes/src/gview.rs: "\"galaxy\" if a.gb(\"nebulae\", true)"
- [x] Fly to search ("ts2010" flies the camera), hover card with Open note, camera turn and zoom, filters — hosts/hearth-ui/apps/notes/src/gview.rs: "pub fn fly_to(a: &mut App, i: usize)"

## 3d Solar system
- [x] Each project a sun (empty ones dim, "no notes yet"), each kind a ring, notes as planets sized by times opened — hosts/hearth-ui/apps/notes/src/gview.rs: "fn solar_order(a: &App)"
- [x] Moons for linked notes, daily logs as comets, hover card with Open note and Follow, filters — hosts/hearth-ui/apps/notes/src/gview.rs: "Comets' tails (the daily logs) and the planets' moons"

## 3e Black hole
- [x] Home (INDEX.md) as a black hole: orange-gold accretion disk, lensing glow, dark core, "Home" pill — hosts/hearth-ui/apps/notes/src/gview.rs: "The near side of the disk crosses the core"
- [x] Each project a big cloud of star points with fan-shaped link trails pulled toward Home, labelled with its name and page count — hosts/hearth-ui/apps/notes/src/gview.rs: "fn cloud_of(a: &App"
- [x] Faint orbit rings across the map, deep space behind, orphans at the edge — hosts/hearth-ui/apps/notes/src/gview.rs: "Faint orbit rings, the clouds"
- [x] Floating control pill: zoom out, zoom in, reset, pause, fullscreen; "7 backlinks" in the corner — hosts/hearth-ui/apps/notes/src/gview.rs: "fn controls(a: &App)"
- [x] Fly to, filters, hover card with Open note — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Fly here\""

## 3f Customise the graph
- [x] Nodes: a note's own picture or icon as its node, shapes by kind/tag/folder, colour groups, size by links/length/opened — hosts/hearth-ui/apps/notes/src/gview.rs: "\"A note's own picture or icon as its node\""
- [x] Links: curved or straight, arrows, glow, colour by type, hide orphans, hide attachments — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Hide attachments (.png, .py, .html)\""
- [x] Effects: bloom, starfield, particles along links when a note opens, label size, labels fade with zoom — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Labels fade out when zoomed out\""
- [x] Insight: most linked, orphans, clusters — hosts/hearth-ui/apps/notes/src/graph.rs: "pub fn clusters(&self)"
- [x] Saved looks: Galaxy night, Solar system, Ember, Minimal, Neon; save, share as a file, open — hosts/hearth-ui/apps/notes/src/gview.rs: "pub fn look_share(a: &mut App)"
- [x] Neighbourhood view: a mind map of parents, children and siblings around the open note — hosts/hearth-ui/apps/notes/src/gview.rs: "fn neighbourhood(a: &App, i: usize)"
- [x] Per-project looks — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Per-project looks\""
- [x] The local graph — hosts/hearth-ui/apps/notes/src/gview.rs: "Only the selected note and its links (Local graph"

## 3g Ember
- [x] Notes as embers rising from a fire (low and bright = changed lately, high = older), links as heat shimmer, filters, hover card — hosts/hearth-ui/apps/notes/src/gview.rs: "Heat shimmer: a wavy line."

## 3h Railway map
- [x] Kinds as lines, notes as stations, the daily logs as the Main Line with today's train, Dad's line for other/, legend and filter, hover card — hosts/hearth-ui/apps/notes/src/gview.rs: "\"Dad's line · other\""

## 4 Search
- [x] Full-text search with highlighted matches, filters by folder, tag and date, result counts — hosts/hearth-ui/apps/notes/src/search.rs: "fn full_text_with_filters_and_counts"
- [x] Quick switcher (Ctrl+O) with fuzzy matches, open, new tab, make a new note — hosts/hearth-ui/apps/notes/src/searchv.rs: "Quick switcher · Ctrl+O"

## 5 Sync
- [x] "3 changes to push" (and "Up to date" with its time in every page's corner), changed files with a diff — hosts/hearth-ui/apps/notes/src/syncv.rs: "fn diff_card(a: &App)"
- [x] Commit message, Sync now, sync by itself every 2/5/10/30 min (on, 5 min, as drawn; a setting turns it off), committing as Dragongamer9 with his Git config — hosts/hearth-ui/apps/notes/src/sync.rs: "fn sync_commits_as_the_person_and_pushes"
- [x] The workspace check (tools/check.py) with its exact problems and fixes, blocking the push — hosts/hearth-ui/apps/notes/src/sync.rs: "fn the_workspace_check_blocks_the_push_until_its_tidy"
- [x] A warning before committing anything that looks like a secret — hosts/hearth-ui/apps/notes/src/sync.rs: "fn a_secret_stops_the_sync_until_the_person_says"

## 6 Conflict
- [x] This PC / GitHub side by side, Keep mine, Keep theirs, Merge both; copies always kept outside the workspace — hosts/hearth-ui/apps/notes/src/sync.rs: "fn a_note_changed_on_two_pcs_keeps_both"

## 7 New item
- [x] Project and kind, the name check (lowercase-with-dashes, a suggestion, not taken), status and release — hosts/hearth-ui/apps/notes/src/newitem.rs: "fn names_follow_the_workspace_rules"
- [x] Preview of the README template, the INDEX line and the log line — hosts/hearth-ui/apps/notes/src/newitemv.rs: "\"It will make\""

## 8 Settings
- [x] GitHub through the PC's existing Git sign-in (never a password or token), the repository, commits — hosts/hearth-ui/apps/notes/src/settingsv.rs: "\"Check sign-in\""
- [x] Sync interval, Warn if a note looks like a secret, editor font and size, theme following Hearth's looks, Today's log, graph modes — hosts/hearth-ui/apps/notes/src/settingsv.rs: "\"Day starts at\""

## 9 Controller
- [x] Big focus rings, note list and reader, quick note with the on-screen keyboard, A/B/X/Y hints — hosts/hearth-ui/apps/notes/src/padview.rs: "pub fn osk_key(a: &mut App"
