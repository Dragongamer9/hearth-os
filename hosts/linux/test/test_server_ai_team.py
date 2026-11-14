"""Server mode › AI team (1.12, mockups server-ai-team-dash / -detail): One AI address counts today's requests per device
(numbers and a few words, never what was asked), and Server mode turns them into the live mesh's data."""
import importlib.machinery
import importlib.util
import json
import os
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


share = load('hearth_ai_share_team', 'hearth-ai-share')
sm = load('hearth_server_mode_team', 'hearth-server-mode')
TS = {'ips': ['100.64.0.1'], 'peers': [{'name': 'work-laptop', 'dns': 'work-laptop.tail.ts.net', 'ips': ['100.64.0.7']},
                                       {'name': 'rog-ally', 'dns': 'rog-ally.tail.ts.net', 'ips': ['100.64.0.9']}]}


class Counting(unittest.TestCase):
    def setUp(self):
        self.t = tempfile.TemporaryDirectory()
        self.p = os.path.join(self.t.name, 'ai-team.json')

    def tearDown(self):
        self.t.cleanup()

    def test_who_sent_it(self):
        self.assertEqual(share.team_sender('100.64.0.7', TS), ('work-laptop', ''))
        self.assertEqual(share.team_sender('127.0.0.1', TS), ('This PC', ''))
        self.assertEqual(share.team_sender('100.64.0.7', TS, 'claude'), ('Claude', 'Command Center'))
        self.assertEqual(share.team_sender('100.64.0.7', TS, '<b>x</b>')[0], 'Bxb')        # only plain letters kept

    def test_counted_per_device_and_where_it_was_answered(self):
        now = time.mktime((2026, 11, 14, 10, 0, 0, 0, 0, -1))
        share.team_note('work-laptop', '', 'chat', 'qwen3:30b-a3b', 'chris-hearth', 'chris-hearth', self.p, now)
        share.team_note('work-laptop', '', 'chat', 'qwen3:30b-a3b', 'chris-hearth', 'chris-hearth', self.p, now + 5)
        d = share.team_note('rog-ally', '', 'voice', '', 'pixel-8', 'chris-hearth', self.p, now + 9)
        self.assertEqual((d['total'], d['here']), (3, 2))
        self.assertEqual(d['sent'], {'pixel-8|voice': 1})
        self.assertEqual(d['senders']['work-laptop']['n'], 2)
        self.assertEqual(d['senders']['work-laptop']['last'], 'Asked qwen3:30b-a3b')
        self.assertEqual(d['senders']['rog-ally']['last'], 'Spoke a reply')
        self.assertNotIn('prompt', json.dumps(d))
        d = share.team_note('rog-ally', '', 'chat', 'x', 'chris-hearth', 'chris-hearth', self.p, now + 86400)   # the next day starts again
        self.assertEqual((d['total'], list(d['senders'])), (1, ['rog-ally']))

    def test_the_mesh_s_data(self):
        now = time.time()
        today = {'day': time.strftime('%Y-%m-%d', time.localtime(now)), 'total': 54, 'here': 48, 'sent': {'rog-ally|voice': 6},
                 'senders': {'Work laptop': {'n': 15, 'last': 'Asked qwen3:30b-a3b', 'at': now - 10, 'kind': 'chat'},
                             'Claude': {'n': 10, 'last': 'Asked qwen3', 'at': now - 3600, 'kind': 'chat', 'via': 'Command Center'}},
                 'lastEvent': {'who': 'Claude', 'via': 'Command Center', 'kind': 'chat', 'model': 'qwen3:30b', 'at': now - 30}}
        engines = [{'id': 'chat', 'state': 'working'}]
        who = [{'name': 'Living room TV', 'what': ['Media']}]
        t = sm.team_view(today, who, engines, {'total': 3, 'making': True}, {'ai': 'small', 'pictures': 'stop'}, True, 'chris-hearth', now)
        names = [x['name'] for x in t['senders']]
        self.assertEqual(names[:2], ['Work laptop', 'Claude'])
        self.assertIn('Living room TV', names)
        self.assertIn('Pictures queue', names)
        self.assertTrue(t['senders'][0]['active'])
        self.assertFalse(t['senders'][1]['active'])
        self.assertEqual(t['today']['busiest'], ['Work laptop', 15])
        self.assertEqual(t['today']['sent'], [['rog-ally', 6, 'voices']])
        self.assertEqual(t['play'], [['AI chat', 'small model'], ['Pictures', 'stop, keep the queue'], ['Other devices', 'take over']])
        self.assertEqual(t['address'], 'http://chris-hearth:8189/v1')
        self.assertTrue(t['event'].startswith('Claude (Command Center) asked for a reply · answered by qwen3'))
        self.assertEqual(t['chip'], 'Busy')

    def test_the_web_page_counts_itself(self):
        sm.team_web_open(path=self.p)
        sm.team_web_open(path=self.p)
        d = sm.team_today(path=self.p)
        self.assertEqual(d['senders']['Web page']['n'], 2)


if __name__ == '__main__':
    unittest.main()
