#!/usr/bin/env python3
"""CI's pretend world for the Hearth Store's pictures (hearth-apps-rust-shots.d/store.sh): the image's list of apps
with pictures made here, pretend GitHub releases (Command Center 3.13.0, Hearth AI 0.6.0) with their SHA256SUMS.txt,
and four home folders:
  home-a  Command Center 3.13.0 and Hearth AI 0.5.0 installed (store-1, 2, 3, 4, 6, 7, 9), the owner's private list
  home-b  as home-a, but Command Center's AppImage changed, not allowed to run, its menu entry gone (store-5)
  home-c  nothing installed, offline, the list seen this morning (store-8)
Usage: hearth-store-fixture.py OUT CATALOG
Nothing here touches the network; the Store reads it through HEARTH_STORE_FIXTURE and XDG_*_HOME."""
import hashlib
import json
import os
import shutil
import sys
import time

out, catalog = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
files = os.path.join(out, 'files')
os.makedirs(files, exist_ok=True)


def sha(b):
    return hashlib.sha256(b).hexdigest()


# ------------------------------------------------------------------ pictures like the mockup's (store-2)
def pictures():
    try:
        from PIL import Image, ImageDraw
    except ImportError:
        return {}
    made = {}
    bg, line, card, blue, mute = (15, 17, 21), (47, 52, 61), (32, 36, 43), (59, 130, 246), (58, 63, 72)

    def bars(d, x, y, n, w, gap=18, h=7):
        for i in range(n):
            d.rounded_rectangle([x, y + i * gap, x + w - (i * 37) % 60, y + i * gap + h], 3, fill=mute)

    def cc1(path):
        im = Image.new('RGB', (590, 300), bg)
        d = ImageDraw.Draw(im)
        d.rectangle([0, 0, 130, 300], fill=(24, 27, 33))
        bars(d, 14, 26, 8, 92)
        d.rounded_rectangle([14, 188, 116, 212], 6, fill=blue)
        d.rounded_rectangle([275, 18, 430, 80], 8, fill=(29, 51, 88))
        bars(d, 290, 38, 2, 120, 16)
        d.rounded_rectangle([148, 92, 360, 186], 8, fill=card)
        d.text((162, 104), "Ithil · her own mind", fill=(170, 176, 186))
        bars(d, 162, 126, 3, 170, 16)
        d.rounded_rectangle([148, 198, 346, 274], 8, fill=card)
        d.text((162, 210), "Claude leads · repo", fill=(232, 121, 121))
        bars(d, 162, 232, 2, 150, 16)
        d.rectangle([450, 0, 590, 300], fill=(24, 27, 33))
        bars(d, 462, 26, 7, 92)
        im.save(path)

    def cc2(path):
        im = Image.new('RGB', (470, 300), bg)
        d = ImageDraw.Draw(im)
        d.text((20, 22), "Settings", fill=(170, 176, 186))
        for i, n in enumerate(["AI team", "Accounts", "Ithil", "Tools", "Memory", "General"]):
            y = 46 + i * 29
            if i == 0:
                d.rounded_rectangle([20, y, 158, y + 26], 5, fill=(39, 43, 51))
            d.text((30, y + 7), n, fill=(220, 224, 230))
        for i in range(5):
            y = 46 + i * 51
            d.rounded_rectangle([172, y, 450, y + 42], 7, fill=card)
            d.rounded_rectangle([182, y + 17, 318, y + 24], 3, fill=mute)
            on = i in (0, 2, 4)
            d.rounded_rectangle([416, y + 14, 442, y + 28], 7, fill=blue if on else (70, 74, 82))
        im.save(path)

    def cc3(path):
        im = Image.new('RGB', (470, 300), bg)
        d = ImageDraw.Draw(im)
        d.text((20, 22), "The team at work", fill=(170, 176, 186))
        for i, (n, c) in enumerate([("Claude", (232, 121, 121)), ("ChatGPT", (110, 210, 140)), ("Gemini", (110, 160, 240)), ("Ithil", (170, 140, 250))]):
            y = 46 + i * 42
            d.ellipse([20, y, 46, y + 26], outline=c, width=2)
            d.text((58, y + 2), n, fill=(220, 224, 230))
            d.rounded_rectangle([58, y + 18, 200 - i * 20, y + 23], 3, fill=c)
        im.save(path)

    def ai(path, n):
        im = Image.new('RGB', (470, 300), (20, 18, 32))
        d = ImageDraw.Draw(im)
        import random
        r = random.Random(n)
        for _ in range(220):
            x, y = r.randint(60, 410), r.randint(40, 260)
            if ((x - 235) / 175) ** 2 + ((y - 150) / 110) ** 2 <= 1:
                d.ellipse([x - 2, y - 2, x + 2, y + 2], fill=(167, 139, 250) if r.random() > 0.2 else (255, 154, 82))
        im.save(path)

    for i, fn in enumerate([cc1, cc2, cc3], 1):
        p = os.path.join(out, 'pics', 'cc-%d.png' % i)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        fn(p)
        made[('command-center', i)] = p
    for i in range(1, 4):
        p = os.path.join(out, 'pics', 'ai-%d.png' % i)
        ai(p, i)
        made[('hearth-ai', i)] = p
    return made


