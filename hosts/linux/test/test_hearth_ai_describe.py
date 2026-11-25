"""Describe it uses… (the owner, Nov 24, 2026: Describe it sat on "Your AI is making it…" for minutes on his laptop's
slow CPU Ollama while his PC's AI was there over Hearth Link). hearth-ai-check asks where the person said first
(This device / My other devices), else the fastest that answers: a slow engine on this device (measured under 8 words
a second, or no graphics card when not measured) goes after their other devices. Only Settings changes the pick."""
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock

from test_hearth_ai_check import load, server

ROOT = pathlib.Path(__file__).resolve().parents[3]


class DescribeUses(unittest.TestCase):
    def setUp(self):
        self.m = load()
        self.tmp = tempfile.mkdtemp()
        for k, f in (('DESCRIBE', 'describe.json'), ('ONLINE', 'ai-online.json'), ('USES_FILE', 'ai-uses.json'), ('SPEED', 'ai-speed.json')):
            setattr(self.m, k, os.path.join(self.tmp, f))
        self.m.LINK = 'http://127.0.0.1:9/v1'
        self.m.SERVICES = {}

    def test_describe_asks_the_fastest_unless_told(self):
        a, here = server(['dolphin3:8b'])
        l, link = server(['qwen3:30b-a3b'])
        self.m.LINK = link
        eng = {'id': 'ollama', 'label': 'Ollama', 'on': True, 'openai': here}
        try:
            with mock.patch.object(self.m, 'engines', return_value=('ollama', [eng])), mock.patch.object(self.m, 'has_gpu', return_value=False):
                # no graphics card, not measured: this laptop's Ollama is slow, so the PC over Hearth Link answers
                self.assertEqual(self.m.for_use('describe')['from'], 'link')
                st = self.m.status()
                self.assertEqual(st['answering']['from'], 'engine')        # Hearth's AI keeps your order
                self.assertEqual(st['uses']['describe']['label'], 'Fastest available')
                self.assertTrue(st['uses']['describe']['ready'])
                self.assertTrue(self.m.set_use('describe', json.dumps({'from': 'here'}))['ok'])
                self.assertEqual(self.m.for_use('describe')['from'], 'engine')   # your pick, slow or not
                self.assertEqual(self.m.status()['uses']['describe']['label'], 'This device')
                self.assertTrue(self.m.set_use('describe', json.dumps({'from': 'link'}))['ok'])
                self.assertEqual(self.m.for_use('describe')['from'], 'link')
                self.assertFalse(self.m.set_use('photos', json.dumps({'from': 'here'}))['ok'])
                self.assertFalse(self.m.set_use('describe', json.dumps({'from': 'somewhere'}))['ok'])
                self.assertTrue(self.m.set_use('describe', json.dumps({'from': 'fastest'}))['ok'])
                self.assertNotIn('describe', json.loads(pathlib.Path(self.m.USES_FILE).read_text())['uses'])
                # measured fast here: it's asked first again
                pathlib.Path(self.m.SPEED).write_text(json.dumps({'dolphin3:8b': {'words': 30}}))
                self.assertEqual(self.m.for_use('describe')['from'], 'engine')
                # measured slow (Describe it's own "felt" speed): the other device
                pathlib.Path(self.m.SPEED).write_text(json.dumps({'dolphin3:8b': {'felt': 3.3}}))
                self.assertEqual(self.m.for_use('describe')['from'], 'link')
                self.assertEqual(self.m.status()['uses']['describe']['words'], None)   # the fastest is the link's
        finally:
            a.shutdown()
            l.shutdown()

    def test_the_picked_place_not_answering_falls_back_and_says_so(self):
        a, here = server(['qwen3:8b'])
        eng = {'id': 'ollama', 'label': 'Ollama', 'on': True, 'openai': here}
        self.m.set_use('describe', json.dumps({'from': 'link'}))
        try:
            with mock.patch.object(self.m, 'engines', return_value=('ollama', [eng])), mock.patch.object(self.m, 'has_gpu', return_value=True):
                r = self.m.for_use('describe')
                st = self.m.status()
        finally:
            a.shutdown()
        self.assertEqual(r['from'], 'engine')
        self.assertIn("isn't answering", r['note'])
        self.assertFalse(st['uses']['describe']['ready'] and st['uses']['describe']['via'] == '')


