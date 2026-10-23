# Hearth OS on Anatase

**Status: testing.** The image is built and checked by CI. It hasn't been booted on real hardware yet. It's
published as `ghcr.io/dragongamer9/hearth-os:testing` only. `:latest` is promoted by hand from a `:testing` build
that has been tried on devices, and it's what the Bazzite-based Hearth OS offers as the recommended move.

Hearth OS is leaving Bazzite for **Anatase** (`i.anatase.org/anatase:stable`, also `:testing`): Fedora 45, bootc,
one image for AMD, Intel and Nvidia graphics, with Handheld Daemon (HHD) for handheld controls instead of
InputPlumber.

| Piece | Where |
|---|---|
| Image recipe | `os-bases/anatase/Containerfile` |
| Image build (packages, Hearth, Steam, boot splash, name) | `os-bases/anatase/build.sh` |
| Anatase-specific system files, layered over `os/system_files` | `os-bases/anatase/system_files/` |
| CI: build, check, publish `:testing` | `.github/workflows/base-anatase.yml` |
| The move from the Bazzite-based Hearth OS | `hearth-admin move-to-new-os`, `hosts/hearth-core/src/newos.rs` |
| The first start after the move | `os/system_files/usr/libexec/nimbus/hearth-clean-install` (+ `.service`) |
| Handheld Daemon in Hearth | `hosts/hearth-core/src/hhd.rs`, `hosts/linux/desktop/shell/hearth-hhd` |

## What's the same as on Bazzite

`build.sh` follows `os/build_files/build.sh` step by step, because Anatase is Fedora too:

- the Hearth app, Hearth Browser, the session (Game Mode with gamescope, Desktop Mode with labwc + Quickshell);
- `hearth-admin` and its polkit rule, so Settings › Updates works the same (bootc, with rpm-ostree where the
  system has packages layered on top);
- the Hearth look, the boot splash, the controller on the login screen;
- Steam installed in the system (the `steam` package, from RPM Fusion when Anatase doesn't have it), so a Bazzite
  home folder's Steam games in `~/.local/share/Steam` are found straight after the move. If that package can't be
  had at build time, `hearth-steam.service` installs Flathub's Steam (`com.valvesoftware.Steam`) at first boot.

## What's different

- **Handheld Daemon instead of InputPlumber.** HHD turns the handheld's buttons into an ordinary virtual
  controller, so the Guide button reaches Hearth like any controller's and opens Quick Access. HHD can't hold the
  Guide press back from the game the way InputPlumber does. Without an overlay app HHD's Quick Access button sends Guide + A; Hearth
  drops the A (`guide.rs` chord guard) so it doesn't pick something (with Hearth's overlay app, below, it opens Quick Access). The controller type and Nintendo layout in
  Settings › Controls, the power limit, power modes, and Settings › Controls › Handheld Daemon (gyro, back
  buttons, fan, lights) all go through HHD's local API. On Bazzite none of this runs: every HHD path checks that
  HHD is there first, and InputPlumber and the existing power-limit paths are used as before.
- **HHD's updater and Game Mode autologin are off** (`hhd.service.d/zz-hearth.conf`): Hearth updates the system
  itself, and `nimbus-autologin` signs into Hearth's session.
- **Quick Access over games is HHD's overlay app, and that app is Hearth's** (all HHD editions): build.sh links
  `desktop/shell/hearth-overlay` as `/usr/bin/hhd-ui` and points `HHD_OVERLAY` at it
  (`hhd.service.d/zz-hearth-overlay.conf`). It runs Hearth's Quick Access page in Hearth Browser's Electron
  (`hosts/linux/src/overlay`, GPU-composited like hhd-ui), speaks HHD's stdin/stdout protocol (`cmd:`/`action:` in,
  `stat:`/`grab:` out) and talks to Hearth over `$XDG_RUNTIME_DIR/hearth/overlay.sock` (`overlay_link.rs`; this user
  only). HHD shows it over the game and holds the controllers while it's open; its QAM button opens it, Guide still
  opens it through Hearth. Hearth then doesn't make its own WebKit overlay window. In Desktop Mode HHD's QAM button
  opens Mantel's Quick Access. Bazzite doesn't install it and keeps Hearth's own overlay.
  The page it draws is the running Hearth's (Hearth's hello on the socket: the WebKit app serves its built-in copy
  over the socket, the Electron window app names its folder), not the copy in the image's Electron package. It is
  also Quick Access on Hearth's starting screen while a game or Steam loads. When HHD has stopped the app (its
  window handling hits an error, e.g. while windows it watches come and go), a Guide/QAM press waits for it (3 s,
  9 s once Hearth has asked HHD to start it through `POST /api/v1/event {"type":"special","event":"overlay"}`)
  instead of showing the main window's Quick Access, and Hearth asks HHD to start it after 6 s away anyway (at most
  3 times in 10 minutes). The main window's panel is a last resort, logged `overlay: LAST RESORT`.
