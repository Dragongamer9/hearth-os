"""Hearth's own login screen (1.14, a test: greetd + Hearth's greeter, SDDM always there to fall back to).

The red flag: an update must never leave a device unable to start or sign in. So these check, with the real
hearth-login-screen code in a test folder (systemctl replaced): it's off by default and SDDM is used; it only turns on
when everything is in place; automatic sign-in is the same account and session in both login screens (what
nimbus-autologin writes, run for real); the fallback to SDDM (once per start, not while shutting down, paused after two
failures in a row until turned on again, and the setting itself never changed); the units and the polkit rule; the
session launcher (Relogin like SDDM's); what the greeter lists."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import types
import unittest
from unittest import mock

ROOT = pathlib.Path(__file__).resolve().parents[3]
LS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-login-screen'
SS = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-session-start'
AUTOLOGIN = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'nimbus-autologin'
INFO = ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'hearth' / 'greeter' / 'greeter-info'
GREETER = ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'hearth' / 'greeter'
UNITS = ROOT / 'os-bases' / 'peppermint' / 'system_files' / 'usr' / 'lib' / 'systemd' / 'system'
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
BUILD = ROOT / 'os-bases' / 'peppermint' / 'build.sh'
RULE = ROOT / 'os' / 'system_files' / 'usr' / 'share' / 'polkit-1' / 'rules.d' / '49-hearth-login.rules'
SESSION_DIR = ROOT / 'hosts' / 'linux' / 'session'


def load(path, name, env):
    with mock.patch.dict(os.environ, env):
        loader = importlib.machinery.SourceFileLoader(name, str(path))
        spec = importlib.util.spec_from_loader(name, loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
    return m


class FakePwd:
    def __init__(self, users):
        self.users = users

    def getpwnam(self, name):
        if name not in self.users:
            raise KeyError(name)
        return types.SimpleNamespace(pw_name=name, pw_uid=self.users[name])


class Box(unittest.TestCase):
    """A machine in a folder: Hearth's image files (greetd, cage, Quickshell, the greeter), SDDM's config, /run."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.root = self.tmp + '/root'
        self.calls = []
        log = self.tmp + '/systemctl.log'
        self.sysctl = self.tmp + '/systemctl'
        with open(self.sysctl, 'w') as f:
            f.write('#!/bin/sh\necho "$*" >> %s\n'
                    'case "$1" in is-system-running) cat %s/sysstate 2>/dev/null || echo running;; '
                    'is-active) cat %s/unitstate 2>/dev/null || echo active;; list-jobs) cat %s/jobs 2>/dev/null;; esac\nexit 0\n'
                    % (log, self.tmp, self.tmp, self.tmp))
        os.chmod(self.sysctl, 0o755)
        self.log = log
        for p in ('/usr/sbin/greetd', '/usr/bin/cage', '/usr/bin/quickshell', '/usr/libexec/nimbus/hearth-greeter',
                  '/usr/libexec/nimbus/hearth-session-start'):
            self.exe(p)
        self.write('/usr/share/hearth/greeter/shell.qml', 'ShellRoot {}\n')
        self.write('/etc/pam.d/greetd', 'auth include common-auth\n')
        self.write('/proc/cmdline', 'root=UUID=x quiet splash\n')
        self.write('/proc/sys/kernel/random/boot_id', 'boot-1\n')
        for f in ('nimbus.desktop', 'hearth-classic.desktop'):
            self.write('/usr/share/wayland-sessions/' + f, (SESSION_DIR / f).read_text())
        self.m = self.module()

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def module(self):
        m = load(LS, 'hearth_login_screen_t', {'HEARTH_LOGIN_ROOT': self.root, 'HEARTH_SETTINGS_IO_DIR': str(SHELL),
                                                'HEARTH_LOGIN_SYSTEMCTL': self.sysctl, 'HEARTH_LOGIN_TIMEOUT': '1',
                                                'XDG_STATE_HOME': self.tmp + '/state'})
        m.pwd = FakePwd({'_greetd': 990, 'alex': 1000})
        return m

    def write(self, rel, text):
        p = self.root + rel
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, 'w') as f:
            f.write(text)
        return p

    def exe(self, rel):
        p = self.write(rel, '#!/bin/sh\n')
        os.chmod(p, 0o755)

    def read(self, rel):
        with open(self.root + rel) as f:
            return f.read()

    def exists(self, rel):
        return os.path.exists(self.root + rel)

    def state(self):
        try:
            return json.loads(self.read('/var/lib/hearth/login-screen-state.json'))
        except FileNotFoundError:
            return {}

    def mode(self):
        return json.loads(self.read('/run/hearth/login-mode.json'))

    def turn(self, on):
        with mock.patch.object(self.m.os, 'geteuid', return_value=0):
            self.assertEqual(self.m.cmd_set(on), 0)

    def boot(self, n):
        self.write('/proc/sys/kernel/random/boot_id', 'boot-%d\n' % n)
        for p in ('/run/hearth/login-greetd', '/run/hearth/greetd.toml', '/run/hearth/login-fell-back', '/run/hearth/login-mode.json'):
            if self.exists(p):
                os.remove(self.root + p)

    def systemctl_calls(self):
        try:
            with open(self.log) as f:
                return f.read().splitlines()
        except FileNotFoundError:
            return []

    def autologin(self, no_autologin=False, user='alex'):
        """Runs the real nimbus-autologin with its /etc in the test folder (and `id` answering for uid 1000)."""
        text = AUTOLOGIN.read_text().replace('/etc/', self.root + '/etc/').replace('/usr/lib/systemd/system/plasmalogin.service', self.root + '/none').replace('/usr/bin/plasmalogin', self.root + '/none')
        script = self.tmp + '/nimbus-autologin'
        with open(script, 'w') as f:
            f.write(text)
        os.makedirs(self.tmp + '/bin', exist_ok=True)
        with open(self.tmp + '/bin/id', 'w') as f:
            f.write('#!/bin/sh\n[ "$1" = -nu ] && [ "$2" = 1000 ] && { echo %s; exit 0; }\nexit 1\n' % user)
        os.chmod(self.tmp + '/bin/id', 0o755)
        if no_autologin:
            self.write('/etc/nimbus/no-autologin', '')
        env = dict(os.environ, PATH=self.tmp + '/bin:' + os.environ['PATH'])
        r = subprocess.run(['bash', script], env=env, capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


class OffByDefault(Box):
    def test_nothing_set_means_sddm_and_nothing_written_for_greetd(self):
        self.assertEqual(self.m.cmd_select(), 0)
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertFalse(self.exists('/run/hearth/greetd.toml'))
        self.assertEqual(self.mode()['mode'], 'sddm')
        self.assertFalse(self.exists('/etc/hearth/login-screen.json'), 'nothing may write the setting by itself')

    def test_turned_off_means_sddm(self):
        self.turn(True)
        self.turn(False)
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertEqual(json.loads(self.read('/etc/hearth/login-screen.json')), {'screen': 'sddm'})

    def test_a_damaged_setting_is_sddm_and_never_overwritten(self):
        self.write('/etc/hearth/login-screen.json', '{"screen": "hea')
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertEqual(self.mode()['mode'], 'sddm')

    def test_an_error_anywhere_in_select_is_sddm(self):
        self.turn(True)
        with mock.patch.object(self.m, 'requirements', side_effect=RuntimeError('boom')), \
                mock.patch.dict(os.environ, {'INVOCATION_ID': 'x'}):
            self.assertEqual(self.m.main(['x', 'select']), 0)
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertIn('error', self.mode()['why'])


class TurnedOn(Box):
    def test_on_with_everything_there_starts_greetd_with_the_greeter(self):
        self.turn(True)
        self.m.cmd_select()
        self.assertTrue(self.exists('/run/hearth/login-greetd'))
        conf = self.read('/run/hearth/greetd.toml')
        self.assertIn('[default_session]\ncommand = "/usr/libexec/nimbus/hearth-greeter"\nuser = "_greetd"', conf)
        self.assertIn('vt = 7', conf)
        self.assertNotIn('initial_session', conf)
        self.assertEqual(self.mode()['mode'], 'greetd')
        self.assertEqual(self.state()['pending']['boot'], 'boot-1')

    def test_the_setting_is_written_through_hearth_settings_io(self):
        self.write('/etc/hearth/login-screen.json', '{"screen": "sddm", "other": 1}\n')
        self.turn(True)
        self.assertEqual(json.loads(self.read('/etc/hearth/login-screen.json')), {'screen': 'hearth', 'other': 1})
        self.assertEqual(json.loads(self.read('/etc/hearth/login-screen.json.bak')), {'screen': 'sddm', 'other': 1})

    def test_anything_missing_keeps_sddm(self):
        self.turn(True)
        for rel, why in (('/usr/bin/cage', 'cage'), ('/usr/sbin/greetd', 'greetd'), ('/usr/bin/quickshell', 'Quickshell'),
                         ('/usr/share/hearth/greeter/shell.qml', 'greeter'), ('/etc/pam.d/greetd', 'PAM')):
            saved = self.read(rel)
            os.remove(self.root + rel)
            self.boot(2)
            self.m.cmd_select()
            self.assertFalse(self.exists('/run/hearth/login-greetd'), rel)
            self.assertIn(why, self.mode()['why'])
            self.write(rel, saved)
            os.chmod(self.root + rel, 0o755)
        self.m.pwd = FakePwd({'alex': 1000})
        self.boot(3)
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertIn('_greetd', self.mode()['why'])

    def test_live_recovery_boot_line_and_seats_keep_sddm(self):
        self.turn(True)
        for rel, text, why in (('/etc/hearth-live', 'installer\n', 'USB'), ('/etc/hearth-recovery', 'recovery\n', 'USB'),
                               ('/etc/hearth/seats.json', '{}', 'seat')):
            self.write(rel, text)
            self.boot(2)
            self.m.cmd_select()
            self.assertFalse(self.exists('/run/hearth/login-greetd'), rel)
            self.assertIn(why, self.mode()['why'])
            os.remove(self.root + rel)
        for line in ('quiet hearth.login=sddm', 'quiet hearth.clean'):
            self.write('/proc/cmdline', line + '\n')
            self.boot(3)
            self.m.cmd_select()
            self.assertFalse(self.exists('/run/hearth/login-greetd'), line)

    def test_select_only_runs_from_systemd(self):
        m = load(LS, 'hls_plain', {'HEARTH_LOGIN_ROOT': ''})
        with mock.patch.dict(os.environ, {}, clear=False):
            os.environ.pop('INVOCATION_ID', None)
            for cmd in ('select', 'watch', 'fallback', 'run-greetd'):
                self.assertEqual(m.main(['x', cmd]), 1)


class Autologin(Box):
    def test_game_mode_autologin_is_the_same_in_both_login_screens(self):
        self.autologin()
        sddm = self.read('/etc/sddm.conf.d/zzz-nimbus-autologin.conf')
        self.assertIn('User=alex\nSession=nimbus.desktop\nRelogin=true', sddm)     # SDDM's, unchanged
        self.assertEqual(self.m.sddm_autologin(), ('alex', 'nimbus.desktop', True))
        self.turn(True)
        self.m.cmd_select()
        conf = self.read('/run/hearth/greetd.toml')
        self.assertIn('[initial_session]\ncommand = "/usr/libexec/nimbus/hearth-session-start --relogin nimbus.desktop"\nuser = "alex"', conf)

    def test_no_autologin_shows_the_greeter(self):
        self.autologin(no_autologin=True)
        self.assertIsNone(self.m.sddm_autologin())
        self.turn(True)
        self.m.cmd_select()
        self.assertTrue(self.exists('/run/hearth/login-greetd'))
        self.assertNotIn('initial_session', self.read('/run/hearth/greetd.toml'))

    def test_sddm_main_conf_wins_like_sddm_and_later_files_win(self):
        self.write('/etc/sddm.conf.d/zz-bazzite.conf', '[Autologin]\nUser=other\nSession=steam.desktop\n')
        self.write('/etc/sddm.conf.d/zzz-nimbus-autologin.conf', '[Autologin]\nUser=alex\nSession=nimbus.desktop\nRelogin=true\n')
        self.assertEqual(self.m.sddm_autologin(), ('alex', 'nimbus.desktop', True))
        self.write('/etc/sddm.conf', '[General]\nNumlock=on\n[Autologin]\nUser=alex\nSession=hearth-classic.desktop\n')
        self.assertEqual(self.m.sddm_autologin(), ('alex', 'hearth-classic.desktop', True))

    def test_an_x11_or_missing_autologin_session_keeps_sddm(self):
        self.turn(True)
        self.write('/etc/sddm.conf.d/zzz-nimbus-autologin.conf', '[Autologin]\nUser=alex\nSession=xfce.desktop\nRelogin=true\n')
        self.write('/usr/share/xsessions/xfce.desktop', '[Desktop Entry]\nExec=startxfce4\n')
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertIn('X11', self.mode()['why'])
        self.write('/etc/sddm.conf.d/zzz-nimbus-autologin.conf', '[Autologin]\nUser=alex\nSession=gone.desktop\n')
        self.boot(2)
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.write('/etc/sddm.conf.d/zzz-nimbus-autologin.conf', '[Autologin]\nUser=nobody-here\nSession=nimbus.desktop\n')
        self.boot(3)
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))


