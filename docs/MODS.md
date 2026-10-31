# Hearth Mods: Nexus Mods and Wabbajack

Hearth Mods (`hearth-mods`, `hosts/hearth-gaming`) came in 0.6.1 with Controller Lab and Saves. For 0.6.5 it gets
Nexus Mods downloads, a choice of how mods go into a game, and Wabbajack modlists. The page has four tabs (LB / RB):
**Games**, **Downloads**, **Modlists** and **Accounts** (Nexus Mods, LoversLab, MEGA, Google).

## Code

| File | What |
|---|---|
| `src/nexus.rs` | The account (a personal API key in `~/.config/nimbus/nexus.json`, mode 600), the v1 API through curl (address and key on curl's stdin, never its command line), `nxm://` links, the request quota, which Nexus game a library game is |
| `src/downloads.rs` | The download queue (`mods/downloads.json`): two at a time, resumable (`curl -C -`), survives closing the app; files in `mods/downloads/<nexus game>/` unless a modlist asks for its own folder and file name |
| `src/mods.rs` | Deploy modes per game: `hardlink` (default), `copy`, `overlay` (Beta Experimental); the link deploy (`sync_files`), unpacking and the fuse-overlayfs mount are shared with the organizer |
| `src/organizer/` | The Hearth Mod Organizer (below): `mod.rs` instances, profiles, mod list, installers, import, commands; `mo2.rs` MO2's files; `vfs.rs` the merged view, deploy, run; `plugins.rs` Bethesda plugins; `fomod.rs` FOMOD; `kinds.rs` the games it knows |
| `src/wabbajack.rs` | The modlist gallery, installs through jackify-engine, files it can't fetch (read from the .wabbajack too), MO2's ini, the list's Proton prefix, the library entry |
| `src/accounts.rs` | LoversLab, MEGA and Google sign-ins (`~/.config/nimbus/{loverslab,mega,google}.json`, made mode 600 before anything is written), OAuth with PKCE through the browser and a loopback address, what the engine gets in its environment |
| `src/guided.rs` | The guided Hearth Browser flow: Wabbajack's xxHash64, checking a downloaded file against the list, catching it in Hearth Browser's download folder, opening a page in Hearth Browser |
| `src/apps/mods.rs` | The window's calls and the command line; the nxm inbox |
| `ui/mods.html` | The page (preview data when opened in a browser) |
| `hearth-core/src/library.rs` `modlist_games()` | Installed modlists in Hearth's library (`mods/modlists/<id>.json` + `.desktop`) |

The page uses the shared on-screen keyboard (`ui/osk`, copied to `hosts/hearth-gaming/ui/osk` by
`tools/osk-sync.py`; kit.js opens it with `HearthOSK.open` and leaves the controller to it while it's open) and the
standard Linux file chooser (tauri-plugin-dialog through the portal: Hearth Files in Hearth OS) for mod files,
folders and `.wabbajack` files. Hearth's own folder browser (`KIT.pickPath`) is only the fallback.

## Nexus Mods

- Sign in with a personal API key (Accounts › Open my API key page › Paste my key). A Hearth app slug for
  Nexus' SSO sign-in needs Hearth registered with Nexus Mods; until then it's the personal key.
- **Free accounts**: every file comes from the site's **Mod Manager Download** button (Nexus' rule: the API's
  download link needs Premium, or the key in an nxm link). The button opens `hearth-mods --nxm <link>`
  (`hearth-mods-nxm.desktop`, `x-scheme-handler/nxm`), which drops the link in `mods/nxm-inbox/`; the open window takes
  it within a second, or a window opens on Downloads.
- **Premium**: Hearth asks the API for download links itself: a game's *From Nexus Mods… › Download a mod by its
  address*, expired links renewed, and Wabbajack lists without clicking.
- A finished download: *Install into a game…* unpacks it into the usual Install dialog, remembering where it came
  from (`ModRec.nexus`, for update checks later).

## Deploy modes (Games › a game › Links / Copies / Overlay)

- **Links** (default): hard links to Hearth's copy of the mod: no extra space, instant; a copy across drives. A game
  that edits a modded file in place edits Hearth's copy too (as Vortex).
- **Copies**: as Hearth Mods always did.
- **Overlay** (Beta Experimental): mods for the game folder go into `mods/<game>/layer/`, which fuse-overlayfs lays
  over the game folder; what the game writes there goes to `upper/`, so the game folder is never changed. Switching
  needs the game closed. `hearth-mods-remount.service` (user, at sign-in) runs `hearth-mods --remount`.
- Switching takes every mod out the old way and puts it back the new way.

## Wabbajack modlists

- The engine is **jackify-engine** (Jackify's native Linux wabbajack-cli; GPL-3.0 like Wabbajack and Hearth; source
  <https://github.com/Omni-guides/dev-jackify-engine>). The images put the newest Linux release in
  `/usr/lib/hearth/jackify-engine` (with `SOURCE.txt`); otherwise *Get the Wabbajack engine* downloads it into
  `~/.local/share/nimbus/tools/jackify-engine`. `$HEARTH_JACKIFY_ENGINE` overrides both.
- Gallery: `jackify-engine list-modlists --json` (cached 6 h). Install: `install --show-file-progress -m <repo/name>`
  (or `-w file.wabbajack`) `-o <folder> -d <downloads> -g <game folder>`, with `NEXUS_API_KEY`.
- Files the engine can't download (`manual_download_required` events): Nexus files become "needs-link" items in
  Downloads (Premium: downloaded straight away); files from other sites go through the guided flow below. When the
  engine's ones are all in the list's downloads folder, Hearth writes `{"command":"continue"}` to the engine.
- jackify-engine doesn't send LoversLab files through that step: without a LoversLab sign-in they just fail. So
  Hearth reads the list itself (the `.wabbajack` is a zip with the list in `modlist`; a gallery list's is in
  `~/Jackify/downloaded_mod_lists/`) and puts LoversLab and Vector Plexus files in the guided flow at once, while the
  engine downloads the rest. A line `Failed to download '<file>'` puts that file in the guided flow too, when the list
  gives it a page (not Nexus, the Wabbajack CDN or game files). If the engine stops with files missing, the install
  waits ("Needs files") and starts again by itself when they're all there (at most 3 times in a row; then Resume).

## Accounts (Accounts tab)

Each is signed in by you, kept in its own file in `~/.config/nimbus` readable by you only, given to the engine in its
environment (never on a command line, never in a log), and has its own **Sign out**. What the engine reads is
jackify-engine's own: `EncryptedJsonTokenProvider` takes the variable named after its key (`lovers-lab` →
`LOVERS_LAB`, `mega-login` → `MEGA_LOGIN`) when its own encrypted file isn't there.

| Account | What happens | Needs from the owner |
|---|---|---|
| Nexus Mods | As above (personal API key) | Nothing (SSO later: Hearth registered with Nexus) |
| LoversLab | Sign in to LoversLab… opens LoversLab's sign-in in Hearth Browser; LoversLab sends you back to `http://127.0.0.1:47165/hearth-loverslab`, Hearth trades the code for a token (PKCE) and gives it to the engine as `LOVERS_LAB`, so its own LoversLab downloader (`api.loverslab.com`, scope `downloads`) fetches LoversLab files. Renewed with the refresh token before each install | **Hearth's own LoversLab OAuth client** (see below). Until then the section says "Not available yet" and LoversLab files go through the guided flow |
| MEGA | Not signed in, the engine gets MEGA's anonymous login (`{"login":null}`), so MEGA files download by themselves within MEGA's free transfer quota (MEGA limits anonymous downloads per IP over a few hours). Signed in (email and password typed in Hearth with the on-screen keyboard), Hearth works out the login MEGA's own client library uses (v2 accounts: PBKDF2-HMAC-SHA512 with the salt MEGA gives for the email, 100,000 rounds), checks it with MEGA, keeps only that (never the password) and gives it as `MEGA_LOGIN`; your account's quota applies. Accounts with two-step sign-in and pre-2018 (v1) accounts can't be used (Hearth says so) | Nothing |
| Google | jackify-engine downloads public Google Drive files without an account (and Wabbajack's CDN mirrors them); it has no Google sign-in and Jackify has none either. When Google says "quota exceeded" the file fails; signed in to Google, Hearth then downloads it itself through the Drive API with your account (`drive.readonly`) and checks it against the list | **Hearth's own Google OAuth client** (see below). Until then it says "Not available yet" and those files go through the guided flow |

Jackify itself has no LoversLab, MEGA or Google sign-in screen: its engine stores a MEGA login made with
`jackify-engine mega-login -e <email> -p <password>` (a password on a command line, which Hearth doesn't do) and has
Wabbajack's LoversLab downloader. Upstream Wabbajack (October 2026) has dropped its LoversLab API downloader: LoversLab
files are "downloaded in a browser" there (`Wabbajack.Downloaders.ManualSources`).

### What the owner registers (nothing is borrowed from Wabbajack)

LoversLab's OAuth app that jackify-engine names (client `0b543a01…`) is Wabbajack's; Hearth doesn't use it. Once
Hearth has its own clients, put them in the build (`HEARTH_LOVERSLAB_CLIENT_ID`, `HEARTH_GOOGLE_CLIENT_ID`,
`HEARTH_GOOGLE_CLIENT_SECRET` as environment variables when hearth-gaming is built) or, to try them on a device
first, in `~/.config/nimbus/oauth-clients.json` or `/usr/share/hearth/oauth-clients.json`:
`{"loverslab": {"clientId": "…"}, "google": {"clientId": "…", "clientSecret": "…"}}`.

- **LoversLab**: ask LoversLab's staff for an OAuth client for Hearth OS (LoversLab runs Invision Community; clients
  are made by its admins). Ask for: a **public** client (no secret) with **PKCE**, grant type **authorization code**
  with refresh tokens, scope **`downloads`** (the downloads API, `GET /api/downloads/files/<id>`), redirect URI
  exactly **`http://127.0.0.1:47165/hearth-loverslab`**. Mention it's for downloading the files Wabbajack modlists
  name, by signed-in users, the way Wabbajack did. LoversLab may say no (Wabbajack itself now sends people to the
  browser); then the guided flow stays the way.
- **Google**: in Google Cloud Console, make a project, enable the **Google Drive API**, set up the OAuth consent
  screen (app name Hearth OS, scope `https://www.googleapis.com/auth/drive.readonly`), then Credentials › Create
  OAuth client ID › **Desktop app**. Its client ID and secret go in as above (Google says a desktop app's secret
  isn't secret, but keep it out of the repo all the same). `drive.readonly` is a restricted scope: while the app is
  in Testing, add your own Google account as a test user (sign-ins then last 7 days); publishing for everyone needs
  Google's verification.

## The guided Hearth Browser flow

For every file still not automatic (LoversLab until it's registered, Vector Plexus, "manual" download sites, MEGA or
Google files that failed, anything the engine reported failed that has a page): the modlist shows **"3 of 12
LoversLab files to fetch in Hearth Browser" › Fetch them**. Hearth opens the exact file page in Hearth Browser
(`/usr/bin/hearth-browser`, else the default browser), where you're signed in with your own browser session; you press
Download. Hearth watches the browser's download folder (your Downloads folder, and the folder chosen in Hearth
Browser's settings, read from its `prefs.js`), skips files still downloading (`.part`), and takes a file when it has
the list's name (or `name (1).ext`) or, when the list gives a hash, any finished file of the right size; it checks the
size and Wabbajack's xxHash64 (from the engine's event, or the `.wabbajack`), moves it into the list's downloads
folder, and opens the next page by itself. A wrong file stays where it is, with a note ("probably another version").
With a controller: A opens the page again, X "I have it: choose the file…" (checked the same way), Y skips for now
(skipped files come back at the end), B stops. When all are there the install carries on by itself.
- Afterwards: `ModOrganizer.ini`'s `gamePath` and `download_directory` as Wine `Z:` paths (backup next to it), the
  list's own prefix in `prefixes/wabbajack-<id>` with the Windows parts these lists need (`umu-run winetricks`, the
  same sets Jackify uses), and a library entry that starts `ModOrganizer.exe` with umu (`GAMEID=umu-<steam app>`).
- Games Hearth knows: Skyrim SE / LE / VR, Fallout 4 / 4 VR / New Vegas / 3, Oblivion (and Remastered), Enderal SE,
  Starfield, Cyberpunk 2077, Baldur's Gate 3. Others install, but their prefix may need setting up by hand.
- Lists that don't use Mod Organizer 2 aren't set up yet. Collections (nxm://…/collections/…) aren't supported yet.

## Hearth Mod Organizer

Hearth's own mod organizer: a native Linux engine in Rust (`src/organizer/`) that works the way
**Mod Organizer 2** does and reads and writes MO2's own files, so an instance opens in both. Credit: MO2's design
and file formats come from [ModOrganizer2](https://github.com/ModOrganizer2) (modorganizer, its game plugins and
installer_fomod), GPL-3.0 like Hearth; no MO2 code is copied, it's written anew in Rust.

### What it does

- **Instances, MO2's layout.** One per game: `ModOrganizer.ini`, `mods/<mod>/` (each with `meta.ini`),
  `downloads/`, `profiles/<profile>/` and `overwrite/`; Hearth's own notes in `.hearth/`. New instances live in
  `~/.local/share/nimbus/mods/organizer/<id>/` (list: `instances.json` there); an existing MO2 folder is used in place.
- **Profiles**: `modlist.txt` (highest priority first; `+` on, `-` off, `*` the game's own), `plugins.txt` (`*`
  before active plugins in Skyrim SE / Fallout 4 / Starfield; just the active ones in older games, with
  `loadorder.txt`), `settings.ini` (`LocalSaves`, `LocalSettings`). Profile inis and saves are used while playing.
- **The game's folder is never changed.** The enabled mods are merged by priority (higher wins; overwrite on top)
  matching paths without regard to case, the way Windows and Wine see them (the game's own spelling of its folders
  wins, so `textures/` and `Textures/` are one folder). The winners are hard-linked into `.hearth/layer` and
  **fuse-overlayfs** lays it over the game's `Data` folder (the game folder itself for other games), with
  `overwrite/` as the layer that takes everything the game writes. Without fuse-overlayfs (or when it won't mount)
  the **links** fallback puts the files in the Data folder, setting aside each game file it replaces, and puts
  everything back afterwards; anything the game wrote there moves to `overwrite/`, so the folder ends byte-identical.
- **Profile files in the Proton folder** while it runs: the profile's `plugins.txt` (and `loadorder.txt`) in
  `AppData/Local/<game>/`, its own inis in `Documents/My Games/<game>/` (copied back to the profile afterwards), its
  own saves (the `Saves` folder links to the profile's `saves/`). The game's own files are set aside and put back.
- **Crash-safe.** What's deployed is written first (`.hearth/active.json`); a deploy left by a crash (or by hand) is
  undone at the next run, and at sign-in by `hearth-mods --remount` (`hearth-mods-remount.service`). Changes wait
  while the game runs through the organizer. It won't deploy over Hearth Mods' simple list for the same game.
- **Conflicts**: which mod wins each file, per mod "wins over" / "loses to" with counts and a file list;
  `.mohidden` files and folders are left out (hide / unhide); each mod's `meta.ini` never reaches the game.
- **Bethesda plugins**: the TES4 header of each `.esp/.esm/.esl` (master and light flags, masters, author,
  description; Oblivion's shorter header), the game's own masters and Creation Club plugins always first, masters
  before the rest, load indexes (`FE:xxx` for light plugins), missing masters (missing, off, or loading after),
  BSA/BA2 archives with the plugin that loads each. **Sort** keeps your order and only fixes what the game requires
  (masters first, every plugin after its masters). LOOT's masterlist isn't read yet.
- **Games**: Skyrim SE/AE, Skyrim (LE), Skyrim VR, Enderal SE, Fallout 4, Fallout 4 VR, New Vegas, Fallout 3,
  Oblivion, Starfield (after MO2's game plugins); any other game is "generic" (mods into its folder, no plugins).
- **Installers**: plain archives (the Data folder found through wrapping folders; files meant for the game's own
  folder, like a script extender's loader, are named in a note, since the organizer doesn't change that folder);
  **FOMOD** (`fomod/ModuleConfig.xml`, UTF-16 or UTF-8: steps, groups and their types, plugin types fixed or by
  pattern, condition flags, file/flag dependencies, visible steps, conditional installs, priorities, the mod's own
  requirements); **BAIN** basics (numbered sub-packages, `00` ones chosen by default).
- **Wabbajack lists run natively.** When a list finishes installing, `wabbajack.rs` hands its folder to the organizer
  (import; a list's own "Stock Game"/"Game Root" copy is used when it has one, its Proton folder is the list's
  `prefixes/wabbajack-<id>/pfx`). The list's library entry then starts `hearth-mods --organizer <id> run`; Windows
  MO2 stays as the fallback per list (`launcher mo2`). Any MO2 folder can be imported by hand too.
- **Play.** Each instance has a library entry (`mods/modlists/org-<id>.json` + `.desktop`, or the Wabbajack list's
  own), so Game Mode's Play button starts `hearth-mods --organizer <id> run`: mount the view, start the program,
  wait for it (and anything still running in the game's folder) to end, then take the view away. Windows programs
  start with `umu-run` in the game's Proton folder as Hearth starts Steam games (`GAMEID=umu-<app>`, `STORE=steam`,
  `PROTONPATH`, default `GE-Proton`, or `$HEARTH_PROTON`); without umu, Steam starts the game (`-applaunch`, no
  script extender). The executables list is the script extender's loader (SKSE, F4SE...) if it's in the game folder,
  the game, its launcher, and MO2's `[customExecutables]`. Default: the script extender, else the game.

### Commands (JSON)

`hearth-mods --organizer <args…>` prints one JSON object; errors are `{"ok": false, "error": "…"}` with exit
code 1. The Desktop Mods app calls the same thing (`invoke('organizer', {args})`; `organizer_run` starts `run` in
its own process). Game Mode screens can run the command line the same way.

| Command | Answer |
|---|---|
| `instances` | `{instances: [instance + {deployed, mods, modsOn}]}` |
| `games` | the game kinds it knows |
| `create <library game id> [--kind skyrimse] [--name T]` | a new instance for a library game (or its existing one): `{created, summary}` |
| `import <folder> [--name T] [--prefix <pfx>] [--wabbajack <job>]` | uses an MO2 folder in place: `{imported, notes, summary}` |
| `recover` | undoes every deploy left behind: `{recovered: [{id, result or error}]}` |
| `<inst>` or `<inst> summary` | the summary (below) |
| `<inst> mods / plugins / archives / executables / profiles / warnings / status` | one part of it: `{instance, <part>}` (`status` is `deployed`) |
| `<inst> conflicts [<mod>]` | per mod `{name, files, winning, losing, hidden, winsOver, losesTo}`; with a mod, its files `{path, status: wins/loses/alone/hidden, over / to, replacesGameFile}` |
| `<inst> enable / disable / remove <mod>` | `{summary}` |
| `<inst> move <mod> <priority>` or `move <mod> --by <±n>` | priority 0 is the lowest; higher wins |
| `<inst> hide / unhide <mod> <file>` | `.mohidden` |
| `<inst> plugin-enable / plugin-disable <plugin>`, `plugin-move <plugin> <place>`, `sort` | writes plugins.txt and loadorder.txt |
| `<inst> profile <name>`, `profile-create <name> [--from P]`, `profile-settings [--local-saves on/off] [--local-inis on/off]` | |
| `<inst> install <archive or folder or token> [--name N] [--choices JSON or @file] [--off]` | installs, or answers `{needs: "fomod", token, name, fomod, evaluation}` / `{needs: "bain", token, name, packages: [{name, default}]}` |
| `<inst> fomod <archive>` | unpacks and reads the installer only (same answer as `install` without choices) |
| `<inst> fomod-eval <token> --choices JSON` | `{evaluation}` for those choices |
| `<inst> cancel <token>` | throws an unpacked mod away |
| `<inst> deploy [--links]` / `undeploy` | puts the view in place by hand (overlay, or links) / takes it away |
| `<inst> run [--exe <title>]` | Play: `{ok, title, mode, exit, undeploy, notes}` once the game has ended |
| `<inst> launcher hearth/mo2` | what Play starts (mo2: Wabbajack lists only) |
| `<inst> forget` | takes it out of Hearth's list (its folder stays) |

Every changing command answers `{ok, summary, …}` (and `deployProblem` if a by-hand deploy couldn't be made again).

**Summary**: `instance` `{id, title, dir, game, gameName, gameDir, dataDir, prefix, libraryId, wabbajack, launcher,
profile, modsDir, downloadsDir, overwriteDir, plugins}`; `profiles` `[{name, active, localSaves, localInis}]`;
`mods` lowest priority first, then Overwrite: `[{id (the folder name: use it in commands), name, priority, enabled,
kind: mod/separator/unmanaged/overwrite, version, nexusId, installedFrom, plugins, files, conflicts: {winsOver,
losesTo, winning, losing}, hidden}]` (files and conflicts are null for mods that are off); `plugins` in load order
`[{name, active, implicit, master, light, mod, masters, author, description, loadIndex, missing: [{master, why:
missing/inactive/loadsAfter}], problem}]`; `archives` `[{name, mod, plugin, loaded}]`; `executables` `[{title,
binary, args, cwd, extender, from: game/mo2, present}]`; `deployed` null or `{mode: overlay/hardlink, target, since,
profile, running, byHand, mounted, notes}`; `warnings`; `sort: {loot: false, note}`.

**FOMOD for a controller**: `fomod` is `{name, image, info: {name, author, version, website, description}, requires,
required, steps: [{name, visible, groups: [{name, type: SelectExactlyOne/SelectAtMostOne/SelectAtLeastOne/SelectAll/
SelectAny, plugins: [{name, description, image, imagePath, files, flags, defaultType, patterns}]}]}], conditional}`
(steps, groups and plugins already in the installer's order). Choices are `[[[plugin indexes] per group] per step]`;
`evaluation` is `{steps: [{visible, groups: [{selected, types, problem}]}], flags, errors, files, choices}`: show
the visible steps one at a time, render each plugin with its current type (NotUsable greyed, Required fixed), send
each change to `fomod-eval`, and install with `evaluation.choices` when `errors` is empty.

### Not done yet

LOOT's masterlist (the sort helper only fixes what the game requires); files for the game's own folder (MO2's Root
Builder: a script extender's loader must be put in the game folder by hand, or come with a list's Stock Game); BSA
invalidation and archive order; Nexus downloads straight into an instance's `downloads/` and its download list;
mod categories, notes and update checks from `meta.ini`; FOMOD scripts (C# installers) and BAIN wizards; MO2's own
plugins (tools that run inside MO2) aren't run. Starfield's light/medium plugins need checking.

## Command line

```
hearth-mods --nxm <nxm://...>          the nxm handler
hearth-mods --nexus-key <key>          sign in
hearth-mods --downloads                the queue, as JSON
hearth-mods --deploy <game> <mode>     hardlink | copy | overlay
hearth-mods --remount                  overlays back (at sign-in)
hearth-mods --modlists                 the Wabbajack gallery, as JSON
hearth-mods --wabbajack-engine         download the engine
hearth-mods --page downloads           open on a tab (games, downloads, modlists, organizer, accounts; nexus opens accounts)
hearth-mods --organizer <args…>        the Hearth Mod Organizer, as JSON (above)
```

## Not tested yet

Written without a Nexus account or a real Wabbajack install to try against (the build machine can't reach Nexus):
the queue, the engine's event and progress lines (taken from Jackify's own parser), MO2 under umu, and the overlay
on a real game need testing on Hearth OS. The Hearth Mod Organizer is tested in CI on fake games with synthesized
mods (priority, conflicts, MO2's files, plugin headers, FOMOD, import, a link deploy leaving the game folder
byte-identical, and the overlay where the runner has fuse-overlayfs); running real games through it (Proton, script
extenders, Wabbajack lists) needs testing on a device. The accounts and the guided flow were written without any LoversLab, MEGA
or Google account (CI tests the file checks, the queue, the hashes and that sign-in files are mode 600): the guided
flow with a real LoversLab account, the MEGA sign-in (the login format the engine reads), anonymous MEGA downloads,
and the LoversLab and Google sign-ins once registered need testing on a device.
