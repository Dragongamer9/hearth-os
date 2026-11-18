"""Firehold on Plasma (a test, docs/FIREHOLD.md "Firehold on Plasma"): its helper's rules, checked without Plasma.

  * The Desktop Mode choice is opt-in (no line: KDE Plasma), saved only when picked, only its own line, never by
    itself; the session clears Plasma's shell variable so plain Plasma never gets Firehold's shell.
  * Settings (firehold.json) are written a key at a time through hearth_settings_io and not at all when unchanged.
  * The red flag on performance: no Timer and no animation in Firehold's QML.
  * The KWin script only uses KWin's public script API and never comes from archive/kwin.
"""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
FP = ROOT / 'hosts/linux/desktop/firehold-plasma'
HELPER = FP / 'hearth-firehold-plasma'
SESSION = ROOT / 'hosts/linux/session/nimbus-session'


def load():
    loader = importlib.machinery.SourceFileLoader('hearth_firehold_plasma', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_firehold_plasma', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class Helper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.env = dict(os.environ)
        os.environ.update({'HOME': self.tmp.name, 'XDG_CONFIG_HOME': os.path.join(self.tmp.name, '.config'),
                           'XDG_STATE_HOME': os.path.join(self.tmp.name, '.state'),
                           'XDG_RUNTIME_DIR': os.path.join(self.tmp.name, 'run')})
        os.makedirs(os.environ['XDG_RUNTIME_DIR'])
        os.environ.pop('HEARTH_FIREHOLD_PLASMA', None)
        self.h = load()

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.env)
        self.tmp.cleanup()

    def conf(self):
        return pathlib.Path(self.h.session_conf())

    def test_the_choice_is_opt_in_and_only_its_line(self):
        self.assertEqual(self.h.choice(), 'plasma')
        self.h.choose('plasma')
        self.assertFalse(self.conf().exists(), 'choosing what it already is writes nothing')
        self.conf().parent.mkdir(parents=True)
        self.conf().write_text('# mine\nHEARTH_DESKTOP_WM=kwin\nGAMESCOPE_ARGS="-W 1920"\n')
        self.h.choose('firehold')
        text = self.conf().read_text()
        self.assertIn('HEARTH_DESKTOP_WM=kwin\n', text)
        self.assertIn('GAMESCOPE_ARGS="-W 1920"\n', text)
        self.assertIn('# mine\n', text)
        self.assertTrue(text.endswith('HEARTH_PLASMA_DESKTOP=firehold\n'))
        self.assertEqual(self.h.choice(), 'firehold')
        before = self.conf().stat().st_mtime_ns
        self.h.choose('firehold')
        self.assertEqual(self.conf().stat().st_mtime_ns, before, 'unchanged: not written again')
        self.h.choose('plasma')
        self.assertEqual(self.h.choice(), 'plasma')
        self.assertTrue(pathlib.Path(str(self.conf()) + '.bak').exists(), 'the last good copy is kept')
        with self.assertRaises(ValueError):
            self.h.choose('mantel')

    def test_settings_a_key_at_a_time_and_not_when_unchanged(self):
        p = pathlib.Path(self.h.cfg_path())
        p.parent.mkdir(parents=True)
        p.write_text(json.dumps({'firehold.unlocked': True, 'posture': 'auto', 'overlay': 'basic'}))
        before = p.stat().st_mtime_ns
        self.h.set_setting('posture', 'auto')
        self.assertEqual(p.stat().st_mtime_ns, before)
        self.h.set_setting('startIn', 'couch')
        d = json.loads(p.read_text())
        self.assertEqual(d, {'firehold.unlocked': True, 'posture': 'auto', 'overlay': 'basic', 'startIn': 'couch'})
        with self.assertRaises(ValueError):
            self.h.set_setting('bad key!', 1)

    def test_a_damaged_settings_file_is_never_replaced_by_defaults(self):
        p = pathlib.Path(self.h.cfg_path())
        p.parent.mkdir(parents=True)
        p.write_text('{"posture": "couch"')
        self.assertEqual(self.h.start_posture(), 'desk')   # read as defaults for now...
        self.assertTrue(any(x.name.startswith('firehold.json.broken') for x in p.parent.iterdir()) or p.read_text().startswith('{"posture": "couch"'),
                        '...but the damaged file is kept')

    def test_start_posture_follows_firehold_js(self):
        p = pathlib.Path(self.h.cfg_path())
        p.parent.mkdir(parents=True)
        cases = [({}, '', 'desk'), ({'posture': 'couch'}, '', 'couch'), ({'posture': 'desk', 'startIn': 'couch'}, '', 'desk'),
                 ({'startIn': 'couch'}, '', 'couch'), ({'startIn': 'last'}, 'couch', 'couch'), ({'startIn': 'last'}, 'desk', 'desk')]
        for cfg, last, want in cases:
            p.write_text(json.dumps(cfg))
            sp = pathlib.Path(self.h.state_path())
            sp.parent.mkdir(parents=True, exist_ok=True)
            sp.write_text(last)
            self.assertEqual(self.h.start_posture(), want, (cfg, last))

    def test_shell_for_falls_back_to_plain_plasma(self):
        self.h.shell_exists = lambda p: False
        self.assertEqual(self.h.shell_for('couch'), 'org.kde.plasma.desktop')
        self.assertEqual(self.h.shell_for('desk'), 'org.kde.plasma.desktop')
        self.h.shell_exists = lambda p: True
        self.assertEqual(self.h.shell_for('couch'), 'org.hearthos.firehold')
        self.assertEqual(self.h.shell_for('desk'), 'org.hearthos.firehold.desk')

    def test_requests_are_checked(self):
        sent = []
        self.h.dbus_send = lambda *a, **k: sent.append(a) or ''
        self.h.ask('guide')
        self.h.ask('tab:library')
        self.assertEqual(len(sent), 2)
        self.assertIn('org.hearthos.firehold.couch', sent[0][-1])
        for bad in ('guide"); x(', 'tab:../x', ''):
            with self.assertRaises(ValueError):
                self.h.ask(bad)

    def test_session_helper_does_nothing_outside_firehold(self):
        self.assertEqual(self.h.session(), 0)

    def test_a_failed_start_goes_to_plain_plasma_and_keeps_the_choice(self):
        asked = []
        self.h.restart_shell = lambda pkg: asked.append(pkg)
        self.h.notify = lambda *a: None
        self.h.choose('firehold')
        self.assertFalse(self.h.watch_ready('org.hearthos.firehold', 0, secs=1))
        self.assertEqual(asked, ['org.kde.plasma.desktop'])
        self.assertEqual(self.h.choice(), 'firehold')
        self.h.write_small(os.path.join(self.h.run_dir(), 'ready-org.hearthos.firehold'), '1')
        self.assertTrue(self.h.watch_ready('org.hearthos.firehold', 0, secs=1))


