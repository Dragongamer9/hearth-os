#!/usr/bin/env python3
"""Every commit is the owner's (CLAUDE.md rule 3): fails when a commit in BASE..HEAD (default origin/main..HEAD) was
authored or committed as noreply@anthropic.com, or its message carries Co-authored-by, Claude-Session or
"Generated with" lines. Reads git only; changes nothing. Run by tidy.yml.
  attribution-check.py [BASE] [HEAD]"""
import re
import subprocess
import sys

# Known before this check existed (Nov 16 merges into bughunt); the owner decides whether history is rewritten.
# Until then they're reported as notices, not failures. Nothing else may be added here without his word.
KNOWN = {'3d0429a91', '9b2135cb9'}
BAD_EMAIL = 'noreply@anthropic.com'
# The trailer lines tools add (a line of their own, at its start): Co-authored-by:, Claude-Session:, "🤖 Generated with …".
BAD_LINE = re.compile(r'^\s*(co-authored-by\s*:|claude-session\s*:|(\U0001F916\s*)?generated with\b)', re.I | re.M)


def commits(base, head):
    out = subprocess.run(['git', 'log', '--format=%H%x1f%ae%x1f%ce%x1f%B%x1e', '%s..%s' % (base, head)],
                         capture_output=True, text=True, check=True).stdout
    for rec in out.split('\x1e'):
        rec = rec.strip('\n')
        if not rec:
            continue
        sha, ae, ce, body = rec.split('\x1f', 3)
        yield sha, ae, ce, body


def problems(base, head):
    bad, known = [], []
    for sha, ae, ce, body in commits(base, head):
        why = []
        if ae.lower() == BAD_EMAIL:
            why.append('author ' + ae)
        if ce.lower() == BAD_EMAIL:
            why.append('committer ' + ce)
        m = BAD_LINE.search(body)
        if m:
            why.append('message line "%s"' % m.group(0).strip()[:60])
        if why:
            (known if sha[:9] in KNOWN else bad).append((sha[:9], body.splitlines()[0][:80] if body else '', why))
    return bad, known


def main(argv):
    base = argv[1] if len(argv) > 1 else 'origin/main'
    head = argv[2] if len(argv) > 2 else 'HEAD'
    bad, known = problems(base, head)
    for sha, subj, why in known:
        print('::notice title=attribution::%s "%s" (%s): known, waiting for the owner' % (sha, subj, ', '.join(why)))
    for sha, subj, why in bad:
        print('::error title=attribution::%s "%s": %s (CLAUDE.md rule 3: commits only as Dragongamer9, no attribution lines)'
              % (sha, subj, ', '.join(why)))
    print('attribution: %d commit(s) not the owner\'s, %d known' % (len(bad), len(known)))
    return 1 if bad else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
