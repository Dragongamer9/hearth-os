#!/bin/bash
# Firehold on Plasma at rest (the owner's red flag: his devices must never freeze or lag again). Plain KDE Plasma and
# Firehold's Couch one after the other on the same runner, each its own KWin on a virtual 1920x1080 screen
# with software drawing (CI has no GPU), nothing open, for SECS (600: ten minutes). Measured after the start's own
# work: the processor time of plasmashell and KWin (and Firehold's session helper), and plasmashell's memory at the
# start and the end.
#   firehold-plasma-idle.sh [SECS]
# Fails (::error::) when Firehold's plasmashell uses clearly more than plain Plasma's (more than plain + 0.5 points of
# a core and more than 1%), its KWin more than plain's + 1 point, its helper more than 0.5%, or Firehold's plasmashell
# grows by more than plain's growth + 15 MB.
set +e +o pipefail
SECS=${1:-600}

cat > /tmp/idle-one.sh <<'IN'
set +e
KIND=$1; SECS=$2
export XDG_RUNTIME_DIR=/tmp/xdg-$KIND HOME=/tmp/home-$KIND LC_ALL=C.UTF-8 LIBGL_ALWAYS_SOFTWARE=1
export XDG_CURRENT_DESKTOP=KDE XDG_SESSION_TYPE=wayland KDE_FULL_SESSION=true KDE_SESSION_VERSION=6
mkdir -p "$XDG_RUNTIME_DIR" "$HOME"; chmod 700 "$XDG_RUNTIME_DIR"
[ -e /etc/xdg/menus/applications.menu ] || export XDG_MENU_PREFIX=$(basename "$(ls /etc/xdg/menus/*-applications.menu | head -1)" applications.menu)
HZ=$(getconf CLK_TCK)
ticks() { awk '{print $14 + $15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
rss() { awk '/^VmRSS:/{print int($2 / 1024)}' "/proc/$1/status" 2>/dev/null || echo 0; }
cp "$(command -v kwin_wayland)" /tmp/kwin-$KIND
/tmp/kwin-$KIND --virtual --width 1920 --height 1080 --no-lockscreen --socket idle-$KIND >/tmp/idle-$KIND-kwin.log 2>&1 &
C=$!
for i in $(seq 120); do [ -S "$XDG_RUNTIME_DIR/idle-$KIND" ] && break; sleep 0.5; done
export WAYLAND_DISPLAY=idle-$KIND QT_QPA_PLATFORM=wayland
for k in /usr/libexec/kactivitymanagerd /usr/lib64/libexec/kactivitymanagerd /usr/bin/kactivitymanagerd; do [ -x "$k" ] && { "$k" >/dev/null 2>&1 & break; }; done
for k in /usr/libexec/kglobalacceld /usr/lib64/libexec/kglobalacceld /usr/bin/kglobalacceld; do [ -x "$k" ] && { "$k" >/dev/null 2>&1 & break; }; done
sleep 2
H=""
if [ "$KIND" = firehold ]; then
  export HEARTH_FIREHOLD_PLASMA=1 PLASMA_DEFAULT_SHELL=org.hearthos.firehold
  mkdir -p "$HOME/.cache/hearth"
  hearth-firehold-plasma begin org.hearthos.firehold
  plasmashell --no-respawn -p org.hearthos.firehold >/tmp/idle-$KIND-shell.log 2>&1 &
  S=$!
  hearth-firehold-plasma session >/tmp/idle-$KIND-helper.log 2>&1 &
  H=$!
else
  plasmashell --no-respawn -p org.kde.plasma.desktop >/tmp/idle-$KIND-shell.log 2>&1 &
  S=$!
fi
sleep 60     # the start's own work done
if ! kill -0 "$S" 2>/dev/null || ! kill -0 "$C" 2>/dev/null; then echo "RESULT $KIND gone gone gone 0 0 0"; echo "$KIND: shell $(kill -0 $S 2>/dev/null && echo up || echo gone), kwin $(kill -0 $C 2>/dev/null && echo up || echo gone)" >&2; tail -5 /tmp/idle-$KIND-kwin.log >&2; exit 0; fi
s0=$(ticks $S); c0=$(ticks $C); m0=$(rss $S); h0=0; [ -n "$H" ] && h0=$(ticks $H)
sleep "$SECS"
s1=$(ticks $S); c1=$(ticks $C); m1=$(rss $S); h1=0; [ -n "$H" ] && h1=$(ticks $H)
kill -0 "$S" 2>/dev/null || { echo "RESULT $KIND gone gone gone $m0 0 0"; exit 0; }
pc() { echo $(( ($2 - $1) * 10000 / HZ / SECS )); }
echo "RESULT $KIND $(pc $s0 $s1) $(pc $c0 $c1) $(pc $h0 $h1) $m0 $m1 $(rss $C)"
kill $S $C $H 2>/dev/null
IN

# One after the other (two KWins at once in one container didn't keep running).
dbus-run-session -- bash /tmp/idle-one.sh plasma "$SECS" > /tmp/idle-plasma.out 2>/tmp/idle-plasma.err
sleep 5
dbus-run-session -- bash /tmp/idle-one.sh firehold "$SECS" > /tmp/idle-firehold.out 2>/tmp/idle-firehold.err
read -r _ _ ps pc ph pm0 pm1 pcm <<<"$(grep '^RESULT' /tmp/idle-plasma.out | tail -1)"
read -r _ _ fs fc fh fm0 fm1 fcm <<<"$(grep '^RESULT' /tmp/idle-firehold.out | tail -1)"
num() { case "$1" in ''|*[!0-9]*) return 1;; esac; }
p() { num "$1" && printf '%d.%02d%%' $(($1 / 100)) $(($1 % 100)) || echo "$1"; }
if ! num "$ps" || ! num "$fs"; then
  for k in plasma firehold; do
    echo "::warning title=at rest, $k::$(grep -v udisks2 /tmp/idle-$k-shell.log 2>/dev/null | tail -25 | cut -c1-200 | tr '\n' '|' | cut -c1-3000) || helper: $(tail -c 600 /tmp/idle-$k-helper.log /tmp/home-$k/.local/state/nimbus/firehold-plasma.log 2>/dev/null | tr '\n' '|')"
  done
  echo "::error title=at rest::no result (plain: '$ps', Firehold: '$fs'): $(tail -c 400 /tmp/idle-firehold.err 2>/dev/null | tr '\n' ' ')"
  exit 1
fi
msg="over ${SECS} s at rest (virtual screen, software drawing): plasmashell $(p "$ps") plain vs $(p "$fs") Firehold of a core; KWin $(p "$pc") vs $(p "$fc"); Firehold's helper $(p "$fh"); plasmashell memory ${pm0} → ${pm1} MB plain vs ${fm0} → ${fm1} MB Firehold; KWin ${pcm} vs ${fcm} MB"
echo "::notice title=at rest: plain Plasma vs Firehold::$msg"
bad=""
[ "$fs" -gt $((ps + 50)) ] && [ "$fs" -gt 100 ] && bad="$bad plasmashell uses more processor time;"
[ "$fc" -gt $((pc + 100)) ] && bad="$bad KWin uses more processor time;"
[ "$fh" -gt 50 ] && bad="$bad the helper uses more than 0.5%;"
[ $((fm1 - fm0)) -gt $(( (pm1 - pm0) + 15 )) ] && bad="$bad plasmashell's memory grows;"
if [ -n "$bad" ]; then echo "::error title=at rest (red flag)::Firehold:$bad $msg"; exit 1; fi
echo "RESULT plain $ps $pc $pm0 $pm1 firehold $fs $fc $fh $fm0 $fm1"
exit 0
