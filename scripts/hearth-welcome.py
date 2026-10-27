#!/usr/bin/env python3
"""Hearth OS Admin Edition V2.4 Welcome Center.
Launches only installed local applications; never invokes a shell or sudo.
"""
import pathlib
import shutil
import subprocess
import tkinter as tk
from tkinter import ttk, messagebox

ACTIONS = [
    ("🎮  Hearth Gaming", "Launch the Hearth gaming interface", ["/usr/bin/hearth", "--windowed"]),
    ("🛠  Admin & Recovery", "System diagnostics and recovery readiness", ["/usr/bin/hearth-admin-center"]),
    ("🔄  Update Status", "Show installed Hearth package information", ["/usr/bin/hearth-upgrade", "status"]),
    ("🎨  Desktop Setup", "Apply Hearth Admin wallpaper (reversible)", ["/usr/bin/hearth-admin-desktop", "apply"]),
    ("🖥  Desktop Status", "Inspect desktop identity and settings", ["/usr/bin/hearth-admin-desktop", "status"]),
    ("📋  System Report", "View a read-only system report", ["/usr/bin/hearth-admin-report", "--section", "overview"]),
]
OPTIONAL = [
    ("🤖  AI Command Center", "Launch the separately installed Hearth AI application", [
        "hearth-ai-command-center", "hearth-ai", "hearth-command-center"
    ]),
]
class Welcome:
    def __init__(self, root):
        self.root = root
        root.title("Hearth OS — Admin Edition | Welcome")
        root.geometry("950x730")
        root.minsize(660, 550)
        root.configure(bg="#101824")
        main = tk.Frame(root, bg="#101824", padx=32, pady=26)
        main.pack(fill="both", expand=True)
        tk.Label(main, text="🐉  HEARTH OS", fg="#f4e6d0", bg="#101824",
                 font=("Sans", 32, "bold")).pack(anchor="w")
        tk.Label(main, text="ADMIN EDITION  •  V2.4", fg="#edae66", bg="#101824",
                 font=("Sans", 15, "bold")).pack(anchor="w", pady=(0, 8))
        tk.Label(main, text="Your system. Your tools. Your control.", fg="#b7c4d1", bg="#101824",
                 font=("Sans", 12)).pack(anchor="w", pady=(0, 24))
        grid = tk.Frame(main, bg="#101824")
        grid.pack(fill="both", expand=True)
        for idx, (name, desc, command) in enumerate(ACTIONS):
            self.card(grid, idx, name, desc, command)
        for name, desc, candidates in OPTIONAL:
            executable = next((shutil.which(item) for item in candidates if shutil.which(item)), None)
            self.card(grid, len(ACTIONS), name, desc if executable else "Not installed — AI integration is optional",
                      [executable] if executable else None)
        tk.Label(main, text="Linux Mint-compatible foundation  •  Admin tools default to read-only",
                 fg="#9db1c2", bg="#101824", font=("Sans", 10)).pack(anchor="w", pady=(20, 0))

    def card(self, parent, index, title, description, command):
        frame = tk.Frame(parent, bg="#1b2938", padx=16, pady=12)
        frame.grid(row=index // 2, column=index % 2, sticky="nsew", padx=6, pady=6)
        parent.grid_columnconfigure(index % 2, weight=1)
        tk.Label(frame, text=title, bg="#1b2938", fg="#f4e6d0", font=("Sans", 14, "bold")).pack(anchor="w")
        tk.Label(frame, text=description, bg="#1b2938", fg="#c4cdd5", font=("Sans", 10),
                 wraplength=360, justify="left").pack(anchor="w", pady=(4, 9))
        button = ttk.Button(frame, text="Open" if command else "Not installed",
                            command=lambda: self.launch(command) if command else None)
        button.pack(anchor="w")
        if not command:
            button.configure(state="disabled")

    def launch(self, command):
        if not command or not pathlib.Path(command[0]).exists():
            messagebox.showwarning("Unavailable", "This Hearth component is not installed.")
            return
        try:
            if command[1:] in (["status"], ["--section", "overview"]):
                result = subprocess.run(command, capture_output=True, text=True, timeout=18,
                                        stdin=subprocess.DEVNULL, check=False)
                self.show_output(result.stdout + "\n" + result.stderr)
            else:
                if command[-1] == "apply" and not messagebox.askyesno("Desktop change", "Apply the Hearth wallpaper to your desktop? You can restore it later."):
                    return
                subprocess.Popen(command, stdin=subprocess.DEVNULL, start_new_session=True)
        except (OSError, subprocess.TimeoutExpired) as exc:
            messagebox.showerror("Could not launch", str(exc))

    def show_output(self, content):
        win = tk.Toplevel(self.root)
        win.title("Hearth OS — Status")
        win.geometry("750x500")
        output = tk.Text(win, bg="#111925", fg="#e4e9f0", font=("Monospace", 10), wrap="word")
        output.pack(fill="both", expand=True)
        output.insert("1.0", content[:30000])
        output.configure(state="disabled")

if __name__ == "__main__":
    root = tk.Tk()
    Welcome(root)
    root.mainloop()
