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
- [ ] Picture from the card fills the screen; nothing else on screen when the bar is hidden
- [ ] Top pill when it starts: source, 1080p60, "Move the mouse or press any key to show the bar · Esc leaves full screen"
- [ ] Slim bar, hides 2 s after the mouse stops: card name, USB 3.0 · MJPEG, green 1080p60 signal badge
- [ ] Delay readout (45 ms) and the Low delay toggle
- [ ] Volume: speaker button mutes and unmutes (M), slider with its number
- [ ] Picture fit: Fit, Fill, Stretch
- [ ] Screenshot, Record, Full screen (tooltip "Leave full screen · F11"), Recordings, Settings
- [ ] While recording: only the REC pill (time, size, free space) stays
- [ ] "Screenshot saved" toast with a thumbnail, its path, Show and Copy

## 2 No signal / no card
- [ ] Card found, no signal: title, status line (card, USB 3.0 port, sound, HDMI), Settings button
- [ ] Tip: use a USB 3.0 port, with "You're on USB 3.0" when it is
- [ ] Tip: turn off HDCP on the Xbox at Settings › General › TV & display options › Video modes (untick Allow HDCP)
- [ ] Tip: set the Xbox to 1080p at 60 Hz
- [ ] Tip: check the HDMI cable (into HDMI IN, Xbox on)
- [ ] No card: the Xbox › card › This PC picture, "Plug in your capture card", the three short tips, "Looking for a capture card…" (it waits for a USB plug-in event and doesn't poll)
- [ ] The picture comes up by itself when the signal or card arrives

## 3 Settings
- [ ] Capture card picker with its connection (USB 3.0, chip name)
- [ ] Resolution picker and frame rate 30 / 50 / 60
- [ ] Video format MJPEG / YUYV with a plain explanation
- [ ] Low delay
- [ ] Sound comes from (the card's USB microphone, picked by itself), Play the Xbox's sound on/off
- [ ] Recordings folder and Screenshots folder, each with Open and Change…
- [ ] Start full screen, Keep the screen awake (only while a picture is showing)
- [ ] Hearth Store section: "Capture Card is a Hearth Store app", Up to date, Open in the Store
- [ ] "Your choices are saved the moment you change one" (settings_file.rs; opening writes nothing)

## 4 Recordings and screenshots
- [ ] Tabs Recordings · 6, Screenshots · 14, Everything; Newest first
- [ ] Grid of thumbnails with length, date and size; the picked one outlined
- [ ] Side panel: preview, name, date, details (length, picture, size, folder)
- [ ] Recordings: Play, Show in folder, Rename, Delete; Screenshots: Copy, Show in folder, Rename, Delete
- [ ] Free space; A/X/Y/B and LB/RB hints

## 5 Game Mode
- [ ] Opens full screen; a top info pill (source, 1080p60, delay, Low delay) and the hints fade after 3 s
- [ ] Hints: ☰ Quick menu, X Screenshot, Y Record, ◀▶ Volume, hold B to leave
- [ ] Quick menu (right panel): status line, Low delay, Volume, Mute, Picture Fit/Fill/Stretch, Screenshot, Record, Recordings & screenshots (with counts), Settings, Leave Capture Card; big focus ring; A / ◀▶ / B hints

## Every window page
- [ ] Title bar: the app's tile, "Capture Card · Hearth", minimize, maximize, close
- [ ] Sidebar: Live view, Recordings & screenshots with its count, Settings
- [ ] Sidebar foot: the card's status (name and signal, or No capture card · Plug one into USB 3.0) and "From the Hearth Store"
- [ ] The controller hints strip along the bottom (A Select, B Back, LB/RB Live view / Recordings / Settings)

## Behind the pages
- [ ] Video through GStreamer: v4l2src, MJPEG decoded by the graphics card (VA-API) where it can be, else jpegdec; YUYV converted; into the window (needs testing on a device)
- [ ] The card's sound played on this PC's speakers through PipeWire, with volume and mute (needs testing on a device)
- [ ] Recording to MP4 (H.264 by the graphics card where it can be, AAC or Opus sound) and PNG screenshots (needs testing on a device)
- [ ] Light at rest: nothing polls (CI measures the no-card and no-signal pages at rest, and the viewer while it shows a picture)
