#!/usr/bin/env python3
"""Pretend worlds for Hearth Notes' CI pictures (hearth-apps-rust-shots.d/notes.sh): a copy of the hearth-workspace
layout with the mockups' own content (docs/mockups/hearth-notes), a local bare repository standing in for GitHub
(remote.origin.url says github.com, git's insteadOf points it at the local one), and a stand-in `gh` that only answers
`gh auth status`. No token, password or address anywhere. Each world is <out>/<name>/home (HOME) with hearth-workspace
in it:
  main      up to date with "GitHub" (synced 2:58 AM)
  today     main plus one change to push
  sync      three changes, and the workspace check finds two problems (no README, not in INDEX.md)
  conflict  the day's log changed here and on another PC: a merge waiting
  newitem   main without hearth/mockups/hearth-notes (the New item wizard makes it)
  custom    main with the graph customised as page 3f draws it
  fixture.py OUT REPO_ROOT"""
import json
import os
import shutil
import subprocess
import sys
from datetime import datetime, timezone

out, repo = os.path.abspath(sys.argv[1]), os.path.abspath(sys.argv[2])
UTC = timezone.utc


def ts(y, mo, d, h, mi):
    """A US Central time on the night of Nov 20-21, 2026 (CST, -6 h) as seconds."""
    return int(datetime(y, mo, d, h + 6, mi, tzinfo=UTC).timestamp()) if h + 6 < 24 else int(datetime(y, mo, d + 1, h + 6 - 24, mi, tzinfo=UTC).timestamp())


NOW = ts(2026, 11, 21, 3, 4)  # 3:04 AM on Nov 21: with the day starting at 6 AM, still Nov 20's log
SYNCED = ts(2026, 11, 21, 2, 58)

ASSETS = ['1.14-kwin-shots', '1.15-shots', '1.16-fedora-release-shots', '1.16-games-desktop-icons-shots', '1.16-kde-settings-shots', '1.16-labwc-fedora-shots',
          '1.16-player-discs-shots', '1.16-server-mode-fedora-shots', '1.16-store-shots', '1.16-vkbasalt-shots', 'firehold-preview-shots', 'music-new-horizons',
          'plasma-hearth-shots', 'site-gold-shots']
MOCKUPS = ['2.0-desktop', 'anatase-kde', 'boot-splash-gpu', 'firehold-plasma-more', 'g15-app', 'gold-theme', 'hearth-for-plasma', 'hearth-notes', 'hearth-store',
           'hearth-tv', 'hearth-tv-firehold', 'photos-1.14', 'rust-apps', 'site-redesign', 'train-wallpapers']
NOTES = ['1.14-release-prep', '1.16-fedora-release-channel', 'chat-history', 'video-style']
RESEARCH = ['2.0-desktop-names', 'kde-kcm-inventory']
VIDEOS = ['1.13-features', '1.13-trailer', '1.14-gold-trailer', '1.14-trailer', '1.15-release']
APPROVED = {'hearth-store', 'firehold-plasma-more', '2.0-desktop', 'hearth-tv', 'photos-1.14', 'g15-app'}

STORE = """# Hearth Store (mockups and plan)

Status: **approved** (Nov 19, 2026) · Belongs to: **1.16**

#mockup #1.16 #approved

## The pages
- [x] Home: the featured app, Your apps, every app
- [x] An app's page: what's new, Check and repair, Uninstall
- [x] Updates, Installing, Repair, Offline, Store settings
- [ ] Tried on the ROG Ally with a controller

## Pictures
The built Store's pictures: [[hearth/assets/1.16-store-shots]]
![[store-1-home.png]]

## Rebuild
```bash
cd hearth/mockups/hearth-store
python3 make.py # store.html → build/store-N-*.png
```

## Owner decisions
Nov 19: "Not offering anything Debian": Fedora only. See [[hearth-notes]].

| Logitech G15 | hearth-os, built in from 1.15 |
"""

