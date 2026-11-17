"""hearth-screens (1.14, docs/SCREENS.md): Desktop Mode's screens under labwc (wlr-randr) and KWin (kscreen-doctor),
the saved settings (screens.json + display.sh: never lost, never written by reading, a damaged file never replaced
with defaults), restore at start (no unlisted mode, never every screen off), KWin's Night Light / tearing in kwinrc,
screenshots' pictures, and the pages and helpers that use it."""
import contextlib
import importlib.machinery
import io
import importlib.util
import json
import os
import pathlib
import struct
import tempfile
import unittest
import zlib
from unittest import mock

SHELL = pathlib.Path(__file__).resolve().parents[1] / 'desktop' / 'shell'
REPO = pathlib.Path(__file__).resolve().parents[3]
loader = importlib.machinery.SourceFileLoader('hscreens', str(SHELL / 'hearth-screens'))
spec = importlib.util.spec_from_loader('hscreens', loader)
hs = importlib.util.module_from_spec(spec)
loader.exec_module(hs)

WLR = [
    {'name': 'DP-1', 'description': 'LG Electronics LG 27GP850 (DP-1)', 'make': 'LG Electronics', 'model': 'LG 27GP850', 'serial': '',
     'physical_size': {'width': 600, 'height': 340}, 'enabled': True,
     'modes': [{'width': 2560, 'height': 1440, 'refresh': 164.958, 'preferred': True, 'current': True},
               {'width': 2560, 'height': 1440, 'refresh': 59.951, 'preferred': False, 'current': False},
               {'width': 1920, 'height': 1080, 'refresh': 60.0, 'preferred': False, 'current': False}],
     'position': {'x': 0, 'y': 0}, 'transform': 'normal', 'scale': 1.0, 'adaptive_sync': False},
    {'name': 'eDP-1', 'description': 'BOE 0x0BCA (eDP-1)', 'make': 'BOE', 'model': '0x0BCA', 'serial': '',
     'physical_size': {'width': 155, 'height': 97}, 'enabled': True,
     'modes': [{'width': 1920, 'height': 1200, 'refresh': 60.0, 'preferred': True, 'current': True}],
     'position': {'x': 2560, 'y': 0}, 'transform': '90', 'scale': 1.0, 'adaptive_sync': False},
]

KSCREEN = {'features': 1, 'outputs': [
    {'id': 1, 'name': 'DP-1', 'type': 14, 'pos': {'x': 0, 'y': 0}, 'scale': 1.25, 'size': {'width': 2560, 'height': 1440}, 'rotation': 1,
     'currentModeId': '2', 'preferredModes': ['1'], 'connected': True, 'enabled': True, 'priority': 1, 'clones': [],
     'sizeMM': {'width': 600, 'height': 340}, 'replicationSource': 0,
     'modes': [{'id': '1', 'name': '2560x1440@165', 'size': {'width': 2560, 'height': 1440}, 'refreshRate': 164.958},
               {'id': '2', 'name': '2560x1440@60', 'size': {'width': 2560, 'height': 1440}, 'refreshRate': 59.951},
               {'id': '3', 'name': '1920x1080@60', 'size': {'width': 1920, 'height': 1080}, 'refreshRate': 60.0}],
     'vrrPolicy': 2, 'hdr': True, 'sdr-brightness': 250, 'wcg': False},
    {'id': 2, 'name': 'HDMI-A-1', 'type': 11, 'pos': {'x': 2048, 'y': 0}, 'scale': 1, 'rotation': 8, 'currentModeId': '5',
     'preferredModes': ['5'], 'connected': True, 'enabled': True, 'priority': 2, 'sizeMM': {'width': 0, 'height': 0},
     'modes': [{'id': '5', 'name': '1920x1080@60', 'size': {'width': 1920, 'height': 1080}, 'refreshRate': 60.0}]},
    {'id': 3, 'name': 'DP-2', 'connected': False, 'enabled': False, 'modes': []},
]}
KSCREEN_TEXT = ('\x1b[01;32mOutput: \x1b[0;0m1 DP-1\n\tenabled\n\tHDR: enabled\n\tICC profile: /home/me/.local/share/icc/lg.icc\n'
                '\tColor profile source: ICC\n\x1b[01;32mOutput: \x1b[0;0m2 HDMI-A-1\n\tICC profile: incapable\n\tColor profile source: incapable\n')


class FakeRun:
    """wlr-randr / kscreen-doctor stand-ins: answers from the given lists, records what was asked."""
    def __init__(self, wlr=None, ks=None, ks_text=KSCREEN_TEXT, refuse_mode=False):
        self.wlr, self.ks, self.ks_text, self.refuse_mode, self.calls = wlr, ks, ks_text, refuse_mode, []

    def __call__(self, cmd, timeout=10, env=None):
        self.calls.append(list(cmd))
        if cmd[:2] == ['wlr-randr', '--json']:
            return (0, json.dumps(self.wlr), '') if self.wlr is not None else (127, '', 'no')
        if cmd[0] == 'wlr-randr':
            return (1 if self.refuse_mode and '--mode' in cmd else 0), '', ''
        if cmd[:2] == ['kscreen-doctor', '-j']:
            return (0, json.dumps(self.ks), '') if self.ks is not None else (1, '', '')
        if cmd[:2] == ['kscreen-doctor', '-o']:
            return 0, self.ks_text, ''
        if cmd[0] == 'kscreen-doctor':
            return 0, '', ''
        return 127, '', 'unknown'