class Session(unittest.TestCase):
    def test_plain_plasma_never_gets_firehold_shell(self):
        s = SESSION.read_text()
        run = s[s.index('run_plasma() {'):s.index('\n}\n', s.index('run_plasma() {'))]
        self.assertIn('unset-environment PLASMA_DEFAULT_SHELL HEARTH_FIREHOLD_PLASMA', run)
        self.assertIn('unset PLASMA_DEFAULT_SHELL HEARTH_FIREHOLD_PLASMA', run)
        self.assertIn('[ "$(firehold_plasma_choice)" = firehold ]', run)
        self.assertNotIn('choose', run, 'the session never changes the choice')

    def test_the_choice_is_read_only_from_its_line(self):
        fn = re.search(r'^firehold_plasma_choice\(\) \{.*?^\}', SESSION.read_text(), re.S | re.M).group(0)
        with tempfile.TemporaryDirectory() as d:
            conf = os.path.join(d, 'session.conf')
            for text, want in (('', 'plasma'), ('HEARTH_PLASMA_DESKTOP=firehold\n', 'firehold'),
                               ('export HEARTH_PLASMA_DESKTOP="firehold"\nHEARTH_PLASMA_DESKTOP=plasma\n', 'plasma'),
                               ('HEARTH_PLASMA_DESKTOP=weird\n', 'plasma')):
                with open(conf, 'w') as f:
                    f.write(text)
                out = subprocess.run(['sh', '-c', fn + '\nfirehold_plasma_choice'], env={'SESSION_CONF': conf, 'PATH': os.environ['PATH']},
                                     capture_output=True, text=True).stdout.strip()
                self.assertEqual(out, want, text)


class Performance(unittest.TestCase):
    def test_no_timers_or_animations_in_firehold_qml(self):
        bad = []
        for f in FP.rglob('*.qml'):
            t = f.read_text()
            for pat in (r'\bTimer\s*\{', r'Animation\s*\{', r'\bBehavior\s+on\b', r'\bAnimatedImage\b', r'\bAnimator\s*\{'):
                if re.search(pat, t):
                    bad.append('%s: %s' % (f.relative_to(ROOT), pat))
        self.assertEqual(bad, [])


class KWinScript(unittest.TestCase):
    def test_public_api_and_names_match_the_helper(self):
        js = (FP / 'kwin/contents/code/main.js').read_text()
        helper = HELPER.read_text()
        for name in ('Firehold Couch posture', 'Firehold Desk posture'):
            self.assertIn('registerShortcut("%s"' % name, js)
            self.assertIn("'%s'" % name, helper)
        for g in ('follow', 'fullscreen', 'window'):
            self.assertIn('registerShortcut("Firehold games %s"' % g, js)
        self.assertNotIn('setTimeout', js)
        self.assertNotIn('archive/kwin', helper)
        self.assertFalse(re.search(r'hearthwindows|hearth-kwin-bridge', js))


if __name__ == '__main__':
    unittest.main()
