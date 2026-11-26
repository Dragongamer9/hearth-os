#!/usr/bin/env python3
"""A stand-in for Home Assistant's /api/camera_proxy_stream/<camera> (MJPEG, multipart/x-mixed-replace), for Hearth
Home's moving camera tiles in CI (mjpeg.rs): each camera's picture with a bar sweeping across it and the time to the
tenth of a second, 15 pictures a second, so two pictures of the window taken a moment apart differ.

  home-mjpeg.py PORT DIR    (DIR: the fixture's camera.<id>.png stills, drawn on)
"""
import io
import os
import socketserver
import sys
import time
from http.server import BaseHTTPRequestHandler

from PIL import Image, ImageDraw

PORT, DIR = int(sys.argv[1]), sys.argv[2]


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def do_GET(self):
        cam = self.path.rsplit('/', 1)[-1]
        if not self.path.startswith('/api/camera_proxy_stream/'):
            self.send_error(404)
            return
        try:
            base = Image.open(os.path.join(DIR, cam + '.png')).convert('RGB').resize((640, 360))
        except OSError:
            base = Image.new('RGB', (640, 360), (30, 40, 50))
        self.send_response(200)
        self.send_header('Content-Type', 'multipart/x-mixed-replace;boundary=frame')
        self.end_headers()
        t0 = time.time()
        try:
            while True:
                t = time.time() - t0
                im = base.copy()
                d = ImageDraw.Draw(im)
                x = int((t * 160) % 640)
                d.rectangle([x, 0, x + 18, 360], fill=(255, 138, 61))
                d.text((14, 330), 'LIVE %06.1f s' % t, fill=(255, 255, 255))
                b = io.BytesIO()
                im.save(b, 'JPEG', quality=70)
                j = b.getvalue()
                self.wfile.write(b'--frame\r\nContent-Type: image/jpeg\r\nContent-Length: %d\r\n\r\n' % len(j) + j + b'\r\n')
                self.wfile.flush()
                time.sleep(1 / 15)
        except (BrokenPipeError, ConnectionResetError):
            pass


class S(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True


S(('127.0.0.1', PORT), H).serve_forever()
