#!/usr/bin/env python3
"""hearth-proton-run: how Hearth OS starts Windows games and programs with Proton, 100% offline and without Steam
(docs/WINDOWS-APPS.md, "Proton: offline and without Steam"). The image has it as /usr/bin/hearth-proton-run.

A drop-in for umu-run's command line, so everything that used umu-run (Hearth Games' GOG, Epic and Steam-engine
games, games you added yourself, Hearth Recomps' Windows builds, Hearth's own Battle.net setup, Hearth Mods' Play
modded, Mod Organizer 2 and Repair) starts the same way, without umu-launcher's downloads:

    hearth-proton-run <program.exe> [arguments]      runs it in WINEPREFIX with Proton
    hearth-proton-run winetricks [verbs]             GE-Proton's own winetricks in WINEPREFIX (downloads Windows parts)
    hearth-proton-run --print-proton                 which Proton a start would use, and why

It reads what umu-run reads, the way umu-run reads it (so prefixes umu made carry on unchanged): WINEPREFIX (default
~/Games/<STORE>, else ~/Games/umu/<GAMEID>), PROTONPATH (a folder, or a name such as
"GE-Proton", "GE-Proton11-3" or "UMU-Proton"; a pick that isn't on this device, or none, means the GE-Proton built into
Hearth OS), GAMEID (Proton's game fixes; default umu-default), STORE, PROTON_VERB, STEAM_COMPAT_INSTALL_PATH, and any
Proton or Wine variable, which pass through.

What it never does: download anything (no umu-launcher, no Steam Linux Runtime, no UMU-Proton), use or need Steam or a
Steam folder (~/.local/share/Steam may not exist), or let Proton's own Python reach the internet: GE-Proton's
protonfixes would look for AMD's FSR4 files online at every start, run winetricks downloads some game fixes ask for,
and fetch files for a few others. Here those find no network (block_network) and carry on without them, the way
protonfixes does on a PC with no internet; files it cached before are still used. The game itself is not limited.
The one exception is the explicit `winetricks` command (Hearth Mods' Repair): winetricks downloads the Windows parts
you asked for, which needs the internet by nature; Proton itself still doesn't.
"""
import errno
import os
import time
import re
import sys

SHELL = os.path.dirname(os.path.realpath(__file__))
OFFLINE = 'Hearth runs Proton offline: no internet from Proton itself'
VERBS = ('waitforexitandrun', 'run', 'runinprefix')


class Fail(Exception):
    pass


def log(msg):
    try:
        print('hearth-proton-run: ' + msg, file=sys.stderr, flush=True)
    except (OSError, ValueError):
        pass


# ---- inside Proton's own Python (--in-proton)
def _local(host):
    """This PC itself (loopback), which Proton may still talk to."""
    if isinstance(host, bytes):
        host = host.decode('ascii', 'replace')
    return host is None or (isinstance(host, str) and (host in ('', 'localhost', '::1') or host.startswith('127.')))


def block_network():
    """Switches the internet off for this Python process only (Proton's script and protonfixes): connecting to
    anything but this PC fails at once, and so does looking a name up, before any packet leaves. What protonfixes does
    then is what it does on a PC without internet: no FSR4 download (its cached files are still used), winetricks for
    game fixes skipped ("No internet connection"), other downloads fail and are logged. Wine and the game are separate
    processes and aren't affected."""
    import socket

    def refuse():
        return OSError(errno.ENETUNREACH, OFFLINE)

    real_connect, real_connect_ex = socket.socket.connect, socket.socket.connect_ex

    def connect(self, address):
        if self.family in (socket.AF_INET, socket.AF_INET6) and not _local(address[0] if isinstance(address, tuple) else address):
            raise refuse()
        return real_connect(self, address)

    def connect_ex(self, address):
        if self.family in (socket.AF_INET, socket.AF_INET6) and not _local(address[0] if isinstance(address, tuple) else address):
            return errno.ENETUNREACH
        return real_connect_ex(self, address)

    real_gai = socket.getaddrinfo

    def getaddrinfo(host, *a, **k):
        if _local(host):
            return real_gai(host, *a, **k)
        raise socket.gaierror(socket.EAI_NONAME, OFFLINE)

    def gethostbyname(host):
        if _local(host):
            return '127.0.0.1'
        raise socket.gaierror(socket.EAI_NONAME, OFFLINE)

    def gethostbyname_ex(host):
        if _local(host):
            return ('localhost', [], ['127.0.0.1'])
        raise socket.gaierror(socket.EAI_NONAME, OFFLINE)

    socket.socket.connect = connect
    socket.socket.connect_ex = connect_ex
    socket.getaddrinfo = getaddrinfo
    socket.gethostbyname = gethostbyname
    socket.gethostbyname_ex = gethostbyname_ex
    # A proxy on this PC (HTTPS_PROXY=127.0.0.1:…) would otherwise carry urllib's requests out: web requests from
    # Proton's Python are refused before they start, whatever the proxy settings (file:// still works).
    import http.client
    import urllib.error
    import urllib.request
    real_open = urllib.request.OpenerDirector.open

    def opener_open(self, fullurl, *a, **k):
        url = fullurl if isinstance(fullurl, str) else getattr(fullurl, 'full_url', '')
        if not str(url).lower().startswith('file:'):
            raise urllib.error.URLError(OFFLINE)
        return real_open(self, fullurl, *a, **k)

    def http_connect(self):
        raise refuse()

    urllib.request.OpenerDirector.open = opener_open
    http.client.HTTPConnection.connect = http_connect


