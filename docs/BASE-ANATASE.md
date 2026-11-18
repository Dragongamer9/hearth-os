# Hearth OS on Anatase

**Back as a test (1.15, the owner, Nov 17: "Bring Anatase back and up to speed on 1.15 so i can test it").** Hearth OS
on Debian stays the main edition: the installer, the website and every Debian device are unchanged. Anatase is offered
as **Hearth OS on Anatase (test)** in Settings › Updates › Switch edition, to try beside Debian on the owner's own
devices. It ran smoother than Debian for him; he left it because Debian lets him unlock the file system, so Anatase now
has that too.

| | Hearth OS on Anatase (test) |
|---|---|
| Base | Anatase (`i.anatase.org/anatase:stable`, Fedora 45 or newer), bootc, composefs |
| Image | `ghcr.io/dragongamer9/hearth-os:test-1.15` only (never `:latest` or `:testing`, which hold the old pre-archive build) |
| Game Mode | Steam Game Mode with Hearth Sync (Decky built in), Hearth Game Mode as on Debian |
| Desktop Mode | stock KDE Plasma (plasmashell on KDE's own KWin) with Hearth's apps in Plasma's menu; "Back to Game Mode" in the menu and on the taskbar (Super+G) |
| Not here | Mantel and labwc (archived on Anatase), Firehold (its shell needs Mantel; a later stage is Firehold as a Plasma shell), Hearth's own login screen (Anatase's stays) |
| Unlock the file system | Settings › Updates & Recovery, off unless you turn it on (below) |
| Workflow | `.github/workflows/base-anatase.yml`: by hand; publishes only from main, only with publish ticked, only to the test tag |

## Desktop Mode: KDE Plasma

`nimbus-session` starts Plasma (`run_plasma`) when the image says so (`/usr/share/nimbus/desktop-mode` is `plasma`;
the Debian image has no such file, so Debian keeps Mantel). Plasma gets its own desktop names (`XDG_CURRENT_DESKTOP=KDE`)
and Fedora's menu file (`XDG_MENU_PREFIX` when `applications.menu` has a prefix), and Hearth's one-app-of-each-kind
copies (made for Mantel, they hide Hearth's apps from "KDE") are left out, so Hearth's apps and KDE's both show.

- **Back to Game Mode**: `hearth-gamemode.desktop` ("Back to Game Mode", Super+G), pinned first on Plasma's taskbar once
  per account (`hearth-game-mode-launcher.js`, a Plasma update script). It runs `nimbus-mode game`, which logs Plasma out
  through `org.kde.Shutdown` (and ends it after 10 s if an app holds it up); the session then starts Game Mode.
- **A Plasma that can't start** (ends within 20 s without asking for a mode): Steam Game Mode, never no screen.
- **Your Desktop Mode choice from Debian** (Mantel on labwc, KWin or Firehold, in `session.conf`) is never read or
  changed here, so it's still yours when you go back to Debian. Plasma says once that it took its place.
- **Screen scale**: KDE's own display settings own it. Only once per account, before KDE has saved any screen settings,
  and only on the ROG Ally family, the built-in screen starts at 150% (KWin picks 210% there by itself); nothing from
  Mantel's saved settings is read.
- **Hearth's apps without Mantel**: Settings (opens the Hearth app's Settings), Logs and the other Hearth Tools, Send logs,
  Disks, Text, Photos, Documents, Player, Calculator, Time, Contacts, Maps, Office, Paint, Recorder, Camera, Share,
  Nearby Share, Hearth Connect, Hearth Link, Controller Lab, Saves, Mods, Hearth Games, Hearth Browser, Bazaar (the app
  store), and Hearth's KDE apps (Hearth Files from Dolphin, Terminal, Text, Reader, Capture, Calculator).
  **Mantel's own parts that aren't here** (KDE's take their place): Task Manager (KDE's System Monitor), Quick Settings,
  notifications, the on-screen keyboard, the classic Hearth Files window and Hearth's own file chooser (Mantel's QML
  picker: GTK's and KDE's dialogs instead), Mantel's Capture, Restore and Reliability windows, the Resource Monitor.

## Switching editions (both ways, red flag 2)

Settings › Updates › Switch edition (`hearth-admin move-to-new-os --edition anatase|peppermint`):

