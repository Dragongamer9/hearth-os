"""Tests for desktop/shell/hearth-pad (controller → desktop). Run: python3 -m unittest discover -s test -p 'test_*.py'"""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import json
import importlib.util
import pathlib
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-pad'
loader = importlib.machinery.SourceFileLoader('hearth_pad', str(PATH))
spec = importlib.util.spec_from_loader('hearth_pad', loader)
hp = importlib.util.module_from_spec(spec)
loader.exec_module(hp)


class Out:
    def __init__(self):
        self.log = []
        self.sent = []
        self.moved = [0, 0]
        self.wheeled = [0, 0]
        self.rumbled = []

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

    def rumble(self, strength, ms):
        self.rumbled.append((strength, ms))


def make():
    out = Out()
    return hp.Pad(out), out


def press(pad, name, t=0.0, hold=0.05):
    pad.button(name, True, t)
    pad.button(name, False, t + hold)


class GameMode(unittest.TestCase):
    """--game: Game Mode's controller mouse (Quick Access › Controller mouse)."""

    def make(self):
        out = Out()
        return hp.Pad(out, game=True), out

    def test_only_the_mouse(self):
        pad, out = self.make()
        press(pad, 'a')
        press(pad, 'lt')
        press(pad, 'b')
        self.assertEqual(out.log, [('BTN_LEFT', True), ('BTN_LEFT', False), ('BTN_RIGHT', True), ('BTN_RIGHT', False),
                                   ('KEY_ESC', True), ('KEY_ESC', False)])
        for name in ('guide', 'menu', 'y', 'lb', 'rb', 'view'):
            press(pad, name)
        self.assertEqual(len(out.log), 6, 'Guide, Menu, Y, LB/RB and View are left to the game and Hearth')
        self.assertEqual(pad.mode, 'desk')
        self.assertEqual(out.sent, [])
        pad.axes(0, 0, 1.0, 0)
        pad.tick(1.0, 0.05)
        self.assertGreater(out.moved[0], 0)

    def test_paused_while_hearth_is_in_front(self):
        pad, out = self.make()
        pad.command({'cmd': 'pause', 'on': True})
        press(pad, 'a')
        pad.axes(0, 0, 1.0, 0)
        pad.tick(1.0, 0.05)
        self.assertEqual((out.log, out.moved), ([], [0, 0]))

    def test_view_and_menu_turn_it_off(self):
        pad, out = self.make()
        pad.button('view', True, 0)
        pad.button('menu', True, 0.05)
        self.assertTrue(pad.done)
        self.assertEqual(out.sent, [{'t': 'mouse', 'on': False}])


