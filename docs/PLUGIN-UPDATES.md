# Decky plugin updates (without a system update)

Hearth's Decky plugins, **Hearth Sync**, **Hearth Link**, **Hearth Music** and **Hearth Eras** (and any new Hearth plugin), can update
by themselves between Hearth OS updates, and new Hearth plugins arrive the same way, without a system update. The
plugins built into the system image are never changed: they're always there and always the fallback.

**The rules above everything:** a plugin update can never stop a device from starting (plugins only run inside Decky,
in Steam Game Mode; nothing here touches booting, the image, the session or system files), the owner picks every
version number, and your settings are never changed by themselves.

## Hearth OS only

The plugins are part of Hearth OS and aren't published anywhere else: they're not submitted to Decky's store, and the
plugins release is only read by Hearth's own updater.

- Each plugin's backend checks when Decky loads it that it's on Hearth OS (`hosts/decky-shared/hearth_only.py`):
  `/etc/os-release` with `NAME=Hearth` or a `VARIANT_ID` starting with `hearth`, **and** one of Hearth's own files
  (the built-in Decky service `hearth-decky@.service` or `/usr/libexec/nimbus`). On any other system it starts none of
  its features and logs why once; the Quick Access panel shows one small invitation instead (the Hearth flame, what the
  plugin does on Hearth OS, what Hearth OS gives you, a **Get Hearth OS** button that opens hearthos.fun in Steam's
  browser, "Free and open source"). No pop-ups, no repeated errors. The frontend part is
  `hosts/decky-shared/hearth-only.tsx`, copied unchanged into each plugin's `src/` (a test checks the copies match).
- The update channel (`hearth-decky plugins-*`) refuses to run anywhere but Hearth OS.
- **Honestly: this is a check, not DRM.** The repository is public and anyone can change the check. There's no
  obfuscation, no licence key, and nothing is sent anywhere: no device identity, no phoning home.

## For the owner: publishing a plugin update

1. Pick the version. It must be **newer than the Hearth OS version the devices run** (a device only takes a plugin
   newer than both the one it has and the one built into its image). For plugins on top of Hearth OS 1.1, for example
   `1.1.1`, or `1.1.0.1` if you want to keep `1.1.1` for the next Hearth OS. The next Hearth OS release (with its own,
   newer built-in plugins) then wins by itself.
2. GitHub › Actions › **Decky plugins release** › Run workflow, on **main**: `version` = your number, tick `publish`.
   Leave `min_hearth_os` / `min_decky` empty (they default to the Hearth OS version in `hosts/hearth-core/Cargo.toml`
   and the Decky version Hearth OS pins), or set them when the plugins need something newer.
3. It tests, builds the three plugins with your version, writes `plugins.json` and `SHA256SUMS`, checks them with the
   device's own code, and makes the release `plugins-v<version>`: published, **never marked "Latest"** (Hearth OS's
   own updates keep using the newest Hearth OS release), older plugins releases hidden as drafts. Hearth OS releases
   (`release.yml`, `tidy-releases.yml`) leave plugins releases alone.

Without `publish` (or from another branch) it's a dry run: everything is built and kept as an artifact, nothing is
released. Nothing publishes on a push.

## How it works

### The release

`plugins-v<version>` has:

- one `<id>.zip` per plugin (each `package.sh`'s layout): every `hosts/decky-hearth-<name>/` with `package.json` and
  `package.sh`, found by `tools/decky_plugins.py` (never `archive/`);
- `plugins.json`: `{format: 1, version, plugins: [{id, name, description, version, archive, sha256, size, min_hearth_os,
  min_decky}], retired: [id]}` (each plugin's version is the one in its `package.json`; name and description come from
  its `plugin.json`; `retired` is optional, see below);
- `SHA256SUMS`.

A plugin id must be `decky-hearth-` plus lowercase letters, digits and dashes (`^decky-hearth-[a-z0-9][a-z0-9-]{0,40}$`);
the device refuses a manifest naming anything else, and nothing outside `~/homebrew/plugins/<id>` is ever written or
removed.

### On the device (as you; `hearth-decky plugins-*`)

1. **Check** (`plugins-check`, Settings › Updates › Decky plugins › Check now): GitHub's public API, the newest
   published `plugins-v*` release of this repository (drafts and prereleases never), downloads only from this
   repository's own release files. A plugin is picked only when it's newer than **both** the installed and the
   built-in version, it didn't fail before, and the device has the minimum Hearth OS and Decky versions.
2. **Update** (`plugins-update`, the Update button): each picked archive is downloaded, its SHA-256 checked against
   `plugins.json` **and** `SHA256SUMS`, unpacked into a staging folder (only files under its own folder; no links,
   absolute paths or `..`), checked for `plugin.json`, `main.py`, `package.json` (with the right version) and
   `dist/index.js`, and moved whole to `~/.local/share/hearth/decky-plugins/<name>` (the download before stays as
   `<name>.prev`).
3. **Put in place** by Decky's own service (`hearth-decky@<you>`, as root, before Decky starts, in `refresh_plugins`):
   the copy is made, checked again against what was verified, and swapped into `~/homebrew/plugins/<name>`; the copy
   before it is kept as `~/homebrew/hearth-prev/<name>.prev` (not inside `plugins/`, where Decky would load it as a
   second plugin). When Steam Game Mode is running, Update restarts **only Decky** (never Steam or the session), so it
   happens right away; otherwise at the next Steam Game Mode start.
4. **The image's plugins never replace a newer download**: `refresh_plugins` keeps the downloaded one while it's newer
   than the built-in one. A Hearth OS update that brings a newer built-in version wins.
5. **New plugins**: a plugin in the manifest that this device has never had (not built in, never downloaded) is listed
   under Settings › Updates › Decky plugins › **New from Hearth** with its name and description. Update (and the daily
   timer, when "Update plugins automatically" is on) installs it only when **Also install new Hearth plugins** is
   ticked: off unless you tick it, saved as `installNew` in `~/.config/nimbus/plugin-updates.json` with the settings
   helpers, never changed by itself (`hearth-decky plugins-new-set on|off`). It's then checked, staged, re-checked as
   root, swapped in and load-checked exactly like the others, and updates like them from then on. A folder of that name
   that isn't from Hearth (no Hearth download mark) is left alone.

State: `~/.local/state/nimbus/plugin-updates.json` (installed versions, built-in or downloaded, last check, last error,
rollbacks, versions that failed). Send logs has a **Decky plugin updates** section with it.

### If a downloaded plugin doesn't load: rollback

A downloaded plugin is on trial until its backend has run for 30 seconds and written its mark
(`~/homebrew/data/<name>/hearth-loaded.json`, from `hearth_only.py`). If there's no mark:

- **within 3 minutes of its first run** (a watcher next to Decky; only if Decky is still running), or
- **after two Steam Game Mode starts**,

the copy before it comes back (`<name>.prev`), or the built-in one, or, for a new plugin with nothing to go back to,
it's **removed**; that version is never picked again, its download
is removed, and why is written to the state file (Settings › Updates shows it as the last error). Only Decky is
restarted. Because plugins only run inside Decky, even a broken plugin can't stop Hearth OS or Steam Game Mode from
starting.

### Use the built-in plugins

`plugins-builtin` (Settings › Updates › Use the built-in plugins): the downloaded copies go and the image's plugins come
back (now, if Decky is running; else at the next Steam Game Mode start); new plugins, which have no built-in copy, go.
Those versions aren't downloaded again automatically; a newer release still can be.

### Retiring a plugin

A plugin is **never removed just because a manifest leaves it out**. To take one away, add its id to `RETIRED` in
`tools/decky_plugins.py`: from then on it's never built into a plugins release (its code can stay in the repository),
and every release's `plugins.json` lists it under `retired` (the release workflow checks both). On the next Update
(or automatic update) the device removes its download of it, and at the next Decky start its copy in Decky's folder:
a downloaded copy always goes; a built-in one only once the image no longer ships it (while it does, the built-in copy
comes back). The newest release's list counts, so a plugin dropped from `retired` can be offered again.

Retired so far: **Hearth Classic** (`decky-hearth-look`, November 2026, the owner's call). It was never built into the
image, so the next plugins release takes it off every device that downloaded it. Its home and skin live on as the
"Hearth Classic" era of **Hearth Eras** (`decky-hearth-eras`, docs/HEARTH-ERAS.md), which reuses its code.

### Update plugins automatically

**Off unless you turn it on.** Saved in `~/.config/nimbus/plugin-updates.json` (`auto`) with the shared settings helper
(atomic write, `.bak`, merge-on-write; docs/SETTINGS-PERSISTENCE.md); the default is never written, and checks,
updates and rollbacks never change it. When on, the user timer `hearth-plugin-updates.timer` checks once a day (a
failed check, e.g. offline, is tried again next time) and downloads; a new plugin is put in place right away when Decky
isn't running a game, otherwise at the next Steam Game Mode start.

## Adding a new plugin

1. Copy `hosts/decky-shared/template` to `hosts/decky-hearth-<name>` (lowercase, digits, dashes) and copy
   `hosts/decky-shared/hearth-only.tsx` into its `src/`. Replace "NAME" in `plugin.json` (its `name` and
   `publish.description` are what devices show under "New from Hearth"), `package.json`, `main.py` and `src/index.tsx`.
   The template's `0.0.0` is only a placeholder: the plugins release writes the version you pick.
2. Keep the template's Hearth OS check and load mark: `main.py` imports `hearth_only` (the shared
   `hosts/decky-shared/hearth_only.py`, which `package.sh` copies in), `_main` returns at once off Hearth OS and
   otherwise starts `hearth_only.mark_loaded_later(decky)`, and the panel is wrapped in `<HearthOnly>`.
   `python3 tools/decky_plugins.py check` (run in CI by `decky-plugins-release.yml`) fails when any of this is missing.
3. Add backend tests in `hosts/decky-hearth-<name>/tests/test_*.py` (CI runs them), and land it on `bughunt` as usual.
4. Publish a plugins release (above) from main: the new plugin is found and packed by itself. Devices list it under
   "New from Hearth" and install it when "Also install new Hearth plugins" is ticked. Putting it in the image as well
   (the image's build) makes it a built-in plugin in the next Hearth OS release.

## Tests

`hosts/linux/test/test_plugin_updates.py` (run by `decky-plugins-release.yml`): manifest parsing, only newer than both
installed and built-in, SHA-256 mismatch refused, minimum versions, atomic swap and `.prev`, rollback of a plugin that
doesn't load (both ways), the image never replacing a newer download, "use built-in", the switch off by default and
persisted, no drafts or prereleases, a network failure changing nothing; new plugins (a valid unknown id offered as
new, invalid ids refused, the box off by default and ticked, automatic plus the box, a failed load removing a new
plugin, retired removing only downloaded copies, refresh handling downloaded-only plugins, the root swap refusing names
outside the pattern, the workflow's discovery finding every plugin folder and the template passing the check); and the
Hearth OS check passing on Hearth's
os-release and failing on SteamOS and plain Debian, with each plugin starting nothing there.
