#!/usr/bin/env python3
"""A pretend phone that speaks KDE Connect's protocol (version 8) with only Python's standard library, for testing
Hearth Connect without a real phone: it announces itself over UDP, takes Hearth's TCP connection, upgrades it to TLS
with its own self-signed certificate (made with the openssl command), pairs, and sends and receives packets and files
the way KDE Connect's Android app does. Used by test_phone_link.py and by phone-link.yml's screenshots (`scene`).

  python3 fake_phone.py scene          pairs with the Hearth Connect running here and fills it with a phone's day
"""
import hashlib
import json
import os
import queue
import socket
import ssl
import subprocess
import sys
import tempfile
import threading
import time
import uuid

UDP_PORT = 1716

INCOMING = [
    'kdeconnect.ping', 'kdeconnect.battery', 'kdeconnect.share.request', 'kdeconnect.notification.request',
    'kdeconnect.notification.reply', 'kdeconnect.notification.action', 'kdeconnect.clipboard', 'kdeconnect.clipboard.connect',
    'kdeconnect.mpris', 'kdeconnect.mpris.request', 'kdeconnect.findmyphone.request', 'kdeconnect.mousepad.echo',
    'kdeconnect.mousepad.keyboardstate', 'kdeconnect.runcommand', 'kdeconnect.telephony.request_mute', 'kdeconnect.sms.request',
    'kdeconnect.sms.request_conversations', 'kdeconnect.sms.request_conversation', 'kdeconnect.systemvolume', 'kdeconnect.sftp.request',
    'kdeconnect.contacts.request_all_uids_timestamps', 'kdeconnect.contacts.request_vcards_by_uid',
    'kdeconnect.connectivity_report.request', 'kdeconnect.photo.request',
]
OUTGOING = [
    'kdeconnect.ping', 'kdeconnect.battery', 'kdeconnect.share.request', 'kdeconnect.notification', 'kdeconnect.clipboard',
    'kdeconnect.mousepad.request', 'kdeconnect.mpris', 'kdeconnect.mpris.request', 'kdeconnect.findmyphone.request',
    'kdeconnect.runcommand.request', 'kdeconnect.telephony', 'kdeconnect.sms.messages', 'kdeconnect.presenter',
    'kdeconnect.systemvolume.request', 'kdeconnect.sftp', 'kdeconnect.contacts.response_uids_timestamps',
    'kdeconnect.contacts.response_vcards', 'kdeconnect.connectivity_report', 'kdeconnect.photo',
]


def now_ms():
    return int(time.time() * 1000)


def packet(kind, body, **extra):
    p = {'id': now_ms(), 'type': kind, 'body': body}
    p.update(extra)
    return (json.dumps(p) + '\n').encode()


# ---- certificates: what KDE Connect's verification key is made of (the SubjectPublicKeyInfo of each)

def _tlv(b, i):
    tag, ln = b[i], b[i + 1]
    i += 2
    if ln & 0x80:
        n = ln & 0x7f
        ln = int.from_bytes(b[i:i + n], 'big')
        i += n
    return tag, i, ln


def spki(der):
    """The SubjectPublicKeyInfo (DER) of an X.509 certificate (DER)."""
    _, i, _ = _tlv(der, 0)               # Certificate
    _, j, ln = _tlv(der, i)              # tbsCertificate
    fields, k, end = [], j, j + ln
    while k < end:
        tag, c, l2 = _tlv(der, k)
        fields.append((tag, k, c + l2))
        k = c + l2
    if fields and fields[0][0] == 0xa0:  # [0] version
        fields = fields[1:]
    _, s, e = fields[5]                  # serial, signature, issuer, validity, subject, subjectPublicKeyInfo
    return der[s:e]


def verification_key(cert_a, cert_b, timestamp):
    a, b = spki(cert_a), spki(cert_b)
    if a < b:
        a, b = b, a
    return hashlib.sha256(a + b + str(timestamp).encode()).hexdigest()[:8].upper()


