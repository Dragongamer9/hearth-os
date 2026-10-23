#!/usr/bin/env python3
"""Nearby Share on a CI runner, end to end:
  1. two Nearby Share instances: files (one big, a folder) and a text message from one to the other, a declined
     request, a PIN (refused without it, accepted with it), discovery by multicast
  2. against the official LocalSend CLI (LS_CLI=path, from LocalSend's releases or built from its source):
     LocalSend -> Nearby Share (file and text), and Nearby Share -> LocalSend (a file; the CLI saves files from devices
     it trusts without asking, and only trusts a sender that proves its certificate: so this also proves Hearth's
     client certificate (mTLS) works), and that each discovers the other.
Prints a summary (also to the job summary); exits 1 if a required check failed."""
import hashlib
import json
import os
import re
import shutil
import signal
import socket
import subprocess
import sys
import time

BIN = os.environ['NEARBY_BIN']
TMP = os.environ.get('RUNNER_TEMP', '/tmp')
LS = os.environ.get('LS_CLI', '')
results = []
procs = []


def ok(name, cond, detail='', required=True):
    results.append((name, bool(cond), required, detail))
    level = 'notice' if cond else ('error' if required else 'warning')
    msg = (str(detail)[:900] if detail else '').replace('%', '%25').replace('\r', '').replace('\n', '%0A')
    if not cond:
        print('::%s title=%s::%s' % (level, ('ok: ' if cond else 'FAILED: ' if required else 'warn: ') + name.replace(':', ' -').replace(',', ';'), msg or '-'), flush=True)
    print(('PASS ' if cond else ('FAIL ' if required else 'WARN ')) + name + (' :: ' + str(detail)[:400] if detail else ''), flush=True)
    return cond


def my_ip():
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(('10.255.255.255', 1))
        return s.getsockname()[0]
    finally:
        s.close()


def sha(p):
    return hashlib.sha256(open(p, 'rb').read()).hexdigest()


def env(home, dest=None, **extra):
    e = dict(os.environ, HEARTH_NEARBY_HOME=home, HEARTH_NEARBY_TEST='1')
    if dest:
        e['HEARTH_NEARBY_DEST'] = dest
    e.update(extra)
    return e


def serve(name, port, answer, pin=None):
    home, dest = os.path.join(TMP, 'nb-' + name), os.path.join(TMP, 'recv-' + name)
    os.makedirs(dest, exist_ok=True)
    log = open(os.path.join(TMP, name + '.log'), 'w')
    args = [BIN, '--serve', '--answer', answer, '--port', str(port), '--alias', 'Hearth ' + name]
    if pin:
        args += ['--pin', pin]
    p = subprocess.Popen(args, env=env(home, dest), stdout=log, stderr=subprocess.STDOUT)
    procs.append(p)
    time.sleep(1.5)
    return dest, os.path.join(TMP, name + '.log')


def send(to, *files, text=None, pin=None, home='sender'):
    args = [BIN, '--send-to', to]
    if pin:
        args += ['--pin', pin]
    if text is not None:
        args += ['--text', text]
    args += list(files)
    p = subprocess.run(args, env=env(os.path.join(TMP, 'nb-' + home)), capture_output=True, text=True, timeout=120)
    return p.returncode, p.stdout + p.stderr


def events(log):
    out = []
    for line in open(log):
        try:
            out.append(json.loads(line))
        except ValueError:
            pass
    return out


