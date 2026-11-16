#!/usr/bin/env bash
# Hearth TV: install it on a Google TV / Android TV set from a computer on the same network (docs/HEARTH-TV.md).
#
#   tools/hearth-tv/install.sh <TV address> [hearth-tv-<version>.apk]
#   tools/hearth-tv/install.sh <TV address> --pair <pairing address:port> <pairing code> [apk]   (Wireless debugging)
#
# What it does, in order (nothing is switched off here: Hearth TV's first start does TV Cleanup, and asks):
#   1. connects to the TV's adb (the TV asks "Allow debugging?": tick Always allow);
#   2. saves the TV's package list and settings to ~/hearth-tv-backup/<date>/ (and a restore script beside them);
#   3. installs Hearth TV (the APK given, or the newest from GitHub's releases);
#   4. lets Hearth TV update itself and show its notice, keeps network debugging on (adb tcpip 5555) so Hearth TV's
#      TV Cleanup can talk to the TV's own adb, and makes Hearth TV the home screen (Google TV's home stays installed);
#   5. opens Hearth TV.
# Undo everything: tools/hearth-tv/restore.sh <TV address>.
set -euo pipefail

ADB=${ADB:-$(command -v adb || true)}
[ -n "$ADB" ] || ADB="$HOME/platform-tools/adb"
[ -x "$ADB" ] || { echo "adb not found: install Android's platform-tools or set ADB=/path/to/adb"; exit 1; }

TV=${1:?usage: install.sh <TV address> [--pair host:port code] [apk]}; shift
case "$TV" in *:*) ;; *) TV="$TV:5555" ;; esac
if [ "${1:-}" = "--pair" ]; then
  "$ADB" pair "$2" "$3"; shift 3
fi
APK=${1:-}

say() { printf '\n== %s\n' "$*"; }
say "Connecting to the TV at $TV (look at the TV: tick Always allow, then Allow)"
"$ADB" connect "$TV" >/dev/null || true
for i in $(seq 1 60); do
  st=$("$ADB" -s "$TV" get-state 2>/dev/null || true)
  [ "$st" = device ] && break
  [ $((i % 10)) -eq 1 ] && echo "   waiting for the TV ($("$ADB" devices | grep "$TV" | awk '{print $2}'))…"
  sleep 2
done
[ "$("$ADB" -s "$TV" get-state 2>/dev/null)" = device ] || { echo "The TV didn't allow the connection. Turn on Developer options › Network debugging (or USB debugging) on the TV and try again."; exit 1; }
sh() { "$ADB" -s "$TV" shell "$@"; }
echo "   connected: $(sh getprop ro.product.manufacturer | tr -d '\r') $(sh getprop ro.product.model | tr -d '\r'), Android $(sh getprop ro.build.version.release | tr -d '\r')"

BK="$HOME/hearth-tv-backup/$(date +%Y-%m-%d-%H%M%S)"
say "Saving the TV's package list and settings to $BK"
mkdir -p "$BK"
sh pm list packages -f > "$BK/packages-all.txt"
sh pm list packages -d > "$BK/packages-disabled.txt"
sh pm list packages -e > "$BK/packages-enabled.txt"
for n in global secure system; do sh settings list $n > "$BK/settings-$n.txt"; done
sh cmd package resolve-activity --brief -c android.intent.category.HOME -a android.intent.action.MAIN > "$BK/home-before.txt" || true
getprop_all=$(sh getprop) && printf '%s\n' "$getprop_all" > "$BK/getprop.txt"
cp "$(dirname "$0")/restore.sh" "$BK/restore.sh" 2>/dev/null || true
echo "   saved ($(wc -l < "$BK/packages-all.txt") packages)"

if [ -z "$APK" ]; then
  say "Getting the newest Hearth TV from GitHub"
  url=$(curl -fsSL "https://api.github.com/repos/Dragongamer9/hearth-os/releases?per_page=15" | grep -o '"browser_download_url": *"[^"]*hearth-tv-[0-9.]*\.apk"' | head -1 | sed 's/.*"\(http[^"]*\)"/\1/')
  [ -n "$url" ] || { echo "No Hearth TV release yet: download the hearth-tv-<version> artifact from GitHub Actions › Hearth TV, unzip it, and run: install.sh $TV path/to/hearth-tv-<version>.apk"; exit 1; }
  APK="$BK/$(basename "$url")"; curl -fL -o "$APK" "$url"
fi
say "Installing $(basename "$APK")"
"$ADB" -s "$TV" install -r "$APK"

say "Letting Hearth TV update itself and keep TV Cleanup working"
sh appops set dev.hearthos.tv REQUEST_INSTALL_PACKAGES allow || true
sh pm grant dev.hearthos.tv android.permission.POST_NOTIFICATIONS 2>/dev/null || true
sh cmd package set-home-activity dev.hearthos.tv/.MainActivity >/dev/null 2>&1 || true
# Android 10 and up pick the home app by its role; set-home-activity alone isn't always enough (TCL, Android 12).
sh cmd role add-role-holder android.app.role.HOME dev.hearthos.tv 0 >/dev/null 2>&1 || true
echo "   Hearth TV is the home screen (if Home still opens Google TV: Hearth TV's TV Cleanup › Google TV's home)"
# Network debugging stays on for Hearth TV's own TV Cleanup (127.0.0.1:5555), until the TV restarts.
"$ADB" -s "$TV" tcpip 5555 >/dev/null 2>&1 || true
sleep 3; "$ADB" connect "$TV" >/dev/null 2>&1 || true

say "Opening Hearth TV"
sh am start -n dev.hearthos.tv/.MainActivity >/dev/null
cat <<EOF

Done. On the TV, Hearth TV's first start walks you through your PC, your apps and TV Cleanup.
When it cleans up, the TV asks once more to "Allow debugging" (this time for Hearth TV itself): tick Always allow.

Your backup: $BK
Put the TV back the way it was any time: $(dirname "$0")/restore.sh ${TV%:5555}
EOF
