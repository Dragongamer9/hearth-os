# Archived parts of Hearth OS

Parts that aren't in Hearth OS (1.0 on; from 1.16 Hearth OS is Fedora-based) but are kept in the repository so they can come back later. Archived code isn't
built, shipped or tested automatically; its workflows run by hand only (some ask you to type "archived" first). Code
that was moved lives in `archive/` under its old path, and its docs in `docs/archive/`.

## Overview

| Part | Archived | Code | Docs |
|---|---|---|---|
| Hearth OS on Ubuntu and on CachyOS | 2026-10-22 | `archive/os-bases/ubuntu`, `archive/os-bases/cachyos`; `base-ubuntu.yml` / `base-cachyos.yml` (by hand, type "archived") | [BASE-UBUNTU](archive/BASE-UBUNTU.md), [BASE-CACHYOS](archive/BASE-CACHYOS.md) |
| Hearth OS on Debian (Peppermint): **archived**, merged into the Fedora edition | 2026-11-19, on the owner's word ("Only Fedora, Archive Debian", "mark Debian as Archived"); goes out with 1.16 | stays in place: `os-bases/peppermint` (the Fedora build copies `os-bases/peppermint/system_files`, and many tests and CI scripts read its `build.sh`, so moving it would break the Fedora edition); `base-peppermint.yml` by hand only (no push trigger, publish off by default); `release.yml` makes no Debian app bundle. Debian devices get nothing new and keep running the version they have, still startable (the owner's dad's PC stays pinned on `hearth-peppermint:testing-1.13.0-20261116`). The USB downloads are still the Debian ones until the Fedora ISOs come (planned with 1.16) | [BASE-PEPPERMINT](BASE-PEPPERMINT.md) |
| Hearth OS on Bazzite | 0.1.3 was its last update | `os/build_files`, `os/Containerfile` (stay: `os/` also holds the shared system files, the installer and Recovery) | |
| Multi-seat and Couch Co-op | 1.0 (hidden) | stays in place (`hosts/hearth-core`, `os/system_files/usr/libexec/nimbus/hearth-seats`, `hearth-coop`) | [MULTI-SEAT](archive/MULTI-SEAT.md), [COUCH-COOP](archive/COUCH-COOP.md) |
| Hearth Plasma, Hearth GNOME, Hearth COSMIC, Hearth Deepin | 1.0: Mantel is the only desktop | stays in place for now (`hosts/linux/desktop/{plasma,gnome,cosmic}`, their parts in `os/system_files`): the image build, tests and Hearth's core still refer to them | [PLASMA](archive/PLASMA.md), [GNOME](archive/GNOME.md), [COSMIC](archive/COSMIC.md), [DEEPIN](archive/DEEPIN.md) |
| KDE Plasma and GNOME as Desktop Mode's desktop (1.13's downloads, and 1.14's "look like Mantel") | 1.14: Mantel moves to KWin (the owner, Nov 16: "archive the KDE and GNOME choices, since we're migrating to KWin"); a pick from before opens Mantel and says so once | `archive/desktops-kde-gnome` (the helpers `hearth-desktops`, `hearth-desktops-helper`, `hearth-desktop-look`, `MantelLookExport.qml`, the old Your desktop page, the KDE tray item and start menu, GNOME's Hearth section, the login entries, the session's part, their tests) | [DESKTOPS-KDE-GNOME](archive/DESKTOPS-KDE-GNOME.md) |
| The old Hearth Decky plugin (Steam takeover) | 1.0: Hearth Sync (`hosts/decky-hearth-sync`) replaced it | `archive/hosts/decky-hearth`; `decky-hearth.yml` | [STEAM-GAME-MODE](STEAM-GAME-MODE.md) |
| The Recovery USB made from Hearth OS on Anatase | 1.12: the Recovery USB is made from Hearth OS on Debian | `archive/os/recovery-anatase` (its build script, Containerfile and boot menu) | |
| The ARM build of Hearth OS on Debian (hearth-peppermint-arm64) | 1.12: paused by the owner | the branch `feature/arm64` (kept as it is; its images are no longer built) | |
| The small online installer | 1.0: the offline Debian USB and the Recovery USB are offered | stays in place (`os/installer`); `installer-iso.yml` by hand | |
| Hearth's app store (Apps) | 2026-10-24: Bazaar replaced it | `archive/app-store` | below |
| Firehold (Rust, test): firehold-shell (Couch, Desk, the Guide in iced) on firehold-comp, Desktop Mode's choice on the Fedora edition | 2026-11-19: the owner, "focus on the Plasma Firehold for the moment and archive the Rust Firehold" | `archive/hosts/firehold-shell`, `archive/.github/workflows/firehold-rust.yml` and its scripts (`archive/.github/scripts/firehold-rust-*.sh`), the Plasma-menu choice (`archive/os-bases/anatase/system_files`: hearth-desktop-choice, its two menu entries, its keys rc.xml). `hosts/firehold-comp` stays in place: Debian's Firehold preview (Settings › Your desktop) builds it. A Fedora device that had picked it opens Firehold on Plasma instead and is told once; its pick (`~/.config/nimbus/firehold-rust.json`) is only read, and the next pick replaces it | [FIREHOLD-RUST](archive/FIREHOLD-RUST.md), checklist `docs/archive/checklists/firehold-rust.md` |
| Hearth Friends (Matrix) | 1.6: replaced by Discord, with the owner's okay | `archive/hosts/hearth-friends`, `archive/hosts/hearth-core/src/matrix_friends.rs`, `archive/hosts/linux/desktop/shell/Friends.qml`, `archive/tools/friends-*.py`; `friends.yml` (by hand, type "archived") | [FRIENDS](archive/FRIENDS.md), below |

