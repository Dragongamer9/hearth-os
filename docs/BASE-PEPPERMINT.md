# Hearth OS on Peppermint OS (Beta Experimental)

**Status: Beta Experimental.** The image is built and checked by CI. It hasn't been booted on real hardware yet.
It isn't the main Hearth. It's published as `ghcr.io/dragongamer9/hearth-peppermint:testing` and never as
`:latest`.

The Peppermint build works like the other Hearth images (Bazzite in `os/`, CachyOS and Ubuntu in `os-bases/`):

- It's one bootc image.
- It updates as a whole and rolls back as a whole.
- Hearth's installer installs it.
- It boots into the Hearth session: Game Mode (gamescope + the Hearth app) and Desktop Mode (labwc + Quickshell).

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
| Peppermint updater (apt) | No | apt can't change an image-based system. Hearth updates with `bootc upgrade`. |
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
