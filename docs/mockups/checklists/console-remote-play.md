# Hearth Console Remote Play (a Hearth Store app)

Mockups: console-remote-play/remote-play-1a-welcome.png, console-remote-play/remote-play-1b-sign-in-window.png, console-remote-play/remote-play-1c-signed-in.png, console-remote-play/remote-play-2-my-consoles.png, console-remote-play/remote-play-3-connecting.png, console-remote-play/remote-play-4-streaming-bar.png, console-remote-play/remote-play-4b-streaming-data-warning.png, console-remote-play/remote-play-5-connection-trouble.png, console-remote-play/remote-play-6-settings.png, console-remote-play/remote-play-8-cloud-optional.png, console-remote-play/remote-play-g1-ally-sign-in.png, console-remote-play/remote-play-g2-ally-consoles.png, console-remote-play/remote-play-g3-ally-connecting.png, console-remote-play/remote-play-g5-ally-trouble.png, console-remote-play/remote-play-g6-ally-settings.png, console-remote-play/remote-play-g7a-ally-streaming-overlay.png, console-remote-play/remote-play-g7b-ally-quick-menu.png, console-remote-play/remote-play-g8-ally-cloud-optional.png
Release: next

Approved by the owner on Nov 20, 2026 (06:14). Source of the pictures: hearth-workspace
`hearth/mockups/remote-play` (remote-play.html, make.py; the decisions are in its README). It's a Hearth Store app
with its own updates, apart from Hearth OS releases: `Release: next` until the owner picks its number. The plan is
docs/CONSOLE-REMOTE-PLAY.md. Yellow notes on pages 1b, 4b, g7a and g7b explain behaviour and aren't parts.
Parts that need a real Xbox are ticked only once pictured with CI's stand-in and say "(needs testing on a device)".

