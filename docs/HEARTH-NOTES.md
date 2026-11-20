# Hearth Notes (working name; the release is the owner's to pick)

Dragongamer9's own Obsidian, connected to his hearth-workspace repository: Markdown notes with live preview and
`[[links]]`, the daily log as a dashboard, a Board made from checklists, a graph in ten modes, search, Git sync that runs
the workspace's own check before every push, conflicts settled without losing anything, the New item wizard, Settings
and a controller layout.

- App: `hosts/hearth-ui/apps/notes` (pure Rust, hearth-ui, iced 0.14: wgpu with tiny-skia as its fallback), the program
  `hearth-notes-app`, menu entry "Hearth Notes" (`os-bases/anatase/system_files/usr/share/applications/hearth-notes.desktop`).
- Approved mockups: `docs/mockups/hearth-notes/notes-1…9.png` (approved Nov 20, 03:20: "looks amazing lets do it"),
  checklist `docs/mockups/checklists/hearth-notes.md`; the plan and the owner's open questions: hearth-workspace
  `hearth/mockups/hearth-notes/README.md`.
- In the Hearth Store as **Built in** (catalog serial 2): the Store installs only AppImages from an app's own GitHub
  releases; Hearth's own hearth-ui apps (like Logitech G15) are part of Hearth OS, so Hearth Notes is in the Fedora
  image and updates with it.

## Where things are

| What | Where |
|---|---|
| The vault | Any folder (Settings › Repository › Change…); by default `~/hearth-workspace`. Missing: a first-run step clones `Dragongamer9/hearth-workspace` with this PC's Git sign-in |
| Settings | `~/.config/nimbus/hearth-notes.json` (settings_file.rs: atomic, .bak, merged; written only when you change one) |
| A note's last good copy | `~/.local/share/hearth/notes/bak/<vault>/<note>.bak` (outside the vault, so nothing extra is committed) |
| Productivity and habits | `~/.local/share/hearth/notes/trackers.json` (not `log/.trackers.json`: the workspace check allows only `log/YYYY-MM-DD.md` in `log/`, and the log's format never changes) |
| Copies kept from a conflict | `~/.local/share/hearth/notes/kept/<date time>/<note>.this-pc` and `.github` |
| Times each note was opened | `~/.local/share/hearth/notes/opened.json` (Solar system's planet sizes, the controller's recent notes; only when you open one) |

## Rules it keeps

- **Signing in**: only the PC's own Git sign-in (GitHub CLI's, Git's credential helper, or an SSH key). Hearth Notes
  never asks for, stores or sends a password or token; Git runs with `GIT_TERMINAL_PROMPT=0`, so it can't ask either.
  Not signed in: it says to run `gh auth login` once.
- **Commits** are made with the person's own Git config (user.name, user.email), with their message and nothing added.
- **Before every push**: secret-looking lines (tokens, passwords, private keys) stop it until you decide; then
  `python3 tools/check.py` runs, and while it finds problems nothing is committed or pushed (each problem is shown as
  the check says it, with Make README from template / Add the INDEX line). Then commit, pull (a merge, never a rebase),
  push.
- **Sync by itself** every 5 minutes (as drawn; 2, 5, 10, 30 or Off in Settings): one timer, skipped while a game runs
  (gamescope or Hearth's hearth-playing mark) and while the check fails.
- **Saving** a note: a moment after typing stops, all at once (temporary file, fsync, rename), its old text kept as .bak.
- **Today's log** is `log/YYYY-MM-DD.md` in US Central time (worked out in the app, whatever the PC's time zone). "Day
  starts at" is 6:00 AM by default (the owner works overnight): until then Today is still the day before. It's in
  Settings › Looks and Today.
- **Light at rest**: nothing polls. The graph modes animate only while their page is in front, the window is focused and
  something was done there in the last 30 s; Reduce motion (Hearth's) draws them still; with the CPU drawing (no graphics
  card) fewer stars and particles are drawn at 15 frames a second. CI measures both pages at rest.
- **Counts are real**: a project with no notes shows "no notes yet"; nothing in the app is made up (the mockups' Black
  hole page counts were only in the picture).

## Pictures in CI

`hearth-apps-rust.yml` (dispatch with `only=notes` for just these) pictures every page on pretend worlds
(`.github/scripts/hearth-notes-fixture.py`: the workspace's layout with the mockups' content, a local repository as
GitHub, the night of Nov 20 at 3:04 AM): `.github/scripts/hearth-apps-rust-shots.d/notes.sh` (notes-1…13). Tests:
`cargo test` in `hosts/hearth-ui/apps/notes` (Markdown and links, the 6 AM log day, trackers, sync against a local bare
repository, check.py blocking, secrets, conflict keep-both, new items, the Board, search).