class Phone:
    """One pretend phone. `inbox` gets every packet Hearth sends it."""

    def __init__(self, name='Pixel 8', folder=None, tcp_port=None):
        self.name = name
        self.id = uuid.uuid4().hex                       # 32 characters, as KDE Connect's own ids
        self.dir = folder or tempfile.mkdtemp(prefix='fake-phone-')
        self.cert_pem = os.path.join(self.dir, 'cert.pem')
        self.key_pem = os.path.join(self.dir, 'key.pem')
        subprocess.run(['openssl', 'req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:prime256v1', '-nodes',
                        '-keyout', self.key_pem, '-out', self.cert_pem, '-days', '3650',
                        '-subj', '/O=KDE/OU=Kde connect/CN=' + self.id], check=True, capture_output=True)
        self.cert_der = ssl.PEM_cert_to_DER_cert(open(self.cert_pem).read())
        self.listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        for p in ([tcp_port] if tcp_port else range(1720, 1739)):
            try:
                self.listener.bind(('127.0.0.1', p))
                self.tcp_port = p
                break
            except OSError:
                continue
        else:
            raise RuntimeError('no free port for the phone')
        self.listener.listen(4)
        self.inbox = queue.Queue()
        self.seen = []                                    # every packet received, for checks
        self.tls = None
        self.peer = None
        self.pc_cert = None
        self.lock = threading.Lock()

    def identity(self, with_port=True):
        body = {'deviceId': self.id, 'deviceName': self.name, 'deviceType': 'phone', 'protocolVersion': 8,
                'incomingCapabilities': INCOMING, 'outgoingCapabilities': OUTGOING}
        if with_port:
            body['tcpPort'] = self.tcp_port
        return packet('kdeconnect.identity', body)

    def announce(self, host='127.0.0.1'):
        u = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        u.sendto(self.identity(), (host, UDP_PORT))
        u.close()

    def _ctx_client(self):
        c = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        c.check_hostname = False
        c.verify_mode = ssl.CERT_NONE
        c.load_cert_chain(self.cert_pem, self.key_pem)
        return c

    def _ctx_server(self):
        c = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        c.verify_mode = ssl.CERT_NONE
        c.load_cert_chain(self.cert_pem, self.key_pem)
        return c

    def link(self, timeout=20):
        """Waits for Hearth to connect (after `announce`), then TLS (Hearth is the TLS server) and identities."""
        self.listener.settimeout(timeout)
        sock, _ = self.listener.accept()
        sock.settimeout(timeout)
        line = b''
        while not line.endswith(b'\n'):
            ch = sock.recv(1)
            if not ch:
                raise RuntimeError('Hearth closed the connection before its identity')
            line += ch
        first = json.loads(line)
        assert first['type'] == 'kdeconnect.identity', first
        self.tls = self._ctx_client().wrap_socket(sock)
        self.pc_cert = self.tls.getpeercert(binary_form=True)
        self.tls.sendall(self.identity(with_port=False))
        self.rfile = self.tls.makefile('rb')
        self.peer = json.loads(self.rfile.readline())
        self.first_identity = first
        threading.Thread(target=self._read, daemon=True).start()
        return self.peer

    def _read(self):
        try:
            for line in self.rfile:
                try:
                    p = json.loads(line)
                except ValueError:
                    continue
                self.seen.append(p)
                self.inbox.put(p)
        except (OSError, ValueError):
            pass

    def send(self, kind, body, payload=None):
        """A packet to Hearth; with `payload` (bytes) a file follows on its own TLS connection, the phone serving it."""
        extra = {}
        if payload is not None:
            srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            srv.bind(('127.0.0.1', 0))
            srv.listen(1)
            extra = {'payloadSize': len(payload), 'payloadTransferInfo': {'port': srv.getsockname()[1]}}

            def serve():
                srv.settimeout(30)
                try:
                    c, _ = srv.accept()
                    t = self._ctx_server().wrap_socket(c, server_side=True)
                    t.sendall(payload)
                    t.close()
                finally:
                    srv.close()
            threading.Thread(target=serve, daemon=True).start()
        with self.lock:
            self.tls.sendall(packet(kind, body, **extra))

    def wait(self, kind, pred=None, timeout=15):
        """The next packet of that kind (matching `pred`), from what came already or what comes next."""
        end = time.time() + timeout
        for p in list(self.seen):
            if p['type'] == kind and (pred is None or pred(p['body'])) and not p.get('_taken'):
                p['_taken'] = True
                return p
        while time.time() < end:
            try:
                p = self.inbox.get(timeout=max(0.05, end - time.time()))
            except queue.Empty:
                break
            if p['type'] == kind and (pred is None or pred(p['body'])) and not p.get('_taken'):
                p['_taken'] = True
                return p
        return None

    def fetch(self, p):
        """A file Hearth offers (it serves it; the phone connects as the TLS client): its bytes."""
        port = p['payloadTransferInfo']['port']
        s = socket.create_connection(('127.0.0.1', port), timeout=20)
        t = self._ctx_client().wrap_socket(s)
        want, got = p.get('payloadSize', -1), b''
        while want < 0 or len(got) < want:
            chunk = t.recv(65536)
            if not chunk:
                break
            got += chunk
        t.close()
        return got

    def pair(self):
        ts = int(time.time())
        self.send('kdeconnect.pair', {'pair': True, 'timestamp': ts})
        return ts


def call(cmd, args=None):
    """One request to the Hearth Connect service here (its --call), its answer."""
    binary = os.environ.get('HEARTH_CONNECT_BIN', 'hearth-connect')
    p = subprocess.run([binary, '--call', cmd, json.dumps(args or {})], capture_output=True, text=True, timeout=30)
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip())
    return json.loads(p.stdout)


