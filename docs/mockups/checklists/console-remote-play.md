# Hearth Console Remote Play (a Hearth Store app)

Mockups: console-remote-play/remote-play-1a-welcome.png, console-remote-play/remote-play-1b-sign-in-window.png, console-remote-play/remote-play-1c-signed-in.png, console-remote-play/remote-play-2-my-consoles.png, console-remote-play/remote-play-3-connecting.png, console-remote-play/remote-play-4-streaming-bar.png, console-remote-play/remote-play-4b-streaming-data-warning.png, console-remote-play/remote-play-5-connection-trouble.png, console-remote-play/remote-play-6-settings.png, console-remote-play/remote-play-8-cloud-optional.png, console-remote-play/remote-play-g1-ally-sign-in.png, console-remote-play/remote-play-g2-ally-consoles.png, console-remote-play/remote-play-g3-ally-connecting.png, console-remote-play/remote-play-g5-ally-trouble.png, console-remote-play/remote-play-g6-ally-settings.png, console-remote-play/remote-play-g7a-ally-streaming-overlay.png, console-remote-play/remote-play-g7b-ally-quick-menu.png, console-remote-play/remote-play-g8-ally-cloud-optional.png
Release: next

Approved by the owner on Nov 20, 2026 (06:14). Source of the pictures: hearth-workspace
`hearth/mockups/remote-play` (remote-play.html, make.py; the decisions are in its README). It's a Hearth Store app
with its own updates, apart from Hearth OS releases: `Release: next` until the owner picks its number. The plan is
docs/CONSOLE-REMOTE-PLAY.md. Yellow notes on pages 1b, 4b, g7a and g7b explain behaviour and aren't parts.
Parts that need a real Xbox are ticked only once pictured with CI's stand-in and say "(needs testing on a device)".

## 1 Welcome and sign-in
- [ ] Welcome: app icon (no Microsoft or Xbox marks), "Play your Xbox from anywhere", what it does
- [ ] Sign in with Microsoft button that opens a sign-in window inside the app showing Microsoft's own page
- [ ] Line under it: "You type your password on Microsoft's page, never into this app"
- [ ] Three short needs: the account on your Xbox, Remote features on, about 10 Mbps upload at home
- [ ] "Hearth Console Remote Play is a Hearth app. It isn't made by Microsoft."
- [ ] Sign-in window inside the app, over the dimmed app: lock, "Sign in with Microsoft", "Microsoft's own page" label, ✕
- [ ] Read-only address line (login.live.com), "this page comes from Microsoft"
- [ ] Microsoft's real page in the window (only an outline in the mockup)
- [ ] Steps: Microsoft's page opened ✓, Sign in on Microsoft's page (spinner), The window closes by itself
- [ ] Note: never sees or stores the password; keeps only the sign-in token, in the system keyring; Sign out removes it
- [ ] Reload page, Cancel
- [ ] Signed in: big check, "Signed in as" + gamertag only (no email), Signed in chip, Find my Xbox