class Desktop(unittest.TestCase):
    def test_a_clicks_and_drags(self):
        pad, out = make()
        pad.button('a', True, 0)
        pad.button('rt', True, 0.1)     # both held: still one press
        pad.button('a', False, 0.2)
        self.assertEqual(out.log, [('BTN_LEFT', True)])
        pad.button('rt', False, 0.3)
        self.assertEqual(out.log, [('BTN_LEFT', True), ('BTN_LEFT', False)])

    def test_x_right_click_b_escape(self):
        pad, out = make()
        press(pad, 'x')
        press(pad, 'b')
        self.assertEqual(out.log, [('BTN_RIGHT', True), ('BTN_RIGHT', False), ('KEY_ESC', True), ('KEY_ESC', False)])

    def test_right_stick_moves_pointer_left_stick_scrolls(self):
        pad, out = make()
        pad.axes(0, -1, 1, 0)
        for i in range(100):
            pad.tick(i * 0.01, 0.01)
        self.assertGreater(out.moved[0], 1000)
        self.assertEqual(out.moved[1], 0)
        self.assertGreater(out.wheeled[0], 0)       # stick up scrolls up

    def test_pointer_speed_setting(self):
        pad, out = make()
        pad.command({'cmd': 'speed', 'value': 0.5})
        pad.axes(0, 0, 1, 0)
        for i in range(100):
            pad.tick(i * 0.01, 0.01)
        slow = out.moved[0]
        pad2, out2 = make()
        pad2.axes(0, 0, 1, 0)
        for i in range(100):
            pad2.tick(i * 0.01, 0.01)
        self.assertAlmostEqual(slow, out2.moved[0] / 2, delta=3)
        pad.command({'cmd': 'speed', 'value': 'nonsense'})   # ignored
        self.assertEqual(pad.speed, 0.5)

    def test_dead_zone(self):
        pad, out = make()
        pad.axes(0.1, 0.1, 0.1, -0.1)
        for i in range(100):
            pad.tick(i * 0.01, 0.01)
        self.assertEqual(out.moved, [0, 0])
        self.assertEqual(out.wheeled, [0, 0])

    def test_view_dpad_snaps_and_view_tap_shows_help(self):
        pad, out = make()
        pad.button('view', True, 0)
        press(pad, 'left', 0.1)
        pad.button('view', False, 0.2)
        self.assertEqual(out.log, [('KEY_LEFTMETA', True), ('KEY_LEFT', True), ('KEY_LEFT', False), ('KEY_LEFTMETA', False)])
        self.assertEqual(out.sent, [])              # used with the D-pad: no help
        press(pad, 'view', 1.0, 0.1)
        self.assertEqual(out.sent, [{'t': 'help'}])

    def test_view_b_closes_window(self):
        pad, out = make()
        pad.button('view', True, 0)
        press(pad, 'b', 0.1)
        self.assertIn(('KEY_Q', True), out.log)
        self.assertNotIn(('KEY_ESC', True), out.log)

    def test_dpad_holds_arrow_keys(self):
        pad, out = make()
        pad.button('down', True, 0)
        self.assertEqual(out.log, [('KEY_DOWN', True)])
        pad.button('down', False, 1)
        self.assertEqual(out.log, [('KEY_DOWN', True), ('KEY_DOWN', False)])

    def test_bumpers_switch_windows_holding_alt(self):
        pad, out = make()
        press(pad, 'rb', 0)
        press(pad, 'rb', 0.3)
        self.assertEqual(out.log.count(('KEY_LEFTALT', True)), 1)
        self.assertEqual(out.log.count(('KEY_TAB', True)), 2)
        pad.tick(0.5, 0.01)
        self.assertNotIn(('KEY_LEFTALT', False), out.log)
        pad.tick(1.3, 0.01)
        self.assertEqual(out.log[-1], ('KEY_LEFTALT', False))

    def test_menu_opens_launcher(self):
        pad, out = make()
        press(pad, 'menu')
        self.assertEqual(out.sent, [{'t': 'launcher'}])

    def test_paused_by_fullscreen_but_guide_still_works(self):
        pad, out = make()
        pad.button('a', True, 0)
        pad.command({'cmd': 'pause', 'on': True})
        self.assertEqual(out.log, [('BTN_LEFT', True), ('BTN_LEFT', False)])     # released
        press(pad, 'x', 1)
        pad.axes(0, 0, 1, 0)
        pad.tick(1.1, 0.05)
        self.assertEqual(out.moved, [0, 0])
        self.assertEqual(len(out.log), 2)
        press(pad, 'guide', 2)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': True, 'sel': -1})

    def test_mouse_toggle_reports_state(self):
        pad, out = make()
        pad.command({'cmd': 'mouse', 'on': False})
        self.assertTrue(pad.paused)
        self.assertEqual(out.sent[-1], {'t': 'mouse', 'on': False})
        pad.command({'cmd': 'mouse', 'on': True})
        self.assertFalse(pad.paused)

    def test_keys_context_sends_enter_and_escape(self):
        pad, out = make()
        pad.command({'cmd': 'keys', 'on': True})
        press(pad, 'a')
        press(pad, 'b')
        self.assertEqual([k for k, d in out.log if d], ['KEY_ENTER', 'KEY_ESC'])


