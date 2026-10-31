# Hearth OS on Peppermint OS (Beta Experimental)

**Status: Beta Experimental.** The image is built and checked by CI. It hasn't been booted on real hardware yet.
It isn't the main Hearth. It's published as `ghcr.io/dragongamer9/hearth-peppermint:testing` and never as
`:latest`.

The Peppermint build works like the other Hearth images (Bazzite in `os/`, CachyOS and Ubuntu in `os-bases/`):

- It's one bootc image.
- It updates as a whole and rolls back as a whole.
- Hearth's installer installs it.
- It boots into the Hearth session: Game Mode (gamescope + the Hearth app) and Desktop Mode (labwc + Quickshell).

Unlike Anatase, **Peppermint is a normal writable system out of the box**: `sudo apt install` and `.deb` files
install anywhere (into `/usr` too), kept across restarts, and what you add comes back by itself after an update.
bootc stays: updates, rollback, the automatic fallback and Recovery work as before. It can be locked in Settings.
Anatase is the locked, immutable edition. See "Writable system" below.

| Piece | Where |
|---|---|
| Image recipe | `os-bases/peppermint/Containerfile` |
| Builder stage: ostree, bootc, bootupd, Quickshell, RyzenAdj, Handheld Daemon | `os-bases/peppermint/builder.sh` |
| Image build (packages, Hearth, boot pieces, bootc layout) | `os-bases/peppermint/build.sh` |
| Debian-specific system files, layered over `os/system_files` | `os-bases/peppermint/system_files/` |
| CI: build, check, publish `:testing` | `.github/workflows/base-peppermint.yml` |

## What Peppermint contributes, and what it doesn't

Peppermint OS today (October 2026) has three editions:

- **Debian edition.** The flagship. It was re-based on **Debian 13 "trixie"** in October 2025, and refreshed ISOs
  are moving to the XLibre X server.
- **Devuan edition ("Excalibur", June 2026).** The same system without systemd. You choose SysVinit, OpenRC or
  runit.
- **PepVoid.** Based on Void Linux. Still in testing.

On top of Debian, the Peppermint Debian edition adds:

- **No package repository of its own.** The maintainer says trixie and Excalibur use the upstream Debian and
  Devuan repositories only. There are no extra Peppermint repos, and no Peppermint kernel: it's Debian's.
- **A curated live ISO.** It includes Xfce, LibreWolf, Calamares, a few defaults and the Peppermint look. The ISO
  used to be built with live-build. Newer builds use mmdebstrap and Peppermint's own spin tools.