def scene():
    """For the screenshots: a paired phone with battery, signal, notifications, music, messages, contacts and a call."""
    ph = Phone('Pixel 8')
    for _ in range(20):
        ph.announce()
        try:
            ph.link(timeout=3)
            break
        except (socket.timeout, OSError):
            continue
    ph.pair()
    for _ in range(40):
        st = call('state')
        d = next((x for x in st['devices'] if x['id'] == ph.id), None)
        if d and d['pair'].get('state') == 'asking':
            break
        time.sleep(0.25)
    if os.environ.get('SCENE_ASKING') == '1':
        # The pairing request stays on screen until the workflow accepts it (`--call accept`).
        print(json.dumps({'id': ph.id, 'asking': True}), flush=True)
        while not (next((x for x in call('state')['devices'] if x['id'] == ph.id), None) or {}).get('paired'):
            time.sleep(0.5)
    else:
        call('accept', {'id': ph.id})
    time.sleep(1)
    ph.send('kdeconnect.battery', {'currentCharge': 81, 'isCharging': False, 'thresholdEvent': 0})
    ph.send('kdeconnect.connectivity_report', {'signalStrengths': {'1': {'networkType': '5G', 'signalStrength': 3}}})
    ph.send('kdeconnect.contacts.response_uids_timestamps', {'uids': ['c1', 'c2'], 'c1': 11, 'c2': 12})
    time.sleep(0.5)
    ph.send('kdeconnect.contacts.response_vcards', {'uids': ['c1', 'c2'],
            'c1': 'BEGIN:VCARD\nVERSION:2.1\nFN:Ada Lovelace\nTEL;CELL:+1 555 0100\nEND:VCARD',
            'c2': 'BEGIN:VCARD\nVERSION:2.1\nFN:Grace Hopper\nTEL;CELL:+1 555 0199\nEND:VCARD'})
    ph.send('kdeconnect.mpris', {'playerList': ['Spotify']})
    ph.send('kdeconnect.mpris', {'player': 'Spotify', 'title': 'Clair de Lune', 'artist': 'Claude Debussy', 'isPlaying': True, 'nowPlaying': 'Clair de Lune'})
    t0 = now_ms()
    msgs = [
        (7, '+15550100', 'Lunch at 1? The usual place', t0 - 600000, 1, 1),
        (7, '+15550100', 'Sure, see you there', t0 - 540000, 2, 1),
        (7, '+15550100', 'Running 5 min late!', t0 - 60000, 1, 0),
        (9, '+15550199', 'The slides look great', t0 - 7200000, 1, 1),
        (11, '+15550142', 'Your package arrives today', t0 - 86400000, 1, 1),
    ]
    ph.send('kdeconnect.sms.messages', {'version': 2, 'messages': [
        {'event': 1, 'body': b, 'addresses': [{'address': a}], 'date': d, 'type': ty, 'read': r, 'thread_id': t, '_id': i + 1, 'sub_id': 1}
        for i, (t, a, b, d, ty, r) in enumerate(msgs)]})
    ph.send('kdeconnect.notification', {'id': 'n1', 'appName': 'Messages', 'title': 'Ada Lovelace', 'text': 'Running 5 min late!',
            'ticker': 'Ada Lovelace: Running 5 min late!', 'isClearable': True, 'time': str(t0), 'requestReplyId': 'r1', 'actions': ['Mark as read']})
    ph.send('kdeconnect.notification', {'id': 'n2', 'appName': 'Calendar', 'title': 'Standup in 10 minutes', 'text': 'Room 4 · Video call',
            'isClearable': True, 'time': str(t0 - 1000)})
    ph.send('kdeconnect.notification', {'id': 'n3', 'appName': 'Gmail', 'title': 'Your receipt', 'text': 'Thanks for your order',
            'isClearable': True, 'time': str(t0 - 2000), 'silent': True})
    try:
        call('command_add', {'name': 'Lock the screen', 'command': 'loginctl lock-session'})
        call('command_add', {'name': 'Start Steam', 'command': 'steam'})
    except RuntimeError:
        pass
    if os.environ.get('SCENE_CALL') == '1':
        ph.send('kdeconnect.telephony', {'event': 'ringing', 'phoneNumber': '+1 555 0199', 'contactName': ''})
    print(json.dumps({'id': ph.id, 'paired': True}), flush=True)
    # Answers what Hearth asks for while the pictures are taken.
    while True:
        p = ph.wait('kdeconnect.sms.request_conversation', timeout=60)
        if p:
            ph.send('kdeconnect.sms.messages', {'version': 2, 'messages': [
                {'event': 1, 'body': b, 'addresses': [{'address': a}], 'date': d, 'type': ty, 'read': r, 'thread_id': t, '_id': i + 1}
                for i, (t, a, b, d, ty, r) in enumerate(msgs) if t == p['body'].get('threadID')]})


if __name__ == '__main__':
    if sys.argv[1:] == ['scene']:
        scene()
    else:
        print(__doc__)
