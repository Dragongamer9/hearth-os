#!/bin/bash
# Legion Go S test (the owner's, Nov 28, 2026): the Xbox driver (xpad) holds USB interfaces 1.2 (touchpad) and 1.3
# (settings channel) of the controller, so Handheld Daemon can't find them. This frees 1.1-1.3 from xpad, gives 1.2 and
# 1.3 to the normal HID driver (usbhid) and 1.1 (the gamepad) back to xpad, then shows what each has and what HHD says.
# Only until the next restart: a restart puts everything back as it was.
[ "$(id -u)" = 0 ] || { echo "Run it with sudo."; exit 1; }
D=/sys/bus/usb/drivers
dev=""
for i in /sys/bus/usb/devices/*; do
  [ "$(cat "$i/idVendor" 2>/dev/null)" = 1a86 ] && case "$(cat "$i/idProduct" 2>/dev/null)" in e310|e311) dev=$(basename "$i");; esac
done
[ -n "$dev" ] || { echo "No Legion Go S controller found: nothing changed."; exit 1; }
echo "Controller: $dev"
for n in 1 2 3; do echo "$dev:1.$n" > $D/xpad/unbind 2>/dev/null && echo "freed $dev:1.$n from xpad"; done
for n in 2 3; do echo "$dev:1.$n" > $D/usbhid/bind && echo "gave $dev:1.$n to usbhid"; done
echo "$dev:1.1" > $D/xpad/bind && echo "gave $dev:1.1 back to xpad"
sleep 8
for n in 0 1 2 3 4 5 6; do echo "  1.$n -> $(basename "$(readlink -f "/sys/bus/usb/devices/$dev:1.$n/driver" 2>/dev/null)")"; done
ls /sys/bus/hid/devices/ | grep -i 1a86
echo "--- Handheld Daemon:"; journalctl -u hhd -n 14 --no-pager -o cat
echo "--- kernel:"; dmesg | tail -n 10
