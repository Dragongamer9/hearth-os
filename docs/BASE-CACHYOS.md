> **Archived (2026-10-22).** Hearth OS on CachyOS isn't built or offered for now, to focus on Anatase and Peppermint; it comes back after that. Systems on it can switch from Settings › Updates › Switch edition.

# Hearth OS on CachyOS (Beta Experimental)

**Status: Beta Experimental.** The recipe and CI workflow are written; the image has not yet been built by CI or
booted on real hardware. It isn't the main Hearth (that is moving to Anatase). It's published as
`ghcr.io/dragongamer9/hearth-cachyos:testing`, never as `:latest`.

The CachyOS build works like the other Hearth images (Bazzite in `os/`, Peppermint and Ubuntu in `os-bases/`):

- It's one bootc image, AMD/Intel and Nvidia together.
- It updates as a whole and rolls back as a whole.
- Hearth's installer installs it.
- It boots into the Hearth session: Game Mode (gamescope + the Hearth app) and Desktop Mode (labwc + Quickshell).

| Piece | Where |
|---|---|
| Image recipe (ctx stage, pacman layout, bootc + bootupd builder stage) | `os-bases/cachyos/Containerfile` |
| Image build (packages, Hearth, boot loader, initramfs, bootc layout) | `os-bases/cachyos/build.sh` |
| CachyOS-specific system files, layered over `os/system_files` | `os-bases/cachyos/system_files/` |
| CI: build, check, publish `:testing` | `.github/workflows/base-cachyos.yml` |

## What CachyOS contributes

| CachyOS part | In this image | Notes |
|---|---|---|
| x86-64-v3 repositories (`cachyos-v3`, `cachyos-core-v3`, `cachyos-extra-v3`) | **Yes**: the base image is `docker.io/cachyos/cachyos-v3` | Every package rebuilt for AVX2-class CPUs, see "CPU support" below. |
| `linux-cachyos-deckify` kernel | **Yes** (default `KERNEL`) | CachyOS's handheld kernel: linux-cachyos (BORE/EEVDF tuning, sched_ext) plus Steam Deck, ROG Ally, Legion Go and MSI Claw patches. It runs desktops too, so one image serves both. `--build-arg KERNEL=linux-cachyos` swaps in the desktop kernel. |
| Nvidia open driver | **Yes**: CachyOS's `nvidia-utils` + `lib32-nvidia-utils`, and the kernel module prebuilt for the kernel (`linux-cachyos-deckify-nvidia-open`) when the repositories have it, otherwise `nvidia-open-dkms` built for the image's kernel | `--build-arg NVIDIA=0` leaves it out. |
| Handheld Daemon | **Yes, required**: `hhd` from CachyOS's repositories, enabled system-wide | If the repositories don't carry it, `build.sh` builds hhd's own PKGBUILD at the pinned tag (`HHD_REF=v4.1.12`); CI posts a notice when that happens. |
| `cachyos-settings` (zram, sysctl, ananicy-cpp, udev rules) and `scx-scheds` (sched_ext; `scx_loader` set to `scx_lavd`, Auto) | Yes (optional: a warning if missing) | |
| Valve's gamescope (CachyOS's handheld repo), `lib32-gamescope`, `steamdeck-dsp`, `jupiter-fan-control` (Deck only), `umu-launcher`, `ryzenadj` | Yes (gamescope required, the rest optional) | |
| CachyOS Handheld Edition's session (`cachyos-handheld`, `gamescope-session-cachyos`, Plasma Login Manager, `steamos-manager`, InputPlumber) | **No** | Hearth's own session replaces it. SteamOS Manager and InputPlumber are masked: they'd fight HHD over the power limit and the controller. |
| CachyOS installer (Calamares), `cachyos-hello`, kernel manager, Limine/systemd-boot setup | No | Hearth's installer deploys the image; bootupd + GRUB boot it; kernels come with the image. |

Where CachyOS shows up: `PRETTY_NAME="Hearth OS (CachyOS edition)"`, `VARIANT_ID=hearth-cachyos` (`ID` stays
`cachyos`, `ID_LIKE=arch`), and `/usr/share/nimbus/image-info.json` (`"base": "cachyos"`, `"march": "x86-64-v3"`,
the kernel and whether Nvidia is in).

## CPU support (x86-64-v3)

