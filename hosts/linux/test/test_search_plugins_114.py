"""Hearth 1.14's newer search plugins (hosts/linux/desktop/shell/search/*.js, docs/KDE-FEATURES-1.14.md): bookmarks and
browser history, virtual desktops and activities, kill an app, world clock, spell check and dictionary, special
characters, colours, power and session, media control, App store results, Hearth contacts, calendar events and notes;
their helper hearth-search-sources (read-only; browsers' databases from a copy), SearchSources.qml, and that they're
registered in runners.js, hearth-search and Settings › Search. The plugins run in node the way QML loads them."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import signal
import sqlite3
import subprocess
import tempfile
import unittest
from datetime import date, datetime, timezone

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parents[2]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
SEARCH = SHELL / 'search'
NODE = shutil.which('node')
NEW = ['browser', 'desktops', 'kill', 'worldclock', 'spell', 'chars', 'colours', 'power', 'media', 'store', 'contacts', 'events', 'notes']


def calls(*cs):
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False) as f:
        json.dump({'calls': list(cs)}, f)
    r = subprocess.run([NODE, str(HERE / 'search_harness.js'), str(SEARCH / 'runners.js'), f.name], capture_output=True, text=True, timeout=60)
    pathlib.Path(f.name).unlink()
    if r.returncode:
        raise AssertionError(r.stderr)
    return json.loads(r.stdout)


def helper():
    loader = importlib.machinery.SourceFileLoader('hearth_search_sources', str(SHELL / 'hearth-search-sources'))
    spec = importlib.util.spec_from_loader('hearth_search_sources', loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def titles(rows):
    return [r['title'] for r in rows]


@unittest.skipUnless(NODE, 'node is not installed')
class Plugins(unittest.TestCase):
    def test_bookmarks_and_history(self):
        data = {'bookmarks': [{'title': 'Hearth OS', 'url': 'https://hearthos.fun/', 'browser': 'Firefox'},
                              {'title': 'Arch Wiki', 'url': 'https://wiki.archlinux.org/', 'browser': 'Chromium'}],
                'history': [{'title': 'Hearth OS', 'url': 'https://hearthos.fun/', 'browser': 'Firefox', 'visits': 9},
                            {'title': 'Hearth OS docs: search', 'url': 'https://hearthos.fun/docs/search', 'browser': 'Firefox', 'visits': 3}]}
        got = calls(['browser.js', 'results', 'hearth', data], ['browser.js', 'results', 'wiki.arch', data], ['browser.js', 'results', 'h', data],
                    ['browser.js', 'results', 'hearth', None])
        self.assertEqual([(r['group'], r['action']['value']) for r in got[0]],
                         [('Bookmarks', 'https://hearthos.fun/'), ('Browser history', 'https://hearthos.fun/docs/search')])  # no copy twice
        self.assertEqual(got[1][0]['title'], 'Arch Wiki')
        self.assertEqual(got[2], [])
        self.assertEqual(got[3], [])

    def test_desktops_and_activities(self):
        data = {'desktops': [{'n': 1, 'name': 'Desktop 1', 'active': True}, {'n': 2, 'name': 'Code', 'active': False}],
                'activities': [{'id': 'default', 'name': 'Main', 'current': True}, {'id': 'a7', 'name': 'Work', 'current': False}]}
        got = calls(['desktops.js', 'results', 'desktop 2', data], ['desktops.js', 'results', 'code', data], ['desktops.js', 'results', 'work', data],
                    ['desktops.js', 'results', 'activity', data], ['desktops.js', 'results', 'desktop 1', data])
        self.assertEqual(got[0][0]['action'], {'type': 'desktop', 'value': 2})
        self.assertTrue(got[0][0]['top'])
        self.assertEqual(got[1][0]['action'], {'type': 'desktop', 'value': 2})
        self.assertEqual(got[2][0]['action'], {'type': 'activity', 'value': 'a7'})
        self.assertEqual(titles(got[3]), ["Main (you're on it)", 'Switch to the Work activity'])
        self.assertIsNone(got[4][0]['action'])                          # the one you're on isn't offered

    def test_kill_asks_first_and_never_hearths_own(self):
        procs = {'query': 'fire', 'list': [{'name': 'firefox', 'pids': [41, 42], 'cmd': '/usr/lib/firefox/firefox'}]}
        got = calls(['kill.js', 'results', 'kill fire', procs], ['kill.js', 'results', 'kill fire', None], ['kill.js', 'results', 'firefox', procs],
                    ['kill.js', 'results', 'kill quickshell', {'query': 'quickshell', 'list': [{'name': 'quickshell', 'pids': [9]}]}])
        a = got[0][0]['action']
        self.assertEqual((a['type'], a['value']), ('kill', [41, 42]))
        self.assertEqual(a['confirm']['ok'], 'Kill')
        self.assertIn('2 processes', got[0][0]['title'])
        self.assertIsNone(got[1][0]['action'])                          # still looking
        self.assertEqual(got[2], [])                                    # only with "kill"
        self.assertIsNone(got[3][0]['action'])                          # the shell itself is never offered

    def test_world_clock(self):
        now = int(datetime(2026, 11, 17, 8, 30, tzinfo=timezone.utc).timestamp() * 1000)
        z = {'offsets': {'Asia/Tokyo': 540, 'Europe/London': 0, 'America/New_York': -300, 'UTC': 0}, 'local': 'America/Chicago', 'localOffset': -360}
        got = calls(['worldclock.js', 'results', 'time in tokyo', z, now], ['worldclock.js', 'results', 'tokyo time', z, now],
                    ['worldclock.js', 'results', '3pm london to tokyo', z, now], ['worldclock.js', 'results', '15:30 utc in new york', z, now],
                    ['worldclock.js', 'results', '9am in tokyo', z, now], ['worldclock.js', 'results', 'time in sydney', None, now],
                    ['worldclock.js', 'results', 'time in atlantis', z, now], ['worldclock.js', 'results', 'tokyo', z, now])
        self.assertEqual(got[0][0]['title'], '17:30 in Tokyo')
        self.assertIn('15 hours ahead of you', got[0][0]['sub'])
        self.assertEqual(got[1][0]['title'], '17:30 in Tokyo')
        self.assertEqual(got[2][0]['title'], '00:00 in Tokyo (the next day)')
        self.assertEqual(got[3][0]['title'], '10:30 in New York')
        self.assertEqual(got[4][0]['title'], '00:00 in Tokyo (the next day)')
        self.assertIn('standard time', got[5][0]['sub'])               # no offsets: says it's the standard ones
        self.assertEqual(got[5][0]['title'], '18:30 in Sydney')
        self.assertEqual(got[6], [])
        self.assertEqual(got[7], [])

    def test_spell_and_dictionary(self):
        got = calls(['spell.js', 'results', 'spell recieve', {'recieve': {'correct': False, 'suggestions': ['receive', 'relieve']}}],
                    ['spell.js', 'results', 'spell hearth', {'hearth': {'correct': True, 'suggestions': []}}],
                    ['spell.js', 'results', 'define hearth', {}], ['spell.js', 'results', 'spell x', {'x': {'missing': True}}],
                    ['spell.js', 'results', 'receive', {}])
        self.assertEqual(got[0][0]['action'], {'type': 'copy', 'value': 'receive'})
        self.assertTrue(got[0][0]['top'])
        self.assertIn('spelled right', got[1][0]['title'])
        self.assertEqual(got[2][0]['action']['value'], 'https://en.wiktionary.org/wiki/Special:Search?search=hearth')
        self.assertIn('Checking', got[2][1]['title'])
        self.assertIn("isn't installed", got[3][0]['title'])
        self.assertEqual(got[4], [])

    def test_special_characters(self):
        got = calls(['chars.js', 'results', 'degree'], ['chars.js', 'results', 'u+00e9'], ['chars.js', 'results', '&copy;'],
                    ['chars.js', 'results', 'char arrow'], ['chars.js', 'results', 'U+1F525'], ['chars.js', 'results', 'firefox'],
                    ['chars.js', 'results', 'u+d800'])
        self.assertEqual(got[0][0]['action']['value'], '°')
        self.assertEqual(got[1][0]['action']['value'], 'é')
        self.assertEqual(got[2][0]['action']['value'], '©')
        self.assertEqual([r['action']['value'] for r in got[3]][:4], ['→', '←', '↑', '↓'])
        self.assertEqual(got[4][0]['action']['value'], '🔥')
        self.assertEqual(got[5], [])
        self.assertEqual(got[6], [])

    def test_colours(self):
        got = calls(['colours.js', 'results', '#ff8800'], ['colours.js', 'results', '#f80'], ['colours.js', 'results', 'rgb(102, 192, 244)'],
                    ['colours.js', 'results', 'hsl(32, 100%, 50%)'], ['colours.js', 'results', 'decade'], ['colours.js', 'results', 'rgb(300,0,0)'])
        self.assertEqual(titles(got[0]), ['#ff8800', 'rgb(255, 136, 0)', 'hsl(32, 100%, 50%)'])
        self.assertEqual(got[0][0]['swatch'], '#ff8800')
        self.assertEqual(got[1][0]['title'], '#ff8800')
        self.assertEqual(got[2][0]['title'], '#66c0f4')
        self.assertEqual(got[3][0]['title'], '#ff8800')
        self.assertEqual(got[4], [])                                    # a word isn't a colour without "#"
        self.assertEqual(got[5], [])

    def test_power_asks_first_where_it_closes_apps(self):
        got = calls(['power.js', 'results', 'restart'], ['power.js', 'results', 'shut down'], ['power.js', 'results', 'log out'],
                    ['power.js', 'results', 'sleep'], ['power.js', 'results', 'lock'], ['power.js', 'results', 'reboot'], ['power.js', 'results', 'firefox'])
        for g, v in ((got[0], 'restart'), (got[1], 'shutdown'), (got[2], 'logout'), (got[5], 'restart')):
            self.assertEqual(g[0]['action']['value'], v)
            self.assertIn('confirm', g[0]['action'], v + ' must ask first')
        for g, v in ((got[3], 'sleep'), (got[4], 'lock')):
            self.assertEqual(g[0]['action'], {'type': 'power', 'value': v})
        self.assertEqual(got[6], [])

    def test_media(self):
        m = {'player': 'Hearth Music', 'title': 'Hollow Pines', 'artist': 'Hearth', 'playing': True}
        got = calls(['media.js', 'results', 'pause', m], ['media.js', 'results', 'next song', m], ['media.js', 'results', 'pause', None],
                    ['media.js', 'results', 'play', dict(m, playing=False)])
        self.assertEqual(got[0][0]['action'], {'type': 'media', 'value': 'playPause'})
        self.assertEqual(got[0][0]['title'], 'Pause')
        self.assertIn('Hollow Pines', got[0][0]['sub'])
        self.assertEqual(got[1][0]['action']['value'], 'next')
        self.assertEqual(got[2], [])                                    # nothing playing: nothing
        self.assertEqual(got[3][0]['title'], 'Play')

    def test_store(self):
        st = {'query': 'krita', 'apps': [{'id': 'org.kde.krita', 'name': 'Krita', 'summary': 'Digital painting'}], 'error': ''}
        got = calls(['store.js', 'results', 'install krita', st], ['store.js', 'results', 'install krita', None], ['store.js', 'results', 'krita', st],
                    ['store.js', 'results', 'install zzz', {'query': 'zzz', 'apps': [], 'error': ''}])
        self.assertEqual(got[0][0]['action'], {'type': 'page', 'value': 'store:app-org_kde_krita'})
        self.assertIn('Looking', got[1][0]['title'])
        self.assertEqual(got[2], [])                                    # ordinary typing never goes online
        self.assertEqual(got[3][0]['action'], {'type': 'page', 'value': 'store'})

    def test_contacts_events_notes(self):
        people = [{'name': 'Sam Rivera', 'emails': ['sam@example.org'], 'phones': ['+1 555 0100'], 'org': 'Hearth'},
                  {'name': 'Alex Kim', 'emails': [], 'phones': ['555 0199'], 'org': ''}]
        now = int(datetime(2026, 11, 17, 8, 0).timestamp() * 1000)
        evs = [{'title': 'Raid night', 'date': '2026-11-17', 'allDay': False, 'start': now + 3600000 * 11, 'end': now + 3600000 * 13, 'location': 'Discord'},
               {'title': 'Old meeting', 'date': '2026-11-16', 'allDay': False, 'start': now - 86400000, 'end': now - 86000000, 'location': ''},
               {'title': 'Dentist', 'date': '2026-11-18', 'allDay': False, 'start': now + 86400000 + 3600000, 'end': now + 86400000 + 7200000, 'location': ''}]
        notes = [{'title': 'Shopping list', 'path': '/h/Documents/Notes/Shopping list.md', 'text': '# Shopping list\n- [ ] milk\n- [ ] coffee beans'},
                 {'title': 'Ideas', 'path': '/h/Documents/Notes/Ideas.md', 'text': 'Quad mech: coffee break animation'}]
        got = calls(['contacts.js', 'results', 'sam', people], ['contacts.js', 'results', '0199', people],
                    ['events.js', 'results', 'raid', evs, now], ['events.js', 'results', 'old meeting', evs, now], ['events.js', 'results', 'tomorrow', evs, now],
                    ['notes.js', 'results', 'coffee', notes], ['notes.js', 'results', 'shop', notes])
        self.assertEqual(got[0][0]['action'], {'type': 'copy', 'value': 'sam@example.org'})
        self.assertEqual(got[0][1]['action'], {'type': 'copy', 'value': '+1 555 0100'})
        self.assertEqual(got[1][0]['title'], 'Alex Kim')
        self.assertEqual(got[2][0]['title'], 'Raid night')
        self.assertIn('Today · 19:00', got[2][0]['sub'])
        self.assertEqual(got[2][0]['action'], {'type': 'exec', 'value': ['hearth-time', '--app', 'calendar']})
        self.assertEqual(got[3], [])                                    # over already
        self.assertEqual(titles(got[4]), ['Dentist'])
        self.assertEqual(titles(got[5]), ['Shopping list', 'Ideas'])
        self.assertEqual(got[6][0]['action'], {'type': 'file', 'value': '/h/Documents/Notes/Shopping list.md', 'dir': False})

    def test_all_together_and_turned_off(self):
        src = {'media': {'player': 'P', 'title': 'T', 'artist': '', 'playing': True}}
        got = calls(['runners.js', 'run', '#ff8800', {}], ['runners.js', 'run', 'restart', {}], ['runners.js', 'run', 'pause', {'src': src}],
                    ['runners.js', 'run', 'restart', {'off': {'power': True}}], ['runners.js', 'run', 'firefox', {}], ['runners.js', 'run', '>kill x', {}])
        self.assertEqual(got[0][0]['plugin'], 'colours')
        self.assertEqual(got[1][0]['plugin'], 'power')
        self.assertEqual(got[2][0]['plugin'], 'media')
        self.assertEqual(got[3], [])
        self.assertEqual(got[4], [])                                    # an ordinary word finds no plugin answer
        self.assertEqual({r['plugin'] for r in got[5]}, {'commands'})


class Registered(unittest.TestCase):
    def test_in_every_list(self):
        runners = (SEARCH / 'runners.js').read_text()
        plist = json.loads(re.search(r'var PLUGINS = (\[.*?\]);', runners, re.S).group(1))
        hs = (SHELL / 'hearth-search').read_text()
        hlist = json.loads(re.search(r"^PLUGINS = (\[.*?\])", hs, re.S | re.M).group(1).replace("'", '"'))
        page = (SHELL / 'SetSearch.qml').read_text()
        for p in NEW:
            self.assertTrue((SEARCH / (p + '.js')).exists(), p)
            self.assertIn(p, plist)
            self.assertIn('add("%s"' % p, runners)
            self.assertIn(p, hlist)
            self.assertRegex(page, r'\n\s+%s: \["' % p)
        self.assertEqual(sorted(plist), sorted(hlist))

    def test_actions_go_through_sources_and_confirm(self):
        launcher = (SHELL / 'Launcher.qml').read_text()
        self.assertIn('if (sources.handle(a)) return;', launcher)
        self.assertIn('src: sources.src', launcher)
        self.assertIn('dialog: deskDialog', (SHELL / 'shell.qml').read_text())
        q = (SHELL / 'SearchSources.qml').read_text()
        self.assertIn('if (x.confirm) {', q)
        self.assertIn('dialog.confirm(', q)
        # nothing that asks first is done without the dialog
        self.assertRegex(q, r'else console\.warn\("SearchSources: no confirm dialog')
        # opening the menu writes nothing: the helper only reads (and kill only signals)
        self.assertNotRegex((SHELL / 'hearth-search-sources').read_text(), r"open\([^)]*['\"]w")


class Helper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.h = helper()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def firefox(self):
        d = os.path.join(self.tmp, '.mozilla', 'firefox', 'abcd.default-release')
        os.makedirs(d)
        db = os.path.join(d, 'places.sqlite')
        c = sqlite3.connect(db)
        c.executescript('''PRAGMA journal_mode=WAL;
            CREATE TABLE moz_places (id INTEGER PRIMARY KEY, url TEXT, title TEXT, visit_count INTEGER, hidden INTEGER, frecency INTEGER);
            CREATE TABLE moz_bookmarks (id INTEGER PRIMARY KEY, type INTEGER, fk INTEGER, title TEXT, lastModified INTEGER);
            INSERT INTO moz_places VALUES (1, 'https://hearthos.fun/', 'Hearth OS', 9, 0, 900), (2, 'https://example.org/', 'Example', 1, 0, 10),
                                          (3, 'place:sort=8', 'Recent', 0, 0, 0);
            INSERT INTO moz_bookmarks VALUES (1, 1, 1, 'Hearth OS', 5), (2, 2, NULL, 'Folder', 4), (3, 1, 3, 'Smart', 3);''')
        c.commit()
        c.close()
        return db

    def test_browser_reads_a_copy_even_while_the_browser_holds_its_lock(self):
        db = self.firefox()
        chrome = os.path.join(self.tmp, '.config', 'chromium', 'Default')
        os.makedirs(chrome)
        open(os.path.join(chrome, 'Preferences'), 'w').write('{}')
        json.dump({'roots': {'bookmark_bar': {'type': 'folder', 'children': [{'type': 'url', 'name': 'Arch Wiki', 'url': 'https://wiki.archlinux.org/'}]}}},
                  open(os.path.join(chrome, 'Bookmarks'), 'w'))
        before = open(db, 'rb').read()
        live = sqlite3.connect(db, timeout=0)
        live.execute('BEGIN EXCLUSIVE')                     # the browser writing: nobody else may open it
        try:
            r = self.h.browser(self.tmp)
        finally:
            live.rollback()
            live.close()
        self.assertEqual(r['errors'], [])
        self.assertEqual([(b['title'], b['browser']) for b in r['bookmarks']], [('Hearth OS', 'Firefox'), ('Arch Wiki', 'Chromium')])
        self.assertEqual([h['url'] for h in r['history']], ['https://hearthos.fun/', 'https://example.org/'])
        self.assertEqual(open(db, 'rb').read(), before, 'the live file must never change')
        self.assertFalse([f for f in os.listdir(os.path.dirname(db)) if f.endswith('.sqlite') and f != 'places.sqlite'])

    def test_procs_and_kill(self):
        p = subprocess.Popen(['sleep', '300'])
        try:
            r = self.h.procs('sleep')
            self.assertTrue(any(p.pid in g['pids'] for g in r['list']))
            self.assertFalse(any(g['name'] in self.h.KEEP for g in self.h.procs('')['list']))
            sent = []
            k = self.h.kill([str(p.pid), '1', 'abc', str(os.getpid())], send=lambda pid, sig: sent.append((pid, sig)))
            self.assertEqual(k['killed'], [p.pid])
            self.assertEqual(sent, [(p.pid, signal.SIGTERM)])
            self.assertIn('1', [str(x) for x in k['refused']])
            self.assertIn(os.getpid(), k['refused'])
            self.assertEqual(self.h.kill([str(p.pid)])['killed'], [p.pid])
            self.assertEqual(p.wait(10), -signal.SIGTERM)
        finally:
            if p.poll() is None:
                p.kill()

    def test_spell_parses_hunspell(self):
        def fake(out_):
            return lambda argv, **kw: subprocess.CompletedProcess(argv, 0, out_, '')
        self.assertEqual(self.h.spell('recieve', run=fake('@(#) Hunspell\n& recieve 3 0: receive, relieve, reprieve\n\n'))['suggestions'],
                         ['receive', 'relieve', 'reprieve'])
        self.assertTrue(self.h.spell('hearth', run=fake('@(#) Hunspell\n*\n\n'))['correct'])
        self.assertEqual(self.h.spell('zzxq', run=fake('@(#) Hunspell\n# zzxq 0\n\n'))['suggestions'], [])
        self.assertEqual(self.h.spell('rm -rf', run=fake('*'))['suggestions'], [])        # only a word goes to hunspell
        if shutil.which('hunspell'):
            r = self.h.spell('recieve')
            self.assertTrue(r.get('missing') or 'receive' in r['suggestions'], r)

    def test_zones_follow_summer_time(self):
        w = self.h.zones(now=datetime(2026, 7, 1, 12, tzinfo=timezone.utc))
        self.assertEqual(w['offsets']['Europe/London'], 60)
        self.assertEqual(w['offsets']['America/New_York'], -240)
        self.assertEqual(w['offsets']['UTC'], 0)
        self.assertEqual(self.h.zones(now=datetime(2026, 12, 1, 12, tzinfo=timezone.utc))['offsets']['Europe/London'], 0)
        self.assertIsInstance(w['localOffset'], int)

    def test_pim(self):
        os.environ['HEARTH_CONTACTS_DIR'] = os.path.join(self.tmp, 'contacts')
        os.environ['HEARTH_TIME_HOME'] = os.path.join(self.tmp, 'time')
        try:
            book = os.path.join(self.tmp, 'contacts', 'books', 'local')
            os.makedirs(book)
            open(os.path.join(book, 'a.vcf'), 'w').write('BEGIN:VCARD\r\nVERSION:3.0\r\nFN:Sam Rivera\r\nEMAIL;TYPE=home:sam@exam\r\n ple.org\r\n'
                                                         'TEL;TYPE=cell:+1 555 0100\r\nORG:Hearth;Games\r\nEND:VCARD\r\n')
            cal = os.path.join(self.tmp, 'time', 'data', 'hearth-calendar')
            os.makedirs(cal)
            t = date.today().isoformat()
            json.dump({'events': [{'id': '1', 'title': 'Raid night', 'start': t + 'T23:00:00', 'end': t + 'T23:30:00', 'location': 'Discord'},
                                  {'id': '2', 'title': 'Weekly', 'start': t + 'T09:00:00', 'end': t + 'T10:00:00', 'rrule': 'FREQ=WEEKLY'}]},
                      open(os.path.join(cal, 'calendar.json'), 'w'))
            nd = os.path.join(self.tmp, 'time', 'Documents', 'Notes')
            os.makedirs(nd)
            open(os.path.join(nd, 'Shopping list.md'), 'w').write('# Shopping list\n- [ ] milk\n')
            open(os.path.join(nd, '.pinned'), 'w').write('Shopping list.md\n')
            old = shutil.which
            self.h.shutil.which = lambda n: None if n == 'hearth-time' else old(n)
            try:
                r = self.h.pim()
            finally:
                self.h.shutil.which = old
            self.assertEqual(r['contacts'], [{'name': 'Sam Rivera', 'emails': ['sam@example.org'], 'phones': ['+1 555 0100'], 'org': 'Hearth'}])
            self.assertEqual([e['title'] for e in r['events']], ['Raid night'])
            self.assertEqual([(n['title'], os.path.basename(n['path'])) for n in r['notes']], [('Shopping list', 'Shopping list.md')])
        finally:
            os.environ.pop('HEARTH_CONTACTS_DIR', None)
            os.environ.pop('HEARTH_TIME_HOME', None)


if __name__ == '__main__':
    unittest.main()