pics = pictures()
cat = json.load(open(catalog))
for e in cat['apps']:
    if e['id'] in ('command-center', 'hearth-ai'):
        e['pictures'] = ['file://' + pics[(e['id'], i)] for i in range(1, 4) if (e['id'], i) in pics]
json.dump(cat, open(os.path.join(out, 'catalog.json'), 'w'), indent=1)
json.dump({'hearth': '1.16', 'edition': 'Fedora edition'}, open(os.path.join(out, 'this-pc.json'), 'w'))
open(os.path.join(out, 'fuse'), 'w').close()
open(os.path.join(out, 'checked-ago'), 'w').write('120')

# ------------------------------------------------------------------ pretend releases
CC = b'pretend Hearth AI Command Center 3.13.0 AppImage\n'
AI5 = b'pretend Hearth AI 0.5.0 AppImage\n'
AI6 = b'pretend Hearth AI 0.6.0 AppImage\n'
cc_name, ai_name = 'Hearth-AI-Command-Center-3.13.0-x86_64.AppImage', 'Hearth-AI-0.6.0-x86_64.AppImage'
open(os.path.join(files, cc_name), 'wb').write(CC)
open(os.path.join(files, ai_name), 'wb').write(AI6)
open(os.path.join(files, 'cc-SHA256SUMS.txt'), 'w').write('%s  %s\n%s  Hearth-AI-Command-Center-3.13.0-amd64.deb\n' % (sha(CC), cc_name, '0' * 64))
open(os.path.join(files, 'ai-SHA256SUMS.txt'), 'w').write('%s  %s\n%s  Hearth-AI-0.6.0-amd64.deb\n' % (sha(AI6), ai_name, '0' * 64))


def rel(tag, name, size, sums, body, when):
    return {'tag_name': tag, 'name': name, 'draft': False, 'prerelease': False, 'published_at': when,
            'html_url': 'https://github.com/Dragongamer9/x/releases/tag/' + tag, 'body': body,
            'assets': [{'name': os.path.basename(name), 'size': size, 'browser_download_url': 'file://' + os.path.join(files, os.path.basename(name))},
                       {'name': 'SHA256SUMS.txt', 'size': 222, 'browser_download_url': 'file://' + os.path.join(files, sums)}]}


releases = {
    'command-center': rel('v3.13.0', cc_name, 128936986, 'cc-SHA256SUMS.txt',
                          '- Settings that fit on the screen: a compact dialog that always fits the window; only its page scrolls.\n'
                          '- A side list in Settings with search: AI team, Accounts, Ithil, Tools, Memory, Voice & avatar, General.\n'
                          "- One-line rows: each setting's name, a short note, and its switch on the right.\n\nSHA-256:\n    x", '2026-11-15T06:26:37Z'),
    'hearth-ai': rel('v0.6.0', ai_name, 120000000, 'ai-SHA256SUMS.txt',
                     "- Colibri is her brain, with Ollama taking over by itself when Colibri isn't running.\n"
                     "- Colibri's models in two groups, Standard and Uncensored.\n"
                     "- The Mind screen's full 3D brain, lit by her real log: it replays her whole life, or follows her live.\n"
                     '- Tokens per second shown, even when a short answer arrives all at once.\n', '2026-11-19T18:00:00Z'),
}
json.dump(releases, open(os.path.join(out, 'releases.json'), 'w'), indent=1)


# ------------------------------------------------------------------ the homes
def home(name):
    h = os.path.join(out, name)
    for d in ('data', 'config', 'cache'):
        os.makedirs(os.path.join(h, d), exist_ok=True)
    return h