- **Plasma Login Manager.** `nimbus-autologin` writes the `[Autologin]` section of `/etc/plasmalogin.conf` when
  Plasma Login Manager is installed, and `hearth-greeter-pad` recognises its greeter.
- **Which edition this is** is in `/usr/share/nimbus/image-info.json` (`{"base": "anatase", ...}`), read by
  `bridge.rs` `edition()`. The Bazzite-based image has `/usr/share/ublue-os/image-info.json` instead.

## Moving from the Bazzite-based Hearth OS

Settings › Updates › **Move to a new Hearth OS** (only on the Bazzite-based Hearth OS, 0.1.3 and later):

| Choice | Image | |
|---|---|---|
| Hearth OS on Anatase | `ghcr.io/dragongamer9/hearth-os:latest` (`:testing` with `--testing`) | Recommended |
| Ubuntu | `ghcr.io/dragongamer9/hearth-ubuntu:testing` | Beta Experimental |
| CachyOS | `ghcr.io/dragongamer9/hearth-cachyos:testing` | Beta Experimental, x86-64-v3 processors only |
| Peppermint | `ghcr.io/dragongamer9/hearth-peppermint:testing` | Beta Experimental |

- A choice can be picked only once its image is published (asked anonymously from GitHub's registry, remembered
  for ten minutes per image). Until then it says "Not ready yet".
- CachyOS needs the x86-64-v3 processor features (`avx2`, `bmi2`, `fma`, `movbe` in `/proc/cpuinfo`). With Secure
  Boot on (`mokutil --sb-state`, or the SecureBoot EFI variable) Hearth warns that it has to be turned off first.
- Picking one shows what's kept and what's put aside, then asks once more.
- `hearth-admin move-to-new-os [--edition anatase|ubuntu|cachyos|peppermint] [--testing]` maps the edition to its
  image through a fixed table (never a free-form image), writes `/var/lib/hearth/clean-install.json` (which
  account, when, Bazzite's preinstalled Flatpak apps) and runs `bootc switch` (after `rpm-ostree reset` +
  `rpm-ostree rebase` when packages are layered). Nothing changes until the restart, and the old system stays
  in the boot menu.

At the first start of the new system, `hearth-clean-install.service` (enabled in the Anatase, Ubuntu, CachyOS
and Peppermint images; it only runs where `/usr/share/nimbus/image-info.json` exists and the plan is there):

1. **/etc** back to the new system's defaults, except accounts and passwords, drives, device name, machine id,
   time zone, language, keyboard, Wi-Fi networks and SSH host keys. Anything else is moved to
   `/var/lib/hearth/old-etc-<date>`. On Ubuntu, CachyOS and Peppermint the account files start from the new
   system's own, with your account, password and group memberships added (Fedora's system accounts don't fit
   there), then `systemd-sysusers` and `ldconfig` run.
2. **/var**: Bazzite's preinstalled Flatpak apps and Bazzite-only folders are removed.
3. **Home**: only games stay where they are: Steam's `steamapps` and `userdata` (and `~/.steam`'s links to it),
   Hearth's Epic and GOG games and their Wine prefixes, Heroic's and Lutris' games, and games added by hand.
   Everything else goes to `~/.old-hearth-<date>` for 30 days. Settings › System has "Restore a folder" and
   "Delete now".

Every step can run again after a power cut; the log is `journalctl -u hearth-clean-install`.

## Systems switched from another edition

Settings › Updates › Switch edition (`bootc switch`) changes the system, not the boot loader. A system installed
as Peppermint, Ubuntu or CachyOS keeps that edition's GRUB and `grub.cfg` after switching to Anatase, and those only
read the menu `hearth-bls-grub` writes. That script is now in every image, Anatase's too: there it checks whether
`grub.cfg` reads `hearth-bls.cfg` and does nothing on Anatase's own installs. Before this, a switched system's menu
froze at the switch, and its first update left GRUB pointing at a version that was gone. The boot count works the
same on switched systems (it only needs `custom.cfg`).


## Not verified yet

These come from Anatase's description and haven't been checked on a running Anatase system:

- the unit and file names (`hhd.service`, `HHD_BOOTC*` / `HHD_GS_AUTOLOGIN`, Plasma Login Manager's
  `plasmalogin.conf` and greeter process name);
- that Anatase's kernel boots through the Bazzite-based system's boot loader with Secure Boot on (Fedora's
  signing key); the Ubuntu and Peppermint images' kernels are signed by Canonical and Debian, which Fedora's shim
  doesn't trust, so moving there with Secure Boot on may also need it turned off;
- moving with packages layered on top (`rpm-ostree rebase`) to the images that have no rpm database (Ubuntu,
  CachyOS, Peppermint).
