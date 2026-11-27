#!/usr/bin/python3 -I
"""Backups for every device (board docs/mockups/server-expansion/06-backups.png; checklist
docs/mockups/checklists/server-backups.md): each of your devices backs up to the home server every night over
Tailscale, wherever it is, only what changed; restore a file, a whole device, or (with the Recovery USB) a device
that won't start.

It builds on what Hearth already has: the home server's backup receiver (hearth-home-server receive-add: rclone's
restic server on this PC's Tailscale address only, a code per device, each device in its own private folder) and
restic itself (encrypted; only you have the key). The key of each device's backups is made here when you add the
device and kept in this PC's password store (secret-tool) and, on the device, in a root-only file; neither is ever
printed or logged.

On the server (as you):
    backups.py status [DEVICE]       what the Backups page shows (each device, its nights, what's backed up, space)
    backups.py summary               Server home's tile
    backups.py add NAME KIND         a new device: its backup code and key, given once as a join code for the device
    backups.py set DEVICE KEY VALUE  home|saves|settings|system|games on|off, at HH:MM, away on|off,
                                     nights 7|30|90, weeks 3m|1y|forever, lock on|off
    backups.py set - KEY VALUE       r2 on|off, restoreTest on|off, services on|off
    backups.py on | off              Backups for every device: the receiver, the nightly timer, the plans for the devices
    backups.py nightly               the 2:00 AM timer: this server's own services, old versions forgotten by your
                                     rules, the Sunday copy to R2, the monthly restore test
    backups.py now all|DEVICE        back up now (this server's services now; a device at its next look, within the hour)
    backups.py code DEVICE           its join code again (Restore to a new device)
    backups.py files DEVICE NIGHT [SEARCH]   what's in one night's backup (a folder at a time, or what matches SEARCH)
    backups.py restore DEVICE NIGHT PATH back|beside   into Home › Restored from backups (a new folder each time), and
                                     to the device when it's online (Taildrop)
    backups.py serve                 the plans for your devices, on this PC's Tailscale address only (a user service)
On a device:
    backups.py join CODE             this device backs up to your home server (asks for your password once: the key
                                     goes into a root-only file, the nightly timer into the system)
    backups.py put SNAP PATH back|beside   a file or folder from a night's backup: put back where it was (the one there
                                     now kept beside it as NAME (before DATE)) or beside it
    backups.py root ...              (pkexec, root) device-install | device-run [--if-due] | device-put | device-restore
Restoring a whole device that won't start needs the Recovery USB's "Restore from my home server" (recovery-restore
below), which comes with the next Recovery ISO (the owner's go-ahead).
"""
import base64
import importlib.machinery
import importlib.util
import json
import os
import pwd
import random
import re
import secrets
import shutil
import sys
import tempfile
import time
import urllib.parse


