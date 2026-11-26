#!/usr/bin/python3
"""The Hearth Store's login screen for real (store-start-parts.yml; docs/STORE-SYSTEM-PARTS.md "Starting up"): run as
root in a container with cage and Quickshell, the real hearth-login-screen choosing (its machine folder a test one,
systemctl a stand-in), the real hearth-system-part deciding from root-owned copies in /var/lib/hearth/system-parts,
and the real hearth-greeter drawing what greetd would start, on a virtual screen:
  1 a good Store copy: chosen, on screen (its ready mark);
  2 a copy broken on purpose: chosen, never on screen; the fallback (as hearth-login-watch's) starts SDDM, marks the
    copy bad and doesn't count against Hearth's login screen; the next start draws the built-in greeter, on screen.
Results as annotations."""
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import shlex
import shutil
import subprocess
import sys
import time
import types

REPO = pathlib.Path(__file__).resolve().parents[2]
PARTS = pathlib.Path("/var/lib/hearth/system-parts/hearth-greeter")
BUILTIN = pathlib.Path("/usr/share/hearth/greeter")
RT = pathlib.Path("/tmp/xdg")
failed = []


def ann(kind, text):
    print("::%s title=Login screen part (real)::%s" % (kind, text.replace("\n", "|")[:3500]), flush=True)


def greeter_files(dest):
    if dest.exists():
        shutil.rmtree(dest)
    shutil.copytree(REPO / "os/system_files/usr/share/hearth/greeter", dest)
    shell = REPO / "hosts/linux/desktop/shell"
    shutil.copy(shell / "OskBoard.qml", dest)
    shutil.copy(shell / "OskKey.qml", dest)
    (dest / "osk").mkdir(exist_ok=True)
    shutil.copy(shell / "osk/hearth-osk.js", dest / "osk")
    for p in [dest] + list(dest.rglob("*")):
        os.chmod(p, 0o755 if p.is_dir() or p.name == "greeter-info" else 0o644)


def put(version, broken=False):
    if PARTS.exists():
        shutil.rmtree(PARTS)
    d = PARTS / version / "greeter"
    greeter_files(d)
    if broken:
        (d / "shell.qml").write_text("import QtQuick\nShellRoot { this is broken on purpose\n")
    for p in (PARTS.parent.parent, PARTS.parent, PARTS, PARTS / version):
        os.chmod(p, 0o755)
    (PARTS / "part.json").write_text(json.dumps({"id": "hearth-greeter", "version": version, "os": "1.19", "path": "greeter"}))
    return str(d)


def module(root):
    loader = importlib.machinery.SourceFileLoader("hls", str(REPO / "os/system_files/usr/libexec/nimbus/hearth-login-screen"))
    spec = importlib.util.spec_from_loader("hls", loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)

    class Pwd:
        def getpwnam(self, n):
            if n not in ("_greetd", "alex"):
                raise KeyError(n)
            return types.SimpleNamespace(pw_name=n, pw_uid=990 if n == "_greetd" else 1000)
    m.pwd = Pwd()
    return m


