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
# Only when every part HHD needs is there (USB interfaces 2: touchpad, 3: settings, 6: controller): on a Go S that shows
# fewer (the owner's, Nov 28: 0, 4, 5, 6), taking over hid the gamepad and every control stopped. Then nothing changes.
have() { for d in /sys/bus/hid/devices/*:1A86:E31[01].*; do [ "$(cat "$d/../bInterfaceNumber" 2>/dev/null)" = "$1" ] && return 0; done; return 1; }
for i in 02 03 06; do have "$i" || { echo "This Go S's controller doesn't show part $i that Handheld Daemon needs: nothing changed (send logs)."; exit 1; }; done
SRC=$(/usr/lib/hhd/bin/python3 -c 'import hhd, os; print(os.path.dirname(hhd.__file__))')
TMP=$(mktemp -d "$DIR.new.XXXX" 2>/dev/null || { mkdir -p /var/lib/hearth; mktemp -d "$DIR.new.XXXX"; })
cp -a "$SRC" "$TMP/hhd"
/usr/lib/hhd/bin/python3 - "$TMP/hhd/device/legion_go/slim/base.py" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = '''        name=[re.compile(".+Keyboard")],  # "  Legion Controller for Windows  Keyboard"
        # capabilities={EC("EV_KEY"): [EC("KEY_1")]},
        # report_size=64,
        required=True,
    )
'''
new = old + '''    # Hearth: newer Go S firmware names its keyboard part just "Legion Go S"; find it by its keys then.
    if not any(re.match(".+Keyboard", i.get("name", "") or "") for i in enumerate_evs(vid=GOS_VID).values()):
        d_shortcuts = GenericGamepadEvdev(
            vid=[GOS_VID],
            pid=list(GOS_PIDS),
            capabilities={EC("EV_KEY"): [EC("KEY_1")]},
            required=True,
        )
'''
if s.count(old) != 1:
    sys.exit("This Handheld Daemon isn't the one the fix is for (4.1.12): nothing changed.")
s = s.replace(old, new)
if 'enumerate_evs' not in s.split('def ')[0]:
    s = s.replace('import re\n', 'import re\nfrom hhd.controller.physical.evdev import enumerate_evs\n', 1)
open(p, 'w').write(s)
PY
/usr/lib/hhd/bin/python3 -m py_compile "$TMP/hhd/device/legion_go/slim/base.py"
PYTHONPATH="$TMP" /usr/lib/hhd/bin/python3 -c 'import hhd.device.legion_go.slim.base'
rm -rf "$DIR.old"; [ -d "$DIR" ] && mv "$DIR" "$DIR.old"; mv "$TMP" "$DIR"; rm -rf "$DIR.old"
mkdir -p "$(dirname "$DROP")"
printf '%s\n' '# Hearth: the Legion Go S fix for Handheld Daemon (tools/legion-go-s-hhd-fix.sh; "undo" takes it away).' \
  '[Service]' "Environment=PYTHONPATH=$DIR" > "$DROP"
systemctl daemon-reload; systemctl restart hhd
echo "Done: Handheld Daemon restarted with the Go S fix. Undo any time: sudo bash $0 undo"