Built in hosts/console-remote-play (hearth-console-remote-play, pure Rust on hearth-ui, the stream through GStreamer's
webrtcbin) and its sign-in window (helper/hearth-console-remote-play-signin, WebKitGTK). Pictured by
.github/workflows/console-remote-play.yml against a stand-in for Microsoft's services (tests/fixtures/http.json; the
stream page shows the fixture's still picture): crp-1a … crp-8 at 1920 × 1080, crp-g1 … crp-g8 at 1280 × 800, each
checked by eye against its mockup; CI pictures are in hearth-workspace hearth/assets/console-remote-play-shots.
Known differences that come from data, not the build: "Last seen just now" (the app knows when it asked, not when the
Xbox was last online), the Cloud tiles' second line is the game's kind (Microsoft's list has no "played yesterday"),
CI's sign-in address is the stand-in page's file:// address and CI says "this PC" (no battery, so not a laptop).
The Ally's trouble page (g5) is pictured drawn by the CPU (crp-g5-ally-trouble) and by the graphics card
(crp-g5-ally-trouble-gpu); both are whole. Not proven anywhere yet: a real sign-in, Wake, the picture and sound, the delay, the controller and rumble on the Xbox.

## 1 Welcome and sign-in
- [x] Welcome: app icon (no Microsoft or Xbox marks), "Play your Xbox from anywhere", what it does — hosts/console-remote-play/src/desk.rs: "\"Play your Xbox from anywhere\""
- [x] Sign in with Microsoft button that opens a sign-in window inside the app showing Microsoft's own page (needs testing on a device) — hosts/console-remote-play/src/main.rs: "fn sign_in(ally: bool)"
- [x] Line under it: "You type your password on Microsoft's page, never into this app" — hosts/console-remote-play/src/desk.rs: "You type your password on Microsoft's page, never into this app."
- [x] Three short needs: the account on your Xbox, Remote features on, about 10 Mbps upload at home — hosts/console-remote-play/src/desk.rs: "\"Good internet both ends\""
- [x] "Hearth Console Remote Play is a Hearth app. It isn't made by Microsoft." — hosts/console-remote-play/src/desk.rs: "It isn't made by Microsoft."
- [x] Sign-in window inside the app, over the dimmed app: lock, "Sign in with Microsoft", "Microsoft's own page" label, ✕ — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "label(\"Microsoft's own page\", ('chip',))"
- [x] Read-only address line (login.live.com), "this page comes from Microsoft" — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "Read-only address · this page comes from Microsoft"
- [x] Microsoft's real page in the window (only an outline in the mockup) (needs testing on a device) — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "WebKit2.WebContext.new_ephemeral()"
- [x] Steps: Microsoft's page opened ✓, Sign in on Microsoft's page (spinner), The window closes by itself — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "The window closes by itself"
- [x] Note: never sees or stores the password; keeps only the sign-in token, in the system keyring; Sign out removes it — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "<b>Sign out</b> in Settings removes it."
- [x] Reload page, Cancel — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "self.button('Reload page'"
- [x] Signed in: big check, "Signed in as" + gamertag only (no email), Signed in chip, Find my Xbox — hosts/console-remote-play/src/desk.rs: "\"Only your gamertag is shown. Your email and password stay with Microsoft.\""

## 2 My consoles
- [x] Refresh button; the list is asked for only when the page opens or on Refresh (no polling) — hosts/console-remote-play/src/desk.rs: "Hearth asks Microsoft for this list only when this page opens or you press Refresh."
- [x] Console card: plain box picture with its status light, name, model, account, status chip, "Remote features on", last seen — hosts/console-remote-play/src/desk.rs: "format!(\"Last seen {}\", ago(c.seen))"
- [x] Wake and Stream buttons; "Stream wakes it first if it's sleeping" (needs testing on a device) — hosts/console-remote-play/src/desk.rs: "\"Stream wakes it first if it's sleeping.\""
- [x] What the status means: On, Sleeping (Wake works, about 15 s), Offline (can't be woken) — hosts/console-remote-play/src/desk.rs: "\"Asleep but listening. **Wake** turns it on from here, in about 15 seconds.\""
- [x] Can't see your Xbox? 1: Settings › Devices & connections › Remote features › Enable remote features — hosts/console-remote-play/src/desk.rs: "tick **Enable remote features**."
- [x] 2: Settings › General › Power options › Sleep (not Shutdown) — hosts/console-remote-play/src/desk.rs: "With Shutdown (energy saving) it can't be woken from here."
- [x] 3: Leave it plugged in, ideally wired (Ethernet) — hosts/console-remote-play/src/desk.rs: "\"Leave it plugged in, ideally wired\""
- [x] 4: The same account signed in on the Xbox — hosts/console-remote-play/src/desk.rs: "\"Same account\""

## 3 Connecting
- [x] Console picture, "Connecting to Living room", over the internet — hosts/console-remote-play/src/desk.rs: "format!(\"Connecting to {name}\")"
- [x] Steps: Waking your Xbox (done, time taken), Connecting (spinner, timer), Starting the picture (waiting) (needs testing on a device) — hosts/console-remote-play/src/desk.rs: "\"Waking your Xbox\""
- [x] Cancel (focused), "usually 10 to 30 seconds; your Xbox stays on if you cancel" — hosts/console-remote-play/src/desk.rs: "\"This usually takes 10 to 30 seconds. Your Xbox stays on if you cancel.\""

## 4 Streaming (desktop)
- [x] Full screen picture; a top pill when it starts (console, "move the mouse to show the bar · F11 leaves full screen") (needs testing on a device) — hosts/console-remote-play/src/desk.rs: "Move the mouse to show the bar · F11 leaves full screen · your controller plays the Xbox"
- [x] Slim bar that hides 2 s after the mouse stops: console name and model (needs testing on a device) — hosts/console-remote-play/src/main.rs: "after(2000, Msg::BarHide(a.bar_gen))"
- [x] Connection quality (bars + Good / Fair / Poor) (needs testing on a device) — hosts/console-remote-play/src/stream.rs: "(\"Good\", if self.height >= 1080 { 4 } else { 3 })"
- [x] Delay in ms, bitrate in Mbps, resolution badge (1080p60) (needs testing on a device) — hosts/console-remote-play/src/desk.rs: "src(delay, \"delay\", TX)"
- [x] Data used this session — hosts/console-remote-play/src/desk.rs: "\"this session\""
- [x] Volume slider with its number — hosts/console-remote-play/src/desk.rs: "volume_slider(a.prefs.volume, 110.0)"
- [x] Quality: Auto / 720p / 1080p — hosts/console-remote-play/src/desk.rs: "grp(vec![one(t(\"Quality\", 14.0, SOFT)).into()"
- [x] Data saver toggle — hosts/console-remote-play/src/desk.rs: "one(t(\"Data saver\", 14.0, SOFT)).into()"
- [x] Screenshot, Leave full screen, Settings — hosts/console-remote-play/src/desk.rs: "vb(G::Cam, Some(\"Screenshot\")"
- [x] Disconnect (red), tooltip "Leave your Xbox on · it sleeps by itself later" — hosts/console-remote-play/src/desk.rs: "\"Leave your Xbox on · it sleeps by itself later\""
- [x] Bar hidden: nothing covers the game — hosts/console-remote-play/src/main.rs: "if g == a.bar_gen && !a.tip && !a.shows(\"bar\") && !a.menu {"
- [x] Data warning once at the warning level, on every network: "18.1 GB of your 20 GB monthly streaming data used", Turn on Data saver, Keep going — hosts/console-remote-play/src/desk.rs: "\"{} of your {} GB monthly streaming data used\""
- [x] "Screenshot saved" toast with a thumbnail, its path, Show and Copy — hosts/console-remote-play/src/desk.rs: "\"Pictures › Screenshots › {shown}\""

## 5 Connection trouble
- [x] "Couldn't reach your Xbox" with what happened in plain words — hosts/console-remote-play/src/desk.rs: "\"Couldn't reach your Xbox\""
- [x] Status line: Xbox on, signed in, this laptop's network (Wi-Fi band and name); Try again, Try at 720p — hosts/console-remote-play/src/desk.rs: "btn(\"Try at 720p\", None, B::Primary"
- [x] Tip: check the home upload speed (about 10 Mbps or more; under 5 it stutters) — hosts/console-remote-play/src/desk.rs: "\"Check your home upload speed\""
- [x] Tip: use 5 GHz Wi-Fi or the phone's hotspot (with the data it uses) — hosts/console-remote-play/src/desk.rs: "\"Use 5 GHz Wi-Fi or your phone's hotspot\""
- [x] Tip: work Wi-Fi may block game streaming — hosts/console-remote-play/src/desk.rs: "\"Work Wi-Fi may block game streaming\""
- [x] Still nothing? → Show the list (the My consoles checklist) — hosts/console-remote-play/src/desk.rs: "btn(\"Show the list\""

## 6 Settings
- [x] Default quality: Auto / 720p / 1080p — hosts/console-remote-play/src/desk.rs: "\"Default quality\""
- [x] Data saver: Off / On a hotspot / Always, with the data per hour — hosts/console-remote-play/src/desk.rs: "\"720p and a lower bitrate: about 2–3 GB an hour instead of 4–6.\""
- [x] Monthly data limit: on/off, Limit (20 GB default), Warn me at (90 % default), day of the month, a meter with the warning mark, used this month — hosts/console-remote-play/src/desk.rs: "format!(\"Day {day} of {len}\")"
- [x] Counted on every network (home, work Wi-Fi, hotspot); starts again on the 1st; at the limit it asks, never stops a game — hosts/console-remote-play/src/desk.rs: "\"At the limit Hearth asks before streaming more; it never stops a game by itself.\""
- [x] Controller: Built-in controls (Ally) / A paired controller; Vibration on/off — hosts/console-remote-play/src/desk.rs: "(\"builtin\", \"Built-in controls (Ally)\")"
- [x] Open the menu while streaming: hold View + Menu 1 s (keyboard Ctrl+Shift+M) — hosts/console-remote-play/src/desk.rs: "\"Hold **View + Menu** for 1 second. On a keyboard: **Ctrl+Shift+M**.\""
- [x] Start full screen, Keep the screen awake — hosts/console-remote-play/src/desk.rs: "\"Keep the screen awake\""
- [x] Sound plays on (audio output picker) — hosts/console-remote-play/src/desk.rs: "\"Sound plays on\""
- [x] Account: Signed in as gamertag, Sign out — hosts/console-remote-play/src/desk.rs: "btn(\"Sign out\""
- [x] Hearth Store: "a Hearth Store app", updates apart from Hearth OS, not made by Microsoft, Up to date, Open in the Store — hosts/console-remote-play/src/desk.rs: "\"Hearth Console Remote Play is a Hearth Store app\""
- [x] "Your choices are saved the moment you change one" (settings_file.rs; opening writes nothing) — hosts/console-remote-play/src/desk.rs: "\"Your choices are saved the moment you change one, and stay as you left them.\""

## 7 Game Mode on the Ally
- [x] Top bar: icon, Console Remote Play, LB / RB tabs Consoles · Cloud · Settings, the gamertag (or "Not signed in yet") — hosts/console-remote-play/src/ally.rs: "\"Not signed in yet\""
- [x] Sign-in: the in-app sign-in window (Microsoft's page label, read-only address, Page opened / Signing in…, the token note, Cancel), Hearth's on-screen keyboard for its boxes — hosts/console-remote-play/helper/hearth-console-remote-play-signin: "Hearth's on-screen keyboard comes up for each box"
- [x] Consoles: the console card with Wake and Stream, the status legend, the four-step checklist; A Stream, X Wake, Y Refresh, B Back — hosts/console-remote-play/src/ally.rs: "h(&[\"Y\"], \"Refresh\")"
- [x] Connecting: the three steps and Cancel — hosts/console-remote-play/src/ally.rs: "\"Usually 10 to 30 seconds.\""
- [x] Streaming overlay: picture letterboxed to 16:9, stats pill (quality, delay, bitrate, resolution, data), fades after 3 s (needs testing on a device) — hosts/console-remote-play/src/ally.rs: "\"{delay} · {:.1} Mbps · {} · {}\""
- [x] Hint: hold View + Menu 1 s for the Console Remote Play menu; every other button plays the Xbox — hosts/console-remote-play/src/ally.rs: "\"Hold 1 s: Console Remote Play menu\""
- [x] Quick menu: status line, data (now and this month, every network), Quality, Data saver, Volume, Show stats, Screenshot, Settings, Vibration, Disconnect, the combo reminder; big focus ring; A / ◀▶ / B hints (needs testing on a device) — hosts/console-remote-play/src/ally.rs: "\"Game still running\""
- [x] While the quick menu is open the Xbox gets no button presses; B goes back to the game (needs testing on a device) — hosts/console-remote-play/src/spad.rs: "if flags.menu_open.load(Ordering::Relaxed) {"
- [x] Trouble: the three tips, Try again, Try at 720p, Back to consoles — hosts/console-remote-play/src/ally.rs: "btn(\"Back to consoles\""
- [x] Settings: every setting from page 6 in Game Mode rows, the data limit counted on every network — hosts/console-remote-play/src/ally.rs: "\"{} used on every network · warns at {} % ({})\""
- [x] Cloud (optional): Optional and Game Pass Ultimate chips, Recently played / All cloud games / Search, tiles with focus ring; A Play, Y Search, B Back — hosts/console-remote-play/src/ally.rs: "h(&[\"Y\"], \"Search\")"

## 8 Cloud (optional)
- [x] Only shown when the account has Game Pass Ultimate; "Optional" in the sidebar and title — hosts/console-remote-play/src/desk.rs: "\"Games that run on Microsoft's servers instead of your Xbox, if your account has Game Pass Ultimate."
- [x] Recently played / All cloud games / Search — hosts/console-remote-play/src/desk.rs: "(\"All cloud games\".into()"
- [x] Game tiles (made-up names and art in the mockup), the focused one outlined — hosts/console-remote-play/src/desk.rs: "a.ring && a.focus == j + 1"
- [x] Uses the same bar, quality and data settings as the Xbox stream (needs testing on a device) — hosts/console-remote-play/src/main.rs: "fn begin_cloud(a: &mut App, i: usize)"
- [x] The delay number shows in the bar at all times (not only with Show stats) — hosts/console-remote-play/src/desk.rs: "let delay = s.delay_ms.map(|d| format!(\"{d} ms\"))"

## Every window page (desktop)
- [x] The sidebar: My consoles with its count, Cloud marked optional, Settings — hosts/console-remote-play/src/desk.rs: "nav_item(G::Cloud, \"Cloud\""
- [x] The gamertag and the console status at the bottom of the sidebar — hosts/console-remote-play/src/desk.rs: "format!(\"{name} · {}\", p.words())"
- [x] "From the Hearth Store · not made by Microsoft" — hosts/console-remote-play/src/desk.rs: "\"From the Hearth Store · not made by Microsoft\""
- [x] The controller hints strip — hosts/console-remote-play/src/desk.rs: "hints(left, right, 54.0, 28.0, 28.0, BG)"
- [x] No Microsoft or Xbox logos or brand art anywhere; the console is plain text and a plain box — hosts/console-remote-play/src/look.rs: "never a real console's picture"