def load_common():
    p = os.path.join(os.path.dirname(os.path.realpath(__file__)), 'common.py')
    loader = importlib.machinery.SourceFileLoader('hearth_server_common', p)
    spec = importlib.util.spec_from_loader('hearth_server_common', loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


C = load_common()
Fail = C.Fail
PART = 'backups'
PLAN_PORT = 8096 + 1  # beside the receiver (8095) and Jellyfin (8096): 8097, on the Tailscale address only
RECEIVE_PORT = 8095
DEVICE_CONF = os.environ.get('HEARTH_DEVICE_BACKUP_CONF') or '/etc/hearth/device-backup.json'
DEVICE_STATE = os.environ.get('HEARTH_DEVICE_BACKUP_STATE') or '/var/lib/hearth/device-backup'
NAME_RE = re.compile(r'^[a-z0-9][a-z0-9\-]{0,30}$')
USER_RE = re.compile(r'^[a-z0-9][a-z0-9\-]{2,39}$')
KINDS = ('handheld', 'laptop', 'pc', 'tv', 'other')
CATS = [('home', 'Your Home (documents, pictures, settings)'), ('saves', 'Game saves (Steam, Hearth Saves, emulators)'),
        ('settings', 'Hearth\'s settings and apps list'), ('system', 'The whole device (to put it all back)'), ('games', 'Installed games')]
DEFAULT_DEVICE = {'home': True, 'saves': True, 'settings': True, 'system': True, 'games': False, 'at': '02:00', 'away': True,
                  'nights': 30, 'weeks': '1y', 'lock': True}
DEFAULTS = {'on': False, 'devices': {}, 'r2': False, 'restoreTest': True, 'services': True, 'at': '02:00'}


def cfg():
    return C.settings(PART, DEFAULTS)


def restic():
    return os.environ.get('HEARTH_RESTIC') or C.which('restic') or '/usr/bin/restic'


def receive_path():
    hs = C._read_obj(os.path.join(C.CONFIG, 'nimbus', 'home-server.json')) or {}
    rc = hs.get('receive') if isinstance(hs.get('receive'), dict) else {}
    return os.environ.get('HEARTH_BACKUP_RECEIVE') or rc.get('path') or ''


def repo_of(dev):
    base = receive_path()
    return os.path.join(base, dev['user']) if base and dev.get('user') else ''


def pw_cmd(key):
    """restic asks the password store itself (--password-command): the key never passes through here."""
    if os.environ.get('HEARTH_TEST_SECRETS'):
        return ['--password-file', _test_pw_file(key)]
    return ['--password-command', 'secret-tool lookup hearth-app home-server key %s' % key]


def _test_pw_file(key):
    p = os.path.join(C.RUN, 'test-%s.pw' % key)
    os.makedirs(C.RUN, exist_ok=True)
    with open(p, 'w') as f:
        f.write(C.secret(key))
    os.chmod(p, 0o600)
    return p


def restic_json(repo, key, args, timeout=300):
    code, out = C.run([restic(), '-r', repo] + pw_cmd(key) + ['--json', '--no-lock'] + args, timeout=timeout)
    if code != 0:
        return None
    try:
        return json.loads(out)
    except ValueError:
        return [json.loads(l) for l in out.splitlines() if l.strip().startswith('{')]


def parse_t(s):
    """restic's times (RFC 3339 with nanoseconds) as seconds."""
    import datetime
    s = re.sub(r'(\.\d{6})\d+', r'\1', s or '')
    try:
        return datetime.datetime.fromisoformat(s.replace('Z', '+00:00')).timestamp()
    except ValueError:
        return 0


def cat_sizes(snap):
    """What each part of a device's backup weighs, as the device wrote it in its newest backup's tags (size-home=…)."""
    outd = {}
    for t_ in (snap or {}).get('tags') or []:
        m = re.fullmatch(r'size-(home|saves|settings|system)=(\d+)', t_)
        if m:
            outd[m.group(1)] = C.size_words(int(m.group(2))).replace('.0 ', ' ')
    return outd


_TS = {}


def where_now(d, did):
    """at home, or away (Tailscale says it's reached from outside your home network), from this PC's Tailscale."""
    if 'peers' not in _TS:
        code, out = C.run(['tailscale', 'status', '--json'], timeout=10) if C.which('tailscale') else (1, '')
        try:
            _TS['peers'] = list((json.loads(out).get('Peer') or {}).values()) if code == 0 else []
        except ValueError:
            _TS['peers'] = []
    host = d.get('host') or did
    p = next((x for x in _TS['peers'] if (x.get('HostName') or '').lower() == host.lower()), None)
    if not p:
        return ''
    if not p.get('Online'):
        return 'off'
    addr = p.get('CurAddr') or ''
    if addr and re.match(r'(10\.|192\.168\.|172\.(1[6-9]|2\d|3[01])\.)', addr):
        return 'at home'
    return 'away'


def snapshots(dev):
    """A device's backups, newest last, from its own repository on this PC (read with its key)."""
    repo = repo_of(dev)
    if not repo or not os.path.isdir(repo):
        return []
    j = restic_json(repo, 'backup-%s' % dev['user'], ['snapshots'])
    outl = []
    for s in j or []:
        if not isinstance(s, dict):
            continue
        sm = s.get('summary') if isinstance(s.get('summary'), dict) else {}
        outl.append({'id': s.get('short_id') or s.get('id', '')[:8], 'at': parse_t(s.get('time')), 'tags': s.get('tags') or [], 'host': s.get('hostname', ''),
                     'added': int(sm.get('data_added_packed') or sm.get('data_added') or 0), 'size': int(sm.get('total_bytes_processed') or 0)})
    outl.sort(key=lambda x: x['at'])
    return outl


def night_of(t):
    """A backup made between noon and noon belongs to that night (2:04 AM on the 26th is the night of the 25th)."""
    return time.strftime('%Y-%m-%d', time.localtime(t - 12 * 3600))


def last_night(now):
    """The newest night whose backups are done (they run in the small hours)."""
    return night_of(now - 6 * 3600)


def nights(snaps, now, n=14):
    """The last n nights: ok (backed up on time), late (tried again later and made it), missed."""
    outl = []
    by = {}
    for s in snaps:
        by.setdefault(night_of(s['at']), []).append(s)
    for i in range(n - 1, -1, -1):
        key = night_of(now - 6 * 3600 - i * 86400)
        ss = by.get(key) or []
        if not ss:
            outl.append({'night': key, 'tone': 'missed'})
            continue
        lt = time.localtime(ss[0]['at'])
        late = lt.tm_hour >= 8 or lt.tm_hour < 0
        outl.append({'night': key, 'tone': 'late' if late else 'ok'})
    return outl


def stats(dev, s):
    """All versions' size (restic stats raw-data), worked out at night and kept, never while the page is open."""
    return ((s.get('sizes') or {}).get(dev['user']) or 0)


def plan_tags(snap):
    return {t.split('=', 1)[0]: (t.split('=', 1)[1] if '=' in t else True) for t in snap.get('tags') or []}


# ------------------------------------------------------------------ the page
def status(pick=''):
    c = cfg()
    s = C.state(PART)
    now = time.time()
    devs = c.get('devices') or {}
    cards = []
    all_ok = True
    for did, d in devs.items():
        d = dict(DEFAULT_DEVICE, **d)
        snaps = snapshots(dict(d, id=did))
        last = snaps[-1] if snaps else None
        ns = nights(snaps, now)
        ok_last = bool(last) and now - last['at'] < 36 * 3600
        all_ok = all_ok and ok_last
        where = where_now(d, did) or d.get('where') or ''
        lt = time.localtime(last['at']) if last else None
        when = ('tonight %s' % C.clock(last['at']) if night_of(last['at']) == last_night(now) else '%s %s' % (C.day(last['at']), C.clock(last['at']))) if last else 'not yet'
        cards.append({'id': did, 'name': d.get('name', did), 'kind': d.get('kind', 'other'), 'line': ' · '.join(x for x in ('Hearth OS' if d.get('hearth', True) else '', where) if x),
                      'state': 'ok' if ok_last else ('warn' if last else 'off'), 'last': '%s · %s%s' % (when, C.size_words(last['added']).replace('.0 ', ' '), ' sent' if did == next(iter(devs)) else '') if last else 'not backed up yet',
                      'whole': '%s · %d versions' % (C.size_words(last['size']).replace('.0 ', ' ') if last else '—', len(snaps)), 'nights': [x['tone'] for x in ns],
                      'missed': [C.day(time.mktime(time.strptime(x['night'], '%Y-%m-%d'))) for x in ns if x['tone'] == 'late'],
                      'plan': {k: d.get(k) for k in ('home', 'saves', 'settings', 'system', 'games', 'at', 'away', 'nights', 'weeks', 'lock')},
                      'sizes': cat_sizes(last), 'snaps': [{'id': x['id'], 'at': x['at']} for x in snaps[-60:]], 'user': d.get('user', '')})
    if not pick and cards:
        pick = cards[0]['id']
    sel = next((x for x in cards if x['id'] == pick), None)
    picks = []
    if sel:
        snaps = sel['snaps']
        seen = set()
        for x in reversed(snaps):
            label = 'Tonight' if night_of(x['at']) == last_night(now) else C.day(x['at'])
            if now - x['at'] > 7 * 86400:
                label = C.day(x['at']) + ' (week)'
            if label in seen:
                continue
            seen.add(label)
            picks.append({'id': x['id'], 'label': label, 'at': C.clock(x['at']) if label == 'Tonight' else C.day(x['at'])})
            if len(picks) >= 8:
                break
    base = receive_path()
    space = None
    if os.environ.get('HEARTH_TEST_DRIVE'):
        t_ = json.loads(os.environ['HEARTH_TEST_DRIVE'])
        space = {'total': t_['total'], 'used': t_['total'] - t_['free']}
    elif base and os.path.isdir(base):
        try:
            v = os.statvfs(base)
            space = {'total': v.f_blocks * v.f_frsize, 'used': (v.f_blocks - v.f_bfree) * v.f_frsize}
        except OSError:
            space = None
    sizes = s.get('sizes') or {}
    by_dev = sorted([(devs[k].get('name', k), sizes.get(devs[k].get('user', ''), 0)) for k in devs], key=lambda x: -x[1])
    svc_size = sizes.get('server', 0)
    biggest = max([x[1] for x in by_dev] + [svc_size, 1])
    total = sum(x[1] for x in by_dev) + svc_size
    steps = s.get('steps') or []
    r2_gb = total / 1e9
    test = s.get('restoreTest') or {}
    return {
        'ok': True, 'on': bool(c.get('on')), 'tag': ('All %d backed up last night' % len(cards)) if cards and all_ok else ('%d of %d backed up last night' % (sum(1 for x in cards if x['state'] == 'ok'), len(cards)) if cards else 'No devices yet'),
        'tagTone': 'ok' if cards and all_ok else 'warn', 'devices': cards, 'pick': pick, 'nights': picks, 'versions': len(sel['snaps']) if sel else 0,
        'nightsNote': 'Last 14 nights · green backed up · orange tried again later' + ((' (%s)' % '; '.join('%s was %s on %s' % (the(x['name']), 'asleep' if x['kind'] == 'pc' else 'off', ', '.join(x['missed'][:2])) for x in cards if x['missed'])) if any(x['missed'] for x in cards) else ''),
        'space': {'name': 'Backup drive · %s' % C.tb_words(space['total'] / 1e9) if space else 'No backup drive picked yet', 'frac': space['used'] / space['total'] if space and space['total'] else 0,
                  'line': '%s used by %d devices and %s' % (C.tb_words(total / 1e9), len(cards), 'its own services' if not svc_size else '%d services' % len((s.get('services') or {}))) if space else 'Settings › Home server › Backups & sync: Backups from your other Hearth devices picks the folder'},
        'byDevice': [{'name': n, 'size': C.size_words(b).replace('.0 ', ' '), 'frac': b / biggest} for n, b in by_dev] + ([{'name': 'Server\'s own services', 'size': C.size_words(svc_size).replace('.0 ', ' '), 'frac': svc_size / biggest}] if svc_size else []),
        'r2': bool(c.get('r2')), 'r2Line': 'Infrequent Access · encrypted · %s ≈ $%d / month · weekly' % (C.tb_words(r2_gb), round(r2_gb * 0.01)),
        'restoreTest': bool(c.get('restoreTest')), 'testLine': ('%s: put back %d random files from each device, %s' % (C.day(test['at']), test.get('n', 50), 'all matched' if test.get('ok') else '%d didn\'t match' % test.get('bad', 0))) if test else 'Not tried yet: the first one runs on the 1st of the month',
        'steps': steps[-6:], 'stepsLine': steps_line(steps),
        'device': device_side(),
    }


def the(name):
    """ "the truck laptop", "the ROG Ally", "Dad's PC" (as a sentence says it)."""
    if "'" in name.split()[0]:
        return name
    first = name.split()[0]
    return 'the ' + (name if first.isupper() else name[0].lower() + name[1:])


def steps_line(steps):
    """"5.6 GB sent in all · 34 min": what last night sent, from the first backup to the last device's."""
    if not steps:
        return 'Nothing yet: the first night is tonight'
    devs = [x for x in steps if 'R2' not in x.get('what', '')]
    sent = sum(x.get('bytes', 0) for x in devs)
    span = devs[-1]['at'] - devs[0]['at'] if len(devs) > 1 else 0
    return '%s sent in all · %s' % (C.size_words(sent).replace('.0 ', ' '), C.duration_words(span) if span else 'a few minutes')


def device_side():
    """On a device: is it set to back up to a server (the root-only file is there), its last run."""
    st_ = {}
    try:
        with open(os.path.join(DEVICE_STATE, 'last.json')) as f:
            st_ = json.load(f)
    except (OSError, ValueError):
        pass
    return {'joined': os.path.exists(DEVICE_CONF), 'last': st_}


def summary():
    s = status()
    c = cfg()
    devs = s['devices']
    names = ', '.join(x['name'].replace('ROG ', '') for x in devs[:4])
    st_ = C.state(PART)
    sizes = st_.get('sizes') or {}
    total = sum(sizes.values())
    keep = max([int((x.get('plan') or {}).get('nights') or 30) for x in devs] or [30])
    tonight_items(c)
    return {'ok': True, 'big': str(len(devs)), 'of': 'devices · %s' % ('all done' if s['tagTone'] == 'ok' and devs else 'not all done' if devs else 'none yet'),
            'lines': [names or 'Add a device: it backs up here every night', 'kept %d nights · %s%s' % (keep, C.tb_words(total / 1e9), ' · copy on R2' if c.get('r2') else '')],
            'alert': '', 'spark': [min(1.0, x.get('bytes', 0) / 4e9) for x in (st_.get('steps') or [])][-24:] or [0.3, 0.4, 0.35, 0.5],
            'count': '%d devices' % len(devs) if devs else '', 'state': s['tagTone'] if devs else 'off'}


def tonight_items(c):
    """What the dashboard's Tonight shows from Backups (kept in this part's state)."""
    if not c.get('on'):
        items = []
    else:
        n = len(c.get('devices') or {})
        items = [{'text': '%s · back up %d device%s and this server\'s services' % (c.get('at', '02:00').lstrip('0') + ' AM' if c.get('at', '02:00') < '12' else c.get('at'), n, '' if n == 1 else 's'), 'order': 2}]
        if time.localtime().tm_wday == 5 and c.get('r2'):
            items.append({'text': '3:10 AM · copy the backups to R2', 'order': 3})
    if C.state(PART).get('tonight') != items:
        C.state_update(PART, lambda d: dict(d, tonight=items))


# ------------------------------------------------------------------ adding a device, its choices
def add(name, kind):
    if not NAME_RE.fullmatch(name or ''):
        raise Fail('A device\'s name is small letters, digits and - (like rog-ally)')
    if kind not in KINDS:
        raise Fail('handheld, laptop, pc, tv or other')
    c = cfg()
    if name in (c.get('devices') or {}):
        raise Fail('That device is here already')
    helper = os.path.join(os.environ.get('HEARTH_SHELL_DIR') or '/usr/share/nimbus/desktop/shell', 'hearth-home-server')
    code, out = C.run(['python3', helper, 'receive-add', name], timeout=60)
    try:
        r = json.loads(out.strip().splitlines()[-1]) if out.strip() else {}
    except ValueError:
        r = {}
    if code != 0 or not r.get('code'):
        raise Fail(r.get('error') or 'The backup receiver couldn\'t make a code (turn on Backups from your other Hearth devices in Settings › Home server first)')
    user, rest_pw = r['code'].split(':', 1)
    key = secrets.token_urlsafe(24)
    # the key of this device's backups: in this PC's password store (only you have it), and in the join code
    if not os.environ.get('HEARTH_TEST_SECRETS'):
        for k, v, label in (('backup-%s' % user, key, 'Hearth backups: %s' % name), ('backup-rest-%s' % user, rest_pw, 'Hearth backups: %s\'s code' % name)):
            code2, _ = C.run(['secret-tool', 'store', '--label=' + label, 'hearth-app', 'home-server', 'key', k], input_text=v, timeout=30)
            if code2 != 0:
                raise Fail('The key couldn\'t be kept in the password store, so the device wasn\'t added')
    join = join_code(user, rest_pw, key)
    devs = dict(c.get('devices') or {})
    devs[name] = dict(DEFAULT_DEVICE, name=pretty_name(name), kind=kind, user=user, added=int(time.time()))
    C.set_key(PART, 'devices', devs)
    return {'ok': True, 'join': 'hearth-backup:' + join, 'said': 'On the %s: Settings › Updates & recovery › Back up to my home server, then paste the code (shown once)' % pretty_name(name)}


def join_code(user, rest_pw, key):
    ip = ts_ip()
    return base64.urlsafe_b64encode(json.dumps({'server': ip, 'port': RECEIVE_PORT, 'plan': PLAN_PORT, 'user': user, 'rest': rest_pw, 'key': key}).encode()).decode()


def code_of(did):
    """A device's join code again (Restore to a new device: the new one joins as the old one and puts its backup
    back). From this PC's password store; shown once."""
    d = (cfg().get('devices') or {}).get(did)
    if not d:
        raise Fail('That device isn\'t here')
    rest_pw, key = C.secret('backup-rest-%s' % d['user']), C.secret('backup-%s' % d['user'])
    if not rest_pw or not key:
        raise Fail('Its code isn\'t in this PC\'s password store any more')
    return {'ok': True, 'join': 'hearth-backup:' + join_code(d['user'], rest_pw, key),
            'said': 'On the new device: Settings › Updates & recovery › Restore from the server, then paste this code (shown once)'}


def pretty_name(n):
    return ' '.join({'rog': 'ROG', 'pc': 'PC', 'dads': 'Dad\'s', 'tv': 'TV'}.get(w, w.capitalize()) for w in n.split('-'))


def ts_ip():
    w = os.environ.get('HEARTH_TEST_TSIP')
    if w:
        return w
    code, out = C.run(['tailscale', 'ip', '-4'], timeout=10) if C.which('tailscale') else (1, '')
    ip = out.strip().splitlines()[0] if code == 0 and out.strip() else ''
    if not re.fullmatch(r'100\.\d{1,3}\.\d{1,3}\.\d{1,3}', ip):
        raise Fail('Turn on Tailscale and sign in first: backups come only over Tailscale')
    return ip


def set_choice(dev, key, value):
    c = cfg()
    if dev == '-':
        if key not in ('r2', 'restoreTest', 'services'):
            raise Fail('r2, restoreTest or services')
        C.set_key(PART, key, value == 'on')
        return {'ok': True}
    devs = dict(c.get('devices') or {})
    if dev not in devs:
        raise Fail('That device isn\'t here')
    d = dict(DEFAULT_DEVICE, **devs[dev])
    if key in ('home', 'saves', 'settings', 'system', 'games', 'away', 'lock'):
        d[key] = value == 'on'
    elif key == 'at':
        if not re.fullmatch(r'([01]\d|2[0-3]):[0-5]\d', value):
            raise Fail('A time like 02:00')
        d['at'] = value
    elif key == 'nights':
        if value not in ('7', '30', '90'):
            raise Fail('7, 30 or 90')
        d['nights'] = int(value)
    elif key == 'weeks':
        if value not in ('3m', '1y', 'forever'):
            raise Fail('3m, 1y or forever')
        d['weeks'] = value
    else:
        raise Fail('Not one of a device\'s choices')
    devs[dev] = d
    C.set_key(PART, 'devices', devs)
    if key == 'lock':
        C.run(['pkexec', '/usr/libexec/nimbus/hearth-home-server-helper', 'receive-lock', 'on' if any(dict(DEFAULT_DEVICE, **x).get('lock') for x in devs.values()) else 'off'], timeout=60)
    return {'ok': True}


# ------------------------------------------------------------------ the server's own nightly run
SERVICE_DIRS = {'vaultwarden': '~/.local/share/hearth/oss/vaultwarden', 'nextcloud': '~/.local/share/hearth/oss/nextcloud',
                'homeassistant': '~/.local/share/hearth/oss/homeassistant', 'comfyui': '~/.local/share/hearth/comfyui',
                'kokoro': '~/.local/share/hearth/kokoro', 'colibri': '~/.config/hearth-colibri', 'jellyfin': '~/.config/jellyfin',
                'ollama': '~/.ollama', 'vault': '~/.local/state/hearth/server', 'settings': '~/.config/nimbus'}
KEEP_WEEKS = {'3m': '13', '1y': '52', 'forever': 'unlimited'}


def nightly(now=None):
    c = cfg()
    if not c.get('on'):
        return {'ok': True, 'said': 'Off'}
    C.low_priority()
    started = time.time()
    steps = []
    failed = []
    st_ = C.state(PART)
    # wait for the game: up to 3 hours, then the next night
    waited = 0
    while C.playing() and waited < 3 * 3600:
        time.sleep(300)
        waited += 300
    if C.playing():
        C.event(PART, 'Backups waited for your game and try again tomorrow night', 'warn')
        return {'ok': True, 'said': 'Waited for your game'}
    base = receive_path()
    svc_times = dict(st_.get('services') or {})
    if c.get('services') and base and os.path.isdir(base):
        repo = os.path.join(base, 'server')
        dirs = [(k, os.path.expanduser(v)) for k, v in SERVICE_DIRS.items() if os.path.isdir(os.path.expanduser(v))]
        if dirs:
            if not os.path.isdir(repo):
                ensure_key('backup-server')
                C.run([restic(), 'init', '-r', repo] + pw_cmd('backup-server'), timeout=120)
            code, out = C.run([restic(), '-r', repo] + pw_cmd('backup-server') + ['backup', '--json', '--tag', 'hearth-server', '--exclude-caches'] + [d for _, d in dirs], timeout=4 * 3600)
            added = _added(out)
            if code == 0:
                for k, _ in dirs:
                    svc_times[k] = int(time.time())
                steps.append({'at': int(started), 'what': 'server\'s %d services' % len(dirs), 'bytes': added})
            else:
                failed.append({'id': 'server', 'name': 'This server\'s services', 'why': 'restic stopped: Send logs has the reason'})
    # every device: its versions kept by your rules (only here: the devices can't delete, the lock)
    sizes = dict(st_.get('sizes') or {})
    for did, d in (c.get('devices') or {}).items():
        d = dict(DEFAULT_DEVICE, **d)
        repo = repo_of(d)
        if not repo or not os.path.isdir(repo):
            continue
        key = 'backup-%s' % d['user']
        C.run([restic(), '-r', repo] + pw_cmd(key) + ['forget', '--prune', '--keep-daily', str(int(d.get('nights') or 30)), '--keep-weekly', KEEP_WEEKS.get(d.get('weeks'), '52')], timeout=4 * 3600)
        j = restic_json(repo, key, ['stats', '--mode', 'raw-data'], timeout=3600)
        if isinstance(j, dict):
            sizes[d['user']] = int(j.get('total_size') or 0)
        snaps = snapshots(d)
        if snaps and night_of(snaps[-1]['at']) == night_of(time.time()):
            steps.append({'at': int(snaps[-1]['at']), 'what': '%s · %s' % (d.get('name', did), C.size_words(snaps[-1]['added']).replace('.0 ', ' ')), 'bytes': snaps[-1]['added']})
        elif not snaps or time.time() - snaps[-1]['at'] > 36 * 3600:
            failed.append({'id': did, 'name': d.get('name', did), 'why': 'It was off or away from the internet; it tries again when it\'s back'})
    srv = os.path.join(base, 'server') if base else ''
    if srv and os.path.isdir(srv):
        j = restic_json(srv, 'backup-server', ['stats', '--mode', 'raw-data'], timeout=3600)
        if isinstance(j, dict):
            sizes['server'] = int(j.get('total_size') or 0)
    # Sundays: the second copy on R2 (encrypted already; the vault's R2 keys)
    if c.get('r2') and time.localtime().tm_wday == 6:
        n = r2_copy(c)
        steps.append({'at': int(time.time()), 'what': 'copy to R2 · Sunday', 'bytes': n})
    # the 1st: the restore test
    if c.get('restoreTest') and time.localtime().tm_mday == 1:
        restore_test(c)
    steps.sort(key=lambda x: x['at'])

    def change(d):
        d['steps'] = steps
        d['services'] = svc_times
        d['sizes'] = sizes
        d['failed'] = failed
        d['lastNight'] = int(time.time())
        return d
    C.state_update(PART, change)
    C.event(PART, 'Nightly backups: %d devices, %d services' % (len(c.get('devices') or {}), len([k for k in svc_times if svc_times[k] >= started - 60])), 'warn' if failed else 'ok')
    for f in failed:
        C.notify('%s didn\'t back up last night. %s' % (f['name'], f['why']), 'Backups')
    return {'ok': True, 'steps': steps, 'failed': failed}


def _added(out):
    for l in reversed((out or '').splitlines()):
        try:
            j = json.loads(l)
        except ValueError:
            continue
        if j.get('message_type') == 'summary':
            return int(j.get('data_added_packed') or j.get('data_added') or 0)
    return 0


def ensure_key(key):
    if C.secret(key):
        return
    k = secrets.token_urlsafe(24)
    C.run(['secret-tool', 'store', '--label=Hearth backups: this server', 'hearth-app', 'home-server', 'key', key], input_text=k, timeout=30)


def r2_copy(c):
    """Each repository copied to the vault's R2 bucket as restic's own copy (still encrypted with your keys)."""
    vc = C.settings('vault', {'r2': {}})
    r2 = vc.get('r2') or {}
    acct, bucket = r2.get('account'), r2.get('bucket')
    if not acct or not bucket:
        return 0
    env = dict(os.environ, AWS_ACCESS_KEY_ID=C.secret('r2-access'), AWS_SECRET_ACCESS_KEY=C.secret('r2-secret'), AWS_DEFAULT_REGION='auto')
    total = 0
    for did, d in (c.get('devices') or {}).items():
        repo = repo_of(d)
        if not repo or not os.path.isdir(repo):
            continue
        dest = 's3:https://%s.r2.cloudflarestorage.com/%s/backups/%s' % (acct, bucket, d['user'])
        key = 'backup-%s' % d['user']
        C.run([restic(), '-r', dest] + pw_cmd(key) + ['init', '--copy-chunker-params', '--from-repo', repo, '--from-password-command', 'secret-tool lookup hearth-app home-server key %s' % key], timeout=600, env=env)
        code, out = C.run([restic(), '-r', dest] + pw_cmd(key) + ['copy', '--from-repo', repo, '--from-password-command', 'secret-tool lookup hearth-app home-server key %s' % key], timeout=12 * 3600, env=env)
        total += 1 if code == 0 else 0
    env.pop('AWS_ACCESS_KEY_ID', None)
    env.pop('AWS_SECRET_ACCESS_KEY', None)
    return total


def restore_test(c, n=50):
    """50 random files from each device's newest backup put back into a scratch folder and checked against the
    backup's own record of them (size and content, restic's own check), then removed."""
    bad = 0
    total = 0
    for did, d in (c.get('devices') or {}).items():
        repo = repo_of(d)
        if not repo or not os.path.isdir(repo):
            continue
        key = 'backup-%s' % d['user']
        snaps = snapshots(d)
        if not snaps:
            continue
        files = [x for x in (restic_json(repo, key, ['ls', snaps[-1]['id']], timeout=1800) or []) if isinstance(x, dict) and x.get('type') == 'file']
        pick = random.sample(files, min(n, len(files)))
        with tempfile.TemporaryDirectory(prefix='hearth-restore-test-') as tmp:
            for f in pick:
                code, _ = C.run([restic(), '-r', repo] + pw_cmd(key) + ['restore', snaps[-1]['id'], '--target', tmp, '--include', f['path']], timeout=600)
                p = os.path.join(tmp, f['path'].lstrip('/'))
                total += 1
                if code != 0 or not os.path.isfile(p) or os.path.getsize(p) != int(f.get('size') or 0):
                    bad += 1
        C.run([restic(), '-r', repo] + pw_cmd(key) + ['check', '--read-data-subset', '2%'], timeout=6 * 3600)
    rec = {'at': int(time.time()), 'n': n, 'ok': bad == 0, 'bad': bad, 'total': total}
    C.state_update(PART, lambda d: dict(d, restoreTest=rec))
    return rec


# ------------------------------------------------------------------ restoring (on the server)
def files(did, night, search=''):
    c = cfg()
    d = (c.get('devices') or {}).get(did)
    if not d:
        raise Fail('That device isn\'t here')
    d = dict(DEFAULT_DEVICE, **d)
    snaps = snapshots(d)
    snap = next((x for x in snaps if x['id'] == night), snaps[-1] if snaps else None)
    if not snap:
        raise Fail('No backups of it yet')
    entries = [x for x in (restic_json(repo_of(d), 'backup-%s' % d['user'], ['ls', snap['id']], timeout=600) or []) if isinstance(x, dict) and x.get('path')]
    q = (search or '').lower()
    hits = [x for x in entries if q and q in x['path'].lower()] if q else entries[:200]
    # the folder that matches first, and its files; whether each differs from the newest backup
    newest = {x['path']: x for x in (restic_json(repo_of(d), 'backup-%s' % d['user'], ['ls', snaps[-1]['id']], timeout=600) or []) if isinstance(x, dict) and x.get('path')} if snap is not snaps[-1] else {}
    folder = next((x for x in hits if x.get('type') == 'dir'), None)
    rows = []
    if folder:
        kids = [x for x in entries if os.path.dirname(x['path']) == folder['path']]
        rows.append({'path': folder['path'], 'name': short_path(folder['path']), 'dir': True, 'line': '%d files · changed %s' % (len(kids), C.day(parse_t(folder.get('mtime'))))})
        for x in kids[:12]:
            now_x = newest.get(x['path'])
            rows.append({'path': x['path'], 'name': os.path.basename(x['path']), 'dir': x.get('type') == 'dir',
                         'line': '%s · %s' % (C.size_words(x.get('size', 0)).replace('.0 ', ' '), C.clock(parse_t(x.get('mtime'))) if night_of(parse_t(x.get('mtime'))) == night_of(snap['at']) else C.day(parse_t(x.get('mtime')))),
                         'different': bool(now_x and now_x.get('mtime') != x.get('mtime'))})
    else:
        for x in hits[:12]:
            rows.append({'path': x['path'], 'name': short_path(x['path']), 'dir': x.get('type') == 'dir', 'line': C.size_words(x.get('size', 0)), 'different': False})
    return {'ok': True, 'night': snap['id'], 'rows': rows}


def short_path(p):
    """The part of a path people know (after the home folder)."""
    m = re.match(r'^/(var/)?home/[^/]+/(.*)$', p)
    return m.group(2) if m else p.lstrip('/')


def restore(did, night, path, mode):
    """Into Home › Restored from backups › <device> <night>: a new folder each time, never over anything; then to the
    device itself when it's online (Taildrop), where Put back on that device puts it in place."""
    c = cfg()
    d = (c.get('devices') or {}).get(did)
    if not d or mode not in ('back', 'beside'):
        raise Fail('A device, a night, a path and back or beside')
    d = dict(DEFAULT_DEVICE, **d)
    dest = os.path.join(C.HOME, 'Restored from backups', '%s %s' % (d.get('name', did), time.strftime('%Y-%m-%d %H%M')))
    os.makedirs(dest, exist_ok=True)
    code, _ = C.run([restic(), '-r', repo_of(d)] + pw_cmd('backup-%s' % d['user']) + ['restore', night, '--target', dest, '--include', path], timeout=3600)
    if code != 0:
        raise Fail('It couldn\'t be put back: the backup didn\'t open')
    got = os.path.join(dest, path.lstrip('/'))
    sent = False
    host = d.get('host') or did
    if C.which('tailscale') and os.path.exists(got) and os.path.isfile(got):
        sent = C.run(['tailscale', 'file', 'cp', got, '%s:' % host], timeout=300)[0] == 0
    C.event(PART, 'Put back %s from %s\'s backup' % (os.path.basename(path), d.get('name', did)))
    return {'ok': True, 'folder': dest, 'said': ('%s is in Restored from backups%s' % (os.path.basename(path), ', and on its way to the %s' % d.get('name', did) if sent else ''))}


# ------------------------------------------------------------------ the plans for your devices (a user service)
def serve():
    """A small read-only page on this PC's Tailscale address: GET /plan with a device's own backup code (the same
    code rclone checks) answers what that device backs up and when. Nothing else is served."""
    import hashlib
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    ht = os.environ.get('HEARTH_BACKUP_HTPASSWD') or '/etc/hearth/backup-receiver/htpasswd'

    def check(user, pw):
        try:
            with open(ht) as f:
                for l in f:
                    u, _, h = l.strip().partition(':')
                    if u == user and h.startswith('{SHA}'):
                        return secrets.compare_digest(h[5:], base64.b64encode(hashlib.sha1(pw.encode()).digest()).decode())
        except OSError:
            pass
        return False

    class H(BaseHTTPRequestHandler):
        def log_message(self, *a):
            pass

        def do_GET(self):
            auth = self.headers.get('Authorization', '')
            try:
                user, _, pw = base64.b64decode(auth.split(' ', 1)[1]).decode().partition(':') if auth.startswith('Basic ') else ('', '', '')
            except (ValueError, IndexError):
                user, pw = '', ''
            if self.path != '/plan' or not USER_RE.fullmatch(user or '') or not check(user, pw):
                self.send_response(404)
                self.end_headers()
                return
            dev = next((dict(DEFAULT_DEVICE, **d) for d in (cfg().get('devices') or {}).values() if d.get('user') == user), None)
            body = json.dumps({k: dev.get(k) for k in ('home', 'saves', 'settings', 'system', 'games', 'at', 'away')} if dev else {}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)
    ip = ts_ip()
    ThreadingHTTPServer((ip, PLAN_PORT), H).serve_forever()


UNITS = {
    'hearth-server-backups.service': '[Unit]\nDescription=Hearth home server: every device\'s backups, each night\n\n[Service]\nType=oneshot\nNice=19\nIOSchedulingClass=idle\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part backups nightly\n',
    'hearth-server-backups.timer': '[Unit]\nDescription=Hearth home server: backups at 2:00 AM\n\n[Timer]\nOnCalendar=*-*-* 02:00:00\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n',
    'hearth-server-backups-plan.service': '[Unit]\nDescription=Hearth home server: what each device backs up (Tailscale only)\nAfter=network-online.target\n\n[Service]\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part backups serve\nRestart=on-failure\nRestartSec=60\n\n[Install]\nWantedBy=default.target\n',
}


def turn(on):
    d = os.path.join(C.CONFIG, 'systemd', 'user')
    C.set_key(PART, 'on', on)
    if on:
        for name, text in UNITS.items():
            C._atomic(os.path.join(d, name), text)
    if C.which('systemctl') and not os.environ.get('HEARTH_TEST_NO_SYSTEMCTL'):
        C.run(['systemctl', '--user', 'daemon-reload'])
        for u in ('hearth-server-backups.timer', 'hearth-server-backups-plan.service'):
            C.run(['systemctl', '--user', 'enable' if on else 'disable', '--now', u])
    if not on:
        for name in UNITS:
            try:
                os.unlink(os.path.join(d, name))
            except OSError:
                pass
    tonight_items(cfg())
    return {'ok': True, 'said': 'Every device backs up here each night now' if on else 'Backups for every device are off (what\'s backed up stays)'}


def now_(which):
    if which in ('all', 'server'):
        C.run(['systemctl', '--user', 'start', '--no-block', 'hearth-server-backups.service'], timeout=20)
    return {'ok': True, 'said': 'This server backs up now; each device backs up at its next look (within the hour)'}


# ------------------------------------------------------------------ on a device
def join(code):
    if not code.startswith('hearth-backup:'):
        raise Fail("Paste the code the server's Backups page showed (it starts with hearth-backup:)")
    try:
        j = json.loads(base64.urlsafe_b64decode(code.split(':', 1)[1] + '==').decode())
    except (ValueError, UnicodeDecodeError):
        raise Fail('That code isn\'t whole: copy it again')
    clean_join(j)
    code_, out = C.run(['pkexec', '/usr/libexec/nimbus/hearth-home-server-helper', 'server-part', 'backups', 'root', 'device-install'], input_text=json.dumps(j), timeout=120)
    if code_ != 0:
        try:
            raise Fail(json.loads(out.strip().splitlines()[-1]).get('error') or 'It wasn\'t set up')
        except (ValueError, IndexError):
            raise Fail('It wasn\'t set up (the password wasn\'t given?)')
    return {'ok': True, 'said': 'This device backs up to your home server every night now'}


def clean_join(j):
    if not isinstance(j, dict):
        raise Fail('That code isn\'t whole')
    if not re.fullmatch(r'100\.\d{1,3}\.\d{1,3}\.\d{1,3}|[a-z0-9\-]{1,63}(\.[a-z0-9\-]{1,63})*', str(j.get('server', ''))):
        raise Fail('The code names no server')
    if not USER_RE.fullmatch(str(j.get('user', ''))) or not re.fullmatch(r'[A-Za-z0-9\-]{8,64}', str(j.get('rest', ''))) or not re.fullmatch(r'[A-Za-z0-9_\-]{16,64}', str(j.get('key', ''))):
        raise Fail('That code isn\'t whole')
    for k in ('port', 'plan'):
        if not isinstance(j.get(k), int) or not 1 <= j[k] <= 65535:
            raise Fail('That code isn\'t whole')
    return j


DEVICE_UNITS = {
    'hearth-device-backup.service': '[Unit]\nDescription=Hearth: back up this device to your home server\nAfter=network-online.target\n\n[Service]\nType=oneshot\nNice=19\nIOSchedulingClass=idle\nExecStart=/usr/libexec/hearth/hearth-system-part run hearth-home-server-helper /usr/libexec/nimbus/hearth-home-server-helper server-part backups root device-run --if-due\n',
    'hearth-device-backup.timer': '[Unit]\nDescription=Hearth: look whether this device\'s backup is due, every hour\n\n[Timer]\nOnBootSec=20min\nOnCalendar=hourly\nRandomizedDelaySec=10min\nPersistent=true\n\n[Install]\nWantedBy=timers.target\n',
}


def caller_user():
    uid = os.environ.get('PKEXEC_UID') or os.environ.get('SUDO_UID')
    if os.geteuid() != 0 and not os.environ.get('HEARTH_TEST_ROOT'):
        raise Fail('This needs the system (it\'s asked for through pkexec)')
    if uid is None and not os.environ.get('HEARTH_TEST_ROOT'):
        raise Fail('Started without pkexec')
    try:
        return pwd.getpwuid(int(uid if uid is not None else os.getuid()))
    except (KeyError, ValueError):
        raise Fail('Who asked isn\'t known')


def root_cmd(argv):
    if not argv:
        raise Fail('device-install, device-run, device-put or device-restore')
    cmd = argv[0]
    if cmd == 'device-install':
        who = caller_user()
        j = clean_join(json.loads(sys.stdin.read() or '{}'))
        conf = {'server': j['server'], 'port': j['port'], 'plan': j['plan'], 'user': j['user'], 'rest': j['rest'], 'key': j['key'], 'owner': who.pw_name}
        os.makedirs(os.path.dirname(DEVICE_CONF), mode=0o755, exist_ok=True)
        C._atomic(DEVICE_CONF, json.dumps(conf) + '\n', 0o600)
        unit_dir = os.environ.get('HEARTH_DEVICE_UNIT_DIR') or '/etc/systemd/system'
        for n, t in DEVICE_UNITS.items():
            C._atomic(os.path.join(unit_dir, n), t)
        if not os.environ.get('HEARTH_TEST_NO_SYSTEMCTL'):
            C.run(['systemctl', 'daemon-reload'])
            C.run(['systemctl', 'enable', '--now', 'hearth-device-backup.timer'])
        return {'ok': True}
    if cmd == 'device-remove':
        caller_user()
        for p in [DEVICE_CONF] + [os.path.join('/etc/systemd/system', n) for n in DEVICE_UNITS]:
            if p.endswith('.timer') and not os.environ.get('HEARTH_TEST_NO_SYSTEMCTL'):
                C.run(['systemctl', 'disable', '--now', os.path.basename(p)])
            try:
                os.unlink(p)
            except OSError:
                pass
        return {'ok': True, 'said': 'This device doesn\'t back up to the server any more (its backups there stay)'}
    if cmd == 'device-run':
        return device_run('--if-due' in argv)
    if cmd == 'device-put' and len(argv) == 4:
        return device_put(argv[1], argv[2], argv[3])
    if cmd == 'device-restore' and len(argv) == 2:
        return device_restore(argv[1])
    raise Fail('Not one of the device\'s commands')


def device_conf():
    try:
        with open(DEVICE_CONF) as f:
            return json.load(f)
    except (OSError, ValueError):
        raise Fail('This device isn\'t set to back up to a home server')


def restic_env(conf):
    """restic's REST credentials and its key, for this one run (root's own environment, never on a command line)."""
    fd, pwf = tempfile.mkstemp(prefix='hearth-backup-', dir=os.environ.get('HEARTH_TEST_TMP') or '/run')
    with os.fdopen(fd, 'w') as f:
        f.write(conf['key'])
    os.chmod(pwf, 0o600)
    env = dict(os.environ, RESTIC_PASSWORD_FILE=pwf, RESTIC_REST_USERNAME=conf['user'], RESTIC_REST_PASSWORD=conf['rest'])
    return env, pwf


def repo_url(conf):
    return 'rest:http://%s:%d/%s/' % (conf['server'], conf['port'], conf['user'])


def on_power():
    """Plugged in, or at 50% battery or more (a handheld at night)."""
    base = '/sys/class/power_supply'
    bats = []
    try:
        for d in os.listdir(base):
            t = open(os.path.join(base, d, 'type')).read().strip()
            if t == 'Mains' and open(os.path.join(base, d, 'online')).read().strip() == '1':
                return True
            if t == 'Battery':
                bats.append(int(open(os.path.join(base, d, 'capacity')).read().strip()))
    except (OSError, ValueError):
        pass
    return not bats or max(bats) >= 50


def plan_of(conf):
    st_, j = C.http_json('GET', 'http://%s:%d/plan' % (conf['server'], conf['plan']), headers={'Authorization': 'Basic ' + base64.b64encode(('%s:%s' % (conf['user'], conf['rest'])).encode()).decode()}, timeout=15)
    if st_ == 200 and isinstance(j, dict) and j:
        return dict(DEFAULT_DEVICE, **j)
    return dict(DEFAULT_DEVICE)


def device_paths(plan, owner):
    home = pwd.getpwnam(owner).pw_dir
    inc, exc = [], ['--exclude-caches', '--exclude', os.path.join(home, '.cache')]
    if plan.get('system'):
        inc += ['/etc', '/var/lib/hearth', '/var/home' if os.path.isdir('/var/home') else '/home', os.path.join(DEVICE_STATE, 'manifest.json')]
    else:
        if plan.get('home'):
            inc.append(home)
        if plan.get('saves'):
            inc += [p for p in (os.path.join(home, '.local/share/Steam/userdata'), os.path.join(home, '.local/share/hearth/saves'), os.path.join(home, '.config/retroarch/saves'), os.path.join(home, '.local/share/dolphin-emu/GC')) if os.path.isdir(p)]
        if plan.get('settings'):
            inc += [p for p in (os.path.join(home, '.config/nimbus'), os.path.join(home, '.local/share/nimbus'), os.path.join(home, '.local/share/hearth/parts'), '/etc/hearth') if os.path.isdir(p)]
    if not plan.get('games'):
        exc += ['--exclude', os.path.join(home, '.local/share/Steam/steamapps/common'), '--exclude', os.path.join(home, 'Games'), '--exclude', os.path.join(home, '.var/app/com.valvesoftware.Steam')]
    tags = [k for k in ('home', 'saves', 'settings', 'system') if plan.get(k)]
    return sorted(set(inc)), exc, tags


def part_sizes(plan, owner):
    """Each part's size (du at the lowest priority, a minute at most each), for the server's Backups page."""
    outd = {}
    for k in ('home', 'saves', 'settings', 'system'):
        if not plan.get(k):
            continue
        inc, _, _ = device_paths(dict({x: False for x in ('home', 'saves', 'settings', 'system')}, **{k: True, 'games': plan.get('games')}), owner)
        total = 0
        for p in inc:
            if os.path.exists(p):
                code, out = C.run(['nice', '-n', '19', 'du', '-sb', p], timeout=60)
                try:
                    total += int(out.split()[0]) if code == 0 else 0
                except (ValueError, IndexError):
                    pass
        outd[k] = total
    return outd


def due(plan, last, now):
    at = plan.get('at') or '02:00'
    h, m = int(at[:2]), int(at[3:])
    lt = time.localtime(now)
    today_at = time.mktime((lt.tm_year, lt.tm_mon, lt.tm_mday, h, m, 0, 0, 0, -1))
    if now < today_at:
        today_at -= 86400
    if last >= today_at:
        return False
    return now >= today_at or (plan.get('away') and now - last > 24 * 3600)


def device_run(if_due=False):
    conf = device_conf()
    now = time.time()
    os.makedirs(DEVICE_STATE, mode=0o700, exist_ok=True)
    last = {}
    try:
        with open(os.path.join(DEVICE_STATE, 'last.json')) as f:
            last = json.load(f)
    except (OSError, ValueError):
        pass
    plan = plan_of(conf)
    if if_due and not due(plan, float(last.get('at') or 0), now):
        return {'ok': True, 'said': 'Not due'}
    if C.playing():
        return {'ok': True, 'said': 'Waiting: a game is running'}
    if not on_power():
        return {'ok': True, 'said': 'Waiting: plug it in or charge it to 50%'}
    # what Hearth OS and its apps are (so a whole-device restore knows what to put back)
    man = {'at': int(now)}
    for k, cmd in (('bootc', ['bootc', 'status', '--json']), ('flatpaks', ['flatpak', 'list', '--app', '--columns=application'])):
        code, out = C.run(cmd, timeout=60)
        man[k] = out if code == 0 else ''
    C._atomic(os.path.join(DEVICE_STATE, 'manifest.json'), json.dumps(man) + '\n', 0o600)
    inc, exc, tags = device_paths(plan, conf['owner'])
    tags = tags + ['size-%s=%d' % (k, n) for k, n in part_sizes(plan, conf['owner']).items()]
    env, pwf = restic_env(conf)
    try:
        code, _ = C.run([restic(), '-r', repo_url(conf), 'snapshots', '--json'], timeout=120, env=env)
        if code != 0:
            C.run([restic(), '-r', repo_url(conf), 'init'], timeout=120, env=env)
        args = [restic(), '-r', repo_url(conf), 'backup', '--json', '--host', os.uname().nodename] + sum((['--tag', t] for t in ['hearth'] + tags), []) + exc + inc
        code, out = C.run(args, timeout=8 * 3600, env=env)
    finally:
        try:
            os.unlink(pwf)
        except OSError:
            pass
    rec = {'at': int(now), 'ok': code == 0, 'bytes': _added(out)}
    C._atomic(os.path.join(DEVICE_STATE, 'last.json'), json.dumps(rec) + '\n', 0o644)
    if code != 0:
        raise Fail('The backup didn\'t finish (it tries again within the hour)')
    return {'ok': True, 'said': 'Backed up: %s sent' % C.size_words(rec['bytes'])}


def device_put(snap, path, mode):
    """A file or folder of yours (inside your Home only) from one night: back where it was (what's there now is
    kept beside it, NAME (before DATE)) or beside it as NAME (DATE). Never over anything without keeping it."""
    who = caller_user()
    conf = device_conf()
    if not re.fullmatch(r'[0-9a-f]{8,64}', snap) or mode not in ('back', 'beside'):
        raise Fail('A night and back or beside')
    real = os.path.realpath(path)
    home = os.path.realpath(who.pw_dir)
    if not real.startswith(home + os.sep):
        raise Fail('Only your own files are put back from here')
    env, pwf = restic_env(conf)
    try:
        with tempfile.TemporaryDirectory(prefix='hearth-put-', dir=os.path.dirname(real) if os.path.isdir(os.path.dirname(real)) else None) as tmp:
            code, _ = C.run([restic(), '-r', repo_url(conf), 'restore', snap, '--target', tmp, '--include', real], timeout=3600, env=env)
            got = os.path.join(tmp, real.lstrip('/'))
            if code != 0 or not os.path.exists(got):
                raise Fail('That night\'s backup doesn\'t have it')
            stamp = time.strftime('%b %d %H%M')
            base, ext = os.path.splitext(real)
            if mode == 'beside':
                dest = '%s (%s)%s' % (base, stamp, ext)
            else:
                if os.path.exists(real):
                    os.rename(real, '%s (before %s)%s' % (base, stamp, ext))
                dest = real
            shutil.move(got, dest)
            for root, dirs, fs in os.walk(dest) if os.path.isdir(dest) else [(os.path.dirname(dest), [], [os.path.basename(dest)])]:
                for n in dirs + fs:
                    os.lchown(os.path.join(root, n), who.pw_uid, who.pw_gid)
            os.lchown(dest, who.pw_uid, who.pw_gid)
    finally:
        try:
            os.unlink(pwf)
        except OSError:
            pass
    return {'ok': True, 'said': 'Put back%s' % ('' if mode == 'back' else ' beside the one you have')}


def device_restore(snap):
    """The whole device as it was that night: your Home, Hearth's settings and lists and /etc/hearth, put back over
    what's there (restic restore). Hearth OS itself, the boot menu and the boot chain are never touched (the device
    always starts: red flag 2); the apps list is put back for the Store to install them again. Restart afterwards."""
    caller_user()
    conf = device_conf()
    if not re.fullmatch(r'[0-9a-f]{8,64}', snap):
        raise Fail('Pick a night')
    env, pwf = restic_env(conf)
    try:
        code, _ = C.run([restic(), '-r', repo_url(conf), 'restore', snap, '--target', '/', '--include', '/var/home', '--include', '/home', '--include', '/var/lib/hearth', '--include', '/etc/hearth',
                         '--exclude', '/var/lib/hearth/system-parts', '--exclude', DEVICE_CONF], timeout=12 * 3600, env=env)
    finally:
        try:
            os.unlink(pwf)
        except OSError:
            pass
    if code != 0:
        raise Fail('It couldn\'t all be put back: what was put back stays, the rest is as it was')
    return {'ok': True, 'said': 'This device is back the way it was that night: restart it to finish'}


def put(snap, path, mode):
    """On a device, as you: asks the system (pkexec) to put it back."""
    code, out = C.run(['pkexec', '/usr/libexec/nimbus/hearth-home-server-helper', 'server-part', 'backups', 'root', 'device-put', snap, os.path.abspath(os.path.expanduser(path)), mode], timeout=3600)
    try:
        r = json.loads(out.strip().splitlines()[-1])
    except (ValueError, IndexError):
        r = {'ok': False, 'error': 'It wasn\'t put back'}
    if not r.get('ok'):
        raise Fail(r.get('error') or 'It wasn\'t put back')
    return r


def main(argv):
    if not argv:
        raise Fail('status, summary, add, set, on, off, nightly, now, files, restore, serve, join, put or root')
    cmd, rest = argv[0], argv[1:]
    if cmd == 'status':
        return status(rest[0] if rest else '')
    if cmd == 'summary':
        return summary()
    if cmd == 'add' and len(rest) == 2:
        return add(rest[0], rest[1])
    if cmd == 'set' and len(rest) == 3:
        return set_choice(*rest)
    if cmd in ('on', 'off'):
        return turn(cmd == 'on')
    if cmd == 'nightly':
        return nightly()
    if cmd == 'now' and len(rest) == 1:
        return now_(rest[0])
    if cmd == 'code' and len(rest) == 1:
        return code_of(rest[0])
    if cmd == 'files' and len(rest) in (2, 3):
        return files(rest[0], rest[1], rest[2] if len(rest) == 3 else '')
    if cmd == 'restore' and len(rest) == 4:
        return restore(*rest)
    if cmd == 'serve':
        serve()
        return None
    if cmd == 'join' and len(rest) == 1:
        return join(rest[0])
    if cmd == 'put' and len(rest) == 3:
        return put(*rest)
    if cmd == 'root':
        return root_cmd(rest)
    raise Fail('Not one of the backups\' commands')


if __name__ == '__main__':
    sys.exit(C.main_guard(main))
