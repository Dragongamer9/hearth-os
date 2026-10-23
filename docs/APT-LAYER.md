# The writable layer: apt on Hearth OS Peppermint

Hearth OS's Peppermint edition (Beta Experimental) is still an image underneath. It updates in one piece (bootc),
and the previous version stays in the boot menu to roll back to. On top of that image sits a **permanent writable
layer**, so the system doesn't feel locked:

- **`apt install`** and opening a **`.deb`** file work. So does `sudo apt install ./file.deb`.
- **`apt remove` and `apt purge`** work, including on packages that came with the system.
- **`apt upgrade` and `apt full-upgrade`** work, including on packages that came with the system.
- **Everything stays through restarts.**
- **After a system update, Hearth puts your changes back** on the new version by itself, in the background, a
  minute or so after you sign in. A notification tells you how it went:
  - What you **installed** is installed again: from your software sources, or from the kept copy of a `.deb`.
  - What you **removed** is removed again, unless the new version needs it. "Needs" means removing it would
    take other system packages with it. Then it's skipped and you're told.
  - What you **upgraded** is upgraded again, but only while the new version's own package is still older than
    yours. If the new version already has your version or newer, the image caught up. Hearth stops tracking that
    upgrade.
  - Anything that doesn't fit is skipped and reported. It never stops the boot or the update.
- **Only a few things stay managed by Hearth** (refused, with a message saying why):
  - The **kernel, initramfs and boot loader** (`linux-image-*`, `linux-modules-*`, `grub*`, `shim*`,
    `dracut*`, `initramfs-tools`, `systemd-boot`, `bootc`, `ostree`, …). They're loaded before the writable
    layer exists, so a change couldn't take effect. They update with Hearth's system updates. `apt upgrade`
    passes over them instead of failing.
  - **Removing what Hearth's own tools run on.** That's apt, dpkg, Python, mount, sudo and pkexec, and systemd,
    plus what they depend on (about a hundred packages, listed in `/usr/share/hearth/apt-layer/protected`).
    Removing one would break apt itself, and with it the way back. Upgrading them is fine.

The rest of the file system:

| Where | Writable? | Kept through updates? |
|---|---|---|
| `/usr` | Yes, through the layer | Your changes are put back on each new version (see above) |
| `/etc` | Yes (always was) | Yes. Files you changed are kept; ones you didn't follow the new version |
| `/var`, `/home` | Yes (always was) | Yes |
| `/opt` | Yes: it's a link to `/var/opt` | Yes. Packages that install into `/opt` (Chrome, Discord, …) work |
| `/usr/local` | Yes: it's a link to `/var/usrlocal` | Yes |
| `/` itself (new top-level folders) | No | That's the image. Use `/var`, `/opt` or your home folder |
| `/boot` | Managed by Hearth (bootc and ostree) | |

Flatpak (the Apps store) is still the simplest way to get apps. The layer is for everything else.

## Using it

| You do | What happens |
|---|---|
| `sudo apt update`, then `sudo apt install vlc` | Installed. apt prints "Hearth: your changes are kept through restarts and put back after system updates". |
| Open a `.deb` file (Hearth Files, or any file manager) | A terminal opens, asks for your password, and installs it and what it needs. A copy of the file is kept so it can come back after updates without the internet. |
| `sudo apt remove nano` | Removed, and kept removed after updates (unless a new version needs it). |
| `sudo apt full-upgrade` | Upgrades everything that has a newer version, except the kernel and boot loader. Re-applied after updates while the new version is still older. |
| `sudo apt install linux-image-…`, `grub-…`, `dracut-…` | Refused: the kernel, initramfs and boot loader come with the system image. |
| `sudo apt remove python3` | Refused: Hearth's own tools need it. |
| `hearth-apt-layer status` | What you installed, removed and upgraded, and what happened after the last update. |
| `sudo hearth-apt-layer retry` | Try the skipped changes again. This is also a button in Desktop Mode's Settings › Updates. |
| `sudo hearth-apt-layer forget NAME` | Stop putting that change back after updates. |
| `sudo hearth-apt-layer reset` | Start this version's layer empty at the next restart. Your changes are then put back fresh. |
| Boot option `hearth.apt-layer=0` | One start without the layer (`/usr` as the image has it). For when a change broke something. Your files aren't touched. |

Rolling back (the previous version in the boot menu) gives you that version with its own layer, as it was.

## How it works

```
/usr  (what you see)                                   overlayfs, mounted early at boot
 ├── upper: /var/lib/hearth/layer/usr/<deployment>/upper   your packages' files, removals, dpkg's changes
 └── lower: the image's /usr (read-only, composefs)        the system
```

