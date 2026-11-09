"""Which Game Mode (docs/HEARTH-GAME-MODE.md, Hearth 1.10): Steam Game Mode (the default) or Hearth Game Mode.

  * the choice's helper (desktop/shell/hearth-game-mode): Steam Game Mode by default, only its own line of session.conf
    written (atomic, .bak, through hearth_settings_io), nothing written when it's already that, anything else refused,
    the old developer switch read but never written, the fallback note shown once per place;
  * the session's reading of it (nimbus-session game_mode_choice) gives the same answer as the helper for every file;
  * the whole session, run for real with stand-ins for gamescope, Steam, Hearth's app and labwc: the default is Steam Game
    Mode exactly as before, Hearth Game Mode when picked, and when it can't start or crashes the fallback order
    Hearth Game Mode -> Steam Game Mode -> Mantel for that start, with the reason in the note and the log, two failures
    pausing it until you ask for it, and the stored choice never changed by the session (byte for byte).

Nothing here starts gamescope, Steam, labwc or Hearth for real."""
import sys
sys.dont_write_bytecode = True
import hashlib
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts/linux/session/nimbus-session'
MODE = ROOT / 'hosts/linux/session/nimbus-mode'
WATCHDOG = ROOT / 'hosts/linux/session/nimbus-watchdog'
HELPER = ROOT / 'hosts/linux/desktop/shell/hearth-game-mode'


def block(name):
    m = re.search(r'# >>> %s\n(.*?)# <<< %s' % (name, name), SESSION.read_text(), re.S)
    assert m, name + ' block not found'
    return m.group(1)


