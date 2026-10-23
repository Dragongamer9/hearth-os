#!/usr/bin/env python3
"""Hearth Connect against the real KDE Connect (kdeconnectd from Fedora, in a container on Docker's network), on a CI
runner. Both are told nothing about each other: they find each other by UDP broadcast, link over TLS, pair (Hearth
asks, KDE Connect accepts), then: pings, files both ways, a declined file, shared text, battery, find my PC, the
phone-as-touchpad packets, notifications, and unpairing. Prints a summary; exits 1 if a required check failed.

Needs: the container `kde` running kdeconnectd with its session bus at unix:path=/tmp/bus (connect.yml starts it),
/kc mounted from KC_DIR, and Hearth Connect's service running with HEARTH_CONNECT_SOCKET/HEARTH_CONNECT_DEST set.
"""
import hashlib
import json
import os
import subprocess
import sys
import time

BIN = os.environ['HEARTH_CONNECT_BIN']
DEST = os.environ['HEARTH_CONNECT_DEST']
KC = os.environ['KC_DIR']          # host side of the container's /kc
BUS = 'DBUS_SESSION_BUS_ADDRESS=unix:path=/tmp/bus'
results = []


def ok(name, cond, detail='', required=True):
    results.append((name, bool(cond), required, detail))
    level = 'notice' if cond else ('error' if required else 'warning')
    msg = (str(detail)[:900] if detail else '').replace('%', '%25').replace('\r', '').replace('\n', '%0A')
    print('::%s title=%s::%s' % (level, ('ok: ' if cond else 'FAILED: ' if required else 'warn: ') + name.replace(':', ' -').replace(',', ';'), msg or '-'), flush=True)
    print(('PASS ' if cond else ('FAIL ' if required else 'WARN ')) + name + (' :: ' + detail if detail else ''), flush=True)
    return cond


def kde(*args, check=False, timeout=60):
    cmd = ['docker', 'exec', 'kde', 'env', BUS, 'QT_QPA_PLATFORM=offscreen'] + list(args)
    p = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
    if check and p.returncode != 0:
        print('kde command failed:', args, p.stdout, p.stderr)
    return p


def hearth(cmd, args=None):
    p = subprocess.run([BIN, '--call', cmd, json.dumps(args or {})], capture_output=True, text=True, timeout=60)
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip())
    return json.loads(p.stdout)


def wait(what, fn, secs=40, every=1.0):
    end = time.time() + secs
    last = None
    while time.time() < end:
        try:
            last = fn()
            if last:
                return last
        except Exception as e:  # noqa: BLE001
            last = e
        time.sleep(every)
    print('timed out waiting for', what, '; last:', last)
    return None


def dev(state, kid):
    return next((d for d in state['devices'] if d['id'] == kid), None)


def sha(path):
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for b in iter(lambda: f.read(1 << 20), b''):
            h.update(b)
    return h.hexdigest()


def gdbus_get(path, iface, prop):
    p = kde('gdbus', 'call', '--session', '--dest', 'org.kde.kdeconnect', '--object-path', path,
            '--method', 'org.freedesktop.DBus.Properties.Get', iface, prop)
    return p.stdout.strip()