def show(cmd, label):
    """Runs the greeter greetd would start (its command line from greetd.toml); True once it's on screen."""
    for f in RT.glob("hearth-greeter-*"):
        f.unlink()
    log = open("/tmp/greeter-%s.log" % label, "w")
    p = subprocess.Popen(cmd, stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    on = False
    for _ in range(80):
        if (RT / "hearth-greeter-ready").exists():
            on = True
            break
        if p.poll() is not None:
            break
        time.sleep(0.5)
    subprocess.run(["pkill", "-s", str(p.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        os.killpg(p.pid, 9)
    except OSError:
        pass
    p.wait()
    log.close()
    return on, pathlib.Path("/tmp/greeter-%s.log" % label).read_text(errors="replace")[-1500:]


def command(root):
    conf = (root / "run/hearth/greetd.toml").read_text()
    line = [l for l in conf.splitlines() if l.startswith("command = ")][0]
    words = shlex.split(json.loads(line[len("command = "):]))
    words[0] = str(REPO / "os/system_files/usr/libexec/nimbus/hearth-greeter")
    return words


def main():
    root = pathlib.Path("/tmp/lroot")
    if root.exists():
        shutil.rmtree(root)
    for p in ("usr/sbin/greetd", "usr/bin/cage", "usr/bin/quickshell", "usr/libexec/nimbus/hearth-greeter", "usr/libexec/nimbus/hearth-session-start"):
        f = root / p
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text("#!/bin/sh\n")
        f.chmod(0o755)
    for rel, text in (("usr/share/hearth/greeter/shell.qml", "x"), ("etc/pam.d/greetd", "auth include x\n"), ("proc/cmdline", "quiet\n"),
                      ("proc/sys/kernel/random/boot_id", "boot-1\n"), ("etc/hearth/login-screen.json", '{"screen": "hearth"}\n')):
        f = root / rel
        f.parent.mkdir(parents=True, exist_ok=True)
        f.write_text(text)
    for s in ("nimbus.desktop", "hearth-classic.desktop"):
        f = root / "usr/share/wayland-sessions" / s
        f.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy(REPO / "hosts/linux/session" / s, f)
    sysctl = pathlib.Path("/tmp/systemctl")
    sysctl.write_text('#!/bin/sh\necho "$*" >> /tmp/systemctl.log\ncase "$1" in is-system-running) echo running;; is-active) echo active;; esac\nexit 0\n')
    sysctl.chmod(0o755)
    pathlib.Path("/tmp/version").write_text("1.19\n")
    os.environ.update(HEARTH_LOGIN_ROOT=str(root), HEARTH_SETTINGS_IO_DIR=str(REPO / "hosts/linux/desktop/shell"),
                      HEARTH_LOGIN_SYSTEMCTL=str(sysctl), HEARTH_LOGIN_SYSTEM_PART=str(REPO / "os/system_files/usr/libexec/hearth/hearth-system-part"),
                      HEARTH_SYSTEM_PARTS_ALLOWED=str(REPO / "os/system_files/usr/share/hearth/store/system-parts.json"),
                      HEARTH_VERSION_FILE="/tmp/version", HEARTH_SYSTEM_PARTS_CATALOG="/tmp/none.json")
    greeter_files(BUILTIN)        # the built-in greeter where the image has it
    m = module(root)

    # 1. a good copy
    d = put("1.19.1")
    m.cmd_select()
    cmd = command(root)
    if cmd[1:] != [d]:
        failed.append("a good copy wasn't chosen: %s" % cmd)
    on, log = show(cmd, "good")
    ann("notice" if on else "error", "a good Store copy (%s): %s" % (d, "on screen" if on else "not on screen: " + log))
    if not on:
        failed.append("the good copy didn't show")
    else:
        m.mark_good("the greeter showed")

    # 2. broken on purpose
    (root / "proc/sys/kernel/random/boot_id").write_text("boot-2\n")
    for f in ("login-greetd", "greetd.toml", "login-fell-back", "login-mode.json"):
        try:
            (root / "run/hearth" / f).unlink()
        except FileNotFoundError:
            pass
    d = put("1.19.2", broken=True)
    m.cmd_select()
    cmd = command(root)
    on, log = show(cmd, "broken")
    if on or cmd[1:] != [d]:
        failed.append("the broken copy: chosen %s, on screen %s" % (cmd[1:], on))
    m.cmd_fallback("it didn't show within 40 s")
    st = json.loads((root / "var/lib/hearth/login-screen-state.json").read_text())
    bad = json.loads((PARTS / "state.json").read_text()).get("bad", []) if (PARTS / "state.json").exists() else []
    calls = pathlib.Path("/tmp/systemctl.log").read_text()
    ann("notice", "broken on purpose: on screen %s · SDDM started %s · marked bad %s · failures counted %s · greeter said: %s"
        % (on, "start --no-block sddm.service" in calls, bad, st.get("failures"), log[-600:]))
    if "start --no-block sddm.service" not in calls:
        failed.append("SDDM wasn't started")
    if bad != ["1.19.2"]:
        failed.append("the broken copy wasn't marked bad: %s" % bad)
    if int(st.get("failures") or 0) != 0:
        failed.append("the copy's failure counted against Hearth's login screen")

    # the next start: the built-in greeter, on screen
    (root / "proc/sys/kernel/random/boot_id").write_text("boot-3\n")
    for f in ("login-greetd", "greetd.toml", "login-fell-back", "login-mode.json"):
        try:
            (root / "run/hearth" / f).unlink()
        except FileNotFoundError:
            pass
    m.cmd_select()
    cmd = command(root)
    if cmd[1:]:
        failed.append("the next start didn't use the built-in greeter: %s" % cmd)
    on, log = show(cmd, "builtin")
    ann("notice" if on else "error", "the next start, the built-in greeter: %s" % ("on screen" if on else "not on screen: " + log))
    if not on:
        failed.append("the built-in greeter didn't show after the fallback")
    for f in failed:
        ann("error", f)
    if not failed:
        ann("notice", "passed: the good copy showed; the broken one fell back to SDDM, was marked bad without pausing Hearth's login screen, and the built-in greeter showed at the next start")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
