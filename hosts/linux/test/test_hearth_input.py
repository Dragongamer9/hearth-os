"""Tests for desktop/shell/hearth-input (Hearth Input: controller → keyboard and mouse for games).
Run: python3 -m unittest discover -s hosts/linux/test -q. No devices needed: evdev is faked."""
import sys
sys.dont_write_bytecode = True   # keep desktop/shell clean (it ships as-is)
import copy
import importlib.machinery
import importlib.util
import json
import math
import pathlib
import unittest

PATH = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell' / 'hearth-input'
loader = importlib.machinery.SourceFileLoader('hearth_input', str(PATH))
spec = importlib.util.spec_from_loader('hearth_input', loader)
hi = importlib.util.module_from_spec(spec)
loader.exec_module(hi)

TEMPLATES = {t['id']: t['layout'] for t in hi.TABLES['templates']}


class Out:
    def __init__(self):
        self.log = []          # (code, down) in order
        self.down = set()
        self.moved = [0, 0]
        self.wheeled = [0, 0]
        self.osds = []

    def key(self, code, down):
        self.log.append((code, down))
        (self.down.add if down else self.down.discard)(code)

    def move(self, dx, dy):
        self.moved[0] += dx
        self.moved[1] += dy

    def wheel(self, v, h):
        self.wheeled[0] += v
        self.wheeled[1] += h

    def osd(self, text):
        self.osds.append(text)


def engine(layout=None, prefs=None, **changes):
    lay = copy.deepcopy(layout or TEMPLATES['shooter'])
    lay.update(changes)
    out = Out()
    return hi.Engine(out, hi.validate_layout(lay), prefs), out


def run(e, t0, t1, step=0.01):
    t = t0
    while t < t1 - 1e-9:
        t = round(t + step, 6)
        e.tick(t, step)
    return t


# ---------------------------------------------------------------------- layouts

class Layouts(unittest.TestCase):
    def test_every_template_is_valid(self):
        self.assertEqual(set(TEMPLATES), {'shooter', 'pointclick', 'retro', 'mouse', 'rts', 'browser'})
        for tid, lay in TEMPLATES.items():
            v = hi.validate_layout(lay)
            self.assertEqual(v['base'], tid)
            self.assertEqual(set(v['buttons']), set(hi.BUTTONS))

    def test_shooter_matches_the_plan(self):
        v = hi.validate_layout(TEMPLATES['shooter'])
        b = v['buttons']
        self.assertEqual(b['a']['press'].raw, 'key:SPACE')
        self.assertEqual((b['b']['press'].raw, b['b']['hold'].raw), ('key:C', 'key:LEFTCTRL'))
        self.assertEqual(b['rt']['press'].raw, 'mouse:left')
        self.assertEqual(b['lt']['press'].raw, 'mouse:right')
        self.assertEqual(b['menu']['press'].raw, 'key:ESC')
        self.assertEqual((b['lb']['press'].raw, b['rb']['press'].raw), ('key:Q', 'key:E'))
        self.assertEqual(v['sticks']['ls']['mode'], 'keys')
        self.assertEqual([v['sticks']['ls'][d].raw for d in ('up', 'left', 'down', 'right')], ['key:W', 'key:A', 'key:S', 'key:D'])
        self.assertEqual(v['sticks']['rs']['mode'], 'mouse')

    def test_strategy_matches_the_plan(self):
        v = hi.validate_layout(TEMPLATES['rts'])
        self.assertEqual(v['name'], 'Strategy (RTS)')
        b, sb = v['buttons'], v['layers']['shift']['buttons']
        self.assertEqual(v['sticks']['rs']['mode'], 'mouse')
        self.assertEqual((b['rt']['press'].raw, b['lt']['press'].raw), ('mouse:left', 'mouse:right'))
        self.assertEqual([b[d]['press'].raw for d in ('up', 'down', 'left', 'right')], ['key:UP', 'key:DOWN', 'key:LEFT', 'key:RIGHT'])
        self.assertEqual((b['lb']['press'].kind, b['lb']['press'].arg), ('layer', 'shift'))
        self.assertEqual((b['menu']['press'].raw, b['a']['press'].raw, b['rb']['press'].raw), ('key:ESC', 'key:SPACE', 'key:LEFTCTRL'))
        self.assertEqual(b['view']['press'].kind, 'radial')
        self.assertEqual([sb[d]['press'].raw for d in ('up', 'right', 'down', 'left')], ['key:1', 'key:2', 'key:3', 'key:4'])
        self.assertEqual([sb[f]['press'].raw for f in ('y', 'b', 'a', 'x')], ['key:LEFTCTRL+1', 'key:LEFTCTRL+2', 'key:LEFTCTRL+3', 'key:LEFTCTRL+4'])
        self.assertEqual(len(v['radial']), 8)

    def test_strategy_control_groups_while_lb_is_held(self):
        e, out = engine(TEMPLATES['rts'])
        e.button('up', True, 0.0); e.button('up', False, 0.05)
        self.assertEqual(out.log[0], ('KEY_UP', True))       # D-pad scrolls the map
        out.log.clear()
        e.button('lb', True, 0.1)
        e.button('up', True, 0.15); e.button('up', False, 0.2)
        e.button('y', True, 0.25); e.button('y', False, 0.3)
        e.button('lb', False, 0.35)
        downs = [c for c, d in out.log if d]
        self.assertEqual(downs[0], 'KEY_1')                   # LB + D-pad up: select group 1
        self.assertIn('KEY_LEFTCTRL', downs)                  # LB + Y: Ctrl + 1 (make group 1)
        self.assertNotIn('KEY_UP', downs)

    def bad(self, change, word):
        lay = copy.deepcopy(TEMPLATES['shooter'])
        change(lay)
        with self.assertRaises(hi.LayoutError) as cm:
            hi.validate_layout(lay)
        self.assertIn(word, str(cm.exception))

    def test_strict_checks(self):
        self.bad(lambda l: l['buttons'].update(a='key:NOPE'), 'unknown key')
        self.bad(lambda l: l['buttons'].update(a='exec:rm -rf /'), 'unknown action')
        self.bad(lambda l: l['buttons'].update(a='key:'), 'unknown key')
        self.bad(lambda l: l['buttons'].update(a='key:A+B+C+D+E'), '1 to 4')
        self.bad(lambda l: l['buttons'].update(a='key:A+A'), 'same key')
        self.bad(lambda l: l['buttons'].update(guide='key:A'), 'unknown setting')
        self.bad(lambda l: l['buttons'].update(a={'press': 'key:A', 'command': 'x'}), 'unknown setting')
        self.bad(lambda l: l['buttons'].update(a={'press': 'key:A', 'threshold': 0.5}), 'unknown setting')
        self.bad(lambda l: l['buttons'].update(a={'press': 'key:A', 'hold': 'key:B', 'turbo': True}), 'turbo')
        self.bad(lambda l: l.update(version=1, buttons={'a': {'press': 'shift', 'hold': 'key:B'}}), 'tap and hold')
        self.bad(lambda l: l['buttons'].update(a={'press': 'layer:nope'}), "no layer")
        self.bad(lambda l: l['buttons'].update(rt={'press': 'mouse:left', 'threshold': 2}), 'between')
        self.bad(lambda l: l['buttons'].update(rt={'press': 'mouse:left', 'threshold': True}), 'number')
        self.bad(lambda l: l['sticks']['rs'].update(sensitivity=50), 'between')
        self.bad(lambda l: l['sticks']['rs'].update(sensitivity=float('nan')), 'number')
        self.bad(lambda l: l['sticks']['rs'].update(up='key:W'), 'unknown setting')
        self.bad(lambda l: l['sticks']['rs'].update(deadzone=0.5, outer=0.55), 'outer')
        self.bad(lambda l: l['sticks'].update(ls={'mode': 'teleport'}), 'mode')
        self.bad(lambda l: l['sticks']['ls'].update(up='radial'), "can't be used here")
        self.bad(lambda l: l.update(version=1, shift={'buttons': {'a': 'shift'}}), "can't be used here")
        self.bad(lambda l: l.update(shift={'buttons': {}}), 'unknown setting')
        self.bad(lambda l: l.update(radial=['key:A'] * 9), 'up to 8')
        self.bad(lambda l: l.update(radial=['gyro:toggle']), "can't be used here")
        self.bad(lambda l: l.update(name='x' * 61), 'name')
        self.bad(lambda l: l.update(name='bad\nname'), 'name')
        self.bad(lambda l: l.update(base='nope'), 'template')
        self.bad(lambda l: l.update(hold_ms=20), 'between')
        self.bad(lambda l: l.update(hold_ms=300.5), 'whole')
        self.bad(lambda l: l.update(version=3), 'version')
        self.bad(lambda l: l.update(script='x'), 'unknown setting')

    def test_shorthand_and_defaults(self):
        v = hi.validate_layout({'buttons': {'a': 'key:LEFTCTRL+C'}})
        self.assertEqual(v['buttons']['a']['press'].codes, ('KEY_LEFTCTRL', 'KEY_C'))
        self.assertEqual(v['buttons']['b']['press'].kind, 'none')
        self.assertEqual(v['sticks']['ls'], {'mode': 'none'})
        self.assertEqual(v['name'], 'Custom')
        self.assertEqual(hi.action_label('key:LEFTCTRL+C'), 'Ctrl + C')
        dumped = hi.dump_layout(hi.validate_layout(TEMPLATES['shooter']))
        self.assertEqual(hi.dump_layout(hi.validate_layout(dumped)), dumped)

    def test_prefs(self):
        self.assertEqual(hi.validate_prefs(None), {'sensitivity': 1.0, 'gyro': False})
        self.assertEqual(hi.validate_prefs({'sensitivity': 2, 'gyro': True}), {'sensitivity': 2.0, 'gyro': True})
        with self.assertRaises(hi.LayoutError):
            hi.validate_prefs({'sensitivity': 9})
        with self.assertRaises(hi.LayoutError):
            hi.validate_prefs({'run': 'x'})


