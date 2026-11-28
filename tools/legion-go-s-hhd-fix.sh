#!/bin/bash
# Legion Go S: Handheld Daemon 4.1.12 finds the controller's shortcut keys by the name "... Keyboard", but newer Go S
# firmware calls that part just "Legion Go S", so HHD restarts the controller every 3 s and the controls only partly
# work (the owner's Go S, Nov 28, 2026, log MK7T-YJPX-7T). This puts a patched copy of HHD's Python package in
# /var/lib/hearth/hhd-gos and points hhd.service at it with a drop-in in /etc: the patch looks for the keys by what they
# are (KEY_1) when no "Keyboard" is there, and changes nothing when one is. Nothing in /usr or the boot chain changes.
#   sudo bash legion-go-s-hhd-fix.sh          put it in place (and restart HHD)
#   sudo bash legion-go-s-hhd-fix.sh undo     take it away again (and restart HHD)
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "Run it with sudo."; exit 1; }
DIR=/var/lib/hearth/hhd-gos
DROP=/etc/systemd/system/hhd.service.d/zz-hearth-gos-fix.conf
if [ "${1:-}" = undo ]; then
  rm -f "$DROP"; rm -rf "$DIR"; systemctl daemon-reload; systemctl restart hhd
  echo "Taken away: Handheld Daemon is back to the built-in one."; exit 0
fi
grep -qi 'Legion Go S' /sys/class/dmi/id/product_version /sys/class/dmi/id/product_name 2>/dev/null \
  || grep -q 'N: Name=.*Legion Go S' /proc/bus/input/devices || { echo "This isn't a Legion Go S: nothing changed."; exit 1; }
# Only when the parts the patched HHD needs are there: the keys (USB interface 0), the controller (6) and its settings
# channel (the interface with IN endpoint 0x84, as the kernel's hid-lenovo-go-s finds it; 3 on the Go S HHD was written
# for, 4 on the owner's, Nov 28). Otherwise nothing changes (the first try hid the gamepad and every control stopped).
have() { for d in /sys/bus/hid/devices/*:1A86:E31[01].*; do [ "$(cat "$d/../bInterfaceNumber" 2>/dev/null)" = "$1" ] && return 0; done; return 1; }
cfg=no; for d in /sys/bus/hid/devices/*:1A86:E31[01].*; do [ -e "$d/../ep_84" ] && cfg=yes; done; have 03 && cfg=yes
have 00 && have 06 && [ $cfg = yes ] || { echo "This Go S's controller doesn't show the parts Handheld Daemon needs: nothing changed (send logs)."; exit 1; }
SRC=$(/usr/lib/hhd/bin/python3 -c 'import hhd, os; print(os.path.dirname(hhd.__file__))')
TMP=$(mktemp -d "$DIR.new.XXXX" 2>/dev/null || { mkdir -p /var/lib/hearth; mktemp -d "$DIR.new.XXXX"; })
cp -a "$SRC" "$TMP/hhd"
/usr/lib/hhd/bin/python3 - "$TMP/hhd/device/legion_go/slim/base.py" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()


def sub(old, new, n=1):
    global s
    if s.count(old) != n:
        sys.exit("This Handheld Daemon isn't the one the fix is for (4.1.12): nothing changed.")
    s = s.replace(old, new)


# 1. Where the controller's parts are on this Go S (newer firmware numbers them differently).
HELPERS = '''
def _gos_ifaces():
    """Hearth: the Go S controller's USB interfaces that are there: {number: has IN endpoint 0x84}."""
    import glob, os as _os
    out = {}
    for d in glob.glob("/sys/bus/hid/devices/*:1A86:E31[01].*"):
        try:
            n = int(open(_os.path.join(d, "..", "bInterfaceNumber")).read().strip(), 16)
        except Exception:
            continue
        out[n] = _os.path.exists(_os.path.join(d, "..", "ep_84"))
    return out


def _gos_cfg_iface():
    """The settings channel: the interface with IN endpoint 0x84 (as the kernel's hid-lenovo-go-s finds it), else 3."""
    for n, ep84 in sorted(_gos_ifaces().items()):
        if ep84 and n != 6:
            return n
    return 3

'''
sub("\ndef plugin_run(", HELPERS + "\ndef plugin_run(")
# 2. The settings channel wherever it is.
sub("        interface=3,\n", "        interface=_gos_cfg_iface(),\n", 2)
# 3. No touchpad part on this Go S: run without it (the touchpad stays the controller's own mouse) instead of failing.
sub('    uses_touch = d_params.get("uses_touch", False)\n',
    '    uses_touch = d_params.get("uses_touch", False)\n'
    '    if uses_touch and 2 not in _gos_ifaces():\n'
    '        logger.warning("Hearth: this Go S shows no touchpad part (USB interface 2): the touchpad stays a mouse.")\n'
    '        uses_touch = False\n')
# 4. The shortcut keys: by name, or by their keys when this firmware names them just "Legion Go S".
OLD = '''        name=[re.compile(".+Keyboard")],  # "  Legion Controller for Windows  Keyboard"
        # capabilities={EC("EV_KEY"): [EC("KEY_1")]},
        # report_size=64,
        required=True,
    )
'''
sub(OLD, OLD + '''    # Hearth: newer Go S firmware names its keyboard part just "Legion Go S"; find it by its keys then.
    if not any(re.match(".+Keyboard", i.get("name", "") or "") for i in enumerate_evs(vid=GOS_VID).values()):
        d_shortcuts = GenericGamepadEvdev(
            vid=[GOS_VID],
            pid=list(GOS_PIDS),
            capabilities={EC("EV_KEY"): [EC("KEY_1")]},
            required=True,
        )
''')
open(p, "w").write(s)
PY
/usr/lib/hhd/bin/python3 -m py_compile "$TMP/hhd/device/legion_go/slim/base.py"
PYTHONPATH="$TMP" /usr/lib/hhd/bin/python3 -c 'import hhd.device.legion_go.slim.base'
rm -rf "$DIR.old"; [ -d "$DIR" ] && mv "$DIR" "$DIR.old"; mv "$TMP" "$DIR"; rm -rf "$DIR.old"
mkdir -p "$(dirname "$DROP")"
printf '%s\n' '# Hearth: the Legion Go S fix for Handheld Daemon (tools/legion-go-s-hhd-fix.sh; "undo" takes it away).' \
  '[Service]' "Environment=PYTHONPATH=$DIR" > "$DROP"
systemctl daemon-reload
since=$(date '+%Y-%m-%d %H:%M:%S'); systemctl restart hhd
# Watch it for 20 s: if HHD still keeps losing the controller, take the fix away by itself so the controls come back.
echo "Checking the controller for 20 seconds..."; sleep 20
fails=$(journalctl -u hhd --since "$since" --no-pager -o cat 2>/dev/null | grep -c 'Assuming controller disconnected' || true)
if [ "${fails:-0}" -ge 2 ]; then
  rm -f "$DROP"; rm -rf "$DIR"; systemctl daemon-reload; systemctl restart hhd
  echo "It didn't hold (HHD lost the controller $fails times), so the fix took itself away: controls are as before. Send logs."
  exit 1
fi
echo "Done: Handheld Daemon is running with the Go S fix and kept the controller. Undo any time: sudo bash $0 undo"
