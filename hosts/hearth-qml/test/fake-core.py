#!/usr/bin/env python3
"""A stand-in for `nimbus --core-stdio`, for the Qt renderer's self-test in CI (hearth-qml --self-test, with
HEARTH_CORE_BIN pointing here). It speaks the same channel (hosts/hearth-rs/src/winhost.rs: one JSON object a line)
and answers the calls the Qt screens make with a small sample library (the open games whose art is on Hearth's
website, site/img/games), so every screen has something to draw. Not part of Hearth: nothing here is real."""
import json, os, sys, threading, time

HERE = os.path.dirname(os.path.abspath(__file__))
ART = os.environ.get('HEARTH_FAKE_ART') or os.path.abspath(os.path.join(HERE, '..', '..', '..', 'site', 'img', 'games'))
LOG = sys.stderr
lock = threading.Lock()
now_ms = lambda: int(time.time() * 1000)
DAY = 86400000

def out(obj):
    with lock:
        sys.stdout.write(json.dumps(obj) + '\n')
        sys.stdout.flush()

def log(t):
    LOG.write('[fake-core] %s\n' % t); LOG.flush()

def game(gid, title, source, slug, **kw):
    g = {'id': gid, 'title': title, 'source': source, 'fav': False, 'hidden': False, 'minutes': 0, 'lastPlayed': 0,
         'added': now_ms() - 30 * DAY, 'sizeBytes': 0, 'path': '', 'canRemove': False, 'installed': True, 'state': 'installed',
         'cloud': False, 'canUninstall': False, 'installKind': None,
         'art': {k: 'nimbus-art://file/%s/%s' % (slug, k) for k in ('cover', 'hero', 'wide', 'logo')}}
    g.update(kw)
    return g

T = now_ms()
GAMES = [
    game('steam:1', 'SuperTuxKart', 'steam', 'stk', minutes=640, lastPlayed=T - 3600000, sizeBytes=1_400_000_000, fav=True, path='/home/me/.local/share/Steam/steamapps/common/SuperTuxKart', canUninstall=True),
    game('gog:2', '0 A.D.', 'gog', '0ad', minutes=1200, lastPlayed=T - 2 * DAY, sizeBytes=3_100_000_000, fav=True, installKind='nimbus', canUninstall=True),
    game('epic:3', 'The Battle for Wesnoth', 'epic', 'wesnoth', minutes=300, lastPlayed=T - 5 * DAY, sizeBytes=520_000_000, installKind='nimbus', canUninstall=True),
    game('steam:4', 'OpenTTD', 'steam', 'openttd', minutes=90, lastPlayed=T - 9 * DAY, sizeBytes=60_000_000, fav=True),
    game('manual:5', 'Veloren', 'manual', 'veloren', minutes=45, lastPlayed=T - 20 * DAY, added=T - 3 * DAY),
    game('gog:6', 'Endless Sky', 'gog', 'es', installed=False, state='downloading', installKind='nimbus', progress=0.42,
         bytesDone=210_000_000, bytesTotal=500_000_000, bytesPerSec=12_500_000, added=T - DAY),
    game('steam:7', 'Hedgewars', 'steam', 'none', installed=False, state='notInstalled', installKind='steam'),
]
RUNNING = {'id': None}

def by_id(i):
    return next((g for g in GAMES if g['id'] == i), None)

def event(name, data, to='main'):
    out({'t': 'msg', 'to': to, 'msg': {'type': 'event', 'name': name, 'data': data}})

def later(sec, fn):
    threading.Timer(sec, fn).start()

