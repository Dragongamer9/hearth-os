# Hearth OS on Ubuntu (Beta Experimental)

**Status: Beta Experimental.** The image recipe and its CI checks are written, but the image hasn't been built
in CI or booted on hardware yet. It isn't the main Hearth (that's moving to Anatase). It's published as
`ghcr.io/dragongamer9/hearth-ubuntu:testing` and never as `:latest`.

The Ubuntu build works like the other Hearth images (Bazzite in `os/`, Peppermint and CachyOS in `os-bases/`):

- It's one bootc image.
- It updates as a whole and rolls back as a whole.
- Hearth's installer installs it.
- It boots into the Hearth session: Game Mode (gamescope + the Hearth app) and Desktop Mode (labwc +
  Quickshell).

The user's requirements, and how this build meets each one:

| Requirement | How |
|---|---|
| Ubuntu 26.04 LTS | Starts from `docker.io/library/ubuntu:26.04` ("Resolute Raccoon"). |
| Snap-free | The container image has no snapd. `/etc/apt/preferences.d/hearth-no-snap` pins snapd to -10, so no package can pull it in. Steam is the `steam-installer` deb, and the app store is Flatpak (Flathub). The build fails if snapd appears, and CI checks it. |
| Handheld Daemon (HHD), not InputPlumber | HHD 4.1.12 is built from source into `/usr/lib/hhd`. `hhd.service` runs system-wide from the first boot, and Hearth talks to it at `/run/hhd/api`. |
| Image-based (bootc) | bootc 1.17.1 and ostree 2026.4 are built from source. The bootc filesystem layout is applied, and bootupd puts Ubuntu's signed shim and GRUB on the EFI partition. |
| One image, with Nvidia | AMD, Intel and Nvidia are in one image. Nvidia uses **Canonical's prebuilt, signed** open kernel module, so it loads with Secure Boot on. |

| Piece | Where |
|---|---|
| Image recipe | `os-bases/ubuntu/Containerfile` |
| Builder stage: ostree, bootc, bootupd, Quickshell, RyzenAdj, Handheld Daemon | `os-bases/ubuntu/builder.sh` |
| Image build (packages, Hearth, boot pieces, bootc layout) | `os-bases/ubuntu/build.sh` |
| Ubuntu-specific system files, layered over `os/system_files` | `os-bases/ubuntu/system_files/` |
| CI: build, check, publish `:testing` | `.github/workflows/base-ubuntu.yml` |

## How it's built

It's the Peppermint (Debian 13) recipe, which builds in CI, carried over to Ubuntu's archive. Below, the
Ubuntu-specific parts are **in bold**.

### Base stage (Containerfile)

- All four components (main, restricted, universe, multiverse) are enabled, plus i386 multiarch for Steam.
  Recommends are off.
- **snapd is pinned out.**
- **The container image's trimming is undone**: `/etc/dpkg/dpkg.cfg.d/excludes` would drop docs and man pages.
- **The container image's `ubuntu` account (uid 1000) is removed.** Hearth's installer gives uid 1000 to the
  player, and `nimbus-autologin` signs in uid 1000. Without this, the autologin would pick `ubuntu`. The build
  fails if uid 1000 is still taken.
- **GNU coreutils replaces Ubuntu 26.04's default Rust coreutils** (`coreutils-from-gnu`). Hearth's scripts are
  written and tested against GNU's, on Fedora and Debian. Build with `--build-arg GNU_COREUTILS=0` to keep
  Ubuntu's default. If the switch fails, the build carries on with a warning.

### Builder stage (`builder.sh`)

This stage is the same as Peppermint's, with the same pinned versions:

- ostree 2026.4 from the release tarball, plus equivs placeholders for `libostree-1-1`, `ostree` and
  `ostree-boot`, which are held. One known ostree with its `50ostree` dracut module, rather than whatever the
  archive has.
- bootc v1.17.1, without SELinux.
- bootupd v0.3.2.
- Quickshell v0.3.2, built against wayland-protocols 1.46.
- RyzenAdj v0.19.0.
- HHD v4.1.12 in a venv with `--system-site-packages`.
- `builder-runtime-deps.txt`: the Ubuntu packages these programs link against. `build.sh` installs them before
  running any of them.

### Image stage (`build.sh`)