- **Debian → Anatase (test)**: the Debian version you're on is **pinned** (`ostree admin pin`), so it stays in the boot
  menu however many Anatase updates come. The kept container images are dropped first (composefs and SELinux: layers
  Debian's ostree stored came out without SELinux labels and Anatase started to a black screen from them). Your **home
  folder is left exactly as it is** (a test switch: `keepHome`). At Anatase's first start `hearth-clean-install` sets /etc
  up for Fedora (accounts merged, the rest of Debian's /etc set aside in /var/lib/hearth/old-etc-<date>, Hearth's own
  settings in /etc/hearth kept), relabels /etc and /var for SELinux, and the home folders in the background.
- **The boot menu**: a PC that came from Debian keeps Debian's signed shim and GRUB with Hearth's own menu. Anatase keeps
  that menu following its updates and a rollback (`hearth-bls-grub`, only where `/boot/grub2/hearth-bls.cfg` exists), and
  bootupd never writes Fedora's loader over Debian's there. Secure Boot: Debian's shim doesn't start Fedora's kernel, so
  Settings says to turn Secure Boot off first when it's on.
- **The EFI partition** (the owner's laptop, Nov 17: after the move from Debian to the old Anatase build, Fedora's
  bootupd updated the boot loader by itself, removing Debian's shim and GRUB and putting Anatase's in EFI/anatase
  without grub.cfg and bootuuid.cfg, so the PC stopped at `grub>`). On a PC whose menu is Hearth's, `bootupctl` never
  updates or adopts the boot loader (a wrapper; `HEARTH_BOOTUPD_ALLOW=1` overrides), so Debian's loader stays, and
  `hearth-esp-guard` gives any loader folder that lacks them the grub.cfg and bootuuid.cfg that find /boot's menu (which
  also repairs a PC that already got there). CI proves it in a virtual PC: Debian 1.13 → Anatase → Debian, every
  restart through the firmware with nothing pressed (`.github/scripts/edition-switch-vm.sh`).
- **Anatase → Debian**: the very Debian version you came from starts again, as you left it (`bootc rollback` when it's
  the previous one, else `ostree admin set-default` to it). Only when it's gone (or an update is waiting for a restart)
  is Debian fetched fresh, again keeping the home folder.
- A PC switched by hand from a Debian without this Hearth (`sudo bootc switch ...`, no plan) is recognised at Anatase's
  first start by Debian's accounts in /etc/passwd, and gets the same /etc set-up (never the home folder).

## Unlock the file system

The owner's main need. Anatase's `/usr` is a read-only composefs image. What Fedora bootc offers and what each costs:

| Way | Survives a restart | Survives an update | Cost |
|---|---|---|---|
| `bootc usr-overlay` (= `ostree admin unlock`) | no | no | a temporary layer in memory |
| `ostree admin unlock --hotfix` | yes | no | the whole deployment cloned; the next update drops it |
| `rpm-ostree install` (layering) | yes | yes | a new deployment per change, a restart each time, slower updates; can block updates when Fedora's packages move |
| **Hearth's unlock** (`hearth-unlock`, dnf form) | **yes** | **dnf packages: put back by themselves; files copied by hand: no** | one overlay over /usr per version; a reinstall after each update (in the background, online) |

So Hearth OS on Anatase gets the same design as Debian's writable system, in Fedora's form
(`os-bases/anatase/system_files/usr/bin/hearth-unlock`):

- **Off unless you turn it on** (`/etc/hearth/system-unlocked`, written only by the switch: Settings › Updates &
  Recovery › Unlock the file system, or `sudo hearth-unlock`). It never flips by itself (red flag 9).
- Unlocked, each OS version gets its own writable layer over /usr (`/var/lib/hearth/unlock/layers/<commit>/upper`,
  mounted early at every start). `dnf install` and rpm files install anywhere; rpm's database lives in /usr, so it always
  matches its layer.
- **After a restart**: everything you changed in /usr is still there.
- **After an update**: the new version starts as the clean new image, then what you installed with dnf (and rpm files
  installed through Hearth, which keeps a copy) is installed again in the background once online
  (`hearth-unlock-reapply.service`). Files you copied into /usr by hand are not carried over: they stay in the old
  version's layer. A package that can't come back is listed in a notice.
- /etc and /var are writable anyway and kept across updates, as on every bootc system.
- **Going back** (rollback, the automatic fallback, Recovery) starts the previous version with its own layer, as it was.
- **Safe start**: a start with your changes that never reached Hearth (45 s) means the next start is without them, and
  Hearth offers to turn them back on. The boot menu writers, the fallback, the update's finalizing and Recovery run from
  the image's own /usr.
- **The boot chain** (kernel, GRUB, shim, dracut, ostree, bootc, bootupd) never comes from dnf: excluded in
  `/etc/dnf/libdnf5.conf.d/20-hearth-boot-chain.conf`, and every plan is checked first.