def run_in_proton(proton_dir, argv):
    """Runs <proton_dir>/proton with `argv` (a verb and its arguments) in this process, unchanged, with the network off.
    Never returns: Proton's script ends with sys.exit."""
    import runpy
    block_network()
    script = os.path.join(proton_dir, 'proton')
    if not os.path.isfile(script):
        raise Fail("%s has no proton script" % proton_dir)
    sys.argv = [script] + list(argv)
    # Proton imports its own modules (protonfixes, filelock, utilities) from its folder, as when it's run directly.
    sys.path[:] = [proton_dir] + [p for p in sys.path if p and os.path.realpath(p) != SHELL]
    runpy.run_path(script, run_name='__main__')


# ---- a start (umu-run's command line)
def _home():
    return os.environ.get('HOME') or os.path.expanduser('~')


def link_users(prefix):
    """The same user folders umu-launcher sets up: drive_c/users/<you> and steamuser, one a link to the other, so saves
    in prefixes umu made before stay where the game looks for them."""
    try:
        import pwd
        user = pwd.getpwuid(os.getuid()).pw_name
    except (ImportError, KeyError):
        user = os.environ.get('USER') or ''
    if not user or user == 'steamuser':
        return
    users = os.path.join(prefix, 'pfx', 'drive_c', 'users')
    mine, steam = os.path.join(users, user), os.path.join(users, 'steamuser')
    try:
        if not os.path.exists(mine) and not os.path.exists(steam):
            os.makedirs(steam, exist_ok=True)
            if not os.path.lexists(mine):
                os.symlink('steamuser', mine)
        elif os.path.isdir(mine) and not os.path.exists(steam) and not os.path.lexists(steam):
            os.symlink(user, steam)
        elif not os.path.exists(mine) and not os.path.lexists(mine) and os.path.isdir(steam):
            os.symlink('steamuser', mine)
    except OSError as e:
        log("couldn't link the prefix's user folders (%s); Proton sets them up itself" % e)


def prepare_prefix(prefix):
    """The prefix exactly as umu-launcher prepares it (setup_pfx), so prefixes umu made before carry on unchanged:
    the folder, Proton's `pfx` in it pointing at itself (unless pfx is a real folder), shadercache, gstreamer-1.0,
    tracked_files, and the user folders."""
    os.makedirs(prefix, exist_ok=True)
    for d in ('shadercache', 'gstreamer-1.0'):
        os.makedirs(os.path.join(prefix, d), exist_ok=True)
    pfx = os.path.join(prefix, 'pfx')
    if os.path.islink(pfx):
        os.unlink(pfx)
    if not os.path.isdir(pfx):
        os.symlink('.', pfx)
    with open(os.path.join(prefix, 'tracked_files'), 'a'):
        pass
    link_users(prefix)


