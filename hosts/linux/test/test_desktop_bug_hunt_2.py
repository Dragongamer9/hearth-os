"""Desktop Mode bug hunt (after 1.8).

Desktop Mode hands text to its helpers (passwords, a keyboard layout, a schedule) on stdin, never through
Process.environment: the Quickshell on Hearth's Debian (Qt 6.8) can't take a JavaScript object there ("Unable to
assign QJSValue to QVariantHash" in the owner's logs), so the helpers got nothing and those saves failed."""
import os
import pathlib
import re
import subprocess
import sys
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
ECHO = r'''
import json, os, sys
print(json.dumps({"env": os.environ.get("HEARTH_INPUT"), "stdin": sys.stdin.read(), "args": sys.argv[1:]}))
'''


def feed_of(name, prop):
    m = re.search(r"readonly property string %s: ('(?:[^'\\]|\\.)*')" % prop, (SHELL / name).read_text())
    assert m, (name, prop)
    return m.group(1)[1:-1]


class NoEnvironmentObjects(unittest.TestCase):
    def test_no_process_environment_object_in_the_shell(self):
        for f in sorted(SHELL.glob('*.qml')):
            for n, line in enumerate(f.read_text().splitlines(), 1):
                if line.lstrip().startswith('//'):
                    continue
                self.assertIsNone(re.search(r'^\s*environment\s*:', line), '%s:%d %s' % (f.name, n, line.strip()))

    def test_the_writers_send_on_stdin(self):
        for f, proc in (('SetJob.qml', 'proc'), ('FileManager.qml', 'procIn'), ('SetSignin.qml', 'admin')):
            s = (SHELL / f).read_text()
            self.assertIn('stdinEnabled: %s.input !== ""' % proc, s, f)
            self.assertIn('onStarted: { if (%s.input !== "") { %s.write(%s.input); %s.input = ""; } }' % ((proc,) * 4), s, f)
        w = (SHELL / 'SetWifi.qml').read_text()
        self.assertIn('stdinEnabled: action.psk !== ""', w)
        self.assertIn('action.write(action.psk + "\\n")', w)
        self.assertIn('run(["nmcli", "--ask", "device", "wifi", "connect", n.ssid], "Connected to " + n.ssid, pw);', w)
        self.assertNotIn('HEARTH_WIFI_PSK', w)
        self.assertIn('["env", "HEARTH_PAD_SIM=" + pad.simDir, "python3", pad.helper]', (SHELL / 'Pad.qml').read_text())

    def test_files_uses_the_same_wrapper(self):
        self.assertEqual(feed_of('SetJob.qml', 'feed'), feed_of('FileManager.qml', 'inputFeed'))


class Wrapper(unittest.TestCase):
    """SetJob's sh wrapper: what came on stdin reaches the helper on stdin and in HEARTH_INPUT, exactly."""

    def run_feed(self, flag, data, *args):
        with tempfile.TemporaryDirectory() as d:
            helper = os.path.join(d, 'helper')
            pathlib.Path(helper).write_text(ECHO)
            cmd = ['sh', '-c', feed_of('SetJob.qml', 'feed') + sys.executable + ' "$@" 2>&1', 'sh', flag, helper] + list(args)
            env = dict(os.environ)
            env.pop('HEARTH_INPUT', None)
            r = subprocess.run(cmd, input=data.encode() if data is not None else None,
                               stdin=subprocess.DEVNULL if data is None else None,
                               capture_output=True, timeout=20, env=env)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
            import json
            return json.loads(r.stdout.decode().strip().splitlines()[-1])

    def test_text_arrives_in_both_places(self):
        text = '{"layouts": [{"layout": "de", "variant": ""}], "pw": "a b $x `y` \\"q\\" ünï"}\n\n'
        r = self.run_feed('1', text, 'keyboard-set', '--x')
        self.assertEqual(r['stdin'], text)
        self.assertEqual(r['env'], text)
        self.assertEqual(r['args'], ['keyboard-set', '--x'])

    def test_no_text_is_empty_and_never_waits(self):
        r = self.run_feed('0', None, 'status')
        self.assertEqual((r['stdin'], r['env'], r['args']), ('', '', ['status']))

    def test_hearth_system_reads_it(self):
        # pointer-set with the request the Mouse page sends: it parses (it failed with "expected a JSON object").
        with tempfile.TemporaryDirectory() as home:
            env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=os.path.join(home, '.config'), XDG_RUNTIME_DIR=os.path.join(home, 'run'))
            env.pop('LABWC_PID', None)
            env.pop('HEARTH_INPUT', None)
            cmd = ['sh', '-c', feed_of('SetJob.qml', 'feed') + 'python3 "$@" 2>&1', 'sh', '1', str(SHELL / 'hearth-system'), 'pointer-set']
            r = subprocess.run(cmd, input=b'{"mouse": {"speed": 0.5}}', capture_output=True, timeout=30, env=env)
            self.assertNotIn(b'expected', r.stdout)
            self.assertIn(b'"ok": true', r.stdout)


class SavesAreNotLost(unittest.TestCase):
    def test_settings_writer_tries_a_failed_save_again(self):
        w = (SHELL / 'SettingsWriter.qml').read_text()
        self.assertIn('w.pending = Object.assign({}, w.inflight, w.pending);', w)   # newer changes win
        self.assertIn('retry.restart();', w)
        self.assertIn('if (retry.running) return;', w)
        self.assertRegex(w, r'w\.failures < 5')

    def test_desk_switches_say_when_they_could_not_save(self):
        d = (SHELL / 'DeskState.qml').read_text()
        for k in ('"dnd"', '"autoArrange"', '"desktopIcons"'):
            self.assertIn('call(["set", %s, on ? "on" : "off"], (r, err) => failedOr(r, err));' % k, d)
        self.assertIn('function failedOr(r, err)', d)
        self.assertIn('file.reload();', d)


class RightClickMenuFits(unittest.TestCase):
    def test_todays_menu_scrolls_and_keeps_the_row_in_view(self):
        m = (SHELL / 'PopMenu.qml').read_text()
        self.assertIn('height: Math.min(col.implicitHeight + 12, root.height - 16)', m)
        self.assertIn('id: flick', m)
        self.assertIn('contentHeight: col.implicitHeight', m)
        self.assertIn('onSelChanged: reveal()', m)
        self.assertIn('onItemsChanged: flick.contentY = 0', m)
        self.assertIn('id: rows', m)
        self.assertEqual(m.count('{'), m.count('}'))


if __name__ == '__main__':
    unittest.main()
