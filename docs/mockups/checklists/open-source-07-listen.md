# Hearth Listen (whisper.cpp, MIT) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/07-listen.html)

Mockups: open-source-apps/07-listen.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps/listen), made of blocks (studio: Edit layout, Customize, its own
blocks window), built on the open-source project the board credits: whisper.cpp's own command (whisper-cli) and models.

Board: Hearth Listen (whisper.cpp, MIT): recordings, a transcript, live captions, and Send to Voice Studio.
The board's top bar (Apps, Desktops, the clock, Wi-Fi · Sound, Game Mode) and the wallpaper are the desktop's, not
this app's. Pictures: hearth-apps-rust.yml, listen.sh (listen-1-board … listen-8-edit-how).

## Every part of the board

### The window and its title bar
- [x] The LI letter mark (Hearth's own, blue) and the name Hearth Listen — hosts/hearth-ui/apps/listen/src/main.rs: "w::mark(\"LI\", \"bl\", 22.0)"
- [x] "· speech to words, on your own PC" after the name — hosts/hearth-ui/apps/listen/src/main.rs: "· speech to words, on your own PC"
- [x] ● Record (in the accent): records your microphone (pw-record, 16 kHz) and makes it into words when stopped (■ Stop · time while recording) — hosts/hearth-ui/apps/listen/src/main.rs: "\"● Record\".to_string()"
- [x] Open a file: any sound or video file (ffmpeg makes it 16 kHz) made into words — hosts/hearth-ui/apps/listen/src/main.rs: "w::chip(&p, \"Open a file\", false, Sz::Sm, Some(Msg::OpenFile))"
- [x] – ▢ ✕ (minimize, maximize, close; the window moved by its title bar, sized by its edges) — hosts/hearth-ui/apps/listen/src/main.rs: "TitleMsgs { drag: Msg::Drag, maximize: Msg::Max, minimize: Some(Msg::Min), close: Msg::Close }"

### Recordings (the side)
- [x] ⌕ Search every word: every recording's words searched, the list narrowed to the ones that have them, each with how many lines, the words lit in the transcript — hosts/hearth-ui/apps/listen/src/main.rs: "w::search(&p, \"Search every word\", &a.q, Msg::Search)"
- [x] RECORDINGS label — hosts/hearth-ui/apps/listen/src/main.rs: "w::cap(&p, \"Recordings\")"
- [x] Each recording: its dot, its name and "Today 1:12 AM · 4:12" / "Yesterday · 2:41" / "Nov 24 · 6:08" — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"Today {}\", hearth_ui::time::clock(ts))"
- [x] The open one lit: the warm fill, the accent line on its left, its dot in the accent — hosts/hearth-ui/apps/listen/src/main.rs: "w::lit_row(&p, inner.into(), on, 10.0, 56.0)"

### The recording
- [x] Its name as the title (1.18 video, voice-over take 3) — hosts/hearth-ui/apps/listen/src/main.rs: "w::h1(&p, r.title.clone())"
- [x] "Recorded in Voice Studio · 4:12 · English · 2 voices · made into words in 21 s" — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"made into words in {} s\", r.took.round())"
- [x] Copy: the words to the clipboard — hosts/hearth-ui/apps/listen/src/main.rs: "iced::clipboard::write(whisper::to_txt(&r.segs))"
- [x] Export ▾ .txt .srt .vtt: the menu, then a save window (kdialog) or ~/Documents — hosts/hearth-ui/apps/listen/src/main.rs: "\"Export ▾ .txt .srt .vtt\""
- [x] Send to Voice Studio (in the accent): really sent to Voice Studio on port 8890 with its API key; it lands as a finished transcript with text, SRT, VTT and JSON — hosts/linux/desktop/shell/hearth-voice-studio: "if path == '/api/jobs/words':"
- [x] The player's ▶ (❚❚ while playing): plays from the playhead (pw-play) — hosts/hearth-ui/apps/listen/src/main.rs: "if a.playing() { \"❚❚\" } else { \"▶\" }"
- [x] The waveform in blue, drawn from the recording's own sound; pressed anywhere, it plays from there — hosts/hearth-ui/apps/listen/src/main.rs: "Msg::SeekFrac((at.x / b.width.max(1.0)).clamp(0.0, 1.0))"
- [x] The playhead in the accent with its glow — hosts/hearth-ui/apps/oss-kit/src/w.rs: "Pal::a(self.head_color, 0.18)"
- [x] "1:48 / 4:12" — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"{} / {}\", whisper::mmss(a.pos), whisper::mmss(r.secs))"
- [x] 0.75× 1× 1.5× (kept) — hosts/hearth-ui/apps/listen/src/main.rs: "sp(0.75, \"0.75×\"), sp(1.0, \"1×\"), sp(1.5, \"1.5×\")"
- [x] Each line's time (1:31 …), pressed to play from there — hosts/hearth-ui/apps/listen/src/main.rs: "on_press(Msg::Seek(s.t0))"
- [x] VOICE 1 in the accent, VOICE 2 in blue (whisper.cpp's tinydiarize turns) — hosts/hearth-ui/apps/listen/src/main.rs: "tb(format!(\"VOICE {}\", s.voice), 12.0, voice_colour(&p, s.voice))"
- [x] What was said, line by line, with lines between them — hosts/hearth-ui/apps/listen/src/main.rs: "push(ui::hline(Pal::mix(p.line, p.bg, 0.4)))"
- [x] The line being said now lit (warm, rounded) — hosts/hearth-ui/apps/listen/src/main.rs: "let bg = if now { Pal::mix(p.panel, p.accent, 0.13) } else { Color::TRANSPARENT };"
- [x] The words being said right now marked ("can be laid out the same way"), from whisper.cpp's own word times — hosts/hearth-ui/apps/listen/src/whisper.rs: "fn word_times(s: &Value) -> Vec<f64>"

### How it listens
- [x] How it listens (heading) — hosts/hearth-ui/apps/listen/src/main.rs: "w::h2(&p, \"How it listens\")"
- [x] Model "Large v3 turbo ▾": whisper.cpp's models, each got from whisper.cpp's page and checked by its SHA-1 — hosts/hearth-ui/apps/listen/src/whisper.rs: "(\"large-v3-turbo\", \"Large v3 turbo\", \"1.5 GB\", \"4af2b29d7ec73d781377bfd1758ca957a807e941\")"
- [x] Runs on "Nvidia PC · RTX 5070 ▾" (the graphics card nvidia-smi names, or the processor) — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"Nvidia PC · {} ▾\", a.gpu)"
- [x] Language "Find it by itself ▾" — hosts/hearth-ui/apps/listen/src/whisper.rs: "(\"auto\", \"Find it by itself\")"
- [x] Tell voices apart (on) — hosts/hearth-ui/apps/listen/src/main.rs: "w::kv(&p, \"Tell voices apart\""
- [x] Commas and capitals (on) — hosts/hearth-ui/apps/listen/src/main.rs: "w::kv(&p, \"Commas and capitals\""
- [x] Translate into English (off; whisper.cpp's -tr) — hosts/hearth-ui/apps/listen/src/whisper.rs: "a.push(\"-tr\".into());"
- [x] Words it should know "Hearth, Mantel, Ithil +12" (whisper.cpp's prompt; edited in place) — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"{} +{}\", words[..3].join(\", \"), words.len() - 3)"
- [x] LIVE CAPTIONS label — hosts/hearth-ui/apps/listen/src/main.rs: "w::cap(&p, \"Live captions\")"
- [x] Captions for anything playing (on: the captions bar starts) — hosts/hearth-ui/apps/listen/src/main.rs: "\"Captions for anything playing\""
- [x] Where: Top / Bottom / Window — hosts/hearth-ui/apps/listen/src/main.rs: "s3(\"window\", \"Window\", &wh, Msg::CapWhere)"
- [x] Size slider with "28 px" — hosts/hearth-ui/apps/listen/src/main.rs: "format!(\"{} px\", a.cap_size as i32)"
- [x] Listen to: Mic / What's playing / Both — hosts/hearth-ui/apps/listen/src/main.rs: "s3(\"playing\", \"What's playing\", &from, Msg::CapListen)"
- [x] Ties to Voice Studio card: the VS mark and "Words come back with their times, so Voice Studio can cut takes by what was said and make subtitles for both video sizes." — hosts/hearth-ui/apps/listen/src/main.rs: "so Voice Studio can cut takes by what was said and make subtitles for both video sizes."
- [x] "Nothing is sent anywhere. Models: 1.6 GB in ~/.local/share/hearth/listen" (the models' real size) — hosts/hearth-ui/apps/listen/src/main.rs: "Nothing is sent anywhere. Models: {} in ~/.local/share/hearth/listen"

### Live captions (the bar on the desktop)
- [x] Its own bar on top of everything, rounded, at the bottom (or top, or a window) — hosts/hearth-ui/apps/listen/src/captions.rs: "s.position = window::Position::SpecificWith(at_bottom);"
- [x] LIVE CAPTIONS tag — hosts/hearth-ui/apps/listen/src/captions.rs: "w::tag(&p, \"Live captions\", Tg::Acc)"
- [x] The words, the newest still muted ("…and every Hearth app can now be laid out exactly the way you want it, just drag"), heard by whisper.cpp from what's playing (parec, the output's monitor) — hosts/hearth-ui/apps/listen/src/captions.rs: "iced::widget::span(c.new.clone()).color(p.muted).size(size)"
- [x] "from: Firefox video" (the program playing, from PipeWire) — hosts/hearth-ui/apps/listen/src/captions.rs: "format!(\"from: {}\", c.from)"
- [x] Pause — hosts/hearth-ui/apps/listen/src/captions.rs: "if c.paused { \"Go on\" } else { \"Pause\" }"
- [x] ✕ (captions off) — hosts/hearth-ui/apps/listen/src/captions.rs: "let _ = pr.set(\"captions\", json!(false));"

### The credit and the blocks
- [x] "Hearth Listen · Built on whisper.cpp (MIT)" — hosts/hearth-ui/apps/listen/src/main.rs: "w::credit(&p, \"Hearth Listen\", \"Built on whisper.cpp (MIT)\")"
- [x] Every part a block (4 pages, 14 blocks: moved, sized, hidden, customized), Edit layout in the side, its own blocks window (13 buttons, 3 panes, 5 info) — hosts/hearth-ui/apps/listen/src/main.rs: "name: \"Listen\","
- [x] whisper.cpp installed the Hearth way when it isn't there (its pinned release built for this PC, in your own folder) — hosts/linux/desktop/shell/hearth-listen-install: "COMMIT=48f628a84833905ee4a0658ee6d4a5c915ce1997"