LOGS = {
    '2026-11-15': "# 2026-11-15\n\n- 21:10 Made: Photos for 1.14, the new viewer. Stored: hearth/mockups/photos-1.14.\n- 22:30 Made: the 1.14 trailer (hearth/videos/1.14-trailer).\n- Note: hearth-workspace started tonight; everything made goes here.\n",
    '2026-11-16': "# 2026-11-16\n\n- 20:05 Made: KWin pictures for 1.14 (hearth/assets/1.14-kwin-shots).\n- 23:40 Decided: Hearth on Plasma looks like Mantel (hearth/mockups/hearth-for-plasma).\n",
    '2026-11-17': "# 2026-11-17\n\n- 01:20 Made: 1.15 release prep (hearth/notes/1.14-release-prep).\n- Owner: 1.14 pulled, back to 1.13 on both devices.\n- 04:10 Made: hearth/videos/1.15-release, the gold cut.\n",
    '2026-11-18': "# 2026-11-18\n\n- 00:40 Made: the Logitech G15 app plan: mockups of the G15 drivers and its screen (hearth/mockups/g15-app).\n- 02:15 Made: hearth/mockups/hearth-tv-firehold and hearth/mockups/site-redesign.\n- Waiting: Fill in the older videos' style (hearth/notes/video-style).\n",
    '2026-11-19': "# 2026-11-19\n\n- 00:30 Made: Store mockups approved — hearth-store (hearth/mockups/hearth-store). #1.16\n- G15 full app done: hearth-os feature/logitech-g15, every part pictured.\n- [x] G15 app: all 62 checklist parts ticked #1.15\n- 03:00 Made: Dad's Trains 2010 freeware, first look (other/research/ts2010-freeware).\n- Laptop (Fedora edition) autostart cleanup; Fedora image: check run 44469568330.\n- the Fedora USB and Recovery USB plan (hearth/notes/1.16-fedora-release-channel).\n",
    '2026-11-20': """# 2026-11-20

- 00:22 Made: KDE settings pages, the open parts: Get New… from the KDE Store, own command shortcuts, Calibrate HDR.
- 01:10 Made: Server mode on the Fedora edition: the dashboard is a pure-Rust app in cage. Waiting: A try on the Nvidia PC (Server mode).
- 01:36 Decided: Sharper graphics (vkBasalt) approved, 01:36.
- 01:50 Decided: Quick ACKs on the loopback for 1.16 (Trains 2010: 3½ min → about 30 s).
- 02:20 Made: Hearth Games desktop icons finished in CI.
- 02:30 Made: 1.16 Fedora release channel and Fedora USB (hearth/notes/1.16-fedora-release-channel). #1.16
- 02:40 Waiting: The owner's go-ahead to upload the Fedora USB #1.16
- 02:45 Waiting: Testing on the Ally and laptop (labwc on Fedora) #1.16
- 02:50 Decided: Craft Anything goes on the Command Center roadmap, 02:50.
- Pictures: [[hearth/assets/1.16-kde-settings-shots]] [[hearth/notes/1.16-fedora-release-channel]] [[hearth/assets/1.16-vkbasalt-shots]]
- Note: Long night of 1.16 work; everything green in CI except labwc's busy minute.
- [ ] Waiting: Install tick box: start on or off? #1.16
- [ ] Waiting: Okay to merge Craft Anything to main #roadmap
- [/] Merge vkBasalt into the 1.16 work #1.16
- [ ] Look at labwc's busy minute 10 min after start #1.16
- [x] Push the Hearth Notes mockups #draft
""",
}

TASKS = {
    'other/research/ts2010-freeware': "- [ ] Run the TS2010 freeware scripts on Dad's PC #dad\n",
    'hearth/notes/video-style': "- [ ] Fill in the older videos' style #video\n",
    'hearth/assets/1.16-kde-settings-shots': "- [ ] Test KWallet and the KDE Store downloads on a device #1.16\n",
    'hearth/assets/1.16-labwc-fedora-shots': "- [/] labwc on Fedora: Firehold's memory with real game pictures #1.16\n",
    'hearth/assets/1.16-store-shots': "checked against its mockups\n- [x] Hearth Store built, checklist complete #1.16\n",
    'hearth/mockups/firehold-plasma-more': "- [x] Firehold on Plasma mockups approved #1.16\n",
    'hearth/assets/1.16-vkbasalt-shots': "- [x] vkBasalt CI green #1.16\n",
    'hearth/mockups/hearth-notes': "- [ ] Waiting: Hearth Notes mockups: the owner's look #draft\n",
}


def write(root, rel, text, mtime=None):
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(text)
    t = mtime or ts(2026, 11, 19, 22, 0)
    os.utime(p, (t, t))