- **A handful of Python tools that aren't packaged.** These are Welcome, Hub (settings), Kumo (site-specific
  browsers) and an apt-based updater. They're copied from git
  ([codeberg.org/Peppermint_OS/PepProTools](https://codeberg.org/Peppermint_OS/PepProTools), last commit June
  2023; it needs PyQt5/WebEngine from pip). The 2026 plan is to develop newer Tkinter versions on PepVoid first.

**The honest summary:** once Hearth replaces the desktop and the session, Peppermint contributes very little
beyond plain Debian trixie. Its distinguishing parts are a desktop, an installer, an updater and some tools.
Hearth replaces every one of those:

| Peppermint part | In this image | Why |
|---|---|---|
| Debian 13 trixie base + Debian repos (main, contrib, non-free, non-free-firmware) | **Yes**: this is the base | It *is* Peppermint's base. |
| Debian kernel | **Yes, but from trixie-backports** | Handhelds need newer kernels than trixie's 6.12. Peppermint has no kernel of its own. |
| Xfce desktop | **Optional, on by default** (`PEPPERMINT_DESKTOP=1`) as a second login-screen session, "Peppermint desktop (Xfce)" | It's the one recognisably Peppermint thing, and a plain fallback desktop. It costs roughly 150 MB, about 2% of the image; the X server is there anyway for SDDM's login screen. Hearth's session stays the default and the autologin. Build with `--build-arg PEPPERMINT_DESKTOP=0` to drop it. |
| XLibre X server | No | Hearth is Wayland (gamescope, labwc). XLibre isn't in Debian's repositories, and Xfce runs on Debian's X.Org. |
| Calamares installer | No | Hearth's own installer deploys the image (`ostree container image deploy` + bootupd). |
| Peppermint updater (apt) | No | Hearth updates the system with `bootc upgrade` (a whole new version). apt still works for your own packages (see "Writable system"), and they come back after each update. |
| Welcome / Hub / Kumo | No | They're unpackaged, unmaintained since 2023, and need pip-installed PyQt5/WebEngine. Hub and the updater drive apt and Xfce settings, which don't apply here. Hearth Settings covers the same ground. |
| Devuan edition | **Can't be used** | bootc and ostree need systemd. |

Where Peppermint shows up:

- The name: `PRETTY_NAME="Hearth OS (Peppermint edition, Debian 13)"`, `VARIANT_ID=hearth-peppermint`. `ID`
  stays `debian`, so tools still know what's underneath.
- The optional Xfce session.
- `/usr/share/nimbus/image-info.json` (`"base": "peppermint"`).

If Peppermint later publishes a package repository (for example its PepVoid tools ported to Debian), `build.sh`
is where it would be added.

## How it's built

### The bootc approach

The image starts from the official `debian:trixie` image, the way bootcrew's `debian-bootc` does. That repo is now
`bootcrew/mono`; it uses Debian unstable, and `linuxsnow/debian-bootc-core` does Debian stable. On top of that:

1. **ostree 2026.4 is built from source** (`builder.sh`). Trixie ships ostree 2025.2, but bootc 1.17 needs
   libostree 2025.3 or newer. It's installed into the same library directory as Debian's package would be.
   Placeholder packages (`libostree-1-1`, `ostree`, `ostree-boot`, version `2026.4-0hearth1`, built with equivs
   and held) tell dpkg it's there, because Debian's flatpak depends on `libostree-1-1`. Frostyard/Snow Linux
   does the same on Debian stable.
2. **bootc 1.17.1** is built without SELinux (Debian doesn't use it). **bootupd 0.3.2** is built too; it's what
   Hearth's installer uses to put the boot loader on the EFI partition.
3. **The bootc filesystem layout** comes from bootcrew:
   - `/var` is emptied into tmpfiles.d entries.
   - The home folder is `/var/home`.
   - `/opt` moves to `/usr/lib/opt`.
   - The `/ostree`, `/root`, `/srv`, `/mnt` and `/usr/local` links are set up.
   - composefs is set to `maybe`, and the root is read-only.
4. **dpkg's database moves to `/usr/lib/sysimage/dpkg`**, linked back from `/var/lib/dpkg` at boot. The list of
   installed packages then updates with the image (this matches what the CachyOS build does with pacman's
   database).
5. **System accounts that later images add** (sddm, daemons) are listed in `/usr/lib/sysusers.d/hearth-image.conf`.
   `systemd-sysusers` runs every boot, because its update condition never fires on ostree. The machine's
   `/etc/passwd` is kept across updates, so without this a new image's accounts would never appear.
6. **The initramfs** is built with dracut: generic (boots any PC), with ostree's prepare-root, bootc's module and
   Plymouth (Hearth's theme). nouveau and Nvidia's modules are left out of it.

### The boot loader: Debian's signed shim and GRUB

Secure Boot works like on Debian: the chain goes Microsoft → Debian's shim → Debian's GRUB → Debian's signed
backports kernel. `build.sh` lays the files out the way bootupd expects:

- `/usr/lib/efi/shim/<version>/EFI/{debian,BOOT}`
- `/usr/lib/efi/grub2/<version>/EFI/debian`

Then it runs `bootupctl backend generate-update-metadata`. The folder has to be `EFI/debian`: Debian's signed
GRUB has that folder built in as where it finds `grub.cfg`.

Debian's GRUB has no `blscfg` command. Fedora's GRUB reads ostree's boot entries with it. So
`/usr/libexec/nimbus/hearth-bls-grub` writes the menu (`/boot/grub2/hearth-bls.cfg`) from
`/boot/loader/entries`, and bootupd's static `grub.cfg` sources it. This is the same approach and interface as
the Ubuntu image. The menu is rewritten at these points:

- at boot
- when ostree changes the entries (`hearth-bls-grub-refresh.path`: rollback, kargs)
- at shutdown, right after a staged update is finalized (`hearth-bls-grub.service`)

The installer also has to run it once after deploying (see "Changes needed outside this folder" below).

### What's installed

| Area | What | From |
|---|---|---|
| Kernel | `linux-image-amd64` + headers | **trixie-backports** (signed). Newer than 6.12 for the ROG Ally/Ally X, Legion Go/Go S, Steam Deck OLED, MSI Claw. |
| Firmware | amd/intel/nvidia graphics, iwlwifi, realtek, atheros, mediatek, brcm, SOF + Intel sound, Cirrus amps (ROG Ally speakers), microcode | non-free-firmware, backports where newer |
| Graphics | Mesa (Vulkan/GL/VA) with its i386 half | trixie-backports |
| Nvidia | Open kernel module (DKMS, built for the image's kernel) + driver, in the **same image** | **NVIDIA's own Debian 13 repository**: Debian's 550 driver doesn't build against the backports kernels. The i386 libraries are tried but not required. |
| Steam | `steam-installer` + `steam-devices`, i386 multiarch, licence pre-accepted, `/usr/bin/steam` link | contrib |
| Game Mode | `gamescope` (Valve's upstream; it has `GAMESCOPECTRL_BASELAYER_WINDOW/APPID`, `GAMESCOPE_FPS_LIMIT`, `GAMESCOPECTRL_REQUEST_SCREENSHOT`, `--mangoapp`; CI checks each one), `mangoapp`, `mangohud`, `gamemode` | trixie-backports (contrib, 3.16.22+) |
| Handhelds | **Handheld Daemon 4.1.12**: its own venv at `/usr/lib/hhd`, with Debian's python3-evdev/yaml/rich/xlib/serial/pyroute2/gi/dbus; udev rules, `hhd.service` (system-wide, on from the first boot). Also `acpi_call` (Legion Go) and RyzenAdj. | Built from the hhd-dev/hhd tag |
| Power | `power-profiles-daemon`, only started when HHD is turned off (`/etc/hearth/no-hhd`). HHD provides the same D-Bus service with the device's real limits, and the two can't both run. | main |
| Desktop Mode | labwc (backports), Quickshell (built), grim, slurp, brightnessctl, wlr-randr, wlsunset, xdotool, gnome-keyring, portals (wlr + gtk), foot, polkit agent, QML modules | main / builder |
| App runtime | `libwebkit2gtk-4.1-0`, GStreamer, the Electron libraries Hearth Browser needs | main |
| System | systemd, NetworkManager, BlueZ, PipeWire + WirePlumber, Flatpak (+ Flathub), udisks2, upower, fwupd, fonts, Plymouth, SDDM | main |
| Sign-in | SDDM + `nimbus-autologin.service` (the same as the Bazzite build: `os/system_files`), the Hearth SDDM theme (Qt 6, which trixie's SDDM is) | |
| Updates | `bootc-fetch-apply-updates.timer`, which only stages the update (never reboots mid-game) and updates system Flatpaks. "Update now" goes through `hearth-admin` → `bootc upgrade`. | |

The Hearth app comes from `hosts/hearth-rs`. CI builds it **inside a `debian:trixie` container**, not on the
Ubuntu runner. A runner build would link fine today (same sonames). But `ubuntu-latest` moves to newer Ubuntu
releases, whose glibc trixie (2.41) doesn't have. Hearth Browser (Electron) is built on the runner, as in
`os.yml`. `build.sh` and CI fail the build if `ldd` finds a missing library.

## Writable system (hearth-unlock)

Owner's decision (October 2026): Peppermint is writable out of the box, without removing bootc. Anatase is the
immutable edition.

| Piece | Where |
|---|---|
| The tool (unlock, lock, status, reapply, the hooks) | `os-bases/peppermint/system_files/usr/bin/hearth-unlock` |
| At every start: this version's layer over `/usr` | `hearth-unlock.service` (before `sysinit.target`, never fails the start) |
| Services, device rules and kernel modules the layer brought | `hearth-unlock-units.service` |
| A start with your changes counts as good once Hearth is up (45 s) | `hearth-unlock-good.service` |
| After an update: your packages put back; old layers tidied | `hearth-unlock-reapply.service` (after the network) |
| apt's and dpkg's hooks, the boot-chain pin | `system_files/usr/share/hearth/unlock/` → `/etc/apt/apt.conf.d/50-hearth-unlock`, `/etc/dpkg/dpkg.cfg.d/hearth-unlock`, `/etc/apt/preferences.d/hearth-boot-chain` (installed at the end of `build.sh`, after the last apt run) |
| Settings: Game Mode › Updates & Recovery › Writable system; Desktop Settings › Updates | `ui/index.html` (`unlockRows`), `hosts/linux/desktop/shell/SetUpdates.qml`, through `hearth-admin unlock …` |
| `/etc` that points into the layer, on starts without it | `/usr/lib/systemd/system-generators/hearth-etc-guard` |
| The boot writers, the update's finalizing and Recovery from the image's `/usr` | `*.service.d/20-hearth-image-usr.conf`, `hearth-unlock-ldcache.service` |
| Recovery: "Restart without your system changes" | `os/system_files/usr/libexec/nimbus/hearth-recovery` |
| `.deb` files on the system itself (Install Package window) | `hearth-debbox` → `hearth-admin deb-install` → `hearth-unlock --install-deb` (always asks for an administrator's password) |
| Tests | `hosts/linux/test/test_hearth_unlock.py`; real overlay + apt: `.github/scripts/hearth-unlock-check.sh` (boot-guard.yml on Debian 13, base-peppermint.yml in the built image) |

### How it works

**Why not `ostree admin unlock --hotfix` or `bootc usroverlay`.** `bootc usroverlay` (and plain `ostree admin
unlock`) is transient: gone at the next restart. `--hotfix` keeps changes, but it clones the running deployment as
the rollback, which pushes the real previous version out (bootc keeps two), and its overlay is set up by
`ostree-prepare-root` in the initramfs, where a mistake means a device that doesn't start. So Hearth keeps its own
persistent overlay, mounted by an ordinary service after the root is up:

- **One layer per OS version.** The layer is named after the running deployment's ostree commit
  (`/var/lib/hearth/unlock/layers/<commit>/upper`, read from the `ostree=` boot argument, else `ostree admin status`)
  and mounted early at every start as `overlay hearth-usr` over the read-only `/usr` (composefs underneath: two
  overlay levels, which the kernel allows). If anything goes wrong there, the start carries on read-only.
- **dpkg and apt stay consistent with the files.** dpkg's database is already in `/usr/lib/sysimage/dpkg`; apt's
  auto/manual marks move to `/usr/lib/sysimage/apt/extended_states` (`Dir::State::extended_states`). Both are in
  `/usr`, so they live in the same layer as the files they describe. apt's lists and downloads stay in `/var`.
- **`/etc` and `/var` as on any bootc system** (see "What is shared between versions" below). `/etc` is the machine's own and merged into each new version
  (repositories you add, config files, services you enable are kept), `/var` is shared by every version. At every
  start `ldconfig` refreshes `/etc/ld.so.cache` for the `/usr` that start has.
- **The record.** After every dpkg run (dpkg's `post-invoke` hook, so `dpkg -i` too), once apt and dpkg let go of
  their locks, `/var/lib/hearth/unlock/record.json` follows: packages you asked for beyond the image (apt's manual
  marks minus `/usr/share/hearth/apt-baseline.list`), image packages you removed, and a copy of every `.deb`
  installed from a file (apt's `DPkg::Pre-Install-Pkgs` hook → `/var/lib/hearth/unlock/debs/`). It's built from
  changes, so installing something on a new version before the reapply has run doesn't drop the rest. A plain
  `dpkg -i` isn't seen by apt's hook: that package is recorded but its file isn't kept.
- **The boot chain is the image's.** GRUB, shim, the kernel, ostree, bootc, bootupd and dracut are pinned out of
  apt (priority -1): their maintainer scripts would write the boot menu or the EFI partition outside bootc. A
  kernel installed with apt would never be booted anyway (ostree boots the version's own kernel).

### On an update

1. `bootc upgrade` stages the new version as always (the layer doesn't take part).
2. The first start of the new version is the **clean new image** (its commit has no layer yet), with your `/etc`
   and `/var`. Hearth comes up as usual.
3. `hearth-unlock-reapply.service` (after the network, in the background) runs `apt-get update`, removes image
   packages you removed (only when apt would take nothing else with them), and installs your packages and saved
   `.deb` files (all at once; if that fails, one at a time). Existing config files are kept
   (`--force-confold`). Offline: tried again at the next start. A package that can't come back: Hearth shows a
   notice ("Some of your packages didn't come back") and `/var/lib/hearth/unlock/reapply.log` says why. Nothing
   here ever blocks a start.
4. The previous version keeps its own layer of `/usr`, untouched by the new version. Layers of versions ostree no
   longer has are removed (only when `ostree admin status` gives a sensible answer that names the running version).

**What is shared between versions** (not layered, so the previous version sees the current state, not the state it
had): `/var` as a whole, which includes `/opt` (`/var/opt`: a `.deb` that installs into `/opt` writes there),
`/usr/local` (`/var/usrlocal`), debconf's answers (`/var/cache/debconf`), apt's lists and downloads, and `/boot`.
`/etc` is per version (ostree copies and merges it at each update), but within one version it's the same on every
start, with or without the layer.

### On a bad update, or a bad change

- **The update itself doesn't start:** nothing new — the automatic fallback starts the previous version (with its
  own layer) after two failed starts, and Recovery's Go back works.
- **A package you added stops Hearth from starting:** when a start with a layer that holds changes never reached
  Hearth (Game Mode, Desktop Mode or the login screen for 45 s), the next start leaves the layer off (**safe
  start**) and Hearth says so, with **Turn my changes back on**. This comes before the automatic fallback (start
  2 vs start 3), so a bad package never costs you the new version. If the clean start fails too, the fallback goes
  back to the previous version as before.
- **By hand:** Hearth Recovery › Restart without your system changes; or the kernel option `hearth.clean` (one start
  without the layer); or Lock (the image as it was made).
- An empty layer never triggers a safe start, so people who don't use apt never see one; a mount that fails doesn't
  count towards one either.
- **A safe start (and `hearth.clean`, Lock, the first start of a new version) doesn't undo `/etc`.** Packages change
  `/etc` too (services they enable, the display manager, config files). So that those settings can't stop Hearth
  from starting without the layer, the `hearth-etc-guard` generator (in the image's `/usr`, run before any unit is
  loaded) lifts them for that start, in `/run` only: a `display-manager.service` or `default.target` pointing at a
  unit that isn't there falls back to SDDM / `graphical.target` (and SDDM's `/etc/X11/default-display-manager` check
  is lifted), a unit in `/etc` that replaces one of the image's and runs a missing program falls back to the
  image's, and a drop-in that runs a missing program is left out. debconf's `shared/default-x-display-manager` is
  SDDM in the image, so installing gdm3/lightdm/xdm doesn't take the display over in the first place.
- **The device turns off during an install** (a dpkg cut off half way: its journal isn't empty, or a package is half
  installed or configured in the layer): that start goes without the layer, with the notice "Finishing an
  interrupted install"; once Hearth is starting, the layer is mounted and `dpkg --configure -a` and `apt-get -f
  install` finish the job; the next start has it again. If that fails, starts stay without it until it's fixed
  (`sudo dpkg --configure -a`). To make this rare, the reapply service lets the running apt step finish at shutdown
  (ExecStop, up to 5 minutes) and starts no new one.
- **What must never run from the layer runs from the image.** Before the layer goes over `/usr`, the image's `/usr`
  is kept read-only at `/run/hearth/usr-image` (a private mount, so the layer doesn't reach it). The boot-menu
  writers (`hearth-bls-grub` and its refresh, `hearth-boot-guard`, its refresh and `hearth-boot-good`),
  `ostree-finalize-staged`, `bootc-fetch-apply-updates` and `hearth-recovery` get it as their `/usr`
  (`BindReadOnlyPaths=-/run/hearth/usr-image:/usr` drop-ins in the image), so a package that breaks `sh`, `sed`,
  Python or a library can't stop the boot menu or an update from being written. Updates started from Settings (Update
  now, through `hearth-admin`) still run from the layer: a failure there only means the update isn't staged.
- **apt can't reach `/boot` or swap the system's core.** `update-initramfs`, `update-grub(2)` and `grub-mkconfig`
  are diverted to no-ops and the kernel's install hooks (`/etc/kernel/*.d`) are removed in the image. Pinned out
  of apt (priority -1): GRUB, shim, kernels and headers, ostree, bootc, bootupd, dracut, initramfs-tools,
  systemd-boot, efibootmgr, DKMS and every `*-dkms` module, microcode, fwupd, mokutil, memtest86+. Held at the
  image's exact versions (`/etc/apt/preferences.d/hearth-image-core`, generated at build): every package built from
  systemd (udev, libsystemd-shared…), glibc (libc6, libc-bin…), D-Bus and Mesa; a package that needs newer ones
  can't be installed (the reapply notice says so after an update).
- **`/etc/ld.so.cache`** is rebuilt at every start for that start's `/usr`, and from the image's `/usr` just before
  an update is finalized (`hearth-unlock-ldcache.service`), so a new version never starts with the layer's cache.

### Locking it again

Settings › Updates & Recovery › **Unlocked system** (Game Mode), Desktop Settings › Updates, or
`sudo hearth-unlock --lock`. It applies from the next restart (taking `/usr` away from running programs isn't
safe); your packages stay saved and come back with `sudo hearth-unlock` (or the switch), which is immediate.
`hearth-unlock --status` shows where things are (`--json` for scripts).

### `.deb` files

The Install Package window (`hearth-debbox`) asks `hearth-unlock --status --json`: while the system is writable a
`.deb` installs **on the system itself** with apt (dependencies from Debian; drivers and services work), through
`hearth-admin deb-install`, which always asks for an administrator's password like `sudo apt` would. Locked (and on
Anatase) it goes into Hearth's Debian box as before. Bazaar/Flatpak are unchanged.

## How to run it

CI: Actions → **Hearth OS (Peppermint base)** → Run workflow. It also runs on pushes to `linux-host` that touch
`os-bases/peppermint/**` or the workflow. Failures appear as annotations.

Locally, from the repository root, with the Hearth app unpacked into `os/nimbus-app`:

```sh
sudo podman build -f os-bases/peppermint/Containerfile --build-arg NIMBUS_VERSION=0.0.0 -t hearth-peppermint .
# options: --build-arg NVIDIA=0 (no Nvidia driver)   --build-arg PEPPERMINT_DESKTOP=0 (no Xfce session)
```

To switch an existing Hearth (or any bootc system) to it for testing:

```sh
sudo bootc switch ghcr.io/dragongamer9/hearth-peppermint:testing
```

This is a different distribution underneath, so treat it as a test install. A fresh install from Hearth's
installer is the intended path, and it needs the installer change below.

## Gaps and known risks

- **Nothing here has booted on hardware yet.** Everything below the image (boot, Secure Boot, suspend,
  handheld controls) is untested.
- **The installer must write GRUB's menu.** Without `hearth-bls-grub --boot <target>/boot` after deploying, the
  first boot shows only "no boot entries yet". The same applies to `bootc install to-disk`. Run it once by hand
  in a VM.
- **Secure Boot with Nvidia:** the DKMS-built Nvidia module (and `acpi_call`) is signed with a throwaway key, so
  with Secure Boot on it won't load. AMD/Intel are unaffected. Fixing this needs a Hearth MOK enrolled at install
  time, or Secure Boot turned off on Nvidia PCs.
- **Nvidia comes from NVIDIA's repository.** A broken upstream release or a kernel too new for it fails the
  build. That's on purpose: the image needs the module. Build with `NVIDIA=0` to get an AMD/Intel-only image
  meanwhile. The 32-bit Nvidia libraries depend on NVIDIA's repository carrying them; CI warns if they're missing.
- **The image is larger than it needs to be.** The C compiler and kernel headers stay in the image, because the
  DKMS packages depend on them.
- **The Steam client bootstraps on first launch.** Debian's `steam-installer` downloads the real client into the
  user's home on first run, so first launch needs the internet.
- **No InputPlumber or Steam-specific services.** The image has no SteamOS Manager and no Deck fan control:
  HHD covers controllers, TDP and the power button. There's no Steam Deck fan daemon (`jupiter-fan-control` isn't in Debian), so the Deck's firmware fan curve applies. That needs checking on a Deck.
- **`hearth-admin` partly assumes rpm-ostree** (the hibernation kargs path). See below.
- **Peppermint's own tools aren't included** (see the table above). If wanted, they'd need packaging first.

## Test checklist

On a VM first (UEFI, Secure Boot off), then a ROG Ally, Legion Go, Steam Deck, an AMD desktop and an Nvidia
desktop:

- [ ] CI is green, and every check in "Check the image" passes (warnings noted).
- [ ] Install with Hearth's installer (with the installer change), or `bootc install to-disk` +
      `hearth-bls-grub --boot`.
- [ ] GRUB shows "Hearth OS (Peppermint edition, Debian 13)" and boots. The Hearth splash shows.
- [ ] With Secure Boot **on** (AMD/Intel), it still boots through Debian's shim.
- [ ] It autologins into Hearth's Game Mode. Steam bootstraps and launches in Game Mode. A Proton game runs.
- [ ] The FPS limit, screenshot and performance overlay (`--mangoapp`) work in Game Mode.
- [ ] Switch to Desktop Mode: labwc + Quickshell taskbar, Hearth Settings, Files, Task Manager, a polkit prompt.
- [ ] Wi-Fi, Bluetooth controller pairing, sound (ROG Ally speakers: Cirrus firmware), brightness, suspend and
      resume.
- [ ] HHD: built-in controller, gyro, TDP slider (Ally: firmware attributes; Legion Go: acpi_call), and power
      button in Hearth.
- [ ] Nvidia desktop: `nvidia` module loaded, gamescope runs, Hearth isn't sluggish, games run. Suspend works.
- [ ] Settings › Updates: Update now stages `:testing`. Restart → new version. `bootc rollback` → GRUB menu
      updated, previous version boots.
- [ ] A background update gets staged by the timer and doesn't reboot by itself.
- [ ] Login screen (autologin off in Settings): Hearth theme, controller works, "Peppermint desktop (Xfce)"
      session starts.
- [ ] Flatpak from Flathub installs and updates.
- [ ] Writable: `hearth-unlock --status` says unlocked; `sudo apt install htop` works and survives a restart; a
      `.deb` double-clicked in Files installs on the system (password asked).
- [ ] Update with packages added: the new version starts, and they come back by themselves within a few minutes
      online; Go back starts the previous version with its packages.
- [ ] Settings › Updates & Recovery › Unlocked system off → restart → locked; on → writable at once.
- [ ] A package that breaks Hearth's start (e.g. a broken service ordered before the display manager): the next
      start is a safe start with the notice; Turn my changes back on works.

## Changes needed outside this folder

These aren't made here: those files belong to the main build, the installer and the other agents.

1. **`os/live/src/hearth-install`** (installer):
   - After `deploy()` and `add_kargs()`, run `<dep>/usr/libexec/nimbus/hearth-bls-grub --boot <TARGET>/boot` when
     that file exists. The Ubuntu image needs the same step.
   - `ensure_boot_entry()` hardcodes `\EFI\fedora\…`. It should use the vendor folder that bootupd installed
     (`EFI/debian` here).
   - `SKIP_DIRS` should include the image's own vendor folder (`debian`, and `ubuntu` for that image). Otherwise
     Hearth's boot menu lists Hearth's own loader as another system.
2. **`os/system_files/usr/libexec/nimbus/hearth-boot-first`**: also hardcodes `\EFI\fedora\…` when it has to
   create the entry. It should detect the vendor folder.
3. **`os/system_files/usr/libexec/nimbus/hearth-admin`**: the hibernation setup changes kargs with `rpm-ostree
   kargs`. Without rpm-ostree, it should fall back to `ostree admin kargs edit-in-place` (or bootc's kargs).
   Its update path already uses bootc first.
4. **`hosts/linux/desktop/labwc/autostart`**: add Debian's polkit agent path
   `/usr/lib/policykit-1-gnome/polkit-gnome-authentication-agent-1`. The image links it to
   `/usr/libexec/polkit-gnome-authentication-agent-1` in the meantime.
5. **`hosts/hearth-core/src/bridge.rs` `edition()`**: it reads only `/usr/share/ublue-os/image-info.json`. It
   could also read `/usr/share/nimbus/image-info.json` (written by this build and the CachyOS build), so the
   player counter can tell the bases apart.
6. **Shared Debian-family pieces**: `hearth-bls-grub`, its units, the `10_blscfg.cfg` override, the wheel polkit
   and sudoers files and the sysusers drop-in exist in both `os-bases/ubuntu` and `os-bases/peppermint`. Once
   both settle, they could move to one shared folder.

**Hearth Plasma.** KDE Plasma 6 (the workspace only, no recommends, plus Dolphin) is installed as Desktop Mode's second desktop, in Hearth's look; Hearth Classic (labwc + Mantel) stays and is the fallback, and SDDM stays the display manager. See docs/PLASMA.md.

**Hearth GNOME.** GNOME Shell (minimal, no recommends: shell, session, Settings, portal, plus GNOME Files, Console and Text Editor shown only there) is installed as Desktop Mode's third desktop, with Hearth's own Shell extensions; SDDM stays the display manager. See docs/GNOME.md.
