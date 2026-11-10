"""Server mode (docs/SERVER-MODE.md, Hearth 1.6): the session's run_server (run for real between nimbus-session's >>> and <<<
marks, with stand-ins for labwc and the helper), the whole session with Server mode off and on (it's opt-in: the default
start stays Game Mode, and every way out of Server mode ends in Game Mode), nimbus-mode server, the helper
(hearth-server-mode: settings written only when changed, the schedule, Start in through hearth_settings_io, the PIN kept
only as a hash, the web page reachable only from Tailscale and this PC, requests checked again, the countdown and its
cancel, the agent) and the root helper's part (the small model while you play, ComfyUI's queue kept, the dashboard's
service actions, the graphics card's idle limit). Nothing here starts labwc, Steam or a real service; every patched global
and environment variable is put back after each test."""
import sys
sys.dont_write_bytecode = True
import http.client
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import threading
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts/linux/session/nimbus-session'
MODE = ROOT / 'hosts/linux/session/nimbus-mode'
SHELL = ROOT / 'hosts/linux/desktop/shell'
HELPER = SHELL / 'hearth-server-mode'
AUTOSTART = ROOT / 'hosts/linux/desktop/server/labwc/autostart'


def block(name):
    m = re.search(r'# >>> %s\n(.*?)# <<< %s' % (name, name), SESSION.read_text(), re.S)
    assert m, name + ' block not found'
    return m.group(1)


def load_helper(env):
    old = {k: os.environ.get(k) for k in env}
    os.environ.update(env)
    try:
        loader = importlib.machinery.SourceFileLoader('hearth_server_mode_t', str(HELPER))
        spec = importlib.util.spec_from_loader('hearth_server_mode_t', loader)
        m = importlib.util.module_from_spec(spec)
        loader.exec_module(m)
        return m
    finally:
        for k, v in old.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v


FAKE_HELPER = '''#!/usr/bin/python3
import os, sys
a = sys.argv[1:]
open(os.environ['FAKE_CALLS'], 'a').write(' '.join(a) + '\\n')
if a == ['session-check']:
    print(os.environ.get('FAKE_ON', 'on'))
elif a == ['sleep-idle']:
    print('off')
elif a[:1] == ['start-mode']:
    print(os.environ.get('FAKE_START', a[1]))
'''

LABWC = '''#!/bin/sh
# Stand-in for labwc in Server mode: what it does comes from $LABWC_DO (one word per start, from a list).
n=$(cat "$HOME/labwc-n" 2>/dev/null || echo 0); n=$((n + 1)); echo $n > "$HOME/labwc-n"
echo "start server=${HEARTH_SERVER_MODE:-0}" >> "$HOME/labwc-ran"
do=$(echo "$LABWC_DO" | cut -d, -f$n)
[ -z "$HEARTH_SERVER_MODE" ] && exit 0
case "$do" in
  up-game) : > "$XDG_RUNTIME_DIR/hearth-server-dash-up"; echo game > "$XDG_RUNTIME_DIR/nimbus-next" ;;
  up-desktop) : > "$XDG_RUNTIME_DIR/hearth-server-dash-up"; echo desktop > "$XDG_RUNTIME_DIR/nimbus-next" ;;
  up-logout) : > "$XDG_RUNTIME_DIR/hearth-server-dash-up"; echo logout > "$XDG_RUNTIME_DIR/nimbus-next" ;;
  never-up) sleep 30 ;;
  crash) exit 1 ;;
  gaveup) : > "$XDG_RUNTIME_DIR/hearth-server-dash-up"; echo "the dashboard keeps closing" > "$XDG_RUNTIME_DIR/hearth-server-dash-failed" ;;
  *) exit 0 ;;
esac
exit 0
'''


class SessionBase(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-server-mode-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.share = os.path.join(self.tmp, 'share')
        for d in (self.home, self.run_dir, self.bin, self.share):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.calls = os.path.join(self.tmp, 'calls')
        self.helper = os.path.join(self.tmp, 'hearth-server-mode')
        self.write(self.helper, FAKE_HELPER)
        self.qml = os.path.join(self.tmp, 'server-mode.qml')
        self.write(self.qml, '// stand-in\n')
        self.labwc_dir = os.path.join(self.tmp, 'server-labwc')
        os.makedirs(self.labwc_dir)
        self.write(os.path.join(self.labwc_dir, 'rc.xml'), '<labwc_config/>\n')
        self.write(os.path.join(self.labwc_dir, 'autostart'), '#!/bin/sh\n')
        self.write(os.path.join(self.bin, 'labwc'), LABWC)
        self.write(os.path.join(self.bin, 'quickshell'), '#!/bin/sh\nexit 0\n')
        self.addCleanup(lambda: subprocess.run(['pkill', '-KILL', '-f', self.tmp], capture_output=True))

    def write(self, p, text):
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def env(self, **kw):
        e = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_SHARE': self.share,
             'HEARTH_SERVER_HELPER': self.helper, 'HEARTH_SERVER_QML': self.qml, 'HEARTH_SERVER_LABWC': self.labwc_dir,
             'FAKE_CALLS': self.calls, 'HEARTH_CMDLINE': os.devnull, 'HEARTH_PAD_HELD': os.devnull, 'SERVER_UP_SECS': '3',
             'NIMBUS_APP': '/nonexistent', 'HEARTH_STEAM_FIRSTRUN_LAUNCHERS': '/nonexistent'}
        e.update(kw)
        return e

    def read(self, name):
        try:
            return open(os.path.join(self.run_dir, name)).read().strip()
        except OSError:
            return None

    def labwc_starts(self):
        p = os.path.join(self.home, 'labwc-ran')
        return open(p).read().splitlines() if os.path.exists(p) else []


class RunServer(SessionBase):
    """run_server and its neighbours on their own (the block between the marks)."""

    def run_block(self, script_tail='run_server; echo "rc=$?"', **env):
        log = os.path.join(self.tmp, 'session.log')
        script = os.path.join(self.tmp, 'run.sh')
        with open(script, 'w') as f:
            f.write('RUN=%s\nNEXT="$RUN/nimbus-next"\nLOG=%s\nSHARE=%s\n' % (self.run_dir, log, self.share))
            f.write('log() { echo "$*" >>"$LOG"; }\nhave() { command -v "$1" >/dev/null 2>&1; }\n')
            f.write(block('server mode') + '\n' + script_tail + '\n')
        r = subprocess.run(['/bin/sh', script], env=self.env(**env), capture_output=True, text=True, timeout=60)
        self.log = open(log).read() if os.path.exists(log) else ''
        return r

    def test_server_mode_off_opens_game_mode(self):
        r = self.run_block(FAKE_ON='off', LABWC_DO='up-game')
        self.assertIn('rc=0', r.stdout, r.stderr)
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn('Server mode is off', self.read('hearth-server-mode-failed'))
        self.assertEqual(self.labwc_starts(), [], 'labwc never starts while Server mode is off')

    def test_missing_labwc_or_files_open_game_mode(self):
        os.remove(os.path.join(self.bin, 'labwc'))
        self.run_block(PATH=self.bin + ':/bin', LABWC_DO='up-game')
        if shutil.which('labwc', path='/bin'):
            self.skipTest('labwc is installed here')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn("isn't installed", self.read('hearth-server-mode-failed'))

    def test_missing_dashboard_file_opens_game_mode(self):
        os.remove(self.qml)
        self.run_block(LABWC_DO='up-game')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn('files are missing', self.read('hearth-server-mode-failed'))

    def test_a_dashboard_that_never_appears_is_closed_and_game_mode_opens(self):
        t = time.time()
        self.run_block(LABWC_DO='never-up')
        self.assertLess(time.time() - t, 20, 'closed after SERVER_UP_SECS, never left waiting')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn("didn't appear", self.read('hearth-server-mode-failed'))
        self.assertIsNone(self.read('hearth-server-mode.pid'))

    def test_a_crash_right_away_opens_game_mode(self):
        self.run_block(LABWC_DO='crash')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn('closed right away', self.read('hearth-server-mode-failed'))

    def test_the_dashboard_giving_up_opens_game_mode(self):
        self.run_block(LABWC_DO='gaveup')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn('keeps closing', self.read('hearth-server-mode-failed'))

    def test_play_switches_to_game_mode_and_leaves_the_way_back(self):
        self.run_block(LABWC_DO='up-game')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIsNone(self.read('hearth-server-mode-failed'))
        self.assertIsNotNone(self.read('hearth-server-return'), 'leaving Game Mode comes back to Server mode')
        self.assertIn('leaving', open(self.calls).read(), 'power limits back to how they were, every time')
        self.assertEqual(open(os.path.join(self.home, '.local/state/nimbus/server-mode-fails')).read().strip(), '',
                         'a switch you asked for is not a failure')

    def test_desktop_from_the_menu_leaves_no_way_back(self):
        self.run_block(LABWC_DO='up-desktop')
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.assertIsNone(self.read('hearth-server-return'))

    def test_coming_back_turns_the_mark_into_the_back_panel(self):
        open(os.path.join(self.run_dir, 'hearth-server-return'), 'w').close()
        self.run_block(LABWC_DO='up-logout')
        self.assertIsNotNone(self.read('hearth-server-back'), '"Back in Server mode" shows once')
        self.assertIsNone(self.read('hearth-server-return'))

    def test_two_failures_pause_server_mode_until_you_open_it(self):
        for _ in range(2):
            self.run_block(LABWC_DO='crash')
        out = self.run_block('server_start_mode', LABWC_DO='crash').stdout.strip()
        self.assertEqual(out, 'game')
        self.assertIn('paused', self.log)
        out = self.run_block('server_unpause; server_start_mode').stdout.strip()
        self.assertEqual(out, 'server', 'a request you make ends the pause')

    def test_start_mode_honours_recovery_and_the_kernel_line(self):
        cmd = os.path.join(self.tmp, 'cmdline')
        open(cmd, 'w').write('quiet hearth.mode=desktop\n')
        self.assertEqual(self.run_block('server_start_mode', HEARTH_CMDLINE=cmd).stdout.strip(), 'desktop')
        self.assertEqual(self.run_block('server_start_mode').stdout.strip(), 'server')
        self.assertEqual(self.run_block('server_start_mode', FAKE_ON='off').stdout.strip(), 'game')

    def test_the_way_back_only_while_server_mode_did_not_fail(self):
        open(os.path.join(self.run_dir, 'hearth-server-return'), 'w').close()
        self.assertIn('yes', self.run_block('server_return && echo yes').stdout)
        open(os.path.join(self.run_dir, 'hearth-server-mode-failed'), 'w').write('x\n')
        self.assertNotIn('yes', self.run_block('server_return && echo yes').stdout, 'never back and forth by itself')

    def test_steam_game_mode_failing_goes_back_to_server_mode_only_with_the_mark(self):
        def steam_back(**env):
            script = os.path.join(self.tmp, 'sb.sh')
            with open(script, 'w') as f:
                f.write('RUN=%s\nNEXT="$RUN/nimbus-next"\nLOG=/dev/null\nSHARE=%s\n' % (self.run_dir, self.share))
                f.write('log() { :; }\nhave() { command -v "$1" >/dev/null 2>&1; }\nlegacy_game() { false; }\n')
                f.write(block('steam game mode') + block('server mode') + 'steam_back; steam_back_name\n')
            return subprocess.run(['/bin/sh', script], env=self.env(**env), capture_output=True, text=True, timeout=30).stdout.split()
        self.assertEqual(steam_back()[0], 'desktop', 'without Server mode: exactly as before')
        open(os.path.join(self.run_dir, 'hearth-server-return'), 'w').close()
        out = steam_back()
        self.assertEqual(out[0], 'server')
        self.assertIn('Server', out)


