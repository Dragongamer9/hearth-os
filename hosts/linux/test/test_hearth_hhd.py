"""Tests for desktop/shell/hearth-hhd (Handheld Daemon's API: state, settings tree, the extra controls Hearth may
change) and the Handheld Daemon parts of hearth-controller and hearth-power, with a made-up HHD (its state and
settings as /api/v1/state and /api/v1/settings answer them, shortened)."""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock

SHELL = pathlib.Path(__file__).resolve().parent.parent / 'desktop' / 'shell'


def load(name):
    loader = importlib.machinery.SourceFileLoader(name.replace('-', '_'), str(SHELL / name))
    spec = importlib.util.spec_from_loader(name.replace('-', '_'), loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


hh = load('hearth-hhd')
hc = load('hearth-controller')
hp = load('hearth-power')

STATE = {
    'controllers': {'rog_ally': {
        'controller_mode': {'mode': 'uinput', 'uinput': {'paddles_as': 'noob', 'nintendo_qam': False},
                            'dualsense': {'paddles_as': 'steam_input', 'led_support': False}},
        'imu': True, 'swap_xbox': False, 'nintendo_mode': False, 'imu_hz': 200}},
    'hhd': {'steamos': {'tdp_status': 'enabled', 'tdp_min': 5, 'tdp_max': 30, 'tdp_default': 15}, 'version': 'x'},
    'tdp': {'unified': {'tdp': {'mode': 'custom', 'custom': {'tdp': 17}}, 'fan_full_speed': False}},
    'rgb': {'handheld': {'mode': {'mode': 'solid', 'solid': {'brightness': 'high'}}}},
    'info': {'ac': True},
}
SETTINGS = {
    'controllers': {'rog_ally': {'type': 'container', 'title': 'Ally Controller', 'children': {
        'controller_mode': {'type': 'mode', 'modes': {
            'uinput': {'type': 'container', 'title': 'Xbox', 'children': {
                'paddles_as': {'type': 'multiple', 'options': {'steam_input': 'Steam Input (Elite)', 'noob': 'Keyboard/Overlay', 'disabled': 'Disabled'}},
                'nintendo_qam': {'type': 'bool'}}},
            'dualsense': {'type': 'container', 'title': 'Dualsense', 'children': {
                'paddles_as': {'type': 'multiple', 'options': {'steam_input': 'Steam Input (Edge)', 'touchpad': 'Touchpad Clicks'}},
                'led_support': {'type': 'bool'}}},
            'hori_steam': {'type': 'container', 'title': 'Steam Controller', 'children': {}}}},
        'imu': {'type': 'bool', 'title': 'Motion Support'},
        'imu_hz': {'type': 'discrete', 'options': [100, 200]},
        'swap_xbox': {'type': 'bool'},
        'nintendo_mode': {'type': 'bool'}}}},
    'tdp': {'unified': {'type': 'container', 'children': {
        'tdp': {'type': 'mode', 'modes': {'balanced': {'title': 'Balanced'}, 'custom': {'title': 'Custom'}, 'quiet': {'title': 'Quiet'}, 'performance': {'title': 'Performance'}}},
        'fan_full_speed': {'type': 'bool'}}}},
    'rgb': {'handheld': {'type': 'container', 'children': {'mode': {'type': 'mode', 'modes': {
        'disabled': {'title': 'Off'}, 'solid': {'title': 'Solid', 'children': {'brightness': {'type': 'multiple', 'options': {'low': 'Low', 'high': 'High'}}}},
        'rainbow': {'title': 'Rainbow'}}}}}},
}


class Api(unittest.TestCase):
    def test_unroll(self):
        u = hh.unroll({'a': {'b': 1, 'c': {'d': 'x'}}, 'e': [1]})
        self.assertEqual(u, {'a.b': 1, 'a.c.d': 'x', 'e': [1]})

    def test_controllers(self):
        c = hh.controllers(STATE, SETTINGS)
        self.assertEqual(len(c), 1)
        self.assertEqual(c[0]['key'], 'rog_ally')
        self.assertEqual(c[0]['name'], 'Ally Controller')
        self.assertEqual(c[0]['mode'], 'uinput')
        self.assertEqual(c[0]['modes'], ['uinput', 'dualsense', 'hori_steam'])
        self.assertIn('paddles_as', c[0]['children']['dualsense'])
        self.assertIs(c[0]['nintendo'], False)
        # Without the settings tree: the modes the state has.
        self.assertEqual(hh.controllers(STATE)[0]['modes'], ['uinput', 'dualsense'])

    def test_power_limit(self):
        t = hh.tdp_info(STATE, SETTINGS)
        self.assertTrue(t['enabled'])
        self.assertEqual((t['min'], t['max'], t['default'], t['watts'], t['driver']), (5, 30, 15, 17, 'unified'))
        self.assertIn('quiet', t['modes'])
        self.assertEqual(hh.hearth_mode('quiet'), 'power-saver')
        st = json.loads(json.dumps(STATE))
        st['tdp']['unified']['tdp']['mode'] = 'balanced'
        self.assertIsNone(hh.tdp_info(st)['watts'])
        st['hhd']['steamos']['tdp_status'] = 'conflict'
        self.assertFalse(hh.tdp_info(st)['enabled'])


class Extras(unittest.TestCase):
    def test_setting_node(self):
        self.assertEqual(hh.setting_node(SETTINGS, 'controllers.rog_ally.imu')['type'], 'bool')
        self.assertEqual(hh.setting_node(SETTINGS, 'controllers.rog_ally.controller_mode.uinput.paddles_as')['type'], 'multiple')
        self.assertEqual(hh.setting_node(SETTINGS, 'rgb.handheld.mode.mode')['type'], 'mode')
        self.assertEqual(hh.setting_node(SETTINGS, 'rgb.handheld.mode.solid.brightness')['type'], 'multiple')
        self.assertIsNone(hh.setting_node(SETTINGS, 'controllers.rog_ally.nope'))
        self.assertIsNone(hh.setting_node(SETTINGS, 'controllers.legion_go.imu'))
        self.assertIsNone(hh.setting_node(SETTINGS, 'rgb'))

    def test_only_listed_settings_of_this_device(self):
        ex = hh.extras(STATE, SETTINGS)
        keys = [c['key'] for c in ex]
        self.assertEqual(keys, ['controllers.rog_ally.imu', 'controllers.rog_ally.controller_mode.uinput.paddles_as',
                                'controllers.rog_ally.swap_xbox', 'tdp.unified.fan_full_speed', 'rgb.handheld.mode.mode'])
        # Not: the back buttons of the DualSense mode (not in use), the Nintendo mode (Hearth's own row),
        # the IMU rate, the power mode.
        by = {c['key']: c for c in ex}
        self.assertEqual(by['controllers.rog_ally.imu'], dict(by['controllers.rog_ally.imu'], kind='bool', value=True, group='controller'))
        self.assertEqual([o['id'] for o in by['controllers.rog_ally.controller_mode.uinput.paddles_as']['options']], ['steam_input', 'noob', 'disabled'])
        self.assertEqual(by['rgb.handheld.mode.mode']['value'], 'solid')
        self.assertEqual([o['label'] for o in by['rgb.handheld.mode.mode']['options']], ['Off', 'Solid', 'Rainbow'])

    def test_a_value_hhd_does_not_offer_is_not_shown(self):
        st = json.loads(json.dumps(STATE))
        st['rgb']['handheld']['mode']['mode'] = 'something_new'
        self.assertNotIn('rgb.handheld.mode.mode', [c['key'] for c in hh.extras(st, SETTINGS)])
        self.assertEqual(hh.extras({}, SETTINGS), [])
        self.assertEqual(hh.extras(STATE, {}), [])

    def test_check_change(self):
        ex = hh.extras(STATE, SETTINGS)
        self.assertIs(hh.check_change(ex, 'controllers.rog_ally.imu', False), False)
        self.assertEqual(hh.check_change(ex, 'rgb.handheld.mode.mode', 'rainbow'), 'rainbow')
        for key, v in (('controllers.rog_ally.imu', 'yes'), ('rgb.handheld.mode.mode', 'disco'), ('hhd.settings.update_stable', True),
                       ('controllers.rog_ally.nintendo_mode', True), ('tdp.unified.tdp.mode', 'custom')):
            with self.assertRaises(hh.HhdError):
                hh.check_change(ex, key, v)
        self.assertIs(hh.parse_value('true'), True)
        self.assertIs(hh.parse_value('off'), False)
        self.assertEqual(hh.parse_value('noob'), 'noob')

    def test_set_sends_only_that_key(self):
        sent = []

        def fake(method, path, body=None, timeout=6):
            if path.startswith('/api/v1/state') and method == 'POST':
                sent.append(body)
                return STATE
            if path.startswith('/api/v1/state'):
                return STATE
            if path == '/api/v1/settings':
                return SETTINGS
            if path == '/api/v1/version':
                return {'version': 5}
            raise AssertionError(path)
        with mock.patch.object(hh, 'request', fake):
            out = hh.set_extra('tdp.unified.fan_full_speed', True)
            self.assertEqual(sent, [{'tdp.unified.fan_full_speed': True}])
            self.assertTrue(out['running'])
            with self.assertRaises(hh.HhdError):
                hh.set_extra('hhd.settings.tdp_enable', False)
            self.assertEqual(len(sent), 1)


class Controller(unittest.TestCase):
    def dev(self):
        return hh.controllers(STATE, SETTINGS)[0]

    def test_types_this_device_offers(self):
        ids = [t[0] for t in hc.hhd_types([self.dev()])]
        # xbox-elite falls back to Xbox with Steam Input back buttons (no xbox_elite mode with HHD's Steam Controller).
        self.assertEqual(ids, ['', 'xbox-elite', 'xbox-series', 'ds5', 'ds5-edge', 'deck'])
        self.assertEqual(hc.hhd_pick('xbox-elite', ['uinput']), ('uinput', {'paddles_as': 'steam_input'}))
        self.assertEqual(hc.hhd_pick('xbox-elite', ['xbox_elite', 'uinput']), ('xbox_elite', {}))
        self.assertIsNone(hc.hhd_pick('deck', ['uinput']))

    def test_changes(self):
        cur = hh.unroll(STATE)
        ch = hc.hhd_changes(self.dev(), {'map': {}, 'target': 'ds5'}, cur, None)
        self.assertEqual(ch, {'controllers.rog_ally.controller_mode.mode': 'dualsense',
                              'controllers.rog_ally.controller_mode.dualsense.paddles_as': 'touchpad'})
        # Nintendo layout: only when it's part of your settings.
        self.assertEqual(hc.hhd_changes(self.dev(), {'map': hc.NINTENDO, 'target': ''}, cur, None), {'controllers.rog_ally.nintendo_mode': True})
        self.assertEqual(hc.hhd_changes(self.dev(), {'map': {}, 'target': ''}, cur, None), {})
        # Automatic goes back to HHD's own choice from before.
        cur2 = dict(cur, **{'controllers.rog_ally.controller_mode.mode': 'dualsense'})
        self.assertEqual(hc.hhd_changes(self.dev(), {'map': {}, 'target': ''}, cur2, 'uinput'), {'controllers.rog_ally.controller_mode.mode': 'uinput'})
        # Reset turns the Nintendo mode off.
        cur3 = dict(cur, **{'controllers.rog_ally.nintendo_mode': True})
        d = dict(self.dev(), nintendo=True)
        self.assertEqual(hc.hhd_changes(d, {'map': {}, 'target': ''}, cur3, None, reset=True), {'controllers.rog_ally.nintendo_mode': False})

    def test_status_and_apply_through_hhd(self):
        sent = []
        tmp = tempfile.mkdtemp()

        class FakeHhd:
            HhdError = hh.HhdError
            unroll = staticmethod(hh.unroll)
            controllers = staticmethod(hh.controllers)
            available = staticmethod(lambda: True)
            state = staticmethod(lambda poll=False: STATE)
            settings = staticmethod(lambda: SETTINGS)
            set_state = staticmethod(lambda ch: sent.append(ch) or STATE)
        env = {'XDG_STATE_HOME': tmp, 'XDG_CONFIG_HOME': tmp}
        with mock.patch.dict(os.environ, env), mock.patch.object(hc, 'hhd', lambda: FakeHhd), mock.patch.object(hc, 'available', lambda: False):
            st = hc.status()
            self.assertEqual(st['backend'], 'hhd')
            self.assertEqual(st['buttons'], [])
            self.assertIs(st['nintendo'], False)
            self.assertIn('Handheld Daemon', st['devices'][0]['name'])
            hc.save_conf({'map': {}, 'target': 'deck'})
            hc.apply_all()
            self.assertEqual(sent[-1], {'controllers.rog_ally.controller_mode.mode': 'hori_steam'})
            self.assertEqual(hc.hhd_originals(), {'rog_ally': 'uinput'})   # what Automatic goes back to
            with self.assertRaises(RuntimeError):
                hc.cmd_set('South', 'East')   # single buttons: not through HHD


class Power(unittest.TestCase):
    def test_limit_from_hhd_set_through_steamos_manager_first(self):
        calls = []

        class FakeHhd:
            MODE_NAMES = hh.MODE_NAMES
            tdp_info = staticmethod(hh.tdp_info)
            hearth_mode = staticmethod(hh.hearth_mode)
            state = staticmethod(lambda poll=False: STATE)
            settings = staticmethod(lambda: SETTINGS)
            set_tdp = staticmethod(lambda w: calls.append(('hhd', w)) or w)

        def busctl(bus, *args, timeout=6):
            calls.append(('busctl',) + args[:2])
            if args[0] == 'introspect':
                return ''
            if args[0] == 'set-property':
                return ''
            raise hp.Fail('no such property')   # Anatase's SteamOS Manager can't say the current limit
        with mock.patch.object(hp, 'hhd', lambda: FakeHhd), mock.patch.object(hp, 'busctl', busctl), \
                mock.patch.object(hp, 'remember', lambda w: None), mock.patch.object(hp, 'remembered', lambda: None):
            st = hp.tdp_status()
            self.assertEqual(st, {'supported': True, 'watts': 17, 'min': 5, 'max': 30, 'via': 'hhd', 'mode': 'custom'})
            self.assertEqual(hp.set_tdp(17), 17)
            self.assertIn(('busctl', 'set-property', 'com.steampowered.SteamOSManager1'), calls)
            self.assertNotIn(('hhd', 17), calls)   # HHD had it after SteamOS Manager passed it on
            calls.clear()
            self.assertEqual(hp.set_tdp(12), 12)   # HHD still says 17: its own event then
            self.assertIn(('hhd', 12), calls)


if __name__ == '__main__':
    unittest.main()