def call(method, a):
    if method == 'host.info':
        return {'platform': 'linux', 'name': 'Hearth OS (fake core)', 'host': 'fake core', 'version': '0.0.0-test', 'renderer': 'qt',
                'features': ['renderer', 'qtRenderer', 'restartApp', 'sendLogs'], 'edition': 'test'}
    if method == 'library.get':
        return {'games': GAMES, 'sources': {}, 'counts': {}, 'runningId': RUNNING['id'], 'steamBackground': True}
    if method == 'library.downloads':
        for g in GAMES:
            if g['state'] == 'downloading':
                g['progress'] = min(0.99, (g.get('progress') or 0) + 0.01)
                g['bytesDone'] = int(g['progress'] * g['bytesTotal'])
        return {'active': [g for g in GAMES if g['state'] in ('downloading', 'queued', 'paused', 'updating')], 'changed': False,
                'speeds': [[T + i * 1000, 9_000_000 + (i % 7) * 900_000] for i in range(40)], 'recent': [{'id': 'manual:5'}]}
    if method == 'profile.get':
        return {'name': 'Player One', 'accent': '#3fd0b8', 'background': 'default', 'avatar': 'initials'}
    if method == 'sys.status':
        return {'battery': {'present': True, 'pct': 82, 'charging': False}, 'audio': {'volume': 60, 'muted': False},
                'display': {'supported': True, 'brightness': 70}, 'wifi': {'available': True, 'on': True, 'ssid': 'Home', 'bars': 3}, 'bt': {'available': True, 'on': True}}
    if method == 'perf.stats':
        return {'cpu': 23, 'gpu': 41, 'vramUsed': 2.1, 'ramUsed': 6.4, 'ramTotal': 16, 'battery': {'present': True, 'pct': 82, 'charging': False},
                'canFpsLimit': True, 'fpsLimit': 60, 'canOverlay': True, 'perfOverlay': 0, 'tdp': {'supported': True, 'min': 5, 'max': 30, 'watts': 15},
                'powerMode': 'balanced', 'powerModes': ['efficiency', 'balanced', 'performance'], 'runningId': RUNNING['id']}
    if method == 'display.renderer':
        return {'pick': a.get('set') or 'qt', 'running': 'qt', 'available': True, 'electron': True, 'qt': True, 'nvidia': False, 'qtCrashes': 0, 'env': None}
    if method == 'osk.get':
        return {'settings': {}, 'systemLayouts': [], 'learned': None}
    if method == 'input.tables':
        return {'templates': [{'id': 'fps', 'name': 'Shooter (WASD and mouse)'}, {'id': 'rts', 'name': 'Strategy (mouse)'}], 'keys': []}
    if method == 'input.get':
        return {'game': a.get('game'), 'layout': 'off', 'custom': None, 'prefs': {'sensitivity': 1, 'gyro': False}}
    if method == 'logs.sendToDeveloper':
        return {'code': 'TEST-1234', 'url': 'https://example.invalid/logs/TEST-1234'}
    if method == 'game.launch':
        gid = a.get('id')
        def started():
            RUNNING['id'] = gid
            event('game.started', {'id': gid})
            later(1.0, lambda: event('game.ready', {'id': gid}))
        later(0.4, started)
        return {'tracked': True}
    if method == 'game.close':
        gid = RUNNING['id']
        def ended():
            RUNNING['id'] = None
            g = by_id(gid)
            if g: g['lastPlayed'] = now_ms(); event('game.exited', g)
        later(0.3, ended)
        return {'ok': True}
    if method == 'game.install':
        g = by_id(a.get('id'))
        if g: g.update(state='queued')
        return {}
    if method in ('game.uninstall',):
        g = by_id(a.get('id'))
        if g: g.update(installed=False, state='notInstalled')
        return {'games': GAMES}
    if method == 'library.setFav':
        g = by_id(a.get('id'))
        if g: g['fav'] = bool(a.get('on'))
        return None
    return None

def main():
    log('started with %s; art from %s' % (sys.argv[1:], ART))
    for line in sys.stdin:
        line = line.strip()
        if not line.startswith('{'):
            continue
        try:
            m = json.loads(line)
        except ValueError:
            continue
        t = m.get('t')
        if t == 'page':
            win, msg = m.get('win', 'main'), m.get('msg') or {}
            ty = msg.get('type')
            if ty == 'call':
                try:
                    v = call(msg.get('method'), msg.get('args') or {})
                    out({'t': 'msg', 'to': win, 'msg': {'type': 'result', 'id': msg.get('id'), 'ok': True, 'value': v}})
                except Exception as e:  # a bug here mustn't hang the test
                    out({'t': 'msg', 'to': win, 'msg': {'type': 'result', 'id': msg.get('id'), 'ok': False, 'error': str(e)}})
            elif ty == 'ready' and win == 'main':
                out({'t': 'msg', 'to': 'main', 'msg': {'type': 'toast', 'text': 'Connected to the fake core (self-test)'}})
            elif ty == 'log':
                log('page: %s' % msg.get('text'))
            elif ty in ('power', 'resume', 'overlay'):
                log('page asked: %s' % json.dumps(msg))
        elif t == 'art':
            uri = m.get('uri') or ''
            parts = uri.split('/')
            path = None
            if len(parts) >= 2:
                slug, kind = parts[-2], parts[-1]
                p = os.path.join(ART, '%s-%s.webp' % (slug, kind))
                if os.path.isfile(p):
                    path = p
            out({'t': 'art', 'id': m.get('id'), 'path': path, 'mime': 'image/webp'})
        elif t == 'win':
            pass
    log('input closed; leaving')

if __name__ == '__main__':
    main()
