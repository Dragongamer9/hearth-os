#!/usr/bin/env python3
"""Hearth keeps answering while one of its file pickers is open (hearth-core picker.rs).

Runs the windowless core (`hearth --core-stdio`, the channel in hosts/hearth-rs/src/winhost.rs) as the Electron
window app does, asks for Library › Stores' install folder (accounts.pickFolder), and leaves the picker open: the
core must ask the window app for a folder ({"t":"ask","op":"pickFolder"}), answer other calls meanwhile, say the
picker opened and closed ({name:'picker'}), and answer the first call with the folder once it's picked. Before
0.6.5 the picker held the bridge: every other call waited and the fail-safe stopped Hearth after 16 s.

    python3 tools/picker-e2e.py path/to/hearth
Prints GitHub annotations; exits 1 when something's wrong.
"""
import json
import os
import queue
import subprocess
import sys
import tempfile
import threading
import time


def main():
    exe = sys.argv[1] if len(sys.argv) > 1 else "target/release/hearth"
    home = tempfile.mkdtemp(prefix="hearth-picker-e2e-")
    env = dict(os.environ, HOME=home, XDG_DATA_HOME=os.path.join(home, ".local/share"), XDG_CONFIG_HOME=os.path.join(home, ".config"),
               XDG_CACHE_HOME=os.path.join(home, ".cache"))
    env.pop("NIMBUS_SESSION", None)
    p = subprocess.Popen([exe, "--core-stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env, text=True, bufsize=1)
    lines = queue.Queue()

    def read():
        for line in p.stdout:
            line = line.strip()
            if not line.startswith("{"):
                continue
            try:
                lines.put(json.loads(line))
            except ValueError:
                pass
    threading.Thread(target=read, daemon=True).start()

    def send(v):
        p.stdin.write(json.dumps(v) + "\n")
        p.stdin.flush()

    def call(i, method, args=None):
        send({"t": "page", "win": "main", "msg": {"type": "call", "id": i, "method": method, "args": args or {}}})

    seen = []

    def wait(test, secs):
        end = time.time() + secs
        for v in seen:
            if test(v):
                return v
        while time.time() < end:
            try:
                v = lines.get(timeout=max(0.05, end - time.time()))
            except queue.Empty:
                break
            seen.append(v)
            if test(v):
                return v
        return None

    def reply(i):
        return lambda v: v.get("t") == "msg" and v.get("msg", {}).get("type") == "result" and v["msg"].get("id") == i

    def fail(why):
        print(f"::error title=Picker keeps Hearth answering::{why}; lines: {json.dumps(seen[-12:])[:2500]}")
        p.kill()
        sys.exit(1)

    try:
        call(1, "host.info")
        if not wait(reply(1), 30):
            fail("the core didn't answer host.info")
        call(2, "accounts.pickFolder")
        ask = wait(lambda v: v.get("t") == "ask" and v.get("op") == "pickFolder", 15)
        if not ask:
            fail("the core didn't ask the window app for a folder")
        if wait(reply(2), 1):
            fail("accounts.pickFolder was answered before anything was picked")
        if not wait(lambda v: v.get("t") == "msg" and v.get("msg", {}).get("name") == "picker" and v["msg"].get("data", {}).get("open") is True, 5):
            fail("the page wasn't told a picker opened")
        # The picker stays open: other calls are answered meanwhile, for longer than the fail-safe's 15 s.
        t0 = time.time()
        for n in range(4):
            call(10 + n, "library.get" if n % 2 else "host.info")
            r = wait(reply(10 + n), 10)
            if not r or not r["msg"].get("ok"):
                fail(f"call {10 + n} wasn't answered while the picker was open")
            time.sleep(5)
        waited = time.time() - t0
        if wait(lambda v: v.get("t") == "msg" and v.get("msg", {}).get("name") == "watchdog", 0.5):
            fail("the fail-safe took the open picker for Hearth being stuck")
        folder = os.path.join(home, "Games here")
        send({"t": "answer", "id": ask["id"], "value": folder})
        r = wait(reply(2), 15)
        if not r:
            fail("accounts.pickFolder wasn't answered after the folder was picked")
        if not r["msg"].get("ok") or r["msg"].get("value", {}).get("installRoot") != folder:
            fail(f"accounts.pickFolder answered {r['msg']}")
        if r.get("to") != "main":
            fail(f"the answer went to {r.get('to')!r}, not the window that asked")
        if not wait(lambda v: v.get("t") == "msg" and v.get("msg", {}).get("name") == "picker" and v["msg"].get("data", {}).get("open") is False, 5):
            fail("the page wasn't told the picker closed")
        # Closed without a choice: answered with the folder unchanged.
        call(3, "accounts.pickFolder")
        ask = wait(lambda v: v.get("t") == "ask" and v.get("op") == "pickFolder" and v is not ask, 15)
        if not ask:
            fail("the second picker didn't open")
        send({"t": "answer", "id": ask["id"], "value": None})
        r = wait(reply(3), 15)
        if not r or r["msg"].get("value", {}).get("installRoot") != folder:
            fail(f"a cancelled picker answered {r and r['msg']}")
        print(f"::notice title=Picker keeps Hearth answering::4 calls answered over {waited:.0f} s with the folder picker open; the choice came back to the call")
    finally:
        try:
            p.stdin.close()
        except OSError:
            pass
        try:
            p.wait(timeout=10)
        except subprocess.TimeoutExpired:
            p.kill()


if __name__ == "__main__":
    main()
