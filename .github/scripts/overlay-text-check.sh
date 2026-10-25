#!/bin/bash
# Quick Access over games shows text: run inside the built image (base-anatase.yml, base-peppermint.yml).
#
# Handheld Daemon starts its overlay app (/usr/bin/hhd-ui = hearth-overlay) as the user with only HOME, DISPLAY,
# STEAM_OVERLAY=1 and PATH=/usr/bin:/bin (hhd/plugins/overlay/overlay.py inject_overlay). On the Nvidia PC (Anatase,
# 0.6.5) Quick Access came up there with no text at all: Chromium's own fontconfig (older than Fedora 45's 2.17) found
# no font ("Could not find any font: Sans, sans") and complained about /etc/fonts/conf.d. This starts the overlay app
# the same way on a virtual screen, in its self-test mode (HEARTH_OVERLAY_SELFTEST=1: it draws its own Quick Access
# page and says whether text shows), and fails when no text shows or Chromium still finds a font file it can't read.
# Also reported (not failed): the Electron started bare, the way 0.6.5 started it (no Hearth font setup).
set -u
fail=0
enc() { sed -e 's/%/%25/g' | sed -e ':a;N;$!ba;s/\n/%0A/g' | cut -c1-3800; }
note() { printf '::notice title=%s::%s\n' "$1" "$(printf '%s' "$2" | enc)"; }
warn() { printf '::warning title=%s::%s\n' "$1" "$(printf '%s' "$2" | enc)"; }
err() { printf '::error title=%s::%s\n' "$1" "$(printf '%s' "$2" | enc)"; fail=1; }

APP=/usr/lib/hearth-browser/nimbus
HHDUI=/usr/bin/hhd-ui
[ -x "$APP" ] || { err "overlay text" "$APP (Hearth Browser's Electron) isn't installed"; exit 1; }
[ -x "$HHDUI" ] || { err "overlay text" "$HHDUI (hearth-overlay) isn't installed"; exit 1; }

# fontconfig in the image, as the system sees it.
sysfc=$( { rpm -q fontconfig 2>/dev/null || dpkg-query -W fontconfig 2>/dev/null; echo "fc-match sans-serif: $(fc-match sans-serif 2>&1)"; echo "fc-match Sans: $(fc-match Sans 2>&1)"; echo "fc-list: $(fc-list 2>/dev/null | wc -l) fonts"; \
  for f in 48-guessfamily 48-spacing 49-sansserif; do [ -e /etc/fonts/conf.d/$f.conf ] && echo "conf.d/$f.conf -> $(readlink -f /etc/fonts/conf.d/$f.conf) ($(rpm -qf "$(readlink -f /etc/fonts/conf.d/$f.conf)" 2>/dev/null || dpkg -S "$(readlink -f /etc/fonts/conf.d/$f.conf)" 2>/dev/null | cut -d: -f1))"; done; \
  echo "system fontconfig warnings: $(fc-match sans-serif 2>&1 >/dev/null | head -3)"; } 2>&1)
note "fontconfig in the image" "$sysfc"
[ "$(fc-list 2>/dev/null | wc -l)" -gt 0 ] || err "fonts" "fc-list finds no fonts in the image"
fc-match sans-serif >/dev/null 2>&1 || err "fonts" "fc-match sans-serif fails in the image"

# A virtual screen.
# (Xvfb also needs xkbcomp and the keyboard data to start.)
if ! command -v Xvfb >/dev/null 2>&1 || ! command -v xkbcomp >/dev/null 2>&1; then
  { dnf5 -y install xorg-x11-server-Xvfb xorg-x11-xkb-utils xkeyboard-config || dnf -y install xorg-x11-server-Xvfb xorg-x11-xkb-utils xkeyboard-config || \
    { mkdir -p /var/lib/apt/lists/partial /var/cache/apt/archives/partial && apt-get update && apt-get install -y --no-install-recommends xvfb x11-xkb-utils xkb-data; }; } >/tmp/xvfb-install.log 2>&1
fi
mkdir -p /tmp/.X11-unix && chmod 1777 /tmp/.X11-unix
# Xvfb crashes in some build containers (or can't be installed in the image); then Chromium draws headless instead
# (--ozone-platform=headless). Skia and fontconfig still lay out and rasterise the text there, so whether a font is
# found (and whether Chromium's fontconfig complains) is checked the same way: the check always runs.
HEADLESS=()
if command -v Xvfb >/dev/null 2>&1; then
  Xvfb :7 -screen 0 1280x800x24 -nolisten tcp >/tmp/xvfb.log 2>&1 &
  for _ in $(seq 1 75); do [ -e /tmp/.X11-unix/X7 ] && break; sleep 0.2; done