# ---------------------------------------------------------------------- the engine

class Sticks(unittest.TestCase):
    def test_directions_and_thresholds(self):
        d = hi.stick_directions
        self.assertEqual(d(0.3, 0, 0.45), set())
        self.assertEqual(d(0.5, 0, 0.45), {'right'})
        self.assertEqual(d(-0.9, 0, 0.45), {'left'})
        self.assertEqual(d(0, -0.9, 0.45), {'up'})
        self.assertEqual(d(0.7, 0.7, 0.45), {'right', 'down'})
        self.assertEqual(d(-0.7, -0.7, 0.45), {'left', 'up'})
        # 8 equal sectors: 20° off straight right is still only right, 25° is a diagonal.
        import math
        a = math.radians(20)
        self.assertEqual(d(math.cos(a), math.sin(a), 0.45), {'right'})
        a = math.radians(25)
        self.assertEqual(d(math.cos(a), math.sin(a), 0.45), {'right', 'down'})
        # Hysteresis: held, it lets go only below 80% of the threshold.
        self.assertEqual(d(0.40, 0, 0.45), set())
        self.assertEqual(d(0.40, 0, 0.45, held={'right'}), {'right'})
        self.assertEqual(d(0.35, 0, 0.45, held={'right'}), set())

    def test_mouse_curve(self):
        c = hi.stick_curve
        self.assertEqual(c(0.05, 0.1, 0.95, 2), 0.0)
        self.assertEqual(c(0.1, 0.1, 0.95, 2), 0.0)
        self.assertEqual(c(0.95, 0.1, 0.95, 2), 1.0)
        self.assertEqual(c(1.0, 0.1, 0.95, 2), 1.0)
        self.assertAlmostEqual(c(0.525, 0.1, 0.95, 2), 0.25)
        self.assertAlmostEqual(c(0.525, 0.1, 0.95, 1), 0.5)
        self.assertLess(c(0.3, 0.1, 0.95, 2.5), c(0.3, 0.1, 0.95, 1.5))   # a higher curve is finer near the centre

    def test_wasd_from_the_left_stick(self):
        e, out = engine()
        e.set_axes(0.0, lx=0.0, ly=-0.9)
        e.tick(0.01, 0.01)
        self.assertEqual(out.down, {'KEY_W'})
        e.set_axes(0.02, lx=0.7, ly=-0.7)
        e.tick(0.02, 0.01)
        self.assertEqual(out.down, {'KEY_W', 'KEY_D'})
        e.set_axes(0.03, lx=0.0, ly=0.0)
        e.tick(0.03, 0.01)
        self.assertEqual(out.down, set())

    def test_mouse_from_the_right_stick(self):
        e, out = engine()
        e.set_axes(0.0, rx=1.0)
        run(e, 0.0, 1.0)
        self.assertAlmostEqual(out.moved[0], hi.MOUSE_SPEED, delta=5)
        self.assertEqual(out.moved[1], 0)
        e2, out2 = engine(prefs={'sensitivity': 0.5, 'gyro': False})
        e2.set_axes(0.0, rx=1.0)
        run(e2, 0.0, 1.0)
        self.assertAlmostEqual(out2.moved[0], hi.MOUSE_SPEED / 2, delta=5)
        e3, out3 = engine()
        e3.set_axes(0.0, rx=0.05, ry=0.05)     # inside the dead zone
        run(e3, 0.0, 1.0)
        self.assertEqual(out3.moved, [0, 0])

    def test_scroll_from_a_stick(self):
        e, out = engine(TEMPLATES['mouse'])
        e.set_axes(0.0, ly=-1.0)
        run(e, 0.0, 1.0)
        self.assertAlmostEqual(out.wheeled[0], hi.SCROLL_SPEED * 120, delta=130)
        self.assertGreater(out.wheeled[0], 0)   # stick up scrolls up