class Base(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        n = os.path.join(self.d.name, 'nimbus')
        self.p = [mock.patch.object(hs, 'NIMBUS', n), mock.patch.object(hs, 'SAVED', os.path.join(n, 'screens.json')),
                  mock.patch.object(hs, 'DISPLAY_SH', os.path.join(n, 'display.sh')),
                  mock.patch.object(hs, 'KWINRC', os.path.join(self.d.name, 'kwinrc')),
                  mock.patch.object(hs, 'DESKTOP_JSON', os.path.join(self.d.name, 'desktop.json')),
                  mock.patch.object(hs, 'SYS_DRM', os.path.join(self.d.name, 'drm')),
                  mock.patch.object(hs.io, 'log_event'), mock.patch.object(hs.io, 'notice'),
                  mock.patch.object(hs, '_gio', return_value=(None, None))]
        for x in self.p:
            x.start()
        os.makedirs(n)
        self.nimbus = n

    def tearDown(self):
        for x in self.p:
            x.stop()
        self.d.cleanup()

    def files(self):
        out = {}
        for root, _, fs in os.walk(self.d.name):
            for f in fs:
                p = os.path.join(root, f)
                out[os.path.relpath(p, self.d.name)] = pathlib.Path(p).read_bytes()
        return out


class Reading(Base):
    def test_labwc_keeps_wlr_randrs_shape_and_adds_the_new_fields(self):
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            outs = hs.read('labwc')
        self.assertEqual([o['name'] for o in outs], ['DP-1', 'eDP-1'])
        for k in ('modes', 'position', 'transform', 'scale', 'physical_size', 'description', 'enabled'):
            self.assertIn(k, outs[0])
        self.assertEqual(outs[1]['rotation'], 'left')
        self.assertEqual(outs[0]['hdr'], {'capable': False, 'on': False})
        self.assertFalse(outs[0]['vrr']['capable'])

    def test_kwin_from_kscreen_doctor_in_the_same_shape(self):
        outs = hs.parse_kscreen(json.dumps(KSCREEN), KSCREEN_TEXT, {})
        self.assertEqual([o['name'] for o in outs], ['DP-1', 'HDMI-A-1'])      # DP-2 isn't connected
        dp = outs[0]
        cur = [m for m in dp['modes'] if m['current']]
        self.assertEqual((cur[0]['width'], cur[0]['height'], round(cur[0]['refresh'])), (2560, 1440, 60))
        self.assertTrue([m for m in dp['modes'] if m['preferred']][0]['refresh'] > 160)
        self.assertEqual(dp['position'], {'x': 0, 'y': 0})
        self.assertEqual(dp['scale'], 1.25)
        self.assertEqual(dp['hdr'], {'capable': True, 'on': True})
        self.assertEqual(dp['sdrBrightness'], 250)
        self.assertEqual(dp['vrr'], {'capable': True, 'policy': 'automatic'})
        self.assertEqual(dp['wcg'], {'capable': True, 'on': False})
        self.assertEqual(dp['icc'], {'capable': True, 'path': '/home/me/.local/share/icc/lg.icc', 'source': 'ICC'})
        hd = outs[1]
        self.assertEqual((hd['rotation'], hd['transform']), ('right', '270'))
        self.assertFalse(hd['hdr']['capable'] or hd['vrr']['capable'] or hd['icc']['capable'])

    def test_newer_libkscreen_without_enabled_means_on(self):
        ks = {'outputs': [{'id': 1, 'name': 'WL-0', 'connected': True, 'currentModeId': '1', 'scale': 1, 'rotation': 1, 'priority': 0,
                           'modes': [{'id': '1', 'size': {'width': 1920, 'height': 1080}, 'refreshRate': 60.0}]}]}
        o = hs.parse_kscreen(json.dumps(ks), '', {})[0]
        self.assertTrue(o['enabled'])
        self.assertTrue(o['modes'][0]['current'])

    def test_edid_names_the_screen(self):
        raw = bytearray(128)
        raw[8:10] = bytes([0x1E, 0x6D])           # GSM (LG)
        raw[54:57] = b'\0\0\0'
        raw[57] = 0xFC
        raw[59:59 + 10] = b'LG 27GP850'
        outs = hs.parse_kscreen(json.dumps(KSCREEN), '', {'DP-1': bytes(raw)})
        self.assertEqual(outs[0]['model'], 'LG 27GP850')
        self.assertEqual(outs[0]['make'], 'GSM')

    def test_get_and_info_write_nothing(self):
        pathlib.Path(hs.DISPLAY_SH).write_text("#!/bin/sh\nwlr-randr --output 'DP-1' --on --mode '1920x1080@60.000Hz' --scale 1\n")
        before = self.files()
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            hs.read('labwc')
            r = hs.info('labwc')
        self.assertEqual(self.files(), before)
        self.assertEqual(r['saved']['outputs']['DP-1']['mode'], {'w': 1920, 'h': 1080, 'r': 60.0})
        self.assertEqual(r['tearing']['why'], hs.NEEDS_KWIN)
        self.assertFalse(r['mirror']['capable'])

    def test_wm_follows_the_session(self):
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}):
            self.assertEqual(hs.wm(), 'kwin')
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc', 'XDG_CURRENT_DESKTOP': 'KDE'}):
            self.assertEqual(hs.wm(), 'labwc')


class Changing(Base):
    def test_labwc_command(self):
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            live = hs.read('labwc')
        a = hs.ops_labwc(live, {'DP-1': {'mode': {'w': 2560, 'h': 1440, 'r': 165}, 'scale': 1.5, 'pos': {'x': 0, 'y': 0}, 'rotation': 'right'}})
        self.assertEqual(a, ['wlr-randr', '--output', 'DP-1', '--mode', '2560x1440@164.958Hz', '--scale', '1.5', '--pos', '0,0', '--transform', '270'])
        with self.assertRaises(hs.Refused):
            hs.ops_labwc(live, {'DP-1': {'mode': {'w': 3840, 'h': 2160, 'r': 60}}})
        with self.assertRaises(hs.Refused) as e:
            hs.ops_labwc(live, {'DP-1': {'hdr': True}})
        self.assertIn(hs.NEEDS_KWIN, str(e.exception))

    def test_kwin_command(self):
        live = hs.parse_kscreen(json.dumps(KSCREEN), KSCREEN_TEXT, {})
        ops = hs.ops_kwin(live, {'DP-1': {'mode': {'w': 2560, 'h': 1440, 'r': 165}, 'scale': 1.25, 'rotation': 'left', 'vrr': 'always',
                                          'hdr': False, 'sdrBrightness': 300, 'icc': 'EDID'}})
        self.assertEqual(ops, ['output.DP-1.mode.1', 'output.DP-1.scale.1.25', 'output.DP-1.rotation.left', 'output.DP-1.vrrpolicy.always',
                               'output.DP-1.hdr.disable', 'output.DP-1.sdr-brightness.300', 'output.DP-1.colorProfileSource.EDID'])
        self.assertEqual(hs.ops_kwin(live, {'HDMI-A-1': {'enabled': False}}), ['output.HDMI-A-1.disable'])
        for bad in ({'HDMI-A-1': {'hdr': True}}, {'HDMI-A-1': {'vrr': 'always'}}, {'DP-1': {'icc': '/nope.icc'}}, {'DP-9': {'scale': 1}}):
            with self.assertRaises(hs.Refused):
                hs.ops_kwin(live, bad)

    def test_never_every_screen_off(self):
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR[:1])):
            with self.assertRaises(hs.Refused):
                hs.apply({'outputs': {'DP-1': {'enabled': False}}}, 'labwc')

    def test_a_refused_mode_is_reported(self):
        fake = FakeRun(wlr=WLR, refuse_mode=True)
        with mock.patch.object(hs, 'RUN', fake):
            _, problem = hs.apply({'outputs': {'DP-1': {'mode': {'w': 1920, 'h': 1080, 'r': 60}}}}, 'labwc')
        self.assertIn("didn't accept 1920 × 1080 at 60 Hz", problem)
        self.assertFalse(os.path.exists(hs.SAVED))          # apply never saves


