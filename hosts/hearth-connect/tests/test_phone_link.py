#!/usr/bin/env python3
"""Hearth Connect's phone link against a pretend phone (fake_phone.py: KDE Connect's protocol in plain Python), over
localhost TLS: finding each other, pairing with the verification key both compute, and a packet (or more) for every
feature: ping, battery, notifications with buttons and Reply (through a stand-in notification service on a private
session bus), clipboard both ways, files/text/links both ways, find my phone, media both ways, remote input, run
commands, calls (pausing and resuming a stand-in player), text messages, contacts, the presentation remote, this PC's
volume, browsing the phone's files (a stand-in sshfs), the phone's camera, unpairing, and Phone link on/off keeping
every other setting. Needs HEARTH_CONNECT_BIN (a built hearth-connect), openssl, dbus-daemon; xclip with DISPLAY for
the clipboard, python3-gi for the notification service (each skips with a notice when missing).

  HEARTH_CONNECT_BIN=target/release/hearth-connect python3 -m unittest tests/test_phone_link.py -v
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import fake_phone  # noqa: E402

BIN = os.environ.get('HEARTH_CONNECT_BIN', '')
GH = os.environ.get('GITHUB_ACTIONS') == 'true'


def note(level, title, text=''):
    if GH:
        t = str(text)[:900].replace('%', '%25').replace('\r', '').replace('\n', '%0A')
        print('::%s title=%s::%s' % (level, title.replace(':', ' -').replace(',', ';'), t or '-'), flush=True)


def wait(fn, secs=15, every=0.2):
    end = time.time() + secs
    last = None
    while time.time() < end:
        try:
            last = fn()
            if last:
                return last
        except Exception as e:  # noqa: BLE001
            last = None
            err = e  # noqa: F841
        time.sleep(every)
    return None


FAKE_PLAYERCTL = r'''#!/bin/sh
# A stand-in playerctl: one player, "fakeplayer", playing; every call logged.
echo "$*" >> "$FAKE_LOG/playerctl.log"
case "$*" in
  "-l") echo fakeplayer ;;
  *status*) cat "$FAKE_LOG/player-status" 2>/dev/null || echo Playing ;;
  *pause*) echo Paused > "$FAKE_LOG/player-status" ;;
  *" play") echo Playing > "$FAKE_LOG/player-status" ;;
  *metadata*) printf 'Playing\037A song\037An artist\037An album\0371000000\0372000000\0370.5' ;;
esac
exit 0
'''
FAKE_WPCTL = r'''#!/bin/sh
echo "$*" >> "$FAKE_LOG/wpctl.log"
case "$1" in
  get-volume) echo "Volume: $(cat "$FAKE_LOG/vol" 2>/dev/null || echo 0.40)" ;;
  set-volume) echo "$3" > "$FAKE_LOG/vol" ;;
esac
exit 0
'''
FAKE_SSHFS = r'''#!/bin/sh
# A stand-in sshfs: what it was given, and the password it read on stdin; "mounts" by leaving a mark.
read -r pw
printf '%s\n' "$*" > "$FAKE_LOG/sshfs-args"
printf '%s\n' "$pw" > "$FAKE_LOG/sshfs-password"
mkdir -p "$2" && touch "$2/.mounted"
exit 0
'''


class NotifServer:
    """A stand-in org.freedesktop.Notifications (Gio): records Notify calls, emits the signals Mantel would."""
    XML = '''<node><interface name="org.freedesktop.Notifications">
      <method name="Notify"><arg type="s" direction="in"/><arg type="u" direction="in"/><arg type="s" direction="in"/>
        <arg type="s" direction="in"/><arg type="s" direction="in"/><arg type="as" direction="in"/><arg type="a{sv}" direction="in"/>
        <arg type="i" direction="in"/><arg type="u" direction="out"/></method>
      <method name="CloseNotification"><arg type="u" direction="in"/></method>
      <method name="GetCapabilities"><arg type="as" direction="out"/></method>
      <method name="GetServerInformation"><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/><arg type="s" direction="out"/></method>
      <signal name="ActionInvoked"><arg type="u"/><arg type="s"/></signal>
      <signal name="NotificationReplied"><arg type="u"/><arg type="s"/></signal>
      <signal name="NotificationClosed"><arg type="u"/><arg type="u"/></signal>
    </interface></node>'''

    def __init__(self, address):
        from gi.repository import Gio, GLib
        self.Gio, self.GLib = Gio, GLib
        self.address = address
        self.calls = []
        self.next = 100
        ready = threading.Event()
        threading.Thread(target=self._run, args=(ready,), daemon=True).start()
        ready.wait(10)

    def _run(self, ready):
        Gio, GLib = self.Gio, self.GLib
        ctx = GLib.MainContext.new()
        ctx.push_thread_default()
        self.conn = Gio.DBusConnection.new_for_address_sync(self.address, Gio.DBusConnectionFlags.AUTHENTICATION_CLIENT | Gio.DBusConnectionFlags.MESSAGE_BUS_CONNECTION, None, None)
        node = Gio.DBusNodeInfo.new_for_xml(self.XML)
        self.conn.register_object('/org/freedesktop/Notifications', node.interfaces[0], self._method, None, None)
        self.conn.call_sync('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'RequestName',
                            GLib.Variant('(su)', ('org.freedesktop.Notifications', 4)), None, Gio.DBusCallFlags.NONE, -1, None)
        loop = GLib.MainLoop.new(ctx, False)
        ready.set()
        loop.run()

    def _method(self, conn, sender, path, iface, method, params, inv):
        GLib = self.GLib
        if method == 'Notify':
            app, replaces, icon, summary, body, actions, hints, timeout = params.unpack()
            nid = replaces or self.next
            if not replaces:
                self.next += 1
            self.calls.append({'id': nid, 'summary': summary, 'body': body, 'actions': list(actions), 'hints': dict(hints)})
            inv.return_value(GLib.Variant('(u)', (nid,)))
        elif method == 'CloseNotification':
            (nid,) = params.unpack()
            inv.return_value(None)
            self.signal('NotificationClosed', '(uu)', (nid, 3))
        elif method == 'GetCapabilities':
            inv.return_value(GLib.Variant('(as)', (['body', 'actions', 'inline-reply'],)))
        else:
            inv.return_value(GLib.Variant('(ssss)', ('fake', 'hearth', '1', '1.2')))

    def signal(self, name, sig, args):
        self.conn.emit_signal(None, '/org/freedesktop/Notifications', 'org.freedesktop.Notifications', name, self.GLib.Variant(sig, args))

    def find(self, pred):
        return next((c for c in reversed(self.calls) if pred(c)), None)


@unittest.skipUnless(BIN and os.path.exists(BIN), 'HEARTH_CONNECT_BIN (a built hearth-connect) is needed')
class PhoneLink(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        c = cls
        c.tmp = tempfile.mkdtemp(prefix='phone-link-')
        c.home, c.dest, c.fake = (os.path.join(c.tmp, x) for x in ('home', 'recv', 'fake'))
        c.bin = os.path.join(c.tmp, 'bin')
        for d in (c.home, c.dest, c.fake, c.bin):
            os.makedirs(d)
        for name, text in (('playerctl', FAKE_PLAYERCTL), ('wpctl', FAKE_WPCTL), ('sshfs', FAKE_SSHFS)):
            p = os.path.join(c.bin, name)
            with open(p, 'w') as f:
                f.write(text)
            os.chmod(p, 0o755)
        c.env = dict(os.environ, HEARTH_CONNECT_HOME=c.home, HEARTH_CONNECT_DEST=c.dest, HEARTH_CONNECT_SOCKET=os.path.join(c.tmp, 'run', 'connect.sock'),
                     HEARTH_CONNECT_TEST='1', HEARTH_CONNECT_FAKE_BATTERY='57', FAKE_LOG=c.fake, PATH=c.bin + ':' + os.environ.get('PATH', ''),
                     XDG_CONFIG_HOME=os.path.join(c.tmp, 'config'), HEARTH_CONNECT_BIN=BIN)
        os.environ.update({k: c.env[k] for k in ('HEARTH_CONNECT_SOCKET', 'HEARTH_CONNECT_HOME', 'HEARTH_CONNECT_BIN')})
        # A private session bus, and the stand-in notification service on it.
        c.bus = subprocess.Popen(['dbus-daemon', '--session', '--nofork', '--print-address=1', '--address=unix:path=' + os.path.join(c.tmp, 'bus')],
                                 stdout=subprocess.PIPE, text=True)
        addr = c.bus.stdout.readline().strip()
        c.env['DBUS_SESSION_BUS_ADDRESS'] = addr
        c.notif = None
        try:
            c.notif = NotifServer(addr)
        except Exception as e:  # noqa: BLE001
            note('warning', 'no stand-in notification service', e)
        c.svc = subprocess.Popen([BIN, '--daemon'], env=c.env, stdout=open(os.path.join(c.tmp, 'daemon.log'), 'w'), stderr=subprocess.STDOUT)
        assert wait(lambda: os.path.exists(c.env['HEARTH_CONNECT_SOCKET']), 15), 'the service did not start'
        c.events = []
        c.ev = subprocess.Popen([BIN, '--events'], env=c.env, stdout=subprocess.PIPE, text=True)
        threading.Thread(target=lambda: [c.events.append(json.loads(l)) for l in c.ev.stdout if l.strip().startswith('{')], daemon=True).start()
        c.phone = fake_phone.Phone('Pixel 8', folder=os.path.join(c.tmp, 'phone'))

    @classmethod
    def tearDownClass(cls):
        for p in ('ev', 'svc', 'bus'):
            pr = getattr(cls, p, None)
            if pr and pr.poll() is None:
                pr.terminate()
                try:
                    pr.wait(5)
                except subprocess.TimeoutExpired:
                    pr.kill()
        log = os.path.join(cls.tmp, 'daemon.log')
        if os.path.exists(log):
            note('notice', 'service log', open(log).read()[-1500:])

    # ---- helpers
    def call(self, cmd, args=None):
        p = subprocess.run([BIN, '--call', cmd, json.dumps(args or {})], capture_output=True, text=True, timeout=30, env=self.env)
        if p.returncode != 0:
            raise RuntimeError(p.stderr.strip())
        return json.loads(p.stdout)

    def dev(self):
        return next((d for d in self.call('state')['devices'] if d['id'] == self.phone.id), None)

    def event(self, name, pred=lambda d: True, secs=15):
        return wait(lambda: next((e for e in self.events if e.get('event') == name and pred(e.get('data') or {})), None), secs)

    def got(self, kind, pred=None, secs=15):
        p = self.phone.wait(kind, pred, timeout=secs)
        self.assertIsNotNone(p, 'the phone never got ' + kind)
        return p

    def need_pair(self):
        if not (self.dev() or {}).get('paired'):
            self.skipTest('not paired (an earlier step failed)')

    # ---- the steps, in order
    def test_01_discovery_and_tls(self):
        def attempt():
            self.phone.announce()
            try:
                return self.phone.link(timeout=3)
            except OSError:
                return None
        peer = wait(attempt, 30, 0.1)
        self.assertTrue(peer, "Hearth didn't connect to the phone after its announcement")
        self.assertEqual(self.phone.first_identity['body'].get('targetDeviceId'), self.phone.id)
        self.assertEqual(peer['body']['protocolVersion'], 8)
        for k in ('kdeconnect.sms.messages', 'kdeconnect.telephony', 'kdeconnect.runcommand.request', 'kdeconnect.sftp', 'kdeconnect.presenter',
                  'kdeconnect.contacts.response_vcards', 'kdeconnect.systemvolume.request', 'kdeconnect.photo'):
            self.assertIn(k, peer['body']['incomingCapabilities'])
        self.assertTrue(wait(lambda: (self.dev() or {}).get('reachable')), 'Hearth lists the phone as reachable')
        note('notice', 'discovery', 'linked over TLS; Hearth ' + peer['body']['deviceId'])

    def test_02_pairing_with_the_same_key(self):
        ts = self.phone.pair()
        d = wait(lambda: (lambda x: x if x and x['pair'].get('state') == 'asking' else None)(self.dev()))
        self.assertTrue(d, 'Hearth shows the pairing request')
        mine = fake_phone.verification_key(self.phone.cert_der, self.phone.pc_cert, ts)
        self.assertEqual(d['pair']['key'], mine, 'both compute the same verification key')
        self.assertFalse(d['paired'], 'nothing is paired before the person accepts')
        self.call('accept', {'id': self.phone.id})
        self.got('kdeconnect.pair', lambda b: b.get('pair') is True)
        self.assertTrue(self.dev()['paired'])
        paired = json.load(open(os.path.join(self.home, 'paired.json')))
        self.assertEqual(paired[0]['id'], self.phone.id)
        self.assertTrue(paired[0]['pauseOnCall'] and paired[0]['commands'])
        note('notice', 'pairing', 'key ' + mine)

    def test_03_what_a_paired_phone_hears_first(self):
        self.need_pair()
        self.assertEqual(self.got('kdeconnect.battery')['body']['currentCharge'], 57)
        for k in ('kdeconnect.mousepad.keyboardstate', 'kdeconnect.notification.request', 'kdeconnect.mpris.request', 'kdeconnect.runcommand',
                  'kdeconnect.sms.request_conversations', 'kdeconnect.contacts.request_all_uids_timestamps', 'kdeconnect.connectivity_report.request'):
            self.got(k)

    def test_04_ping_both_ways(self):
        self.need_pair()
        self.phone.send('kdeconnect.ping', {'message': 'hi from the phone'})
        self.assertTrue(wait(lambda: any(e['event'] == 'ping' and e['data'].get('message') == 'hi from the phone' for e in self.call('state')['log'])))
        self.call('ping', {'id': self.phone.id})
        self.got('kdeconnect.ping')

    def test_05_battery_and_signal(self):
        self.need_pair()
        self.phone.send('kdeconnect.battery', {'currentCharge': 81, 'isCharging': True, 'thresholdEvent': 0})
        self.phone.send('kdeconnect.connectivity_report', {'signalStrengths': {'1': {'networkType': '5G', 'signalStrength': 3}}})
        d = wait(lambda: (lambda x: x if x['battery'] and x['battery']['charge'] == 81 and x['plugins']['signal'] else None)(self.dev()))
        self.assertTrue(d)
        self.assertTrue(d['battery']['charging'])
        self.assertEqual(d['plugins']['signal']['1']['networkType'], '5G')

    def test_06_notifications_buttons_and_reply(self):
        self.need_pair()
        self.phone.send('kdeconnect.notification', {'id': 'n1', 'appName': 'Messages', 'title': 'Ada', 'text': 'Lunch?', 'isClearable': True,
                                                     'requestReplyId': 'r1', 'actions': ['Mark as read'], 'time': str(fake_phone.now_ms())})
        d = wait(lambda: (lambda x: x if x['notifications'] else None)(self.dev()))
        self.assertTrue(d)
        self.assertEqual(d['notifications'][0]['replyId'], 'r1')
        # Reply from Settings › Phone (the service's "reply").
        self.call('reply', {'id': self.phone.id, 'replyId': 'r1', 'message': 'Yes!'})
        self.assertEqual(self.got('kdeconnect.notification.reply')['body'], {'requestReplyId': 'r1', 'message': 'Yes!'})
        if not self.notif:
            note('warning', 'notification pop-up', 'python3-gi missing: the pop-up path was not checked')
            return
        c = wait(lambda: self.notif.find(lambda c: c['summary'] == 'Messages: Ada'))
        self.assertTrue(c, 'the notification reached the desktop: %s' % self.notif.calls[-3:])
        self.assertIn('inline-reply', c['actions'])
        self.assertIn('phone:Mark as read', c['actions'])
        self.assertIn('x-kde-reply-placeholder-text', c['hints'])
        # Mantel's Reply box, its buttons, and × (dismissed on the phone too).
        self.notif.signal('NotificationReplied', '(us)', (c['id'], 'On my way'))
        self.assertEqual(self.got('kdeconnect.notification.reply', lambda b: b.get('message') == 'On my way')['body']['requestReplyId'], 'r1')
        self.notif.signal('ActionInvoked', '(us)', (c['id'], 'phone:Mark as read'))
        self.assertEqual(self.got('kdeconnect.notification.action')['body'], {'key': 'n1', 'action': 'Mark as read'})
        self.notif.signal('NotificationClosed', '(uu)', (c['id'], 2))
        self.assertEqual(self.got('kdeconnect.notification.request', lambda b: 'cancel' in b)['body']['cancel'], 'n1')
        # The phone cancels one: it goes here too.
        self.phone.send('kdeconnect.notification', {'id': 'n1', 'isCancel': True})
        self.assertTrue(wait(lambda: not self.dev()['notifications']))

    def test_07_clipboard_both_ways(self):
        self.need_pair()
        # Wayland (KWin, labwc) through wl-clipboard, X through xclip: what desktop.rs uses.
        if os.environ.get('WAYLAND_DISPLAY') and shutil.which('wl-paste'):
            get, put, how = ['wl-paste', '--no-newline'], ['wl-copy'], 'wl-clipboard on ' + os.environ.get('HEARTH_WM', 'Wayland')
        elif os.environ.get('DISPLAY') and shutil.which('xclip'):
            get, put, how = ['xclip', '-selection', 'clipboard', '-o'], ['xclip', '-selection', 'clipboard', '-i'], 'xclip'
        else:
            note('warning', 'clipboard', 'no display here: only the packets were checked')
            self.phone.send('kdeconnect.clipboard', {'content': 'from the phone'})
            self.assertTrue(self.event('clipboard'))
            return
        self.phone.send('kdeconnect.clipboard', {'content': 'from the phone'})
        self.assertTrue(wait(lambda: subprocess.run(get, capture_output=True, text=True, timeout=5).stdout == 'from the phone'), 'phone → PC (' + how + ')')
        p = subprocess.Popen(put, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        p.stdin.write(b'from the PC')
        p.stdin.close()
        self.got('kdeconnect.clipboard', lambda b: b.get('content') == 'from the PC', 15)
        note('notice', 'clipboard', 'both ways through ' + how)

    def test_08_files_text_links_from_the_phone(self):
        self.need_pair()
        data = os.urandom(300 * 1024 + 7)
        self.phone.send('kdeconnect.share.request', {'filename': 'IMG_2041.jpg', 'numberOfFiles': 1, 'totalPayloadSize': len(data)}, payload=data)
        f = wait(lambda: next((x for x in self.call('state')['files'] if x['name'] == 'IMG_2041.jpg' and x['done']), None))
        self.assertTrue(f, 'the file was offered and fetched')
        self.assertFalse(os.path.exists(os.path.join(self.dest, 'IMG_2041.jpg')), 'not saved before accepting')
        self.call('file', {'token': f['token'], 'accept': True})
        self.assertTrue(wait(lambda: os.path.exists(os.path.join(self.dest, 'IMG_2041.jpg'))))
        self.assertEqual(open(os.path.join(self.dest, 'IMG_2041.jpg'), 'rb').read(), data)
        self.phone.send('kdeconnect.share.request', {'text': 'shared text'})
        self.phone.send('kdeconnect.share.request', {'url': 'https://hearthos.fun'})
        self.assertTrue(wait(lambda: {'shared text', 'https://hearthos.fun'} <= {t.get('text') or t.get('url') for t in self.call('state')['texts']}))

    def test_09_files_to_the_phone(self):
        self.need_pair()
        src = os.path.join(self.tmp, 'to-phone.bin')
        data = os.urandom(200 * 1024 + 3)
        open(src, 'wb').write(data)
        self.call('share', {'id': self.phone.id, 'paths': [src]})
        p = self.got('kdeconnect.share.request', lambda b: b.get('filename') == 'to-phone.bin')
        self.assertEqual(self.phone.fetch(p), data)
        self.assertTrue(self.event('sent'))
        self.call('text', {'id': self.phone.id, 'text': 'hello phone'})
        self.got('kdeconnect.share.request', lambda b: b.get('text') == 'hello phone')
        self.call('url', {'id': self.phone.id, 'url': 'https://example.com'})
        self.got('kdeconnect.share.request', lambda b: b.get('url') == 'https://example.com')

    def test_10_find_my_phone_and_pc(self):
        self.need_pair()
        self.call('ring', {'id': self.phone.id})
        self.got('kdeconnect.findmyphone.request')
        self.phone.send('kdeconnect.findmyphone.request', {})
        self.assertTrue(wait(lambda: any(e['event'] == 'ring' for e in self.call('state')['log'])))

    def test_11_media_both_ways(self):
        self.need_pair()
        self.phone.send('kdeconnect.mpris.request', {'requestPlayerList': True})
        self.assertIn('Fakeplayer', self.got('kdeconnect.mpris', lambda b: 'playerList' in b)['body']['playerList'])
        self.phone.send('kdeconnect.mpris.request', {'player': 'Fakeplayer', 'action': 'Next'})
        self.assertTrue(wait(lambda: '-p fakeplayer next' in open(os.path.join(self.fake, 'playerctl.log')).read()))
        self.phone.send('kdeconnect.mpris', {'playerList': ['Spotify']})
        self.got('kdeconnect.mpris.request', lambda b: b.get('player') == 'Spotify' and b.get('requestNowPlaying'))
        self.phone.send('kdeconnect.mpris', {'player': 'Spotify', 'title': 'Clair de Lune', 'isPlaying': True})
        self.assertTrue(wait(lambda: self.dev()['playing'].get('Spotify', {}).get('title') == 'Clair de Lune'))
        self.call('media', {'id': self.phone.id, 'player': 'Spotify', 'action': 'PlayPause'})
        self.got('kdeconnect.mpris.request', lambda b: b.get('action') == 'PlayPause')

    def test_12_remote_input(self):
        self.need_pair()
        self.phone.send('kdeconnect.mousepad.request', {'dx': 4, 'dy': 3, 'sendAck': True})
        self.assertTrue(self.got('kdeconnect.mousepad.echo')['body'].get('isAck'))
        st = self.call('state')
        note('notice', 'remote input', 'virtual mouse ready: %s %s' % (st.get('inputReady'), st.get('inputError') or ''))

    def test_13_run_commands(self):
        self.need_pair()
        mark = os.path.join(self.tmp, 'command-ran')
        r = self.call('command_add', {'name': 'Leave a mark', 'command': 'touch ' + mark})
        p = self.got('kdeconnect.runcommand', lambda b: 'Leave a mark' in b.get('commandList', ''))
        self.assertIn(r['key'], json.loads(p['body']['commandList']))
        self.phone.send('kdeconnect.runcommand.request', {'key': r['key']})
        self.assertTrue(wait(lambda: os.path.exists(mark)), 'the command ran')
        # Off for this phone: it sees none, and can't run one.
        os.remove(mark)
        self.call('option', {'id': self.phone.id, 'key': 'commands', 'on': False})
        self.got('kdeconnect.runcommand', lambda b: b.get('commandList') == '{}')
        self.phone.send('kdeconnect.runcommand.request', {'key': r['key']})
        time.sleep(1.5)
        self.assertFalse(os.path.exists(mark), 'a phone with Run commands off ran a command')
        self.call('option', {'id': self.phone.id, 'key': 'commands', 'on': True})
        self.phone.send('kdeconnect.runcommand.request', {'requestCommandList': True})
        self.got('kdeconnect.runcommand', lambda b: 'Leave a mark' in b.get('commandList', ''))
        self.call('command_remove', {'key': r['key']})
        self.assertEqual(self.call('commands'), [])

    def test_14_contacts(self):
        self.need_pair()
        self.phone.send('kdeconnect.contacts.response_uids_timestamps', {'uids': ['c1'], 'c1': 123})
        self.assertEqual(self.got('kdeconnect.contacts.request_vcards_by_uid')['body']['uids'], ['c1'])
        self.phone.send('kdeconnect.contacts.response_vcards', {'uids': ['c1'], 'c1': 'BEGIN:VCARD\nVERSION:2.1\nFN:Ada Lovelace\nTEL;CELL:+1 555 0100\nEND:VCARD'})
        people = wait(lambda: self.call('contacts', {'id': self.phone.id}))
        self.assertEqual(people[0]['name'], 'Ada Lovelace')

    def test_15_calls_pause_and_resume_media(self):
        self.need_pair()
        open(os.path.join(self.fake, 'player-status'), 'w').write('Playing\n')
        open(os.path.join(self.fake, 'playerctl.log'), 'a').close()
        self.phone.send('kdeconnect.telephony', {'event': 'ringing', 'phoneNumber': '+15550100'})
        self.assertTrue(wait(lambda: '-p fakeplayer pause' in open(os.path.join(self.fake, 'playerctl.log')).read()), 'the player was paused')
        d = wait(lambda: (lambda x: x if x['plugins']['call'] else None)(self.dev()))
        self.assertEqual(d['plugins']['call']['state'], 'ringing')
        self.assertEqual(d['plugins']['call']['who'], 'Ada Lovelace', 'named from the contacts')
        if self.notif:
            c = wait(lambda: self.notif.find(lambda c: c['summary'] == 'Ada Lovelace is calling'))
            self.assertTrue(c)
            self.assertIn('mute', c['actions'])
            self.notif.signal('ActionInvoked', '(us)', (c['id'], 'mute'))
            self.got('kdeconnect.telephony.request_mute')
        self.call('mute_call', {'id': self.phone.id})
        self.got('kdeconnect.telephony.request_mute')
        self.phone.send('kdeconnect.telephony', {'event': 'talking', 'phoneNumber': '+15550100', 'isCancel': True})
        self.assertTrue(wait(lambda: open(os.path.join(self.fake, 'playerctl.log')).read().rstrip().endswith('-p fakeplayer play')), 'the player plays again')
        self.assertTrue(wait(lambda: self.dev()['plugins']['call'] is None))

    def test_16_text_messages(self):
        self.need_pair()
        t0 = fake_phone.now_ms()
        self.phone.send('kdeconnect.sms.messages', {'version': 2, 'messages': [
            {'event': 1, 'body': 'Lunch?', 'addresses': [{'address': '+15550100'}], 'date': t0 - 5000, 'type': 1, 'read': 0, 'thread_id': 7, '_id': 1},
            {'event': 1, 'body': 'Sure', 'addresses': [{'address': '+15550100'}], 'date': t0 - 1000, 'type': 2, 'read': 1, 'thread_id': 7, '_id': 2},
            {'event': 1, 'body': 'Old one', 'addresses': [{'address': '+15550142'}], 'date': t0 - 900000, 'type': 1, 'read': 1, 'thread_id': 9, '_id': 3},
        ]})
        th = wait(lambda: (lambda x: x if len(x) == 2 else None)(self.call('sms_threads', {'id': self.phone.id})))
        self.assertTrue(th)
        self.assertEqual(th[0]['thread'], 7)
        self.assertEqual(th[0]['last'], 'Sure')
        self.assertEqual(th[0]['names'], ['Ada Lovelace'])
        msgs = self.call('sms_thread', {'id': self.phone.id, 'thread': 7})
        self.assertEqual([m['body'] for m in msgs], ['Lunch?', 'Sure'])
        self.assertEqual(self.got('kdeconnect.sms.request_conversation')['body']['threadID'], 7)
        self.call('sms_send', {'id': self.phone.id, 'to': '+15550100', 'text': 'On my way'})
        b = self.got('kdeconnect.sms.request')['body']
        self.assertEqual(b['addresses'], [{'address': '+15550100'}])
        self.assertEqual(b['messageBody'], 'On my way')
        self.call('sms_send', {'id': self.phone.id, 'thread': 9, 'text': 'Thanks'})
        self.assertEqual(self.got('kdeconnect.sms.request', lambda b: b.get('messageBody') == 'Thanks')['body']['addresses'], [{'address': '+15550142'}])

    def test_17_presentation_remote(self):
        self.need_pair()
        self.phone.send('kdeconnect.presenter', {'dx': 0.02, 'dy': -0.01})
        self.assertTrue(self.event('presenter', lambda d: d.get('moving')))
        self.phone.send('kdeconnect.presenter', {'stop': True})
        self.assertTrue(self.event('presenter', lambda d: d.get('stop')))
        # Next slide: a special key on the mousepad packet (Page Down is 12... KDE's "right" is 6).
        self.phone.send('kdeconnect.mousepad.request', {'specialKey': 6, 'sendAck': True})
        self.got('kdeconnect.mousepad.echo', lambda b: b.get('specialKey') == 6)

    def test_18_this_pcs_volume(self):
        self.need_pair()
        self.phone.send('kdeconnect.systemvolume.request', {'requestSinks': True})
        s = self.got('kdeconnect.systemvolume', lambda b: 'sinkList' in b)['body']['sinkList'][0]
        self.assertEqual((s['volume'], s['muted']), (40, False))
        self.phone.send('kdeconnect.systemvolume.request', {'name': s['name'], 'volume': 70})
        self.assertEqual(self.got('kdeconnect.systemvolume', lambda b: b['sinkList'][0]['volume'] == 70)['body']['sinkList'][0]['volume'], 70)
        self.assertIn('set-volume @DEFAULT_AUDIO_SINK@ 0.70', open(os.path.join(self.fake, 'wpctl.log')).read())

    def test_19_browse_the_phones_files(self):
        self.need_pair()
        self.call('browse', {'id': self.phone.id})
        self.got('kdeconnect.sftp.request', lambda b: b.get('startBrowsing'))
        self.phone.send('kdeconnect.sftp', {'ip': '127.0.0.1', 'port': 1740, 'user': 'kdeconnect', 'password': 'one-time-secret', 'path': '/',
                                             'multiPaths': ['/storage/emulated/0', '/storage/1234-5678'], 'pathNames': ['Phone', 'SD card']})
        sf = wait(lambda: (lambda x: x if x['plugins']['sftp']['mounted'] else None)(self.dev()))
        self.assertTrue(sf, 'mounted: %s' % (self.dev() or {}).get('plugins'))
        m = sf['plugins']['sftp']
        self.assertTrue(os.path.exists(os.path.join(m['mounted'], '.mounted')))
        self.assertEqual([f['name'] for f in m['folders']], ['Phone', 'SD card'])
        args = open(os.path.join(self.fake, 'sshfs-args')).read()
        self.assertIn('kdeconnect@127.0.0.1:/', args)
        self.assertIn('password_stdin', args)
        self.assertNotIn('one-time-secret', args, 'the password went on the command line')
        self.assertEqual(open(os.path.join(self.fake, 'sshfs-password')).read().strip(), 'one-time-secret')
        self.call('unmount', {'id': self.phone.id})
        self.assertIsNone(self.dev()['plugins']['sftp']['mounted'])

    def test_20_take_a_photo(self):
        self.need_pair()
        self.call('photo', {'id': self.phone.id})
        self.got('kdeconnect.photo.request')
        data = os.urandom(50 * 1024)
        self.phone.send('kdeconnect.photo', {'filename': 'PXL_photo.jpg'}, payload=data)
        self.assertTrue(wait(lambda: os.path.exists(os.path.join(self.dest, 'PXL_photo.jpg'))), 'kept without asking (you asked for it)')
        self.assertEqual(open(os.path.join(self.dest, 'PXL_photo.jpg'), 'rb').read(), data)

    def test_21_unpair_from_the_phone(self):
        self.need_pair()
        self.phone.send('kdeconnect.pair', {'pair': False})
        self.assertTrue(wait(lambda: not self.dev()['paired']))
        self.assertEqual(json.load(open(os.path.join(self.home, 'paired.json'))), [])
        self.assertTrue(os.path.exists(os.path.join(self.home, 'paired.json.bak')))

    def test_22_phone_link_off_and_on_keeps_settings(self):
        self.call('rename', {'name': 'Hearth Test PC'})
        self.call('add_ip', {'ip': '192.168.1.23'})
        sp = os.path.join(self.home, 'settings.json')
        before = json.load(open(sp))
        self.assertEqual(before['name'], 'Hearth Test PC')
        off = subprocess.run([BIN, '--turn-off'], env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(off.returncode, 0, off.stderr)
        self.assertTrue(wait(lambda: self.svc.poll() is not None, 15), 'the service stopped')
        after = json.load(open(sp))
        self.assertIs(after['enabled'], False)
        self.assertEqual((after['name'], after['manual']), (before['name'], before['manual']), 'off kept every other setting')
        self.assertEqual(subprocess.run([BIN, '--autostart'], env=self.env, timeout=30).returncode, 0)
        time.sleep(1)
        self.assertFalse(os.path.exists(self.env['HEARTH_CONNECT_SOCKET']), "sign-in doesn't start it while it's off")
        on = subprocess.run([BIN, '--turn-on'], env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(on.returncode, 0, on.stderr)
        st = self.call('state')
        self.assertEqual(st['name'], 'Hearth Test PC')
        self.assertEqual(st['manual'], ['192.168.1.23'])
        on2 = json.load(open(sp))
        self.assertIs(on2['enabled'], True)
        self.assertTrue(os.path.exists(sp + '.bak'))
        # And the settings helper Mantel uses says the same.
        helper = os.path.join(os.path.dirname(__file__), '..', '..', 'linux', 'desktop', 'shell', 'hearth-phone')
        h = subprocess.run([sys.executable, helper, 'tile'], env=self.env, capture_output=True, text=True, timeout=30)
        tile = json.loads(h.stdout.strip().splitlines()[-1])
        self.assertTrue(tile['enabled'] and tile['running'], h.stdout + h.stderr)
        self.call('quit')


if __name__ == '__main__':
    unittest.main(verbosity=2)
