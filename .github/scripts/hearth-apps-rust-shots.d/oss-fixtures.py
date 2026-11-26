#!/usr/bin/python3
"""Made-up data for the open-source apps' pictures (docs/mockups/open-source-apps), the boards' own words and numbers,
so each picture can be laid beside its board. Nothing here is a real account or a real service.

  oss-fixtures.py listen DIR     Hearth Listen's folder (recordings with their words, models, a pretend whisper-cli
                                 and nvidia-smi in DIR/bin for PATH)
  oss-fixtures.py imagine DIR    Hearth Imagine's data, choices and ComfyUI folder (sparse model files)
  oss-fixtures.py passwords DIR  Hearth Passwords: a made-up vault, already opened (HEARTH_PASSWORDS_FIXTURE), its
                                 choices and its own little files (when GitHub was used, when its code was added)
  oss-fixtures.py cloud DIR      Hearth Cloud: made-up answers from Nextcloud (HEARTH_CLOUD_FIXTURE)
  oss-fixtures.py home DIR       Hearth Home: made-up Home Assistant states, rooms, energy (HEARTH_HOME_FIXTURE)
  oss-fixtures.py comfy PORT [idle]   a pretend ComfyUI on 127.0.0.1:PORT: its API (/system_stats, /object_info,
                                 /queue, /history, /view, /prompt) and /ws telling that picture 3 is on step 18 of 28
"""
import json
import os
import sys
import time


def rng(seed):
    """kit.js's little generator, so the bars are the board's bars."""
    s = seed * 9301 + 49297

    def r():
        nonlocal s
        s = (s * 9301 + 49297) % 233280
        return s / 233280
    return r


def bars(seed, n, lo, hi):
    r = rng(seed)
    return [round((lo + (hi - lo) * r()) / 100, 3) for _ in range(n)]


def today_at(h, m):
    t = time.localtime()
    return int(time.mktime((t.tm_year, t.tm_mon, t.tm_mday, h, m, 0, 0, 0, -1)))


def write(p, v):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w', encoding='utf-8') as f:
        json.dump(v, f, ensure_ascii=False)


def sparse(p, size):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'wb') as f:
        f.truncate(size)


def fake(p, body):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write('#!/bin/sh\n' + body + '\n')
    os.chmod(p, 0o755)