class Fallback(Box):
    def setUp(self):
        super().setUp()
        self.turn(True)
        self.m.cmd_select()
        self.assertTrue(self.exists('/run/hearth/login-greetd'))

    def test_a_failure_starts_sddm_once_and_counts_once(self):
        self.m.cmd_fallback('the greeter crashed')
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        calls = self.systemctl_calls()
        self.assertIn('stop hearth-greetd.service', calls)
        self.assertIn('start --no-block sddm.service', calls)
        self.assertEqual(self.state()['failures'], 1)
        self.assertNotIn('tripped', self.state())
        m = self.mode()
        self.assertEqual((m['mode'], m['fellBack']), ('sddm', True))
        self.m.cmd_fallback('again (OnFailure after the stop)')
        self.assertEqual(self.state()['failures'], 1, 'one start counts once')
        self.assertEqual(json.loads(self.read('/etc/hearth/login-screen.json')), {'screen': 'hearth'}, 'the setting never changes by itself')

    def test_two_failures_in_a_row_pause_it_until_turned_on_again(self):
        self.m.cmd_fallback('first')
        self.boot(2)
        self.m.cmd_select()
        self.assertTrue(self.exists('/run/hearth/login-greetd'), 'one failure: tried again at the next start')
        self.m.cmd_fallback('second')
        st = self.state()
        self.assertTrue(st['tripped'])
        self.boot(3)
        self.m.cmd_select()
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertIn('paused', self.mode()['why'])
        self.assertEqual(json.loads(self.read('/etc/hearth/login-screen.json')), {'screen': 'hearth'})
        self.turn(True)       # Settings › Turn on again
        self.assertFalse(self.state().get('tripped'))
        self.boot(4)
        self.m.cmd_select()
        self.assertTrue(self.exists('/run/hearth/login-greetd'))

    def test_a_good_start_between_failures_starts_the_count_over(self):
        self.m.cmd_fallback('first')
        self.boot(2)
        self.m.cmd_select()
        self.m.mark_good('signed in')
        self.boot(3)
        self.m.cmd_select()
        self.m.cmd_fallback('again')
        self.assertEqual(self.state()['failures'], 1)
        self.assertFalse(self.state().get('tripped'))

    def test_a_start_that_never_finished_counts_at_the_next_one(self):
        # boot 1 chose greetd and then the PC was switched off (black screen, power button held)
        self.boot(2)
        self.m.cmd_select()
        self.assertEqual(self.state()['failures'], 1)
        self.assertTrue(self.exists('/run/hearth/login-greetd'))
        self.boot(3)
        self.m.cmd_select()
        self.assertTrue(self.state()['tripped'])
        self.assertFalse(self.exists('/run/hearth/login-greetd'))

    def test_nothing_happens_while_shutting_down(self):
        with open(self.tmp + '/sysstate', 'w') as f:
            f.write('stopping\n')
        self.m.cmd_fallback('greetd stopped at shutdown')
        self.assertTrue(self.exists('/run/hearth/login-greetd'))
        self.assertNotIn('start --no-block sddm.service', self.systemctl_calls())
        self.assertEqual(self.state().get('failures', 0), 0)
        os.remove(self.tmp + '/sysstate')
        with open(self.tmp + '/jobs', 'w') as f:
            f.write('123 reboot.target start waiting\n')
        self.m.cmd_fallback('greetd stopped at a restart')
        self.assertEqual(self.state().get('failures', 0), 0)