else
  echo "no Xvfb: $(tail -3 /tmp/xvfb-install.log 2>/dev/null)" >/tmp/xvfb.log
fi
if [ ! -e /tmp/.X11-unix/X7 ]; then
  note "overlay text" "the virtual screen (Xvfb :7) didn't start here; drawing headless instead: $(tail -4 /tmp/xvfb.log)"
  HEADLESS=(--ozone-platform=headless)
fi

# A user like the owner's: a home folder, /run/user/<uid>, nothing else from a session.
mkdir -p /var/home 2>/dev/null
useradd -m -d /var/home/ovtest ovtest >/dev/null 2>&1 || useradd -m ovtest >/dev/null 2>&1
uid=$(id -u ovtest); gid=$(id -g ovtest); home=$(getent passwd ovtest | cut -d: -f6)
mkdir -p "/run/user/$uid" && chown "$uid:$gid" "/run/user/$uid" && chmod 700 "/run/user/$uid"
# HHD's environment, exactly (plus the self-test switch). --no-sandbox: no setuid sandbox or user namespaces in the
# build container; --disable-gpu: no GPU on the runner. Neither changes how fonts are found.
as_hhd() { setpriv --reuid="$uid" --regid="$gid" --init-groups env -i HOME="$home" DISPLAY=:7 STEAM_OVERLAY=1 PATH=/usr/bin:/bin HEARTH_OVERLAY_SELFTEST=1 "$@"; }

run() {  # run <name> <command...>: the self-test's JSON line and Chromium's font complaints
  local name=$1; shift
  ( sleep 70 | as_hhd timeout 60 "$@" --no-sandbox --disable-gpu "${HEADLESS[@]}" >"/tmp/ov-$name.log" 2>&1 ); echo $? >"/tmp/ov-$name.code"
}

# 1) the way 0.6.5 started it: the Electron alone, no Hearth font setup (reported only).
run bare "$APP" --hhd-overlay
# 2) the way Handheld Daemon starts it now: hearth-overlay (hhd-ui).
run hhd "$HHDUI"

for name in bare hhd; do
  log=/tmp/ov-$name.log; code=$(cat /tmp/ov-$name.code)
  res=$(grep -o 'overlay-selftest: .*' "$log" | tail -1)
  nofont=$(grep -c 'Could not find any font' "$log")
  fcw=$(grep -c 'Fontconfig warning\|Fontconfig error' "$log")
  summary="exit $code; ${res:-no self-test line}; 'Could not find any font': $nofont; fontconfig warnings: $fcw
$(grep -m3 'Fontconfig' "$log")
$(grep 'hearth-overlay:' "$log" | head -4)
$(grep -m2 'Could not find any font\|ERROR' "$log")"
  if [ "$name" = bare ]; then note "overlay as 0.6.5 started it (bare Electron, HHD's environment)" "$summary"; continue; fi
  if printf '%s' "$res" | grep -q '"ok":true' && [ "$nofont" = 0 ]; then
    note "overlay text shows (as Handheld Daemon starts it)" "$summary"
  else
    err "overlay text (as Handheld Daemon starts it)" "Quick Access drew no text: $summary
--- log tail:
$(tail -25 "$log")"
  fi
  if [ "$fcw" != 0 ]; then
    conf="/run/user/$uid/hearth-overlay-fonts/fonts.conf"
    err "overlay fontconfig" "Chromium's fontconfig still complains ($fcw warnings) about the setup hearth-overlay gives it: $(grep -m5 'Fontconfig' "$log")
--- files in $conf: $(grep -o '<include[^>]*>[^<]*</include>' "$conf" 2>/dev/null | sed 's/<[^>]*>//g' | tr '\n' ' ')
--- where they came from: $(grep -h -o '<include[^>]*>[^<]*</include>' /etc/fonts/conf.d/*.conf 2>/dev/null | sort -u | head -20 | tr '\n' ' ')"
  else
    note "overlay fontconfig" "0 fontconfig warnings from Chromium with hearth-overlay's setup"
  fi
done
exit "$fail"