def listen(d):
    T = [("1:31", 1, "This is Hearth OS one point eighteen. Everything you see here is yours to change."),
         ("1:38", 1, "Start with the desktop. Panels go on any edge, or float anywhere you like."),
         ("1:45", 2, "And it's not just the desktop."),
         ("1:48", 1, "Every Hearth app can be laid out the same way: Files, Settings, Task Manager, Capture."),
         ("1:56", 1, "Right-click any part and pick Customize."),
         ("2:02", 2, "Then press Keep this, and it stays that way. Always."),
         ("2:09", 1, "Your settings never change by themselves."),
         ("2:14", 2, "Next: the Hearth Store, with every part updating on its own."),
         ("2:21", 1, "No system update, no restart. Open the Store and press Update."),
         ("2:29", 1, "And when there is a new Hearth OS, the one before stays startable."),
         ("2:36", 2, "That's Hearth OS one point eighteen.")]
    secs = lambda s: int(s.split(':')[0]) * 60 + int(s.split(':')[1])
    segs = []
    for i, (at, v, text) in enumerate(T):
        a = float(secs(at))
        b = float(secs(T[i + 1][0])) if i + 1 < len(T) else a + 5
        seg = {'start': a, 'end': b, 'text': text, 'speaker': 'VOICE %d' % v}
        if at == '1:48':
            # whisper.cpp's word times (-ojf) for the line being said: "can be laid out the same way" at 1:48.9
            seg['words'] = [108.0, 108.2, 108.45, 108.6, 108.8, 109.0, 109.3, 109.6, 109.9, 110.3, 110.8, 111.4, 112.0, 112.3, 112.9]
        segs.append(seg)
    now = time.time()
    R = [("take3", "1.18 video, voice-over take 3", today_at(1, 12), 252, "Voice Studio", segs, 21),
         ("take2", "1.18 video, voice-over take 2", today_at(1, 2), 260, "Voice Studio", None, 22),
         ("noclip", "Noclip Protocol, guard lines", int(now - 86400), 161, "Hearth Listen", None, 9),
         ("mech", "Mech game ideas (truck)", int(now - 2 * 86400), 368, "Hearth Listen", None, 30),
         ("dad", "Call with Dad about Trainz", int(now - 3 * 86400), 750, "Hearth Listen", None, 55),
         ("ithil", "Ithil voice test", int(now - 4 * 86400), 48, "Voice Studio", None, 4),
         ("gravesleep", "Gravesleep intro", int(now - 5 * 86400), 115, "Voice Studio", None, 10),
         ("store", "Store video, scratch", int(now - 6 * 86400), 182, "Voice Studio", None, 15)]
    for n, (rid, title, when, dur, src, words, took) in enumerate(R):
        rd = os.path.join(d, 'recordings', rid)
        write(os.path.join(rd, 'meta.json'), {'title': title, 'created': when, 'source': src, 'secs': dur, 'peaks': bars(11 + n, 160, 10, 95)})
        if words is None:
            words = [{'start': 0.0, 'end': 4.0, 'text': title + '.', 'speaker': ''}]
        write(os.path.join(rd, 'words.json'), {'language': 'en', 'duration': float(dur), 'took': took, 'model': 'large-v3-turbo', 'engine': 'whisper.cpp v1.8.7', 'segments': words})
    # whisper.cpp's models (sparse files of their real sizes: nothing is downloaded here)
    sparse(os.path.join(d, 'models', 'ggml-large-v3-turbo.bin'), 1624555275)
    sparse(os.path.join(d, 'models', 'ggml-small.en-tdrz.bin'), 487601967)
    fake(os.path.join(d, 'bin', 'whisper-cli'), 'echo "usage: whisper-cli [options] file0 (pretend, for the pictures)"')
    fake(os.path.join(d, 'fakes', 'nvidia-smi'), 'echo "NVIDIA GeForce RTX 5070"')


CABIN = 'A cozy cabin in a snowy pine forest at night, warm orange light in the windows, a hearth glow, wide shot, soft film look'
CABIN_TITLE = 'A cozy cabin in a snowy pine forest at night, w…'


def imagine(d):
    data = os.path.join(d, 'data', 'hearth', 'imagine')
    draft = {'title': CABIN_TITLE, 'prompt': CABIN, 'leaveOut': 'people, text, blurry', 'look': 'Film', 'size': '16:9', 'count': 4, 'steps': 28}
    write(os.path.join(data, 'draft.json'), draft)
    recent = [draft] + [{'title': t, 'prompt': p, 'leaveOut': 'text', 'look': l, 'size': sz, 'count': 4, 'steps': 28} for t, p, l, sz in (
        ('Noclip Protocol poster, yellow halls', 'Noclip Protocol poster, yellow halls, endless office rooms, buzzing lights', 'Game art', '16:9'),
        ('Hearth wallpaper, ember sky, 9:16', 'Hearth wallpaper, ember sky, glowing coals, dark warm gradient', 'Wallpaper', '9:16'),
        ('Quad mech in fog, concept art', 'Quad mech in fog, concept art, four slow legs, dawn light', 'Game art', '16:9'))]
    write(os.path.join(data, 'recent.json'), recent)
    write(os.path.join(data, 'speed.json'), {'secsPerStep': 0.125})
    write(os.path.join(d, 'config', 'nimbus', 'imagine.json'), {'view': 'both', 'look': 'Film', 'size': '16:9', 'count': 4, 'steps': 28, 'seed': 482113, 'model': 'hearth-xl-1.0.safetensors'})
    c = os.path.join(d, 'ComfyUI', 'models')
    sparse(os.path.join(c, 'checkpoints', 'hearth-xl-1.0.safetensors'), 6_900_000_000)
    sparse(os.path.join(c, 'checkpoints', 'Fast 1-step.safetensors'), 4_100_000_000)
    sparse(os.path.join(c, 'upscale_models', 'Upscaler 4×.pth'), 64_000_000)