def git(cwd, *args, when=None, quiet=False):
    env = dict(os.environ, GIT_TERMINAL_PROMPT='0')
    if when:
        d = datetime.fromtimestamp(when, UTC).strftime('%Y-%m-%dT%H:%M:%SZ')
        env.update(GIT_AUTHOR_DATE=d, GIT_COMMITTER_DATE=d)
    r = subprocess.run(['git', '-C', cwd] + list(args), capture_output=True, text=True, env=env)
    if r.returncode != 0 and not quiet:
        print('::warning title=notes fixture::git %s: %s' % (' '.join(args), (r.stderr or r.stdout)[-400:]))
    return r.stdout


def workspace(root):
    write(root, 'README.md', "# hearth-workspace\n\nDragongamer9's store for everything made outside the project repos.\n")
    write(root, 'CLAUDE.md', "# Working in hearth-workspace\n\nThe layout is fixed: <project>/<kind>/<item>/.\n")
    write(root, '.gitignore', "build/\n__pycache__/\n*.pyc\n")
    index = ["# Index", "", "Every item in this repo, with its status.", "", "## Hearth OS"]
    for kind, names in (('assets', ASSETS), ('mockups', MOCKUPS), ('notes', NOTES), ('research', RESEARCH), ('videos', VIDEOS)):
        for n in names:
            status = 'approved' if n in APPROVED else ('checked' if kind == 'assets' else 'draft')
            index.append('- `hearth/%s/%s`: %s. Status: %s.' % (kind, n, n.replace('-', ' '), status))
    index += ["", "## Other", "- `other/notes/organize-downloads`: Dad's downloads. Status: draft.", "- `other/research/ts2010-freeware`: Dad's Trains 2010 freeware. Status: draft.", ""]
    write(root, 'INDEX.md', '\n'.join(index))
    for kind, names in (('assets', ASSETS), ('mockups', MOCKUPS), ('notes', NOTES), ('research', RESEARCH), ('videos', VIDEOS)):
        for n in names:
            item = 'hearth/%s/%s' % (kind, n)
            if n == 'hearth-store':
                write(root, item + '/README.md', STORE)
                write(root, item + '/make.py', "# renders store.html\n")
                write(root, item + '/store.html', "<html></html>\n")
                os.makedirs(os.path.join(root, item, 'build'), exist_ok=True)
                shutil.copy(os.path.join(repo, 'docs/mockups/hearth-store/store-1-home.png'), os.path.join(root, item, 'build/store-1-home.png'))
                continue
            status = 'approved' if n in APPROVED else ('checked' if kind == 'assets' else 'draft')
            tags = {'mockups': '#mockup', 'assets': '#shots', 'videos': '#video', 'notes': '#notes', 'research': '#research'}[kind]
            if n.startswith('1.16') or n in ('firehold-plasma-more', 'rust-apps', 'gold-theme', 'hearth-for-plasma', 'site-redesign', 'boot-splash-gpu'):
                tags += ' #1.16'
            tags += ' #approved' if status == 'approved' else (' #draft' if status == 'draft' else '')
            body = "# %s\n\nStatus: **%s** · Belongs to: %s\n\n%s\n" % (n.replace('-', ' ').title(), status, '1.16' if '#1.16' in tags else 'not yet assigned', tags)
            if n == 'hearth-notes':
                body += "\nThe owner's own Obsidian, connected to hearth-workspace. Pictures in [[hearth/assets/1.16-store-shots]] style.\n"
            if n == '1.16-store-shots':
                body += "\nCI pictures of every page of the built Store, checked against [[hearth/mockups/hearth-store]].\n"
            if n == 'video-style':
                body += "\nHow the videos look: landscape and vertical, the trailers' music ([[hearth/videos/1.15-release]]).\n"
            if n == '1.16-fedora-release-channel':
                body += "\n- The Fedora offline USB (installer ISO, edition anatase) built and installed in CI.\n- Still Debian-made: the Recovery USB.\n- Waiting: the owner's go-ahead to upload the ISO (the Fedora USB).\n- Pictures: [[hearth/assets/1.16-fedora-release-shots]]\n"
            if n == '1.16-fedora-release-shots':
                body += "\nthe Fedora offline USB installing in CI's virtual PC.\n"
            if n == 'g15-app':
                body += "\nThe G15 app's pages, approved Nov 18; built: [[hearth/mockups/hearth-store]] lists it.\n"
            if n in ('hearth-tv', 'hearth-tv-firehold', 'rust-apps'):
                body += "\nSee [[hearth/mockups/hearth-store]] and [[hearth/notes/video-style]].\n"
            body += TASKS.get(item, '')
            write(root, item + '/README.md', body)
    write(root, 'other/notes/organize-downloads/README.md', "# Organize downloads\n\nDad's downloads folder, sorted. #dad\n")
    write(root, 'other/research/ts2010-freeware/README.md', "# TS2010 freeware\n\nStatus: **draft** · Dad's Trains 2010 freeware routes and scripts. #dad #draft\n\nSee [[log/2026-11-19]].\n" + TASKS['other/research/ts2010-freeware'])
    write(root, 'tools/README.md', "# tools\n\ncheck.py: the organization check.\n")
    shutil.copy(os.path.join(repo, '.github/scripts/hearth-notes-check.py'), os.path.join(root, 'tools/check.py'))
    for d, text in LOGS.items():
        write(root, 'log/%s.md' % d, text, ts(2026, 11, int(d[-2:]), 23, 30) if d != '2026-11-20' else ts(2026, 11, 21, 3, 4))
    # Modified today (the log's day): their times.
    os.utime(os.path.join(root, 'hearth/mockups/hearth-notes/README.md'), (ts(2026, 11, 21, 3, 2),) * 2)
    os.utime(os.path.join(root, 'hearth/notes/1.16-fedora-release-channel/README.md'), (ts(2026, 11, 21, 2, 20),) * 2)


