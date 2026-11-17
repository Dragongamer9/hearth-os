"""Game Mode beside the desktop (1.14, docs/SIDE-BY-SIDE.md): off by default, the switch is one session.conf line,
nothing starts by itself, and with it off nimbus-mode does the usual switch."""
import os
import pathlib
import subprocess
import tempfile
import unittest

REPO = pathlib.Path(__file__).resolve().parents[3]
HELPER = REPO / 'os/system_files/usr/libexec/nimbus/hearth-side-by-side'
UNIT = REPO / 'os/system_files/usr/lib/systemd/system/hearth-side-game@.service'
IO = REPO / 'hosts/linux/desktop/shell/hearth_settings_io.py'


class SideBySide(unittest.TestCase):
    def env(self, d):
        return {**os.environ, 'HOME': d, 'XDG_CONFIG_HOME': d + '/cfg', 'XDG_DATA_HOME': d + '/data', 'XDG_RUNTIME_DIR': d,
                'HEARTH_SETTINGS_IO': str(IO), 'PATH': '/usr/bin:/bin'}

    def test_off_by_default_and_the_switch_is_one_line(self):
        with tempfile.TemporaryDirectory() as d:
            e = self.env(d)
            conf = pathlib.Path(d, 'cfg/nimbus/session.conf')
            conf.parent.mkdir(parents=True)
            conf.write_text('# mine\nHEARTH_GAME_MODE_CHOICE=steam\n')
            st = subprocess.run(['sh', str(HELPER), 'status'], env=e, capture_output=True, text=True).stdout
            self.assertIn('"on": false', st)
            self.assertEqual(subprocess.run(['sh', str(HELPER), 'game'], env=e).returncode, 1, 'off: game does nothing')
            subprocess.run(['sh', str(HELPER), 'on'], env=e, check=True)
            self.assertEqual(conf.read_text(), '# mine\nHEARTH_GAME_MODE_CHOICE=steam\nHEARTH_SIDE_BY_SIDE=1\n')
            self.assertIn('"on": true', subprocess.run(['sh', str(HELPER), 'status'], env=e, capture_output=True, text=True).stdout)
            subprocess.run(['sh', str(HELPER), 'off'], env=e, check=True)
            self.assertIn('HEARTH_SIDE_BY_SIDE=0', conf.read_text())
            self.assertIn('# mine', conf.read_text())

    def test_the_unit_never_starts_by_itself(self):
        u = UNIT.read_text()
        self.assertNotRegex(u, r'(?m)^\[Install\]')
        self.assertIn('HEARTH_SIDE=1', u)
        self.assertIn('Restart=no', u)

    def test_with_it_off_nimbus_mode_switches_as_before(self):
        with tempfile.TemporaryDirectory() as d:
            e = {**self.env(d), 'LABWC_PID': '999999', 'HEARTH_SIDE_HELPER': str(HELPER)}
            fake = pathlib.Path(d, 'bin'); fake.mkdir()
            (fake / 'labwc').write_text('#!/bin/sh\nexit 0\n'); (fake / 'labwc').chmod(0o755)
            e['PATH'] = '%s:/usr/bin:/bin' % fake
            subprocess.run(['sh', str(REPO / 'hosts/linux/session/nimbus-mode'), 'steam'], env=e, check=True, timeout=20)
            self.assertEqual(pathlib.Path(d, 'nimbus-next').read_text().strip(), 'steam')


if __name__ == '__main__':
    unittest.main()


