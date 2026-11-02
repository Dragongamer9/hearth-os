# Updates: version numbers

How Hearth OS is updated is on the website (hearthos.fun/docs/updates.html, from `tools/build-docs.py`); Hearth app
updates between system updates are in `docs/APP-UPDATES.md`. This page is about the numbers.

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
