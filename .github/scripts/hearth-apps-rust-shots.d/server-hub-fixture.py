#!/usr/bin/env python3
"""The Home server helper's more pages (boards docs/mockups/server-expansion/05, 07, 08, 09, 10) as their sources
answer on the board's night (Thu Nov 26, 2026, 1:24 AM, US Central): the game servers in Podman with their players
(A2S and Minecraft's list ping), the cache's numbers as hearth-server-dns writes them, Sunshine's stream, Jellyfin's
answers and the other Hearth PCs as hearth-device-status lists them. hearth-server-hub builds every page from these
with its own code (HEARTH_SERVER_HUB_FIXTURE), so the pictures show what it makes of them. Plus your choices
(server-hub.json): the schedules, mods, who it's shared with, what's wanted.

  server-hub-fixture.py FIXTURE.json CONFIG_DIR
"""
import datetime
import json
import math
import os
import sys

out, cfg = sys.argv[1], sys.argv[2]
os.environ['TZ'] = 'America/Chicago'
try:
    import time
    time.tzset()
except AttributeError:
    pass


def at(y, mo, d, h=12, mi=0):
    return datetime.datetime(y, mo, d, h, mi).timestamp()


NOW = at(2026, 11, 26, 1, 24)
GB = 1e9
fx = {'now': NOW, 'me': 'Dragongamer9', 'myDevice': 'the ROG Ally', 'server': 'Nvidia PC', 'mcversion': '1.21'}

# ---- game servers
app = lambda gid, name, opts, pw=None: {'id': gid, 'name': name, 'kind': 'game', 'installed': True, 'on': True, 'running': True,
                                        'opts': [{'key': k, 'value': v} for k, v in opts.items()], 'password': pw, 'size': ''}
fx['apps'] = {'podman': True, 'apps': [
    app('valheim', 'Valheim', {'serverName': 'Hearth', 'worldName': 'Ashfjord', 'crossplay': 'off'}, {'set': True, 'min': 5, 'required': True}),
    app('minecraft', 'Minecraft', {'type': 'VANILLA', 'memory': '4G', 'players': '8'}),
    app('palworld', 'Palworld', {'serverName': 'Ranch night', 'players': '16'}, {'set': False, 'min': 1, 'required': False}),
    dict(app('gravesleep', 'Gravesleep', {}), on=False, running=False),
    dict(app('quadmech', 'Quad mech game', {}), on=False, running=False),
]}
fx['stats'] = {'hearth-app-valheim': {'cpu': 6.2, 'mem': 3.1 * GB, 'started': at(2026, 11, 25, 23, 40)},
               'hearth-app-minecraft': {'cpu': 4.9, 'mem': 2.4 * GB, 'started': at(2026, 11, 25, 21, 2)},
               'hearth-app-palworld': {'cpu': 3.1, 'mem': 0.7 * GB, 'started': at(2026, 11, 26, 0, 15)}}
fx['players.valheim'] = {'n': 3, 'max': 10, 'names': [{'name': 'Dragongamer9', 'secs': 2 * 3600 + 14 * 60}, {'name': 'Sam', 'secs': 6000}, {'name': 'Jordan', 'secs': 1320}]}
fx['players.minecraft'] = {'n': 2, 'max': 8, 'names': [{'name': 'Dad', 'secs': 0}, {'name': 'Sam', 'secs': 0}]}
fx['players.palworld'] = {'n': 2, 'max': 16, 'names': []}
fx['world.valheim'] = {'day': 214, 'seed': 'f8Kq2Lm'}
fx['ts'] = {'dns': 'valheim.hearth-pc.ts.net', 'peers': [{'name': 'sam-pc', 'online': True, 'shared': True}, {'name': 'jordan-laptop', 'online': True, 'shared': True}]}
fx['worlds.valheim'] = [{'file': '2026-11-26 0400 nightly.tar.gz', 'size': 212e6}, {'file': '2026-11-25 2338 stopped.tar.gz', 'size': 209e6},
                        {'file': '2026-11-25 0400 nightly.tar.gz', 'size': 206e6}]