| Area | What | Notes |
|---|---|---|
| Kernel | **`linux-image-generic-hwe-26.04`** (the hardware-enablement series, which moves to each new Ubuntu release's kernel). It falls back to `linux-image-generic` (7.0) if the archive lacks it. `--build-arg KERNEL=ga` forces 7.0. | The versioned image behind the metapackage is installed directly, so the kernel's Recommends stay out and Nvidia's module can be matched to it exactly. Signed by Canonical. `vmlinuz` is copied to `/usr/lib/modules/<kver>/` (or de-symlinked if it's already there). |
| Firmware | **26.04's split `linux-firmware-*` packages** for PCs and handhelds: AMD, Intel, Nvidia and Qualcomm graphics; Intel, Realtek, MediaTek, Broadcom, Qualcomm and Marvell wireless; `-misc` (Cirrus amps for the ROG Ally speakers); plus SOF and microcode | The data-centre firmware (Mellanox, Netronome, QLogic, Prestera) is left out. If the split names aren't there, it falls back to the whole `linux-firmware`. |
| Graphics | Mesa (Vulkan, GL, VA) with its i386 half | main |
| Nvidia | **`linux-modules-nvidia-<branch>-open-<kver>`**: Canonical's prebuilt, signed open module, using the newest branch built for this kernel (580 at the time of writing). The userspace is the no-DKMS set: `nvidia-headless-no-dkms-<branch>-open`, `nvidia-utils`, `libnvidia-gl` and friends, with i386 GL/compute where available. | Fallback if there's no prebuilt module for the kernel (say, a brand-new HWE kernel): `nvidia-dkms-<branch>-open`, unsigned, with a CI warning. Suspend helpers only run when an Nvidia driver is loaded. `nvidia-drm.modeset=1 fbdev=1` are kargs. |
| Steam | `steam-installer` + `steam-devices` (multiverse), with the i386 steam-libs, the licence pre-accepted and a `/usr/bin/steam` link | Never the snap. |
| Game Mode | `gamescope` (multiverse, 3.16.2x). It's Valve's upstream, with `GAMESCOPECTRL_BASELAYER_WINDOW/APPID`, `GAMESCOPE_FPS_LIMIT`, `GAMESCOPECTRL_REQUEST_SCREENSHOT` and `--mangoapp`; CI checks each one. Also `mangoapp`, `mangohud` and `gamemode`. | Ubuntu installs these in `/usr/games`, so they're linked into `/usr/bin`. |
| Handhelds | HHD (enabled; turn it off with `/etc/hearth/no-hhd`) and its udev rules, hwdb and D-Bus policy; RyzenAdj; `acpi_call` | **acpi_call is built with DKMS, the module is kept, and DKMS, the compiler and the headers are purged**, so the image carries no toolchain. Kernel 7.0 also has Lenovo's WMI firmware attributes for Legion Go TDP. |
| Power | `power-profiles-daemon`, which only runs when HHD is off (HHD owns the same D-Bus name) | |
| Desktop Mode | labwc, Quickshell (built), grim, slurp, brightnessctl, wlr-randr, wlsunset, xdotool, gnome-keyring, portals (wlr + gtk), foot, a polkit agent and the QML modules | |
| App runtime | `libwebkit2gtk-4.1-0`, GStreamer, the Electron libraries | |
| System | systemd 259, NetworkManager (**`10-globally-managed-devices.conf` emptied**: Ubuntu's default leaves wired devices to netplan), systemd-resolved, BlueZ, PipeWire + WirePlumber, Flatpak + Flathub, udisks2, upower, fwupd, Noto/DejaVu/Liberation fonts, Plymouth with the Hearth theme (also set as **Ubuntu's `default.plymouth` alternative**), SDDM | |
| Sign-in | SDDM + `nimbus-autologin.service` + the Hearth SDDM theme (Qt 6), all from `os/system_files`. `hearth-greeter-pad` and `hearth-boot-first` are enabled. | |
| Updates | `bootc-fetch-apply-updates.timer`, which only stages (never reboots mid-game) and updates system Flatpaks. "Update now" goes through `hearth-admin` → `bootc upgrade`. | Same drop-in as Peppermint. The earlier draft's own `hearth-auto-update` timer was dropped in favour of it, matching what os.yml checks. |
| User namespaces | **`kernel.apparmor_restrict_unprivileged_userns = 0`** | Ubuntu's kernel can restrict user namespaces to programs with an AppArmor profile. This image has no AppArmor userspace or profiles, and Steam's runtime, Proton, Flatpak, the Chromium sandbox and Couch Co-op all need user namespaces. |

### Boot loader: Ubuntu's signed shim and GRUB

- `shim-signed`, `shim` and `grub-efi-amd64-signed` are **unpacked with `apt-get download` + `dpkg-deb -x`,
  not installed.** Their maintainer scripts run `grub-install` and `update-grub` against the build container.
- They're laid out for bootupd as `/usr/lib/efi/shim/<ver>/EFI/{ubuntu,BOOT}` and
  `/usr/lib/efi/grub2/<ver>/EFI/ubuntu`, then `bootupctl backend generate-update-metadata` runs. The folder has
  to be `EFI/ubuntu`: Ubuntu's signed GRUB has it built in.
