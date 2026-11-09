#!/bin/bash
# Windows apps end to end (winapps.yml), in a Debian 13 container set up like Hearth OS's image for this: the same
# pinned GE-Proton (os-bases/peppermint/build.sh), Mesa's Vulkan drivers (64- and 32-bit), Steam's package (its
# 32-bit libraries), no Steam folder in the home and no internet needed. Runs as a normal user through hearth-winapps
# itself: a 64-bit and a 32-bit console program, a .bat, an Inno-Setup-like installer whose name doesn't say "setup"
# (its Start menu shortcut must become the app), and a Direct3D 11 device (DXVK on Mesa's software Vulkan).
# Every check posts a GitHub annotation with what happened and the end of the app's log.
#   e2e.sh setup          (root: packages, GE-Proton, the test programs, hearth-proton-run as in the image)
#   e2e.sh run            (the test user)
#   e2e.sh offline        (the test user, networking off: every way Hearth starts Proton, docs/OFFLINE-PROTON.md)
#   e2e.sh offline-image  (root, inside Hearth OS's own image with --network=none: makes the test user, then offline)
set -u
REPO="${REPO:-$(cd "$(dirname "$0")/../.." && pwd)}"
W="${W:-/opt/winapps-test}"
SHELLDIR=/opt/hearth-shell
note() { echo "::notice title=$1::$(printf '%s' "$2" | tr '\n' '|' | cut -c1-3800)"; }
err() { echo "::error title=$1::$(printf '%s' "$2" | tr '\n' '|' | cut -c1-3800)"; FAILED=1; }