fx['worldsCfg'] = {'on': True, 'at': '04:00', 'keep': 30}
fx['build.gravesleep'] = {'at': at(2026, 11, 25, 18)}
fx['build.quadmech'] = {'at': at(2026, 11, 19, 15)}
fx['build.noclip'] = None
fx['gamesHelper'] = False
fx['worldsBytes'] = 3.4 * GB
week = {}
start = datetime.datetime.fromtimestamp(NOW).replace(minute=0, second=0, microsecond=0) - datetime.timedelta(days=7)
shape = [3, 4, 2, 6, 5, 6, 3, 6, 2, 5, 2, 3, 6, 1, 6, 2, 5, 3, 4, 6, 7, 1, 1, 2, 6, 3, 4, 6]
for i in range(28):
    ts = start + datetime.timedelta(hours=i * 6 + 3)
    week[ts.strftime('%Y-%m-%dT%H')] = shape[i]
sat9 = datetime.datetime(2026, 11, 21, 21)
week[sat9.strftime('%Y-%m-%dT%H')] = 9
fx['playersWeek'] = week
fx['console.valheim'] = ['[11:40 PM] Game server connected', '[11:52 PM] Got connection SteamID 7656119 (Sam)', '[12:59 AM] Got connection (Jordan)']

# ---- game cache (hearth-server-dns' status.json, its Game cache numbers)
games = [("Baldur's Gate 3", 'Steam', 148, 296, ['Nvidia PC', 'Ally', 'laptop'], at(2026, 11, 26, 1, 20)),
         ('Cyberpunk 2077', 'GOG', 92, 92, ['Nvidia PC', 'Ally'], at(2026, 11, 24, 20)),
         ('Helldivers 2', 'Steam', 81, 162, ['Nvidia PC', 'laptop', 'Ally'], at(2026, 11, 23, 19)),
         ('Forza Horizon 5', 'Xbox app', 110, 110, ['Xbox', 'Nvidia PC'], at(2026, 11, 20, 18)),
         ('Valheim', 'Steam', 1.6, 4.8, ['everyone'], at(2026, 11, 26, 0, 5)),
         ('Fortnite', 'Epic', 54, 54, ['Nvidia PC', 'Ally'], at(2026, 11, 18, 16)),
         ('Hearth OS updates and Store apps', 'Hearth', 11, 38, ['every Hearth device'], at(2026, 11, 26, 1, 0))]
more = [{'game': 'game %d' % i, 'store': 'Steam', 'miss': 114 * GB / 34, 'hit': 1150 * GB / 34, 'devices': [], 'last': at(2026, 10, 1)} for i in range(34)]
month_days = []
for d in range(1, 27):
    hit = [14, 52, 50, 4, 9, 37, 30, 34, 26, 24, 22, 2, 46, 45, 30, 32, 10, 30, 3, 9, 21, 9, 42, 38, 6, 16][d - 1] * GB
    month_days.append({'hit': hit, 'miss': hit * 0.19})
fx['dns'] = {'cache': {'on': True, 'running': True, 'sizeGB': 1000, 'dir': '/run/media/games/Hearth game cache',
                       'stores': {'steam': True, 'epic': True, 'blizzard': True, 'xbox': True},
                       'view': {'used': 612 * GB, 'total': {'hit': 1.9e12, 'miss': 0.4e12, 'since': at(2026, 9, 2, 12)},
                                'monthHit': 642 * GB, 'monthMiss': 121 * GB, 'monthDays': month_days, 'internetMbps': 300, 'waitMbps': 103,
                                'lanSpeed': '2.5 Gb/s', 'driveLabel': 'Games drive · 4 TB',
                                'now': [{'device': 'ROG Ally', 'what': "Baldur's Gate 3 update", 'rate': 117.5e6}],
                                'games': [{'game': g, 'store': s_, 'miss': c * GB, 'hit': h * GB, 'devices': dv, 'last': l} for g, s_, c, h, dv, l in games] + more,
                                'devicesMonth': {'ROG Ally': 412 * GB, 'Laptop': 188 * GB, 'Xbox': 116 * GB, 'Truck laptop': 41 * GB},
                                'away': [["Dad's PC", 'away']],
                                'notes': {'Truck laptop': 'at home', "flow:Dad's PC": 'its own internet (away)', 'flow:Xbox': 'idle · cached too'},
                                'devices': [{'name': 'ROG Ally', 'rate': 117.5e6}, {'name': 'Laptop', 'rate': 0}, {'name': "Dad's PC", 'rate': 0}, {'name': 'Xbox', 'rate': 0}]}}}
