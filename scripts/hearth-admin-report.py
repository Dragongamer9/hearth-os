#!/usr/bin/env python3
"""Hearth Admin Edition diagnostics: read-only, offline, explicit report export.

No sudo, no remote uploads, no install/repair actions. Standard-library only.
"""
import argparse
import datetime as dt
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys

SECTIONS = {
    "overview": [
        ["uname", "-a"], ["uptime"], ["free", "-h"], ["df", "-hT"],
        ["cat", "/etc/os-release"],
    ],
    "hardware": [
        ["lscpu"], ["lsblk", "-o", "NAME,SIZE,TYPE,FSTYPE,MOUNTPOINTS,MODEL"],
        ["lspci", "-nn"], ["lsusb"],
    ],
    "storage": [
        ["lsblk", "-f"], ["findmnt", "-r"], ["df", "-hT"],
        ["blkid"],
    ],
    "network": [
        ["ip", "-brief", "address"], ["ip", "route"], ["nmcli", "-t", "device", "status"],
    ],
    "boot": [
        ["findmnt", "/"], ["uname", "-r"], ["ls", "-l", "/boot"],
        ["systemctl", "--failed", "--no-pager", "--plain"],
    ],
    "updates": [
        ["dpkg-query", "-W", "hearth-os-app"],
        ["apt-cache", "policy", "hearth-os-app"],
        ["apt", "list", "--upgradable"],
    ],
    "recovery": [
        ["which", "timeshift"], ["which", "efibootmgr"],
        ["which", "smartctl"], ["which", "fsck"],
    ],
}
MAX_COMMAND_OUTPUT = 12000
MAX_REPORT_BYTES = 250000
SENSITIVE = [
    (re.compile(r"(?i)(authorization\s*[:=]\s*bearer\s+)\S+"), r"\1[REDACTED]"),
    (re.compile(r"(?i)((?:api[_-]?key|access[_-]?token|secret|password)\s*[:=]\s*)\S+"), r"\1[REDACTED]"),
    (re.compile(r"\b(?:[0-9a-fA-F]{2}:){5}[0-9a-fA-F]{2}\b"), "[MAC REDACTED]"),
    (re.compile(r"\b(?:\d{1,3}\.){3}\d{1,3}\b"), "[IP REDACTED]"),
]

def redact(text, home=None, hostname=None):
    home = home or str(pathlib.Path.home())
    hostname = hostname or os.uname().nodename
    if home and len(home) > 1:
        text = text.replace(home, "~")
    if hostname and len(hostname) > 2:
        text = text.replace(hostname, "[HOST]")
    for pattern, replacement in SENSITIVE:
        text = pattern.sub(replacement, text)
    return text

def run_readonly(argv):
    if not shutil.which(argv[0]):
        return "(tool unavailable)"
    try:
        result = subprocess.run(argv, capture_output=True, text=True, timeout=12,
                                stdin=subprocess.DEVNULL, check=False,
                                env={**os.environ, "LC_ALL": "C"})
        combined = (result.stdout + ("\n" + result.stderr if result.stderr else "")).strip()
        return (combined or "(no output)")[:MAX_COMMAND_OUTPUT]
    except (OSError, subprocess.TimeoutExpired) as exc:
        return "(unavailable: " + str(exc) + ")"

def gather(section):
    if section not in SECTIONS:
        raise ValueError("Unknown diagnostics section")
    report = {
        "product": "Hearth OS Admin Edition",
        "schema": 1,
        "section": section,
        "collected_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "read_only": True,
        "commands": [],
    }
    for argv in SECTIONS[section]:
        result = redact(run_readonly(argv))
        report["commands"].append({"command": argv, "output": result})
    return report

def render_plain(report):
    lines = [
        report["product"], "Section: " + report["section"],
        "Collected (UTC): " + report["collected_at"],
        "READ ONLY — No repairs or installations performed.", "",
    ]
    for item in report["commands"]:
        lines.extend(["$ " + " ".join(item["command"]), item["output"], ""])
    return "\n".join(lines)

def save_report(section, output):
    report = gather(section)
    rendered = render_plain(report)
    data = rendered.encode("utf-8")
    if len(data) > MAX_REPORT_BYTES:
        raise ValueError("Refusing report larger than safety limit")
    target = pathlib.Path(output).expanduser()
    if target.is_symlink() or target.exists():
        raise FileExistsError("Refusing to overwrite an existing file")
    target.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(target, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as stream:
        stream.write(data)
    return target

def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--section", choices=list(SECTIONS), default="overview")
    parser.add_argument("--json", action="store_true", help="Print a redacted JSON report")
    parser.add_argument("--output", help="Export a redacted, private report to a NEW file (mode 0600)")
    args = parser.parse_args(argv)
    if args.output:
        target = save_report(args.section, args.output)
        print("Report saved locally:", target)
    else:
        report = gather(args.section)
        print(json.dumps(report, indent=2) if args.json else render_plain(report))
    return 0

if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError) as exc:
        print("Hearth Admin: " + str(exc), file=sys.stderr)
        sys.exit(1)
