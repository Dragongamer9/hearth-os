# Apps you add with apt (Ubuntu and Peppermint editions)

On Hearth OS's Ubuntu and Peppermint editions (Beta Experimental) the system is an image: it's read-only and
updates in one piece (bootc). The system is "partly unlocked" on top of that. You can install apps with
`sudo apt install` or by opening a `.deb` file, and they stay:

- **After a restart:** they're still there.
- **After a system update:** Hearth puts them back on the new version by itself, in the background, a minute or
  so after you sign in. A notification tells you how it went.
- **When one doesn't fit the new version:** for example, it needs an older library than the new system has.
  That package is skipped and you're told why (a notification, Settings › Updates, the journal). It never stops
  the update or the boot.

Flatpak (the Apps store) stays the recommended way to get apps. The apt layer is for what isn't on Flathub:
terminal tools, drivers' user-space parts, apps published as `.deb` files.

## Using it

| You do | What happens |
|---|---|
| `sudo apt update`, then `sudo apt install vlc` | Installed into your added apps. apt prints "Hearth: installing into your added apps". |
| Open a `.deb` file (Hearth Files, or any file manager) | A terminal opens, asks for your password, installs it and what it needs. A copy of the file is kept so it can come back after updates without the internet. |
| `sudo apt install ./discord.deb` | The same, from the terminal. |
| `sudo apt remove vlc` | Removed, and taken off the list (it won't come back after updates). |
| `sudo apt upgrade` / `full-upgrade` | Upgrades **only the packages you added**. The system's own packages are held at the image's versions. They update with Hearth's system updates (Settings › Updates). |
| `sudo apt remove nano` (part of the system) | Refused: "nano is part of Hearth OS and can't be removed". Nothing changes. |
| `sudo apt install linux-image-…`, `grub-…`, `dracut-…` | Refused: the kernel, initramfs and boot loader come with the image. Installing them here couldn't take effect. |
| `hearth-apt-layer status` | Your added packages, and what happened after the last update. |
| `sudo hearth-apt-layer retry` | Try the skipped ones again (also a button in Desktop Mode's Settings › Updates). |
| `sudo hearth-apt-layer forget NAME` | Stop trying to put a package back. |
| `sudo hearth-apt-layer reset` | Start this version's layer empty at the next restart. Everything on your list is then put back fresh. |
| Boot option `hearth.apt-layer=0` | One start without the layer (`/usr` read-only, as before). For when something you added breaks the system. Your files aren't touched. |

You can also boot the previous system version from the boot menu. Each version has its own layer, so rolling
back gives you that version as it was.

## How it works

```
/usr  (what you see)                                   overlayfs, mounted early at boot
 ├── upper: /var/lib/hearth/layer/usr/<deployment>/upper   your packages' files, dpkg's changes
 └── lower: the image's /usr (read-only, composefs)        the system
```

- **One layer per deployment.** `hearth-apt-layer.service` (before `sysinit.target`, so before the display
  manager and almost everything else) mounts an overlay on `/usr`. Its upper directory is in `/var`, which is
  the machine's and survives restarts and updates. It's keyed by the booted deployment's ostree commit (from the
  `ostree=` boot option; `ostree admin status` as a fallback). bootc makes one commit per image, so a new image
  means a new, **empty** layer. Files from one image never leak into the next, and nothing you installed can
  shadow a newer system file. Layers of deployments that are gone (not the booted, rollback or staged one) are
  deleted.
- **dpkg's database moves with it.** These images keep dpkg's database in `/usr/lib/sysimage/dpkg` (with
  `/var/lib/dpkg` linking to it; see the build's bootc layout). So it's inside the overlay. apt sees exactly the
  image's packages plus yours, and a new image brings its own, correct database. Nothing has to be rebuilt or
  merged. apt's auto/manual marks (`/var/lib/apt/extended_states`) stay in `/var`.
- **The list** (`/var/lib/hearth/layer/state.json`). dpkg runs `hearth-apt-layer record` after every change
  (`/etc/dpkg/dpkg.cfg.d/hearth-apt-layer`, so plain `dpkg -i` counts too). Your packages are the ones installed
  now that aren't in the image (`/usr/share/hearth/apt-layer/image-packages`) and weren't pulled in as a
  dependency. Something that was installed in this deployment and now isn't has been removed, so it comes off
  the list. Something on the list that has never been installed in this deployment stays (it's on its way back,
  or it was skipped). `.deb` files installed from a file (anything apt didn't download into
  `/var/cache/apt/archives`) are copied to `/var/lib/hearth/layer/debs/`.
- **Added apps.** `/var/lib/hearth/layer/added-apps` lists the desktop files your packages put in
  `/usr/share/applications`, one per line, readable by everyone. It's rewritten by the same hook and after a
  put-back. Desktop Mode's app menu uses it to file them as your apps, not system apps.
- **Putting them back.** After an update, the mount finds an empty layer and a non-empty list, and writes
  `pending`. `hearth-apt-layer-sync.service` runs after the network is online. It's `Type=exec`, so the login
  doesn't wait. It runs `apt-get update`, then installs the list in one go with
  `apt-get install --no-remove --force-confold`. A kept `.deb` is used when the software sources don't have the
  package, or only an older version, or there's no internet. If installing everything at once fails, each
  package is tried on its own. The ones that still fail are skipped with apt's reason. The result is in
  `/var/lib/hearth/layer/status.json`. Desktop Mode's Settings › Updates shows it (with Try again), Game Mode's
  Settings › Updates shows it (plus a notification in its list), and a desktop notification goes to the signed-in
  session. Without the internet, repo packages wait for the next start or Try again.
- **The guard rails** only act while the layer is mounted:
  - Every system package is pinned at the image's version with priority 1001
    (`/etc/apt/preferences.d/zz-hearth-image`, a link into the image). So apt never picks another version of
    one: `upgrade`/`full-upgrade` only touch your packages, and a package that needs a newer system library
    fails cleanly with "Depends: … but … is to be installed".
  - `hearth-apt-layer-debs` (apt's `DPkg::Pre-Install-Pkgs`, protocol 1: every `.deb` about to be unpacked)
    refuses upgrading, downgrading or reinstalling a system package, and kernel/initramfs/boot loader packages.
    Protocol 1 is used because apt's protocols 2 and 3 leave reinstalls out (checked with apt 2.8).
  - `hearth-apt-layer apt-hook` (protocol 3: the whole plan) refuses removing a system package.
  - A refusal stops apt before dpkg runs: nothing changes.
  - `update-initramfs` is a no-op in the image. A package's trigger can't try to rebuild the initramfs.
- **Things systemd and udev read early.** systemd loads units before the layer is mounted, so units your
  packages added and you enabled are started by the sync service (after a `daemon-reload`). udev rules are
  reloaded at mount if your packages have any. `/etc/ld.so.cache` is rebuilt at every mount. It lives in `/etc`,
  which keeps a changed file through updates, so without the rebuild it could point at libraries the new image
  no longer has.
- **Opening a `.deb`.** `hearth-install-deb.desktop` (the default for `application/vnd.debian.binary-package`
  in `/usr/share/applications/mimeapps.list`) runs `hearth-deb-open`. That opens a terminal running
  `pkexec hearth-admin install-deb FILE`. Hearth's polkit rule lets the person at the device run `hearth-admin`
  without a password, **except `install-deb`, which always asks** (`AUTH_ADMIN`). `install-deb` reads the file
  as the person who asked (so only files they can read), copies it into a root-only folder, checks it is a valid
  package, and installs it through apt like the terminal would. Game Mode has no file browser that opens `.deb`
  files.

## Files

| Path | What |
|---|---|
| `os-bases/apt-layer/` | Shared by both editions: `system_files/` (copied into the image), `image-setup.sh` (run at the end of each build.sh), `ci-check.sh` (CI). |
| `/usr/libexec/nimbus/hearth-apt-layer` (`/usr/bin/hearth-apt-layer`) | Everything above. |
| `hearth-apt-layer.service`, `hearth-apt-layer-sync.service` | Mount early; put back after updates. |
| `/var/lib/hearth/layer/` | `usr/<deployment>/{upper,work}`, `state.json`, `status.json`, `pending`, `debs/`, `added-apps`. |
| `/run/hearth/apt-layer/active.json` | The mounted layer (the hooks only act when it exists). |

## What's verified, and what isn't

- **Unit tests** (`hosts/linux/test/test_hearth_apt_layer.py`) cover the list logic, apt's hook formats
  (captured from apt 2.8), the guard rails, the put-back plan, the deployment key, the pins and `added-apps`.
- **Locally against real apt and dpkg** (Ubuntu 24.04, a local repository, without the overlay): recording,
  removing, refusing removals, reinstalls and upgrades of "system" packages, pins holding `full-upgrade`, a local
  `.deb` kept and put back, a package that no longer fits skipped while the others come back, retry and forget.
- **CI** (`os-bases/apt-layer/ci-check.sh`, in base-ubuntu.yml and base-peppermint.yml) runs in each built
  image inside a privileged container: the overlay mounts on the image's `/usr`, `apt install hello` works and
  lands in the layer, the guard rails refuse, a `.deb` goes in through `hearth-admin install-deb`, and a
  simulated new deployment starts empty and gets `hello` back while a no-longer-fitting `.deb` is skipped and
  reported.
- **Not verified without a booted machine:** the systemd ordering at boot; the key from a real `ostree=` boot
  option on composefs; a real `bootc upgrade` followed by the put-back; notifications; the polkit password
  prompt and the double-click; services from added packages starting after `daemon-reload`.

## Known limits

- A package whose maintainer scripts change things outside `/usr` (accounts, `/etc`, `/var`) keeps those
  changes through updates, as on any Debian system. Its configuration files in `/etc` are kept (`--force-confold`
  when putting it back).
- `/etc/alternatives` links changed by your packages are the machine's. Putting the package back registers
  them again.
- Packages installed with `dpkg -i` directly are recorded, but the file isn't kept. They come back after updates
  only if your software sources have them.
- `dpkg` run directly (not through apt) bypasses the guard rails, like any `--force` option would. Whatever it
  breaks is gone with the next system update (a new, empty layer), or at once with `hearth-apt-layer reset`
  and a restart.
- Kernel modules (DKMS) built by added packages aren't in the initramfs and need headers that match the image's
  kernel.