fx['cacheInstalled'] = False

# ---- remote play
fx['rd'] = {'stream': {'on': True, 'device': 'TCL 55" TV', 'app': 'Gravesleep', 'since': at(2026, 11, 26, 0, 12), 'height': 2160, 'fps': 60, 'hdr': True,
                       'mbps': 38, 'delay': 9, 'pad': 'Xbox controller', 'ontime': 99.6}}
fx['gpu'] = {'enc': 22, 'gpu': 71}
fx['delays'] = [round(9 + 1.6 * math.sin(i / 3.0) + (1.2 if i % 11 == 0 else 0)) for i in range(60)]
fx['streamed'] = {'2026-11-25|TV': 540, '2026-11-24|ROG Ally': 240, '2026-11-23|Truck laptop': 130, '2026-11-22|Phone': 35}
fx['library'] = [{'id': 'gravesleep', 'name': 'Gravesleep', 'art': ['#5a3a2a', '#1a1410']}, {'id': 'valheim', 'name': 'Valheim', 'art': ['#3a5a7a', '#1a2a2a']},
                 {'id': 'bg3', 'name': "Baldur's Gate 3", 'art': ['#6a3a9a', '#20142a']}, {'id': 'noclip', 'name': 'Noclip Protocol', 'art': ['#8a7a3a', '#2a2614']},
                 {'id': 'quadmech', 'name': 'Quad mech game', 'art': ['#3a6a5a', '#142420']}, {'id': 'bfme2', 'name': 'BFME II · Edain', 'art': ['#7a3a2a', '#2a1410']}]
fx['libraryCount'] = 86
fx['truckLast'] = 'Mon on break · 41 min'
fx['sunshineInstalled'] = False

# ---- media
fx['jf.counts'] = {'MovieCount': 418, 'SeriesCount': 64, 'EpisodeCount': 2190, 'SongCount': 4612}
fx['homeVideos'] = 192
fx['moviesSize'] = '3.1 TB'
fx['jf.sessions'] = [{'UserName': 'Dad', 'DeviceName': 'his PC', 'NowPlayingItem': {'Name': 'Ironwood (2021)'}, 'PlayState': {'PositionTicks': 48 * 60 * 10 ** 7}}]
iso = lambda ts: datetime.datetime.fromtimestamp(ts, datetime.timezone.utc).isoformat().replace('+00:00', 'Z')
fx['jf.timers'] = [{'Name': 'Night Harbor s3 e5', 'StartDate': iso(at(2026, 11, 26, 20)), 'ChannelName': 'Channel 8.1'},
                   {'Name': 'Rail Journeys', 'StartDate': iso(at(2026, 11, 28, 13)), 'ChannelName': '11.2', 'For': 'Dad'}]
fx['jf.latest'] = [{'Name': 'Ironwood (2021)'}, {'Name': 'Night Harbor s3 e2'}, {'Name': 'Coast Lines'}, {'Name': 'Prairie Rails (1951)'}]
fx['jobs'] = [
    {'id': 'j1', 'state': 'getting', 'title': 'Ironwood · disc 2', 'line': 'ripping from the Blu-ray drive', 'progress': 0.58, 'left': '58% · 14 min left', 'art': ['#7a2a2a', '#2a1010']},
    {'id': 'j2', 'state': 'getting', 'title': 'Night Harbor · s3 e4', 'line': 'recording · Channel 8.1', 'progress': 0.3, 'live': True, 'left': '● on air · late rerun · until 1:30 AM', 'art': ['#2a3a5a', '#10141e']},
    {'id': 'j3', 'state': 'getting', 'title': 'Prairie Rails (1951)', 'line': 'public domain · Internet Archive', 'progress': 0.72, 'left': '72% · 3 min left', 'art': ['#3a5a3a', '#101a10']},
    {'id': 'j4', 'state': 'ready', 'title': 'Ironwood · disc 1', 'line': 'named, art found, subtitles in English', 'ask': 'is this "Ironwood (2021)"?', 'guess': 'Ironwood (2021)', 'art': ['#5a3a7a', '#1a1028']},
    {'id': 'j5', 'state': 'ready', 'title': 'Night Harbor · s3 e3', 'line': 'ads cut out · 42 min', 'line2': 'going into Jellyfin in 2 min', 'art': ['#2a3a5a', '#10141e']},
    {'id': 'j6', 'state': 'ready', 'title': 'Truck trip, Nov 8', 'line': 'your own video · 3 clips joined', 'art': ['#5a5a2a', '#1a1a0c']}]
