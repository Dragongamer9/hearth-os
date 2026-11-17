# KDE Plasma and GNOME in nimbus-session (1.13 and 1.14 before KWin), archived Nov 16, 2026: the parts of
# hosts/linux/session/nimbus-session that picked, started, watched and tidied up after them. Not run by anything.
# ---- 1.13: the desktop for Desktop Mode: Mantel (Hearth Classic, always built in), KDE Plasma or GNOME (add-ons
# downloaded when picked: docs/DESKTOPS.md). The login screen's entries pass --desktop=classic|kde|gnome, which is
# saved (HEARTH_DESKTOP= in session.conf) so automatic sign-in keeps it; anything else is only logged.
for a in "$@"; do
  case "$a" in
    --desktop=classic|--desktop=kde|--desktop=plasma|--desktop=gnome) want_desktop="${a#--desktop=}" ;;
    --desktop=*) log "desktop: ${a#--desktop=} asked for on the login screen; not a desktop Hearth has" ;;
  esac
done
# The desktop picked for Desktop Mode, read again at every start (Settings can change it while Game Mode runs).
# plasma (an earlier Hearth's name) and kde are the same; cosmic and deepin aren't offered any more: Mantel.
desktop_choice() {
  # "Pick at the login screen too: Just that time" (Settings): the login screen's pick for this sign-in only.
  o=$(cat "$RUN/hearth-desktop-once" 2>/dev/null)
  case "$o" in classic|kde|gnome) echo "$o"; return 0 ;; esac
  d=$(sed -n 's/^[[:space:]]*\(export[[:space:]]*\)\{0,1\}HEARTH_DESKTOP=["'"'"']*\([a-z]*\).*/\2/p' "$SESSION_CONF" 2>/dev/null | tail -n 1)
  case "$d" in kde|plasma) echo kde ;; gnome) echo gnome ;; *) echo classic ;; esac
}
# The login screen's pick, saved like Settings saves it (only that one line changes; the rest of session.conf stays).
save_desktop_choice() {
  case "$1" in plasma) set -- kde ;; esac
  case "$1" in classic|kde|gnome) ;; *) return 0 ;; esac
  # Settings › Your desktop › Pick at the login screen too: "This one" (the default) saves it; "Just that time" doesn't.
  lp=$(sed -n 's/^[[:space:]]*\(export[[:space:]]*\)\{0,1\}HEARTH_LOGIN_PICK=["'"'"']*\([a-z]*\).*/\2/p' "$SESSION_CONF" 2>/dev/null | tail -n 1)
  if [ "$lp" = once ]; then echo "$1" >"$RUN/hearth-desktop-once"; log "desktop: $1 picked on the login screen (this sign-in only)"; return 0; fi
  rm -f "$RUN/hearth-desktop-once"
  [ "$(desktop_choice)" = "$1" ] && return 0
  mkdir -p "$(dirname "$SESSION_CONF")"
  tmp="$SESSION_CONF.tmp.$$"
  { grep -v '^[[:space:]]*\(export[[:space:]]*\)\{0,1\}HEARTH_DESKTOP=' "$SESSION_CONF" 2>/dev/null; echo "HEARTH_DESKTOP=$1"; } >"$tmp" \
    && { [ -f "$SESSION_CONF" ] && cp -f "$SESSION_CONF" "$SESSION_CONF.bak"; mv -f "$tmp" "$SESSION_CONF"; } || rm -f "$tmp"
  log "desktop: $1 picked on the login screen (saved)"
}
[ -n "${want_desktop:-}" ] && save_desktop_choice "$want_desktop"
unset want_desktop
  # 1.13: KDE Plasma or GNOME when picked and its add-on is here (merged by systemd-sysext); Mantel otherwise, and
  # whenever it didn't come up (no desktop within 60 s, or it crashed straight away): for the rest of this sign-in.
  # Twice in a row → Mantel until it starts again from Settings, and Settings says why. The pick stays saved.
  d=$(desktop_choice)
  if [ "$d" != classic ] && [ -z "$HEARTH_SEAT" ]; then
    if [ -f /run/hearth/safe-graphics ]; then log "$d: safe graphics this time, Mantel instead"
    elif [ -e "$RUN/hearth-$d-failed" ]; then log "$d: didn't start earlier this sign-in, Mantel instead"
    elif [ "$(desk_fails "$d")" -ge 2 ]; then log "$d: didn't start twice in a row; Mantel until you start it again from Settings"; desk_notice "$d" twice
    elif ! "can_$d"; then log "$d: its add-on isn't here (not downloaded yet, or made for another Hearth version); Mantel instead"; desk_notice "$d" missing
    else
      "run_$d"; drc=$?
      if [ "$drc" -ne 200 ]; then desk_fails_reset "$d"; return "$drc"; fi
      : >"$RUN/hearth-$d-failed"; desk_fails_add "$d"
      log "$d: didn't start; Mantel for this sign-in"
      desk_notice "$d" failed
      "settle_$d"
    fi
    have labwc || return 1
  fi
  run_classic
}