class Buttons(unittest.TestCase):
    def test_plain_and_combo(self):
        e, out = engine(buttons={'a': 'key:LEFTCTRL+C'})
        e.button('a', True, 0.0)
        self.assertEqual(out.log, [('KEY_LEFTCTRL', True), ('KEY_C', True)])
        e.button('a', False, 0.1)
        self.assertEqual(out.log[2:], [('KEY_C', False), ('KEY_LEFTCTRL', False)])

    def test_two_sources_share_a_key(self):
        e, out = engine(buttons={'a': 'key:SPACE', 'b': 'key:SPACE'})
        e.button('a', True, 0.0)
        e.button('b', True, 0.0)
        e.button('a', False, 0.1)
        self.assertIn('KEY_SPACE', out.down)     # B still holds it
        e.button('b', False, 0.2)
        self.assertNotIn('KEY_SPACE', out.down)
        self.assertEqual(out.log, [('KEY_SPACE', True), ('KEY_SPACE', False)])

    def test_tap_vs_hold(self):
        e, out = engine()      # shooter: B taps C, holds Ctrl (hold_ms 300)
        e.button('b', True, 0.0)
        run(e, 0.0, 0.2)
        self.assertEqual(out.log, [])            # nothing yet: could still become a hold
        e.button('b', False, 0.2)
        self.assertEqual(out.log, [('KEY_C', True)])
        run(e, 0.2, 0.3)
        self.assertEqual(out.log, [('KEY_C', True), ('KEY_C', False)])   # a short pulse
        out.log.clear()
        e.button('b', True, 1.0)
        run(e, 1.0, 1.29)
        self.assertEqual(out.log, [])
        run(e, 1.29, 1.32)
        self.assertEqual(out.log, [('KEY_LEFTCTRL', True)])
        run(e, 1.32, 2.0)
        self.assertEqual(out.down, {'KEY_LEFTCTRL'})
        e.button('b', False, 2.0)
        self.assertEqual(out.down, set())
        self.assertNotIn(('KEY_C', True), out.log)

    def test_mode_shift_layer(self):
        e, out = engine(version=1, buttons={'lb': 'shift', 'a': 'key:SPACE'},
                        shift={'buttons': {'a': 'key:J'}, 'sticks': {'ls': {'mode': 'keys', 'up': 'key:UP'}}})
        e.button('a', True, 0.0)
        e.button('a', False, 0.1)
        e.button('lb', True, 0.2)
        e.button('a', True, 0.3)
        self.assertIn('KEY_J', out.down)
        self.assertNotIn('KEY_SPACE', out.down)
        e.button('lb', False, 0.4)        # A keeps doing what it started with until let go
        self.assertIn('KEY_J', out.down)
        e.button('a', False, 0.5)
        self.assertEqual(out.down, set())
        # Sticks follow the layer while it's held.
        e.button('lb', True, 1.0)
        e.set_axes(1.0, ly=-1.0)
        e.tick(1.0, 0.01)
        self.assertEqual(out.down, {'KEY_UP'})
        e.button('lb', False, 1.1)
        e.tick(1.1, 0.01)
        self.assertEqual(out.down, {'KEY_W'})

    def test_turbo(self):
        e, out = engine(buttons={'x': {'press': 'key:F', 'turbo': True}}, turbo_hz=10)
        e.button('x', True, 0.0)
        run(e, 0.0, 1.0)
        presses = [x for x in out.log if x == ('KEY_F', True)]
        self.assertGreaterEqual(len(presses), 9)
        self.assertLessEqual(len(presses), 11)
        e.button('x', False, 1.0)
        self.assertNotIn('KEY_F', out.down)

    def test_triggers_threshold(self):
        e, out = engine()       # shooter: RT left click at 0.35
        e.set_axes(0.0, rt=0.3)
        self.assertNotIn('BTN_LEFT', out.down)
        e.set_axes(0.1, rt=0.4)
        self.assertIn('BTN_LEFT', out.down)
        e.set_axes(0.2, rt=0.32)            # hysteresis: still held
        self.assertIn('BTN_LEFT', out.down)
        e.set_axes(0.3, rt=0.2)
        self.assertNotIn('BTN_LEFT', out.down)

    def test_scroll_button_repeats(self):
        e, out = engine(TEMPLATES['pointclick'])
        e.button('up', True, 0.0)
        self.assertEqual(out.wheeled[0], 120)
        run(e, 0.0, 0.3)
        self.assertEqual(out.wheeled[0], 120)
        run(e, 0.3, 1.0)
        self.assertGreater(out.wheeled[0], 120 * 5)
        e.button('up', False, 1.0)
        w = out.wheeled[0]
        run(e, 1.0, 2.0)
        self.assertEqual(out.wheeled[0], w)

    def test_radial_menu(self):
        e, out = engine()      # shooter: R5 opens it, 8 items
        e.button('r5', True, 0.0)
        self.assertTrue(out.osds and out.osds[-1].startswith('Radial menu'))
        e.set_axes(0.05, rx=1.0, ry=0.0)    # right = slot 2
        e.tick(0.05, 0.01)
        self.assertEqual(out.moved, [0, 0])   # the right stick picks, it doesn't move the mouse
        e.button('r5', False, 0.1)
        self.assertEqual(out.log, [('KEY_7', True)])
        run(e, 0.1, 0.3)
        self.assertEqual(out.down, set())
        # The d-pad picks too, without pressing its own key.
        out.log.clear()
        e.button('r5', True, 1.0)
        e.button('down', True, 1.0)
        e.button('down', False, 1.05)
        e.button('r5', False, 1.1)
        self.assertEqual(out.log, [('KEY_M', True)])
        # Let go of the button with the stick still pushed: the pointer doesn't move until the stick is back in
        # the middle, then it moves as usual.
        out.moved = [0, 0]
        e.button('r5', True, 2.0)
        e.set_axes(2.05, rx=1.0, ry=0.0)
        e.tick(2.05, 0.01)
        e.button('r5', False, 2.1)
        run(e, 2.1, 2.4)
        self.assertEqual(out.moved, [0, 0])
        e.set_axes(2.4, rx=0.0, ry=0.0)
        e.tick(2.41, 0.01)
        e.set_axes(2.42, rx=1.0, ry=0.0)
        run(e, 2.42, 2.6)
        self.assertGreater(out.moved[0], 0)
        self.assertEqual(hi.radial_index(0, -1, 8), 0)
        self.assertEqual(hi.radial_index(-1, 0, 8), 6)
        self.assertEqual(hi.radial_index(0, 1, 3), 2 if round(3 / 2) == 2 else 1)

    def test_gyro(self):
        e, out = engine(buttons={'l4': 'gyro:toggle', 'r4': 'gyro:hold'})
        e.gyro(0.0, -10.0, 0.0)             # turning at 10°/s
        run(e, 0.0, 1.0)
        self.assertEqual(out.moved, [0, 0])  # off
        e.button('l4', True, 1.0)
        self.assertEqual(out.osds[-1], 'Gyro aiming on')
        run(e, 1.0, 2.0)
        self.assertAlmostEqual(out.moved[0], (10 - 1.0) * hi.GYRO_SCALE, delta=3)   # minus the dead zone
        e.button('l4', False, 2.0)
        e.button('l4', True, 2.1)            # toggled off
        x = out.moved[0]
        run(e, 2.1, 3.0)
        self.assertEqual(out.moved[0], x)
        e.button('r4', True, 3.0)            # held: on while held
        run(e, 3.0, 3.5)
        self.assertGreater(out.moved[0], x)
        # Started on from Quick Access.
        e2, out2 = engine(prefs={'sensitivity': 1.0, 'gyro': True})
        e2.gyro(0.5, -0.5, 0)               # inside the dead zone: no drift
        run(e2, 0, 1)
        self.assertEqual(out2.moved, [0, 0])

    def test_start_ignores_buttons_already_down_and_release_all(self):
        e, out = engine()
        e.start({'a'}, {'lt': 0.0, 'rt': 0.9})
        e.button('a', True, 0.0)            # the A that started the game: still down
        e.set_axes(0.0, rt=0.9)
        self.assertEqual(out.log, [])
        e.button('a', False, 0.1)
        e.set_axes(0.1, rt=0.0)
        self.assertEqual(out.log, [])
        e.button('a', True, 0.2)            # a new press works
        self.assertEqual(out.down, {'KEY_SPACE'})
        e.button('lb', True, 0.2)
        e.set_axes(0.2, lx=1.0, rt=1.0)
        e.tick(0.2, 0.01)
        self.assertEqual(out.down, {'KEY_SPACE', 'KEY_Q', 'KEY_D', 'BTN_LEFT'})
        e.button('b', True, 0.2)            # a pending tap/hold doesn't fire on release_all
        e.release_all()
        self.assertEqual(out.down, set())
        e.set_axes(0.2)
        run(e, 0.2, 1.0)
        self.assertEqual(out.down, set())
        self.assertNotIn(('KEY_C', True), out.log)


