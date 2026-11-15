# Updates: version numbers

How Hearth OS is updated is on the website (hearthos.fun/docs/updates.html, from `tools/build-docs.py`); Hearth app
updates between system updates are in `docs/APP-UPDATES.md`. Hearth's Decky plugins update on their own channel:
`plugins-v<version>` releases the owner publishes by hand with his own number, never marked "Latest"
(`docs/PLUGIN-UPDATES.md`). This page is about the numbers.

## Version numbers

- **From 1.0 on, Hearth OS is MAJOR.MINOR.** The first number is the big release (1, then 2 for the next big shift);
  the second counts the updates after it: 1.0, 1.1, 1.2 … 1.9, 1.10, 1.11 … then 2.0. The owner picks every number.
- **Files say three parts, people see two.** Build files need three parts, so the version files (csproj, Cargo.toml,
  package.json), git tags, release file names, image labels, `/usr/share/nimbus/version`, app bundle manifests, logs
  and the player counter say `1.0.0`, `1.1.0`. Everything people read shows `1.0`, `1.1`: Settings › About and Updates
  (Game Mode and Mantel), hearthfetch, the System Info app, the Decky plugin's update line, Hearth Link's device
  health, GitHub release titles and the website (release notes, docs header, download versions). Only a trailing
  `.0` is dropped, and only from 1.0 on: 0.6.63 stays 0.6.63, and a 1.0.1 (if there ever is one) shows whole.
- **Comparing versions** is by number, part by part, with missing parts counting as 0: `1.0` = `1.0.0`, 1.0 is newer
  than 0.6.63, 1.10 is newer than 1.9. Every updater does this (hearth-core `appchannel::cmp_versions` and
  `appupdate::newer`, Ember's `UpdateService.IsNewer`, `hosts/linux/src/updates.js`, `tools/hearth_version.py`), and
  each has tests with these cases. `tidy-releases.yml` keeps the newest plain `vX.Y` / `vX.Y.Z` tag of any major.
- **The date.** Each release's date is shown next to its number: "Hearth OS 1.0 · November 2, 2026", the month in the
  reader's language. It comes from the release's heading in `docs/ROADMAP.md`, written by the release step:
  `## 1.0 (released 2026-11-02)`. From there it goes, with nothing else to set:
  - into the image as `/usr/share/nimbus/version-date` (base-anatase.yml / base-peppermint.yml, next to
    `/usr/share/nimbus/version`), which Settings, Mantel's About, the System Info app read offline;
  - into the Hearth program itself (`hosts/hearth-core/build.rs`), so AppImages and app bundles carry their own;
  - into each app bundle's manifest (`date`, `tools/app-bundle.py`);
  - onto the release notes page (`tools/build-docs.py`).
  Headings without a date (`## 0.6.63 (released)`, every release before 1.0) still work: those versions show just the
  number. `python3 tools/hearth_version.py date 1.0.0` prints the date a build will get.

## Smaller updates (1.13)

An update used to be the whole 2–3 GB image every time: the rechunk step (chunkah) had no way to tell which files
belong together on Debian, so everything landed in one piece stamped with the build's time, and that piece changed
with every build. Now:

- **Debian frozen to this week's Monday** (`tools/debian_snapshot.py`, `os-bases/peppermint/apt-snapshot.sh`):
  the image is built from snapshot.debian.org as Debian was at 00:00 UTC on Monday, so every build in a week gets
  byte-for-byte the same packages. It moves by itself each week. For a security fix sooner, put a newer date
  (YYYYMMDD) in `os-bases/peppermint/debian-snapshot`; `live` there turns the freeze off. If snapshot.debian.org
  doesn't answer, the build goes on from the live repositories and says so (only that update is bigger). A device's
  own apt (Writable system) always uses Debian's live repositories.
- **Every file in its own piece** (`os-bases/peppermint/components.py`, the last step of build.sh): a file from a
  Debian package belongs to its source package ("deb/mesa"), anything else to its folder ("hearth/usr/lib/hearth-office",
  "hearth/usr/share/nimbus/desktop"), so each Debian package and each Hearth app gets layers of its own.
- **A fixed time** for files without one of their own (chunkah's `SOURCE_DATE_EPOCH`, the snapshot's), not the
  build's.
- **Published exactly as made**: the rechunked image is pushed from chunkah's own output with its layers' digests
  kept (devices compare digests), not recompressed by podman.
- **Each build says how big its update is**: the Update size step compares the new image with the published one
  and with the last build of the same branch (an annotation and the run's summary: MB to download, which pieces).
- **Downloaded quietly**: the background update (bootc's timer, staged for the next restart, never restarting by
  itself) runs at the lowest processor and disk priority. The previous version always stays in the boot menu.

The first update in the new layout downloads everything once (every layer is new); the ones after only what changed.