- **One layer per deployment.** `hearth-apt-layer.service` mounts an overlay on `/usr`. It runs before
  `sysinit.target`, so before the display manager and almost everything else. The overlay's upper directory is
  in `/var`, which is the machine's and survives restarts and updates. It's keyed by the booted deployment's
  ostree commit (from the `ostree=` boot option, with `ostree admin status` as a fallback). bootc makes one
  commit per image, so a new image means a new, empty layer. Files from one image never leak into the next, and
  nothing in the layer can shadow a newer system file. Removals are overlay "whiteouts" in the upper directory,
  so they don't leak either. Layers of deployments that are gone (not the booted, rollback or staged one) are
  deleted.
- **dpkg's database moves with it.** The image keeps dpkg's database in `/usr/lib/sysimage/dpkg`, and
  `/var/lib/dpkg` links to it. So the database is inside the overlay: apt sees the image's packages with your
  changes, and a new image brings its own correct database. apt's auto/manual marks
  (`/var/lib/apt/extended_states`) stay in `/var`.
- **Your changes** live in `/var/lib/hearth/layer/state.json`. dpkg runs `hearth-apt-layer record` after every
  change (`/etc/dpkg/dpkg.cfg.d/hearth-apt-layer`, so plain `dpkg -i` counts too). It compares what's installed
  with the image's list (`/usr/share/hearth/apt-layer/image-packages`) and records three things:
  - **Installed:** not in the image, and not just pulled in as a dependency.
  - **Removed:** image packages that aren't installed.
  - **Upgraded:** image packages at another version.

  A change that's undone (you remove what you installed, reinstall what you removed, or go back to the image's
  version) comes off the list. An entry that this deployment hasn't shown yet (it's still being put back, or it
  was skipped) stays. `.deb` files installed from a file (anything apt didn't download into
  `/var/cache/apt/archives`) are copied to `/var/lib/hearth/layer/debs/`.
- **Putting changes back.** After an update, the mount finds an empty layer and a non-empty list, and writes
  `pending`. `hearth-apt-layer-sync.service` runs after the network is online. It's `Type=exec`, so the login
  doesn't wait. It runs `apt-get update`, then works in this order:
  1. **Removals.** It simulates `apt-get -s remove`. If that would remove anything besides your list, the new
     image needs the package: it's skipped and reported.
  2. **Upgrades.** `apt-get install --only-upgrade`, but only where the new image's version is older than
     yours. Where it isn't, the entry is dropped ("caught up").
  3. **Installs.** `apt-get install --no-remove --force-confold`, using kept `.deb` files when the software
     sources don't have the package, or only an older version, or there's no internet.

  Each step tries everything at once, then one at a time if that fails. Failures are skipped with apt's reason.
  The result goes to `/var/lib/hearth/layer/status.json`, which shows up in these places:
  - Desktop Mode's Settings › Updates, with Try again.
  - Game Mode's Settings › Updates, plus a notification in its list.
  - A desktop notification to the signed-in session.
  - The journal (`journalctl -u hearth-apt-layer-sync`).

  Without the internet, the repository part waits for the next start or Try again.
- **The guard rails** only act while the layer is mounted. A refusal stops apt before dpkg runs, so nothing
  changes.
  - `hearth-apt-layer-debs` uses apt's `DPkg::Pre-Install-Pkgs`, protocol 1, which lists every `.deb` about to
    be unpacked. It refuses kernel, initramfs and boot loader packages. Protocol 1 is used because protocols 2
    and 3 leave reinstalls out (checked with apt 2.8).
  - `hearth-apt-layer apt-hook` uses protocol 3, the whole plan. It refuses removing the kernel and boot
    packages and the protected list.
  - The image's kernel and boot packages are pinned at the image's versions with priority 1001
    (`/etc/apt/preferences.d/zz-hearth-image`, a link into the image). So `apt upgrade` passes them over.
  - `update-initramfs` is a no-op in the image. A package's trigger can't try to rebuild the initramfs.
- **Things systemd and udev read early.** systemd loads its units before the layer is mounted. The sync service
  runs `daemon-reload` and starts the units your packages added that you enabled. udev rules are reloaded at
  mount if your packages have any. `/etc/ld.so.cache` is rebuilt at every mount: it lives in `/etc`, which keeps
  a changed file through updates.
- **`/opt` and `/usr/local`.** These are links into `/var` (`/var/opt`, `/var/usrlocal`) in the bootc layout.
  They're writable, and packages that install there work. Their files stay through updates on their own. Putting
  such a package back re-registers it with dpkg (dpkg overwrites the files it finds).
