#!/usr/bin/env python3
"""Hearth Admin Edition: read-only system and recovery diagnostics.

No elevated privileges, disk writes, package installation, or repair operations.
"""
import os
import platform
import shutil
import subprocess
import tkinter as tk
from tkinter import ttk, messagebox

COMMANDS = {
    "System overview": [["uname", "-a"], ["uptime"], ["free", "-h"], ["df", "-h", "/"]],
    "Drives and filesystems": [["lsblk", "-o", "NAME,SIZE,TYPE,FSTYPE,MOUNTPOINTS,MODEL"], ["df", "-hT"]],
    "Boot environment": [["findmnt", "/"], ["ls", "-la", "/boot"], ["cat", "/etc/os-release"]],
    "Updates (informational)": [["apt-cache", "policy", "hearth-os-app"], ["dpkg-query", "-W", "hearth-os-app"]],
    "Recovery availability": [["which", "timeshift"], ["which", "efibootmgr"], ["which", "smartctl"], ["which", "fsck"]],
}

def diagnostic(command):
    if not shutil.which(command[0]):
        return "(tool not installed)"
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=8, check=False,
                                env={**os.environ, "LC_ALL": "C"}, stdin=subprocess.DEVNULL)
        output = (result.stdout + ("\n" + result.stderr if result.stderr else "")).strip()
        return (output or "(no output)")[:12000]
    except (OSError, subprocess.TimeoutExpired) as exc:
        return f"(unavailable: {exc})"

class AdminApp:
    def __init__(self, root):
        self.root = root
        root.title("Hearth OS Admin Edition — System & Recovery")
        root.geometry("980x700")
        root.minsize(720, 500)
        root.configure(bg="#171b24")
        style = ttk.Style()
        style.theme_use("clam")
        style.configure("TFrame", background="#171b24")
        style.configure("TLabel", background="#171b24", foreground="#eef0f5", font=("Sans", 11))
        style.configure("TButton", padding=9, font=("Sans", 10))
        body = ttk.Frame(root, padding=20)
        body.pack(fill="both", expand=True)
        ttk.Label(body, text="HEARTH  •  ADMIN EDITION", font=("Sans", 20, "bold")).pack(anchor="w")
        ttk.Label(body, text="System information and recovery readiness • Read-only diagnostics").pack(anchor="w", pady=(2, 16))
        toolbar = ttk.Frame(body)
        toolbar.pack(fill="x")
        self.section = tk.StringVar(value="System overview")
        chooser = ttk.Combobox(toolbar, state="readonly", values=list(COMMANDS), textvariable=self.section, width=34)
        chooser.pack(side="left")
        chooser.bind("<<ComboboxSelected>>", lambda _event: self.refresh())
        ttk.Button(toolbar, text="Refresh", command=self.refresh).pack(side="left", padx=10)
        ttk.Button(toolbar, text="Copy report", command=self.copy).pack(side="left")
        self.info = ttk.Label(body, text="")
        self.info.pack(anchor="w", pady=(14, 6))
        frame = ttk.Frame(body)
        frame.pack(fill="both", expand=True)
        self.output = tk.Text(frame, wrap="none", font=("Monospace", 10), bg="#10141c", fg="#e6eaf2",
                              insertbackground="#f2a65a", relief="flat", padx=14, pady=14)
        self.output.pack(side="left", fill="both", expand=True)
        bar = ttk.Scrollbar(frame, command=self.output.yview)
        bar.pack(side="right", fill="y")
        self.output.configure(yscrollcommand=bar.set, state="disabled")
        ttk.Label(body, text="No repairs are performed. Always back up your files before changing disks, boot settings, or partitions.").pack(anchor="w", pady=(12, 0))
        self.refresh()

    def refresh(self):
        section = self.section.get()
        parts = [f"Hearth OS Admin Edition  •  {section}", f"Host: {platform.node()}", ""]
        for command in COMMANDS[section]:
            parts.append("$ " + " ".join(command))
            parts.append(diagnostic(command))
            parts.append("")
        self.report = "\n".join(parts)
        self.output.configure(state="normal")
        self.output.delete("1.0", "end")
        self.output.insert("1.0", self.report)
        self.output.configure(state="disabled")
        self.info.configure(text=f"{section} — local diagnostics only")

    def copy(self):
        if not messagebox.askyesno("Copy system information?", "The report may include device names, hostnames and mount paths. Copy it to your clipboard?"):
            return
        self.root.clipboard_clear()
        self.root.clipboard_append(self.report)

if __name__ == "__main__":
    root = tk.Tk()
    AdminApp(root)
    root.mainloop()
