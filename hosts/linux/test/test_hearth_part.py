"""hearth-part: quick updates through the Hearth Store (1.17, docs/HEARTH-STORE.md "Quick updates").

Red flag 2: a Store copy is used only when it's whole, made for this Hearth OS and hasn't failed; anything else means
the built-in one, so an app or desktop update never stops a device from starting.
"""
import hashlib
import importlib.machinery
import importlib.util
import io
import json
import os
import pathlib
import subprocess
import sys
import tarfile
import tempfile
import unittest
import urllib.request

REPO = pathlib.Path(__file__).resolve().parents[3]
TOOL = REPO / "os/system_files/usr/libexec/hearth/hearth-part"


def load(tmp_path, version="1.17"):
    (tmp_path / "version").write_text(version + "\n")
    os.environ.update(HEARTH_PARTS=str(tmp_path / "parts"), HEARTH_PARTS_STATE=str(tmp_path / "state"),
                      XDG_RUNTIME_DIR=str(tmp_path / "run"), HEARTH_VERSION_FILE=str(tmp_path / "version"),
                      HEARTH_PART_QUICK="5")
    loader = importlib.machinery.SourceFileLoader("hearth_part", str(TOOL))
    spec = importlib.util.spec_from_loader("hearth_part", loader)
    m = importlib.util.module_from_spec(spec)
    loader.exec_module(m)
    m._notify = lambda text: None
    m._euid = lambda: 1000  # a person's account, even when the tests run as root
    return m



class raises:
    """Like pytest.raises: the block must raise one of these (CI runs the tests with plain unittest)."""

    def __init__(self, kind):
        self.kind = kind

    def __enter__(self):
        return self

    def __exit__(self, kind, value, tb):
        if kind is None:
            raise AssertionError(f"{self.kind.__name__} wasn't raised")
        return issubclass(kind, self.kind)

def put(tmp_path, pid="files", version="1.17.1", made_for="1.17", body="#!/bin/sh\nexit 0\n"):
    d = tmp_path / "parts" / pid / version
    d.mkdir(parents=True, exist_ok=True)
    prog = d / "app"
    prog.write_text(body)
    prog.chmod(0o755)
    (tmp_path / "parts" / pid / "part.json").write_text(json.dumps({"id": pid, "version": version, "os": made_for, "path": "app"}))
    return str(prog)


def make_builtin(tmp_path):
    b = tmp_path / "builtin"
    b.write_text("#!/bin/sh\necho built-in \"$@\"\n")
    b.chmod(0o755)
    return str(b)


def test_without_a_copy_the_built_in_one(tmp_path, builtin):
    m = load(tmp_path)
    assert m.chosen("files", builtin) == builtin


def test_a_copy_made_for_this_hearth_os_is_used(tmp_path, builtin):
    m = load(tmp_path, "1.17")
    p = put(tmp_path)
    assert m.chosen("files", builtin) == p
    m2 = load(tmp_path, "1.17.3")
    assert m2.chosen("files", builtin) == p


def test_a_copy_for_another_hearth_os_is_not(tmp_path, builtin):
    put(tmp_path, made_for="1.16")
    assert load(tmp_path, "1.17").chosen("files", builtin) == builtin
    put(tmp_path, made_for="1.1")
    assert load(tmp_path, "1.17").chosen("files", builtin) == builtin  # "1.1" never matches 1.17


def test_broken_or_odd_copies_fall_back(tmp_path, builtin):
    m = load(tmp_path)
    put(tmp_path)
    (tmp_path / "parts/files/part.json").write_text("{not json")
    assert m.chosen("files", builtin) == builtin
    (tmp_path / "parts/files/part.json").write_text(json.dumps({"version": "../x", "os": "1.17", "path": "app"}))
    assert m.chosen("files", builtin) == builtin
    (tmp_path / "parts/files/part.json").write_text(json.dumps({"version": "1.17.1", "os": "1.17", "path": "../../x"}))
    assert m.chosen("files", builtin) == builtin
    (tmp_path / "parts/files/part.json").write_text(json.dumps({"version": "1.17.1", "os": "1.17", "path": "missing"}))
    assert m.chosen("files", builtin) == builtin
    assert m.chosen("../files", builtin) == builtin
    (tmp_path / "version").write_text("")
    put(tmp_path)
    assert m.chosen("files", builtin) == builtin  # no Hearth OS version known: the built-in one


def test_a_failed_start_the_built_in_one_for_this_sign_in_then_twice_until_the_next_version(tmp_path, builtin):
    m = load(tmp_path)
    p = put(tmp_path)
    m.failed("files")
    assert m.chosen("files", builtin) == builtin
    os.unlink(tmp_path / "run/hearth-part-files.failed")  # signed in again
    assert m.chosen("files", builtin) == p
    m.failed("files")
    os.unlink(tmp_path / "run/hearth-part-files.failed")
    assert m.chosen("files", builtin) == builtin  # twice in a row
    p2 = put(tmp_path, version="1.17.2")
    assert m.chosen("files", builtin) == p2  # a new version starts clean


def test_try_again_forgets_the_failures(tmp_path, builtin):
    m = load(tmp_path)
    p = put(tmp_path)
    m.failed("files")
    m.failed("files")
    assert m.chosen("files", builtin) == builtin
    m.retry("files")
    assert m.chosen("files", builtin) == p


