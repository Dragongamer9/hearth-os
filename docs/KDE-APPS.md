# Hearth's KDE apps

Part of the desktop update (ROADMAP "Desktop update"): KDE apps rebuilt and renamed as Hearth apps, for Hearth Plasma
(the KDE Plasma desktop themed as Hearth, built on its own branch) and for Hearth Classic. Everything stays free
software under KDE's licences (GPL), and KDE is credited: each app's own About window is KDE's, with its authors, and
Hearth Files' About adds what Hearth changed and where to find it.

| Hearth app | Built from | How | Menu entry | Replaces (stays installed) |
|---|---|---|---|---|
| **Files** (Hearth Files) | KDE Dolphin | built from Dolphin's release source + Hearth's patch + Hearth's sources, in CI | `hearth-files-kde.desktop` | the old Hearth Files (only outside Hearth Classic and Game Mode, see below) |
| **Terminal** (Hearth Terminal) | KDE Konsole | the base's package, Hearth's profile and colors | `hearth-terminal-kde.desktop` | Terminal (classic) |
| **Text Editor** (Hearth Text) | KDE Kate | the base's package, Hearth's colors | `hearth-text-kde.desktop` | Text Editor (classic) |
| **Reader** (document viewer) | KDE Okular | the base's package, Hearth's colors | `hearth-reader.desktop` | nothing: Documents stays the default for PDFs and e-books |
| **Capture** (Hearth Capture) | KDE Spectacle | the base's package, Hearth's colors | `hearth-capture.desktop` | nothing (Recorder stays as it is) |
| **Calculator** (Hearth Calculator) | KDE KCalc | the base's package, Hearth's colors | `hearth-calculator-kde.desktop` | Calculator (classic) |

Kept as they are: Photos, Contacts, Time, Paint, Office and every other Hearth app.

**Which desktop shows which** (docs/APP-KINDS.md, one app of each kind per desktop): Hearth Plasma shows these KDE-based
apps; Hearth Classic and Game Mode show the classic Hearth ones (Files, Terminal, Text Editor, Calculator, Documents)
and open files in them. KDE's own entries and stock KDE duplicates (KWrite, Gwenview, Elisa...) are hidden.

Where it all lives: `hosts/hearth-kde/` (`kio-hearth/`, `dolphin/`, `apps/`, `config/`, `install.sh`,
`hearth_kde.py`, `hearth-files-kde`, `tests/`), `.github/scripts/hearth-kde-build.sh`, `.github/workflows/kde-apps.yml`,
the `kde` job in `base-anatase.yml` and `base-peppermint.yml`, and the "Hearth's KDE apps" block in both
`os-bases/*/build.sh`.

## Hearth Files (Dolphin)

Everything Dolphin does, plus Hearth's:

- **Hearth Storage in Places**: "Hearth Storage" (`hearth:/`: your paired Hearth Storage servers, then their shared
  folders) and one place per shared folder ("Movies on Den PC"), from Hearth Link, brought up to date each time Hearth
  Files starts and at sign-in. They're KDE's Places (`~/.local/share/user-places.xbel`), so every KDE app's Open /
  Save dialog has them too. A place Hearth added that you remove stays removed (`~/.config/nimbus/hearth-files-kde.json`
  remembers what Hearth added); a share that's no longer shared loses its place.
- **Controller**: Hearth Files reads the controllers itself while one of its windows is in front:
  D-pad moves between files (held: repeats), **A** opens, **B** goes back (or closes a menu, dialog or rename),
  **X** is the file's menu, **Y** filters the folder, **Menu** is Dolphin's menu, **LB/RB** back/forward,
  **LT/RT** previous/next tab, **View** jumps to Places and back. In Hearth Classic, Hearth's controller helper
  (hearth-pad) already types arrows, Enter, Esc and the Menu key for it (Pad.qml treats Hearth Files' windows, app id
  `org.kde.dolphin`, like its own menus) and Y opens the on-screen keyboard; Hearth Files then only does LB/RB, LT/RT
  and View, so nothing happens twice. `HEARTH_FILES_NO_CONTROLLER=1` turns it off.