## 2 My consoles
- [ ] Refresh button; the list is asked for only when the page opens or on Refresh (no polling)
- [ ] Console card: plain box picture with its status light, name, model, account, status chip, "Remote features on", last seen
- [ ] Wake and Stream buttons; "Stream wakes it first if it's sleeping"
- [ ] What the status means: On, Sleeping (Wake works, about 15 s), Offline (can't be woken)
- [ ] Can't see your Xbox? 1: Settings › Devices & connections › Remote features › Enable remote features
- [ ] 2: Settings › General › Power options › Sleep (not Shutdown)
- [ ] 3: Leave it plugged in, ideally wired (Ethernet)
- [ ] 4: The same account signed in on the Xbox

## 3 Connecting
- [ ] Console picture, "Connecting to Living room", over the internet
- [ ] Steps: Waking your Xbox (done, time taken), Connecting (spinner, timer), Starting the picture (waiting)
- [ ] Cancel (focused), "usually 10 to 30 seconds; your Xbox stays on if you cancel"

## 4 Streaming (desktop)
- [ ] Full screen picture; a top pill when it starts (console, "move the mouse to show the bar · F11 leaves full screen")
- [ ] Slim bar that hides 2 s after the mouse stops: console name and model
- [ ] Connection quality (bars + Good / Fair / Poor)
- [ ] Delay in ms, bitrate in Mbps, resolution badge (1080p60)
- [ ] Data used this session
- [ ] Volume slider with its number
- [ ] Quality: Auto / 720p / 1080p
- [ ] Data saver toggle
- [ ] Screenshot, Leave full screen, Settings
- [ ] Disconnect (red), tooltip "Leave your Xbox on · it sleeps by itself later"
- [ ] Bar hidden: nothing covers the game
- [ ] Data warning once at the warning level, on every network: "18.1 GB of your 20 GB monthly streaming data used", Turn on Data saver, Keep going
- [ ] "Screenshot saved" toast with a thumbnail, its path, Show and Copy

## 5 Connection trouble
- [ ] "Couldn't reach your Xbox" with what happened in plain words
- [ ] Status line: Xbox on, signed in, this laptop's network (Wi-Fi band and name); Try again, Try at 720p
- [ ] Tip: check the home upload speed (about 10 Mbps or more; under 5 it stutters)
- [ ] Tip: use 5 GHz Wi-Fi or the phone's hotspot (with the data it uses)
- [ ] Tip: work Wi-Fi may block game streaming
- [ ] Still nothing? → Show the list (the My consoles checklist)

## 6 Settings
- [ ] Default quality: Auto / 720p / 1080p
- [ ] Data saver: Off / On a hotspot / Always, with the data per hour
- [ ] Monthly data limit: on/off, Limit (20 GB default), Warn me at (90 % default), day of the month, a meter with the warning mark, used this month
- [ ] Counted on every network (home, work Wi-Fi, hotspot); starts again on the 1st; at the limit it asks, never stops a game
- [ ] Controller: Built-in controls (Ally) / A paired controller; Vibration on/off
- [ ] Open the menu while streaming: hold View + Menu 1 s (keyboard Ctrl+Shift+M)
- [ ] Start full screen, Keep the screen awake
- [ ] Sound plays on (audio output picker)
- [ ] Account: Signed in as gamertag, Sign out
- [ ] Hearth Store: "a Hearth Store app", updates apart from Hearth OS, not made by Microsoft, Up to date, Open in the Store
- [ ] "Your choices are saved the moment you change one" (settings_file.rs; opening writes nothing)

## 7 Game Mode on the Ally
- [ ] Top bar: icon, Console Remote Play, LB / RB tabs Consoles · Cloud · Settings, the gamertag (or "Not signed in yet")
- [ ] Sign-in: the in-app sign-in window (Microsoft's page label, read-only address, Page opened / Signing in…, the token note, Cancel), Hearth's on-screen keyboard for its boxes
- [ ] Consoles: the console card with Wake and Stream, the status legend, the four-step checklist; A Stream, X Wake, Y Refresh, B Back
- [ ] Connecting: the three steps and Cancel
- [ ] Streaming overlay: picture letterboxed to 16:9, stats pill (quality, delay, bitrate, resolution, data), fades after 3 s
- [ ] Hint: hold View + Menu 1 s for the Console Remote Play menu; every other button plays the Xbox
- [ ] Quick menu: status line, data (now and this month, every network), Quality, Data saver, Volume, Show stats, Screenshot, Settings, Vibration, Disconnect, the combo reminder; big focus ring; A / ◀▶ / B hints
- [ ] While the quick menu is open the Xbox gets no button presses; B goes back to the game
- [ ] Trouble: the three tips, Try again, Try at 720p, Back to consoles
- [ ] Settings: every setting from page 6 in Game Mode rows, the data limit counted on every network
- [ ] Cloud (optional): Optional and Game Pass Ultimate chips, Recently played / All cloud games / Search, tiles with focus ring; A Play, Y Search, B Back

## 8 Cloud (optional)
- [ ] Only shown when the account has Game Pass Ultimate; "Optional" in the sidebar and title
- [ ] Recently played / All cloud games / Search
- [ ] Game tiles (made-up names and art in the mockup), the focused one outlined
- [ ] Uses the same bar, quality and data settings as the Xbox stream
- [ ] The delay number shows in the bar at all times (not only with Show stats)

## Every window page (desktop)
- [ ] The sidebar: My consoles with its count, Cloud marked optional, Settings
- [ ] The gamertag and the console status at the bottom of the sidebar
- [ ] "From the Hearth Store · not made by Microsoft"
- [ ] The controller hints strip
- [ ] No Microsoft or Xbox logos or brand art anywhere; the console is plain text and a plain box
