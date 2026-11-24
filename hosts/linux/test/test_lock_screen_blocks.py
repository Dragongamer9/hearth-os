"""The lock screen made of blocks (docs/mockups/checklists/lock-screen-blocks.md).

What must always hold: the password block is always there, once, inside the screen and big enough; nothing on the lock
screen can open an app, a file or a setting (every press dropped, only display blocks); the unlock path stays Lock.qml's
own (PAM) and the blocks are loaded so that if they fail the plain lock screen shows; the layout is kept through the
settings helper (merge-on-write, a .bak kept)."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
HELPER = SHELL / 'hearth-lockscreen'


def load_helper():
    loader = importlib.machinery.SourceFileLoader('hearth_lockscreen', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_lockscreen', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


LS = load_helper()


class Sanitize(unittest.TestCase):
    def test_password_always_there_once_and_last(self):
        s, p = LS.sanitize({'blocks': [{'block': 'clock', 'x': 0, 'y': 0, 'w': 4, 'h': 2}]})
        self.assertEqual([b['block'] for b in s['blocks']].count('password'), 1)
        self.assertEqual(s['blocks'][-1]['block'], 'password', 'drawn last: always on top')
        self.assertTrue(any('always there' in x for x in p))
        s, p = LS.sanitize({'blocks': [{'block': 'password', 'x': 1, 'y': 1, 'w': 6, 'h': 5}, {'block': 'password'}, {'block': 'clock'}]})
        self.assertEqual([b['block'] for b in s['blocks']], ['clock', 'password'])

    def test_password_big_enough_and_inside(self):
        s, _ = LS.sanitize({'grid': {'cols': 24, 'rows': 14}, 'blocks': [{'block': 'password', 'x': 40, 'y': 40, 'w': 1, 'h': 1}]})
        pw = s['blocks'][-1]
        self.assertGreaterEqual(pw['w'], 5)
        self.assertGreaterEqual(pw['h'], 4)
        self.assertLessEqual(pw['x'] + pw['w'], 24)
        self.assertLessEqual(pw['y'] + pw['h'], 14)

    def test_nothing_opens_anything(self):
        s, p = LS.sanitize({'blocks': [
            {'block': 'weather', 'press': {'action': 'open-app', 'target': 'firefox'}},
            {'block': 'hero'}, {'block': 'tilerow', 'items': ['files']}, {'block': 'guide'}, {'block': 'dock'},
            {'block': 'icons', 'items': ['home', 'bin', 'app:firefox', '../etc']},
            {'block': 'own:printer-camera'}, {'block': 'own:../../x'},
            {'block': 'clock', 'style': {'sound': 'pop', 'focus': 'grow', 'aboveWindows': True, 'colour': 'red; x', 'corners': 18}}]})
        names = [b['block'] for b in s['blocks']]
        self.assertEqual(names, ['weather', 'icons', 'own:printer-camera', 'clock', 'password'])
        for b in s['blocks']:
            self.assertNotIn('press', b)
        self.assertEqual(s['blocks'][1]['items'], ['home', 'bin'])
        self.assertEqual(s['blocks'][3]['style'], {'corners': 18})
        self.assertTrue(any('nothing on the lock screen opens anything' in x for x in p))

    def test_privacy_defaults_and_values(self):
        s, _ = LS.sanitize({'blocks': []})
        self.assertEqual(s['privacy'], {'notifications': 'names', 'media': True})
        s, _ = LS.sanitize({'blocks': [], 'privacy': {'notifications': 'count', 'media': False}})
        self.assertEqual(s['privacy'], {'notifications': 'count', 'media': False})
        s, _ = LS.sanitize({'blocks': [], 'privacy': {'notifications': 'everything'}})
        self.assertEqual(s['privacy']['notifications'], 'names')

    def test_not_a_layout(self):
        for bad in (None, [], 'x', 3):
            s, p = LS.sanitize(bad)
            self.assertEqual(s['blocks'][-1]['block'], 'password')

    def test_check_command(self):
        out = subprocess.run([sys.executable, str(HELPER), 'check', '{"blocks":[{"block":"weather","press":{"action":"files"}}]}'],
                             capture_output=True, text=True)
        o = json.loads(out.stdout)
        self.assertTrue(o['ok'])
        self.assertEqual(o['screen']['blocks'][-1]['block'], 'password')
        out = subprocess.run([sys.executable, str(HELPER), 'check', 'not json'], capture_output=True, text=True)
        self.assertFalse(json.loads(out.stdout)['ok'])


class ReadyMade(unittest.TestCase):
    def test_every_ready_desktop_has_a_matching_lock_screen(self):
        desks = {f.stem for f in (SHELL / 'screens').glob('*.json')}
        locks = {f.stem for f in (SHELL / 'lock-screens').glob('*.json')}
        self.assertTrue(desks <= locks, 'missing: %s' % (desks - locks))
        self.assertIn('hearth', locks)
        for f in sorted((SHELL / 'lock-screens').glob('*.json')):
            raw = json.loads(f.read_text())
            s, p = LS.sanitize(raw)
            self.assertEqual(p, [], '%s: %s' % (f.name, p))
            self.assertEqual(s['blocks'][-1]['block'], 'password', f.name)
            pw = s['blocks'][-1]
            for b in s['blocks'][:-1]:
                overlap = b['x'] < pw['x'] + pw['w'] and pw['x'] < b['x'] + b['w'] and b['y'] < pw['y'] + pw['h'] and pw['y'] < b['y'] + b['h']
                self.assertFalse(overlap, '%s: %s covers the password block' % (f.name, b['id']))

    def test_board_1_and_2(self):
        h = LS.ready('hearth')
        self.assertEqual([b['block'] for b in h['blocks']], ['clock', 'date', 'weather', 'media', 'notifications', 'downloads', 'battery', 'password'])
        r = LS.ready('retro')
        self.assertEqual(r['blocks'][0]['value'], 'Hearth is locked')
        self.assertEqual(r['blocks'][1]['items'], ['home', 'bin'])
        self.assertEqual(r['blocks'][-1]['shows'], 'window')
        self.assertEqual(LS.ready('../../etc/passwd')['blocks'], h['blocks'])


class Keeping(unittest.TestCase):
    def setUp(self):
        self.home = tempfile.mkdtemp()
        self.env = dict(os.environ, HOME=self.home, XDG_RUNTIME_DIR=self.home, XDG_STATE_HOME=os.path.join(self.home, 'state'))
        self.file = os.path.join(self.home, '.local', 'share', 'nimbus', 'lock-screen.json')

    def run_h(self, *a):
        r = subprocess.run([sys.executable, str(HELPER)] + list(a), capture_output=True, text=True, env=self.env)
        return json.loads(r.stdout.strip().splitlines()[-1])

    def test_keep_merges_and_keeps_a_bak(self):
        os.makedirs(os.path.dirname(self.file))
        json.dump({'other': 1, 'on': False}, open(self.file, 'w'))
        o = self.run_h('keep', json.dumps({'screen': {'blocks': [{'block': 'clock'}, {'block': 'password'}]}, 'match': False}))
        self.assertTrue(o['ok'], o)
        d = json.load(open(self.file))
        self.assertEqual(d['other'], 1, 'merge on write: other keys stay')
        self.assertTrue(d['on'])
        self.assertEqual(d['screen']['blocks'][-1]['block'], 'password')
        self.assertTrue(os.path.exists(self.file + '.bak'))

    def test_never_over_a_file_that_cant_be_read(self):
        os.makedirs(os.path.dirname(self.file))
        good = {'on': True, 'match': True, 'mine': 'kept'}
        json.dump(good, open(self.file, 'w'))
        self.run_h('plain')                                  # a good write: makes the .bak
        open(self.file, 'w').write('{"on": tru')            # damaged
        self.run_h('match')
        d = json.load(open(self.file))
        self.assertEqual(d.get('mine'), 'kept', 'the damaged file was put back from its .bak, never replaced by defaults')

    def test_plain_and_match(self):
        self.run_h('match')
        self.assertEqual(json.load(open(self.file)), {'on': True, 'match': True})
        self.run_h('plain')
        self.assertEqual(json.load(open(self.file)), {'on': False, 'match': True})

    def test_try_is_made_safe(self):
        o = self.run_h('try', json.dumps({'blocks': [{'block': 'dock', 'press': {'action': 'files'}}]}))
        self.assertTrue(o['ok'])
        t = json.load(open(os.path.join(self.home, 'hearth-lock-try.json')))
        self.assertEqual([b['block'] for b in t['blocks']], ['password'])


class WhatTheBlocksRead(unittest.TestCase):
    def test_downloads_from_steam_manifests(self):
        home = tempfile.mkdtemp()
        lib = os.path.join(home, '.local', 'share', 'Steam', 'steamapps')
        os.makedirs(lib)
        open(os.path.join(lib, 'appmanifest_10.acf'), 'w').write('"AppState"\n{\n "name" "Ember Trails"\n "BytesToDownload" "1000"\n "BytesDownloaded" "620"\n}\n')
        open(os.path.join(lib, 'appmanifest_20.acf'), 'w').write('"AppState"\n{\n "name" "Done Game"\n "BytesToDownload" "1000"\n "BytesDownloaded" "1000"\n}\n')
        old = os.environ.get('HOME')
        os.environ['HOME'] = home
        try:
            self.assertEqual(LS.downloads(), {'items': [{'name': 'Ember Trails', 'percent': 62}]})
        finally:
            os.environ['HOME'] = old

    def test_friends_online_only(self):
        home = tempfile.mkdtemp()
        steam = os.path.join(home, '.local', 'share', 'Steam')
        os.makedirs(os.path.join(steam, 'steamapps'))
        cfg = os.path.join(steam, 'userdata', '111', 'config')
        os.makedirs(cfg)
        open(os.path.join(cfg, 'localconfig.vdf'), 'w').write('"UserLocalConfigStore"\n{\n "friends"\n {\n  "222"\n  {\n   "name" "Dad"\n  }\n  "333"\n  {\n   "name" "Sam"\n  }\n }\n}\n')
        xml = {76561197960265728 + 222: '<onlineState>in-game</onlineState><gameName><![CDATA[Trains 2010]]></gameName>',
               76561197960265728 + 333: '<onlineState>offline</onlineState>'}
        old = dict(os.environ)
        os.environ.update(HOME=home, XDG_CACHE_HOME=os.path.join(home, 'cache'))
        try:
            self.assertEqual(LS.friends(fetch=lambda i: xml[i]), {'online': [{'name': 'Dad', 'game': 'Trains 2010'}]})
        finally:
            os.environ.clear()
            os.environ.update(old)


class TheLockStaysSafe(unittest.TestCase):
    lock = (SHELL / 'Lock.qml').read_text()
    blocks = (SHELL / 'LockBlocks.qml').read_text()
    pw = (SHELL / 'LockPassword.qml').read_text()

    def code(self, t):
        return '\n'.join(l for l in t.splitlines() if not l.lstrip().startswith('//'))

    def test_blocks_are_loaded_so_a_failure_keeps_the_plain_lock(self):
        self.assertIn('source: "LockBlocks.qml"', self.lock)
        self.assertIn('source: "LockPassword.qml"', self.lock)
        self.assertIsNone(re.search(r'^\s*Lock(Blocks|Password)\s*\{', self.lock, re.M), 'never instantiated directly')
        self.assertIn('readonly property bool blocksOn: blocksReady && pwLoader.status === Loader.Ready && !!pwLoader.item', self.lock)
        # the plain lock's own password box shows whenever the blocks aren't on
        self.assertIn('visible: !surface.blocksOn\n                    Layout.alignment: Qt.AlignHCenter; Layout.topMargin: 24', self.lock)

    def test_unlocking_is_still_pam_in_lock_qml(self):
        self.assertEqual(self.lock.count('PamContext {'), 2)                 # the password and the fingerprint
        self.assertIn('if (result === PamResult.Success) { root.locked = false; root.typed = ""; }', self.lock)
        self.assertIn('item.unlock.connect(() => root.tryUnlock());', self.lock)
        for t in (self.blocks, self.pw):
            c = self.code(t)
            self.assertNotIn('locked = false', c)
            self.assertNotIn('Pam', c)

    def test_blocks_only_draw(self):
        c = self.code(self.blocks)
        for bad in ('TextInput', 'TextEdit', 'Keys.', 'focus: true', 'forceActiveFocus', 'xdg-open', 'open-app', 'execute()', 'scr.act'):
            self.assertNotIn(bad, c, bad)
        # one MouseArea: the media block's previous / pause / next (it steers what's playing, nothing more)
        self.assertEqual(c.count('MouseArea {'), 1)
        self.assertIn('p.previous()', c)
        # the only thing it runs by itself besides reading: removing the editor's Try it file
        self.assertEqual(re.findall(r'execDetached\((\[[^\]]*\])', c), ['["rm", "-f", tryFile.path]'])
        # your AI's blocks have nothing to press on the lock screen
        self.assertIn('enabled: false\n            OwnBlock', self.blocks)

    def test_password_block_only_signals(self):
        c = self.code(self.pw)
        self.assertNotIn('Process', c)
        self.assertNotIn('execDetached', c)
        self.assertEqual(set(re.findall(r'signal (\w+)\(\)', c)), {'unlock', 'keys', 'shutDown'})

    def test_the_editor_never_removes_the_password_block(self):
        e = (SHELL / 'ScreenEditor.qml').read_text()
        self.assertIn('if (lockMode && cur.block === "password") { error = "The password block is always there', e)
        self.assertIn('function coversPw(b, i)', e)
        self.assertIn('Pill { visible: !!ed.cur && ed.cur.block !== "password"; text: "Remove this block"', e)


if __name__ == '__main__':
    unittest.main()