class WholeSession(SessionBase):
    """nimbus-session for real (stand-ins for labwc and the helper; no Steam or gamescope here)."""

    def session(self, conf, **env):
        os.makedirs(os.path.join(self.home, '.config/nimbus'), exist_ok=True)
        with open(os.path.join(self.home, '.config/nimbus/session.conf'), 'w') as f:
            f.write(conf)
        r = subprocess.run(['sh', str(SESSION)], env=self.env(**env), capture_output=True, text=True, timeout=90)
        log = pathlib.Path(self.home, '.local/share/nimbus/session.log')
        return r, log.read_text() if log.exists() else ''

    def setUp(self):
        super().setUp()
        for tool in ('steam', 'gamescope'):
            if shutil.which(tool):
                self.skipTest(tool + ' is installed here')

    def test_the_default_start_never_touches_server_mode(self):
        r, log = self.session('', LABWC_DO='')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertNotIn('server mode', log.lower())
        self.assertFalse(os.path.exists(self.calls), 'the helper is never even asked')
        self.assertEqual(self.labwc_starts(), ['start server=0'], 'Game Mode (no Steam here) -> Desktop Mode, as before')

    def test_server_mode_asked_for_while_off_starts_game_mode(self):
        r, log = self.session('NIMBUS_START_MODE=server\n', FAKE_ON='off', FAKE_START='game', LABWC_DO='')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertNotIn('start server=1', self.labwc_starts())
        self.assertIn('steam game mode', log)

    def test_play_then_steam_failing_comes_back_to_server_mode_then_logout(self):
        r, log = self.session('NIMBUS_START_MODE=server\n', LABWC_DO='up-game,up-logout')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertEqual(self.labwc_starts(), ['start server=1', 'start server=1'], log)
        self.assertIn('Server mode instead', log, 'Steam Game Mode not starting goes back to Server mode (the mark)')
        self.assertIn('server mode ended (code 0); logout asked for', log)
        self.assertIn('leaving the session', log)

    def test_server_mode_crashing_always_ends_in_game_mode(self):
        r, log = self.session('NIMBUS_START_MODE=server\n', LABWC_DO='crash')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertIn('Game Mode instead', log)
        self.assertIn('steam game mode', log)
        # Steam isn't here, so Game Mode falls back to Desktop Mode (never back to the Server mode that failed).
        self.assertEqual(self.labwc_starts(), ['start server=1', 'start server=0'], log)


class NimbusMode(SessionBase):
    def test_server_is_a_mode_and_current_knows_it(self):
        e = self.env()
        subprocess.run(['sh', str(MODE), 'server'], env=e, capture_output=True, text=True, timeout=20)
        self.assertEqual(self.read('nimbus-next'), 'server')
        e['HEARTH_SERVER_MODE'] = '1'
        e['LABWC_PID'] = '999999'
        self.assertEqual(subprocess.run(['sh', str(MODE), 'current'], env=e, capture_output=True, text=True).stdout.strip(), 'server')
        self.assertIn('server', subprocess.run(['sh', str(MODE), 'nope'], env=e, capture_output=True, text=True).stderr)

    def test_autostart_gives_up_after_three_quick_closes(self):
        q = os.path.join(self.tmp, 'q.qml')
        open(q, 'w').write('//\n')
        self.write(os.path.join(self.bin, 'quickshell'), '#!/bin/sh\nexit 1\n')
        self.write(os.path.join(self.bin, 'labwc'), '#!/bin/sh\n[ "$1" = --exit ] && touch "$HOME/labwc-exit"\nexit 0\n')
        e = self.env(HEARTH_SERVER_QML=q, LABWC_PID=str(os.getpid()))
        subprocess.run(['sh', str(AUTOSTART)], env=e, capture_output=True, timeout=30)
        for _ in range(100):
            if self.read('hearth-server-dash-failed'):
                break
            time.sleep(0.1)
        self.assertIn('keeps closing', self.read('hearth-server-dash-failed') or '')
        self.assertTrue(os.path.exists(os.path.join(self.home, 'labwc-exit')), 'labwc is ended: the session opens Game Mode')


# ------------------------------------------------------------------ the helper

