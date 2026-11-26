#!/usr/bin/env python3
"""The board's Hearth Remap (board 10) as Input Remapper keeps it: presets in its own format
(~/.config/input-remapper-2/presets/<device>/<profile>.json), its config (keystroke_sleep_ms), and Hearth's own
remap.json (when each profile turns on, notes, the macros). The macros are written exactly as Hearth compiles them
(ir.rs Macro::compile), so the app reads them back as its macros.

  remap-fixture.py CFG FIXTURE.json      make them
  remap-fixture.py --check CFG           exit 1 if any file changed since (opening the app must write nothing)
"""
import hashlib
import json
import os
import sys

FARM = 'key(b).wait(50).key(f).key(BTN_LEFT).wait(100).key(Escape)'
ARCHERS = 'modify(Control_L, key(a))'
SORT = 'modify(Super_L, key(e))'
QUICK = 'key(F5)'
SIGN = 'key(t).key(h).key(a).key(n).key(k).key(s)'


def k(code, sym, target='keyboard'):
    return {'input_combination': [{'type': 1, 'code': code}], 'target_uinput': target, 'output_symbol': sym, 'mapping_type': 'key_macro'}


def a(code, th, sym):
    return {'input_combination': [{'type': 3, 'code': code, 'analog_threshold': th}], 'target_uinput': 'keyboard', 'output_symbol': sym, 'mapping_type': 'key_macro'}


def pointer(x, y):
    return [{'input_combination': [{'type': 3, 'code': c, 'analog_threshold': 0}], 'target_uinput': 'mouse', 'output_type': 2, 'output_code': r,
             'mapping_type': 'analog', 'deadzone': 0.08, 'gain': 1.28, 'expo': 0.0} for c, r in ((x, 0), (y, 1))]


DPAD = [a(17, -1, 'modify(Control_L, key(1))'), a(16, 1, 'modify(Control_L, key(2))'), a(17, 1, 'modify(Control_L, key(3))'), a(16, -1, 'modify(Control_L, key(4))')]
ARROWS = [a(0, -10, 'Left'), a(0, 10, 'Right'), a(1, -10, 'Up'), a(1, 10, 'Down')]
XBOX = 'Xbox Wireless Controller'
PRESETS = {
    (XBOX, 'Everywhere'): [k(305, 'BTN_RIGHT', 'mouse')] + pointer(3, 4) + DPAD + [k(314, QUICK, 'keyboard + mouse')],
    (XBOX, 'Battle for Middle-earth II'): [k(310, 'Shift_L'), k(307, FARM, 'keyboard + mouse'), k(304, 'BTN_LEFT', 'mouse'), k(305, 'BTN_RIGHT', 'mouse')] + ARROWS + pointer(3, 4) + DPAD + [k(314, 'F12')],
    (XBOX, 'Desktop'): [k(304, 'BTN_LEFT', 'mouse'), k(305, 'BTN_RIGHT', 'mouse')] + pointer(0, 1),
    (XBOX, 'Hearth Files'): [k(304, 'BTN_LEFT', 'mouse'), k(305, 'BTN_RIGHT', 'mouse'), k(311, 'Delete')] + pointer(0, 1),
    (XBOX, 'Noclip Protocol'): [k(310, 'q'), k(311, 'e')],
    ('8BitDo SN30 Pro', 'Battle for Middle-earth II'): [k(308, ARCHERS, 'keyboard + mouse')],
    ('Gaming mouse', 'Hearth Files'): [k(276, SORT, 'keyboard + mouse')],
    ('Keyboard', 'Everywhere'): [k(67, SIGN, 'keyboard + mouse')],
}
REMAP = {
    'profiles': {
        XBOX + '/Everywhere': {'when': 'everywhere'},
        XBOX + '/Battle for Middle-earth II': {'when': 'game:2450', 'label': 'BFME II', 'short': 'BFME II', 'from': "the Game Fixer's game list", 'deadzone': 8, 'speed': 60,
                                               'show': True, 'holdOff': True, 'notes': {'btn:310': {'note': 'add to group'}, 'ls': {'note': 'camera', 'says': 'Move the camera'}}},
        XBOX + '/Desktop': {'when': 'desktop', 'about': 'controller as a mouse'},
        XBOX + '/Hearth Files': {'when': 'app:hearth-files', 'short': 'Files'},
        XBOX + '/Noclip Protocol': {'when': 'game:3417950'},
        '8BitDo SN30 Pro/Battle for Middle-earth II': {'when': 'game:2450', 'short': 'BFME II'},
        'Gaming mouse/Hearth Files': {'when': 'app:hearth-files', 'short': 'Files'},
    },
    'macros': [
        {'id': 'farm', 'name': 'Build farm', 'colour': '#ff8a3d', 'mode': 'once', 'steps': [['press', 'b'], ['wait', 50], ['press', 'f'], ['click', 'left'], ['wait', 100], ['press', 'Escape']]},
        {'id': 'archers', 'name': 'Select all archers', 'colour': '#5ec8ff', 'mode': 'once', 'steps': [['press', 'Control_L + a']]},
        {'id': 'sort', 'name': 'Sort my Downloads', 'colour': '#9be36b', 'mode': 'once', 'steps': [['press', 'Super_L + e']]},
        {'id': 'quick', 'name': 'Quick save', 'colour': '#e36bd3', 'mode': 'once', 'steps': [['press', 'F5']]},
        {'id': 'sign', 'name': 'Type my sign-off', 'colour': '#ffd36b', 'mode': 'once', 'steps': [['type', 'thanks']]},
    ],
}
FIX = {'devices': [{'name': XBOX, 'kind': 'controller', 'sub': 'Bluetooth · player 1'}, {'name': 'Keyboard', 'kind': 'keyboard', 'sub': 'USB · 104 keys'},
                   {'name': 'Gaming mouse', 'kind': 'mouse', 'sub': 'USB · 7 buttons'}, {'name': '8BitDo SN30 Pro', 'kind': 'controller', 'sub': 'Bluetooth · player 2'},
                   {'name': "ROG Ally's buttons", 'kind': 'controller', 'sub': 'built in · back paddles'}],
       'state': {XBOX: 'RUNNING'}}


def files(cfg):
    out = {}
    for root, _, fs in os.walk(cfg):
        for f in fs:
            p = os.path.join(root, f)
            out[p] = hashlib.sha256(open(p, 'rb').read()).hexdigest()
    return out


if sys.argv[1] == '--check':
    cfg = sys.argv[2]
    was = json.load(open(cfg + '.sums'))
    now = files(cfg)
    bad = sorted(set(was.items()) ^ set(now.items()))
    if bad:
        print('changed: %s' % ', '.join(sorted({p for p, _ in bad})))
        sys.exit(1)
    sys.exit(0)
cfg, fix = sys.argv[1], sys.argv[2]
os.makedirs(cfg, exist_ok=True)
for (dev, name), maps in PRESETS.items():
    d = os.path.join(cfg, 'input-remapper-2', 'presets', dev)
    os.makedirs(d, exist_ok=True)
    json.dump(maps, open(os.path.join(d, name + '.json'), 'w'), indent=2)
json.dump({'version': '2.0.1', 'autoload': {}, 'macros': {'keystroke_sleep_ms': 25}}, open(os.path.join(cfg, 'input-remapper-2', 'config.json'), 'w'))
os.makedirs(os.path.join(cfg, 'hearth'), exist_ok=True)
json.dump(REMAP, open(os.path.join(cfg, 'hearth', 'remap.json'), 'w'), indent=1)
json.dump(FIX, open(fix, 'w'))
json.dump(files(cfg), open(cfg + '.sums', 'w'))
