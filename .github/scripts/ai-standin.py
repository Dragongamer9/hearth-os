"""Stand-in AIs for Describe it's pictures (ai-describe.sh, ai-describe-qml.yml): an OpenAI-style /v1/models on a port,
naming the models given. Nothing here thinks or answers chats; it only lets hearth-ai-check see that an AI is there.
  python3 ai-standin.py PORT MODEL [MODEL...]"""
import http.server
import json
import sys

port, models = int(sys.argv[1]), sys.argv[2:]


class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path.rstrip('/').endswith('/models'):
            body = json.dumps({'object': 'list', 'data': [{'id': m, 'object': 'model'} for m in models]}).encode()
        elif self.path.startswith('/api/tags'):
            body = json.dumps({'models': [{'name': m, 'size': 4900000000} for m in models]}).encode()
        else:
            self.send_response(404)
            self.end_headers()
            return
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *a):
        pass


http.server.HTTPServer(('127.0.0.1', port), H).serve_forever()