setup() {
  set -e
  export DEBIAN_FRONTEND=noninteractive
  dpkg --add-architecture i386
  sed -i 's/^Components: main$/Components: main contrib non-free non-free-firmware/' /etc/apt/sources.list.d/debian.sources
  apt-get update -qq
  echo 'steam steam/question select I AGREE' | debconf-set-selections
  echo 'steam steam/license note ' | debconf-set-selections
  echo 'steam-installer steam/question select I AGREE' | debconf-set-selections
  echo 'steam-installer steam/license note ' | debconf-set-selections
  apt-get install -y -qq --no-install-recommends ca-certificates curl python3 xvfb xauth procps file \
    mesa-vulkan-drivers mesa-vulkan-drivers:i386 libgl1-mesa-dri libgl1-mesa-dri:i386 libglx-mesa0 libglx-mesa0:i386 \
    libegl-mesa0 libgbm1 libvulkan1 libvulkan1:i386 vulkan-tools mesa-utils icoutils libfreetype6 libfreetype6:i386 libfontconfig1:i386 \
    gcc-mingw-w64-x86-64 gcc-mingw-w64-i686 >/dev/null
  # Steam's package as in the image (it brings the 32-bit libraries); Steam itself is never started or signed in to.
  apt-get install -y -qq --no-install-recommends steam-installer steam-devices >/dev/null \
    || echo "::warning title=Steam's package::steam-installer didn't install here; the test goes on without its 32-bit libraries"
  GE=$(sed -n 's/^GE_PROTON=//p' "$REPO/os-bases/peppermint/build.sh")
  SUM=$(sed -n 's/^GE_PROTON_SHA256=//p' "$REPO/os-bases/peppermint/build.sh")
  curl -fsSL --retry 5 --retry-delay 10 -o /tmp/ge.tar.gz "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/$GE/$GE.tar.gz"
  echo "$SUM  /tmp/ge.tar.gz" | sha256sum -c -
  mkdir -p /usr/share/hearth/proton /usr/share/steam/compatibilitytools.d
  tar -xzf /tmp/ge.tar.gz -C /usr/share/hearth/proton && rm /tmp/ge.tar.gz
  ln -sfn "/usr/share/hearth/proton/$GE" "/usr/share/steam/compatibilitytools.d/$GE"
  install -Dm644 "$REPO/os/system_files/usr/share/hearth/proton-rules.json" /usr/share/hearth/proton-rules.json
  mkdir -p "$SHELLDIR"
  cp "$REPO/hosts/linux/desktop/shell/hearth-winapps" "$REPO/hosts/linux/desktop/shell/hearth_proton.py" \
     "$REPO/hosts/linux/desktop/shell/hearth_proton_run.py" "$REPO/hosts/linux/desktop/shell/hearth_settings_io.py" "$SHELLDIR/"
  # hearth-proton-run where the image has it (/usr/bin, its Python in /usr/share/nimbus/desktop/shell).
  mkdir -p /usr/share/nimbus/desktop
  ln -sfn "$SHELLDIR" /usr/share/nimbus/desktop/shell
  install -Dm755 "$REPO/os/system_files/usr/bin/hearth-proton-run" /usr/bin/hearth-proton-run
  note "GE-Proton in the test" "$GE: $(ls "/usr/share/hearth/proton/$GE" | tr '\n' ' ') umu.exe in its prefix: $(find "/usr/share/hearth/proton/$GE/files/share/default_pfx" -iname umu.exe 2>/dev/null | head -1)"
  mkdir -p "$W"
  S="$REPO/tools/winapps-e2e"
  x86_64-w64-mingw32-gcc -O2 -o "$W/hello64.exe" "$S/hello.c"
  i686-w64-mingw32-gcc -O2 -o "$W/hello32.exe" "$S/hello.c"
  x86_64-w64-mingw32-gcc -O2 -o "$W/dx11.exe" "$S/dx11.c" -ld3d11
  x86_64-w64-mingw32-gcc -O2 -municode -o "$W/hearth_test_4.0.exe" "$S/installer.c" -lole32 -luuid -lshell32
  printf '@echo off\r\necho bat ok %%*\r\necho bat ran > bat-ran.txt\r\n' > "$W/test.bat"
  file "$W"/*.exe
  id tester >/dev/null 2>&1 || useradd -m tester
  chown -R tester: "$W"
  note "FreeType in the test container (Wine's text)" "$(ls /usr/lib/x86_64-linux-gnu/libfreetype.so.6 /usr/lib/i386-linux-gnu/libfreetype.so.6 2>&1 | tr '\n' ' ')"
  note "Vulkan in the test container" "$(vulkaninfo --summary 2>&1 | grep -E 'deviceName|driverName|apiVersion' | head -6)"
}

wa() { timeout 900 xvfb-run -a -s '-screen 0 1280x720x24' python3 "$SHELLDIR/hearth-winapps" "$@"; }
tailof() { tail -n "${2:-40}" "$1" 2>/dev/null; }
appdir() { echo "$HOME/Games/Windows apps/$1"; }
jget() { python3 -c 'import json,sys; d=json.loads(sys.stdin.read().strip().splitlines()[-1]); print(d.get(sys.argv[1], ""))' "$1"; }

run() {
  FAILED=0
  cd "$HOME"
  [ ! -e "$HOME/.steam" ] && [ ! -e "$HOME/.local/share/Steam" ] || err "setup" "the test home has a Steam folder"

  # 1. A 64-bit console program, opened directly (the first start also makes the prefix).
  out=$(wa open "$W/hello64.exe" 2>&1); echo "$out"
  log="$(appdir hello64)/logs/last.log"
  if [ -f "$W/ran.txt" ] && grep -q 'bits=64' "$W/ran.txt"; then
    note "64-bit program ran" "$(cat "$W/ran.txt"); result: $(echo "$out" | tail -1)"
  else err "64-bit program didn't run" "result: $(echo "$out" | tail -1); log: $(tailof "$log" 60)"; fi
  grep -q 'hearth-winapps-test ok bits=64' "$log" && note "its output is in its log" "$(grep -m1 'hearth-winapps-test ok' "$log")" \
    || echo "::warning title=program output not in the log::$(tailof "$log" 30 | tr '\n' '|' | cut -c1-3000)"
  grep -q 'started with GE-Proton' "$HOME/.local/share/nimbus/bridge.log" && note "Hearth's log line" "$(grep winapps "$HOME/.local/share/nimbus/bridge.log" | tail -1)" \
    || err "no line in Hearth's log" "$(tail -3 "$HOME/.local/share/nimbus/bridge.log" 2>&1)"
  rm -f "$W/ran.txt"

  # 2. A 32-bit one (most old Windows programs; Wine's 32-bit side and the 32-bit libraries).
  out=$(wa open "$W/hello32.exe" 2>&1); echo "$out"
  if [ -f "$W/ran.txt" ] && grep -q 'bits=32' "$W/ran.txt"; then note "32-bit program ran" "$(cat "$W/ran.txt")"
  else err "32-bit program didn't run" "result: $(echo "$out" | tail -1); log: $(tailof "$(appdir hello32)/logs/last.log" 60)"; fi

  # 3. A .bat file (through cmd /c), in its own folder.
  out=$(wa open "$W/test.bat" 2>&1); echo "$out"
  if [ -f "$W/bat-ran.txt" ]; then note ".bat ran" "$(tailof "$(appdir test)/logs/last.log" 6)"
  else err ".bat didn't run" "result: $(echo "$out" | tail -1); log: $(tailof "$(appdir test)/logs/last.log" 60)"; fi

  # 4. An Inno-Setup-like installer: its Start menu shortcut (arguments, folder) becomes the app.
  out=$(wa open "$W/hearth_test_4.0.exe" 2>&1); echo "$out"
  id=$(echo "$out" | jget id)
  inst=$(echo "$out" | jget installed)
  case "$inst" in
    */Hearth\ Test/bin/HearthTest.exe) note "installer's program found" "$inst ($(grep 'installer installed' "$HOME/.local/share/nimbus/bridge.log" | tail -1))" ;;
    *) err "installer's program not found" "installed: '$inst'; result: $(echo "$out" | tail -1); log: $(tailof "$(appdir "hearth test")/logs/last.log" 40); shortcuts: $(find "$HOME/Games/Windows apps" -iname '*.lnk' | head)" ;;
  esac
  if [ -n "$id" ]; then
    out=$(wa run "$id" 2>&1); echo "$out"
    ran=$(find "$HOME/Games/Windows apps" -path '*Hearth Test/ran.txt' | head -1)
    if [ -n "$ran" ] && grep -q '\[--from-shortcut\]\[two words\]' "$ran"; then note "installed app started from its shortcut" "$(cat "$ran") in ${ran%/ran.txt}"
    else err "installed app didn't start as its shortcut says" "ran.txt: '${ran}' $(cat "$ran" 2>/dev/null); result: $(echo "$out" | tail -1); log: $(tailof "$(appdir "hearth test")/logs/last.log" 40)"; fi
  fi

  # 5. Direct3D 11 through DXVK (Mesa's software Vulkan here; Intel's or AMD's on a device). Reported, not failed:
  # the runners have no graphics card.
  out=$(wa open "$W/dx11.exe" 2>&1); echo "$out"
  note "Direct3D 11 (DXVK)" "$(cat "$W/dx11.txt" 2>/dev/null || echo 'no dx11.txt'); $(grep -iE 'dxvk|vulkan|err' "$(appdir dx11)/logs/last.log" | head -12)"

  # 6. The same program the 1.7.0 way, for the record: Proton's steam.exe with no Steam client (no UMU_ID).
  GE=$(ls /usr/share/steam/compatibilitytools.d | grep GE-Proton | tail -1)
  mkdir -p /tmp/oldway/pfx-holder && ln -sfn . /tmp/oldway/pfx-holder/pfx 2>/dev/null
  rm -f "$W/ran.txt"
  (cd "$W" && STEAM_COMPAT_DATA_PATH=/tmp/oldway/pfx-holder WINEPREFIX=/tmp/oldway/pfx-holder STEAM_COMPAT_CLIENT_INSTALL_PATH=/tmp/oldway \
     SteamAppId=0 SteamGameId=oldway timeout 600 xvfb-run -a python3 "/usr/share/steam/compatibilitytools.d/$GE/proton" waitforexitandrun "$W/hello64.exe" > /tmp/oldway.log 2>&1; echo "code $?" >> /tmp/oldway.log)
  note "1.7.0's way (steam.exe, no Steam client)" "ran: $([ -f "$W/ran.txt" ] && echo yes || echo no); $(tail -15 /tmp/oldway.log)"

  # 7. Send logs' section.
  wa report > /tmp/report.txt 2>&1
  grep -q 'Built-in GE-Proton: GE-Proton' /tmp/report.txt && note "Send logs: Windows apps" "$(head -40 /tmp/report.txt)" \
    || err "Send logs' Windows apps section" "$(head -40 /tmp/report.txt)"
  return $FAILED
}