class Saving(Base):
    def test_keep_writes_screens_json_and_display_sh_keeping_the_rest(self):
        pathlib.Path(hs.DISPLAY_SH).write_text("#!/bin/sh\nwlr-randr --output 'HDMI-A-1' --on --mode '1920x1080@60.000Hz' --scale 1\n"
                                               "wlr-randr --output 'DP-1' --scale 2\n")
        live = [dict(WLR[0], scale=1.25, rotation='normal')]
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            hs.keep(['DP-1'], 'labwc', [hs._fill(dict(o), 'labwc') for o in live])
        sh = pathlib.Path(hs.DISPLAY_SH).read_text()
        self.assertIn("wlr-randr --output 'HDMI-A-1' --on --mode '1920x1080@60.000Hz' --scale 1\n", sh)
        self.assertIn("wlr-randr --output 'DP-1' --on --mode '2560x1440@164.958Hz' --scale 1.25 --pos 0,0\n", sh)
        self.assertNotIn('--scale 2', sh)
        self.assertTrue(os.access(hs.DISPLAY_SH, os.X_OK))
        self.assertTrue(os.path.exists(hs.DISPLAY_SH + '.bak'))
        sv = json.loads(pathlib.Path(hs.SAVED).read_text())
        self.assertEqual(sv['outputs']['DP-1']['scale'], 1.25)
        self.assertEqual(sv['outputs']['DP-1']['mode'], {'w': 2560, 'h': 1440, 'r': 164.958})
        # display.sh's lines read back as what was kept
        self.assertEqual(hs.parse_display_sh(sh)['DP-1']['scale'], 1.25)

    def test_what_you_set_on_kwin_survives_a_keep_on_labwc(self):
        pathlib.Path(hs.SAVED).write_text(json.dumps({'version': 1, 'outputs': {'DP-1': {'hdr': True, 'vrr': 'always', 'sdrBrightness': 300, 'scale': 1}}, 'tearing': False}))
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            hs.keep(['DP-1'], 'labwc', hs.read('labwc'))
        sv = json.loads(pathlib.Path(hs.SAVED).read_text())
        self.assertEqual((sv['outputs']['DP-1']['hdr'], sv['outputs']['DP-1']['vrr'], sv['outputs']['DP-1']['sdrBrightness']), (True, 'always', 300))
        self.assertIs(sv['tearing'], False)

    def test_kwin_keep_saves_hdr_vrr_and_profile(self):
        live = hs.parse_kscreen(json.dumps(KSCREEN), KSCREEN_TEXT, {})
        hs.keep(['DP-1'], 'kwin', live)
        e = json.loads(pathlib.Path(hs.SAVED).read_text())['outputs']['DP-1']
        self.assertEqual((e['hdr'], e['sdrBrightness'], e['vrr'], e['icc'], e['wcg']), (True, 250, 'automatic', '/home/me/.local/share/icc/lg.icc', False))
        self.assertIn("wlr-randr --output 'DP-1' --on --mode '2560x1440@59.951Hz' --scale 1.25", pathlib.Path(hs.DISPLAY_SH).read_text())

    def test_display_sh_newer_than_screens_json_counts(self):
        pathlib.Path(hs.SAVED).write_text(json.dumps({'outputs': {'DP-1': {'scale': 1.5, 'hdr': True}}}))
        t = os.stat(hs.SAVED).st_mtime
        pathlib.Path(hs.DISPLAY_SH).write_text("wlr-randr --output 'DP-1' --scale 2\nwlr-randr --output 'eDP-1' --off\n")
        os.utime(hs.DISPLAY_SH, (t + 60, t + 60))
        sv = hs.saved()['outputs']
        self.assertEqual((sv['DP-1']['scale'], sv['DP-1']['hdr'], sv['eDP-1']['enabled']), (2.0, True, False))
        os.utime(hs.DISPLAY_SH, (t - 60, t - 60))
        self.assertEqual(hs.saved()['outputs']['DP-1']['scale'], 1.5)

    def test_a_damaged_file_is_never_overwritten_by_reading_or_restore(self):
        pathlib.Path(hs.SAVED).write_text('{"outputs": {"DP-1": ')
        before = self.files()
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)), mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            hs.info('labwc')
            hs.restore()
        self.assertEqual(self.files(), before)

    def test_a_damaged_file_comes_back_from_its_bak_on_keep(self):
        good = {'version': 1, 'outputs': {'eDP-1': {'scale': 1.5}, 'DP-1': {'hdr': True}}}
        pathlib.Path(hs.SAVED + '.bak').write_text(json.dumps(good))
        pathlib.Path(hs.SAVED).write_text('{"outputs": ')
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            hs.keep(['DP-1'], 'labwc', hs.read('labwc'))
        sv = json.loads(pathlib.Path(hs.SAVED).read_text())
        self.assertEqual(sv['outputs']['eDP-1'], {'scale': 1.5})          # the other screen's settings are back
        self.assertTrue(sv['outputs']['DP-1']['hdr'])
        self.assertTrue([f for f in os.listdir(self.nimbus) if f.startswith('screens.json.broken-')])

    def test_display_lines_round_trip(self):
        e = {'enabled': True, 'mode': {'w': 1920, 'h': 1080, 'r': 143.981}, 'scale': 1.25, 'pos': {'x': 10, 'y': 0}, 'rotation': 'inverted'}
        line = hs.display_line('HDMI-A-1', e)
        self.assertEqual(line, "wlr-randr --output 'HDMI-A-1' --on --mode '1920x1080@143.981Hz' --scale 1.25 --pos 10,0 --transform 180")
        self.assertEqual(hs.parse_display_sh(line)['HDMI-A-1'], e)
        self.assertEqual(hs.display_line('DP-1', {'enabled': False}), "wlr-randr --output 'DP-1' --off")


class Restore(Base):
    def test_restore_leaves_out_a_mode_the_screen_does_not_list(self):
        sv = {'outputs': {'DP-1': {'mode': {'w': 3840, 'h': 2160, 'r': 60}, 'scale': 1.5}}}
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            changes, left = hs.restore_plan(hs.read('labwc'), sv, 'labwc')
        self.assertEqual(changes['DP-1'], {'scale': 1.5})
        self.assertTrue(any('3840' in x for x in left))

    def test_restore_never_turns_every_screen_off(self):
        sv = {'outputs': {'DP-1': {'enabled': False}}}
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR[:1])):
            changes, left = hs.restore_plan(hs.read('labwc'), sv, 'labwc')
        self.assertNotIn('enabled', changes.get('DP-1', {}))
        self.assertTrue(left)

    def test_restore_on_kwin_puts_back_only_what_differs(self):
        live = hs.parse_kscreen(json.dumps(KSCREEN), KSCREEN_TEXT, {})
        sv = {'outputs': {'DP-1': {'mode': {'w': 2560, 'h': 1440, 'r': 164.958}, 'scale': 1.25, 'hdr': True, 'vrr': 'always', 'sdrBrightness': 250},
                          'HDMI-A-1': {'hdr': True}}}
        changes, left = hs.restore_plan(live, sv, 'kwin')
        self.assertEqual(changes, {'DP-1': {'mode': {'w': 2560, 'h': 1440, 'r': 164.958}, 'vrr': 'always'}})
        self.assertEqual(left, [])

    def test_restore_runs_one_command_and_tells_kwin_about_night_light(self):
        pathlib.Path(hs.SAVED).write_text(json.dumps({'outputs': {'DP-1': {'scale': 1.5}}, 'tearing': False}))
        pathlib.Path(hs.DESKTOP_JSON).write_text(json.dumps({'nightLight': True, 'nightTemp': 3200, 'nightMode': 'times', 'nightFrom': '21:00', 'nightTo': '06:30'}))
        pathlib.Path(hs.KWINRC).write_text('[Plugins]\nblurEnabled=false\n\n[Compositing]\nLatencyPolicy=Low\n')
        fake = FakeRun(ks=KSCREEN)
        with mock.patch.object(hs, 'RUN', fake), mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}):
            r = hs.restore()
        self.assertEqual(r['changed'], {'DP-1': {'scale': 1.5}})
        self.assertIn(['kscreen-doctor', 'output.DP-1.scale.1.5'], fake.calls)
        rc = pathlib.Path(hs.KWINRC).read_text()
        for line in ('[Plugins]\nblurEnabled=false', 'LatencyPolicy=Low', 'AllowTearing=false', '[NightColor]', 'Active=true',
                     'NightTemperature=3200', 'EveningBeginFixed=2100', 'MorningBeginFixed=0630', 'Mode=Times'):
            self.assertIn(line, rc)

    def test_autoscale_for_small_sharp_screens(self):
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            live = hs.read('labwc')
        self.assertIsNone(hs.auto_scale(live[0]))                # 27" 1440p: 108 dpi
        self.assertEqual(hs.auto_scale(live[1]), 1.25)            # 7" 1200-wide (turned): 1200 / 6.1 in ≈ 197 dpi
        changes, _ = hs.restore_plan(live, {'outputs': {}}, 'labwc')
        self.assertEqual(changes, {'eDP-1': {'scale': 1.25}})