class Helper(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp(prefix='hearth-server-helper-')
        self.addCleanup(shutil.rmtree, self.d, True)
        self.run_dir = os.path.join(self.d, 'run')
        os.makedirs(self.run_dir, mode=0o700)
        env = {'XDG_CONFIG_HOME': os.path.join(self.d, 'config'), 'XDG_STATE_HOME': os.path.join(self.d, 'state'),
               'XDG_RUNTIME_DIR': self.run_dir, 'HEARTH_HOME_SERVER_SERVICES_STATE': os.path.join(self.d, 'services.json'),
               'HEARTH_HOME_SERVER_PLAN': os.path.join(self.d, 'plan.json'), 'HEARTH_NIMBUS_MODE': '/bin/true'}
        self.m = load_helper(env)
        self.calls = []
        self.m.run = lambda cmd, timeout=10, env=None, inp=None: (self.calls.append(cmd) or (0, '', ''))
        self.m.spawn = lambda args: self.calls.append(['spawn'] + args)

    def settings(self):
        p = self.m.SETTINGS
        return json.load(open(p)) if os.path.exists(p) else None

    def test_the_ai_chat_core_is_named_after_the_loaded_model(self):
        self.assertEqual(self.m.pretty_model('qwen3:30b-a3b'), 'Qwen3 30B')
        self.assertEqual(self.m.pretty_model('huihui_ai/qwen3-abliterated:14b'), 'Qwen3-abliterated 14B')
        self.assertEqual(self.m.pretty_model('llava'), 'Llava')
        self.assertEqual(self.m.pretty_model(''), 'AI chat')

    def test_the_schedules_next_switch_is_shown_and_nothing_when_its_off(self):
        sched = {'on': True, 'weekday': [{'at': '00:00', 'mode': 'server'}, {'at': '18:00', 'mode': 'game'}],
                 'weekend': [{'at': '01:00', 'mode': 'server'}, {'at': '10:00', 'mode': 'game'}]}
        tue = time.strptime('2026-11-10 02:14', '%Y-%m-%d %H:%M')
        self.assertEqual(self.m.next_change(sched, tue)['text'], 'Game Mode at 6:00 PM')
        late = time.strptime('2026-11-10 19:00', '%Y-%m-%d %H:%M')
        self.assertEqual(self.m.next_change(sched, late)['text'], 'Server mode at 12:00 AM tomorrow')
        fri = time.strptime('2026-11-13 19:00', '%Y-%m-%d %H:%M')
        self.assertEqual(self.m.next_change(sched, fri)['text'], 'Server mode at 1:00 AM tomorrow', 'Saturday is the weekend')
        self.assertIsNone(self.m.next_change(dict(sched, on=False), tue))
        self.assertIsNone(self.m.next_change(None, tue))
        self.assertEqual(self.calls, [], 'reading the schedule runs nothing')

    def test_the_web_pages_qr_code_is_made_once_and_only_when_the_page_is_on(self):
        self.assertEqual(self.m.web_qr(False), '')
        made = []
        self.m.shutil = type('S', (), {'which': staticmethod(lambda n: '/usr/bin/' + n)})
        self.m.run = lambda cmd, timeout=10, env=None, inp=None: (made.append(cmd) or (0, '<svg>qr</svg>', ''))
        p = self.m.web_qr(True)
        self.assertTrue(p.startswith(self.run_dir) and open(p).read() == '<svg>qr</svg>')
        self.assertEqual(oct(os.stat(os.path.dirname(p)).st_mode & 0o777), '0o700')
        self.assertEqual(self.m.web_qr(True), p)
        self.assertEqual(len(made), 1, 'made once for the same address')
        self.assertEqual(made[0][:3], ['qrencode', '-t', 'SVG'])

    def test_settings_are_written_only_when_changed_and_never_with_defaults(self):
        self.assertFalse(self.m.set_key('dim', '2'), 'the default already')
        self.assertIsNone(self.settings(), 'nothing written for a default')
        self.assertTrue(self.m.set_key('on', 'on'))
        self.assertEqual(self.settings(), {'on': True})
        self.assertTrue(self.m.set_key('permPower', 'off'))
        self.assertEqual(self.settings(), {'on': True, 'web': {'perms': {'power': False}}}, 'only the key you changed')
        self.assertTrue(self.m.load()['web']['perms']['services'], 'the rest still default')
        for k, v in (('on', 'yes'), ('gpuWatts', '999'), ('dim', '3'), ('nope', 'on'), ('off', '-1')):
            with self.assertRaises(self.m.Fail):
                self.m.set_key(k, v)

    def test_a_damaged_file_is_never_overwritten_with_defaults(self):
        os.makedirs(os.path.dirname(self.m.SETTINGS), exist_ok=True)
        open(self.m.SETTINGS, 'w').write('{"on": tr')
        self.assertIs(self.m.peek()['on'], False)
        self.assertEqual(open(self.m.SETTINGS).read(), '{"on": tr', 'reading never repairs over it')

    def test_the_schedule(self):
        with self.assertRaises(self.m.Fail):
            self.m.schedule_set(json.dumps({'weekday': [{'at': '25:00', 'mode': 'server'}], 'weekend': [{'at': '01:00', 'mode': 'game'}]}))
        with self.assertRaises(self.m.Fail):
            self.m.schedule_set(json.dumps({'weekday': [{'at': '01:00', 'mode': 'steam'}], 'weekend': [{'at': '01:00', 'mode': 'game'}]}))
        self.assertTrue(self.m.schedule_set(json.dumps({'weekday': [{'at': '17:30', 'mode': 'game'}, {'at': '00:00', 'mode': 'server'}],
                                                        'weekend': [{'at': '10:00', 'mode': 'game'}, {'at': '01:00', 'mode': 'server'}]})))
        s = self.m.load()['schedule']
        self.assertEqual([c['at'] for c in s['weekday']], ['00:00', '17:30'], 'kept in order')
        self.assertFalse(s['on'], 'saving a schedule never turns it on')
        sched = dict(s, on=True)

        def at(wday, h, mi=0):
            return time.struct_time((2026, 11, 2 + wday, h, mi, 0, wday, 306, 0))
        self.assertEqual(self.m.schedule_mode(sched, at(4, 14, 14)), 'server')
        self.assertEqual(self.m.schedule_mode(sched, at(4, 18)), 'game')
        self.assertEqual(self.m.schedule_mode(sched, at(5, 0, 30)), 'game', 'before the first change: the day before\'s last')
        self.assertEqual(self.m.schedule_mode(sched, at(5, 9)), 'server')
        self.assertEqual(self.m.schedule_mode(dict(sched, on=False), at(4, 18)), '')

    def test_start_mode(self):
        self.assertEqual(self.m.start_mode('server'), 'game', 'Server mode asked for while it is off: Game Mode')
        self.assertEqual(self.m.start_mode('desktop'), 'desktop')
        self.assertEqual(self.m.start_mode('rubbish'), 'game')
        self.m.set_key('on', 'on')
        self.assertEqual(self.m.start_mode('server'), 'server')
        self.m.set_key('scheduleOn', 'on')
        self.assertIn(self.m.start_mode('desktop'), ('server', 'game'), 'the schedule picks')

    def test_start_in_changes_only_its_line(self):
        os.makedirs(os.path.dirname(self.m.SESSION_CONF), exist_ok=True)
        open(self.m.SESSION_CONF, 'w').write('NIMBUS_GAMESCOPE_ARGS="-W 1280"\nNIMBUS_START_MODE=desktop\nFOO=1\n')
        self.assertEqual(self.m.start_in(), 'desktop')
        self.m.start_in('server')
        self.assertEqual(open(self.m.SESSION_CONF).read(), 'NIMBUS_GAMESCOPE_ARGS="-W 1280"\nNIMBUS_START_MODE=server\nFOO=1\n')
        self.assertTrue(os.path.exists(self.m.SESSION_CONF + '.bak'))
        with self.assertRaises(self.m.Fail):
            self.m.start_in('steam; rm -rf /')

    def test_the_pin_is_kept_only_as_a_hash_and_a_new_pin_signs_everyone_out(self):
        for bad in ('123', 'abcd', '1234567890123', '12 34'):
            with self.assertRaises(self.m.Fail):
                self.m.pin_set(bad)
        self.m.pin_set('482916')
        raw = open(self.m.WEB_STATE).read()
        self.assertNotIn('482916', raw)
        self.assertEqual(os.stat(self.m.WEB_STATE).st_mode & 0o777, 0o600)
        h = json.loads(raw)['pin']
        self.assertTrue(self.m.pin_check('482916', h))
        self.assertFalse(self.m.pin_check('482917', h))
        tok = self.m.session_new('ROG Ally')
        self.assertTrue(self.m.session_ok(tok))
        self.assertNotIn(tok, open(self.m.WEB_STATE).read(), 'sign-ins are kept as hashes too')
        self.m.pin_set('111111')
        self.assertFalse(self.m.session_ok(tok))
        self.assertFalse(self.m.session_ok('../../etc'))

    def test_only_tailscale_and_this_pc_may_open_the_page(self):
        for ip in ('127.0.0.1', '::1', '100.101.102.103', '100.64.0.1', 'fd7a:115c:a1e0::1', '::ffff:100.70.1.1'):
            self.assertTrue(self.m.peer_allowed(ip), ip)
        for ip in ('192.168.1.20', '10.0.0.5', '8.8.8.8', '100.128.0.1', 'fe80::1', '2001:db8::1', 'garbage'):
            self.assertFalse(self.m.peer_allowed(ip), ip)
        self.assertEqual(self.m.listen_addresses.__defaults__, None)
        self.m.ts_ip4 = lambda st=None: '100.72.1.2'
        self.assertEqual(self.m.listen_addresses(), ['127.0.0.1', '100.72.1.2'], 'never 0.0.0.0')

    def test_requests_are_checked_again(self):
        for bad in ({'kind': 'mode', 'to': 'steam'}, {'kind': 'power', 'what': 'off'}, {'kind': 'service', 'name': 'sshd', 'action': 'restart'},
                    {'kind': 'service', 'name': 'ai', 'action': 'rm'}, {'kind': 'shell', 'cmd': 'id'}, [], None):
            with self.assertRaises(self.m.Fail):
                self.m.clean_request(bad)
        self.assertEqual(self.m.clean_request({'kind': 'mode', 'to': 'game', 'extra': 1}), {'kind': 'mode', 'to': 'game'})

    def mode_now(self, m, dash=False):
        open(self.m.MODE_NOW, 'w').write(m + '\n')
        if dash:
            open(self.m.PID, 'w').write(str(os.getpid()))
            open(self.m.DASH_UP, 'w').close()

    def test_a_web_switch_counts_down_on_the_home_screen_and_can_be_cancelled(self):
        self.mode_now('server', dash=True)
        self.assertEqual(self.m.handle({'kind': 'mode', 'to': 'game', 'from': 'web', 'by': 'Work laptop'}), 'countdown')
        c = self.m.countdown_view()
        self.assertEqual((c['to'], c['by'], c['total']), ('game', 'Work laptop', 10))
        self.assertEqual(self.m.cancel(), {'cancelled': True})
        self.m.agent_step(now=time.time() + 60)
        self.assertFalse([x for x in self.calls if x and x[0] == '/bin/true'], 'cancelled: nothing switched')

    def test_the_countdown_switches_at_its_deadline(self):
        self.mode_now('server', dash=True)
        self.m.handle({'kind': 'mode', 'to': 'game', 'from': 'web'})
        self.m.agent_step(now=time.time() + 1)
        self.assertFalse([x for x in self.calls if x[:1] == ['/bin/true']], 'not before the 10 seconds')
        self.m.agent_step(now=time.time() + 11)
        self.assertIn(['/bin/true', 'game'], self.calls)
        self.assertIsNone(self.m.countdown_view())

    def test_never_while_a_game_runs_or_away_from_desktop_mode(self):
        self.mode_now('desktop')
        self.assertEqual(self.m.handle({'kind': 'mode', 'to': 'server'}), 'refused')
        self.mode_now('steam')
        self.m.game_running = lambda: True
        self.assertEqual(self.m.handle({'kind': 'mode', 'to': 'server'}), 'wait')
        self.assertEqual(self.m.handle({'kind': 'power', 'what': 'sleep'}), 'wait')
        self.m.game_running = lambda: False
        self.assertEqual(self.m.handle({'kind': 'mode', 'to': 'server'}), 'done')
        self.assertIn(['/bin/true', 'server'], self.calls)
        self.assertEqual(self.m.handle({'kind': 'mode', 'to': 'game'}), 'already')

    def test_the_schedule_acts_only_at_its_changes_and_retries_while_a_game_runs(self):
        s = {'on': True, 'schedule': dict(self.m.DEFAULTS['schedule'], on=True)}
        t1 = time.struct_time((2026, 11, 6, 17, 59, 0, 4, 310, 0))
        t2 = time.struct_time((2026, 11, 6, 18, 1, 0, 4, 310, 0))
        self.mode_now('steam')
        self.assertEqual(self.m.schedule_step(s, None, t1), 'server', 'the first look only remembers')
        self.assertEqual(self.m.schedule_step(s, 'server', t1), 'server')
        self.assertFalse(self.calls, 'no change: nothing happens (a switch you made stays)')
        self.mode_now('server', dash=True)
        self.assertEqual(self.m.schedule_step(s, 'server', t2), 'game')
        self.assertEqual(self.m.countdown_view()['from'], 'schedule')
        os.remove(self.m.COUNTDOWN)
        self.mode_now('steam')
        self.m.game_running = lambda: True
        s2 = {'on': True, 'schedule': {'on': True, 'weekday': [{'at': '00:00', 'mode': 'game'}, {'at': '18:00', 'mode': 'server'}],
                                       'weekend': [{'at': '00:00', 'mode': 'game'}]}}
        self.assertEqual(self.m.schedule_step(s2, 'game', t2), 'game', 'a game runs: tried again next time')

    def test_services_only_known_names_and_a_pause_is_remembered(self):
        with self.assertRaises(self.m.Fail):
            self.m.service('restart', 'sshd')
        self.m.service('pause', 'pictures')
        self.assertIn(['systemctl', '--user', 'stop', 'hearth-comfyui.service'], self.calls)
        self.assertEqual(self.m.user_paused(), ['pictures'])
        self.m.service('resume', 'pictures')
        self.assertEqual(self.m.user_paused(), [])

    def test_go_puts_power_back_first_and_nudges_light_mode(self):
        os.makedirs(self.m.RT, exist_ok=True)
        json.dump({'on': True, 'mode': 'balanced', 'gpu': True}, open(self.m.IDLE_STATE, 'w'))
        self.m.shutil = type('S', (), {'which': staticmethod(lambda n: '/usr/bin/' + n)})
        self.m.go('game')
        i_reset = next(i for i, c in enumerate(self.calls) if c[-2:] == ['gpu-limit', 'reset'])
        i_go = self.calls.index(['/bin/true', 'game'])
        self.assertLess(i_reset, i_go, 'full power before Game Mode')
        self.assertIn(['spawn', 'nudge', '--wait-game'], self.calls)
        with self.assertRaises(self.m.Fail):
            self.m.go('steam')

    def test_the_dashboard_and_summary_from_a_fixture(self):
        fx = os.path.join(self.d, 'fx.json')
        json.dump({'host': 'chris-hearth', 'tiles': [{'id': 'ai', 'title': 'AI chat', 'state': 'running', 'chip': 'Running'}],
                   'status': {'level': 'ok', 'title': 'All good', 'sub': '8 services running'}, 'devices': 2}, open(fx, 'w'))
        self.m.FIXTURE = fx
        s = self.m.summary()
        self.assertEqual(s['host'], 'chris-hearth')
        self.assertEqual(s['services'][0]['id'], 'ai')
        page = self.m.page_html(s)
        self.assertIn('"chris-hearth"', page)
        self.assertNotIn('/*HEARTH_DATA*/null', page)
        self.assertNotIn('</script><script', json.dumps(page_bits(page)))

    def test_ai_small_follows_server_mode_until_you_choose(self):
        self.assertFalse(self.m.ai_small_effective({'ai': {}}, {'on': False}))
        self.assertTrue(self.m.ai_small_effective({'ai': {}}, {'on': True}))
        self.assertFalse(self.m.ai_small_effective({'ai': {'small': False}}, {'on': True}), 'your choice wins')
        p = self.m.pauses_view({'ai': {'on': True}, 'lightMode': True}, {'on': True})
        self.assertEqual(p['ai'], 'small')
        self.assertEqual(self.m.pauses_view({'ai': {'on': True, 'pause': True}}, {'on': True})['ai'], 'stop')


def page_bits(page):
    return re.findall(r'const EMBED = (.*?);\n', page)[:1]


class WebPage(unittest.TestCase):
    """The web server itself, on 127.0.0.1 and a free port."""

    def setUp(self):
        self.d = tempfile.mkdtemp(prefix='hearth-server-web-')
        self.addCleanup(shutil.rmtree, self.d, True)
        run_dir = os.path.join(self.d, 'run')
        os.makedirs(run_dir, mode=0o700)
        fx = os.path.join(self.d, 'fx.json')
        json.dump({'host': 'chris-hearth', 'tiles': [], 'status': {'level': 'ok', 'title': 'All good', 'sub': ''}}, open(fx, 'w'))
        self.m = load_helper({'XDG_CONFIG_HOME': os.path.join(self.d, 'c'), 'XDG_STATE_HOME': os.path.join(self.d, 's'),
                              'XDG_RUNTIME_DIR': run_dir, 'HEARTH_SERVER_DASH_FIXTURE': fx})
        self.m.ts_status = lambda: {}
        self.m.spawn = lambda args: None
        self.m.pin_set('2468')
        from http.server import ThreadingHTTPServer
        self.srv = ThreadingHTTPServer(('127.0.0.1', 0), self.m.make_handler())
        self.port = self.srv.server_address[1]
        threading.Thread(target=self.srv.serve_forever, daemon=True).start()
        self.addCleanup(self.srv.server_close)
        self.addCleanup(self.srv.shutdown)

    def req(self, method, path, body=None, headers=None, cookie=''):
        c = http.client.HTTPConnection('127.0.0.1', self.port, timeout=10)
        h = {'Content-Type': 'application/json'}
        if cookie:
            h['Cookie'] = cookie
        h.update(headers or {})
        c.request(method, path, json.dumps(body) if body is not None else None, h)
        r = c.getresponse()
        data = r.read()
        c.close()
        return r.status, r.getheader('Set-Cookie') or '', data

    def login(self, pin='2468'):
        return self.req('POST', '/api/login', {'pin': pin}, {'X-Hearth': '1'})

    def test_nothing_without_the_pin(self):
        self.assertEqual(self.req('GET', '/api/status')[0], 401)
        self.assertEqual(self.req('POST', '/api/action', {'kind': 'mode', 'to': 'game'}, {'X-Hearth': '1'})[0], 401)
        st, _, page = self.req('GET', '/')
        self.assertEqual(st, 200)
        self.assertIn(b'PIN', page)

    def test_writes_need_this_page_and_a_wrong_pin_is_limited(self):
        self.assertEqual(self.req('POST', '/api/login', {'pin': '2468'})[0], 403, 'no X-Hearth header: not from this page')
        self.assertEqual(self.req('POST', '/api/login', {'pin': '2468'}, {'X-Hearth': '1', 'Origin': 'http://evil.example'})[0], 403)
        for _ in range(5):
            self.assertEqual(self.login('0000')[0], 401)
        self.assertEqual(self.login()[0], 429, 'five wrong PINs: wait 5 minutes')

    def test_signed_in_it_shows_status_and_sends_requests_the_pc_checks(self):
        st, cookie, _ = self.login()
        self.assertEqual(st, 200)
        self.assertIn('HttpOnly', cookie)
        self.assertIn('SameSite=Strict', cookie)
        ck = cookie.split(';')[0]
        st, _, data = self.req('GET', '/api/status', cookie=ck)
        self.assertEqual(st, 200)
        self.assertEqual(json.loads(data)['host'], 'chris-hearth')
        st, _, _ = self.req('POST', '/api/action', {'kind': 'mode', 'to': 'game'}, {'X-Hearth': '1'}, ck)
        self.assertEqual(st, 202)
        self.assertEqual(len(os.listdir(self.m.REQUESTS)), 1, 'a request for the agent, nothing run here')
        self.assertEqual(self.req('POST', '/api/action', {'kind': 'shell', 'cmd': 'id'}, {'X-Hearth': '1'}, ck)[0], 400)
        self.m.set_key('permModes', 'off')
        self.assertEqual(self.req('POST', '/api/action', {'kind': 'mode', 'to': 'desktop'}, {'X-Hearth': '1'}, ck)[0], 403)
        self.assertEqual(self.req('POST', '/api/logout', {}, {'X-Hearth': '1'}, ck)[0], 200)
        self.assertEqual(self.req('GET', '/api/status', cookie=ck)[0], 401)

    def test_the_summary_for_other_hearth_devices_can_be_turned_off(self):
        self.assertEqual(self.req('GET', '/api/summary')[0], 200, 'read only, on by default (Tailscale devices only)')
        self.m.set_key('permDevices', 'off')
        self.assertEqual(self.req('GET', '/api/summary')[0], 401)

    def test_other_addresses_are_refused(self):
        h = self.m.make_handler()
        self.assertFalse(self.m.peer_allowed('192.168.0.9'))
        self.assertTrue(callable(getattr(h, 'guard')))


class SettingAPin(WebPage):
    """"Server mode's web page won't let me set a PIN" (1.6 and 1.7): every way setting or typing the PIN could fail, and
    that each failure says what to do. The real cause: the page sent Referrer-Policy: no-referrer, so browsers sent
    "Origin: null" with every POST and the PC refused the PIN as "Not from this page"."""

    def browser_login(self, pin='2468', **headers):
        h = {'X-Hearth': '1'}
        h.update({k.replace('_', '-'): v for k, v in headers.items()})
        return self.req('POST', '/api/login', {'pin': pin}, h)

    def test_the_page_no_longer_makes_browsers_send_origin_null(self):
        st, _, page = self.req('GET', '/')
        c = http.client.HTTPConnection('127.0.0.1', self.port, timeout=10)
        c.request('GET', '/')
        r = c.getresponse()
        r.read()
        c.close()
        self.assertEqual(r.getheader('Referrer-Policy'), 'same-origin', 'no-referrer makes every POST say Origin: null')
        self.assertIn(b"referrerPolicy:'same-origin'", page, 'the page asks for its real Origin on each request too')

    def test_a_browser_signing_in_the_way_1_6_and_1_7_pages_did(self):
        # What Chrome and Firefox send for a POST from a page under Referrer-Policy: no-referrer.
        self.assertEqual(self.browser_login(Origin='null')[0], 200)
        self.assertEqual(self.browser_login(Origin='null', Sec_Fetch_Site='same-origin')[0], 200)

    def test_any_tailscale_name_or_address_works(self):
        for host in ('chris-hearth:8460', 'chris-hearth.tail1234.ts.net:8460', '100.101.102.103:8460', 'CHRIS-HEARTH:8460'):
            st = self.browser_login(Host=host, Origin='http://' + host.lower(), Sec_Fetch_Site='same-origin')[0]
            self.assertEqual(st, 200, host)
            st = self.browser_login(Host=host, Origin='http://' + host.lower())[0]
            self.assertEqual(st, 200, host + ' (a browser without Sec-Fetch-Site)')
        # Through tailscale serve (https, Host rewritten): the browser still says it's the same page.
        self.assertEqual(self.browser_login(Host='127.0.0.1:8460', Origin='https://chris-hearth.tail1234.ts.net',
                                            Sec_Fetch_Site='same-origin')[0], 200)

    def test_other_sites_still_cant_press_its_buttons(self):
        self.assertEqual(self.req('POST', '/api/login', {'pin': '2468'}, {'Origin': 'null'})[0], 403, 'no X-Hearth: refused')
        self.assertEqual(self.browser_login(Origin='http://evil.example', Sec_Fetch_Site='cross-site')[0], 403)
        self.assertEqual(self.browser_login(Origin='null', Sec_Fetch_Site='cross-site')[0], 403)
        self.assertEqual(self.browser_login(Sec_Fetch_Site='same-site')[0], 403)
        st, _, data = self.browser_login(Origin='http://evil.example')
        self.assertEqual(st, 403)
        self.assertIn('Reload the page', json.loads(data)['error'], 'says what to do')

    def test_typing_mistakes_say_what_to_do_and_dont_count_as_wrong_pins(self):
        for typed in ('24', '24a8', '', '1234567890123'):
            st, _, data = self.browser_login(typed)
            self.assertEqual(st, 400, typed)
            self.assertIn('4 to 12 digits', json.loads(data)['error'])
        self.assertEqual(self.browser_login(' 2468 ')[0], 200, 'spaces around it are fine')
        self.assertEqual(self.browser_login('２４６８')[0], 200, "a phone keyboard's full-width digits are fine")
        st, _, data = self.browser_login('1357')
        self.assertEqual(st, 401)
        self.assertIn('Settings › Home server › Server mode', json.loads(data)['error'])

    def test_no_pin_yet_says_where_to_set_it(self):
        self.m.pin_clear()
        st, _, data = self.browser_login()
        self.assertEqual(st, 403)
        self.assertIn('PIN for the web page', json.loads(data)['error'])
        st, _, data = self.req('GET', '/api/status')
        self.assertEqual(st, 401)
        self.assertIs(json.loads(data)['pin'], False)
        self.assertIn('PIN for the web page', json.loads(data)['error'])


class SettingAPinOnThePc(unittest.TestCase):
    """Settings › Home server › Server mode › PIN for the web page: the helper's side, as Settings runs it."""

    def setUp(self):
        self.d = tempfile.mkdtemp(prefix='hearth-server-pin-')
        self.addCleanup(shutil.rmtree, self.d, True)
        os.chmod(self.d, 0o700)
        run_dir = os.path.join(self.d, 'run')
        os.makedirs(run_dir, mode=0o700)
        self.env = {'XDG_CONFIG_HOME': os.path.join(self.d, 'config'), 'XDG_STATE_HOME': os.path.join(self.d, 'state'),
                    'XDG_RUNTIME_DIR': run_dir, 'HEARTH_HOME_SERVER_PLAN': os.path.join(self.d, 'plan.json'),
                    'HEARTH_NIMBUS_MODE': '/bin/true'}
        self.m = load_helper(self.env)

    def cli(self, *args, inp=None):
        env = dict(os.environ, **self.env)
        env.pop('HEARTH_INPUT', None)
        if inp is not None:
            env['HEARTH_INPUT'] = inp
        p = subprocess.run([sys.executable, str(HELPER)] + list(args), capture_output=True, text=True, env=env,
                           input=inp or '', timeout=60)
        return p.returncode, p.stdout.strip()

    def test_settings_way_of_setting_it(self):
        # SetJob: the PIN in HEARTH_INPUT (and on stdin), "error: ..." on failure.
        self.assertEqual(self.cli('pin-set', inp='482916'), (0, '{"pin": true}'))
        self.assertEqual(self.cli('status')[0], 0)
        self.assertEqual(oct(os.stat(self.m.WEB_STATE).st_mode & 0o777), '0o600')
        code, o = self.cli('pin-set', inp='48a9')
        self.assertEqual(code, 1)
        self.assertTrue(o.startswith('error: ') and '4 to 12 digits' in o, o)
        self.assertTrue(self.m.pin_check('482916', json.load(open(self.m.WEB_STATE))['pin']), 'a bad PIN changes nothing')

    def test_what_settings_and_phones_type(self):
        for typed in ('482916\n', ' 482916 ', '４８２９１６'):
            self.m.pin_set(typed)
            self.assertTrue(self.m.pin_check('482916', json.load(open(self.m.WEB_STATE))['pin']), repr(typed))
        msgs = {}
        for typed in ('', '123', '12 34', 'abcd', '1234567890123', '१२३'):
            with self.assertRaises(self.m.Fail) as c:
                self.m.pin_set(typed)
            msgs[typed] = str(c.exception)
            self.assertIn('4 to 12 digits', msgs[typed], typed)
        self.assertIn('too short (3 digits)', msgs['123'])
        self.assertIn('spaces', msgs['12 34'])
        self.assertIn('numbers only', msgs['abcd'])
        self.assertIn('too long', msgs['1234567890123'])

    def test_the_settings_page_and_the_helper_agree(self):
        qml = (SHELL / 'SetServerMode.qml').read_text()
        self.assertIn('SetPinPad', qml, 'a PIN pad a controller can type on (no on-screen keyboard in Server mode)')
        self.assertNotIn('\\d{4,12}', qml)
        self.assertIn('/^[0-9]{4,12}$/', qml)
        self.assertEqual(self.m.PIN_RE.pattern, '^[0-9]{4,12}$')
        pad = (SHELL / 'SetPinPad.qml').read_text()
        self.assertIn('maxLength: 12', qml)
        for k in ('Qt.Key_Left', 'Qt.Key_Right', 'Qt.Key_Up', 'Qt.Key_Down', 'Qt.Key_Backspace', 'Qt.Key_Return'):
            self.assertIn(k, pad)

    @unittest.skipIf(os.geteuid() == 0, 'root can write anywhere')
    def test_a_folder_that_isnt_yours_says_how_to_fix_it(self):
        d = os.path.dirname(self.m.WEB_STATE)
        os.makedirs(d)
        os.chmod(d, 0o500)
        self.addCleanup(os.chmod, d, 0o700)
        with self.assertRaises(self.m.Fail) as c:
            self.m.pin_set('482916')
        self.assertIn('chown', str(c.exception))
        self.assertIn('set the PIN again', str(c.exception))

    def test_the_web_page_not_starting_is_reported(self):
        self.m.set_key('webOn', 'on')
        self.m.pin_set('482916')
        calls = []

        def run(cmd, timeout=10, env=None, inp=None):
            calls.append(cmd)
            return (1, '', 'Failed to restart hearth-server-mode-web.service: Unit not found.') if 'restart' in cmd or 'enable' in cmd else (0, '', '')
        self.m.run = run
        self.m.spawn = lambda args: None
        with self.assertRaises(self.m.Fail) as c:
            self.m.web_apply()
        self.assertIn('Unit not found', str(c.exception))
        self.assertIn('off and on again', str(c.exception))
        self.m.run = lambda cmd, timeout=10, env=None, inp=None: (0, '', '')
        self.assertEqual(self.m.web_apply(), {'web': 'on'})


# ------------------------------------------------------------------ the root helper's part

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from test_hearth_home_server import hh, hs  # noqa: E402
from test_hearth_home_server_3 import Stage3  # noqa: E402


class RootPart(Stage3):
    def setUp(self):
        super().setUp()
        for k in ('COMFY_QUEUE', 'GPU_LIMIT_STATE'):
            self.saved[k] = getattr(hh, k)
        hh.COMFY_QUEUE = os.path.join(self.etc, 'run/comfyui-queue.json')
        hh.GPU_LIMIT_STATE = os.path.join(self.etc, 'run/gpu-limit')
        for k in ('comfy_api', 'user_systemctl', 'game_running', 'nvidia_power'):
            self.saved_fns.setdefault(k, getattr(hh, k))

    def plan(self, **kw):
        p = {'user': 'player', 'lightMode': True, 'ai': {'on': True, 'unload': True, 'small': True}, 'media': {'on': False},
             'pictures': {'on': False}, 'voices': {'on': False}}
        p.update(kw)
        os.makedirs(os.path.dirname(hh.PLAN), exist_ok=True)
        json.dump(p, open(hh.PLAN, 'w'))

    def test_the_web_page_port_is_only_for_tailscale(self):
        self.assertIn(8460, hh.services_plan({'serverWeb': True}, 'player')['ports']['tcp'])
        self.assertNotIn(8460, hh.services_plan({}, 'player')['ports']['tcp'])

    def test_the_small_model_on_the_processor_while_you_play_and_gone_after(self):
        self.plan()
        self.install('ollama')
        self.active.add('hearth-ollama.service')
        loaded = ['qwen3:30b-a3b']
        sent = []

        def api(m, p, b=None, timeout=10):
            sent.append((m, p, b))
            if p == '/api/ps':
                return {'models': [{'name': n} for n in loaded]}
            if p == '/api/tags':
                return {'models': [{'name': 'qwen3:30b-a3b'}, {'name': 'qwen3:14b'}]}
            if p == '/api/generate' and b.get('keep_alive') == 0:
                loaded.remove(b['model'])
            elif p == '/api/generate':
                loaded.append(b['model'])
            return {}
        hh.ollama_api = api
        hh.game_running = lambda: True
        r = hh.services_run(now=100)
        gens = [b for m, p, b in sent if p == '/api/generate']
        self.assertEqual(gens[0], {'model': 'qwen3:30b-a3b', 'keep_alive': 0}, 'the big one leaves the graphics card first')
        self.assertEqual(gens[1]['model'], 'qwen3:14b')
        self.assertEqual(gens[1]['options'], {'num_gpu': 0}, 'on the processor only')
        self.assertEqual(json.load(open(hh.SERVICES_STATE))['small'], 'qwen3:14b')
        self.assertTrue(any('small model' in x for x in r['did']))
        sent.clear()
        hh.services_run(now=160)
        self.assertFalse([b for m, p, b in sent if p == '/api/generate'], 'kept loaded, not reloaded every minute')
        hh.game_running = lambda: False
        hh.services_run(now=220)
        self.assertIn({'model': 'qwen3:14b', 'keep_alive': 0}, [b for m, p, b in sent if p == '/api/generate'])
        self.assertNotIn('small', json.load(open(hh.SERVICES_STATE)))
        self.assertFalse([1 for m, p, b in sent if p.startswith('/api/delete') or p.startswith('/api/pull')], 'never deleted or downloaded')

    def test_no_small_model_downloaded_means_nothing_is_loaded(self):
        self.plan()
        self.install('ollama')
        self.active.add('hearth-ollama.service')
        sent = []
        hh.ollama_api = lambda m, p, b=None, timeout=10: (sent.append(b) or ({'models': [{'name': 'qwen3:30b-a3b'}]} if p in ('/api/tags', '/api/ps') else {}))
        hh.game_running = lambda: True
        hh.services_run(now=1)
        self.assertFalse([b for b in sent if b and b.get('model') == 'qwen3:14b'])

    def test_comfyui_finishes_its_picture_keeps_its_queue_and_gets_it_back(self):
        self.plan(ai={'on': False}, pictures={'on': True, 'pause': True})
        calls, posted = [], []
        hh.user_systemctl = lambda user, action, unit: (calls.append((action, unit)) or ((0, '') if action == 'is-active' and not any(a == 'stop' for a, _ in calls) else (0, '')))
        queue = {'queue_running': [[5, 'id5', {'1': {'class_type': 'KSampler'}}, {'client_id': 'laptop'}, []]],
                 'queue_pending': [[7, 'id7', {'2': {}}, {'client_id': 'laptop'}, []], [6, 'id6', {'3': {}}, {}, []]]}
        hh.comfy_api = lambda m, p, b=None, timeout=5: (queue if p == '/queue' else {} if p == '/system_stats' else posted.append(b) or {})
        hh.game_running = lambda: True
        r = hh.services_run(now=1000)
        self.assertNotIn(('stop', 'hearth-comfyui.service'), calls, 'the picture being made finishes first')
        self.assertTrue(any('waiting for the picture' in x for x in r['did']))
        r = hh.services_run(now=1000 + hh.COMFY_WAIT + 1)
        self.assertIn(('stop', 'hearth-comfyui.service'), calls, 'not forever: the game gets the graphics card')
        kept = json.load(open(hh.COMFY_QUEUE))['items']
        self.assertEqual([list(k['prompt'])[0] for k in kept], ['1', '3', '2'], 'the running one first, then in order')
        self.assertEqual(os.stat(hh.COMFY_QUEUE).st_mode & 0o777, 0o600)
        hh.game_running = lambda: False
        hh.services_run(now=2000)
        self.assertEqual([list(b['prompt'])[0] for b in posted], ['1', '3', '2'], 'back in the queue after')
        self.assertEqual(posted[0]['client_id'], 'laptop')
        self.assertFalse(os.path.exists(hh.COMFY_QUEUE))

    def test_the_abliterated_14b_is_the_small_model_when_picked_and_downloaded(self):
        self.plan(ai={'on': True, 'unload': True, 'small': True, 'smallModel': 'huihui_ai/qwen3-abliterated:14b'})
        self.install('ollama')
        self.active.add('hearth-ollama.service')
        have = ['qwen3:30b-a3b', 'qwen3:14b']
        sent = []
        hh.ollama_api = lambda m, p, b=None, timeout=10: (sent.append((p, b)) or ({'models': [{'name': n} for n in have]} if p == '/api/tags' else {'models': []} if p == '/api/ps' else {}))
        hh.game_running = lambda: True
        hh.services_run(now=1)
        self.assertIn('qwen3:14b', [b['model'] for p, b in sent if p == '/api/generate' and b.get('options')], 'not downloaded: qwen3:14b instead')
        have.append('huihui_ai/qwen3-abliterated:14b')
        sent.clear()
        hh.services_run(now=61)
        self.assertIn('huihui_ai/qwen3-abliterated:14b', [b['model'] for p, b in sent if p == '/api/generate' and b.get('options')])
        self.assertEqual(hh.clean_ai({'smallModel': 'evil:1b'})['smallModel'], 'qwen3:14b', 'only the two choices')

    def test_svc_only_hearth_units_that_are_set_up(self):
        with self.assertRaises(hh.Fail):
            hh.svc('restart', 'sshd')
        with self.assertRaises(hh.Fail):
            hh.svc('enable', 'ollama')
        with self.assertRaises(hh.Fail):
            hh.svc('restart', 'ollama')     # not set up: no drop-in
        os.makedirs(os.path.dirname(hh.APP_DROPIN % 'hearth-ollama'), exist_ok=True)
        open(hh.APP_DROPIN % 'hearth-ollama', 'w').close()
        self.assertTrue(hh.svc('restart', 'ollama')['ok'])
        self.assertIn(['systemctl', 'restart', '--no-block', 'hearth-ollama.service'], self.rec.calls)

    def test_the_idle_gpu_limit_stays_in_the_cards_range_and_reset_only_after_hearth_set_it(self):
        hh.nvidia_power = lambda: {'min': 100.0, 'max': 250.0, 'default': 220.0, 'current': 220.0}
        self.rec.tools.add('nvidia-smi')
        self.assertEqual(hh.gpu_limit('reset'), {'ok': True, 'gpuLimit': 'untouched'})
        self.assertEqual(hh.gpu_limit('80')['gpuLimit'], 100, 'never below the card\'s own minimum')
        self.assertIn(['nvidia-smi', '-pl', '100'], self.rec.calls)
        self.assertEqual(hh.gpu_limit('reset')['gpuLimit'], 'default')
        self.assertIn(['nvidia-smi', '-pl', '220'], self.rec.calls)
        with self.assertRaises(hh.Fail):
            hh.gpu_limit('100; reboot')


class AiPayload(unittest.TestCase):
    def test_the_small_model_choice_follows_server_mode_until_chosen(self):
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d, True)
        saved = hs.SERVER_MODE
        self.addCleanup(setattr, hs, 'SERVER_MODE', saved)
        hs.SERVER_MODE = os.path.join(d, 'server-mode.json')
        self.assertIs(hs.ai_payload({'ai': {'on': True, 'small': None}})['small'], False)
        json.dump({'on': True, 'web': {'on': True}}, open(hs.SERVER_MODE, 'w'))
        self.assertIs(hs.ai_payload({'ai': {'on': True, 'small': None}})['small'], True)
        self.assertIs(hs.ai_payload({'ai': {'on': True, 'small': False}})['small'], False)
        self.assertTrue(hs.server_web_on())