- CI proves it in the built image (`.github/scripts/anatase-unlock-check.sh`): locked by default, unlock, dnf and an rpm
  file into /usr, a restart, an update with the reapply, a hand-copied file staying behind, a rollback, lock/unlock, a
  safe start, the boot chain excluded.

## Trying it on a device

Publishing needs the owner's go-ahead (red flag 5). With it: Actions › **Hearth OS (Anatase base, test)** › Run workflow,
with **test_tag** `test-1.15` (any branch; only `test-<version>` is accepted), or **publish** ticked on `main`. The image
job builds, runs every required check, and only then pushes that same image to `ghcr.io/dragongamer9/hearth-os:test-1.15`
and a dated `test-1.15-<date>` copy, and checks both tags have the digest it pushed. Nothing else is ever pushed.

On the device (ROG Ally or the laptop), from Hearth OS on Debian:

- **From a Debian with 1.15's Switch edition**: Settings › Updates › Switch edition › **Hearth OS on Anatase (test)**.
- **From Debian 1.13** (its Switch edition refuses Anatase and doesn't know the test tag): the same move with 1.15's own
  `hearth-admin`, taken from the repository at a fixed commit (it pins Debian, forgets the kept container images,
  leaves the home folder as it is and switches to `:test-1.15`; nothing changes until the restart):

  ```
  curl -fsSL -o /tmp/hearth-admin https://raw.githubusercontent.com/Dragongamer9/hearth-os/<commit>/os/system_files/usr/libexec/nimbus/hearth-admin
  sudo PKEXEC_UID=$(id -u) python3 /tmp/hearth-admin move-to-new-os --edition anatase
  ```

- If Secure Boot is on, turn it off in the firmware first (Debian's shim won't start Fedora's kernel). Then restart. The
  first start takes a little longer (/etc set up for Fedora, SELinux labels).

**Back to Debian**: Settings › Updates › Switch edition › **Hearth OS on Debian** (the same Debian, as you left it), or
in a terminal on Anatase `sudo bootc rollback` (when Debian is the previous version) and restart, or pick the Debian
entry in the boot menu once. Debian stays pinned in the boot menu the whole time.

## What was here before (kept for reference: the archived 1.0 Anatase)

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
- **Hearth Plasma.** KDE Plasma 6 (the workspace only) is installed as Desktop Mode's second desktop, in Hearth's look; Hearth Classic (labwc + Mantel) stays and is the fallback. See docs/archive/PLASMA.md.
- **Hearth GNOME.** GNOME Shell (minimal: shell, session, Settings, portal, plus GNOME Files, Console and Text Editor shown only there) is installed as Desktop Mode's third desktop, with Hearth's own Shell extensions; no GDM. See docs/archive/GNOME.md.
- **Which edition this is** is in `/usr/share/nimbus/image-info.json` (`{"base": "anatase", ...}`), read by
  `bridge.rs` `edition()`. The Bazzite-based image has `/usr/share/ublue-os/image-info.json` instead.

## Moving from the Bazzite-based Hearth OS

Settings › Updates › **Move to a new Hearth OS** (only on the Bazzite-based Hearth OS, 0.1.3 and later):

| Choice | Image | |
|---|---|---|
| Hearth OS on Anatase | `ghcr.io/dragongamer9/hearth-os:latest` (`:testing` with `--testing`) | Recommended |
| Ubuntu | `ghcr.io/dragongamer9/hearth-ubuntu:testing` | Beta Experimental |
| CachyOS | `ghcr.io/dragongamer9/hearth-cachyos:testing` | Beta Experimental, x86-64-v3 processors only |
| Peppermint | `ghcr.io/dragongamer9/hearth-peppermint:testing` | Beta Experimental; writable by default (can be locked in Settings) |

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

## Not verified yet

These come from Anatase's description and haven't been checked on a running Anatase system:

- the unit and file names (`hhd.service`, `HHD_BOOTC*` / `HHD_GS_AUTOLOGIN`, Plasma Login Manager's
  `plasmalogin.conf` and greeter process name);
- that Anatase's kernel boots through the Bazzite-based system's boot loader with Secure Boot on (Fedora's
  signing key); the Ubuntu and Peppermint images' kernels are signed by Canonical and Debian, which Fedora's shim
  doesn't trust, so moving there with Secure Boot on may also need it turned off;
- moving with packages layered on top (`rpm-ostree rebase`) to the images that have no rpm database (Ubuntu,
  CachyOS, Peppermint).
