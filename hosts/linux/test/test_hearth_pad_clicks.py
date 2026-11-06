"""hearth-pad: the controller's clicks reach windows and labwc's title bars (Desktop Mode, Mantel).

The owner's report: the right stick moved the pointer fine, but A / RT and X / LT clicks failed in almost every app,
worst on the title bar (close, dragging a window: "9 times out of 10 it won't work"), so everything was done with the
D-pad and A. Causes covered here:
- in a window or menu that takes the keys (keys {on}: Hearth Files, Settings, Task Manager, every Open / Save dialog,
  Hearth's menus) A was Enter and RT / LT did nothing, while the pointer still moved;
- a thumb still on the right stick moved the pointer between press and release (off the close button);
- a keys {on} whose "off" got lost stayed on (no time limit, not said again on focus changes);
- a quick tap that went down and up between two looks of the main loop was lost.
Run: python3 -m unittest discover -s test -p 'test_hearth_pad_clicks.py'"""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import importlib.machinery
import importlib.util
import pathlib
import tempfile
import types
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-pad'
APPS = PATH.parent / 'controller-apps.json'
loader = importlib.machinery.SourceFileLoader('hearth_pad_clicks', str(PATH))
spec = importlib.util.spec_from_loader('hearth_pad_clicks', loader)
hp = importlib.util.module_from_spec(spec)
loader.exec_module(hp)


class Out:
    """A timeline of what would go to uinput: ('move', dx, dy) and ('press', name, down)."""

    def __init__(self):
        self.ev = []
        self.sent = []

    def move(self, dx, dy):
        self.ev.append(('move', dx, dy))

    def wheel(self, v, h):
        self.ev.append(('wheel', v, h))

    def press(self, name, down):
        self.ev.append(('press', name, down))

    def send(self, obj):
        self.sent.append(obj)

    def rumble(self, strength, ms):
        pass

    def presses(self):
        return [(n, d) for k, n, d in (e for e in self.ev if e[0] == 'press')]

    def moved(self):
        return sum(abs(e[1]) + abs(e[2]) for e in self.ev if e[0] == 'move')


class Base(unittest.TestCase):
    def setUp(self):
        self.notes = []
        old = hp.note
        hp.note = self.notes.append              # pad.log stays out of the home folder
        self.addCleanup(setattr, hp, 'note', old)

    def make(self, **kw):
        out = Out()
        return hp.Pad(out, **kw), out

    def run_ticks(self, pad, t0, t1, dt=0.01):
        t = t0
        while t < t1 - 1e-9:
            t += dt
            pad.tick(t, dt)
        return t


class UinputDevices(Base):
    """Clicks go out on the same uinput device as the pointer's motion, which declares the buttons (libinput / labwc
    treat it as a mouse), each in its own SYN_REPORT frame; keys go on the keyboard device."""

    class FakeUInput:
        made = []

        def __init__(self, caps, name=''):
            self.caps, self.name, self.frames, self.cur = caps, name, [], []
            UinputDevices.FakeUInput.made.append(self)

        def write(self, t, code, value):
            self.cur.append((t, code, value))

        def syn(self):
            self.frames.append(self.cur)
            self.cur = []

    def evdev(self):
        names = ['EV_KEY', 'EV_REL', 'REL_X', 'REL_Y', 'REL_WHEEL', 'REL_HWHEEL', 'REL_WHEEL_HI_RES', 'REL_HWHEEL_HI_RES',
                 'BTN_LEFT', 'BTN_RIGHT', 'BTN_MIDDLE'] + list(hp.KEYS_USED)
        ec = types.SimpleNamespace(**{n: 1000 + i for i, n in enumerate(names)})
        ec.EV_KEY, ec.EV_REL = 1, 2
        self.FakeUInput.made = []
        return types.SimpleNamespace(ecodes=ec, UInput=self.FakeUInput), ec

    def test_buttons_on_the_pointer_device_with_syn(self):
        evdev, e = self.evdev()
        out = hp.Output(evdev)
        ptr, kbd = self.FakeUInput.made
        self.assertEqual(ptr.name, 'Hearth controller pointer')
        for b in (e.BTN_LEFT, e.BTN_RIGHT, e.BTN_MIDDLE):
            self.assertIn(b, ptr.caps[e.EV_KEY], 'the pointer device must declare its buttons')
        for r in (e.REL_X, e.REL_Y):
            self.assertIn(r, ptr.caps[e.EV_REL])
        self.assertNotIn(e.BTN_LEFT, kbd.caps[e.EV_KEY], 'no mouse buttons on the keyboard device')
        out.move(3, 0)
        out.press('BTN_LEFT', True)
        out.press('BTN_LEFT', False)
        out.press('BTN_RIGHT', True)
        out.press('BTN_RIGHT', False)
        out.press('KEY_ENTER', True)
        self.assertEqual(ptr.frames, [[(e.EV_REL, e.REL_X, 3)], [(e.EV_KEY, e.BTN_LEFT, 1)], [(e.EV_KEY, e.BTN_LEFT, 0)],
                                      [(e.EV_KEY, e.BTN_RIGHT, 1)], [(e.EV_KEY, e.BTN_RIGHT, 0)]],
                         'press and release each in a frame of their own, on the pointer device')
        self.assertEqual(kbd.frames, [[(e.EV_KEY, e.KEY_ENTER, 1)]])
        self.assertEqual(ptr.cur, [])
        self.assertEqual(kbd.cur, [])


