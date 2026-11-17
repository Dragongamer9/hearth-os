"""hearth_askpass: what hearth-askpass (SSH_ASKPASS) and pinentry-hearth (GnuPG's pinentry) share (1.14): asking Mantel
for a passphrase in Hearth's look (AskPass.qml), and falling back to the classic programs when Mantel isn't there.

How the answer comes back: a private FIFO in $XDG_RUNTIME_DIR/hearth-askpass (0700, yours only) whose path goes to
Mantel with the question (quickshell ipc ... call askpass ask); Mantel writes "ok\\n<the passphrase>" or "cancel\\n"
into it and nothing else. The passphrase is never on a command line, in a file on a drive or a log. Anything that
goes wrong (no Mantel, no answer in 5 minutes, an error here) means "ask the classic way" or "cancelled", never a hang.
"""
import os
import secrets
import select
import shutil
import subprocess
import time

SHELL = os.path.join(os.environ.get('NIMBUS_SHARE') or '/usr/share/nimbus', 'desktop', 'shell')
TIMEOUT = 300


def runtime_dir():
    d = os.environ.get('XDG_RUNTIME_DIR') or '/run/user/%d' % os.getuid()
    return os.path.join(d, 'hearth-askpass')


def quickshell():
    return shutil.which('quickshell') or shutil.which('qs')


def ipc(args, timeout=5):
    qs = quickshell()
    if not qs or not os.path.isdir(SHELL):
        return 127, ''
    try:
        p = subprocess.run([qs, 'ipc', '-p', SHELL, 'call', 'askpass'] + args, capture_output=True, text=True, timeout=timeout)
        return p.returncode, (p.stdout or '').strip()
    except (OSError, subprocess.SubprocessError):
        return 127, ''


def mantel_here():
    """Mantel's askpass answers "pong" when it's running in this session."""
    if not (os.environ.get('WAYLAND_DISPLAY') or os.environ.get('DISPLAY')):
        return False
    code, out = ipc(['ping'], timeout=3)
    return code == 0 and out.endswith('pong')


def ask(kind, title, desc, prompt):
    """(True, answer) when the person answered, (False, '') when they cancelled; None when Mantel couldn't ask."""
    d = runtime_dir()
    try:
        os.makedirs(d, mode=0o700, exist_ok=True)
        os.chmod(d, 0o700)
        fifo = os.path.join(d, secrets.token_hex(12))
        os.mkfifo(fifo, 0o600)
    except OSError:
        return None
    try:
        fd = os.open(fifo, os.O_RDONLY | os.O_NONBLOCK)
        code, _ = ipc(['ask', fifo, kind, title[:200], desc[:2000], prompt[:300]])
        if code != 0:
            os.close(fd)
            return None
        data, writer_seen, end = b'', False, time.time() + TIMEOUT
        while time.time() < end:
            r, _, _ = select.select([fd], [], [], 1.0)
            if not r:
                continue
            chunk = os.read(fd, 4096)
            if chunk:
                data += chunk
                writer_seen = True
                if len(data) > 8192:
                    break
            elif writer_seen:
                break          # the writer closed: that's the whole answer
            else:
                time.sleep(0.1)    # no writer yet (a read of 0 before it opened)
        os.close(fd)
    finally:
        try:
            os.remove(fifo)
        except OSError:
            pass
    text = data.decode('utf-8', 'replace')
    if text.startswith('ok\n'):
        return True, text[3:].rstrip('\n')
    return False, ''