class Watch(Box):
    def setUp(self):
        super().setUp()
        self.turn(True)
        self.m.cmd_select()

    def test_no_greeter_on_screen_in_time_is_sddm(self):
        self.m.TIMEOUT = 0.3
        self.m.cmd_watch(poll=0.05)
        self.assertFalse(self.exists('/run/hearth/login-greetd'))
        self.assertIn('didn\'t show', self.state()['last']['why'])
        self.assertIn('start --no-block sddm.service', self.systemctl_calls())

    def test_the_greeter_on_screen_then_a_sign_in_marks_it_working(self):
        self.m.TIMEOUT = 0.3
        self.write('/run/user/990/hearth-greeter-ready', '')
        seen = {'n': 0}
        real = self.m.signed_in_through_greetd

        def later():
            seen['n'] += 1
            if seen['n'] == 12:
                self.write('/run/systemd/sessions/3', 'UID=1000\nUSER=alex\nSERVICE=greetd\nCLASS=user\nTYPE=wayland\n')
                self.write('/run/systemd/sessions/2', 'UID=990\nSERVICE=greetd-greeter\nCLASS=greeter\n')
            return real()
        with mock.patch.object(self.m, 'signed_in_through_greetd', side_effect=later):
            self.m.cmd_watch(poll=0.05)
        st = self.state()
        self.assertEqual(st['failures'], 0)
        self.assertNotIn('pending', st)
        self.assertTrue(self.exists('/run/hearth/login-greetd'), 'shown: no timeout once it was on screen')

    def test_the_greeters_own_restart_counts_as_working(self):
        self.write('/run/user/990/hearth-greeter-ok', '')
        self.m.cmd_watch(poll=0.05)
        self.assertNotIn('pending', self.state())

    def test_greetd_stopping_is_sddm(self):
        with open(self.tmp + '/unitstate', 'w') as f:
            f.write('failed\n')
        with mock.patch.object(self.m.time, 'monotonic', side_effect=[0, 5, 5, 5, 5, 5, 5]):
            self.m.cmd_watch(poll=0)
        self.assertIn('greetd stopped', self.state()['last']['why'])

    def test_only_a_user_session_from_greetd_counts(self):
        self.write('/run/systemd/sessions/2', 'SERVICE=greetd\nCLASS=greeter\n')
        self.write('/run/systemd/sessions/c1', 'SERVICE=sddm\nCLASS=user\n')
        self.assertFalse(self.m.signed_in_through_greetd())
        self.write('/run/systemd/sessions/5', 'SERVICE=greetd\nCLASS=user\n')
        self.assertTrue(self.m.signed_in_through_greetd())


