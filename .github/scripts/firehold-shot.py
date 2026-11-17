#!/usr/bin/python3
"""Posts one picture as SHOT notices (firehold.yml; the owner's sessions rebuild it from them): shots/NAME.png as a
JPEG in base64 pieces of 3800, at most 8, after a "SHOT-PARTS" line. One step per picture (10 notices a step).
  firehold-shot.py NAME"""
import base64
import hashlib
import io
import os
import sys

from PIL import Image

name = sys.argv[1]
p = 'shots/%s.png' % name
if not os.path.exists(p):
    print('::error title=pictures::no %s' % name)
    raise SystemExit(1)
im = Image.open(p).convert('RGB')
for w, q in ((960, 45), (900, 38), (800, 34), (720, 30), (640, 26), (560, 22)):
    b = io.BytesIO()
    im.resize((w, round(im.height * w / im.width))).save(b, 'JPEG', quality=q)
    data = base64.b64encode(b.getvalue()).decode()
    if len(data) <= 8 * 3800:
        break
parts = [data[i:i + 3800] for i in range(0, len(data), 3800)]
print('::notice title=SHOT-PARTS %s %d::%s' % (name, len(parts), hashlib.sha256(b.getvalue()).hexdigest()[:16]))
for i, part in enumerate(parts, 1):
    print('::notice title=SHOT %s %d::%s' % (name, i, part))
