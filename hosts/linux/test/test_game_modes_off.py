"""Game Mode on this PC (Settings › System › Startup; the owner, Nov 19: "hearth game mode cant be an app that Opens,
needs to just be game mode. Also need to add a option that turns off the game modes").

desktop.json's gameModeOff (missing: On). Off: the PC starts in Desktop Mode, nimbus-mode's game / steam / hearth /
hearth-page do nothing but say so, Plasma's Back to Game Mode (menu, taskbar, Super+G), Mantel's Game Mode buttons and
the Guide menu's Game Mode are hidden, and every fallback of the session can still open Game Mode (recovery). Settings'
links to Hearth's own pages open Hearth Game Mode at that page, never Hearth in a window.
"""
import json
import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SESSION_DIR = ROOT / 'hosts' / 'linux' / 'session'
MODE = SESSION_DIR / 'nimbus-mode'
SESSION = (SESSION_DIR / 'nimbus-session').read_text(encoding='utf-8')
SETTINGS = ROOT / 'hosts' / 'hearth-ui' / 'apps' / 'settings' / 'src'
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'


def block(text, name):
    return text.split('# >>> %s\n' % name, 1)[1].split('# <<< %s\n' % name, 1)[0]


class Env:
    """A runtime folder, a home with desktop.json, and nimbus-mode run in them."""

    def __init__(self, off=None):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)
        self.run = self.dir / 'run'
        self.run.mkdir()
        self.home = self.dir / 'home'
        (self.home / '.local' / 'share' / 'nimbus').mkdir(parents=True)
        self.json = self.home / '.local' / 'share' / 'nimbus' / 'desktop.json'
        if off is not None:
            self.json.write_text(json.dumps({'look': 'hearth', 'gameModeOff': off}, indent=2))
        self.env = dict(PATH='/usr/bin:/bin', HOME=str(self.home), XDG_RUNTIME_DIR=str(self.run), USER='tester',
                        HEARTH_LIVE_FLAG=str(self.dir / 'no-live'), XDG_STATE_HOME=str(self.dir / 'state'))

    def mode(self, *args, **extra):
        env = dict(self.env, **extra)
        return subprocess.run(['sh', str(MODE)] + list(args), env=env, capture_output=True, text=True, timeout=30)

    def next(self):
        p = self.run / 'nimbus-next'
        return p.read_text().strip() if p.exists() else None

    def close(self):
        self.tmp.cleanup()