class TitleBar(Base):
    """labwc's close button (a Click) and dragging the title bar (a Drag)."""

    CLOSE = (980, 1004, 0, 30)    # x0, x1, y0, y1 of a close button

    def pos(self, out, start):
        x, y = start
        for e in out.ev:
            if e[0] == 'move':
                x, y = x + e[1], y + e[2]
        return x, y

    def inside(self, p):
        x0, x1, y0, y1 = self.CLOSE
        return x0 <= p[0] <= x1 and y0 <= p[1] <= y1

    def test_tap_with_drift_is_a_clean_click_on_close(self):
        """RT pulled with the thumb still pushing the right stick a little: press and release with no motion between."""
        for btn in ('rt', 'a'):
            pad, out = self.make()
            start = (992, 15)
            pad.axes(0, 0, 0.35, 0.2)            # a thumb still on the stick
            t = self.run_ticks(pad, 0, 0.05)
            before = len(out.ev)
            pad.button(btn, True, t)
            t = self.run_ticks(pad, t, t + 0.15)
            pad.button(btn, False, t)
            frame = out.ev[before:]
            self.assertEqual(frame[0], ('press', 'BTN_LEFT', True))
            self.assertEqual(frame[-1], ('press', 'BTN_LEFT', False))
            self.assertEqual([e for e in frame if e[0] == 'move'], [], 'no motion between press and release (%s)' % btn)
            # labwc's Click: pressed and released over the close button.
            at_press = self.pos(types.SimpleNamespace(ev=out.ev[:before]), start)
            self.assertTrue(self.inside(at_press))
            self.assertTrue(self.inside(self.pos(out, start)))

    def test_press_hold_move_release_drags(self):
        pad, out = self.make()
        pad.button('rt', True, 0.0)
        pad.axes(0, 0, 1.0, 0)
        t = self.run_ticks(pad, 0.0, 0.6)
        pad.button('rt', False, t)
        kinds = [e[0] if e[0] == 'move' else (e[1], e[2]) for e in out.ev]
        self.assertEqual(kinds[0], ('BTN_LEFT', True))
        self.assertEqual(kinds[-1], ('BTN_LEFT', False))
        self.assertIn('move', kinds[1:-1], 'held and moved: the pointer moves with the button down (labwc: Drag)')
        self.assertGreater(out.moved(), 200)
        first_move = next(i for i, e in enumerate(out.ev) if e[0] == 'move')
        self.assertEqual(first_move, 1, 'the press comes first, then the motion')

    def test_pointer_moves_again_after_the_click(self):
        pad, out = self.make()
        pad.axes(0, 0, 1.0, 0)
        pad.button('a', True, 0.0)
        pad.button('a', False, 0.05)
        self.run_ticks(pad, 0.05, 0.3)
        self.assertGreater(out.moved(), 50, 'a click only holds the pointer while the button is down')

    def test_game_mode_clicks_settle_too(self):
        pad, out = self.make(game=True)
        pad.axes(0, 0, 0.5, 0)
        pad.button('rt', True, 0.0)
        self.run_ticks(pad, 0.0, 0.1)
        pad.button('rt', False, 0.1)
        self.assertEqual(out.ev, [('press', 'BTN_LEFT', True), ('press', 'BTN_LEFT', False)])