# ---------------------------------------------------------------------- version 2: sets, layers, activators

CASES = json.loads((pathlib.Path(__file__).resolve().parent / 'fixtures' / 'input-layout-cases.json').read_text())
OLD_RTS = {'version': 1, 'name': 'Strategy (RTS)', 'base': 'rts',
           'buttons': {'lb': 'shift', 'a': 'key:SPACE', 'rt': {'press': 'mouse:left', 'threshold': 0.3}},
           'shift': {'buttons': {'up': 'key:1', 'y': 'key:LEFTCTRL+1'}},
           'radial': ['key:5'], 'hold_ms': 300, 'turbo_hz': 10}


class Version2(unittest.TestCase):
    def test_shared_cases(self):
        """The same cases input_map.rs checks: both sides accept and refuse the same layouts."""
        for lay in CASES['good']:
            hi.validate_layout(copy.deepcopy(lay))
        for lay, word in CASES['bad']:
            with self.assertRaises(hi.LayoutError, msg=json.dumps(lay)) as cm:
                hi.validate_layout(copy.deepcopy(lay))
            self.assertIn(word, str(cm.exception), json.dumps(lay))

    def test_version_1_files_are_migrated(self):
        v = hi.validate_layout(copy.deepcopy(OLD_RTS))
        self.assertEqual(v['layer_order'], ['shift'])
        self.assertEqual(v['buttons']['lb']['press'].raw, 'layer:shift')
        self.assertEqual(v['layers']['shift']['buttons']['up']['press'].raw, 'key:1')
        d = hi.dump_layout(v)
        self.assertEqual(d['version'], 2)
        self.assertEqual(d['layers'][0]['id'], 'shift')
        self.assertNotIn('shift', d)
        self.assertEqual(hi.dump_layout(hi.validate_layout(d)), d)
        # A v1 file still behaves the same: LB held, D-pad up is 1.
        e, out = engine(OLD_RTS)
        e.button('lb', True, 0); e.button('up', True, 0.1)
        self.assertEqual(out.down, {'KEY_1'})

    def test_everything_round_trips(self):
        lay = CASES['good'][3]
        d = hi.dump_layout(hi.validate_layout(copy.deepcopy(lay)))
        self.assertEqual(hi.dump_layout(hi.validate_layout(d)), d)
        self.assertEqual(d['buttons']['a']['chords'][1], {'with': 'rb', 'do': 'mouse:left', 'turbo': True})
        self.assertEqual(d['buttons']['y'], {'double': 'key:Y'})
        self.assertTrue(hi.uses_gyro(hi.validate_layout(copy.deepcopy(lay))))

    def test_double_press(self):
        e, out = engine(buttons={'a': {'press': 'key:A', 'double': 'key:D'}}, double_ms=250)
        e.button('a', True, 0.0); e.button('a', False, 0.05)
        run(e, 0.05, 0.2)
        self.assertEqual(out.log, [])                       # might still be a double
        run(e, 0.2, 0.4)
        self.assertEqual(out.log[0], ('KEY_A', True))       # it wasn't: a single tap
        out.log.clear()
        e.button('a', True, 1.0); e.button('a', False, 1.05)
        e.button('a', True, 1.15)
        self.assertEqual(out.down, {'KEY_D'})               # the second press holds the double
        run(e, 1.15, 1.6)
        self.assertNotIn(('KEY_A', True), out.log)
        e.button('a', False, 1.6)
        self.assertEqual(out.down, set())
        # Held past the window: the plain press, held.
        e.button('a', True, 3.0)
        run(e, 3.0, 3.4)
        self.assertEqual(out.down, {'KEY_A'})

    def test_chord_release_toggle(self):
        e, out = engine(buttons={'lb': 'key:Q', 'a': {'press': 'key:A', 'release': 'key:R', 'chords': [{'with': 'lb', 'do': 'key:C'}]},
                                 'b': {'press': 'key:B', 'toggle': True}})
        e.button('lb', True, 0.0)
        e.button('a', True, 0.1)
        self.assertEqual(out.down, {'KEY_Q', 'KEY_C'})      # LB held: A's chord, not A
        e.button('a', False, 0.2); e.button('lb', False, 0.2)
        run(e, 0.2, 0.4)
        self.assertNotIn(('KEY_R', True), out.log)          # no release press after a chord
        e.button('a', True, 1.0); e.button('a', False, 1.1)
        self.assertIn(('KEY_R', True), out.log)
        e.button('b', True, 2.0); e.button('b', False, 2.1)
        self.assertIn('KEY_B', out.down)                    # toggled on, stays down
        e.button('b', True, 3.0); e.button('b', False, 3.1)
        self.assertNotIn('KEY_B', out.down)

    def test_long_press_turbo(self):
        e, out = engine(buttons={'x': {'press': 'key:F', 'hold': 'key:G', 'hold_turbo': True}}, turbo_hz=10, hold_ms=200)
        e.button('x', True, 0.0)
        run(e, 0.0, 1.2)
        self.assertGreaterEqual(len([x for x in out.log if x == ('KEY_G', True)]), 8)
        e.button('x', False, 1.2)
        self.assertEqual(out.down, set())

    def test_sets_and_layers(self):
        lay = copy.deepcopy(CASES['good'][3])
        e, out = engine(lay)
        e.button('view', True, 0.0); e.button('view', False, 0.1)     # to the Driving set
        self.assertEqual(out.osds[-1], 'Action set: Driving')
        e.set_axes(0.2, rt=1.0)
        self.assertEqual(out.down, {'KEY_W'})
        e.set_axes(0.3, rt=0.0)
        e.button('view', True, 0.4); e.button('view', False, 0.5)     # and back
        self.assertEqual(e.set_id, 'default')
        e.button('rb', True, 1.0); e.button('rb', False, 1.1)         # Menus layer on
        e.button('a', True, 1.2)
        self.assertEqual(out.down, {'KEY_ENTER'})
        e.button('a', False, 1.3)
        e.button('rb', True, 1.4); e.button('rb', False, 1.5)         # ... off again (the layer's own RB)
        self.assertEqual(e.active_layers(), [])
        e.button('lb', True, 2.0)                                     # Aiming held: its A switches the set
        self.assertEqual(e.active_layers(), ['aim'])
        self.assertEqual(e.stick_binding('rs')['mode'], 'mouse')
        e.button('lb', False, 2.1)
        self.assertEqual(e.stick_binding('rs')['mode'], 'flick')

    def test_dpad_wheel_flick(self):
        e, out = engine(sticks={'ls': {'mode': 'dpad', 'up': 'key:UP', 'right': 'key:RIGHT', 'down': 'key:DOWN', 'left': 'key:LEFT'},
                                'rs': {'mode': 'wheel', 'threshold': 0.5}})
        e.set_axes(0.0, lx=0.7, ly=-0.6)                    # a diagonal: only the stronger way
        e.tick(0.0, 0.01)
        self.assertEqual(out.down, {'KEY_RIGHT'})
        # A quarter turn clockwise: a quarter of WHEEL_NOTCHES notches down.
        for i in range(10):
            a = math.pi / 2 * i / 9
            e.set_axes(0.1 + i * 0.01, rx=math.sin(a), ry=-math.cos(a))
            e.tick(0.1 + i * 0.01, 0.01)
        self.assertAlmostEqual(out.wheeled[0], -hi.WHEEL_NOTCHES * 120 / 4, delta=2)
        f, fo = engine(sticks={'rs': {'mode': 'flick', 'turn': 4000, 'threshold': 0.7}, 'ls': {'mode': 'none'}})
        f.set_axes(0.0, rx=1.0)                             # flicked right: a quarter turn, over FLICK_TIME
        t = run(f, 0.0, 0.3)
        self.assertAlmostEqual(fo.moved[0], 1000, delta=2)
        for i in range(1, 10):                              # then round to straight down: another quarter
            a = math.pi / 2 + math.pi / 2 * i / 9
            f.set_axes(t, rx=math.sin(a), ry=-math.cos(a))
            t = run(f, t, t + 0.01)
        self.assertAlmostEqual(fo.moved[0], 2000, delta=3)
        f.set_axes(t)
        run(f, t, t + 0.1)
        f.set_axes(t + 0.1, rx=-1.0)                        # flick left from the centre: back a quarter
        run(f, t + 0.1, t + 0.5)
        self.assertAlmostEqual(fo.moved[0], 1000, delta=4)

    def test_gyro_modes(self):
        e, out = engine(gyro={'when': 'always', 'deadzone': 0}, buttons={'l4': 'gyro:ratchet'})
        e.gyro(0, -10, 0)
        run(e, 0, 0.5)
        moved = out.moved[0]
        self.assertGreater(moved, 0)
        e.button('l4', True, 0.5)                           # ratchet: off while held
        run(e, 0.5, 1.0)
        self.assertEqual(out.moved[0], moved)
        s, so = engine(gyro={'when': 'stick', 'deadzone': 0, 'axis': 'roll'}, sticks={'rs': {'mode': 'none'}, 'ls': {'mode': 'none'}})
        s.gyro(0, 0, -10)
        run(s, 0, 0.5)
        self.assertEqual(so.moved, [0, 0])                  # not touching the stick
        s.set_axes(0.5, rx=0.3)
        run(s, 0.5, 1.0)
        self.assertGreater(so.moved[0], 0)                  # touching: roll turns