def fixtures(tmp, desktop_url):
    """What hearth-ai-share devices and hearth-link-settings ai-status would say: the PC ready with an Nvidia card, the
    Ally playing, and the office PC that Hearth Link couldn't confirm (its certificate didn't match)."""
    devs = {'devices': [
        {'device': 'laptop', 'name': 'laptop', 'via': 'this device', 'online': True, 'services': []},
        {'device': 'desktop', 'name': 'Desktop', 'via': 'tailscale', 'online': True, 'available': True, 'gpu': {'vendor': 'nvidia', 'vramMB': 12288},
         'services': [{'kind': 'chat', 'engine': 'ollama', 'openai': desktop_url, 'url': desktop_url[:-3], 'healthy': True,
                       'models': ['qwen3:30b-a3b', 'nomic-embed-text:latest']}]},
        {'device': 'rog-ally', 'name': 'ROG Ally', 'via': 'tailscale', 'online': True, 'gaming': True, 'reason': 'playing',
         'gpu': {'vendor': 'amd', 'vramMB': 4096}, 'services': [{'kind': 'chat', 'engine': 'ollama', 'openai': 'http://100.64.0.3:11434/v1', 'models': ['llama3.2:3b']}]}]}
    link = {'servers': [], 'unreached': [{'id': 'ab12', 'name': 'Office PC', 'mismatch': True}, {'id': 'cd34', 'name': 'Old laptop', 'mismatch': False}], 'using': None}
    d, l = os.path.join(tmp, 'devs.json'), os.path.join(tmp, 'link.json')
    pathlib.Path(d).write_text(json.dumps(devs))
    pathlib.Path(l).write_text(json.dumps(link))
    return d, l


class SharedDevices(unittest.TestCase):
    """Settings › AI › Shared from your other devices (the owner, Nov 24): every AI your devices share, read only."""
    def setUp(self):
        self.m = load()
        self.tmp = tempfile.mkdtemp()
        for k, f in (('DESCRIBE', 'describe.json'), ('ONLINE', 'ai-online.json'), ('USES_FILE', 'ai-uses.json'), ('SPEED', 'ai-speed.json')):
            setattr(self.m, k, os.path.join(self.tmp, f))
        self.m.LINK = 'http://127.0.0.1:9/v1'
        self.m.SERVICES = {}

    def test_the_list_and_using_one_for_describe_it(self):
        pc, base = server(['qwen3:30b-a3b'])
        d, l = fixtures(self.tmp, base)
        try:
            with mock.patch.dict(os.environ, {'HEARTH_AI_SHARED_FIXTURE': d, 'HEARTH_AI_LINK_FIXTURE': l}), \
                    mock.patch.object(self.m, 'engines', return_value=('ollama', [])):
                rows = {r['device']: r for r in self.m.shared()['devices']}
                self.assertNotIn('laptop', rows)                                  # this device isn't "shared from"
                self.assertEqual((rows['desktop']['state'], rows['desktop']['gpu'], rows['desktop']['models']), ('ready', 'nvidia', ['qwen3:30b-a3b']))
                self.assertEqual(rows['rog-ally']['state'], 'playing')
                self.assertEqual(rows['office-pc']['state'], 'unconfirmed')
                self.assertFalse(rows['office-pc']['canUse'])
                self.assertNotIn('old-laptop', rows)                              # just away: not listed as a problem
                self.assertFalse(rows['desktop']['inUse'])
                self.assertFalse(self.m.use_shared('office-pc')['ok'])
                self.assertTrue(self.m.use_shared('desktop')['ok'])
                saved = json.loads(pathlib.Path(self.m.USES_FILE).read_text())['uses']['describe']
                self.assertEqual(saved, {'url': base.rstrip('/'), 'model': 'qwen3:30b-a3b', 'label': 'Desktop'})
                self.assertTrue({r['device']: r for r in self.m.shared()['devices']}['desktop']['inUse'])
        finally:
            pc.shutdown()


class OnPath(unittest.TestCase):
    """hearth-ai-check is a command (the owner had to type python3 /usr/share/nimbus/desktop/shell/hearth-ai-check)."""
    def test_both_images_put_it_on_path(self):
        for b in ('os-bases/peppermint/build.sh', 'os-bases/anatase/build.sh'):
            t = (ROOT / b).read_text()
            self.assertIn('chmod 755 /usr/share/nimbus/desktop/shell/hearth-ai-check', t, b)
            self.assertIn('ln -sfn /usr/share/nimbus/desktop/shell/hearth-ai-check /usr/bin/hearth-ai-check', t, b)

    def test_it_runs_from_a_link_and_is_executable(self):
        tool = ROOT / 'hosts/linux/desktop/shell/hearth-ai-check'
        self.assertTrue(tool.read_text().startswith('#!/usr/bin/python3'))
        self.assertTrue(os.access(tool, os.X_OK))
        self.assertIn('os.path.realpath(__file__)', tool.read_text())   # finds its helpers through the link


if __name__ == '__main__':
    unittest.main()
