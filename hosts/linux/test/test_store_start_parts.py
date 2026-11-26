"""gamescope, the session start and Hearth's own login screen as Hearth Store parts (the owner, Nov 26: "Yep add it.
That will allow us to push updates to it"; docs/STORE-SYSTEM-PARTS.md "Starting up").

Red flag 2 (an update must never leave a device unable to start, freeze or lag), checked with the real code:
  * without a Store copy nothing changes: the built-in program starts, exactly as before;
  * a Store copy that ends with an error, never comes up, or whose start before never got as far as "good" is stopped,
    the built-in one starts at once with the same request, and that copy is never used again (until another version
    or Try it again), and the person is told (the session log and a notice that waits for the desktop);
  * an end that was asked for (signed out, shut down, a switch) is never counted as a failure;
  * nothing changes by itself without telling: the only change is the copy's own failure mark, with a way back;
  * the login screen's copy is a root-owned system part: one that doesn't work is marked bad, SDDM takes that start,
    the built-in greeter the next, and it never counts towards pausing Hearth's login screen.
"""
import sys
sys.dont_write_bytecode = True
import importlib.machinery
import importlib.util
import json
import os
import pathlib
import re
import shutil
import signal
import subprocess
import tempfile
import time
import unittest
from unittest import mock

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[2]
PART = REPO / "os/system_files/usr/libexec/hearth/hearth-part"
SYSPART = REPO / "os/system_files/usr/libexec/hearth/hearth-system-part"
SESSION = REPO / "hosts/linux/session/nimbus-session"
GREETER = REPO / "os/system_files/usr/libexec/nimbus/hearth-greeter"
ALLOWED = REPO / "os/system_files/usr/share/hearth/store/system-parts.json"
QUICK = REPO / ".github/workflows/quick-update.yml"
CATALOG = REPO / "os/system_files/usr/share/hearth/store/catalog.json"


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_loader(name, loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    return m


def block(name):
    text = SESSION.read_text()
    m = re.search(r"^# >>> %s\n(.*?)^# <<< %s\n" % (name, name), text, re.M | re.S)
    assert m, name
    return m.group(1)


class Base(unittest.TestCase):
    """A person's home with the real hearth-part (as a person's account, even when CI runs as root)."""

    def setUp(self):
        self.t = pathlib.Path(tempfile.mkdtemp())
        (self.t / "version").write_text("1.19\n")
        (self.t / "bin").mkdir()
        self.out = self.t / "out.txt"
        self.notes = self.t / "notices.txt"
        # notify-send answers (a desktop is up): the notice reaches the person.
        ns = self.t / "bin/notify-send"
        ns.write_text('#!/bin/sh\nshift 3 2>/dev/null; echo "$*" >> %s\n' % self.notes)
        ns.chmod(0o755)
        self.builtin = self.t / "builtin"
        self.builtin.write_text('#!/bin/sh\necho "BUILTIN $* part=${HEARTH_SESSION_PART:-}" >> %s\n' % self.out)
        self.builtin.chmod(0o755)
        self.env = dict(os.environ, HEARTH_PARTS=str(self.t / "parts"), HEARTH_PARTS_STATE=str(self.t / "state"),
                        XDG_RUNTIME_DIR=str(self.t / "run"), HEARTH_VERSION_FILE=str(self.t / "version"),
                        HEARTH_SESSION_LOG=str(self.t / "session.log"), PATH=str(self.t / "bin") + ":" + os.environ["PATH"],
                        HEARTH_PART_UP_SECS="2", HEARTH_PART_GOOD_SECS="1")
        self.env.pop("HEARTH_SESSION_PART", None)
        # A runner: the real hearth-part, as a person's account (it never uses a home copy as root).
        self.runner = self.t / "hearth-part"
        self.runner.write_text("#!/bin/sh\nexec %s -c 'import importlib.machinery, importlib.util, sys\n"
                               "l = importlib.machinery.SourceFileLoader(\"hp\", sys.argv[1])\n"
                               "s = importlib.util.spec_from_loader(\"hp\", l); m = importlib.util.module_from_spec(s); l.exec_module(m)\n"
                               "m._euid = lambda: 1000\nsys.exit(m.main([\"hearth-part\"] + sys.argv[2:]))' %s \"$@\"\n"
                               % (sys.executable, PART))
        self.runner.chmod(0o755)
        # `id -u` answers as a person's account (the session's lines never use a Store copy as root)
        (self.t / "bin/id").write_text('#!/bin/sh\n[ "$1" = -u ] && { echo 1000; exit 0; }\nexec /usr/bin/id "$@"\n')
        (self.t / "bin/id").chmod(0o755)

    def tearDown(self):
        # anything a test left running
        subprocess.run(["pkill", "-f", str(self.t)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        shutil.rmtree(self.t, ignore_errors=True)

    def put(self, pid="gamescope", version="1.19.1", body="#!/bin/sh\nexit 0\n", name="prog", made_for="1.19", extra=None):
        d = self.t / "parts" / pid / version
        d.mkdir(parents=True, exist_ok=True)
        p = d / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(body)
        p.chmod(0o755)
        for n, b in (extra or {}).items():
            (d / n).write_text(b)
            (d / n).chmod(0o755)
        (self.t / "parts" / pid / "part.json").write_text(json.dumps({"id": pid, "version": version, "os": made_for, "path": name}))
        return p

    def state(self, pid="gamescope"):
        try:
            return json.loads((self.t / "state" / (pid + ".json")).read_text())
        except FileNotFoundError:
            return {}

    def trial(self, pid="gamescope", up="children", args=("--a", "b"), timeout=40, wait=True):
        cmd = [str(self.runner), "trial", pid, str(self.builtin), "--up", up, "--"] + list(args)
        if not wait:
            return subprocess.Popen(cmd, env=self.env)
        return subprocess.run(cmd, env=self.env, timeout=timeout, capture_output=True, text=True)

    def said(self):
        return self.out.read_text().splitlines() if self.out.exists() else []

    def wait_notice(self):
        for _ in range(40):
            if self.notes.exists() and self.notes.read_text().strip():
                return self.notes.read_text()
            time.sleep(0.1)
        return ""



class Trial(Base):
    """hearth-part trial, run for real in a child process."""

    # ---- without a Store copy: exactly as before
    def test_no_store_copy_starts_the_built_in_one(self):
        r = self.trial()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["BUILTIN --a b part="])
        self.assertFalse((self.t / "state").exists(), "nothing is written without a Store copy")

    def test_a_copy_for_another_hearth_os_is_never_tried(self):
        self.put(made_for="1.18", body="#!/bin/sh\necho STORE >> %s\n" % self.out)
        self.trial()
        self.assertEqual(self.said(), ["BUILTIN --a b part="])

    # ---- a copy that works
    def test_a_copy_that_comes_up_is_used_and_marked_good(self):
        self.put(body='#!/bin/sh\necho "STORE $*" >> %s\nsleep 3 &\nwait\n' % self.out)
        r = self.trial()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["STORE --a b"])
        st = self.state()
        self.assertNotIn("pending", st)
        self.assertEqual(int(st.get("fails", 0)), 0)
        self.assertIn("working", (self.t / "session.log").read_text())

    # ---- copies that fail: the built-in one at once, and never that copy again
    def test_a_copy_that_ends_with_an_error_falls_back_at_once_and_for_good(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\nexit 3\n' % self.out)
        r = self.trial()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["STORE", "BUILTIN --a b part="], "the built-in one, with the same request")
        st = self.state()
        self.assertGreaterEqual(st["fails"], 2, "never used again")
        self.assertIn("ended right away", st["why"])
        self.assertIn("Try it again", self.wait_notice(), "the person is told, with the way back")
        self.trial()
        self.assertEqual(self.said()[-1], "BUILTIN --a b part=")
        self.assertEqual(self.said().count("STORE"), 1, "a copy that failed is never started again")

    def test_a_copy_that_never_comes_up_is_stopped_and_the_built_in_one_starts(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\nexec sleep 60\n' % self.out)
        t0 = time.monotonic()
        r = self.trial()
        self.assertLess(time.monotonic() - t0, 20)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["STORE", "BUILTIN --a b part="])
        self.assertIn("didn't come up", self.state()["why"])
        left = subprocess.run(["pgrep", "-f", "sleep 60"], capture_output=True, text=True).stdout.split()
        self.assertFalse([p for p in left if str(self.t) in pathlib.Path(f"/proc/{p}/cmdline").read_text(errors="replace")], "the hung copy is gone")

    def test_a_start_before_that_never_came_up_means_the_built_in_one(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\nsleep 3 &\nwait\n' % self.out)
        (self.t / "state").mkdir()
        # the PC was switched off on a black screen with this copy on trial (its trial is long gone)
        (self.t / "state/gamescope.json").write_text(json.dumps({"version": "1.19.1", "fails": 0, "pending": {"version": "1.19.1", "pid": 999999, "at": 1}}))
        self.trial()
        self.assertEqual(self.said(), ["BUILTIN --a b part="])
        self.assertIn("never came up", self.state()["why"])
        self.assertGreaterEqual(self.state()["fails"], 2)

    def test_another_start_still_on_trial_is_left_alone(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\n' % self.out)
        fake = self.t / "x" / "hearth-part"
        fake.parent.mkdir()
        fake.write_text("#!/bin/sh\nsleep 20\n")
        fake.chmod(0o755)
        other = subprocess.Popen([str(fake), "trial", "gamescope"])
        try:
            time.sleep(0.3)
            (self.t / "state").mkdir()
            (self.t / "state/gamescope.json").write_text(json.dumps({"version": "1.19.1", "fails": 0, "pending": {"version": "1.19.1", "pid": other.pid, "at": 1}}))
            self.trial()
            self.assertEqual(self.said(), ["BUILTIN --a b part="])
            self.assertEqual(int(self.state().get("fails", 0)), 0, "not counted as a failure")
        finally:
            other.kill()
            other.wait()

    def test_a_crash_soon_after_coming_up_counts(self):
        self.env["HEARTH_PART_GOOD_SECS"] = "8"
        self.put(body='#!/bin/sh\necho STORE >> %s\nsleep 5 &\nsleep 1.2\nkill -SEGV $$\n' % self.out)
        self.trial()
        self.assertIn("crashed", self.state().get("why", ""))
        self.trial()
        self.assertEqual(self.said()[-1], "BUILTIN --a b part=")

    def test_an_end_that_was_asked_for_is_not_a_failure(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\nexec sleep 60\n' % self.out)
        self.env["HEARTH_PART_UP_SECS"] = "30"
        p = self.trial(wait=False)
        time.sleep(1.0)
        p.send_signal(signal.SIGTERM)          # signed out / shut down / a switch of modes
        p.wait(timeout=15)
        st = self.state()
        self.assertNotIn("pending", st)
        self.assertEqual(int(st.get("fails", 0)), 0)
        self.assertNotIn("BUILTIN --a b part=", self.said())

    def test_try_it_again_uses_the_copy_again(self):
        self.put(body='#!/bin/sh\necho STORE >> %s\nexit 3\n' % self.out)
        self.trial()
        subprocess.run([str(self.runner), "retry", "gamescope"], env=self.env, check=True)
        self.trial()
        self.assertEqual(self.said().count("STORE"), 2)

    def test_a_program_whose_libraries_are_missing_is_never_started(self):
        p = self.put()
        shutil.copy("/bin/true", p)
        # this Hearth OS lacks one of its libraries (what ldd says then)
        (self.t / "bin/ldd").write_text('#!/bin/sh\necho "\tlibliftoff.so.0 => not found"\necho "\tlibc.so.6 => /lib/libc.so.6"\n')
        (self.t / "bin/ldd").chmod(0o755)
        self.trial()
        self.assertEqual(self.said(), ["BUILTIN --a b part="])
        self.assertIn("hasn't got what it needs: libliftoff.so.0", self.state()["why"])

    def test_a_program_that_cant_run_here_falls_back(self):
        self.put(body="\x7fELF this is not a whole program\n")
        self.trial()
        self.assertEqual(self.said(), ["BUILTIN --a b part="])
        self.assertGreaterEqual(self.state()["fails"], 2)

    def test_the_session_counts_as_up_when_one_of_hearths_screens_starts(self):
        self.put(pid="hearth-session", name="nimbus-session",
                 body='#!/bin/sh\necho STORE >> %s\n"$(dirname "$0")/labwc" &\nwait\n' % self.out,
                 extra={"labwc": "#!/bin/sh\nsleep 3\n"})
        r = self.trial(pid="hearth-session", up="session")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["STORE"])
        self.assertNotIn("pending", self.state("hearth-session"))
        self.assertEqual(int(self.state("hearth-session").get("fails", 0)), 0)

    def test_a_session_that_brings_no_screen_up_falls_back(self):
        self.put(pid="hearth-session", name="nimbus-session", body='#!/bin/sh\necho STORE >> %s\nsleep 2 &\nexec sleep 60\n' % self.out)
        self.trial(pid="hearth-session", up="session")
        self.assertEqual(self.said(), ["STORE", "BUILTIN --a b part="])
        self.assertIn("didn't come up", self.state("hearth-session")["why"])

    def test_a_request_it_cant_read_still_starts_the_built_in_one(self):
        r = subprocess.run([str(self.runner), "trial", "gamescope", str(self.builtin), "--bogus", "--", "x"], env=self.env,
                           capture_output=True, text=True, timeout=20)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["BUILTIN x part="])


class SessionHooks(Base):
    """The lines at the top of nimbus-session and before Game Mode's gamescope, run for real."""

    def hook(self, extra_env=None, tool=None):
        stand_in = self.t / "nimbus-session"
        stand_in.write_text("#!/bin/sh\n" + block("session part") + 'echo "FELL-THROUGH $* part=${HEARTH_SESSION_PART:-}" >> %s\n' % self.out)
        stand_in.chmod(0o755)
        env = dict(self.env, HEARTH_PART_TOOL=str(tool or self.runner), HOME=str(self.t / "home"))
        env.update(extra_env or {})
        return subprocess.run(["sh", str(stand_in), "--x"], env=env, capture_output=True, text=True, timeout=60)

    def test_without_a_store_copy_the_session_runs_as_before(self):
        r = self.hook()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["FELL-THROUGH --x part="], "no Store copy: not even hearth-part is asked")

    def test_a_broken_store_session_means_this_built_in_one_at_once(self):
        self.put(pid="hearth-session", name="nimbus-session", body='#!/bin/sh\necho "STORE $*" >> %s\nexit 1\n' % self.out)
        r = self.hook()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.said(), ["STORE --x", "FELL-THROUGH --x part=1"], "the built-in session, with the same request, once")
        self.assertGreaterEqual(self.state("hearth-session")["fails"], 2)
        self.hook()
        self.assertEqual(self.said()[-1], "FELL-THROUGH --x part=1")
        self.assertEqual(self.said().count("STORE --x"), 1)

    def test_a_working_store_session_is_started_once(self):
        self.put(pid="hearth-session", name="nimbus-session",
                 body='#!/bin/sh\necho "STORE $* part=$HEARTH_SESSION_PART" >> %s\n"$(dirname "$0")/labwc" &\nwait\n' % self.out,
                 extra={"labwc": "#!/bin/sh\nsleep 3\n"})
        self.hook()
        self.assertEqual(self.said(), ["STORE --x part=1"], "the Store's copy skips these lines (no loop)")

    def test_a_broken_tool_never_stops_the_session(self):
        self.put(pid="hearth-session", name="nimbus-session", body="#!/bin/sh\nexit 1\n")
        bad = self.t / "broken-tool"
        bad.write_text("#!/bin/sh\nexit 5\n")
        bad.chmod(0o755)
        self.hook(tool=bad)
        self.assertEqual(self.said(), ["FELL-THROUGH --x part=1"])

    def test_never_for_root_an_extra_player_or_game_mode_beside_the_desktop(self):
        self.put(pid="hearth-session", name="nimbus-session", body='#!/bin/sh\necho STORE >> %s\n' % self.out)
        for env in ({"HEARTH_SEAT": "2"}, {"HEARTH_SIDE": "1"}):
            self.hook(env)
        self.assertNotIn("STORE", self.said())
        self.assertIn('[ "$(id -u)" != 0 ]', block("session part"))

    def gamescope_block(self, extra_env=None):
        script = "log() { :; }\n" + block("gamescope part") + 'echo "GS=$GS"\n'
        env = dict(self.env, HEARTH_PART_TOOL=str(self.runner), PATH=str(self.t / "bin") + ":" + self.env["PATH"])
        env.update(extra_env or {})
        (self.t / "bin/gamescope").write_text("#!/bin/sh\necho BUILTIN-GAMESCOPE\n")
        (self.t / "bin/gamescope").chmod(0o755)
        r = subprocess.run(["sh", "-c", script], env=env, capture_output=True, text=True, timeout=30)
        return r.stdout.strip()

    def test_game_modes_gamescope_is_plain_without_a_store_copy(self):
        self.assertEqual(self.gamescope_block(), "GS=gamescope")

    def test_game_modes_gamescope_goes_through_the_trial_with_a_store_copy(self):
        self.put(name="gamescope/gamescope", body="#!/bin/sh\nexit 0\n")
        (self.t / "parts/gamescope/part.json").write_text(json.dumps({"id": "gamescope", "version": "1.19.1", "os": "1.19", "path": "gamescope/gamescope"}))
        gs = self.gamescope_block()
        self.assertIn(" trial gamescope %s --up children" % (self.t / "bin/gamescope"), gs)
        self.assertTrue(gs.endswith(" --"))
        self.assertEqual(self.gamescope_block({"HEARTH_SEAT": "2"}), "GS=gamescope", "never for an extra player")

    def test_both_game_modes_start_gamescope_through_it(self):
        text = SESSION.read_text()
        self.assertIn('    $GS $full $extra $NIMBUS_GAMESCOPE_ARGS $seatargs -- "$WATCHDOG" "$NIMBUS_APP"', text)
        self.assertIn("    exec $GS -e -f --xwayland-count 2 $sextra", text)
        self.assertNotRegex(text, r"(?m)^\s*(exec )?gamescope (\$full|-e -f --xwayland-count 2 \$sextra)")
        # the session hook comes before anything else runs
        self.assertLess(text.index("# >>> session part"), text.index('CONF="${XDG_CONFIG_HOME'))


