"""Tests for desktop/shell/hearth-pad (controller → desktop). Run: python3 -m unittest discover -s test -p 'test_*.py'"""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
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