def test_ok_clears_one_failure(tmp_path, builtin):
    m = load(tmp_path)
    p = put(tmp_path)
    m.failed("files")
    os.unlink(tmp_path / "run/hearth-part-files.failed")
    m.ok("files")
    m.failed("files")
    os.unlink(tmp_path / "run/hearth-part-files.failed")
    assert m.chosen("files", builtin) == p  # not two in a row


def env(tmp_path):
    return dict(os.environ, HEARTH_PARTS=str(tmp_path / "parts"), HEARTH_PARTS_STATE=str(tmp_path / "state"),
                XDG_RUNTIME_DIR=str(tmp_path / "run"), HEARTH_VERSION_FILE=str(tmp_path / "version"), HEARTH_PART_QUICK="5",
                PATH="/usr/bin:/bin")


def test_run_a_copy_that_fails_goes_straight_to_the_built_in_one(tmp_path, builtin):
    if os.geteuid() == 0:
        return  # as root the Store's copy is never used (the test below); CI runs these as a person
    load(tmp_path)
    put(tmp_path, body="#!/bin/sh\nexit 3\n")
    r = subprocess.run([sys.executable, str(TOOL), "run", "files", builtin, "--page", "x"], capture_output=True, text=True, env=env(tmp_path))
    assert r.stdout.strip() == "built-in --page x"
    assert (tmp_path / "run/hearth-part-files.failed").exists()


def test_run_a_good_copy(tmp_path, builtin):
    if os.geteuid() == 0:
        return  # as root the Store's copy is never used (the test below); CI runs these as a person
    load(tmp_path)
    put(tmp_path, body="#!/bin/sh\necho store \"$@\"\n")
    r = subprocess.run([sys.executable, str(TOOL), "run", "files", builtin, "a"], capture_output=True, text=True, env=env(tmp_path))
    assert r.stdout.strip() == "store a" and r.returncode == 0


def test_root_never_runs_a_copy_from_a_home_folder(tmp_path, builtin):
    """Run as root (sudo keeping your HOME, a root service), the copy in the home folder is never started."""
    m = load(tmp_path)
    put(tmp_path)
    m._euid = lambda: 0
    assert m.chosen("files", builtin) == builtin
    src = TOOL.read_text()
    assert "if _euid() == 0 or not _safe_id(pid):" in src


def tarball(files):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz") as t:
        for name, (data, mode) in files.items():
            ti = tarfile.TarInfo(name)
            ti.size = len(data)
            ti.mode = mode
            t.addfile(ti, io.BytesIO(data))
    return buf.getvalue()


class Resp(io.BytesIO):
    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def test_install_checks_the_sha256_and_switches_in_one_go(tmp_path, builtin):
    m = load(tmp_path)
    data = tarball({"app": (b"#!/bin/sh\nexit 0\n", 0o755)})
    m.urllib.request.urlopen = lambda url, timeout=0: Resp(data)
    with raises(SystemExit):
        m.install("files", "1.17.1", "https://x/files.tar.gz", "00" * 32, "1.17", "app")
    assert m.chosen("files", builtin) == builtin  # a wrong download changes nothing
    m.install("files", "1.17.1", "https://x/files.tar.gz", hashlib.sha256(data).hexdigest(), "1.17", "app")
    assert m.chosen("files", builtin).endswith("/parts/files/1.17.1/app")
    with raises(SystemExit):
        m.install("files", "1.17.2", "http://x/files.tar.gz", hashlib.sha256(data).hexdigest(), "1.17", "app")
    assert not [p for p in (tmp_path / "parts/files").iterdir() if p.name.startswith(".")]  # nothing half done left


def test_install_never_writes_outside_its_folder(tmp_path):
    m = load(tmp_path)
    data = tarball({"../../evil": (b"x", 0o644)})
    m.urllib.request.urlopen = lambda url, timeout=0: Resp(data)
    with raises(Exception):
        m.install("files", "1.17.1", "https://x/f.tar.gz", hashlib.sha256(data).hexdigest(), "1.17", "app")
    assert not (tmp_path / "evil").exists() and not (tmp_path / "parts/evil").exists()


def test_mantel_and_the_rust_apps_start_through_it():
    auto = (REPO / "hosts/linux/desktop/labwc/autostart").read_text()
    assert 'hearth-part path mantel "$shell_dir"' in auto and '[ -f "$pick/shell.qml" ] || pick="$shell_dir"' in auto
    assert "hearth-part failed mantel" in auto
    build = (REPO / "os-bases/anatase/build.sh").read_text()
    assert 'install -Dm755 "/ctx/nimbus-app/hearth-ui/$b" "/usr/libexec/hearth/apps/$b"' in build
    assert 'exec /usr/libexec/hearth/apps/%s "$@"' in build


class HearthPart(unittest.TestCase):
    """Runs every test_ function above with a fresh folder (tmp_path) and built-in program (builtin)."""


def _case(fn):
    def run(self):
        saved_env, saved_open = dict(os.environ), urllib.request.urlopen
        try:
            with tempfile.TemporaryDirectory() as d:
                tmp_path = pathlib.Path(d)
                names = fn.__code__.co_varnames[:fn.__code__.co_argcount]
                args = {"tmp_path": tmp_path}
                if "builtin" in names:
                    args["builtin"] = make_builtin(tmp_path)
                fn(**{n: args[n] for n in names})
        finally:
            os.environ.clear()
            os.environ.update(saved_env)
            urllib.request.urlopen = saved_open
    run.__name__ = fn.__name__
    return run


for _n, _f in list(globals().items()):
    if _n.startswith("test_") and callable(_f):
        setattr(HearthPart, _n, _case(_f))
        del globals()[_n]
