# Hearth Notes (approved Nov 20, 2026, 03:20; the plan and the owner's decisions: hearth-workspace hearth/mockups/hearth-notes/README.md)

Mockups: hearth-notes/notes-1-main.png, hearth-notes/notes-2-today.png, hearth-notes/notes-2b-board.png, hearth-notes/notes-3-graph-classic.png, hearth-notes/notes-3b-graph-modes.png, hearth-notes/notes-3c-galaxy.png, hearth-notes/notes-3d-solar-system.png, hearth-notes/notes-3e-black-hole.png, hearth-notes/notes-3f-customise-graph.png, hearth-notes/notes-3g-ember.png, hearth-notes/notes-3h-railway-map.png, hearth-notes/notes-4-search.png, hearth-notes/notes-5-sync.png, hearth-notes/notes-6-conflict.png, hearth-notes/notes-7-new-item.png, hearth-notes/notes-8-settings.png, hearth-notes/notes-9-controller.png
Release: next

The release is to be picked by the owner ("next" until he gives it). The app is hosts/hearth-ui/apps/notes
(hearth-notes-app; docs/HEARTH-NOTES.md), built into the Fedora image and listed in the Hearth Store as Built in.
Pictures: hearth-apps-rust.yml (.github/scripts/hearth-apps-rust-shots.d/notes.sh, notes-1…13), each opened next to
its mockup before a part is ticked.

## 1 Main window
- [ ] Thin icon rail (files, search, today, board, graph, sync, new item, settings, help)
- [ ] Folder tree project > kind > item, with numbers (00 Atlas, 10 hearth, 20 other, 90 log, 99 tools) and counts
- [ ] Atlas of maps of content (Home = INDEX.md, hearth map, other map, daily logs map)
- [ ] Tabs across the top of each pane (note, log, Board; Graph view, Local graph)
- [ ] Live preview Markdown: headings, checklist, a `[[link]]` shown raw on the cursor's line, an embedded picture, a code block
- [ ] `[[links]]` with autocomplete while typing
- [ ] Source and Read modes (Live / Source / Read)
- [ ] Saves right away and atomically, with a .bak (outside the workspace)
- [ ] Live graph pane in the middle, following the open note
- [ ] Right column: month calendar with dots on days with a log and Today, outline, backlinks, tags, modified today
- [ ] Status bar: backlinks, properties, words, characters, saved
- [ ] The vault: any folder, defaulting to the hearth-workspace clone; a first-run step clones it with the PC's own Git sign-in, and says how to sign in when Git isn't
- [ ] Hearth's looks and accent followed

## 2 Today
- [ ] Action buttons: Home, New note, Today's log, This week, Atlas, Close
- [ ] Date title, "Friday, November 20, 2026", « Yesterday | Tomorrow », US Central time
- [ ] The day starts at 6:00 AM (a setting, shown in Settings)
- [ ] Productivity bar (0–10) and optional habit tracker (Writing, Workout, Reading), kept apart from the log
- [ ] Quick add: Made / Changed / Decided / Stored at / Waiting, with the line it will add
- [ ] Sections Made, Decided, Stored at, Waiting, What happened today, New tasks
- [ ] Right column: month calendar, sections of the page, still waiting from the last 5 logs, modified today

## 2b Board
- [ ] Columns Waiting, To do, Doing, Done this week, made from `- [ ]` and "Waiting:" lines
- [ ] Each card names its note and tag, coloured by project; group by status, project, tag or date
- [ ] Moving a card ticks its box or edits its line in the note

## 3 Graph, Classic
- [ ] Nodes coloured by project (all 8 projects plus daily logs), filters by project and tag, colour by, link distance
- [ ] Hover card with Open note and Show only its links; legend
- [ ] Drawn on the graphics card with a light CPU fallback; animates only while visible; Reduce motion still; ~0 % CPU at rest
- [ ] Real counts from the vault: a project with no notes says "no notes yet"

## 3b Graph modes
- [ ] 10 modes with live previews: Galaxy, Solar system, Black hole, Ember, Railway map, Night sky, Neural, City at night, Timeline river, Classic
- [ ] Reduce motion, Customise, Stays light

## 3c Galaxy
- [ ] Starfield, notes as stars, links as faint light trails, a nebula per project
- [ ] Fly to search ("ts2010" flies the camera), hover card with Open note, camera turn and zoom, filters

## 3d Solar system
- [ ] Each project a sun (empty ones dim, "no notes yet"), each kind a ring, notes as planets sized by times opened
- [ ] Moons for linked notes, daily logs as comets, hover card with Open note and Follow, filters

## 3e Black hole
- [ ] Home (INDEX.md) as a black hole: orange-gold accretion disk, lensing glow, dark core, "Home" pill
- [ ] Each project a big cloud of star points with fan-shaped link trails pulled toward Home, labelled with its name and page count
- [ ] Faint orbit rings across the map, deep space behind, orphans at the edge
- [ ] Floating control pill: zoom out, zoom in, reset, pause, fullscreen; "7 backlinks" in the corner
- [ ] Fly to, filters, hover card with Open note

## 3f Customise the graph
- [ ] Nodes: a note's own picture or icon as its node, shapes by kind/tag/folder, colour groups, size by links/length/opened
- [ ] Links: curved or straight, arrows, glow, colour by type, hide orphans, hide attachments
- [ ] Effects: bloom, starfield, particles along links when a note opens, label size, labels fade with zoom
- [ ] Insight: most linked, orphans, clusters
- [ ] Saved looks: Galaxy night, Solar system, Ember, Minimal, Neon; save, share as a file, open
- [ ] Neighbourhood view: a mind map of parents, children and siblings around the open note
- [ ] Per-project looks
- [ ] The local graph

## 3g Ember
- [ ] Notes as embers rising from a fire (low and bright = changed lately, high = older), links as heat shimmer, filters, hover card

## 3h Railway map
- [ ] Kinds as lines, notes as stations, the daily logs as the Main Line with today's train, Dad's line for other/, legend and filter, hover card

## 4 Search
- [ ] Full-text search with highlighted matches, filters by folder, tag and date, result counts
- [ ] Quick switcher (Ctrl+O) with fuzzy matches, open, new tab, make a new note

## 5 Sync
- [ ] "3 changes to push" (and "Up to date" with its time in every page's corner), changed files with a diff
- [ ] Commit message, Sync now, sync by itself every 2/5/10/30 min (on, 5 min, as drawn; a setting turns it off), committing as Dragongamer9 with his Git config
- [ ] The workspace check (tools/check.py) with its exact problems and fixes, blocking the push
- [ ] A warning before committing anything that looks like a secret

## 6 Conflict
- [ ] This PC / GitHub side by side, Keep mine, Keep theirs, Merge both; copies always kept outside the workspace

## 7 New item
- [ ] Project and kind, the name check (lowercase-with-dashes, a suggestion, not taken), status and release
- [ ] Preview of the README template, the INDEX line and the log line

## 8 Settings
- [ ] GitHub through the PC's existing Git sign-in (never a password or token), the repository, commits
- [ ] Sync interval, Warn if a note looks like a secret, editor font and size, theme following Hearth's looks, Today's log, graph modes

## 9 Controller
- [ ] Big focus rings, note list and reader, quick note with the on-screen keyboard, A/B/X/Y hints