## Back from the archive

| Part | Archived | Back | Docs |
|---|---|---|---|
| Hearth Game Mode (Hearth's own Game Mode) | 1.0: Steam Game Mode replaced it (its code stayed in place: `ui/`, `hosts/hearth-rs`, `hosts/hearth-core`) | 1.10: a choice in Settings › Gaming › Game Mode next to Steam Game Mode (the default); falls back to Steam Game Mode, then Mantel, when it can't start | [HEARTH-GAME-MODE](HEARTH-GAME-MODE.md) |
| Hearth OS on Anatase (Fedora, image-based) | November 2026 (1.0) | 1.16, as the only edition (decided with 1.15's work; 1.15 itself wasn't released; the owner, Nov 19: "Only Fedora, Archive Debian"; it came back as a test on Nov 17): shown as **Fedora**, image `ghcr.io/dragongamer9/hearth-os:test-1.15` (what the owner's ROG Ally and laptop follow), stock KDE Plasma as Desktop Mode, Unlock the file system. `os-bases/anatase`, `base-anatase.yml` (by hand, publishes only from main or to an owner-approved test tag) | [BASE-ANATASE](BASE-ANATASE.md) |

## Hearth's app store (Apps): archived 2026-10-24

Hearth OS ships **Bazaar** (`io.github.kolunmi.Bazaar`, a full Flathub store) as its app store on every edition
instead. Hearth's own store didn't show apps reliably.

**Where the code is:** `archive/app-store/`, in the same folders it had before:

| File | What it was |
|---|---|
| `archive/app-store/hosts/linux/desktop/shell/AppStore.qml` | The Apps window in Mantel (browse, search, an app's page, install, update, remove) |
| `archive/app-store/hosts/linux/desktop/shell/hearth-apps` | Its helper: Flathub's catalog (appstream), popular apps, flatpak install/update/remove with progress |
| `archive/app-store/.github/scripts/hearth-store-check.sh` | The image check that the store lists apps offline |
| `archive/app-store/hosts/linux/test/test_hearth_apps_*.py` | Its tests (run them from `archive/app-store/hosts/linux`: `python3 -m unittest discover -s test`) |

The image builds no longer install `hearth-apps.desktop`, and nothing outside `archive/` opens the store. The icon
`branding/icons/apps/hearth-apps.svg` stays in the icon set.

**What took its place:**

- `hosts/linux/desktop/shell/hearth-flatpak`: the parts the rest of Hearth still needs: whether your own Flatpak
  folder can be used (Settings › Apps' "Fix your Flatpak folder", the bug report), removing a Flatpak app (Mantel's
  app menu › Uninstall…) and opening Bazaar.
- `/usr/libexec/nimbus/hearth-flatpak-setup` (`hearth-flatpak-setup.service`, both editions): Flathub and Bazaar for
  everyone at every boot, retried every 15 minutes while offline; its last run is in `/var/lib/hearth/flatpak-setup.json`.
- Bazaar is pinned on Mantel's taskbar (new accounts, and once for existing ones when it's installed; a pin of the old
  Apps becomes Bazaar), is the **App store** in Settings › Default apps, opens from Game Mode's menu (**Apps**: full
  screen, controller mouse on) and from `quickshell ipc … call apps open` (older Apps links and keybinds).
- Bazaar gets Hearth's colours without changing it: `theme/apply.sh` writes Hearth's GTK 4 / GTK 3 colours (with your
  profile's accent) to `~/.config/gtk-4.0` and `gtk-3.0`, and Flatpak overrides let Bazaar read them.

**To bring it back:** move the files back to their old places, put `AppStore { id: appStore }` back in `shell.qml`
(and the launcher's Uninstall… to `appStore.open(entry.id)`), the `hearth-apps.desktop` entry back in
`os-bases/anatase/build.sh` and `os-bases/peppermint/build.sh`, and the store check back in `base-anatase.yml` and
`base-peppermint.yml` (see the git history of this change).

## Hearth Friends (Matrix): archived in 1.6

Discord replaced it, with the owner's okay ([DISCORD](DISCORD.md)): the bar's Friends button is Discord's, Quick
Access and Settings point to Discord, and "Playing <game>", invites and Play together go through Discord's Rich
Presence.

**Where the code is:** `archive/hosts/hearth-friends` (the Matrix service), `archive/hosts/hearth-core/src/matrix_friends.rs`
(the core's side as it was; its Play together part lives on in `hosts/hearth-core/src/play_together.rs`),
`archive/hosts/linux/desktop/shell/Friends.qml` (Mantel's panel), `archive/tools/friends-e2e.py` and
`friends-oauth-e2e.py` (the end-to-end tests); `.github/workflows/friends.yml` runs only by hand, with "archived" typed.
Hearth's own Game Mode page (`ui/index.html`) keeps its Matrix screens, hidden: the core no longer offers `mfriends`,
and the old `mfriends.*` calls answer that Discord replaced it.

**What happens on devices:** the image no longer has `/usr/bin/hearth-friends` or its user service, and nothing starts
it; a leftover enable link from an older image points at nothing and systemd skips it. People's Matrix data (keyring
entries, `~/.local/share/hearth/friends/`) and settings are left untouched and aren't read.

**To bring it back:** move the folders back, put `pub mod matrix_friends;` back in `hearth-core/src/lib.rs` and its
calls in `bridge.rs`, `play/mod.rs` and `core_service.rs`, `Friends { }` in `shell.qml` and the buttons in `Bar.qml` /
`Islands.qml`, and the build and service back in `base-*.yml` and `os-bases/*/build.sh` (see the git history of this
change).
