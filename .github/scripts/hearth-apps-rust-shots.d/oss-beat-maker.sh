# Hearth Beat Maker (hosts/hearth-ui/apps/beat-maker; board docs/mockups/open-source-apps/18-beat-maker.png, checklist
# docs/mockups/checklists/open-source-18-beat-maker.md): its pictures at the board's size (1920 x 1080, back to
# 1280 x 800 after), in Hearth's own look, with the board's song (Ember Rising: 7 tracks, 48 bars at 128 BPM in D
# minor) as a Hearth Beat Maker file in a pretend home, a pretend LMMS data folder (its presets and samples, empty
# files, for the Library's counts) and 31 recordings. Then the song is rendered with --render and the WAV checked.
# Sourced by hearth-apps-rust-test.sh inside its headless screen.
BH=/tmp/oss-beat-home
rm -rf "$BH"; mkdir -p "$BH"
python3 - "$BH" <<'PY' || echo "::error title=oss-beat fixture::it couldn't be made"
import json, math, os, struct, sys, wave
home = sys.argv[1]
M = os.path.join(home, "Music", "Hearth Beat Maker")
os.makedirs(os.path.join(M, "recordings"), exist_ok=True)
def tone(path, secs, f):
    w = wave.open(path, "wb"); w.setnchannels(1); w.setsampwidth(2); w.setframerate(44100)
    w.writeframes(b"".join(struct.pack("<h", int(3000 * math.sin(2 * math.pi * f * i / 44100) * (0.5 + 0.5 * math.sin(i / 9000.0)))) for i in range(int(secs * 44100))))
    w.close()
take = os.path.join(M, "recordings", "take-3.wav")
tone(take, 22.0, 180)
for k in range(30):
    open(os.path.join(M, "recordings", "take-%02d-old.wav" % (k + 4)), "wb").write(b"")
L = os.path.join(home, "lmms")
os.makedirs(os.path.join(L, "presets", "TripleOscillator"), exist_ok=True)
for k in range(55):
    open(os.path.join(L, "presets", "TripleOscillator", "Preset%02d.xpf" % k), "w").close()
n = 0
for kit in range(19):
    d = os.path.join(L, "samples", "drums", "kit%02d" % kit); os.makedirs(d, exist_ok=True)
    for j in range(40):
        open(os.path.join(d, "hit%02d.ogg" % j), "w").close(); n += 1
d = os.path.join(L, "samples", "instruments"); os.makedirs(d, exist_ok=True)
for j in range(1840 - n):
    open(os.path.join(d, "note%04d.ogg" % j), "w").close()
def inst(kind, name, sub, mark, a, b, knobs):
    return {"kind": kind, "name": name, "sub": sub, "mark": mark, "from": a, "to": b, "knobs": knobs, "delay": False, "glue": False}
def beat(name, swing, rows):
    return {"name": name, "beat": {"steps": 16, "swing": swing, "rows": [{"name": r, "drum": r, "hits": [110 if i in on and i % 4 == 0 else (92 if i in on else 0) for i in range(16)]} for r, on in rows]}}
def notes(name, ns):
    return {"name": name, "notes": ns}
N = [[10,0,4],[7,0,4],[3,0,4],[10,4,4],[5,4,4],[1,4,4],[12,8,4],[8,8,4],[5,8,4],[10,12,6],[7,12,6],[3,12,6],[3,18,2],[1,20,4],[5,20,4],[8,20,4]]
chords_b = [[72 - r, s * 0.5, l * 0.5, int((20 + (i * 37) % 30) / 46 * 127)] for i, (r, s, l) in enumerate(N)]
pats = [
    beat("Drop A", 12, [("Kick", [0, 4, 8, 12]), ("Snare", [4, 12]), ("Clap", [12, 15]), ("Closed hat", [0, 2, 4, 6, 8, 10, 12, 14]), ("Open hat", [7, 15]), ("Tom", [13, 14]), ("Big hit", [0]), ("Rim", [3, 11])]),
    beat("Intro hits", 0, [("Big hit", [0]), ("Kick", [0, 8]), ("Tom", [12, 14])]),
    notes("Bass line", [[38, 0, 1.5, 100], [38, 2, 0.5, 90], [41, 3, 1, 95], [36, 4, 1.5, 100], [36, 6, 0.5, 90], [43, 7, 1, 95]]),
    notes("Chords A", [[62, 0, 4, 90], [65, 0, 4, 90], [69, 0, 4, 90], [58, 4, 4, 90], [62, 4, 4, 90], [65, 4, 4, 90]]),
    notes("Chords B", chords_b),
    notes("Hook", [[74, 0, 1, 100], [77, 1, 0.5, 95], [76, 1.5, 1.5, 100], [74, 3, 1, 90], [72, 4, 2, 100], [69, 6, 2, 95]]),
    notes("Swell", [[50, 0, 8, 80], [57, 0, 8, 80], [62, 0, 8, 80]]),
    notes("Swell high", [[62, 0, 8, 85], [69, 0, 8, 85], [74, 0, 8, 85]]),
    notes("Rise", [[72, 0, 8, 110]]),
    notes("Hit", [[36, 0, 4, 120]]),
    {"name": "take 3 · from Hearth Listen", "audio": take},
]
P = {p["name"]: i for i, p in enumerate(pats)}
def clips(spec):
    return [{"start": a * 2, "len": (z - a) * 2, "pattern": P[n]} for a, z, n in spec]