def passwords(d):
    it = lambda i, n, user='', pw='', kind='login', folder='', **k: dict({'id': i, 'name': n, 'user': user, 'password': pw, 'kind': kind, 'folder': folder}, **k)
    twice1, twice2 = 'Mosswood-Lantern-77', 'Harbor$Cobalt2026'
    items = [
        it('gh', 'GitHub', 'Dragongamer9', 'r7#Vq4mLp!8eZw2Tn%kHs9Yb', folder='Hearth', totp='otpauth://totp/GitHub:Dragongamer9?secret=JBSWY3DPEHPK3PXP&issuer=GitHub',
           uri='https://github.com', notes='Owner account for Hearth-Organization. Recovery codes are in the secure note "GitHub recovery".',
           changed='2026-11-15T21:04:00Z', made='2026-08-19T18:30:00Z', history=[{'when': '2026-11-15T21:04:00Z', 'what': 'Password changed'}]),
        it('hs', 'Hearth Store developer', 'Dragongamer9', 'k9!Pz2#Wq7mR4tLx8vN3', folder='Hearth'),
        it('wifi', 'Home Wi-Fi', 'WPA3', twice2, kind='wifi', folder='Home'),
        it('router', 'Router', 'admin page', twice2, folder='Home'),
        it('ts', 'Tailscale', 'sign-in with GitHub', 'Tq8#vL2!pZ6mW4xR9sK1', folder='Hearth'),
        it('home', 'Hearth Home', 'on the Nvidia PC', 'Hm4!qZ8#rT2vW6pL9xN3', folder='Home'),
        it('epic', 'Epic', 'games', twice1, folder='Games'),
        it('gog', 'GOG', 'games', twice1, folder='Games'),
        it('power', 'Electric company', 'account', 'sunny123'),
        it('truck', 'Truck insurance', 'account', 'Tr8#uck!Ins2Wq6Lp4', notes='Truck insurance offers 2-step codes in its account page.'),
        it('card', 'Bank card', '', '', kind='card', last4='4821'),
        it('dad', 'Trainz account', "Dad's", 'Dz7!Tr2#ain4Qw9', folder='Shared with Dad'),
    ]
    write(os.path.join(d, 'vault.json'), {'syncedAgo': 120, 'items': items, 'folders': ['Hearth', 'Games', 'Home', 'Shared with Dad'],
                                          'devices': [{'name': 'Phone', 'type': 0, 'identifier': 'p'}, {'name': 'Ally', 'type': 8, 'identifier': 'a'}]})
    data = os.path.join(d, 'data', 'hearth', 'passwords')
    write(os.path.join(data, 'events.json'), {'gh': [{'when': '2026-10-02T20:00:00Z', 'what': 'Code added'}]})
    write(os.path.join(data, 'used.json'), {'gh': int(time.time()) - 3600})
    write(os.path.join(d, 'config', 'nimbus', 'passwords.json'), {'email': 'owner@example.com', 'server': 'http://nvidia-pc.example.ts.net:8222', 'fillBrowsers': False, 'fillAsk': True, 'fillApps': True,
                                                                  'makerHow': 'password', 'makerLength': 24, 'makerNoLookalikes': False})