class AdultModels(unittest.TestCase):
    """Uncensored (18+) models in Settings › Home server › AI chat: hidden until "Show 18+ models" is on, and "Download
    any model by name" only takes model names (never a URL, a path or an option)."""

    def setUp(self):
        names = ('load', 'services_status', 'http_json', 'seen_state', 'pull_file')
        saved = {k: getattr(hs, k) for k in names}
        self.addCleanup(lambda: [setattr(hs, k, v) for k, v in saved.items()])
        self.data = {'showAdult': False, 'ai': {}, 'showHidden': False, 'ccDays': 30}
        hs.load = lambda: dict(self.data)
        hs.services_status = lambda data=None: {'ai': {'active': True}, 'tsName': '', 'pictures': {}, 'voices': {}, 'playing': False}
        hs.http_json = lambda m, u, b=None, timeout=5: ({'models': [{'name': 'qwen3:30b-a3b'}, {'name': 'dolphin3:8b'}]} if u.endswith('/api/tags') else {'models': []})
        hs.seen_state = lambda data: {}
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d, True)
        hs.pull_file = lambda: os.path.join(d, 'pull.json')

    def test_names_are_checked(self):
        for ok in ('qwen3:8b', 'dolphin3', 'huihui_ai/qwen3-abliterated:30b-a3b', 'hf.co/TheDrummer/Rocinante-12B-v1.1-GGUF:Q4_K_M',
                   'hf.co/bartowski/Some.Model-GGUF', 'llama3.2:3b-instruct-q4_K_M'):
            self.assertTrue(hs.model_name_ok(ok), ok)
        for bad in ('', 'http://evil/x', '--insecure', '-rf', 'a b', 'qwen3:8b;rm -rf /', '../etc/passwd', 'a/b/c', 'hf.co/../x',
                    'Qwen3:8b', 'name:tag:more', 'x' * 300, '$(id)', 'hf.co/user', None):
            self.assertFalse(hs.model_name_ok(bad), bad)
        with self.assertRaises(hs.Fail):
            hs.ai_pull('http://example.com/model.gguf')

    def test_18_plus_models_only_with_the_switch_on(self):
        r = hs.ai_status()
        ids = [m['id'] for m in r['models']]
        self.assertIn('qwen3:30b-a3b', ids)
        self.assertFalse([i for i in ids if 'abliterated' in i or i.startswith('dolphin3') or 'Rocinante' in i], 'a family PC never lists them')
        self.assertFalse(r['showAdult'])
        self.assertIn('dolphin3:8b', r['others'], 'one you downloaded yourself still shows by name')
        self.data['showAdult'] = True
        r = hs.ai_status()
        adult = [m for m in r['models'] if m['group'] == 'adult']
        self.assertEqual([m['id'] for m in adult], ['huihui_ai/qwen3-abliterated:30b-a3b', 'huihui_ai/qwen3-abliterated:14b', 'dolphin3:8b',
                                                    'hf.co/TheDrummer/Rocinante-12B-v1.1-GGUF:Q4_K_M'])
        self.assertTrue(all(m['note'] and m['size'] for m in adult), 'each with a one-line description and its size')
        self.assertTrue(next(m for m in adult if m['id'] == 'dolphin3:8b')['have'])
        self.assertNotIn('dolphin3:8b', r['others'])
        for m in hs.MODELS + hs.ADULT_MODELS:
            self.assertTrue(hs.model_name_ok(m['id']), m['id'])

    def test_the_switch_is_off_by_default_and_display_only(self):
        self.assertIs(hs.DEFAULTS['showAdult'], False)
        self.assertIn('showAdult', hs.BOOL_KEYS)
        self.assertNotIn('showAdult', hs.APPLY_KEYS, 'showing them changes nothing on the system')
        self.assertEqual(hs.NESTED['aiSmallModel'][2], ('qwen3:14b', 'huihui_ai/qwen3-abliterated:14b'))


