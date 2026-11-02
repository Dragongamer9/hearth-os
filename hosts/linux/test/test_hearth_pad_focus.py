"""Controller apps own the controller (docs/CONTROLLER-FOCUS.md): which window is in front → what hearth-pad does.
Run: python3 -m unittest discover -s test -p 'test_*.py'"""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import socket
import tempfile
import unittest

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'
loader = importlib.machinery.SourceFileLoader('hearth_pad_focus', str(SHELL / 'hearth-pad'))
spec = importlib.util.spec_from_loader('hearth_pad_focus', loader)
hp = importlib.util.module_from_spec(spec)
loader.exec_module(hp)


class Out:
    def __init__(self):
        self.log, self.sent, self.moved, self.wheeled = [], [], [0, 0], [0, 0]

    def move(self, dx, dy):
        self.moved[0] += dx
        self.moved[1] += dy

    def wheel(self, v, h):
        self.wheeled[0] += v
        self.wheeled[1] += h

    def press(self, name, down):
        self.log.append((name, down))

    def send(self, obj):
        self.sent.append(obj)

    def rumble(self, s, ms):
        pass


def press(pad, name, t=0.0, hold=0.05):
    pad.button(name, True, t)
    pad.button(name, False, t + hold)


class Base(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.old_log = hp.PAD_LOG
        hp.PAD_LOG = os.path.join(self.tmp.name, 'pad.log')
        self.addCleanup(setattr, hp, 'PAD_LOG', self.old_log)
        apps = os.path.join(self.tmp.name, 'controller-apps.json')
        with open(apps, 'w') as f:
            json.dump({'apps': [{'id': 'hearth-mods', 'name': 'Mods'}, {'id': 'io.github.dragongamer9.hearth.logs'}]}, f)
        self.appdir = os.path.join(self.tmp.name, 'applications')
        os.makedirs(self.appdir)
        self.owners = hp.Owners(apps, dirs=[self.appdir])

    def make(self, **kw):
        out = Out()
        return hp.Pad(out, owners=self.owners, **kw), out

    def padlog(self):
        try:
            with open(hp.PAD_LOG) as f:
                return f.read()
        except OSError:
            return ''


class OwnersTest(Base):
    def test_listed_ids_any_case_and_desktop_suffix(self):
        self.assertTrue(self.owners.owns('hearth-mods'))
        self.assertTrue(self.owners.owns('Hearth-Mods.desktop'))
        self.assertTrue(self.owners.owns('io.github.dragongamer9.hearth.logs'))
        self.assertFalse(self.owners.owns('foot'))
        self.assertFalse(self.owners.owns(''))

    def test_desktop_key(self):
        with open(os.path.join(self.appdir, 'org.example.Game.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nName=Game\nStartupWMClass=game-window\nX-Hearth-Controller=true\n'
                    '[Desktop Action new]\nX-Hearth-Controller=false\n')
        with open(os.path.join(self.appdir, 'plain.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nName=Plain\nStartupWMClass=plain\n')
        with open(os.path.join(self.appdir, 'off.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nName=Off\nX-Hearth-Controller=false\n')
        self.assertTrue(self.owners.owns('org.example.Game', now=100.0))
        self.assertTrue(self.owners.owns('game-window', now=100.0))
        self.assertFalse(self.owners.owns('plain', now=100.0))
        self.assertFalse(self.owners.owns('off', now=100.0))

    def test_desktop_rescan_is_rate_limited(self):
        self.assertFalse(self.owners.owns('late-app', now=10.0))
        with open(os.path.join(self.appdir, 'late-app.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nName=Late\nX-Hearth-Controller=true\n')
        self.assertFalse(self.owners.owns('late-app', now=11.0))          # not looked again yet
        self.assertTrue(self.owners.owns('late-app', now=10.0 + hp.DESKTOP_RESCAN + 1))

    def test_claims_last_while_held(self):
        self.owners.claim('my-qml-app', 'a')
        self.owners.claim('my-qml-app', 'b')
        self.assertTrue(self.owners.owns('my-qml-app'))
        self.owners.drop('a')
        self.assertTrue(self.owners.owns('my-qml-app'))
        self.owners.unclaim('my-qml-app', 'b')
        self.assertFalse(self.owners.owns('my-qml-app'))

    def test_shipped_list_is_valid(self):
        data = json.loads((SHELL / 'controller-apps.json').read_text())
        ids = [hp.app_key(a['id']) for a in data['apps']]
        self.assertEqual(len(ids), len(set(ids)), 'an app id is listed twice')
        for must in ('hearth-mods', 'hearth-saves', 'hearth-controller-lab', 'hearth-photos', 'hearth-player'):
            self.assertIn(must, ids)
        for a in data['apps']:
            self.assertTrue(a.get('name'), a)

    def test_missing_list_still_works(self):
        o = hp.Owners(os.path.join(self.tmp.name, 'nope.json'), dirs=[self.appdir])
        self.assertFalse(o.owns('hearth-mods'))
        self.assertIn('unreadable', self.padlog())


class FocusModeTest(Base):
    """focus {app} → the helper's mode."""

    def test_controller_app_in_front_gets_clean_input(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods', 'title': 'Mods'})
        self.assertEqual(pad.owner, 'hearth-mods')
        self.assertIn({'t': 'owner', 'app': 'hearth-mods', 'mouse': False}, out.sent)
        for b in ('a', 'b', 'x', 'y', 'rt', 'lt', 'l3', 'up', 'down', 'menu', 'view', 'lb', 'rb'):
            press(pad, b)
        pad.axes(0.9, 0.9, 1.0, 1.0)
        pad.tick(1.0, 0.05)
        self.assertEqual(out.log, [], 'no clicks or keys (no Alt+Tab either) while the app has the controller')
        self.assertEqual(out.moved, [0, 0])
        self.assertEqual(out.wheeled, [0, 0])
        self.assertFalse([m for m in out.sent if m.get('t') in ('launcher', 'help', 'osk')])
        self.assertIn("'hearth-mods' in front reads the controller itself", self.padlog())

    def test_other_window_gets_the_desktop_controls_back(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.command({'cmd': 'focus', 'app': 'foot'})
        self.assertEqual(pad.owner, '')
        press(pad, 'a')
        self.assertEqual(out.log, [('BTN_LEFT', True), ('BTN_LEFT', False)])
        pad.axes(0, 0, 1.0, 0)
        pad.tick(1.0, 0.05)
        self.assertGreater(out.moved[0], 0)
        self.assertIn("'foot' in front: desktop controls on", self.padlog())

    def test_switch_releases_what_was_held(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'foot'})
        pad.button('a', True, 0.0)            # dragging on the desktop...
        pad.button('down', True, 0.0)
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})   # ...a controller app comes up
        self.assertIn(('BTN_LEFT', False), out.log)
        self.assertIn(('KEY_DOWN', False), out.log)

    def test_guide_still_opens_the_hearth_menu_and_holds_the_controller(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        self.assertFalse(pad.wants_grab, 'never held while the app reads it')
        pad.button('guide', True, 0.0)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': True, 'sel': -1})
        self.assertTrue(pad.wants_grab, 'held while the Hearth menu is open: the app must not get those presses')
        pad.button('guide', False, 0.1)
        press(pad, 'right', 0.2)
        press(pad, 'a', 0.4)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': False, 'pick': 2})
        self.assertFalse(pad.wants_grab, 'let go once it closed')
        self.assertEqual(out.log, [])

    def test_guide_x_keyboard_holds_the_controller(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.button('guide', True, 0.0)
        press(pad, 'x', 0.05)
        pad.button('guide', False, 0.2)
        self.assertIn({'t': 'osk', 'open': True}, out.sent)
        pad.command({'cmd': 'osk', 'open': True})
        self.assertTrue(pad.wants_grab)
        press(pad, 'a', 0.5)                  # types on Hearth's keyboard, not in the app
        self.assertIn({'t': 'oskpad', 'b': 'a', 'down': True}, out.sent)
        pad.command({'cmd': 'osk', 'open': False})
        self.assertFalse(pad.wants_grab)

    def test_y_does_not_open_mantels_keyboard_in_a_controller_app(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        press(pad, 'y')
        self.assertNotIn({'t': 'osk', 'open': True}, out.sent)

    def test_guide_r3_gives_the_pointer_back_in_that_app(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.button('guide', True, 0.0)
        press(pad, 'r3', 0.05)
        pad.button('guide', False, 0.2)
        self.assertTrue(pad.owner_mouse)
        self.assertEqual(pad.mode, 'desk', 'the Hearth menu Guide opened is closed again')
        self.assertIn({'t': 'owner', 'app': 'hearth-mods', 'mouse': True}, out.sent)
        self.assertTrue(pad.wants_grab, 'the app does not also act on the pointer\'s presses')
        press(pad, 'a', 0.5)
        self.assertEqual(out.log[-2:], [('BTN_LEFT', True), ('BTN_LEFT', False)])
        # Again: back to the app.
        pad.button('guide', True, 1.0)
        press(pad, 'r3', 1.05)
        pad.button('guide', False, 1.2)
        self.assertFalse(pad.owner_mouse)
        self.assertFalse(pad.wants_grab)
        n = len(out.log)
        press(pad, 'a', 1.5)
        self.assertEqual(len(out.log), n)

    def test_pointer_override_ends_with_the_window(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.command({'cmd': 'padmouse'})
        self.assertTrue(pad.owner_mouse)
        pad.command({'cmd': 'focus', 'app': 'foot'})
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        self.assertFalse(pad.owner_mouse)

    def test_guide_r3_without_a_controller_app_does_nothing_special(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'foot'})
        pad.button('guide', True, 0.0)
        press(pad, 'r3', 0.05)
        self.assertFalse(pad.owner_mouse)
        self.assertNotIn('toast', [m.get('t') for m in out.sent])

    def test_hearth_menus_still_get_keys_over_a_controller_app(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.command({'cmd': 'keys', 'on': True})     # Quick Access / the launcher opened over it
        press(pad, 'a')
        self.assertIn(('KEY_ENTER', True), out.log)
        pad.command({'cmd': 'keys', 'on': False})    # closed: the app has it again
        n = len(out.log)
        press(pad, 'a')
        self.assertEqual(len(out.log), n)

    def test_claim_over_the_socket_and_its_end(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'my-qml-app'})
        self.assertEqual(pad.owner, '')
        pad.command({'cmd': 'claim', 'app': 'my-qml-app', 'who': 7})
        self.assertEqual(pad.owner, 'my-qml-app')
        pad.command({'cmd': 'release', 'app': 'my-qml-app', 'who': 7})
        self.assertEqual(pad.owner, '')

    def test_other_modes_never_stand_aside(self):
        out = Out()
        game = hp.Pad(out, game=True, owners=self.owners)
        game.command({'cmd': 'focus', 'app': 'hearth-mods'})
        self.assertEqual(game.owner, '')
        plain = Out()
        pad = hp.Pad(plain)                   # no Owners: Plasma / GNOME / COSMIC / standalone
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        self.assertEqual(pad.owner, '')
        press(pad, 'a')
        self.assertEqual(plain.log, [('BTN_LEFT', True), ('BTN_LEFT', False)])

    def test_fullscreen_pause_and_owner_together(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'hearth-mods'})
        pad.command({'cmd': 'pause', 'on': True})
        pad.command({'cmd': 'pause', 'on': False})
        press(pad, 'a')
        self.assertEqual(out.log, [])


class SocketTest(Base):
    def test_claim_status_and_drop_on_close(self):
        pad, out = self.make()
        pad.command({'cmd': 'focus', 'app': 'my-qml-app'})
        path = os.path.join(self.tmp.name, 'run', 'hearth', 'pad.sock')
        link = hp.OwnerLink(path)
        self.addCleanup(link.srv.close)
        self.assertEqual(os.stat(path).st_mode & 0o777, 0o600)
        self.assertEqual(os.stat(os.path.dirname(path)).st_mode & 0o777, 0o700)
        c = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        c.connect(path)
        link.read(link.srv.fileno(), pad)     # accept
        (fd,) = list(link.cli)
        c.sendall(b'{"claim": "my-qml-app"}\n')
        link.read(fd, pad)
        reply = json.loads(c.recv(4096).decode())
        self.assertEqual(reply['owner'], 'my-qml-app')
        self.assertEqual(pad.owner, 'my-qml-app')
        c.close()
        link.read(fd, pad)                    # EOF: the claim goes with the connection
        self.assertEqual(pad.owner, '')
        self.assertEqual(link.cli, {})


class SimTest(Base):
    def test_sim_output_and_controller(self):
        out = hp.SimOutput(self.tmp.name)
        out.press('BTN_LEFT', True)
        out.move(3, -2)
        with open(os.path.join(self.tmp.name, 'out.log')) as f:
            self.assertEqual(f.read(), 'press BTN_LEFT 1\nmove 3 -2\n')
        sim = hp.SimController(self.tmp.name)
        with open(sim.path, 'a') as f:
            f.write('{"b": "a", "down": true}\n{"axes": [0, 0, 1, 0]}\nnot json\n')
        self.assertEqual(sim.poll(), 2)
        self.assertTrue(sim.btn['a'])
        self.assertEqual(sim.ax['rx'], 1.0)


class AppsTest(unittest.TestCase):
    """Every Hearth app that reads the controller uses the shared reader (ui/pad/hearth-gamepad.js) and is known as a
    controller app."""
    ROOT = SHELL.parent.parent.parent.parent

    def test_pad_sync_check(self):
        import subprocess
        r = subprocess.run([sys.executable, str(self.ROOT / 'tools' / 'pad-sync.py'), '--check'], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_shipped_desktop_files_say_so(self):
        for d in ('hosts/hearth-tools/desktop', 'os/system_files/usr/share/applications'):
            for f in sorted((self.ROOT / d).glob('*.desktop')):
                if f.name in ('hearth-player.desktop', 'hearth-calculator.desktop') or d.startswith('hosts/hearth-tools'):
                    self.assertTrue(hp.desktop_controller_ids(str(f)), f)

    def test_every_synced_app_is_listed(self):
        spec2 = importlib.util.spec_from_file_location('pad_sync', str(self.ROOT / 'tools' / 'pad-sync.py'))
        ps = importlib.util.module_from_spec(spec2)
        spec2.loader.exec_module(ps)
        listed = {hp.app_key(a['id']) for a in json.loads((SHELL / 'controller-apps.json').read_text())['apps']}
        for dest in ps.TARGETS:
            app = dest.split('/')[1]          # hosts/hearth-photos/ui → hearth-photos
            if app in ('hearth-gaming', 'hearth-tools', 'hearth-time'):
                continue                      # one program, several windows (listed by window)
            self.assertIn(app, listed, dest)


if __name__ == '__main__':
    unittest.main()