class Typing(unittest.TestCase):
    def test_char_keys(self):
        self.assertEqual(hi.char_keys('a'), ['KEY_A'])
        self.assertEqual(hi.char_keys('A'), ['KEY_LEFTSHIFT', 'KEY_A'])
        self.assertEqual(hi.char_keys('?'), ['KEY_LEFTSHIFT', 'KEY_SLASH'])
        self.assertEqual(hi.char_keys(' '), ['KEY_SPACE'])
        self.assertIsNone(hi.char_keys('é'))
        self.assertEqual(hi.char_keys('z', 'de'), ['KEY_Y'])
        self.assertEqual(hi.char_keys('Z', 'de'), ['KEY_LEFTSHIFT', 'KEY_Y'])
        self.assertEqual(hi.char_keys('ü', 'de'), ['KEY_LEFTBRACE'])
        self.assertEqual(hi.char_keys('@', 'de'), ['KEY_RIGHTALT', 'KEY_Q'])
        self.assertEqual(hi.char_keys('a', 'fr'), ['KEY_Q'])
        self.assertEqual(hi.char_keys('1', 'fr'), ['KEY_LEFTSHIFT', 'KEY_1'])
        self.assertEqual(hi.char_keys('é', 'fr'), ['KEY_2'])
        self.assertIsNone(hi.char_keys('É', 'fr'))
        self.assertEqual(hi.char_keys('Å', 'fi'), ['KEY_LEFTSHIFT', 'KEY_LEFTBRACE'])
        self.assertEqual(hi.keymap_layout('de(nodeadkeys),us')[0], 'de')
        self.assertEqual(hi.keymap_layout('../etc')[0], 'us')
        self.assertIsNone(hi.char_keys('?', 'xx') and None)
        codes = hi.keymap_codes()
        self.assertIn('KEY_102ND', codes)
        self.assertIn('KEY_RIGHTALT', codes)


