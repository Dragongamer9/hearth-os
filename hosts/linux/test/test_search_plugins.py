"""Hearth's search plugins (hosts/linux/desktop/shell/search/*.js, like KRunner's runners) for the app menus' search:
the calculator, units, currency (offline-safe), web shortcuts, commands, emoji, recent documents and open windows,
and runners.js putting them together. Runs them in node the way QML loads them (search_harness.js)."""
import importlib.util
import json
import pathlib
import shutil
import subprocess
import tempfile
import unittest

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SEARCH = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'search'
NODE = shutil.which('node')


def calls(*cs):
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False) as f:
        json.dump({'calls': list(cs)}, f)
    r = subprocess.run([NODE, str(HERE / 'search_harness.js'), str(SEARCH / 'runners.js'), f.name], capture_output=True, text=True, timeout=60)
    pathlib.Path(f.name).unlink()
    if r.returncode:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout)


@unittest.skipUnless(NODE, 'node is not installed')
class Plugins(unittest.TestCase):
    def test_calculator(self):
        qs = ['2+2', '=sqrt(16)', '15% of 80', '(3+4)*2^3', '0xff+1', '3 x 4', '10 % 3', '2**10', 'sin(90) deg', '=2pi',
              'hello', '42', 'firefox', '1/0', 'task manager', '']
        got = calls(*[['calc.js', 'answer', q] for q in qs])
        want = ['4', '4', '12', '56', '256', '12', '1', '1024', '1', '6.2831853072', None, None, None, None, None, None]
        self.assertEqual([g and g['text'] for g in got], want)

    def test_never_eval(self):
        src = (SEARCH / 'calc.js').read_text()
        self.assertNotIn('eval(', src)
        self.assertNotIn('Function(', src)
        self.assertIsNone(calls(['calc.js', 'answer', '=alert(1)'])[0])

    def test_units(self):
        qs = ['10 km to mi', '72 f in c', '3.5 kg = lb', '500 MB to GB', '60 mph to km/h', '100 c to k', '90 min in hours',
              '5 kg to km', 'cats to dogs', '10 km']
        got = calls(*[['units.js', 'answer', q] for q in qs])
        self.assertEqual([g and g['text'] for g in got],
                         ['6.2137 mi', '22.2222 °C', '7.7162 lb', '0.5 GB', '96.5606 km/h', '373.15 K', '1.5 hours', None, None, None])

    def test_currency_offline_says_so(self):
        got = calls(['currency.js', 'answer', '20 usd to eur', None], ['currency.js', 'answer', '€50 in $', None],
                    ['currency.js', 'answer', '10 eur to usd', {'date': '2026-11-15', 'source': 'ecb', 'rates': {'EUR': 1, 'USD': 1.2}}],
                    ['currency.js', 'answer', '10 eur to usd', {'broken': True}],
                    ['currency.js', 'answer', '10 km to mi', None], ['currency.js', 'answer', '5 xyz to eur', None])
        self.assertEqual(got[0]['text'], '17.70 EUR')
        self.assertIn('Built-in rates', got[0]['sub'])
        self.assertIn('may be out of date', got[0]['sub'])
        self.assertEqual(got[1]['text'], '56.50 USD')
        self.assertEqual(got[2]['text'], '12.00 USD')
        self.assertIn('ECB rates from 2026-11-15', got[2]['sub'])
        self.assertIn('Built-in', got[3]['sub'])          # unusable saved rates: the built-in ones, said so
        self.assertIsNone(got[4])
        self.assertIsNone(got[5])

    def test_web_shortcuts(self):
        got = calls(['web.js', 'url', 'gg:hearth os'], ['web.js', 'url', 'wp: Linux'], ['web.js', 'url', 'yt:lofi beats'],
                    ['web.js', 'url', 'hearthos.fun'], ['web.js', 'url', 'https://example.org/a?b'], ['web.js', 'url', 'zz:thing'],
                    ['web.js', 'url', 'files'], ['web.js', 'url', 'task manager'])
        self.assertEqual(got[0]['url'], 'https://www.google.com/search?q=hearth+os')
        self.assertEqual(got[1]['url'], 'https://en.wikipedia.org/wiki/Special:Search?search=Linux')
        self.assertEqual(got[2]['url'], 'https://www.youtube.com/results?search_query=lofi+beats')
        self.assertEqual(got[3]['url'], 'https://hearthos.fun')
        self.assertEqual(got[4]['url'], 'https://example.org/a?b')
        self.assertEqual(got[5:], [None, None, None])

    def test_commands_only_when_asked(self):
        got = calls(['runners.js', 'run', '>htop', {}], ['runners.js', 'run', 'htop', {}])
        self.assertEqual([r['action'] for r in got[0]], [{'type': 'command', 'value': 'htop', 'terminal': True},
                                                          {'type': 'command', 'value': 'htop', 'terminal': False}])
        self.assertEqual(got[1], [])

    def test_emoji(self):
        got = calls(['emoji.js', 'results', 'thumbs up'], ['emoji.js', 'results', ':fire'], ['emoji.js', 'results', 'he'])
        self.assertEqual(got[0][0]['glyph'], '👍')
        self.assertEqual(got[1][0]['glyph'], '🔥')
        self.assertTrue(got[1][0]['top'])
        self.assertEqual(got[2], [])                       # two letters without ":" find no emoji (too many)

    def test_recent_and_windows(self):
        recent = [{'name': 'Budget 2026.ods', 'path': '/home/a/Budget 2026.ods', 'folder': 'Documents'},
                  {'name': 'notes.txt', 'path': '/home/a/notes.txt', 'folder': 'Home'}]
        wins = [{'id': 'w1', 'title': 'Budget 2026.ods — Sheets', 'appId': 'hearth-sheets'}, {'id': 'w2', 'title': 'Steam', 'appId': 'steam'}]
        got = calls(['recent.js', 'results', 'budget', recent], ['windows.js', 'results', 'steam', wins], ['windows.js', 'results', 'steam', []])
        self.assertEqual(got[0][0]['action'], {'type': 'file', 'value': '/home/a/Budget 2026.ods', 'dir': False})
        self.assertEqual(got[1][0]['action'], {'type': 'window', 'value': 'w2'})
        self.assertEqual(got[2], [])

    def test_runners_together(self):
        got = calls(['runners.js', 'run', '2+2', {}], ['runners.js', 'run', 'gg:x', {}], ['runners.js', 'run', '2+2', {'off': {'calc': True}}],
                    ['runners.js', 'run', 'firefox', {}], ['runners.js', 'run', '', {}])
        self.assertEqual(got[0][0]['plugin'], 'calc')
        self.assertTrue(got[0][0]['top'])
        self.assertEqual(got[1][0]['plugin'], 'web')
        self.assertEqual(got[2], [])
        self.assertEqual(got[3], [])
        self.assertEqual(got[4], [])

    def test_emoji_data_current(self):
        spec = importlib.util.spec_from_file_location('build_search_emoji', ROOT / 'tools' / 'build-search-emoji.py')
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        self.assertEqual((SEARCH / 'emoji-data.js').read_text(encoding='utf-8'), mod.text(),
                         'run python3 tools/build-search-emoji.py')


if __name__ == '__main__':
    unittest.main()