# ---- KDE Plasma and GNOME (1.13, docs/DESKTOPS.md): their own look, with Hearth's tray item / quick settings section.
# Added by add-ons (systemd-sysext images in /var/lib/extensions, made for this exact build); nothing here may stop
# Mantel or Game Mode starting.
DESK_STATE="${XDG_STATE_HOME:-$HOME/.local/state}/hearth"
desk_fails() { n=$(sed -n "s/^$1 //p" "$DESK_STATE/desktop-fails" 2>/dev/null | tail -n 1); echo "${n:-0}"; }
desk_fails_set() { mkdir -p "$DESK_STATE"; { grep -v "^$1 " "$DESK_STATE/desktop-fails" 2>/dev/null; [ "$2" -gt 0 ] && echo "$1 $2"; } >"$DESK_STATE/desktop-fails.tmp" && mv -f "$DESK_STATE/desktop-fails.tmp" "$DESK_STATE/desktop-fails"; }
desk_fails_add() { desk_fails_set "$1" $(( $(desk_fails "$1") + 1 )); }
desk_fails_reset() { [ "$(desk_fails "$1")" -gt 0 ] && desk_fails_set "$1" 0; return 0; }
# Why Mantel opened instead: Settings › Desktop › Your desktop shows it (and Mantel's notice reads it once).
desk_notice() { mkdir -p "$DESK_STATE"; printf '%s %s %s\n' "$1" "$2" "$(date +%s)" >"$DESK_STATE/desktop-notice"; }
can_kde() { have startplasma-wayland && have kwin_wayland && have plasmashell; }
can_gnome() { have gnome-session && have gnome-shell; }
DESK_TIMEOUT="${HEARTH_DESKTOP_TIMEOUT:-60}"
# A process of this user by that name that's really running (not a zombie left by one that ended).
alive() {
  for p in $(pgrep -u "$(id -u)" -x "$1" 2>/dev/null); do
    st=$(sed -n 's/^State:[[:space:]]*\([A-Za-z]\).*/\1/p' "/proc/$p/status" 2>/dev/null)
    [ -n "$st" ] && [ "$st" != Z ] && return 0
  done
  return 1
}
# Start a desktop and wait for it to be up (all of the named programs running); 200 = didn't come up or crashed.
desk_run() {   # name, log, command, programs that mean "up"…
  dname=$1; dlog=$2; dcmd=$3; shift 3
  [ -f "$dlog" ] && [ "$(wc -c <"$dlog")" -gt 1000000 ] && mv -f "$dlog" "$dlog.old"
  echo "--- $(date '+%F %T') $dname" >>"$dlog"
  (
    unset QT_QPA_PLATFORMTHEME LABWC_PID WLR_NO_HARDWARE_CURSORS GDK_DEBUG
    case "$dname" in
      kde) export HEARTH_DESKTOP=kde XDG_CURRENT_DESKTOP=KDE XDG_SESSION_DESKTOP=KDE DESKTOP_SESSION=plasma ;;
      gnome) export HEARTH_DESKTOP=gnome XDG_CURRENT_DESKTOP=GNOME XDG_SESSION_DESKTOP=gnome DESKTOP_SESSION=gnome XDG_SESSION_TYPE=wayland ;;
    esac
    exec $dcmd
  ) >>"$dlog" 2>&1 &
  dpid=$!
  echo "$dpid" >"$RUN/hearth-$dname.pid"
  started=$(date +%s); up=0
  while kill -0 "$dpid" 2>/dev/null; do
    all=1; for prog in "$@"; do alive "$prog" || all=0; done
    [ "$all" = 1 ] && { up=1; break; }
    [ $(( $(date +%s) - started )) -ge "$DESK_TIMEOUT" ] && break
    sleep 1
  done
  if [ "$up" = 0 ] && kill -0 "$dpid" 2>/dev/null; then
    log "$dname: no desktop after ${DESK_TIMEOUT}s"
    kill -TERM "$dpid" 2>/dev/null; sleep 3; kill -KILL "$dpid" 2>/dev/null
  fi
  [ "$up" = 1 ] && { log "$dname: up"; desk_fails_reset "$dname"; rm -f "$DESK_STATE/desktop-notice"; desk_companion "$dname"; }
  wait "$dpid"; rc=$?
  rm -f "$RUN/hearth-$dname.pid"
  if [ "$up" = 0 ]; then log "$dname ended before its desktop came up (code $rc)"; return 200; fi
  if [ "$rc" -ne 0 ] && [ ! -s "$NEXT" ] && [ $(( $(date +%s) - started )) -lt 30 ]; then log "$dname crashed right after starting (code $rc)"; return 200; fi
  log "$dname ended (code $rc)"
  return "$rc"
}
# The controller (Hearth's pad service and on-screen keyboard) once the desktop is up: hearth-companion (it checks
# HEARTH_DESKTOP, runs one at a time and ends with the desktop; without it the desktop works, just without the pad).
desk_companion() {
  c="$SHARE/desktop/shell/hearth-companion"
  [ -f "$c" ] || { log "$1: no controller helper ($c)"; return 0; }
  ( HEARTH_DESKTOP="$1"; export HEARTH_DESKTOP; exec sh "$c" "$1" session ) </dev/null >/dev/null 2>&1 &
  return 0
}
# KDE's first start: Hearth's panel layout (KDE's own panel with Hearth's menu and the Hearth tray item) comes first
# in XDG_DATA_DIRS for KDE only (/usr/share/hearth-kde: plasma/layout-templates); a panel you already have stays.
run_kde() {
  [ -d /usr/share/hearth-kde ] && case ":$XDG_DATA_DIRS:" in *":/usr/share/hearth-kde:"*) ;; *) XDG_DATA_DIRS="/usr/share/hearth-kde:$XDG_DATA_DIRS"; export XDG_DATA_DIRS ;; esac
  # 1.14: Mantel's look in Plasma (hearth-desktop-look; off with Settings › Your desktop's switch). Time-limited and
  # never in the way: Plasma starts whatever happens here.
  dl="$SHARE/desktop/shell/hearth-desktop-look"
  [ -f "$dl" ] && { timeout 40 python3 "$dl" kde-pre >/dev/null 2>&1 || log "desktop look: KDE before-start step didn't finish"; ( sleep 3; timeout 90 python3 "$dl" kde-post >/dev/null 2>&1 ) & }
  log "desktop mode: KDE Plasma"; desk_run kde "$(dirname "$LOG")/kde.log" startplasma-wayland kwin_wayland plasmashell; r=$?; XDG_DATA_DIRS=$(printf '%s' "$XDG_DATA_DIRS" | sed 's#^/usr/share/hearth-kde:##'); export XDG_DATA_DIRS; return "$r"; }
