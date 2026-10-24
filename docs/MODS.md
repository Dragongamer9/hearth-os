# Hearth Mods: Nexus Mods and Wabbajack

Hearth Mods (`hearth-mods`, `hosts/hearth-gaming`) came in 0.6.1 with Controller Lab and Saves. For 0.6.5 it gets
Nexus Mods downloads, a choice of how mods go into a game, and Wabbajack modlists. The page has four tabs (LB / RB):
**Games**, **Downloads**, **Modlists** and **Nexus account**.

## Code

| File | What |
|---|---|
| `src/nexus.rs` | The account (a personal API key in `~/.config/nimbus/nexus.json`, mode 600), the v1 API through curl (address and key on curl's stdin, never its command line), `nxm://` links, the request quota, which Nexus game a library game is |
| `src/downloads.rs` | The download queue (`mods/downloads.json`): two at a time, resumable (`curl -C -`), survives closing the app; files in `mods/downloads/<nexus game>/` unless a modlist asks for its own folder and file name |
| `src/mods.rs` | Deploy modes per game: `hardlink` (default), `copy`, `overlay` (Beta Experimental) |
| `src/wabbajack.rs` | The modlist gallery, installs through jackify-engine, files it can't fetch, MO2's ini, the list's Proton prefix, the library entry |
| `src/apps/mods.rs` | The window's calls and the command line; the nxm inbox |
| `ui/mods.html` | The page (preview data when opened in a browser) |
| `hearth-core/src/library.rs` `modlist_games()` | Installed modlists in Hearth's library (`mods/modlists/<id>.json` + `.desktop`) |

The page uses the shared on-screen keyboard (`ui/osk`, copied to `hosts/hearth-gaming/ui/osk` by
`tools/osk-sync.py`; kit.js opens it with `HearthOSK.open` and leaves the controller to it while it's open) and the
standard Linux file chooser (tauri-plugin-dialog through the portal: Hearth Files in Hearth OS) for mod files,
folders and `.wabbajack` files. Hearth's own folder browser (`KIT.pickPath`) is only the fallback.

## Nexus Mods

- Sign in with a personal API key (Nexus account tab › Open my API key page › Paste my key). A Hearth app slug for
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
  Downloads (Premium: downloaded straight away); files from other sites are picked up from `~/Downloads` by name and
  size. When all are in the list's downloads folder, Hearth writes `{"command":"continue"}` to the engine.
- Afterwards: `ModOrganizer.ini`'s `gamePath` and `download_directory` as Wine `Z:` paths (backup next to it), the
  list's own prefix in `prefixes/wabbajack-<id>` with the Windows parts these lists need (`umu-run winetricks`, the
  same sets Jackify uses), and a library entry that starts `ModOrganizer.exe` with umu (`GAMEID=umu-<steam app>`).
- Games Hearth knows: Skyrim SE / LE / VR, Fallout 4 / 4 VR / New Vegas / 3, Oblivion (and Remastered), Enderal SE,
  Starfield, Cyberpunk 2077, Baldur's Gate 3. Others install, but their prefix may need setting up by hand.
- Lists that don't use Mod Organizer 2 aren't set up yet. Collections (nxm://…/collections/…) aren't supported yet.

## Command line

```
hearth-mods --nxm <nxm://...>          the nxm handler
hearth-mods --nexus-key <key>          sign in
hearth-mods --downloads                the queue, as JSON
hearth-mods --deploy <game> <mode>     hardlink | copy | overlay
hearth-mods --remount                  overlays back (at sign-in)
hearth-mods --modlists                 the Wabbajack gallery, as JSON
hearth-mods --wabbajack-engine         download the engine
hearth-mods --page downloads           open on a tab (games, downloads, modlists, nexus)
```

## Not tested yet

Written without a Nexus account or a real Wabbajack install to try against (the build machine can't reach Nexus):
the queue, the engine's event and progress lines (taken from Jackify's own parser), MO2 under umu, and the overlay
on a real game need testing on Hearth OS.
