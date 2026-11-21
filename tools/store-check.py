#!/usr/bin/env python3
"""Checks a Hearth Store list before it's signed and published (docs/HEARTH-STORE.md).

    python3 tools/store-check.py <list.json> [<built-in list.json>]

Fails (exit 1, with ::error lines for GitHub) when the list isn't valid JSON, has no whole-number serial, has a
serial lower than the built-in list's (devices would ignore it), or an app without an id, a name and somewhere to get
it from that isn't whole (a GitHub repo without its asset, a pinned version without its SHA-256; an app with
neither is shown as Coming). Two apps with the same id also fail.
"""
import json
import sys


def problems(lst, built=None):
    out = []
    if not isinstance(lst, dict):
        return ["the list isn't a JSON object"]
    serial = lst.get("serial")
    if not isinstance(serial, int) or isinstance(serial, bool):
        out.append("the list has no whole-number serial")
    elif built is not None and isinstance(built.get("serial"), int) and serial < built["serial"]:
        out.append(f"serial {serial} is lower than the one built into Hearth OS ({built['serial']}): devices would ignore it")
    apps = lst.get("apps")
    if not isinstance(apps, list) or not apps:
        return out + ["the list has no apps"]
    seen = set()
    for i, a in enumerate(apps):
        who = a.get("id") or f"app {i + 1}" if isinstance(a, dict) else f"app {i + 1}"
        if not isinstance(a, dict):
            out.append(f"{who} isn't a JSON object")
            continue
        for k in ("id", "name"):
            if not isinstance(a.get(k), str) or not a.get(k).strip():
                out.append(f"{who} has no {k}")
        if a.get("id") in seen:
            out.append(f"{who} is in the list twice")
        seen.add(a.get("id"))
        part = a.get("part")
        if part is not None:
            import re
            ok = isinstance(part, dict) and all(isinstance(part.get(k), str) and part.get(k) for k in ("id", "version", "os", "path", "url", "sha256"))
            if not ok:
                out.append(f"{who}'s quick update needs an id, version, os, path, url and sha256")
            elif not (part["url"].startswith("https://") and re.fullmatch(r"[0-9a-f]{64}", part["sha256"]) and ".." not in part["path"].split("/")):
                out.append(f"{who}'s quick update needs an https link, a 64-character SHA-256 and a path inside it")
        pin = a.get("pin")
        if pin is not None:
            if not (isinstance(pin, dict) and all(isinstance(pin.get(k), str) and pin.get(k) for k in ("version", "url", "sha256"))):
                out.append(f"{who}'s pinned version needs a version, a url and a sha256")
        elif "github" in a and not (isinstance(a["github"], str) and a["github"].count("/") == 1 and a.get("asset")):
            out.append(f"{who}'s GitHub repo needs to be owner/name, with the asset to download")
    return out


def main(argv):
    if len(argv) < 2:
        print(__doc__)
        return 2
    try:
        lst = json.load(open(argv[1], encoding="utf-8"))
    except (OSError, ValueError) as e:
        print(f"::error title=Hearth Store list::{argv[1]} can't be read as JSON: {e}")
        return 1
    built = None
    if len(argv) > 2:
        try:
            built = json.load(open(argv[2], encoding="utf-8"))
        except (OSError, ValueError):
            built = None
    bad = problems(lst, built)
    for p in bad:
        print(f"::error title=Hearth Store list::{p}")
    if bad:
        return 1
    print(f"::notice title=Hearth Store list::fine: serial {lst['serial']}, {len(lst['apps'])} apps")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