class LoginScreenPart(unittest.TestCase):
    """hearth-login-screen with the Store's greeter, and the real hearth-system-part deciding (as root's own files)."""

    def setUp(self):
        sys.path.insert(0, str(HERE))
        import test_hearth_login_screen as tl
        self.tl = tl
        self.saved = dict(os.environ)
        self.t = pathlib.Path(tempfile.mkdtemp())
        os.chmod(self.t, 0o755)
        (self.t / "version").write_text("1.19\n")
        self.parts = self.t / "sys/system-parts"
        self.parts.mkdir(parents=True)
        os.chmod(self.t / "sys", 0o755)
        os.chmod(self.parts, 0o755)
        allowed = json.loads(ALLOWED.read_text())
        (self.t / "allowed.json").write_text(json.dumps(allowed))
        os.environ.update(HEARTH_SYSTEM_PARTS=str(self.parts), HEARTH_SYSTEM_PARTS_ALLOWED=str(self.t / "allowed.json"),
                          HEARTH_SYSTEM_PARTS_CATALOG=str(self.t / "catalog.json"), HEARTH_SYSTEM_PARTS_KEY=str(self.t / "key.pub"),
                          HEARTH_VERSION_FILE=str(self.t / "version"), HEARTH_SYSTEM_PARTS_OWNER=str(os.geteuid()),
                          HEARTH_LOGIN_SYSTEM_PART=str(SYSPART))
        box = self.tl.Box("run")
        box.setUp()
        self.box = box
        self.m = box.m

    def tearDown(self):
        self.box.tearDown()
        os.environ.clear()
        os.environ.update(self.saved)
        shutil.rmtree(self.t, ignore_errors=True)
        sys.path.remove(str(HERE))

    def put_greeter(self, version="1.19.1"):
        d = self.parts / "hearth-greeter" / version / "greeter"
        d.mkdir(parents=True)
        for x in (self.parts / "hearth-greeter", d.parent, d):
            os.chmod(x, 0o755)
        for f in ("shell.qml", "Login.qml", "Theme.qml", "greeter-info"):
            (d / f).write_text("// the Store's\n")
            os.chmod(d / f, 0o644)
        (self.parts / "hearth-greeter" / "part.json").write_text(json.dumps({"id": "hearth-greeter", "version": version, "os": "1.19", "path": "greeter"}))
        return str(d)

    def on(self):
        self.box.turn(True)

    def bad_list(self):
        try:
            return json.loads((self.parts / "hearth-greeter/state.json").read_text()).get("bad", [])
        except FileNotFoundError:
            return []

    def test_without_a_store_copy_greetd_starts_the_built_in_greeter(self):
        self.on()
        self.m.cmd_select()
        conf = self.box.read("/run/hearth/greetd.toml")
        self.assertIn('command = "/usr/libexec/nimbus/hearth-greeter"\n', conf)
        self.assertNotIn("part", self.box.state()["pending"])

    def test_the_store_copy_is_used_on_trial(self):
        d = self.put_greeter()
        self.on()
        self.m.cmd_select()
        conf = self.box.read("/run/hearth/greetd.toml")
        self.assertIn('command = "/usr/libexec/nimbus/hearth-greeter %s"' % d, conf)
        self.assertEqual(self.box.state()["pending"]["part"], "1.19.1")
        self.assertEqual(self.box.mode()["part"], "1.19.1")

    def test_a_store_copy_that_fails_is_left_out_and_never_pauses_hearths_login_screen(self):
        self.put_greeter()
        self.on()
        self.m.cmd_select()
        self.m.cmd_fallback("it didn't show within 60 s")
        self.assertIn("start --no-block sddm.service", self.box.systemctl_calls(), "SDDM at once, as always")
        st = self.box.state()
        self.assertEqual(int(st.get("failures") or 0), 0, "the copy's failure, not the login screen's")
        self.assertEqual(st["partFailed"]["version"], "1.19.1")
        self.assertEqual(self.bad_list(), ["1.19.1"], "never used again")
        self.assertIn("Try it again", self.m.notice_text(st))
        self.assertEqual(json.loads(self.box.read("/etc/hearth/login-screen.json")), {"screen": "hearth"}, "the setting never changes")
        self.box.boot(2)
        self.m.cmd_select()
        self.assertTrue(self.box.exists("/run/hearth/login-greetd"), "Hearth's login screen at the next start")
        self.assertIn('command = "/usr/libexec/nimbus/hearth-greeter"\n', self.box.read("/run/hearth/greetd.toml"), "the built-in greeter")
        # the built-in path then behaves exactly as before: two failures in a row pause it
        self.m.cmd_fallback("first")
        self.box.boot(3)
        self.m.cmd_select()
        self.m.cmd_fallback("second")
        self.assertTrue(self.box.state()["tripped"])

    def test_a_start_with_the_store_copy_that_never_finished_counts_for_the_copy(self):
        self.put_greeter()
        self.on()
        self.m.cmd_select()
        # black screen, power button held: the next start
        self.box.boot(2)
        self.m.cmd_select()
        self.assertEqual(self.bad_list(), ["1.19.1"])
        self.assertEqual(int(self.box.state().get("failures") or 0), 0)
        self.assertIn('command = "/usr/libexec/nimbus/hearth-greeter"\n', self.box.read("/run/hearth/greetd.toml"))

    def test_status_says_which_greeter(self):
        self.put_greeter()
        self.on()
        self.m.cmd_select()
        out = io_capture(self.m.cmd_status)
        self.assertEqual(json.loads(out)["part"], "1.19.1")

    def test_a_copy_anyone_else_can_change_is_never_used(self):
        d = self.put_greeter()
        os.chmod(pathlib.Path(d) / "Login.qml", 0o666)
        self.on()
        self.m.cmd_select()
        self.assertIn('command = "/usr/libexec/nimbus/hearth-greeter"\n', self.box.read("/run/hearth/greetd.toml"))

    def test_the_greeter_only_takes_a_system_parts_folder(self):
        text = GREETER.read_text()
        self.assertIn("/var/lib/hearth/system-parts/hearth-greeter/*/*)", text)
        self.assertIn('*..*) ;;', text)
        self.assertIn('-p "$GREETER_DIR"', text)
        fake = self.t / "home/greeter"
        fake.mkdir(parents=True)
        (fake / "shell.qml").write_text("x")
        (fake / "Login.qml").write_text("x")
        qs = self.t / "bin/quickshell"
        qs.parent.mkdir()
        qs.write_text("#!/bin/sh\n")
        qs.chmod(0o755)
        env = dict(os.environ, PATH=str(qs.parent) + ":" + os.environ["PATH"], HEARTH_GREETER_COMPOSITOR="echo",
                   XDG_RUNTIME_DIR=str(self.t / "rt"))
        for arg in (str(fake), "/var/lib/hearth/system-parts/hearth-greeter/../../../../tmp"):
            r = subprocess.run(["sh", str(GREETER), arg], env=env, capture_output=True, text=True, timeout=30)
            self.assertIn("-p /usr/share/hearth/greeter", r.stdout, "anything else is the built-in greeter")