def world(name, change=None):
    base = os.path.join(out, name)
    home = os.path.join(base, 'home')
    ws = os.path.join(home, 'hearth-workspace')
    gh = os.path.join(base, 'github', 'Dragongamer9', 'hearth-workspace.git')
    os.makedirs(gh, exist_ok=True)
    workspace(ws)
    if name in ('newitem', 'sync'):
        shutil.rmtree(os.path.join(ws, 'hearth/mockups/hearth-notes'))
        idx = os.path.join(ws, 'INDEX.md')
        keep = ''.join(l for l in open(idx).read().splitlines(True) if 'hearth/mockups/hearth-notes' not in l)
        open(idx, 'w').write(keep)
    git(gh, 'init', '-q', '--bare')
    git(gh, 'symbolic-ref', 'HEAD', 'refs/heads/main')
    git(ws, 'init', '-q', '-b', 'main')
    git(ws, 'config', 'user.name', 'Dragongamer9')
    git(ws, 'config', 'user.email', 'Dragongamer9@users.noreply.github.com')
    git(ws, 'config', 'url.%s.insteadOf' % gh, 'https://github.com/Dragongamer9/hearth-workspace')
    git(ws, 'remote', 'add', 'origin', 'https://github.com/Dragongamer9/hearth-workspace')
    git(ws, 'add', '-A')
    git(ws, 'commit', '-q', '-m', 'The workspace', when=SYNCED - 60)
    git(ws, 'push', '-q', '-u', 'origin', 'main')
    git(ws, 'fetch', '-q')
    fh = os.path.join(ws, '.git/FETCH_HEAD')
    if os.path.exists(fh):
        os.utime(fh, (SYNCED, SYNCED))
    if change:
        change(ws, gh, base)
    # The app's own files on this PC: how often notes were opened (Solar system's planet sizes, the controller's recent
    # notes) and today's trackers.
    data = os.path.join(home, '.local/share/hearth/notes')
    os.makedirs(data, exist_ok=True)
    json.dump({'other/research/ts2010-freeware/README.md': 50, 'hearth/mockups/hearth-store/README.md': 41, 'hearth/notes/video-style/README.md': 30,
               'hearth/mockups/hearth-notes/README.md': 12, 'hearth/assets/1.16-store-shots/README.md': 9}, open(os.path.join(data, 'opened.json'), 'w'))
    key = ws.strip('/').replace('/', '-')
    json.dump({'days': {key + '/2026-11-20': {'productivity': 7, 'habits': {'Writing': True}}}}, open(os.path.join(data, 'trackers.json'), 'w'))
    os.makedirs(os.path.join(home, '.config/nimbus'), exist_ok=True)
    os.makedirs(os.path.join(home, '.local/share/nimbus'), exist_ok=True)