class Radial(unittest.TestCase):
    def test_stick_picks_item_and_a_chooses(self):
        pad, out = make()
        press(pad, 'guide')
        pad.axes(0, 0, 1, 0)          # right stick to the right: item 2 of 8
        pad.tick(0.1, 0.01)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': True, 'sel': 2})
        pad.axes(0, 0, 0, 0)
        press(pad, 'a', 0.2)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': False, 'pick': 2})
        self.assertEqual(pad.mode, 'desk')
        self.assertEqual(out.log, [])  # no click leaked to the desktop

    def test_down_left_diagonal(self):
        pad, out = make()
        press(pad, 'guide')
        pad.axes(-0.7, 0.7, 0, 0)     # left stick down-left: item 5
        pad.tick(0.1, 0.01)
        self.assertEqual(pad.sel, 5)

    def test_b_and_guide_close(self):
        pad, out = make()
        press(pad, 'guide')
        press(pad, 'b')
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': False, 'pick': -1})
        press(pad, 'guide')
        press(pad, 'guide')
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': False, 'pick': -1})

    def test_a_without_selection_does_nothing(self):
        pad, out = make()
        press(pad, 'guide')
        press(pad, 'a')
        self.assertEqual(pad.mode, 'radial')


class Keyboard(unittest.TestCase):
    """The on-screen keyboard is Hearth's shared one (desktop/shell/Keyboard.qml): Y asks for it, and while it's open
    hearth-pad hands it the buttons and the left stick; it types itself (wtype), or through hearth-pad (emit / key)."""

    def typed(self, out):
        return [k for k, d in out.log if d]

    def test_y_asks_for_it(self):
        pad, out = make()
        press(pad, 'y')
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        self.assertEqual(pad.mode, 'desk')          # until the shell says it's open

    def test_buttons_go_to_the_keyboard(self):
        pad, out = make()
        pad.command({'cmd': 'osk', 'open': True})
        press(pad, 'a', 1)
        press(pad, 'x', 2)
        press(pad, 'lt', 3)
        got = [(m['b'], m['down']) for m in out.sent if m.get('t') == 'oskpad']
        self.assertEqual(got, [('a', True), ('a', False), ('x', True), ('x', False), ('lt', True), ('lt', False)])
        self.assertEqual(self.typed(out), [])        # nothing typed by hearth-pad itself

    def test_rt_and_right_stick_stay_the_pointer(self):
        pad, out = make()
        pad.command({'cmd': 'osk', 'open': True})
        pad.button('rt', True, 1.0)
        self.assertIn(('BTN_LEFT', True), out.log)
        pad.button('rt', False, 1.1)
        pad.axes(0, 0, 1, 0)
        pad.tick(1.2, 0.05)
        self.assertGreater(out.moved[0], 0)

    def test_left_stick_goes_to_the_keyboard(self):
        pad, out = make()
        pad.command({'cmd': 'osk', 'open': True})
        pad.axes(0.0, -1.0, 0.0, 0.0)
        pad.tick(1.0, 0.016)
        axes = [m for m in out.sent if m.get('t') == 'oskaxes']
        self.assertEqual(axes[-1]['ly'], -1.0)
        self.assertEqual(out.wheeled, [0, 0])        # no scrolling meanwhile

    def test_split_sticks_take_both(self):
        pad, out = make()
        pad.command({'cmd': 'osk', 'open': True, 'split': True})
        pad.axes(-1.0, 0.0, 1.0, 0.0)
        pad.tick(1.0, 0.05)
        self.assertEqual(out.moved, [0, 0])          # the right stick isn't the pointer now
        self.assertEqual([m for m in out.sent if m.get('t') == 'oskaxes'][-1]['rx'], 1.0)
        press(pad, 'rt', 2)
        self.assertEqual([m['b'] for m in out.sent if m.get('t') == 'oskpad'][-2:], ['rt', 'rt'])

    def test_closing(self):
        pad, out = make()
        pad.command({'cmd': 'osk', 'open': True})
        pad.command({'cmd': 'osk', 'open': False})
        self.assertEqual(pad.mode, 'desk')

    def test_typing_without_wtype(self):
        pad, out = make()
        pad.command({'cmd': 'emit', 'text': 'Hi @'})
        pad.command({'cmd': 'key', 'key': 'bksp', 'n': 2})
        self.assertEqual(self.typed(out), ['KEY_LEFTSHIFT', 'KEY_H', 'KEY_I', 'KEY_SPACE', 'KEY_LEFTSHIFT', 'KEY_2', 'KEY_BACKSPACE', 'KEY_BACKSPACE'])

    def test_ctrl_c_and_every_terminal_character(self):
        for ch in '"|:{}\\~<>?_+`^\'[];,./-=':
            self.assertIn(ch, hp.CHARS, ch)
        pad, out = make()
        pad.command({'cmd': 'emit', 'text': 'c', 'mods': ['ctrl']})
        self.assertEqual(self.typed(out), ['KEY_LEFTCTRL', 'KEY_C'])

    def test_rumble_command(self):
        pad, out = make()
        pad.command({'cmd': 'rumble', 'strength': 0.5, 'ms': 20})
        self.assertEqual(out.rumbled, [(0.5, 20)])