# ---------------------------------------------------------------------- when the controller is taken

class Grab(unittest.TestCase):
    def state(self, p, now, **kw):
        m = {'cmd': 'state', 'game': 'steam:1', 'front': True, 'overlay': False, 'layout': TEMPLATES['shooter'], 'prefs': None}
        m.update(kw)
        p.state(m, now)

    def test_policy(self):
        p = hi.Policy()
        self.assertIsNone(p.want(0))                       # nothing heard from Hearth
        self.state(p, 0)
        lay = p.want(0)
        self.assertEqual(lay['name'], 'Shooter / action')
        self.state(p, 1, prefs={'sensitivity': 2.0, 'gyro': True})
        self.assertIs(p.want(1), lay)                      # same layout: same object (no restart), new prefs
        self.assertEqual(p.prefs, {'sensitivity': 2.0, 'gyro': True})
        self.state(p, 2, front=False)
        self.assertIsNone(p.want(2))                       # game not in front
        self.state(p, 3, overlay=True)
        self.assertIsNone(p.want(3))                       # Quick Access open
        self.state(p, 4, layout=None)
        self.assertIsNone(p.want(4))                       # layout Off
        self.state(p, 5, game=None)
        self.assertIsNone(p.want(5))
        self.state(p, 6)
        self.assertIsNotNone(p.want(6))
        self.assertIsNone(p.want(6 + hi.HEARTBEAT + 0.1))  # Hearth went quiet
        self.state(p, 20)
        p.guide(20)
        self.assertIsNone(p.want(20.5))                    # Guide: back to Hearth for a moment
        self.assertIsNotNone(p.want(20 + hi.GUIDE_SUSPEND + 0.1))

    def test_bad_layout_from_hearth_is_refused(self):
        p = hi.Policy()
        lay = copy.deepcopy(TEMPLATES['shooter'])
        lay['buttons']['a'] = 'key:ROOT'
        self.state(p, 0, layout=lay)
        self.assertIsNone(p.want(0))
        self.assertIn('unknown key', p.error)

    def test_test_mode(self):
        p = hi.Policy()
        self.state(p, 0, front=False, game=None, layout=None)
        p.start_test(TEMPLATES['retro'], 10, 0)
        self.assertEqual(p.want(1)['name'], 'Retro / arrow keys')   # works with Hearth in front
        self.assertIsNone(p.want(11))
        p.start_test(TEMPLATES['retro'], 999, 20)
        self.assertEqual(p.test[1], 20 + hi.TEST_MAX)
        self.state(p, 21, front=False, game=None, layout=None, overlay=True)
        self.assertIsNone(p.want(21))                               # Quick Access still wins
        with self.assertRaises(hi.LayoutError):
            p.start_test({'buttons': {'a': 'key:NOPE'}}, 5, 0)


# ---------------------------------------------------------------------- the daemon, with fake evdev

class E:
    EV_KEY, EV_REL, EV_ABS = 1, 2, 3
    BTN_SOUTH, BTN_EAST, BTN_NORTH, BTN_WEST = 0x130, 0x131, 0x133, 0x134
    BTN_TL, BTN_TR, BTN_TL2, BTN_TR2 = 0x136, 0x137, 0x138, 0x139
    BTN_SELECT, BTN_START, BTN_MODE, BTN_THUMBL, BTN_THUMBR = 0x13a, 0x13b, 0x13c, 0x13d, 0x13e
    BTN_DPAD_UP, BTN_DPAD_DOWN, BTN_DPAD_LEFT, BTN_DPAD_RIGHT = 0x220, 0x221, 0x222, 0x223
    BTN_TRIGGER_HAPPY1 = 0x2c0
    BTN_LEFT, BTN_RIGHT, BTN_MIDDLE, BTN_SIDE, BTN_EXTRA = 0x110, 0x111, 0x112, 0x113, 0x114
    ABS_X, ABS_Y, ABS_Z, ABS_RX, ABS_RY, ABS_RZ, ABS_GAS, ABS_BRAKE, ABS_HAT0X, ABS_HAT0Y = 0, 1, 2, 3, 4, 5, 9, 10, 16, 17
    REL_X, REL_Y, REL_HWHEEL, REL_WHEEL = 0, 1, 6, 8

    def __getattr__(self, name):
        if name.startswith('BTN_TRIGGER_HAPPY'):
            return 0x2c0 + int(name[17:]) - 1
        if name.startswith('KEY_'):
            return 1000 + abs(hash(name)) % 1000
        raise AttributeError(name)


class Abs:
    def __init__(self, lo, hi_):
        self.min, self.max, self.resolution = lo, hi_, 0


class Info:
    vendor = 0x045e


class Ev:
    def __init__(self, type_, code, value):
        self.type, self.code, self.value = type_, code, value


class Dev:
    def __init__(self, name='Microsoft X-Box 360 pad', busy=False):
        self.name, self.busy, self.grabbed, self.info, self.fd = name, busy, False, Info(), 7
        self.queue = []

    def capabilities(self, absinfo=True):
        e = E()
        return {e.EV_KEY: [e.BTN_SOUTH, e.BTN_MODE], e.EV_ABS: [(c, Abs(-32768, 32767)) for c in (0, 1, 3, 4)] + [(2, Abs(0, 255)), (5, Abs(0, 255))]}

    def grab(self):
        if self.busy:
            raise OSError(16, 'Device or resource busy')
        self.grabbed = True

    def ungrab(self):
        self.grabbed = False

    def read(self):
        q, self.queue = self.queue, []
        return q

    def close(self):
        pass


class FakeEvdev:
    ecodes = E()

    class UInput:
        def __init__(self, caps, name):
            self.name = name
            self.events = []

        def write(self, t, c, v):
            self.events.append((t, c, v))

        def syn(self):
            pass

        def close(self):
            pass


class Peer:
    def __init__(self):
        self.subscribed = False
        self.got = []

    def send(self, obj):
        self.got.append(obj)
        return True


