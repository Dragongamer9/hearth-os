#!/usr/bin/env python3
"""The board's mouse (board 11) as libratbag's ratbagd describes it on D-Bus (org.freedesktop.ratbag1: the manager, the
device, its five profiles kept on the mouse, their DPI steps, buttons and lights), each object's properties by path, so
Hearth Mouse's own reading code (hosts/hearth-ui/apps/mouse/src/rb.rs) draws it. Plus the battery UPower would say.

  mouse-fixture.py OUT.json
"""
import json
import sys

B = '/org/freedesktop/ratbag1'
DEV = B + '/device/m0'
CHOICES = list(range(100, 26001, 50))
out = {B: {'Devices': [DEV], 'APIVersion': 2}, 'battery': 76}
profiles = []
for pi, name in enumerate(['Everyday', 'BFME II', 'Shooters', 'Hearth CAD', '']):
    pp = B + '/profile/m0/p%d' % pi
    profiles.append(pp)
    res, buttons, leds = [], [], []
    for ri, dpi in enumerate([400, 800, 1600, 3200, 6400]):
        rp = B + '/resolution/m0/p%d/r%d' % (pi, ri)
        res.append(rp)
        out[rp] = {'Index': ri, 'Resolution': [dpi, dpi], 'IsActive': ri == 2, 'IsDefault': ri == 2, 'IsDisabled': False, 'Resolutions': CHOICES}
    for bi, m in enumerate([[1, 1], [1, 2], [1, 3], [1, 4], [1, 5], [2, (1 << 30) + 7]]):
        bp = B + '/button/m0/p%d/b%d' % (pi, bi)
        buttons.append(bp)
        out[bp] = {'Index': bi, 'Mapping': m, 'ActionTypes': [0, 1, 2, 3, 4]}
    for li, (mode, col, kind) in enumerate([(1, [255, 138, 61], 1), (3, [176, 79, 200], 2)]):
        lp = B + '/led/m0/p%d/l%d' % (pi, li)
        leds.append(lp)
        out[lp] = {'Index': li, 'Mode': mode, 'Modes': [0, 1, 2, 3], 'Color': col, 'ColorDepth': 1, 'EffectDuration': 1000, 'Brightness': 255, 'Type': kind}
    out[pp] = {'Index': pi, 'Name': name, 'Enabled': bool(name), 'IsActive': pi == 0, 'IsDirty': False, 'ReportRate': 1000, 'ReportRates': [125, 500, 1000, 2000, 4000],
               'AngleSnapping': 0, 'Resolutions': res, 'Buttons': buttons, 'Leds': leds}
out[DEV] = {'Name': 'Gaming mouse (USB receiver)', 'Model': 'usb:046d:c539:0', 'Profiles': profiles}
json.dump(out, open(sys.argv[1], 'w'))
