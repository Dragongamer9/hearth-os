"""Server mode › Ithil (1.13, mockups server-ithil-dash / -detail; checklist docs/mockups/checklists/server-ithil.md):
Ithil's files are only read (her state, settings, sleeps and her event log, opened read-only); the tile and the opened
view come from them, and her 3D brain from her real log entries."""
import importlib.machinery
import importlib.util
import json
import os
import sqlite3
import tempfile
import time
import unittest

SHELL = os.path.join(os.path.dirname(__file__), '..', 'desktop', 'shell')


def load(name, f):
    loader = importlib.machinery.SourceFileLoader(name, os.path.join(SHELL, f))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hi = load('hearth_ithil_t', 'hearth_ithil.py')


def iso(t):
    return time.strftime('%Y-%m-%dT%H:%M:%S', time.gmtime(t)) + '.000Z'


class Ithil(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.home = self.t.name
        hi.HOME = self.home
        self.now = time.time()
        db = sqlite3.connect(os.path.join(self.home, 'events.db'))
        db.execute('CREATE TABLE events (id INTEGER PRIMARY KEY AUTOINCREMENT, ts TEXT NOT NULL, actor TEXT NOT NULL, kind TEXT NOT NULL, '
                   'body TEXT, ref_id INTEGER, purged INTEGER NOT NULL DEFAULT 0)')
        rows = [(iso(self.now - 300), 'owner', 'owner.message', {'text': 'Do you remember what shift I work?'}, None),
                (iso(self.now - 299), 'system', 'system.input', {'observedKind': 'asks'}, None),
                (iso(self.now - 298), 'system', 'mood.snapshot', {'energy': 0.72, 'social': 0.58, 'curiosity': 0.7, 'stability': 0.8}, None),
                (iso(self.now - 290), 'ithil', 'ithil.message', {'text': 'Overnight, you get off at 6.'}, None),
                (iso(self.now - 200), 'ithil', 'memory.pin', {'key': 'mem:x', 'value': 'He works overnight'}, None),
                (iso(self.now - 100), 'system', 'gate.reject', {'limit': 'version_numbers', 'effect': 'file.write', 'requestId': 'r-1'}, None)]
        for ts, actor, kind, body, ref in rows:
            db.execute('INSERT INTO events (ts, actor, kind, body, ref_id) VALUES (?, ?, ?, ?, ?)', (ts, actor, kind, json.dumps(body), ref))
        db.commit(); db.close()
        self.w('state.json', {'name': 'Ithil', 'status': 'awake', 'current': {'mood': 'curious', 'wants': "the graphics card's heat tonight"}})
        self.w('settings.json', {'chatModel': 'huihui_ai/qwen3-abliterated:8b-v2', 'sleepTime': '06:10', 'allowCommandCenter': True})
        with open(os.path.join(self.home, 'sleeps.jsonl'), 'w') as f:
            f.write(json.dumps({'at': int((self.now - 3600) * 1000), 'end': int((self.now - 3000) * 1000), 'replayed': 41, 'energyBefore': 0.29, 'energyAfter': 0.88}) + '\n')
        self.w('link-clients.json', {'Team lead': {'at': int(self.now * 1000) - 5000, 'via': 'Tailscale'}})
        self.w('connect.json', {'token': 'secret-token-never-shown', 'port': 47860})

    def tearDown(self):
        self.t.cleanup()

    def w(self, name, obj):
        with open(os.path.join(self.home, name), 'w') as f:
            json.dump(obj, f)

    def test_the_tile_and_the_opened_view(self):
        v = hi.view(self.now)
        self.assertEqual(v['name'], 'Ithil')
        self.assertEqual(v['needs'], {'energy': 0.72, 'social': 0.58, 'curiosity': 0.7, 'stability': 0.8})
        self.assertEqual(v['mood'], 'curious')                         # her needs' words (energy .72 isn't "rested")
        self.assertEqual(v['focus'], "the graphics card's heat tonight")
        self.assertEqual(v['today'], {'talks': 1, 'memories': 1, 'blocks': 1})
        self.assertEqual(v['sleepTime'], '06:10')
        self.assertIn(':47860', v['address'])
        self.assertEqual(v['sleepLine'][1], '41 moments replayed, energy .29 → .88')
        self.assertTrue(v['sleepLine'][0].startswith('Ithil slept '))
        self.assertEqual(v['reach'][0]['name'], 'Team lead')
        self.assertIn('Connected', v['reach'][0]['chip'])
        self.assertIn('Tailscale', v['reach'][0]['chip'])
        self.assertTrue(v['reach'][-1]['never'])
        self.assertNotIn('secret-token', json.dumps(v))               # her key is never shown

    def test_not_on_this_pc(self):
        hi.HOME = os.path.join(self.home, 'nope')
        orig = hi.app_path
        hi.app_path = lambda: None
        try:
            self.assertIsNone(hi.view(self.now))
        finally:
            hi.app_path = orig

    def test_her_brain_lights_from_her_real_log(self):
        b = hi.brain(24, self.now)
        regs = {e['reg'] for e in b['events']}
        for r in ('senses', 'prediction', 'workspace', 'memory', 'knowledge', 'gate', 'needs', 'confidence', 'sleep'):
            self.assertIn(r, regs)
        self.assertTrue(any(e.get('bc') for e in b['events']))
        self.assertTrue(any(e.get('block') for e in b['events']))
        self.assertEqual(len(b['moods']), 1)
        self.assertEqual(len(b['sleeps']), 1)
        whole = hi.brain(0, self.now)
        self.assertLessEqual(whole['from'], int((self.now - 300) * 1000))

    def test_her_log_is_opened_read_only(self):
        db = hi._db()
        with self.assertRaises(sqlite3.OperationalError):
            db.execute("INSERT INTO events (ts, actor, kind) VALUES ('x', 'owner', 'owner.message')")
        db.close()

    def test_mood_words_follow_her_rules(self):
        self.assertEqual(hi.mood_words({'energy': 0.2, 'curiosity': 0.9, 'stability': 0.3}), ['tired', 'curious', 'unsettled'])
        self.assertEqual(hi.mood_words({'energy': 0.5, 'curiosity': 0.4, 'stability': 0.8}), ['steady'])


class Screens(unittest.TestCase):
    def read(self, f):
        with open(os.path.join(SHELL, f)) as fh:
            return fh.read()

    def test_dashboard_tile_two_wide_after_the_team(self):
        sm = self.read('hearth-server-mode')
        self.assertIn("tiles.insert(1, {'id': 'ithil'", sm)
        self.assertIn("'wide': True, 'ithil': ithil", sm)
        self.assertIn("('Ithil', '', ('running' if ithil['running'] else 'stopped') if ithil else 'off')", sm)
        dash = self.read('ServerDash.qml')
        self.assertIn('Layout.columnSpan: modelData.wide ? 2 : 1', dash)
        self.assertIn('Quickshell.execDetached(["python3", dash.helper, "ithil-start"])', dash)
        self.assertIn("'s brain with it)", dash)
        tile = self.read('ServerTile.qml')
        self.assertIn('IthilBrain { anchors.fill: parent; compact: true', tile)

    def test_opened_view(self):
        d = self.read('IthilDetail.qml')
        for words in ('"● Live"', '"Last 24 hours"', '"Her whole life"', '"Right now"', '"Today"', '"Her brain on this PC"', '"Reach her from"',
                      '"Talk to her"', '"Restart her app"', '"Put her to sleep (asks twice)"', '"Her settings"', '"Ⓑ Back to the dashboard"',
                      'move through time (the 4th dimension)', 'Pausing the AI team asks first, because her brain is in it.'):
            self.assertIn(words, d)

    def test_the_brain_is_the_team_s(self):
        js = self.read('ithil-brain.js')
        for part in ("name:'Senses'", "name:'Event log'", "name:'Workspace'", "name:'Hard-limit gate'", "name:'Sleep cycle'"):
            self.assertIn(part, js)
        self.assertIn('function activity(data, T, nowMs, live, asleep)', js)


if __name__ == '__main__':
    unittest.main()