def two_instances(ip):
    src = os.path.join(TMP, 'src')
    os.makedirs(os.path.join(src, 'Album', 'Day 1'), exist_ok=True)
    big = os.path.join(src, 'big.bin')
    with open(big, 'wb') as f:
        f.write(os.urandom(48 * 1024 * 1024 + 123))
    with open(os.path.join(src, 'Album', 'Day 1', 'photo.jpg'), 'wb') as f:
        f.write(os.urandom(70000))
    dest, log = serve('A', 53317, 'accept')
    code, out = send(ip + ':53317', big, os.path.join(src, 'Album'))
    ok('Nearby Share -> Nearby Share: a 48 MB file and a folder', code == 0 and sha(big) == sha(os.path.join(dest, 'big.bin'))
       and sha(os.path.join(src, 'Album', 'Day 1', 'photo.jpg')) == sha(os.path.join(dest, 'Album', 'Day 1', 'photo.jpg')), out)
    code, out = send(ip + ':53317', big)
    ok('the same name again is kept as "big (1).bin" (nothing overwritten)', code == 0 and os.path.exists(os.path.join(dest, 'big (1).bin')), out)
    code, out = send(ip, text='hello from the other Hearth')
    time.sleep(0.5)
    ok('a text message is shown, not saved', code == 0 and any(e['event'] == 'message' and e['data']['text'] == 'hello from the other Hearth' for e in events(log)), out)
    ok('the sender proved its certificate to the receiver', any(e['event'] == 'message' and e['data'].get('verified') for e in events(log)))
    no = [n for n in os.listdir(dest) if n.endswith('.nearby-part') or n.startswith('.')]
    ok('no half-written files left', not no, no)

    destb, _ = serve('B', 53318, 'decline')
    code, out = send(ip + ':53318', big)
    ok('a declined request sends nothing', code == 3 and not os.listdir(destb), out)

    destc, logc = serve('C', 53319, 'accept', pin='4242')
    code, out = send(ip + ':53319', os.path.join(src, 'Album', 'Day 1', 'photo.jpg'))
    ok('with a PIN set, sending without it is refused', code == 4 and not os.listdir(destc), out)
    code, out = send(ip + ':53319', os.path.join(src, 'Album', 'Day 1', 'photo.jpg'), pin='1111')
    ok('a wrong PIN is refused', code == 4, out)
    code, out = send(ip + ':53319', os.path.join(src, 'Album', 'Day 1', 'photo.jpg'), pin='4242')
    ok('the right PIN works', code == 0 and os.path.exists(os.path.join(destc, 'photo.jpg')), out)

    p = subprocess.run([BIN, '--discover', '6'], env=env(os.path.join(TMP, 'nb-disc')), capture_output=True, text=True, timeout=60)
    try:
        found = json.loads(p.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError):
        found = []
    ok('discovery by multicast finds the running instances', any(d.get('alias') == 'Hearth A' for d in found), [d.get('alias') for d in found], required=False)