class NimbusMode(unittest.TestCase):
    def test_status_follows_the_setting_and_missing_is_on(self):
        for off, want in ((None, 'on'), (False, 'on'), (True, 'off')):
            e = Env(off)
            try:
                self.assertEqual(e.mode('gamemodes', 'status').stdout.strip(), want, off)
            finally:
                e.close()

    def test_live_usb_and_extra_players_always_have_game_mode(self):
        e = Env(True)
        try:
            (e.dir / 'live').write_text('')
            self.assertEqual(e.mode('gamemodes', 'status', HEARTH_LIVE_FLAG=str(e.dir / 'live')).stdout.strip(), 'on')
            self.assertEqual(e.mode('gamemodes', 'status', HEARTH_SEAT='seat1').stdout.strip(), 'on')
        finally:
            e.close()

    def test_off_refuses_every_game_mode_and_writes_nothing(self):
        e = Env(True)
        try:
            for m in ('game', 'steam', 'hearth'):
                r = e.mode(m)
                self.assertEqual(r.returncode, 3, (m, r.stderr))
                self.assertIn('Game Mode is off on this PC', r.stderr)
                self.assertIsNone(e.next(), m)
            r = e.mode('hearth-page', 'sources')
            self.assertEqual(r.returncode, 3)
            self.assertFalse((e.run / 'hearth-start-page').exists())
            self.assertIsNone(e.next())
            # The setting itself is never touched by nimbus-mode.
            self.assertTrue(json.loads(e.json.read_text())['gameModeOff'])
        finally:
            e.close()

    def test_recovery_still_switches_with_game_mode_off(self):
        e = Env(True)
        p = subprocess.Popen(['sleep', '30'])
        try:
            (e.run / 'hearth-plasma.pid').write_text(str(p.pid))
            r = e.mode('game', HEARTH_GM_RECOVERY='1')
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual(e.next(), 'game')
        finally:
            if p.poll() is None:
                p.kill()
            p.wait()
            e.close()

    def test_on_switches_as_before(self):
        e = Env(False)
        p = subprocess.Popen(['sleep', '30'])
        try:
            (e.run / 'hearth-plasma.pid').write_text(str(p.pid))
            self.assertEqual(e.mode('game').returncode, 0)
            self.assertEqual(e.next(), 'game')
        finally:
            if p.poll() is None:
                p.kill()
            p.wait()
            e.close()

    def test_hearth_page_is_hearth_game_mode_at_that_page(self):
        e = Env()
        p = subprocess.Popen(['sleep', '30'])
        try:
            (e.run / 'hearth-plasma.pid').write_text(str(p.pid))
            self.assertEqual(e.mode('hearth-page', 'rm -rf').returncode, 2)
            self.assertEqual(e.mode('hearth-page', 'nope').returncode, 2)
            self.assertIsNone(e.next())
            r = e.mode('hearth-page', 'sources')
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertEqual((e.run / 'hearth-start-page').read_text().strip(), 'sources')
            self.assertEqual(e.next(), 'hearth', 'a mode, never a window')
        finally:
            if p.poll() is None:
                p.kill()
            p.wait()
            e.close()

    def test_apply_hides_and_shows_plasmas_back_to_game_mode(self):
        e = Env(True)
        try:
            self.assertEqual(e.mode('gamemodes', 'apply').stdout.strip(), 'off')
            hidden = e.run / 'hearth-gm-hidden' / 'applications' / 'hearth-gamemode.desktop'
            self.assertIn('Hidden=true', hidden.read_text())
            e.json.write_text(json.dumps({'gameModeOff': False}))
            self.assertEqual(e.mode('gamemodes', 'apply').stdout.strip(), 'on')
            self.assertFalse(hidden.exists(), 'turned back on: the entries come back')
            self.assertTrue(hidden.parent.is_dir(), 'the folder stays, so Plasma sees the next change')
        finally:
            e.close()

    def test_taskbar_pin_comes_back_only_if_it_was_there(self):
        text = MODE.read_text()
        pin = text.split('gm_pin() {', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('print("had")', pin)
        self.assertIn('[ -e "$GM_PINNED" ]', pin, 'put back only when Hearth took it off')
        self.assertIn('timeout 10', pin)


class Session(unittest.TestCase):
    def sh(self, script, off=None, setup=False):
        e = Env(off)
        try:
            prog = 'log() { :; }\nRUN=%s\n' % e.run
            prog += 'setup_pending() { %s; }\n' % ('return 0' if setup else 'return 1')
            prog += block(SESSION, 'game modes off') + script
            return subprocess.run(['sh', '-c', prog], env=e.env, capture_output=True, text=True, timeout=30).stdout.split()
        finally:
            e.close()

    def test_off_starts_in_desktop_mode(self):
        self.assertEqual(self.sh('gm_start_mode game', off=True), ['desktop'])
        self.assertEqual(self.sh('gm_start_mode game', off=False), ['game'])
        self.assertEqual(self.sh('gm_start_mode game'), ['game'], 'missing: On, nothing changes')
        self.assertEqual(self.sh('gm_start_mode server', off=True), ['server'], 'Server mode is not a Game Mode')
        self.assertEqual(self.sh('gm_start_mode desktop', off=True), ['desktop'])

    def test_first_time_setup_still_runs(self):
        self.assertEqual(self.sh('gm_start_mode game', off=True, setup=True), ['game'])
        self.assertEqual(self.sh('gm_after_setup ""', off=True), ['desktop'])
        self.assertEqual(self.sh('gm_after_setup ""', off=False), ['steam'], 'as before')
        self.assertEqual(self.sh('gm_after_setup steam', off=True), ['desktop'])
        self.assertEqual(self.sh('gm_after_setup desktop', off=True), ['desktop'])
        self.assertEqual(self.sh('gm_after_setup logout', off=True), ['logout'])

    def test_hidden_entry_made_at_plasma_start(self):
        out = self.sh('gm_hide_prepare; ls "$GM_HIDE_DIR/applications"', off=True)
        self.assertEqual(out, ['hearth-gamemode.desktop'])
        self.assertEqual(self.sh('gm_hide_prepare; ls "$GM_HIDE_DIR/applications"', off=False), [])

    def test_where_the_session_reads_it(self):
        main = SESSION.split('QUICK_ENDS=0\nreset_controllers\n', 1)[1]
        i_server = main.index('[ "$mode" = server ] && mode="$(server_start_mode)"')
        i_gm = main.index('[ -z "$HEARTH_SEAT" ] && mode="$(gm_start_mode "$mode")"')
        i_resolve = main.index('mode="$(resolve_mode "$mode")"')
        self.assertLess(i_server, i_gm)
        self.assertLess(i_gm, i_resolve)
        self.assertIn('gm_after_setup "$next"', main)
        plasma = SESSION.split('run_plasma() {', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('gm_hide_prepare; dd="$GM_HIDE_DIR:$dd"', plasma, "the hidden copy comes first in Plasma's menus")

    def test_recovery_fallbacks_are_never_gated(self):
        # Desktop Mode that can't start still gets a working screen from Game Mode.
        plasma = SESSION.split('run_plasma() {', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('echo steam >"$NEXT"', plasma)
        self.assertIn('log "last resort: Hearth\'s Game Mode"; mode=game', SESSION)
        resolve = SESSION.split('resolve_mode() {', 1)[1].split('\n}\n', 1)[0]
        self.assertNotIn('game_modes_off', resolve, 'a fallback asking for Game Mode always gets it')
        auto = (ROOT / 'hosts' / 'linux' / 'desktop' / 'labwc' / 'autostart').read_text()
        self.assertIn('HEARTH_GM_RECOVERY=1 nimbus-mode game', auto, "Mantel's give_up is a recovery")
        self.assertNotRegex(auto, r'(?m)^\s*nimbus-mode game\s*$')

    def test_start_page_reaches_hearths_first_start_only(self):
        game = SESSION.split('run_game() {', 1)[1].split('\n}\n', 1)[0]
        self.assertIn('rm -f "$RUN/hearth-start-page"', game)
        self.assertIn("tr -cd 'a-z'", game)
        wd = (SESSION_DIR / 'nimbus-watchdog').read_text()
        self.assertIn('${HEARTH_START_PAGE:+--page "$HEARTH_START_PAGE"}', wd)
        self.assertIn('pid=$!\n  HEARTH_START_PAGE=""', wd)
        # nimbus-mode's page list is hearth-core's start_page list.
        core = (ROOT / 'hosts' / 'hearth-core' / 'src' / 'bridge.rs').read_text()
        pages = re.search(r'const PAGES: &\[&str\] = &\[(.*?)\];', core, re.S).group(1)
        core_pages = set(re.findall(r'"([a-z]+)"', pages))
        mode = MODE.read_text().split('  hearth-page)\n', 1)[1].split('case "$2" in\n', 1)[1].split(') ;;', 1)[0]
        self.assertEqual(set(mode.strip().split('|')), core_pages)


class EntriesHidden(unittest.TestCase):
    def test_settings_rust(self):
        sysrs = (SETTINGS / 'cats' / 'system.rs').read_text()
        self.assertIn('"Game Mode on this PC"', sysrs)
        self.assertIn('serde_json::json!({"gameModeOff": true})', sysrs)
        self.assertIn('Game Mode stays available for recovery', sysrs)
        self.assertIn('"nimbus-mode", "gamemodes", "apply"', sysrs)
        home = (SETTINGS / 'cats' / 'home.rs').read_text()
        self.assertIn('"Game Mode: off"', home)

    def test_guide_menu(self):
        pad = SETTINGS.parent.parent / 'pad' / 'src'
        main = (pad / 'main.rs').read_text()
        view = (pad / 'view.rs').read_text()
        self.assertIn('"game" if gm_hidden("game")', main)
        self.assertIn('GM_OFF.store(game_modes_off()', main)
        self.assertEqual(view.count('if gm_hidden('), 3, 'its disc, its icon and its label')

    def test_mantel(self):
        st = (SHELL / 'Settings.qml').read_text()
        self.assertIn('property bool gameModeOff: false', st)
        self.assertIn('onGameModeOffChanged: save("gameModeOff")', st)
        self.assertIn('"Game Mode on this PC"', (SHELL / 'SetStartup.qml').read_text())
        for f in ('Bar.qml', 'Islands.qml', 'PanelBlocks.qml', 'Launcher.qml'):   # a panel's blocks: PanelBlocks.qml
            self.assertIn('visible: !Settings.gameModeOff', (SHELL / f).read_text(), f)
        self.assertIn('Settings.gameModeOff ? [] :', (SHELL / 'PowerMenu.qml').read_text())
        self.assertIn('Settings.gameModeOff ? [] :', (SHELL / 'WinTools.qml').read_text())
        self.assertIn('e.id === "game-mode" && Settings.gameModeOff', (SHELL / 'DesktopMenu.qml').read_text())


class NeverAWindow(unittest.TestCase):
    def test_settings_never_opens_hearth_in_a_window(self):
        for f in SETTINGS.rglob('*.rs'):
            text = f.read_text()
            for m in re.finditer(r'"nimbus", "--windowed", "--page", "([a-z]+)"', text):
                # Only the live USB's installer: there Hearth's app is the installer (its own Game Mode).
                self.assertEqual(m.group(1), 'install', '%s opens Hearth in a window' % f.name)
            self.assertNotIn("Opens in the Hearth app's window", text, f.name)
        kit = (SETTINGS / 'cats' / 'apps' / 'kit.rs').read_text()
        self.assertIn('&["nimbus-mode", "hearth-page", page]', kit)

    def test_hearth_itself_stays_out_of_the_menus(self):
        for b in ('anatase', 'peppermint'):
            build = (ROOT / 'os-bases' / b / 'build.sh').read_text()
            entry = build.split('cat > /usr/share/applications/nimbus.desktop <<DESKTOP\n', 1)[1].split('\nDESKTOP\n', 1)[0]
            self.assertIn('NoDisplay=true', entry, b)


if __name__ == '__main__':
    unittest.main()