def plan(exe, args, environ, cwd=None):
    """(command, environment, what it says) for a start, without starting anything (the tests use it)."""
    import hearth_proton as hp
    gameid = (environ.get('GAMEID') or '').strip() or 'umu-default'
    if environ.get('WINEPREFIX') == '':
        raise Fail('WINEPREFIX is set but empty')
    # As umu-run: WINEPREFIX, else ~/Games/<STORE>, else ~/Games/umu/<GAMEID>; Proton's data folder is the prefix itself.
    default = os.path.join(_home(), 'Games', environ['STORE']) if environ.get('STORE') else os.path.join(_home(), 'Games', 'umu', gameid)
    prefix = os.path.realpath(os.path.abspath(os.path.expanduser(environ.get('WINEPREFIX') or default)))
    r = hp.resolve(environ.get('PROTONPATH'))
    if not r:
        raise Fail("There's no Proton on this device. Hearth OS has GE-Proton built in (/usr/share/hearth/proton); "
                   "this PC's image doesn't.")
    name, path, note = r
    app = gameid[gameid.find('-') + 1:] if re.match(r'^umu-\w+$', gameid) else '0'  # umu's SteamAppId
    verb = environ.get('PROTON_VERB') if environ.get('PROTON_VERB') in VERBS else 'waitforexitandrun'
    cwd = cwd or os.getcwd()
    if exe == 'winetricks':
        target = os.path.join(path, 'protonfixes', 'winetricks')
        if not os.path.isfile(target):
            raise Fail("%s has no winetricks of its own (protonfixes/winetricks)" % name)
        verb = 'waitforexitandrun'
    elif os.path.exists(os.path.join(cwd, os.path.expanduser(exe))):
        target = os.path.abspath(os.path.join(cwd, os.path.expanduser(exe)))
    else:
        target = exe  # as umu-run: a Windows command (winecfg, msiexec, cmd...) or something inside the prefix
    found = exe != 'winetricks' and os.path.isabs(target)
    install = environ.get('STEAM_COMPAT_INSTALL_PATH') or (os.path.dirname(target) if found else '')
    env = hp.offline_env(environ, prefix, umu_id=gameid, app_id=app, install_dir=install)
    env.update({'GAMEID': gameid, 'PROTONPATH': path, 'EXE': target, 'PROTON_VERB': verb,
                'STEAM_COMPAT_TOOL_PATHS': path, 'STEAM_COMPAT_SHADER_PATH': os.path.join(prefix, 'shadercache')})
    if exe == 'winetricks' and environ.get('UMU_LOG') not in ('debug', '1'):
        env['WINETRICKS_SUPER_QUIET'] = '1'
    cmd = hp.proton_argv(path, verb, [target] + list(args))
    said = ['%s with %s (%s)%s' % (exe if exe == 'winetricks' else target, name, path, ': ' + note if note else ''),
            'prefix %s, game id %s, store %s; offline and without Steam (no umu-launcher, no Steam Runtime)' % (
                prefix, gameid, environ.get('STORE') or 'none')]
    return cmd, env, said


def main(argv):
    if argv[:1] == ['--in-proton'] and len(argv) >= 3:
        run_in_proton(argv[1], argv[2:])
        return 0
    if argv[:1] in (['-h'], ['--help']):
        print(__doc__)
        return 0
    if not argv:
        log('usage: hearth-proton-run <program.exe> [arguments] | winetricks [verbs] | --print-proton')
        return 2
    sys.path.insert(0, SHELL)
    import hearth_proton as hp
    if argv[:1] == ['--print-proton']:
        r = hp.resolve(os.environ.get('PROTONPATH'))
        if not r:
            print('none')
            return 1
        print('%s\t%s%s' % (r[0], r[1], '\t' + r[2] if r[2] else ''))
        return 0
    cmd, env, said = plan(argv[0], argv[1:], os.environ)
    for line in said:
        log(line)
    prepare_prefix(env['WINEPREFIX'])
    key = (os.environ.get('HEARTH_PROTON_KEY') or '').strip()
    if not key:
        os.execvpe(cmd[0], cmd, env)
        return 127
    return watch(cmd, env, key, hp)


QUICK_SECS = 15        # closed this soon with an error: that Proton didn't work for it
GOOD_SECS = 90         # ran this long: it works


def watch(cmd, env, key, hp, now=time.monotonic, popen=None):
    """The Proton detector (1.11): a game Hearth picked the Proton for is started as a child and watched. Closed right
    away with an error → noted as failed (the next Proton is tried first next time, and you're told); ran a while →
    remembered as the one that works. Signals are passed on, and its exit code is the game's."""
    import signal
    import subprocess
    popen = popen or subprocess.Popen
    name = os.path.basename((env.get('PROTONPATH') or '').rstrip('/'))
    t0 = now()
    p = popen(cmd, env=env)
    for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
        try:
            signal.signal(sig, lambda s, f: p.send_signal(s))
        except (ValueError, OSError):
            pass
    code = p.wait()
    took = now() - t0
    try:
        if code != 0 and took < QUICK_SECS and name:
            hp.note_failed(key, name)
            nxt = next((c['name'] for c in hp.candidates(key) if c['name'] != name and c.get('path')), '')
            log('%s closed right away with %s (code %s)%s' % (key, name, code, ': next time %s first' % nxt if nxt else ''))
            try:
                subprocess.Popen(['notify-send', '-a', 'Hearth', '-i', 'dialog-information', 'The game closed right away',
                                  'It didn\'t start with %s.%s' % (name, ' Next time Hearth tries %s first. Start it again to try.' % nxt if nxt else '')],
                                 stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except OSError:
                pass
        elif took >= GOOD_SECS and name:
            hp.note_worked(key, name)
    except Exception as e:  # noqa: BLE001 - noting never changes how the game ended
        log('proton detector: %s' % e)
    return code


if __name__ == '__main__':
    try:
        sys.exit(main(sys.argv[1:]))
    except Fail as e:
        log(str(e))
        sys.exit(1)
