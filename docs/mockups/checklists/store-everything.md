# Every Hearth part in the Hearth Store, and Fedora updates (approved Nov 23, 2026)

The owner, Nov 23: "Every thing in Hearth OS that we made Needs to be in the hearth Store so it can receive updates
without system updates. Also has to be a way to add the Fedora system to it as well for fast updates", then "Lets do
fedora updates by date and nothing updates by its self" and "Yes work on it". Built in the Hearth Store app
(hosts/hearth-ui/apps/store), quick-update.yml and hearth-part; the look is store-win11's. Sources: hearth-workspace
hearth/mockups/2026-11-23-store-everything. Every part keeps a main and a test version (red flag 13).

Mockups: store-everything/store-1-hearth-parts.png, store-everything/store-2-hearth-os-fedora-updates.png, store-everything/store-3-updating.png
Release: next

## 1. Hearth parts
- [ ] "Hearth parts" with "Checked N min ago · N updates · none needs a restart", Check for updates, Update all
- [ ] The line: everything Hearth makes, each with its own updates, no system update and no restart; a part that doesn't start goes back to the one before by itself
- [ ] Filter pills with counts: All, Desktop, Apps, Game Mode, Services, Looks and sounds
- [ ] Desktop: Mantel, Mantel in Rust (test version), Wallpapers and looks, Hearth sounds
- [ ] Game Mode: Game Mode host, Hearth Sync (Decky), Hearth Link (Decky)
- [ ] Services: Home server helper (with what the update fixes), Hearth Link, AI engines, Hearth core, Recovery and Safe start ("with system updates")
- [ ] Each row: its letter icon, name, a short line, installed version, Up to date or the new version with Update
- [ ] The note: every part checked with SHA-256 against the signed list; anything that touches starting the PC only comes with a system update
- [ ] Every Hearth part, not only those drawn: apps, Decky plugins, helpers, services, gaming pieces, looks and sounds

## 2. Library › Hearth OS
- [ ] Title "Library › Hearth OS" with Check for updates
- [ ] The system card: THE SYSTEM · FEDORA <n> · HEARTH OS <version>, "Fedora updates · <date>", packages, what they fix, size (only what changed), Get it now, about how long and "applies on your next restart"
- [ ] Three kinds of update: Hearth parts (seconds · no restart), Fedora updates (minutes · next restart, named by date, downloaded when you say, ready), Hearth OS <next> (bigger · next restart, when Dragongamer9 releases it, not yet)
- [ ] Safe every time: the one before stays startable (named, in the boot menu), back by itself if the new one doesn't reach Game Mode twice, your settings apps and parts stay
- [ ] What's in this one: package changes old → new, and N more, See all
- [ ] Fedora updates by themselves: a switch that starts off (off: nothing downloads until Get it now; on: in the background, applies at restart; never restarts on its own)

## 3. Updating
- [ ] Library with Check for updates
- [ ] Fedora updates · <date> downloading after Get it now: what changed, MB of MB, you can keep playing, progress, Pause
- [ ] Parts done: Mantel (the desktop came back with the new one), Home server helper (services started again)
- [ ] "Fedora updates are ready for your next restart": nothing restarts by itself, the version before stays in the boot menu and comes back by itself; Later, Restart now
- [ ] Done before: the history, History