x86-64-v3 means AVX, AVX2, BMI1/2, FMA, F16C, LZCNT, MOVBE and XSAVE. Every Hearth target has it:

| Device | CPU | x86-64-v3 |
|---|---|---|
| Steam Deck LCD / OLED | AMD "Aerith" / "Sephiroth", Zen 2 | Yes (Zen 2 has AVX2, FMA3, BMI2, MOVBE, F16C). CachyOS's own handheld edition uses the v3 repositories on the Deck. |
| ROG Ally / Ally X | Ryzen Z1 / Z1 Extreme, Zen 4 | Yes (v4 too) |
| ROG Xbox Ally / Ally X (2025) | Ryzen Z2 A (Zen 2) / Z2 Extreme (Zen 5) | Yes |
| Legion Go / Go 2 / Go S | Z1 Extreme / Z2 Extreme / Z2 Go (Zen 3+) | Yes |
| MSI Claw | Intel Core Ultra (Meteor Lake, Lunar Lake) | Yes |
| Desktops | Intel Haswell (2013) and newer Core CPUs; AMD Zen (2017) and newer; also AMD Excavator | Yes |

Not supported by the v3 image: Intel Core CPUs before Haswell, Atom/Pentium/Celeron before Gracemont (they lack
AVX), AMD before Excavator. On those, the v3 image dies at boot with an illegal instruction (there's no
graceful message). For such PCs build a baseline image:

```sh
sudo podman build -f os-bases/cachyos/Containerfile \
  --build-arg BASE_IMAGE=docker.io/cachyos/cachyos:latest --build-arg MARCH=x86-64 -t hearth-cachyos:v1 .
```

The installer should check the CPU before installing the v3 image (see "Changes needed outside this folder").

## How it's built

### The bootc approach (bootcrew's arch-bootc, on CachyOS's container)

1. **pacman's database, cache and log move to `/usr/lib/sysimage`** (the Containerfile, exactly as
   bootcrew/arch-bootc does). The list of installed packages then updates with the image, and `/var` stays the
   machine's. NoExtract is dropped and glibc reinstalled, so locales and man pages exist.
2. **bootc 1.17.1 and bootupd 0.3.2 are built from source** in a builder stage (neither is packaged for Arch).
   bootc is built without SELinux (`--no-default-features --features "install-to-disk pre-6.15"`, the same as
   the Peppermint build). The builder fails early if Arch's ostree is older than 2025.3 (bootc's minimum). Only
   `/output` reaches the image.
3. **ostree, composefs and skopeo come from Arch's repositories**, installed before anything runs bootc.
4. **The initramfs** is built once with dracut (`system_files/usr/lib/dracut/dracut.conf.d/30-hearth-cachyos.conf`):
   generic (boots any PC), ostree's prepare-root (the dracut module is found by glob, since ostree 2024 names it
   `50ostree`, not `98ostree`), bootc's `51bootc`, and Plymouth with Hearth's theme. Nvidia's modules and nouveau
   are left out. The pacman hooks that would build host-only initramfs images into `/boot` (dracut's, mkinitcpio's,
   systemd-boot managers') are masked; DKMS's hooks stay.
5. **The bootc filesystem layout** (bootcrew): `/var` emptied into `tmpfiles.d` entries, `/var/home`, `/opt` moved
   to `/usr/lib/opt`, the `/ostree`, `/root`, `/srv`, `/mnt`, `/usr/local` links, composefs `maybe`, read-only
   sysroot.
6. **System accounts**: Arch packages ship `sysusers.d` files; `build.sh` also lists every system account in
   `/usr/lib/sysusers.d/hearth-image.conf`, and `systemd-sysusers` runs every boot (its update condition never
   fires on ostree).
7. **Tidy-up**: orphaned build dependencies, pacman's sync databases and the build container's pacman keyring
   (`/etc/pacman.d/gnupg`, otherwise every install would share the same local signing key) are removed.

### The boot loader: GRUB through bootupd, Secure Boot off

Hearth's installer puts the boot loader on the EFI partition with `bootupctl backend install --with-static-configs`
and writes the GRUB menu with `hearth-bls-grub`. Arch and CachyOS have no signed shim, so `build.sh`:

- builds GRUB as one EFI file (`grub-mkimage`) with every module the static configs need inside, and
  `/EFI/cachyos` built in as its folder;
- lays it out for bootupd: `/usr/lib/efi/grub2/<ver>/EFI/cachyos/grubx64.efi`, and the same file as
  `/usr/lib/efi/shim/<ver>/EFI/cachyos/shimx64.efi` and `EFI/BOOT/BOOTX64.EFI`. bootupd requires a `shimx64.efi` to
  find the vendor folder and to name in the firmware's boot entry; without Secure Boot there's nothing for a shim
  to do, so GRUB takes its name;
- runs `bootupctl backend generate-update-metadata`.

Arch's GRUB has no `blscfg` command, so the menu comes from `hearth-bls-grub` (the same script and units as the
Peppermint image): `/boot/grub2/hearth-bls.cfg`, written from ostree's `/boot/loader/entries` at boot, when the
entries change (rollback, kargs) and at shutdown after a staged update is finalized. bootupd's `10_blscfg.cfg` is
replaced by one that sources it. The installer already runs `hearth-bls-grub --boot` after deploying, and
already knows `EFI/cachyos` (`efi_loader`, `SKIP_DIRS`, `hearth-boot-first`).