def cloud(d):
    m = 60
    recent = [('Gravesleep story outline.odt', 'Projects/Gravesleep', 3 * m, 'Laptop'), ('Center console v6.3mf', 'Projects/Truck', 36 * m, 'Nvidia PC'),
              ('Hearth budget 2026.ods', 'Documents', 26 * 3600, 'Phone'), ('Mech leg sketch.png', 'Projects/Quad Mech', 30 * 3600, 'ROG Ally'),
              ('Trainz route notes.txt', 'Shared/Trainz routes', 2 * 86400 + 3600, "Dad's PC"), ('Truck insurance card.pdf', 'Documents/Truck', 4 * 86400, 'Phone'),
              ('Noclip level ideas.md', 'Projects/Noclip', 5 * 86400, 'Laptop')]
    ev = [{'title': 'Thanksgiving', 'allDay': True, 'at': 0, 'mins': 1440, 'calendar': 'Family', 'place': "at Dad's", 'color': '#ffd36b'},
          {'title': 'Dinner', 'at': 14 * 3600, 'mins': 120, 'note': 'Bring the pie · shared by Dad', 'color': '#ff8a3d'},
          {'title': 'Work, overnight shift', 'at': 22 * 3600, 'mins': 480, 'repeats': 'FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR', 'color': '#5ec8ff'},
          {'title': 'Parts pickup', 'at': -2 * 86400 + 10 * 3600, 'mins': 30, 'color': '#5ec8ff'},
          {'title': 'Print the console lid', 'at': 2 * 86400 + 15 * 3600, 'mins': 60, 'color': '#5ec8ff'}]
    contacts = [{'name': 'Dad', 'phone': True, 'email': True, 'groups': ['family']}, {'name': 'Pizza place', 'phone': True, 'address': True},
                {'name': 'Work, shift lead', 'phone': True}, {'name': 'Truck shop', 'phone': True, 'address': True}, {'name': 'Sam (Noclip tester)', 'other': ['Discord'], 'email': True}]
    shares = [{'id': '1', 'name': 'Trainz routes', 'with': 'Dad', 'kind': 0, 'canChange': True}, {'id': '2', 'name': 'Hearth OS ISO test', 'kind': 3, 'expires': '2026-12-03 00:00:00', 'url': 'http://nvidia-pc:8090/s/x1'},
              {'id': '3', 'name': 'Truck console prints', 'kind': 3, 'url': 'http://nvidia-pc:8090/s/x2'}, {'id': '4', 'name': 'Family calendar', 'with': '4 people', 'kind': 0}]
    devices = [{'name': 'Phone', 'what': 'files, calendar, contacts', 'when': '2 min ago'}, {'name': 'Laptop · Hearth OS', 'what': 'Documents, Projects', 'when': 'just now'},
               {'name': 'ROG Ally', 'what': 'Documents only', 'when': '1 h ago'}, {'name': "Dad's PC", 'what': 'shared folder "Trainz routes"', 'shared': True}]
    write(os.path.join(d, 'cloud.json'), {'recent': [{'name': n, 'where': w, 'ago': a, 'from': f} for n, w, a, f in recent], 'events': ev, 'contacts': contacts + [{'name': 'Contact %d' % i, 'phone': True} for i in range(209)],
                                          'shares': shares + [{'id': str(9 + i), 'name': 'Shared %d' % i, 'kind': 3, 'url': 'x'} for i in range(2)], 'devices': devices,
                                          'used': 1_240_000_000_000, 'total': 4_000_000_000_000, 'files': 48210, 'bin': 12})
    write(os.path.join(d, 'config', 'nimbus', 'cloud.json'), {'server': 'http://nvidia-pc.example.ts.net:8090', 'user': 'me', 'pinnedContacts': ['Dad', 'Pizza place', 'Work, shift lead', 'Truck shop', 'Sam (Noclip tester)']})


