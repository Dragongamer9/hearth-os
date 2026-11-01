"""Steam Game Mode (Experimental, docs/STEAM-GAME-MODE.md): nimbus-session's run_steam_mode (run for real between its
>>> and <<< marks, with stand-ins for gamescope and Steam), nimbus-mode steam, and the Decky Loader helper
(hearth-decky through hearth-admin). Nothing here starts Steam, signs in or touches the real system."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import pwd
import re
import shutil
import subprocess
import tempfile
import time
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[3]
SESSION = ROOT / 'hosts/linux/session/nimbus-session'
MODE = ROOT / 'hosts/linux/session/nimbus-mode'
DECKY = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-decky'
ADMIN = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-admin'
RULES = ROOT / 'os/system_files/usr/share/polkit-1/rules.d/49-hearth-admin.rules'
STEAMOS_UPDATE = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-steamos-update'


def block():
    text = SESSION.read_text()
    m = re.search(r'# >>> steam game mode\n(.*?)# <<< steam game mode', text, re.S)
    assert m, 'steam game mode block not found'
    return m.group(1)


def load_decky():
    loader = importlib.machinery.SourceFileLoader('hearth_decky', str(DECKY))
    spec = importlib.util.spec_from_loader('hearth_decky', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class SteamModeSession(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-steam-mode-')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        os.makedirs(self.run_dir)
        os.makedirs(self.bin)
        # Only these programs and the basic tools: no real gamescope, Steam, pkexec or flatpak.
        for tool in ('sh', 'cat', 'date', 'mkdir', 'chmod', 'rm', 'printf', 'env', 'echo', 'sleep', 'timeout', 'true', 'grep', 'kill'):
            p = shutil.which(tool)
            if p:
                os.symlink(p, os.path.join(self.bin, tool))

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake(self, name, body):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(p, 0o755)

    def run_mode(self, extra_env=None, before=''):
        script = os.path.join(self.tmp, 'run.sh')
        with open(script, 'w') as f:
            f.write('RUN=%s\nNEXT="$RUN/nimbus-next"\nLOG=%s\nSESSION_CONF=%s\nENV_GAMESCOPE_ARGS=""\n'
                    % (self.run_dir, os.path.join(self.tmp, 'session.log'), os.path.join(self.tmp, 'session.conf')))
            f.write('log() { echo "$*" >>"$LOG"; }\nhave() { command -v "$1" >/dev/null 2>&1; }\n')
            f.write(before + '\n' + block() + '\nrun_steam_mode\necho "rc=$?"\n')
        env = {'PATH': self.bin, 'HOME': os.path.join(self.tmp, 'home'), 'STEAM_MODE_MIN_SECS': '20',
               'HEARTH_DECKY_UNIT': os.path.join(self.tmp, 'no-decky.service')}
        env.update(extra_env or {})
        r = subprocess.run(['/bin/sh', script], env=env, capture_output=True, text=True, timeout=60)
        self.assertIn('rc=0', r.stdout, r.stderr)
        return r

    def read(self, name):
        try:
            return open(os.path.join(self.run_dir, name)).read().strip()
        except OSError:
            return None

    def test_no_steam_falls_back_to_hearths_game_mode(self):
        self.fake('gamescope', 'exit 0')
        self.run_mode()
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn("isn't installed", self.read('hearth-steam-mode-failed'))

    def test_no_gamescope_falls_back(self):
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertTrue(self.read('hearth-steam-mode-failed'))

    def test_extra_multiseat_player_never_gets_it(self):
        self.fake('gamescope', 'echo ran >"$HOME.ran"')
        self.fake('steam', 'exit 0')
        self.run_mode({'HEARTH_SEAT': '2'})
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertFalse(os.path.exists(os.path.join(self.tmp, 'home.ran')))

    def test_flathub_steam_in_use_is_the_one_started(self):
        # Both Steams installed, but the games and sign-in are in Flathub's Steam's folder: that one starts
        # (the system Steam would show no installed games).
        home = os.path.join(self.tmp, 'home')
        os.makedirs(os.path.join(home, '.local/share/flatpak/app/com.valvesoftware.Steam'))
        os.makedirs(os.path.join(home, '.var/app/com.valvesoftware.Steam/.local/share/Steam/config'))
        with open(os.path.join(home, '.var/app/com.valvesoftware.Steam/.local/share/Steam/config/loginusers.vdf'), 'w') as f:
            f.write('"users" {}\n')
        argv = os.path.join(self.tmp, 'argv')
        self.fake('gamescope', 'for a in "$@"; do echo "$a"; done >%s\nexit 1' % argv)
        self.fake('steam', 'exit 0')
        self.fake('flatpak', 'exit 0')
        self.run_mode()
        tail = open(argv).read().split('\n')
        tail = tail[tail.index('--') + 1:]
        self.assertEqual(tail[:2], ['flatpak', 'run'])
        self.assertIn('com.valvesoftware.Steam', tail)
        # No sign-in in Flathub's folder: the system Steam, as before.
        os.remove(os.path.join(home, '.var/app/com.valvesoftware.Steam/.local/share/Steam/config/loginusers.vdf'))
        self.run_mode()
        tail = open(argv).read().split('\n')
        self.assertEqual(tail[tail.index('--') + 1], 'env')

    def test_steam_that_fails_right_away_comes_back_with_a_note(self):
        argv = os.path.join(self.tmp, 'argv')
        self.fake('gamescope', 'for a in "$@"; do echo "$a"; done >%s\nexit 1' % argv)
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn('ended right away (code 1)', self.read('hearth-steam-mode-failed'))
        args = open(argv).read().split('\n')
        self.assertEqual(args[:2], ['-e', '-f'])
        tail = args[args.index('--') + 1:]
        self.assertEqual(tail[0], 'env')
        self.assertTrue(tail[1].startswith('PATH=%s/hearth-steam-bin:' % self.run_dir))
        self.assertEqual(tail[2:6], ['steam', '-gamepadui', '-steamos3', '-steampal'])
        self.assertIsNone(self.read('hearth-steam-mode.pid'), 'the running mark is gone afterwards')

    def test_steam_ending_normally_goes_back_to_hearth(self):
        seen = os.path.join(self.tmp, 'seen')
        self.fake('gamescope', 'cat "$RUN_DIR/hearth-steam-mode.pid" >%s; echo "$HEARTH_STEAM_MODE $STEAM_MULTIPLE_XWAYLANDS" >>%s; exit 0' % (seen, seen))
        self.fake('steam', 'exit 0')
        self.run_mode({'RUN_DIR': self.run_dir})
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIsNone(self.read('hearth-steam-mode-failed'))
        lines = open(seen).read().split('\n')
        self.assertTrue(lines[0].isdigit(), 'nimbus-mode can see Steam Game Mode is running')
        self.assertEqual(lines[1], '1 1')

    def test_switch_to_desktop_from_steam_is_kept(self):
        self.fake('gamescope', 'echo desktop >"$RUN_DIR/nimbus-next"; exit 0')
        self.fake('steam', 'exit 0')
        self.run_mode({'RUN_DIR': self.run_dir})
        self.assertEqual(self.read('nimbus-next'), 'desktop')

    def test_steams_switch_to_desktop_asks_hearths_session(self):
        self.fake('gamescope', 'exit 0')
        self.fake('steam', 'exit 0')
        self.run_mode()
        shim = open(os.path.join(self.run_dir, 'hearth-steam-bin', 'steamos-session-select')).read()
        self.assertIn('exec nimbus-mode desktop', shim)
        self.assertIn('gamescope*|game*) exec nimbus-mode game', shim)

    def test_steams_os_update_runs_hearths_updater(self):
        self.fake('gamescope', 'exit 0')
        self.fake('steam', 'exit 0')
        helper = os.path.join(self.tmp, 'hearth-steamos-update')
        with open(helper, 'w') as f:
            f.write('#!/bin/sh\nexit 7\n')
        os.chmod(helper, 0o755)
        self.run_mode({'HEARTH_STEAMOS_UPDATE': helper})
        shim = os.path.join(self.run_dir, 'hearth-steam-bin', 'steamos-update')
        self.assertIn('exec %s "$@"' % helper, open(shim).read())
        r = subprocess.run([shim, 'check'], capture_output=True)
        self.assertEqual(r.returncode, 7, 'the exit code reaches Steam')
        # Without Hearth's updater installed: no steamos-update at all (Steam then has none, as before).
        os.remove(shim)
        self.run_mode({'HEARTH_STEAMOS_UPDATE': os.path.join(self.tmp, 'missing')})
        self.assertFalse(os.path.exists(shim))
        self.assertNotIn('/usr/bin/steamos-update', SESSION.read_text(), 'nothing system-wide')

    def test_decky_started_and_stopped_only_when_hearth_installed_it(self):
        calls = os.path.join(self.tmp, 'pkexec-calls')
        self.fake('pkexec', 'echo "$@" >>%s' % calls)
        self.fake('gamescope', 'exit 0')
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertFalse(os.path.exists(calls), 'no Decky unit: nothing asked')
        # (/usr/libexec/nimbus/hearth-admin isn't on a CI runner, so decky_ctl stays quiet there too.)

    def fake_core(self, body):
        """A stand-in for Hearth's app that knows --core-service (the session checks for the word first)."""
        self.fake('nimbus', '# --core-service\n' + body)

    def test_core_service_runs_with_steam_and_stops_after(self):
        args = os.path.join(self.tmp, 'core-args')
        pidf = os.path.join(self.tmp, 'core-pid')
        seen = os.path.join(self.tmp, 'seen')
        self.fake_core('echo "$@" >%s; echo $$ >%s; exec sleep 30' % (args, pidf))
        # Steam (gamescope) runs a moment: the core is already running by then.
        self.fake('gamescope', 'sleep 1; [ -s %s ] && echo core-up >%s; exit 0' % (pidf, seen))
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertEqual(open(seen).read().strip(), 'core-up', 'started before Steam')
        self.assertEqual(open(args).read().split(), ['--core-service', '--while', os.path.join(self.run_dir, 'hearth-steam-mode.pid')])
        pid = int(open(pidf).read())
        for _ in range(50):
            try:
                os.kill(pid, 0)
            except OSError:
                break
            time.sleep(0.1)
        else:
            self.fail('the core service is still running after Steam Game Mode ended')
        self.assertEqual(self.read('nimbus-next'), 'game')
        self.assertIn("Hearth's core started", open(os.path.join(self.tmp, 'session.log')).read())

    def test_core_service_that_fails_never_stops_steam_game_mode(self):
        ran = os.path.join(self.tmp, 'steam-ran')
        self.fake_core('exit 1')
        self.fake('gamescope', 'echo yes >%s; exit 0' % ran)
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertTrue(os.path.exists(ran), 'Steam Game Mode ran')
        self.assertIsNone(self.read('hearth-steam-mode-failed'))
        self.assertEqual(self.read('nimbus-next'), 'game')

    def test_no_core_service_when_turned_off_or_hearth_too_old(self):
        args = os.path.join(self.tmp, 'core-args')
        self.fake_core('echo "$@" >%s' % args)
        self.fake('gamescope', 'exit 0')
        self.fake('steam', 'exit 0')
        self.run_mode({'HEARTH_STEAM_CORE': '0'})
        self.assertFalse(os.path.exists(args), 'HEARTH_STEAM_CORE=0 leaves it out')
        # A Hearth without --core-service would open its whole window: never started.
        self.fake('nimbus', 'echo "$@" >%s' % args)
        self.run_mode()
        self.assertFalse(os.path.exists(args))
        self.assertIn('no core service', open(os.path.join(self.tmp, 'session.log')).read())

    def test_hearth_input_runs_with_steam_game_mode(self):
        calls = os.path.join(self.tmp, 'input-calls')
        self.fake('gamescope', 'echo steam >>%s; exit 0' % calls)
        self.fake('steam', 'exit 0')
        self.run_mode(before='input_start() { echo start >>%s; }\ninput_stop() { echo stop >>%s; }' % (calls, calls))
        self.assertEqual(open(calls).read().split(), ['start', 'steam', 'stop'])
        # Without Hearth Input's functions (or with the core turned off): nothing, and no error.
        os.remove(calls)
        self.fake('gamescope', 'exit 0')
        r = self.run_mode()
        self.assertNotIn('not found', r.stderr)

    def test_core_service_never_does_hearths_game_mode_work(self):
        text = block()
        start = text[text.index('steam_core_start() {'):text.index('steam_core_stop() {')]
        for word in ('padmouse', 'gamescopectl', 'InputPlumber', 'busctl', 'pkill', 'steam_keep', 'reset_controllers'):
            self.assertNotIn(word, start)
        self.assertIn('--while "$RUN/hearth-steam-mode.pid"', start)

    def test_session_loop_never_starts_or_repeats_steam_mode(self):
        text = SESSION.read_text()
        self.assertIn('[ "$mode" = steam ] && mode=game', text)
        self.assertIn('[ "$mode" = steam ] && [ "$next" = steam ] && next=game', text)
        self.assertIn('[ -n "$HEARTH_SEAT" ] && [ "$next" = steam ] && next=game', text)
        self.assertIn('game|desktop|steam)', text)
        self.assertIn('elif [ "$mode" = steam ]; then run_steam_mode; code=$?', text)
        r = subprocess.run(['bash', '-n', str(SESSION)], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


def start_lines():
    """The session's own lines that decide to start in Steam Game Mode (after the >>> <<< block, before the loop)."""
    text = SESSION.read_text()
    m = re.search(r'\nSD_FIRST=""\n(if \[ "\$mode" = game \] && steam_default_wanted && steam_default_before; then\n.*?\nfi)\n', text, re.S)
    assert m, 'the start-in-Steam-Game-Mode lines not found'
    return m.group(1)


def load_pad_held():
    path = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-pad-held'
    loader = importlib.machinery.SourceFileLoader('hearth_pad_held', str(path))
    spec = importlib.util.spec_from_loader('hearth_pad_held', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class StartInSteamGameMode(unittest.TestCase):
    """Start in Steam Game Mode (HEARTH_STEAM_DEFAULT=1): the session's start decision, run_steam_mode and the
    failure rules, run for real with stand-ins for gamescope and Steam."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-steam-default-')
        self.run_dir = os.path.join(self.tmp, 'run')
        self.bin = os.path.join(self.tmp, 'bin')
        self.home = os.path.join(self.tmp, 'home')
        self.conf = os.path.join(self.tmp, 'session.conf')
        self.state = os.path.join(self.home, '.local/state/nimbus')
        os.makedirs(self.run_dir)
        os.makedirs(self.bin)
        os.makedirs(self.home)
        for tool in ('sh', 'cat', 'date', 'mkdir', 'chmod', 'rm', 'printf', 'env', 'echo', 'sleep', 'timeout', 'true',
                     'sed', 'grep', 'tail', 'mv'):
            p = shutil.which(tool)
            if p:
                os.symlink(p, os.path.join(self.bin, tool))
        self.fake('gamescope', 'echo ran >>"$RUN_DIR/gamescope-ran"; exit 0')
        self.fake('steam', 'exit 0')
        self.sign_in()
        self.set_conf('NIMBUS_GAMESCOPE_ARGS=""\nHEARTH_STEAM_DEFAULT=1\n')
        self.cmdline = os.path.join(self.tmp, 'cmdline')
        open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz root=UUID=x rhgb quiet\n')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake(self, name, body):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(p, 0o755)

    def sign_in(self):
        d = os.path.join(self.home, '.local/share/Steam/config')
        os.makedirs(d, exist_ok=True)
        open(os.path.join(d, 'loginusers.vdf'), 'w').write('"users" { "1" { "AccountName" "x" } }\n')

    def set_conf(self, text):
        open(self.conf, 'w').write(text)

    def start(self, extra_env=None):
        """Sign-in: the session's start lines, then (if it chose Steam Game Mode) one run like the loop does."""
        script = os.path.join(self.tmp, 'start.sh')
        with open(script, 'w') as f:
            f.write('RUN=%s\nNEXT="$RUN/nimbus-next"\nLOG=%s\nSESSION_CONF=%s\nENV_GAMESCOPE_ARGS=""\n'
                    % (self.run_dir, os.path.join(self.tmp, 'session.log'), self.conf))
            f.write('log() { echo "$*" >>"$LOG"; }\nhave() { command -v "$1" >/dev/null 2>&1; }\n')
            f.write(block() + '\nmode=game\nrm -f "$NEXT"\n' + start_lines() + '\necho "start=$mode"\n')
            f.write('if [ "$mode" = steam ]; then run_steam_mode; steam_default_after; fi\necho "next=$(cat "$NEXT" 2>/dev/null)"\n')
        env = {'PATH': self.bin, 'HOME': self.home, 'RUN_DIR': self.run_dir, 'STEAM_MODE_MIN_SECS': '20',
               'HEARTH_DECKY_UNIT': os.path.join(self.tmp, 'no-decky.service'), 'HEARTH_CMDLINE': self.cmdline,
               'HEARTH_LIVE_FLAG': os.path.join(self.tmp, 'no-live'), 'HEARTH_SAFE_FLAG': os.path.join(self.tmp, 'no-safe'),
               'HEARTH_PAD_HELD': os.path.join(self.tmp, 'no-pad-held'), 'USER': 'player'}
        env.update(extra_env or {})
        r = subprocess.run(['/bin/sh', script], env=env, capture_output=True, text=True, timeout=60)
        out = dict(l.split('=', 1) for l in r.stdout.split('\n') if '=' in l)
        self.assertIn('start', out, r.stderr)
        return out

    def on(self):
        return 'HEARTH_STEAM_DEFAULT=1' in open(self.conf).read()

    def note(self):
        try:
            return open(os.path.join(self.state, 'steam-default-off')).read().strip()
        except OSError:
            return None

    def fails(self):
        try:
            return [l for l in open(os.path.join(self.state, 'steam-default-fails')).read().split('\n') if l]
        except OSError:
            return []

    def ran(self):
        return os.path.exists(os.path.join(self.run_dir, 'gamescope-ran'))

    def test_off_by_default(self):
        self.set_conf('NIMBUS_GAMESCOPE_ARGS=""\n')
        self.assertEqual(self.start()['start'], 'game')
        self.assertFalse(self.ran())
        self.set_conf('HEARTH_STEAM_DEFAULT=0\n')
        self.assertEqual(self.start()['start'], 'game')

    def test_on_starts_steam_game_mode_and_a_good_run_clears_the_count(self):
        r = self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})
        self.assertEqual(r['start'], 'steam')
        self.assertTrue(self.ran())
        self.assertEqual(r['next'], 'game', "a plain exit goes to Hearth's Game Mode")
        self.assertTrue(self.on())
        self.assertEqual(self.fails(), [], 'a good run clears the count')
        self.assertIsNone(self.note())

    def test_switch_asked_for_is_followed_and_not_a_failure(self):
        self.fake('gamescope', 'echo desktop >"$RUN_DIR/nimbus-next"; exit 0')
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'desktop'))
        self.assertTrue(self.on())
        self.assertEqual(self.fails(), [], 'Switch to Desktop right away is not a failed start')
        self.fake('gamescope', 'echo game >"$RUN_DIR/nimbus-next"; exit 0')
        r = self.start()
        self.assertEqual(r['next'], 'game')
        self.assertEqual(self.fails(), [])

    def test_error_within_20s_turns_it_off(self):
        self.fake('gamescope', 'exit 1')
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'game'))
        self.assertFalse(self.on(), 'session.conf says 0 now')
        self.assertIn('HEARTH_STEAM_DEFAULT=0', open(self.conf).read())
        self.assertIn('ended right away (code 1)', self.note())
        self.assertIn('turned off by itself', open(os.path.join(self.tmp, 'session.log')).read())
        self.assertEqual(self.start()['start'], 'game', 'the next sign-in is Hearth\'s Game Mode')

    def test_two_short_runs_in_a_row_turn_it_off(self):
        r = self.start()
        self.assertEqual(r['start'], 'steam')
        self.assertTrue(self.on(), 'one short run: still on')
        self.assertEqual(len(self.fails()), 1)
        r = self.start()
        self.assertEqual(r['start'], 'steam')
        self.assertFalse(self.on())
        self.assertIn('twice within 10 minutes', self.note())
        self.assertEqual(self.start()['start'], 'game')

    def test_starts_that_never_finished_count(self):
        # Two starts in the last 10 minutes that never came back (the device hung or was turned off): no third try.
        import time as t
        os.makedirs(self.state)
        now = int(t.time())
        open(os.path.join(self.state, 'steam-default-fails'), 'w').write('%d\n%d\n' % (now - 300, now - 60))
        r = self.start()
        self.assertEqual(r['start'], 'game')
        self.assertFalse(self.ran())
        self.assertFalse(self.on())
        self.assertIn("didn't start properly twice", self.note())

    def test_old_failures_dont_count(self):
        import time as t
        os.makedirs(self.state)
        now = int(t.time())
        open(os.path.join(self.state, 'steam-default-fails'), 'w').write('%d\n%d\n' % (now - 4000, now - 3000))
        r = self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})
        self.assertEqual(r['start'], 'steam')
        self.assertTrue(self.on())

    def test_missing_steam_or_gamescope_turns_it_off(self):
        os.remove(os.path.join(self.bin, 'steam'))
        self.assertEqual(self.start()['start'], 'game')
        self.assertFalse(self.on())
        self.assertIn("isn't installed", self.note())
        self.assertFalse(self.ran())

    def test_safe_graphics_turns_it_off(self):
        safe = os.path.join(self.tmp, 'safe')
        open(safe, 'w').close()
        self.assertEqual(self.start({'HEARTH_SAFE_FLAG': safe})['start'], 'game')
        self.assertFalse(self.on())
        self.assertIn('safe graphics', self.note())

    def test_not_signed_in_turns_it_off(self):
        os.remove(os.path.join(self.home, '.local/share/Steam/config/loginusers.vdf'))
        self.assertEqual(self.start()['start'], 'game')
        self.assertFalse(self.on())
        self.assertIn("isn't signed in", self.note())

    def test_b_or_guide_held_skips_it_this_time(self):
        held = os.path.join(self.tmp, 'pad-held')
        open(held, 'w').write('# stand-in\n')
        self.fake('python3', 'echo "$@" >"$RUN_DIR/pad-asked"; exit 0')
        self.assertEqual(self.start({'HEARTH_PAD_HELD': held})['start'], 'game')
        self.assertFalse(self.ran())
        self.assertTrue(self.on(), 'skipping once leaves the setting on')
        self.assertIn('--window', open(os.path.join(self.run_dir, 'pad-asked')).read())
        self.fake('python3', 'exit 1')   # nothing held: Steam Game Mode as usual
        self.assertEqual(self.start({'HEARTH_PAD_HELD': held, 'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam')

    def test_kernel_option_skips_it_this_time(self):
        open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz quiet hearth.mode=game\n')
        self.assertEqual(self.start()['start'], 'game')
        self.assertTrue(self.on())
        open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz quiet hearth.mode=gamex\n')
        self.assertEqual(self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam')

    def test_extra_seats_and_the_live_usb_never_get_it(self):
        self.assertEqual(self.start({'HEARTH_SEAT': '2'})['start'], 'game')
        self.assertTrue(self.on(), "an extra player's start doesn't change the setting")
        live = os.path.join(self.tmp, 'hearth-live')
        open(live, 'w').close()
        self.assertEqual(self.start({'HEARTH_LIVE_FLAG': live})['start'], 'game')
        self.assertEqual(self.start({'USER': 'liveuser'})['start'], 'game')
        self.assertFalse(self.ran())

    def test_session_loop_wiring(self):
        text = SESSION.read_text()
        # Only from Hearth's Game Mode start (Desktop Mode start, Recovery's Desktop Mode this time and extra players
        # are decided first), and the loop counts manual Steam Game Mode runs too while it's on.
        i_seat = text.index('[ -n "$HEARTH_SEAT" ] && mode=game\n')
        i_start = text.index('if [ "$mode" = game ] && steam_default_wanted && steam_default_before; then')
        self.assertLess(i_seat, i_start)
        self.assertIn('[ "$mode" = steam ] && [ -z "$SD_FIRST" ] && { steam_default_before || :; }', text)
        self.assertIn('[ "$mode" = steam ] && steam_default_after', text)
        self.assertIn('[ "$mode" = steam ] && [ "$next" = steam ] && next=game', text, 'never follows itself')


class PadHeld(unittest.TestCase):
    def test_b_or_guide(self):
        m = load_pad_held()
        self.assertTrue(m.wanted({305}))
        self.assertTrue(m.wanted({316, 304}))
        self.assertFalse(m.wanted({304, 314, 315}), 'A, Select, Start are not a skip')
        buf = bytearray(96)
        buf[305 // 8] |= 1 << (305 % 8)
        self.assertEqual(m.keys_from(buf), {305})

    def test_looks_for_the_window_then_gives_up(self):
        m = load_pad_held()
        d = tempfile.mkdtemp()
        try:
            for n in ('event0', 'event1'):
                open(os.path.join(d, n), 'w').close()
            pat = os.path.join(d, 'event*')
            self.assertFalse(m.held(0, pat, read=lambda p: {304}))
            self.assertTrue(m.held(0, pat, read=lambda p: {305} if p.endswith('event1') else set()))
            self.assertFalse(m.held(0, pat), 'plain files are not controllers')
        finally:
            shutil.rmtree(d, ignore_errors=True)


class NimbusModeSteam(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-nimbus-mode-')
        self.bin = os.path.join(self.tmp, 'bin')
        os.makedirs(self.bin)
        for tool in ('sh', 'cat', 'id', 'kill', 'echo', 'rm'):
            p = shutil.which(tool)
            if p:
                os.symlink(p, os.path.join(self.bin, tool))
        self.calls = os.path.join(self.tmp, 'calls')
        for name in ('pkill', 'setsid'):
            with open(os.path.join(self.bin, name), 'w') as f:
                f.write('#!/bin/sh\necho "%s $*" >>%s\n' % (name, self.calls))
            os.chmod(os.path.join(self.bin, name), 0o755)

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def mode(self, *args):
        env = {'PATH': self.bin, 'XDG_RUNTIME_DIR': self.tmp}
        return subprocess.run(['/bin/sh', str(MODE), *args], env=env, capture_output=True, text=True, timeout=20)

    def test_steam_is_a_mode_you_can_ask_for(self):
        r = self.mode('steam')
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(open(os.path.join(self.tmp, 'nimbus-next')).read().strip(), 'steam')
        self.assertIn('pkill -TERM -x gamescope', open(self.calls).read())

    def test_current_says_steam_and_leaving_closes_steam_first(self):
        with open(os.path.join(self.tmp, 'hearth-steam-mode.pid'), 'w') as f:
            f.write(str(os.getpid()))
        self.assertEqual(self.mode('current').stdout.strip(), 'steam')
        self.mode('game')
        self.assertEqual(open(os.path.join(self.tmp, 'nimbus-next')).read().strip(), 'game')
        # setsid runs in the background: wait for it.
        for _ in range(100):
            if os.path.exists(self.calls) and 'setsid' in open(self.calls).read():
                break
            time.sleep(0.05)
        calls = open(self.calls).read()
        self.assertIn('steam -shutdown', calls)
        self.assertIn('setsid', calls)

    def test_usage_lists_steam(self):
        self.assertIn('game|desktop|steam|logout|current', self.mode('bogus').stderr)


class DeckyHelper(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-decky-')
        self.root = os.path.join(self.tmp, 'root')
        self.home = os.path.join(self.tmp, 'home')
        os.makedirs(self.home)
        self.old = dict(os.environ)
        os.environ['HEARTH_DECKY_ROOT'] = self.root
        os.environ['HEARTH_DECKY_TEST_HOME'] = self.home
        self.m = load_decky()
        self.user = pwd.getpwuid(os.getuid()).pw_name

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.old)
        shutil.rmtree(self.tmp, ignore_errors=True)

    def ship_plugin(self):
        d = os.path.join(self.root, 'usr/share/hearth/decky-hearth')
        os.makedirs(os.path.join(d, 'dist'))
        open(os.path.join(d, 'plugin.json'), 'w').write('{"name": "Hearth"}')
        open(os.path.join(d, 'dist', 'index.js'), 'w').write('// built')

    def test_service_never_starts_at_boot_and_keeps_its_file(self):
        t = self.m.unit_text('/home/x', 'x', '/var/lib/hearth-decky/PluginLoader')
        self.assertNotIn('[Install]', t)
        self.assertNotIn('WantedBy', t)
        self.assertIn('Environment=KEEP_SYSTEMD_SERVICE=1', t)
        self.assertIn('Environment=UNPRIVILEGED_USER=x', t)
        self.assertIn('Environment=PRIVILEGED_PATH=/home/x/homebrew', t)
        self.assertIn('ExecStart=/var/lib/hearth-decky/PluginLoader', t)
        self.assertIn('Restart=on-failure', t)

    def test_picks_the_newest_stable_release_for_the_cpu(self):
        url = 'https://github.com/SteamDeckHomebrew/decky-loader/releases/download/v3.1.0/'
        rel = [{'tag_name': 'v3.2.0-pre1', 'prerelease': True, 'assets': [{'name': 'PluginLoader', 'browser_download_url': url + 'PluginLoader'}]},
               {'tag_name': 'v3.1.0', 'prerelease': False, 'assets': [{'name': 'PluginLoader', 'browser_download_url': url + 'PluginLoader'},
                                                                      {'name': 'PluginLoader-arm64', 'browser_download_url': url + 'PluginLoader-arm64'}]}]
        self.assertEqual(self.m.pick_asset(rel, 'x86_64'), ('v3.1.0', url + 'PluginLoader'))
        self.assertEqual(self.m.pick_asset(rel, 'aarch64'), ('v3.1.0', url + 'PluginLoader-arm64'))
        evil = [{'tag_name': 'v9', 'prerelease': False, 'assets': [{'name': 'PluginLoader', 'browser_download_url': 'https://evil.example/PluginLoader'}]}]
        self.assertIsNone(self.m.pick_asset(evil, 'x86_64'))
        self.assertIsNone(self.m.pick_asset({'message': 'rate limited'}, 'x86_64'))

    def test_install_and_remove(self):
        self.ship_plugin()
        elf = b'\x7fELF' + b'\0' * (2 * 1024 * 1024)
        r = self.m.install(self.user, download=lambda: ('v3.1.0', lambda: elf))
        self.assertEqual(r['via'], 'hearth')
        self.assertTrue(r['plugin'])
        binary = os.path.join(self.root, 'var/lib/hearth-decky/PluginLoader')
        self.assertTrue(os.access(binary, os.X_OK))
        unit = open(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')).read()
        self.assertIn('ExecStart=' + binary, unit)
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth/plugin.json')))
        cef = os.path.join(self.home, '.steam/steam/.cef-enable-remote-debugging')
        self.assertTrue(os.path.isfile(cef))
        self.m.remove(self.user)
        self.assertFalse(os.path.exists(binary))
        self.assertFalse(os.path.exists(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')))
        self.assertFalse(os.path.exists(os.path.join(self.home, 'homebrew/plugins/decky-hearth')))
        self.assertFalse(os.path.exists(cef), 'the CEF debugging file Hearth made is taken away again')

    def test_a_cef_file_you_had_stays_on_remove(self):
        cef = os.path.join(self.home, '.steam/steam/.cef-enable-remote-debugging')
        os.makedirs(os.path.dirname(cef))
        open(cef, 'w').close()
        self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
        self.m.remove(self.user)
        self.assertTrue(os.path.exists(cef))

    def test_not_a_program_is_refused(self):
        with self.assertRaises(ValueError):
            self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'<html>rate limited</html>'))
        self.assertFalse(os.path.exists(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')))

    def test_deckys_own_install_is_left_alone(self):
        self.ship_plugin()
        up = os.path.join(self.root, 'etc/systemd/system/plugin_loader.service')
        os.makedirs(os.path.dirname(up))
        open(up, 'w').write('[Service]\n')
        r = self.m.install(self.user, download=lambda: self.fail('must not download'))
        self.assertEqual(r['via'], 'decky')
        self.assertTrue(r['plugin'])
        self.assertFalse(os.path.exists(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')))

    def test_only_peoples_accounts(self):
        with self.assertRaises(ValueError):
            self.m.person('root')
        with self.assertRaises(ValueError):
            self.m.person('no-such-account-hearth')

    def test_admin_and_polkit(self):
        admin = ADMIN.read_text()
        self.assertIn("cmd in ('decky-install', 'decky-remove')", admin)
        self.assertIn("helper, cmd[6:], caller().pw_name", admin)
        rules = RULES.read_text()
        m = re.search(r'/\^\\S\+\\s\+decky-install\(\\s\|\$\)/', rules)
        self.assertTrue(m, 'decky-install asks for an administrator\'s password')
        rx = re.compile(r'^\S+\s+decky-install(\s|$)')
        self.assertTrue(rx.search('/usr/libexec/nimbus/hearth-admin decky-install'))
        self.assertFalse(rx.search('/usr/libexec/nimbus/hearth-admin decky-start'))
        self.assertFalse(rx.search('/usr/libexec/nimbus/hearth-admin decky-remove'))


class SteamosUpdate(unittest.TestCase):
    """hearth-steamos-update: Steam's steamos-update contract (check: 0 available / 7 up to date; apply: "NN%" lines,
    0 ready for a restart, 7 nothing new), backed by a stand-in for hearth-admin. Never restarts anything."""
    def setUp(self):
        self.tmp = tempfile.mkdtemp(prefix='hearth-steamos-update-')
        self.admin = os.path.join(self.tmp, 'hearth-admin')
        self.calls = os.path.join(self.tmp, 'calls')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake_admin(self, check_json, update_lines='', update_json=None, code=0):
        with open(self.admin, 'w') as f:
            f.write('#!/bin/sh\necho "$*" >>%s\ncase "$1" in\n' % self.calls)
            f.write("  update-check) echo 'log: bootc says hi'; echo '%s'; exit %d ;;\n" % (json.dumps(check_json), code))
            f.write("  update) printf '%s'; echo '%s'; exit %d ;;\n" % (update_lines, json.dumps(update_json or check_json), code))
            f.write('  *) exit 3 ;;\nesac\n')
        os.chmod(self.admin, 0o755)

    def upd(self, *args):
        env = dict(os.environ, HEARTH_STEAMOS_UPDATE_ADMIN=self.admin, HEARTH_STEAMOS_UPDATE_PKEXEC='')
        return subprocess.run([sys.executable, str(STEAMOS_UPDATE)] + list(args), env=env, capture_output=True, text=True, timeout=30)

    def test_check(self):
        self.fake_admin({'booted': {'version': '0.6.60'}, 'staged': None, 'available': None})
        r = self.upd('check')
        self.assertEqual(r.returncode, 7, r.stderr)
        self.assertIn('up to date', r.stderr)
        self.fake_admin({'booted': {}, 'staged': None, 'available': {'version': '0.6.61'}})
        r = self.upd('--enable-duplicate-detection', 'check')
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn('Update available: 0.6.61', r.stdout)
        self.fake_admin({'booted': {}, 'staged': {'version': '0.6.61'}, 'available': None})
        self.assertEqual(self.upd('check').returncode, 0, 'downloaded and waiting for a restart counts as available')
        with open(self.admin, 'w') as f:
            f.write('#!/bin/sh\necho "error: no network"\nexit 1\n')
        r = self.upd('check')
        self.assertNotIn(r.returncode, (0, 7))
        self.assertIn('no network', r.stderr)

    def test_supports_duplicate_detection_does_nothing(self):
        self.fake_admin({})
        self.assertEqual(self.upd('--supports-duplicate-detection').returncode, 0)
        self.assertFalse(os.path.exists(self.calls), 'hearth-admin not run')

    def test_apply_prints_steams_progress(self):
        lines = 'progress: 0.0 Starting\\nlog: x\\nprogress: 12.4 Downloading\\nprogress: 12.9 Downloading\\nprogress: 55.0 Installing\\nstage: {}\\nprogress: 40.0 odd\\nprogress: 90.0 Updating apps\\n'
        self.fake_admin({}, lines, {'booted': {}, 'staged': {'version': '0.6.61'}, 'available': None})
        r = self.upd()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.split(), ['0%', '12%', '55%', '90%', '100%'], 'whole numbers, never back, 100% at the end')
        self.assertIn('Restart to finish', r.stderr)
        self.assertEqual(open(self.calls).read().split(), ['update'])
        self.fake_admin({}, 'progress: 100.0 Hearth OS is already up to date.\\n', {'booted': {}, 'staged': None, 'available': None})
        self.assertEqual(self.upd('--enable-duplicate-detection').returncode, 7)
        with open(self.admin, 'w') as f:
            f.write("#!/bin/sh\nprintf 'progress: 5.0 x\\nerror: another update is already running\\n'\nexit 1\n")
        r = self.upd()
        self.assertEqual(r.returncode, 1)
        self.assertIn('another update is already running', r.stderr)

    def test_never_restarts(self):
        text = STEAMOS_UPDATE.read_text()
        for word in ('reboot', 'systemctl', 'shutdown', 'loginctl'):
            self.assertNotIn(word + ' ', text.split('"""', 2)[2], word)


if __name__ == '__main__':
    unittest.main()