**This boots with Secure Boot turned off only.** The Steam Deck doesn't enforce Secure Boot. The ROG Ally,
Legion Go, MSI Claw and most desktops ship with it on (for Windows): it has to be turned off in the firmware.

### What's installed

| Area | What |
|---|---|
| Kernel | `linux-cachyos-deckify` + headers (headers for DKMS), `linux-firmware`, `amd-ucode`, `intel-ucode` |
| Boot | dracut, ostree, composefs, bootc, bootupd, GRUB (EFI), efibootmgr, Plymouth (Hearth theme) |
| Graphics | Mesa + lib32 (RADV, ANV, VA-API: `intel-media-driver`), Vulkan loader + lib32, Xwayland |
| Nvidia | `nvidia-utils`, `lib32-nvidia-utils`, open kernel module (prebuilt or DKMS), suspend helpers only when the driver is loaded, `nvidia-drm.modeset=1 fbdev=1` kargs |
| Steam / Game Mode | `steam`, Valve's `gamescope` (+ `lib32-gamescope`), `mangohud` (carries `mangoapp`) + lib32, `gamemode` + lib32, `umu-launcher`; fonts (Liberation, Noto incl. CJK and emoji, DejaVu) |
| Handhelds | **Handheld Daemon** (`hhd.service`, system-wide, from the first boot; off with `/etc/hearth/no-hhd`), `acpi_call` for the Legion Go (prebuilt or DKMS; warned if missing), RyzenAdj, `uinput` loaded at boot, Steam Deck DSP and fan daemon (Deck only) |
| Power | `power-profiles-daemon`, installed and enabled but only started when HHD is off (HHD provides the same D-Bus service with the device's real limits). InputPlumber, SteamOS Manager and tuned are masked. |
| Desktop Mode | labwc, Quickshell (`quickshell`, else `quickshell-git`), grim, slurp, brightnessctl, wlr-randr, wlsunset, xdotool, gnome-keyring, polkit-gnome, portals (wlr + gtk), foot, qt6-wayland/svg, 7zip, fuse-overlayfs, bubblewrap, X tools |
| App runtime | `webkit2gtk-4.1`, GStreamer (base/good/bad/libav), GTK 3, NSS, CUPS/ALSA/libXss (Hearth Browser) |
| System | systemd (+ resolved, timesyncd), NetworkManager, BlueZ, PipeWire + WirePlumber (+ pulse/alsa/jack, lib32), Flatpak (+ Flathub), udisks2, upower, fwupd, sudo (wheel), polkit, python + python-evdev/yaml |
| Sign-in | SDDM (+ X server for its greeter) + `nimbus-autologin.service`, Hearth's SDDM theme (Qt 6) |
| Updates | `bootc-fetch-apply-updates.timer`, stage-only (never reboots mid-game) and updates system Flatpaks; off with `/etc/hearth/no-auto-update`. "Update now" goes through `hearth-admin` → `bootc upgrade`. |

The Hearth app comes from `hosts/hearth-rs`. CI builds it **inside the `cachyos/cachyos-v3` container**, so it
links exactly the WebKitGTK/GTK/libsoup sonames the image has and never needs a newer glibc. (An Ubuntu-built
binary would probably work too, since the sonames match, but `ubuntu-latest` changes underneath.) Hearth Browser
(Electron) is built on the runner, as in `os.yml`. `build.sh` and CI fail if `ldd` finds a missing library.

## How to run it

CI: Actions → **Hearth OS (CachyOS base)** → Run workflow. It also runs on pushes to `linux-host` that touch
`os-bases/cachyos/**` or the workflow. Failures appear as annotations (each failed check is its own annotation;
build failures carry the end of the log).

Locally, from the repository root, with the Hearth app unpacked into `os/nimbus-app` (`hearth` binary, and
`browser/` for Hearth Browser):

```sh
sudo podman build -f os-bases/cachyos/Containerfile --build-arg NIMBUS_VERSION=0.0.0 -t hearth-cachyos .
# options: --build-arg NVIDIA=0   --build-arg KERNEL=linux-cachyos
#          --build-arg BASE_IMAGE=docker.io/cachyos/cachyos:latest --build-arg MARCH=x86-64   (pre-Haswell PCs)
```

To try it from an existing Hearth (or any bootc system), Secure Boot off:

```sh
sudo bootc switch ghcr.io/dragongamer9/hearth-cachyos:testing
```

That keeps the old system's boot loader (which may be a signed Fedora shim/GRUB that reads BLS entries itself), so
it's only a partial test. A fresh install with Hearth's installer is the intended path.

## Gaps and known risks

- **Not built by CI yet, nothing booted on hardware.** The package names were checked against CachyOS's
  PKGBUILDs and Arch's, not against a live repository (the authoring environment couldn't reach CachyOS's
  mirrors or Docker Hub). Expect a first CI run to surface a renamed or missing package.
