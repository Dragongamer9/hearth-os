# Capture Card (a Hearth Store app)

Mockups: capture-card/capture-card-1-viewer.png, capture-card/capture-card-1b-viewer-recording.png, capture-card/capture-card-2-no-signal.png, capture-card/capture-card-2b-no-card.png, capture-card/capture-card-3-settings.png, capture-card/capture-card-4-recordings.png, capture-card/capture-card-4b-screenshots.png, capture-card/capture-card-5-game-mode-viewer.png, capture-card/capture-card-5b-game-mode-quick-menu.png
Release: next

Store app 1.0 (the owner's number, Nov 20): `Release: next` because Release names a Hearth OS update and Hearth OS 1.0
is already released; the app's own version is in hosts/capture-card/Cargo.toml. It's a Hearth Store app with its own
updates, apart from Hearth OS releases; like Console Remote Play it needs its own GitHub repo for the Store, which the
owner makes later.

Approved by the owner on Nov 20, 2026 (13:35), with the name Capture Card. Source of the pictures: hearth-workspace
`hearth/mockups/capture-viewer` (viewer.html, make.py). The small questions weren't answered, so they're built as
drawn: keyboard, touchpad or a paired controller all work; recordings open in Hearth Player; the delay readout is
shown, measured from the card to the screen; recording is MP4 at 1080p60 with no quality setting. Yellow notes on
pages 1b and 5 explain behaviour and aren't parts. The game in the pictures is made up.

Built in hosts/capture-card (hearth-capture-card, pure Rust on hearth-ui, the picture and sound through GStreamer).
Parts that need a real capture card are ticked once pictured with CI's stand-in card and say "(needs testing on a
device)".

## 1 Viewer (desktop, full screen)
- [x] Picture from the card fills the screen; nothing else on screen when the bar is hidden — hosts/capture-card/src/desk.rs: "\"stretch\" => iced::ContentFit::Fill"
- [x] Top pill when it starts: source, 1080p60, "Move the mouse or press any key to show the bar · Esc leaves full screen" — hosts/capture-card/src/desk.rs: "Move the mouse or press any key to show the bar · Esc leaves full screen"
- [x] Slim bar, hides 2 s after the mouse stops: card name, USB 3.0 · MJPEG, green 1080p60 signal badge — hosts/capture-card/src/main.rs: "after(2000, Msg::BarHide(a.bar_gen))"
- [x] Delay readout (45 ms) and the Low delay toggle — hosts/capture-card/src/desk.rs: "src(delay_words(a), \"delay\".into())"
- [x] Volume: speaker button mutes and unmutes (M), slider with its number — hosts/capture-card/src/desk.rs: "Msg::Mute), volume_slider(a.prefs.volume, 120.0)"
- [x] Picture fit: Fit, Fill, Stretch — hosts/capture-card/src/desk.rs: "(\"Stretch\".into(), f == \"stretch\", Msg::Fit(\"stretch\"))"
- [x] Screenshot, Record, Full screen (tooltip "Leave full screen · F11"), Recordings, Settings — hosts/capture-card/src/desk.rs: "\"Leave full screen · F11\""
- [x] While recording: only the REC pill (time, size, free space) stays — hosts/capture-card/src/desk.rs: "format!(\"REC {}\", rec_clock(since.elapsed().as_secs()))"
- [x] "Screenshot saved" toast with a thumbnail, its path, Show and Copy — hosts/capture-card/src/desk.rs: "one(t6(\"Screenshot saved\", 15.0, TX))"

