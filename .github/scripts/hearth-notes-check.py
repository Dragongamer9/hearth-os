#!/usr/bin/env python3
"""A copy of hearth-workspace's organization check (its tools/check.py, RED FLAG 2), for Hearth Notes' CI pictures: the
pretend workspace runs it before a push exactly as the real one does. python3 tools/check.py"""
import os, re, subprocess, sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROJECTS = {'hearth', 'beacon', 'quad-mech', 'gravesleep', 'tributary', 'hearth-ai', 'command-center', 'other'}
KINDS = {'videos', 'mockups', 'research', 'notes', 'assets'}
TOP_FILES = {'README.md', 'CLAUDE.md', 'INDEX.md', '.gitignore'}
TOP_DIRS = PROJECTS | {'tools', 'log', '.github', '.git'}
NAME = re.compile(r'^[a-z0-9][a-z0-9._-]*$')
MAX = 50 * 1000 * 1000
bad = []

def tracked():
    out = subprocess.run(['git', 'ls-files', '--cached', '--others', '--exclude-standard'], cwd=ROOT, capture_output=True, text=True).stdout
    return [p for p in out.splitlines() if p]

files = tracked()
index = open(os.path.join(ROOT, 'INDEX.md')).read() if os.path.exists(os.path.join(ROOT, 'INDEX.md')) else ''
items = set()
for p in files:
    parts = p.split('/')
    if len(parts) == 1:
        if p not in TOP_FILES: bad.append(f'{p}: a loose file at the top (put it in an item folder)')
        continue
    if parts[0] not in TOP_DIRS: bad.append(f'{p}: "{parts[0]}" isn\'t a project in the layout (add it to CLAUDE.md and INDEX.md first)')
    if parts[0] == 'log':
        if not re.match(r'^log/\d{4}-\d{2}-\d{2}\.md$', p): bad.append(f'{p}: logs are log/YYYY-MM-DD.md')
        continue
    if parts[0] in PROJECTS:
        if len(parts) < 4: bad.append(f'{p}: a loose file (projects hold <kind>/<item>/ folders)'); continue
        if parts[1] not in KINDS: bad.append(f'{p}: "{parts[1]}" isn\'t a kind in the layout')
        items.add('/'.join(parts[:3]))
    for seg in parts[:-1]:
        if seg not in ('.github',) and not NAME.match(seg): bad.append(f'{p}: folder "{seg}" isn\'t lowercase-with-dashes')
    try:
        if os.path.getsize(os.path.join(ROOT, p)) > MAX: bad.append(f'{p}: over 50 MB (finished videos are release files)')
    except OSError:
        pass
for it in sorted(items):
    if f'{it}/README.md' not in files: bad.append(f'{it}: no README.md')
    if it not in index: bad.append(f'{it}: not listed in INDEX.md')
if bad:
    print('hearth-workspace isn\'t tidy:'); [print('  - ' + b) for b in bad]; sys.exit(1)
print(f'tidy: {len(items)} items, {len(files)} files')