- **Secure Boot must be off** (unsigned GRUB, unsigned kernel, DKMS modules). Signing with a Hearth key plus a
  MOK enrolled at install time, or shipping a signed shim, would fix it later.
- **`shimx64.efi` is GRUB.** It satisfies bootupd and the installer's boot-entry code; if a future bootupd
  validates the shim, this needs revisiting.
- **The installer's live USB runs its own `bootupctl`** with `--src-root` pointing at this image. It must be a
  bootupd recent enough to read the `/usr/lib/efi` layout. Same as the Peppermint image.
- **x86-64-v3 only by default.** Pre-Haswell PCs need the baseline build (above); there's no runtime fallback.
- **Handheld Daemon's origin.** If CachyOS's repositories drop `hhd`, the build compiles the pinned tag; bump
  `HHD_REF` in `build.sh` deliberately. CI warns if the installed HHD is older than 4.x (Hearth uses its
  `/run/hhd/api` socket).
- **Nvidia:** if CachyOS has no prebuilt `linux-cachyos-deckify-nvidia-open`, DKMS builds the module in CI; a
  driver/kernel mismatch fails the build on purpose (`NVIDIA=0` for an AMD/Intel-only image meanwhile).
  `nvidia-utils` blacklists nouveau, so Nvidia cards older than the GTX 16 / RTX 20 series get no driver.
- **Steam Deck:** controls go through Steam (HHD doesn't drive the Deck's controller); the DSP and fan daemon are
  installed if the repositories have them. Deck-specific bits from CachyOS's `jupiter-hw-support` (SD card
  automount, firmware tools) are not included.