class KeysMode(Base):
    """keys {on}: a menu or a window that takes the keys (Hearth Files, Settings, Task Manager, Open / Save dialogs)."""

    def keys(self, pad, on=True, now=0.0, **kw):
        pad.command(dict({'cmd': 'keys', 'on': on, 'now': now}, **kw))

    def test_triggers_always_click(self):
        pad, out = self.make()
        self.keys(pad)
        pad.button('rt', True, 1.0)
        pad.button('rt', False, 1.1)
        pad.button('lt', True, 1.2)
        pad.button('lt', False, 1.3)
        pad.button('l3', True, 1.4)
        pad.button('l3', False, 1.5)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False), ('BTN_RIGHT', True), ('BTN_RIGHT', False),
                                         ('BTN_MIDDLE', True), ('BTN_MIDDLE', False)])

    def test_a_is_enter_until_the_pointer_moves(self):
        pad, out = self.make()
        self.keys(pad)
        pad.button('a', True, 1.0)
        pad.button('a', False, 1.1)
        self.assertEqual(out.presses(), [('KEY_ENTER', True), ('KEY_ENTER', False)], 'a menu just opened: A picks')
        del out.ev[:]
        pad.axes(0, 0, 1.0, 0)                   # the right stick: the pointer to the title bar
        t = self.run_ticks(pad, 1.1, 1.4)
        pad.axes(0, 0, 0, 0)
        self.assertGreater(out.moved(), 0)
        pad.button('a', True, t + 0.5)
        pad.button('a', False, t + 0.6)
        pad.button('x', True, t + 0.7)
        pad.button('x', False, t + 0.8)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False), ('BTN_RIGHT', True), ('BTN_RIGHT', False)])
        del out.ev[:]
        pad.button('down', True, t + 1.0)        # the D-pad: back to the keys
        pad.button('down', False, t + 1.1)
        pad.button('a', True, t + 1.2)
        pad.button('a', False, t + 1.3)
        pad.button('x', True, t + 1.4)
        pad.button('x', False, t + 1.5)
        self.assertEqual(out.presses(), [('KEY_DOWN', True), ('KEY_DOWN', False), ('KEY_ENTER', True), ('KEY_ENTER', False),
                                         ('KEY_COMPOSE', True), ('KEY_COMPOSE', False)])

    def test_a_held_drags_in_keys_mode(self):
        pad, out = self.make()
        self.keys(pad)
        pad.axes(0, 0, 1.0, 0)
        t = self.run_ticks(pad, 0.0, 0.2)
        pad.button('a', True, t)
        t = self.run_ticks(pad, t, t + 0.5)
        pad.button('a', False, t)
        p = out.presses()
        self.assertEqual(p, [('BTN_LEFT', True), ('BTN_LEFT', False)])
        i = out.ev.index(('press', 'BTN_LEFT', True))
        self.assertTrue(any(e[0] == 'move' for e in out.ev[i:]), 'held: the window follows the pointer')

    def test_release_follows_its_press(self):
        """A went down as Enter; the stick moves before it comes up: no stray mouse release (and the other way round)."""
        pad, out = self.make()
        self.keys(pad)
        pad.button('a', True, 1.0)
        pad.axes(0, 0, 1.0, 0)
        self.run_ticks(pad, 1.0, 1.2)
        pad.button('a', False, 1.3)
        self.assertEqual(out.presses(), [('KEY_ENTER', True), ('KEY_ENTER', False)])

    def test_a_menu_opening_gives_a_back_to_enter(self):
        pad, out = self.make()
        pad.axes(0, 0, 1.0, 0)
        self.run_ticks(pad, 0.0, 0.2)
        pad.axes(0, 0, 0, 0)
        self.keys(pad, now=0.5)                  # the launcher opened after the pointer moved
        pad.button('a', True, 0.6)
        pad.button('a', False, 0.7)
        self.assertIn(('KEY_ENTER', True), out.presses())

    def test_menu_closing_ends_keys(self):
        pad, out = self.make()
        self.keys(pad)
        self.keys(pad, on=False, now=1.0)
        pad.button('a', True, 1.1)
        pad.button('a', False, 1.2)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])
        self.assertIn('keys: off', self.notes[-1])

    def test_keys_lapse_without_the_shell_saying_them_again(self):
        pad, out = self.make()
        self.keys(pad, now=0.0, ttl=12, why='launcher')
        self.assertTrue(pad.keys_ctx)
        self.assertIn('launcher', self.notes[-1])
        pad.tick(8.0, 0.01)
        self.keys(pad, now=8.0, ttl=12, why='launcher')   # Pad.qml's repeat
        pad.tick(15.0, 0.01)
        self.assertTrue(pad.keys_ctx, 'said again in time: still on')
        pad.tick(20.5, 0.01)
        self.assertFalse(pad.keys_ctx, 'never said again: it can\'t stick')
        self.assertIn('stopped saying', self.notes[-1])
        pad.button('a', True, 21.0)
        pad.button('a', False, 21.1)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])

    def test_huge_ttl_is_capped_and_no_ttl_is_old_behaviour(self):
        pad, _ = self.make()
        self.keys(pad, now=0.0, ttl=1e9)
        pad.tick(hp.KEYS_TTL_MAX + 1, 0.01)
        self.assertFalse(pad.keys_ctx)
        self.keys(pad, now=0.0)                  # COSMIC / GNOME shells: no ttl, stays until they say off
        pad.tick(10000.0, 0.01)
        self.assertTrue(pad.keys_ctx)

    def test_repeat_doesnt_drop_a_drag_or_held_arrow(self):
        pad, out = self.make()
        self.keys(pad, ttl=12)
        pad.button('rt', True, 0.5)
        pad.button('down', True, 0.6)
        self.keys(pad, now=4.0, ttl=12)          # the repeat while dragging
        self.assertNotIn(('BTN_LEFT', False), out.presses())
        self.assertNotIn(('KEY_DOWN', False), out.presses())

    def test_focus_says_keys_again(self):
        pad, out = self.make()
        self.keys(pad, now=0.0)
        pad.command({'cmd': 'focus', 'app': 'firefox', 'title': 'Firefox', 'keys': False, 'ttl': 12, 'now': 1.0})
        self.assertFalse(pad.keys_ctx, 'a normal window in front: the keys mode ends with it')
        pad.button('a', True, 1.1)
        pad.button('a', False, 1.2)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])
        pad.command({'cmd': 'focus', 'app': 'org.kde.dolphin', 'title': 'Home', 'keys': True, 'why': 'window', 'ttl': 12, 'now': 2.0})
        self.assertTrue(pad.keys_ctx)
        pad.command({'cmd': 'focus', 'app': 'foot', 'title': 'foot'})   # an older shell (no keys in focus): unchanged
        self.assertTrue(pad.keys_ctx)

    def test_plasma_keeps_its_own_keys(self):
        """Hearth Plasma's keys context (KWin's popups): A stays Enter even after the pointer moved."""
        pad, out = self.make(plasma=True)
        pad.command({'cmd': 'window', 'cls': 'plasmashell', 'caption': '', 'fullscreen': False, 'kind': 'popup', 'now': 0.0})
        self.assertTrue(pad.keys_ctx)
        pad.axes(0, 0, 1.0, 0)
        self.run_ticks(pad, 0.0, 0.2)
        pad.button('a', True, 0.5)
        pad.button('a', False, 0.6)
        self.assertIn(('KEY_ENTER', True), out.presses())

    def test_cosmic_menus_still_enter(self):
        out = Out()
        pad = hp.CosmicPad(out)
        pad.command({'cmd': 'keys', 'on': True})
        pad.button('a', True, 0.1)
        pad.button('a', False, 0.2)
        self.assertEqual(out.presses(), [('KEY_ENTER', True), ('KEY_ENTER', False)])


