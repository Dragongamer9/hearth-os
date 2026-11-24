"""hearth-system-part: root-owned system parts from the Hearth Store (1.18, docs/STORE-SYSTEM-PARTS.md).

The rules this checks: a program that runs as root never runs a file from your home folder or one anyone but root
can change; only the helpers Hearth names can be system parts; what gets installed comes from the list signed with the
owner's key (never from the caller); the SHA-256 must match; a copy that doesn't work goes back to the one before by
itself; and nothing here is ever about starting the PC.
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

REPO = pathlib.Path(__file__).resolve().parents[3]
TOOL = REPO / "os/system_files/usr/libexec/hearth/hearth-system-part"
ALLOWED_FILE = REPO / "os/system_files/usr/share/hearth/store/system-parts.json"
HELPERS = ["hearth-home-server-helper", "hearth-storage-helper", "hearth-ai-share-helper"]


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


class SystemPart(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.TemporaryDirectory()
        t = pathlib.Path(self.d.name)
        self.t = t
        (t / "sys").mkdir(mode=0o755)
        os.chmod(t, 0o755)
        (t / "version").write_text("1.18\n")
        (t / "allowed.json").write_text(json.dumps({"parts": {"helper": {"name": "Helper", "builtin": str(t / "builtin"), "services": []}}}))
        (t / "builtin").write_text("#!/bin/sh\necho built-in \"$@\"\n")
        os.chmod(t / "builtin", 0o755)
        self.saved = dict(os.environ)
        os.environ.update(HEARTH_SYSTEM_PARTS=str(t / "sys/parts"), HEARTH_SYSTEM_PARTS_ALLOWED=str(t / "allowed.json"),
                          HEARTH_SYSTEM_PARTS_CATALOG=str(t / "catalog.json"), HEARTH_SYSTEM_PARTS_KEY=str(t / "key.pub"),
                          HEARTH_VERSION_FILE=str(t / "version"), HEARTH_SYSTEM_PARTS_OWNER=str(os.geteuid()),
                          HEARTH_PART_QUICK="5")
        loader = importlib.machinery.SourceFileLoader("hearth_system_part", str(TOOL))
        spec = importlib.util.spec_from_loader("hearth_system_part", loader)
        self.m = importlib.util.module_from_spec(spec)
        loader.exec_module(self.m)
        self.m.restart_services = lambda pid: None

    def tearDown(self):
        os.environ.clear()
        os.environ.update(self.saved)
        self.d.cleanup()

    def put(self, version="1.18.1", made_for="1.18", pid="helper", body="#!/bin/sh\necho store \"$@\"\n", previous=None):
        d = self.t / "sys/parts" / pid / version
        d.mkdir(parents=True, exist_ok=True)
        for x in (self.t / "sys/parts", self.t / "sys/parts" / pid, d):
            os.chmod(x, 0o755)
        (d / "helper").write_text(body)
        os.chmod(d / "helper", 0o755)
        (self.t / "sys/parts" / pid / "part.json").write_text(json.dumps({"id": pid, "version": version, "os": made_for, "path": "helper", "previous": previous}))
        return str(d / "helper")

    def catalog(self, part, serial=5):
        (self.t / "catalog.json").write_text(json.dumps({"serial": serial, "apps": [{"id": "part-helper", "name": "Helper", "kind": "builtin", "part": part}]}))

    # ---- using a copy
    def test_a_good_copy_is_used(self):
        p = self.put()
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), p)

    def test_only_the_helpers_hearth_names(self):
        self.put(pid="other")
        self.assertEqual(self.m.chosen("other", "/bin/true"), "/bin/true")
        names = json.loads(ALLOWED_FILE.read_text())["parts"]
        for bad in ("boot", "grub", "recovery", "greet", "login", "session"):
            self.assertFalse(any(bad in k for k in names), f"{bad}: what starts the PC is never a system part")
        self.assertEqual(sorted(names), sorted(HELPERS))

    def test_a_copy_anyone_else_can_change_is_never_used(self):
        p = self.put()
        os.chmod(p, 0o775)
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), str(self.t / "builtin"))
        os.chmod(p, 0o755)
        os.chmod(self.t / "sys/parts", 0o777)
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), str(self.t / "builtin"))

    def test_a_copy_owned_by_someone_else_is_never_used(self):
        self.put()
        os.environ["HEARTH_SYSTEM_PARTS_OWNER"] = str(os.geteuid() + 1)
        self.m.OWNER = os.geteuid() + 1
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), str(self.t / "builtin"))

    def test_a_link_out_of_the_parts_folder_is_never_used(self):
        home = self.t / "home-copy"
        home.write_text("#!/bin/sh\necho from home\n")
        os.chmod(home, 0o755)
        d = self.t / "sys/parts/helper/1.18.1"
        d.mkdir(parents=True)
        os.chmod(self.t / "sys/parts", 0o755)
        os.chmod(self.t / "sys/parts/helper", 0o755)
        os.symlink(home, d / "helper")
        (self.t / "sys/parts/helper/part.json").write_text(json.dumps({"id": "helper", "version": "1.18.1", "os": "1.18", "path": "helper"}))
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), str(self.t / "builtin"))

    def test_only_for_this_hearth_os(self):
        self.put(made_for="1.17")
        self.assertEqual(self.m.chosen("helper", str(self.t / "builtin")), str(self.t / "builtin"))

    def test_two_failures_go_back_to_the_one_before_then_the_built_in_one(self):
        old = self.put(version="1.18.0")
        new = self.put(version="1.18.1", previous="1.18.0")
        self.assertEqual(self.m.chosen("helper", "x"), new)
        self.m.failed("helper", "1.18.1")
        self.assertEqual(self.m.chosen("helper", "x"), new, "one failure isn't enough")
        self.m.failed("helper", "1.18.1")
        self.assertEqual(self.m.chosen("helper", "x"), old, "back to the one before by itself")
        self.m.failed("helper", "1.18.0")
        self.m.failed("helper", "1.18.0")
        self.assertEqual(self.m.chosen("helper", "x"), "x")

    def test_run_a_broken_copy_falls_back_at_once(self):
        self.put(body="#!/usr/bin/python3\nraise SyntaxError('x')\n")
        env = dict(os.environ)
        r = subprocess.run([sys.executable, str(TOOL), "run", "helper", str(self.t / "builtin"), "a"], capture_output=True, text=True, env=env)
        self.assertEqual(r.stdout.strip(), "built-in a")
        st = json.loads((self.t / "sys/parts/helper/state.json").read_text())
        self.assertEqual(st["fails"], {"1.18.1": 1})

    def test_run_a_good_copy(self):
        self.put()
        r = subprocess.run([sys.executable, str(TOOL), "run", "helper", str(self.t / "builtin"), "a"], capture_output=True, text=True, env=dict(os.environ))
        self.assertEqual(r.stdout.strip(), "store a")

    # ---- installing
    def test_install_takes_only_the_signed_lists_copy(self):
        data = tarball({"helper": (b"#!/bin/sh\necho new\n", 0o4777)})
        sha = hashlib.sha256(data).hexdigest()
        self.catalog({"id": "helper", "system": True, "version": "1.18.1", "os": "1.18", "path": "helper", "url": "https://x/h.tar.gz", "sha256": sha})
        v = self.m.install("helper", opener=lambda url, timeout: Resp(data))
        self.assertEqual(v, "1.18.1")
        p = self.t / "sys/parts/helper/1.18.1/helper"
        mode = p.stat().st_mode
        self.assertFalse(mode & 0o022, "writable only by root")
        self.assertFalse(mode & 0o6000, "no set-id bits")
        self.assertEqual(self.m.chosen("helper", "x"), str(p))

    def test_a_test_version_comes_only_from_the_lists_tests(self):
        data = tarball({"helper": (b"#!/bin/sh\necho test\n", 0o755)})
        sha = hashlib.sha256(data).hexdigest()
        main = {"id": "helper", "system": True, "version": "1.18.1", "os": "1.18", "path": "helper", "url": "https://x/main", "sha256": "1" * 64}
        test = dict(main, version="1.18.2", url="https://x/test", sha256=sha)
        (self.t / "catalog.json").write_text(json.dumps({"serial": 5, "apps": [{"id": "part-helper", "name": "H", "part": main}],
                                                         "tests": [{"id": "test-helper", "name": "H (test)", "part": test}]}))
        got = []
        v = self.m.install("helper", opener=lambda url, timeout: (got.append(url), Resp(data))[1], test=True)
        self.assertEqual((v, got), ("1.18.2", ["https://x/test"]))

    def test_install_refuses_a_wrong_sha(self):
        data = tarball({"helper": (b"#!/bin/sh\n", 0o755)})
        self.catalog({"id": "helper", "system": True, "version": "1.18.1", "os": "1.18", "path": "helper", "url": "https://x/h.tar.gz", "sha256": "0" * 64})
        with self.assertRaises(SystemExit):
            self.m.install("helper", opener=lambda url, timeout: Resp(data))
        self.assertFalse((self.t / "sys/parts/helper/part.json").exists())

    def test_install_refuses_what_isnt_a_system_part(self):
        data = tarball({"helper": (b"#!/bin/sh\n", 0o755)})
        sha = hashlib.sha256(data).hexdigest()
        self.catalog({"id": "helper", "version": "1.18.1", "os": "1.18", "path": "helper", "url": "https://x/h.tar.gz", "sha256": sha})
        with self.assertRaises(SystemExit):
            self.m.install("helper", opener=lambda url, timeout: Resp(data))
        with self.assertRaises(SystemExit):
            self.m.install("not-listed", opener=lambda url, timeout: Resp(data))

    def test_a_refreshed_list_counts_only_when_signed(self):
        try:
            from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
            from cryptography.hazmat.primitives import serialization
        except ImportError:
            self.skipTest("python3-cryptography isn't here")
        k = Ed25519PrivateKey.generate()
        pub = k.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        (self.t / "key.pub").write_text("# test\n" + pub.hex() + "\n")
        self.catalog({"id": "helper", "system": True, "version": "1.18.0", "os": "1.18", "path": "helper", "url": "https://x/old", "sha256": "1" * 64}, serial=5)
        newer = json.dumps({"serial": 6, "apps": [{"id": "part-helper", "name": "Helper", "part": {"id": "helper", "system": True, "version": "1.18.2", "os": "1.18", "path": "helper", "url": "https://x/new", "sha256": "2" * 64}}]})
        good = k.sign(newer.encode()).hex()
        serial, apps = self.m.signed_list({"catalog": newer, "sig": good})
        self.assertEqual(serial, 6)
        serial, _ = self.m.signed_list({"catalog": newer, "sig": "00" * 64})
        self.assertEqual(serial, 5, "an unsigned list is ignored")
        older = json.dumps({"serial": 4, "apps": []})
        serial, _ = self.m.signed_list({"catalog": older, "sig": k.sign(older.encode()).hex()})
        self.assertEqual(serial, 5, "an older signed list never wins")

    # ---- the helpers start through it, only when a copy is installed
    def test_the_helpers_start_through_it_only_with_a_copy(self):
        for h in HELPERS:
            src = (REPO / "os/system_files/usr/libexec/nimbus" / h).read_text()
            self.assertIn(f'_hsp_os.path.isfile("/var/lib/hearth/system-parts/{h}/part.json")', src)
            self.assertIn('_hsp_os.environ.get("HEARTH_SYSTEM_PART") != "1"', src)

    def test_the_game_mode_host_starts_through_hearth_part_only_with_a_copy(self):
        build = (REPO / "os-bases/peppermint/build.sh").read_text()
        self.assertIn('[ -e "$HOME/.local/share/hearth/parts/hearth-game-mode/part.json" ] && [ -x /usr/libexec/hearth/hearth-part ] && exec /usr/libexec/hearth/hearth-part run hearth-game-mode /usr/lib/nimbus/nimbus', build)
        self.assertIn('exec /usr/lib/nimbus/nimbus "$@"', build)

    def test_hearth_part_never_runs_a_home_copy_as_root(self):
        src = (REPO / "os/system_files/usr/libexec/hearth/hearth-part").read_text()
        self.assertIn("if _euid() == 0 or not _safe_id(pid):", src)


if __name__ == "__main__":
    unittest.main()