def today(ws, gh, base):
    with open(os.path.join(ws, 'log/2026-11-20.md'), 'a') as f:
        f.write('- 03:02 Made: Hearth Notes mockups: the pages plus the graph modes.\n')
    os.utime(os.path.join(ws, 'log/2026-11-20.md'), (ts(2026, 11, 21, 3, 4),) * 2)


def sync(ws, gh, base):
    with open(os.path.join(ws, 'log/2026-11-20.md'), 'a') as f:
        f.write('- 03:04 Made: Hearth Notes mockups: 9 pages plus the graph modes.\nStored: hearth/mockups/hearth-notes. Waiting: the owner\'s look.\n')
    write(ws, 'hearth/mockups/hearth-notes/notes.html', ''.join('<div class="p%d"></div>\n' % i for i in range(640)), ts(2026, 11, 21, 3, 0))
    write(ws, 'hearth/mockups/hearth-notes/make.py', ''.join('# step %d\n' % i for i in range(14)), ts(2026, 11, 21, 3, 1))


CONFLICT_BASE = "# 2026-11-20\n\n- 00:22 Made: KDE settings pages, the open parts…\n- 01:10 Made: Server mode on the Fedora edition…\n"


def conflict(ws, gh, base):
    write(ws, 'log/2026-11-20.md', CONFLICT_BASE, ts(2026, 11, 21, 1, 10))
    git(ws, 'add', '-A')
    git(ws, 'commit', '-q', '-m', 'log', when=ts(2026, 11, 21, 1, 11))
    git(ws, 'push', '-q')
    lap = os.path.join(base, 'laptop')
    subprocess.run(['git', 'clone', '-q', gh, lap], capture_output=True)
    git(lap, 'config', 'user.name', 'Dragongamer9')
    git(lap, 'config', 'user.email', 'Dragongamer9@users.noreply.github.com')
    write(lap, 'log/2026-11-20.md', CONFLICT_BASE + "- 03:06 Changed: laptop autostart: firewalld stays off,\nufw keeps the Hearth Link rules (owner).\n")
    git(lap, 'commit', '-q', '-am', 'laptop autostart', when=ts(2026, 11, 21, 3, 6))
    git(lap, 'push', '-q')
    write(ws, 'log/2026-11-20.md', CONFLICT_BASE + "- 03:04 Made: Hearth Notes mockups: 9 pages plus the graph modes.\nStored: hearth/mockups/hearth-notes. Waiting: the owner's look.\n- 03:10 Decided: graph modes go in (owner, 03:01).\n", ts(2026, 11, 21, 3, 10))
    git(ws, 'commit', '-q', '-am', 'Hearth Notes mockups', when=ts(2026, 11, 21, 3, 10))
    git(ws, 'pull', '-q', '--no-rebase', '--no-edit', quiet=True)
    os.utime(os.path.join(ws, 'log/2026-11-20.md'), (ts(2026, 11, 21, 3, 10),) * 2)


def custom(ws, gh, base):
    home = os.path.dirname(ws)
    cfg = os.path.join(home, '.config/nimbus')
    os.makedirs(cfg, exist_ok=True)
    g = {'pictures': True, 'shapesBy': 'kind', 'colourBy': 'project', 'sizeBy': 'links', 'linkStyle': 'curved', 'arrows': True, 'glow': True, 'orphans': True,
         'hideAttachments': True, 'bloom': 0.62, 'starfield': True, 'particles': True, 'labelSize': 0.4, 'labelsFade': True, 'look': 'Ember'}
    json.dump({'graph': {'classic.' + k: v for k, v in g.items()}}, open(os.path.join(cfg, 'hearth-notes.json'), 'w'), indent=1)


os.makedirs(out, exist_ok=True)
binp = os.path.join(out, 'bin')
os.makedirs(binp, exist_ok=True)
with open(os.path.join(binp, 'gh'), 'w') as f:
    f.write('#!/bin/sh\n# A stand-in for GitHub CLI in CI\'s pictures: it only says who is signed in (no token anywhere).\n'
            '[ "$1 $2" = "auth status" ] && { echo "github.com"; echo "  ✓ Logged in to github.com account Dragongamer9 (keyring)"; exit 0; }\nexit 1\n')
os.chmod(os.path.join(binp, 'gh'), 0o755)
world('main')
world('today', today)
world('sync', sync)
world('conflict', conflict)
world('newitem')
world('custom', custom)
print(NOW)