class StatusAndNotice(Box):
    def test_status_says_what_this_start_used(self):
        self.turn(True)
        self.m.cmd_select()
        self.m.cmd_fallback('it didn\'t show within 60 s')
        out = []
        with mock.patch('builtins.print', side_effect=lambda s, **k: out.append(s)):
            self.m.cmd_status()
        d = json.loads(out[0])
        self.assertTrue(d['on'])
        self.assertEqual(d['thisStart'], 'sddm')
        self.assertTrue(d['fellBack'])
        self.assertTrue(d['canStart'])

    def test_the_person_is_told_once(self):
        self.turn(True)
        self.m.cmd_select()
        self.m.cmd_fallback('the greeter crashed')
        sent = []

        def run(cmd, **k):
            sent.append(cmd)
            return types.SimpleNamespace(returncode=0)
        with mock.patch.object(self.m.shutil, 'which', return_value='/usr/bin/notify-send'), \
                mock.patch.object(self.m.subprocess, 'run', side_effect=run), \
                mock.patch.dict(os.environ, {'XDG_STATE_HOME': self.tmp + '/state'}):
            self.m.cmd_notice(tries=2, wait=0)
            self.m.cmd_notice(tries=2, wait=0)
        self.assertEqual(len(sent), 1)
        self.assertIn('usual login screen', sent[0][-1])


