# Archived with KWin (1.15, the owner, Nov 17: "stop all progress on it and Archive it"): nimbus-session's KWin part
# as it was in 1.14 (run_kwin, its lock guard, its wait for Mantel, settle_kwin). Not run. docs/archive/KWIN.md

# ---- 1.14: Mantel runs on KWin (the owner, Nov 16: "Mantel moves fully from labwc to KWin"; docs/DESKTOPS.md "Mantel
# on KWin"). labwc stays in the image and is the safe start's fallback, for this sign-in, whenever KWin isn't there,
# ends within HEARTH_KWIN_EARLY_SECS (15 s) or Mantel doesn't say it's up within HEARTH_KWIN_UP_SECS (45 s; Mantel
# writes $XDG_RUNTIME_DIR/hearth-shell-up, shell.qml). Twice in a row → labwc until you turn KWin back on (Settings ›
# System › Your desktop, or the notice's button: hearth-your-desktop wm kwin). Each time Mantel says why once
# (desk_notice kwin missing|failed|twice, DesktopNotice.qml). session.conf's HEARTH_DESKTOP_WM=kwin|labwc picks (no
# line: kwin); only hearth-your-desktop writes it. Safe graphics, a second player's seat (multi-seat) and Server mode
# stay on labwc. What Mantel and everything it starts are told: HEARTH_WM=kwin|labwc, and HEARTH_DESKTOP_PID (the
# window manager's process id; labwc also sets LABWC_PID, which is never set under KWin: labwc --reconfigure would
# send its signal to KWin).
KWIN_BIN="${HEARTH_KWIN_BIN:-kwin_wayland}"   # (tests: one that isn't there)
KWIN_UP_SECS="${HEARTH_KWIN_UP_SECS:-45}"
KWIN_EARLY_SECS="${HEARTH_KWIN_EARLY_SECS:-15}"
KWIN_UP_MAX_SECS="${HEARTH_KWIN_UP_MAX_SECS:-120}"   # the longest wait for a Mantel that's still busy starting (run_kwin)
KWIN_STALL_SECS="${HEARTH_KWIN_STALL_SECS:-15}"      # ...ended sooner when it does nothing for this long
KWIN_BUSY_TICKS="${HEARTH_KWIN_BUSY_TICKS:-50}"      # "busy": at least this much processor time (1/100 s) in that time
# Mantel's processor time so far (all of this user's quickshell processes, in clock ticks), or nothing when none runs.
mantel_cpu_ticks() {
  mt=0; mfound=""
  for mp in $(pgrep -u "$(id -u)" -x quickshell 2>/dev/null); do
    ms=$(cat "/proc/$mp/stat" 2>/dev/null) || continue
    ms=${ms##*) }   # after "pid (name) ": state is the first field, utime the 12th, stime the 13th
    # shellcheck disable=SC2086  # split into fields on purpose
    set -- $ms
    [ $# -ge 13 ] || continue
    mt=$(( mt + ${12} + ${13} )); mfound=1
  done
  [ -n "$mfound" ] && echo "$mt"
  return 0
}

KWIN_GOOD_SECS="${HEARTH_KWIN_GOOD_SECS:-600}"   # a run this long before a crash starts the failure count over

# Mantel on KWin: KWin's environment is labwc's environment file (the same folder, with your accessibility lines),
# with XDG_CURRENT_DESKTOP=Hearth:Nimbus:KDE (Hearth first: its portals, hearth-portals.conf (feature/kwin-input);
# Nimbus next: Hearth's default apps (nimbus-mimeapps.list), its file chooser's checks and app kinds go by it; KDE last),
# PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock (KWin's screen locker shows Mantel's lock: feature/kwin-input) and
# Hearth's KWin defaults ($SHARE/desktop/kwin/xdg: kwinrc, kscreenlockerrc) first in XDG_CONFIG_DIRS: system defaults under yours, so your own
# ~/.config/kwinrc is read on top and never written over by Hearth. KWin starts the same autostart as labwc.
# 200 = didn't come up (or ended right after starting); anything else is KWin's own exit code.
run_kwin() {
  log "desktop mode: Mantel on KWin"
  KWIN_LOG="$(dirname "$LOG")/kwin.log"
  [ -f "$KWIN_LOG" ] && [ "$(wc -c <"$KWIN_LOG")" -gt 1000000 ] && mv -f "$KWIN_LOG" "$KWIN_LOG.old"
  echo "--- $(date '+%F %T') Desktop Mode (KWin)" >>"$KWIN_LOG"
  auto="$SHARE/desktop/kwin/autostart"; [ -f "$auto" ] || auto="$CONF/autostart"
  rm -f "$RUN/hearth-shell-up" "$RUN/hearth-kwin-shell-failed" "$RUN/hearth-kwin-inverted"
  # Your look first (theme/apply.sh: kdeglobals and Hearth's Aurorae frames for your accent), so KWin's first window
  # already has Hearth's frames. Never in the way: at most 20 s, and a failure is only logged.
  # HEARTH_THEME_APPLIED=1 tells the autostart it's done (it ran it a second time before Mantel: seconds on a slow laptop).
  theme_done=""
  if [ -f "$SHARE/desktop/theme/apply.sh" ]; then
    if { if have timeout; then timeout 20 sh "$SHARE/desktop/theme/apply.sh" >>"$KWIN_LOG" 2>&1; else sh "$SHARE/desktop/theme/apply.sh" >>"$KWIN_LOG" 2>&1; fi; }; then
      theme_done=1
    else log "kwin: theme/apply.sh didn't finish (code $?); KWin starts with the frames it has"; fi
  fi
  # Your own KDE settings that Hearth's fixed KWin keys override (kwin/xdg's [$i] keys: KWin's own locking, its input
  # method, the Super key): told once which and why (hearth-your-desktop kwin-overrides; it only reads your files).
  # Never in the way: at most 10 s, a failure is only logged.
  if [ -z "$HEARTH_SEAT" ] && [ -f "$SHARE/desktop/shell/hearth-your-desktop" ] && have python3; then
    if have timeout; then timeout 10 python3 "$SHARE/desktop/shell/hearth-your-desktop" kwin-overrides >>"$KWIN_LOG" 2>&1
    else python3 "$SHARE/desktop/shell/hearth-your-desktop" kwin-overrides >>"$KWIN_LOG" 2>&1; fi \
      || log "kwin: couldn't check which of your KDE settings are overridden (code $?); KWin starts anyway"
  fi
  # The lock guard (red flag 2: never a lockout): without Hearth's lock screen, KWin runs without its screen locker
  # (--no-lockscreen) and Lock.qml says why it won't lock. Images never get here (their build fails when
  # hearth-lockcheck --as-kwin does). The person is told once, through Hearth's own notice (DesktopNotice.qml, not a
  # notification Do not disturb could hide); told again if it breaks again after working.
  lock=--no-lockscreen
  if kwin_lock_ready; then lock=""; log "kwin: Hearth's lock screen is there; KWin's screen locker on"; rm -f "$DESK_STATE/kwin-lock-off-told"
  else
    why=$(kwin_lock_why)
    log "kwin: Hearth's lock screen isn't there ($why); KWin without its screen locker (--no-lockscreen)"
    if [ -z "$HEARTH_SEAT" ] && [ ! -e "$DESK_STATE/kwin-lock-off-told" ]; then
      mkdir -p "$DESK_STATE"; : >"$DESK_STATE/kwin-lock-off-told"; desk_notice kwin-lock "$why"
    fi
  fi
  (
    unset LABWC_PID WLR_NO_HARDWARE_CURSORS
    if [ -f "$CONF/environment" ]; then
      while IFS= read -r line || [ -n "$line" ]; do
        case "$line" in [A-Za-z_]*=*) ;; *) continue ;; esac
        case "${line%%=*}" in *[!A-Za-z0-9_]*) continue ;; esac   # only NAME=value lines (export with a bad name would end this shell)
        # shellcheck disable=SC2163  # exporting the NAME=value line itself is the point
        export "$line"
      done <"$CONF/environment"
    fi
    # Hearth's KWin defaults, before the system's folders (yours, ~/.config, come before all of them anyway): the
    # session's (desktop/kwin/xdg) and the windows part's (desktop/kwin/windows/xdg: desktops, Mantel's keys). KWin
    # reads every folder; the first one wins a key.
    kx=""
    for xdg in "$SHARE/desktop/kwin/xdg" "$SHARE/desktop/kwin/windows/xdg"; do
      [ -d "$xdg" ] && case ":${XDG_CONFIG_DIRS:-/etc/xdg}:" in *":$xdg:"*) ;; *) kx="${kx:+$kx:}$xdg" ;; esac
    done
    [ -n "$kx" ] && XDG_CONFIG_DIRS="$kx:${XDG_CONFIG_DIRS:-/etc/xdg}"
    export XDG_CONFIG_DIRS XDG_CURRENT_DESKTOP=Hearth:Nimbus:KDE XDG_SESSION_TYPE=wayland HEARTH_WM=kwin
    if [ -n "$theme_done" ]; then export HEARTH_THEME_APPLIED=1; else unset HEARTH_THEME_APPLIED; fi
    [ -z "$lock" ] && export PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock
    # $lock: --no-lockscreen unless Hearth's lock screen is there (kwin_lock_ready).
    # $HEARTH_KWIN_ARGS: extra options for tests (--virtual --width 1920 --height 1080 in kwin-test.yml).
    # Right before KWin starts, the environment it gets goes to $XDG_RUNTIME_DIR/hearth/kwin.env (env -0, mode 600 in a
    # 700 folder, written whole then moved in place): hearth-lockcheck reads it when KWin's own /proc environ can't be
    # read (KWin's file capability). A failed write never stops KWin: it's logged, and Lock.qml then won't lock.
    exec sh -c 'HEARTH_DESKTOP_PID=$$; export HEARTH_DESKTOP_PID
      d="${XDG_RUNTIME_DIR:-/tmp}/hearth"
      if ! ( umask 077; mkdir -p "$d" && chmod 700 "$d" && env -0 >"$d/kwin.env.tmp" && mv -f "$d/kwin.env.tmp" "$d/kwin.env" ) 2>/dev/null; then
        rm -f "$d/kwin.env.tmp" 2>/dev/null; echo "nimbus-session: couldn'"'"'t write $d/kwin.env (the lock check will say so)" >&2
      fi
      exec "$1" --xwayland $3 $HEARTH_KWIN_ARGS "$2"' sh "$KWIN_BIN" "$auto" "$lock"
  ) >>"$KWIN_LOG" 2>&1 &
  kpid=$!
  echo "$kpid" >"$RUN/hearth-kwin.pid"
  # Waiting for Mantel (the owner's i3-N305 laptop, Nov 17: on the first start after an update Mantel's process only
  # began 27 s after KWin, with a cold disk, and was still loading at 45 s, so KWin was ended though nothing was wrong):
  # KWIN_UP_SECS (45 s) as before, then longer only while Mantel is alive and busy (its quickshell's processor time
  # keeps growing: it's loading), at most KWIN_UP_MAX_SECS (120 s) in all. A Mantel that's gone, or idle for
  # KWIN_STALL_SECS (15 s: stuck, not loading), is ended as before and labwc opens (red flag 2: the fallback stays).
  started=$(date +%s); up=0; told_busy=""; busy_ref=""; busy_at=""
  while kill -0 "$kpid" 2>/dev/null; do
    [ -e "$RUN/hearth-shell-up" ] && { up=1; break; }
    [ -s "$NEXT" ] || [ -e "$RUN/hearth-kwin-shell-failed" ] && break
    now=$(date +%s)
    if [ $(( now - started )) -ge "$KWIN_UP_SECS" ]; then
      [ $(( now - started )) -ge "$KWIN_UP_MAX_SECS" ] && break
      cpu=$(mantel_cpu_ticks)
      [ -n "$busy_at" ] || { busy_at=$now; busy_ref=${cpu:-0}; }
      if [ -n "$cpu" ] && [ $(( cpu - busy_ref )) -ge "$KWIN_BUSY_TICKS" ]; then
        busy_ref=$cpu; busy_at=$now
        if [ -z "$told_busy" ]; then
          told_busy=1; log "kwin: Mantel is still starting after ${KWIN_UP_SECS}s and busy loading; waiting while it keeps going (at most ${KWIN_UP_MAX_SECS}s)"
        fi
      fi
      [ $(( now - busy_at )) -ge "$KWIN_STALL_SECS" ] && break
    fi
    sleep 1
  done
  if [ "$up" = 0 ] && kill -0 "$kpid" 2>/dev/null && [ ! -s "$NEXT" ]; then
    if [ -n "$told_busy" ]; then
      log "kwin: Mantel didn't say it's up after $(( $(date +%s) - started ))s (it stopped making progress, or ${KWIN_UP_MAX_SECS}s went by); ending KWin"
    else
      log "kwin: Mantel didn't say it's up after ${KWIN_UP_SECS}s; ending KWin"
    fi
    kill -TERM "$kpid" 2>/dev/null; sleep 3; kill -KILL "$kpid" 2>/dev/null
  fi
  [ "$up" = 1 ] && log "kwin: Mantel up after $(( $(date +%s) - started ))s"
  # While it runs: a switch asked for (the next mode written) gets 8 s for KWin to end by itself, then it's ended
  # (TERM, then KILL), so a KWin that hangs can never hold the switch up.
  asked=""
  while kill -0 "$kpid" 2>/dev/null; do
    if [ -s "$NEXT" ] || [ -e "$RUN/hearth-kwin-shell-failed" ]; then
      [ -n "$asked" ] || asked=$(date +%s)
      if [ $(( $(date +%s) - asked )) -ge 8 ]; then
        log "kwin: still running 8s after the switch was asked for; ending it"
        kill -TERM "$kpid" 2>/dev/null; sleep 3; kill -KILL "$kpid" 2>/dev/null; break
      fi
    fi
    sleep 1
  done
  wait "$kpid"; rc=$?
  rm -f "$RUN/hearth-kwin.pid"
  wm_env_back
  # Mantel kept closing under KWin (the autostart's give_up): the safe start, like KWin not coming up.
  if [ -e "$RUN/hearth-kwin-shell-failed" ]; then log "kwin: $(head -c 200 "$RUN/hearth-kwin-shell-failed")"; return 200; fi
  # A switch you asked for (nimbus-mode wrote the next mode) is never a failure, however quick.
  if [ -s "$NEXT" ]; then [ "$up" = 1 ] && desk_fails_reset kwin; log "kwin ended (code $rc)"; return "$rc"; fi
  if [ "$up" = 0 ]; then log "kwin ended before Mantel came up (code $rc)"; return 200; fi
  ran=$(( $(date +%s) - started ))
  if [ "$ran" -lt "$KWIN_EARLY_SECS" ]; then log "kwin ended right after starting (code $rc)"; return 200; fi
  # Ended by itself with no switch asked for. Code 0 is a clean end (the count starts over). Anything else (a crash, a
  # signal: 128+n) is a failure like not starting (red flag 2): returning KWin's code would end the session, and with
  # Start in: Desktop and the login screen's Relogin a KWin that keeps crashing 20-60 s in would loop for ever, with
  # no way out on a handheld. So: 200, labwc for the rest of this sign-in, counted and told (run_classic). A KWin that
  # ran HEARTH_KWIN_GOOD_SECS (10 minutes) or more before crashing starts the count over first: one crash after a long
  # run is counted once, never as "twice in a row" with an old one.
  if [ "$rc" -ne 0 ]; then
    [ "$ran" -ge "$KWIN_GOOD_SECS" ] && desk_fails_reset kwin
    log "kwin crashed after ${ran}s (code $rc); Mantel on labwc for the rest of this sign-in"
    return 200
  fi
  desk_fails_reset kwin
  log "kwin ended (code $rc)"
  return "$rc"
}
# KWin's own screen locker is how Mantel locks under KWin (feature/kwin-input), but only when Hearth's lock screen is
# really there and really loads: its Plasma shell package where kscreenlocker_greet looks, the greeter itself, and the
# greeter's own test run (hearth-lockcheck --as-kwin, with PLASMA_DEFAULT_SHELL as KWin gets it) showing "hearth-lock-
# screen: loaded". Otherwise KWin starts with --no-lockscreen, so it can never lock with KDE's (or a broken) lock
# screen: never a lockout. The test run is only done once per image (or a changed lock screen or greeter): a pass is
# kept in $DESK_STATE/kwin-lock-proven under a key of the image's version and both files' checksums; a failure is
# never kept, so the next start tries again. At most HEARTH_LOCK_CHECK_SECS (30 s); a check that can't run fails.
# (kscreenlockerrc in desktop/kwin/xdg turns KWin's own automatic locking off either way, fixed with [$i]: Mantel's
# settings decide when to lock.)
KWIN_LOCK_WHY=""
kwin_lock_pkg() { printf '%s' "${HEARTH_LOCK_PKG:-/usr/share/plasma/shells/org.hearth.mantel-lock}"; }
kwin_lock_greeter() {
  if [ -n "${HEARTH_LOCK_GREETER:-}" ]; then [ -x "$HEARTH_LOCK_GREETER" ] && printf '%s' "$HEARTH_LOCK_GREETER"; return; fi
  for g in /usr/lib/*/libexec/kscreenlocker_greet /usr/libexec/kscreenlocker_greet; do [ -x "$g" ] && { printf '%s' "$g"; return; }; done
}
kwin_lock_key() {
  kl_pkg=$(kwin_lock_pkg); kl_g=$(kwin_lock_greeter)
  printf 'v=%s qml=%s meta=%s greeter=%s:%s' "$(head -c 64 "$SHARE/version" 2>/dev/null | tr -d '\n ')" \
    "$(cksum <"$kl_pkg/contents/lockscreen/LockScreen.qml" 2>/dev/null | tr ' ' :)" "$(cksum <"$kl_pkg/metadata.json" 2>/dev/null | tr ' ' :)" \
    "$kl_g" "$(cksum <"$kl_g" 2>/dev/null | tr ' ' :)"
}
kwin_lock_ready() {
  KWIN_LOCK_WHY=""
  [ -f "$(kwin_lock_pkg)/contents/lockscreen/LockScreen.qml" ] || { KWIN_LOCK_WHY=nopackage; return 1; }
  [ -n "$(kwin_lock_greeter)" ] || { KWIN_LOCK_WHY=nolocker; return 1; }
  lc="$SHARE/desktop/shell/hearth-lockcheck"
  if [ ! -f "$lc" ] || ! have python3 || ! have timeout; then
    KWIN_LOCK_WHY=notloaded; log "kwin: the lock screen's check (hearth-lockcheck) can't run here"; return 1
  fi
  key=$(kwin_lock_key)
  proven="$DESK_STATE/kwin-lock-proven"
  [ "$(cat "$proven" 2>/dev/null)" = "$key" ] && return 0
  # Up to 25 s for the greeter's test run (it ends as soon as the lock screen says it loaded: usually a few seconds).
  # The owner's i3-N305 laptop, Nov 17: on the first start after an update (a cold disk) the greeter hadn't said
  # anything after 8 s, so KWin ran without its locker. Only the first start of an image pays this (a pass is kept).
  out=$(PLASMA_DEFAULT_SHELL=org.hearth.mantel-lock HEARTH_LOCK_PROBE_TIME="${HEARTH_LOCK_PROBE_TIME:-25}" \
    timeout "${HEARTH_LOCK_CHECK_SECS:-30}" python3 "$lc" --as-kwin 2>&1); rc=$?
  if [ "$rc" = 0 ]; then
    mkdir -p "$DESK_STATE" && printf '%s\n' "$key" >"$proven.tmp" && mv -f "$proven.tmp" "$proven"
    log "kwin: Hearth's lock screen loads in KWin's greeter (checked once for this image)"
    return 0
  fi
  rm -f "$proven" "$proven.tmp"
  KWIN_LOCK_WHY=notloaded
  log "kwin: Hearth's lock screen didn't load in KWin's greeter's test run (code $rc): $(printf '%s' "$out" | tr '\n' ' ' | head -c 400)"
  return 1
}
# Which part of Hearth's lock screen is missing or didn't load (for the notice and the log): nopackage, nolocker or
# notloaded (kwin_lock_ready's own answer).
kwin_lock_why() {
  [ -n "$KWIN_LOCK_WHY" ] && { echo "$KWIN_LOCK_WHY"; return; }
  [ -f "$(kwin_lock_pkg)/contents/lockscreen/LockScreen.qml" ] || { echo nopackage; return; }
  echo nolocker
}
# Leaving KWin: the activation environment (D-Bus and systemd's, which the autostart gave KWin's values) back to this
# session's own, so what starts in Game Mode next doesn't think it's in KWin.
wm_env_back() {
  rm -f "$RUN/hearth/kwin.env" "$RUN/hearth/kwin.env.tmp"   # KWin's environment (run_kwin): only while it runs
  if have systemctl; then
    timeout 4 systemctl --user unset-environment HEARTH_WM PLASMA_DEFAULT_SHELL >/dev/null 2>&1
    if [ -n "${XDG_CONFIG_DIRS:-}" ]; then timeout 4 systemctl --user import-environment XDG_CURRENT_DESKTOP XDG_CONFIG_DIRS >/dev/null 2>&1
    else timeout 4 systemctl --user import-environment XDG_CURRENT_DESKTOP >/dev/null 2>&1; timeout 4 systemctl --user unset-environment XDG_CONFIG_DIRS >/dev/null 2>&1; fi
  fi
  have dbus-update-activation-environment && timeout 4 dbus-update-activation-environment XDG_CURRENT_DESKTOP >/dev/null 2>&1
  return 0
}

# What a KWin that didn't come up leaves behind, before labwc starts.
settle_kwin() {
  pkill -TERM -u "$(id -u)" -x quickshell 2>/dev/null
  pkill -KILL -u "$(id -u)" -x kwin_wayland 2>/dev/null; pkill -KILL -u "$(id -u)" -f 'kwin_wayland_wrapper' 2>/dev/null
  rm -f "$RUN/hearth-kwin.pid" "$RUN/hearth-shell-up" "$RUN/hearth-kwin-shell-failed" "$RUN/hearth/kwin.env" "$RUN/hearth/kwin.env.tmp"
  sleep 1
  return 0
}
