# Hearth, the OS

Hearth is Bazzite (deck edition: handheld support, SteamOS-style autologin) with the Hearth session installed
under `/usr` and booted into. It's a [bootc](https://containers.github.io/bootc/) image, so the whole system
updates as one piece and rolls back if an update goes wrong.

```
os/
  Containerfile                 FROM ghcr.io/ublue-os/bazzite-deck:stable, adds Hearth
  build_files/build.sh          Installs the app, session, Desktop Mode (labwc + Quickshell), branding
  system_files/                 Files copied into the image (autologin into the Hearth session)
  disk_config/iso.toml          Installer settings
  disk_config/disk.toml         Disk image settings (VMs)
```

The "Hearth OS" workflow (`.github/workflows/os.yml`) builds the image on every push to main that touches the
OS, the Linux host or the UI, and every Monday to pick up Bazzite's updates. It's published as
`ghcr.io/dragongamer9/hearth` (AMD and Intel graphics, from `bazzite-deck`) and `ghcr.io/dragongamer9/hearth-nvidia`
(Nvidia GTX 16 / RTX 20 and newer, from `bazzite-deck-nvidia`), each tagged `latest`, the version, and version-date.
Both work on handhelds and desktop PCs.

## Install

**From an existing Bazzite or Fedora Atomic install** (keeps your files):

```sh
sudo bootc switch ghcr.io/dragongamer9/hearth:latest          # AMD / Intel graphics
sudo bootc switch ghcr.io/dragongamer9/hearth-nvidia:latest   # Nvidia graphics
systemctl reboot
```

**Fresh install:** run the "Hearth OS" workflow by hand with *Build the installer ISO* ticked, download the ISO
from the run, and write it to a USB stick (Fedora Media Writer, Ventoy, or `dd`).

Going back to plain Bazzite: `sudo bootc switch ghcr.io/ublue-os/bazzite-deck:stable`.

## What's different from Bazzite

- Boots straight into the Hearth session (Game Mode) for the first user. To get the login screen instead:
  `sudo mkdir -p /etc/nimbus && sudo touch /etc/nimbus/no-autologin`.
- Desktop Mode is labwc with the Hearth taskbar; KDE Plasma and Steam are still there.
- Hearth updates with the system (`bootc upgrade`, or Bazzite's automatic updates), not by itself.

## Signing

Images are signed with Cosign when the `SIGNING_SECRET` repository secret holds a private key:

```sh
cosign generate-key-pair      # leave the password empty; commit cosign.pub, never cosign.key
gh secret set SIGNING_SECRET < cosign.key
```