class KWinSettings(Base):
    def test_ini_set_keeps_every_other_line(self):
        t = '[General]\nA=1\n\n[NightColor]\nActive=false\nOther=x\n\n[Z]\nB=2\n'
        out = hs.ini_set(t, 'NightColor', {'Active': 'true', 'NightTemperature': '4000'})
        self.assertEqual(out, '[General]\nA=1\n\n[NightColor]\nActive=true\nOther=x\nNightTemperature=4000\n\n[Z]\nB=2\n')
        self.assertEqual(hs.ini_set('', 'Compositing', {'AllowTearing': 'false'}), '[Compositing]\nAllowTearing=false\n')
        self.assertEqual(hs.ini_get(out, 'NightColor', 'NightTemperature'), '4000')

    def test_night_values(self):
        self.assertEqual(hs.night_kwinrc({'nightLight': False, 'nightTemp': 99999, 'nightMode': 'always', 'nightFrom': 'x', 'nightTo': '7:05'}),
                         {'Active': 'false', 'Mode': 'Constant', 'NightTemperature': 6500, 'EveningBeginFixed': '1930', 'MorningBeginFixed': '0705', 'TransitionTime': 30})

    def test_night_only_writes_when_something_changed(self):
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'kwin'}):
            self.assertTrue(hs.night({'nightLight': True})['changed'])
            self.assertFalse(hs.night({'nightLight': True})['changed'])
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}):
            self.assertEqual(hs.night(), {'engine': 'wlsunset'})


KSCREEN_TEXT_114 = (KSCREEN_TEXT.replace('\tHDR: enabled\n', '\tHDR: enabled\n\t\tSDR brightness: 250 nits\n\t\tSDR gamut wideness: 40%\n'
                                       '\t\tPeak brightness: 603.5 nits, overridden with: 450 nits\n\t\tMax average brightness: 400 nits\n'
                                       '\t\tMin brightness: 0.05 nits\n')
                    .replace('\tColor profile source: ICC\n', '\tColor profile source: ICC\n\tColor power preference: prefer accuracy\n'
                             '\tBrightness control: supported, set to 70% and dimming to 100%\n'))
KSCREEN_114 = json.loads(json.dumps(KSCREEN))
KSCREEN_114['outputs'][1].update({'overscan': 0, 'rgbRange': 0})


