# Hearth app updates (without a system update)

Hearth's own app (the Hearth program with its screens, and its Chromium window app) can update by itself in a few
minutes, between Hearth OS system updates. The Hearth built into the system image is never changed: it's always
there, always startable, and always the fallback.

**The rule above everything:** an app update must never leave a device unable to start, and Game Mode must always come
up. Everything below exists to keep that true.

## What can and can't update this way

| Updates by itself (the app bundle)                           | Only with a system update (image-only)                        |
|--------------------------------------------------------------|---------------------------------------------------------------|
| The Hearth program (`nimbus`: hearth-rs with hearth-core, with Game Mode's screens from `ui/` built in) | Anything that runs as root or at boot: systemd units, `hearth-admin`, polkit rules, udev rules, `hearth-boot-guard`, Recovery |
| Hearth's Chromium window app (`electron/`: hosts/electron with its `resources/ui`, `resources/js` and Theater's files) | The session and login scripts: `nimbus-session`, `nimbus-mode`, `nimbus-watchdog`, the `.desktop` session files, SDDM |
|                                                              | Hearth's Decky plugins (their own channel: docs/PLUGIN-UPDATES.md) |
|                                                              | Desktop Mode (Mantel's QML and its `hearth-*` scripts in /usr/share/nimbus): they run before and beside Game Mode, outside the start counter below |
|                                                              | The Qt renderer (`/usr/lib/hearth-qml`, built against the image's Qt) |
|                                                              | Hearth's other apps (Terminal, Disks, Photos...), Hearth Browser, the kernel, drivers, the bootloader |

When the Qt renderer (Experimental) is chosen, Hearth always uses its built-in version.

## How it works

### On each release (release.yml, on main)

1. `app-bundle` builds the Hearth program for each edition exactly as that edition's image does (Anatase: the Ubuntu
   runner; Peppermint: inside Debian trixie), from the release's own commit, and runs the end-to-end test
   (`hosts/linux/test/test_hearth_app_channel.py`) with it.
2. `app-electron` builds the Chromium window app.
3. `app-pack` packs `hearth-app-<edition>-x86_64.tar.zst` with `tools/app-bundle.py` and writes its manifest,
   `hearth-app-<edition>-x86_64.json`:
   `{format, version, edition, arch, abi, min_image_version, max_image_version, commit, sha256, size, files: [{path,
   sha256, size, mode} or {path, link}]}`. The real Anatase bundle is installed and chosen once with a throwaway key.
   Then the manifest is signed (`ssh-keygen -Y sign -n hearth-app`, giving `.json.sig`) with the
   `HEARTH_APP_SIGNING_KEY` secret, if it's set and matches the image's public key.
4. `app-publish` adds the three files to the release that same run made (never to a release from another commit, never
   from a branch, never unsigned). Without the secret, the bundles are built and tested, and a notice says they
   weren't published. The release itself goes out either way.

The version is the release's own number (the owner's); nothing here picks a version number.

### The image's side (base-anatase.yml, base-peppermint.yml)

- `/usr/share/nimbus/version`: the built-in Hearth's version (already there).
- `/usr/share/nimbus/app-abi`: the image's app ABI tag, `tools/app-bundle.py abi`: the number in `os/app-abi` plus a
  fingerprint of the image-only files the app talks to (hearth-admin, the polkit rules, the session scripts). A bundle
  runs only on an image with exactly its tag, so it never meets image-side files it wasn't built with. Raise the number
  in `os/app-abi` when an app change needs something new from the image that the fingerprint can't see (a new system
  package, a new service).
- `/usr/share/nimbus/app-signing.pub`: the public key bundles must be signed with (a placeholder until the owner adds
  the real one; with the placeholder, the image never uses an app update).

### On the device (hearth-core appchannel.rs; everything as you, only in ~/.local/share/hearth/app)

Game Mode's Hearth checks the newest release 2 minutes after it starts and every 6 hours (Settings › Updates › Hearth
app updates: **Automatic** by default, **Ask**, or **Off**). It downloads the manifest and signature, checks the
signature, the edition, processor, ABI tag and image version range, and that it's newer than both the built-in version
and the current bundle; then downloads the archive into `staging/`, checks its size and SHA-256, unpacks it (files,
folders and links inside it only), checks every file's SHA-256 against the manifest and that there's nothing extra,
moves it to `<version>/`, and only then switches the `current` link to it, atomically. The one before stays as
`previous`. The new version is used from the next start (Settings offers a restart). Bundles the system has caught up
with, and older ones, are removed.

```
~/.local/share/hearth/app/
  current -> 0.6.61          previous -> 0.6.60+app.1
  0.6.61/  nimbus  electron/  manifest.json  manifest.json.sig
  staging/
  marks/  trying  good/<version>  bad/<version>  ui-up  last-fallback  last-check.json
```

### Start-up: which Hearth runs

The session's watchdog always starts the image's own program (`/usr/bin/nimbus` → `/usr/lib/nimbus/nimbus`). That
program, before anything else, decides (`appchannel::decide`); a bundle never decides anything. It starts the bundle
only when **all** of these hold:

