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
