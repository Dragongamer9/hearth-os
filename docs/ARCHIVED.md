# Archived parts of Hearth OS

Parts that aren't in Hearth OS 1.0 but are kept in the repository so they can come back later. Archived code isn't
built, shipped or tested automatically; its workflows run by hand only (some ask you to type "archived" first). Code
that was moved lives in `archive/` under its old path, and its docs in `docs/archive/`.

## Overview

| Part | Archived | Code | Docs |
|---|---|---|---|
| Hearth OS on Anatase (Fedora, image-based) | November 2026 (1.0); installs keep their last version | `os-bases/anatase` (stays: it shares checks and scripts with the Debian build); workflow `base-anatase.yml` | [BASE-ANATASE](archive/BASE-ANATASE.md) |
| Hearth OS on Ubuntu and on CachyOS | 2026-10-22 | `archive/os-bases/ubuntu`, `archive/os-bases/cachyos`; `base-ubuntu.yml` / `base-cachyos.yml` (by hand, type "archived") | [BASE-UBUNTU](archive/BASE-UBUNTU.md), [BASE-CACHYOS](archive/BASE-CACHYOS.md) |
| Hearth OS on Bazzite | 0.1.3 was its last update | `os/build_files`, `os/Containerfile` (stay: `os/` also holds the shared system files, the installer and Recovery) | |
| Hearth's own Game Mode | 1.0: Steam Game Mode replaced it | stays in place (`ui/`, `hosts/hearth-rs`, `hosts/hearth-core`): it still runs first-time setup and the fallback; Ember keeps its own Game Mode | |
| Multi-seat and Couch Co-op | 1.0 (hidden) | stays in place (`hosts/hearth-core`, `os/system_files/usr/libexec/nimbus/hearth-seats`, `hearth-coop`) | [MULTI-SEAT](archive/MULTI-SEAT.md), [COUCH-COOP](archive/COUCH-COOP.md) |
| Hearth Plasma, Hearth GNOME, Hearth COSMIC, Hearth Deepin | 1.0: Mantel is the only desktop | stays in place for now (`hosts/linux/desktop/{plasma,gnome,cosmic}`, their parts in `os/system_files`): the image build, tests and Hearth's core still refer to them | [PLASMA](archive/PLASMA.md), [GNOME](archive/GNOME.md), [COSMIC](archive/COSMIC.md), [DEEPIN](archive/DEEPIN.md) |
| The old Hearth Decky plugin (Steam takeover) | 1.0: Hearth Sync (`hosts/decky-hearth-sync`) replaced it | `archive/hosts/decky-hearth`; `decky-hearth.yml` | [STEAM-GAME-MODE](STEAM-GAME-MODE.md) |
| The small online installer | 1.0: the offline Debian USB and the Recovery USB are offered | stays in place (`os/installer`); `installer-iso.yml` by hand | |
| Hearth for Android and Hearth Linux on Android | 2026-10-21 | `archive/hosts/android`; `android.yml` (by hand, type "archived") | [ANDROID](archive/ANDROID.md), [ANDROID-LINUX](archive/ANDROID-LINUX.md) |
| Hearth's app store (Apps) | 2026-10-24: Bazaar replaced it | `archive/app-store` | below |
| Hearth Friends (Matrix) | 1.6: replaced by Discord, with the owner's okay | `archive/hosts/hearth-friends`, `archive/hosts/hearth-core/src/matrix_friends.rs`, `archive/hosts/linux/desktop/shell/Friends.qml`, `archive/tools/friends-*.py`; `friends.yml` (by hand, type "archived") | [FRIENDS](archive/FRIENDS.md), below |
| Hearth Music (Decky plugin) | November 2026: ThemeDeck replaced it, the owner's call | `archive/hosts/decky-hearth-music`; `decky-hearth-music.yml` (by hand, type "archived") | [HEARTH-MUSIC](archive/HEARTH-MUSIC.md), [THEMEDECK](THEMEDECK.md), below |

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

## Hearth Music (Decky plugin): archived November 2026

**ThemeDeck** replaced it as the game music plugin in Steam Game Mode, the owner's call ([THEMEDECK](THEMEDECK.md)):
BrenticusMaximus's Decky plugin (BSD-3-Clause), shipped unmodified at a pinned release.

**Where the code is:** `archive/hosts/decky-hearth-music` (the plugin: backend, panel, tests), its doc
[HEARTH-MUSIC](archive/HEARTH-MUSIC.md); `.github/workflows/decky-hearth-music.yml` runs only by hand, with "archived"
typed (its backend tests that check hearth-decky shipping it no longer pass: the image doesn't ship it).

**What happens on devices:** the image no longer ships it. At the first Steam Game Mode start after the update,
`hearth-decky` copies your own Hearth Music songs (music files you picked for a game) into ThemeDeck once, then takes
Hearth's copy of Hearth Music out of Decky's plugins folder (`RETIRED_BUILTIN`). It's also in `RETIRED` in
`tools/decky_plugins.py`, so plugins releases never build it again and list it as retired: a downloaded copy is
removed from devices still on an older image (their built-in copy stays until their Hearth OS update). Hearth Music's
settings and picks in `~/homebrew/settings/decky-hearth-music` and its cache in `~/.cache/hearth-music` are left where
they are. Hearth Look and Hearth Eras no longer mark their tiles for it, and Hearth Link no longer syncs its settings.

**To bring it back:** move the folder and doc back, take it out of `RETIRED` and `RETIRED_BUILTIN`, put it back in
`PLUGINS` (hearth-decky), the build step in `base-peppermint.yml` and the copy in `os-bases/peppermint/build.sh`, and
the parts in Hearth Look, Hearth Eras and Hearth Link (see the git history of this change).