class Daemon(unittest.TestCase):
    def make(self, *devs):
        d = hi.Daemon(FakeEvdev)
        for i, dev in enumerate(devs or [Dev()]):
            d.pads['/dev/input/event%d' % i] = hi.Pad(dev, d.e)
        return d

    def state(self, d, peer, now, **kw):
        m = {'cmd': 'state', 'game': 'steam:1', 'front': True, 'overlay': False, 'layout': TEMPLATES['shooter']}
        m.update(kw)
        self.assertIsNone(d.command(peer, m, now))

    def test_grab_follows_the_game(self):
        dev = Dev()
        d, peer = self.make(dev), Peer()
        self.state(d, peer, 0)
        self.assertTrue(dev.grabbed)
        self.assertIsNotNone(d.engine)
        self.assertTrue(peer.subscribed)
        self.state(d, peer, 1, front=False)
        self.assertFalse(dev.grabbed)
        self.assertIsNone(d.engine)
        self.state(d, peer, 2)
        self.assertTrue(dev.grabbed)
        d.apply(2 + hi.HEARTBEAT + 1)          # Hearth stopped talking
        self.assertFalse(dev.grabbed)

    def test_prefs_update_without_restart(self):
        d, peer = self.make(), Peer()
        self.state(d, peer, 0)
        eng = d.engine
        self.state(d, peer, 1, prefs={'sensitivity': 2.0, 'gyro': True})
        self.assertIs(d.engine, eng)
        self.assertEqual(eng.prefs['sensitivity'], 2.0)
        self.assertTrue(eng.gyro_on)

    def test_overlay_lets_go_before_answering(self):
        dev = Dev()
        d, peer = self.make(dev), Peer()
        self.state(d, peer, 0)
        d.engine.button('a', True, 0)           # Space held when Quick Access opens
        kbd = d.out.kbd.events
        r = d.command(Peer(), {'cmd': 'overlay', 'on': True}, 0.1)
        self.assertEqual(r, {'ok': True, 'grabbed': False})
        self.assertFalse(dev.grabbed)
        self.assertEqual(kbd[-1][2], 0)          # ... and let go
        r = d.command(Peer(), {'cmd': 'overlay', 'on': False}, 0.2)
        self.assertTrue(r['grabbed'])
        self.assertTrue(dev.grabbed)

    def test_someone_elses_controller_is_left_alone(self):
        mine, hhds = Dev(), Dev('Asus Keyboard', busy=True)
        d, peer = self.make(mine, hhds), Peer()
        self.state(d, peer, 0)
        self.assertTrue(mine.grabbed)
        p = d.pads['/dev/input/event1']
        self.assertTrue(p.busy)
        self.assertFalse(p.grabbed)
        st = d.status(0)
        self.assertEqual([x['busy'] for x in st['devices']], [False, True])
        self.assertTrue(st['active'])

    def test_guide_goes_to_hearth(self):
        dev = Dev()
        d, peer = self.make(dev), Peer()
        d.clients.append(peer)
        self.state(d, peer, 0)
        d.on_guide(0.5)
        self.assertFalse(dev.grabbed)
        self.assertEqual(peer.got[-1], {'t': 'guide'})
        d.apply(0.5 + hi.GUIDE_SUSPEND + 0.1)    # Quick Access didn't open: the layout comes back
        self.assertTrue(dev.grabbed)

    def test_guide_x_opens_the_keyboard(self):
        dev = Dev()
        d, peer = self.make(dev), Peer()
        d.clients.append(peer)
        self.state(d, peer, 0)
        pad = d.pads['/dev/input/event0']
        d.engine.button('a', True, 0)                 # Space held when Guide goes down: let go at once
        pad.handle(Ev(E.EV_KEY, E.BTN_MODE, 1))
        d.guide_step(0.1, True, False)
        self.assertTrue(dev.grabbed)                  # still held: X may come
        self.assertEqual(d.out.kbd.events[-1][2], 0)
        self.assertEqual(peer.got, [])
        d.guide_step(0.2, False, True)                # X while Guide is held
        self.assertEqual(peer.got[-1], {'t': 'osk'})
        self.assertFalse(dev.grabbed)
        # Guide alone: Quick Access as before, once it's let go (or held GUIDE_CHORD).
        d.apply(0.2 + hi.GUIDE_SUSPEND + 0.1)
        self.assertTrue(dev.grabbed)
        d.guide_step(5.0, True, False)
        pad.handle(Ev(E.EV_KEY, E.BTN_MODE, 0))
        d.guide_step(5.05, False, False)
        self.assertEqual(peer.got[-1], {'t': 'guide'})
        self.assertFalse(dev.grabbed)
        d.apply(10)
        pad.handle(Ev(E.EV_KEY, E.BTN_MODE, 1))
        d.guide_step(10.0, True, False)
        d.guide_step(10.0 + hi.GUIDE_CHORD, False, False)
        self.assertEqual(peer.got[-1], {'t': 'guide'})

    def test_typing_for_the_keyboard(self):
        d, peer = self.make(), Peer()
        r = d.command(peer, {'cmd': 'type', 'text': 'Hi!é', 'layout': 'us'}, 0)
        self.assertEqual(r, {'ok': True, 'layout': 'us', 'skipped': 'é'})
        d.type_tick(5)
        e = E()
        downs = [c for t, c, v in d.out.kbd.events if v == 1]
        self.assertEqual(downs, [e.KEY_LEFTSHIFT, e.KEY_H, e.KEY_I, e.KEY_LEFTSHIFT, e.KEY_1])
        self.assertEqual(len(d.out.kbd.events), 10)
        self.assertEqual(d.command(peer, {'cmd': 'key', 'key': 'bksp', 'n': 3}, 6), {'ok': True})
        d.type_tick(10)
        self.assertEqual(len([1 for t, c, v in d.out.kbd.events if c == e.KEY_BACKSPACE and v == 1]), 3)
        self.assertFalse(d.command(peer, {'cmd': 'key', 'key': 'rm -rf'}, 11)['ok'])
        self.assertFalse(d.command(peer, {'cmd': 'type', 'text': 'x' * (hi.TYPE_MAX + 1)}, 11)['ok'])
        self.assertFalse(d.command(peer, {'cmd': 'type', 'text': 'x', 'mods': ['super']}, 11)['ok'])
        # A display name that isn't one is ignored (never passed to xdotool).
        r = d.command(peer, {'cmd': 'type', 'text': 'é', 'display': ':0; reboot'}, 12)
        self.assertEqual(r['skipped'], 'é')

    def test_feed_and_held_buttons(self):
        dev = Dev()
        d, peer = self.make(dev), Peer()
        pad = d.pads['/dev/input/event0']
        pad.handle(Ev(E.EV_KEY, E.BTN_SOUTH, 1))   # A held while the game comes to the front
        self.state(d, peer, 0)
        d.feed(0.01, None)
        self.assertEqual(d.out.kbd.events, [])
        pad.handle(Ev(E.EV_KEY, E.BTN_SOUTH, 0))
        d.feed(0.02, None)
        pad.handle(Ev(E.EV_KEY, E.BTN_SOUTH, 1))
        d.feed(0.03, None)
        self.assertEqual(len(d.out.kbd.events), 1)
        pad.handle(Ev(E.EV_ABS, E.ABS_RX, 32767))
        for i in range(10):
            d.feed(0.04 + i * 0.01, None)
        self.assertTrue(any(c == E.REL_X and v > 0 for t, c, v in d.out.ptr.events))
        msgs = []
        d.out.osd_cb = msgs.append
        d.out.osd('x')
        self.assertEqual(msgs, ['x'])

    def test_test_command_and_status(self):
        d, peer = self.make(), Peer()
        r = d.command(peer, {'cmd': 'test', 'layout': TEMPLATES['retro'], 'seconds': 15}, 0)
        self.assertEqual(r, {'ok': True, 'seconds': 15})
        self.assertEqual(d.status(1)['layout'], 'Retro / arrow keys')
        self.assertEqual(d.status(1)['test'], 14)
        r = d.command(peer, {'cmd': 'test', 'layout': {'buttons': {'a': 'key:NOPE'}}}, 1)
        self.assertFalse(r['ok'])
        self.assertEqual(d.command(peer, {'cmd': 'test', 'stop': True}, 2), {'ok': True})
        self.assertIsNone(d.engine)
        self.assertFalse(d.command(peer, {'cmd': 'run', 'args': 'x'}, 3)['ok'])

    def test_sony_pads_swap_x_and_y(self):
        e = E()
        self.assertEqual(hi.device_buttons(e)[e.BTN_NORTH], 'x')
        m = hi.device_buttons(e, hi.SONY)
        self.assertEqual((m[e.BTN_NORTH], m[e.BTN_WEST]), ('y', 'x'))
        self.assertEqual(hi.device_buttons(e)[e.BTN_TRIGGER_HAPPY5], 'r4')
        # An Xbox controller over Bluetooth without xpadneo: its Xbox button is KEY_HOMEPAGE.
        self.assertEqual(hi.device_buttons(e)[e.KEY_HOMEPAGE], 'guide')

    def test_right_stick_on_z_without_rx(self):
        # hid-generic (an Xbox controller over Bluetooth without xpadneo): right stick Z/RZ, triggers BRAKE/GAS.
        class Bt(Dev):
            def capabilities(self, absinfo=True):
                return {E.EV_KEY: [E.BTN_SOUTH], E.EV_ABS: [(c, Abs(0, 65535)) for c in (0, 1, 2, 5)] + [(9, Abs(0, 1023)), (10, Abs(0, 1023))]}
        p = hi.Pad(Bt(), E())
        p.handle(Ev(E.EV_ABS, E.ABS_Z, 65535))
        p.handle(Ev(E.EV_ABS, E.ABS_RZ, 32768))
        self.assertEqual((p.ax['rx'], round(p.ax['ry'], 3), p.trigger('lt'), p.trigger('rt')), (1.0, 0.0, 0.0, 0.0))
        p.handle(Ev(E.EV_ABS, E.ABS_BRAKE, 1023))
        self.assertEqual(p.trigger('lt'), 1.0)
        x = hi.Pad(Dev(), E())                    # xpad: Z is the left trigger
        x.handle(Ev(E.EV_ABS, E.ABS_Z, 255))
        x.handle(Ev(E.EV_ABS, E.ABS_RX, 32767))
        self.assertEqual((x.trigger('lt'), x.ax['rx']), (1.0, 1.0))

    def test_a_controller_on_a_reused_node_is_found(self):
        class Kbd(Dev):
            def capabilities(self, absinfo=True):
                return {E.EV_KEY: [E().KEY_A]}

            def input_props(self):
                return []
        path = '/dev/input/event9'
        nodes, ids = {path: Kbd('Some Keyboard')}, {path: 'input1'}

        class Evdev(FakeEvdev):
            @staticmethod
            def list_devices():
                return list(nodes)

            @staticmethod
            def InputDevice(p):
                return nodes[p]
        old = hi.node_id
        hi.node_id = lambda p: ids.get(p)
        try:
            d = hi.Daemon(Evdev)
            d.scan()
            self.assertIn(path, d.ignored_paths)
            nodes[path], ids[path] = Dev(), 'input2'   # the keyboard went, a controller got its node number
            d.scan()
            self.assertIn(path, d.pads)
            self.assertNotIn(path, d.ignored_paths)
        finally:
            hi.node_id = old

    def test_motion_detection(self):
        e = E()
        self.assertTrue(hi.is_motion([6], {e.EV_ABS: [0, 1, 2, 3, 4, 5]}, e))
        self.assertFalse(hi.is_motion([], {e.EV_ABS: [0, 1, 2, 3, 4, 5]}, e))
        self.assertFalse(hi.is_motion([6], {e.EV_ABS: [0, 1, 2]}, e))

        class M:
            def __init__(self, n):
                self.name = n

        class P:
            name = 'Sony Interactive Entertainment DualSense Wireless Controller'
        ms = [M('Some laptop accelerometer'), M('Sony Interactive Entertainment DualSense Wireless Controller Motion Sensors')]
        self.assertIs(hi.pick_motion(ms, [P()]), ms[1])
        self.assertIsNone(hi.pick_motion([], [P()]))


