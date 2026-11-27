# Every Hearth part in the Hearth Store (1.18 plan)

The owner, Nov 23, 2026: "Every thing in Hearth OS that we made Needs to be in the hearth Store so it can receive
updates without system updates", then "Yes work on it" and "1.18 is the version and everything in the hearth store gets
bumped to 1.18.0". Approved mockups: docs/mockups/checklists/store-everything.md.

## What's in the Store already, and what isn't

| Kind | Today | 1.18 |
|---|---|---|
| Apps run as you (Files, Settings, Task Manager, Capture, Notes, G15, Family, Server, Resource monitor, System Restore, Reliability, Ideas, file chooser, controller, the Store) | Store parts (hearth-part, in your home) | the same, 1.18.0 |
| Mantel, Mantel in Rust (with their own hearth-layout) | Store parts | the same, 1.18.0 |
| Hearth's own Game Mode (hearth-rs and ui/), Hearth Sync, the Decky plugins | system update only | **user parts** (they run as you): like the apps |
| Hearth Link, Hearth's gaming pieces, looks and sounds | system update only | **user parts** |
| Helpers that run as root (home server, storage, AI engines, sharing… /usr/libexec/nimbus) and Hearth's services and timers | system update only | **system parts** (below) |
| Boot menu, Safe start, Recovery | system update only | **stays with system updates** (red flag 2: the one before always stays startable); a way to push a fix when it's needed is planned below ("The boot menu and Recovery") |
| gamescope, the session start, Hearth's own login screen | system update only | **Store parts on trial** (the owner, Nov 26: "Yep add it. That will allow us to push updates to it"): "Starting up" below |
| Fedora underneath | Hearth OS image updates | **Fedora updates, by date** (Library › Hearth OS), applied at the next restart; nothing by itself |

## System parts: safe for programs that run as root

A user part lives in `~/.local/share/hearth/parts`, which any program running as you can change. A program that runs
as root must never run a file from there: that would let anything you run become root. So system parts are their own
kind:

- **Where:** `/var/lib/hearth/system-parts/<id>/<version>/`, owned by root, never writable by anyone else.
- **Installing:** only `hearth-system-part install` run as root (the Store asks through polkit, admin password, the
  same rule as Hearth's other root helpers). It downloads, checks the SHA-256 against the list signed with the owner's
  key (the same list as user parts), unpacks into a new folder, checks every file is root's and not group- or
  world-writable, then switches `current` in one rename. The version before stays next to it.
- **Using:** each root helper and service starts through `hearth-system-part run <id> <built-in>`: the Store's copy
  only when it's root's, made for this Hearth OS, and hasn't failed; otherwise the built-in one in /usr. Anything
  wrong means the built-in one.
- **Services:** a part with services restarts them after installing (no restart of the PC). One that fails to start
  twice goes back to the version before by itself and says so (the Store's Hearth parts page).
- **Never:** the units in the boot chain, the boot menu, Safe start and Recovery are system parts. Those come only
  with system updates. The login screen's own screens are, on trial (below).

## Versions

The owner picks every number. At the 1.18 release every part goes to **1.18.0** (Nov 23); after that each part counts
its own updates (1.18.1, 1.18.2…), and each keeps a main and a test version (red flag 13).

## Steps

1. User parts for Game Mode, Hearth Sync, the Decky plugins, Hearth Link, gaming, looks and sounds (quick-update.yml
   options and their start lines, like the apps').
2. `hearth-system-part` and its polkit rule; the root helpers and services start through it; tests that a part in
   your home is never run as root and that a part not owned by root is never used.
3. The Store: the Hearth parts page grouped as the mockup (Desktop, Apps, Game Mode, Services, Looks and sounds), the
   system parts asking for the admin password, Library › Hearth OS with Fedora updates by date.
4. The release: every part built and listed at 1.18.0 with the Hearth OS 1.18 release, when the owner says push.

## Built (Nov 24, feature/store-118)

- **Step 1:** quick-update.yml has every Hearth part with a main and a test channel: the apps, Mantel, Mantel in Rust,
  Desktop layouts (hearth-layout), the Game Mode host (`hearth-game-mode`: /usr/bin/nimbus starts the Store's copy
  through hearth-part only when one is installed, else /usr/lib/nimbus/nimbus as before), Hearth Link, and the three
  root helpers as system parts. Every part is built in Debian trixie, the image's own base (part-build-tools.sh).
  Looks, sounds and the AI engines live in Mantel's folder, so they come with Mantel's part; Hearth core is built into
  the Game Mode host and the apps, so it comes with them; the Store says so ("with Mantel").
- **The Decky plugins are not home parts:** Decky runs them from a root process, so a copy from your home folder
  would be a way to root. Their main versions keep their own channel (hearth-decky plugins-*, the plugins release,
  checked and put in place by root), which the Store shows and starts. Their **test versions** are system parts
  (quick-update.yml, Channel: test): installed root-owned with the admin password, only the copy the signed list names;
  at Decky's next start hearth-decky puts that copy in Decky's folder on trial (apply_system_part), and one that gives
  no load mark goes back to the one before by itself and is never used again (`hearth-system-part bad`).
- **Gaming pieces:** Controller Lab, Saves, Mods, Hearth Games, the colour filter (hearth-gamma) and the lightweight
  Steam engine (hearth-steam) are user parts with main and test versions; their /usr/bin starts (and the Steam
  engine's service) use the Store's copy only when one is installed.
- **Step 2:** `hearth-system-part` (path, run, install, remove, retry, status), `system-parts.json` (only the home
  server, storage and AI sharing helpers), hearth-admin `system-part-install ID [test]` (admin password,
  49-hearth-admin.rules); the three helpers re-start through it only when a copy is installed. Tests:
  hosts/linux/test/test_hearth_system_part.py. hearth-part never uses a home copy when run as root.
- **Step 3:** the Store's Hearth parts page, Library › Hearth OS and Library while updating (checklist
  store-everything.md). On Debian the system's updates are newer Hearth OS images named by date (bootc, through
  hearth-admin update); "by themselves" is off unless turned on (`hearth-admin auto-update`, /etc/hearth/auto-update:
  bootc's timer runs only then), and nothing ever restarts the PC by itself.
- **Waiting:** the `store` branch's live list needs the new entries (groups, system parts) before 1.18.0 parts are
  published; the release workflow should fill the list's `system` block (date, packages, what they fix, size, the
  package changes) for each dated image.

## Starting up: gamescope, the session start and the login screen (Nov 26, feature/store-session-parts)

The owner, Nov 26: "Need to add this to the store as well", then "Yep add it. That will allow us to push updates to
it". Three pieces of the start are Store parts, each with a main and a test channel (quick-update.yml), each with the
copy built into Hearth OS kept as it is and used whenever the Store's copy can't be:

| Part | quick-update.yml | Kind | Store group | Takes effect | Its hook (in the image) |
|---|---|---|---|---|---|
| gamescope (Game Mode's screen) | `gamescope` | user part, `~/.local/share/hearth/parts/gamescope` | Game Mode | after you sign out | nimbus-session's `# >>> gamescope part`: Game Mode's gamescope (Steam's and Hearth's) starts through `hearth-part trial` only when a Store copy may be used |
| Session start (nimbus-session: opens Game Mode or Desktop Mode after you sign in) | `hearth-session` | user part | Game Mode | after you sign out | nimbus-session's first lines (`# >>> session part`): the built-in one starts the Store's through `hearth-part trial` |
| Login screen (Hearth's own greeter: its QML and greeter-info) | `hearth-greeter` | system part, root-owned in `/var/lib/hearth/system-parts/hearth-greeter`, admin password | Services | after a restart | hearth-login-screen asks `hearth-system-part path` at each start and gives greetd `hearth-greeter <folder>`; hearth-greeter takes only a system part's folder |

**How each falls back (red flag 2):**

- **gamescope and the session start** (`hearth-part trial`): tried only when the copy was installed by the Store from
  the signed list, is made for this Hearth OS, hasn't failed, and (a program) every library it needs is on this
  system (ldd). It ends with an error before it came up → the built-in one starts at once with the same request. It
  doesn't come up in time (gamescope: starts nothing, Xwayland or the game, within 30 s; the session: no gamescope,
  labwc, KWin or cage of yours within 60 s) → it's stopped (its whole process group) and the built-in one starts. The
  start before with that copy never got as far as "good" (switched off on a black screen, a freeze) → the built-in one.
  It crashes soon after coming up → the built-in one from the next start. Each of these marks the copy as not working
  (`fails` 2 in `~/.local/state/hearth/parts/<id>.json`, so the Store shows "back to the one before"): it's never
  started again until the Store installs another version or you press **Try it again**. You're told: the session log
  (Send logs) and a notice that waits for the desktop's notifications (up to half an hour). An end that was asked for
  (signing out, shutting down, a switch of modes) never counts. After coming up and running a while (gamescope 20 s,
  the session 45 s) nothing is watched any more: no wake-ups while you play. Never for root, an extra multi-seat
  player or Game Mode beside the desktop.
- **Login screen** (hearth-login-screen): the Store's greeter is chosen only when hearth-system-part allows it
  (root's, writable by nobody else, made for this Hearth OS, not marked bad, its files there). A start with it that
  doesn't show, crashes, never gets anyone signed in, or never finishes (power button held) is the copy's failure:
  SDDM for that start as always, the copy marked `bad` (`hearth-system-part bad`), the built-in greeter from the next
  start, and it **never counts towards pausing** Hearth's login screen (that count is only for the built-in one, which
  works exactly as before: two failures in a row and SDDM until you turn it on again). You're told after you sign in
  (hearth-login-notice), with Try it again in the Store. SDDM itself, the display manager, isn't a part.
- Nothing here changes a setting: the only things written are the copy's own failure marks, each with a way back.

**Tests and CI:** `hosts/linux/test/test_store_start_parts.py` (the trial for real in child processes, the session's
and gamescope's hooks, the login screen with the real hearth-system-part); `store-start-parts.yml` (gamescope built as
the part is built, then started headless in Debian trixie: no copy, the CI-built copy, copies broken on purpose; the
login screen drawn by cage and Quickshell from a good Store copy, a broken one, then the built-in one; the Store app's
tests); base-peppermint.yml's virtual PC (`debian-desktop-vm.sh`): after the desktop test, a session start that never
brings a screen up and a gamescope that ends at once are installed in the account's home, the session is ended, the
automatic sign-in comes back, and a screen must come up through the built-in ones with the copies marked.

**Devices already on 1.18 can't use these three until the Hearth OS version that has the hooks.** 1.18's
`/usr/bin/nimbus-session`, its gamescope start and its hearth-login-screen never look for a Store copy, and the only
way to change them on a running 1.18 PC is `hearth-admin deb-install` into the writable system: that would replace the
image's own session start in place (the very copy this design promises never to replace), be put back by hearth-unlock
over every newer Hearth OS after an update, and put a new step in front of signing in on every device at once. That
isn't safe enough for the start-up chain, so the hooks are image-only: they come with the next Hearth OS version
(publish these parts with that version in "os"; 1.18's Store doesn't offer a part that isn't made for it). From then
on, every update to the three is a Store update.

**When it takes effect** (the owner, Nov 26: "if somthing needs a PC restart it will tell the user"): a part's entry in
the Store list can say `"restart": "signout"` or `"pc"` (quick-update.yml sets it per part: gamescope and the session
start sign out, the login screen restart). The Store shows "Takes effect after you sign out / restart" on the part's
page and its row, says it when it starts installing, and afterwards asks **Sign out now / Restart now** or **Later**;
it never signs out or restarts by itself, and not while a game runs. The copy you have keeps working until then.

## The boot menu and Recovery (a plan, not built)

The owner, Nov 26: "Yeah boot menu and recovery are part of Major updates but, should have it to were if we need to
push an update we can". Not built in this branch, because it couldn't be proven safe here; the plan:

- **Use what's there, not a new mechanism.** The boot menu (GRUB's entries, written by `hearth-bls-grub` from
  ostree's deployments) and Recovery (`hearth-recovery`, in each deployment's `/usr`) already update **beside** the
  current ones: a new Hearth OS image is staged by bootc as a new deployment, the running one stays as the previous
  entry, and `hearth-boot-guard` counts starts in GRUB's environment and starts the previous version by itself after
  two that never reached Hearth (45 s of Game Mode, Desktop Mode or the login screen), then makes it the default and
  pauses updates. Rewriting GRUB's files or Recovery in place from the Store would skip all of that.
- **So a pushed fix is a small dated Hearth OS image** (the same version, a new date: Library › Hearth OS already
  lists updates by date) that the Store offers as its own row ("Boot menu and Recovery fix"), installed with the
  admin password through `hearth-admin update`, with the restart notice (Restart now / Later, never by itself, not
  while a game runs). If the PC doesn't reach Hearth after that restart, the boot guard starts the version before by
  itself; it already says so after signing in.
- **To prove before building:** boot-guard.yml and base-peppermint.yml's boot-look VM with the new image's GRUB
  snippet and Recovery broken on purpose, still reaching the login prompt on the previous entry; the owner's go-ahead
  for each such image, as for a release (red flag 1 and the release rules).

## System packages: Debian's own updates (Nov 26, feature/store-session-parts)

The owner said yes to a "System packages" part: Debian's fixes through the Store. Built, for the Hearth OS version
after 1.18 (hearth-unlock and hearth-admin are in the image):

- **What it is:** a row on the Store's Hearth parts page (Services: "System packages"). **Check for updates** asks
  `hearth-unlock --upgradable` (as you, read only, from apt's lists as they are: the fixes on offer, "N fixes").
  **Update** runs `pkexec hearth-admin system-packages-update` (always the admin password, 49-hearth-admin.rules) →
  `hearth-unlock --upgrade-system`: apt's lists refreshed, then `apt-get upgrade` in the writable system (only
  upgrades: nothing new, nothing removed), recorded like any apt change. Never by itself, and the Store refuses while
  a game runs.
- **What it can't change:** the boot chain stays pinned out of apt (GRUB, shim, kernels, ostree, bootc, dracut, DKMS,
  microcode, fwupd: `/etc/apt/preferences.d/hearth-boot-chain`; a plan that would touch one is refused) and systemd,
  glibc, D-Bus and Mesa are held at the image's versions (`hearth-image-core`). So **the kernel and graphics drivers
  don't come this way**: they come with Hearth OS images. What comes is Debian's other packages (security fixes in
  libraries and programs).
- **Taking effect:** after a restart. The Store says so and asks **Restart now / Later** (never by itself).
- **The version before stays startable:** the changes are in the writable system's layer for this version, so a
  start that doesn't reach Hearth leaves the layer off (Safe start, "Turn my changes back on"); a Hearth OS update
  starts from its own clean image.
- **Tests:** `test_hearth_unlock.py` (the plan, only upgrades, the boot chain refused, root only, the admin password);
  `hearth-unlock-check.sh` (boot-guard.yml on Debian 13 and base-peppermint.yml in the built image): the real
  `--upgradable` as an ordinary account, the real `--upgrade-system` with apt, nothing left on offer after it, /boot
  untouched, the next start still unlocked with your own packages.