class Screens114(Base):
    """1.14's KDE rows: main screen, colour intensity and accuracy, HDR calibration, overscan, RGB range, Super+P,
    night light by location with its fade, X11 apps, the edge barrier, desktops, DDC/CI, auto-brightness."""

    def live(self):
        return hs.parse_kscreen(json.dumps(KSCREEN_114), KSCREEN_TEXT_114, {})

    def test_kwin_reads_the_new_fields(self):
        dp, tv = self.live()
        self.assertEqual(dp['priority'], 1)
        self.assertEqual(dp['sdrGamut'], {'capable': True, 'value': 40})
        self.assertEqual(dp['colorPower'], {'capable': True, 'value': 'accuracy'})
        self.assertEqual((dp['hdrCal']['capable'], dp['hdrCal']['peak'], dp['hdrCal']['peakSet'], dp['hdrCal']['avg'], dp['hdrCal']['avgSet'], dp['hdrCal']['min']),
                         (True, 603.5, 450.0, 400.0, None, 0.05))
        self.assertEqual(dp['kwinBrightness'], {'capable': True, 'value': 70})
        self.assertFalse(dp['overscan']['capable'])
        self.assertEqual((tv['overscan'], tv['rgbRange']), ({'capable': True, 'value': 0}, {'capable': True, 'value': 'automatic'}))
        self.assertFalse(tv['hdrCal']['capable'])
        for o in (dp, tv):                     # not in KWin 6.3: hidden on the page
            self.assertFalse(o['depth']['capable'])
            self.assertFalse(o['redTint']['capable'])
            self.assertFalse(o['edr']['capable'])
        edr = hs.parse_kscreen(json.dumps(KSCREEN_114), KSCREEN_TEXT_114.replace('\tICC profile: incapable', '\tExtended dynamic range: never\n\tICC profile: incapable'), {})
        self.assertEqual(edr[1]['edr'], {'capable': True, 'policy': 'never'})

    def test_kwin_commands_for_the_new_keys(self):
        live = self.live()
        self.assertEqual(hs.ops_kwin(live, {'HDMI-A-1': {'priority': 1, 'overscan': 5, 'rgbRange': 'full'}}),
                         ['output.HDMI-A-1.priority.1', 'output.HDMI-A-1.overscan.5', 'output.HDMI-A-1.rgbrange.full'])
        self.assertEqual(hs.ops_kwin(live, {'DP-1': {'sdrGamut': 70, 'colorPower': 'efficiency', 'hdrCal': {'peak': 800, 'avg': None, 'min': 0.02}}}),
                         ['output.DP-1.sdrGamut.70', 'output.DP-1.colorPowerTradeoff.preferEfficiency',
                          'output.DP-1.maxAverageBrightnessOverride.disable', 'output.DP-1.maxBrightnessOverride.800',
                          'output.DP-1.minBrightnessOverride.200'])
        for bad in ({'DP-1': {'overscan': 3}}, {'HDMI-A-1': {'sdrGamut': 50}}, {'HDMI-A-1': {'hdrCal': {'peak': 500}}},
                    {'HDMI-A-1': {'rgbRange': 'tv'}}, {'HDMI-A-1': {'edr': 'always'}}, {'HDMI-A-1': {'colorPower': 'accuracy'}}):
            with self.assertRaises(hs.Refused):
                hs.ops_kwin(live, bad)
        # HDR coming on in the same change brings its colour intensity with it.
        self.assertIn('output.DP-1.sdrGamut.30', hs.ops_kwin([dict(live[0], sdrGamut={'capable': False, 'value': 0})], {'DP-1': {'hdr': True, 'sdrGamut': 30}}))
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            for k, v in (('priority', 1), ('overscan', 2), ('rgbRange', 'full'), ('sdrGamut', 5), ('colorPower', 'accuracy'), ('hdrCal', {})):
                with self.assertRaises(hs.Refused) as c:
                    hs.ops_labwc(hs.read('labwc'), {'DP-1': {k: v}})
                self.assertIn(hs.NEEDS_KWIN, str(c.exception))

    def test_keep_and_restore_the_new_keys(self):
        live = self.live()
        hs.keep(['DP-1', 'HDMI-A-1'], 'kwin', live)
        sv = json.loads(pathlib.Path(hs.SAVED).read_text())['outputs']
        self.assertEqual((sv['DP-1']['priority'], sv['DP-1']['sdrGamut'], sv['DP-1']['colorPower'], sv['DP-1']['hdrCal']),
                         (1, 40, 'accuracy', {'peak': 450.0, 'avg': None, 'min': None}))
        self.assertEqual((sv['HDMI-A-1']['overscan'], sv['HDMI-A-1']['rgbRange'], sv['HDMI-A-1']['priority']), (0, 'automatic', 2))
        # After a restart KWin forgot them: restore puts back only what differs.
        saved = {'outputs': {'DP-1': dict(sv['DP-1'], sdrGamut=60), 'HDMI-A-1': dict(sv['HDMI-A-1'], priority=1, rgbRange='limited')}}
        changes, left = hs.restore_plan(live, saved, 'kwin')
        self.assertEqual(changes, {'DP-1': {'sdrGamut': 60}, 'HDMI-A-1': {'priority': 1, 'rgbRange': 'limited'}})
        self.assertEqual(left, [])
        # labwc never touches them.
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            ch, _ = hs.restore_plan(hs.read('labwc'), saved, 'labwc')
        self.assertFalse(any(k in c for c in ch.values() for k in ('priority', 'sdrGamut', 'rgbRange')))

    def test_super_p_plans(self):
        live = self.live()
        self.assertEqual(hs.switch_plan('main', live, 'kwin'), {'HDMI-A-1': {'enabled': False}})
        self.assertEqual(hs.switch_plan('other', live, 'kwin'), {'DP-1': {'enabled': False}})
        off = [live[0], dict(live[1], enabled=False)]
        self.assertEqual(hs.switch_plan('extend', off, 'kwin'), {'HDMI-A-1': {'enabled': True, 'pos': {'x': 2048, 'y': 0}}})
        self.assertEqual(hs.switch_plan('extend', live, 'kwin'), {})
        self.assertEqual(hs.switch_state(live), 'extend')
        self.assertEqual(hs.switch_state(off), 'main')
        self.assertEqual(hs.switch_state([dict(live[0], enabled=False), live[1]]), 'other')
        with self.assertRaises(hs.Refused):
            hs.switch_plan('mirror', live, 'kwin', "KWin 6.3 can't mirror screens yet")
        with self.assertRaises(hs.Refused):
            hs.switch_plan('other', live[:1], 'kwin')
        # The plan never leaves every screen off (apply checks it too).
        for mode in ('main', 'other', 'extend'):
            ch = hs.switch_plan(mode, live, 'kwin')
            on = {o['name'] for o in live if o['enabled']}
            for n, e in ch.items():
                if 'enabled' in e:
                    (on.add if e['enabled'] else on.discard)(n)
            self.assertTrue(on, mode)

    def test_night_by_location_and_fade(self):
        base = {'nightLight': True, 'nightTemp': 3500, 'nightFrom': '20:00', 'nightTo': '07:00', 'nightFade': 15}
        r = hs.night_kwinrc(dict(base, nightMode='location', nightPlace='Austin', nightLat=30.2672, nightLon=-97.7431))
        self.assertEqual((r['Mode'], r['LatitudeFixed'], r['LongitudeFixed'], r['TransitionTime']), ('Location', '30.2672', '-97.7431', 15))
        # No place picked yet: your times, never KWin's network lookup (Automatic).
        r = hs.night_kwinrc(dict(base, nightMode='location', nightPlace=''))
        self.assertEqual(r['Mode'], 'Times')
        self.assertNotIn('LatitudeFixed', r)
        self.assertEqual(hs.night_kwinrc(dict(base, nightFade=999))['TransitionTime'], 120)
        self.assertEqual(hs.night_kwinrc(dict(base, nightMode='location', nightPlace='X', nightLat=95, nightLon=0))['Mode'], 'Times')
        sh = (SHELL / 'shell.qml').read_text()
        self.assertIn('"-l", String(Settings.nightLat), "-L", String(Settings.nightLon)', sh)
        self.assertIn('String(Math.max(1, Settings.nightFade) * 60)', sh)
        st = (SHELL / 'Settings.qml').read_text()
        for k in ('nightPlace', 'nightLat', 'nightLon', 'nightFade'):
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), st)

    def test_kde_files_change_only_their_keys(self):
        """X11 apps' scaling and the edge barrier go through hearth-kwin-settings set (the KDE files' one writer)."""
        cfg = os.path.join(self.d.name, 'config')
        os.makedirs(cfg)
        kwinrc, kdeglobals = os.path.join(cfg, 'kwinrc'), os.path.join(cfg, 'kdeglobals')
        pathlib.Path(kwinrc).write_text('[EdgeBarrier]\nOther=1\n\n[Plugins]\nblurEnabled=false\n')
        pathlib.Path(kdeglobals).write_text('[Colors:View]\nBackgroundNormal=1,2,3\n')
        env = {'XDG_CONFIG_HOME': cfg, 'XDG_CONFIG_DIRS': os.path.join(self.d.name, 'nothing'), 'DBUS_SESSION_BUS_ADDRESS': 'unix:path=/nonexistent'}
        with mock.patch.dict(os.environ, env):
            self.assertTrue(hs.x11_scale_status('kwin')['self'])
            hs.x11_scale(False)
            hs.barrier({'px': 40, 'corner': False})
            self.assertFalse(hs.x11_scale_status('kwin')['self'])
            self.assertEqual(hs.barrier_status('kwin'), {'capable': True, 'px': 40, 'corner': False, 'why': ''})
            with self.assertRaises(hs.Refused):            # never a group the Display page doesn't own
                hs.kde_set('kwinrc', {'NotAGroup': {'X': 1}})
        kg = pathlib.Path(kdeglobals).read_text()
        self.assertIn('[Colors:View]\nBackgroundNormal=1,2,3', kg)
        self.assertIn('XwaylandClientsScale=false', kg)
        rc = pathlib.Path(kwinrc).read_text()
        self.assertIn('Other=1', rc)
        self.assertIn('EdgeBarrier=40', rc)
        self.assertIn('CornerBarrier=false', rc)
        self.assertIn('blurEnabled=false', rc)
        self.assertFalse(os.path.exists(hs.SAVED))          # KDE's own files are their settings' one home

    def test_options_in_screens_json_merge(self):
        pathlib.Path(hs.SAVED).write_text(json.dumps({'version': 1, 'outputs': {'DP-1': {'scale': 1.5}}, 'tearing': False}))
        hs.save_entries({}, {'autoBrightness': True})
        hs.save_entries({}, {'desktopsMainOnly': True})
        sv = hs.saved()
        self.assertEqual((sv['outputs']['DP-1']['scale'], sv['tearing'], sv['autoBrightness'], sv['desktopsMainOnly']), (1.5, False, True, True))
        self.assertEqual(hs.desktops_status('kwin', sv)['mode'], 'main')
        self.assertFalse(hs.desktops_status('kwin', sv)['perScreen'])

    def test_main_desktops_script(self):
        js = (SHELL / 'hearth-main-desktops.js').read_text()
        self.assertIn('var MAIN = "HEARTH_MAIN";', js)
        self.assertIn('registerShortcut("Hearth main desktops off"', js)
        self.assertIn('pinned[id]', js)                     # only the windows it changed go back
        calls = []
        fake_variant = mock.Mock()
        fake_variant.unpack.return_value = (3,)

        def kc(method, *a, **k):
            calls.append((method,) + a)
            return fake_variant
        with mock.patch.object(hs, 'kwin_call', side_effect=kc), mock.patch.dict(os.environ, {'XDG_RUNTIME_DIR': self.d.name}):
            hs.main_desktops(True, self.live())
            hs.main_desktops(False)
        copy = pathlib.Path(self.d.name, 'hearth-main-desktops.js').read_text()
        self.assertIn('var MAIN = "DP-1";', copy)
        self.assertEqual([c[0] for c in calls], ['unloadScript', 'loadScript', 'run', 'invokeShortcut', 'unloadScript'])

    def test_ddc(self):
        detect = ('Display 1\n   I2C bus:  /dev/i2c-5\n   DRM connector:           card1-DP-1\n   Monitor: GSM:LG 27GP850:\n\n'
                  'Invalid display\n   I2C bus:  /dev/i2c-7\n   DRM connector:           card1-HDMI-A-1\n\n'
                  'Display 2\n   I2C bus:  /dev/i2c-6\n   DRM_connector:           card1-DP-2\n')
        self.assertEqual(hs.parse_ddc_detect(detect), {'DP-1': 5, 'DP-2': 6})
        self.assertEqual(hs.parse_vcp('VCP 10 C 50 100\n'), (50, 100))
        self.assertIsNone(hs.parse_vcp('VCP 10 ERR\n'))

        def run(cmd, timeout=10, env=None):
            if cmd[:2] == ['ddcutil', 'detect']:
                return 0, detect, ''
            if 'getvcp' in cmd:
                return (0, 'VCP 10 C 30 80\n', '') if cmd[2] == '5' else (1, '', 'no answer')
            if 'setvcp' in cmd:
                run.set = cmd
                return 0, '', ''
            return 127, '', ''
        with mock.patch.object(hs, 'RUN', run), mock.patch.object(hs, 'have', return_value=True), mock.patch.object(hs.glob, 'glob', return_value=['/dev/i2c-5']):
            d = hs.ddc_list()
            self.assertEqual(d['screens'], [{'name': 'DP-1', 'bus': 5, 'value': 30, 'max': 80}])
            self.assertEqual(hs.ddc_set('DP-1', 50)['value'], 40)
            self.assertEqual(run.set, ['ddcutil', '--bus', '5', 'setvcp', '10', '40'])
            with self.assertRaises(hs.Refused):
                hs.ddc_set('DP-2', 50)
        self.assertFalse(os.path.exists(hs.SAVED))          # the screen keeps its brightness itself
        with mock.patch.object(hs, 'have', return_value=False):
            self.assertFalse(hs.ddc_list()['available'])

    def test_auto_brightness_needs_a_sensor(self):
        iio = os.path.join(self.d.name, 'iio')
        with mock.patch.object(hs, 'IIO', iio), mock.patch.object(hs.ProxyLight, 'present', return_value=False):
            self.assertIsNone(hs.light_sensor())
            self.assertIsNone(hs.lux())
            self.assertFalse(hs.auto_status(hs.saved())['capable'])      # no sensor: the rows stay hidden
            with mock.patch.object(hs, 'have', return_value=True):
                self.assertEqual(hs.autobright()['why'], 'no light sensor')
            dev = os.path.join(iio, 'iio:device0')
            os.makedirs(dev)
            pathlib.Path(dev, 'in_illuminance_raw').write_text('200\n')
            pathlib.Path(dev, 'in_illuminance_scale').write_text('1.5\n')
            self.assertEqual(hs.lux(), 300.0)                              # the kernel's files where the proxy isn't running
            self.assertEqual(hs.light_source()[0], 'sysfs')
        self.assertEqual(hs.lux_to_pct(0), 10)
        self.assertLess(hs.lux_to_pct(50), hs.lux_to_pct(300))
        self.assertEqual(hs.lux_to_pct(100000), 100)
        self.assertFalse(hs.saved()['autoBrightness'])                     # off until you turn it on
        self.assertFalse(os.path.exists(hs.SAVED))

    def test_auto_brightness_reads_iio_sensor_proxy(self):
        class Bus:
            def __init__(self):
                self.calls = []

            def call_sync(self, dest, path, iface, method, args, rtype, flags, timeout, cancel):
                self.calls.append((dest, path, iface, method, args.unpack() if args is not None else None, flags))
                if method == 'Get':
                    val = {'HasAmbientLight': GLib.Variant('b', True), 'LightLevel': GLib.Variant('d', 250.0),
                           'LightLevelUnit': GLib.Variant('s', 'lux')}[args.unpack()[1]]
                    return GLib.Variant('(v)', (val,))
                return GLib.Variant('()', ())
        try:
            import gi
            gi.require_version('Gio', '2.0')
            from gi.repository import Gio, GLib
        except (ImportError, ValueError):
            self.skipTest('no python3-gi')
        bus = Bus()
        with mock.patch.object(hs, '_gio', return_value=(Gio, GLib)), mock.patch.object(Gio, 'bus_get_sync', return_value=bus):
            p = hs.ProxyLight()
            self.assertTrue(p.present())
            self.assertTrue(p.claim())
            self.assertEqual(p.read(), 250.0)
            p.release()
            self.assertEqual(hs.light_source()[0], 'iio-sensor-proxy')
        methods = [c[3] for c in bus.calls]
        self.assertIn('ClaimLight', methods)
        self.assertIn('ReleaseLight', methods)
        self.assertTrue(all(c[0] == 'net.hadess.SensorProxy' and c[1] == '/net/hadess/SensorProxy' for c in bus.calls))
        self.assertTrue(all(c[5] == Gio.DBusCallFlags.NO_AUTO_START for c in bus.calls))   # never waits for a proxy that isn't there

    def test_auto_brightness_smooths_offsets_and_pauses(self):
        ab = hs.AutoBright()
        self.assertEqual(ab.tick(300, 50), 56)          # toward about 66 %, in a small step
        cur = 56
        for _ in range(10):
            w = ab.tick(300, cur)
            cur = w if w is not None else cur
        self.assertAlmostEqual(cur, hs.lux_to_pct(300), delta=3)
        self.assertIsNone(ab.tick(310, cur))            # a small change in the room: left alone
        # A hand over the sensor for one reading doesn't drop the screen far.
        w = ab.tick(0, cur)
        self.assertGreaterEqual(w, cur - hs.AutoBright.STEP)
        cur = w
        for _ in range(12):
            w = ab.tick(300, cur)
            cur = w if w is not None else cur
        # "How bright": +20 puts it 20 above what the room picks.
        for _ in range(12):
            w = ab.tick(300, cur, 20)
            cur = w if w is not None else cur
        self.assertAlmostEqual(cur, hs.lux_to_pct(300) + 20, delta=3)
        self.assertEqual(hs.clamp_offset(99), 40)
        self.assertEqual(hs.clamp_offset('x'), 0)
        # Set by hand (the keys, Quick settings, the slider): paused while the light stays the same.
        self.assertIsNone(ab.tick(300, 30, 20))
        self.assertTrue(ab.paused)
        for _ in range(10):
            self.assertIsNone(ab.tick(300, 30, 20))
        # The room gets much darker: it follows again, from where you left it.
        w = None
        for _ in range(10):
            w = ab.tick(10, 30, 20)
            if w is not None:
                break
        self.assertFalse(ab.paused)
        self.assertIsNotNone(w)
        self.assertLessEqual(abs(w - 30), hs.AutoBright.STEP)

    def test_auto_brightness_loop_with_a_fake_sensor(self):
        class FakeSensor:
            def __init__(self, levels):
                self.levels, self.claimed, self.released = list(levels), False, False

            def present(self):
                return True

            def claim(self):
                self.claimed = True
                return True

            def release(self):
                self.released = True

            def read(self):
                return self.levels.pop(0) if len(self.levels) > 1 else self.levels[0]
        state = {'pct': 40}
        sets = []

        def run(cmd, timeout=10, env=None):
            if cmd[:2] == ['brightnessctl', '-m']:
                return 0, 'intel_backlight,backlight,%d,%d%%,1000\n' % (state['pct'] * 10, state['pct']), ''
            if cmd[:2] == ['brightnessctl', '-q']:
                state['pct'] = int(cmd[-1].rstrip('%'))
                sets.append(state['pct'])
                return 0, '', ''
            return 127, '', ''
        status = os.path.join(self.d.name, 'ab.json')
        sensor = FakeSensor([1000] * 8)
        pathlib.Path(hs.SAVED).write_text(json.dumps({'version': 1, 'autoBrightness': True, 'autoBrightnessOffset': -10}))
        with mock.patch.object(hs, 'light_source', return_value=('iio-sensor-proxy', sensor)), mock.patch.object(hs, 'RUN', run), \
                mock.patch.object(hs, 'have', return_value=True), mock.patch.object(hs, 'STATUS', status), mock.patch.object(hs.time, 'sleep'):
            hs.autobright(rounds=8)
            self.assertTrue(sensor.claimed and sensor.released)
            self.assertTrue(sets and all(abs(b - a) <= hs.AutoBright.STEP for a, b in zip([40] + sets, sets)))   # it fades
            self.assertAlmostEqual(state['pct'], hs.lux_to_pct(1000) - 10, delta=3)
            # Someone sets 20 % by hand partway through: it's left there while the light stays.
            sensor2 = FakeSensor([1000] * 30)
            with mock.patch.object(hs, 'light_source', return_value=('iio-sensor-proxy', sensor2)):
                ticks = {'n': 0}
                real_tick = hs.AutoBright.tick

                def tick(self, lx, current, offset=0):
                    ticks['n'] += 1
                    if ticks['n'] == 4:
                        state['pct'] = 20          # the brightness keys
                        current = 20
                    return real_tick(self, lx, current, offset)
                with mock.patch.object(hs.AutoBright, 'tick', tick):
                    hs.autobright(rounds=12)
            self.assertEqual(state['pct'], 20)
            self.assertTrue(json.loads(pathlib.Path(status).read_text()).get('running') is False)
            # Turned off in Settings: the loop ends at once and sets nothing.
            hs.save_entries({}, {'autoBrightness': False})
            n = len(sets)
            hs.autobright(rounds=5)
            self.assertEqual(len(sets), n)
        sv = json.loads(pathlib.Path(hs.SAVED).read_text())
        self.assertEqual(sv['autoBrightnessOffset'], -10)       # the loop never writes your settings

    def test_auto_brightness_option_saves_through_the_helpers(self):
        pathlib.Path(hs.SAVED).write_text(json.dumps({'version': 1, 'outputs': {'DP-1': {'scale': 1.5}}, 'autoBrightness': True}))
        with mock.patch.object(hs, 'light_source', return_value=('iio-sensor-proxy', object())), mock.patch.object(hs, 'wm', return_value='labwc'), \
                mock.patch.object(hs, 'info', return_value={}), contextlib.redirect_stdout(io.StringIO()):
            hs.main(['hearth-screens', 'option', json.dumps({'autoBrightnessOffset': 75})])
        sv = hs.saved()
        self.assertEqual((sv['autoBrightness'], sv['autoBrightnessOffset'], sv['outputs']['DP-1']['scale']), (True, 40, 1.5))
        self.assertTrue(os.path.exists(hs.SAVED + '.bak'))
        with mock.patch.object(hs, 'light_source', return_value=('iio-sensor-proxy', object())):
            st = hs.auto_status(sv)
        self.assertEqual((st['capable'], st['on'], st['offset']), (True, True, 40))
        t = (SHELL / 'SetDisplay.qml').read_text()
        for part in ('title: "How bright"', 'page.option({ autoBrightnessOffset: page.autoOffWant }, "")', 'visible: !!page.autoB && page.autoB.capable',
                     'Auto-brightness is paused'):
            self.assertIn(part, t)

    def test_page_and_switcher(self):
        t = (SHELL / 'SetDisplay.qml').read_text()
        for part in ('title: "Make this the main screen"', 'title: "Colour intensity"', 'title: "Colour accuracy"', 'title: "Calibrate HDR"',
                     'title: "Extended Dynamic Range"', 'title: "Overscan"', 'title: "RGB range"', 'title: "Scaling X11 apps"',
                     'title: "Sticky edge between screens"', 'title: "Sticky corners"', 'title: "Virtual desktops"', 'title: "Auto-brightness"',
                     '"Brightness · " + modelData.label', 'label: "Sunset to sunrise"', 'title: "Fade"', 'HdrCalibrate { id: cal; page: page }'):
            self.assertIn(part, t)
        # What can leave a screen unusable waits for Keep.
        for k in ('overscan: page.overWant', 'rgbRange: v', 'edr: v'):
            self.assertIn(k, t)
        self.assertIn('page.tryChange(c)', t[t.index('title: "RGB range"'):t.index('title: "RGB range"') + 600])
        sw = (SHELL / 'ScreenSwitcher.qml').read_text()
        for part in ('"This screen only"', '"Duplicate"', '"Extend"', '"Second screen only"', 'job.run(["plan", c.mode]', 'sw.countdown = 15',
                     'sw.back(true)', 'job.run(["keep", JSON.stringify(names)]'):
            self.assertIn(part, sw)
        self.assertIn('Loader { source: "ScreenSwitcher.qml" }', (SHELL / 'shell.qml').read_text())
        keys = json.loads((SHELL.parent / 'kwin' / 'windows' / 'shortcuts.json').read_text())
        self.assertTrue(any(s['keys'] == 'Meta+P' and s['labwc'] == 'W-p' and 'screens switcher' in s['run'] for s in keys['shortcuts']))
        self.assertIn('<keybind key="W-p">', (SHELL.parent / 'labwc' / 'rc.xml').read_text())
        cal = (SHELL / 'HdrCalibrate.qml').read_text()
        self.assertIn('page.countdown = 15', cal)         # Finish goes through Keep these settings?


