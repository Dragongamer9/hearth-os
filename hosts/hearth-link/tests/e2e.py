#!/usr/bin/env python3
"""Hearth Link end to end: two real `hearth-link serve` instances, each in its own network namespace with its own
home folder, driven over their sockets the way Hearth's UI does (`hearth-link call ...`). Run as root (namespaces).

  e2e.py lan        one network: pairing first: too many wrong codes stop it, a code that ran out, a short code
                    (looked up on a local pkarr relay: a typo caught at once, a code nobody shows, then pairing,
                    then the code used up), the long link; then discovery (kind and edition shown) and pairing by
                    picking the device and its 6-digit code (a wrong code first, then retried), a multi-GB synthetic Steam
                    game copied (speed posted), a copy killed halfway and resumed, an update copied as only its
                    changed chunks, every file's hash compared, Hearth data synced both ways, quiet while playing,
                    unpairing, Hearth Storage (shared folder, media link, game archive there and back, backups)
  e2e.py internet   both devices behind their own NAT, a local iroh relay in between: pairing with a short code
                    (found through a pkarr relay on the "internet" too, nothing else typed), a copy
                    through the relay with direct UDP blocked (speed posted, the copy says it's relayed), then with
                    UDP allowed (hole punching; posted as information)

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


def lan():
    shutil.rmtree(ROOT, ignore_errors=True)
    os.makedirs(ROOT)
    lan_network()
    pk, pk_url = pkarr_relay('hlA', '10.77.0.1')
    env = {'HEARTH_LINK_RELAY': 'off', 'HEARTH_LINK_NO_DNS': '1', 'HEARTH_LINK_PKARR': pk_url}
    a = Dev('A', 'hlA', dict(env, HEARTH_LINK_KIND='desktop', HEARTH_LINK_EDITION='Anatase'))
    b = Dev('B', 'hlB', dict(env, HEARTH_LINK_KIND='handheld', HEARTH_LINK_EDITION='Peppermint'))
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
    env = {'HEARTH_LINK_RELAY': url, 'HEARTH_LINK_NO_DNS': '1', 'HEARTH_LINK_PKARR': pk_url}
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
    a.kill(signal.SIGTERM)
    b.kill(signal.SIGTERM)
    relay.terminate()
    pk.terminate()


if __name__ == '__main__':
    what = sys.argv[1] if len(sys.argv) > 1 else 'lan'
    try:
        try:
            {'lan': lan, 'internet': internet}[what]()
        except SystemExit:
            raise
        except Exception as e:
            import traceback
            fail('%s: %s\n%s' % (type(e).__name__, e, traceback.format_exc()[-1500:]))
    finally:
        for p in ('hlA', 'hlB', 'hiA', 'natA', 'inet', 'natB', 'hiB'):
            subprocess.run('ip netns pids %s 2>/dev/null | xargs -r kill -9' % p, shell=True)
