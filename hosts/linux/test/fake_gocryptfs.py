#!/usr/bin/python3
"""A stand-in for gocryptfs (and fusermount, called as fake_fusermount) for test_hearth_briefcase.py, where FUSE can't
be mounted. Same arguments and exit codes as the real one (12: wrong password); the password comes on stdin. It keeps
only a salted hash of the password; "mounting" makes the mount point a symlink to the stored files (the helper's
HEARTH_BRIEFCASE_FAKE_MOUNTS=1 counts a symlink as mounted). Every argv is appended to $FAKE_ARGV_LOG."""
import hashlib
import json
import os
import sys


def log_argv():
    f = os.environ.get('FAKE_ARGV_LOG')
    if f:
        with open(f, 'a') as fh:
            fh.write(json.dumps(sys.argv) + '\n')


def h(salt, pw):
    return hashlib.sha256((salt + pw).encode()).hexdigest()


def conf(cipher):
    return os.path.join(cipher, 'gocryptfs.conf')


def read_lines(n):
    data = sys.stdin.read().split('\n')
    return (data + [''] * n)[:n]


def main():
    log_argv()
    me = os.path.basename(sys.argv[0])
    args = [a for a in sys.argv[1:] if a not in ('-q', '--')]
    if 'fusermount' in me:
        mp = args[-1]
        if os.path.exists(os.environ.get('FAKE_BUSY', '/nonexistent-busy')):
            print('fusermount: failed to unmount %s: Device or resource busy' % mp, file=sys.stderr)
            return 1
        if os.path.islink(mp):
            os.unlink(mp)
        return 0
    if args and args[0] == '-init':
        cipher = args[1]
        (pw,) = read_lines(1)
        salt = os.urandom(8).hex()
        os.makedirs(os.path.join(cipher, 'data'), exist_ok=True)
        with open(conf(cipher), 'w') as f:
            json.dump({'salt': salt, 'hash': h(salt, pw)}, f)
        return 0
    if args and args[0] == '-passwd':
        cipher = args[1]
        old, new = read_lines(2)
        c = json.load(open(conf(cipher)))
        if h(c['salt'], old) != c['hash']:
            print('Password incorrect.', file=sys.stderr)
            return 12
        salt = os.urandom(8).hex()
        with open(conf(cipher), 'w') as f:
            json.dump({'salt': salt, 'hash': h(salt, new)}, f)
        return 0
    cipher, mp = args[0], args[1]
    (pw,) = read_lines(1)
    c = json.load(open(conf(cipher)))
    if h(c['salt'], pw) != c['hash']:
        print('Password incorrect.', file=sys.stderr)
        return 12
    if os.path.isdir(mp) and not os.path.islink(mp):
        os.rmdir(mp)
    os.symlink(os.path.join(os.path.abspath(cipher), 'data'), mp)
    return 0


if __name__ == '__main__':
    sys.exit(main())
