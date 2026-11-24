#!/usr/bin/python3
"""Copy one account from the Debian edition that's running into the Fedora (Anatase) deployment next to it, when
Fedora's first start didn't bring it over (the owner's dad's PC, Nov 21, 2026: Fedora's /etc had no account for him,
so its login and lock screens had nobody to let in). Run from Hearth OS on Debian, as root, in a mount namespace where
/sysroot is writable:

    h=$(openssl passwd -6) && sudo env H="$h" unshare -m sh -c \
        'mount -o remount,rw /sysroot && python3 hearth-copy-account.py chris'

The same user and group numbers (so the home folder stays the person's), the groups the person is in on Debian that
Fedora also has, plus wheel (Fedora's sudo), and the password hash in $H. Each file Fedora's /etc gets changed in is
kept first as <file>.hearth-bak, and written whole with the old one's owner, mode and SELinux label. Nothing else is
touched; Debian's own files are only read.
"""
import glob
import os
import shutil
import sys


def rd(p):
    with open(p) as f:
        return f.read().splitlines()


def wr(p, lines):
    shutil.copy2(p, p + '.hearth-bak')
    t = p + '.tmp'
    with open(t, 'w') as f:
        f.write('\n'.join(lines) + '\n')
    shutil.copystat(p, t)
    os.chown(t, os.stat(p).st_uid, os.stat(p).st_gid)
    os.replace(t, p)


def fedora_etc():
    for d in glob.glob('/ostree/deploy/*/deploy/*.[0-9]'):
        try:
            osr = rd(os.path.join(d, 'usr/lib/os-release'))
        except OSError:
            continue
        if 'ID=anatase' in osr or 'ID=fedora' in osr:
            return os.path.join(d, 'etc')
    raise SystemExit('stopped: no Fedora (Anatase) system next to this one')


def main():
    user = sys.argv[1] if len(sys.argv) > 1 else 'chris'
    h = os.environ.get('H', '')
    if not h.startswith('$'):
        raise SystemExit('stopped: no password hash in $H (h=$(openssl passwd -6) first)')
    etc = fedora_etc()
    me = [l for l in rd('/etc/passwd') if l.startswith(user + ':')]
    if not me:
        raise SystemExit('stopped: no account %s on this Debian' % user)
    me = me[0].split(':')
    uid, gid = me[2], me[3]
    me[6] = '/bin/bash'
    pw = rd(etc + '/passwd')
    for l in pw:
        f = l.split(':')
        if len(f) > 2 and f[2] == uid and f[0] != user:
            raise SystemExit('stopped: user number %s is %s in Fedora' % (uid, f[0]))
    wr(etc + '/passwd', [l for l in pw if not l.startswith(user + ':')] + [':'.join(me)])
    sh = [l for l in rd(etc + '/shadow') if not l.startswith(user + ':')]
    wr(etc + '/shadow', sh + ['%s:%s:20000:0:99999:7:::' % (user, h)])
    mine = {'wheel'}
    for l in rd('/etc/group'):
        f = l.split(':')
        if len(f) > 3 and user in f[3].split(','):
            mine.add(f[0])
    added = set()

    def groups(p, own):
        out, have = [], False
        for l in rd(p):
            f = l.split(':')
            if f[0] == user:
                have = True
            elif len(f) > 3 and f[0] in mine:
                m = [x for x in f[3].split(',') if x]
                if user not in m:
                    m.append(user)
                f[3] = ','.join(m)
                added.add(f[0])
            out.append(':'.join(f))
        if not have:
            out.append(own)
        wr(p, out)
    groups(etc + '/group', '%s:x:%s:' % (user, gid))
    if os.path.exists(etc + '/gshadow'):
        groups(etc + '/gshadow', '%s:!::' % user)
    print('done: %s is in Fedora (user %s), groups: %s' % (user, uid, ' '.join(sorted(added))))


if __name__ == '__main__':
    main()
