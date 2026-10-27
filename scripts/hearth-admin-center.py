#!/usr/bin/env python3
"""Hearth OS Admin Center — read-only system, recovery and upgrade diagnostics."""
import importlib.util
import pathlib
import tkinter as tk
from tkinter import ttk, messagebox, filedialog

# The report utility is packaged alongside this application; importing it does not
# run commands until the user selects a category and refreshes.
REPORT = pathlib.Path(__file__).with_name("hearth-admin-report")
if not REPORT.exists():
    REPORT = pathlib.Path(__file__).with_name("hearth-admin-report.py")
spec = importlib.util.spec_from_file_location("hearth_admin_report", REPORT)
# The installed utility has no .py extension, so explicitly create a source loader.
from importlib.machinery import SourceFileLoader
loader = SourceFileLoader("hearth_admin_report", str(REPORT))
spec = importlib.util.spec_from_loader(loader.name, loader)
reporter = importlib.util.module_from_spec(spec)
loader.exec_module(reporter)

SECTIONS = {
    "System overview": "overview",
    "Hardware": "hardware",
    "Disks & filesystems": "storage",
    "Network (redacted)": "network",
    "Boot health": "boot",
    "Updates": "updates",
    "Recovery tools": "recovery",
}

class AdminApp:
    def __init__(self, root):
        self.root = root
        root.title("Hearth OS Admin Edition — V2.2")
        root.geometry("1080x750")
        root.minsize(720, 500)
        root.configure(bg="#171b24")
        style = ttk.Style()
        style.theme_use("clam")
        style.configure("TFrame", background="#171b24")
        style.configure("TLabel", background="#171b24", foreground="#eef0f5", font=("Sans", 11))
        style.configure("TButton", padding=9, font=("Sans", 10))
        body = ttk.Frame(root, padding=20)
        body.pack(fill="both", expand=True)
        ttk.Label(body, text="HEARTH  •  ADMIN EDITION  2.2", font=("Sans", 20, "bold")).pack(anchor="w")
        ttk.Label(body, text="Hardware • Storage • Networking • Boot • Updates • Recovery", font=("Sans", 11)).pack(anchor="w", pady=(3, 15))
        toolbar = ttk.Frame(body)
        toolbar.pack(fill="x")
        self.section = tk.StringVar(value="System overview")
        chooser = ttk.Combobox(toolbar, state="readonly", values=list(SECTIONS), textvariable=self.section, width=26)
        chooser.pack(side="left")
        chooser.bind("<<ComboboxSelected>>", lambda _event: self.refresh())
        ttk.Button(toolbar, text="Refresh", command=self.refresh).pack(side="left", padx=8)
        ttk.Button(toolbar, text="Copy redacted report", command=self.copy).pack(side="left")
        ttk.Button(toolbar, text="Save report locally", command=self.export).pack(side="left", padx=8)
        self.info = ttk.Label(body, text="")
        self.info.pack(anchor="w", pady=(13, 7))
        frame = ttk.Frame(body)
        frame.pack(fill="both", expand=True)
        self.output = tk.Text(frame, wrap="none", font=("Monospace", 10), bg="#10141c", fg="#e6eaf2",
                              insertbackground="#f2a65a", relief="flat", padx=14, pady=14)
        self.output.pack(side="left", fill="both", expand=True)
        scroll = ttk.Scrollbar(frame, command=self.output.yview)
        scroll.pack(side="right", fill="y")
        self.output.configure(yscrollcommand=scroll.set, state="disabled")
        ttk.Label(body, text="READ ONLY — No commands run as root. Review all reports before sharing; some system paths may still be sensitive.").pack(anchor="w", pady=(13, 0))
        self.refresh()

    def refresh(self):
        key = SECTIONS[self.section.get()]
        try:
            self.report = reporter.render_plain(reporter.gather(key))
        except Exception as exc:
            self.report = "Unable to collect diagnostics: " + str(exc)
        self.output.configure(state="normal")
        self.output.delete("1.0", "end")
        self.output.insert("1.0", self.report)
        self.output.configure(state="disabled")
        self.info.configure(text=self.section.get() + " • local only • no automatic uploads")

    def copy(self):
        if messagebox.askyesno("Copy diagnostics?", "Copy the current redacted diagnostics to your clipboard? Please review before sharing."):
            self.root.clipboard_clear()
            self.root.clipboard_append(self.report)

    def export(self):
        if not messagebox.askyesno("Save report?", "Export the current diagnostics to a local text file? Existing files will not be overwritten."):
            return
        path = filedialog.asksaveasfilename(title="Save Hearth diagnostics", defaultextension=".txt",
                                           initialfile="hearth-" + SECTIONS[self.section.get()] + ".txt",
                                           filetypes=[("Text report", "*.txt")])
        if not path:
            return
        try:
            output = pathlib.Path(path)
            if output.exists() or output.is_symlink():
                raise FileExistsError("That file already exists")
            data = self.report.encode("utf-8")
            if len(data) > reporter.MAX_REPORT_BYTES:
                raise ValueError("Report exceeds size limit")
            import os
            fd = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, "wb") as stream:
                stream.write(data)
            messagebox.showinfo("Saved", "Saved locally. Review report before sharing.")
        except (OSError, ValueError) as exc:
            messagebox.showerror("Not saved", str(exc))

if __name__ == "__main__":
    root = tk.Tk()
    AdminApp(root)
    root.mainloop()
