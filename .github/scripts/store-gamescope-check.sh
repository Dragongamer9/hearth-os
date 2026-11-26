#!/bin/bash
# The Hearth Store's gamescope for real (store-start-parts.yml; docs/STORE-SYSTEM-PARTS.md "Starting up"), in Debian
# trixie with the image's own gamescope (trixie-backports) on a virtual screen (--backend headless, software Vulkan):
#   1 the part built as quick-update.yml builds it (gamescope-part-build.sh: Debian's source, ldd checked);
#   2 no Store copy: hearth-part's trial starts the built-in gamescope, which runs the app;
#   3 the CI-built copy as the Store installs it: used, comes up, marked good;
#   4 a copy that hangs and one that ends at once (broken on purpose): stopped, the built-in gamescope runs the app,
#     each marked as not working; 5 the next start goes straight to the built-in one.
# Source /src (read only). Results as annotations.
set -uo pipefail
ann() { echo "::$1 title=gamescope part (real)::$(echo "$2" | tr '\n' '|' | cut -c1-3500)"; }
FAILED=0
fail() { FAILED=1; ann error "$1"; }
bash /src/.github/scripts/gamescope-part-build.sh || exit 1
apt-get install -y -qq --no-install-recommends -t trixie-backports gamescope >/dev/null 2>&1 || apt-get install -y -qq gamescope >/dev/null
apt-get install -y -qq --no-install-recommends mesa-vulkan-drivers libvulkan1 xwayland python3 procps >/dev/null
B=$(command -v gamescope || ls /usr/games/gamescope)
useradd -m t 2>/dev/null
mkdir -p /tmp/r && chown t /tmp/r && chmod 700 /tmp/r
echo 1.19 > /tmp/version
cat > /usr/local/bin/gs-builtin <<EOF
#!/bin/sh
echo BUILTIN >> /tmp/calls.log
exec $B "\$@"
EOF
chmod 755 /usr/local/bin/gs-builtin
: > /tmp/calls.log; : > /tmp/inner.log; chmod 666 /tmp/calls.log /tmp/inner.log
P=/home/t/.local/share/hearth/parts/gamescope
S=/home/t/.local/state/hearth/parts/gamescope.json
start() {   # one Game Mode start through the trial; the app says it's up and stays a few seconds
  runuser -u t -- env HOME=/home/t XDG_RUNTIME_DIR=/tmp/r HEARTH_VERSION_FILE=/tmp/version PATH=/usr/local/bin:/usr/bin:/bin:/usr/games \
    timeout 120 python3 /src/os/system_files/usr/libexec/hearth/hearth-part trial gamescope /usr/local/bin/gs-builtin \
    --up children --up-secs 20 --good-secs 3 --path-first -- --backend headless -W 640 -H 400 -- \
    sh -c "echo INNER-UP-$1 >> /tmp/inner.log; sleep 6" > "/tmp/run-$1.log" 2>&1
  echo "run $1: code $? · calls $(grep -c BUILTIN /tmp/calls.log) · inner $(grep -c "INNER-UP-$1" /tmp/inner.log) · state $(tr -d '\n ' < $S 2>/dev/null)"
}
put() {    # put VERSION FILE: as the Store installs a copy
  rm -rf "$P"; mkdir -p "$P/$1/gamescope"
  cp -a "$2"/. "$P/$1/gamescope/"
  printf '{"id": "gamescope", "version": "%s", "os": "1.19", "path": "gamescope/gamescope"}\n' "$1" > "$P/part.json"
  # a new install starts with a clean slate, as hearth-part install does (this sign-in's failure mark goes)
  rm -f "$S" /tmp/r/hearth-part-gamescope.failed; chown -R t: /home/t/.local
}
r=$(start none); echo "$r"
grep -q 'INNER-UP-none' /tmp/inner.log && headless=1 || headless=0
[ "$headless" = 1 ] || ann warning "the built-in gamescope couldn't run on a virtual screen here, so only the choice of copy is checked: $(tail -5 /tmp/run-none.log)"
[ "$(grep -c BUILTIN /tmp/calls.log)" = 1 ] || fail "no Store copy: the built-in gamescope wasn't started ($r)"

put 1.19.1 /out/gamescope
r=$(start built); echo "$r"; ann notice "the CI-built copy: $r"
if [ "$headless" = 1 ]; then
  [ "$(grep -c BUILTIN /tmp/calls.log)" = 1 ] || fail "the CI-built copy wasn't used: $r $(tail -5 /tmp/run-built.log)"
  grep -q INNER-UP-built /tmp/inner.log || fail "the CI-built gamescope didn't run the app: $(tail -8 /tmp/run-built.log)"
  grep -q '"pending"' "$S" && fail "the CI-built copy wasn't marked good: $(cat $S)"
else
  # gamescope can't run here at all (no GPU), so the CI-built copy can only show it's tried, fails like the built-in
  # one does here, and hands over to it at once, marked as not working: the same path a copy broken on a device takes.
  grep -q 'gamescope version' /tmp/run-built.log || grep -q 'vulkan' /tmp/run-built.log || fail "the CI-built copy wasn't started: $(tail -5 /tmp/run-built.log)"
  [ "$(grep -c BUILTIN /tmp/calls.log)" = 2 ] || fail "after the CI-built copy failed here, the built-in one wasn't started: $r"
  grep -q '"fails": *[2-9]' "$S" || fail "the CI-built copy that failed here wasn't marked: $(cat $S 2>/dev/null)"
fi

mkdir -p /tmp/hang /tmp/quit
printf '#!/bin/sh\nexec sleep 300\n' > /tmp/hang/gamescope; printf '#!/bin/sh\necho broken >&2\nexit 1\n' > /tmp/quit/gamescope
chmod 755 /tmp/hang/gamescope /tmp/quit/gamescope
for k in hang quit; do
  put "1.19.9" "/tmp/$k"
  n0=$(grep -c BUILTIN /tmp/calls.log)
  r=$(start "$k"); echo "$r"; ann notice "broken on purpose ($k): $r"
  [ "$(grep -c BUILTIN /tmp/calls.log)" = $((n0 + 1)) ] || fail "$k: the built-in gamescope didn't take over ($r)"
  [ "$headless" = 1 ] && { grep -q "INNER-UP-$k" /tmp/inner.log || fail "$k: Game Mode's app didn't run after the fallback: $(tail -5 /tmp/run-$k.log)"; }
  grep -q '"fails": *[2-9]' "$S" || fail "$k: the broken copy wasn't marked as not working: $(cat $S 2>/dev/null)"
  pgrep -u t -f 'sleep 300' >/dev/null && fail "$k: the hung copy is still running"
  # 5. the next start: straight to the built-in one, the broken copy never started again
  t0=$(date +%s); r=$(start "$k-again"); echo "$r"
  [ "$(grep -c BUILTIN /tmp/calls.log)" = $((n0 + 2)) ] || fail "$k: the next start didn't use the built-in gamescope ($r)"
  [ $(( $(date +%s) - t0 )) -lt 18 ] || fail "$k: the next start waited on the broken copy again"
done
[ "$FAILED" = 0 ] && ann notice "passed: built-in without a copy, the CI-built copy used, broken copies replaced by the built-in gamescope at once and from then on"
exit $FAILED
