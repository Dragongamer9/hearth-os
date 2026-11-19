#!/usr/bin/python3
"""Stand-ins for Firehold on Plasma's pictures (firehold-plasma.yml, firehold-plasma-shots.sh), the approved mockups'
sample games, people and devices (docs/mockups/firehold-plasma 01-12; their source firehold.html in hearth-workspace):

  firehold-plasma-fixtures.py HOME OUT    writes, under HOME: Hearth's library as hearth-firehold's cache, each game's
                                          art (the mockup's colours and shapes, drawn here), the Couch home's cards
                                          (OUT/home.json: downloads, controllers, devices, progress), the bars' line
                                          (OUT/sys.json), firehold.json (pins, who's playing), Hearth's profiles, the
                                          media apps' menu entries; and OUT/mpris.py, a stand-in media player.
  firehold-plasma-fixtures.py --low OUT   the same cards with Player 2's controller at 12% (mockup 02's notice)
"""
import json
import math
import os
import random
import sys
import time

M = 60000
H = 60 * M
D = 24 * H

# id, title, source, store, colour 1, colour 2, motif (the mockup's G table)
G = [
    ('beacon:rotwk', 'Rise of the Witch-king', 'beacon', 'Beacon', '#3b2a5a', '#c0562a', 'tower'),
    ('beacon:bfme2', 'Battle for Middle-earth II', 'beacon', 'Beacon', '#24324a', '#8a5a2a', 'tower'),
    ('beacon:bfme1', 'Battle for Middle-earth', 'beacon', 'Beacon', '#3a3a24', '#a8742a', 'ridge'),
    ('beacon:aotr', 'Age of the Ring', 'beacon', 'Beacon', '#4a2020', '#d08a3a', 'tower'),
    ('beacon:conquest', 'LOTR Conquest', 'beacon', 'Beacon', '#202a3a', '#5a7a9a', 'ridge'),
    ('beacon:swgb', 'Galactic Battlegrounds', 'beacon', 'Beacon', '#2a2230', '#b0803a', 'stars'),
    ('beacon:rol', 'Rise of Legends', 'beacon', 'Beacon', '#2a2a20', '#8a7a3a', 'ridge'),
    ('steam:9420', 'Universe at War', 'steam', 'Steam', '#16283a', '#3aa0a0', 'city'),
    ('winapp:trainz', 'Trains 2010', 'winapp', 'Windows app', '#2a3a2a', '#c09050', 'rail'),
    ('steam:553850', 'Helldivers 2', 'steam', 'Steam', '#2a2a20', '#d0b030', 'stars'),
    ('steam:1086940', "Baldur's Gate 3", 'steam', 'Steam', '#2a1a2a', '#9a3a4a', 'ridge'),
    ('steam:548430', 'Deep Rock Galactic', 'steam', 'Steam', '#2a1f14', '#c07a2a', 'city'),
    ('steam:1145350', 'Hades II', 'steam', 'Steam', '#2a1030', '#d04a6a', 'ridge'),
    ('steam:526870', 'Satisfactory', 'steam', 'Steam', '#1e2a30', '#d09a3a', 'city'),
    ('steam:1245620', 'Elden Ring', 'steam', 'Steam', '#26221a', '#b09a5a', 'tower'),
    ('gog:witcher3', 'The Witcher 3', 'gog', 'GOG', '#1e2a22', '#8a9a5a', 'ridge'),
    ('gog:hk', 'Hollow Knight', 'gog', 'GOG', '#141a2a', '#5a7aaa', 'grave'),
    ('epic:aw2', 'Alan Wake 2', 'epic', 'Epic', '#141418', '#5a6a8a', 'grave'),
    ('manual:quad', 'Quad Mech', 'manual', 'Your game', '#2a2a30', '#d07a3a', 'mech'),
    ('manual:grave', 'Gravesleep', 'manual', 'Your game', '#1a1a24', '#6a5aa0', 'grave'),
    ('winapp:office', 'Old Office', 'winapp', 'Windows app', '#2a2622', '#8a7a6a', 'city'),
    ('winapp:ts12', 'Trainz Simulator 12', 'winapp', 'Windows app', '#22302a', '#a0a060', 'rail'),
]
KIND = {'steam': 'steam', 'gog': 'gog', 'epic': 'epic', 'manual': 'yours', 'winapp': 'winapp', 'beacon': 'other'}


