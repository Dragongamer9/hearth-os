#!/usr/bin/env python3
"""Hearth's own system sounds (1.14): short, soft tones made here from sine waves, nothing recorded or taken from
anyone, so they carry Hearth OS's own licence (GPL-3.0, the repo's LICENSE). Run it to make them again:

  python3 tools/make-hearth-sounds.py [OUT_DIR]     (default hosts/linux/desktop/sounds/hearth/stereo)

The names are the freedesktop sound naming spec's, so the folder is a sound theme (index.theme next to it) that
falls back to the freedesktop theme for anything it hasn't got. Sounds.qml plays them (Settings › Sounds).
"""
import math
import os
import struct
import sys
import wave

RATE = 44100


def tone(freqs, dur, vol=0.32, attack=0.008, decay=None, bend=0.0, harmonics=(1.0, 0.18, 0.05)):
    """One note (several frequencies together: a soft chord), with a quick attack and an exponential fade."""
    n = int(RATE * dur)
    decay = decay or dur / 3.2
    out = []
    for i in range(n):
        t = i / RATE
        env = min(1.0, t / attack) * math.exp(-t / decay)
        f_mul = 1 + bend * (t / dur)
        s = 0.0
        for f in freqs:
            for h, a in enumerate(harmonics, 1):
                s += a * math.sin(2 * math.pi * f * f_mul * h * t)
        out.append(s / (len(freqs) * sum(harmonics)) * env * vol)
    return out


def seq(*parts, gap=0.0):
    out = []
    for p in parts:
        out.extend(p)
        out.extend([0.0] * int(RATE * gap))
    return out


def mix(a, b, at):
    out = list(a) + [0.0] * max(0, int(RATE * at) + len(b) - len(a))
    o = int(RATE * at)
    for i, v in enumerate(b):
        out[o + i] += v
    return out


# Notes (Hz): the flame's warm major sixths, nothing shrill.
C5, E5, G5, A5, C6, E6, G6, A4, D5, F5, B4, G4, E4 = 523.25, 659.25, 783.99, 880.0, 1046.5, 1318.5, 1568.0, 440.0, 587.33, 698.46, 493.88, 392.0, 329.63

SOUNDS = {
    'message-new-instant': lambda: mix(tone([E6], 0.22, 0.26), tone([A5, E6], 0.42, 0.24), 0.09),
    'dialog-information': lambda: mix(tone([C6], 0.2, 0.24), tone([G5], 0.36, 0.22), 0.1),
    'dialog-warning': lambda: seq(tone([A5], 0.16, 0.3), tone([A5], 0.3, 0.3), gap=0.04),
    'dialog-error': lambda: mix(tone([E5], 0.22, 0.32), tone([B4], 0.42, 0.32), 0.14),
    'complete': lambda: mix(mix(tone([C5], 0.3, 0.24), tone([E5], 0.3, 0.24), 0.08), tone([G5, C6], 0.5, 0.24), 0.16),
    'device-added': lambda: mix(tone([G5], 0.16, 0.26, bend=0.08), tone([C6], 0.3, 0.26), 0.08),
    'device-removed': lambda: mix(tone([C6], 0.16, 0.26, bend=-0.08), tone([G5], 0.3, 0.26), 0.08),
    'audio-volume-change': lambda: tone([A5], 0.07, 0.22, attack=0.003, decay=0.025),
    'battery-low': lambda: seq(tone([E5], 0.18, 0.3), tone([C5], 0.34, 0.3), gap=0.05),
    'power-plug': lambda: tone([E5, A5], 0.32, 0.22, bend=0.12),
    'power-unplug': lambda: tone([A5, E5], 0.32, 0.22, bend=-0.1),
    'bell': lambda: tone([C6], 0.35, 0.22, harmonics=(1.0, 0.3, 0.12, 0.05)),
    'camera-shutter': lambda: seq(tone([G6], 0.03, 0.18, attack=0.001, decay=0.01), tone([E6], 0.05, 0.16, attack=0.001, decay=0.02), gap=0.03),
    'desktop-login': lambda: mix(mix(tone([G4], 0.5, 0.2), tone([C5], 0.5, 0.2), 0.12), tone([E5, G5], 0.8, 0.2), 0.24),
    'trash-empty': lambda: tone([E5], 0.25, 0.2, bend=-0.3, attack=0.002),
}


def write(path, samples):
    with wave.open(path, 'wb') as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(RATE)
        w.writeframes(b''.join(struct.pack('<h', max(-32767, min(32767, int(s * 32767)))) for s in samples))


def main(argv):
    out = argv[1] if len(argv) > 1 else os.path.join(os.path.dirname(__file__), '..', 'hosts', 'linux', 'desktop', 'sounds', 'hearth', 'stereo')
    os.makedirs(out, exist_ok=True)
    for name, make in sorted(SOUNDS.items()):
        s = make()
        s += [0.0] * int(RATE * 0.02)
        write(os.path.join(out, name + '.wav'), s)
        print(name, '%.2f s' % (len(s) / RATE))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
