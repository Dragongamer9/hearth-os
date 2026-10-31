#!/usr/bin/env python3
"""End-to-end tests for Hearth's gaming apps (hosts/hearth-gaming), run by .github/workflows/gaming.yml.

  e2e.py saves  <bin dir> <work dir>   Saves against a fake home: a Steam game with Proton saves (Hollow Knight) and
                                       a native one (Celeste), a test save-location list, backup, restore with a
                                       safety copy, restore on "another PC", scheduled backups, then the real list
  e2e.py mods   <bin dir> <work dir>   Mods against a fake game folder: install from a zip and a folder, load order,
                                       off puts the game's files back, remove
  e2e.py pads   <bin dir>              Controller Lab against virtual controllers (/dev/uinput; run as root):
                                       buttons, sticks, triggers, motion sensors, players, rumble, calibration

Each check prints "ok: ..." or "FAIL: ..."; the exit code is the number of failures.
"""
import json, os, shutil, subprocess, sys, time, zipfile

FAILS = []


def check(what, cond, detail=''):
    print(('ok: ' if cond else 'FAIL: ') + what + ('' if cond or not detail else '  [%s]' % str(detail)[:800]), flush=True)
    if not cond:
        FAILS.append(what)


def put(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, 'w') as f:
        f.write(text)


def read(path):
    try:
        with open(path) as f:
            return f.read()
    except OSError:
        return '<missing>'


def run(cmd, env, timeout=300, ok_codes=(0,)):
    r = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=timeout, stdin=subprocess.DEVNULL)
    if r.returncode not in ok_codes:
        print('  $ %s -> %d\n  stdout: %s\n  stderr: %s' % (' '.join(cmd), r.returncode, r.stdout[-1500:], r.stderr[-1500:]), flush=True)
    return r


def as_json(r):
    try:
        return json.loads(r.stdout)
    except ValueError:
        return None


TEST_MANIFEST = r'''
Hollow Knight:
  files:
    "<base>/hollow_knight_Data/Config.ini":
      tags: [config]
      when:
        - os: windows
        - os: linux
    "<home>/AppData/LocalLow/Team Cherry/Hollow Knight/*.dat":
      tags: [save]
      when:
        - os: windows
    "<xdgConfig>/unity3d/Team Cherry/Hollow Knight/*.dat":
      tags: [save]
      when:
        - os: linux
  installDir:
    Hollow Knight: {}
  steam:
    id: 367520
Celeste:
  files:
    "<xdgData>/Celeste/Saves/*.celeste":
      tags: [save]
      when:
        - os: linux
  installDir:
    Celeste: {}
  steam:
    id: 504230
'''


def fake_home(root, library_name='SteamLibrary', prefix=True):
    """A home folder with Steam, Hollow Knight (Windows game: saves in its Proton prefix) and Celeste (native)."""
    home = os.path.join(root, 'home')
    steam = os.path.join(home, '.local/share/Steam')
    lib = os.path.join(root, library_name)
    put(os.path.join(steam, 'steamapps/libraryfolders.vdf'),
        '"libraryfolders" { "0" { "path" "%s" } "1" { "path" "%s" } }' % (steam, lib))
    put(os.path.join(lib, 'steamapps/appmanifest_367520.acf'), '"AppState" { "appid" "367520" "name" "Hollow Knight" "installdir" "Hollow Knight" "StateFlags" "4" }')
    put(os.path.join(steam, 'steamapps/appmanifest_504230.acf'), '"AppState" { "appid" "504230" "name" "Celeste" "installdir" "Celeste" "StateFlags" "4" }')
    hk = os.path.join(lib, 'steamapps/common/Hollow Knight')
    put(os.path.join(hk, 'hollow_knight.exe'), 'exe')
    put(os.path.join(hk, 'hollow_knight_Data/Config.ini'), 'fullscreen=1')
    put(os.path.join(hk, 'Data/a.txt'), 'original a')
    put(os.path.join(steam, 'steamapps/common/Celeste/Celeste'), 'elf')
    pfx_user = os.path.join(lib, 'steamapps/compatdata/367520/pfx/drive_c/users/steamuser')
    if prefix:
        os.makedirs(pfx_user, exist_ok=True)
    return home, hk, pfx_user