def hexc(c):
    return tuple(int(c[i:i + 2], 16) for i in (1, 3, 5))


def art(path, c1, c2, motif, seed):
    """The mockup's art(): a diagonal gradient and a shape (tower, rail, mech, city, stars, grave, ridge)."""
    from PIL import Image, ImageDraw
    W, Hh = 400, 225
    s = 1.0
    a, b = hexc(c1), hexc(c2)
    img = Image.new('RGB', (W, Hh))
    px = img.load()
    for y in range(Hh):
        for x in range(W):
            t = (x / W + y / Hh) / 2
            px[x, y] = tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))
    over = Image.new('RGBA', (W, Hh), (0, 0, 0, 0))
    d = ImageDraw.Draw(over)
    sh, sh2, lt = (0, 0, 0, 71), (0, 0, 0, 115), (255, 255, 255, 46)
    P = lambda pts: [(x * s, y * s) for x, y in pts]
    rnd = random.Random(seed)
    if motif == 'tower':
        d.ellipse([(300 - 34) * s, (70 - 34) * s, (300 + 34) * s, (70 + 34) * s], fill=(255, 210, 160, 64))
        d.polygon(P([(0, 225), (0, 160), (70, 120), (140, 165), (220, 110), (300, 170), (400, 130), (400, 225)]), fill=sh)
        d.polygon(P([(250, 225), (262, 60), (270, 40), (278, 60), (290, 225)]), fill=sh2)
        d.polygon(P([(0, 225), (0, 195), (120, 175), (260, 200), (400, 180), (400, 225)]), fill=sh2)
    elif motif == 'rail':
        d.rectangle(P([(0, 120), (400, 225)]), fill=sh)
        d.polygon(P([(170, 120), (230, 120), (330, 225), (70, 225)]), fill=(0, 0, 0, 64))
        for i in range(7):
            y = 125 + i * i * 2.3
            w = 30 + i * i * 5.5
            d.rectangle(P([(200 - w, y), (200 + w, y + 2 + i)]), fill=(0, 0, 0, 89))
        d.line(P([(190, 120), (110, 225)]), fill=lt, width=6)
        d.line(P([(210, 120), (290, 225)]), fill=lt, width=6)
        d.ellipse([(90 - 24) * s, (60 - 24) * s, (90 + 24) * s, (60 + 24) * s], fill=(255, 230, 190, 64))
    elif motif == 'mech':
        d.polygon(P([(0, 225), (0, 170), (120, 150), (260, 175), (400, 150), (400, 225)]), fill=sh)
        d.rounded_rectangle(P([(170, 80), (250, 120)]), radius=12, fill=sh2)
        d.rectangle(P([(235, 90), (275, 98)]), fill=sh2)
        for x1, y1, x2, y2, w in ((180, 118, 150, 170, 16), (240, 118, 270, 170, 16), (195, 118, 185, 172, 14), (225, 118, 235, 172, 14)):
            d.line(P([(x1, y1), (x2, y2)]), fill=sh2, width=w)
    elif motif == 'city':
        for i in range(14):
            h = 40 + rnd.random() * 110
            d.rectangle(P([(i * 30, 225 - h), (i * 30 + 26, 225)]), fill=sh if i % 2 else sh2)
        d.ellipse([(320 - 18) * s, (50 - 18) * s, (320 + 18) * s, (50 + 18) * s], fill=lt)
    elif motif == 'stars':
        for _ in range(40):
            x, y, r = rnd.random() * 400, rnd.random() * 225, rnd.random() * 1.6 + 0.4
            d.ellipse([(x - r) * s, (y - r) * s, (x + r) * s, (y + r) * s], fill=(255, 255, 255, 140))
        d.ellipse([(290 - 70) * s, (150 - 70) * s, (290 + 70) * s, (150 + 70) * s], fill=sh)
        d.ellipse([(290 - 110) * s, (150 - 16) * s, (290 + 110) * s, (150 + 16) * s], outline=lt, width=6)
    elif motif == 'grave':
        d.ellipse([(110 - 36) * s, (70 - 36) * s, (110 + 36) * s, (70 + 36) * s], fill=(220, 230, 255, 56))
        d.polygon(P([(0, 225), (0, 175), (400, 160), (400, 225)]), fill=sh2)
        for i in range(6):
            x = 40 + i * 60 + rnd.random() * 10
            y = 150 + rnd.random() * 15
            d.rounded_rectangle(P([(x, y), (x + 16, y + 30)]), radius=16, fill=sh2)
    else:
        cx = 80 + rnd.random() * 240
        d.ellipse([(cx - 30) * s, 30 * s, (cx + 30) * s, 90 * s], fill=(255, 220, 170, 56))
        d.polygon(P([(0, 225), (0, 150), (90, 90 + rnd.random() * 40), (180, 160), (280, 80 + rnd.random() * 40), (400, 150), (400, 225)]), fill=sh)
        d.polygon(P([(0, 225), (0, 190), (140, 165), (300, 195), (400, 175), (400, 225)]), fill=sh2)
    img = Image.alpha_composite(img.convert('RGBA'), over).convert('RGB').resize((800, 450), Image.BICUBIC)
    img.save(path, quality=90)


