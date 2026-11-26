# Hearth Studio (OBS Studio, GPL-2) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/12-studio.html)

Mockups: open-source-apps/12-studio.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Studio (OBS Studio, GPL-2): scenes and sources (Capture Card, screen, camera, mic), going live, recording, the audio mixer.

## The window
- [x] The desktop's top bar names it "Hearth Studio" (its menu entry's name) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — os/system_files/usr/share/applications/hearth-studio.desktop: "Name=Hearth Studio"
- [x] Its title bar: the ST mark (Hearth's own red square), "Hearth Studio", ● LIVE 0:42:13 and ● REC 0:42:10 (OBS's timecodes; ❚❚ PAUSED when paused), – ▢ ✕ (and Edit layout) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"● LIVE {}\""
- [x] "60 fps · CPU 6% · GPU encoder · 6.2 Mbps · 0 dropped": OBS's own numbers (GetStats, the stream's bytes, its encoder setting) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"{} dropped\""

## What viewers see
- [x] OBS's own picture of the program scene, once a second while Hearth Studio is in front (the game, the round camera, the title overlay and the stats are that scene's sources) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "\"GetSourceScreenshot\""
- [x] "PREVIEW · WHAT VIEWERS SEE · 1920 × 1080" (OBS's canvas size) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "PREVIEW · WHAT VIEWERS SEE · {} × {}"
- [x] When OBS isn't there: Install OBS Studio (Flathub, for you only), Start OBS (its remote control turned on first), or its password (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"Install OBS Studio\""

## Going live
- [x] "Going live" and CONNECTED (Hearth Studio signed in to OBS's remote control) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"CONNECTED\""
- [x] Where: the place OBS goes live to (Twitch on the board) as an accent chip, + to set it (Twitch, YouTube, Kick, your stream key handed to OBS) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "\"SetStreamServiceSettings\""
- [ ] Where: Twitch and YouTube at once (OBS streams to one place at a time; going to two at once needs OBS's own multi-RTMP plugin, which has no remote control for Hearth Studio to drive. Hearth Studio shows and sets the one OBS has)
- [x] Title: "Quad Mech devlog: legs that finally walk", kept, and shown on the stream by your "Overlay: title" text source (the platforms' own titles need their accounts, which Hearth doesn't sign in to) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "Add a text source called \\\"Overlay: title\\\" to show it on the stream."
- [x] Quality 720p · 1080p60 · 1440p (OBS's output size and frame rate; changes once nothing is live) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"1080p60\""
- [x] Also make a 9:16 for phones: each recording also cropped to a 1080 × 1920 copy when it ends (ffmpeg) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "\"crop=ih*9/16:ih,scale=1080:1920\""
- [x] ■ End stream (● Go live when it isn't) and ■ Stop recording (● Start recording) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"■ End stream\""
- [x] ❚❚ Pause recording (▶ Resume), ↺ Save last 30 s (OBS's replay buffer: it's started first if it isn't) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"↺ Save last 30 s\""
- [x] ◉ Virtual camera and ◫ Studio mode (OBS's own) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"◫ Studio mode\""

## Stream health
- [x] "Stream health" and "both look good" (from the dropped frames and OBS's congestion) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"both look good\""
- [x] The upload over the last minute, as a graph (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "ui::graph(ui::Graph { values: vals"
- [x] Viewers 38 (Twitch's count through decapi.me, no account or key; YouTube's needs its account, so it shows —) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "https://decapi.me/twitch/viewercount/{}"
- [x] Frames dropped "0 of 152,040", Upload "6.2 Mbps of 40" (your upload speed, kept in Hearth Studio's settings) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"{:.1} Mbps of {}\""
- [x] Recording to "~/Videos/Studio · 18.4 GB so far" (OBS's recording folder and the recording's size) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"{d} · {} so far\""

## Scenes, what's in them, the sound, switching
- [x] Scenes + (a new scene): OBS's scenes in its own order, the one live lit; a press switches to it (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "fn scenes(a: &App)"
- [x] In this scene + Add (camera, capture card, whole screen, game window, mic, sound, a title, a picture): each source with its kind's colour, ◉ / ○ shown or hidden (pressed: OBS's own), the lock, hidden ones dimmed, the picked one lit (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"+ Add\""
- [x] Sound, "drag to set · click the dot to mute": each source with sound, OBS's live meter (20 times a second), the fader (OBS's cubic one), its dB or "muted", the dot that mutes (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"drag to set · click the dot to mute\""
- [x] "Mic clean-up: noise removal on · leveller on" (the mic's filters in OBS) and Filters… (turn them on or off, or add them) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "Mic clean-up: noise removal {} · leveller {}"
- [x] Switching scenes: Cut · Fade · Slide · Ember (OBS's transitions; Ember is Hearth's own, added to your scene collection while OBS is closed) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/obs.rs: "Hearth's own transition: an ember-orange fade"
- [x] Length "300 ms" (OBS's transition time) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/main.rs: "\"SetCurrentSceneTransitionDuration\""
- [x] HOTKEYS: Ctrl+1–6 · scenes, Ctrl+Shift+R · save last 30 s, Controller: ≡ + Y · next scene (they work while it's open; the controller's from /dev/input) (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "\"Controller: ≡ + Y · next scene\""
- [x] "Hearth Studio · Built on OBS Studio (GPL-2)" (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "Built on OBS Studio (GPL-2)"

## Behind the board
- [x] Drives OBS through obs-websocket v5 (its sign-in with the password OBS keeps, every request and event): a small WebSocket client of its own (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/ws.rs: "pub fn auth(password: &str, salt: &str, challenge: &str) -> String"
- [x] Every part a block (the title bar, what viewers see, going live, the stream's health, scenes, what's in them, the sound, switching, the credit), moved, sized, hidden and customized in Edit layout (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "pub const BLOCKS: &[PageDef]"
- [x] Hearth Studio's own blocks window: its buttons (go live, record, pause, save last 30 s, virtual camera, studio mode, new scene, add, filters, Start OBS), panes and info (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/view.rs: "name: \"Hearth Studio\""
- [x] Its settings kept the safe way (settings_file), only when you change something; OBS's own files only while OBS is closed, merged, with a .bak (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — hosts/hearth-ui/apps/studio/src/obs.rs: "Nothing is written while OBS runs."
- [x] In the Hearth Store's list (built in) and the quick-update list; its menu entry and icon (CI pictures oss-studio-1…7, on a pretend OBS speaking obs-websocket; needs testing with OBS on a device) — os/system_files/usr/share/hearth/store/catalog.json: "\"partId\": \"hearth-studio-app\""
