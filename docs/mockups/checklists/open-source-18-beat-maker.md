# Hearth Beat Maker (LMMS, GPL-2) (approved Nov 26, 2026; source: hearth-workspace hearth/mockups/2026-11-26-store-apps/18-beat-maker.html)

Mockups: open-source-apps/18-beat-maker.png
Release: next

The owner, Nov 26: "The 18 app mockups look amazing remember to make them out of rust and add the blocks modular system
to them". A pure-Rust Hearth app (hosts/hearth-ui/apps), made of blocks (studio: Edit layout, Customize, its own blocks
window), built on the open-source project the board credits.

Board: Hearth Beat Maker (LMMS, GPL-2): a song from patterns, the piano roll and instruments, for Hearth's trailer music.

## The window
- [x] The desktop's top bar names it "Hearth Beat Maker" (its menu entry's name) (CI pictures oss-beat-maker-1…6; needs testing on a device) — os/system_files/usr/share/applications/hearth-beat-maker.desktop: "Name=Hearth Beat Maker"
- [x] Its title bar: the BM mark (Hearth's own pink square), "Hearth Beat Maker", "· Ember Rising (new trailer song)", – ▢ ✕ (and Edit layout) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"· {} ({})\""
- [x] ⏮ ▶ ■ ●: back to the start, play (Hearth's synth through PipeWire), stop, record your mic onto the Voice-over track (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "chip(&p, \"⏮\", Ck::Normal, None, Some(Msg::ToStart))"
- [x] "0:42.6 · bar 23", "128 BPM" (press, right-press or scroll to change), "4 / 4", "D minor" (the key: pick it) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"{}:{:04.1} · bar {}\""
- [x] "CPU 9%" (its own, while playing) and Export… (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"CPU {}%\""

## Instruments and sounds
- [x] "Instruments and sounds" + (add one) and ⌕ Find a sound (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"⌕ Find a sound\""
- [x] IN THIS SONG: Ember Pad (synth · warm), Glass Lead (synth · bright), Deep Bass (synth · sub), Drum kit · Cinematic (kick, snare, hats, toms), Strings (sampled · 4 layers), Rises and hits (trailer effects), each with its mark; the picked one lit (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/song.rs: "\"Drum kit · Cinematic\", \"kick, snare, hats, toms\""
- [x] LIBRARY: Synths · 64, Drum kits · 22, Samples · 1,840, My recordings · 31, Effects · reverb, delay, glue…: Hearth's own and LMMS's, counted, each opening its list (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "\"reverb, delay, glue…\""
- [x] EMBER PAD: Cutoff, Warmth, Attack, Reverb knobs (drag up and down, or scroll), heard as you play (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "[\"Cutoff\", \"Warmth\", \"Attack\", \"Reverb\"]"

## Song
- [x] "Song · 48 bars · 1:30" and Draw · Pick · Cut (draw a clip, pick and drag one, cut one in two) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "(\"Draw\", a.mode == Mode::Draw, Msg::Mode(Mode::Draw))"
- [x] TRACKS and the bars (every other one numbered; a press there moves the playhead) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "ctext(\"TRACKS\""
- [x] Each track: its colour, name, M S (mute, solo), its clips on the bars with their patterns' names (Intro hits, Drop A, Bass line, Chords A, Chords B, Hook, Swell, Swell high, Rise, Hit, take 3 · from Hearth Listen) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "f.fill_text(ctext(\"M\", Point::new(x0 - 32.0, y)"
- [x] The playhead, lit (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "Pal::a(p.accent, 0.18)"

## Beat
- [x] Beat · "Drop A" · 16 steps and Swing 12% (press to change it) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"Swing {}%\""
- [x] Kick, Snare, Clap, Closed hat, Open hat, Tom, Big hit, Rim: 16 steps each, hits lit, the off-beats a little dimmer (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/song.rs: "(Drum::BigHit, \"Big hit\")"
- [x] "Click to add a hit · drag up and down for how hard" and + Row (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "Click to add a hit · drag up and down for how hard"

## Notes
- [x] Notes · Ember Pad · "Chords B", 1/4 · 1/8 · 1/16 (the grid), Fit to D minor ✓ (notes kept in the key), Make chords… (triads or sevenths in the key) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"Make chords…\""
- [x] The keys (C at the top and C4 at the bottom, black keys dark), the grid, the notes (add, move, size, delete), the picked ones lighter, the playhead (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "let label = if pc == 0 { format!(\"C{}\", key / 12 - 1)"
- [x] HOW HARD: each note's velocity, dragged up and down (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "ctext(\"HOW HARD\""

## Export for Hearth's videos
- [x] "Export for Hearth's videos" ✕ and Name (Ember Rising) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "\"Export for Hearth's videos\""
- [x] Files: .wav, .mp3 (ffmpeg), stems (each track on its own); an LMMS project (.mmp) with them (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "std::fs::write(dir.join(format!(\"{sl}.mmp\")), song::to_mmp(sg))"
- [x] Add to the hearthos.fun music list (your hearth-os folder: site/music and the site's list; not committed for you) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "\"var TRACKS = [\\n\""
- [x] Mark the beats for video cuts (every beat and bar, and the big hits, as JSON) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "fn beat_marks(sg: &Song) -> Value"
- [x] "Saved to hearth-workspace": the song's source in hearth-workspace's hearth/assets with its README and INDEX line (its rules: renders aren't kept there) (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "Saved to hearth-workspace: hearth/assets/{item} (push it there yourself)"
- [x] Cancel and Export (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "chip(&p, \"Export\", Ck::Acc, None, Some(Msg::ExportGo))"
- [x] "Hearth Beat Maker · Built on LMMS (GPL-2)" (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "Built on LMMS (GPL-2)"

## Behind the board
- [x] Built on LMMS: the song written as an LMMS project (TripleOscillator, Kicker, AudioFileProcessor tracks), opened in LMMS (LMMS · open this song in LMMS), LMMS installed from Flathub when it isn't here (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/song.rs: "pub fn to_mmp(sg: &Song) -> String"
- [x] Hearth's own synth renders what you hear and what's exported (synths, bass, strings, drums, rises and hits, recordings); `--render` does it with no window (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/synth.rs: "pub fn mix(sg: &Song, only: Option<usize>, from: f32) -> Vec<i16>"
- [x] The song saved as you change it (settings_file's safe write, a .bak), and which song was open (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/main.rs: "settings_file::write_bytes(&self.file, text.as_bytes(), song::valid)"
- [x] Every part a block (the title bar, instruments and sounds and their parts, the song, the beat, the notes and their parts, the credit), moved, sized, hidden and customized in Edit layout (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "pub const BLOCKS: &[PageDef]"
- [x] Beat Maker's own blocks window: its buttons (play, stop, back to the start, record, export, LMMS, draw, pick, cut, chords, fit), panes (the beat, the knobs, the library) and info (CI pictures oss-beat-maker-1…6; needs testing on a device) — hosts/hearth-ui/apps/beat-maker/src/view.rs: "name: \"Beat Maker\""
- [x] In the Hearth Store's list (built in) and the quick-update list; its menu entry and icon (CI pictures oss-beat-maker-1…6; needs testing on a device) — os/system_files/usr/share/hearth/store/catalog.json: "\"partId\": \"hearth-beat-maker-app\""