def library(home, now):
    played = {'beacon:rotwk': now - 41 * M, 'winapp:trainz': now - 1 * H - 2 * M, 'steam:553850': now - 26 * H, 'steam:9420': now - 3 * D - H,
              'manual:quad': now - 4 * D - H, 'gog:witcher3': now - 9 * D, 'steam:1086940': now - 12 * D, 'steam:548430': now - 13 * D,
              'steam:1145350': now - 14 * D, 'beacon:bfme2': now - 2 * D - H, 'beacon:bfme1': now - 40 * D, 'winapp:office': now - 50 * D}
    out = []
    adir = os.path.join(home, '.cache', 'hearth', 'art')
    os.makedirs(adir, exist_ok=True)
    for n, (gid, title, src, store, c1, c2, motif) in enumerate(G):
        f = os.path.join(adir, gid.replace(':', '_') + '.jpg')
        art(f, c1, c2, motif, n + 3)
        g = {'id': gid, 'title': title, 'source': src, 'store': store, 'kind': KIND[src], 'installed': gid not in ('beacon:conquest',),
             'update': gid == 'beacon:aotr', 'savesBackedUp': gid in ('beacon:rotwk', 'winapp:trainz'), 'cloudSaves': gid == 'steam:1086940',
             'devBuild': gid == 'manual:grave', 'cover': '', 'wide': f, 'hero': f, 'icon': '', 'lastPlayed': played.get(gid, 0),
             'minutes': 130 if gid == 'steam:553850' else 0, 'system': '', 'chapter': ''}
        if gid == 'beacon:swgb':
            g['tag'] = 'Beacon · Other games'
        if gid == 'beacon:rotwk':
            g['tag'] = 'Beacon'
        if gid == 'beacon:conquest':
            g['size'] = '4.2 GB'
        out.append(g)
    counts = {'all': 412, 'installed': 268}
    for k in ('steam', 'gog', 'epic', 'winapp', 'emu', 'yours'):
        counts[k] = sum(1 for g in out if g['kind'] == k)
    return {'games': out, 'counts': counts}


