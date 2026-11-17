"""Settings › Keyboard on KWin (1.14): keyboard.json (hearth-system keyboard-set, merged) put in KWin's kxkbrc [Layout]
and kcminputrc [Keyboard] by hearth-kwin-settings keyboard-apply, at every KWin start (input-watch) and when saved.
Only what the person chose is written, nothing when nothing changed, other keys and KDE options kept, a .bak kept,
a file that can't be read left alone, and the start never stopped. No KWin here (no session bus)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
sys.path.insert(0, str(SHELL))


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


K = load(SHELL / 'hearth-kwin-settings', 'hearth_kwin_settings_kbd_test')
S = load(SHELL / 'hearth-system', 'hearth_system_kbd_test')


class Env(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        t = self.tmp.name
        self.cfg = os.path.join(t, 'home', '.config')
        os.makedirs(os.path.join(self.cfg, 'nimbus'))
        os.makedirs(os.path.join(t, 'xdg'))
        self.env = mock.patch.dict(os.environ, {'HOME': os.path.join(t, 'home'), 'XDG_CONFIG_HOME': self.cfg,
                                                'XDG_CONFIG_DIRS': os.path.join(t, 'xdg'), 'XDG_RUNTIME_DIR': t,
                                                'XDG_STATE_HOME': os.path.join(t, 'state'),
                                                'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/nonexistent'})
        self.env.start()
        K._BUS[:] = [None]

    def tearDown(self):
        self.env.stop()
        K._BUS[:] = []
        self.tmp.cleanup()

    def p(self, name):
        return os.path.join(self.cfg, name)

    def put(self, name, text):
        with open(self.p(name), 'w') as f:
            f.write(text)

    def get(self, name):
        with open(self.p(name)) as f:
            return f.read()

    def saved(self, d):
        self.put('nimbus/keyboard.json', json.dumps(d))


class Changes(unittest.TestCase):
    def test_layouts_switch_compose_mode_repeat_numlock(self):
        c = K.keyboard_changes({'layouts': [{'layout': 'de', 'variant': 'nodeadkeys'}, {'layout': 'us', 'variant': ''}],
                                'switch': 'grp:alt_shift_toggle', 'delay': 400, 'rate': 30, 'numlock': 'on',
                                'compose': 'compose:ralt', 'switchMode': 'WinClass'}, {}, {})
        self.assertEqual(c['kxkbrc']['Layout'], {'Use': 'true', 'LayoutList': 'de,us', 'VariantList': 'nodeadkeys,',
                                                 'Options': 'grp:alt_shift_toggle,compose:ralt', 'ResetOldOptions': 'true',
                                                 'SwitchMode': 'WinClass'})
        self.assertEqual(c['kcminputrc']['Keyboard'], {'RepeatDelay': '400', 'RepeatRate': '30', 'NumLock': '0'})

    def test_nothing_when_kwin_has_it_already(self):
        saved = {'layouts': [{'layout': 'fr', 'variant': ''}], 'switch': '', 'delay': 600, 'rate': 25}
        now = {'Use': 'true', 'LayoutList': 'fr', 'VariantList': ''}
        self.assertEqual(K.keyboard_changes(saved, now, {'RepeatDelay': '600', 'RepeatRate': '25'}), {})

    def test_only_what_was_picked(self):
        # Nothing chosen in Hearth: KWin's own (or your KDE ones) stay.
        self.assertEqual(K.keyboard_changes({}, {'LayoutList': 'cz', 'Options': 'caps:escape'}, {'NumLock': '1'}), {})
        # One layout: no switch shortcut; your other KDE options (caps:escape) kept.
        c = K.keyboard_changes({'layouts': [{'layout': 'us', 'variant': ''}], 'switch': 'grp:caps_toggle'},
                               {'Options': 'caps:escape,grp:alt_shift_toggle', 'ResetOldOptions': 'true'}, {})
        self.assertEqual(c['kxkbrc']['Layout']['Options'], 'caps:escape')
        # The compose key taken off: only its option goes.
        c = K.keyboard_changes({'compose': ''}, {'Options': 'caps:escape,compose:menu'}, {})
        self.assertEqual(c['kxkbrc']['Layout']['Options'], 'caps:escape')

    def test_bad_values_are_left_out(self):
        c = K.keyboard_changes({'layouts': [{'layout': 'de;rm', 'variant': ''}], 'delay': 5, 'rate': True,
                                'numlock': 'maybe', 'compose': 'evil:x', 'switchMode': 'Desktop'}, {}, {})
        self.assertEqual(c, {})


class Apply(Env):
    def test_writes_kwins_files_once_with_a_backup(self):
        self.put('kxkbrc', '# mine\n[Layout]\nDisplayNames=,\nOptions=caps:escape\n\n[Other]\nx=1\n')
        self.put('kcminputrc', '[Mouse]\ncursorTheme=Breeze\n')
        self.saved({'layouts': [{'layout': 'de', 'variant': ''}, {'layout': 'fr', 'variant': ''}],
                    'switch': 'grp:alt_shift_toggle', 'delay': 300, 'rate': 40, 'numlock': 'off'})
        r = K.keyboard_apply()
        self.assertTrue(r['ok'])
        self.assertEqual(sorted(r['written']), ['kcminputrc', 'kxkbrc'])
        x = self.get('kxkbrc')
        for line in ('# mine', 'DisplayNames=,', 'LayoutList=de,fr', 'Options=caps:escape,grp:alt_shift_toggle', '[Other]', 'x=1'):
            self.assertIn(line, x)
        i = self.get('kcminputrc')
        for line in ('cursorTheme=Breeze', 'RepeatDelay=300', 'RepeatRate=40', 'NumLock=1'):
            self.assertIn(line, i)
        self.assertTrue(os.path.exists(self.p('kxkbrc.bak')))
        # The next start: nothing changed, nothing written.
        before = (os.stat(self.p('kxkbrc')).st_mtime_ns, os.stat(self.p('kcminputrc')).st_mtime_ns)
        self.assertEqual(K.keyboard_apply()['written'], [])
        self.assertEqual(before, (os.stat(self.p('kxkbrc')).st_mtime_ns, os.stat(self.p('kcminputrc')).st_mtime_ns))

    def test_nothing_chosen_writes_nothing(self):
        self.assertEqual(K.keyboard_apply()['written'], [])
        self.assertFalse(os.path.exists(self.p('kxkbrc')))
        self.assertFalse(os.path.exists(self.p('kcminputrc')))

    def test_a_file_that_cant_be_read_is_left_and_the_start_goes_on(self):
        self.put('kxkbrc', '[Layout\nLayoutList=cz\n')
        self.saved({'layouts': [{'layout': 'de', 'variant': ''}], 'delay': 500})
        r = K.keyboard_apply()                  # never raises
        self.assertEqual(self.get('kxkbrc'), '[Layout\nLayoutList=cz\n')
        self.assertIn('ok', r)

    def test_input_watch_applies_the_keyboard_first(self):
        calls = []
        with mock.patch.object(K, 'keyboard_apply', lambda: calls.append('kbd')), \
                mock.patch.object(K, 'pointer_apply', side_effect=RuntimeError('no KWin')):
            K.input_watch()                     # no bus here: returns after applying, and a pointer error doesn't stop it
        self.assertEqual(calls, ['kbd'])


class KeyboardSet(Env):
    def setUp(self):
        super().setUp()
        self.patches = [mock.patch.object(S, 'KEYBOARD_JSON', self.p('nimbus/keyboard.json')),
                        mock.patch.object(S, 'LABWC_ENV', self.p('labwc/environment')),
                        mock.patch.object(S, 'keyboard_layouts', lambda: {'layouts': [
                            {'layout': 'us', 'variant': '', 'name': 'English (US)'},
                            {'layout': 'de', 'variant': '', 'name': 'German'}]}),
                        mock.patch.object(S, 'runtime_dir', lambda: '/nonexistent-hearth-labwc')]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        super().tearDown()

    def test_extras_are_merged_and_kept(self):
        self.saved({'layouts': [{'layout': 'us', 'variant': ''}], 'numlock': 'on', 'compose': 'compose:menu', 'other': 1})
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            S.keyboard_set(json.dumps({'layouts': [{'layout': 'de', 'variant': ''}], 'switch': '', 'delay': 500, 'rate': 30}))
        d = json.loads(self.get('nimbus/keyboard.json'))
        self.assertEqual((d['numlock'], d['compose'], d['other'], d['delay']), ('on', 'compose:menu', 1, 500))
        self.assertEqual(d['layouts'], [{'layout': 'de', 'variant': ''}])
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            S.keyboard_set(json.dumps({'layouts': [{'layout': 'de', 'variant': ''}], 'switchMode': 'Window', 'compose': ''}))
        d = json.loads(self.get('nimbus/keyboard.json'))
        self.assertEqual((d['switchMode'], d['compose'], d['numlock']), ('Window', '', 'on'))

    def test_unknown_extras_are_refused(self):
        for bad in ({'numlock': 'blink'}, {'compose': 'compose:zz'}, {'switchMode': 'Desktop'}):
            with self.assertRaises(S.Fail):
                S.keyboard_set(json.dumps(dict({'layouts': [{'layout': 'us', 'variant': ''}]}, **bad)))

    def test_status_says_kwin_and_the_extras(self):
        self.saved({'numlock': 'off', 'compose': 'bogus'})
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}):
            st = S.keyboard_status()
        self.assertTrue(st['kwin'])
        self.assertEqual((st['numlock'], st['compose'], st['switchMode']), ('off', None, None))
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            self.assertFalse(S.keyboard_status()['kwin'])

    def test_on_kwin_it_asks_kwin_to_take_it(self):
        seen = []
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}), \
                mock.patch.object(S, 'run', lambda cmd, **kw: (seen.append(cmd), (0, '{"ok": true, "kwin": true}'))[1]):
            r = S.keyboard_set(json.dumps({'layouts': [{'layout': 'us', 'variant': ''}]}))
        self.assertTrue(r['reloaded'])
        self.assertEqual(seen[-1][-1], 'keyboard-apply')


class Page(unittest.TestCase):
    def test_rows_are_kwin_only(self):
        q = (SHELL / 'SetKeyboard.qml').read_text()
        for t in ('Num Lock when Desktop Mode starts', 'Compose key', 'Remember the layout'):
            i = q.index('title: "' + t)
            self.assertIn('visible: page.kwin', q[q.rindex('SetChoice {', 0, i):i])
        self.assertIn('if (numlock !== null) req.numlock', q)

    def test_session_start_runs_it_in_the_background(self):
        a = (ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc' / 'autostart').read_text()
        self.assertIn('hearth-kwin-settings" input-watch >/dev/null 2>&1 ) &', a)


if __name__ == '__main__':
    unittest.main()
