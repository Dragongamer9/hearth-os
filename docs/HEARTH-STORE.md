# Hearth Store (Hearth OS 1.16, Fedora edition)

The Hearth Store is where Dragongamer9's own apps that aren't part of Hearth OS are installed, updated, checked and
repaired, and uninstalled: Hearth AI Command Center and Hearth AI today, more later. The apps that come with Hearth OS
(Logitech G15, Beacon, Hearth USB maker) are listed too, marked **Built in**: they update with Hearth OS. Flathub's
apps stay in Settings › Apps › App store (its page has a "Hearth's own apps" row that opens the Store, and the Store's
home has "Looking for other apps? Open App store").

- App: `hosts/hearth-ui/apps/store` (pure Rust, hearth-ui), the program `hearth-store-app`, menu entry "Hearth Store".
- Approved mockups: `docs/mockups/hearth-store/store-1…9.png`, checklist `docs/mockups/checklists/hearth-store.md`
  (the plan and the owner's decisions: hearth-workspace `hearth/mockups/hearth-store/README.md`).
- Only in Hearth OS, Fedora edition (the owner, Nov 19: "Build it to where it's built into Hearth OS and not available
  elsewhere"; "Not offering anything Debian"). It's in the image, updates with Hearth OS, and has no version, download
  or AppImage of its own.

## How an app is installed

On every app's page: **Installs as: AppImage in your home folder**. No password, no restart, nothing on the system.

| What | Where |
|---|---|
| The app | `~/.local/share/hearth/store/apps/<app>/<App>.AppImage` (no version in its name) and `store.json` (what was installed: version, SHA-256, link) |
| Its menu entry and icon | `~/.local/share/applications/hearth-store-<app>.desktop`, `~/.local/share/icons/hicolor/scalable/apps/hearth-store-<app>.svg` |
| What the Store saw last | `~/.cache/hearth/hearth-store/` (releases.json, pictures, a refreshed list) |
| The Store's settings | `~/.config/nimbus/hearth-store.json` (written only when you change one, through settings_file.rs) |

- **Install**: the release's AppImage is downloaded as a hidden file in the app's folder, its SHA-256 is checked against
  the release's `SHA256SUMS.txt` (a file that doesn't match is deleted, and nothing changes), then it's put in place
  with one rename, with its menu entry and icon.
- **Update**: the same download next to the old file, which keeps working meanwhile. If the app is open it's asked to
  close (it saves first), then the new file takes the old one's place in **one rename**, so the app is never left half
  updated. The old version is kept (`<App>.AppImage.previous`) until the new one has started fine. Cancel works until
  that swap.
- **Check and repair**: the installed version's AppImage against its release's SHA-256, that it's allowed to run,
  that AppImages can start (FUSE; without it they're unpacked to run), its menu entry and icon. Only what's wrong is
  fixed: the same version is downloaded again, checked and put back. A repair never changes versions, and never
  touches the app's settings or data.
- **Uninstall**: "Keep my settings and data" is on by default. The app's folder, menu entry and icon go; with the
  switch off, its own folders (from the list of apps, e.g. `~/HearthAI`, `~/.config/Hearth AI`) go to the **Trash**,
  never deleted.

Why AppImages and not .rpm, system extensions or Flatpaks: rpm-ostree / bootc layering makes a new boot entry and
needs a restart, and a systemd-sysext is merged at every start and needs root (both touch starting, which is a red
flag); the apps make no Flatpaks, and Command Center's own updater only knows the AppImage.

## Apps that update themselves (Command Center)

The owner (Nov 19): "Leave it to the app". The Store installs Command Center's AppImage from its GitHub release (checked
against the release's `SHA256SUMS.txt`), under a name without a version (`Hearth-AI-Command-Center.AppImage`), in your
home folder, so the app's own updater (`electron/updater.cjs`, electron-updater) replaces it in place exactly as it
does today. From then on the Store never replaces its files: it shows **Updates itself**, and when there's a newer
release its button is **Open to update**. When the file has changed (the app updated itself), the Store reads its
SHA-256 once, matches it with the newest release and shows that version.

## The list of apps

- Built in: `/usr/share/hearth/store/catalog.json` (source: `os/system_files/usr/share/hearth/store/catalog.json`),
  updated with each Hearth OS release. Raise its `serial` whenever it changes.
- Refreshed between releases from `https://hearthos.fun/store/catalog.json` only when it's **signed** by the owner's
  key and its serial is higher. The public half of the key is built into the image
  (`/usr/share/hearth/store/catalog-key.pub`, 64 hex characters). Without that file the Store never asks for a list
  from the web: it uses the built-in one. A list that isn't signed, is signed by another key, or was changed is ignored.
- The website workflow (`.github/workflows/site.yml`) publishes and signs the list from the repository secret
  `HEARTH_STORE_SIGNING_KEY`; without the secret it publishes nothing (the Store is fine with the built-in list).
- **The live list is the `store` branch's `catalog.json`** (1.17), so apps are added or changed without a Hearth OS
  release: change it there, raise its `serial` by one, then Actions › **Publish the Hearth Store list** (only on the
  owner's okay). That checks it (`tools/store-check.py`: valid, a serial higher than the published one and the
  built-in one, every app whole) and starts the website workflow on main, which signs and publishes it. Every website
  publish signs whichever of the branch's list and the built-in list has the higher serial, and each Hearth OS release
  brings the built-in list up to the branch's.
- Each app's newest version comes from its **public GitHub release page** (no account, no sign-in, no token): stable is
  the newest published release (never drafts; pre-releases only with "Get test versions" on). The list may instead
  pin a version with its SHA-256 (`"pin": {"version", "url", "sha256", "size"}`).

### Setting up the signing key (the owner only; nobody else makes or sees the private key)

1. On your own PC: `openssl genpkey -algorithm ed25519 -out hearth-store-key.pem` (keep this file private).
2. The public half for Hearth OS:
   `openssl pkey -in hearth-store-key.pem -pubout -outform DER | tail -c 32 | od -An -v -tx1 | tr -d ' \n'`
   and put those 64 characters in `os/system_files/usr/share/hearth/store/catalog-key.pub` (it ships
   with the next Hearth OS release).
3. In GitHub: Settings › Secrets and variables › Actions › New repository secret `HEARTH_STORE_SIGNING_KEY`, with the
   whole content of `hearth-store-key.pem`.

## The private list (only on the owner's own devices)

The owner (Nov 19), about Hearth OS CAD and the Trains 2010 remake: "Only on my own devices". They are never in the
public list or on the website. On his devices he adds `~/.config/nimbus/store-private.json` by hand (nothing in
Hearth writes or publishes it):

```json
{"apps": [
  {"id": "hearth-cad", "name": "Hearth OS CAD", "card": "3D design with a slicer for your printer", "icon": "hex", "from": "#38bdf8", "to": "#0e7490", "kind": "coming"},
  {"id": "trains-2010", "name": "Trains 2010", "card": "The train sim, made native for Linux", "icon": "T", "from": "#f97316", "to": "#c2410c", "kind": "coming"}
]}
```

When one of them has public releases, its entry gets `"kind": "install"`, `"github": "owner/repo"` and `"file"`, the same
as the public ones. Private apps are marked "Only on your devices" on their page and never replace a public app.

## Finding updates (light at rest)

- When the Store opens (at most every 15 minutes; what was seen is shown meanwhile, "Checked for updates 2 min ago").
- At most once a day from the user timer `hearth-store-check.timer` (`hearth-store-app --check`, no window): it does
  nothing while a game runs (gamescope, or Hearth's hearth-playing mark), on a metered connection, or when Store
  settings says "Only when the Store opens". A few small requests (each app's release page and its SHA256SUMS.txt; the
  signed list when the image has the key). One notice per new version ("Hearth AI 0.6.0 is ready") when "Tell me when
  an update is ready" is on (the default, the owner, Nov 19). No service runs at rest; nothing polls.
- Downloads wait on metered connections unless "Download on metered connections" is on. Offline, the Store shows the
  list it saw last, says when, and installs and updates wait.

## Hearth AI's releases

The owner (Nov 19): "Add Hearth AI releases". `Hearth-Organization/Hearth-AI` has `.github/workflows/release.yml` on its
branch `ci/releases` (not merged, never run): a version tag the owner pushes (his number, matching package.json) builds
and tests her and makes a **draft** release with the AppImage, the .deb, `latest-linux.yml` and `SHA256SUMS.txt`;
nothing is public until he presses Publish. Until Hearth AI has a published release, the Store shows her as "Coming".

## Hearth's own parts, and what never is one

Every Hearth part (apps, Mantel, Game Mode, Decky plugins, Hearth Link, helpers) updates from the Store with a main
and a test version (docs/STORE-SYSTEM-PARTS.md). From the Hearth OS after 1.18 (the hooks are in the image), so are
**gamescope**, the **session start** and **Hearth's own login screen** (the owner, Nov 26: "Yep add it"), each on trial
with the built-in copy to fall back to at once ("Starting up" there). The **boot menu, Safe start and Recovery** are
never Store parts: they change only with Hearth OS updates, which keep the version before startable. A part whose
update takes effect only after signing out or a restart says so ("restart": "signout" or "pc" in its list entry):
"Takes effect after you sign out / restart" on its page and before installing, then **Sign out now / Restart now** or
**Later**, never by itself and never while a game runs. **System packages** (Debian's own fixes, in the writable system,
with the admin password) has its own row with the same restart notice; the kernel and drivers aren't in it (they stay
pinned and come with Hearth OS).

## Safety

- Never touches boot files, the boot menu, sessions or services (the daily timer only reads release pages and writes
  the Store's cache).
- Never changes a setting by itself; opening it writes none (CI checks).
- Only https downloads, each checked against its release's SHA-256 before it's used.
- Nothing secret is stored, needed or sent.

## Pictures in CI

`hearth-apps-rust.yml` pictures every page on a pretend world (`.github/scripts/hearth-store-fixture.py`,
`HEARTH_STORE_FIXTURE`): `.github/scripts/hearth-apps-rust-shots.d/store.sh` (store-1…13). Tests: `cargo test` in
`hosts/hearth-ui/apps/store` (signature, SHA-256 check, the one-rename swap, repair, uninstall to the Trash, settings).