def cards(now, low=False):
    import datetime
    t = datetime.datetime.now().replace(hour=0, minute=0, second=0, microsecond=0) - datetime.timedelta(hours=2)
    last_night = int(t.timestamp() * 1000)
    return {
        'downloads': [
            {'id': 'steam:553850', 'title': 'Helldivers 2', 'store': 'Steam', 'what': 'update', 'percent': 68, 'state': 'running',
             'line': 'Steam · 4.1 of 6.0 GB · 6 min'},
            {'id': 'gog:witcher3', 'title': 'The Witcher 3', 'store': 'Heroic', 'what': 'patch', 'percent': 26, 'state': 'running',
             'line': 'Heroic · 820 MB of 3.2 GB · 14 min'},
            {'id': 'steam:526870', 'title': 'Satisfactory', 'store': 'Steam', 'what': 'update', 'percent': 0, 'state': 'waiting',
             'line': 'Steam · 0 MB of 1.2 GB'}],
        'osUpdate': True,
        'controllers': [
            {'name': 'Xbox Wireless Controller', 'kind': 'pad', 'percent': 72, 'charging': False, 'player': 1, 'low': False},
            {'name': 'Xbox Wireless Controller 2', 'kind': 'pad', 'percent': 12 if low else 18, 'charging': False, 'player': 2, 'low': True},
            {'name': 'Arctis 7', 'kind': 'headset', 'percent': 60, 'charging': False, 'low': False},
            {'name': 'MX Master 3', 'kind': 'mouse', 'percent': 40, 'charging': False, 'low': False}],
        'devices': [
            {'id': 'd1' * 16, 'name': "Dad's PC", 'online': True, 'kind': 'desktop', 'mode': 'desktop', 'playing': 'Trains 2010',
             'uptime': 3700, 'family': True, 'wake': True, 'battery': None, 'lastSeen': now},
            {'id': 'a1' * 16, 'name': 'ROG Ally', 'online': False, 'kind': 'handheld', 'mode': '', 'playing': '', 'battery': 64,
             'savesAt': now - 10 * M, 'wake': True, 'family': False, 'lastSeen': now - 10 * M},
            {'id': 'b1' * 16, 'name': 'Laptop', 'online': False, 'kind': 'laptop', 'mode': '', 'playing': '', 'battery': None,
             'backupAt': last_night, 'wake': False, 'family': False, 'lastSeen': now - 20 * H}],
        'savesToStorage': True, 'alsoOn': 'ROG Ally',
        'progress': {
            'beacon:rotwk': {'series': 'The Battle for Middle-earth II', 'campaign': 'Angmar', 'chapter': 5, 'of': 8, 'name': 'The Fall of Arthedain',
                             'percent': 62, 'hours': 31, 'mod': 'Edain 4.8 submod', 'launcher': 'Beacon'},
            'winapp:trainz': {'who': 'Dad', 'percent': 100, 'runner': 'Proton 9', 'layout': 'The game wants a mouse · Hearth’s “Train cab” layout',
                              'saves': '3 saved routes · backed up to Hearth Storage at 9:12 PM'},
            'steam:553850': {'percent': 40}, 'steam:9420': {'campaign': 'Novus', 'percent': 70}, 'manual:quad': {'build': 'Build 0.4'},
            'gog:witcher3': {'percent': 22}, 'beacon:aotr': {'version': '8.4', 'launcher': 'Beacon', 'mod': 'Witch-king mod', 'size': '1.1 GB'},
            'manual:grave': {'launcher': 'Godot'}, 'epic:aw2': {'note': 'Free this week · 2 days'}},
        'added': {'beacon:aotr': now - 1 * H, 'steam:526870': now - 2 * H, 'manual:grave': now - 5 * H, 'epic:aw2': now - 2 * D, 'winapp:ts12': now - 3 * D},
        'profiles': [{'id': 'main', 'name': 'Chris'}, {'id': 'dad', 'name': 'Dad'}],
        'mediaNotes': {'plex': '2 new episodes', 'spotify': 'Paused · Howard Shore', 'hearth-photos': 'Trip 2026 · 214'},
    }


MEDIA = [('org.jellyfin.JellyfinDesktop', 'Jellyfin', 'jellyfin', ''), ('youtube', 'YouTube', 'firefox --new-window https://youtube.com', 'web'),
         ('tv.plex.PlexDesktop', 'Plex', 'plex', ''), ('com.spotify.Client', 'Spotify', 'spotify', ''),
         ('twitch', 'Twitch', 'firefox --new-window https://twitch.tv', 'web'), ('hearth-photos', 'Hearth Photos', 'hearth-photos', ''),
         ('com.moonlight_stream.Moonlight', 'Moonlight', 'moonlight', ''), ('hearth-tv', 'Hearth TV', 'hearth-tv', '')]