- **Opening a `.deb`.** `hearth-install-deb.desktop` is the default for `application/vnd.debian.binary-package`
  in `/usr/share/applications/mimeapps.list`. It runs `hearth-deb-open`, which opens a terminal running
  `pkexec hearth-admin install-deb FILE`. Hearth's polkit rule lets the person at the device run `hearth-admin`
  without a password, **except `install-deb`, which always asks** (`AUTH_ADMIN`). `install-deb` reads the file
  as the person who asked (so only files they can read), copies it into a root-only folder, checks it's a valid
  package, and installs it through apt like the terminal would. Game Mode has no file browser that opens `.deb`
  files.
- **The app menu.** `/var/lib/hearth/layer/added-apps` lists the desktop files that your installed packages put
  in `/usr/share/applications`, one per line, readable by everyone. It's rewritten after every change and after
  a put-back. Desktop Mode's app menu uses it to show them as your apps, not system apps.

## Files

| Path | What |
|---|---|
| `os-bases/apt-layer/` | `system_files/` (copied into the image), `image-setup.sh` (run at the end of build.sh), `ci-check.sh` (CI). Used by Peppermint, and wired into the archived Ubuntu edition. |
| `/usr/libexec/nimbus/hearth-apt-layer` (`/usr/bin/hearth-apt-layer`) | Everything above. |
| `hearth-apt-layer.service`, `hearth-apt-layer-sync.service` | Mount early; put back after updates. |
| `/var/lib/hearth/layer/` | `usr/<deployment>/{upper,work}`, `state.json`, `status.json`, `pending`, `debs/`, `added-apps`. |
| `/usr/share/hearth/apt-layer/` | `image-packages`, `protected`, `image-pins.pref` (made at build time). |
| `/run/hearth/apt-layer/active.json` | The mounted layer. The hooks only act when it exists. |

## What's verified, and what isn't

- **Unit tests** (`hosts/linux/test/test_hearth_apt_layer.py`) cover these parts:
  - recording installs, removals and upgrades, and dropping undone ones
  - apt's hook formats (captured from apt 2.8)
  - the guard rails
  - the put-back plans: removals with "the image needs it", upgrades with "caught up", installs with kept
    `.deb`s
  - the deployment key, the pins and `added-apps`
- **Locally against real apt and dpkg** (Ubuntu 24.04, a local repository, without the overlay), across
  simulated new deployments:
  - an image package upgraded, removed and protected
  - removal and upgrade put back
  - "the new system needs it" skipped
  - "caught up" dropped
- **CI** (`os-bases/apt-layer/ci-check.sh` in base-peppermint.yml) runs inside the built image, in a privileged
  container:
  - the overlay mounts on the image's `/usr`
  - `apt install hello` lands in the layer
  - removing `nano` works and is recorded, and removing `python3` is refused
  - a boot package is refused, and `full-upgrade` runs and passes the kernel over
  - a `.deb` goes in through `hearth-admin install-deb`, and a package installs into `/opt`
  - `/opt` and `/usr/local` are writable
  - a simulated new deployment starts empty, gets `hello` back, removes `nano` again, and skips a
    no-longer-fitting `.deb` with a report

  Upgrading an image package is only exercised in CI when the archive has a newer version than the image at
  that moment.
- **Not verified without a booted machine:**
  - systemd ordering at boot
  - the key from a real `ostree=` boot option on composefs
  - a real `bootc upgrade` followed by the put-back
  - notifications
  - the polkit password prompt and the double-click
  - services from added packages starting after `daemon-reload`

## Known limits

- **Changes outside `/usr` stay.** A package whose maintainer scripts change things outside `/usr` (accounts,
  `/etc`, `/var`, `/opt`) keeps those changes through updates, as on any Debian system. Configuration files in
  `/etc` are kept (`--force-confold` when putting a package back).
- **`/opt` and `/usr/local` are shared** by every deployment. Removing a package that installed into `/opt` on
  one version also removes its files for the rollback version.
- **`dpkg -i` files aren't kept.** Packages installed with `dpkg -i` directly are recorded, but the file isn't
  kept. They come back after updates only if your software sources have them.
- **`dpkg` run directly bypasses the guard rails** (not through apt), like any `--force` option would. Whatever
  it breaks is gone with the next system update (a new, empty layer), or at once with `hearth-apt-layer reset`
  and a restart, or by booting with `hearth.apt-layer=0`.
- **DKMS kernel modules** built by added packages aren't in the initramfs, and need headers that match the
  image's kernel.