def env_for(home):
    e = {k: v for k, v in os.environ.items() if not k.startswith('XDG_')}
    e['HOME'] = home
    e['USER'] = 'runner'
    return e


# ---------------------------------------------------------------------- Saves

def saves(bindir, work):
    exe = os.path.join(bindir, 'hearth-saves')
    shutil.rmtree(work, ignore_errors=True)
    home, hk, pfx_user = fake_home(os.path.join(work, 'pc1'))
    hk_save = os.path.join(pfx_user, 'AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat')
    put(hk_save, 'progress 1')
    put(os.path.join(home, '.local/share/Celeste/Saves/0.celeste'), '<SaveData>chapter 3</SaveData>')
    put(os.path.join(work, 'manifest.yaml'), TEST_MANIFEST)
    env = env_for(home)
    env['HEARTH_SAVES_MANIFEST'] = os.path.join(work, 'manifest.yaml')

    r = run([exe, '--list'], env)
    lst = as_json(r) or {}
    games = {g['id']: g for g in lst.get('games', [])}
    check('Saves lists both Steam games from Hearth\'s library', 'steam:367520' in games and 'steam:504230' in games, list(games))
    hkg = games.get('steam:367520', {})
    check('it finds Hollow Knight\'s Proton prefix', (hkg.get('prefix') or '').endswith('compatdata/367520/pfx'), hkg.get('prefix'))
    check('Hollow Knight: the save in the prefix and the config in the game folder', (hkg.get('saves') or {}).get('count') == 2, hkg.get('saves'))
    check('Celeste: its native save', (games.get('steam:504230', {}).get('saves') or {}).get('count') == 1, games.get('steam:504230'))
    check('the test save-location list was used', lst.get('manifest', {}).get('ok') is True, lst.get('manifest'))

    usb = os.path.join(work, 'usb')
    os.makedirs(usb)
    r = run([exe, '--backup', 'all', '--to', usb], env)
    out = as_json(r) or {}
    check('back up all: two games backed up', r.returncode == 0 and len(out.get('done', [])) == 2, r.stdout[-600:])
    zips = [os.path.join(dp, f) for dp, _, fs in os.walk(usb) for f in fs if f.endswith('.zip')]
    check('one zip per game, in "Hearth Saves/<game>/"', len(zips) == 2 and all('/Hearth Saves/' in z for z in zips), zips)
    hk_zip = next((z for z in zips if 'Hollow Knight' in z), None)
    if hk_zip:
        names = zipfile.ZipFile(hk_zip).namelist()
        check('the zip keeps files by portable names', 'files/prefix-user/AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat' in names and 'files/game/hollow_knight_Data/Config.ini' in names, names)
        meta = json.loads(zipfile.ZipFile(hk_zip).read('hearth-saves.json'))
        check('its description says which game', meta.get('id') == 'steam:367520' and meta.get('title') == 'Hollow Knight', meta)
    r = run([exe, '--backups', usb], env)
    check('the backups are listed back', len(as_json(r) or []) == 2, r.stdout[-400:])

    # Play on, then restore: the old save comes back, the new one is kept as a safety copy.
    put(hk_save, 'progress 2 (newer)')
    r = run([exe, '--restore', hk_zip or '', '--yes'], env)
    res = as_json(r) or {}
    check('restore puts the backed-up save back', read(hk_save) == 'progress 1', read(hk_save))
    check('restore made a safety copy first', bool(res.get('safety')) and os.path.exists(res.get('safety') or '/nope'), res)
    if res.get('safety'):
        check('the safety copy has the newer save', 'progress 2' in zipfile.ZipFile(res['safety']).read('files/prefix-user/AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat').decode(), '')
    r = run([exe, '--restore', hk_zip or ''], env, ok_codes=(1,))
    check('without --yes, a restore asks first (and changes nothing when not answered)', r.returncode == 1 and 'Nothing changed' in r.stderr, r.stderr[-300:])

    # "Another Hearth PC": the game in another library folder, its prefix elsewhere.
    home2, hk2, pfx_user2 = fake_home(os.path.join(work, 'pc2'), library_name='Games/SteamLibrary')
    env2 = env_for(home2)
    env2['HEARTH_SAVES_MANIFEST'] = env['HEARTH_SAVES_MANIFEST']
    r = run([exe, '--restore', hk_zip or '', '--yes'], env2)
    check('a backup from one PC restores on another (the prefix is somewhere else there)',
          read(os.path.join(pfx_user2, 'AppData/LocalLow/Team Cherry/Hollow Knight/user1.dat')) == 'progress 1', r.stdout[-500:] + r.stderr[-500:])

    # Scheduled backups: due → backed up; again → unchanged, nothing new.
    put(os.path.join(home, '.config/nimbus/saves.json'), json.dumps({'schedule': 'daily', 'dest': usb, 'keep': 2}))
    r = run([exe, '--auto'], env)
    check('the scheduled backup runs when due', r.returncode == 0 and '2 backed up' in r.stdout, r.stdout + r.stderr)
    r = run([exe, '--auto', '--force'], env)
    check('a second run skips games whose saves didn\'t change', r.returncode == 0 and '0 backed up' in r.stdout and '2 unchanged' in r.stdout, r.stdout + r.stderr)
    r = run([exe, '--auto'], env)
    check('and isn\'t due again straight away', 'Not due' in r.stdout, r.stdout)
    auto = [f for dp, _, fs in os.walk(usb) for f in fs if '(automatic)' in f]
    check('automatic backups are marked as such', len(auto) == 2, auto)

    # The real list, from GitHub (needs the network: a warning, not a failure, if GitHub can't be reached).
    del env['HEARTH_SAVES_MANIFEST']
    env['XDG_CACHE_HOME'] = os.path.join(work, 'cache')
    t = time.time()
    r = run([exe, '--refresh-manifest'], env, timeout=600, ok_codes=(0, 1))
    info = as_json(r) or {}
    if info.get('ok'):
        check('the real save-location list downloads and reads (%d games, %.0f s)' % (info.get('games', 0), time.time() - t), info.get('games', 0) > 10000, info)
        t = time.time()
        r = run([exe, '--list'], env)
        lst = as_json(r) or {}
        g = {x['id']: x for x in lst.get('games', [])}
        check('with the real list, Hollow Knight\'s Proton save is found (%.1f s from the cache)' % (time.time() - t),
              (g.get('steam:367520', {}).get('saves') or {}).get('count', 0) >= 1, g.get('steam:367520'))
        check('with the real list, Celeste\'s native save is found', (g.get('steam:504230', {}).get('saves') or {}).get('count', 0) >= 1, g.get('steam:504230'))
    else:
        print('::warning title=Saves::the real save-location list couldn\'t be downloaded: %s' % (info.get('error') or r.stderr[-300:]))


