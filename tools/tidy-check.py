#!/usr/bin/env python3
"""Quick checks that keep the repository tidy (1.13 tidy-up; tidy.yml runs them on every push, in seconds):

  1. Settings' search index is up to date with its pages (tools/build-settings-index.py --check).
  2. No roadmap item is listed twice (the same item ticked in two places, or once ticked and once still planned).
  3. Tests don't only pass on one kind of machine: no test reads a fixed home folder (/home/<name>, /root, /var/home),
     the author's own paths, or this repository's absolute location instead of finding it from the test file.

  python3 tools/tidy-check.py            prints ::error lines for GitHub and exits 1 on a problem
"""
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
problems = []


def bad(title, text):
    problems.append((title, text))
    print('::error title=%s::%s' % (title, text))


def settings_index():
    r = subprocess.run([sys.executable, os.path.join(ROOT, 'tools', 'build-settings-index.py'), '--check'],
                       capture_output=True, text=True)
    if r.returncode != 0:
        bad('settings index', 'hosts/linux/desktop/shell/settings-index.js is out of date: run python3 tools/build-settings-index.py (%s)'
            % (r.stdout + r.stderr).strip()[-300:].replace('\n', ' '))


def norm(item):
    t = re.sub(r'\((needs testing on a device|checklist [^)]*|mockups? approved [^)]*)\)', '', item, flags=re.I)
    t = re.sub(r'[^a-z0-9 ]+', ' ', t.lower())
    return ' '.join(t.split())[:140]


def roadmap_twice(path=os.path.join(ROOT, 'docs', 'ROADMAP.md')):
    seen = {}
    for n, line in enumerate(open(path, encoding='utf-8'), 1):
        m = re.match(r'^- \[( |x)\] (.*)', line)
        if not m:
            continue
        k = norm(m.group(2))
        if len(k) < 40:
            continue                     # short items ("Fix: …") can repeat across releases on purpose
        if k in seen:
            bad('roadmap', 'docs/ROADMAP.md lines %d and %d are the same item: %s' % (seen[k], n, m.group(2)[:120]))
        else:
            seen[k] = n


MACHINE = re.compile(r'''['"](/home/(?!you\b|user\b|test\b|liveuser\b)[a-z][\w.-]*|/var/home/[a-z][\w.-]*|/root/)[^'"]*['"]''')


FS = re.compile(r'\b(open|exists|isdir|isfile|listdir|scandir|stat|getsize|read_text|iterdir|glob)\(')
HOME = re.compile(r'''(expanduser\(['"]~['"]\)|Path\.home\(\))''')


def machine_tests(dirs=('hosts/linux/test',)):
    for d in dirs:
        for top, _, files in os.walk(os.path.join(ROOT, d)):
            for f in files:
                if not (f.startswith('test_') and f.endswith('.py')):
                    continue
                p = os.path.join(top, f)
                for n, line in enumerate(open(p, encoding='utf-8', errors='replace'), 1):
                    if line.lstrip().startswith('#'):
                        continue
                    # Only where the test really reads the machine (a fixed path handed to a function as data is fine).
                    m = MACHINE.search(line) if FS.search(line) else None
                    if not m and HOME.search(line):
                        m = HOME.search(line)
                    if m:
                        bad('test needs one machine', '%s:%d reads %s: use a temporary folder or find it from the test file'
                            % (os.path.relpath(p, ROOT), n, m.group(1)))


def main():
    settings_index()
    roadmap_twice()
    machine_tests()
    print('tidy: %s' % ('%d problem(s)' % len(problems) if problems else 'all good'))
    return 1 if problems else 0


if __name__ == '__main__':
    sys.exit(main())