class Socket(unittest.TestCase):
    def test_user_only_socket_and_a_request(self):
        import os
        import socket
        import stat
        import tempfile
        with tempfile.TemporaryDirectory() as tmp:
            old = hi.runtime_dir
            hi.runtime_dir = lambda: tmp
            try:
                d = hi.Daemon(FakeEvdev)
                self.assertTrue(d.listen())
                path = hi.socket_path()
                self.assertEqual(stat.S_IMODE(os.stat(path).st_mode), 0o600)
                self.assertEqual(stat.S_IMODE(os.stat(os.path.dirname(path)).st_mode), 0o700)
                c = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
                c.settimeout(2)
                c.connect(path)
                d.accept()
                self.assertEqual(len(d.clients), 1)
                c.sendall(b'{"cmd": "status"}\nnot json\n')
                import time
                time.sleep(0.05)
                d.read_client(d.clients[0], 0)
                lines = c.recv(65536).decode().strip().split('\n')
                self.assertTrue(json.loads(lines[0])['ok'])
                self.assertEqual(json.loads(lines[1]), {'ok': False, 'error': 'not JSON'})
                c.close()
                d.shutdown()
                self.assertFalse(os.path.exists(path))
            finally:
                hi.runtime_dir = old


if __name__ == '__main__':
    unittest.main()