fx['music'] = [['Hearth trailer music', 'from hearthos.fun · New Horizons, Never Frozen, Feels Like Home…', '12', ['#ff9a4a', '#c0501a']],
               ['Your CDs, ripped', 'named and given covers by themselves', '3,880', ['#5ec8ff', '#2c7fd8']],
               ['Beat Maker songs', 'from Hearth Beat Maker', '31', ['#9be36b', '#3ba55c']],
               ['Truck drive mix', 'playlist · on the truck laptop offline', '96', ['#e36bd3', '#8a2a9a']]]
fx['musicLine'] = '4,612 songs · 38 GB · on the phone too'
fx['photos'] = {'done': True, 'line': '23,418 photos and videos · 312 GB', 'syncing': True,
                'rows': [['Moved from iCloud Photos and iCloud Drive', 'finished Nov 19 · keeps syncing new ones', ''],
                         ['Found by AI photos: people, places, things', 'Dad · the truck · Thanksgiving 2025', ''],
                         ['Same photo twice', '412 found · ', 'Look through them']]}
fx['space'] = {'title': 'Media drive · 6 TB', 'used': 0.63, 'line': '3.8 TB used · TV and movies 3.4, photos 0.31, music 0.04'}
fx['mediaInstalled'] = False

# ---- PCs (hearth-device-status: this PC and the paired ones that run Hearth Server)
fx['self'] = {'name': 'Nvidia PC', 'gpu': {'name': 'RTX 5070 12 GB', 'use': 41}, 'ram': {'total': 32}, 'drives': [{'total': 22000}], 'where': 'at home',
              'cpu': {'use': 18, 'temp': 62}, 'uptime': 9 * 86400 + 3600, 'watts': 186,
              'jobs': [{'name': 'Colibri · Qwen3.6 35B (Big AI)', 'kind': 'AI', 'share': {'AI': ['Colibri']}},
                       {'name': 'Jellyfin, Imagine, Kokoro, Home, Shield', 'kind': 'services', 'share': {'AI': ['Imagine', 'Kokoro'], 'services': ['most']}},
                       {'name': 'Valheim, Palworld', 'kind': 'games', 'share': {'games': ['Valheim', 'Palworld']}},
                       {'name': 'Model vault · Backups (first copy)', 'kind': 'storage', 'share': {'backups': ['first copy']}}]}
fx['peers'] = [
    {'hearthServer': True, 'name': "Dad's PC", 'gpu': {'name': 'Radeon RX 6750 XT 12 GB', 'use': 35}, 'ram': {'total': 32}, 'drives': [{'total': 2000}],
     'where': "at Dad's", 'via': 'tailscale', 'cpu': {'use': 26, 'temp': 48}, 'note': "Dad's using it", 'limited': True, 'mood': 'blue',
     'jobs': [{'name': 'Ollama small model and Listen for the family', 'kind': 'AI', 'share': {'AI': ['Ollama', 'Listen']}},
              {'name': 'Minecraft (Dad plays on it)', 'kind': 'games', 'share': {'games': ['Minecraft']}},
              {'name': 'Backups, the second copy away from home', 'kind': 'storage', 'share': {'backups': ['second copy, away']}},
              {'name': 'Steps back when Dad plays Trainz', 'kind': 'always', 'on': False, 'share': {}}]},
    {'hearthServer': True, 'name': 'Old laptop', 'cpuName': 'Core i5', 'ram': {'total': 16}, 'drives': [{'total': 1000}], 'where': 'on the shelf at home',
     'cpu': {'use': 9, 'temp': 41}, 'note': 'lid shut', 'battery': {'percent': 100, 'charging': True}, 'ups': True, 'watts': 14, 'mood': 'green',
     'jobs': [{'name': 'Hearth Shield (a second one, so ads stay blocked)', 'kind': 'network', 'share': {'services': ['Shield']}},
              {'name': "Passwords · a copy that's always on", 'kind': 'services', 'share': {'services': ['Passwords']}},
              {'name': "Wakes the Nvidia PC when it's needed", 'kind': 'always on', 'share': {}},
              {'name': 'Gravesleep test server', 'kind': 'games', 'share': {'games': ['Gravesleep']}}]}]