- The Secure Boot chain is Microsoft → Canonical's shim → Canonical's GRUB → Canonical's kernel, and Canonical's
  Nvidia module.
- Ubuntu's GRUB has no `blscfg` command, so it doesn't read ostree's entries. `hearth-bls-grub` (the same script
  as Peppermint's, copied in) writes `/boot/grub2/hearth-bls.cfg` from `/boot/loader/entries`. bootupd's static
  `grub.cfg` sources it through the `10_blscfg.cfg` override. The menu is rewritten at boot, when the entries
  change (`hearth-bls-grub-refresh.path`) and at shutdown after a staged update is finalized.

### Initramfs and layout

- **dracut** (26.04's default anyway) builds a generic initramfs with `ostree`, `bootc` and `plymouth`. nouveau
  and the Nvidia modules are left out.
- The bootc layout is the same as Peppermint's:
  - `/var` becomes tmpfiles.d entries.
  - dpkg's database moves to `/usr/lib/sysimage/dpkg`.
  - `/opt` moves to `/usr/lib/opt`.
  - The home folder is `/var/home`.
  - `/usr/lib/sysusers.d/hearth-image.conf` lists the system accounts, and sysusers runs every boot.
  - composefs is set to `maybe`, and the root is read-only.

### The Hearth app

`hosts/hearth-rs` is built **inside an `ubuntu:26.04` container** in CI, so it links exactly the image's
WebKitGTK and glibc (2.43). `ubuntu-latest` runners may be on an older Ubuntu. Hearth Browser (Electron) is built
on the runner, as in `os.yml`.

### Apps you add with apt and .deb files

`sudo apt install` and opening a `.deb` file work. The app goes into a writable layer on top of the read-only
image (an overlay on `/usr`, one per deployment, kept in `/var/lib/hearth/layer`). It stays through restarts, and
after a system update it's put back on the new version by itself. A package that no longer fits is skipped and
reported. The system's own packages are pinned and protected: `apt upgrade` only upgrades what you added, and
removing system packages or installing kernel and boot loader packages is refused. Shared with the other
Debian-family edition in `os-bases/apt-layer/`. **The details are in [APT-LAYER.md](APT-LAYER.md).**

## How to run it

**In CI:** Actions → **Hearth OS (Ubuntu base)** → Run workflow. It also runs on pushes to `linux-host` that
touch `os-bases/ubuntu/**` or the workflow. Every failed check is its own `::error` annotation, including
`--check-update-path`'s output and `bootc container lint`. A notice says which kernel and Nvidia branch went in.

**Locally,** from the repository root, with the Hearth app unpacked into `os/nimbus-app`:

```sh
sudo podman build -f os-bases/ubuntu/Containerfile --build-arg NIMBUS_VERSION=0.0.0 -t hearth-ubuntu .
# options: --build-arg NVIDIA=0   --build-arg KERNEL=ga   --build-arg GNU_COREUTILS=0
```

**To try it on an existing bootc system:** `sudo bootc switch ghcr.io/dragongamer9/hearth-ubuntu:testing`.
Treat that as a test, not a real install. A fresh install from Hearth's installer is the intended path.

## What's verified, and what isn't

- **Verified locally:**
  - `bash -n` on `build.sh` and `builder.sh`.
  - `sh -n` on `hearth-bls-grub` and on the CI check script.
  - The workflow parses as YAML.
  - The kernel, Nvidia-branch and kernel-version parsing was tested against sample `apt-cache` output.
  - `hearth-bls-grub` was run against sample ostree entries.
- **Not yet verified (needs CI):** the image build itself. These package names were checked against the 26.04
  archive listings: `linux-modules-nvidia-580-open-*`, `nvidia-headless-no-dkms-580-open`, gamescope in
  multiverse, mangoapp, the 17 split firmware names. These are best-effort and may fail first:
  - `coreutils-from-gnu` (non-fatal).
  - `linux-image-generic-hwe-26.04` (it falls back).
  - Whether `nvidia-headless-no-dkms-*-open` pulls DKMS in anyway (only a warning).
  - Whether Ubuntu's `shim-signed` ships `mmx64` and `fbx64` (taken from `shim` if so).
- **Not yet verified (needs hardware):** everything from boot onward.

## Gaps and known risks

- **The image hasn't booted.** That means boot, the Secure Boot chain, suspend and handheld controls are all
  unverified.
- **acpi_call is unsigned.** It won't load with Secure Boot on. Legion Go TDP then relies on the kernel's Lenovo
  WMI firmware attributes, which kernel 7.0 has. The same applies to the Nvidia DKMS fallback, which is used
  only if no prebuilt module exists.
- **The Nvidia branch moves with the archive.** The newest branch built for the kernel is picked automatically.
  Pre-Turing cards (GTX 10 series and older) aren't supported by the open module. nouveau stays out of the
  initramfs but isn't blacklisted.
- **NetworkManager on Ubuntu stores profiles through netplan** (`/etc/netplan/90-NM-*.yaml`). `/etc` persists
  across updates, so this should be fine, but it's different from the other editions.
- **SDDM's greeter runs on X.Org** (still in 26.04 for non-GNOME sessions). If Ubuntu drops it, the greeter
  needs a Wayland compositor.
- **The Steam client bootstraps on first launch** (it downloads into the home folder), so first launch needs
  the internet.
- **No Steam Deck fan daemon** (`jupiter-fan-control` isn't in Ubuntu), so the Deck's firmware curve applies.
- **GNU coreutils makes the image differ from stock Ubuntu.** Ubuntu tools that rely on uutils-only behaviour
  would notice, but none are installed.

## Test checklist

On a VM first (UEFI, Secure Boot off, then on), then a ROG Ally, Legion Go, Steam Deck, an AMD desktop and an
Nvidia desktop:

- [ ] CI is green and every check passes (warnings noted).
- [ ] Install with Hearth's installer (it runs `hearth-bls-grub --boot`), or `bootc install to-disk` + `hearth-bls-grub --boot`.
- [ ] GRUB shows "Hearth OS (Ubuntu edition, 26.04 LTS)" and boots, and the Hearth splash shows.
- [ ] With Secure Boot **on**, it boots through Canonical's shim on AMD, Intel **and Nvidia** (`lsmod | grep nvidia`).
- [ ] It autologins into Game Mode. Steam bootstraps, Game Mode runs, and a Proton game works.
- [ ] The FPS limit, screenshot and performance overlay (`--mangoapp`) work.
- [ ] Desktop Mode: labwc + Quickshell taskbar, Hearth Settings, Files, Task Manager, a polkit prompt.
- [ ] Wi-Fi, wired network (managed by NetworkManager), Bluetooth controller pairing, sound (Ally speakers),
      brightness, suspend and resume.
- [ ] HHD: built-in controller, gyro, TDP slider, and the power button in Hearth. `hearth-hhd status` shows it.
- [ ] Nvidia desktop: gamescope runs, games run, suspend works.
- [ ] Settings › Updates stages `:testing` → restart → new version. `bootc rollback` → GRUB menu updated.
- [ ] The background update is staged by the timer and never reboots by itself.
- [ ] Flatpak from Flathub installs and updates. `snap` doesn't exist.
- [ ] Login screen (autologin off): the Hearth theme shows and the controller works.

## Changes needed outside this folder

Several items the Peppermint doc listed are already done on `linux-host`:

- The installer runs `hearth-bls-grub` and skips its own vendor folder.
- `efi_loader` and `hearth-boot-first` know `ubuntu`.
- `bridge.rs` reads `/usr/share/nimbus/image-info.json`.
- labwc's autostart has Debian's polkit agent path.

What's left:

1. **`os/system_files/usr/libexec/nimbus/hearth-admin`**:
   - The hibernation setup (around line 558) uses `rpm-ostree kargs`. Without rpm-ostree it needs a fallback to
     `ostree admin kargs edit-in-place --append-if-missing=… --delete-if-present=…`.
   - Check that its rpm-ostree update paths are only tried when `rpm-ostree` exists. The bootc path is the only
     one here.
2. **`os/live/src/hearth-install`**: `hearth-bls-grub` runs *before* `bootloader()` (bootupd). That's fine if
   bootupd leaves other files in `/boot/grub2` alone. Running it again after `bootloader()` would remove the
   doubt.
3. **`os/build_files/build.sh` and the other base builds**: the shared `chmod 755` list of desktop shell scripts
   doesn't include the new `shell/hearth-hhd`. This build chmods it if present. The shared list should include
   it once it lands.
4. **`os/system_files/usr/lib/systemd/system/nimbus-autologin.service`**: `After=bazzite-autologin.service` is
   harmless here (the unit doesn't exist).
5. **Shared Debian-family pieces**: these are now identical copies in `os-bases/ubuntu` and
   `os-bases/peppermint`, apart from comments. They could move to one shared folder.
   - `hearth-bls-grub` and its three units
   - `10_blscfg.cfg`
   - the HHD, power-profiles-daemon, bootc-timer and sysusers drop-ins
   - the wheel polkit and sudoers files, and the `hearth-seat` PAM file
   - `prepare-root.conf`, the resolv.conf tmpfiles