from test_hearth_home_server import Base as HsBase, USER as HS_USER  # noqa: E402


class WholeShares(HsBase):
    """Sharing a whole data drive, your whole Home (read only by default, one confirmation; read & write a second) or the
    system drive (read only only, two confirmations, its secrets vetoed). Hidden files stay vetoed on every share."""

    def setUp(self):
        super().setUp()
        self.mnt = os.path.join(self.d, 'mnt')
        self.drive = os.path.join(self.mnt, 'Drive2')
        os.makedirs(os.path.join(self.drive, 'Movies'))
        hh.drive_bases = lambda user: [self.mnt]
        self.saved_fns.setdefault('is_mount', hh.is_mount)
        hh.is_mount = lambda p: os.path.realpath(p) == os.path.realpath(self.drive)

    def whole(self, kind, path, **kw):
        s = {'id': 'abcdef12', 'name': {'drive': 'Drive2', 'home': 'Home', 'system': 'System drive'}[kind], 'path': path,
             'write': False, 'reach': 'home', 'recycle': True, 'whole': kind}
        s.update(kw)
        return s

    def test_a_data_drive_root_is_allowed(self):
        real, kind, why = hh.share_path_problem(self.drive, HS_USER, self.home, {'nvme0n1'})
        self.assertEqual((real, kind), (os.path.realpath(self.drive), 'drive'), why)
        good, _ = hh.check_shares([self.whole('drive', self.drive, write=True)], HS_USER, self.home, {'nvme0n1'})
        self.assertEqual(good[0]['whole'], 'drive')
        self.assertTrue(good[0]['write'], 'a data drive may be read & write')
        self.assertIsNone(hh.share_path_problem(self.mnt, HS_USER, self.home, {'nvme0n1'})[0], 'the folder drives sit in is not a drive')
        hh.mount_source = lambda p: '/dev/nvme0n1p3'
        self.assertNotEqual(hh.share_path_problem(self.drive, HS_USER, self.home, {'nvme0n1'})[1], 'drive', 'never a system disk')

    def test_home_is_read_only_by_default_needs_a_confirmation_and_hides_dot_files(self):
        with self.assertRaises(hh.Fail):
            hh.check_shares([self.whole('home', self.home)], HS_USER, self.home, set())
        good, _ = hh.check_shares([self.whole('home', self.home, confirm=['home'])], HS_USER, self.home, set())
        self.assertEqual(good[0]['whole'], 'home')
        self.assertFalse(good[0]['write'])
        text = hh.conf_text(good, HS_USER, False)
        self.assertIn('   veto files = /.*/', text, '.ssh, .config, .local and the keyrings never shared')
        self.assertIn('   hide dot files = yes', text)
        self.assertIn('   read only = yes', text)
        self.assertIn('   guest ok = no', text)
        with self.assertRaises(hh.Fail):
            hh.check_shares([self.whole('home', self.home, confirm=['home'], write=True)], HS_USER, self.home, set())
        good, _ = hh.check_shares([self.whole('home', self.home, confirm=['home', 'home-write'], write=True)], HS_USER, self.home, set())
        self.assertTrue(good[0]['write'], 'read & write after the second confirmation')
        self.assertEqual(good[0]['people'], {}, 'only you')

    def test_the_system_drive_read_only_with_its_secrets_vetoed(self):
        with self.assertRaises(hh.Fail):
            hh.check_shares([self.whole('system', '/', confirm=['system'])], HS_USER, self.home, set())
        good, _ = hh.check_shares([self.whole('system', '/', confirm=['system', 'system-sure'])], HS_USER, self.home, set())
        self.assertEqual((good[0]['path'], good[0]['whole'], good[0]['write'], good[0]['recycle']), ('/', 'system', False, False))
        text = hh.conf_text(good, HS_USER, True)
        sec = text.split('[System drive]')[1]
        self.assertIn('read only = yes', sec)
        veto = re.search(r'veto files = (.*)', sec).group(1)
        for name in ('/.*/', '/shadow/', '/gshadow/', '/ssh/', '/root/', '/proc/', '/sys/', '/dev/', '/run/', '/hearth/'):
            self.assertIn(name, veto)
        self.assertIn('hosts allow = 100.64.0.0/10', sec, 'the Tailscale-only switch still wins')
        self.assertNotIn('guest ok = yes', text)

    def test_the_system_drive_is_never_read_and_write(self):
        with self.assertRaises(hh.Fail) as e:
            hh.check_shares([self.whole('system', '/', confirm=['system', 'system-sure'], write=True)], HS_USER, self.home, set())
        self.assertIn('read only', str(e.exception))

    def test_the_old_rules_still_hold_for_everything_else(self):
        self.assertIsNone(hh.path_problem('/', HS_USER, self.home, set())[0], "people's folders, backups: still never /")
        self.assertIsNone(hh.path_problem(self.home, HS_USER, self.home, set())[0])
        for bad in ('/etc', '/usr', os.path.join(self.home, '.ssh'), self.home + '/a;b', self.home + '/$(x)', 'relative'):
            self.assertIsNone(hh.share_path_problem(bad, HS_USER, self.home, set())[0], bad)
        link = os.path.join(self.home, 'Videos', 'etc-link')
        os.symlink('/etc', link)
        self.assertIsNone(hh.share_path_problem(link, HS_USER, self.home, set())[0], 'a link is judged by where it points')


