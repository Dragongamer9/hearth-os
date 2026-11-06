"""Steam Game Mode (Hearth OS's Game Mode, docs/STEAM-GAME-MODE.md): nimbus-session's run_steam_mode (run for real between its
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
DECKY_RULES = ROOT / 'os/system_files/usr/share/polkit-1/rules.d/49-hearth-decky.rules'
DECKY_TEMPLATE = ROOT / 'os/system_files/usr/lib/systemd/system/hearth-decky@.service'
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
               'HEARTH_DECKY_UNIT': os.path.join(self.tmp, 'no-decky.service'),
               'HEARTH_DECKY_BUILTIN': os.path.join(self.tmp, 'no-decky@.service'), 'USER': 'player'}
        env.update(extra_env or {})
        r = subprocess.run(['/bin/sh', script], env=env, capture_output=True, text=True, timeout=60)
        self.assertIn('rc=0', r.stdout, r.stderr)
        return r

    def read(self, name):
        try:
            return open(os.path.join(self.run_dir, name)).read().strip()
        except OSError:
            return None

    def test_no_steam_falls_back_to_desktop_mode(self):
        self.fake('gamescope', 'exit 0')
        self.run_mode()
        self.assertEqual(self.read('nimbus-next'), 'desktop', "Desktop Mode, never Hearth's archived Game Mode")
        self.assertIn("isn't installed", self.read('hearth-steam-mode-failed'))

    def test_no_gamescope_falls_back(self):
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.assertTrue(self.read('hearth-steam-mode-failed'))

    def test_steam_never_set_up_starts_desktop_mode_instead_of_a_black_screen(self):
        # Hearth 1.0 on a laptop that had never opened Steam: Debian's launcher asked its "Steam installer" question in
        # a window Steam Game Mode never shows, so the screen stayed black. Desktop Mode now, and the note says why.
        ran = os.path.join(self.tmp, 'ran')
        self.fake('gamescope', 'echo yes >%s' % ran)
        self.fake('steam', 'exit 0')
        for tool in ('readlink',):
            p = shutil.which(tool)
            if p and not os.path.exists(os.path.join(self.bin, tool)):
                os.symlink(p, os.path.join(self.bin, tool))
        launcher = os.path.join(self.bin, 'steam')
        os.makedirs(os.path.join(self.tmp, 'home', '.steam', 'debian-installation'))
        self.run_mode({'HEARTH_STEAM_FIRSTRUN_LAUNCHERS': launcher})
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.assertIn("isn't set up", self.read('hearth-steam-mode-failed'))
        self.assertFalse(os.path.exists(ran), 'gamescope must not start')

    def test_steam_set_up_before_still_starts_steam_game_mode(self):
        ran = os.path.join(self.tmp, 'ran')
        self.fake('gamescope', 'echo yes >%s' % ran)
        self.fake('steam', 'exit 0')
        for tool in ('readlink',):
            p = shutil.which(tool)
            if p and not os.path.exists(os.path.join(self.bin, tool)):
                os.symlink(p, os.path.join(self.bin, tool))
        launcher = os.path.join(self.bin, 'steam')
        d = os.path.join(self.tmp, 'home', '.local', 'share', 'Steam')
        os.makedirs(d)
        open(os.path.join(d, 'steam.sh'), 'w').close()
        self.run_mode({'HEARTH_STEAM_FIRSTRUN_LAUNCHERS': launcher})
        self.assertTrue(os.path.exists(ran), 'Steam Game Mode starts when Steam has run before')
        self.assertNotIn("isn't set up", self.read('hearth-steam-mode-failed') or '')

    def test_safe_graphics_falls_back_to_desktop_mode(self):
        ran = os.path.join(self.tmp, 'ran')
        self.fake('gamescope', 'echo yes >%s' % ran)
        self.fake('steam', 'exit 0')
        safe = os.path.join(self.tmp, 'safe')
        open(safe, 'w').close()
        self.run_mode({'HEARTH_SAFE_FLAG': safe})
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.assertIn('safe graphics', self.read('hearth-steam-mode-failed'))
        self.assertFalse(os.path.exists(ran))

    def test_hearths_own_game_mode_only_with_the_developer_switch(self):
        self.fake('gamescope', 'exit 1')
        self.fake('steam', 'exit 0')
        self.run_mode(before='legacy_game() { true; }')
        self.assertEqual(self.read('nimbus-next'), 'game', 'HEARTH_LEGACY_GAME_MODE=1 / live USB: back to Hearth\'s Game Mode')
        self.fake('gamescope', 'exit 0')
        os.remove(os.path.join(self.run_dir, 'nimbus-next'))   # the session empties it before every mode
        self.run_mode(before='legacy_game() { true; }')
        self.assertEqual(self.read('nimbus-next'), 'game')
        os.remove(os.path.join(self.run_dir, 'nimbus-next'))
        self.run_mode(before='legacy_game() { false; }')
        self.assertEqual(self.read('nimbus-next'), 'steam')

    def test_extra_multiseat_player_never_gets_it(self):
        self.fake('gamescope', 'echo ran >"$HOME.ran"')
        self.fake('steam', 'exit 0')
        self.run_mode({'HEARTH_SEAT': '2'})
        self.assertEqual(self.read('nimbus-next'), 'desktop')
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
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.assertIn('ended right away (code 1)', self.read('hearth-steam-mode-failed'))
        args = open(argv).read().split('\n')
        self.assertEqual(args[:4], ['-e', '-f', '--xwayland-count', '2'], 'Steam starts games on a second X display')
        tail = args[args.index('--') + 1:]
        self.assertEqual(tail[0], 'env')
        self.assertTrue(tail[1].startswith('PATH=%s/hearth-steam-bin:' % self.run_dir))
        self.assertEqual(tail[2:6], ['steam', '-gamepadui', '-steamos3', '-steampal'])
        self.assertIsNone(self.read('hearth-steam-mode.pid'), 'the running mark is gone afterwards')

    def test_steam_ending_by_itself_starts_it_again(self):
        seen = os.path.join(self.tmp, 'seen')
        self.fake('gamescope', 'cat "$RUN_DIR/hearth-steam-mode.pid" >%s; echo "$HEARTH_STEAM_MODE $STEAM_MULTIPLE_XWAYLANDS" >>%s; exit 0' % (seen, seen))
        self.fake('steam', 'exit 0')
        self.run_mode({'RUN_DIR': self.run_dir})
        self.assertEqual(self.read('nimbus-next'), 'steam', 'like SteamOS: Steam again (steam_auto_after stops it after two short runs)')
        self.assertIsNone(self.read('hearth-steam-mode-failed'))
        lines = open(seen).read().split('\n')
        self.assertTrue(lines[0].isdigit(), 'nimbus-mode can see Steam Game Mode is running')
        self.assertEqual(lines[1], '1 1')

    def test_switch_to_desktop_from_steam_is_kept(self):
        self.fake('gamescope', 'echo desktop >"$RUN_DIR/nimbus-next"; exit 0')
        self.fake('steam', 'exit 0')
        self.run_mode({'RUN_DIR': self.run_dir})
        self.assertEqual(self.read('nimbus-next'), 'desktop')
        self.fake('gamescope', 'echo logout >"$RUN_DIR/nimbus-next"; exit 0')
        self.run_mode({'RUN_DIR': self.run_dir})
        self.assertEqual(self.read('nimbus-next'), 'logout', 'signing out is kept (the session ends on it)')

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

    def test_builtin_decky_starts_with_steam_and_stops_after_without_pkexec(self):
        calls = os.path.join(self.tmp, 'systemctl-calls')
        pk = os.path.join(self.tmp, 'pkexec-calls')
        seen = os.path.join(self.tmp, 'seen')
        home = os.path.join(self.tmp, 'home')
        cef = os.path.join(home, '.steam/steam/.cef-enable-remote-debugging')
        self.fake('systemctl', 'echo "$@" >>%s' % calls)
        self.fake('pkexec', 'echo "$@" >>%s' % pk)
        # Steam starts after the CEF flag is there and Decky was asked to start.
        self.fake('gamescope', '[ -e %s ] && grep -q start %s && echo ready >%s; exit 0' % (cef, calls, seen))
        self.fake('steam', 'exit 0')
        unit = os.path.join(self.tmp, 'hearth-decky@.service')
        open(unit, 'w').write('[Service]\n')
        self.run_mode({'HEARTH_DECKY_BUILTIN': unit, 'HEARTH_DECKY_UPSTREAM': os.path.join(self.tmp, 'none')})
        time.sleep(0.5)   # (stop runs in the background)
        lines = open(calls).read().split('\n')
        self.assertIn('--no-ask-password start hearth-decky@player.service', lines)
        self.assertIn('--no-ask-password stop hearth-decky@player.service', lines)
        self.assertEqual(open(seen).read().strip(), 'ready', 'the CEF flag and Decky before Steam')
        for d in ('settings', 'data', 'logs'):
            self.assertTrue(os.path.isdir(os.path.join(home, 'homebrew', d)))
        self.assertIn(cef, open(os.path.join(home, 'homebrew/.hearth-made-this')).read())
        self.assertFalse(os.path.exists(pk), 'no password prompt, no pkexec')

    def test_deckys_own_install_means_the_builtin_one_stays_off(self):
        calls = os.path.join(self.tmp, 'systemctl-calls')
        self.fake('systemctl', 'echo "$@" >>%s' % calls)
        self.fake('gamescope', 'exit 0')
        self.fake('steam', 'exit 0')
        unit = os.path.join(self.tmp, 'hearth-decky@.service')
        up = os.path.join(self.tmp, 'plugin_loader.service')
        for f in (unit, up):
            open(f, 'w').write('[Service]\n')
        self.run_mode({'HEARTH_DECKY_BUILTIN': unit, 'HEARTH_DECKY_UPSTREAM': up})
        self.assertFalse(os.path.exists(calls))

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
        self.assertEqual(self.read('nimbus-next'), 'steam')
        self.assertIn("Hearth's core started", open(os.path.join(self.tmp, 'session.log')).read())

    def test_core_service_that_fails_never_stops_steam_game_mode(self):
        ran = os.path.join(self.tmp, 'steam-ran')
        self.fake_core('exit 1')
        self.fake('gamescope', 'echo yes >%s; exit 0' % ran)
        self.fake('steam', 'exit 0')
        self.run_mode()
        self.assertTrue(os.path.exists(ran), 'Steam Game Mode ran')
        self.assertIsNone(self.read('hearth-steam-mode-failed'))
        self.assertEqual(self.read('nimbus-next'), 'steam')

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

    # ---- multi-seat beside Steam Game Mode (docs/STEAM-GAME-MODE.md "Multi-seat")
    def seats(self, enabled=True, host_check=1, host_rc=0, gs_args=''):
        """A stand-in hearth-seats (run through a fake python3) and its seats.json; returns the env for run_mode."""
        helper = os.path.join(self.tmp, 'hearth-seats')
        open(helper, 'w').write('# stand-in\n')
        conf = os.path.join(self.tmp, 'seats.json')
        with open(conf, 'w') as f:
            json.dump({'enabled': enabled, 'mode': 'nested', 'seats': []}, f)
        calls = os.path.join(self.tmp, 'seats-calls')
        self.fake('python3', 'echo "$*" >>%s\ncase "$2" in gamescope-args) echo "%s" ;; host-check) exit %d ;; '
                  'host) exit %d ;; esac\nexit 0' % (calls, gs_args, host_check, host_rc))
        # Multi-seat is dropped for now: these run with the developer switch that brings it back.
        return {'HEARTH_SEATS_HELPER': helper, 'HEARTH_SEATS_CONFIG': conf, 'HEARTH_LEGACY_MULTIPLAYER': '1'}

    def seats_calls(self):
        try:
            return open(os.path.join(self.tmp, 'seats-calls')).read()
        except OSError:
            return ''

    def test_multiseat_is_off_without_the_developer_switch(self):
        argv = os.path.join(self.tmp, 'argv')
        self.fake('gamescope', 'for a in "$@"; do echo "$a"; done >%s\nexit 0' % argv)
        self.fake('steam', 'exit 0')
        env = self.seats(enabled=True, host_check=0, gs_args='--prefer-vk-device 1002:73bf')
        del env['HEARTH_LEGACY_MULTIPLAYER']
        self.run_mode(env)
        self.assertEqual(self.seats_calls(), '', 'multi-seat set up and on, but dropped: hearth-seats is never asked')
        args = open(argv).read().split('\n')
        self.assertEqual(args[:args.index('--')], ['-e', '-f', '--xwayland-count', '2'])

    def test_without_multiseat_steam_game_mode_is_unchanged(self):
        argv = os.path.join(self.tmp, 'argv')
        self.fake('gamescope', 'for a in "$@"; do echo "$a"; done >%s\nexit 0' % argv)
        self.fake('steam', 'exit 0')
        self.fake('mangoapp', 'exit 0')
        env = self.seats(enabled=False)
        self.run_mode(env)
        self.assertEqual(self.seats_calls(), '', 'multi-seat off: hearth-seats is never asked')
        args = open(argv).read().split('\n')
        self.assertEqual(args[:5], ['-e', '-f', '--xwayland-count', '2', '--mangoapp'])
        self.assertEqual(args[5], '--')

    def test_card_per_player_keeps_player_one_on_their_card(self):
        argv = os.path.join(self.tmp, 'argv')
        self.fake('gamescope', 'for a in "$@"; do echo "$a"; done >%s\nexit 0' % argv)
        self.fake('steam', 'exit 0')
        self.fake('mangoapp', 'exit 0')
        self.run_mode(self.seats(host_check=1, gs_args='--prefer-vk-device 1002:73bf --prefer-output DP-1'))
        args = open(argv).read().split('\n')
        self.assertEqual(args[:args.index('--')], ['-e', '-f', '--xwayland-count', '2', '--prefer-vk-device', '1002:73bf', '--prefer-output', 'DP-1'])
        self.assertNotIn('--mangoapp', args, 'no performance overlay while other players play')
        self.assertNotIn(' host ', self.seats_calls())
        self.assertEqual(self.read('nimbus-next'), 'steam')

    def test_shared_screens_run_steam_inside_the_host(self):
        ran = os.path.join(self.tmp, 'gamescope-ran')
        self.fake('gamescope', 'echo yes >%s; exit 0' % ran)
        self.fake('steam', 'exit 0')
        self.run_mode(self.seats(host_check=0, host_rc=0))
        host = [l for l in self.seats_calls().splitlines() if ' host ' in l + ' ']
        self.assertEqual(len(host), 1, self.seats_calls())
        words = host[0].split()
        self.assertEqual(words[2:6], ['--', 'gamescope', '-e', '-f'])
        self.assertIn('-gamepadui', words)
        self.assertFalse(os.path.exists(ran), "Steam's gamescope ran inside the host only")
        self.assertIn('hosting the other players', open(os.path.join(self.tmp, 'session.log')).read())
        self.assertEqual(self.read('nimbus-next'), 'steam')

    def test_host_that_cant_start_never_stops_steam_game_mode(self):
        ran = os.path.join(self.tmp, 'gamescope-ran')
        self.fake('gamescope', 'echo yes >%s; exit 0' % ran)
        self.fake('steam', 'exit 0')
        self.run_mode(self.seats(host_check=0, host_rc=3))
        self.assertTrue(os.path.exists(ran), 'Steam Game Mode started without the host')
        self.assertIsNone(self.read('hearth-steam-mode-failed'))
        self.assertIn("host couldn't start", open(os.path.join(self.tmp, 'session.log')).read())

    def test_host_can_be_left_out(self):
        ran = os.path.join(self.tmp, 'gamescope-ran')
        self.fake('gamescope', 'echo yes >%s; exit 0' % ran)
        self.fake('steam', 'exit 0')
        env = self.seats(host_check=0)
        env['HEARTH_STEAM_SEATS_HOST'] = '0'
        self.run_mode(env)
        self.assertTrue(os.path.exists(ran))
        self.assertNotIn('host', self.seats_calls())

    def test_steam_game_mode_never_starts_or_stops_other_players(self):
        text = block()
        part = text[text.index('STEAM_SEATS='):text.index('# ---- Steam Game Mode failing twice')]
        for word in ('hearth-seat@', 'systemctl', 'seats-stop', 'seats-start', 'pkill', 'loginctl', 'kill '):
            self.assertNotIn(word, part)

    def test_session_loop_wiring(self):
        text = SESSION.read_text()
        # Every "game" is Steam Game Mode (first-time setup while pending; Hearth's own only with the developer switch).
        self.assertIn('game) if legacy_game; then echo game; elif setup_pending; then echo setup; else echo steam; fi ;;', text)
        self.assertIn('[ "$mode" = steam ] && mode="$(steam_start_mode)"', text)
        self.assertIn('elif [ "$mode" = steam ]; then run_steam_mode; code=$?', text)
        self.assertIn('[ -n "$HEARTH_SEAT" ] && [ "$next" = steam ] && next=game', text)
        self.assertIn('to="$next"; [ -z "$HEARTH_SEAT" ] && to="$(resolve_mode "$next")"', text)
        self.assertIn('[ "$mode" = setup ] && [ -z "$next" ] && {', text, 'a setup run that just ends goes to Steam Game Mode')
        self.assertIn('[ "${HEARTH_LEGACY_GAME_MODE:-0}" = 1 ]', text, 'the developer switch, off by default')
        self.assertNotIn('HEARTH_STEAM_DEFAULT', text, 'starting in Steam Game Mode is always on now')
        r = subprocess.run(['bash', '-n', str(SESSION)], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)


def load_pad_held():
    path = ROOT / 'os/system_files/usr/libexec/nimbus/hearth-pad-held'
    loader = importlib.machinery.SourceFileLoader('hearth_pad_held', str(path))
    spec = importlib.util.spec_from_loader('hearth_pad_held', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


class StartInSteamGameMode(unittest.TestCase):
    """Starting in Steam Game Mode (always, docs/STEAM-GAME-MODE.md): the session's start decision (steam_start_mode),
    run_steam_mode as the loop runs it (steam_auto_before / steam_auto_after), and the two-failures pause, run for real
    with stand-ins for gamescope and Steam."""

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
                     'sed', 'grep', 'tail', 'mv', 'kill'):
            p = shutil.which(tool)
            if p:
                os.symlink(p, os.path.join(self.bin, tool))
        self.fake('gamescope', 'echo ran >>"$RUN_DIR/gamescope-ran"; exit 0')
        self.fake('steam', 'exit 0')
        open(self.conf, 'w').write('NIMBUS_GAMESCOPE_ARGS=""\n')
        self.cmdline = os.path.join(self.tmp, 'cmdline')
        open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz root=UUID=x rhgb quiet\n')

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def fake(self, name, body):
        p = os.path.join(self.bin, name)
        with open(p, 'w') as f:
            f.write('#!/bin/sh\n' + body + '\n')
        os.chmod(p, 0o755)

    def start(self, extra_env=None, explicit=False):
        """A sign-in (or, with explicit, a "Try Steam Game Mode again"): the start decision, then one Steam Game Mode
        run the way the loop does it if that's where it went."""
        script = os.path.join(self.tmp, 'start.sh')
        with open(script, 'w') as f:
            f.write('RUN=%s\nNEXT="$RUN/nimbus-next"\nLOG=%s\nSESSION_CONF=%s\nENV_GAMESCOPE_ARGS=""\n'
                    % (self.run_dir, os.path.join(self.tmp, 'session.log'), self.conf))
            f.write('log() { echo "$*" >>"$LOG"; }\nhave() { command -v "$1" >/dev/null 2>&1; }\n')
            f.write(block() + '\nmode=steam\nrm -f "$NEXT"\n')
            if not explicit:
                f.write('[ "$mode" = steam ] && mode="$(steam_start_mode)"\n')
            f.write('echo "start=$mode"\n')
            f.write('if [ "$mode" = steam ]; then steam_auto_before; run_steam_mode; steam_auto_after; fi\necho "next=$(cat "$NEXT" 2>/dev/null)"\n')
        env = {'PATH': self.bin, 'HOME': self.home, 'RUN_DIR': self.run_dir, 'STEAM_MODE_MIN_SECS': '20',
               'HEARTH_DECKY_UNIT': os.path.join(self.tmp, 'no-decky.service'),
               'HEARTH_DECKY_BUILTIN': os.path.join(self.tmp, 'no-decky@.service'), 'HEARTH_CMDLINE': self.cmdline,
               'HEARTH_SAFE_FLAG': os.path.join(self.tmp, 'no-safe'),
               'HEARTH_PAD_HELD': os.path.join(self.tmp, 'no-pad-held'), 'USER': 'player'}
        env.update(extra_env or {})
        r = subprocess.run(['/bin/sh', script], env=env, capture_output=True, text=True, timeout=60)
        out = dict(l.split('=', 1) for l in r.stdout.split('\n') if '=' in l)
        self.assertIn('start', out, r.stderr)
        return out

    def paused(self):
        try:
            return open(os.path.join(self.state, 'steam-default-off')).read().strip()
        except OSError:
            return None

    def note(self):
        try:
            return open(os.path.join(self.run_dir, 'hearth-steam-mode-failed')).read().strip()
        except OSError:
            return None

    def fails(self):
        try:
            return [l for l in open(os.path.join(self.state, 'steam-default-fails')).read().split('\n') if l]
        except OSError:
            return []

    def ran(self):
        return os.path.exists(os.path.join(self.run_dir, 'gamescope-ran'))

    def test_starts_in_steam_game_mode_and_a_good_run_clears_the_count(self):
        r = self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})
        self.assertEqual(r['start'], 'steam')
        self.assertTrue(self.ran())
        self.assertEqual(r['next'], 'steam', 'Steam ending by itself after a good run: Steam again')
        self.assertEqual(self.fails(), [], 'a good run clears the count')
        self.assertIsNone(self.paused())

    def test_not_signed_in_still_starts(self):
        # Steam's own screens sign in (as on SteamOS): no sign-in is no reason to stay out of Steam Game Mode.
        self.assertEqual(self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam')

    def test_switch_asked_for_is_followed_and_not_a_failure(self):
        self.fake('gamescope', 'echo desktop >"$RUN_DIR/nimbus-next"; exit 0')
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'desktop'))
        self.assertEqual(self.fails(), [], 'Switch to Desktop right away is not a failed start')
        self.assertIsNone(self.note())

    def test_error_within_20s_opens_desktop_mode_and_twice_pauses_it(self):
        self.fake('gamescope', 'exit 1')
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'desktop'))
        self.assertIn('ended right away (code 1)', self.note())
        self.assertIsNone(self.paused(), 'one failure: Desktop Mode this time, Steam Game Mode again at the next start')
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'desktop'))
        self.assertIn('ended right away', self.paused())
        self.assertIn('paused', open(os.path.join(self.tmp, 'session.log')).read())
        os.remove(os.path.join(self.run_dir, 'hearth-steam-mode-failed'))
        r = self.start()
        self.assertEqual(r['start'], 'desktop', 'paused: Desktop Mode at every start until you try again')
        self.assertIn('ended right away', self.note(), 'and Mantel says why again')

    def test_two_short_runs_in_a_row_open_desktop_mode(self):
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'steam'), 'one short run: Steam once more')
        self.assertEqual(len(self.fails()), 1)
        r = self.start()
        self.assertEqual((r['start'], r['next']), ('steam', 'desktop'))
        self.assertIn('closed by itself twice within 10 minutes', self.paused())
        self.assertIn('closed by itself twice', self.note())
        self.assertEqual(self.start()['start'], 'desktop')

    def test_starts_that_never_finished_count(self):
        # Two starts in the last 10 minutes that never came back (the device hung or was turned off): no third try.
        os.makedirs(self.state)
        now = int(time.time())
        open(os.path.join(self.state, 'steam-default-fails'), 'w').write('%d\n%d\n' % (now - 300, now - 60))
        r = self.start()
        self.assertEqual(r['start'], 'desktop')
        self.assertFalse(self.ran())
        self.assertIn("didn't start properly twice", self.paused())
        self.assertIn("didn't start properly twice", self.note())

    def test_old_failures_dont_count(self):
        os.makedirs(self.state)
        now = int(time.time())
        open(os.path.join(self.state, 'steam-default-fails'), 'w').write('%d\n%d\n' % (now - 4000, now - 3000))
        r = self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})
        self.assertEqual(r['start'], 'steam')

    def test_trying_again_always_runs_and_a_good_run_ends_the_pause(self):
        os.makedirs(self.state)
        open(os.path.join(self.state, 'steam-default-off'), 'w').write('Steam ended right away (code 1)\n')
        self.assertEqual(self.start()['start'], 'desktop')
        r = self.start({'STEAM_DEFAULT_GOOD_SECS': '0'}, explicit=True)
        self.assertTrue(self.ran(), '"Try Steam Game Mode again" runs even while paused')
        self.assertIsNone(self.paused(), 'a good run ends the pause')
        self.assertEqual(self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam')

    def test_missing_steam_or_gamescope_opens_desktop_mode(self):
        os.remove(os.path.join(self.bin, 'steam'))
        self.assertEqual(self.start()['start'], 'desktop')
        self.assertIn("isn't installed", self.note())
        self.assertIsNone(self.paused(), 'checked again at every start (installing Steam is enough)')
        self.assertFalse(self.ran())
        self.assertEqual(self.fails(), [])

    def test_safe_graphics_opens_desktop_mode(self):
        safe = os.path.join(self.tmp, 'safe')
        open(safe, 'w').close()
        self.assertEqual(self.start({'HEARTH_SAFE_FLAG': safe})['start'], 'desktop')
        self.assertIn('safe graphics', self.note())
        self.assertIsNone(self.paused())

    def test_b_or_guide_held_opens_desktop_mode_this_time(self):
        held = os.path.join(self.tmp, 'pad-held')
        open(held, 'w').write('# stand-in\n')
        self.fake('python3', 'echo "$@" >"$RUN_DIR/pad-asked"; exit 0')
        self.assertEqual(self.start({'HEARTH_PAD_HELD': held})['start'], 'desktop')
        self.assertFalse(self.ran())
        self.assertIsNone(self.paused(), 'only this time')
        self.assertIsNone(self.note(), 'asked for: no "didn\'t start" notice')
        self.assertIn('--window', open(os.path.join(self.run_dir, 'pad-asked')).read())
        self.fake('python3', 'exit 1')   # nothing held: Steam Game Mode as usual
        self.assertEqual(self.start({'HEARTH_PAD_HELD': held, 'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam')

    def test_kernel_option(self):
        open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz quiet hearth.mode=desktop\n')
        self.assertEqual(self.start()['start'], 'desktop')
        self.assertIsNone(self.paused())
        for arg in ('hearth.mode=game', 'hearth.mode=desktopx'):
            open(self.cmdline, 'w').write('BOOT_IMAGE=/vmlinuz quiet %s\n' % arg)
            self.assertEqual(self.start({'STEAM_DEFAULT_GOOD_SECS': '0'})['start'], 'steam', arg)

    def test_session_start_wiring(self):
        text = SESSION.read_text()
        # Recovery's Desktop Mode this time and extra players are decided before Steam Game Mode's start checks.
        i_rec = text.index('[ -f /run/hearth/start-desktop ] && [ -z "$HEARTH_SEAT" ] && mode=desktop')
        i_start = text.index('[ "$mode" = steam ] && mode="$(steam_start_mode)"')
        self.assertLess(i_rec, i_start)
        self.assertIn('sd=""; [ "$mode" = steam ] && ! legacy_game && { steam_auto_before; sd=1; }', text)
        self.assertIn('[ -n "$sd" ] && steam_auto_after', text)


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
        # Up to 20 s by the clock (a busy CI runner was slower than 5 s once; other test files stub time.sleep out).
        end = time.monotonic() + 20
        while time.monotonic() < end:
            if os.path.exists(self.calls) and 'setsid' in open(self.calls).read():
                break
            time.sleep(0.05)
        calls = open(self.calls).read()
        self.assertIn('steam -shutdown', calls)
        self.assertIn('setsid', calls)

    def test_usage_lists_steam(self):
        self.assertIn('game|desktop|steam|server|logout|current', self.mode('bogus').stderr)


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
        d = os.path.join(self.root, 'usr/share/hearth/decky-hearth-sync')
        os.makedirs(os.path.join(d, 'dist'))
        open(os.path.join(d, 'plugin.json'), 'w').write('{"name": "Hearth Sync", "author": "Dragongamer9"}')
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
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/plugin.json')))
        cef = os.path.join(self.home, '.steam/steam/.cef-enable-remote-debugging')
        self.assertTrue(os.path.isfile(cef))
        self.m.remove(self.user)
        self.assertFalse(os.path.exists(binary))
        self.assertFalse(os.path.exists(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')))
        self.assertFalse(os.path.exists(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync')))
        self.assertFalse(os.path.exists(cef), 'the CEF debugging file Hearth made is taken away again')

    def test_start_brings_the_images_new_plugin(self):
        self.ship_plugin()
        os.makedirs(os.path.join(self.root, 'etc/systemd/system'))
        open(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service'), 'w').write(
            self.m.unit_text(self.home, 'x', '/var/lib/hearth-decky/PluginLoader'))
        self.m.copy_plugin(self.home, owner_root=False)
        self.assertNotIn('plugin', self.m.start_stop('start'), 'the same plugin: left as it is')
        open(os.path.join(self.root, 'usr/share/hearth/decky-hearth-sync/dist/index.js'), 'w').write('// new version')
        self.assertEqual(self.m.start_stop('start')['plugin'], 'updated')
        self.assertEqual(open(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/dist/index.js')).read(), '// new version')
        self.assertNotIn('plugin', self.m.start_stop('stop'))

    def test_hearth_link_ships_next_to_hearth_sync(self):
        self.ship_plugin()
        d = os.path.join(self.root, 'usr/share/hearth/decky-hearth-link')
        os.makedirs(os.path.join(d, 'dist'))
        open(os.path.join(d, 'plugin.json'), 'w').write('{"name": "Hearth Link", "author": "Dragongamer9"}')
        open(os.path.join(d, 'dist', 'index.js'), 'w').write('// built')
        r = self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
        self.assertEqual(r['others'], {'decky-hearth-link': True})
        link = os.path.join(self.home, 'homebrew/plugins/decky-hearth-link')
        self.assertTrue(os.path.isfile(os.path.join(link, 'dist/index.js')))
        self.assertNotIn('decky-hearth-link', self.m.start_stop('start'), 'the same plugin: left as it is')
        open(os.path.join(d, 'dist', 'index.js'), 'w').write('// new version')
        self.assertEqual(self.m.start_stop('start')['decky-hearth-link'], 'updated')
        self.assertEqual(open(os.path.join(link, 'dist/index.js')).read(), '// new version')
        self.m.remove(self.user)
        self.assertFalse(os.path.exists(link))

    def test_an_image_without_hearth_link_is_fine(self):
        self.ship_plugin()
        r = self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
        self.assertEqual(r['others'], {})
        self.assertNotIn('decky-hearth-link', self.m.start_stop('start'))

    def old_plugin(self, meta='{"name": "Hearth", "author": "Dragongamer9"}', folder='decky-hearth'):
        d = os.path.join(self.home, 'homebrew/plugins', folder)
        os.makedirs(os.path.join(d, 'dist'))
        open(os.path.join(d, 'plugin.json'), 'w').write(meta)
        return d

    def test_hearth_sync_replaces_the_old_hearth_plugin_hearth_put_there(self):
        self.ship_plugin()
        old = self.old_plugin()
        r = self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
        self.assertTrue(r['plugin'])
        self.assertTrue(r['oldPluginRemoved'])
        self.assertFalse(os.path.exists(old))
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/dist/index.js')))

    def test_start_takes_the_old_plugin_away_and_adds_hearth_sync(self):
        # An install from before Hearth Sync: Decky with the old Hearth plugin only.
        self.ship_plugin()
        os.makedirs(os.path.join(self.root, 'etc/systemd/system'))
        open(os.path.join(self.root, 'etc/systemd/system/hearth-decky.service'), 'w').write(
            self.m.unit_text(self.home, 'x', '/var/lib/hearth-decky/PluginLoader'))
        old = self.old_plugin()
        out = self.m.start_stop('start')
        self.assertEqual(out['plugin'], 'updated')
        self.assertEqual(out['oldPlugin'], 'removed')
        self.assertFalse(os.path.exists(old))
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/plugin.json')))
        self.assertNotIn('oldPlugin', self.m.start_stop('start'), 'nothing left to remove')

    def test_a_plugin_that_isnt_hearths_is_never_removed(self):
        self.ship_plugin()
        for meta in ('{"name": "Some Store Plugin", "author": "someone"}', '{"name": "Hearth", "author": "someone else"}', 'not json'):
            d = self.old_plugin(meta)
            self.assertFalse(self.m.old_plugin_ours(self.home))
            self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
            self.assertTrue(os.path.isdir(d), meta)
            self.m.remove(self.user)
            self.assertTrue(os.path.isdir(d), meta)
            shutil.rmtree(d)
        # A link named like it (pointing at someone's own folder) is left alone too.
        real = self.old_plugin(folder='mine')
        os.symlink(real, os.path.join(self.home, 'homebrew/plugins/decky-hearth'))
        self.assertFalse(self.m.old_plugin_ours(self.home))
        self.assertFalse(self.m.remove_old_plugin(self.home))
        self.assertTrue(os.path.isdir(real))

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

    def build_in(self):
        d = os.path.join(self.root, 'usr/lib/hearth-decky')
        os.makedirs(d)
        open(os.path.join(d, 'PluginLoader'), 'wb').write(b'\x7fELF' + b'\0' * 64)
        os.chmod(os.path.join(d, 'PluginLoader'), 0o755)
        open(os.path.join(d, 'version'), 'w').write('v3.3.1\n')
        u = os.path.join(self.root, 'usr/lib/systemd/system')
        os.makedirs(u)
        shutil.copy(DECKY_TEMPLATE, u)

    def test_builtin_prepare_makes_the_user_side_and_hearth_sync(self):
        self.ship_plugin()
        self.build_in()
        self.assertTrue(self.m.builtin())
        self.assertEqual(self.m.builtin_version(), 'v3.3.1')
        old = self.old_plugin()
        out = self.m.prepare(self.user)
        self.assertEqual(out['plugin'], 'updated')
        self.assertEqual(out['oldPlugin'], 'removed')
        self.assertFalse(os.path.exists(old))
        self.assertTrue(os.path.isfile(os.path.join(self.home, '.steam/steam/.cef-enable-remote-debugging')))
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/plugin.json')))
        self.assertNotIn('plugin', self.m.prepare(self.user), 'the same plugin: left as it is')
        env = self.m.decky_env(self.home, 'x', base={})
        self.assertEqual(env['UNPRIVILEGED_USER'], 'x')
        self.assertEqual(env['PRIVILEGED_PATH'], os.path.join(self.home, 'homebrew'))
        self.assertEqual(env['KEEP_SYSTEMD_SERVICE'], '1')

    def test_builtin_migrates_an_install_from_before(self):
        self.ship_plugin()
        self.m.install(self.user, download=lambda: ('v3.1.0', lambda: b'\x7fELF' + b'\0' * (2 * 1024 * 1024)))
        old_unit = os.path.join(self.root, 'etc/systemd/system/hearth-decky.service')
        self.assertTrue(os.path.isfile(old_unit))
        self.build_in()
        out = self.m.prepare(self.user)
        self.assertTrue(out['migrated'])
        self.assertFalse(os.path.exists(old_unit))
        self.assertFalse(os.path.exists(os.path.join(self.root, 'var/lib/hearth-decky')))
        self.assertTrue(os.path.isfile(os.path.join(self.home, 'homebrew/plugins/decky-hearth-sync/plugin.json')))
        # A hearth-decky.service someone wrote themselves is never removed.
        open(old_unit, 'w').write('[Service]\nExecStart=/opt/mine\n')
        self.assertNotIn('migrated', self.m.prepare(self.user))
        self.assertTrue(os.path.isfile(old_unit))

    def test_builtin_install_downloads_nothing_and_remove_is_refused(self):
        self.ship_plugin()
        self.build_in()
        r = self.m.install(self.user, download=lambda: self.fail('must not download'))
        self.assertEqual(r['via'], 'builtin')
        self.assertEqual(r['version'], 'v3.3.1')
        self.assertTrue(r['plugin'])
        with self.assertRaises(ValueError):
            self.m.remove(self.user)

    def test_builtin_doesnt_start_next_to_deckys_own(self):
        self.build_in()
        up = os.path.join(self.root, 'etc/systemd/system/plugin_loader.service')
        os.makedirs(os.path.dirname(up))
        open(up, 'w').write('[Service]\n')
        with self.assertRaises(ValueError):
            self.m.prepare(self.user)
        t = DECKY_TEMPLATE.read_text()
        self.assertIn('ConditionPathExists=!/etc/systemd/system/plugin_loader.service', t)
        self.assertIn('User=root', t)
        self.assertIn('ExecStart=/usr/bin/python3 /usr/libexec/nimbus/hearth-decky run %i', t)
        self.assertNotIn('[Install]', t)

    def test_builtin_polkit_rule_is_only_your_own_instance(self):
        r = DECKY_RULES.read_text()
        self.assertIn('"org.freedesktop.systemd1.manage-units"', r)
        self.assertIn('action.lookup("unit") == "hearth-decky@" + subject.user + ".service"', r)
        self.assertIn("subject.local && subject.active)", r)
        self.assertNotIn("isInGroup", r, "every account gets Decky (the owner's choice)")
        self.assertIn('["start", "stop", "restart"]', r)

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
