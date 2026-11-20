#!/usr/bin/env python3
"""Hearth Link end to end: two real `hearth-link serve` instances, each in its own network namespace with its own
home folder, driven over their sockets the way Hearth's UI does (`hearth-link call ...`). Run as root (namespaces).

  e2e.py lan        one network: pairing first: too many wrong codes stop it, a code that ran out, a short code
                    (looked up on a local pkarr relay: a typo caught at once, a code nobody shows, then pairing,
                    then the code used up), the long link; then discovery (kind and edition shown) and pairing by
                    picking the device and its 6-digit code (a wrong code first, then retried), a multi-GB synthetic Steam
                    game copied (speed posted), a copy killed halfway and resumed, an update copied as only its
                    changed chunks, every file's hash compared, Hearth data synced both ways, quiet while playing,
                    unpairing, Hearth Storage (shared folder, media link, game archive there and back, backups),
                    Continue on this device, updating every device at once, local AI over Hearth Link
  e2e.py family     pair once for all your devices, friends' devices, Send to TV (a PC to a Hearth TV), a family
                    member's device (shared among your devices, nothing personal until its own switches say so), and
                    Family and friends with Moonlight (moonlight.rs): off until turned on, a person added with a code,
                    asked every time (no, then yes), Moonlight's TCP and UDP through Hearth Link to a stand-in
                    Sunshine from Moonlight's own private network, its PIN, Stop sharing now, Always allow with Games
                    only and a time limit, turning it off ending the session at once, removing the person
  e2e.py internet   both devices behind their own NAT, a local iroh relay in between: pairing with a short code
                    (found through a pkarr relay on the "internet" too, nothing else typed), a copy
                    through the relay with direct UDP blocked (speed posted, the copy says it's relayed), then with
                    UDP allowed (hole punching; posted as information); Moonlight's tunnel through the relay and
                    then hole punched (TCP speed, UDP packets and how many came back, posted)

Environment: LINK_BIN (the hearth-link binary), RELAY_BIN (iroh-relay, for internet), SCALE (game size factor, 1.0).
"""
import hashlib
import json
import os
import random
import re
import shutil
import signal
import subprocess
import sys
import time

BIN = os.environ.get('LINK_BIN', 'hearth-link')
ROOT = '/tmp/hl-e2e'
SCALE = float(os.environ.get('SCALE', '1'))
MB = 1 << 20


def sh(cmd, check=True):
    r = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if check and r.returncode != 0:
        raise RuntimeError('%s: %s %s' % (cmd, r.stdout, r.stderr))
    return r.stdout


def notice(title, msg):
    print('::notice title=%s::%s' % (title, msg), flush=True)


PROGRESS = []


def tail(path, n=1800):
    try:
        with open(path, 'rb') as f:
            f.seek(0, 2)
            f.seek(max(0, f.tell() - n))
            return f.read().decode('utf-8', 'replace')
    except OSError:
        return ''


def esc(t):
    return t.replace('%', '%25').replace('\r', '').replace('\n', '%0A')


def fail(msg):
    print('::error title=Hearth Link e2e::%s' % esc(msg + '\n--- progress\n' + '\n'.join(PROGRESS[-25:])), flush=True)
    for name in ('A', 'B', 'relay', 'pkarr'):
        t = tail(os.path.join(ROOT, '%s.log' % name))
        if t:
            print('::error title=%s.log::%s' % (name, esc(t)), flush=True)
    raise SystemExit(1)


def log(msg):
    line = '[%s] %s' % (time.strftime('%H:%M:%S'), msg)
    PROGRESS.append(line)
    print(line, flush=True)


class Dev:
    def __init__(self, name, ns, extra_env=None):
        self.name, self.ns = name, ns
        self.base = os.path.join(ROOT, name)
        self.home = os.path.join(self.base, 'home')
        self.run = os.path.join(self.base, 'run')
        for d in (self.home, self.run):
            os.makedirs(d, exist_ok=True)
        os.chmod(self.run, 0o700)
        self.env = dict(os.environ)
        self.env.update({
            'HOME': self.home, 'USER': name, 'XDG_DATA_HOME': os.path.join(self.home, '.local/share'),
            'XDG_CONFIG_HOME': os.path.join(self.home, '.config'), 'XDG_RUNTIME_DIR': self.run,
            'HEARTH_LINK_SOCKET': os.path.join(self.run, 'link.sock'), 'RUST_BACKTRACE': '1',
        })
        self.env.update(extra_env or {})
        self.proc = None
        self.id = None
        self.logf = None

    @property
    def data(self):
        return os.path.join(self.home, '.local/share/nimbus')

    @property
    def link(self):
        return os.path.join(self.data, 'link')

    @property
    def steam(self):
        return os.path.join(self.home, '.local/share/Steam')

    def start(self):
        self.logf = open(os.path.join(ROOT, '%s.log' % self.name), 'a')
        self.proc = subprocess.Popen(['ip', 'netns', 'exec', self.ns, BIN, 'serve'], env=self.env, stdout=self.logf, stderr=subprocess.STDOUT)
        for _ in range(100):
            try:
                st = self.call('status', timeout=5)
                self.id = st['me']['id']
                if st['me']['name'] != self.name:
                    # Named after the test's device (A, B) rather than the runner, as the person would name them.
                    self.call('settings.set', {'name': self.name}, timeout=10)
                    st = self.call('status', timeout=5)
                return st
            except Exception:
                time.sleep(0.2)
        fail('%s did not start (see its log)' % self.name)

    def kill(self, sig=signal.SIGKILL):
        if self.proc and self.proc.poll() is None:
            # ip netns exec execs the program: the pid is hearth-link's.
            self.proc.send_signal(sig)
            try:
                self.proc.wait(10)
            except subprocess.TimeoutExpired:
                self.proc.kill()

    def call(self, cmd, args=None, timeout=120, ok=True):
        r = subprocess.run([BIN, 'call', cmd, json.dumps(args or {})], env=self.env, capture_output=True, text=True, timeout=timeout + 5)
        try:
            v = json.loads(r.stdout.strip().splitlines()[-1])
        except Exception:
            raise RuntimeError('%s %s: no answer: %r %r' % (self.name, cmd, r.stdout, r.stderr))
        if ok and r.returncode != 0:
            raise RuntimeError('%s %s: %s' % (self.name, cmd, v.get('error') if isinstance(v, dict) else v))
        return v

    def status(self):
        return self.call('status')

    def job(self, jid):
        for j in self.status()['transfers']:
            if j['id'] == jid:
                return j
        return None


# ------------------------------------------------------------------ a pkarr relay (lookup.rs), as n0's and pkarr's

PKARR_PORT = 8099
PKARR = r'''
import http.server, sys
store = {}
class H(http.server.BaseHTTPRequestHandler):
    def do_PUT(self):
        body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        store[self.path] = body
        self.send_response(204)
        self.end_headers()
    def do_GET(self):
        b = store.get(self.path)
        if b is None:
            self.send_response(404)
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        self.send_response(200)
        self.send_header("Content-Type", "application/pkarr.org/relays#payload")
        self.send_header("Content-Length", str(len(b)))
        self.end_headers()
        self.wfile.write(b)
    def log_message(self, *a):
        sys.stderr.write("pkarr: %s %s\n" % (self.command, self.path[:24]))
http.server.ThreadingHTTPServer((sys.argv[1], int(sys.argv[2])), H).serve_forever()
'''


def pkarr_relay(ns, ip):
    path = os.path.join(ROOT, 'pkarr.py')
    with open(path, 'w') as f:
        f.write(PKARR)
    lf = open(os.path.join(ROOT, 'pkarr.log'), 'w')
    p = subprocess.Popen(['ip', 'netns', 'exec', ns, sys.executable, '-I', path, ip, str(PKARR_PORT)], stdout=lf, stderr=subprocess.STDOUT)
    time.sleep(1)
    return p, 'http://%s:%d/pkarr' % (ip, PKARR_PORT)


ALPHABET = '23456789ABCDEFGHJKMNPQRSTUVWXYZ'
CODE_RE = re.compile(r'^[%s]{3}-[%s]{3}-[%s]{4}$' % ((ALPHABET,) * 3))


def short_code(data):
    """A valid short code for 9 characters (shortcode.rs: weights 1..10, sum ≡ 0 mod 31)."""
    v = [ALPHABET.index(c) for c in data]
    s = sum((i + 1) * x for i, x in enumerate(v)) % 31
    return data + ALPHABET[((31 - s) % 31) * 28 % 31]


# ------------------------------------------------------------------ synthetic games

def rand_bytes(n, rng):
    return rng.randbytes(n) if hasattr(rng, 'randbytes') else os.urandom(n)


def write_random(path, size):
    with open(path, 'wb') as f:
        left = size
        while left > 0:
            n = min(left, 8 * MB)
            f.write(os.urandom(n))
            left -= n


def write_text(path, size, seed):
    words = ('hearth link copies games fast over quic with blake3 chunks zstd where it helps resume verify '
             'steam gog epic beacon xodus proton prefix save settings shader texture level map sound ').split()
    rng = random.Random(seed)
    line = ' '.join(rng.choice(words) for _ in range(4000)).encode()
    with open(path, 'wb') as f:
        left = size
        i = 0
        while left > 0:
            piece = (b'%08d ' % i) + line
            f.write(piece[:left])
            left -= min(left, len(piece))
            i += 1


def steam_library(home):
    s = os.path.join(home, '.local/share/Steam')
    os.makedirs(os.path.join(s, 'steamapps/common'), exist_ok=True)
    with open(os.path.join(s, 'steamapps/libraryfolders.vdf'), 'w') as f:
        f.write('"libraryfolders"\n{\n\t"0"\n\t{\n\t\t"path"\t\t"%s"\n\t}\n}\n' % s)
    return s


