# Resource Monitor (hearth-resmon-app, the Fedora edition's; docs/mockups/checklists/resource-monitor.md): every tab of
# the approved mockups resmon-1 … resmon-6 with the mockups' numbers (resmon-fixture.py stands in for hearth-resmon:
# HEARTH_RESMON_HELPER, never set on a PC), Your PC from the test machines in hosts/linux/test/fixtures/resmon, the
# right-click menu and End task's question. Then, with the real helper on this machine: its first frame, its CPU with
# the window open and nobody touching it (live, like Task Manager: limit 3 % for the app), and minimized (moved to
# sway's scratchpad: the compositor stops asking it to draw): the app and its sampler must stand still (limit 0.3 %),
# then start again when it's shown.
RFX="$ROOT/.github/scripts/hearth-apps-rust-shots.d/resmon-fixture.py"
RPC="$ROOT/hosts/linux/test/fixtures/resmon"
export HEARTH_RESMON_HELPER="$RFX" HEARTH_RESMON_PC_FIXTURE="$RPC/atx-desktop.json"
AI="Hearth AI Command Center"
run resmon-1-overview hearth-resmon 0 "$BIN/hearth-resmon-app" --page overview --tick "$AI"
run resmon-2-processor hearth-resmon 0 "$BIN/hearth-resmon-app" --page cpu --tick "$AI"
run resmon-3-memory hearth-resmon 0 "$BIN/hearth-resmon-app" --page memory --tick "$AI"
RESMON_DISK_NAMES=1 run resmon-4-disk hearth-resmon 0 "$BIN/hearth-resmon-app" --page disk
run resmon-5-network hearth-resmon 0 "$BIN/hearth-resmon-app" --page network
RESMON_GPU_NAMES=1 run resmon-6-graphics hearth-resmon 0 "$BIN/hearth-resmon-app" --page gpu
run resmon-7-your-pc hearth-resmon 0 "$BIN/hearth-resmon-app" --page pc
run resmon-8-menu hearth-resmon 0 "$BIN/hearth-resmon-app" --page cpu --show menu
run resmon-9-end-asks hearth-resmon 0 "$BIN/hearth-resmon-app" --page memory --show end
run resmon-10-overview-unfolded hearth-resmon 0 "$BIN/hearth-resmon-app" --page overview --unfold ov:disk,ov:network,ov:gpu
run resmon-11-libraries hearth-resmon 0 "$BIN/hearth-resmon-app" --page cpu --tick "$AI" --unfold cpu:libs
HEARTH_RESMON_PC_FIXTURE="$RPC/laptop.json" run resmon-12-your-pc-laptop hearth-resmon 0 "$BIN/hearth-resmon-app" --page pc
HEARTH_RESMON_PC_FIXTURE="$RPC/rog-ally.json" run resmon-13-your-pc-ally hearth-resmon 0 "$BIN/hearth-resmon-app" --page pc
HEARTH_RESMON_PC_FIXTURE="$RPC/fm2-desktop.json" run resmon-14-your-pc-fm2 hearth-resmon 0 "$BIN/hearth-resmon-app" --page pc --part cpu
unset HEARTH_RESMON_HELPER HEARTH_RESMON_PC_FIXTURE