1. Nothing forces the built-in version (below).
2. Not a live USB or the installer.
3. Hearth app updates aren't Off, and a `current` bundle exists.
4. The image has an app ABI and a real signing key.
5. The bundle isn't marked bad.
6. Its manifest's signature checks out with the image's key; it's for this edition, processor and ABI; the image's
   version is inside its range; it's newer than the built-in version (when the system catches up, the built-in wins).
7. Its files are all there with the right sizes, and its program's SHA-256 is right.
8. The renderer isn't Qt; if Chromium would draw, the bundle has its own Chromium window app and Chromium's sandbox can
   work (unprivileged user namespaces; a bundle in your home folder can't have the setuid helper).
9. In anything but Game Mode's own start (a Hearth window in Desktop Mode, an extra multi-seat player), the bundle must
   already be marked good.

**The start counter (Game Mode's own start only).** Before a start with a bundle, `marks/trying` records it. The
built-in program stays as the bundle's parent and watches it. The bundle's core writes `marks/ui-up` when Hearth's
screen first comes up, and marks itself **good** after running 60 seconds from there (or when it leaves on purpose
after its screen came up). A start that doesn't get there counts as failed: the bundle crashed, exited before its
screen came up (counted as a crash, so the watchdog starts Hearth again instead of ending the session), or didn't bring
its screen up within 90 seconds (the built-in parent stops it). **Two failed starts in a row** mark that version bad;
the third start is the built-in Hearth, and the next screen says why (also in Settings › Updates and Send logs).

**Forcing the built-in version:**

- Settings › Updates › **Use the built-in version** (sets the current bundle aside; a newer one can still come later).
- Settings › Updates › Hearth app updates › **Off**.
- The boot option `hearth.app=builtin` (add it at the boot menu with `e`).
- `~/.config/nimbus/app-builtin` (until removed), or `$XDG_RUNTIME_DIR/hearth-app-builtin` (this sign-in).
- `/run/hearth/app-builtin`, and Hearth Recovery's safe graphics (`/run/hearth/safe-graphics`).
- `HEARTH_APP=builtin` in the environment.
- From a terminal: `nimbus --app-use-builtin`; `nimbus --app-status` shows everything.

Holding B at start isn't used: the session has no reliable way to read the controller before Hearth starts.

### Every fallback path

| What happens                                                    | Result                                                    |
|-----------------------------------------------------------------|-----------------------------------------------------------|
| No bundle, mode Off, a force flag, live USB/installer            | Built-in                                                  |
| Image without app ABI or with the placeholder key                | Built-in (and no checks or downloads)                     |
| Signature wrong, missing, or by another key                      | Refused at install; built-in at start                     |
| Other edition, processor or ABI; image outside the version range | Refused at install; built-in at start                     |
| Bundle not newer than the built-in version (system caught up)    | Built-in; the bundle is removed                           |
| A file changed or missing after install                          | Built-in                                                  |
| Archive damaged, wrong size, an unlisted file, a path or link outside the bundle | Not installed; `current` unchanged       |
| Qt renderer chosen; Chromium needed but no sandbox or no window app | Built-in                                               |
| Desktop Mode window or extra player before the bundle was good   | Built-in                                                  |
| The bundle's program can't be started at all                     | Marked bad at once; built-in in the same start            |
| Crash, early exit or no screen within 90 s, twice in a row       | Marked bad; built-in from the third start, note shown     |
| Anything unexpected while choosing (a panic)                     | Built-in                                                  |

## Signing key (the owner, once)

The private key never goes in the repository, an image, or a log. On your own computer:

```sh
ssh-keygen -t ed25519 -N "" -C hearth-app -f hearth-app-signing
```

1. **Secret:** GitHub › Dragongamer9/hearth-os › Settings › Secrets and variables › Actions › New repository secret,
   name `HEARTH_APP_SIGNING_KEY`, value: the whole contents of `hearth-app-signing` (from
   `-----BEGIN OPENSSH PRIVATE KEY-----` to the END line).
2. **Public key:** replace the placeholder lines in `os/system_files/usr/share/nimbus/app-signing.pub` with the one line
   in `hearth-app-signing.pub` (`ssh-ed25519 AAAA… hearth-app`), and commit it. Images built from then on trust it.
3. Keep `hearth-app-signing` somewhere safe and offline (a password manager). Anyone with it could sign a Hearth app
   update for every Hearth OS device.

`release.yml` signs only when the secret is set **and** matches the public key in the repository; otherwise the release
goes out without app bundles and a notice says why. To change the key, put the new public key on its own line next to
the old one in `app-signing.pub` (both are trusted), let images with both go out, then switch the secret and remove the
old line.

## Tests

- `hosts/hearth-core/src/appchannel.rs` unit tests: signatures (good, changed, another key, another namespace,
  damaged), manifest fit (ABI, edition, version, ranges, unsafe paths), file checks, version comparison, the start
  counter and bad marks, link switching and tidying, installing real archives.
- `hosts/linux/test/test_hearth_app_channel.py`: packing and the ABI tag everywhere; with a built Hearth (`HEARTH_BIN`,
  in rust.yml and release.yml) the start-up choice end to end with a throwaway key made in CI: the bundle is picked,
  and every fallback path above ends at the built-in Hearth.
