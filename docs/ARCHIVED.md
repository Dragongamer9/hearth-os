# Archived parts of Hearth OS

Code that's kept in the repository but not built, shipped or tested, so it can come back later.

## Hearth's app store (Apps) — archived 2026-10-24

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

## Also archived

See docs/ROADMAP.md › Archived: Hearth OS on Bazzite, on Ubuntu and on CachyOS, and Hearth for Android.
