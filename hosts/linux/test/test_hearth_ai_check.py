"""hearth-ai-check (Settings › AI, the approved mockups ai-1 to ai-4; the owner, Nov 23: "fix the AI problem"): which
AI answers, in Describe it's order; a model that can only search is never the one for writing; nothing answering is
said in plain words; it never prints a key or changes anything."""
import http.server
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import threading
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
TOOL = ROOT / 'hosts/linux/desktop/shell/hearth-ai-check'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_ai_check', str(TOOL))
    spec = importlib.util.spec_from_loader('hearth_ai_check', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def server(models):
    class H(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            body = json.dumps({'data': [{'id': m} for m in models]}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *a):
            pass
    s = http.server.HTTPServer(('127.0.0.1', 0), H)
    threading.Thread(target=s.serve_forever, daemon=True).start()
    return s, 'http://127.0.0.1:%d/v1' % s.server_address[1]


class AiCheck(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.tmp = tempfile.mkdtemp()
        self.m.DESCRIBE = os.path.join(self.tmp, 'describe.json')
        self.m.ONLINE = os.path.join(self.tmp, 'ai-online.json')
        self.m.USES_FILE = os.path.join(self.tmp, 'ai-uses.json')
        self.m.SPEED = os.path.join(self.tmp, 'ai-speed.json')
        self.m.LINK = 'http://127.0.0.1:9/v1'
        self.m.SERVICES = {}

    def engines(self, *es):
        return mock.patch.object(self.m, 'engines', return_value=('ollama', list(es)))

    def test_a_search_only_model_is_never_for_writing(self):
        self.assertEqual(self.m.pick_model(['nomic-embed-text:latest', 'qwen3:30b-a3b']), 'qwen3:30b-a3b')
        self.assertEqual(self.m.pick_model(['nomic-embed-text:latest']), '')

    def test_the_engine_that_answers_with_a_writing_model(self):
        s, base = server(['nomic-embed-text:latest', 'qwen3:30b-a3b'])
        try:
            with self.engines({'id': 'ollama', 'label': 'Ollama', 'on': True, 'openai': base, 'model': ''}):
                st = self.m.status()
        finally:
            s.shutdown()
        self.assertEqual(st['answering']['from'], 'engine')
        self.assertEqual(st['answering']['model'], 'qwen3:30b-a3b')
        self.assertFalse([p for p in st['problems'] if p['kind'] == 'none'])

    def test_only_search_models_is_a_problem_and_hearth_link_answers(self):
        s1, base = server(['nomic-embed-text:latest'])
        s2, link = server(['qwen3:30b-a3b'])
        self.m.LINK = link
        try:
            with self.engines({'id': 'ollama', 'label': 'Ollama', 'on': True, 'openai': base, 'model': ''}):
                st = self.m.status()
        finally:
            s1.shutdown()
            s2.shutdown()
        self.assertIn('no-model', [p['kind'] for p in st['problems']])
        self.assertEqual(st['answering']['from'], 'link')

    def test_nothing_answering_says_what_to_do(self):
        with self.engines({'id': 'ollama', 'label': 'Ollama', 'on': False, 'openai': 'http://127.0.0.1:9/v1', 'model': ''}):
            st = self.m.status()
        self.assertIsNone(st['answering'])
        none = [p for p in st['problems'] if p['kind'] == 'none']
        self.assertTrue(none and 'Hearth Link' in none[0]['why'])

    def test_describe_its_own_choice_is_its_own(self):
        # describe.json is Describe it's own pick (Settings › AI › What uses your AI), not Hearth's AI for everything
        s, base = server(['llama3.2'])
        pathlib.Path(self.m.DESCRIBE).write_text(json.dumps({'url': base, 'model': '', 'label': 'My PC'}))
        try:
            with self.engines():
                st = self.m.status()
                for_d = self.m.for_use('describe')
        finally:
            s.shutdown()
        self.assertEqual(st['uses']['describe']['label'], 'My PC')
        self.assertTrue(st['uses']['describe']['ready'])
        self.assertEqual(st['uses']['describe']['model'], 'llama3.2')
        self.assertEqual((for_d['label'], for_d['model']), ('My PC', 'llama3.2'))
        self.assertFalse(st['uses']['photos']['own'])

    def test_each_use_and_the_order_are_saved_and_followed(self):
        a, base_a = server(['qwen3:8b'])
        l, base_l = server(['llama3.2'])
        self.m.LINK = base_l
        here = {'id': 'ollama', 'label': 'Ollama', 'on': True, 'openai': base_a}
        try:
            with self.engines(here):
                self.assertEqual(self.m.status()['answering']['label'], 'Ollama')     # this device first by default
                self.assertTrue(self.m.set_order('link,here,online')['ok'])
                st = self.m.status()
                self.assertEqual(st['order'], ['link', 'here', 'online'])
                self.assertEqual(st['answering']['from'], 'link')                    # your order is followed
                self.assertFalse(self.m.set_order('link,link,online')['ok'])
                self.assertTrue(self.m.set_use('sortdl', json.dumps({'url': base_a, 'model': 'qwen3:8b', 'label': 'Ollama'}))['ok'])
                self.assertEqual(self.m.for_use('sortdl')['url'], base_a.rstrip('/'))
                self.assertEqual(self.m.for_use('ithil')['from'], 'link')            # the rest stay on Hearth's AI
                self.assertTrue(self.m.set_use('sortdl', 'hearth')['ok'])
                self.assertEqual(self.m.for_use('sortdl')['from'], 'link')
                self.assertFalse(self.m.set_use('sortdl', json.dumps({'url': 'file:///etc'}))['ok'])
        finally:
            a.shutdown()
            l.shutdown()
        saved = json.loads(pathlib.Path(self.m.USES_FILE).read_text())
        self.assertEqual(saved['order'], ['link', 'here', 'online'])
        self.assertNotIn('sortdl', saved['uses'])

    def test_speed_is_only_shown_when_measured(self):
        self.assertEqual(self.m.read_speeds(), {})
        pathlib.Path(self.m.SPEED).write_text(json.dumps({'qwen3:8b': {'words': 31.4, 'at': 1}}))
        self.assertEqual(self.m.read_speeds()['qwen3:8b']['words'], 31.4)

    def test_it_never_prints_a_key(self):
        text = TOOL.read_text()
        self.assertNotIn("print(key", text)
        self.assertIn("'secret-tool', 'lookup'", text)
        self.assertIn('capture_output=True', text)


if __name__ == '__main__':
    unittest.main()