# ---------------------------------------------------------------------- Mods

def mods(bindir, work):
    exe = os.path.join(bindir, 'hearth-mods')
    shutil.rmtree(work, ignore_errors=True)
    home, hk, _ = fake_home(os.path.join(work, 'pc'))
    env = env_for(home)
    g = 'steam:367520'
    z = os.path.join(work, 'Cool Mod v1.2.zip')
    with zipfile.ZipFile(z, 'w') as f:
        f.writestr('Cool Mod v1.2/Data/a.txt', 'mod one a')
        f.writestr('Cool Mod v1.2/Data/new/b.txt', 'mod one b')
    r = run([exe, '--list'], env)
    ids = [x['id'] for x in (as_json(r) or [])]
    check('Mods lists the game', g in ids, ids)
    r = run([exe, '--install', g, z], env)
    out = as_json(r) or {}
    mods1 = out.get('summary', {}).get('mods', [])
    check('a mod installs from a zip (its wrapping folder left out)', r.returncode == 0 and len(mods1) == 1 and mods1[0]['name'] == 'Cool Mod v1.2', r.stdout[-600:])
    check('its files are in the game folder', read(os.path.join(hk, 'Data/a.txt')) == 'mod one a' and read(os.path.join(hk, 'Data/new/b.txt')) == 'mod one b')
    m1 = mods1[0]['id'] if mods1 else 'm0'
    folder = os.path.join(work, 'Other Mod')
    put(os.path.join(folder, 'Data/a.txt'), 'mod two a')
    r = run([exe, '--install', g, folder, '--name', 'Other Mod'], env)
    out = as_json(r) or {}
    ms = out.get('summary', {}).get('mods', [])
    m2 = ms[1]['id'] if len(ms) > 1 else 'm1'
    check('a second mod from a folder wins the file they share (lower in the list)', read(os.path.join(hk, 'Data/a.txt')) == 'mod two a', read(os.path.join(hk, 'Data/a.txt')))
    check('the first one shows it was replaced', ms and ms[0].get('overriddenBy') == ['Other Mod'], ms[:1])
    run([exe, '--move', g, m2, '-1'], env)
    check('load order: moving it up lets the first win again', read(os.path.join(hk, 'Data/a.txt')) == 'mod one a', read(os.path.join(hk, 'Data/a.txt')))
    run([exe, '--disable', g, m1], env)
    run([exe, '--disable', g, m2], env)
    check('with both off, the game\'s own file is back', read(os.path.join(hk, 'Data/a.txt')) == 'original a', read(os.path.join(hk, 'Data/a.txt')))
    check('and the folder made for a mod is gone', not os.path.exists(os.path.join(hk, 'Data/new')))
    run([exe, '--enable', g, m1], env)
    check('on again', read(os.path.join(hk, 'Data/new/b.txt')) == 'mod one b')
    r = run([exe, '--remove', g, m1], env)
    check('removing a mod takes it out and leaves the game as it was', read(os.path.join(hk, 'Data/a.txt')) == 'original a' and not os.path.exists(os.path.join(hk, 'Data/new/b.txt')), r.stdout[-300:])
    r = run([exe, '--mods', g], env)
    check('one mod left in the list', len((as_json(r) or {}).get('mods', [])) == 1, r.stdout[-300:])
    files = sorted(os.path.relpath(os.path.join(dp, f), hk) for dp, _, fs in os.walk(hk) for f in fs)
    check('the game folder is exactly as it started', files == ['Data/a.txt', 'hollow_knight.exe', 'hollow_knight_Data/Config.ini'], files)
    serve(exe, env, g, work)