## 2 No signal / no card
- [x] Card found, no signal: title, status line (card, USB 3.0 port, sound, HDMI), Settings button — hosts/capture-card/src/desk.rs: "\"HDMI: **no signal**\""
- [x] Tip: use a USB 3.0 port, with "You're on USB 3.0" when it is — hosts/capture-card/src/desk.rs: "\"You're on USB 3.0\""
- [x] Tip: turn off HDCP on the Xbox at Settings › General › TV & display options › Video modes (untick Allow HDCP) — hosts/capture-card/src/desk.rs: "then untick **Allow HDCP**"
- [x] Tip: set the Xbox to 1080p at 60 Hz — hosts/capture-card/src/desk.rs: "\"Set the Xbox to 1080p at 60 Hz\""
- [x] Tip: check the HDMI cable (into HDMI IN, Xbox on) — hosts/capture-card/src/desk.rs: "Xbox HDMI out goes into the card's **HDMI IN**"
- [x] No card: the Xbox › card › This PC picture, "Plug in your capture card", the three short tips, "Looking for a capture card…" (it waits for a USB plug-in event and doesn't poll) — hosts/capture-card/src/desk.rs: "Looking for a capture card… (it watches for USB devices being plugged in; nothing runs in between)"
- [x] The picture comes up by itself when the signal or card arrives — hosts/capture-card/src/main.rs: "The signal came back while waiting: the picture at your mode."

## 3 Settings
- [x] Capture card picker with its connection (USB 3.0, chip name) — hosts/capture-card/src/desk.rs: "format!(\"Connected on {} · {}\", c.usb_words(), c.chip)"
- [x] Resolution picker and frame rate 30 / 50 / 60 — hosts/capture-card/src/desk.rs: "\"Resolution and frame rate\""
- [x] Video format MJPEG / YUYV with a plain explanation — hosts/capture-card/src/desk.rs: "YUYV is uncompressed: a touch sharper, more work for USB."
- [x] Low delay — hosts/capture-card/src/desk.rs: "\"Shows each frame the moment it arrives, without smoothing."
- [x] Sound comes from (the card's USB microphone, picked by itself), Play the Xbox's sound on/off — hosts/capture-card/src/desk.rs: "format!(\"{base} (the card's sound)\")"
- [x] Recordings folder and Screenshots folder, each with Open and Change… — hosts/capture-card/src/desk.rs: "Msg::ChangeDir(\"screenshotsDir\")"
- [x] Start full screen, Keep the screen awake (only while a picture is showing) — hosts/capture-card/src/main.rs: "\"--why=Showing your capture card's picture\""
- [x] Hearth Store section: "Capture Card is a Hearth Store app", Up to date, Open in the Store — hosts/capture-card/src/desk.rs: "\"Capture Card is a Hearth Store app\""
- [x] "Your choices are saved the moment you change one" (settings_file.rs; opening writes nothing) — hosts/capture-card/src/prefs.rs: "Opening the app or a"

## 4 Recordings and screenshots
- [x] Tabs Recordings · 6, Screenshots · 14, Everything; Newest first — hosts/capture-card/src/desk.rs: "\"Newest first ▾\""
- [x] Grid of thumbnails with length, date and size; the picked one outlined — hosts/capture-card/src/desk.rs: "format!(\"{} · {}\", cc::when(it.mtime, now), cc::size(it.size))"
- [x] Side panel: preview, name, date, details (length, picture, size, folder) — hosts/capture-card/src/desk.rs: "kv(\"Saved in\""
- [x] Recordings: Play, Show in folder, Rename, Delete; Screenshots: Copy, Show in folder, Rename, Delete — hosts/capture-card/src/desk.rs: "btn(\"Delete\", None, B::Danger"
- [x] Free space; A/X/Y/B and LB/RB hints — hosts/capture-card/src/desk.rs: "lbrb(\"Recordings / Screenshots\")"

## 5 Game Mode
- [x] Opens full screen; a top info pill (source, 1080p60, delay, Low delay) and the hints fade after 3 s — hosts/capture-card/src/game.rs: "format!(\"Delay {} · {ld}\", delay_words(a))"
- [x] Hints: ☰ Quick menu, X Screenshot, Y Record, ◀▶ Volume, hold B to leave — hosts/capture-card/src/game.rs: "hint(&[\"B\"], \"Hold to leave\", 17.0)"
- [x] Quick menu (right panel): status line, Low delay, Volume, Mute, Picture Fit/Fill/Stretch, Screenshot, Record, Recordings & screenshots (with counts), Settings, Leave Capture Card; big focus ring; A / ◀▶ / B hints — hosts/capture-card/src/game.rs: "title(\"Leave Capture Card\")"

## Every window page
- [x] Title bar: the app's tile, "Capture Card · Hearth", minimize, maximize, close — hosts/capture-card/src/desk.rs: "one(t6(\"Capture Card\", 17.0, TX))"
- [x] Sidebar: Live view, Recordings & screenshots with its count, Settings — hosts/capture-card/src/desk.rs: "nav_item(G::List, \"Recordings & screenshots\""
- [x] Sidebar foot: the card's status (name and signal, or No capture card · Plug one into USB 3.0) and "From the Hearth Store" — hosts/capture-card/src/desk.rs: "one(t(\"From the Hearth Store\", 13.0, MUT))"
- [x] The controller hints strip along the bottom (A Select, B Back, LB/RB Live view / Recordings / Settings) — hosts/capture-card/src/desk.rs: "lbrb(\"Live view / Recordings / Settings\")"

## Behind the pages
- [x] Video through GStreamer: v4l2src, MJPEG decoded by the graphics card (VA-API) where it can be, else jpegdec; YUYV converted; into the window (needs testing on a device) — hosts/capture-card/src/video.rs: "(true, true) => \"vajpegdec name=dec\""
- [x] The card's sound played on this PC's speakers through PipeWire, with volume and mute (needs testing on a device) — hosts/capture-card/src/video.rs: "pipewiresrc name=asrc target-object"
- [x] Recording to MP4 (H.264 by the graphics card where it can be, AAC or Opus sound) and PNG screenshots (needs testing on a device) — hosts/capture-card/src/video.rs: "mp4mux name=mux fragment-duration=1000"
- [x] Light at rest: nothing polls (CI measures the no-card and no-signal pages at rest, and the viewer while it shows a picture) — .github/scripts/capture-card-test.sh: "measure cc-2b-no-card RESULT 20 $pid"
