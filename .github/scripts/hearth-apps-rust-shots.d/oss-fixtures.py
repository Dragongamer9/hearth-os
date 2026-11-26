#!/usr/bin/python3
"""Made-up data for the open-source apps' pictures (docs/mockups/open-source-apps), the boards' own words and numbers,
so each picture can be laid beside its board. Nothing here is a real account or a real service.

  oss-fixtures.py listen DIR     Hearth Listen's folder (recordings with their words, models, a pretend whisper-cli
                                 and nvidia-smi in DIR/bin for PATH)
"""
import json
import os
import sys
import time


def rng(seed):
    """kit.js's little generator, so the bars are the board's bars."""
    s = seed * 9301 + 49297

    def r():
        nonlocal s
        s = (s * 9301 + 49297) % 233280
        return s / 233280
    return r


def bars(seed, n, lo, hi):
    r = rng(seed)
    return [round((lo + (hi - lo) * r()) / 100, 3) for _ in range(n)]


def today_at(h, m):
    t = time.localtime()
    return int(time.mktime((t.tm_year, t.tm_mon, t.tm_mday, h, m, 0, 0, 0, -1)))


def write(p, v):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(v, f, ensure_ascii=False)


def sparse(p, size):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.truncate(size)


def fake(p, body):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write('#!/bin/sh\n' + body + '\n')
    os.chmod(p, 0o755)


def listen(d):
    T = [("1:31", 1, "This is Hearth OS one point eighteen. Everything you see here is yours to change."),
         ("1:38", 1, "Start with the desktop. Panels go on any edge, or float anywhere you like."),
         ("1:45", 2, "And it's not just the desktop."),
         ("1:48", 1, "Every Hearth app can be laid out the same way: Files, Settings, Task Manager, Capture."),
         ("1:56", 1, "Right-click any part and pick Customize."),
         ("2:02", 2, "Then press Keep this, and it stays that way. Always."),
         ("2:09", 1, "Your settings never change by themselves."),
         ("2:14", 2, "Next: the Hearth Store, with every part updating on its own."),
         ("2:21", 1, "No system update, no restart. Open the Store and press Update."),
         ("2:29", 1, "And when there is a new Hearth OS, the one before stays startable."),
         ("2:36", 2, "That's Hearth OS one point eighteen.")]
    secs = lambda s: int(s.split(':')[0]) * 60 + int(s.split(':')[1])
    segs = []
    for i, (at, v, text) in enumerate(T):
        a = float(secs(at))
        b = float(secs(T[i + 1][0])) if i + 1 < len(T) else a + 5
        seg = {'start': a, 'end': b, 'text': text, 'speaker': 'VOICE %d' % v}
        if at == '1:48':
            # whisper.cpp's word times (-ojf) for the line being said: "can be laid out the same way" at 1:48.9
            seg['words'] = [108.0, 108.2, 108.45, 108.6, 108.8, 109.0, 109.3, 109.6, 109.9, 110.3, 110.8, 111.4, 112.0, 112.3, 112.9]
        segs.append(seg)
    now = time.time()
    R = [("take3", "1.18 video, voice-over take 3", today_at(1, 12), 252, "Voice Studio", segs, 21),
         ("take2", "1.18 video, voice-over take 2", today_at(1, 2), 260, "Voice Studio", None, 22),
         ("noclip", "Noclip Protocol, guard lines", int(now - 86400), 161, "Hearth Listen", None, 9),
         ("mech", "Mech game ideas (truck)", int(now - 2 * 86400), 368, "Hearth Listen", None, 30),
         ("dad", "Call with Dad about Trainz", int(now - 3 * 86400), 750, "Hearth Listen", None, 55),
         ("ithil", "Ithil voice test", int(now - 4 * 86400), 48, "Voice Studio", None, 4),
         ("gravesleep", "Gravesleep intro", int(now - 5 * 86400), 115, "Voice Studio", None, 10),
         ("store", "Store video, scratch", int(now - 6 * 86400), 182, "Voice Studio", None, 15)]
    for n, (rid, title, when, dur, src, words, took) in enumerate(R):
        rd = os.path.join(d, 'recordings', rid)
        write(os.path.join(rd, 'meta.json'), {'title': title, 'created': when, 'source': src, 'secs': dur, 'peaks': bars(11 + n, 160, 10, 95)})
        if words is None:
            words = [{'start': 0.0, 'end': 4.0, 'text': title + '.', 'speaker': ''}]
        write(os.path.join(rd, 'words.json'), {'language': 'en', 'duration': float(dur), 'took': took, 'model': 'large-v3-turbo', 'engine': 'whisper.cpp v1.8.7', 'segments': words})
    # whisper.cpp's models (sparse files of their real sizes: nothing is downloaded here)
    sparse(os.path.join(d, 'models', 'ggml-large-v3-turbo.bin'), 1624555275)
    sparse(os.path.join(d, 'models', 'ggml-small.en-tdrz.bin'), 487601967)
    fake(os.path.join(d, 'bin', 'whisper-cli'), 'echo "usage: whisper-cli [options] file0 (pretend, for the pictures)"')
    fake(os.path.join(d, 'fakes', 'nvidia-smi'), 'echo "NVIDIA GeForce RTX 5070"')


if __name__ == '__main__':
    {'listen': listen}[sys.argv[1]](sys.argv[2])