# ---- Proton offline and without Steam (docs/OFFLINE-PROTON.md; owner's red flag). Run with networking off
# (podman run --network=none): every way Hearth starts a Windows program with Proton must work from what's on the disk.
#   WA   hearth-winapps (the direct path; default: the image's /usr/bin/hearth-winapps, else this test's copy)
#   W    the folder with hello64.exe and hello32.exe
offline() {
  FAILED=0
  cd "$HOME"
  local RUNNER=/usr/bin/hearth-proton-run WA="${WA:-}" GE X=()
  [ -n "$WA" ] || { [ -x /usr/bin/hearth-winapps ] && WA=/usr/bin/hearth-winapps || WA="python3 $SHELLDIR/hearth-winapps"; }
  if [ -n "${DISPLAY:-}" ]; then X=(); elif command -v xvfb-run >/dev/null; then X=(xvfb-run -a -s '-screen 0 1280x720x24'); fi
  [ ! -e "$HOME/.steam" ] && [ ! -e "$HOME/.local/share/Steam" ] || err "offline: setup" "the test home has a Steam folder"
  # 0. The network really is off (else this proves nothing), and umu-run is never what runs: a fake one first on the PATH
  # leaves a mark if anything calls it.
  if python3 -c 'import socket; socket.create_connection(("1.1.1.1", 443), 5)' 2>/dev/null; then
    err "offline: the network isn't off" "this check must run with podman run --network=none"; return 1
  fi
  note "offline: networking is off" "$(python3 -c 'import socket
try: socket.create_connection(("1.1.1.1", 443), 5)
except OSError as e: print(e)' 2>&1)"
  mkdir -p "$HOME/fakebin"
  printf '#!/bin/sh\necho "$0 $*" >> "$HOME/umu-was-called"\nexit 99\n' > "$HOME/fakebin/umu-run"
  chmod +x "$HOME/fakebin/umu-run"
  export PATH="$HOME/fakebin:$PATH"

  # 1. What it picks with nothing set: the built-in GE-Proton.
  GE=$($RUNNER --print-proton | cut -f1)
  case "$GE" in GE-Proton*) note "offline: hearth-proton-run picks the built-in Proton" "$($RUNNER --print-proton)" ;;
    *) err "offline: no built-in Proton picked" "$($RUNNER --print-proton 2>&1)" ;; esac

  # One start through hearth-proton-run (umu-run's command line): $1 name, $2 exe, then VAR=value pairs.
  hpr() {
    local name=$1 exe=$2; shift 2
    local d="$HOME/offline/$name"
    rm -rf "$d"; mkdir -p "$d"; cp "$W/$exe" "$d/"
    ( cd "$d" && env "$@" timeout 900 "${X[@]}" "$RUNNER" "$d/$exe" "arg one" > "$d/out.log" 2>&1; echo "exit code $?" >> "$d/out.log" )
    if grep -q 'ran bits=' "$d/ran.txt" 2>/dev/null && grep -q '\[arg one\]' "$d/ran.txt"; then
      note "offline: $name ran" "$(cat "$d/ran.txt"); $(grep -E '^hearth-proton-run:' "$d/out.log" | head -3); $(tail -1 "$d/out.log")"
    else
      err "offline: $name didn't run" "$(tail -40 "$d/out.log")"
    fi
  }
  # 2. Hearth Games' stores (stores.rs proton_launch: GOG and Epic), its own prefix, nothing picked.
  hpr "a GOG game (Hearth Games, nothing picked)" hello64.exe WINEPREFIX="$HOME/prefixes/gog-1" GAMEID=0 STORE=gog PROTON_VERB=waitforexitandrun
  # 3. A game you added yourself (manual.rs: GAMEID umu-default, umu's default prefix), 32-bit.
  hpr "your own 32-bit game" hello32.exe GAMEID=umu-default
  # 4. A game whose saved Proton isn't on this device: the built-in one for this start, and it says so.
  d="$HOME/offline/a saved Proton that isn't here"
  hpr "a saved Proton that isn't here" hello64.exe WINEPREFIX="$HOME/prefixes/epic-1" GAMEID=0 STORE=egs PROTONPATH=GE-Proton8-1
  grep -q "GE-Proton8-1 isn't on this device" "$d/out.log" && note "offline: it says the saved Proton isn't here" "$(grep -m1 "isn't on this device" "$d/out.log")" \
    || err "offline: no word about the missing saved Proton" "$(head -5 "$d/out.log")"
  # 5. The newest-GE pick and a Steam-engine style start (GAMEID umu-<app>, STORE steam): still offline, no Steam.
  hpr "a GE-Proton (newest) pick, Steam-style game id" hello64.exe WINEPREFIX="$HOME/prefixes/steam-70" GAMEID=umu-70 STORE=steam PROTONPATH=GE-Proton
  # 6. The direct path: Windows apps (hearth-winapps runs the built-in Proton itself).
  rm -f "$W/ran.txt"
  out=$(timeout 900 "${X[@]}" $WA open "$W/hello64.exe" 2>&1); echo "$out" | tail -5
  if [ -f "$W/ran.txt" ]; then note "offline: a Windows app ran (direct path)" "$(cat "$W/ran.txt"); $(echo "$out" | tail -1)"
  else err "offline: a Windows app didn't run (direct path)" "$(echo "$out" | tail -5); log: $(tailof "$(appdir hello64)/logs/last.log" 40)"; fi
  # 7. Nothing downloaded, umu-run never called.
  if [ -e "$HOME/umu-was-called" ]; then err "offline: umu-run was called" "$(cat "$HOME/umu-was-called")"; fi
  for p in "$HOME/.local/share/umu" "$HOME/.local/share/nimbus/tools/umu"; do
    [ ! -e "$p" ] || err "offline: something was downloaded" "$p exists"
  done
  [ "$FAILED" = 0 ] && note "offline: every Proton path worked with the network off" "built-in $GE, no Steam folder, no umu-run, nothing downloaded"
  return $FAILED
}

# Inside Hearth OS's own image (base-peppermint.yml, podman run --network=none): a normal user, the test programs
# from /opt/offline-exe (built on the runner with MinGW), then offline().
offline_image() {
  id tester >/dev/null 2>&1 || useradd -m tester
  mkdir -p "$W" && cp /opt/offline-exe/*.exe "$W/" && chown -R tester: "$W"
  runuser -u tester -- env HOME=/home/tester DISPLAY="${DISPLAY:-}" W="$W" PATH=/usr/bin:/bin bash "$REPO/tools/winapps-e2e/e2e.sh" offline
}

case "${1:-}" in
  setup) setup ;;
  run) run ;;
  offline) offline ;;
  offline-image) offline_image ;;
  *) echo "e2e.sh setup|run|offline|offline-image" >&2; exit 2 ;;
esac
