#!/usr/bin/env python3
"""Every Hearth file a build script expects from system_files must be in git. (Peppermint's hearth-unlock was hidden by
.gitignore's bin/ for a release: the build's `chmod … || true` passed without it.) For each image build script, the
paths it chmods, enables or runs under /usr/bin/hearth-*, /usr/libexec/nimbus/, /usr/lib/systemd/system-generators/
and the hearth-*/nimbus-* units it enables must exist in os/system_files or that base's system_files, or be written by
the script itself. Prints ::error lines and exits 1 on a miss.
  check-system-files.py os-bases/peppermint os-bases/anatase"""
import os
import re
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
tracked = set(subprocess.run(['git', 'ls-files'], cwd=ROOT, capture_output=True, text=True, check=True).stdout.split('\n'))
fail = 0
for base in sys.argv[1:] or ['os-bases/peppermint', 'os-bases/anatase']:
    script = os.path.join(ROOT, base, 'build.sh')
    if not os.path.isfile(script):
        continue
    text = open(script).read()
    trees = ['os/system_files', base + '/system_files']
    want = set()
    for line in text.splitlines():
        line = line.split('#', 1)[0] if not line.lstrip().startswith('#') else ''
        if re.search(r'\bchmod\b', line):
            want |= set(re.findall(r'(/usr/(?:bin/hearth-[\w.-]+|libexec/nimbus/[\w.-]+|lib/systemd/system-generators/[\w.-]+))', line))
        m = re.search(r'\bsystemctl\s+(--global\s+)?enable\b', line)
        if m:
            d = '/usr/lib/systemd/user/' if m.group(1) else '/usr/lib/systemd/system/'
            for u in re.findall(r'\b((?:hearth|nimbus)-[\w@.-]+\.(?:service|path|timer|socket))\b', line):
                want.add(d + u)
    for path in sorted(want):
        if '*' in path:
            continue
        if any((t + path) in tracked for t in trees):
            continue
        # written by the script itself (cat > / install / ln / cp to that path)
        if re.search(r'(>|install\b[^\n]*|ln\b[^\n]*|cp\b[^\n]*)\s*"?' + re.escape(path) + r'\b', text):
            continue
        print(f'::error title=system files::{base}/build.sh uses {path}, which isn\'t in git under {" or ".join(trees)}')
        fail = 1
    print(f'{base}: {len(want)} paths checked')
sys.exit(fail)