if __name__ == '__main__':
    unittest.main()


class BluetoothStickTest(unittest.TestCase):
    """An Xbox controller over Bluetooth without xpadneo (hid-generic) has its right stick on Z/RZ and its
    triggers on BRAKE/GAS: the right stick must move the pointer, not press the triggers."""

    class E:
        EV_KEY, EV_ABS = 1, 3
        ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ, ABS_GAS, ABS_BRAKE, ABS_HAT0X, ABS_HAT0Y = 0, 1, 2, 3, 4, 5, 9, 10, 16, 17

    class Info:
        def __init__(self, lo, hi):
            self.min, self.max = lo, hi

    class Dev:
        def __init__(self, codes):
            self.codes = codes

        def capabilities(self, absinfo=True):
            return {3: [(c, BluetoothStickTest.Info(lo, hi)) for c, (lo, hi) in self.codes.items()]}

    class Ev:
        def __init__(self, t, c, v):
            self.type, self.code, self.value = t, c, v

    def make(self, codes):
        return hp.Controller(self.Dev(codes), self.E)

    def test_bluetooth_right_stick(self):
        e = self.E
        c = self.make({e.ABS_X: (0, 65535), e.ABS_Y: (0, 65535), e.ABS_Z: (0, 65535), e.ABS_RZ: (0, 65535),
                       e.ABS_BRAKE: (0, 1023), e.ABS_GAS: (0, 1023)})
        c.handle(self.Ev(e.EV_ABS, e.ABS_Z, 65535), {})
        self.assertGreater(c.ax['rx'], 0.9)
        self.assertFalse(c.btn.get('lt_axis') or c.btn.get('rt_axis'))
        c.handle(self.Ev(e.EV_ABS, e.ABS_GAS, 1023), {})
        self.assertTrue(c.btn.get('rt_axis'))

    def test_usual_layout_unchanged(self):
        e = self.E
        c = self.make({e.ABS_X: (-32768, 32767), e.ABS_Y: (-32768, 32767), e.ABS_RX: (-32768, 32767), e.ABS_RY: (-32768, 32767),
                       e.ABS_Z: (0, 255), e.ABS_RZ: (0, 255)})
        c.handle(self.Ev(e.EV_ABS, e.ABS_RX, 32767), {})
        self.assertGreater(c.ax['rx'], 0.9)
        c.handle(self.Ev(e.EV_ABS, e.ABS_Z, 255), {})
        self.assertTrue(c.btn.get('lt_axis'))