def ask(sock, call, args=None, timeout=60):
    import socket
    c = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    c.settimeout(timeout)
    c.connect(sock)
    c.sendall((json.dumps({'call': call, 'args': args or {}}) + '\n').encode())
    buf = b''
    while not buf.endswith(b'\n'):
        part = c.recv(65536)
        if not part:
            break
        buf += part
    c.close()
    return json.loads(buf.decode() or '{}')


def serve(exe, env, g, work):
    """Game Mode's way in (hearth-core hearth_mods.rs): the worker without a window, asked over its socket."""
    rt = os.path.join(work, 'run')
    os.makedirs(rt, mode=0o700, exist_ok=True)
    env = dict(env, XDG_RUNTIME_DIR=rt)
    sock = os.path.join(rt, 'hearth-mods.sock')
    p = subprocess.Popen([exe, '--serve'], env=env, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        for _ in range(100):
            if os.path.exists(sock):
                break
            time.sleep(0.1)
        check('the worker opens its socket (only you can use it)', os.path.exists(sock) and (os.stat(sock).st_mode & 0o777) == 0o600)
        r = ask(sock, 'mods_of', {'id': g})
        ms = (r.get('value') or {}).get('mods', [])
        check('Game Mode gets a game\'s mods from the worker', r.get('ok') and len(ms) == 1, r)
        if ms:
            r = ask(sock, 'toggle', {'id': g, 'm': ms[0]['id'], 'on': False})
            check('and turns one off through it', r.get('ok') and r['value']['summary']['mods'][0]['enabled'] is False, r)
            r = ask(sock, 'toggle', {'id': g, 'm': ms[0]['id'], 'on': True})
            check('and on again', r.get('ok') and r['value']['summary']['mods'][0]['enabled'] is True, r)
        r = ask(sock, 'status')
        check('the queue and installs in one call', r.get('ok') and 'downloads' in r['value'] and 'jobs' in r['value'], r)
        r = ask(sock, 'nope')
        check('an unknown call is an error, not a hang', r.get('ok') is False and 'nope' in r.get('error', ''), r)
        r2 = subprocess.run([exe, '--serve'], env=env, capture_output=True, timeout=30, stdin=subprocess.DEVNULL)
        check('a second worker doesn\'t take over', r2.returncode == 3, r2.returncode)
        check('the first one still answers', ask(sock, 'ping').get('ok') is True)
    finally:
        p.terminate()
        try:
            p.wait(10)
        except Exception:
            p.kill()


# ---------------------------------------------------------------------- Controller Lab

def pads(bindir):
    import threading
    from evdev import ecodes as E, AbsInfo, UInput, InputDevice, list_devices
    exe = os.path.join(bindir, 'hearth-controller-lab')

    def node(name):
        for _ in range(50):
            for p in list_devices():
                try:
                    if InputDevice(p).name == name:
                        os.chmod(p, 0o666)
                        return p
                except OSError:
                    pass
            time.sleep(0.1)
        return None

    keys = [E.BTN_SOUTH, E.BTN_EAST, E.BTN_NORTH, E.BTN_WEST, E.BTN_TL, E.BTN_TR, E.BTN_TL2, E.BTN_TR2, E.BTN_SELECT, E.BTN_START, E.BTN_MODE, E.BTN_THUMBL, E.BTN_THUMBR]
    stick, trig, hat = AbsInfo(128, 0, 255, 0, 0, 0), AbsInfo(0, 0, 255, 0, 0, 0), AbsInfo(0, -1, 1, 0, 0, 0)
    ds = UInput({E.EV_KEY: keys, E.EV_ABS: [(E.ABS_X, stick), (E.ABS_Y, stick), (E.ABS_RX, stick), (E.ABS_RY, stick), (E.ABS_Z, trig), (E.ABS_RZ, trig), (E.ABS_HAT0X, hat), (E.ABS_HAT0Y, hat)],
                 E.EV_FF: [E.FF_RUMBLE, E.FF_GAIN]}, name='Hearth Test DualSense', vendor=0x054c, product=0x0ce6, bustype=E.BUS_USB, version=0x110, phys='hearth-e2e-ds/input0')
    acc, gyro = AbsInfo(0, -32768, 32767, 16, 0, 8192), AbsInfo(0, -32768, 32767, 16, 0, 1024)
    motion = UInput({E.EV_ABS: [(E.ABS_X, acc), (E.ABS_Y, acc), (E.ABS_Z, acc), (E.ABS_RX, gyro), (E.ABS_RY, gyro), (E.ABS_RZ, gyro)]},
                    name='Hearth Test DualSense Motion Sensors', vendor=0x054c, product=0x0ce6, bustype=E.BUS_USB, version=0x110, phys='hearth-e2e-ds/input1', input_props=[E.INPUT_PROP_ACCELEROMETER])
    xs, xt = AbsInfo(0, -32768, 32767, 16, 128, 0), AbsInfo(0, 0, 1023, 0, 0, 0)
    xb = UInput({E.EV_KEY: keys[:6] + keys[8:], E.EV_ABS: [(E.ABS_X, xs), (E.ABS_Y, xs), (E.ABS_RX, xs), (E.ABS_RY, xs), (E.ABS_Z, xt), (E.ABS_RZ, xt), (E.ABS_HAT0X, hat), (E.ABS_HAT0Y, hat)]},
                 name='Hearth Test Xbox Pad', vendor=0x045e, product=0x028e, bustype=E.BUS_USB, version=0x110, phys='hearth-e2e-xb/input0')
    p_ds, p_mo, p_xb = node('Hearth Test DualSense'), node('Hearth Test DualSense Motion Sensors'), node('Hearth Test Xbox Pad')
    check('the virtual controllers are in /dev/input', all([p_ds, p_mo, p_xb]), [p_ds, p_mo, p_xb])

    # Rumble: answer the kernel's upload requests the way a real virtual pad's maker does, and see the effect played.
    got = {'upload': 0, 'play': 0}
    stop = threading.Event()

    def serve_ff():
        import select
        while not stop.is_set():
            r, _, _ = select.select([ds.fd], [], [], 0.1)
            if not r:
                continue
            for ev in ds.read():
                if ev.type == E.EV_UINPUT and ev.code == E.UI_FF_UPLOAD:
                    up = ds.begin_upload(ev.value); up.retval = 0; ds.end_upload(up); got['upload'] += 1
                elif ev.type == E.EV_UINPUT and ev.code == E.UI_FF_ERASE:
                    er = ds.begin_erase(ev.value); er.retval = 0; ds.end_erase(er)
                elif ev.type == E.EV_FF and ev.value == 1 and ev.code != E.FF_GAIN:
                    got['play'] += 1
    th = threading.Thread(target=serve_ff, daemon=True)
    th.start()

    def press():
        time.sleep(0.6)
        ds.write(E.EV_KEY, E.BTN_SOUTH, 1); ds.write(E.EV_KEY, E.BTN_NORTH, 1)
        ds.write(E.EV_ABS, E.ABS_X, 255); ds.write(E.EV_ABS, E.ABS_Y, 128); ds.write(E.EV_ABS, E.ABS_RZ, 200); ds.write(E.EV_ABS, E.ABS_HAT0Y, -1); ds.syn()
        motion.write(E.EV_ABS, E.ABS_Y, 8192); motion.write(E.EV_ABS, E.ABS_RX, 1024 * 90); motion.syn()
        xb.write(E.EV_KEY, 0x133, 1); xb.write(E.EV_ABS, E.ABS_Z, 1023); xb.syn()   # 0x133: what xpad sends for X
    threading.Thread(target=press, daemon=True).start()
    r = subprocess.run([exe, '--dump', '1500'], capture_output=True, text=True, timeout=60)
    d = as_json(r) or {}
    print('  dump: %s' % json.dumps(d)[:1500])
    ps = d.get('pads', [])
    check('two controllers (the motion sensors are part of the first, not a third)', len(ps) == 2, [p.get('name') for p in ps])
    a = next((p for p in ps if p.get('name') == 'Hearth Test DualSense'), {})
    x = next((p for p in ps if p.get('name') == 'Hearth Test Xbox Pad'), {})
    check('player numbers in the order they were connected', a.get('player') == 1 and x.get('player') == 2, [(p.get('name'), p.get('player')) for p in ps])
    b = a.get('buttons', {})
    check('PlayStation layout: ✕ (south) and △ (north) pressed', b.get('south') is True and b.get('north') is True and b.get('west') is False, b)
    check('D-pad up from the hat', b.get('up') is True, b)
    check('left stick right, all the way', abs(((a.get('sticks') or {}).get('l') or {}).get('x', 0) - 1.0) < 0.02, (a.get('sticks') or {}).get('l'))
    check('right trigger analogue value', abs(((a.get('triggers') or {}).get('r') or {}).get('v', 0) - 200 / 255) < 0.02, (a.get('triggers') or {}).get('r'))
    mo = a.get('motion') or {}
    check('motion sensors: 1 g on Y, 90 °/s on the gyro', abs((mo.get('accel') or [0, 0])[1] - 1.0) < 0.01 and abs((mo.get('gyro') or [0])[0] - 90) < 0.5, mo)
    check('it says it\'s a virtual pad', a.get('virtual') is True and 'virtual pad' in (a.get('driverLabel') or '').lower(), a.get('driverLabel'))
    check('rumble is reported for the pad that has it', a.get('rumble') is True and x.get('rumble') is False)
    xbtn = x.get('buttons', {})
    check('Xbox layout: X (0x133) is the left button', xbtn.get('west') is True and xbtn.get('north') is False, xbtn)
    check('Xbox trigger 0..1023', abs(((x.get('triggers') or {}).get('l') or {}).get('v', 0) - 1.0) < 0.01, (x.get('triggers') or {}).get('l'))

    r = subprocess.run([exe, '--rumble', p_ds or ''], capture_output=True, text=True, timeout=30)
    time.sleep(0.5)
    check('rumble: the effect is uploaded and played', r.returncode == 0 and got['upload'] >= 1 and got['play'] >= 1, (r.stdout, r.stderr, got))
    r = subprocess.run([exe, '--set-axis', p_ds or '', '0', '10', '245', '6'], capture_output=True, text=True, timeout=30)
    check('calibration: the stick\'s range and dead zone are set in the driver', (as_json(r) or {}) == {'min': 10, 'max': 245, 'flat': 6}, r.stdout + r.stderr)
    stop.set()
    for u in (ds, motion, xb):
        u.close()


def hold(seconds):
    """For the screenshot: a virtual DualSense (with motion sensors) that keeps moving for a while."""
    import math
    from evdev import ecodes as E, AbsInfo, UInput, InputDevice, list_devices
    keys = [E.BTN_SOUTH, E.BTN_EAST, E.BTN_NORTH, E.BTN_WEST, E.BTN_TL, E.BTN_TR, E.BTN_TL2, E.BTN_TR2, E.BTN_SELECT, E.BTN_START, E.BTN_MODE, E.BTN_THUMBL, E.BTN_THUMBR]
    stick, trig, hat = AbsInfo(128, 0, 255, 0, 0, 0), AbsInfo(0, 0, 255, 0, 0, 0), AbsInfo(0, -1, 1, 0, 0, 0)
    ds = UInput({E.EV_KEY: keys, E.EV_ABS: [(E.ABS_X, stick), (E.ABS_Y, stick), (E.ABS_RX, stick), (E.ABS_RY, stick), (E.ABS_Z, trig), (E.ABS_RZ, trig), (E.ABS_HAT0X, hat), (E.ABS_HAT0Y, hat)]},
                 name='DualSense Wireless Controller', vendor=0x054c, product=0x0ce6, bustype=E.BUS_USB, version=0x110, phys='hearth-shot/input0')
    acc, gyro = AbsInfo(0, -32768, 32767, 16, 0, 8192), AbsInfo(0, -32768, 32767, 16, 0, 1024)
    mo = UInput({E.EV_ABS: [(E.ABS_X, acc), (E.ABS_Y, acc), (E.ABS_Z, acc), (E.ABS_RX, gyro), (E.ABS_RY, gyro), (E.ABS_RZ, gyro)]},
                name='DualSense Wireless Controller Motion Sensors', vendor=0x054c, product=0x0ce6, bustype=E.BUS_USB, version=0x110, phys='hearth-shot/input1', input_props=[E.INPUT_PROP_ACCELEROMETER])
    time.sleep(1)
    for p in list_devices():
        try:
            if InputDevice(p).name.startswith('DualSense Wireless'):
                os.chmod(p, 0o666)
        except OSError:
            pass
    end = time.time() + seconds
    t = 0.0
    while time.time() < end:
        t += 0.03
        ds.write(E.EV_ABS, E.ABS_X, int(128 + 120 * math.cos(t * 2))); ds.write(E.EV_ABS, E.ABS_Y, int(128 + 120 * math.sin(t * 2)))
        ds.write(E.EV_ABS, E.ABS_RX, 134); ds.write(E.EV_ABS, E.ABS_RY, 125)
        ds.write(E.EV_ABS, E.ABS_Z, int(127 + 127 * math.sin(t))); ds.write(E.EV_ABS, E.ABS_RZ, 60)
        ds.write(E.EV_KEY, E.BTN_SOUTH, 1 if int(t * 2) % 2 else 0); ds.write(E.EV_KEY, E.BTN_TR, 1 if int(t) % 3 == 0 else 0); ds.syn()
        mo.write(E.EV_ABS, E.ABS_X, int(1500 * math.sin(t))); mo.write(E.EV_ABS, E.ABS_Y, 8192); mo.write(E.EV_ABS, E.ABS_RX, int(60000 * math.sin(t * 3))); mo.write(E.EV_ABS, E.ABS_RY, int(30000 * math.cos(t * 2))); mo.syn()
        time.sleep(0.03)
    ds.close(); mo.close()


if __name__ == '__main__':
    if sys.argv[1] == 'hold':
        hold(float(sys.argv[2]))
        sys.exit(0)
    what = sys.argv[1]
    if what == 'saves':
        saves(sys.argv[2], sys.argv[3])
    elif what == 'mods':
        mods(sys.argv[2], sys.argv[3])
    elif what == 'pads':
        pads(sys.argv[2])
    print('%d failure(s)' % len(FAILS))
    sys.exit(len(FAILS))
