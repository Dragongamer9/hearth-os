# Settings › System › Your desktop with labwc's two choices (1.16, feature/labwc-fedora; docs/LABWC-FEDORA.md):
# all four desktops offered, each picked once, and the "can't start yet" line when the image lacks Quickshell.
# Uses settings-system.sh's answers folder (HEARTH_SYSTEM_FIXTURE), with its own deskchoice answer.
LWFIX=/tmp/settings-labwc-fixture
rm -rf "$LWFIX" && mkdir -p "$LWFIX"
[ -d /tmp/settings-system-fixture ] && cp -a /tmp/settings-system-fixture/. "$LWFIX/"
lw() { # lw NAME FP-CHOICE LABWC-CHOICE MISSING-JSON
  printf '#@fp\n%s\n#@has\n/usr/bin/hearth-firehold-plasma\n/usr/bin/hearth-labwc-desktop\n#@lw\n{"choice": "%s", "available": true, "missing": %s, "fails": 0, "why": ""}\n' "$2" "$3" "$4" > "$LWFIX/deskchoice"
  HEARTH_SYSTEM_FIXTURE="$LWFIX" XDG_CURRENT_DESKTOP=KDE run "$1" hearth-settings 0 "$BIN/hearth-settings-app" --page your-desktop
}
lw settings-sys-your-desktop-4-plasma plasma none '[]'
lw settings-sys-your-desktop-4-firehold-plasma firehold none '[]'
lw settings-sys-your-desktop-4-classic plasma classic '[]'
lw settings-sys-your-desktop-4-firehold-labwc plasma firehold '[]'
lw settings-sys-your-desktop-4-missing plasma classic '["Quickshell"]'
rm -rf "$LWFIX"