fx['tried'] = 'Dad\'s PC turned off for 2 hours. Minecraft moved in 38 s, the game kept going for everyone, and it moved back at 9:14 PM after you said yes.'
fx['triedWhen'] = 'Nov 23'
fx['weekSeen'] = {'Nvidia PC': [{'on': True, 'w': 1.0}],
                  "Dad's PC": [{'on': True, 'w': 0.38}, {'on': False, 'w': 0.04, 'why': 'Nov 21 storm'}, {'on': True, 'w': 0.3}, {'on': False, 'w': 0.06, 'why': 'Nov 23'}, {'on': True, 'w': 0.22}],
                  'Old laptop': [{'on': True, 'w': 1.0}]}
fx['moved'] = [{'at': NOW - i * 86400} for i in range(6)]
fx['found'] = [['Laptop (yours)', 'add'], ['Truck laptop · away', 'not a good server']]
json.dump(fx, open(out, 'w'))

# ---- your choices (server-hub.json)
os.makedirs(os.path.join(cfg, 'nimbus'), exist_ok=True)
choices = {
    'games': {'sel': 'valheim', 'idleStop': 30, 'pauseWhilePlaying': True, 'maxMemGB': 12, 'otherPc': False,
              'shared': {'Sam': [], 'Jordan': [], 'Dad': ['minecraft']},
              'mods': {'valheim': [{'id': 'denikson-BepInExPack_Valheim', 'name': 'BepInEx pack', 'on': True},
                                   {'id': 'Advize-PlantEverything', 'name': 'Plant Everything', 'on': True},
                                   {'id': 'ValheimPlus-ValheimPlus', 'name': 'Valheim Plus', 'on': True},
                                   {'id': 'Ishid4-BetterArchery', 'name': 'Better Archery', 'on': False}]},
              'schedule': {'valheim': {'Mon': ['06:00', '22:00'], 'Tue': ['06:00', '22:00'], 'Thu': ['06:00', '22:00'], 'Fri': ['06:00', '13:00'],
                                       'Sat': ['00:00', '24:00'], 'Sun': ['00:00', '24:00']},
                           'minecraft': {'Wed': ['16:00', '21:00'], 'Sat': ['12:00', '21:00'], 'Sun': ['12:00', '21:00']},
                           'gravesleep': {'Sat': ['19:00', '23:00']}}},
    'media': {'wanted': [
        {'id': 'w1', 'title': 'Night Harbor · season 3', 'kind': 'tv', 'from': 'antenna', 'state': 'wanted', 'what': 'TV · 8 episodes · airs Thursdays', 'line': 'records from the antenna', 'art': ['#2a3a5a', '#10141e']},
        {'id': 'w2', 'title': 'The Long Freight', 'kind': 'movie', 'from': 'disc', 'state': 'wanted', 'what': 'movie · 2019', 'line': 'on your disc shelf · rip it', 'art': ['#5a3a1a', '#1e140a']},
        {'id': 'w3', 'title': 'Winter Line (1948)', 'kind': 'movie', 'from': 'free', 'state': 'wanted', 'what': 'movie · public domain', 'line': 'Internet Archive · free · tonight', 'art': ['#2a3a5a', '#10141e']},
        {'id': 'w4', 'title': 'Coast Lines', 'kind': 'tv', 'from': 'antenna', 'state': 'wanted', 'line': 'records from the antenna'},
        {'id': 'w5', 'title': 'Old Mill Road', 'kind': 'movie', 'from': 'disc', 'state': 'wanted', 'line': 'on your disc shelf'},
        {'id': 'w6', 'title': 'Harvest Moon (1934)', 'kind': 'movie', 'from': 'free', 'state': 'wanted', 'line': 'public domain'}]},
    'pcs': {'sel': "Dad's PC", 'dadMax': 40, 'onlyIdle': True, 'laptopOn': True, 'autoMove': True, 'tell': True, 'moveBack': 'ask'},
}
json.dump(choices, open(os.path.join(cfg, 'nimbus', 'server-hub.json'), 'w'), indent=1)
