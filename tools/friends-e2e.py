#!/usr/bin/env python3
"""Hearth Friends end to end, against a real Matrix homeserver (CI starts Synapse: .github/workflows/friends.yml).

Two users, each with their own home folder, runtime folder and Hearth Friends service:

  python3 tools/friends-e2e.py --bin PATH/hearth-friends --homeserver http://127.0.0.1:8008
      talks to the two services directly (the protocol in hosts/hearth-friends/src/proto.rs)
  python3 tools/friends-e2e.py --cores --bin ... --hearth PATH/hearth --homeserver ...
      the same steps through two Hearth cores (`hearth --core-stdio`), with the calls Game Mode's page makes
      (mfriends.* in hosts/hearth-core/src/matrix_friends.rs)

Needs a D-Bus session with an unlocked Secret Service (GNOME Keyring): the sessions are kept there.
Prints PASS lines; any failure prints FAIL and exits 1.
"""
import argparse
import base64
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile
import time
import zlib

ap = argparse.ArgumentParser()
ap.add_argument('--bin', required=True)
ap.add_argument('--homeserver', required=True)
ap.add_argument('--cores', action='store_true')
ap.add_argument('--hearth')
A = ap.parse_args()
BIN = os.path.abspath(A.bin)
RUN = str(int(time.time()))[-6:]


def fail(msg):
    print('FAIL ' + msg, flush=True)
    for u in USERS:
        u.stop()
        try:
            print('--- %s service log (tail) ---' % u.name)
            print(open(u.log_path).read()[-3000:])
        except OSError:
            pass
    sys.exit(1)


def ok(msg):
    print('PASS ' + msg, flush=True)


def wait(what, fn, secs=40, every=0.7):
    end = time.time() + secs
    last = None
    while time.time() < end:
        try:
            v = fn()
            if v:
                return v
            last = v
        except Exception as e:  # noqa: BLE001
            last = e
        time.sleep(every)
    fail('%s (waited %ds; last: %r)' % (what, secs, last))


