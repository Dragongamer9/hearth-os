# Every Hearth part in the Hearth Store (1.18 plan)

The owner, Nov 23, 2026: "Every thing in Hearth OS that we made Needs to be in the hearth Store so it can receive
updates without system updates", then "Yes work on it" and "1.18 is the version and everything in the hearth store gets
bumped to 1.18.0". Approved mockups: docs/mockups/checklists/store-everything.md.

## What's in the Store already, and what isn't

| Kind | Today | 1.18 |
|---|---|---|
| Apps run as you (Files, Settings, Task Manager, Capture, Notes, G15, Family, Server, Resource monitor, System Restore, Reliability, Ideas, file chooser, controller, the Store) | Store parts (hearth-part, in your home) | the same, 1.18.0 |
| Mantel, Mantel in Rust (with their own hearth-layout) | Store parts | the same, 1.18.0 |
| Hearth's own Game Mode (hearth-rs and ui/), Hearth Sync, the Decky plugins | system update only | **user parts** (they run as you): like the apps |
| Hearth Link, Hearth's gaming pieces, looks and sounds | system update only | **user parts** |
| Helpers that run as root (home server, storage, AI engines, sharing… /usr/libexec/nimbus) and Hearth's services and timers | system update only | **system parts** (below) |
| Boot menu, Safe start, Recovery, the login screen | system update only | **stays with system updates** (red flag 2: the one before always stays startable) |
| Fedora underneath | Hearth OS image updates | **Fedora updates, by date** (Library › Hearth OS), applied at the next restart; nothing by itself |

## System parts: safe for programs that run as root

A user part lives in `~/.local/share/hearth/parts`, which any program running as you can change. A program that runs
as root must never run a file from there: that would let anything you run become root. So system parts are their own
kind:

- **Where:** `/var/lib/hearth/system-parts/<id>/<version>/`, owned by root, never writable by anyone else.
- **Installing:** only `hearth-system-part install` run as root (the Store asks through polkit, admin password, the
  same rule as Hearth's other root helpers). It downloads, checks the SHA-256 against the list signed with the owner's
  key (the same list as user parts), unpacks into a new folder, checks every file is root's and not group- or
  world-writable, then switches `current` in one rename. The version before stays next to it.
- **Using:** each root helper and service starts through `hearth-system-part run <id> <built-in>`: the Store's copy
  only when it's root's, made for this Hearth OS, and hasn't failed; otherwise the built-in one in /usr. Anything
  wrong means the built-in one.
- **Services:** a part with services restarts them after installing (no restart of the PC). One that fails to start
  twice goes back to the version before by itself and says so (the Store's Hearth parts page).
- **Never:** anything that starts the PC (units in the boot chain, the boot menu, the login screen, Recovery) is a
  system part. Those come only with system updates.

## Versions

The owner picks every number. At the 1.18 release every part goes to **1.18.0** (Nov 23); after that each part counts
its own updates (1.18.1, 1.18.2…), and each keeps a main and a test version (red flag 13).

## Steps

1. User parts for Game Mode, Hearth Sync, the Decky plugins, Hearth Link, gaming, looks and sounds (quick-update.yml
   options and their start lines, like the apps').
2. `hearth-system-part` and its polkit rule; the root helpers and services start through it; tests that a part in
   your home is never run as root and that a part not owned by root is never used.
3. The Store: the Hearth parts page grouped as the mockup (Desktop, Apps, Game Mode, Services, Looks and sounds), the
   system parts asking for the admin password, Library › Hearth OS with Fedora updates by date.
4. The release: every part built and listed at 1.18.0 with the Hearth OS 1.18 release, when the owner says push.