class OrdinaryApps(Base):
    """Normal apps aren't taken for controller apps: the desktop controls (and clicks) stay on in them."""

    def test_normal_apps_are_not_owners(self):
        with tempfile.TemporaryDirectory() as d:
            owners = hp.Owners(str(APPS), dirs=[d])
            for app in ('firefox', 'org.mozilla.firefox', 'org.gnome.Nautilus', 'foot', 'code', 'steam', 'chromium',
                        'org.kde.dolphin', 'hearth-settings', 'org.gnome.TextEditor', 'libreoffice-writer', 'discord', ''):
                self.assertFalse(owners.owns(app, now=0.0), app)
            self.assertTrue(owners.owns('hearth-mods', now=0.0))
            pad, out = self.make(owners=owners)
            pad.command({'cmd': 'focus', 'app': 'firefox', 'title': 'Firefox'})
            self.assertEqual(pad.owner, '')
            self.assertFalse(pad.owned)
            pad.button('a', True, 0.0)
            pad.button('a', False, 0.1)
            pad.button('lt', True, 0.2)
            pad.button('lt', False, 0.3)
            self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False), ('BTN_RIGHT', True), ('BTN_RIGHT', False)])

    def test_maximized_isnt_paused(self):
        pad, out = self.make()
        pad.command({'cmd': 'pause', 'on': False})   # Pad.qml: only fullscreen pauses (maximized is not fullscreen)
        pad.button('rt', True, 0.0)
        pad.button('rt', False, 0.1)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])


