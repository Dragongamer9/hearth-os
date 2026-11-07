#!/usr/bin/env python3
"""End to end, in CI (home-server.yml, smb-e2e): a real Samba with Hearth's own config (build.sh's smb.conf and the
home server helper's shares) and a real gvfs. hearth_netshares.py opens the shares the way Hearth Files does, and
every failure must say why (wrong password, share off, Tailscale only, Samba not running, no answer).

  sudo python3 e2e_netshares.py setup USER       Hearth's smb.conf, two shares, a throwaway sharing password, smbd
  dbus-run-session -- python3 e2e_netshares.py check   (as USER, with XDG_RUNTIME_DIR set)

Each case posts a ::notice with what came back (gio's own words too), and an ::error when it isn't what Hearth must say.
"""
import importlib.machinery
import importlib.util
import os
import pathlib
import re
import subprocess
import sys
import time

sys.dont_write_bytecode = True
ROOT = pathlib.Path(__file__).resolve().parents[3]
SHELL = ROOT / 'hosts' / 'linux' / 'desktop' / 'shell'
HELPER = ROOT / 'os' / 'system_files' / 'usr' / 'libexec' / 'nimbus' / 'hearth-home-server-helper'
BASE = '/srv/hearth-e2e'
PASSWORD = 'e2e only, not a secret 7'   # a throwaway account password on a CI runner


def load(name, path):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    mod = importlib.util.module_from_spec(spec)
    loader.exec_module(mod)
    return mod


def setup(user):
    h = load('hhs_e2e', HELPER)
    build = (ROOT / 'os-bases' / 'peppermint' / 'build.sh').read_text()
    base = re.search(r"cat > /etc/samba/smb.conf <<'SMBCONF'\n(.*?)\nSMBCONF\n", build, re.S).group(1) + '\n'
    shares = []
    for name, sub, reach in (('Media Files', 'media', 'home'), ('TsOnly', 'ts', 'tailscale')):
        path = os.path.join(BASE, sub)
        os.makedirs(path, exist_ok=True)
        with open(os.path.join(path, 'hello.txt'), 'w') as f:
            f.write('hello from %s\n' % name)
        subprocess.run(['chown', '-R', user, path], check=True)
        shares.append({'name': name, 'path': path, 'write': True, 'people': {}, 'recycle': False, 'whole': '', 'reach': reach})
    with open(h.OUR_CONF, 'w') as f:
        f.write(h.conf_text(shares, user, False))
    with open(h.SMB_CONF, 'w') as f:
        f.write(h.with_include(base))
    r = subprocess.run(['testparm', '-s', h.SMB_CONF], capture_output=True, text=True)
    print(r.stdout)
    if r.returncode != 0:
        print('::error title=smb.conf::testparm refused Hearth\'s config: %s' % r.stderr.strip().replace('\n', ' | ')[:900])
        sys.exit(1)
    r = subprocess.run(['smbpasswd', '-a', '-s', user], input='%s\n%s\n' % (PASSWORD, PASSWORD), capture_output=True, text=True)
    if r.returncode != 0:
        print('::error title=smbpasswd::%s' % (r.stdout + r.stderr).strip()[:500])
        sys.exit(1)
    subprocess.run(['smbpasswd', '-e', user], check=True)
    subprocess.run(['systemctl', 'restart', 'smbd'], check=True)
    time.sleep(2)
    print(subprocess.run(['ss', '-ltnp'], capture_output=True, text=True).stdout)


def check():
    sys.path.insert(0, str(SHELL))
    import hearth_netshares as ns
    os.environ['XDG_STATE_HOME'] = os.path.join(os.environ['XDG_RUNTIME_DIR'], 'state')
    me = subprocess.run(['id', '-un'], capture_output=True, text=True).stdout.strip()
    bad = []

    def case(title, net, expect, **kw):
        t0 = time.time()
        r = ns.net_open(dict({'name': 'ci-server', 'home': '127.0.0.1', 'away': ''}, **net), at_home=lambda h: True, state={},
                        lookup=lambda *a: None, store=lambda *a: None, **kw)
        took = time.time() - t0
        got = 'ok' if r.get('ok') else r.get('need')
        msg = 'got=%s expected=%s in %.1fs | error: %s | gio: %s | path: %s | shares: %s' % (
            got, expect, took, r.get('error', ''), r.get('detail', ''), r.get('path', ''), r.get('shares', ''))
        print('::notice title=%s::%s' % (title, msg))
        if got not in (expect if isinstance(expect, tuple) else (expect,)):
            bad.append(title)
            print('::error title=%s::%s' % (title, msg))
        if PASSWORD in repr(r):
            bad.append(title + ' (password shown)')
            print('::error title=%s::the password came back in the answer' % title)
        return r

    case('first time asks to sign in', {'share': 'Media Files'}, 'password')
    case('wrong password', {'share': 'Media Files'}, 'password', user=me, password='not it at all')
    r = case('right password opens it', {'share': 'Media Files'}, 'ok', user=me, password=PASSWORD)
    if r.get('ok'):
        try:
            names = os.listdir(r['path'])
        except OSError as e:
            names = ['(%s)' % e]
        print('::notice title=listing::%s' % names)
        if 'hello.txt' not in names:
            bad.append('listing')
            print('::error title=listing::hello.txt isn\'t in the opened folder: %s' % names)
        case('already open', {'share': 'Media Files'}, 'ok')
    case('a share that isn\'t there', {'share': 'Nope'}, 'off', user=me, password=PASSWORD)
    # Samba refuses a share's "hosts allow" at the tree connect (ACCESS_DENIED), which gvfs takes for a wrong password:
    # why Hearth Files now opens a Tailscale-only share through Tailscale (hearth_netshares.choose_address).
    case('Tailscale only, from this network', {'share': 'TsOnly'}, ('refused', 'password'), user=me, password=PASSWORD)
    case('Tailscale only goes through Tailscale', {'share': 'TsOnly', 'reach': 'tailscale', 'away': ''}, 'tailscale')
    case('a whole computer', {'share': ''}, ('pick', 'ok'), user=me, password=PASSWORD)
    case('no answer', {'share': 'Media Files', 'home': '10.255.255.1'}, 'asleep', user=me, password=PASSWORD)
    subprocess.run(['sudo', 'systemctl', 'stop', 'smbd'], check=False)
    time.sleep(1)
    case('Samba not running', {'share': 'Media Files', 'home': '127.0.0.2'}, 'sharing-off', user=me, password=PASSWORD)
    if bad:
        print('::error title=shared folders e2e::%d case(s) not right: %s' % (len(bad), ', '.join(bad)))
        sys.exit(1)
    print('all cases right')


if __name__ == '__main__':
    if sys.argv[1:2] == ['setup']:
        setup(sys.argv[2])
    elif sys.argv[1:2] == ['check']:
        check()
    else:
        sys.exit(__doc__)