class GuideXTest(unittest.TestCase):
    """Guide + X opens and closes the on-screen keyboard (Hearth Classic and, with --osk, Hearth Plasma)."""

    def test_classic(self):
        out = Out()
        p = hp.Pad(out)
        p.button('guide', True, 0.0)              # Guide opens the radial menu as always...
        self.assertEqual(p.mode, 'radial')
        p.button('x', True, 0.1)                  # ... X while it's held: the keyboard instead
        self.assertEqual(p.mode, 'desk')
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        self.assertIn({'t': 'radial', 'open': False, 'pick': -1}, out.sent)
        p.button('x', False, 0.2); p.button('guide', False, 0.2)
        p.command({'cmd': 'osk', 'open': True})   # the shell opened it
        p.button('guide', True, 1.0); p.button('x', True, 1.1)
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': False})
        self.assertEqual(p.mode, 'desk')

    def test_plasma_only_the_chord(self):
        out = Out()
        p = hp.Pad(out, osk_only=True)
        p.button('a', True, 0.0); p.button('a', False, 0.1)
        p.axes(0, 0, 1.0, 0); p.tick(0.2, 0.1)
        self.assertEqual(out.log, [])             # nothing on the desktop: Plasma's own
        self.assertEqual(out.moved, [0, 0])
        self.assertFalse(p.wants_grab)
        p.button('guide', True, 1.0)
        self.assertNotEqual(p.mode, 'radial')
        p.button('x', True, 1.1)
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        p.command({'cmd': 'osk', 'open': True})
        self.assertTrue(p.wants_grab)             # held while it's open
        p.button('x', False, 1.2); p.button('guide', False, 1.2)
        p.button('a', True, 2.0)
        self.assertEqual(out.sent[-1], {'t': 'oskpad', 'b': 'a', 'down': True})


class PlasmaKeyboardFilesTest(unittest.TestCase):
    """Hearth Plasma's keyboard: started by an autostart entry only Hearth Plasma reads, guarded (no quickshell or
    file: nothing), running the shared Keyboard.qml without wtype and hearth-pad --osk."""

    def test_files(self):
        root = PATH.parent.parent.parent.parent.parent
        auto = (root / 'os' / 'system_files' / 'usr' / 'share' / 'hearth-plasma' / 'xdg' / 'autostart' / 'hearth-osk.desktop').read_text()
        self.assertIn('OnlyShowIn=KDE;', auto)
        self.assertIn('command -v quickshell', auto)
        self.assertIn('osk-plasma.qml', auto)
        self.assertIn('exit 0', auto)
        qml = (PATH.parent / 'osk-plasma.qml').read_text()
        self.assertIn('useWtype: false', qml)
        self.assertIn('"--plasma"', qml)
        for part in ('RadialMenu {', 'Help {', 'activateLauncherMenu', 'cmd: "plasma", do: "quick"', 'cmd: "plasma", do: "close"'):
            self.assertIn(part, qml)
        self.assertIn('property bool useWtype', (PATH.parent / 'Keyboard.qml').read_text())
        self.assertNotIn('readonly property var controls', (PATH.parent / 'Help.qml').read_text())
        # KWin's script: on only in Hearth Plasma (its own kwinrc), says what's in front to org.hearth.Pad.
        kwin = root / 'os' / 'system_files' / 'usr' / 'share' / 'kwin' / 'scripts' / 'hearthpad'
        meta = json.loads((kwin / 'metadata.json').read_text())
        self.assertEqual(meta['KPlugin']['Id'], 'hearthpad')
        self.assertFalse(meta['KPlugin']['EnabledByDefault'])
        js = (kwin / 'contents' / 'code' / 'main.js').read_text()
        self.assertIn('"org.hearth.Pad", "/org/hearth/Pad", "org.hearth.Pad", "Window"', js)
        self.assertIn('hearthpadEnabled=true', (root / 'os' / 'system_files' / 'usr' / 'share' / 'hearth-plasma' / 'xdg' / 'kwinrc').read_text())
        self.assertTrue(pathlib.Path(hp.PLASMA_REPORT_JS).is_file())
        for m in ('Window', 'Probe'):
            self.assertIn('<method name="%s">' % m, hp.PLASMA_XML)