def io_capture(fn):
    import io
    import contextlib
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        fn()
    return buf.getvalue()


class SystemPartRules(unittest.TestCase):
    def test_the_greeter_needs_its_files_to_install(self):
        import hashlib, io, tarfile
        t = pathlib.Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, t, True)
        os.chmod(t, 0o755)
        (t / "sys").mkdir(mode=0o755)
        (t / "version").write_text("1.19\n")
        (t / "allowed.json").write_text(ALLOWED.read_text())
        env = dict(HEARTH_SYSTEM_PARTS=str(t / "sys/parts"), HEARTH_SYSTEM_PARTS_ALLOWED=str(t / "allowed.json"),
                   HEARTH_SYSTEM_PARTS_CATALOG=str(t / "catalog.json"), HEARTH_SYSTEM_PARTS_KEY=str(t / "key.pub"),
                   HEARTH_VERSION_FILE=str(t / "version"), HEARTH_SYSTEM_PARTS_OWNER=str(os.geteuid()))
        with mock.patch.dict(os.environ, env):
            m = load(SYSPART, "hsp_greeter")
            m.restart_services = lambda pid: None

            def tarball(files):
                buf = io.BytesIO()
                with tarfile.open(fileobj=buf, mode="w:gz") as tf:
                    for n, data in files.items():
                        ti = tarfile.TarInfo(n)
                        ti.size = len(data)
                        ti.mode = 0o644
                        tf.addfile(ti, io.BytesIO(data))
                return buf.getvalue()

            class R(io.BytesIO):
                def __enter__(self):
                    return self

                def __exit__(self, *a):
                    return False
            for files, ok in (({"greeter/shell.qml": b"x", "greeter/Theme.qml": b"x"}, False),
                              ({"greeter/" + f: b"x" for f in ("shell.qml", "Login.qml", "Theme.qml", "greeter-info")}, True)):
                data = tarball(files)
                (t / "catalog.json").write_text(json.dumps({"serial": 9, "apps": [{"id": "part-hearth-greeter", "part": {
                    "id": "hearth-greeter", "system": True, "version": "1.19.1", "os": "1.19", "path": "greeter",
                    "url": "https://x/g.tar.gz", "sha256": hashlib.sha256(data).hexdigest()}}]}))
                if ok:
                    self.assertEqual(m.install("hearth-greeter", opener=lambda url, timeout: R(data)), "1.19.1")
                    self.assertTrue(m.chosen("hearth-greeter", "/usr/share/hearth/greeter").endswith("/hearth-greeter/1.19.1/greeter"))
                else:
                    with self.assertRaises(SystemExit):
                        m.install("hearth-greeter", opener=lambda url, timeout: R(data))
                    self.assertFalse((t / "sys/parts/hearth-greeter/part.json").exists(), "nothing changed")