class Screenshots(unittest.TestCase):
    def test_raw_pictures_become_pngs(self):
        data = bytes([10, 20, 30, 255, 10, 20, 30, 255]) + b'\0' * 8 + bytes([1, 2, 3, 255, 4, 5, 6, 255]) + b'\0' * 8
        rgb = hs.raw_to_rgb(data, 2, 2, 16, 6)
        self.assertEqual(rgb, bytes([30, 20, 10, 30, 20, 10, 3, 2, 1, 6, 5, 4]))
        self.assertEqual(hs.raw_to_rgb(bytes([1, 2, 3, 4]), 1, 1, 4, 17), bytes([1, 2, 3]))
        self.assertEqual(hs.raw_to_rgb(bytes([0, 1, 0, 2, 0, 3, 0, 4]), 1, 1, 8, 26), bytes([1, 2, 3]))
        v = (3 << 30) | (1023 << 20) | (512 << 10) | 4              # RGB30: R max, G half, B ~0
        self.assertEqual(hs.raw_to_rgb(struct.pack('<I', v), 1, 1, 4, 22), bytes([255, 128, 1]))
        p = hs.png(2, 2, rgb)
        self.assertTrue(p.startswith(b'\x89PNG\r\n\x1a\n'))
        w, h = struct.unpack('>II', p[16:24])
        self.assertEqual((w, h), (2, 2))
        idat = p[p.index(b'IDAT') + 4:p.index(b'IEND') - 8]
        self.assertEqual(zlib.decompress(idat), b'\0' + rgb[:6] + b'\0' + rgb[6:])
        with self.assertRaises(hs.Refused):
            hs.raw_to_rgb(b'', 1, 1, 4, 99)

    def test_grim_style_arguments(self):
        o = hs.parse_shot_args(['-g', '10,20 300x200', '-t', 'png', '/tmp/a.png'])
        self.assertEqual((o['geometry'], o['file']), ((10, 20, 300, 200), '/tmp/a.png'))
        self.assertEqual(hs.parse_shot_args(['-o', 'DP-1', '-c', '-'])['output'], 'DP-1')
        with self.assertRaises(hs.Refused):
            hs.parse_shot_args(['-g', 'nonsense', 'x.png'])