class BackToADesktop(unittest.TestCase):
    """Start-safety review, Nov 17: if the desktop's session can't be shown again, another graphical session of yours
    is (loginctl activate, else chvt to its VT), so tty9 is never left blank."""

    def run_desktop(self, activate_ok):
        with tempfile.TemporaryDirectory() as d:
            b = os.path.join(d, 'bin'); os.makedirs(b)
            uid = os.getuid()
            # sessions: 2 = the desktop (gone), 5 = the side screen (tty9), 7 = another wayland session on VT 2
            lc = '''#!/bin/sh
echo "$*" >> "%(d)s/calls"
case "$1" in
  list-sessions) printf '2 %(u)s me seat0\\n5 %(u)s me seat0\\n7 %(u)s me seat0\\n9 %(u)s me -\\n' ;;
  show-session)
    case "$2" in
      5) if echo "$*" | grep -q Service; then printf 'login\\ntty9\\n'; else printf 'Type=wayland\\nState=active\\nVTNr=9\\n'; fi ;;
      7) printf 'Type=wayland\\nState=online\\nVTNr=2\\n' ;;
      9) printf 'Type=tty\\nState=online\\nVTNr=0\\n' ;;
      *) exit 1 ;;
    esac ;;
  activate) [ "$2" = 7 ] && [ "%(ok)s" = 1 ] && exit 0; exit 1 ;;
esac
''' % {'d': d, 'u': uid, 'ok': '1' if activate_ok else '0'}
            for name, text in (('loginctl', lc), ('chvt', '#!/bin/sh\necho "chvt $*" >> "%s/calls"\n' % d)):
                with open(os.path.join(b, name), 'w') as f:
                    f.write(text)
                os.chmod(os.path.join(b, name), 0o755)
            with open(os.path.join(d, 'hearth-side-desktop-session'), 'w') as f:
                f.write('2\n')
            env = {**os.environ, 'HOME': d, 'XDG_RUNTIME_DIR': d, 'XDG_DATA_HOME': d + '/data', 'PATH': b + ':/usr/bin:/bin'}
            r = subprocess.run(['sh', str(HELPER), 'desktop'], env=env, capture_output=True, text=True, timeout=30)
            with open(os.path.join(d, 'calls')) as f:
                return r.returncode, f.read()

    def test_another_session_is_shown(self):
        rc, calls = self.run_desktop(True)
        self.assertEqual(rc, 0, calls)
        self.assertIn('activate 2', calls)
        self.assertIn('activate 7', calls)
        self.assertNotIn('activate 5', calls, 'never the side screen itself')
        self.assertNotIn('activate 9', calls, 'only graphical sessions')

    def test_else_its_vt(self):
        rc, calls = self.run_desktop(False)
        self.assertEqual(rc, 0, calls)
        self.assertIn('chvt 2', calls)


class SessionPieces(unittest.TestCase):
    SESSION = REPO / 'hosts/linux/session/nimbus-session'

    def test_side_session_has_its_own_failure_file(self):
        s = self.SESSION.read_text()
        side = s[s.index('if [ "${HEARTH_SIDE:-}" = 1 ]; then'):]
        self.assertIn('SM_FAILED="$RUN/hearth-side-steam-mode-failed"', side[:side.index('\n  run_steam_mode\n')])
        self.assertNotIn('"$RUN/hearth-steam-mode-failed"', s[s.index('run_steam_mode() {'):s.index('# Steam Game Mode as the start-up') if '# Steam Game Mode as the start-up' in s else s.index('steam_paused()')])

    def test_steam_close_waits_while_any_pid_is_alive(self):
        s = self.SESSION.read_text()
        fn = s[s.index('any_alive() {'):].split('\n', 1)[0]
        body = s[s.index('steam_close_behind() {'):s.index('\n}\n', s.index('steam_close_behind() {'))]
        self.assertNotIn('kill -0 $pids', body)
        sleeper = subprocess.Popen(['sleep', '30'])
        try:
            r = subprocess.run(['sh', '-c', fn + '\nany_alive 999999 %d' % sleeper.pid])
            self.assertEqual(r.returncode, 0, 'one ended, one alive: still waiting')
            r = subprocess.run(['sh', '-c', fn + '\nany_alive 999999 999998'])
            self.assertEqual(r.returncode, 1)
        finally:
            sleeper.kill(); sleeper.wait()
