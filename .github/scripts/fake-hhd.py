#!/usr/bin/env python3
"""A stand-in Handheld Daemon for desktop-test.yml's screenshots of Settings › Handheld (SetHandheld.qml): HHD's local
API on 127.0.0.1:5335 (hhd-dev/hhd src/hhd/http/api.py: /api/v1/version, settings, state, sections; POST state takes
{dotted key: value} and answers with the new state) serving the whole ROG Ally of the preview (ui/index.html's
HHD_MOCK). No token is checked.  python3 fake-hhd.py ui/index.html [PORT]"""
import json
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

html = open(sys.argv[1], encoding='utf-8').read()
MOCK = json.loads(re.search(r'^const HHD_MOCK = (\{.*\});$', html, re.M).group(1))
STATE = MOCK['state']


def put(d, key, value):
    parts = key.split('.')
    for p in parts[:-1]:
        d = d.setdefault(p, {})
    d[parts[-1]] = value


class H(BaseHTTPRequestHandler):
    def send(self, obj):
        b = json.dumps(obj).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(b)))
        self.end_headers()
        self.wfile.write(b)

    def do_GET(self):
        path = self.path.split('?')[0]
        if path == '/api/v1/version':
            return self.send({'version': 5})
        if path == '/api/v1/settings':
            return self.send(MOCK['settings'])
        if path == '/api/v1/state':
            return self.send(STATE)
        if path == '/api/v1/sections':
            return self.send(MOCK.get('sections') or {})
        self.send_error(404)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers.get('Content-Length') or 0)) or b'{}')
        if self.path.split('?')[0] == '/api/v1/state':
            for k, v in body.items():
                if k.endswith('.reset') or k.endswith('.hibernate'):
                    continue   # buttons
                put(STATE, k, v)
        return self.send(STATE)

    def log_message(self, *a):
        pass


ThreadingHTTPServer(('127.0.0.1', int(sys.argv[2]) if len(sys.argv) > 2 else 5335), H).serve_forever()