class Service:
    """One user's Hearth Friends service, reached over its socket."""

    def __init__(self, name):
        self.name = name
        self.dir = tempfile.mkdtemp(prefix='friends-%s-' % name, dir=os.environ.get('RUNNER_TEMP') or None)
        self.home = os.path.join(self.dir, 'home')
        self.run = os.path.join(self.dir, 'run')
        os.makedirs(self.home)
        os.makedirs(self.run, mode=0o700)
        self.sock = os.path.join(self.run, 'hearth-friends.sock')
        self.log_path = os.path.join(self.dir, 'service.log')
        self.proc = None
        self.core = None
        self.rid = 0

    def env(self):
        e = dict(os.environ)
        e.update(HOME=self.home, XDG_RUNTIME_DIR=self.run, XDG_CONFIG_HOME=os.path.join(self.home, '.config'),
                 XDG_DATA_HOME=os.path.join(self.home, '.local/share'), XDG_CACHE_HOME=os.path.join(self.home, '.cache'),
                 HEARTH_FRIENDS_SOCKET=self.sock, HEARTH_FRIENDS_BIN=BIN)
        return e

    def start(self):
        self.proc = subprocess.Popen([BIN, 'serve'], env=self.env(), stdout=open(self.log_path, 'a'), stderr=subprocess.STDOUT)
        wait('%s service listening' % self.name, lambda: os.path.exists(self.sock), 20, 0.2)

    def stop(self):
        if self.core:
            try:
                self.core.kill()
            except OSError:
                pass
            self.core = None
        if self.proc:
            self.proc.terminate()
            try:
                self.proc.wait(5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
            self.proc = None

    def call(self, method, args=None, timeout=90):
        if A.cores:
            return self.core_call('mfriends.' + method, args or {}, timeout)
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(timeout)
        s.connect(self.sock)
        s.sendall((json.dumps({'id': 1, 'method': method, 'args': args or {}}) + '\n').encode())
        buf = b''
        while not buf.endswith(b'\n'):
            chunk = s.recv(1 << 20)
            if not chunk:
                break
            buf += chunk
        s.close()
        r = json.loads(buf)
        if not r.get('ok'):
            raise RuntimeError(r.get('error'))
        return r.get('value')

    # ---- through a Hearth core (hosts/hearth-rs --core-stdio), as the page calls it
    def start_core(self):
        self.core = subprocess.Popen([os.path.abspath(A.hearth), '--core-stdio'], env=self.env(), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                     stderr=open(os.path.join(self.dir, 'core.log'), 'a'), bufsize=0)
        self.core_buf = b''
        self.core_events = []

    def core_call(self, method, args, timeout):
        self.rid += 1
        rid = self.rid
        msg = {'t': 'page', 'win': 'main', 'msg': {'type': 'call', 'id': rid, 'method': method, 'args': args}}
        self.core.stdin.write((json.dumps(msg) + '\n').encode())
        self.core.stdin.flush()
        end = time.time() + timeout
        import select
        while time.time() < end:
            r, _, _ = select.select([self.core.stdout], [], [], 0.5)
            if not r:
                continue
            chunk = os.read(self.core.stdout.fileno(), 1 << 20)
            if not chunk:
                raise RuntimeError('the core stopped')
            self.core_buf += chunk
            while b'\n' in self.core_buf:
                line, self.core_buf = self.core_buf.split(b'\n', 1)
                try:
                    o = json.loads(line)
                except ValueError:
                    continue
                m = o.get('msg') or {}
                if m.get('type') == 'event':
                    self.core_events.append(m)
                if m.get('type') == 'result' and m.get('id') == rid:
                    if not m.get('ok'):
                        raise RuntimeError(m.get('error'))
                    return m.get('value')
        raise RuntimeError('no answer to %s' % method)


def png(w=64, h=40):
    rows = []
    for y in range(h):
        row = bytearray([0])
        for x in range(w):
            row += bytes([(x * 4) % 256, (y * 6) % 256, 160])
        rows.append(bytes(row))
    raw = b''.join(rows)

    def chunk(t, d):
        return struct.pack('>I', len(d)) + t + d + struct.pack('>I', zlib.crc32(t + d) & 0xffffffff)
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 8, 2, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')


USERS = []
alice, bob, carol = Service('alice'), Service('bob'), Service('carol')
USERS += [alice, bob, carol]
HS = A.homeserver
PW = 'ci-only-pass-' + RUN
names = {u: '%s%s' % (u.name, RUN) for u in USERS}
if A.cores:
    names = {u: '%sc%s' % (u.name, RUN) for u in USERS}

try:
    for u in USERS:
        if A.cores:
            # The core starts the service itself when it isn't running (HEARTH_FRIENDS_BIN points at the build).
            u.start_core()
        else:
            u.start()

    # ---- accounts
    st = alice.call('status')
    if st.get('signedIn'):
        fail('a fresh service says it is signed in')
    for u in USERS:
        r = u.call('register', {'homeserver': HS, 'user': names[u], 'password': PW})
        if not r.get('signedIn'):
            fail('%s: sign-up did not sign in: %r' % (u.name, r))
    ids = {u: u.call('status')['userId'] for u in USERS}
    ok('three accounts made on the homeserver and signed in (%s)' % ', '.join(ids.values()))
    st = alice.call('status')
    if not st.get('keyringOk'):
        fail('the session was not kept in the keyring: %r' % st)
    if 'end-to-end encrypted' not in (st.get('encryption') or ''):
        fail('no end-to-end note in status')
    ok('session kept in the Secret Service; status says where the account lives: %s' % st.get('lives'))
    for u in USERS:
        wait('%s first sync' % u.name, lambda u=u: u.call('status').get('synced'))
    wrong = None
    try:
        alice.call('add', {'userId': 'not an address'})
    except RuntimeError as e:
        wrong = str(e)
    if not wrong:
        fail('a bad address was accepted')

    # ---- a friend request by link, accepted
    link = alice.call('link')
    if not link.get('qr', '').startswith('<?xml') and '<svg' not in (link.get('qr') or ''):
        fail('no QR code: %r' % link)
    r = bob.call('add', {'userId': link['matrixTo']})
    room = r['roomId']
    ok('bob added alice from her matrix.to link (QR code made too)')
    req = wait('alice sees bob\'s friend request', lambda: [x for x in alice.call('requests')['requests'] if x['from'] == ids[bob]])
    alice.call('accept', {'roomId': req[0]['roomId']})
    wait('they are friends on both sides', lambda: any(f['userId'] == ids[bob] and f['state'] == 'friend' for f in alice.call('friends'))
         and any(f['userId'] == ids[alice] and f['state'] == 'friend' for f in bob.call('friends')))
    ok('alice accepted; both list each other as friends')

    # ---- encrypted messages both ways
    bob.call('send', {'roomId': room, 'text': 'hello alice ' + RUN})
    m = wait('alice gets bob\'s message', lambda: [x for x in alice.call('messages', {'roomId': room})['messages'] if x.get('body') == 'hello alice ' + RUN])
    if not m[0].get('encrypted'):
        fail('the message was not end-to-end encrypted: %r' % m[0])
    alice.call('send', {'roomId': room, 'text': 'hi bob ✨ ' + RUN})
    m = wait('bob gets alice\'s reply', lambda: [x for x in bob.call('messages', {'roomId': room})['messages'] if x.get('body') == 'hi bob ✨ ' + RUN])
    if not m[0].get('encrypted'):
        fail('the reply was not encrypted')
    chats = bob.call('chats')
    if not any(c['roomId'] == room and c['encrypted'] for c in chats):
        fail('the chat is not marked encrypted: %r' % chats)
    ok('messages both ways, decrypted on the other side (Megolm), emoji intact')
    wait('bob has an unread count', lambda: any(c['roomId'] == room and c['unread'] >= 1 for c in bob.call('chats')), 20)
    bob.call('read', {'roomId': room})
    if any(c['roomId'] == room and c['unread'] for c in bob.call('chats')):
        fail('reading did not clear the unread badge')
    ok('unread badge counted and cleared with a read receipt')

    # ---- typing
    bob.call('typing', {'roomId': room, 'on': True})
    wait('alice sees bob typing', lambda: any(c['roomId'] == room and ids[bob] in c.get('typing', []) for c in alice.call('chats')), 20)
    bob.call('typing', {'roomId': room, 'on': False})
    ok('typing indicator')

    # ---- a picture (a screenshot)
    shot = os.path.join(alice.dir, 'shot.png')
    open(shot, 'wb').write(png())
    alice.call('sendImage', {'roomId': room, 'path': shot})
    im = wait('bob gets the picture', lambda: [x for x in bob.call('messages', {'roomId': room})['messages'] if x.get('kind') == 'image'])
    data = bob.call('image', {'roomId': room, 'eventId': im[-1]['id']})['url']
    if not data.startswith('data:image/png;base64,') or base64.b64decode(data.split(',', 1)[1])[:8] != b'\x89PNG\r\n\x1a\n':
        fail('the picture did not come through: %r' % data[:80])
    ok('screenshot sent encrypted and opened on the other side')

    # ---- presence and Playing <game>
    alice.call('playing', {'game': {'title': 'Portal 2', 'store': 'steam', 'id': 'steam:620', 'appId': '620'}, 'joinable': True})
    f = wait('bob sees alice playing', lambda: [x for x in bob.call('friends') if x['userId'] == ids[alice] and x.get('game') == 'Portal 2'], 45)
    if not f[0].get('joinable'):
        fail('alice is not shown as joinable: %r' % f[0])
    ok('presence: bob sees "Playing Portal 2 · Joinable" (%s)' % f[0]['presence'])
    alice.call('settings', {'sharePlaying': False})
    wait('the game goes away when sharing is off', lambda: [x for x in bob.call('friends') if x['userId'] == ids[alice] and not x.get('game')], 45)
    alice.call('settings', {'sharePlaying': True})
    ok('"who sees what you\'re playing" off hides it')

    # ---- game invite and Play together
    alice.call('invite', {'roomId': room, 'game': {'title': 'Portal 2', 'store': 'steam', 'id': 'steam:620', 'appId': '620', 'server': '192.168.1.20:27015'}})
    g = wait('bob gets the game invite', lambda: [x for x in bob.call('messages', {'roomId': room})['messages'] if x.get('kind') == 'game'])
    gm = g[-1]['game']
    if gm.get('appId') != '620' or gm.get('server') != '192.168.1.20:27015':
        fail('the invite card lost its details: %r' % gm)
    ok('game invite card with store, id and server address')
    bob.call('play', {'roomId': room, 'play': {'action': 'request', 'session': 'pt-' + RUN, 'game': 'Portal 2', 'pin': '4821', 'device': 'Bob\'s Ally'}})
    p = wait('alice gets the Play together request', lambda: [x for x in alice.call('messages', {'roomId': room})['messages'] if x.get('kind') == 'play'])
    if p[-1]['play'].get('pin') != '4821' or not p[-1].get('encrypted'):
        fail('Play together request wrong: %r' % p[-1])
    ok('Play together request (with its pairing PIN) arrives inside the encrypted chat')

    # ---- a group chat
    gr = alice.call('createGroup', {'name': 'Couch crew ' + RUN, 'users': [ids[bob]]})
    inv = wait('bob is invited to the group', lambda: [x for x in bob.call('requests')['invites'] if x['roomId'] == gr['roomId']])
    bob.call('accept', {'roomId': gr['roomId']})
    bob.call('send', {'roomId': gr['roomId'], 'text': 'group hello'})
    wait('alice gets the group message', lambda: [x for x in alice.call('messages', {'roomId': gr['roomId']})['messages'] if x.get('body') == 'group hello' and x.get('encrypted')])
    ok('group chat: invite from a friend lands with the chats, encrypted messages')

    # ---- a stranger: request, block, report
    carol.call('add', {'userId': ids[alice]})
    wait('carol\'s request is in alice\'s Requests', lambda: [x for x in alice.call('requests')['requests'] if x['from'] == ids[carol]])
    rep = alice.call('report', {'userId': ids[carol], 'reason': 'CI test report'})
    alice.call('block', {'userId': ids[carol]})
    if ids[carol] not in alice.call('blocked'):
        fail('carol is not on the blocked list')
    wait('carol\'s request is gone after blocking', lambda: not [x for x in alice.call('requests')['requests'] if x['from'] == ids[carol]], 20)
    alice.call('block', {'userId': ids[carol], 'on': False})
    ok('stranger\'s request went to Requests; reported (%s), blocked (request gone), unblocked' % rep.get('reported'))

    # ---- mute and settings
    alice.call('mute', {'roomId': room, 'on': True})
    if not any(c['roomId'] == room and c['muted'] for c in alice.call('chats')):
        fail('mute did not stick')
    alice.call('mute', {'roomId': room, 'on': False})
    s = alice.call('settings', {'friendsOnly': True, 'presence': 'unavailable'})
    if s['settings']['presence'] != 'unavailable':
        fail('settings did not change')
    alice.call('settings', {'presence': 'online'})
    ok('mute and privacy settings')

    # ---- the session comes back after a restart (from the keyring), and signing out ends it
    if not A.cores:
        bob.stop()
        bob.start()
        st = wait('bob is signed in again after a restart', lambda: bob.call('status').get('signedIn') and bob.call('status'))
        wait('bob syncs again', lambda: bob.call('status').get('synced'))
        alice.call('send', {'roomId': room, 'text': 'after restart ' + RUN})
        wait('bob still decrypts after the restart', lambda: [x for x in bob.call('messages', {'roomId': room})['messages'] if x.get('body') == 'after restart ' + RUN and x.get('encrypted')])
        ok('restart: session and encryption keys came back from the keyring and the encrypted store')
    r = alice.call('logout')
    if r.get('signedIn'):
        fail('still signed in after signing out')
    ok('signed out')
    # Nothing secret in the services' logs.
    for u in USERS:
        u.stop()
        log = open(u.log_path).read() if os.path.exists(u.log_path) else ''
        if 'syt_' in log or PW in log:
            fail('%s service log has a token or password' % u.name)
    ok('no tokens or passwords in the services\' logs')
except SystemExit:
    raise
except Exception as e:  # noqa: BLE001
    import traceback
    traceback.print_exc()
    fail(repr(e))
finally:
    for u in USERS:
        u.stop()