def home(d):
    L = lambda i, n, on, bri=None, **a: dict({'entity_id': 'light.' + i, 'state': 'on' if on else 'off', 'attributes': dict({'friendly_name': n}, **({'brightness': round(bri * 2.55), 'color_mode': 'color_temp'} if bri else {}), **a)})
    st = [L('ceiling', 'Ceiling', True, 60), L('couch_lamp', 'Lamp by the couch', True, 35), L('tv_backlight', 'TV backlight', True, None, hearth_note='follows the picture', hearth_gradient=True),
          L('hallway', 'Hallway', False), L('porch', 'Porch', False, None, hearth_note='on at sunset'), dict(L('garage', 'Garage', True), agoSecs=7300),
          L('bedroom', 'Bedroom light', True, 20), L('office_desk', 'Office desk', True, 70, hearth_room_note='Hearth Lights'), L('office_shelf', 'Office shelf', True, 50),
          L('kitchen', 'Kitchen', False), L('garage_2', 'Garage bench', False, None, hearth_room_note='door shut')]
    st += [{'entity_id': 'climate.hallway', 'state': 'heat', 'attributes': {'friendly_name': 'Hallway', 'temperature': 70, 'current_temperature': 67, 'hvac_action': 'heating', 'min_temp': 45, 'max_temp': 95}},
           {'entity_id': 'camera.driveway', 'state': 'streaming', 'attributes': {'friendly_name': 'Driveway'}}, {'entity_id': 'camera.front_door', 'state': 'streaming', 'attributes': {'friendly_name': 'Front door'}},
           {'entity_id': 'binary_sensor.driveway_motion', 'state': 'off', 'last_changed': '', 'attributes': {'friendly_name': 'Driveway motion', 'device_class': 'motion', 'hearth_seen': 'a cat'}, 'agoSecs': 1920},
           {'entity_id': 'switch.xbox_and_tv', 'state': 'on', 'attributes': {'friendly_name': 'Xbox and TV'}}, {'entity_id': 'sensor.xbox_and_tv_power', 'state': '96', 'attributes': {}},
           {'entity_id': 'switch.space_heater', 'state': 'on', 'attributes': {'friendly_name': 'Space heater'}}, {'entity_id': 'sensor.space_heater_power', 'state': '300', 'attributes': {}},
           {'entity_id': 'switch.3d_printer', 'state': 'on', 'attributes': {'friendly_name': '3D printer'}}, {'entity_id': 'sensor.3d_printer_power', 'state': '16', 'attributes': {}},
           {'entity_id': 'switch.coffee_maker', 'state': 'off', 'attributes': {'friendly_name': 'Coffee maker', 'hearth_note': 'on at 5:45 AM'}},
           {'entity_id': 'weather.home', 'state': 'clear-night', 'attributes': {'temperature': 41}},
           {'entity_id': 'scene.movie_night', 'state': '2026-11-26T06:50:00+00:00', 'attributes': {'friendly_name': 'Movie night', 'hearth_note': 'lights dim, TV backlight on'}},
           {'entity_id': 'scene.game_night', 'state': '2026-11-20T02:00:00+00:00', 'attributes': {'friendly_name': 'Game night', 'hearth_note': 'Hearth Lights + room'}},
           {'entity_id': 'scene.leaving_for_work', 'state': '2026-11-25T21:30:00+00:00', 'attributes': {'friendly_name': 'Leaving for work', 'hearth_note': 'all off, heat 64°'}},
           {'entity_id': 'scene.bedtime', 'state': '2026-11-25T12:00:00+00:00', 'attributes': {'friendly_name': 'Bedtime', 'hearth_note': 'doors check, porch on'}},
           {'entity_id': 'automation.coffee', 'state': 'on', 'attributes': {'friendly_name': 'Coffee maker on'}},
           {'entity_id': 'automation.heat', 'state': 'on', 'attributes': {'friendly_name': "Heat to 70° before you're home"}},
           {'entity_id': 'automation.porch', 'state': 'on', 'attributes': {'friendly_name': 'Porch light off at sunrise'}}]
    areas = [{'id': 'living_room', 'name': 'Living room', 'entities': ['light.ceiling', 'light.couch_lamp', 'light.tv_backlight', 'light.hallway', 'light.porch', 'light.garage']},
             {'id': 'bedroom', 'name': 'Bedroom', 'entities': ['light.bedroom']}, {'id': 'office', 'name': 'Office', 'entities': ['light.office_desk', 'light.office_shelf']},
             {'id': 'kitchen', 'name': 'Kitchen', 'entities': ['light.kitchen']}, {'id': 'garage', 'name': 'Garage', 'entities': ['light.garage_2']},
             {'id': 'hallway', 'name': 'Hallway', 'entities': ['climate.hallway']}]
    b = bars(21, 24, 10, 90)
    energy = [round(x * 6.4 / sum(b), 3) for x in b]
    write(os.path.join(d, 'home.json'), {'states': st, 'areas': areas, 'energy': energy, 'next': [['automation.coffee', 5 * 3600 + 45 * 60], ['automation.heat', 6 * 3600 + 15 * 60], ['automation.porch', 7 * 3600 + 2 * 60]]})
    write(os.path.join(d, 'config', 'nimbus', 'home.json'), {'room': 'living_room', 'screenName': 'the TV', 'server': 'http://nvidia-pc.example.ts.net:8123'})
    for name, kind in (('camera.driveway', 'drive'), ('camera.front_door', 'door')):
        with open(os.path.join(d, name + '.png'), 'wb') as f:
            f.write(cam_png(kind))