class Grab(unittest.TestCase):
    """Capture's grab: MODE [--screen N] [--geometry X,Y,W,H] [--pointer] --out FILE; exit 3 when there's no window."""
    def test_arguments(self):
        o = hs.parse_grab_args(['area', '--geometry', '10,20,300,200', '--pointer', '--out', '/tmp/a.png'])
        self.assertEqual((o['mode'], o['geometry'], o['cursor'], o['out']), ('area', (10, 20, 300, 200), True, '/tmp/a.png'))
        for bad in (['nope', '--out', 'x'], ['screen'], ['area', '--out', 'x'], ['area', '--geometry', '1,2,0,4', '--out', 'x'], ['all', '--out', 'x', '--what']):
            with self.assertRaises(hs.Refused):
                hs.parse_grab_args(bad)

    def test_labwc_uses_grim(self):
        with mock.patch.object(hs, 'RUN', FakeRun(wlr=WLR)):
            outs = hs.read('labwc')
        g = lambda *a: hs.grim_args(hs.parse_grab_args(list(a)), outs)
        self.assertEqual(g('all', '--out', 'f.png'), ['grim', 'f.png'])
        self.assertEqual(g('screen', '--screen', '2', '--pointer', '--out', '-'), ['grim', '-c', '-o', 'eDP-1', '-'])
        self.assertEqual(g('screen', '--screen', 'DP-1', '--out', 'f.png'), ['grim', '-o', 'DP-1', 'f.png'])
        self.assertEqual(g('screen', '--out', 'f.png'), ['grim', '-o', 'DP-1', 'f.png'])
        self.assertEqual(g('area', '--geometry', '5,6,70,80', '--out', 'f.png'), ['grim', '-g', '5,6 70x80', 'f.png'])
        self.assertEqual(g('window', '--geometry', '5,6,70,80', '--out', 'f.png'), ['grim', '-g', '5,6 70x80', 'f.png'])
        with self.assertRaises(hs.NoWindow):
            g('window', '--out', 'f.png')
        with self.assertRaises(hs.Refused):
            g('screen', '--screen', '9', '--out', 'f.png')

    def test_no_window_exits_3(self):
        with mock.patch.dict(os.environ, {'HEARTH_WM': 'labwc'}), self.assertRaises(SystemExit) as e, \
                contextlib.redirect_stdout(io.StringIO()):
            hs.main(['hearth-screens', 'grab', 'window', '--out', '/tmp/x.png'])
        self.assertEqual(e.exception.code, 3)