- **Hearth look**: in Hearth Plasma (Plasma's color scheme) and Hearth Classic (qt6ct's Hearth palette) the desktop
  already gives it Hearth's colors; anywhere else it takes them from `Hearth.colors` itself. Hearth's icon and name
  in menus and the taskbar, and "Hearth Files" as the program's name (window titles, About, `--help`).

How it's built (so it's easy to keep up with Dolphin):

- `dolphin/versions.txt` pins, per base, the Dolphin release the base itself packages (Anatase/Fedora 45: 26.08.1
  on KDE Frameworks 6.31; Peppermint/Debian trixie: 25.04.3 on KDE Frameworks 6.13), with the release tarball's sha256 from download.kde.org. Same release as the base's package means Hearth
  Files links exactly the base's KDE Frameworks, and Dolphin's plugins from the base's package fit it. A new base
  release: CI says the new version and checksum (a warning with the line to pin) and builds it, but only a pinned,
  checked release goes into an image.
- `dolphin/patches/`: the patch series, applied with `patch -p1`. Today one patch, three lines in `src/main.cpp`:
  call `hearthFilesSetup()` before Dolphin registers its about data.
- `dolphin/hearth/`: Hearth's own sources (`hearthfiles.cpp`: name, credits, colors; `hearthgamepad.cpp`: the
  controller), added to Dolphin's build by `hearth.cmake` (`-DCMAKE_PROJECT_INCLUDE`, so Dolphin's CMakeLists aren't
  edited).
- Installed in its own prefix, `/usr/lib/hearth-files` (program, libraries and plugins only). The base's own
  `dolphin` package is installed too (it brings the libraries, translations and plugins) and isn't changed, except that
  its menu entry `org.kde.dolphin.desktop` gets Hearth's name and icon, starts Hearth Files and is hidden (Hearth's
  entry is the one shown; KDE's stays because it matches the window's app id).
- `/usr/bin/hearth-files-kde` starts it (Places first). If Hearth Files isn't in the image it opens the old Hearth
  Files instead, so a folder always opens.

**When it replaces the old Hearth Files.** Only when CI built it from a pinned release, `--help` says it's Hearth
Files, and it ran for 10 seconds on an offscreen screen without crashing (the `kde` job), and in the image it links
every library and starts (install.sh). Otherwise it's left out and nothing changes. When it's in:

- Hearth Plasma shows it and opens folders in it (`/etc/xdg/kde-mimeapps.list`, docs/APP-KINDS.md);
- Hearth Classic and Game Mode keep their built-in Files, the only file manager they show
  (`/etc/xdg/nimbus-mimeapps.list`): it's part of the Classic desktop (its desktop icons, the portal's file chooser,
  Game Mode's file window), and Classic is the fallback desktop. Settings › Default apps can still pick it.
- The old Hearth Files stays installed either way.

## Hearth Storage in every KDE app: kio-hearth

`hosts/hearth-kde/kio-hearth` is a KIO worker (C++, Qt 6, KDE Frameworks 6) for `hearth:` addresses:

| Address | What it is |
|---|---|
| `hearth:/` | your paired Hearth Storage servers (by name) |
| `hearth:/SERVER` | that server's shared folders |
| `hearth:/SERVER/SHARE/PATH` | a folder or file in a shared folder |

SERVER and SHARE are the same ids as the old Hearth Files' `hearth://SERVER/SHARE/PATH`, which is accepted too (the
path form is used because a 64-character id is longer than a host name may be).