class WholeSharesSettings(unittest.TestCase):
    """The Mantel helper asks for the confirmations, starts every whole share read only on the home network and
    Tailscale, and never lets the system drive be read & write."""

    def setUp(self):
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d, True)
        self.home = os.path.join(d, 'home')
        os.makedirs(os.path.join(self.home, 'Games'))
        saved = {k: getattr(hs, k) for k in ('SETTINGS', 'me')}
        self.addCleanup(lambda: [setattr(hs, k, v) for k, v in saved.items()])
        hs.SETTINGS = os.path.join(d, 'home-server.json')
        hs.me = lambda: ('player', self.home)

    def test_confirmations_and_defaults(self):
        with self.assertRaises(hs.Fail):
            hs.share_add(self.home)
        s = hs.share_add(self.home, None, ['home'])
        self.assertEqual((s['whole'], s['write'], s['reach'], s['name']), ('home', False, 'home', 'Home'))
        with self.assertRaises(hs.Fail):
            hs.share_set(s['id'], 'write', 'on')
        self.assertTrue(hs.share_set(s['id'], 'write', 'on', 'home-write'))
        with self.assertRaises(hs.Fail):
            hs.share_add('/', None, ['system'])
        r = hs.share_add('/', None, ['system', 'system-sure'])
        self.assertEqual((r['whole'], r['write'], r['recycle']), ('system', False, False))
        with self.assertRaises(hs.Fail) as e:
            hs.share_set(r['id'], 'write', 'on', 'home-write')
        self.assertIn('read only', str(e.exception))


