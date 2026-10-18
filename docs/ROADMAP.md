# Hearth OS roadmap: Ember (Windows) and Hearth (Linux)

Hearth OS is the project: one UI with two products. **Ember** is the Windows shell. **Hearth** is the Linux side, which grows in three steps, each usable on its own.

## 1. Hearth OS session on any distro (done, in testing)

- Game Mode: the shared UI in Electron, fullscreen in gamescope (`hosts/linux/`).
- One "Hearth" login session that switches between Game Mode and Desktop Mode without logging out.
- Linux preview AppImages publish to GitHub Releases on every push to `linux-host`.

## 2. Desktop Mode (first version done, in testing)

labwc for windows, Quickshell for everything Hearth OS draws (`hosts/linux/desktop/`).

- [x] labwc config: Hearth OS window colors, keybinds, right-click menu
- [x] Taskbar: launcher button, running apps, tray, volume, network, battery, clock, Game Mode button
- [x] App launcher: search, grid, power actions (Super opens it)
- [x] Notifications
- [x] Wallpaper (Hearth OS default, or `~/.config/nimbus/wallpaper.jpg`)
- [ ] Later: lock screen, Wi-Fi/Bluetooth panels, screen settings (labwc + wlr-randr)

## 3. Hearth OS (started)

An installable, image-based distro that boots straight into Hearth OS, like SteamOS. Built the same way as
Bazzite: a container image on Fedora Atomic (bootc), built in GitHub Actions from the Universal Blue
image template.

**Layout:** `os/` in this repo.

| File | Purpose |
|---|---|
| `os/Containerfile` | `FROM ghcr.io/ublue-os/bazzite:stable` (drivers, Steam, gamescope, codecs already there), then add labwc, Quickshell, Heroic, umu-launcher and Hearth OS |
| `os/build.sh` | Package installs, Hearth OS files into `/usr/lib/nimbus` and `/usr/share/nimbus`, default session = Hearth OS, autologin |
| `os/disk_config/iso.toml` | Installer settings for the ISO |
| `.github/workflows/os.yml` | Build and sign the image, push to `ghcr.io/dragongamer9/hearth`; build the ISO on release |

**Decisions**

- Base: Bazzite (desktop variant). It already solves GPU drivers, Steam, gamescope and handheld support; we replace its session, not its plumbing.
- Hearth OS inside the OS is unpacked (no AppImage): `/usr/lib/nimbus`, session scripts in `/usr/bin`, Desktop Mode in `/usr/share/nimbus`. The AppImage's self-update is turned off there; the whole OS updates as one image.
- Updates: `bootc upgrade` in the background, applied at next boot; the previous image stays in the boot menu for rollback. Hearth OS › Settings › Updates shows OS updates when running on Hearth.
- Signing: Cosign key; the private key is a GitHub secret (`SIGNING_SECRET`), never committed.

**Milestones**

1. Image builds in CI; an existing Bazzite/Fedora Atomic PC can `sudo bootc switch ghcr.io/dragongamer9/hearth` and boot into Hearth OS.
2. ISO: installs on a blank PC, first boot goes straight to Game Mode.
3. First-boot setup in Hearth OS (Wi-Fi, account sign-ins), Plymouth boot splash and login theme in Hearth OS colors.
4. OS updates and rollback in Settings.

Needs from you before milestone 1: a Cosign key pair (`COSIGN_PASSWORD="" cosign generate-key-pair`), with
`cosign.key` stored as the `SIGNING_SECRET` repository secret.