def load_helper():
    loader = importlib.machinery.SourceFileLoader('hearth_game_mode', str(HELPER))
    spec = importlib.util.spec_from_loader('hearth_game_mode', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def sha(p):
    try:
        with open(p, 'rb') as f:
            return hashlib.sha256(f.read()).hexdigest()
    except OSError:
        return None


class Helper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-gm-helper-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.env = dict(os.environ, XDG_CONFIG_HOME=os.path.join(self.tmp, 'cfg'), XDG_RUNTIME_DIR=os.path.join(self.tmp, 'run'),
                        XDG_STATE_HOME=os.path.join(self.tmp, 'state'), HOME=self.tmp)
        self.env.pop('HEARTH_LEGACY_GAME_MODE', None)
        self.env.pop('HEARTH_SESSION_CONF', None)
        os.makedirs(self.env['XDG_RUNTIME_DIR'])
        self.conf = os.path.join(self.tmp, 'cfg', 'nimbus', 'session.conf')

    def run_helper(self, *args):
        r = subprocess.run([sys.executable, str(HELPER)] + list(args), env=self.env, capture_output=True, text=True, timeout=30)
        return r

    def get(self):
        r = self.run_helper('get')
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        return json.loads(r.stdout)

    def write_conf(self, text):
        os.makedirs(os.path.dirname(self.conf), exist_ok=True)
        with open(self.conf, 'w') as f:
            f.write(text)

    def test_default_is_steam_game_mode_and_reading_writes_nothing(self):
        g = self.get()
        self.assertEqual(g['choice'], 'steam')
        self.assertFalse(g['saved'])
        self.assertEqual(g['default'], 'steam')
        self.assertFalse(os.path.exists(self.conf), 'reading never makes the file')
        self.assertEqual(self.run_helper('choice').stdout.strip(), 'steam')

    def test_set_changes_only_its_own_line_with_a_backup(self):
        self.write_conf('NIMBUS_START_MODE=desktop\nNIMBUS_GAMESCOPE_ARGS="-W 1920 -H 1080"\n# mine\n')
        r = self.run_helper('set', 'hearth')
        self.assertEqual(r.returncode, 0, r.stdout)
        self.assertEqual(json.loads(r.stdout)['choice'], 'hearth')
        self.assertEqual(open(self.conf).read(), 'NIMBUS_START_MODE=desktop\nNIMBUS_GAMESCOPE_ARGS="-W 1920 -H 1080"\n# mine\nHEARTH_GAME_MODE_CHOICE=hearth\n')
        self.assertEqual(open(self.conf + '.bak').read(), 'NIMBUS_START_MODE=desktop\nNIMBUS_GAMESCOPE_ARGS="-W 1920 -H 1080"\n# mine\n')
        self.run_helper('set', 'steam')
        self.assertEqual(open(self.conf).read().count('HEARTH_GAME_MODE_CHOICE='), 1, 'the line is replaced, not added again')
        self.assertIn('HEARTH_GAME_MODE_CHOICE=steam\n', open(self.conf).read())
        self.assertIn('NIMBUS_START_MODE=desktop', open(self.conf).read())
        # No temporary files left beside it.
        self.assertEqual([n for n in os.listdir(os.path.dirname(self.conf)) if n.startswith('.session.conf.') and not n.endswith('.lock')], [])

    def test_the_same_choice_again_writes_nothing(self):
        self.write_conf('HEARTH_GAME_MODE_CHOICE=hearth\n')
        before = os.stat(self.conf)
        time.sleep(0.05)
        self.run_helper('set', 'hearth')
        after = os.stat(self.conf)
        self.assertEqual((before.st_ino, before.st_mtime_ns), (after.st_ino, after.st_mtime_ns))
        self.assertFalse(os.path.exists(self.conf + '.bak'))

    def test_anything_else_is_refused_and_nothing_changes(self):
        self.write_conf('HEARTH_GAME_MODE_CHOICE=hearth\n')
        h = sha(self.conf)
        for bad in ('desktop', 'Hearth', '', 'hearth; rm -rf ~', 'steam\nX=1'):
            r = self.run_helper('set', bad)
            self.assertNotEqual(r.returncode, 0, bad)
            self.assertTrue(r.stdout.startswith('error: '), r.stdout)
        self.assertEqual(sha(self.conf), h)

    def test_a_typo_in_the_file_is_steam_game_mode_and_is_kept(self):
        self.write_conf('HEARTH_GAME_MODE_CHOICE=Hearth\n')
        self.assertEqual(self.get()['choice'], 'steam')
        self.assertEqual(open(self.conf).read(), 'HEARTH_GAME_MODE_CHOICE=Hearth\n', 'never "repaired" by reading')

    def test_the_old_developer_switch_is_read_never_written(self):
        self.write_conf('HEARTH_LEGACY_GAME_MODE=1\n')
        g = self.get()
        self.assertEqual((g['choice'], g['saved']), ('hearth', False))
        self.assertEqual(open(self.conf).read(), 'HEARTH_LEGACY_GAME_MODE=1\n')
        # Your own pick wins over it.
        self.run_helper('set', 'steam')
        self.assertEqual(self.get()['choice'], 'steam')
        self.assertIn('HEARTH_LEGACY_GAME_MODE=1\n', open(self.conf).read(), 'the old line is left alone')

    def test_the_fallback_note_is_shown_once_per_place(self):
        run = self.env['XDG_RUNTIME_DIR']
        self.assertEqual(json.loads(self.run_helper('notice', 'steam').stdout), {})
        with open(os.path.join(run, 'hearth-game-mode-failed'), 'w') as f:
            f.write("Hearth's app isn't installed\n")
        self.assertEqual(json.loads(self.run_helper('notice', 'steam').stdout), {'text': "Hearth's app isn't installed"})
        self.assertEqual(json.loads(self.run_helper('notice', 'steam').stdout), {})
        self.assertEqual(json.loads(self.run_helper('notice', 'desktop').stdout)['text'], "Hearth's app isn't installed")
        self.assertEqual(self.get()['fallback'], "Hearth's app isn't installed")
        # A new failure later is shown again.
        time.sleep(0.05)
        p = os.path.join(run, 'hearth-game-mode-failed')
        with open(p, 'w') as f:
            f.write('Hearth Game Mode closed right away (code 1)\n')
        os.utime(p, (time.time() + 5, time.time() + 5))
        self.assertIn('closed right away', json.loads(self.run_helper('notice', 'steam').stdout)['text'])
        self.assertNotEqual(self.run_helper('notice', 'nowhere').returncode, 0)
        self.assertFalse(os.path.exists(self.conf), 'a notice never writes a setting')


class ShellAgreesWithHelper(unittest.TestCase):
    """nimbus-session's game_mode_choice and the helper's choice_from: one set of rules."""

    CASES = ['', 'HEARTH_GAME_MODE_CHOICE=hearth\n', 'HEARTH_GAME_MODE_CHOICE=steam\n', 'export HEARTH_GAME_MODE_CHOICE=hearth\n',
             'HEARTH_GAME_MODE_CHOICE="hearth"\n', "HEARTH_GAME_MODE_CHOICE='hearth'\n", '  HEARTH_GAME_MODE_CHOICE=hearth # mine\n',
             'HEARTH_GAME_MODE_CHOICE=hearth\nHEARTH_GAME_MODE_CHOICE=steam\n', 'HEARTH_GAME_MODE_CHOICE=steam\nHEARTH_GAME_MODE_CHOICE=hearth\n',
             'HEARTH_GAME_MODE_CHOICE=\n', 'HEARTH_GAME_MODE_CHOICE=Hearth\n', 'HEARTH_GAME_MODE_CHOICE=hearthx\n', '#HEARTH_GAME_MODE_CHOICE=hearth\n',
             'HEARTH_LEGACY_GAME_MODE=1\n', 'HEARTH_LEGACY_GAME_MODE=0\n', 'HEARTH_LEGACY_GAME_MODE=1\nHEARTH_GAME_MODE_CHOICE=steam\n',
             'XHEARTH_GAME_MODE_CHOICE=hearth\n', 'NIMBUS_START_MODE=desktop\n']

    def test_same_answer_for_every_file(self):
        h = load_helper()
        tmp = tempfile.mkdtemp(prefix='hearth-gm-parity-')
        self.addCleanup(shutil.rmtree, tmp, True)
        conf = os.path.join(tmp, 'session.conf')
        script = os.path.join(tmp, 's.sh')
        with open(script, 'w') as f:
            f.write('RUN=%s\nSESSION_CONF=%s\nlog() { :; }\nhave() { command -v "$1" >/dev/null 2>&1; }\n%s\ngame_mode_choice\n' % (tmp, conf, block('game mode choice')))
        for text in self.CASES:
            with open(conf, 'w') as f:
                f.write(text)
            out = subprocess.run(['sh', script], env={'PATH': '/usr/bin:/bin', 'HOME': tmp}, capture_output=True, text=True, timeout=20).stdout.strip()
            self.assertEqual(out, h.choice_from(text)[0], repr(text))
        # No file at all: Steam Game Mode.
        os.remove(conf)
        out = subprocess.run(['sh', script], env={'PATH': '/usr/bin:/bin', 'HOME': tmp}, capture_output=True, text=True, timeout=20).stdout.strip()
        self.assertEqual(out, 'steam')

    def test_the_session_never_writes_the_choice(self):
        text = SESSION.read_text()
        for line in text.splitlines():
            if 'HEARTH_GAME_MODE_CHOICE' in line and not line.lstrip().startswith('#'):
                self.assertNotRegex(line, r'>\s*"?\$?\(?gm_conf|conf-set|sed -i|HEARTH_GAME_MODE_CHOICE=(hearth|steam)\b', line)
        self.assertNotIn('conf-set', block('game mode choice'))


FAKE_GAMESCOPE = r'''#!/bin/sh
# Stand-in for gamescope: which Game Mode it was started for (Steam's has -steampal), then what the test asked for that
# run (GS_STEAM / GS_HEARTH: a comma list, one item per run, the last one repeating).
role=hearth
for a in "$@"; do [ "$a" = -steampal ] && role=steam; done
echo "$role" >>"$FAKE_CALLS"
n=$(grep -c "^$role\$" "$FAKE_CALLS")
if [ "$role" = steam ]; then list="$GS_STEAM"; else list="$GS_HEARTH"; fi
item=$(echo "$list" | tr ',' '\n' | sed -n "${n}p")
[ -n "$item" ] || item=$(echo "$list" | tr ',' '\n' | tail -n 1)
R="$XDG_RUNTIME_DIR"
echo "$role env HEARTH_STEAM_ONLY=${HEARTH_STEAM_ONLY:-}" >>"$FAKE_CALLS.env"
case "$item" in
  crash) echo "Hearth's app closed with an error 3 times within 2 minutes (code 1)" >"$R/hearth-game-mode-crashed"; exit 1 ;;
  quick) exit 1 ;;
  quit0) exit 0 ;;
  *) echo "$item" >"$R/nimbus-next"; exit 0 ;;
esac
'''

FAKE_LABWC = r'''#!/bin/sh
echo labwc >>"$FAKE_CALLS"
exit 0
'''


class WholeSession(unittest.TestCase):
    """nimbus-session from start to the end of the session, with stand-ins."""

    def setUp(self):
        for tool in ('steam', 'gamescope', 'labwc'):
            if shutil.which(tool):
                self.skipTest(tool + ' is installed here')
        self.tmp = tempfile.mkdtemp(prefix='hearth-gm-session-')
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.addCleanup(lambda: subprocess.run(['pkill', '-KILL', '-f', self.tmp], capture_output=True))
        self.home = os.path.join(self.tmp, 'home')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        for d in (self.home, self.run_dir, self.bin, os.path.join(self.tmp, 'share')):
            os.makedirs(d)
        os.chmod(self.run_dir, 0o700)
        self.calls = os.path.join(self.tmp, 'calls')
        self.app = os.path.join(self.tmp, 'nimbus-app')
        self.write(self.app, '#!/bin/sh\nexit 0\n')
        self.write(os.path.join(self.bin, 'gamescope'), FAKE_GAMESCOPE)
        self.write(os.path.join(self.bin, 'labwc'), FAKE_LABWC)
        self.write(os.path.join(self.bin, 'steam'), '#!/bin/sh\nexit 0\n')
        self.conf = os.path.join(self.home, '.config/nimbus/session.conf')
        self.state = os.path.join(self.home, '.local/state/nimbus')

    def write(self, p, text):
        with open(p, 'w') as f:
            f.write(text)
        os.chmod(p, 0o755)

    def session(self, conf=None, **env):
        os.makedirs(os.path.dirname(self.conf), exist_ok=True)
        if conf is not None:
            with open(self.conf, 'w') as f:
                f.write(conf)
        before = sha(self.conf)
        if os.path.exists(self.calls):
            os.remove(self.calls)
        e = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_SHARE': os.path.join(self.tmp, 'share'),
             'NIMBUS_APP': self.app, 'HEARTH_CMDLINE': os.devnull, 'HEARTH_PAD_HELD': os.devnull, 'FAKE_CALLS': self.calls,
             'HEARTH_SERVER_HELPER': os.path.join(self.tmp, 'no-server-helper'), 'HEARTH_STEAM_FIRSTRUN_LAUNCHERS': '/nonexistent',
             'HEARTH_DECKY_UNIT': '/nonexistent', 'HEARTH_DECKY_BUILTIN': '/nonexistent', 'HEARTH_DECKY_UPSTREAM': '/nonexistent',
             'HEARTH_STEAM_NET_WAIT': '0', 'HEARTH_LIVE_FLAG': '/nonexistent', 'USER': 'player', 'HEARTH_STEAM_CORE': '0',
             'GS_STEAM': 'desktop', 'GS_HEARTH': 'desktop'}
        e.update(env)
        r = subprocess.run(['sh', str(SESSION)], env=e, capture_output=True, text=True, timeout=120)
        log = pathlib.Path(self.home, '.local/share/nimbus/session.log')
        log = log.read_text() if log.exists() else ''
        self.assertEqual(sha(self.conf), before, 'the session never changes session.conf (your choice):\n' + log)
        return r, log

    def calls_list(self):
        if not os.path.exists(self.calls):
            return []
        with open(self.calls) as f:
            return f.read().split()

    def note(self):
        try:
            with open(os.path.join(self.run_dir, 'hearth-game-mode-failed')) as f:
                return f.read().strip()
        except OSError:
            return None

    # ---- the default stays exactly as before

    def test_default_is_steam_game_mode_and_hearth_never_starts(self):
        r, log = self.session('')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertEqual(self.calls_list(), ['steam', 'labwc'], log)
        self.assertNotIn('hearth game mode', log)
        self.assertIsNone(self.note())
        self.assertIn('steam env HEARTH_STEAM_ONLY=1', open(self.calls + '.env').read(), 'first-time setup still hands over to Steam Game Mode')

    def test_steam_picked_and_failing_goes_to_desktop_as_before(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=steam\n', GS_STEAM='quick')
        self.assertEqual(self.calls_list(), ['steam', 'labwc'], log)
        self.assertIn('Steam ended right away', log)
        self.assertNotIn('hearth game mode', log)

    # ---- Hearth Game Mode picked

    def test_hearth_picked_starts_hearth_game_mode(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertEqual(self.calls_list(), ['hearth', 'labwc'], log)
        self.assertIn('hearth game mode ended (code 0); desktop asked for', log)
        self.assertIsNone(self.note())
        self.assertIn('hearth env HEARTH_STEAM_ONLY=\n', open(self.calls + '.env').read())

    def test_the_old_developer_switch_still_means_hearth_game_mode(self):
        r, log = self.session('HEARTH_LEGACY_GAME_MODE=1\n')
        self.assertEqual(self.calls_list(), ['hearth', 'labwc'], log)

    def test_a_crashing_hearth_falls_back_to_steam_then_desktop_keeping_the_choice(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='crash', GS_STEAM='desktop')
        self.assertEqual(r.returncode, 0, log + r.stderr)
        self.assertEqual(self.calls_list(), ['hearth', 'steam', 'labwc'], log)
        self.assertIn('3 times within 2 minutes', self.note())
        self.assertIn('Steam Game Mode this time (your choice stays Hearth Game Mode)', log)
        self.assertEqual(open(self.conf).read(), 'HEARTH_GAME_MODE_CHOICE=hearth\n')

    def test_hearth_then_steam_both_failing_ends_in_mantel(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='crash', GS_STEAM='quick')
        self.assertEqual(self.calls_list(), ['hearth', 'steam', 'labwc'], log)
        self.assertIn('Steam ended right away', log)
        self.assertNotIn('last resort', log)

    def test_hearth_closing_right_away_falls_back(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='quick')
        self.assertEqual(self.calls_list(), ['hearth', 'steam', 'labwc'], log)
        self.assertIn('closed right away (code 1)', self.note())

    def test_hearth_quitting_on_purpose_right_away_still_never_leaves_no_screen(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='quit0')
        self.assertEqual(self.calls_list(), ['hearth', 'steam', 'labwc'], log)

    def test_hearth_signing_out_after_a_while_ends_the_session(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='quit0', HGM_MIN_SECS='0')
        self.assertEqual(self.calls_list(), ['hearth'], log)
        self.assertIn('leaving the session', log)

    def test_no_steam_means_mantel_straight_after_hearth(self):
        os.remove(os.path.join(self.bin, 'steam'))
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', GS_HEARTH='crash')
        self.assertEqual(self.calls_list(), ['hearth', 'labwc'], log)
        self.assertIn('Desktop Mode this time', log)

    def test_missing_hearth_app_goes_to_steam_with_the_reason(self):
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', NIMBUS_APP=os.path.join(self.tmp, 'nope'))
        self.assertEqual(self.calls_list(), ['steam', 'labwc'], log)
        self.assertEqual(self.note(), "Hearth's app isn't installed")

    def test_two_failures_pause_it_and_asking_for_it_brings_it_back(self):
        c = 'HEARTH_GAME_MODE_CHOICE=hearth\n'
        self.session(c, GS_HEARTH='crash')
        self.session(GS_HEARTH='crash')
        r, log = self.session(GS_HEARTH='crash')
        self.assertEqual(self.calls_list(), ['steam', 'labwc'], 'paused after two failures: straight to Steam Game Mode\n' + log)
        self.assertTrue(os.path.exists(os.path.join(self.state, 'hearth-gm-off')))
        self.assertIn('paused', self.note())
        # Steam Game Mode's Hearth Sync (or Settings) asks for Hearth Game Mode: the pause ends and it runs.
        r, log = self.session(GS_HEARTH='desktop', GS_STEAM='hearth')
        self.assertEqual(self.calls_list(), ['steam', 'hearth', 'labwc'], log)
        self.assertFalse(os.path.exists(os.path.join(self.state, 'hearth-gm-off')))
        self.assertEqual(open(self.conf).read(), c)

    def test_a_good_run_clears_the_count(self):
        c = 'HEARTH_GAME_MODE_CHOICE=hearth\n'
        self.session(c, GS_HEARTH='crash')
        self.session(GS_HEARTH='desktop', HGM_GOOD_SECS='0')
        self.assertFalse(os.path.exists(os.path.join(self.state, 'hearth-gm-fails')))
        r, log = self.session(GS_HEARTH='crash')
        self.assertEqual(self.calls_list()[0], 'hearth', 'one failure since the good run: tried again')

    def test_a_new_choice_applies_the_next_time_game_mode_starts(self):
        # In Steam Game Mode you pick Hearth Game Mode and Switch now: the next "game" is Hearth Game Mode.
        os.makedirs(os.path.dirname(self.conf), exist_ok=True)
        open(self.conf, 'w').close()
        flip = os.path.join(self.bin, 'gamescope')
        body = FAKE_GAMESCOPE.replace('echo "$role" >>"$FAKE_CALLS"\n',
                                      'echo "$role" >>"$FAKE_CALLS"\n[ "$role" = steam ] && echo HEARTH_GAME_MODE_CHOICE=hearth >"$HOME/.config/nimbus/session.conf"\n')
        self.write(flip, body)
        e = {'HOME': self.home, 'XDG_RUNTIME_DIR': self.run_dir, 'PATH': self.bin + ':/usr/bin:/bin', 'NIMBUS_SHARE': os.path.join(self.tmp, 'share'),
             'NIMBUS_APP': self.app, 'HEARTH_CMDLINE': os.devnull, 'HEARTH_PAD_HELD': os.devnull, 'FAKE_CALLS': self.calls,
             'HEARTH_SERVER_HELPER': '/nonexistent', 'HEARTH_STEAM_FIRSTRUN_LAUNCHERS': '/nonexistent', 'HEARTH_DECKY_UNIT': '/nonexistent',
             'HEARTH_DECKY_BUILTIN': '/nonexistent', 'HEARTH_DECKY_UPSTREAM': '/nonexistent', 'HEARTH_STEAM_NET_WAIT': '0',
             'HEARTH_LIVE_FLAG': '/nonexistent', 'USER': 'player', 'HEARTH_STEAM_CORE': '0', 'GS_STEAM': 'game', 'GS_HEARTH': 'desktop'}
        subprocess.run(['sh', str(SESSION)], env=e, capture_output=True, text=True, timeout=120)
        self.assertEqual(self.calls_list(), ['steam', 'hearth', 'labwc'])

    def test_b_or_guide_held_opens_desktop_mode_this_time(self):
        held = os.path.join(self.tmp, 'held.py')
        self.write(held, 'import sys\nsys.exit(0)\n')
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\n', HEARTH_PAD_HELD=held)
        self.assertEqual(self.calls_list(), ['labwc'], log)
        self.assertIn('B or Guide held', log)

    def test_server_modes_way_back_and_guide_go_to_your_game_mode(self):
        # Server mode can't start here (no helper): its "Game Mode instead" is the Game Mode you picked.
        r, log = self.session('HEARTH_GAME_MODE_CHOICE=hearth\nNIMBUS_START_MODE=server\n')
        self.assertEqual(self.calls_list(), ['hearth', 'labwc'], log)


class Wiring(unittest.TestCase):
    def test_nimbus_mode_knows_hearth(self):
        tmp = tempfile.mkdtemp(prefix='hearth-gm-mode-')
        self.addCleanup(shutil.rmtree, tmp, True)
        r = subprocess.run(['sh', str(MODE), 'hearth'], env={'PATH': '/usr/bin:/bin', 'XDG_RUNTIME_DIR': tmp, 'HOME': tmp}, capture_output=True, text=True, timeout=20)
        self.assertEqual(open(os.path.join(tmp, 'nimbus-next')).read().strip(), 'hearth')
        self.assertIn('hearth', subprocess.run(['sh', str(MODE), 'nope'], capture_output=True, text=True).stderr)

    def test_the_watchdog_leaves_the_reason_when_it_gives_up(self):
        text = WATCHDOG.read_text()
        i = text.index('too many crashes, ending the session')
        self.assertIn('hearth-game-mode-crashed', text[i:i + 600])
        r = subprocess.run(['bash', '-n', str(WATCHDOG)], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_the_session_routes_every_game_request_through_the_choice(self):
        text = SESSION.read_text()
        self.assertIn('if [ "$mode" = game ] && [ -z "$HEARTH_SEAT" ] && ! live_session; then run_hearth_mode; code=$?', text)
        self.assertIn('game|desktop|steam|server|hearth)\n      to="$next"; [ -z "$HEARTH_SEAT" ] && to="$(resolve_mode "$next")"', text)
        self.assertIn('hearth) if live_session; then echo game; else hgm_unpause; hgm_route; fi ;;', text)
        self.assertNotIn('legacy_game', text, 'the old switch is now only a way to read the choice')
        for name in ('bash', 'sh'):
            r = subprocess.run([name, '-n', str(SESSION)], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stderr)


if __name__ == '__main__':
    unittest.main()
