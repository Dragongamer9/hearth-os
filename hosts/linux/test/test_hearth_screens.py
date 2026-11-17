"""hearth-screens (1.14, docs/SCREENS.md): Desktop Mode's screens under labwc (wlr-randr) and KWin (kscreen-doctor),
the saved settings (screens.json + display.sh: never lost, never written by reading, a damaged file never replaced
with defaults), restore at start (no unlisted mode, never every screen off), KWin's Night Light / tearing in kwinrc,
screenshots' pictures, and the pages and helpers that use it."""
import importlib.machinery
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