def make_game(steam, appid, name, huge_random, huge_text, small_files, medium):
    """A Steam game: huge random and compressible files, medium ones, many small ones, links, an empty file, a
    read-only one, and its Proton prefix (compatdata) with Wine's links."""
    root = os.path.join(steam, 'steamapps/common', name)
    os.makedirs(root, exist_ok=True)
    if huge_random:
        write_random(os.path.join(root, 'huge.pak'), huge_random)
    if huge_text:
        write_text(os.path.join(root, 'levels.dat'), huge_text, appid)
    for i in range(medium):
        write_random(os.path.join(root, 'medium-%02d.bin' % i), (5 + (i * 7) % 30) * MB)
    rng = random.Random(appid)
    for i in range(small_files):
        d = os.path.join(root, 'data', 'set%02d' % (i % 40), 'sub%d' % (i % 3))
        os.makedirs(d, exist_ok=True)
        size = rng.randint(1, 64) * 1024 + rng.randint(0, 1023)
        p = os.path.join(d, 'file%05d.%s' % (i, 'txt' if i % 2 else 'bin'))
        with open(p, 'wb') as f:
            f.write((('config line %d of hearth link\n' % i) * (size // 30 + 1)).encode()[:size] if i % 2 else os.urandom(size))
    open(os.path.join(root, 'empty.cfg'), 'w').close()
    ro = os.path.join(root, 'readonly.txt')
    with open(ro, 'w') as f:
        f.write('read only\n')
    os.chmod(ro, 0o444)
    os.makedirs(os.path.join(root, 'bin'), exist_ok=True)
    with open(os.path.join(root, 'bin/game.sh'), 'w') as f:
        f.write('#!/bin/sh\necho game\n')
    os.chmod(os.path.join(root, 'bin/game.sh'), 0o755)
    if huge_random:
        os.symlink('../huge.pak', os.path.join(root, 'bin/current.pak'))
    pfx = os.path.join(steam, 'steamapps/compatdata', str(appid), 'pfx')
    os.makedirs(os.path.join(pfx, 'drive_c/users/steamuser/Saved Games/Game'), exist_ok=True)
    os.makedirs(os.path.join(pfx, 'dosdevices'), exist_ok=True)
    write_random(os.path.join(pfx, 'drive_c/users/steamuser/Saved Games/Game/save1.sav'), 2 * MB)
    with open(os.path.join(pfx, 'user.reg'), 'w') as f:
        f.write('WINE REGISTRY Version 2\n' * 2000)
    os.symlink('../drive_c', os.path.join(pfx, 'dosdevices/c:'))
    os.symlink('/', os.path.join(pfx, 'dosdevices/z:'))
    with open(os.path.join(steam, 'steamapps/appmanifest_%d.acf' % appid), 'w') as f:
        f.write('"AppState"\n{\n\t"appid"\t\t"%d"\n\t"name"\t\t"%s"\n\t"StateFlags"\t\t"4"\n\t"installdir"\t\t"%s"\n'
                '\t"SizeOnDisk"\t\t"%d"\n\t"BytesToDownload"\t\t"0"\n}\n' % (appid, name, name, tree_size(root)))
    return root


def tree_size(root):
    n = 0
    for d, _, fs in os.walk(root):
        for f in fs:
            p = os.path.join(d, f)
            if not os.path.islink(p):
                n += os.path.getsize(p)
    return n


def tree_digest(root):
    """Every entry: path, kind, size, permissions, time, content hash (files) or target (links)."""
    out = {}
    for d, dirs, fs in os.walk(root):
        for name in dirs + fs:
            p = os.path.join(d, name)
            rel = os.path.relpath(p, root)
            st = os.lstat(p)
            if os.path.islink(p):
                out[rel] = ('l', os.readlink(p))
            elif os.path.isdir(p):
                out[rel] = ('d',)
            else:
                h = hashlib.sha256()
                with open(p, 'rb') as f:
                    while True:
                        b = f.read(8 * MB)
                        if not b:
                            break
                        h.update(b)
                out[rel] = ('f', st.st_size, st.st_mode & 0o777, int(st.st_mtime), h.hexdigest())
    return out


def same_tree(a, b, what):
    da, db = tree_digest(a), tree_digest(b)
    if da != db:
        missing = sorted(set(da) - set(db))[:5]
        extra = sorted(set(db) - set(da))[:5]
        diff = [k for k in da if k in db and da[k] != db[k]][:5]
        fail('%s differs: missing %s, extra %s, different %s %s' % (what, missing, extra, diff, [(da[k], db[k]) for k in diff[:2]]))
    return len(da)


def wait_job(dev, jid, until=None, limit=900):
    t0 = time.time()
    last = 0
    moved, seen = time.time(), None
    while time.time() - t0 < limit:
        j = dev.job(jid)
        if j is None:
            fail('%s lost the copy %s' % (dev.name, jid))
        if j['state'] == 'error':
            fail('copy failed on %s: %s' % (dev.name, j.get('error')))
        if j['state'] == 'done':
            return j
        if until and until(j):
            return j
        now = (j['state'], j['done'], j['skipped'])
        if now != seen:
            seen, moved = now, time.time()
        elif time.time() - moved > 90:
            fail('the copy on %s made no progress for 90 s: %s' % (dev.name, json.dumps(j)))
        if time.time() - last > 10:
            last = time.time()
            log('%s: %s %s/%s MB, %.0f MB/s, eta %s, relay %s' % (dev.name, j['state'], j['done'] // MB, j['total'] // MB, (j.get('speed') or 0) / MB, j.get('eta'), j.get('relay')))
        time.sleep(0.5)
    fail('the copy took too long: %s' % dev.job(jid))


def wait_for(what, fn, limit=60):
    t0 = time.time()
    while time.time() - t0 < limit:
        try:
            v = fn()
            if v:
                return v
        except Exception as e:
            last = e
        time.sleep(0.5)
    fail('waited too long for %s' % what)


def read_json(p):
    try:
        with open(p) as f:
            return json.load(f)
    except Exception:
        return None


def write_json(p, v):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        json.dump(v, f)


def hearth_applies(dev):
    """What Hearth (hearth-core link.rs apply_inbox) does with the inbox, for the parts this test checks."""
    inbox = read_json(os.path.join(dev.link, 'inbox.json'))
    if not inbox:
        return
    lib = read_json(os.path.join(dev.data, 'library.json')) or {}
    pp = (inbox.get('profiles') or {}).get('main') or {}
    for k, v in (pp.get('set') or {}).items():
        kind, _, gid = k.partition('/')
        if kind in ('fav', 'hidden'):
            key = 'favorites' if kind == 'fav' else 'hidden'
            lst = [x for x in lib.get(key, []) if x != gid]
            if v is True:
                lst.append(gid)
            lib[key] = lst
        elif kind == 'profile':
            lib.setdefault('profile', {}).update(v or {})
    for g, n in (pp.get('add') or {}).items():
        s = lib.setdefault('stats', {}).setdefault(g, {'minutes': 0, 'lastPlayed': 0})
        s['minutes'] += n
    for g, n in (pp.get('last') or {}).items():
        s = lib.setdefault('stats', {}).setdefault(g, {'minutes': 0, 'lastPlayed': 0})
        s['lastPlayed'] = max(s['lastPlayed'], n)
    write_json(os.path.join(dev.data, 'library.json'), lib)
    os.remove(os.path.join(dev.link, 'inbox.json'))


# ------------------------------------------------------------------ networks

def lan_network():
    for ns in ('hlA', 'hlB'):
        sh('ip netns del %s' % ns, check=False)
        sh('ip netns add %s' % ns)
        sh('ip -n %s link set lo up' % ns)
    sh('ip link add vA type veth peer name vB')
    sh('ip link set vA netns hlA')
    sh('ip link set vB netns hlB')
    sh('ip -n hlA addr add 10.77.0.1/24 brd + dev vA')
    sh('ip -n hlB addr add 10.77.0.2/24 brd + dev vB')
    sh('ip -n hlA link set vA up')
    sh('ip -n hlB link set vB up')


def internet_network():
    """A ── natA ── inet (relay) ── natB ── B: each device behind its own NAT (masquerade)."""
    for ns in ('hiA', 'natA', 'inet', 'natB', 'hiB'):
        sh('ip netns del %s' % ns, check=False)
        sh('ip netns add %s' % ns)
        sh('ip -n %s link set lo up' % ns)

    def link(a, an, b, bn):
        sh('ip link add %s type veth peer name %s' % (an, bn))
        sh('ip link set %s netns %s' % (an, a))
        sh('ip link set %s netns %s' % (bn, b))
        sh('ip -n %s link set %s up' % (a, an))
        sh('ip -n %s link set %s up' % (b, bn))
    link('hiA', 'a0', 'natA', 'a1')
    link('natA', 'wa0', 'inet', 'wa1')
    link('inet', 'wb1', 'natB', 'wb0')
    link('natB', 'b1', 'hiB', 'b0')
    sh('ip -n hiA addr add 10.1.0.2/24 dev a0 && ip -n hiA route add default via 10.1.0.1')
    sh('ip -n natA addr add 10.1.0.1/24 dev a1 && ip -n natA addr add 198.18.1.2/24 dev wa0 && ip -n natA route add default via 198.18.1.1')
    sh('ip -n inet addr add 198.18.1.1/24 dev wa1 && ip -n inet addr add 198.18.2.1/24 dev wb1')
    sh('ip -n natB addr add 198.18.2.2/24 dev wb0 && ip -n natB addr add 10.2.0.1/24 dev b1 && ip -n natB route add default via 198.18.2.1')
    sh('ip -n hiB addr add 10.2.0.2/24 dev b0 && ip -n hiB route add default via 10.2.0.1')
    for ns in ('natA', 'inet', 'natB'):
        sh('ip netns exec %s sysctl -qw net.ipv4.ip_forward=1' % ns)
    sh('ip netns exec natA iptables -t nat -A POSTROUTING -o wa0 -j MASQUERADE')
    sh('ip netns exec natB iptables -t nat -A POSTROUTING -o wb0 -j MASQUERADE')


# ------------------------------------------------------------------ the tests

def pair(a, b, ticket=None):
    """B opens pairing; A picks it from the devices it sees on the network (or uses B's long link) and types the
    6-digit code: a wrong one first (told how many tries are left, nothing to start over), then the right one."""
    code = b.call('pair.open')['code']
    if ticket:
        target = ticket
    else:
        n = wait_for('A to list B as ready to pair', lambda: next((n for n in a.status()['nearby'] if n['id'] == b.id and n['pairing']), None), 20)
        if n.get('kind') != 'handheld' or n.get('edition') != 'Peppermint' or n.get('name') != 'B':
            fail('B as A lists it: %s' % n)
        target = n['id']
    bad = '%06d' % ((int(code) + 1) % 1000000)
    r = a.call('pair.join', {'id': target, 'code': bad}, ok=False)
    if not r.get('error', '').startswith('Wrong code') or '4 tries left' not in r['error']:
        fail('a wrong code: %s' % r)
    ev = b.status().get('pairEvent') or {}
    if ev.get('state') != 'wrong' or ev.get('triesLeft') != 4 or ev.get('name') != 'A':
        fail('B after a wrong code: %s' % ev)
    if not b.status()['pairing']:
        fail('a wrong code stopped pairing on B')
    log('wrong code refused, pairing still open: %s' % r['error'])
    time.sleep(0.5)
    r = a.call('pair.join', {'id': target, 'code': code[:3] + ' ' + code[3:]}, timeout=90)
    if r.get('id') != b.id:
        fail('pairing: %s' % r)
    wait_for('both sides to list each other', lambda: any(p['id'] == a.id for p in b.status()['peers']) and any(p['id'] == b.id for p in a.status()['peers']), 20)
    st = b.status()
    if st['pairing'] is not None:
        fail('the code still showed after pairing')
    if (st.get('pairEvent') or {}).get('state') != 'paired':
        fail('B doesn\'t say it paired: %s' % st.get('pairEvent'))
    notice('pairing', 'paired %s and %s %s and its 6-digit code (SPAKE2); a wrong code first said "%s" and the right one then paired, nothing started over'
           % (a.name, b.name, 'with B\'s long link' if ticket else 'by picking B from the devices on the network (shown as %s, %s)' % (n['kind'], n['edition']), r.get('error') or 'Wrong code. 4 tries left'))


def unpair(a, b):
    a.call('unpair', {'id': b.id})
    wait_for('B to forget A', lambda: not any(p['id'] == a.id for p in b.status()['peers']), 20)
    wait_for('A to forget B', lambda: not any(p['id'] == b.id for p in a.status()['peers']), 20)


def lockout(a, b):
    """Five wrong codes stop pairing on B; the right code doesn't work after that."""
    code = b.call('pair.open')['code']
    for i in range(5):
        bad = '%06d' % ((int(code) + 1 + i) % 1000000)
        r = a.call('pair.join', {'id': b.id, 'code': bad}, ok=False)
        want = ['4 tries left', '3 tries left', '2 tries left', '1 try left', 'last try'][i]
        if not r.get('error', '').startswith('Wrong code') or want not in r['error']:
            fail('wrong code %d: %s (wanted "%s")' % (i + 1, r, want))
        time.sleep(0.4)
    st = b.status()
    if st['pairing'] is not None or (st.get('pairEvent') or {}).get('state') != 'locked':
        fail('pairing still open on B after 5 wrong codes: %s %s' % (st['pairing'], st.get('pairEvent')))
    r = a.call('pair.join', {'id': b.id, 'code': code}, ok=False)
    if 'error' not in r or any(p['id'] == a.id for p in b.status()['peers']):
        fail('the right code paired after the lockout: %s' % r)
    notice('lockout', '5 wrong codes told A the tries left each time (4, 3, 2, 1, last); then pairing stopped on B and the right code no longer worked ("%s")' % r['error'])


def expiry(a, b):
    code = b.call('pair.open', {'seconds': 3})['code']
    time.sleep(4.5)
    if b.status()['pairing'] is not None:
        fail('the code still showed after it ran out')
    r = a.call('pair.join', {'id': b.id, 'code': code}, ok=False)
    if 'expired' not in r.get('error', '').lower():
        fail('a code that ran out: %s' % r)
    b.call('pair.close')
    notice('expiry', 'a 6-digit code that ran out: "%s"' % r['error'])


def short_codes(a, b):
    """Pairing from a short code alone: looked up (pkarr), checked as typed, SPAKE2 with it, single use, runs out."""
    r = b.call('pair.open', {'internet': True}, timeout=60)
    net = r.get('netCode') or ''
    if not CODE_RE.match(net):
        fail('the short code: %r' % r)
    # One character off: caught before anything goes over the network.
    i = 5
    typo = net[:i] + (ALPHABET[(ALPHABET.index(net[i]) + 1) % 31]) + net[i + 1:]
    t0 = time.time()
    r = a.call('pair.join', {'code': typo}, ok=False)
    if not r.get('error', '').startswith("That code doesn't look right") or time.time() - t0 > 3:
        fail('a typo in the short code: %s (%.1f s)' % (r, time.time() - t0))
    for bad in (net.replace('-', '')[:9] + '0', net[:-1]):
        r = a.call('pair.join', {'code': bad}, ok=False)
        if not r.get('error', '').startswith("That code doesn't look right"):
            fail('%r: %s' % (bad, r))
    # A code that checks out but nobody shows.
    other = short_code(''.join(random.choice(ALPHABET) for _ in range(9)))
    r = a.call('pair.join', {'code': other}, ok=False, timeout=60)
    if 'No device is showing that code' not in r.get('error', ''):
        fail('a code nobody shows: %s' % r)
    # The right one, typed in small letters with spaces.
    t0 = time.time()
    r = a.call('pair.join', {'code': net.lower().replace('-', ' ')}, timeout=90)
    if r.get('id') != b.id or r.get('name') != 'B':
        fail('pairing with the short code: %s' % r)
    took = time.time() - t0
    wait_for('B to list A', lambda: any(p['id'] == a.id for p in b.status()['peers']), 20)
    st = b.status()
    if st['pairing'] is not None or (st.get('pairEvent') or {}).get('state') != 'paired':
        fail('B after pairing with the short code: %s %s' % (st['pairing'], st.get('pairEvent')))
    # Used: the record says so a moment later.
    r = wait_for('the used code to be refused', lambda: (lambda x: x if 'error' in x else None)(a.call('pair.join', {'code': net}, ok=False, timeout=60)), 30)
    unpair(a, b)
    # Runs out by itself.
    r2 = b.call('pair.open', {'internet': True, 'seconds': 3}, timeout=60)
    time.sleep(7)
    r3 = a.call('pair.join', {'code': r2['netCode']}, ok=False, timeout=60)
    if 'ran out' not in r3.get('error', '') and 'expired' not in r3.get('error', '').lower():
        fail('a short code that ran out: %s' % r3)
    if b.status()['pairing'] is not None:
        fail('pairing still open after the short code ran out')
    notice('short code', 'paired from a %d-character code alone (%s, found through a pkarr relay, SPAKE2 with it) in %.2f s; '
           'a typo was caught at once ("%s..."), a code nobody shows said so, the used code was refused ("%s"), one that ran out too ("%s")'
           % (len(net.replace('-', '')), re.sub(r'[A-Z2-9]', 'X', net), took, "That code doesn't look right", r['error'], r3['error']))


def copy(receiver, sender, game, what, prefix=True):
    games = receiver.call('games', {'peer': sender.id, 'refresh': True}, timeout=60)['games']
    listed = [g for g in games if g['id'] == game]
    if not listed:
        fail('%s does not list %s: %s' % (sender.name, game, [g['id'] for g in games]))
    t0 = time.time()
    j = receiver.call('get', {'peer': sender.id, 'game': game, 'prefix': prefix})
    return j, listed[0], t0


def report(title, j, t0, size):
    wall = time.time() - t0
    secs = (j.get('dataMs') or 1) / 1000.0
    mbs = j['done'] / MB / secs if secs > 0 else 0
    wire = j.get('wire') or 0
    notice(title, '%.2f GB in %.1f s: %.0f MB/s (%.0f Mbit/s) while copying; %.1f s from Get to done; %.0f MB on the wire for %.0f MB of data (compression where it helped); relay: %s'
           % (j['done'] / 1e9, secs, mbs, mbs * 8 * 1.048576, wall, wire / MB, j['done'] / MB, j.get('relay')))
    return mbs


def storage(a, b):
    """Hearth Storage (storage.rs): A as the server, B using it. A shared folder (hidden files not listed or given),
    a file copied over, a byte range played through B's 127.0.0.1 link, a game moved to A's archive (checked
    against B's copy, then B told it's safe), deleted on B and copied back as the Steam game it was, B's saves
    and screenshots backed up (a prefix's Temp left out), and A refusing once it's turned off."""
    import glob
    st = b.call('storage.servers')
    if any(s['id'] == a.id for s in st['servers']):
        fail('A says it is Hearth Storage before it was turned on: %s' % st['servers'])
    a.call('storage.set', {'server': True})
    vids = os.path.join(a.home, 'Videos')
    os.makedirs(os.path.join(vids, 'Films'), exist_ok=True)
    write_random(os.path.join(vids, 'Films', 'clip.mkv'), 5 * MB + 12345)
    with open(os.path.join(vids, '.secret'), 'w') as f:
        f.write('not shared\n')
    share = a.call('storage.share.add', {'path': vids})['share']['id']
    st = b.call('storage.servers')
    srv = [s for s in st['servers'] if s['id'] == a.id]
    if not srv or not any(x['id'] == share for x in (srv[0].get('shares') or [])):
        fail('B does not see A as Hearth Storage with its shared folder: %s' % st['servers'])
    top = [e['name'] for e in b.call('storage.ls', {'server': a.id, 'share': share, 'path': ''})['entries']]
    if top != ['Films']:
        fail('the shared folder lists %s (hidden files must not show)' % top)
    r = b.call('storage.get', {'server': a.id, 'share': share, 'path': '.secret'}, ok=False)
    if 'error' not in r:
        fail('a hidden file was given out: %s' % r)
    # A file copied over (Hearth Link's copy engine: a job in the copies list).
    j = b.call('storage.get', {'server': a.id, 'share': share, 'path': 'Films/clip.mkv'})
    wait_job(b, j['id'], limit=120)
    got = os.path.join(b.home, 'Downloads', 'clip.mkv')
    if hashlib.sha256(open(got, 'rb').read()).hexdigest() != hashlib.sha256(open(os.path.join(vids, 'Films', 'clip.mkv'), 'rb').read()).hexdigest():
        fail('the copied file differs')
    # Played: a range through B's local link, as a player asks for it.
    url = b.call('storage.stream', {'server': a.id, 'share': share, 'path': 'Films/clip.mkv'})['url']
    if not url.startswith('http://127.0.0.1:'):
        fail('the media link is not local: %s' % url)
    out = subprocess.run(['ip', 'netns', 'exec', b.ns, 'curl', '-sS', '-r', '1048000-3100000', '-D', '-', url], capture_output=True, timeout=60)
    head, _, body = out.stdout.partition(b'\r\n\r\n')
    want = open(os.path.join(vids, 'Films', 'clip.mkv'), 'rb').read()[1048000:3100001]
    if b' 206 ' not in head.split(b'\r\n')[0] or body != want:
        fail('the media link answered %r with %d bytes (wanted 206 and %d bytes)' % (head[:200], len(body), len(want)))
    bad = subprocess.run(['ip', 'netns', 'exec', b.ns, 'curl', '-s', '-o', '/dev/null', '-w', '%{http_code}', url.rsplit('/', 2)[0] + '/0000/x'], capture_output=True, text=True, timeout=30)
    if bad.stdout.strip() != '404':
        fail('a made-up media link answered %s' % bad.stdout)
    notice('storage: shared folder', 'listed without hidden files, a file copied over (hash checked), a range played through the 127.0.0.1 link (206, bytes checked), a made-up link refused')

    # The game archive: moved from B to A, then back.
    gb2 = os.path.join(b.steam, 'steamapps/common/Resume Game')
    r = b.call('storage.archive', {'server': a.id, 'game': 'steam:1000002'})
    key = r['key']
    arch = os.path.join(a.home, 'Hearth Storage', 'Archive', key)
    wait_for('the archive on A', lambda: os.path.exists(os.path.join(arch, 'game.json')), 900)
    same_tree(gb2, os.path.join(arch, 'root0'), 'the archived game')
    wait_for('B to hear the game is archived', lambda: (b.status()['storage']['config']['archived'].get('steam:1000002') or {}).get('state') == 'archived', 60)
    listed = b.call('storage.archived', {'server': a.id})['games']
    if not any(g['id'] == 'archive:' + key and g['store'] == 'steam' for g in listed):
        fail('the archive lists %s' % listed)
    ref = os.path.join(ROOT, 'resume-ref')
    shutil.rmtree(ref, ignore_errors=True)
    shutil.copytree(gb2, ref, symlinks=True)
    shutil.rmtree(gb2)
    os.remove(os.path.join(b.steam, 'steamapps/appmanifest_1000002.acf'))
    j = b.call('storage.restore', {'server': a.id, 'game': 'archive:' + key})
    j = wait_job(b, j['id'])
    same_tree(ref, gb2, 'the game copied back from the archive')
    acf = open(os.path.join(b.steam, 'steamapps/appmanifest_1000002.acf')).read()
    if '"StateFlags"\t\t"4"' not in acf:
        fail('the appmanifest after copying back: %s' % acf)
    notice('storage: archive', 'Resume Game moved from B to A (every file checked), B told it was safe to uninstall, deleted on B, copied back from the archive as a Steam game (appmanifest in place, every file checked)')

    # Backups of B's saves and screenshots on A.
    with open(os.path.join(b.home, '.config/user-dirs.dirs'), 'w') as f:
        f.write('XDG_PICTURES_DIR="$HOME/Pictures"\n')
    shots = os.path.join(b.home, 'Pictures/Screenshots/Hearth')
    os.makedirs(shots, exist_ok=True)
    write_random(os.path.join(shots, 'shot1.png'), 300000)
    tmp = os.path.join(b.steam, 'steamapps/compatdata/1000001/pfx/drive_c/users/steamuser/Temp')
    os.makedirs(tmp, exist_ok=True)
    write_random(os.path.join(tmp, 'junk.bin'), 100000)
    b.call('storage.set', {'backupTo': a.id})
    b.call('storage.backup.now')
    base = os.path.join(a.home, 'Hearth Storage', 'Backups')
    def done(what):
        hits = glob.glob(os.path.join(base, 'B (*)', what, 'backup.json'))
        return hits[0] if hits else None
    wait_for('the backups on A', lambda: done('Saves') and done('Screenshots'), 300)
    dev = os.path.dirname(os.path.dirname(done('Saves')))
    save = os.path.join(dev, 'Saves', 'Steam 1000001', 'Saved Games/Game/save1.sav')
    if not os.path.exists(save) or not os.path.exists(os.path.join(dev, 'Screenshots', 'Hearth', 'shot1.png')):
        fail('the backup on A has %s' % [os.path.relpath(os.path.join(d, f), dev) for d, _, fs in os.walk(dev) for f in fs][:30])
    if os.path.exists(os.path.join(dev, 'Saves', 'Steam 1000001', 'Temp')):
        fail("a prefix's Temp folder was backed up")
    if b.status()['storage']['config']['lastBackup'] <= 0:
        fail('B does not say when it last backed up')
    notice('storage: backups', "B's saves (every prefix's user folder, Temp left out) and screenshots are on A")

    # Off: A answers nothing of Hearth Storage anymore.
    a.call('storage.set', {'server': False})
    r = b.call('storage.ls', {'server': a.id, 'share': share, 'path': ''}, ok=False)
    if 'off' not in (r.get('error') or ''):
        fail('A still answered with Hearth Storage off: %s' % r)
    notice('storage: off', 'turned off on A: B is told so')


FAKE_UPDATE = r"""
import json, sys, time
# Settings > Updates > Update now's output (hearth-admin update), without updating anything.
for p in (5, 40, 90):
    print('progress: %.1f Downloading the system update' % p, flush=True)
    time.sleep(0.3)
print('stage: ' + json.dumps({'phase': 'done', 'pct': 100.0, 'desc': 'Done. Restart to finish updating.', 'staged': True}), flush=True)
print(json.dumps({'booted': 'x'}), flush=True)
"""

FAKE_OLLAMA = r"""
import json, sys
from http.server import BaseHTTPRequestHandler, HTTPServer
class H(BaseHTTPRequestHandler):
    protocol_version = 'HTTP/1.1'
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path == '/api/tags':
            b = json.dumps({'models': [{'name': 'tiny:1b'}]}).encode()
            self.send_response(200); self.send_header('Content-Type', 'application/json'); self.send_header('Content-Length', str(len(b))); self.end_headers(); self.wfile.write(b)
        else:
            self.send_response(404); self.send_header('Content-Length', '0'); self.end_headers()
    def do_POST(self):
        n = int(self.headers.get('Content-Length') or 0)
        req = json.loads(self.rfile.read(n) or b'{}')
        if self.path in ('/api/pull', '/api/delete'):
            # Never reached: Hearth Link doesn't pass model management on.
            open(sys.argv[2], 'w').write(self.path)
        # Streamed like Ollama: NDJSON lines, chunked.
        self.send_response(200); self.send_header('Content-Type', 'application/x-ndjson'); self.send_header('Transfer-Encoding', 'chunked'); self.end_headers()
        for w in ('echo: ', req.get('prompt', '')):
            line = (json.dumps({'response': w, 'done': False}) + '\n').encode()
            self.wfile.write(b'%x\r\n' % len(line) + line + b'\r\n')
        self.wfile.write(b'0\r\n\r\n')
HTTPServer(('127.0.0.1', int(sys.argv[1])), H).serve_forever()
"""


def extras(a, b):
    """The Hearth Link extras: Continue on this device (handoff.rs), update every device at once (fleet.rs), local
    AI over Hearth Link (ai.rs)."""
    import threading
    # ---- Continue on this device: Resume Game is running on A; B offers it, asks, waits until it's quit on A.
    game = 'steam:1000002'
    playing = os.path.join(a.run, 'hearth-playing')
    open(playing, 'w').close()
    write_json(os.path.join(a.link, 'now-playing.json'), {'game': game, 'title': 'Resume Game', 'at': int(time.time() * 1000)})
    st = b.call('handoff.status')
    offer = [o for o in st['offers'] if o['peer'] == a.id]
    if not offer or offer[0]['game'] != game or offer[0]['playing'] is not True:
        fail('B does not offer the game running on A: %s' % st)
    got = {}
    t = threading.Thread(target=lambda: got.update(r=b.call('handoff.take', {'peer': a.id, 'game': game, 'wait': 90}, timeout=120, ok=False)))
    t.start()
    wait_for('A to be asked to hand the game over', lambda: any(x['peer'] == b.id and x['game'] == game for x in a.call('handoff.status')['asked']), 30)
    time.sleep(4)
    if 'r' in got:
        fail('B took the game over while it was still running on A: %s' % got)
    # Quit on A (Hearth writes quitAt and removes its "playing" file).
    v = read_json(os.path.join(a.link, 'now-playing.json'))
    v['quitAt'] = int(time.time() * 1000)
    write_json(os.path.join(a.link, 'now-playing.json'), v)
    os.remove(playing)
    t.join(100)
    r = got.get('r') or {}
    if r.get('state') != 'ready' or r.get('name') != 'A':
        fail('the handoff did not finish once the game was quit on A: %s' % r)
    if a.call('handoff.status')['asked']:
        fail('A still shows the ask after the game was handed over')
    notice('handoff', 'Resume Game running on A offered on B; B waited while it ran (A asked), and took it over once quit on A')

    # ---- Update every device at once: B waits while someone plays, then runs Update now (a stand-in) and stages it.
    with open(os.path.join(ROOT, 'fake-update.py'), 'w') as f:
        f.write(FAKE_UPDATE)
    bplay = os.path.join(b.run, 'hearth-playing')
    open(bplay, 'w').close()
    r = a.call('fleet.update', {'here': False})
    rb = [d for d in r['devices'] if d.get('id') == b.id]
    if not rb or rb[0].get('state') not in ('starting', 'waitingGame'):
        fail('B did not take the update: %s' % r)
    wait_for('B to wait while a game runs on it', lambda: any(d.get('id') == b.id and d.get('state') == 'waitingGame' for d in a.call('fleet.status')['devices']), 30)
    time.sleep(3)
    if any(d.get('id') == b.id and d.get('state') != 'waitingGame' for d in a.call('fleet.status')['devices']):
        fail('B started updating while a game ran on it')
    os.remove(bplay)
    done = wait_for('B to stage the update', lambda: [d for d in a.call('fleet.status')['devices'] if d.get('id') == b.id and d.get('state') == 'staged'], 60)
    me = [d for d in a.call('fleet.status')['devices'] if d.get('me')]
    if not me or me[0].get('state') != 'idle':
        fail('A updated itself although it was left out: %s' % me)
    notice('update every device', 'asked from A: B waited while a game ran, then ran Update now and staged it (%s); A, left out, stayed as it was; nothing restarted' % done[0].get('desc'))

    # ---- Local AI: A shares its model server (a fake Ollama on A only); B uses it at 127.0.0.1 on B.
    srv_py = os.path.join(ROOT, 'fake-ollama.py')
    with open(srv_py, 'w') as f:
        f.write(FAKE_OLLAMA)
    tripped = os.path.join(ROOT, 'ollama-managed')
    fake = subprocess.Popen(['ip', 'netns', 'exec', 'hlA', sys.executable, srv_py, '18434', tripped])
    try:
        if any(v['id'] == a.id for v in b.call('ai.status')['servers']):
            fail('A shares its AI before it was turned on')
        r = a.call('ai.set', {'backend': 'http://192.168.1.9:18434'}, ok=False)
        if 'error' not in r:
            fail('A took an AI server address that is not on itself: %s' % r)
        a.call('ai.set', {'serve': True, 'backend': 'http://127.0.0.1:18434'})
        sv = wait_for('B to see A sharing its AI', lambda: [v for v in b.call('ai.status')['servers'] if v['id'] == a.id and v.get('models')], 30)
        if sv[0]['models'] != ['tiny:1b']:
            fail('the shared models are wrong: %s' % sv)
        url = b.call('ai.use', {'server': a.id})['url']
        client = ('import json,sys,urllib.request\n'
                  'def go(p,body):\n'
                  '  try:\n'
                  '    r=urllib.request.urlopen(urllib.request.Request(sys.argv[1]+p,data=json.dumps(body).encode(),headers={"Content-Type":"application/json"}),timeout=60); return r.status, r.read().decode()\n'
                  '  except urllib.error.HTTPError as e: return e.code, e.read().decode()\n'
                  'print(json.dumps([go("/api/generate",{"model":"tiny:1b","prompt":"hello hearth"}), go("/api/pull",{"name":"x"})]))\n')
        out = subprocess.run(['ip', 'netns', 'exec', 'hlB', sys.executable, '-c', client, url], capture_output=True, text=True, timeout=90)
        try:
            (gen_st, gen), (pull_st, _) = json.loads(out.stdout.strip().splitlines()[-1])
        except Exception:
            fail('the AI request on B did not answer: %r %r' % (out.stdout, out.stderr))
        words = ''.join(json.loads(l)['response'] for l in gen.splitlines() if l.strip())
        if gen_st != 200 or words != 'echo: hello hearth':
            fail('the answer through Hearth Link is wrong: %s %r' % (gen_st, gen))
        if pull_st != 403 or os.path.exists(tripped):
            fail('managing models went through Hearth Link: %s' % pull_st)
        a.call('ai.set', {'serve': False})
        out = subprocess.run(['ip', 'netns', 'exec', 'hlB', sys.executable, '-c', client, url], capture_output=True, text=True, timeout=90)
        (gen_st, gen), _ = json.loads(out.stdout.strip().splitlines()[-1])
        if gen_st == 200:
            fail('A still answered after it stopped sharing its AI: %r' % gen)
        b.call('ai.stop')
        notice('local AI', "A's model server (Ollama-style, streamed) used from B at %s over Hearth Link; model management refused; off by default and refused once turned off" % url)
    finally:
        fake.terminate()


def lan():
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT)
    lan_network()
    pk, pk_url = pkarr_relay('hlA', '10.77.0.1')
    env = {'HEARTH_LINK_RELAY': 'off', 'HEARTH_LINK_NO_DNS': '1', 'HEARTH_LINK_PKARR': pk_url}
    a = Dev('A', 'hlA', dict(env, HEARTH_LINK_KIND='desktop', HEARTH_LINK_EDITION='Anatase'))
    # B's "Update now" is a stand-in that only prints what hearth-admin update would (extras()).
    b = Dev('B', 'hlB', dict(env, HEARTH_LINK_KIND='handheld', HEARTH_LINK_EDITION='Peppermint',
                             HEARTH_LINK_UPDATE_CMD='%s %s' % (sys.executable, os.path.join(ROOT, 'fake-update.py'))))
    sa = steam_library(a.home)
    steam_library(b.home)
    log('making the synthetic games')
    g1 = make_game(sa, 1000001, 'Big Game', int(1200 * MB * SCALE), int(900 * MB * SCALE), int(4000 * SCALE), 12)
    g2 = make_game(sa, 1000002, 'Resume Game', int(1500 * MB * SCALE), 0, 500, 4)
    size1, size2 = tree_size(g1), tree_size(g2)
    log('games: %.2f GB and %.2f GB' % (size1 / 1e9, size2 / 1e9))
    a.start()
    b.start()
    wait_for('the devices to hear each other', lambda: any(n['id'] == b.id for n in a.status()['nearby']), 40)
    lockout(a, b)
    expiry(a, b)
    short_codes(a, b)
    # The long link (Other ways), with the 6-digit code.
    pair(a, b, ticket=b.status()['me']['ticket'])
    unpair(a, b)
    pair(a, b)

    # 1. A multi-GB game over the LAN path.
    places = b.call('places', {'store': 'steam'})['places']
    if not places:
        fail('B has no Steam library to copy into')
    j, listed, t0 = copy(b, a, 'steam:1000001', 'Big Game')
    j = wait_job(b, j['id'])
    mbs = report('LAN copy', j, t0, size1)
    if j.get('relay'):
        fail('the LAN copy went through a relay')
    gb = os.path.join(b.steam, 'steamapps/common/Big Game')
    n = same_tree(g1, gb, 'the copied game')
    same_tree(os.path.join(sa, 'steamapps/compatdata/1000001'), os.path.join(b.steam, 'steamapps/compatdata/1000001'), 'its Proton prefix')
    acf = open(os.path.join(b.steam, 'steamapps/appmanifest_1000001.acf')).read()
    if '"StateFlags"\t\t"4"' not in acf or '"installdir"\t\t"Big Game"' not in acf:
        fail('the appmanifest: %s' % acf)
    notice('verified', '%d entries (files, folders, links) and the Proton prefix match, hash for hash; Steam appmanifest in place (%s)' % (n, j.get('note')))

    # 2. Killed halfway, resumed.
    j, _, t0 = copy(b, a, 'steam:1000002', 'Resume Game')
    jid = j['id']
    # Killed as soon as the journal (written every second) lists a few hundred chunks, well before the end.
    jpath = os.path.join(b.link, 'transfers', jid, 'journal')
    made = 40 * 700   # the "made this file" marks come first
    j = wait_job(b, jid, until=lambda x: x['state'] == 'copying' and x['done'] > 0 and os.path.exists(jpath) and os.path.getsize(jpath) > made + 40 * 150)
    before = j['done']
    if j['done'] >= j['total']:
        fail('the copy finished before it could be interrupted')
    log('killing B at %d of %d MB' % (before // MB, j['total'] // MB))
    b.kill()
    time.sleep(1)
    b.start()
    j = wait_job(b, jid)
    # The journal only lists chunks known to be on the drive (flushed first), so the last moments before a crash
    # are sent again; the rest isn't.
    if j['skipped'] < before * 0.6 or j['total'] >= size2 * 0.95:
        fail('resume started over: %d MB already there, %d MB done before the kill' % (j['skipped'] // MB, before // MB))
    same_tree(g2, os.path.join(b.steam, 'steamapps/common/Resume Game'), 'the resumed game')
    notice('resume', 'killed at %d MB of %d MB (kill -9); after the restart it picked up by itself: %d MB were already there, %d MB more sent; every file verified'
           % (before // MB, size2 // MB, j['skipped'] // MB, j['done'] // MB))

    # 3. An update: only the changed chunks.
    with open(os.path.join(g1, 'huge.pak'), 'r+b') as f:
        f.seek(int(500 * MB * SCALE))
        f.write(os.urandom(3 * MB))
    with open(os.path.join(g1, 'levels.dat'), 'ab') as f:
        f.write(os.urandom(2 * MB))
    write_random(os.path.join(g1, 'patch-1.pak'), 7 * MB)
    j, _, t0 = copy(b, a, 'steam:1000001', 'Big Game')
    j = wait_job(b, j['id'])
    same_tree(g1, gb, 'the updated game')
    if j['total'] > 40 * MB:
        fail('the update sent %d MB (only ~13 MB changed)' % (j['total'] // MB))
    notice('update', 'game updated on A (3 MB changed inside a %d MB file, 2 MB appended, a 7 MB file added): %.1f MB sent of %.2f GB, the rest was already there (chunk hashes compared)'
           % (1200 * SCALE, j['total'] / MB, tree_size(g1) / 1e9))

    # 4. Hearth data both ways.
    write_json(os.path.join(a.data, 'library.json'), {'favorites': ['steam:1000001'], 'profile': {'name': 'Ana', 'accent': '#ff8a3d'},
                                                    'collections': [{'id': 'c1', 'name': 'Strategy', 'games': ['steam:1000001']}],
                                                    'stats': {'steam:1000001': {'minutes': 100, 'lastPlayed': 1000}}})
    write_json(os.path.join(b.data, 'library.json'), {'favorites': ['gog:42'], 'stats': {'steam:1000001': {'minutes': 50, 'lastPlayed': 2000}}})
    write_json(os.path.join(a.link, 'ui.json'), {'local': {'nimbus.look': {'layout': 'x-focus', 'madeFor': 'mk'}}})
    write_json(os.path.join(a.home, '.config/nimbus/input/steam%3A1000001.json'), {'preset': 'fps', 'sensitivity': 1.5})
    r = a.call('sync.now')
    if 'error' in (r.get(b.id) or {}):
        fail('sync: %s' % r)
    ib = read_json(os.path.join(b.link, 'inbox.json')) or {}
    ia = read_json(os.path.join(a.link, 'inbox.json')) or {}
    pb, pa = ib.get('profiles', {}).get('main', {}), ia.get('profiles', {}).get('main', {})
    checks = [
        (pb.get('set', {}).get('fav/steam:1000001') is True, 'favorite A→B'),
        ((pb.get('set', {}).get('profile') or {}).get('name') == 'Ana', 'profile A→B'),
        ((pb.get('set', {}).get('col/c1') or {}).get('name') == 'Strategy', 'collection A→B'),
        (pb.get('add', {}).get('steam:1000001') == 100, 'play time A→B (A\'s 100 minutes added)'),
        (pa.get('set', {}).get('fav/gog:42') is True, 'favorite B→A'),
        (pa.get('add', {}).get('steam:1000001') == 50, 'play time B→A (B\'s 50 minutes added)'),
        ((read_json(os.path.join(b.link, 'ui.json')) or {}).get('local', {}).get('nimbus.look', {}).get('layout') == 'x-focus', 'appearance A→B'),
        ((read_json(os.path.join(b.home, '.config/nimbus/input/steam%3A1000001.json')) or {}).get('preset') == 'fps', 'controller layout A→B'),
    ]
    bad = [w for ok, w in checks if not ok]
    if bad:
        fail('sync: %s; inbox on B %s, on A %s' % (bad, ib, ia))
    hearth_applies(a)
    hearth_applies(b)
    la, lb = read_json(os.path.join(a.data, 'library.json')), read_json(os.path.join(b.data, 'library.json'))
    if la['stats']['steam:1000001']['minutes'] != 150 or lb['stats']['steam:1000001']['minutes'] != 150:
        fail('play time after sync: A %s, B %s' % (la['stats'], lb['stats']))
    # Played 20 more minutes on B; removed the favorite on A. Two rounds: nothing counted twice.
    lb['stats']['steam:1000001']['minutes'] = 170
    write_json(os.path.join(b.data, 'library.json'), lb)
    la['favorites'] = [x for x in la['favorites'] if x != 'steam:1000001']
    write_json(os.path.join(a.data, 'library.json'), la)
    for _ in range(2):
        b.call('sync.now')
        hearth_applies(a)
        hearth_applies(b)
    la, lb = read_json(os.path.join(a.data, 'library.json')), read_json(os.path.join(b.data, 'library.json'))
    if la['stats']['steam:1000001']['minutes'] != 170 or lb['stats']['steam:1000001']['minutes'] != 170:
        fail('play time after more play: A %s, B %s' % (la['stats'], lb['stats']))
    if 'steam:1000001' in lb.get('favorites', []):
        fail('a removed favorite came back: %s' % lb.get('favorites'))
    notice('sync', 'favorites, collection, profile, appearance and controller layout went both ways; play time added up per device (100 + 50 = 150, then 170 after 20 more minutes on B, never counted twice); a removed favorite stayed removed')

    # Quiet while a game runs.
    open(os.path.join(b.run, 'hearth-playing'), 'w').close()
    r = a.call('sync.now')
    if (r.get(b.id) or {}).get('error') != 'playing':
        fail('sync went ahead while a game ran on B: %s' % r)
    os.remove(os.path.join(b.run, 'hearth-playing'))
    notice('quiet', 'while a game runs on a device, sync with it waits')

    # Sign-ins are never part of it.
    for root, _, fs in os.walk(b.link):
        for f in fs:
            if 'token' in f or 'loginusers' in f:
                fail('a sign-in file came over: %s' % f)

    # Hearth Storage: A as a home server for B.
    storage(a, b)
    extras(a, b)

    # 5. Unpairing on one side reaches the other.
    a.call('unpair', {'id': b.id})
    wait_for('B to forget A', lambda: not any(p['id'] == a.id for p in b.status()['peers']), 20)
    notice('unpair', 'unpaired on A; B forgot A too')
    a.kill(signal.SIGTERM)
    b.kill(signal.SIGTERM)
    pk.terminate()
    for name in ('A', 'B'):
        t = tail(os.path.join(ROOT, '%s.log' % name), 200000)
        steps = [l for l in t.splitlines() if 'pairing:' in l]
        # Every step is logged, never a code.
        if name == 'B' and (not any('wrong code from' in l for l in steps) or not any('paired with' in l for l in steps)):
            fail('pairing steps missing from %s.log: %s' % (name, steps[-10:]))
        if re.search(r'\b\d{6}\b', '\n'.join(steps)) or re.search(r'[%s]{3}-[%s]{3}-[%s]{4}' % ((ALPHABET,) * 3), '\n'.join(steps)):
            fail('a code in %s.log: %s' % (name, steps))
        if name == 'B':
            notice('pairing log on B', ' | '.join(l.split('pairing: ', 1)[1] for l in steps[-12:])[:1800])
        broke = [l for l in t.splitlines() if 'broke' in l or 'ended:' in l]
        if broke:
            notice('connection events on %s' % name, ' | '.join(broke[-6:])[:1800])
    notice('LAN summary', 'LAN copy %.0f MB/s on this 4-core CI runner (veth between two namespaces: CPU and disk bound, not the wire)' % mbs)


def internet():
    relay_bin = os.environ.get('RELAY_BIN')
    if not relay_bin or not os.path.exists(relay_bin):
        notice('internet path', 'skipped: no iroh-relay')
        return
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT)
    internet_network()
    rl = open(os.path.join(ROOT, 'relay.log'), 'w')
    relay = subprocess.Popen(['ip', 'netns', 'exec', 'inet', relay_bin, '--dev'], stdout=rl, stderr=subprocess.STDOUT)
    time.sleep(2)
    url = 'http://198.18.1.1:3340'
    pk, pk_url = pkarr_relay('inet', '198.18.1.1')
    fakes = {}
    for name, text in (('remote', FAKE_REMOTE),):
        fakes[name] = os.path.join(ROOT, 'fake-%s.py' % name)
        with open(fakes[name], 'w') as f:
            f.write(text)
    env = {'HEARTH_LINK_RELAY': url, 'HEARTH_LINK_NO_DNS': '1', 'HEARTH_LINK_PKARR': pk_url, 'HEARTH_REMOTE_DESKTOP': fakes['remote'],
           'FAKE_HOST_PIN': os.path.join(ROOT, 'B', 'home', 'pin-got.json')}
    env.update(moon_env())
    a, b = Dev('A', 'hiA', env), Dev('B', 'hiB', env)
    sa = steam_library(a.home)
    steam_library(b.home)
    g = make_game(sa, 1000003, 'Net Game', int(400 * MB * SCALE), int(200 * MB * SCALE), 300, 2)
    size = tree_size(g)
    # Relay only: no UDP between the two NATs.
    sh('ip netns exec inet iptables -A FORWARD -p udp -j DROP')
    a.start()
    b.start()
    wait_for('both on the relay', lambda: a.status()['me'].get('relay') and b.status()['me'].get('relay'), 30)
    ticket = b.status()['me']['ticket']
    if '@' not in ticket:
        fail('no relay in the ticket: %s' % ticket)
    # Only the short code: B's id and relay come from the record (the NATs keep A from hearing B on a network).
    net = b.call('pair.open', {'internet': True}, timeout=60).get('netCode') or ''
    if not CODE_RE.match(net):
        fail('the short code: %r' % net)
    t0 = time.time()
    r = a.call('pair.join', {'code': net}, timeout=90)
    if r.get('id') != b.id:
        fail('pairing with the short code over the internet: %s' % r)
    wait_for('B to list A', lambda: any(p['id'] == a.id for p in b.status()['peers']), 20)
    notice('internet pairing', 'paired across two NATs from the short code alone in %.2f s (record on a pkarr relay, connection through the iroh relay)' % (time.time() - t0))
    j, _, t0 = copy(b, a, 'steam:1000003', 'Net Game')
    j = wait_job(b, j['id'])
    if not j.get('relay'):
        notice('relay', 'the copy says it was direct although UDP between the NATs was blocked: %s' % j)
    rmbs = report('relay copy', j, t0, size)
    same_tree(g, os.path.join(b.steam, 'steamapps/common/Net Game'), 'the game copied through the relay')
    # Moonlight through the relay: B lets A connect (B's short code typed on A, A already paired keeps being one of
    # B's devices), Always allow, then A's Moonlight's speed through it.
    sun = fake_sunshine('hiB', 'B')
    b.call('moonlight.share.set', {'on': True})
    net = b.call('moonlight.share.invite', timeout=60)['invite'].get('netCode') or ''
    if not CODE_RE.match(net):
        fail('a short code to add A: %r' % net)
    r = a.call('moonlight.join', {'code': net}, timeout=90)
    if r.get('relation') != 'mine':
        fail('A added to B\'s family and friends with a short code (already one of B\'s devices): %s' % r)
    wait_for('B to list A', lambda: any(p['id'] == a.id for p in b.call('moonlight.share.status')['people']), 20)
    b.call('moonlight.share.person', {'id': a.id, 'always': True})
    wait_for('A to list B', lambda: any(f['id'] == b.id for f in a.call('moonlight.friends', {'check': True}, timeout=30)['friends']), 30)
    res, _ = moon_session(a, b, 'B')
    relayed = a.call('moonlight.friends')['connected'] or {}
    if '--bitrate' not in res.get('args', []):
        notice('moonlight: relay', 'the relayed connection didn\'t lower Moonlight\'s bitrate: %s / %s' % (res.get('args'), relayed))
    notice('moonlight: through the relay', 'Moonlight\'s connections through the iroh relay (UDP between the NATs blocked): %.1f MB/s TCP, %d of %d video-sized packets back (%s)' % (res['tcpMBps'], res['udpBack'], res['udpSent'], 'said relayed, bitrate lowered' if relayed.get('relay') else 'not said relayed'))
    a.call('moonlight.disconnect')
    wait_for('A\'s Moonlight to stop', lambda: os.path.exists(os.path.join(a.home, 'ml-ended.json')), 20)
    # Direct: UDP allowed again; a fresh copy (hole punching through both NATs, if they allow it).
    sh('ip netns exec inet iptables -D FORWARD -p udp -j DROP')
    a.kill(signal.SIGTERM)
    b.kill(signal.SIGTERM)
    shutil.rmtree(os.path.join(b.steam, 'steamapps/common/Net Game'))
    os.remove(os.path.join(b.steam, 'steamapps/appmanifest_1000003.acf'))
    a.start()
    b.start()
    time.sleep(3)
    j, _, t0 = copy(b, a, 'steam:1000003', 'Net Game')
    j = wait_job(b, j['id'])
    dmbs = report('internet copy (UDP allowed)', j, t0, size)
    same_tree(g, os.path.join(b.steam, 'steamapps/common/Net Game'), 'the game copied over the internet path')
    notice('internet summary', 'through the relay: %.0f MB/s; with UDP open between the NATs: %.0f MB/s (%s)' % (rmbs, dmbs, 'direct, hole punched' if not j.get('relay') else 'still relayed'))
    res, _ = moon_session(a, b, 'B')
    notice('moonlight: hole punched', 'Moonlight\'s connections with UDP open between the NATs: %.1f MB/s TCP, %d of %d video-sized packets back (%s)' % (res['tcpMBps'], res['udpBack'], res['udpSent'], 'relayed' if (a.call('moonlight.friends')['connected'] or {}).get('relay') else 'direct'))
    a.call('moonlight.disconnect')
    sun.terminate()
    a.kill(signal.SIGTERM)
    b.kill(signal.SIGTERM)
    relay.terminate()
    pk.terminate()



def family_network():
    """Three devices on one network: a bridge in its own namespace (nothing else's firewall in the way)."""
    for ns in ('hfBr', 'hfA', 'hfB', 'hfC'):
        sh('ip netns del %s' % ns, check=False)
        sh('ip netns add %s' % ns)
        sh('ip -n %s link set lo up' % ns)
    sh('ip -n hfBr link add br0 type bridge && ip -n hfBr link set br0 up')
    for i, x in enumerate('ABC'):
        sh('ip link add f%s0 type veth peer name f%s1' % (x, x))
        sh('ip link set f%s0 netns hf%s' % (x, x))
        sh('ip link set f%s1 netns hfBr' % x)
        sh('ip -n hfBr link set f%s1 master br0 && ip -n hfBr link set f%s1 up' % (x, x))
        sh('ip -n hf%s addr add 10.78.0.%d/24 brd + dev f%s0 && ip -n hf%s link set f%s0 up' % (x, i + 1, x, x, x))


def pair_quick(joiner, host, mine_joiner=True, mine_host=True):
    code = host.call('pair.open', {'mine': mine_host})['code']
    wait_for('%s to list %s as ready to pair' % (joiner.name, host.name), lambda: any(n['id'] == host.id and n['pairing'] for n in joiner.status()['nearby']), 20)
    return joiner.call('pair.join', {'id': host.id, 'code': code, 'mine': mine_joiner}, timeout=90)


def peer_of(dev, other):
    return next((p for p in dev.status()['peers'] if p['id'] == other.id), None)


SEND_TO_TV = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', 'os', 'system_files', 'usr', 'bin', 'hearth-send-to-tv')


def send_to_tv(a, b, c):
    """Send to TV (tv.rs): A (a PC) sends a link and a video file to C (a Hearth TV) with hearth-send-to-tv, the way
    Hearth Browser, Hearth Files and Theater do; C's Hearth TV takes them (tv.inbox) and plays the file from its own
    127.0.0.1 address, read from A over Hearth Link, with seeking (ranges)."""
    def cli(*args):
        r = subprocess.run([sys.executable, SEND_TO_TV, '--json'] + list(args), env=a.env, capture_output=True, text=True, timeout=90)
        try:
            return json.loads(r.stdout.strip().splitlines()[-1])
        except Exception:
            fail('hearth-send-to-tv %s: %r %r' % (args, r.stdout, r.stderr))
    wait_for('A to know C is a TV', lambda: any(t['id'] == c.id for t in a.call('tv.list')['tvs']), 30)
    tvs = a.call('tv.list')['tvs']
    if [t['id'] for t in tvs] != [c.id]:
        fail('A\'s TVs: %s (B is a handheld, not a TV)' % tvs)
    lst = json.loads(subprocess.run([sys.executable, SEND_TO_TV, '--list'], env=a.env, capture_output=True, text=True, timeout=30).stdout)
    if [t['name'] for t in lst['tvs']] != ['C']:
        fail('hearth-send-to-tv --list: %s' % lst)
    # A link (right-click a link › Send to TV).
    r = cli('--title', 'Big Buck Bunny', 'https://www.youtube.com/watch?v=aqz-KE-bpKQ')
    if r.get('sent') is not True or r.get('tv') != 'C':
        fail('sending a link: %s' % r)
    got = c.call('tv.inbox')['items']
    if len(got) != 1 or got[0]['kind'] != 'link' or got[0]['url'] != 'https://www.youtube.com/watch?v=aqz-KE-bpKQ' or got[0]['from'] != 'A' or got[0]['title'] != 'Big Buck Bunny':
        fail('the link on the TV: %s' % got)
    if c.call('tv.inbox')['items']:
        fail('the TV got the link twice')
    # A video file (Hearth Files › Send to TV): it stays on A and plays from C's own address.
    vids = os.path.join(a.home, 'Videos')
    os.makedirs(vids, exist_ok=True)
    film = os.path.join(vids, 'Holiday 2026.mp4')
    data = os.urandom(int(11.5 * MB))
    with open(film, 'wb') as f:
        f.write(data)
    r = cli(film)
    if r.get('sent') is not True:
        fail('sending a file: %s' % r)
    got = c.call('tv.inbox')['items']
    if len(got) != 1 or got[0]['kind'] != 'file' or got[0]['file']['size'] != len(data) or not got[0]['url'].startswith('http://127.0.0.1:'):
        fail('the file on the TV: %s' % got)
    url = got[0]['url']
    out = os.path.join(ROOT, 'tv-got.bin')
    t0 = time.time()
    sh("ip netns exec %s curl -sS -f -o '%s' '%s'" % (c.ns, out, url))
    took = time.time() - t0
    if open(out, 'rb').read() != data:
        fail('the file played on the TV differs from the one on the PC')
    sh("ip netns exec %s curl -sS -f -r 5000000-7000123 -o '%s' '%s'" % (c.ns, out, url))
    if open(out, 'rb').read() != data[5000000:7000124]:
        fail('a seek (range) on the TV got the wrong bytes')
    head = sh("ip netns exec %s curl -sS -I -r 100-199 '%s'" % (c.ns, url))
    if '206' not in head.split('\n')[0] or 'video/mp4' not in head or 'bytes 100-199/%d' % len(data) not in head:
        fail('the player\'s headers: %s' % head)
    # Only from the TV itself: a made-up address is refused.
    bad = sh("ip netns exec %s curl -s -o /dev/null -w '%%{http_code}' 'http://127.0.0.1:%s/%s/x.mp4'" % (c.ns, url.split(':')[2].split('/')[0], '0' * 32), check=False)
    if bad.strip() != '404':
        fail('a made-up address on the TV answered %s' % bad)
    # What doesn't go: a handheld isn't a TV, a text file isn't a video, a file that isn't there.
    r = a.call('tv.send', {'peer': b.id, 'url': 'https://vimeo.com/1084537'}, ok=False)
    if "isn't a Hearth TV" not in json.dumps(r):
        fail('Send to TV to a handheld: %s' % r)
    notes = os.path.join(a.home, 'notes.txt')
    open(notes, 'w').write('x')
    if 'Only videos and music' not in json.dumps(cli(notes)):
        fail('a text file was sent to the TV')
    if "isn't a link or a file" not in json.dumps(cli('/nowhere/film.mp4')):
        fail('a missing file was sent to the TV')
    # A show of the library (Theater's Jellyfin): its id goes, and the TV opens it in its own My Media.
    r = cli('--media', '4f1d2c3b4a5968778695a4b3c2d1e0f0', '--type', 'show', '--title', 'Severance')
    got = c.call('tv.inbox')['items']
    if r.get('sent') is not True or len(got) != 1 or got[0]['media'] != {'id': '4f1d2c3b4a5968778695a4b3c2d1e0f0', 'type': 'show'}:
        fail('sending a show: %s / %s' % (r, got))
    notice('Send to TV', 'A (a PC) sent a link, an %.1f MB video and a show to C (a Hearth TV) with hearth-send-to-tv; the TV took each once; '
           'the video played from the TV\'s own address in %.1f s, the same bytes, with seeking (206, ranges) and nothing for made-up addresses; '
           'a handheld, a text file and a missing file were refused' % (len(data) / 1e6, took))


FAKE_STATUS = """import json, sys
print(json.dumps({'protocol': 1, 'shared': True, 'mode': 'desktop', 'cpu': {'use': 7}}))
"""
FAKE_REMOTE = """import json, os, sys
a = sys.argv[1:]
if a == ['status']:
    print(json.dumps({'installed': True, 'on': True, 'running': True, 'address': {'name': 'dads-pc', 'dns': 'dads-pc.tail.ts.net', 'ip': '100.64.0.7'},
                      'tailscale': {'running': True}, 'clients': [{'name': 'secret'}], 'stream': {'connected': False}}))
elif a and a[0] == 'pin':
    open(os.path.join(os.environ['HOME'], 'pin-got.json'), 'w').write(json.dumps(a))
    print(json.dumps({'ok': True}))
elif a and a[0] == 'family':
    f = os.path.join(os.environ['HOME'], 'family-calls.json')
    calls = json.load(open(f)) if os.path.exists(f) else []
    calls.append(a)
    open(f, 'w').write(json.dumps(calls))
    if a[1:2] == ['start']:
        print(json.dumps({'ok': True, 'port': 47989, 'running': True}))
    elif a[1:2] == ['paired']:
        print(json.dumps({'paired': os.path.exists(os.path.join(os.environ['HOME'], 'pin-got.json'))}))
    else:
        print(json.dumps({'ok': True}))
else:
    print('error: unknown')
    sys.exit(1)
"""


def family_member(a, b, c):
    """A family member's device (family.rs): C (Dad's PC) paired with A as A family member's device. B (A's other
    device) has it too without a code, and C hears B's name only from A. It shows in Your devices with its status (C's
    on by default, A's toward C off until A turns it on), A and B can use its remote desktop and send Moonlight's PIN;
    nothing personal goes either way until C's own switches say so: Use Hearth Storage and Share my game library each
    on and off (only that kind, only from C's side, the library never with settings). Removing it on A removes it
    everywhere."""
    a.call('unpair', {'id': c.id})
    wait_for('C to forget A', lambda: not peer_of(c, a), 30)
    code = a.call('pair.open', {'as': 'family'})['code']
    wait_for('C to list A as ready to pair', lambda: any(n['id'] == a.id and n['pairing'] for n in c.status()['nearby']), 20)
    r = c.call('pair.join', {'id': a.id, 'code': code, 'as': 'mine'}, timeout=90)
    if r.get('relation') != 'family' or r.get('familySide') != 'helper' or r.get('friend') is not False:
        fail('pairing C with A picking A family member\'s device: %s' % r)
    pa, pc = peer_of(a, c), peer_of(c, a)
    if (pa or {}).get('relation') != 'family' or pa.get('familySide') != 'member' or (pc or {}).get('familySide') != 'helper':
        fail('the family marks: %s / %s' % (pa, pc))
    if pc['shares'] != {'status': True, 'library': False, 'saves': False, 'storage': False, 'backup': False} or pa['shares']['status'] is not False:
        fail('the switches to start with: on C %s, on A %s' % (pc['shares'], pa['shares']))
    # Shared among your devices: B from A, and C hears B's name from A.
    wait_for('B to have C as a family member\'s device', lambda: (peer_of(b, c) or {}).get('relation') == 'family', 60)
    wait_for('C to know B (from A)', lambda: (peer_of(c, b) or {}).get('familySide') == 'helper', 60)
    pb, pcb = peer_of(b, c), peer_of(c, b)
    if pb.get('familySide') != 'member' or pb.get('introducedBy') != 'A' or pcb.get('introducedBy') != 'A':
        fail('C on B / B on C: %s / %s' % (pb, pcb))
    notice('family: shared', "C paired with A as a family member's device: B has it too (through A, no code) and C knows B only from A")
    # Your devices: C's status on A and B; A's not on C until A turns it on for C.
    def row(dev, other):
        return next((d for d in dev.call('status.devices')['devices'] if d['id'] == other.id), None)
    wait_for('C\'s status on B', lambda: (row(b, c) or {}).get('shared') is True, 40)
    if (row(a, c) or {}).get('shared') is not True or (row(a, c) or {}).get('relation') != 'family':
        fail('C on A\'s Your devices: %s' % row(a, c))
    ra = row(c, a) or {}
    if ra.get('shared') is not False or "doesn't share" not in (ra.get('error') or ''):
        fail('A\'s status on C before A turned it on: %s' % ra)
    a.call('peer.set', {'id': c.id, 'shares': {'status': True}})
    if (row(c, a) or {}).get('shared') is not True:
        fail('A\'s status on C after A turned it on for C: %s' % row(c, a))
    if (row(c, b) or {}).get('shared') is True:
        fail('B shares its status with C without B turning it on: %s' % row(c, b))
    notice('family: status', "C's status shows on A and B; A's showed on C only once A turned it on for C; B's still doesn't")
    # Remote desktop and its PIN, from A and B; never the other way.
    rd = a.call('peer.remote', {'id': c.id})
    if rd.get('host') != 'dads-pc' or rd.get('on') is not True or 'clients' in rd:
        fail('C\'s remote desktop on A: %s' % rd)
    if b.call('peer.remote', {'id': c.id}).get('host') != 'dads-pc':
        fail('C\'s remote desktop on B')
    a.call('peer.remote.pin', {'id': c.id, 'pin': '4821', 'name': 'Chris desktop'})
    got = read_json(os.path.join(c.home, 'pin-got.json'))
    if got != ['pin', '4821', 'Chris desktop']:
        fail('Moonlight\'s PIN on C: %s' % got)
    r = c.call('peer.remote', {'id': a.id}, ok=False)
    if 'error' not in r:
        fail('C used A\'s remote desktop: %s' % r)
    r = a.call('peer.wake', {'id': c.id}, ok=False)
    if 'only for your own' in json.dumps(r):
        fail('waking C: %s' % r)
    notice('family: help', 'A and B see how to reach C\'s desktop in Moonlight and sent it Moonlight\'s PIN; C can\'t do that to A')
    # Nothing personal until C says so.
    r = a.call('sync.now', {'peer': c.id}, ok=False)
    if 'Share my game library' not in json.dumps(r):
        fail('sync with C before its library switch: %s' % r)
    a.call('storage.set', {'server': True})
    vids = os.path.join(a.home, 'FamilyShare')
    os.makedirs(vids, exist_ok=True)
    write_random(os.path.join(vids, 'photo.jpg'), 4096)
    share = a.call('storage.share.add', {'path': vids})['share']['id']
    r = c.call('storage.ls', {'server': a.id, 'share': share, 'path': ''}, ok=False)
    if 'Use Hearth Storage' not in json.dumps(r):
        fail('C using A\'s Hearth Storage before its switch: %s' % r)
    r = a.call('peer.set', {'id': c.id, 'shares': {'storage': True}}, ok=False)
    if 'Only the family member' not in json.dumps(r):
        fail('A turning on C\'s switch: %s' % r)
    c.call('peer.set', {'id': a.id, 'shares': {'storage': True}})
    wait_for('A to hear C turned on Use Hearth Storage', lambda: (peer_of(a, c) or {}).get('theirShares', {}).get('storage') is True, 20)
    top = [e['name'] for e in c.call('storage.ls', {'server': a.id, 'share': share, 'path': ''})['entries']]
    if top != ['photo.jpg']:
        fail('C listing A\'s shared folder with its switch on: %s' % top)
    r = a.call('sync.now', {'peer': c.id}, ok=False)
    if 'Share my game library' not in json.dumps(r):
        fail('Use Hearth Storage let the library through: %s' % r)
    c.call('peer.set', {'id': a.id, 'shares': {'storage': False}})
    r = c.call('storage.ls', {'server': a.id, 'share': share, 'path': ''}, ok=False)
    if 'Use Hearth Storage' not in json.dumps(r):
        fail('C still using A\'s Hearth Storage after turning it off: %s' % r)
    notice('family: storage', "C couldn't use A's Hearth Storage until C turned on Use Hearth Storage (A can't turn it on); then only that; off again stopped it at once")
    # The library, never settings, both ways, only while C's switch is on.
    write_json(os.path.join(c.data, 'library.json'), {'favorites': ['steam:77'], 'profile': {'name': 'Dad'}})
    write_json(os.path.join(c.link, 'ui.json'), {'local': {'nimbus.look': {'layout': 'dad-layout'}}})
    write_json(os.path.join(a.data, 'library.json'), {'favorites': ['gog:5'], 'profile': {'name': 'Chris'}})
    write_json(os.path.join(a.link, 'ui.json'), {'local': {'nimbus.look': {'layout': 'chris-layout'}}})
    c.call('peer.set', {'id': a.id, 'shares': {'library': True}})
    wait_for('A to hear C turned on Share my game library', lambda: (peer_of(a, c) or {}).get('theirShares', {}).get('library') is True, 20)
    r = a.call('sync.now', {'peer': c.id})
    if 'error' in (r.get(c.id) or {}):
        fail('library sync with C: %s' % r)
    ia, ic = read_json(os.path.join(a.link, 'inbox.json')) or {}, read_json(os.path.join(c.link, 'inbox.json')) or {}
    sa, sc = ia.get('profiles', {}).get('main', {}).get('set', {}), ic.get('profiles', {}).get('main', {}).get('set', {})
    bad = []
    if sa.get('fav/steam:77') is not True: bad.append("C's favorite on A")
    if sc.get('fav/gog:5') is not True: bad.append("A's favorite on C")
    if 'profile' in sa or 'profile' in sc: bad.append('a profile went across')
    if ((read_json(os.path.join(c.link, 'ui.json')) or {}).get('local', {}).get('nimbus.look', {}).get('layout')) != 'dad-layout': bad.append("A's appearance reached C")
    if ((read_json(os.path.join(a.link, 'ui.json')) or {}).get('local', {}).get('nimbus.look', {}).get('layout')) != 'chris-layout': bad.append("C's appearance reached A")
    if bad:
        fail('library with C: %s; inbox on A %s, on C %s' % (bad, ia, ic))
    c.call('peer.set', {'id': a.id, 'shares': {'library': False}})
    r = a.call('sync.now', {'peer': c.id}, ok=False)
    if 'Share my game library' not in json.dumps(r):
        fail('sync after C turned its library off: %s' % r)
    notice('family: library', "with C's Share my game library on, favorites went both ways and no profile or appearance did; off again, nothing syncs")
    # Never: making it yours from one side.
    r = a.call('peer.set', {'id': c.id, 'relation': 'mine'}, ok=False)
    if 'both' not in json.dumps(r):
        fail('making C yours from A alone: %s' % r)
    # Removing it on A removes it everywhere.
    a.call('unpair', {'id': c.id})
    wait_for('C to forget A', lambda: not peer_of(c, a), 30)
    wait_for('B to drop C (removed on A)', lambda: not peer_of(b, c), 60)
    wait_for('C to forget B too', lambda: not peer_of(c, b), 30)
    notice('family: removing', 'removed C on A: B dropped it by itself and told C, which forgot both')


# ------------------------------------------------------------------ Family and friends with Moonlight (moonlight.rs)

FAKE_SUNSHINE = r"""import socket, sys, threading
tag = sys.argv[1].encode()
def serve(c):
    first = True
    while True:
        try:
            d = c.recv(65536)
        except OSError:
            break
        if not d:
            break
        if first:
            c.sendall(tag + b':')
            first = False
        c.sendall(d)
    c.close()
def tcp(port):
    s = socket.socket()
    s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    s.bind(('127.0.0.1', port))
    s.listen(16)
    while True:
        c, _ = s.accept()
        threading.Thread(target=serve, args=(c,), daemon=True).start()
def udp(port):
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    s.bind(('127.0.0.1', port))
    while True:
        d, a = s.recvfrom(65536)
        s.sendto(tag + b':' + d, a)
for p in (47984, 47989, 48010):
    threading.Thread(target=tcp, args=(p,), daemon=True).start()
for p in (47998, 47999, 48000):
    threading.Thread(target=udp, args=(p,), daemon=True).start()
threading.Event().wait()
"""

# Moonlight, as Hearth starts it in its private network: `pair HOST --pin PIN`, then `stream HOST APP [--bitrate N]`.
# Writes what it saw in its HOME (the connecting device's), then stays connected until the tunnel goes away.
FAKE_MOONLIGHT = r"""#!/usr/bin/env python3
import json, os, signal, socket, sys, threading, time
home = os.environ['HOME']
a = sys.argv[1:]
def out(name, v):
    with open(os.path.join(home, name + '.tmp'), 'w') as f:
        f.write(json.dumps(v))
    os.replace(os.path.join(home, name + '.tmp'), os.path.join(home, name))
if a[:1] == ['pair']:
    out('ml-pair.json', {'host': a[1], 'pin': a[3] if len(a) > 3 else '', 'env': 'HEARTH_MOON_PIN' in os.environ})
    f = os.environ.get('FAKE_HOST_PIN', '')
    for _ in range(300):
        if f and os.path.exists(f):
            break
        time.sleep(0.1)
    sys.exit(0)
if a[:1] != ['stream']:
    sys.exit(2)
host, app = a[1], a[2]
nets = [l.split(':')[0].strip() for l in open('/proc/net/dev').read().splitlines()[2:]]
r = {'host': host, 'app': app, 'args': a, 'nets': nets, 'pinInEnv': 'HEARTH_MOON_PIN' in os.environ}
def ended(*_):
    out('ml-ended.json', {'at': time.time()})
    os._exit(0)
signal.signal(signal.SIGTERM, ended)
def tcp(port, data, timeout=15):
    s = socket.create_connection((host, port), timeout=timeout)
    s.sendall(data)
    got = b''
    while len(got) < len(data) + 2:
        d = s.recv(65536)
        if not d:
            break
        got += d
    s.close()
    return got
try:
    r['tcp'] = {str(p): tcp(p, b'hello %d' % p).decode('latin1') for p in (47984, 47989, 48010)}
    u = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    u.settimeout(5)
    r['udp'] = {}
    for p, size in ((47998, 5), (47999, 5), (48000, 5), (47998, 1400), (47998, 3000)):
        payload = os.urandom(size)
        u.sendto(payload, (host, p))
        try:
            d, _ = u.recvfrom(65536)
            r['udp']['%d/%d' % (p, size)] = {'tag': d[:2].decode('latin1'), 'same': d[2:] == payload}
        except socket.timeout:
            r['udp']['%d/%d' % (p, size)] = None
    # Speed: 16 MB echoed through RTSP's port; 1500 video-sized packets, how many come back.
    big = os.urandom(16 << 20)
    s = socket.create_connection((host, 48010), timeout=30)
    t0 = time.time()
    def send():
        s.sendall(big)
    th = threading.Thread(target=send)
    th.start()
    got = 0
    while got < len(big) + 2:
        d = s.recv(1 << 20)
        if not d:
            break
        got += len(d)
    th.join()
    r['tcpMBps'] = round((len(big) / (1 << 20)) / max(time.time() - t0, 0.001), 1)
    r['tcpOk'] = got == len(big) + 2
    s.close()
    u.settimeout(0.2)
    # Paced like a stream: 1000 packets a second (about 10 Mbit/s each way), read back as they come.
    n, back = 1500, 0
    u.setblocking(False)
    t0 = time.time()
    for i in range(n):
        while time.time() < t0 + i / 1000.0:
            try:
                u.recvfrom(65536)
                back += 1
            except (BlockingIOError, OSError):
                time.sleep(0.0002)
        u.sendto(i.to_bytes(4, 'big') + os.urandom(1196), (host, 47998))
    u.setblocking(True)
    u.settimeout(0.2)
    end = time.time() + 3
    while time.time() < end and back < n:
        try:
            u.recvfrom(65536)
            back += 1
        except socket.timeout:
            pass
    r['udpBack'] = back
    r['udpSent'] = n
    try:
        socket.create_connection(('10.78.0.3', 47989), timeout=2).close()
        r['outside'] = 'reached'
    except OSError as e:
        r['outside'] = str(e)
except Exception as e:
    r['error'] = '%s: %s' % (type(e).__name__, e)
out('ml-result.json', r)
for _ in range(1200):
    try:
        tcp(47989, b'ping', 3)
    except Exception:
        ended()
    time.sleep(0.3)
ended()
"""

FAKE_ASK = r"""#!/usr/bin/env python3
import json, os, sys
home = os.environ['HOME']
log = os.path.join(home, 'asked.json')
l = json.load(open(log)) if os.path.exists(log) else []
l.append(sys.argv[1:])
open(log, 'w').write(json.dumps(l))
f = os.path.join(home, 'ask-answer')
print(open(f).read().strip() if os.path.exists(f) else 'deny')
"""

FAKE_NOTICE = r"""#!/usr/bin/env python3
import json, os, sys, time
open(os.path.join(os.environ['HOME'], 'notice.json'), 'w').write(json.dumps({'args': sys.argv[1:], 'pid': os.getpid()}))
time.sleep(900)
"""


def moon_env():
    """The stand-ins every device gets (written once per run)."""
    env = {}
    for name, text, var in (('moonlight', FAKE_MOONLIGHT, 'HEARTH_MOONLIGHT'), ('ask', FAKE_ASK, 'HEARTH_FAMILY_ASK'), ('notice', FAKE_NOTICE, 'HEARTH_FAMILY_NOTICE')):
        f = os.path.join(ROOT, 'fake-%s' % name)
        with open(f, 'w') as fh:
            fh.write(text)
        os.chmod(f, 0o755)
        env[var] = f
    env['HEARTH_LINK_MOON_MINUTE_MS'] = '600'
    return env


def fake_sunshine(ns, tag):
    f = os.path.join(ROOT, 'fake-sunshine.py')
    with open(f, 'w') as fh:
        fh.write(FAKE_SUNSHINE)
    return subprocess.Popen(['ip', 'netns', 'exec', ns, sys.executable, f, tag], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def rm(*paths):
    for p in paths:
        try:
            os.remove(p)
        except OSError:
            pass


def moon_session(a, host, tag, what='desktop'):
    """A connects to HOST; checks what its stand-in Moonlight saw. Returns that."""
    rm(os.path.join(a.home, 'ml-result.json'), os.path.join(a.home, 'ml-ended.json'))
    t0 = time.time()
    r = a.call('moonlight.connect', {'id': host.id}, timeout=150)
    if r.get('ok') is not True:
        fail('connecting: %s' % r)
    res = wait_for("A's Moonlight to finish its checks", lambda: read_json(os.path.join(a.home, 'ml-result.json')), 120)
    took = time.time() - t0
    bad = []
    if res.get('error'):
        bad.append(res['error'])
    if res.get('app') != ('Steam Big Picture' if what == 'games' else 'Desktop'):
        bad.append('app %s' % res.get('app'))
    if res.get('nets') != ['lo']:
        bad.append('Moonlight\'s network has %s (only lo expected)' % res.get('nets'))
    if res.get('pinInEnv'):
        bad.append('the PIN was still in Moonlight\'s environment while streaming')
    for p in ('47984', '47989', '48010'):
        if (res.get('tcp') or {}).get(p) != '%s:hello %s' % (tag, p):
            bad.append('TCP %s: %r' % (p, (res.get('tcp') or {}).get(p)))
    for k, v in (res.get('udp') or {}).items():
        if not v or v.get('tag') != tag + ':' or not v.get('same'):
            bad.append('UDP %s: %s' % (k, v))
    if not res.get('tcpOk'):
        bad.append('the 16 MB came back wrong')
    if 'reached' == res.get('outside'):
        bad.append('Moonlight reached the network outside its own')
    if bad:
        fail('Moonlight through Hearth Link: %s (%s)' % ('; '.join(bad), res))
    return res, took


def moon_family(a, b, c):
    """C is a friend's PC that lets A connect: off until C turns it on; A added with a code; asked every time."""
    sun = fake_sunshine('hfC', 'C')
    rm(os.path.join(c.home, 'pin-got.json'), os.path.join(c.home, 'family-calls.json'))
    st = c.call('moonlight.share.status')
    if st.get('on') is not False or st.get('people'):
        fail('Family and friends before C turned it on: %s' % st)
    r = c.call('moonlight.share.invite', ok=False)
    if 'Turn on' not in json.dumps(r):
        fail('adding a person while off: %s' % r)
    c.call('moonlight.share.set', {'on': True})
    calls = read_json(os.path.join(c.home, 'family-calls.json')) or []
    if ['family', 'on'] not in calls:
        fail('turning it on didn\'t set Sunshine up: %s' % calls)
    if (read_json(os.path.join(c.home, '.config/nimbus/family-moonlight.json')) or {}).get('on') is not True:
        fail('the switch wasn\'t saved')
    # Add a person: C shows a code, A types it (and picks C on the network).
    inv = c.call('moonlight.share.invite')['invite']
    code = inv.get('code')
    if not code:
        fail('no code to add a person: %s' % inv)
    wait_for('A to see C ready to pair', lambda: any(n['id'] == c.id and n['pairing'] for n in a.status()['nearby']), 20)
    r = a.call('moonlight.join', {'id': c.id, 'code': code}, timeout=90)
    if r.get('relation') != 'friend':
        fail('A added with a code: %s' % r)
    wait_for('C to list A', lambda: any(p['id'] == a.id for p in c.call('moonlight.share.status')['people']), 20)
    pa = next(p for p in c.call('moonlight.share.status')['people'] if p['id'] == a.id)
    if pa.get('always') is not False or pa.get('what') != 'desktop' or pa.get('limitMinutes') != 0:
        fail('a new person\'s choices: %s' % pa)
    fr = wait_for('A to list C', lambda: next((f for f in a.call('moonlight.friends', {'check': True}, timeout=30)['friends'] if f['id'] == c.id), None), 30)
    if fr.get('online') is not True or fr.get('sharing') is not True:
        fail('C on A\'s list: %s' % fr)
    if any(f['id'] == c.id for f in b.call('moonlight.friends', {'check': True}, timeout=30)['friends']):
        fail('B (never added) lists C')
    notice('moonlight: adding', 'C turned on Let family and friends connect (Sunshine set up), showed a code, A typed it: A is on C\'s list (asked every time, Whole desktop, no limit) and C is on A\'s, online; B never hears of it')
    # Asked every time: no first.
    with open(os.path.join(c.home, 'ask-answer'), 'w') as f:
        f.write('deny')
    r = a.call('moonlight.connect', {'id': c.id}, timeout=120, ok=False)
    if 'said no' not in json.dumps(r):
        fail('C said no: %s' % r)
    asked = read_json(os.path.join(c.home, 'asked.json')) or []
    if not asked or asked[-1][:3] != ['--ask', '--name', 'A']:
        fail('the question on C: %s' % asked)
    # Then yes: Moonlight pairs with a one-time PIN and its connections go through.
    with open(os.path.join(c.home, 'ask-answer'), 'w') as f:
        f.write('allow')
    res, took = moon_session(a, c, 'C')
    mp = read_json(os.path.join(a.home, 'ml-pair.json')) or {}
    got = read_json(os.path.join(c.home, 'pin-got.json')) or []
    if not mp.get('pin') or got[:2] != ['pin', mp['pin']] or not str(got[2]).startswith('Hearth '):
        fail('Moonlight\'s PIN: A\'s Moonlight had %s, C\'s Sunshine got %s' % (mp, got))
    if not mp.get('host', '').startswith('127.77.'):
        fail('Moonlight\'s address for C: %s' % mp)
    log_a = tail(os.path.join(ROOT, 'A.log'), 400000) + tail(os.path.join(ROOT, 'C.log'), 400000)
    if mp['pin'] in log_a:
        fail('the PIN is in a log')
    st = c.call('moonlight.share.status')
    if (st.get('session') or {}).get('who') != 'A' or not os.path.exists(os.path.join(c.run, 'moonlight-session.json')):
        fail('C\'s session: %s' % st.get('session'))
    nt = read_json(os.path.join(c.home, 'notice.json')) or {}
    if nt.get('args', [])[:3] != ['--notice', '--name', 'A']:
        fail('the notice on C: %s' % nt)
    notice('moonlight: connected', 'C asked (no, then yes); A\'s Moonlight paired with C\'s Sunshine with a one-time PIN (never logged) and its TCP (HTTPS, HTTP, RTSP) and UDP (video, control, audio; a 3000-byte packet split and put back) reached C\'s stand-in Sunshine from a network with only lo; %.1f MB/s TCP, %d of %d packets back; C shows the notice and says A is connected; %.1f s from Connect' % (res['tcpMBps'], res['udpBack'], res['udpSent'], took))
    # Stop sharing now.
    t0 = time.time()
    c.call('moonlight.stop')
    wait_for('A\'s Moonlight to be cut off', lambda: os.path.exists(os.path.join(a.home, 'ml-ended.json')), 20)
    stop_s = time.time() - t0
    wait_for('A to say it isn\'t connected', lambda: a.call('moonlight.friends')['connected'] is None, 20)
    if os.path.exists(os.path.join(c.run, 'moonlight-session.json')):
        fail('the session file stayed after Stop sharing now')
    try:
        os.kill(nt['pid'], 0)
        if open('/proc/%d/stat' % nt['pid']).read().split()[2] != 'Z':
            fail('the notice stayed after Stop sharing now')
    except (OSError, KeyError):
        pass
    # Always allow, Games only, a 30-minute limit (18 s here): no question, Steam Big Picture, ends by itself.
    c.call('moonlight.share.person', {'id': a.id, 'always': True, 'what': 'games', 'limitMinutes': 30})
    n_asked = len(read_json(os.path.join(c.home, 'asked.json')) or [])
    t0 = time.time()
    moon_session(a, c, 'C', 'games')
    if len(read_json(os.path.join(c.home, 'asked.json')) or []) != n_asked:
        fail('C asked although A is set to Always allow')
    if (read_json(os.path.join(a.home, 'ml-pair.json')) or {}).get('pin') != mp['pin']:
        fail('Moonlight was paired again although C\'s Sunshine knew it')
    wait_for('the time limit to end it', lambda: os.path.exists(os.path.join(a.home, 'ml-ended.json')), 40)
    limit_s = time.time() - t0
    # Turning it off ends a session at once and lets nobody new in.
    c.call('moonlight.share.person', {'id': a.id, 'limitMinutes': 0, 'what': 'desktop'})
    moon_session(a, c, 'C')
    t0 = time.time()
    c.call('moonlight.share.set', {'on': False})
    wait_for('turning it off to cut A off', lambda: os.path.exists(os.path.join(a.home, 'ml-ended.json')), 20)
    off_s = time.time() - t0
    if ['family', 'off'] not in (read_json(os.path.join(c.home, 'family-calls.json')) or []):
        fail('turning it off didn\'t undo Sunshine\'s setup')
    r = a.call('moonlight.connect', {'id': c.id}, timeout=60, ok=False)
    if "isn't letting anyone" not in json.dumps(r):
        fail('connecting while it\'s off: %s' % r)
    notice('moonlight: stopping', 'Stop sharing now cut A off in %.1f s; Always allow asked nothing and Games only started Steam Big Picture, the 30-minute limit (18 s here) ended it after %.1f s; turning the switch off cut A off in %.1f s and then refused A' % (stop_s, limit_s, off_s))
    # Removing the person: C forgets A (Sunshine's pairing too), and A's list drops C.
    c.call('moonlight.share.set', {'on': True})
    c.call('moonlight.share.remove', {'id': a.id})
    if ['family', 'forget', 'Hearth ' + a.id[:8]] not in (read_json(os.path.join(c.home, 'family-calls.json')) or []):
        fail('removing A didn\'t unpair its Moonlight from Sunshine')
    wait_for('A to drop C', lambda: not any(f['id'] == c.id for f in a.call('moonlight.friends', {'check': True}, timeout=30)['friends']), 30)
    r = a.call('moonlight.connect', {'id': c.id}, timeout=60, ok=False)
    if 'error' not in r:
        fail('connecting after being removed: %s' % r)
    notice('moonlight: removing', 'C removed A: its Moonlight unpaired from Sunshine, C gone from A\'s list, A can\'t connect')
    c.call('moonlight.share.set', {'on': False})
    sun.terminate()


def family():
    """Pair once for all your devices: A-B, then B-C, and A and C are paired with no code. Removing C on A removes
    it from B too. A friend's device pairs with one device only and nobody else hears of it."""
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT)
    family_network()
    pk, pk_url = pkarr_relay('hfA', '10.78.0.1')
    fakes = {}
    for name, text in (('status', FAKE_STATUS), ('remote', FAKE_REMOTE)):
        fakes[name] = os.path.join(ROOT, 'fake-%s.py' % name)
        with open(fakes[name], 'w') as f:
            f.write(text)
    env = {'HEARTH_LINK_RELAY': 'off', 'HEARTH_LINK_NO_DNS': '1', 'HEARTH_LINK_PKARR': pk_url, 'HEARTH_LINK_NO_FIREWALL': '1',
           'HEARTH_DEVICE_STATUS': fakes['status'], 'HEARTH_REMOTE_DESKTOP': fakes['remote'],
           'FAKE_HOST_PIN': os.path.join(ROOT, 'C', 'home', 'pin-got.json')}
    env.update(moon_env())
    # C is a Hearth TV (Hearth TV runs Hearth Link with kind tv): Send to TV is tried on it (send_to_tv()).
    a, b, c = Dev('A', 'hfA', dict(env, HEARTH_LINK_KIND='desktop')), Dev('B', 'hfB', dict(env, HEARTH_LINK_KIND='handheld')), Dev('C', 'hfC', dict(env, HEARTH_LINK_KIND='tv', HEARTH_LINK_EDITION='Hearth TV'))
    for d in (a, b, c):
        d.start()
    wait_for('the three to hear each other', lambda: all(any(n['id'] == o.id for n in d.status()['nearby']) for d in (a, b, c) for o in (a, b, c) if o is not d), 40)
    r = pair_quick(a, b)
    if r.get('friend') is not False:
        fail('A and B (both My device): %s' % r)
    pair_quick(c, b)
    t0 = time.time()
    wait_for('A and C to be paired through B', lambda: peer_of(a, c) and peer_of(c, a), 60)
    pa, pc = peer_of(a, c), peer_of(c, a)
    if pa.get('friend') or pa.get('introducedBy') != 'B' or pc.get('introducedBy') != 'B':
        fail('A and C as each other lists them: %s / %s' % (pa, pc))
    # They really talk: A asks C for its games (C only answers devices it's paired with).
    g = a.call('games', {'peer': c.id, 'refresh': True}, timeout=60)
    if 'games' not in g:
        fail('A asking C (paired through B): %s' % g)
    notice('your devices', 'paired A-B and B-C with codes; A and C were paired with each other through B in %.1f s, no code, and talk directly' % (time.time() - t0))
    send_to_tv(a, b, c)
    # Removing C on A removes it from B too (and C hears from A).
    a.call('unpair', {'id': c.id})
    wait_for('C to forget A', lambda: not peer_of(c, a), 30)
    wait_for('B to drop C (removed on A)', lambda: not peer_of(b, c), 60)
    wait_for('C to forget B too', lambda: not peer_of(c, b), 30)
    time.sleep(3)
    if peer_of(a, c):
        fail('A added C back from an older list')
    notice('removing', 'removed C on A: B dropped C by itself and told it, C forgot both; nobody added it back')
    # C again, this time as a friend's device of A's (A picks A friend's device): paired with A only.
    r = pair_quick(c, a, mine_joiner=True, mine_host=False)
    if r.get('friend') is not True:
        fail('pairing as a friend: %s' % r)
    if not (peer_of(a, c) or {}).get('friend') or not (peer_of(c, a) or {}).get('friend'):
        fail('the friend mark: %s / %s' % (peer_of(a, c), peer_of(c, a)))
    time.sleep(12)
    if peer_of(b, c) or peer_of(c, b):
        fail('a friend\'s device was passed on to B: %s / %s' % (peer_of(b, c), peer_of(c, b)))
    # A friend can see A's games (for copies) but not sync with it.
    if 'games' not in c.call('games', {'peer': a.id, 'refresh': True}, timeout=60):
        fail('a friend asking for games')
    r = a.call('sync.now', {'peer': c.id}, ok=False)
    if 'own devices' not in json.dumps(r):
        fail('sync with a friend\'s device: %s' % r)
    notice('friends', "C paired with A as a friend's device: B never heard of it, games can be seen for copies, settings and the library don't sync")
    r = a.call('tv.send', {'peer': c.id, 'url': 'https://vimeo.com/1084537'}, ok=False)
    if 'your paired devices' not in json.dumps(r):
        fail("Send to TV to a friend's TV: %s" % r)
    notice('Send to TV: friends', "a friend's TV can't be sent to")
    family_member(a, b, c)
    moon_family(a, b, c)
    for d in (a, b, c):
        d.kill(signal.SIGTERM)
    pk.terminate()
    log_b = tail(os.path.join(ROOT, 'B.log'), 200000)
    notice('family log on B', ' | '.join(l.split('family: ', 1)[1] for l in log_b.splitlines() if 'family: ' in l)[-1500:])

if __name__ == '__main__':
    what = sys.argv[1] if len(sys.argv) > 1 else 'lan'
    # Moonlight's private network (moonlight.rs) runs hearth-link again inside a user namespace where only root is
    # mapped: the runner's home (0750, its own user) can't be entered from there, so the binary is run from /tmp. On a
    # device the person's own user is mapped and hearth-link is /usr/bin/hearth-link.
    if os.path.exists(BIN) and not BIN.startswith('/usr/'):
        os.makedirs('/tmp/hl-bin', exist_ok=True)
        os.chmod('/tmp/hl-bin', 0o755)
        shutil.copy(BIN, '/tmp/hl-bin/hearth-link')
        os.chmod('/tmp/hl-bin/hearth-link', 0o755)
        BIN = '/tmp/hl-bin/hearth-link'
    try:
        try:
            {'lan': lan, 'internet': internet, 'family': family}[what]()
        except SystemExit:
            raise
        except Exception as e:
            import traceback
            fail('%s: %s\n%s' % (type(e).__name__, e, traceback.format_exc()[-1500:]))
    finally:
        for p in ('hlA', 'hlB', 'hiA', 'natA', 'inet', 'natB', 'hiB', 'hfA', 'hfB', 'hfC', 'hfBr'):
            subprocess.run('ip netns pids %s 2>/dev/null | xargs -r kill -9' % p, shell=True)