def cam_png(kind, w=540, h=440):
    from PIL import Image, ImageDraw
    import io
    im = Image.new('RGB', (w, h))
    dr = ImageDraw.Draw(im)
    top, mid, bot = ((26, 42, 58), (42, 58, 42), (26, 20, 16)) if kind == 'drive' else ((42, 36, 32), (58, 42, 32), (26, 20, 16))
    for y in range(h):
        t = y / h
        a, b, u = (top, mid, t / .6) if t < .6 else (mid, bot, (t - .6) / .4)
        dr.line([(0, y), (w, y)], fill=tuple(int(a[i] + (b[i] - a[i]) * u) for i in range(3)))
    if kind == 'drive':
        dr.rounded_rectangle([80, h - 200, 380, h - 60], 24, fill=(42, 42, 42))
    else:
        dr.rounded_rectangle([w // 2 - 70, 80, w // 2 + 70, 320], 12, fill=(74, 58, 42), outline=(90, 74, 58), width=6)
    b = io.BytesIO()
    im.save(b, 'PNG')
    return b.getvalue()


def png(kind, w=448, h=256):
    from PIL import Image, ImageDraw, ImageFilter
    import io
    im = Image.new('RGB', (w, h))
    dr = ImageDraw.Draw(im)
    def grad(top, bottom, y0=0, y1=h):
        for y in range(y0, y1):
            t = (y - y0) / max(1, y1 - y0 - 1)
            dr.line([(0, y), (w, y)], fill=tuple(int(top[i] + (bottom[i] - top[i]) * t) for i in range(3)))
    if kind.startswith('cabin'):
        grad((13, 26, 42), (42, 58, 74), 0, int(h * .6))
        grad((232, 238, 242), (154, 168, 178), int(h * .6), h)
        cx = int(w * (.4 if kind == 'cabin1' else .6))
        glow = Image.new('RGB', (w, h))
        ImageDraw.Draw(glow).ellipse([cx - 26, int(h * .66) - 26, cx + 26, int(h * .66) + 26], fill=(255, 150, 70))
        im = Image.composite(glow, im, glow.convert('L').point(lambda v: 255 if v else 0)).filter(ImageFilter.GaussianBlur(2))
    else:
        cols = {'e1': ((200, 180, 74), (106, 90, 32)), 'e2': ((44, 127, 216), (13, 20, 36)), 'e3': ((255, 138, 61), (58, 29, 14)),
                'e4': ((59, 165, 92), (13, 42, 20)), 'e5': ((176, 79, 200), (26, 13, 42)), 'e6': ((109, 100, 93), (26, 22, 20))}[kind]
        grad(*cols)
    b = io.BytesIO()
    im.save(b, 'PNG')
    return b.getvalue()


def comfy(port, idle=False):
    import base64, hashlib, threading
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    from urllib.parse import urlsplit, parse_qs
    def item(num, pid, title):
        return [num, pid, {}, {'hearth_title': title, 'client_id': 'x'}, ['9']]
    queue = {'queue_running': [], 'queue_pending': []} if idle else {
        'queue_running': [item(103, 'p3', CABIN_TITLE)],
        'queue_pending': [item(104, 'p4', CABIN_TITLE), item(105, 'q1', 'Same, in 9:16 for the phone'), item(106, 'q2', 'Noclip Protocol poster, yellow halls')]}
    def done(num, pid, title, img):
        return {'prompt': item(num, pid, title), 'outputs': {'9': {'images': [{'filename': img + '.png', 'subfolder': 'Imagine', 'type': 'output'}]}}, 'status': {'completed': True}}
    hist = {} if idle else {'p1': done(101, 'p1', CABIN_TITLE, 'cabin1'), 'p2': done(102, 'p2', CABIN_TITLE, 'cabin2')}
    for i in range(6):
        hist['e%d' % i] = done(90 + i, 'e%d' % i, 'earlier %d' % i, 'e%d' % (6 - i))
    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass
        def js(self, v, code=200):
            b = json.dumps(v).encode()
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(b)))
            self.end_headers()
            self.wfile.write(b)
        def do_POST(self):
            n = int(self.headers.get('Content-Length') or 0)
            self.rfile.read(n)
            self.js({'prompt_id': 'new%d' % int(time.time() * 1000), 'number': 200, 'node_errors': {}})
        def do_GET(self):
            u = urlsplit(self.path)
            q = parse_qs(u.query)
            if u.path == '/system_stats':
                return self.js({'system': {'os': 'posix', 'comfyui_version': '0.43.2'}, 'devices': [{'name': 'cuda:0 NVIDIA GeForce RTX 5070 : cudaMallocAsync', 'type': 'cuda', 'vram_total': 12884901888}]})
            if u.path == '/object_info/CheckpointLoaderSimple':
                return self.js({'CheckpointLoaderSimple': {'input': {'required': {'ckpt_name': [['hearth-xl-1.0.safetensors', 'Fast 1-step.safetensors']]}}}})
            if u.path == '/object_info/UpscaleModelLoader':
                return self.js({'UpscaleModelLoader': {'input': {'required': {'model_name': [['Upscaler 4×.pth']]}}}})
            if u.path == '/queue':
                return self.js(queue)
            if u.path == '/history':
                return self.js(hist)
            if u.path.startswith('/history/'):
                k = u.path.rsplit('/', 1)[1]
                return self.js({k: hist[k]} if k in hist else {})
            if u.path == '/view':
                b = png(q.get('filename', ['e1.png'])[0].rsplit('.', 1)[0])
                self.send_response(200)
                self.send_header('Content-Type', 'image/png')
                self.send_header('Content-Length', str(len(b)))
                self.end_headers()
                return self.wfile.write(b)
            if u.path == '/ws':
                k = self.headers.get('Sec-WebSocket-Key', '')
                acc = base64.b64encode(hashlib.sha1((k + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest()).decode()
                self.wfile.write(('HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: %s\r\n\r\n' % acc).encode())
                if not idle:
                    m = json.dumps({'type': 'progress', 'data': {'value': 18, 'max': 28, 'prompt_id': 'p3', 'node': '3'}}).encode()
                    self.wfile.write(b'\x81' + bytes([len(m)]) + m)
                    self.wfile.flush()
                time.sleep(600)
                return
            self.js({'error': 'not here'}, 404)
    ThreadingHTTPServer(('127.0.0.1', port), H).serve_forever()


if __name__ == '__main__':
    if sys.argv[1] == 'comfy':
        comfy(int(sys.argv[2]), len(sys.argv) > 3 and sys.argv[3] == 'idle')
    else:
        {'listen': listen, 'imagine': imagine, 'passwords': passwords, 'cloud': cloud, 'home': home}[sys.argv[1]](sys.argv[2])