class UnitsAndRules(unittest.TestCase):
    def unit(self, name):
        return (UNITS / name).read_text()

    def test_sddm_runs_unless_the_file_exists_and_nothing_conflicts(self):
        drop = self.unit('sddm.service.d/50-hearth-login-screen.conf')
        self.assertIn('ConditionPathExists=!/run/hearth/login-greetd', drop)
        g = self.unit('hearth-greetd.service')
        self.assertIn('ConditionPathExists=/run/hearth/login-greetd', g)
        self.assertIn('OnFailure=hearth-login-fallback.service', g)
        self.assertIn('Restart=no', g)
        for name in ('hearth-greetd.service', 'hearth-login-select.service', 'hearth-login-watch.service', 'hearth-login-fallback.service'):
            text = self.unit(name)
            self.assertNotIn('Conflicts=', text, name)
            self.assertNotIn('display-manager.service\n', text.split('[Install]')[-1] if '[Install]' in text else '', name)
            self.assertNotIn('Alias=', text, name)
        sel = self.unit('hearth-login-select.service')
        self.assertIn('Before=display-manager.service sddm.service hearth-greetd.service', sel)
        self.assertIn('TimeoutStartSec=', sel)

    def test_the_image_keeps_sddm_as_the_display_manager(self):
        b = BUILD.read_text()
        self.assertIn('ln -sf /usr/lib/systemd/system/sddm.service /etc/systemd/system/display-manager.service', b)
        self.assertIn('test "$(readlink /etc/systemd/system/display-manager.service)" = /usr/lib/systemd/system/sddm.service', b)
        self.assertIn('optional greetd cage', b)
        self.assertLess(b.index('optional greetd cage'), b.index('ln -sf /usr/lib/systemd/system/sddm.service'))
        self.assertNotIn('systemctl enable greetd', b)
        self.assertIn('systemctl enable hearth-login-select.service hearth-greetd.service', b)
        self.assertIn('OskBoard.qml', b)

    def test_polkit_allows_only_on_and_off(self):
        rule = RULE.read_text()
        self.assertIn('/usr/libexec/nimbus/hearth-login-screen', rule)
        rx = re.compile(r'^\S+\s+(on|off)$')
        self.assertIn(r'/^\S+\s+(on|off)$/', rule)
        for ok in ('/usr/libexec/nimbus/hearth-login-screen on', '/usr/libexec/nimbus/hearth-login-screen off'):
            self.assertTrue(rx.search(ok))
        for bad in ('/usr/libexec/nimbus/hearth-login-screen fallback x', '/usr/libexec/nimbus/hearth-login-screen on\nx',
                    '/usr/libexec/nimbus/hearth-login-screen select', '/usr/libexec/nimbus/hearth-login-screen on extra'):
            self.assertFalse(rx.search(bad), bad)

    def test_settings_switch_uses_pkexec_with_an_argument_array(self):
        q = (SHELL / 'LoginScreenSwitch.qml').read_text()
        self.assertIn('["pkexec", helper, v ? "on" : "off"]', q)
        self.assertIn('LoginScreenSwitch', (SHELL / 'SetSignin.qml').read_text())

    def test_the_login_screens_pick_reaches_the_session_for_that_sign_in_only(self):
        s = (SESSION_DIR / 'nimbus-session').read_text()
        self.assertIn('case "${HEARTH_LOGIN_MODE:-}" in game|desktop) NIMBUS_START_MODE="$HEARTH_LOGIN_MODE" ;; esac', s)
        self.assertLess(s.index('. "$CONF/session.conf"'), s.index('HEARTH_LOGIN_MODE:-'))

    def test_boot_guard_and_the_controller_know_the_greeter(self):
        bg = (ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-boot-guard').read_text()
        self.assertIn("'cage')", bg)
        pad = (ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-greeter-pad').read_text()
        self.assertIn("'plasma-login-gr', 'cage')", pad)

    def test_the_greeter_never_keeps_the_password(self):
        q = (GREETER / 'Login.qml').read_text()
        self.assertNotRegex(q, r'console\.(log|warn|info)\([^)]*(password|pendingSecret)')
        self.assertNotRegex(q, r'setText\([^)]*password')
        self.assertIn('Greetd.respond(s)', q)
        osk = (GREETER / 'GreeterOsk.qml').read_text()
        self.assertIn('learn: false', osk)
        self.assertIn('mode: "password"', osk)


class SessionStart(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        d = self.tmp + '/sessions'
        os.makedirs(d)
        for f in ('nimbus.desktop', 'hearth-classic.desktop'):
            (pathlib.Path(d) / f).write_text((SESSION_DIR / f).read_text())
        self.m = load(SS, 'hss_t', {'HEARTH_SESSION_DIRS': d})

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_sets_what_sddm_sets_and_passes_the_mode(self):
        path = self.m.find('nimbus.desktop')
        vals = self.m.parse(path)
        self.assertEqual(self.m.command(vals), ['/usr/local/bin/nimbus-session'])
        env = self.m.session_env('nimbus.desktop', vals, {'HEARTH_LOGIN_MODE': 'desktop'})
        self.assertEqual((env['XDG_SESSION_TYPE'], env['XDG_SESSION_DESKTOP'], env['XDG_CURRENT_DESKTOP'], env['HEARTH_LOGIN_MODE']),
                         ('wayland', 'nimbus', 'Nimbus', 'desktop'))
        env = self.m.session_env('nimbus.desktop', vals, {'HEARTH_LOGIN_MODE': 'rm -rf'})
        self.assertNotIn('HEARTH_LOGIN_MODE', env)
        self.assertEqual(self.m.find('../etc/passwd'), '')
        self.assertEqual(self.m.command(self.m.parse(self.m.find('hearth-classic.desktop'))), ['/usr/local/bin/nimbus-session', '--desktop=classic'])

    def test_relogin_like_sddm_but_not_forever(self):
        runs = []
        t = {'now': 0.0}

        class P:
            def __init__(self, argv, env=None):
                runs.append(argv)

            def wait(self):
                t['now'] += 3600 if len(runs) == 1 else 2     # one long session, then three that end right away
                return 0

            def poll(self):
                return 0
        with mock.patch.object(self.m.subprocess, 'Popen', P), mock.patch.object(self.m, 'shutting_down', return_value=False):
            self.m.run_relogin(['nimbus-session'], {}, sleep=lambda s: None, clock=lambda: t['now'])
        self.assertEqual(len(runs), 4)

    def test_no_relogin_while_shutting_down(self):
        runs = []

        class P:
            def __init__(self, argv, env=None):
                runs.append(argv)

            def wait(self):
                return 0

            def poll(self):
                return 0
        with mock.patch.object(self.m.subprocess, 'Popen', P), mock.patch.object(self.m, 'shutting_down', return_value=True):
            self.m.run_relogin(['nimbus-session'], {}, sleep=lambda s: None)
        self.assertEqual(len(runs), 1)


class GreeterInfo(unittest.TestCase):
    def test_accounts_and_sessions(self):
        tmp = tempfile.mkdtemp()
        try:
            (pathlib.Path(tmp) / 'passwd').write_text(
                'root:x:0:0:root:/root:/bin/bash\n_greetd:x:990:990::/var/lib/greetd:/usr/sbin/nologin\n'
                'alex:x:1000:1000:Alex Fire,,,:/home/alex:/bin/bash\nsam:x:1001:1001::/home/sam:/bin/bash\n'
                'nobody:x:65534:65534::/:/usr/sbin/nologin\nsvc:x:1002:1002::/:/bin/false\n')
            os.makedirs(tmp + '/icons')
            (pathlib.Path(tmp) / 'icons' / 'alex').write_bytes(b'\x89PNG')
            os.makedirs(tmp + '/s')
            for f in ('nimbus.desktop', 'hearth-classic.desktop', 'hearth-kde.desktop'):
                # the image's own paths (build.sh puts nimbus-session in /usr/bin): here, a program that's there
                (pathlib.Path(tmp) / 's' / f).write_text((SESSION_DIR / f).read_text().replace('/usr/local/bin/nimbus-session', '/bin/sh'))
            (pathlib.Path(tmp) / 'kb').write_text('XKBMODEL="pc105"\nXKBLAYOUT="us,de"\n')
            env = dict(os.environ, HEARTH_GREETER_PASSWD=tmp + '/passwd', HEARTH_GREETER_ICONS=tmp + '/icons',
                       HEARTH_GREETER_SESSIONS=tmp + '/s', HEARTH_GREETER_KEYBOARD=tmp + '/kb')
            r = subprocess.run([sys.executable, str(INFO), 'info'], env=env, capture_output=True, text=True)
            d = json.loads(r.stdout)
            self.assertEqual([u['name'] for u in d['users']], ['alex', 'sam'])
            self.assertEqual(d['users'][0]['real'], 'Alex Fire')
            self.assertTrue(d['users'][0]['picture'].endswith('/icons/alex'))
            self.assertEqual(d['users'][1]['picture'], '')
            ids = [s['id'] for s in d['sessions']]
            self.assertEqual(ids[:3], ['hearth', 'game', 'desktop'])
            self.assertNotIn('hearth-classic', ids)
            self.assertEqual('hearth-kde' in ids, os.path.exists('/usr/bin/startplasma-wayland'))   # only once KDE is there
            self.assertEqual(d['layouts'], ['us', 'de'])
        finally:
            shutil.rmtree(tmp, ignore_errors=True)


if __name__ == '__main__':
    unittest.main()