- **Size:** kernel headers, DKMS, the compiler toolchain (`base-devel`, already in CachyOS's container image) and
  CJK fonts stay in the image. Removing the headers would make DKMS's pacman hook delete the built modules.
- **Steam bootstraps on first launch** (Arch's `steam` downloads the client into the home folder): first launch
  needs the internet.
- **`hearth-admin` partly assumes rpm-ostree** (see below).

## Test checklist

On a VM first (UEFI, Secure Boot off; a CPU model with AVX2, e.g. `-cpu host`), then a Steam Deck, ROG Ally,
Legion Go, an AMD desktop and an Nvidia desktop:

- [ ] CI is green: every check in "Check the image" passes (warnings noted), `bootc container lint` passes, and
      the `--check-update-path` lines are all `ok`.
- [ ] Install with Hearth's installer. GRUB shows "Hearth OS (CachyOS edition)" and boots; the Hearth splash shows.
- [ ] The firmware's boot entry is "Hearth OS" → `\EFI\cachyos\shimx64.efi`; deleting it still boots via
      `\EFI\BOOT\BOOTX64.EFI`.
- [ ] `uname -r` is the deckify kernel; `cat /sys/kernel/sched_ext/root/ops` shows `lavd` (if scx is in).
- [ ] It autologins into Game Mode. Steam bootstraps and launches; a Proton game runs.
- [ ] Game Mode FPS limit, screenshot and performance overlay (`--mangoapp`) work.
- [ ] Desktop Mode: labwc + Quickshell taskbar, Hearth Settings, Files, Task Manager, a polkit prompt (polkit-gnome).
- [ ] Wi-Fi, Bluetooth controller pairing, sound (ROG Ally speakers; Deck speakers with the DSP), brightness,
      suspend and resume.
- [ ] HHD: `systemctl status hhd`, `/run/hhd/api` exists; built-in controller, gyro, TDP slider (Ally: firmware
      attributes; Legion Go: `acpi_call`), power button in Hearth. `touch /etc/hearth/no-hhd` + reboot →
      power-profiles-daemon takes over.
- [ ] Nvidia desktop: `nvidia` module loaded, gamescope runs, Hearth isn't sluggish, games run, suspend works.
- [ ] Settings › Updates: Update now stages `:testing`; restart → new version; `bootc rollback` → the GRUB menu
      updates and the previous version boots.
- [ ] A background update is staged by the timer and doesn't reboot by itself.
- [ ] Login screen (autologin off in Settings): Hearth theme, controller works.
- [ ] Flatpak from Flathub installs and updates.
- [ ] On a pre-Haswell PC: the v3 image is refused by the installer (once the installer check exists), and the
      baseline build boots.

## Changes needed outside this folder

Not made here: these files belong to the main build, the installer and other agents.

1. **`os/live/src/hearth-install`**:
   - **CPU check**: before installing an image whose `/usr/share/nimbus/image-info.json` says
     `"march": "x86-64-v3"`, check `/proc/cpuinfo` flags for `avx2 bmi1 bmi2 fma f16c movbe abm` (LZCNT) and
     `xsave`, and refuse with a clear message ("this PC's processor is too old for the CachyOS edition"). Without
     it, a v3 install on an old CPU fails at first boot with no explanation.
   - **Secure Boot warning**: when the image has no signed shim (CachyOS edition) and Secure Boot is on
     (`/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c`, last byte 1), tell the person to
     turn it off before installing, or the PC won't start Hearth.
   - Already fine, no change needed: `hearth-bls-grub --boot` after deploy, `EFI/cachyos` in `efi_loader()` and
     `SKIP_DIRS`, `sysroot.bootloader none`, ext4 `/boot`.
2. **`os/system_files/usr/libexec/nimbus/hearth-admin`**:
   - `update()`: when bootc finds nothing new, `rpm_ostree_has_update()` raises `UpgradeFailed` on a system
     without rpm-ostree, which the `except UpgradeFailed` then answers by calling `rpm_ostree_upgrade()`; Popen
     raises `FileNotFoundError` and "Update now" ends in a traceback instead of "up to date". The rpm-ostree
     paths in `update()` and `update_check()` (`rpm_ostree_has_update`, `rpm_ostree_upgrade`) should be skipped
     when `shutil.which('rpm-ostree')` is None (`layered()` and `staged_any()` already catch `OSError`). Same for
     the Peppermint and Ubuntu images.
   - Hibernation setup changes kargs with `rpm-ostree kargs`; without rpm-ostree it should use
     `ostree admin kargs edit-in-place --append-if-missing=…` (or write a `/usr/lib/bootc/kargs.d`-style entry).
3. **`hosts/linux/desktop/labwc/autostart`**: no change needed (Arch's polkit-gnome path
   `/usr/lib/polkit-gnome/…` is already first in its list).
4. **`hosts/hearth-core/src/bridge.rs`**: no change needed (`edition()` already reads
   `/usr/share/nimbus/image-info.json` and knows `"cachyos"`).
5. **Shared pieces**: `hearth-bls-grub`, its units, the `10_blscfg.cfg` override, sudoers/polkit wheel files, the
   sysusers drop-in, the HHD and power-profiles-daemon drop-ins and the stage-only update drop-in are now copied in
   `os-bases/peppermint`, `os-bases/ubuntu` and `os-bases/cachyos`. Once the bases settle they could move to one
   shared folder (for example `os-bases/common/system_files`).