song = {"format": 1, "app": "Hearth Beat Maker", "name": "Ember Rising", "note": "new trailer song", "bpm": 128, "sig": [4, 4], "key": 2, "minor": True, "bars": 48,
        "tracks": [
            {"name": "Drum kit", "color": "#ff9a8a", "mute": False, "solo": False, "inst": inst("drums", "Drum kit · Cinematic", "kick, snare, hats, toms", "DR", "#ff9a8a", "#d8413a", [0.7, 0.5, 0, 0.25]), "clips": clips([(2, 6, "Intro hits"), (8, 16, "Drop A"), (18, 22, "Drop A")])},
            {"name": "Deep Bass", "color": "#7fd6ff", "mute": False, "solo": False, "inst": inst("bass", "Deep Bass", "synth · sub", "BS", "#7fd6ff", "#2c7fd8", [0.35, 0.4, 0.02, 0.05]), "clips": clips([(4, 12, "Bass line"), (14, 22, "Bass line")])},
            {"name": "Ember Pad", "color": "#ffb36b", "mute": False, "solo": False, "inst": inst("synth", "Ember Pad", "synth · warm", "EP", "#ffb36b", "#ff6a1f", [0.52, 0.30, 0.62, 0.45]), "clips": clips([(0, 8, "Chords A"), (8, 16, "Chords B"), (16, 24, "Chords B")])},
            {"name": "Glass Lead", "color": "#b9a2ff", "mute": False, "solo": False, "inst": inst("synth", "Glass Lead", "synth · bright", "LD", "#b9a2ff", "#6a4fe0", [0.78, 0.15, 0.05, 0.35]), "clips": clips([(8, 12, "Hook"), (16, 20, "Hook")])},
            {"name": "Strings", "color": "#7ff0d8", "mute": False, "solo": False, "inst": inst("strings", "Strings", "sampled · 4 layers", "ST", "#7ff0d8", "#1f9f8f", [0.6, 0.35, 0.7, 0.6]), "clips": clips([(0, 10, "Swell"), (14, 24, "Swell high")])},
            {"name": "Rises and hits", "color": "#ffe28a", "mute": False, "solo": False, "inst": inst("fx", "Rises and hits", "trailer effects", "FX", "#ffe28a", "#f0a020", [0.8, 0.5, 0.3, 0.55]), "clips": clips([(3, 4, "Rise"), (7, 8, "Hit"), (13, 14, "Rise"), (15, 16, "Hit")])},
            {"name": "Voice-over", "color": "#c9c1ba", "mute": False, "solo": False, "inst": inst("audio", "Voice-over", "recordings", "VO", "#c9c1ba", "#6d645d", [0.5, 0, 0, 0.2]), "clips": clips([(1, 7, "take 3 · from Hearth Listen")])},
        ], "patterns": pats}
json.dump(song, open(os.path.join(M, "ember-rising.hbm.json"), "w"), indent=1)
PY
swaymsg output HEADLESS-1 resolution 1920x1080 >/dev/null 2>&1
look hearth dark
mkdir -p "$BH/.local/share/nimbus" && cp "$HOME/.local/share/nimbus/desktop.json" "$BH/.local/share/nimbus/desktop.json"
BMA="$BIN/hearth-beat-maker-app"
SONG="$BH/Music/Hearth Beat Maker/ember-rising.hbm.json"
beat() { # NAME ARGS...
  n=$1; shift
  SHOT_WAIT=3.5 run "oss-beat-$n" hearth-beat-maker "${REST:-0}" env HOME="$BH" HEARTH_LMMS_DATA="$BH/lmms" XDG_CONFIG_HOME="$BH/.config" "$@"
}
BOARD=(--open "$SONG" --at 42.6 --track "Ember Pad" --notes "Chords B" --beat "Drop A" --select-notes 9,10,11)
REST=20 beat 1-board-export "$BMA" "${BOARD[@]}" --show export
beat 2-song-and-notes "$BMA" "${BOARD[@]}"
beat 3-library "$BMA" "${BOARD[@]}" --show library
beat 4-edit-layout env HEARTH_ARRANGE_OPEN=beat-maker:window HEARTH_STUDIO_STAGE=beat-maker:none "$BMA" "${BOARD[@]}"
beat 5-customize env HEARTH_ARRANGE_OPEN=beat-maker:notes HEARTH_STUDIO_STAGE=beat-maker:customize "$BMA" "${BOARD[@]}"
beat 6-a-new-song env HOME="$BH/new" "$BMA"
# the song rendered by Hearth's synth with no window: a WAV as long as the song, not silent
if env HOME="$BH" "$BMA" --render "$SONG" /tmp/oss-beat-render.wav > /tmp/oss-beat-render.log 2>&1; then
  python3 - /tmp/oss-beat-render.wav <<'PY'
import sys, wave, struct
w = wave.open(sys.argv[1]); n = w.getnframes(); secs = n / w.getframerate()
data = w.readframes(min(n, 44100 * 20)); peak = max(abs(x) for x in struct.unpack("<%dh" % (len(data) // 2), data))
ok = secs > 88 and peak > 2000
print(("::notice" if ok else "::error") + " title=oss-beat render::Ember Rising rendered: %.1f s, %d channels, loudest %d of 32767" % (secs, w.getnchannels(), peak))
PY
else
  echo "::error title=oss-beat render::--render failed: $(tail -c 500 /tmp/oss-beat-render.log | tr '\n' '|')"
fi
grep -h "panicked" /tmp/oss-beat-*.log 2>/dev/null | head -2 | while read -r l; do echo "::error title=oss-beat::$l"; done
swaymsg output HEADLESS-1 resolution 1280x800 >/dev/null 2>&1
look steam dark
