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


FAKE_FLATPAK = textwrap.dedent(r'''
    #!/bin/sh
    echo "$*" >> "$FAKE_LOG"
    case "$1 $2" in
      "remote-ls --user") printf 'org.kde.krita\tKrita\t5.2\tstable\tflathub\t80 MB\n' ;;
      "remote-ls --system") printf 'org.freedesktop.Platform\tFreedesktop Platform\t24.08\t24.08\tflathub\t200 MB\n' ;;
      "remotes --user") printf 'myrepo\tMy repo\thttps://repo.example/\t\n' ;;
      "remotes --system") printf 'flathub\tFlathub\thttps://dl.flathub.org/repo/\tsystem\nold\tOld\thttps://old/\tsystem,disabled\n' ;;
      "update -y") echo "Updating 50%"; echo "Done 100%" ;;
      "install -y") echo "Installing 40%"; echo "Installing 100%" ;;
      *) : ;;
    esac
''').lstrip()


class Flatpak(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = pathlib.Path(self.tmp.name)
        (t / 'flatpak').write_text(FAKE_FLATPAK)
        (t / 'flatpak').chmod(0o755)
        self.log = t / 'log'
        self.env = dict(os.environ, HEARTH_FLATPAK=str(t / 'flatpak'), FAKE_LOG=str(self.log), XDG_CACHE_HOME=str(t / 'cache'))

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
        self.assertIn('progress: 50 Updating org.kde.krita', lines)
        self.assertIn('update -y --noninteractive --user org.kde.krita', self.log.read_text())

    def test_install_yours_by_default(self):
        code, r, lines = self.run_h('install', 'org.kde.krita')
        self.assertEqual((code, r['installed']), (0, 'org.kde.krita'))
        self.assertIn('install -y --noninteractive --user flathub org.kde.krita', self.log.read_text())
        self.assertEqual(self.run_h('install', '--help')[0], 1)

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