MPRIS = r'''#!/usr/bin/python3
# A stand-in media player on the session bus (MPRIS): Jellyfin playing Band of Brothers, 23:10 of 58:40.
import dbus, dbus.service, dbus.mainloop.glib
from gi.repository import GLib
dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
I, P = 'org.mpris.MediaPlayer2', 'org.mpris.MediaPlayer2.Player'
class Player(dbus.service.Object):
    def __init__(self, bus):
        dbus.service.Object.__init__(self, bus, '/org/mpris/MediaPlayer2')
        self.props = {I: {'Identity': 'Jellyfin', 'DesktopEntry': 'org.jellyfin.JellyfinDesktop', 'CanQuit': False, 'CanRaise': True,
                          'HasTrackList': False, 'SupportedUriSchemes': dbus.Array([], 's'), 'SupportedMimeTypes': dbus.Array([], 's')},
                      P: {'PlaybackStatus': 'Playing', 'Rate': 1.0, 'Volume': 1.0, 'Position': dbus.Int64(1390 * 10**6),
                          'MinimumRate': 1.0, 'MaximumRate': 1.0, 'CanGoNext': True, 'CanGoPrevious': True, 'CanPlay': True,
                          'CanPause': True, 'CanSeek': True, 'CanControl': True,
                          'Metadata': dbus.Dictionary({'mpris:trackid': dbus.ObjectPath('/org/jellyfin/track/1'),
                                                       'mpris:length': dbus.Int64(3520 * 10**6), 'xesam:title': 'S1 E3 “Carentan”',
                                                       'xesam:album': 'Band of Brothers'}, signature='sv')}}
    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature='ss', out_signature='v')
    def Get(self, i, p): return self.props[i][p]
    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature='s', out_signature='a{sv}')
    def GetAll(self, i): return self.props.get(i, {})
    @dbus.service.method(dbus.PROPERTIES_IFACE, in_signature='ssv')
    def Set(self, i, p, v): pass
    @dbus.service.signal(dbus.PROPERTIES_IFACE, signature='sa{sv}as')
    def PropertiesChanged(self, i, c, x): pass
    @dbus.service.method(P)
    def PlayPause(self): pass
    @dbus.service.method(P)
    def Next(self): pass
    @dbus.service.method(P)
    def Play(self): pass
    @dbus.service.method(P)
    def Pause(self): pass
    @dbus.service.method(I)
    def Raise(self): pass
bus = dbus.SessionBus()
name = dbus.service.BusName('org.mpris.MediaPlayer2.jellyfin', bus)
Player(bus)
GLib.MainLoop().run()
'''


def main(argv):
    now = int(time.time() * 1000)
    if argv[1:2] == ['--low']:
        out = argv[2]
        json.dump(cards(now, low=True), open(os.path.join(out, 'home.json'), 'w'))
        return 0
    home, out = argv[1], argv[2]
    os.makedirs(out, exist_ok=True)
    lib = library(home, now)
    cache = os.path.join(home, '.cache', 'hearth', 'firehold-library.json')
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    json.dump(lib, open(cache, 'w'))
    json.dump(cards(now), open(os.path.join(out, 'home.json'), 'w'))
    json.dump({'net': 'Wi-Fi', 'user': 'Chris', 'initial': 'C', 'battery': -1}, open(os.path.join(out, 'sys.json'), 'w'))
    cfg = os.path.join(home, '.config', 'nimbus', 'firehold.json')
    os.makedirs(os.path.dirname(cfg), exist_ok=True)
    json.dump({'firehold.unlocked': True, 'pins': ['winapp:trainz'], 'players': {'winapp:trainz': 'dad'}}, open(cfg, 'w'))
    apps = os.path.join(home, '.local', 'share', 'applications')
    os.makedirs(apps, exist_ok=True)
    for i, name, exe, web in MEDIA:
        with open(os.path.join(apps, i + '.desktop'), 'w') as f:
            f.write('[Desktop Entry]\nType=Application\nName=%s\nExec=%s\nIcon=applications-multimedia\nCategories=AudioVideo;Video;\n' % (name, exe))
    with open(os.path.join(out, 'mpris.py'), 'w') as f:
        f.write(MPRIS)
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
