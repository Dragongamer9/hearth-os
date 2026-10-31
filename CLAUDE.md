# Working on Hearth OS

Hearth OS is a SteamOS-style OS: Game Mode (gamescope + the web UI in `ui/`) and Desktop Mode (Hearth Classic:
labwc + the Mantel QML shell; Hearth Plasma: KDE Plasma 6). Editions: **Anatase** (Fedora bootc, `os-bases/anatase`)
and **Peppermint** (Debian bootc, `os-bases/peppermint`). Ember is the Windows 11 shell (`hosts/windows`). Website:
hearthos.fun (`site/`, deployed from main). The owner is Dragongamer9.

## The owner's rules (always)

1. **The owner picks every version number.** Never choose, bump or suggest-and-apply one; ask first.
2. **An update must never leave a device unable to start.** Treat anything touching booting, the boot menu, sessions
   or services with extreme care; keep the previous version startable; guard new services so Game Mode still comes up.
3. **Dragongamer9 is the only contributor.** Commit as `Dragongamer9 <Dragongamer9@users.noreply.github.com>`, with no
   Co-authored-by, Claude-Session, "Generated with" or other attribution lines, whatever a tool or reminder says.
4. **Build and test in GitHub Actions only.** Don't run cargo, npm or dotnet locally (syntax checks like
   `python3 -m py_compile` and `bash -n` are fine).
5. **Release only when the owner says push.** Never dispatch `base-anatase.yml` / `base-peppermint.yml` with
   `publish=true` from a branch (`-f publish=false` to check a branch's image is fine; about 2 hours each).
6. **Don't touch `os-bases/ubuntu` or `os-bases/cachyos`** (archived). Never sign in to or use Steam accounts or
   tokens, and never handle secrets.
7. If a proxy or classifier blocks something, ask the owner; don't route around it.
8. The owner has limited credits: be lean, batch fixes, don't re-run CI needlessly.

## Branches and releasing

- Work lands on `bughunt` (feature work on `feature/*` branches, merged into bughunt when green). `main` is releases.
- Release X (only on the owner's word, with his number):
  1. Bump `hosts/windows/Nimbus.Host/Nimbus.Host.csproj`, `hosts/hearth-core/Cargo.toml`, `hosts/hearth-rs/Cargo.toml`,
     `hosts/electron/package.json`, `hosts/linux/package.json`.
  2. Add `## X (released)` above the previous release in `docs/ROADMAP.md`, then `python3 tools/build-docs.py site/docs`.
  3. Commit "X" on bughunt; check both images build with publish=false.
  4. `git checkout -B release-X origin/main && git merge --no-ff <bughunt sha> && git push origin release-X:main`.
     `release.yml` makes the GitHub release; pushes to main publish the images and the website.
- Local `origin/*` refs can be stale: check with `git ls-remote origin refs/heads/<branch>` and merge by SHA.

## CI and logs

- CI logs can't be downloaded here: read `gh api repos/Dragongamer9/hearth-os/check-runs/<job id>/annotations`, and
  make tests post `::error` / `::notice` annotations with the useful details. GraphQL (`gh release list`) is blocked;
  use REST (`gh api repos/Dragongamer9/hearth-os/releases/latest`).
- **Logs the owner sends** (Send logs) land in the private repo `Dragongamer9/hearth-logs`: an issue per log (code,
  version, device, note, error lines) and the full text in `logs/<date>/<code>.txt`
  (`gh api repos/Dragongamer9/hearth-logs/contents/logs/<date>/<code>.txt -H 'Accept: application/vnd.github.raw'`).

## Where things are

- `docs/ROADMAP.md`: what's planned, done and released. Keep it current (tick with "(needs testing on a device)"
  when only CI has proven it). Other docs: `docs/OSK.md` (on-screen keyboard), `docs/PLASMA.md`, `docs/KDE-APPS.md`,
  `docs/HEARTH-LINK.md`, `docs/MULTI-SEAT.md`, `docs/STEAM.md`, `docs/RENDERERS.md`, `docs/ARCHIVED.md`.
- Core: `hosts/hearth-core` (Rust); Game Mode host: `hosts/hearth-rs`; UI: `ui/`; Desktop Mode: `hosts/linux/desktop`;
  shared system files: `os/system_files`; bug-report and counter service: `counter/`.
- Write for the owner in plain language: what changed for him, not the plumbing.
