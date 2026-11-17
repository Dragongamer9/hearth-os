#!/usr/bin/env bash
# Hearth TV: put the TV back the way it shipped (docs/HEARTH-TV.md). Every package Hearth TV's TV Cleanup can switch
# off is switched back on, and Google TV's home is the home screen again. Hearth TV stays installed unless you add
# --uninstall.
#   tools/hearth-tv/restore.sh <TV address> [--uninstall]
set -euo pipefail
ADB=${ADB:-$(command -v adb || true)}; [ -n "$ADB" ] || ADB="$HOME/platform-tools/adb"
TV=${1:?usage: restore.sh <TV address> [--uninstall]}; case "$TV" in *:*) ;; *) TV="$TV:5555" ;; esac
"$ADB" connect "$TV" >/dev/null || true
sh() { "$ADB" -s "$TV" shell "$@"; }
PKGS="com.tcl.partnercustomizer com.tcl.channelplus com.tcl.messagebox com.tcl.esticker com.tcl.exhibit com.tcl.tv.recommendation
com.google.android.tvrecommendations com.google.android.leanbacklauncher.recommendations tv.anoki.acr.anokiacroptin com.tcl.logkit
com.tcl.interactive com.google.android.feedback com.android.tv.feedbackconsent com.tcl.waterfall.overseas com.tcl.dashboard
com.tcl.tv.tclhome_passive com.tcl.magiconnectfree com.tcl.smartalexa com.tcl.hotelmenu com.tcl.ocean.instructions
com.tcl.repairguide com.tcl.gamebar com.tcl.usercenter com.google.android.apps.tv.launcherx com.google.android.tvlauncher
com.google.android.katniss tv.samba.ssm.ui com.tcl.tv.plus com.google.android.tungsten.setupwraith"
for p in $PKGS; do
  if sh pm list packages -d | tr -d '\r' | grep -qx "package:$p"; then sh pm enable "$p" >/dev/null && echo "switched back on: $p"; fi
done
home=$(sh cmd package resolve-activity --brief -c android.intent.category.HOME -a android.intent.action.MAIN -p com.google.android.apps.tv.launcherx 2>/dev/null | tr -d '\r' | tail -1)
[ -n "$home" ] && sh cmd package set-home-activity "$home" >/dev/null 2>&1 && echo "Google TV's home is the home screen again"
svc=dev.hearthos.tv/dev.hearthos.tv.HomeButtonService
cur=$(sh settings get secure enabled_accessibility_services | tr -d '\r')
case "$cur" in *"$svc"*) new=$(printf %s "$cur" | tr ':' '\n' | grep -vx "$svc" | paste -sd:); if [ -n "$new" ]; then sh settings put secure enabled_accessibility_services "$new"; else sh settings delete secure enabled_accessibility_services; fi; echo "Home button helper off" ;; esac
if [ "${2:-}" = "--uninstall" ]; then "$ADB" -s "$TV" uninstall dev.hearthos.tv && echo "Hearth TV uninstalled"; fi
echo "Done."