run_gnome() {
  log "desktop mode: GNOME"
  cfg="${XDG_CONFIG_HOME:-$HOME/.config}"
  # GNOME's first-sign-in assistant isn't Hearth's (Hearth's own first-time setup already ran).
  [ -e "$cfg/gnome-initial-setup-done" ] || { mkdir -p "$cfg" && echo yes >"$cfg/gnome-initial-setup-done"; }
  # 1.14: Mantel's look in GNOME (hearth-desktop-look), time-limited, never in the way.
  dl="$SHARE/desktop/shell/hearth-desktop-look"
  [ -f "$dl" ] && { timeout 40 python3 "$dl" gnome-pre >/dev/null 2>&1 || log "desktop look: GNOME before-start step didn't finish"; }
  desk_run gnome "$(dirname "$LOG")/gnome.log" gnome-session gnome-shell
}
# What a desktop leaves behind, so the next mode starts clean, and the session's environment back to Hearth's.
settle_kde() {
  have systemctl && timeout 6 systemctl --user stop plasma-workspace.target >/dev/null 2>&1
  for p in plasmashell kwin_wayland kwin_wayland_wrapper startplasma-wayland; do pkill -KILL -u "$(id -u)" -x "$p" 2>/dev/null; done
  have systemctl && timeout 4 systemctl --user unset-environment KDE_FULL_SESSION KDE_SESSION_VERSION KDE_APPLICATIONS_AS_SCOPE HEARTH_DESKTOP DESKTOP_SESSION >/dev/null 2>&1
  desk_env_back; rm -f "$RUN/hearth-kde.pid"; return 0
}
settle_gnome() {
  have systemctl && timeout 6 systemctl --user stop gnome-session-manager.target gnome-session.target org.gnome.Shell@wayland.service >/dev/null 2>&1
  for p in gnome-shell gnome-session-binary gnome-session-service gnome-session gnome-session-ctl; do pkill -KILL -u "$(id -u)" -x "$p" 2>/dev/null; done
  have systemctl && timeout 4 systemctl --user unset-environment GNOME_SETUP_DISPLAY HEARTH_DESKTOP DESKTOP_SESSION >/dev/null 2>&1
  desk_env_back; rm -f "$RUN/hearth-gnome.pid"; return 0
}
desk_env_back() {
  have systemctl && timeout 4 systemctl --user import-environment XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP XDG_CONFIG_DIRS XDG_DATA_DIRS >/dev/null 2>&1
  have dbus-update-activation-environment && timeout 4 dbus-update-activation-environment XDG_CURRENT_DESKTOP XDG_SESSION_DESKTOP XDG_DATA_DIRS >/dev/null 2>&1
  return 0
}

# The controller helper for KDE Plasma and GNOME (desktop/shell/hearth-companion, started by desk_companion): gone
# between modes.
companion_stop() {
  pkill -TERM -u "$(id -u)" -f 'desktop/shell/hearth-companion' 2>/dev/null
  return 0
}
