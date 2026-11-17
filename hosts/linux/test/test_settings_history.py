"""1.14, the new Settings: the change history behind Settings › Home › Changed lately (with Undo)."""
import json, os, subprocess, sys, tempfile, shutil, unittest

HERE = os.path.dirname(os.path.abspath(__file__))
IO = os.path.join(HERE, '..', 'desktop', 'shell', 'hearth_settings_io.py')
SHELL = os.path.join(HERE, '..', 'desktop', 'shell')


class History(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.env = dict(os.environ, XDG_STATE_HOME=os.path.join(self.tmp, 'state'), HOME=self.tmp)
        self.f = os.path.join(self.tmp, 'desktop.json')
        with open(self.f, 'w') as fh:
            json.dump({'nightLight': False, 'accent': '', 'keep': 1}, fh)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def io(self, *a, env=None):
        return subprocess.run([sys.executable, IO] + list(a), env=env or self.env, capture_output=True, text=True)

    def hist(self):
        return json.loads(self.io('history').stdout)

    def test_recorded_only_with_history_and_only_real_changes(self):
        self.io('merge', self.f, json.dumps({'nightLight': True}))
        self.assertEqual(self.hist(), [])
        self.io('merge', self.f, json.dumps({'nightLight': True, 'accent': '#ff8800'}), '--history')
        h = self.hist()
        self.assertEqual([(c['key'], c['from'], c['to'], c['who']) for c in h], [('accent', '', '#ff8800', 'you')])

    def test_undo_puts_the_old_value_back_and_keeps_the_rest(self):
        self.io('merge', self.f, json.dumps({'accent': '#ff8800'}), '--history')
        cid = self.hist()[0]['id']
        r = self.io('undo', str(cid))
        self.assertEqual(r.stdout.strip(), 'ok')
        d = json.load(open(self.f))
        self.assertEqual(d, {'nightLight': False, 'accent': '', 'keep': 1})
        self.assertEqual(self.hist()[0]['to'], '')   # the undo is itself in the history

    def test_undo_refused_when_changed_again_since(self):
        self.io('merge', self.f, json.dumps({'accent': '#ff8800'}), '--history')
        cid = self.hist()[0]['id']
        self.io('merge', self.f, json.dumps({'accent': '#00ff00'}))
        r = self.io('undo', str(cid))
        self.assertEqual((r.returncode, r.stdout.strip()), (1, 'changed'))
        self.assertEqual(json.load(open(self.f))['accent'], '#00ff00')
        self.assertEqual(self.io('undo', '999').stdout.strip(), 'gone')

    def test_who_and_cap(self):
        env = dict(self.env, HEARTH_CHANGED_BY='Hearth')
        for i in range(5):
            self.io('merge', self.f, json.dumps({'keep': i + 10}), '--history', env=env)
        h = self.hist()
        self.assertEqual(h[0]['to'], 14)
        self.assertTrue(all(c['who'] == 'Hearth' for c in h))

    def test_settings_writes_with_history(self):
        self.assertIn('history: true', open(os.path.join(SHELL, 'Settings.qml')).read())
        self.assertIn('concat(history ? ["--history"] : [])', open(os.path.join(SHELL, 'SettingsWriter.qml')).read())


if __name__ == '__main__':
    unittest.main()