class StoreAndBuild(unittest.TestCase):
    def test_the_three_are_quick_update_choices_in_their_groups(self):
        q = QUICK.read_text()
        opts = {x.strip() for x in re.search(r"options: \[([^\]]*)\]", q, re.S).group(1).replace("\n", " ").split(",")}
        for a in ("gamescope", "hearth-session", "hearth-greeter"):
            self.assertIn(a, opts)
            self.assertIn('[ "$APP" = %s ]' % a, q)
        self.assertIn('"gamescope": "gamemode", "hearth-session": "gamemode", "hearth-greeter": "services"', q)
        self.assertRegex(q, r'system_parts = \([^)]*"hearth-greeter"\)')
        self.assertIn("gamescope-part-build.sh", q)
        b = (REPO / ".github/scripts/gamescope-part-build.sh").read_text()
        self.assertIn("ldd", b)
        self.assertIn("trixie-backports", b)

    def test_each_says_when_it_takes_effect(self):
        q = QUICK.read_text()
        self.assertIn('restarts = {"gamescope": "signout", "hearth-session": "signout", "hearth-greeter": "pc"}', q)
        store = REPO / "hosts/hearth-ui/apps/store/src"
        if not store.is_dir():
            self.skipTest("the Store's source isn't here")
        cat = (store / "catalog.rs").read_text()
        self.assertIn('"signout" => "Takes effect after you sign out"', cat)
        main = (store / "main.rs").read_text()
        self.assertIn("Msg::RestartNow =>", main)
        self.assertIn("net::game_running()", main[main.index("Msg::RestartNow =>"):], "never while a game runs")
        self.assertIn('"Later"', (store / "w11.rs").read_text())

    def test_the_store_lists_them_and_never_the_boot_menu_or_recovery(self):
        d = json.loads(CATALOG.read_text())
        by = {e["id"]: e for e in d["apps"]}
        self.assertEqual((by["gamescope"]["partId"], by["gamescope"]["group"]), ("gamescope", "gamemode"))
        self.assertEqual((by["session-start"]["partId"], by["session-start"]["group"]), ("hearth-session", "gamemode"))
        self.assertEqual((by["login-screen"]["partId"], by["login-screen"]["group"]), ("hearth-greeter", "services"))
        self.assertEqual(by["recovery"].get("with"), "system")
        self.assertNotIn("partId", by["recovery"])

    def test_the_image_has_the_hooks(self):
        build = (REPO / "os-bases/peppermint/build.sh").read_text()
        self.assertIn("install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session", build)
        self.assertIn("hearth-part trial", build)


if __name__ == "__main__":
    unittest.main()