def install(h, app_id, fname, body, version, url):
    d = os.path.join(h, 'data/hearth/store/apps', app_id)
    os.makedirs(d, exist_ok=True)
    f = os.path.join(d, fname)
    open(f, 'wb').write(body)
    os.chmod(f, 0o755)
    st = os.stat(f)
    json.dump({'version': version, 'file': fname, 'sha256': sha(body), 'size': 128936986 if app_id == 'command-center' else 120000000, 'url': url,
               'page': 'https://github.com/Dragongamer9/x/releases/tag/v' + version, 'when': int(time.time()) - 86400, 'fsize': st.st_size, 'mtime': int(st.st_mtime)},
              open(os.path.join(d, 'store.json'), 'w'))
    icons = os.path.join(h, 'data/icons/hicolor/scalable/apps')
    os.makedirs(icons, exist_ok=True)
    open(os.path.join(icons, 'hearth-store-%s.svg' % app_id), 'w').write('<svg xmlns="http://www.w3.org/2000/svg"/>')
    apps = os.path.join(h, 'data/applications')
    os.makedirs(apps, exist_ok=True)
    open(os.path.join(apps, 'hearth-store-%s.desktop' % app_id), 'w').write('[Desktop Entry]\nType=Application\nName=%s\n' % app_id)
    return f


def private(h):
    d = os.path.join(h, 'config/nimbus')
    os.makedirs(d, exist_ok=True)
    json.dump({'apps': [
        {'id': 'hearth-cad', 'name': 'Hearth OS CAD', 'card': '3D design with a slicer for your printer', 'tagline': "123D Design's way of working, with a slicer for your printer",
         'icon': 'hex', 'from': '#38bdf8', 'to': '#0e7490', 'kind': 'coming'},
        {'id': 'trains-2010', 'name': 'Trains 2010', 'card': 'The train sim, made native for Linux', 'tagline': 'Trains 2010, made native for Linux',
         'icon': 'T', 'from': '#f97316', 'to': '#c2410c', 'kind': 'coming'}]}, open(os.path.join(d, 'store-private.json'), 'w'), indent=1)


def seen(h, checked, offline):
    """The list the Store saw last (what store-8 shows offline)."""
    c = os.path.join(h, 'cache/hearth/hearth-store')
    os.makedirs(c, exist_ok=True)
    apps = {}
    for k, r in releases.items():
        a = r['assets'][0]
        apps[k] = {'version': r['tag_name'][1:], 'asset': a['name'], 'url': a['browser_download_url'], 'size': a['size'],
                   'sha256': sha(CC if k == 'command-center' else AI6), 'sumsUrl': r['assets'][1]['browser_download_url'],
                   'notes': [l[2:] for l in r['body'].splitlines() if l.startswith('- ')], 'date': r['published_at'][:10], 'page': r['html_url'], 'test': False}
    json.dump({'checked': checked, 'offline': offline, 'apps': apps, 'told': []}, open(os.path.join(c, 'releases.json'), 'w'))


for name in ('home-a', 'home-b'):
    h = home(name)
    install(h, 'command-center', 'Hearth-AI-Command-Center.AppImage', CC, '3.13.0', 'file://' + os.path.join(files, cc_name))
    install(h, 'hearth-ai', 'Hearth-AI.AppImage', AI5, '0.5.0', 'file://' + os.path.join(files, 'Hearth-AI-0.5.0-x86_64.AppImage'))
    private(h)
    # Her memories (2.3 GB, as a sparse file: nothing is written), for the uninstall dialog's "(2.3 GB)".
    mem = os.path.join(h, 'HearthAI')
    os.makedirs(mem, exist_ok=True)
    with open(os.path.join(mem, 'memories.db'), 'wb') as f:
        f.truncate(2_300_000_000)
b = os.path.join(out, 'home-b/data/hearth/store/apps/command-center/Hearth-AI-Command-Center.AppImage')
open(b, 'wb').write(b'something else\n')
os.chmod(b, 0o644)
os.remove(os.path.join(out, 'home-b/data/applications/hearth-store-command-center.desktop'))

h = home('home-c')
private(h)
lt = time.localtime()
morning = int(time.mktime((lt.tm_year, lt.tm_mon, lt.tm_mday, 9, 12, 0, 0, 0, -1)))
seen(h, morning if morning < time.time() else int(time.time()) - 3600, True)
off = os.path.join(out, 'offline-world')
os.makedirs(off, exist_ok=True)
for f in ('catalog.json', 'this-pc.json', 'releases.json', 'fuse'):
    shutil.copy(os.path.join(out, f), os.path.join(off, f))
open(os.path.join(off, 'offline'), 'w').close()
print('store fixture in', out)