class PlasmaTest(unittest.TestCase):
    """--plasma: Hearth Plasma's whole controller, everything Hearth Classic does, through KWin's shortcuts."""

    def make(self):
        out = Out()
        return hp.Pad(out, plasma=True), out

    def test_pointer_clicks_scroll_like_classic(self):
        pad, out = self.make()
        pad.axes(0, 0, 1.0, 0); pad.tick(0.1, 0.05)
        self.assertGreater(out.moved[0], 0)
        pad.axes(0, -1.0, 0, 0); pad.tick(0.2, 0.05)
        self.assertGreater(out.wheeled[0], 0)
        press(pad, 'rt', 1.0); press(pad, 'a', 1.1); press(pad, 'x', 1.2); press(pad, 'lt', 1.3); press(pad, 'l3', 1.4)
        self.assertEqual([n for n, d in out.log if d], ['BTN_LEFT', 'BTN_LEFT', 'BTN_RIGHT', 'BTN_RIGHT', 'BTN_MIDDLE'])

    def test_guide_menu_and_osk(self):
        pad, out = self.make()
        press(pad, 'guide', 0.0)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': True, 'sel': -1})
        press(pad, 'right', 0.1); press(pad, 'a', 0.2)
        self.assertEqual(out.sent[-1], {'t': 'radial', 'open': False, 'pick': 2})   # Quick Access
        pad.button('guide', True, 1.0); pad.button('x', True, 1.1)
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        pad.button('x', False, 1.2); pad.button('guide', False, 1.2)
        pad.command({'cmd': 'osk', 'open': True})
        self.assertTrue(pad.wants_grab)
        press(pad, 'a', 2.0)
        self.assertIn({'t': 'oskpad', 'b': 'a', 'down': True}, out.sent)
        press(pad, 'y', 3.0, 0.01)
        pad.command({'cmd': 'osk', 'open': False})
        self.assertFalse(pad.wants_grab)
        press(pad, 'y', 4.0)
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        press(pad, 'menu', 5.0)
        self.assertEqual(out.sent[-1], {'t': 'launcher'})

    def test_view_chords_are_kwins(self):
        pad, out = self.make()
        pad.button('view', True, 0.0)
        for b in ('b', 'up', 'down', 'left', 'right', 'a', 'x', 'y'):
            press(pad, b, 0.1)
        pad.button('view', False, 0.2)
        log = [n for n, d in out.log if d]
        self.assertEqual(log[:2], ['KEY_LEFTALT', 'KEY_F4'])                      # close: Alt+F4, not labwc's Super+Q
        self.assertNotIn('KEY_Q', log)
        for k in ('KEY_PAGEUP', 'KEY_PAGEDOWN', 'KEY_LEFT', 'KEY_RIGHT', 'KEY_P', 'KEY_W', 'KEY_V'):
            self.assertIn(k, log)
        self.assertNotIn({'t': 'help'}, out.sent)                                 # View was used: no controls card
        press(pad, 'view', 1.0, 0.1)
        self.assertEqual(out.sent[-1], {'t': 'help'})

    def test_windows_switch_with_lb_rb(self):
        pad, out = self.make()
        press(pad, 'rb', 0.0)
        self.assertEqual(out.log[:3], [('KEY_LEFTALT', True), ('KEY_TAB', True), ('KEY_TAB', False)])
        pad.tick(5.0, 0.01)
        self.assertEqual(out.log[-1], ('KEY_LEFTALT', False))

    def test_menus_take_keys(self):
        pad, out = self.make()
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'caption': '', 'fullscreen': False, 'kind': 'popup'})
        self.assertTrue(pad.keys_ctx)                    # the launcher / tray / Quick Access popup
        out.log.clear()
        press(pad, 'a', 0.0); press(pad, 'down', 0.1); press(pad, 'b', 0.2)
        self.assertEqual([n for n, d in out.log if d], ['KEY_ENTER', 'KEY_DOWN', 'KEY_ESC'])
        out.log.clear()
        press(pad, 'rt', 0.3)                            # RT still clicks
        self.assertEqual(out.log, [('BTN_LEFT', True), ('BTN_LEFT', False)])
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'caption': 'Desktop', 'fullscreen': False, 'kind': 'desktop'})
        self.assertFalse(pad.keys_ctx)                   # the desktop: A clicks its icons
        pad.command({'cmd': 'window', 'cls': 'org.kde.dolphin', 'caption': 'Home', 'fullscreen': False, 'kind': 'normal'})
        self.assertTrue(pad.keys_ctx)
        pad.command({'cmd': 'window', 'cls': 'firefox', 'caption': 'x', 'fullscreen': False, 'kind': 'normal'})
        self.assertFalse(pad.keys_ctx)

    def test_taskbar_and_overview_take_keys_until_done(self):
        pad, out = self.make()
        pad.command({'cmd': 'window', 'cls': 'firefox', 'caption': 'x', 'fullscreen': False, 'kind': 'normal'})
        pad.button('view', True, 0.0); press(pad, 'a', 0.1); pad.button('view', False, 0.2)
        self.assertTrue(pad.keys_ctx)                    # the taskbar has the keyboard focus
        out.log.clear()
        press(pad, 'right', 0.3); press(pad, 'a', 0.4)
        self.assertEqual([n for n, d in out.log if d], ['KEY_RIGHT', 'KEY_ENTER'])
        self.assertFalse(pad.keys_ctx)                   # A opened it: back to the window's own rules
        pad.command({'cmd': 'plasma', 'do': 'overview'})
        self.assertTrue(pad.keys_ctx)
        press(pad, 'b', 1.0)
        self.assertFalse(pad.keys_ctx)

    def test_hearth_menu_actions(self):
        pad, out = self.make()
        pad.command({'cmd': 'plasma', 'do': 'close'})
        self.assertEqual(out.log, [('KEY_LEFTALT', True), ('KEY_F4', True), ('KEY_F4', False), ('KEY_LEFTALT', False)])
        out.log.clear()
        pad.command({'cmd': 'plasma', 'do': 'quick'})
        self.assertEqual([n for n, d in out.log if d], ['KEY_LEFTMETA', 'KEY_LEFTALT', 'KEY_A'])
        out.log.clear()
        pad.command({'cmd': 'plasma', 'do': 'rm -rf'})  # unknown: nothing
        self.assertEqual(out.log, [])

    def test_fullscreen_pauses(self):
        pad, out = self.make()
        pad.command({'cmd': 'window', 'cls': 'steam_app_570', 'caption': 'Dota 2', 'fullscreen': True, 'kind': 'normal'})
        self.assertTrue(pad.paused)
        press(pad, 'a', 0.0)
        pad.axes(0, 0, 1.0, 0); pad.tick(0.1, 0.05)
        self.assertEqual((out.log, out.moved), ([], [0, 0]))
        press(pad, 'guide', 0.2)                         # the Hearth menu still opens
        self.assertEqual(out.sent[-1]['t'], 'radial')
        press(pad, 'b', 0.3)
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'caption': '', 'fullscreen': True, 'kind': 'desktop'})
        self.assertFalse(pad.paused)

    def test_keyboard_opens_on_text_fields_after_the_controller(self):
        pad, out = self.make()
        pad.command({'cmd': 'window', 'cls': 'org.kde.dolphin', 'caption': 'Home', 'fullscreen': False, 'kind': 'normal'})
        press(pad, 'a', 10.0)
        pad.command({'cmd': 'textfield', 'on': True, 'now': 11.0})
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})
        n = len(out.sent)
        pad.command({'cmd': 'textfield', 'on': True, 'now': 11.5})   # still the same field: once
        self.assertEqual(len(out.sent), n)
        pad.command({'cmd': 'textfield', 'on': False, 'now': 12.0})
        pad.command({'cmd': 'textfield', 'on': True, 'now': 40.0})  # the mouse and keyboard got there: no
        self.assertEqual(len(out.sent), n)
        # Plasma's launcher search: the D-pad works its list, the keyboard waits for Y.
        pad.command({'cmd': 'textfield', 'on': False, 'now': 41.0})
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'caption': '', 'fullscreen': False, 'kind': 'popup'})
        press(pad, 'menu', 42.0)
        pad.command({'cmd': 'textfield', 'on': True, 'now': 42.5})
        self.assertNotIn({'t': 'osk', 'open': True}, out.sent[n:])
        # KRunner is a search box: it opens the keyboard when the controller brought it up.
        press(pad, 'a', 50.0)
        pad.command({'cmd': 'window', 'cls': 'org.kde.krunner', 'caption': '', 'fullscreen': False, 'kind': 'normal', 'now': 50.5})
        self.assertEqual(out.sent[-1], {'t': 'osk', 'open': True})

    def test_plasma_commands_ignored_elsewhere(self):
        out = Out()
        pad = hp.Pad(out)                                # Hearth Classic
        pad.command({'cmd': 'plasma', 'do': 'close'})
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'kind': 'popup'})
        self.assertEqual(out.log, [])
        self.assertFalse(pad.keys_ctx)
        self.assertTrue(set(k for c in hp.PLASMA_SHORTCUTS.values() for k in c) <= set(hp.KEYS_USED))


