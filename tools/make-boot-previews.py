#!/usr/bin/env python3
"""The previews Settings › Boot & startup plays (next update after 1.4): for every boot splash theme and every Steam Game
Mode startup animation, a sheet of small frames (JPEG, 400x225 each, 8 to a row, 10 a second) that the page plays
like a flip book, without running Plymouth and without needing a video player:

  os/system_files/usr/share/hearth/boot-previews/splash-<theme>.jpg   the splash as Plymouth draws it, from its own
                                                                       script (tools/plymouth_preview.py runs it):
                                                                       5 s from the moment it appears
  os/system_files/usr/share/hearth/boot-previews/movie-<id>.jpg       the startup animation (its 16:9 WebM)
  os/system_files/usr/share/hearth/boot-previews/index.json           frame size, frames per row, frames per second
                                                                       and each sheet's frame count and poster frame

Run it after changing a splash theme or tools/make-startup-movies.py. hearth-boot-look status hands the page these.

  python3 tools/make-boot-previews.py [--only NAME] [--video DIR]   (Pillow, numpy, ffmpeg)
      --video DIR   also writes each splash's preview as a 1280x720 MP4 (30 frames a second), for looking at
"""
import json
import os
import subprocess
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import plymouth_preview  # noqa: E402

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
THEMES = os.path.join(ROOT, 'os/system_files/usr/share/plymouth/themes')
MOVIES = os.path.join(ROOT, 'os/system_files/usr/share/hearth/startup-movies')
OUT = os.path.join(ROOT, 'os/system_files/usr/share/hearth/boot-previews')
FW, FH, COLS, FPS = 400, 225, 8, 10
SPLASH_SECS = 5.0
RENDER = (1280, 720)


def sheet(frames):
    rows = (len(frames) + COLS - 1) // COLS
    out = Image.new('RGB', (FW * COLS, FH * rows), (0x0b, 0x10, 0x16))
    for i, f in enumerate(frames):
        out.paste(f, ((i % COLS) * FW, (i // COLS) * FH))
    return out


def splash_frames(theme, fps=FPS, size=(FW, FH), secs=SPLASH_SECS):
    s = plymouth_preview.Splash(os.path.join(THEMES, theme), *RENDER)
    if s.errors:
        raise SystemExit(f'{theme}: {s.errors}')
    frames = []
    for _ in range(int(secs * fps)):
        s.advance(1.0 / fps)
        frames.append(s.draw().resize(size, Image.LANCZOS))
    if s.errors:
        raise SystemExit(f'{theme}: {s.errors}')
    return frames


def movie_frames(path):
    cmd = ['ffmpeg', '-v', 'error', '-i', path, '-vf', f'fps={FPS},scale={FW}:{FH}:flags=lanczos', '-f', 'rawvideo',
           '-pix_fmt', 'rgb24', '-']
    raw = subprocess.run(cmd, capture_output=True, check=True).stdout
    n = len(raw) // (FW * FH * 3)
    return [Image.frombytes('RGB', (FW, FH), raw[i * FW * FH * 3:(i + 1) * FW * FH * 3]) for i in range(n)]


def save_video(frames, path, fps):
    w, h = frames[0].size
    p = subprocess.Popen(['ffmpeg', '-v', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{w}x{h}', '-r', str(fps),
                          '-i', '-', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-crf', '20', path], stdin=subprocess.PIPE)
    for f in frames:
        p.stdin.write(f.tobytes())
    p.stdin.close()
    p.wait()


def main(args):
    only = args[args.index('--only') + 1] if '--only' in args else None
    video = args[args.index('--video') + 1] if '--video' in args else None
    os.makedirs(OUT, exist_ok=True)
    ip = os.path.join(OUT, 'index.json')
    try:
        with open(ip) as f:
            index = json.load(f)
    except (OSError, ValueError):
        index = {}
    index.update({'frameWidth': FW, 'frameHeight': FH, 'columns': COLS, 'fps': FPS})
    sheets = index.setdefault('sheets', {})
    themes = sorted(t for t in os.listdir(THEMES) if os.path.isfile(os.path.join(THEMES, t, t + '.script')))
    for t in themes:
        name = 'splash-' + t
        if only and only not in (name, t):
            continue
        frames = splash_frames(t)
        sheet(frames).save(os.path.join(OUT, name + '.jpg'), quality=80, optimize=True, progressive=False)
        sheets[name] = {'frames': len(frames), 'poster': min(len(frames) - 1, 30)}
        if video:
            os.makedirs(video, exist_ok=True)
            save_video(splash_frames(t, 30, RENDER, 8.0), os.path.join(video, f'splash-{t}.mp4'), 30)
        print(name, len(frames), 'frames', os.path.getsize(os.path.join(OUT, name + '.jpg')) // 1024, 'KB', flush=True)
    for f in sorted(os.listdir(MOVIES)):
        if not f.endswith('-16x9.webm'):
            continue
        mid = f[len('hearth-'):-len('-16x9.webm')]
        name = 'movie-' + mid
        if only and only not in (name, mid):
            continue
        frames = movie_frames(os.path.join(MOVIES, f))
        sheet(frames).save(os.path.join(OUT, name + '.jpg'), quality=80, optimize=True, progressive=False)
        sheets[name] = {'frames': len(frames), 'poster': min(len(frames) - 1, 36)}
        print(name, len(frames), 'frames', os.path.getsize(os.path.join(OUT, name + '.jpg')) // 1024, 'KB', flush=True)
    index['sheets'] = dict(sorted(sheets.items()))
    with open(ip + '.tmp', 'w') as f:
        json.dump(index, f, indent=1)
        f.write('\n')
    os.replace(ip + '.tmp', ip)


if __name__ == '__main__':
    main(sys.argv[1:])