class Wiring(unittest.TestCase):
    def test_server_mode_is_off_and_game_mode_is_the_default(self):
        text = SESSION.read_text()
        self.assertIn('mode="${NIMBUS_START_MODE:-game}"', text)
        m = load_helper({'XDG_CONFIG_HOME': tempfile.gettempdir() + '/hearth-none-%d' % os.getpid()})
        self.assertIs(m.DEFAULTS['on'], False)
        self.assertIs(m.DEFAULTS['web']['on'], False)
        self.assertIs(m.DEFAULTS['schedule']['on'], False, 'no schedule by default')

    def test_the_web_unit_is_off_in_the_image(self):
        build = (ROOT / 'os-bases/peppermint/build.sh').read_text()
        self.assertIn('hearth-server-mode-web.service', build.split('systemctl --global disable', 1)[1].splitlines()[0])
        unit = (ROOT / 'os/system_files/usr/lib/systemd/user/hearth-server-mode-web.service').read_text()
        self.assertIn('hearth-server-mode serve', unit)

    def test_settings_pages_are_registered(self):
        pages = (SHELL / 'settings-pages.js').read_text()
        self.assertIn('id: "hs-server", file: "SetServerMode.qml"', pages)
        self.assertIn('id: "startup", file: "SetStartup.qml", cat: "system"', pages)
        for f in ('server-mode.qml', 'ServerDash.qml', 'ServerTile.qml', 'ServerDetail.qml', 'ServerBack.qml', 'ServerLine.qml',
                  'ServerButton.qml', 'SetServerMode.qml', 'SetStartup.qml', 'server-mode-web.html'):
            self.assertTrue((SHELL / f).exists(), f)
        self.assertIn('hearth-server-dash-up', (SHELL / 'ServerDash.qml').read_text(), 'the mark the session waits for')

    def test_the_controller_is_never_switched_off_on_the_dashboard(self):
        # {"cmd": "mouse", "on": false} pauses all of hearth-pad (user_off): in 1.6 only Guide worked on the dashboard
        qml = (SHELL / 'server-mode.qml').read_text()
        self.assertNotRegex(qml, r'cmd:\s*"mouse",\s*on:\s*false')
        self.assertIn('cmd: "mouse", on: true', qml)
        self.assertIn('padRestart', qml, 'hearth-pad starts again if it ends')
        pad = (SHELL / 'hearth-pad').read_text()
        self.assertIn("return self.fs_pause or self.user_off", pad, 'mouse off still means the whole controller is paused')

    def test_server_mode_page_can_be_found_without_search(self):
        # 1.6 shipped with the page only reachable from Settings' search: Home server lists its own cards
        self.assertIn('SetLinkCard { target: "hs-server"', (SHELL / 'SetHomeServer.qml').read_text())
        self.assertIn('SettingsNav.openPage("hs-server")', (SHELL / 'SetStartup.qml').read_text(),
                      'the greyed-out Server mode card in Startup opens the page that turns it on')


if __name__ == '__main__':
    unittest.main()
