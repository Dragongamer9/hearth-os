# Mod Manager, the new one (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-mod-manager/NN-*.html)

Mockups: mod-manager/01-game-mods.png, mod-manager/02-install.png, mod-manager/03-find.png, mod-manager/04-updates.png, mod-manager/05-crash.png, mod-manager/06-describe.png, mod-manager/07-game-mode.png, mod-manager/08-every-game.png
Release: next

The owner, Nov 26, 2026: "We need to update our Mod manager as well to make it way better", and on the boards: "thoes
look good". Our Mod Manager (today's Hearth Mods: its command and Store part stay `hearth-mods`, so Store updates and
Steam shortcuts keep working; the window says "Mod Manager" as on the boards). A pure-Rust Hearth app
(hosts/hearth-ui/apps/mods), made of blocks (Edit layout, Customize, its own blocks window), using Hearth Mods' own
Rust logic (hosts/hearth-gaming: the Hearth Mod Organizer, Nexus, Wabbajack, Play modded, Add to Steam). Ships and
updates through the Hearth Store as `hearth-mods` (red flag 14). Every part keeps what Hearth Mods already does.

## Every board: the window
- [ ] Title bar: the MM mark, "Mod Manager", "· <the page> · <the game>", minimize / maximize / close
- [ ] The status pill on the right: "NVIDIA PC · SYNCED WITH THE ALLY" (board 1), "HEARTH AI · ON YOUR NVIDIA PC" (board 6), "HOME SERVER · CONNECTED" (board 8)
- [ ] The side: "⌕ Search mods, games, files" (one search across mods, games and files)
- [ ] The side: MOD MANAGER sections: Every game, Find mods, Downloads (count), Updates & going back (count), Crashes (count), Describe it, Modlists & collections, Organizer instances (count), Accounts, Settings
- [ ] The side: YOUR GAMES with each game's picture, name and mod count, a dot for things to look at (amber) or updates (blue); the open game marked
- [ ] The side: "Updates in the Hearth Store" with "our Mod Manager is its own Store app"
- [ ] The side: "Made of blocks · Edit layout"
- [ ] Made of blocks: the side, every page and every card can be moved, sized, hidden and customized (Edit layout), with Mod Manager's own buttons, panes and info in its blocks window
- [ ] The credit at the bottom right of each board (board 1: "Hearth Mod Organizer, made to work like Mod Organizer 2 (GPL-3) · LOOT's masterlist (GPL-3) · fuse-overlayfs"; board 2: "FOMOD installers read by Hearth Mod Organizer (written anew in Rust, after Mod Organizer 2's installer, GPL-3)"; board 3: "searches Nexus Mods, Thunderstore, CurseForge and mod.io through their own APIs, with your accounts"; board 4: "changelogs from each mod's own page · snapshots kept beside the instance's Mod Organizer 2 files"; board 5: "reads crash logs from Crash Logger SSE / F4SE (community mods) and Proton's own log"; board 6: "Describe it uses Hearth AI (Colibri, Ollama as the fallback) on your own PC · fps from community reports and your own test starts"; board 7: "Hearth's own Game Mode screens with the shared on-screen keyboard"; board 8: "Battle for Middle-earth II mods through Beacon · Minecraft through Fabric · Stardew through SMAPI · Valheim through BepInEx")
- [ ] Still `hearth-mods` (command, Store part, quick-update.yml main and test, the Store list's serial unchanged), works when run from the Store's copy in the home folder
- [ ] Settings kept with the safe writers (docs/SETTINGS-PERSISTENCE.md); opening it writes nothing
- [ ] Everything Hearth Mods did stays: Nexus Mods, Wabbajack modlists, organizer instances, deploy modes (links, copies, overlay), Play modded, Add to Steam, controller use, nxm links

## Board 1: a game's mods (01-game-mods)
- [ ] The hero: "HEARTH MOD ORGANIZER · WORKS LIKE MOD ORGANIZER 2" and "2 THINGS TO FIX" tags, the game's name, "412 mods · 388 on · 263 plugins · SKSE 2.2.3 · Proton Hearth 9-4 · mods went in 12 min ago"
- [ ] Profile picker ("Profile Survival playthrough ▾")
- [ ] ▶ Play modded, Add to Steam, + Add mods, ⋯
- [ ] "Starts" choice: SKSE, Skyrim, Launcher, xEdit, BodySlide, Pandora (what Play modded starts)
- [ ] Mods card tabs: Mods 412, Plugins 263, Archives 148, Overwrite 3, "lower wins"
- [ ] Filter box "Filter 412 mods" and filters All, On, Off, Conflicts 61, Updates 7
- [ ] The priority ladder: columns #, MOD, VERSION, FROM, CONFLICTS; separators with their mod counts, folding (▾); "PATCHES · 41 mods · stay last"
- [ ] Each row: drag handle ⋮⋮, on/off switch, priority number, name, version, source tag (NEXUS MODS, YOURS), conflict mark (+ 122, − 318, UPDATE, PICKED, NEEDS A FIX)
- [ ] The picked mod marked, the mods it beats green and the ones beating it red (the conflict lens on the ladder)
- [ ] Drag by ⋮⋮ (mouse and touch); "controller: A picks up, ↑↓ moves, A drops"; the legend "+ beats the picked mod", "− beats it"
- [ ] Health card: "2 TO FIX · 1 TO LOOK AT"
- [ ] Health: "2 plugins miss a master" with which ones and Fix it (turns the master's mod back on)
- [ ] Health: "1 mod needs a newer SKSE" with Update SKSE (the new loader in the layer, not in the game folder)
- [ ] Health: "3 mods fully lose every file" with Show them
- [ ] Plugins · load order: IDX, PLUGIN, KIND (ESM, CC, ESM-flag, ESL, ESP, off), MASTERS; a missing master marked ("Lux.esp off"); "263 plugins · 89 light (FE) · masters always first"
- [ ] Sort (LOOT's masterlist, fetched pinned and checked)
- [ ] "Sort · what it would change": each move with its reason ("it's a patch for it", "masterlist: load after", "Lux's own note"), "= 260 others kept where you put them · your order wins", "LOOT's masterlist, Nov 25"
- [ ] Apply 3 moves, Not now, "a snapshot is made first"
- [ ] Conflict lens · <mod>: Files / Mods, "WINS 1,214 FILES", "LOSES 362", "ALONE 2,071"; beats / loses to each mod with file counts
- [ ] FILE BY FILE: each file with whom it wins over or loses to, "All 3,647 files ›"
- [ ] CHANGE WHO WINS: Move below <mod>, Hide one file in this mod, Make a patch with the winners; "Hiding renames nothing in the game · undo any time"
- [ ] Profiles: each with its mods, own saves, own INIs (or shared saves); + New from this one
- [ ] The game folder: NEVER TOUCHED, "While Skyrim runs the mods are a layer over its folder…", "Game files changed 0 of 1,931", "Written by the game last time 14 files → Overwrite"

## Board 2: installing a mod (02-install)
- [ ] Add mods "to Skyrim SE": drop an archive or a folder (.7z · .zip · .rar · .fomod · a mod's folder), the dragged file's card
- [ ] Choose a file…, Paste a mod's address
- [ ] From links: NXM LINKS ON, "\"Mod Manager Download\" on a Nexus Mods page comes straight here, with the mod's page remembered for updates"; each link with its source tag, name, state (READY, progress with % and speed, "for Valheim")
- [ ] Waiting to go in: each archive with what's in it (has an installer · N steps, plain archive · Data found inside), INSTALLING, Install, Install all 3; "Your answers are kept: installing again asks only what's new."
- [ ] "Archives kept on your home server": each file downloaded once, the Ally and the laptop take it from there
- [ ] The installer's head: "INSTALLER (FOMOD) · DONE IN HEARTH", the source tag, the mod's name and version, "by its author · <its words> · size", Mod page, Read me
- [ ] STEPS (done ✓, the current one, the next ones)
- [ ] FROM LAST TIME (the choices kept), ALSO READS (FOMOD steps and pictures · BAIN packages · plain archives)
- [ ] The step's question and its choices with the author's own pictures and words, "pick one", SUGGESTED
- [ ] "Next: patches for what you have": ticked for you (FOUND), greyed ones you can't use (NOT USABLE, "greyed: you don't have …"), change any
- [ ] "Picture from the installer · every choice shows the author's own picture and words, big enough for the TV"
- [ ] ‹ Back, "B back · A choose · RB next step", Cancel, Next ›
- [ ] Before it goes in: FILES, NEW, OVERWRITES; IT WILL WIN OVER (mods, files), IT WILL LOSE TO; THE GAME FOLDER "Game files changed: none · the layer only"
- [ ] Where it goes: In the list, Plugins added, Load order, Needs (✓), Space
- [ ] If the mod is already in: Install as a new version (the old one stays, off), Replace it (a snapshot first), Merge the files in
- [ ] SAFE EITHER WAY: a snapshot of the whole setup before it goes in; removing it later takes every file back out

## Board 3: Find mods (03-find)
- [ ] One search box "one search across every source · for all 8 of your games", All your games ▾, Most endorsed ▾
- [ ] Source tabs with counts: All sources, NEXUS MODS, THUNDERSTORE, CURSEFORGE, MOD.IO
- [ ] Filters: Works with my mods, Hide adult mods, Fits the Ally
- [ ] Each result: picture, source tag, game, name, endorsed / downloads / rating, the check ("✓ works with your mods · needs nothing new", "! same job as …: pick one", "! needs … (yours is …)")
- [ ] Nexus Mods search (the owner's own API key, as today)
- [ ] Thunderstore search (no key)
- [ ] mod.io search (its public API; a key the owner types, kept in the keyring)
- [ ] CurseForge search (a key from the keyring)
- [ ] The picked mod: picture, source and game tags, name, ENDORSED, DOWNLOADS, VERSION, UPDATED, its words
- [ ] NEEDS · PULLED IN BY THEMSELVES: each need with "you have it" or "gets it too" (optional)
- [ ] WORKS WITH YOUR OTHER MODS?: "Yes, checked against your 412 mods" (missing masters, file wins, clashes)
- [ ] One to choose: another result doing the same job
- [ ] WHICH FILE: the mod's files (Main, Classic, Paper, Older 8.2)
- [ ] "Also on: the ROG Ally, the laptop … · Add there as well"
- [ ] Install · N more mod with it, Endorse, Page; "Free account: one click on \"Mod Manager Download\" · Premium: straight in"
- [ ] Whole sets, installed in one go (collections, modlists and modpacks · each goes into a new profile): NEXUS COLLECTION, WABBAJACK LIST, THUNDERSTORE PACK, CURSEFORGE MODPACK cards with name, game, size, what you have already, Install the set / list / pack; All sets ›

## Board 4: Updates and going back (04-updates)
- [ ] "Nothing updates by itself. Hearth checks every mod's page each night and tells you; you pick what goes in, and a snapshot is made first." checked N min ago, Check now
- [ ] Updates · 7: This game, All games, Pinned; Update 3 picked
- [ ] Each update: tick, name, game, old → new version, size, date, source tag (NEXUS MODS, SKSE'S OWN SITE, THUNDERSTORE, MODRINTH), FIXES YOUR CRASH / SAFE / NEW GAME BEST
- [ ] WHAT CHANGED (ITS AUTHOR'S CHANGELOG) for the picked update, with "Nothing to redo: …"
- [ ] GONE FROM ITS SITE: a taken-down mod, "Your copy stays on the home server…", Find one like it
- [ ] SAME UPDATES ON YOUR OTHER DEVICES (the Ally's profile, the laptop from the home server)
- [ ] "Pinned and never offered: …" with Change
- [ ] Snapshots (the whole setup, before every change): the timeline, + Take one now, each with when and why
- [ ] GOING BACK TO <snapshot> CHANGES: Mods, Versions, Load order, Profile INIs, Saves made since
- [ ] ↶ Go back to Tuesday's setup; "Tonight's setup becomes a snapshot too, so you can come forward again"
- [ ] Saves that need a mod: each save with its picture, when, level, NEEDS <mod> / WORKS <snapshot>; "Loading a marked save without Lux: Hearth asks first and offers to turn Lux on for that start only."
- [ ] How updates work: Check for updates (Never / Every night / When I open it), Install updates by themselves, Tell me on the Ally and the TV, Keep snapshots (60 days · up to 20 GB), Old versions kept on the home server
- [ ] OR GO BACK ONE MOD ONLY: versions kept per mod

## Board 5: when a game crashes (05-crash)
- [ ] The head: CRASHED TONIGHT <time>, "<N> MIN INTO PLAY · <profile>", CRASH LOG READ; "Crashed in SkyUI's menu: update SkyUI to 5.2.1" and the plain words
- [ ] Update SkyUI and play, Send logs
- [ ] Fix it in one tap (a snapshot comes first): 1 Update (Do it), 2 Turn it off for one test start (Try), 3 Go back to Tuesday's setup (Go back), 4 Find it by halves (Start)
- [ ] Which mod probably did it: each with very likely / maybe / unlikely, its bar and why
- [ ] Others had it too
- [ ] The crash log: Crash Logger's log, Proton's log, What ran; Copy
- [ ] What happened / Not the cause, in plain words
- [ ] The log with the lines that matter marked; "Read from the log Crash Logger writes · lines that matter are marked"
- [ ] Crash Logger SSE / F4SE and .NET Script Framework logs read
- [ ] Crashes lately: each with when, what, game, device, FIX READY / FIXED; "this week · 3"
- [ ] Send logs: what it sends, a note, Send logs
- [ ] When a game crashes: Read the crash log and tell me, Show it in Game Mode too, Get Crash Logger for games that need it, Fix things by itself (off: every fix waits for you)
- [ ] AFTER A FIX: the test start (main menu, loads the save, opens the menu, 60 seconds, told it's fixed)

## Board 6: Describe it (06-describe)
- [ ] Say what you want "for Skyrim SE · the ROG Ally": the person's words and Hearth AI's answers, "Nothing is installed until you say."
- [ ] Suggestions: Lighter, 70 fps · No grass mods · Only free sources · Explain the order
- [ ] YOU CAN ALSO ASK with examples
- [ ] Tell it more…, Speak, Send; "Hearth AI on the Nvidia PC · reads mod pages, your setup and the Ally's numbers · nothing leaves home"
- [ ] Hearth AI (Colibri on the Nvidia PC over Hearth Link, Ollama the fallback), as Describe it elsewhere
- [ ] The proposed set · N mods, NOT INSTALLED, "in this order · lower wins": groups, #, MOD, FROM (NEXUS MODS, HOME SERVER, YOUR PROFILE), ALLY FPS, SIZE
- [ ] WHY THIS ORDER
- [ ] "All in: −9 fps · 14.2 GB · …", Every file it changes ›
- [ ] On the ROG Ally (1080p · 15 W · FSR on): fps by place with the 60 fps line, "now, without the set", Memory, Space on the Ally, Measure it on the Ally first
- [ ] How it looks: Now / With the set (from the mods' pages)
- [ ] Where Hearth AI runs; "Nothing installed until you say. It goes into a new profile… A snapshot comes first."
- [ ] Install the N mods, Change something, Save for later

## Board 7: Game Mode on the TV and the Ally (07-game-mode)
- [ ] The head: MM, the game, "Mods · <profile> · N mods · on the TV"; LB / RB tabs Mods, Plugins, Find, Updates N, Crashes N, Describe it
- [ ] The ladder big: separators, handle, switch, priority, name, version or state ("5.2 → 5.2.1 ready", "1.1 · needs a newer SKSE"), a picked-up mod ("picked up")
- [ ] READY TO PLAY · Play modded ("Starts with SKSE · the mods are a layer over the game while it runs")
- [ ] HEALTH · N TO FIX with Y Fix both
- [ ] MOVING <mod>: "Now at 151 · wins … · loses …", "Below 202X: would lose 318 more files", beats / loses to with counts, "↑ ↓ to move · A to drop · B puts it back · the conflict lens follows as it moves"
- [ ] "Made of blocks · hold Y to Edit layout"
- [ ] Controller hints: A Drop here, B Put it back, X On / off, Y More, ≡ Play modded, ⧉ Profiles
- [ ] THE SAME ON THE ROG ALLY: Same lists, sized for the screen (the Ally's own profile Vanilla+)
- [ ] QUICK ACCESS ON THE ALLY · MODS: downloads with progress, modlist files to fetch, updates, crash with fix ready
- [ ] TYPING, WHEN IT'S NEEDED: the on-screen keyboard or voice, no file paths; Adult mods hidden unless shown; Desktop's Mods window (⋯ › Open on the desktop)
- [ ] IN STEAM'S GAME MODE TOO: "<Game> (Modded)" in Steam's library through Add to Steam (Hearth Sync)

## Board 8: every game (08-every-game)
- [ ] "Every game, one place", "8 games · 974 mods · 5 organizer instances · each game modded the way its players do it"; Check every game, + Add a game
- [ ] Each game's card: picture, NEEDS A LOOK / HEALTHY / UPDATES, mod count, devices (PC, ALLY, LAPTOP, TRUCK), health line, how its mods go in, ▶ Play modded, Mods
- [ ] Skyrim SE, Fallout 4 (Hearth Mod Organizer · plugins), Cyberpunk 2077 (REDmod and archives), Baldur's Gate 3 (mod.io and Nexus · load order), Stardew Valley (SMAPI), Valheim (BepInEx · Thunderstore), Minecraft (Fabric · modpacks), BFME II · Edain (through Beacon)
- [ ] Found by themselves (moddable games on your devices): Set up mods / Look; "Scans Steam, Heroic, Lutris, Hearth's library and Beacon each start · Not a game? Hide it"
- [ ] On your other devices: DEVICE, GAMES MODDED, PROFILE, STATE; Send a game's mods to a device (Choose…); "Saves made with mods go along with the profile · settings stay each device's own"
- [ ] Kept on your home server: MOD FILES (every version kept), SAVED THIS MONTH (not downloaded twice); "Taken-down mods stay here · in Backups each night"
- [ ] Mod files kept on the home server so each download happens once
- [ ] Accounts: NEXUS MODS, MOD.IO, CURSEFORGE ("Hearth's own app key"), THUNDERSTORE ("no account needed"), each with its state; Manage; "and LoversLab, MEGA, Google for modlists"
- [ ] CurseForge: needs CurseForge's API key (the owner applies)
- [ ] "Updates in the Hearth Store · Mod Manager is its own Store app · up to date"
