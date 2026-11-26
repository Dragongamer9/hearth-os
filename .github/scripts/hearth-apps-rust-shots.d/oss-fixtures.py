#!/usr/bin/python3
"""Made-up data for the open-source apps' pictures (docs/mockups/open-source-apps), the boards' own words and numbers,
so each picture can be laid beside its board. Nothing here is a real account or a real service.

  oss-fixtures.py listen DIR     Hearth Listen's folder (recordings with their words, models, a pretend whisper-cli
                                 and nvidia-smi in DIR/bin for PATH)
  oss-fixtures.py imagine DIR    Hearth Imagine's data, choices and ComfyUI folder (sparse model files)
  oss-fixtures.py comfy PORT [idle]   a pretend ComfyUI on 127.0.0.1:PORT: its API (/system_stats, /object_info,
                                 /queue, /history, /view, /prompt) and /ws telling that picture 3 is on step 18 of 28
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


CABIN = 'A cozy cabin in a snowy pine forest at night, warm orange light in the windows, a hearth glow, wide shot, soft film look'
CABIN_TITLE = 'A cozy cabin in a snowy pine forest at night, w…'


def imagine(d):
    data = os.path.join(d, 'data', 'hearth', 'imagine')
    draft = {'title': CABIN_TITLE, 'prompt': CABIN, 'leaveOut': 'people, text, blurry', 'look': 'Film', 'size': '16:9', 'count': 4, 'steps': 28}
    write(os.path.join(data, 'draft.json'), draft)
    recent = [draft] + [{'title': t, 'prompt': p, 'leaveOut': 'text', 'look': l, 'size': sz, 'count': 4, 'steps': 28} for t, p, l, sz in (
        ('Noclip Protocol poster, yellow halls', 'Noclip Protocol poster, yellow halls, endless office rooms, buzzing lights', 'Game art', '16:9'),
        ('Hearth wallpaper, ember sky, 9:16', 'Hearth wallpaper, ember sky, glowing coals, dark warm gradient', 'Wallpaper', '9:16'),
        ('Quad mech in fog, concept art', 'Quad mech in fog, concept art, four slow legs, dawn light', 'Game art', '16:9'))]
    write(os.path.join(data, 'recent.json'), recent)
    write(os.path.join(data, 'speed.json'), {'secsPerStep': 0.125})
    write(os.path.join(d, 'config', 'nimbus', 'imagine.json'), {'view': 'both', 'look': 'Film', 'size': '16:9', 'count': 4, 'steps': 28, 'seed': 482113, 'model': 'hearth-xl-1.0.safetensors'})
    c = os.path.join(d, 'ComfyUI', 'models')
    sparse(os.path.join(c, 'checkpoints', 'hearth-xl-1.0.safetensors'), 6_900_000_000)
    sparse(os.path.join(c, 'checkpoints', 'Fast 1-step.safetensors'), 4_100_000_000)
    sparse(os.path.join(c, 'upscale_models', 'Upscaler 4×.pth'), 64_000_000)


def png(kind, w=448, h=256):
    from PIL import Image, ImageDraw, ImageFilter
    import io
    im = Image.new('RGB', (w, h))
    dr = ImageDraw.Draw(im)
    def grad(top, bottom, y0=0, y1=h):
        for y in range(y0, y1):
            t = (y - y0) / max(1, y1 - y0 - 1)
            dr.line([(0, y), (w, y)], fill=tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3)))
    if kind.startswith('cabin'):
        grad((13, 26, 42), (42, 58, 74), 0, int(h * .6))
        grad((232, 238, 242), (154, 168, 178), int(h * .6), h)
        cx = int(w * (.4 if kind == 'cabin1' else .6))
        glow = Image.new('RGB', (w, h))
        ImageDraw.Draw(glow).ellipse([cx - 26, int(h * .66) - 26, cx + 26, int(h * .66) + 26], fill=(255, 150, 70))
        im = Image.composite(glow, im, glow.convert('L').point(lambda v: 255 if v else 0)).filter(ImageFilter.GaussianBlur(2))
    else:
        cols = {'e1': ((200, 180, 74), (106, 90, 32)), 'e2': ((44, 127, 216), (13, 20, 36)), 'e3': ((255, 138, 61), (58, 29, 14)),
                'e4': ((59, 165, 92), (13, 42, 20)), 'e5': ((176, 79, 200), (26, 13, 42)), 'e6': ((109, 100, 93), (26, 22, 20))}[kind]
        grad(*cols)
    b = io.BytesIO()
    im.save(b, 'PNG')
    return b.getvalue()


def comfy(port, idle=False):
    import base64, hashlib, threading
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    from urllib.parse import urlsplit, parse_qs
    def item(num, pid, title):
        return [num, pid, {}, {'hearth_title': title, 'client_id': 'x'}, ['9']]
    queue = {'queue_running': [], 'queue_pending': []} if idle else {
        'queue_running': [item(103, 'p3', CABIN_TITLE)],
        'queue_pending': [item(104, 'p4', CABIN_TITLE), item(105, 'q1', 'Same, in 9:16 for the phone'), item(106, 'q2', 'Noclip Protocol poster, yellow halls')]}
    def done(num, pid, title, img):
        return {'prompt': item(num, pid, title), 'outputs': {'9': {'images': [{'filename': img + '.png', 'subfolder': 'Imagine', 'type': 'output'}]}}, 'status': {'completed': True}}
    hist = {} if idle else {'p1': done(101, 'p1', CABIN_TITLE, 'cabin1'), 'p2': done(102, 'p2', CABIN_TITLE, 'cabin2')}
    for i in range(6):
        hist['e%d' % i] = done(90 + i, 'e%d' % i, 'earlier %d' % i, 'e%d' % (6 - i))
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass
        def js(self, v, code=200):
            b = json.dumps(v).encode()
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(b)))
            self.end_headers()
            self.wfile.write(b)
        def do_POST(self):
            n = int(self.headers.get('Content-Length') or 0)
            self.rfile.read(n)
            self.js({'prompt_id': 'new%d' % int(time.time() * 1000), 'number': 200, 'node_errors': {}})
        def do_GET(self):
            u = urlsplit(self.path)
            q = parse_qs(u.query)
            if u.path == '/system_stats':
                return self.js({'system': {'os': 'posix', 'comfyui_version': '0.43.2'}, 'devices': [{'name': 'cuda:0 NVIDIA GeForce RTX 5070 : cudaMallocAsync', 'type': 'cuda', 'vram_total': 12884901888}]})
            if u.path == '/object_info/CheckpointLoaderSimple':
                return self.js({'CheckpointLoaderSimple': {'input': {'required': {'ckpt_name': [['hearth-xl-1.0.safetensors', 'Fast 1-step.safetensors']]}}}})
            if u.path == '/object_info/UpscaleModelLoader':
                return self.js({'UpscaleModelLoader': {'input': {'required': {'model_name': [['Upscaler 4×.pth']]}}}})
            if u.path == '/queue':
                return self.js(queue)
            if u.path == '/history':
                return self.js(hist)
            if u.path.startswith('/history/'):
                k = u.path.rsplit('/', 1)[1]
                return self.js({k: hist[k]} if k in hist else {})
            if u.path == '/view':
                b = png(q.get('filename', ['e1.png'])[0].rsplit('.', 1)[0])
                self.send_response(200)
                self.send_header('Content-Type', 'image/png')
                self.send_header('Content-Length', str(len(b)))
                self.end_headers()
                return self.wfile.write(b)
            if u.path == '/ws':
                k = self.headers.get('Sec-WebSocket-Key', '')
                acc = base64.b64encode(hashlib.sha1((k + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
                self.wfile.write(('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: %s\r\n\r\n' % acc).encode())
                if not idle:
                    m = json.dumps({'type': 'progress', 'data': {'value': 18, 'max': 28, 'prompt_id': 'p3', 'node': '3'}}).encode()
                    self.wfile.write(b'\x81' + bytes([len(m)]) + m)
                    self.wfile.flush()
                time.sleep(600)
                return
            self.js({'error': 'not here'}, 404)
    ThreadingHTTPServer(('127.0.0.1', port), H).serve_forever()


if __name__ == '__main__':
    if sys.argv[1] == 'comfy':
        comfy(int(sys.argv[2]), len(sys.argv) > 3 and sys.argv[3] == 'idle')
    else:
        {'listen': listen, 'imagine': imagine}[sys.argv[1]](sys.argv[2])