class QuickTaps(Base):
    """A press and its release read in the same batch (between two looks of the main loop) still click."""

    class E:
        EV_KEY, EV_ABS = 1, 3
        ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ, ABS_GAS, ABS_BRAKE, ABS_HAT0X, ABS_HAT0Y = 0, 1, 2, 3, 4, 5, 9, 10, 16, 17
        BTN_SOUTH = 304

    class Info:
        def __init__(self, lo, hi):
            self.min, self.max = lo, hi

    class Dev:
        def capabilities(self, absinfo=True):
            I = QuickTaps.Info
            return {3: [(0, I(0, 65535)), (1, I(0, 65535)), (2, I(0, 65535)), (5, I(0, 65535)), (9, I(0, 1023)), (10, I(0, 1023))]}

    class Ev:
        def __init__(self, t, c, v):
            self.type, self.code, self.value = t, c, v

    NAMES = ('a', 'b', 'x', 'y', 'lb', 'rb', 'lt', 'rt', 'view', 'menu', 'guide', 'l3', 'r3', 'up', 'down', 'left', 'right')

    def test_tap_in_one_batch(self):
        e = self.E
        c = hp.Controller(self.Dev(), e)
        bmap = {e.BTN_SOUTH: 'a'}
        pad, out = self.make()
        pressed = set()
        c.handle(self.Ev(e.EV_KEY, e.BTN_SOUTH, 1), bmap)
        c.handle(self.Ev(e.EV_KEY, e.BTN_SOUTH, 0), bmap)
        state, taps = hp.button_state([c], self.NAMES)
        self.assertEqual(state, set())
        self.assertEqual(taps, {'a'})
        pressed = hp.deliver(pad, state, pressed, taps, self.NAMES, 1.0)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])
        state, taps = hp.button_state([c], self.NAMES)   # looked at once: not again
        hp.deliver(pad, state, pressed, taps, self.NAMES, 1.01)
        self.assertEqual(len(out.presses()), 2)

    def test_trigger_tap_in_one_batch(self):
        e = self.E
        c = hp.Controller(self.Dev(), e)              # Xbox over Bluetooth: triggers on BRAKE / GAS
        pad, out = self.make()
        c.handle(self.Ev(e.EV_ABS, e.ABS_GAS, 1023), {})
        c.handle(self.Ev(e.EV_ABS, e.ABS_GAS, 0), {})
        state, taps = hp.button_state([c], self.NAMES)
        hp.deliver(pad, state, set(), taps, self.NAMES, 1.0)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])

    def test_held_across_looks_is_one_press(self):
        e = self.E
        c = hp.Controller(self.Dev(), e)
        bmap = {e.BTN_SOUTH: 'a'}
        pad, out = self.make()
        c.handle(self.Ev(e.EV_KEY, e.BTN_SOUTH, 1), bmap)
        state, taps = hp.button_state([c], self.NAMES)
        pressed = hp.deliver(pad, state, set(), taps, self.NAMES, 1.0)
        c.handle(self.Ev(e.EV_KEY, e.BTN_SOUTH, 0), bmap)
        state, taps = hp.button_state([c], self.NAMES)
        self.assertEqual(taps, set(), 'its press was already seen: a plain release')
        hp.deliver(pad, state, pressed, taps, self.NAMES, 1.2)
        self.assertEqual(out.presses(), [('BTN_LEFT', True), ('BTN_LEFT', False)])


if __name__ == '__main__':
    unittest.main()
