#!/usr/bin/env python3
"""Screenshots as GitHub notices, so they can be seen (and rebuilt) from a run's annotations without downloading it.

  shot-notices.py emit DIR PART STEP    print the notices for job PART, step STEP (0-4)

Every DIR/*.png (sorted by name) becomes a JPEG at its own size (1920 × 1080 for the desktop), as good as fits in
MAX_CHUNKS pieces; the base64 is cut in 3800-character pieces, each one notice titled "SHOT <name> <n>" (n from 1),
the same as desktop-test.yml and photos.yml. GitHub keeps 10 notices a step and 50 a job, so the pieces are laid out
50 to a job, 10 to a step: run it in STEPS steps of PARTS jobs (widgets-test.yml's "shots" matrix). The last notice of
each shot is followed, in the same order, by the next shot's. Needs Pillow.
"""
import base64
import glob
import io
import os
import sys

PIECE = 3800
PER_STEP = 10
PER_JOB = 50
MAX_CHUNKS = 40


def encode(path):
    from PIL import Image
    im = Image.open(path).convert('RGB')
    for scale, q in ((1, 60), (1, 50), (1, 40), (1, 32), (0.75, 40), (0.6, 40), (0.5, 35)):
        x = im if scale == 1 else im.resize((round(im.width * scale), round(im.height * scale)))
        b = io.BytesIO()
        x.save(b, 'JPEG', quality=q, optimize=True)
        s = base64.b64encode(b.getvalue()).decode()
        if len(s) <= MAX_CHUNKS * PIECE:
            return s
    return s


def plan(d):
    items = []
    for p in sorted(glob.glob(os.path.join(d, '*.png'))):
        name = os.path.splitext(os.path.basename(p))[0]
        s = encode(p)
        for i in range(0, len(s), PIECE):
            items.append((name, i // PIECE + 1, s[i:i + PIECE]))
    return items


def main(argv):
    if len(argv) != 5 or argv[1] != 'emit':
        print(__doc__, file=sys.stderr)
        return 2
    d, part, step = argv[2], int(argv[3]), int(argv[4])
    items = plan(d)
    start = part * PER_JOB + step * PER_STEP
    for name, n, piece in items[start:start + PER_STEP]:
        print('::notice title=SHOT %s %d::%s' % (name, n, piece))
    if part == 0 and step == 0:
        names = {}
        for name, n, _ in items:
            names[name] = n
        # (a warning, not a notice: it doesn't take a notice's place)
        print('::warning title=SHOTS::%d pictures in %d pieces over %d jobs: %s' % (
            len(names), len(items), -(-len(items) // PER_JOB), ', '.join('%s (%d)' % kv for kv in names.items())))
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