def localsend(ip):
    if not LS or not os.path.exists(LS):
        ok('LocalSend CLI available', False, 'not downloaded or built; LocalSend checks skipped', required=False)
        return
    print(subprocess.run([LS, '--version'], capture_output=True, text=True).stdout)
    cfg = os.path.join(TMP, 'ls-config')
    os.makedirs(cfg, exist_ok=True)
    lsenv = dict(os.environ, XDG_CONFIG_HOME=cfg, HOME=os.path.join(TMP, 'ls-home'))
    os.makedirs(lsenv['HOME'], exist_ok=True)
    dest = os.path.join(TMP, 'recv-A')
    # LocalSend -> Nearby Share (A is listening on 53317 and accepts). The released CLI is interactive: started with
    # --file, it lists the devices it finds as "[n] name (address)" and sends to the one whose number is typed.
    f = os.path.join(TMP, 'from-localsend.bin')
    with open(f, 'wb') as fh:
        fh.write(os.urandom(5 * 1024 * 1024 + 9))
    slog_path = os.path.join(TMP, 'ls-send.log')
    slog = open(slog_path, 'w')
    cmd = 'stty cols 160 rows 50; exec %s --port 53360 --alias "LocalSend CLI" --destination %s --file %s' % (LS, os.path.join(TMP, 'ls-unused'), f)
    sp = subprocess.Popen(['script', '-qfec', cmd, '/dev/null'], env=dict(lsenv, TERM='xterm-256color'), stdin=subprocess.PIPE, stdout=slog, stderr=subprocess.STDOUT)
    procs.append(sp)
    num = None
    for _ in range(40):
        time.sleep(1)
        text = re.sub(r'\x1b\[[0-9;?]*[A-Za-z]', '', open(slog_path, errors='replace').read())
        m = re.search(r'\[(\d)\] Hearth A ', text)
        if m:
            num = m.group(1)
            break
    ok('LocalSend discovers Nearby Share (listed as a device)', num, text[-500:] if num is None else 'number ' + num)
    got = os.path.join(dest, 'from-localsend.bin')
    if num:
        sp.stdin.write(num.encode())
        sp.stdin.flush()
        for _ in range(60):
            if os.path.exists(got) and os.path.getsize(got) == os.path.getsize(f):
                break
            time.sleep(1)
    time.sleep(1)
    ok('LocalSend -> Nearby Share: a file', os.path.exists(got) and sha(f) == sha(got),
       re.sub(r'\x1b\[[0-9;?]*[A-Za-z]', '', open(slog_path, errors='replace').read())[-700:])

    # Nearby Share -> LocalSend: the CLI receiving, trusting the sender's fingerprint (paired-v2.json).
    fp = subprocess.run([BIN, '--fingerprint'], env=env(os.path.join(TMP, 'nb-sender')), capture_output=True, text=True).stdout.strip()
    os.makedirs(os.path.join(cfg, 'localsend-cli'), exist_ok=True)
    with open(os.path.join(cfg, 'localsend-cli', 'paired-v2.json'), 'w') as fh:
        json.dump({'version': 1, 'devices': {fp: {'alias': 'Hearth sender', 'channels': []}}}, fh)
    recv = os.path.join(TMP, 'ls-recv')
    os.makedirs(recv, exist_ok=True)
    log = open(os.path.join(TMP, 'ls-recv.log'), 'w')
    cmd = 'stty cols 120 rows 40; exec %s --port 53361 --alias "LocalSend receiver" --destination %s' % (LS, recv)
    holder = subprocess.Popen(['sleep', '600'], stdout=subprocess.PIPE)
    rp = subprocess.Popen(['script', '-qfec', cmd, '/dev/null'], env=dict(lsenv, TERM='xterm-256color'), stdin=holder.stdout, stdout=log, stderr=subprocess.STDOUT)
    procs.extend([holder, rp])
    time.sleep(4)
    g = os.path.join(TMP, 'to-localsend.bin')
    with open(g, 'wb') as fh:
        fh.write(os.urandom(3 * 1024 * 1024 + 1))
    code, out = send(ip + ':53361', g)
    time.sleep(1)
    got = os.path.join(recv, 'to-localsend.bin')
    ok('Nearby Share -> LocalSend: a file (LocalSend trusted Hearth by its client certificate)', code == 0 and os.path.exists(got) and sha(got) == sha(g), out)
    a = events(os.path.join(TMP, 'A.log'))
    del a
    p = subprocess.run([BIN, '--discover', '6'], env=env(os.path.join(TMP, 'nb-disc2')), capture_output=True, text=True, timeout=60)
    try:
        found = json.loads(p.stdout.strip().splitlines()[-1])
    except (ValueError, IndexError):
        found = []
    ok('Nearby Share discovers LocalSend (multicast + register)', any('LocalSend' in (d.get('alias') or '') for d in found), [d.get('alias') for d in found], required=False)


def main():
    ip = my_ip()
    print('runner address', ip)
    two_instances(ip)
    localsend(ip)


if __name__ == '__main__':
    try:
        main()
    except Exception:  # noqa: BLE001
        import traceback
        traceback.print_exc()
        results.append(('the test script itself', False, True, 'crashed'))
    finally:
        for p in procs:
            try:
                p.send_signal(signal.SIGINT)
            except OSError:
                pass
        time.sleep(1)
        for p in procs:
            try:
                p.kill()
            except OSError:
                pass
        failed = [r for r in results if not r[1] and r[2]]
        print('\n%d checks, %d failed' % (len(results), len(failed)))
        print('::notice title=%d checks, %d failed::%s' % (len(results), len(failed), '%0A'.join(('pass  ' if g else ('FAIL  ' if q else 'warn  ')) + n for n, g, q, _ in results)))
        with open(os.environ.get('GITHUB_STEP_SUMMARY', '/dev/null'), 'a') as s:
            s.write('### Nearby Share: protocol tests\n\n| Check | Result |\n|---|---|\n')
            for n, good, req, d in results:
                s.write('| %s | %s |\n' % (n, 'pass' if good else ('FAIL' if req else 'warn')))
        for name in ('A', 'B', 'C', 'ls-recv', 'ls-send'):
            pth = os.path.join(TMP, name + '.log')
            if os.path.exists(pth):
                tail = open(pth, errors='replace').read()[-3000:]
                print('---- %s.log (end)' % name)
                print(tail)
                if failed:
                    print('::warning title=%s.log::%s' % (name, tail[-1500:].replace('%', '%25').replace('\n', '%0A')))
        shutil.rmtree(os.path.join(TMP, 'src'), ignore_errors=True)
    sys.exit(1 if failed or not results else 0)
