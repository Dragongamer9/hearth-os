"""Hearth's file index (hosts/linux/desktop/shell/hearth-search, like KDE's Baloo): names and (when asked) documents'
text into SQLite FTS5, hidden folders / caches / game libraries / your exclusions never read, gone files out, paused on
battery, settings written only as you change them (hearth_settings_io), the app menus' and Hearth Files' searches
asking it first, the ECB rates parsed and kept when offline, and the user service at the lowest priority with nothing
at sign-in waiting for it."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest
import zipfile

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
HELPER = SHELL / 'hearth-search'
UNIT = ROOT / 'os' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'user' / 'hearth-search.service'


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Index(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        h = pathlib.Path(self.tmp.name)
        self.home = h
        self.env = dict(os.environ, HOME=str(h), XDG_CONFIG_HOME=str(h / '.config'), XDG_DATA_HOME=str(h / '.local/share'),
                        XDG_CACHE_HOME=str(h / '.cache'), HEARTH_INPUT='')
        for d in ('Documents/Taxes', 'Music', '.secret', 'node_modules', 'Games/steamapps', 'Private', '.cache'):
            (h / d).mkdir(parents=True, exist_ok=True)
        (h / 'Documents/report.txt').write_text('the quarterly budget numbers')
        (h / 'Documents/Taxes/Budget-2026.ods').write_bytes(b'')
        (h / '.secret/budget.txt').write_text('x')
        (h / 'node_modules/budget.js').write_text('x')
        (h / 'Games/steamapps/budget.vdf').write_text('x')
        (h / 'Private/budget-private.txt').write_text('x')
        (h / 'Music/song.mp3').write_bytes(b'')
        with zipfile.ZipFile(h / 'Documents/letter.docx', 'w') as z:
            z.writestr('word/document.xml', '<w:document><w:p><w:t>Dear landlord, about the heating</w:t></w:p></w:document>')
        with zipfile.ZipFile(h / 'Documents/minutes.odt', 'w') as z:
            z.writestr('content.xml', '<office:text><text:p>Café meeting &amp; plans</text:p></office:text>')

    def tearDown(self):
        self.tmp.cleanup()

    def run_h(self, *args, stdin=''):
        env = dict(self.env, HEARTH_INPUT=stdin)
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), capture_output=True, text=True, env=env, timeout=60)
        last = r.stdout.strip().splitlines()[-1] if r.stdout.strip() else ''
        return r.returncode, (json.loads(last) if last and not last.startswith('error:') else last)

    def names(self, hits):
        return sorted(x['name'] for x in hits)

    def test_no_index_yet_says_so(self):
        self.assertEqual(self.run_h('query', 'budget'), (0, None))

    def test_names_and_what_is_never_read(self):
        self.assertEqual(self.run_h('prefs-set', stdin='{"exclude": ["~/Private"]}')[0], 0)
        code, res = self.run_h('index')
        self.assertEqual(code, 0, res)
        _, hits = self.run_h('query', 'budget')
        self.assertEqual(self.names(hits), ['Budget-2026.ods'])        # not hidden, node_modules, steamapps, Private
        self.assertEqual(hits[0]['folder'], 'Documents › Taxes')
        _, hits = self.run_h('query', 'heating')
        self.assertEqual(hits, [])                                     # documents' text: off until you turn it on
        db = self.home / '.local/share/hearth/search/index.db'
        self.assertEqual(oct(db.stat().st_mode & 0o777), '0o600')

    def test_contents_and_gone_files(self):
        self.run_h('prefs-set', stdin='{"contents": true}')
        self.run_h('index')
        _, hits = self.run_h('query', 'landlord')
        self.assertEqual(self.names(hits), ['letter.docx'])
        self.assertIn('[landlord]', hits[0]['snippet'])
        _, hits = self.run_h('query', 'cafe')                         # accents folded
        self.assertEqual(self.names(hits), ['minutes.odt'])
        _, hits = self.run_h('query', 'quarterly', '--under', str(self.home / 'Music'))
        self.assertEqual(hits, [])
        (self.home / 'Documents/report.txt').unlink()
        _, res = self.run_h('index')
        self.assertEqual(res['removed'], 1)
        _, hits = self.run_h('query', 'quarterly')
        self.assertEqual(hits, [])
        # contents off again: the text is dropped from the index
        self.run_h('prefs-set', stdin='{"contents": false}')
        self.run_h('index')
        _, hits = self.run_h('query', 'landlord', '--contents')
        self.assertEqual(hits, [])

    def test_settings_only_what_you_change(self):
        cfg = self.home / '.config/hearth/search.json'
        self.run_h('status')
        self.run_h('index')
        self.assertFalse(cfg.exists(), 'defaults must never be written')
        self.assertEqual(self.run_h('prefs-set', stdin='{"bogus": 1}')[1], 'error: Unknown setting: bogus')
        self.assertEqual(self.run_h('prefs-set', stdin='{"contents": "yes"}')[1], 'error: contents must be true or false')
        self.run_h('prefs-set', stdin='{"pluginsOff": {"emoji": true, "calc": false}}')
        self.assertEqual(json.loads(cfg.read_text()), {'pluginsOff': {'emoji': True}})
        cfg.write_text(json.dumps({'pluginsOff': {'emoji': True}, 'someoneElse': 5}))
        self.run_h('prefs-set', stdin='{"indexing": false}')
        self.assertEqual(json.loads(cfg.read_text()), {'pluginsOff': {'emoji': True}, 'someoneElse': 5, 'indexing': False})
        self.assertEqual(self.run_h('enabled')[0], 1)
        self.run_h('prefs-set', stdin='{"indexing": null}')
        self.assertEqual(self.run_h('enabled')[0], 0)
        # a damaged file is not overwritten with defaults by reading it
        cfg.write_text('{not json')
        self.run_h('status')
        self.assertTrue(cfg.exists())

    def test_rebuild(self):
        self.run_h('index')
        code, res = self.run_h('rebuild')
        self.assertEqual((code, res['ok']), (0, True))
        import time
        for _ in range(100):                                  # the new index is made in the background
            st = self.run_h('status')[1]
            if st['complete'] and not st['running']:
                break
            time.sleep(0.1)
        self.assertTrue(st['complete'])
        self.assertEqual(self.names(self.run_h('query', 'budget')[1]), ['Budget-2026.ods', 'budget-private.txt'])


class Battery(unittest.TestCase):
    def setUp(self):
        self.mod = load('hearth_search_mod', HELPER)
        self.tmp = tempfile.TemporaryDirectory()

    def tearDown(self):
        self.tmp.cleanup()

    def supply(self, name, **files):
        d = pathlib.Path(self.tmp.name) / name
        d.mkdir()
        for k, v in files.items():
            (d / k).write_text(v)

    def test_on_battery(self):
        self.assertFalse(self.mod.on_battery(self.tmp.name))           # a desktop: nothing there
        self.supply('BAT0', type='Battery', status='Discharging')
        self.supply('AC', type='Mains', online='0')
        self.assertTrue(self.mod.on_battery(self.tmp.name))
        (pathlib.Path(self.tmp.name) / 'AC' / 'online').write_text('1')
        self.assertFalse(self.mod.on_battery(self.tmp.name))

    def test_controller_battery_is_not_the_pc(self):
        self.supply('ps-controller-battery', type='Battery', scope='Device', status='Discharging')
        self.assertFalse(self.mod.on_battery(self.tmp.name))

    def test_ecb_rates(self):
        xml = ("<gesmes:Envelope><Cube><Cube time='2026-11-13'><Cube currency='USD' rate='1.1234'/><Cube currency='JPY' rate='170.5'/>"
               "<Cube currency='GBP' rate='0.87'/><Cube currency='CHF' rate='0.93'/><Cube currency='SEK' rate='11.2'/></Cube></Cube></gesmes:Envelope>")
        d = self.mod.parse_ecb(xml)
        self.assertEqual(d['date'], '2026-11-13')
        self.assertEqual(d['rates']['USD'], 1.1234)
        self.assertEqual(d['rates']['EUR'], 1.0)
        with self.assertRaises(self.mod.Fail):
            self.mod.parse_ecb('<html>captive portal</html>')

    def test_offline_keeps_old_rates(self):
        def boom():
            raise OSError('no network')
        old = dict(os.environ)
        try:
            os.environ['XDG_CONFIG_HOME'] = self.tmp.name
            mod = load('hearth_search_mod2', HELPER)
            mod.RATES = os.path.join(self.tmp.name, 'rates.json')
            pathlib.Path(mod.RATES).write_text('{"date": "2026-11-01", "source": "ecb", "rates": {"EUR": 1}}')
            self.assertEqual(mod.rates(fetch=boom)['offline'], True)
            self.assertIn('2026-11-01', pathlib.Path(mod.RATES).read_text())
        finally:
            os.environ.clear()
            os.environ.update(old)


class Wiring(unittest.TestCase):
    def test_service_is_low_priority_and_wanted_by_nothing(self):
        u = UNIT.read_text()
        for line in ('Nice=19', 'IOSchedulingClass=idle', 'ExecCondition=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-search enabled'):
            self.assertIn(line, u)
        self.assertNotIn('\n[Install]', u)
        auto = (ROOT / 'hosts/linux/desktop/labwc/autostart').read_text()
        self.assertIn('systemctl --user start --no-block hearth-search.service >/dev/null 2>&1 &', auto)

    def test_menus_and_files_ask_the_index_first(self):
        self.assertIn("'hearth-search'", (SHELL / 'hearth-appmenu').read_text())
        files = (SHELL / 'hearth-files').read_text()
        self.assertIn('hits = None if hidden or glob else indexed_search(root, query.strip(), limit)', files)

    def test_appmenu_uses_index_hits(self):
        mod = load('hearth_appmenu_mod', SHELL / 'hearth-appmenu')
        mod.indexed = lambda q, limit: [{'path': '/x/a', 'name': 'a', 'folder': 'Home', 'dir': False, 'when': 1}]
        self.assertEqual(mod.find_files('ab')[0]['path'], '/x/a')
        mod.indexed = lambda q, limit: None
        with tempfile.TemporaryDirectory() as t:
            pathlib.Path(t, 'abc.txt').write_text('')
            self.assertEqual(mod.find_files('ab', base=t)[0]['name'], 'abc.txt')


if __name__ == '__main__':
    unittest.main()