def main():
    kid = kde('kdeconnect-cli', '--my-id').stdout.strip()
    me = hearth('state')
    hid = me['deviceId']
    print('KDE Connect device:', kid, ' Hearth device:', hid)
    ok('kdeconnectd is running', len(kid) >= 32, kid)

    # Discovery and the TLS link, both ways round: Hearth sees KDE Connect, KDE Connect sees Hearth.
    kde('kdeconnect-cli', '--refresh')
    linked = wait('Hearth to link with KDE Connect', lambda: (dev(hearth('state'), kid) or {}).get('reachable'))
    ok('found each other and linked over TLS (protocol 8)', linked)
    seen = wait('KDE Connect to list Hearth', lambda: hid in kde('kdeconnect-cli', '-l', '--id-only').stdout)
    ok('KDE Connect lists Hearth as available', seen)
    if not linked:
        return

    # Pairing: Hearth asks, KDE Connect accepts (kdeconnect-cli --pair accepts a pending request).
    r = hearth('pair', {'id': kid})
    ok('pairing request sent, with a verification key', len(r.get('key', '')) == 8, r.get('key', ''))
    time.sleep(1)
    kde('kdeconnect-cli', '-d', hid, '--pair')
    paired = wait('pairing', lambda: (dev(hearth('state'), kid) or {}).get('paired'))
    ok('paired (Hearth side)', paired)
    ok('paired (KDE Connect side)', wait('KDE paired', lambda: hid in kde('kdeconnect-cli', '-a', '--id-only').stdout, 20))
    if not paired:
        return
    time.sleep(2)

    # Ping, KDE Connect -> Hearth.
    kde('kdeconnect-cli', '-d', hid, '--ping-msg', 'hello hearth')
    ok('ping from KDE Connect arrives', wait('ping', lambda: any(e['event'] == 'ping' and e['data'].get('message') == 'hello hearth' for e in hearth('state')['log']), 20))
    ok('ping to KDE Connect is sent', hearth('ping', {'id': kid}) == {})

    # A file, KDE Connect -> Hearth: waits hidden until accepted, then saved.
    up = os.path.join(KC, 'from-kde.bin')
    with open(up, 'wb') as f:
        f.write(os.urandom(3 * 1024 * 1024 + 17))
    kde('kdeconnect-cli', '-d', hid, '--share', '/kc/from-kde.bin')
    offer = wait('file offer', lambda: next((f for f in hearth('state')['files'] if f['name'] == 'from-kde.bin'), None), 30)
    ok('file offered and held for the person (not saved yet)', offer and not os.path.exists(os.path.join(DEST, 'from-kde.bin')), json.dumps(offer))
    if offer:
        wait('download', lambda: next((f for f in hearth('state')['files'] if f['token'] == offer['token'] and f['done']), None), 30)
        hearth('file', {'token': offer['token'], 'accept': True})
        saved = wait('saved file', lambda: os.path.exists(os.path.join(DEST, 'from-kde.bin')), 20)
        ok('file from KDE Connect saved after accepting, identical', saved and sha(os.path.join(DEST, 'from-kde.bin')) == sha(up))

    # A declined file is thrown away.
    with open(os.path.join(KC, 'declined.bin'), 'wb') as f:
        f.write(os.urandom(4096))
    kde('kdeconnect-cli', '-d', hid, '--share', '/kc/declined.bin')
    off2 = wait('second offer', lambda: next((f for f in hearth('state')['files'] if f['name'] == 'declined.bin'), None), 30)
    if off2:
        wait('download 2', lambda: next((f for f in hearth('state')['files'] if f['token'] == off2['token'] and f['done']), None), 20)
        hearth('file', {'token': off2['token'], 'accept': False})
        time.sleep(1)
        hidden = os.path.join(DEST, '.incoming')
        ok('declined file deleted, nothing saved', not os.path.exists(os.path.join(DEST, 'declined.bin')) and not any(n.startswith(off2['token']) for n in (os.listdir(hidden) if os.path.isdir(hidden) else [])))
    else:
        ok('declined file deleted, nothing saved', False, 'no offer')

    # Shared text.
    kde('kdeconnect-cli', '-d', hid, '--share-text', 'text from kde')
    ok('shared text arrives', wait('text', lambda: any(t.get('text') == 'text from kde' for t in hearth('state')['texts']), 20))

    # A file, Hearth -> KDE Connect (saved in its Downloads).
    out = os.path.join(os.environ.get('RUNNER_TEMP', '/tmp'), 'from-hearth.bin')
    with open(out, 'wb') as f:
        f.write(os.urandom(2 * 1024 * 1024 + 5))
    want = sha(out)
    hearth('share', {'id': kid, 'paths': [out]})
    got = wait('file at KDE Connect', lambda: want in kde('sh', '-c', 'sha256sum /root/Downloads/from-hearth*.bin 2>/dev/null').stdout, 40)
    ok('file from Hearth saved by KDE Connect, identical', got)

    # Battery (HEARTH_CONNECT_FAKE_BATTERY=57 on the Hearth side).
    b = wait('battery', lambda: '57' in gdbus_get('/modules/kdeconnect/devices/%s/battery' % hid, 'org.kde.kdeconnect.device.battery', 'charge') and 'yes', 20)
    ok('KDE Connect sees Hearth\'s battery (57%)', b, gdbus_get('/modules/kdeconnect/devices/%s/battery' % hid, 'org.kde.kdeconnect.device.battery', 'charge'))

    # Find my PC.
    kde('kdeconnect-cli', '-d', hid, '--ring')
    ok('find-my-PC request arrives', wait('ring', lambda: any(e['event'] == 'ring' for e in hearth('state')['log']), 20))

    # Remote input: KDE Connect's own remote control sends the same packets as the phone's touchpad.
    path = '/modules/kdeconnect/devices/%s/remotecontrol' % hid
    p = kde('gdbus', 'call', '--session', '--dest', 'org.kde.kdeconnect', '--object-path', path, '--method', 'org.kde.kdeconnect.device.remotecontrol.moveCursor', '(12, 7)')
    kde('gdbus', 'call', '--session', '--dest', 'org.kde.kdeconnect', '--object-path', path, '--method', 'org.kde.kdeconnect.device.remotecontrol.sendCommand', "{'singleclick': <true>}")
    time.sleep(2)
    st = hearth('state')
    devices = open('/proc/bus/input/devices').read() if os.path.exists('/proc/bus/input/devices') else ''
    ok('touchpad packets made the virtual mouse (uinput)', st.get('inputReady') and 'Hearth Connect phone touchpad' in devices,
       'inputReady=%s error=%s gdbus=%s' % (st.get('inputReady'), st.get('inputError'), (p.stdout + p.stderr).strip()[:200]), required=False)

    # Notifications: KDE Connect forwards notifications posted on its desktop (works without a notification server
    # only on some versions, so this one only warns).
    kde('notify-send', '-a', 'CI', 'Hello from KDE', 'notification body')
    ok('a notification from KDE Connect arrives', wait('notification', lambda: any(n.get('title') == 'Hello from KDE' or n.get('text') == 'notification body' for n in (dev(hearth('state'), kid) or {}).get('notifications', [])), 15), required=False)

    # Unpair from the KDE Connect side.
    kde('kdeconnect-cli', '-d', hid, '--unpair')
    ok('unpaired from KDE Connect: Hearth forgets it', wait('unpair', lambda: not (dev(hearth('state'), kid) or {}).get('paired'), 20))


if __name__ == '__main__':
    try:
        main()
    except Exception:  # noqa: BLE001
        import traceback
        traceback.print_exc()
        results.append(('the test script itself', False, True, 'crashed'))
    finally:
        failed = [r for r in results if not r[1] and r[2]]
        print('\n%d checks, %d failed' % (len(results), len(failed)))
        with open(os.environ.get('GITHUB_STEP_SUMMARY', '/dev/null'), 'a') as s:
            s.write('### Hearth Connect against KDE Connect (kdeconnectd)\n\n| Check | Result |\n|---|---|\n')
            for n, good, req, d in results:
                s.write('| %s | %s |\n' % (n, 'pass' if good else ('FAIL' if req else 'warn')))
    sys.exit(1 if [r for r in results if not r[1] and r[2]] or not results else 0)
