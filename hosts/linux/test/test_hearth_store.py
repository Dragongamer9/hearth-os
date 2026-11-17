"""Settings › App store's helper (hosts/linux/desktop/shell/hearth-store, like KDE Discover): Flathub's lists, search
and app pages (plain-text descriptions, screenshots, ODRS ratings and reviews) from saved answers, opening offline with
what it saw last (said so), Flatpak updates and sources with a stand-in flatpak, only real app ids and only https
.flatpakrepo sources (never unverified), and Flathub for everyone never removed."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap
import unittest

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
HELPER = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell' / 'hearth-store'

APPSTREAM = {'name': 'Krita', 'summary': 'Digital painting', 'developer_name': 'KDE', 'icon': 'https://dl.flathub.org/icon.png',
             'description': '<p>Paint &amp; draw.</p><ul><li>Brushes</li><li>Layers</li></ul>', 'project_license': 'GPL-3.0',
             'urls': {'homepage': 'https://krita.org'}, 'metadata': {'flathub::verification::verified': 'true'},
             'screenshots': [{'caption': 'Main', 'sizes': {'624x351': 'https://x/s624.png', '1248x702': 'https://x/s1248.png', '2496x1404': 'https://x/big.png'}}]}
ANSWERS = {
    'collection/popular': {'hits': [{'app_id': 'org.kde.krita', 'name': 'Krita', 'summary': 'Paint', 'icon': 'i'}], 'totalPages': 1},
    'collection/trending': {'hits': [{'app_id': 'com.valvesoftware.Steam', 'name': 'Steam', 'summary': 'Games'}]},
    'collection/recently-added': {'hits': []},
    'collection/category/Game': {'hits': [{'app_id': 'net.lutris.Lutris', 'name': 'Lutris', 'summary': 'Games'}], 'totalPages': 3},
    'search': {'hits': [{'app_id': 'org.gimp.GIMP', 'name': 'GIMP', 'summary': 'Images'}]},
    'appstream/org.kde.krita': APPSTREAM,
    'summary/org.kde.krita': {'download_size': 123456789},
    'ratings/org.kde.krita': {'star0': 0, 'star1': 1, 'star2': 0, 'star3': 1, 'star4': 2, 'star5': 6, 'total': 10},
    'fetch': [{'reviewer_name': 'Sam', 'rating': 80, 'summary': 'Great', 'description': 'Love it', 'date_created': 1790000000}],
}


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_store_mod', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_store_mod', loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


class Flathub(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.mod = load()
        self.mod.CACHE = self.tmp.name
        self.urls = []

    def tearDown(self):
        self.tmp.cleanup()

    def web(self, url, body=None):
        self.urls.append((url, body))
        for k, v in ANSWERS.items():
            if url.split('?')[0].endswith('/' + k):
                return v
        raise OSError('404 ' + url)

    def offline(self, url, body=None):
        raise OSError('no network')

    def test_home_and_category(self):
        h = self.mod.home(getter=self.web)
        self.assertEqual([a['id'] for a in h['popular']], ['org.kde.krita'])
        self.assertEqual(h['trending'][0]['name'], 'Steam')
        self.assertEqual(len(h['categories']), 10)
        self.assertFalse(h['cached'])
        c = self.mod.category('Game', 1, getter=self.web)
        self.assertEqual((c['apps'][0]['id'], c['more']), ('net.lutris.Lutris', True))
        with self.assertRaises(self.mod.Fail):
            self.mod.category('../../etc', getter=self.web)

    def test_search_posts_the_words(self):
        r = self.mod.search('gimp editor', getter=self.web)
        self.assertEqual(r['apps'][0]['id'], 'org.gimp.GIMP')
        self.assertEqual(self.urls[-1][1], {'query': 'gimp editor', 'filters': []})
        self.assertEqual(self.mod.search('g', getter=self.web)['apps'], [])

    def test_app_page(self):
        a = self.mod.app('org.kde.krita', getter=self.web)
        self.assertEqual(a['description'], 'Paint & draw.\n• Brushes\n• Layers')
        self.assertEqual(a['screenshots'], [{'url': 'https://x/s1248.png', 'caption': 'Main'}])
        self.assertEqual(a['ratings'], {'total': 10, 'average': 4.2})
        self.assertEqual(a['reviews'][0]['stars'], 4)
        self.assertEqual(a['size'], 123456789)
        self.assertTrue(a['verified'])
        # ODRS gets the same anonymous hash from everybody
        fetch = [b for u, b in self.urls if u.endswith('/fetch')][0]
        self.assertEqual(fetch['user_hash'], '0' * 40)
        for bad in ('krita', '../x.y.z', 'org.kde.krita; rm'):
            with self.assertRaises(self.mod.Fail):
                self.mod.app(bad, getter=self.web)

    def test_offline_uses_what_it_saw_last(self):
        self.mod.home(getter=self.web)
        self.mod.TTL = -1                       # everything stale
        h = self.mod.home(getter=self.offline)
        self.assertTrue(h['cached'])
        self.assertEqual(h['popular'][0]['id'], 'org.kde.krita')

    def test_offline_with_nothing_saved_says_so(self):
        with self.assertRaises(self.mod.Fail) as e:
            self.mod.home(getter=self.offline)
        self.assertIn("can't be reached", str(e.exception))


# A stand-in flatpak that knows which installation has which remote: installing from a remote that installation
# doesn't have fails the way flatpak does, so a store that asks the wrong installation can't pass.
FAKE_FLATPAK = textwrap.dedent(r'''
    #!/usr/bin/python3
    import json, os, sys
    a = sys.argv[1:]
    open(os.environ['FAKE_LOG'], 'a').write(' '.join(a) + '\n')
    st = json.load(open(os.environ['FAKE_STATE']))
    inst = next((x[2:] for x in a if x in ('--user', '--system')), 'system')
    def save(): json.dump(st, open(os.environ['FAKE_STATE'], 'w'))
    def fail(m): print('error: ' + m, file=sys.stderr); sys.exit(1)
    if a[0] == 'remotes':
        for name, title, url, opts in st['remotes'][inst]:
            if '--columns=name' in a: print(name)
            else: print('\t'.join([name, title, url, opts]))
    elif a[0] == 'remote-ls':
        for row in st['updates'][inst]: print('\t'.join(row))
    elif a[0] == 'remote-add':
        rest = [x for x in a[1:] if not x.startswith('--')]
        if rest[0] not in [r[0] for r in st['remotes'][inst]]:
            st['remotes'][inst].append([rest[0], rest[0], rest[1], inst]); save()
    elif a[0] == 'remote-delete':
        st['remotes'][inst] = [r for r in st['remotes'][inst] if r[0] != a[-1]]; save()
    elif a[0] in ('install', 'update'):
        if a[0] == 'install':
            remote = [x for x in a[1:] if not x.startswith('-')][0]
            if remote not in [r[0] for r in st['remotes'][inst]]:
                fail("No remote refs found for '%s'" % remote)
        print('Working 40%'); print('Working 100%')
    elif a[0] == 'info':
        sys.exit(0 if a[-1] in st.get('installed', {}).get(inst, []) else 1)
''').lstrip()
STOCK = {'remotes': {'user': [['myrepo', 'My repo', 'https://repo.example/', '']],
                     'system': [['flathub', 'Flathub', 'https://dl.flathub.org/repo/', 'system'], ['old', 'Old', 'https://old/', 'system,disabled']]},
         'updates': {'user': [['org.kde.krita', 'Krita', '5.2', 'stable', 'flathub', '80 MB']],
                     'system': [['org.freedesktop.Platform', 'Freedesktop Platform', '24.08', '24.08', 'flathub', '200 MB']]}}


class Flatpak(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = pathlib.Path(self.tmp.name)
        (t / 'flatpak').write_text(FAKE_FLATPAK)
        (t / 'flatpak').chmod(0o755)
        self.log = t / 'log'
        self.state = t / 'state.json'
        self.state.write_text(json.dumps(STOCK))
        self.env = dict(os.environ, HEARTH_FLATPAK=str(t / 'flatpak'), FAKE_LOG=str(self.log), FAKE_STATE=str(self.state),
                        XDG_CACHE_HOME=str(t / 'cache'))

    def tearDown(self):
        self.tmp.cleanup()

    def run_h(self, *args):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), capture_output=True, text=True, env=self.env, timeout=30)
        lines = r.stdout.strip().splitlines()
        last = lines[-1]
        return r.returncode, (last if last.startswith('error:') else json.loads(last)), lines

    def test_updates_and_links(self):
        code, u, _ = self.run_h('updates')
        self.assertEqual([(x['id'], x['where'], x['size']) for x in u['flatpaks']],
                         [('org.kde.krita', 'yours', '80 MB'), ('org.freedesktop.Platform', 'system', '200 MB')])
        self.assertEqual((u['system']['page'], u['firmware']['page']), ('updates', 'firmware'))

    def test_update_streams_progress(self):
        code, r, lines = self.run_h('update', 'org.kde.krita')
        self.assertEqual(code, 0)
        self.assertIn('progress: 40 Updating org.kde.krita', lines)
        self.assertIn('update -y --noninteractive --user org.kde.krita', self.log.read_text())
        self.assertIn('update -y --noninteractive --system org.kde.krita', self.log.read_text())

    def test_install_on_a_stock_system_uses_the_systems_flathub(self):
        # Hearth OS keeps Flathub in the system installation only (hearth-flatpak-setup), like Bazaar uses it
        code, r, lines = self.run_h('install', 'org.kde.krita')
        self.assertEqual((code, r['installed'], r['where']), (0, 'org.kde.krita', 'system'), lines)
        self.assertIn('install -y --noninteractive --system flathub org.kde.krita', self.log.read_text())
        self.assertEqual(self.run_h('install', '--help')[0], 1)

    def test_install_just_for_you_adds_your_flathub_first(self):
        code, r, lines = self.run_h('install', 'org.kde.krita', '--user')
        self.assertEqual((code, r['where']), (0, 'user'), lines)
        log = self.log.read_text()
        self.assertIn('remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo', log)
        self.assertLess(log.index('remote-add --user'), log.index('install -y --noninteractive --user flathub'))

    def test_the_fake_rejects_an_unknown_remote(self):
        st = json.loads(self.state.read_text())
        st['remotes']['system'] = []
        self.state.write_text(json.dumps(st))
        code, r, _ = self.run_h('install', 'org.kde.krita')          # no system Flathub: yours, added first
        self.assertEqual((code, r['where']), (0, 'user'))
        r = subprocess.run([self.env['HEARTH_FLATPAK'], 'install', '-y', '--system', 'flathub', 'x.y.z'], env=self.env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 1)
        self.assertIn('No remote refs', r.stderr)
        code, msg, _ = self.run_h('install', 'org.kde.krita', '--system')
        self.assertEqual(code, 1)
        self.assertIn("isn't set up for everyone yet", msg)

    def test_sources(self):
        code, s, _ = self.run_h('sources')
        self.assertEqual([(x['name'], x['where'], x['enabled']) for x in s['sources']],
                         [('myrepo', 'yours', True), ('flathub', 'system', True), ('old', 'system', False)])
        self.assertEqual(self.run_h('source-add', 'x', 'http://repo/x.flatpakrepo')[1][:6], 'error:')
        self.assertEqual(self.run_h('source-add', 'x', 'https://repo/x')[1][:6], 'error:')        # no key: refused
        self.assertEqual(self.run_h('source-add', '-x', 'https://repo/x.flatpakrepo')[1][:6], 'error:')
        self.assertEqual(self.run_h('source-add', 'kde', 'https://cdn.kde.org/flatpak/kde.flatpakrepo')[0], 0)
        self.assertIn('remote-add --user --if-not-exists kde https://cdn.kde.org/flatpak/kde.flatpakrepo', self.log.read_text())
        self.assertNotIn('no-gpg-verify', HELPER.read_text().replace('never --no-gpg-verify', ''))
        self.assertEqual(self.run_h('source-remove', 'flathub', '--system')[1][:6], 'error:')
        self.assertEqual(self.run_h('source-remove', 'myrepo')[0], 0)


if __name__ == '__main__':
    unittest.main()