class Wiring(unittest.TestCase):
    """The pages and helpers use hearth-screens / hearth-screen-power, and labwc keeps working as before."""
    def test_nothing_calls_wlr_randr_or_wlopm_or_grim_directly_any_more(self):
        for f in ('SetDisplay.qml', 'hearth-gamemode-display', 'hearth-resmon', 'IdlePower.qml', 'SetPower.qml', 'ServerDash.qml'):
            t = (SHELL / f).read_text()
            self.assertNotIn('["wlr-randr"', t, f)
            self.assertNotIn("['wlr-randr', '--json']", t, f)
            self.assertNotIn('["wlopm"', t, f)
            self.assertNotIn('wlopm --off', t, f)
        o = (SHELL.parent / 'open.sh').read_text()
        self.assertNotIn('grim "$SHOT"', o)
        self.assertIn('grab "$SHOT"', o)
        self.assertIn('else grim "$@"; fi', o)

    def test_display_page_keeps_its_mockup_parts_and_says_what_needs_kwin(self):
        t = (SHELL / 'SetDisplay.qml').read_text()
        for s in ('page.gmode && page.gmode.hdr ? "HDR on in Game Mode" : "HDR off"', 'SetHeader { text: "Related" }',
                  'onActivated: SettingsNav.openPage("gameperf")', 'countdown > 0 ? "Keep these settings" : ""',
                  'checked: Settings.nightLight; onToggled: v => Settings.nightLight = v',
                  'helper: "hearth-screens"', 'readonly property string needsKwin: "Needs Mantel on KWin"',
                  'title: "HDR on the desktop"', 'title: "Variable refresh rate (VRR / FreeSync)"', 'title: "Allow tearing in full-screen games"',
                  'title: "Rotation"', 'title: "Colour profile"', 'title: "Show the same thing on both screens (mirror)"', 'title: "How warm"',
                  'page.revert(true)', "the new ones weren't kept within 15 seconds"):
            self.assertIn(s, t)

    def test_night_light_follows_the_compositor(self):
        t = (SHELL / 'shell.qml').read_text()
        self.assertIn('running: !night.kwin && night.available && Settings.nightLight', t)
        self.assertIn('"/hearth-screens", "night"', t)
        s = (SHELL / 'Settings.qml').read_text()
        for k in ('nightTemp', 'nightMode', 'nightFrom', 'nightTo'):
            self.assertIn('on%sChanged: save("%s")' % (k[0].upper() + k[1:], k), s)

    def test_screenshot_trust_for_kwin(self):
        d = (REPO / 'os' / 'system_files' / 'usr' / 'share' / 'applications' / 'hearth-screenshot.desktop').read_text()
        self.assertIn('X-KDE-DBUS-Restricted-Interfaces=org.kde.KWin.ScreenShot2', d)
        self.assertIn('Exec=/usr/libexec/nimbus/hearth-shot-python ', d)
        w = (REPO / 'os' / 'system_files' / 'usr' / 'bin' / 'hearth-screenshot').read_text()
        self.assertIn('py=/usr/libexec/nimbus/hearth-shot-python', w)
        self.assertIn('exec grim "$@"', w)
        b = (REPO / 'os-bases' / 'peppermint' / 'build.sh').read_text()
        self.assertIn('/usr/libexec/nimbus/hearth-shot-python', b)
        self.assertIn('optional libkscreen-bin', b)


if __name__ == '__main__':
    unittest.main()