It doesn't speak Hearth Link's network protocol: it asks the running Hearth Link service on its user socket
(`$XDG_RUNTIME_DIR/hearth-link/link.sock`, the same JSON lines as `hearth-link call`) exactly what the old Hearth
Files asks: `storage.status` (servers, shares, whether Media is on), `storage.servers` (to ask a server that hasn't
answered yet; it wakes it if it can), `storage.ls` (a folder), and `storage.stream` for a file's bytes, which it then
reads from Hearth Link's local stream (`http://127.0.0.1:<port>/<token>/<name>`, the link Hearth Player plays from;
only that loopback address is ever connected to). So pairing, encryption, waking, "away from home" and every check
on the server stay Hearth Link's.

- **Read-only**, like the old Hearth Files: listing, details, opening and copying files to this device. Anything
  that would change a shared folder says "Hearth Storage's shared folders can't be changed from here: copy what you
  need to this device".
- Hidden files are never listed (the server doesn't give them out either), and addresses with `.`/`..`/hidden parts
  are refused before anything is asked.
- Opening a file needs **Media** on, on the server (Settings › Hearth Storage › Media), because that's what lets a
  device read a shared file's bytes; with it off it says so (the classic Hearth Files' copy, which has Hearth Link
  copy the file in the background, works without it).
- Every file's type comes from its name, so KIO never reads a file just to find out what it is.

Tests (`kio-hearth/tests`, run by CTest in CI inside both bases): `hearthcoretest` (addresses, refused addresses,
Hearth Link's answers, a socket that isn't there, the stream's link and head, entries from `storage.status` /
`storage.ls`) and `kiohearthtest`, which goes through real KIO jobs (KIO starts the built worker in its own process,
as in any app) against a fake Hearth Link socket and stream: listing servers, shares and a folder (no hidden files),
stat, reading a 380 KB file, a folder that's really a file, a missing file, `mkdir` refused, a bad address.

## The renamed KDE apps

The base's own packages (`konsole kate okular spectacle kcalc`, no recommended extras), unchanged: their About
windows, authors and licences are KDE's. Hearth adds:

- its menu entries (`apps/*.desktop`): Hearth names and icons (`branding/icons`: the Hearth set, with two new
  icons, `hearth-reader` and `hearth-capture`), `X-Hearth-Based-On=` naming the KDE app;
- KDE's own entries renamed and hidden (`hearth_kde.py rebrand`), so the taskbar and window switcher show Hearth's
  name and icon for the app's windows (they match by app id);
- Hearth's colors: `[UiSettings] ColorScheme=Hearth` in `/etc/xdg/konsolerc`, `katerc`, `okularrc`, `kcalcrc`,
  `spectaclerc` (the apps' own color menus can change it), and Konsole's default profile `Hearth.profile` with the
  `Hearth.colorscheme` palette;
- the old Hearth Terminal, Text Editor and Calculator renamed "… (classic)" and kept. Hearth Plasma shows and uses
  the KDE-based ones, Hearth Classic and Game Mode the classic ones (docs/APP-KINDS.md); Settings › Default apps
  offers both. Places that start the old ones by name (`hearth-terminal -e …`) still do.

Not changed: what the apps call themselves inside their windows and About boxes (Konsole, Kate, Okular, Spectacle,
KCalc). That needs each app rebuilt the way Dolphin is; Hearth Files shows how, if it's wanted later.

## Image size

Measured by install.sh as the growth of `/usr` (a notice on the image build, and `/usr/share/hearth-kde/installed.json`
in the image). Peppermint (no KDE before) pulls in KDE Frameworks for the five apps and Dolphin; Anatase already has
most of KDE Frameworks. Hearth Files itself adds about 4 MB on top of the base's Dolphin package.

Last measured (CI image builds, commit fd67dc7): **Anatase +57 MB**, **Peppermint +269 MB** (KDE Frameworks and the
apps' libraries, which Peppermint didn't have; Hearth Plasma on Peppermint will need most of them anyway). On that
Peppermint build Capture (Spectacle) didn't install (install.sh warns and goes on); the other five did.