# The real helper on this machine: first frame, at rest open, minimized, shown again.
resmon_rest() {
  local t0 pid ff i kid a b ka kb hz open_app open_hlp hid_app hid_hlp back
  hz=$(getconf CLK_TCK)
  t0=$(now_ms)
  "$BIN/hearth-resmon-app" --page overview >/tmp/resmon-rest.log 2>&1 &
  pid=$!
  ff=-1
  for i in $(seq 1 600); do
    if mapped hearth-resmon; then ff=$(( $(now_ms) - t0 )); break; fi
    kill -0 $pid 2>/dev/null || break
    sleep 0.005
  done
  sleep 6
  kid=$(pgrep -P $pid -f 'hearth-resmon serve' | head -1)
  [ -n "$kid" ] || echo "::warning title=resmon rest::the sampler (hearth-resmon serve) isn't running under the app: $(tail -c 600 /tmp/resmon-rest.log | tr '\n' '|')"
  a=$(ticks $pid); ka=$(ticks "${kid:-0}"); sleep 20; b=$(ticks $pid); kb=$(ticks "${kid:-0}")
  open_app=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $hz / 20))"); open_hlp=$(python3 -c "print('%.2f' % (100.0 * ($kb - $ka) / $hz / 20))")
  rss=$(awk '/^VmRSS/ {printf "%.1f", $2/1024}' /proc/$pid/status 2>/dev/null)
  grim "$ROOT/shots/resmon-15-real-open.png" 2>/dev/null || true
  swaymsg "[app_id=hearth-resmon] move scratchpad" >/dev/null 2>&1 || echo "::warning title=resmon rest::sway couldn't hide the window"
  sleep 6
  a=$(ticks $pid); ka=$(ticks "${kid:-0}"); sleep 20; b=$(ticks $pid); kb=$(ticks "${kid:-0}")
  hid_app=$(python3 -c "print('%.2f' % (100.0 * ($b - $a) / $hz / 20))"); hid_hlp=$(python3 -c "print('%.2f' % (100.0 * ($kb - $ka) / $hz / 20))")
  swaymsg "[app_id=hearth-resmon] scratchpad show" >/dev/null 2>&1 || true
  ka=$(ticks "${kid:-0}"); sleep 4; kb=$(ticks "${kid:-0}")
  back=$((kb - ka))
  grim "$ROOT/shots/resmon-16-real-shown-again.png" 2>/dev/null || true
  echo "first frame ${ff} ms · open and untouched: app ${open_app}%, sampler ${open_hlp}% of a core · minimized: app ${hid_app}%, sampler ${hid_hlp}% · shown again: sampler ${back} ticks in 4 s · ${rss} MB" >> "$ROOT/shots/resmon-results.txt"
  echo "::notice title=resmon-rest::first frame ${ff} ms · open and untouched: app ${open_app}% of a core, its sampler ${open_hlp}% · minimized: app ${hid_app}%, sampler ${hid_hlp}% · shown again: the sampler ran ${back} ticks in 4 s · memory ${rss} MB"
  python3 - "$ff" "$open_app" "$open_hlp" "$hid_app" "$hid_hlp" "$back" "$rss" <<'PY'
import sys
ff, oa, oh, ha, hh, back, rss = int(sys.argv[1]), float(sys.argv[2]), float(sys.argv[3]), float(sys.argv[4]), float(sys.argv[5]), int(sys.argv[6]), float(sys.argv[7] or 0)
if ff < 0: print('::error title=resmon-rest::its window never came up')
elif ff > 450: print('::error title=resmon-rest::first frame took %d ms (limit about 300)' % ff)
elif ff > 300: print('::warning title=resmon-rest::first frame took %d ms (limit about 300)' % ff)
if oa > 3.0: print('::error title=resmon-rest::%.2f%% of a core with the window open and untouched (limit 3%%, live like Task Manager)' % oa)
if oh > 8.0: print('::warning title=resmon-rest::its sampler took %.2f%% of a core while open (Mantel\'s, once a second)' % oh)
if ha + hh > 0.3: print('::error title=resmon-rest::minimized it still took %.2f%% (app) + %.2f%% (sampler) of a core (limit 0.3%%: nothing at rest)' % (ha, hh))
if back <= 0: print('::error title=resmon-rest::shown again, its sampler didn\'t start again')
if rss > 80: print('::error title=resmon-rest::%.1f MB resident (limit about 60)' % rss)
elif rss > 60: print('::warning title=resmon-rest::%.1f MB resident (limit about 60)' % rss)
PY
  # The harness's own limits for the minimized case (first frame, memory, and 0.3 % of a core for app + sampler)
  echo "RESULT resmon-rest-minimized firstframe=$ff cpu=$(python3 -c "print('%.2f' % ($hid_app + $hid_hlp))") rss=${rss:-0} pss=-1 threads=$(ls /proc/$pid/task 2>/dev/null | wc -l)" >> "$ROOT/results.txt"
  kill -0 $pid 2>/dev/null || echo "::error title=resmon-rest::it ended by itself: $(tail -c 1500 /tmp/resmon-rest.log | tr '\n' '|')"
  kill $pid 2>/dev/null; wait $pid 2>/dev/null
  sleep 0.5
  pgrep -f 'hearth-resmon serve' >/dev/null && echo "::error title=resmon-rest::its sampler kept running after the window closed"
}
resmon_rest
# Opening it wrote no setting (red flag 9): nothing of its own in the config folder.
ls "$HOME/.config/nimbus" 2>/dev/null | grep -i resmon && echo "::error title=resmon::opening Resource Monitor wrote a setting"