class ButtonMapTest(unittest.TestCase):
    """Sony pads swap X and Y (Triangle is BTN_NORTH); xpad's d-pad buttons and a Bluetooth Xbox controller's
    KEY_HOMEPAGE Guide are read too."""

    class E:
        BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST = 0x130, 0x131, 0x133, 0x134
        BTN_TL, BTN_TR, BTN_TL2, BTN_TR2 = 0x136, 0x137, 0x138, 0x139
        BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR = 0x13a, 0x13b, 0x13c, 0x13d, 0x13e
        BTN_TRIGGER_HAPPY1, BTN_TRIGGER_HAPPY2, BTN_TRIGGER_HAPPY3, BTN_TRIGGER_HAPPY4 = 0x2c0, 0x2c1, 0x2c2, 0x2c3
        KEY_HOMEPAGE = 0xac

    def test_maps(self):
        e = self.E
        m = hp.button_map(e)
        self.assertEqual((m[e.BTN_NORTH], m[e.BTN_WEST]), ('x', 'y'))
        self.assertEqual((m[0x2c0], m[0x2c1], m[0x2c2], m[0x2c3]), ('left', 'right', 'up', 'down'))
        self.assertEqual(m[e.KEY_HOMEPAGE], 'guide')
        s = hp.button_map(e, hp.SONY)
        self.assertEqual((s[e.BTN_NORTH], s[e.BTN_WEST]), ('y', 'x'))


class HotplugTest(unittest.TestCase):
    """A device that goes away between being opened and being asked what it is (ENODEV) is skipped, not fatal."""

    def test_device_gone_while_scanning(self):
        class E:
            EV_KEY, EV_ABS, BTN_SOUTH = 1, 3, 0x130

        class Gone:
            name = 'Wireless Controller'
            closed = False

            def __init__(self, path):
                pass

            def capabilities(self, absinfo=True):
                raise OSError(19, 'No such device')

            def close(self):
                Gone.closed = True

        class Evdev:
            ecodes = E
            InputDevice = Gone

            @staticmethod
            def list_devices():
                return ['/dev/input/event9']
        self.assertEqual(hp.gamepads(Evdev, {}), {})
        self.assertTrue(Gone.closed)
