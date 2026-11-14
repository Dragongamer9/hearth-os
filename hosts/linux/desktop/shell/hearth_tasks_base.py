"""Task Manager's helper (hearth-tasks), its shared parts: settings files, reading /proc and /sys, the parts of Hearth that are never ended. Split out of hearth-tasks in the 1.12 tidy-up;
hearth-tasks re-exports everything here, so `hearth-tasks` and what loads it (Resource Monitor, the tests) are unchanged."""
import fnmatch
import glob
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time

# Settings files are written and read the safe way (docs/SETTINGS-PERSISTENCE.md): atomic, with .bak, restored
# from .bak when damaged, never overwritten with defaults. The module sits next to this file; a copy of this helper
# run on its own elsewhere keeps working without it.
sys.path.insert(0, os.path.dirname(os.path.realpath(__file__)))
try:
    import hearth_settings_io as hsio
except ImportError:
    hsio = None

PROC = '/proc'
SYS = '/sys'

# What keeps the session running: never ended from here. Matched against the program's name, the name it
# was started as, and the script an interpreter runs.
PROTECTED = {
    'systemd': 'System services (systemd)', '(sd-pam)': 'System services (systemd)',
    'gamescope': 'Game Mode screen (gamescope)', 'gamescope-wl': 'Game Mode screen (gamescope)',
    'gamescopereaper': 'Game Mode screen (gamescope)',
    'labwc': 'Desktop windows (labwc)', 'quickshell': 'Mantel taskbar and menus', 'qs': 'Mantel taskbar and menus',
    'nimbus': 'Hearth', 'hearth': 'Hearth', 'nimbus-session': 'Hearth session', 'nimbus-mode': 'Hearth session',
    'nimbus-watchdog': 'Hearth session', 'autostart': 'Hearth session',
    'Xwayland': 'Older-app support (Xwayland)',
    'dbus-daemon': 'App messaging (D-Bus)', 'dbus-broker': 'App messaging (D-Bus)', 'dbus-broker-launch': 'App messaging (D-Bus)',
    'pipewire': 'Sound (PipeWire)', 'pipewire-pulse': 'Sound (PipeWire)', 'wireplumber': 'Sound (WirePlumber)',
    'gnome-keyring-daemon': 'Password store', 'mangoapp': 'Performance overlay',
    'xdg-desktop-portal': 'Desktop portal', 'xdg-desktop-portal-wlr': 'Desktop portal', 'xdg-desktop-portal-gtk': 'Desktop portal',
    'xdg-document-portal': 'Desktop portal', 'xdg-permission-store': 'Desktop portal',
    'polkit-gnome-authentication-agent-1': 'Password prompts', 'polkit-kde-authentication-agent-1': 'Password prompts',
    'lxqt-policykit-agent': 'Password prompts',
    'kwin_wayland': 'Desktop', 'plasmashell': 'Desktop', 'ksmserver': 'Desktop', 'gnome-shell': 'Desktop',
    'sddm-helper': 'Sign-in', 'gdm-wayland-session': 'Sign-in', 'login': 'Sign-in',
}
INTERPRETERS = {'sh', 'bash', 'dash', 'zsh', 'python', 'python3', 'perl', 'env'}
# Hearth's own helpers live here (hearth-pad, hearth-buttons, ...).
SHARE_MARK = '/nimbus/desktop/'
# Services that keep the desktop, sound, sign-ins or Hearth working: not offered in Startup.
ESSENTIAL_UNITS = [
    'dbus', 'dbus-broker', 'pipewire', 'pipewire-pulse', 'wireplumber', 'filter-chain', 'pulseaudio',
    'xdg-desktop-portal*', 'xdg-document-portal', 'xdg-permission-store', 'xdg-user-dirs', 'gvfs-*', 'at-spi*',
    'gnome-keyring*', 'gcr-ssh-agent', 'ssh-agent', 'p11-kit*', 'systemd-*', 'grub-boot-success', 'obex',
    'nimbus*', 'hearth*', 'steamos-*', 'bazzite-*', 'ublue-*', 'inputplumber*', 'gamescope*', 'labwc*',
    'quickshell*', 'app-*', 'session-*', 'dconf', 'evolution-*', 'tracker-*', 'localsearch-*',
]
HEARTH_ID = 'hearth:steamBackground'
OVERRIDE_MARK = 'X-Hearth-Override=true'


class Fail(Exception):
    pass


def read(path, default=None):
    try:
        with open(path, 'rb') as f:
            return f.read().decode('utf-8', 'replace')
    except OSError:
        return default


def read_int(path):
    t = read(path)
    try:
        return int(t.strip()) if t is not None else None
    except ValueError:
        return None


def runtime_dir():
    d = os.environ.get('XDG_RUNTIME_DIR')
    if d and os.path.isdir(d):
        return d
    return '/tmp'


def state_path(name):
    return os.path.join(runtime_dir(), 'hearth-tasks-%d-%s.json' % (os.getuid(), name))


def load_state(name):
    try:
        with open(state_path(name)) as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


def save_state(name, data):
    try:
        tmp = state_path(name) + '.tmp'
        with open(tmp, 'w') as f:
            json.dump(data, f)
        os.replace(tmp, state_path(name))
    except OSError:
        pass


def write_atomic(path, text):
    if hsio:
        return hsio.write_text(path, text)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.tmp'
    with open(tmp, 'w') as f:
        f.write(text)
    os.replace(tmp, path)


CLK_TCK = os.sysconf('SC_CLK_TCK') if hasattr(os, 'sysconf') else 100

