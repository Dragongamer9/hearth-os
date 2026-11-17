#!/usr/bin/env python3
"""A stand-in for Colibri's `coli` launcher (tests only): the commands hearth-colibri uses, answering as coli v2.1.0
does (c/coli, c/setup_flow.py), with no engine, no model and no network. `serve` is a tiny OpenAI-compatible server
(/health, /v1/models, /v1/chat/completions, /profile) so the dashboard's speed and Hearth's engine registry can be
checked end to end. Every call is noted in $FAKE_COLI_LOG."""
import json
import os
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

HERE = os.path.dirname(os.path.realpath(__file__))
CATALOG = os.environ.get('FAKE_COLI_CATALOG') or os.path.join(HERE, 'colibri-catalog.json')
HOME = os.environ.get('COLI_SETUP_HOME') or os.path.expanduser('~/.local/share/colibri')


def note(args):
    log = os.environ.get('FAKE_COLI_LOG')
    if log:
        with open(log, 'a') as f:
            f.write(json.dumps({'args': args, 'home': os.environ.get('COLI_SETUP_HOME', ''),
                                'vulkan': os.environ.get('COLI_VULKAN', '')}) + '\n')


def opt(args, name, default=None):
    return args[args.index(name) + 1] if name in args and args.index(name) + 1 < len(args) else default


def setup(args):
    rows = json.load(open(CATALOG))
    if '--list' in args:
        print('colibri setup\n')
        print(json.dumps(rows, indent=2))
        return 0
    mid, root = opt(args, '--model'), opt(args, '--dir')
    if mid not in [r['id'] for r in rows]:
        print('setup stopped: unknown model %s' % mid, file=sys.stderr)
        return 1
    if os.environ.get('FAKE_COLI_FAIL'):
        print('download stopped: the network went away\nRun the same command again to continue.', file=sys.stderr)
        return 1
    os.makedirs(HOME, exist_ok=True)
    total = 1000
    for done in (250, 1000):
        json.dump({'phase': 'download', 'pid': os.getpid(), 'done': done, 'total': total, 'rate': 5e6},
                  open(os.path.join(HOME, 'install-state.json'), 'w'))
    d = os.path.join(root, mid)
    os.makedirs(d, exist_ok=True)
    open(os.path.join(d, 'config.json'), 'w').write('{}')
    cfg = {'status': 'ready', 'model': {'id': mid}, 'model_dir': d, 'backend': 'vulkan', 'env': {'COLI_VULKAN': '1'},
           'port': 8000, 'host': '127.0.0.1'}
    json.dump(dict(cfg, version=1), open(os.path.join(HOME, 'setup.json'), 'w'))
    json.dump({'phase': 'ready', 'pid': os.getpid()}, open(os.path.join(HOME, 'install-state.json'), 'w'))
    print(json.dumps({'config': cfg, 'config_path': os.path.join(HOME, 'setup.json')}, indent=2))
    return 0


def plan(args):
    print(json.dumps({'version': 2, 'tiers': {'disk': {'model_bytes': 23_100_000_000}, 'ram': {'budget_bytes': 17_000_000_000},
                                               'vram': {'hot_expert_bytes': 9_500_000_000, 'trunk_bytes': 1_200_000_000}},
                      'expected_bottleneck': 'memory bandwidth', 'projected_hit_rate': 0.97, 'warnings': []}, indent=2))
    return 0


def doctor(args):
    print(json.dumps({'schema_version': 1, 'status': 'ok', 'checks': [{'id': 'engine', 'status': 'ok', 'summary': 'ready'}]}))
    return 0


def gguf_import(args):
    src, out = args[0], opt(args, '--out')
    if not src.endswith('.gguf') or not os.path.isfile(src):
        print('not a GGUF file', file=sys.stderr)
        return 1
    os.makedirs(out, exist_ok=True)
    json.dump({'architectures': ['Qwen3MoeForCausalLM']}, open(os.path.join(out, 'config.json'), 'w'))
    json.dump({'gguf': src}, open(os.path.join(out, 'gguf.json'), 'w'))
    print('wrote %s' % out)
    return 0


def serve(args):
    port, mid = int(opt(args, '--port', '8000')), opt(args, '--model-id', 'fake')
    host = opt(args, '--host', '127.0.0.1')
    turns = []

    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def send(self, code, obj):
            b = json.dumps(obj).encode()
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(b)))
            self.end_headers()
            self.wfile.write(b)

        def do_GET(self):
            if self.path == '/health':
                return self.send(200, {'status': 'ok'})
            if self.path == '/v1/models':
                return self.send(200, {'object': 'list', 'data': [{'id': mid, 'object': 'model', 'owned_by': 'colibri'}]})
            if self.path == '/profile':
                return self.send(200, {'turns': turns})
            self.send(404, {'error': 'not found'})

        def do_POST(self):
            n = int(self.headers.get('Content-Length') or 0)
            body = json.loads(self.rfile.read(n) or b'{}')
            if self.path == '/v1/chat/completions':
                turns.append({'wall_s': 2.0, 'completion_tokens': 25})
                return self.send(200, {'id': 'x', 'object': 'chat.completion', 'model': body.get('model'),
                                       'choices': [{'index': 0, 'message': {'role': 'assistant', 'content': 'Hello from Colibri'},
                                                    'finish_reason': 'stop'}]})
            self.send(404, {'error': 'not found'})

    srv = ThreadingHTTPServer((host, port), H)
    print('  OpenAI base URL:     http://%s:%d/v1' % (host, port), flush=True)
    srv.serve_forever()


def main(args):
    note(args)
    if not args:
        return 2
    return {'setup': setup, 'plan': plan, 'doctor': doctor, 'serve': serve, 'import': gguf_import}.get(args[0], lambda a: 2)(args[1:]) or 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
