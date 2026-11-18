#!/bin/bash
# Inside a bare Fedora container (no libyaml, no FUSE): AppImage Maker's command line converts the test packages and
# Fedora's own hello .rpm, and every AppImage it makes is run and must print what it should. Also downloads the
# AppImage runtime itself (no HEARTH_APPIMAGE_RUNTIME), and fetches libyaml from Fedora for the .deb.
#     e2e.sh BINARY PACKAGES_DIR OUT_DIR
set -uo pipefail
bin="$1"; pk="$2"; out="$3"
mkdir -p "$out"
fail=0
note() { echo "::notice title=$1::$2"; }
bad() { echo "::error title=$1::$2"; fail=1; }

rpm -q libyaml >/dev/null 2>&1 && bad "setup" "libyaml is installed in the container, so fetching from Fedora isn't tested"
(cd "$pk" && dnf -q download --arch x86_64 hello >/dev/null 2>&1) || bad "setup" "couldn't download Fedora's hello rpm"

export APPIMAGE_EXTRACT_AND_RUN=1 NO_CLEANUP=
for f in "$pk"/*.deb "$pk"/*.rpm "$pk"/*.tar.gz; do
  [ -f "$f" ] || continue
  n=$(basename "$f")
  if ! "$bin" convert --json -o "$out" "$f" > "$out/$n.json" 2> "$out/$n.log"; then
    bad "$n" "conversion failed: $(tail -c 1500 "$out/$n.log" | tr '\n' '|')"; continue
  fi
  app=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["output"])' "$out/$n.json" 2>/dev/null)
  warn=$(python3 -c 'import json,sys; print(" / ".join(json.load(open(sys.argv[1]))["warnings"]))' "$out/$n.json" 2>/dev/null)
  fed=$(python3 -c 'import json,sys; print(", ".join(json.load(open(sys.argv[1]))["libs_from_fedora"]))' "$out/$n.json" 2>/dev/null)
  [ -x "$app" ] || { bad "$n" "no AppImage made ($app)"; continue; }
  note "$n" "made $(basename "$app") ($(du -h "$app" | cut -f1)); from Fedora: ${fed:-none}; warnings: ${warn:-none}"
  got=$(timeout 120 "$app" a b 2>&1 | tail -3 | tr '\n' ' ')
  case "$n" in
    demo-app*) want="demo ok yaml=0.2.5 data=from-the-package args=2"
               case "$fed" in *libyaml*) ;; *) bad "$n" "libyaml wasn't fetched from Fedora ($fed)";; esac ;;
    demo-tool*) want="demo ok yaml=0.2.5 data=from-the-tarball args=2" ;;
    hello*) want="Hello, world!" ;;
  esac
  case "$got" in
    *"$want"*) note "$n runs" "$got" ;;
    *) bad "$n runs" "wanted '$want', got: $(echo "$got" | cut -c1-600) | log: $(tail -c 800 "$out/$n.log" | tr '\n' '|')" ;;
  esac
  # The AppImage's insides: its menu entry, icon and AppRun.
  d=$(mktemp -d); (cd "$d" && "$app" --appimage-extract >/dev/null 2>&1)
  ls "$d/squashfs-root"/*.desktop >/dev/null 2>&1 || bad "$n" "no .desktop at the top of the AppImage"
  [ -e "$d/squashfs-root/.DirIcon" ] || bad "$n" "no .DirIcon"
  [ -x "$d/squashfs-root/AppRun" ] || bad "$n" "no AppRun"
  case "$n" in demo-app*)
    grep -q '^Exec=demo-app-launch %U' "$d"/squashfs-root/*.desktop || bad "$n" "the menu entry's Exec isn't the launcher: $(grep Exec "$d"/squashfs-root/*.desktop | tr '\n' ' ')"
    grep -q '^Actions=' "$d"/squashfs-root/*.desktop && bad "$n" "Actions= kept without its groups"
    [ "$(readlink "$d/squashfs-root/usr/bin/demo-app")" = "../../opt/demo-app/demo" ] || bad "$n" "the /usr/bin link wasn't made relative: $(readlink "$d/squashfs-root/usr/bin/demo-app")"
    grep -q 'exec \$APPDIR/opt/demo-app/demo' "$d/squashfs-root/usr/bin/demo-app-launch" || bad "$n" "the launcher script still points at /opt: $(cat "$d/squashfs-root/usr/bin/demo-app-launch" | tr '\n' '|')"
    [ -f "$d/squashfs-root/usr/lib/libyaml-0.so.2" ] || bad "$n" "libyaml isn't inside"
    [ -f "$d/squashfs-root/demo-app.png" ] || bad "$n" "the package's icon isn't at the top"
    ;;
  esac
  rm -rf "$d"
done
# Menu entry (--menu) on one of them.
"$bin" convert --menu -o "$out/menu" "$pk"/demo-app_1.0_amd64.deb > /dev/null 2>"$out/menu.log" || bad "menu" "$(tail -c 800 "$out/menu.log" | tr '\n' '|')"
e="$HOME/.local/share/applications/hearth-appimage-demo-app.desktop"
if [ -f "$e" ]; then note "menu entry" "$(grep -E '^(Exec|TryExec|Icon)=' "$e" | tr '\n' ' ')"; else bad "menu" "no menu entry at $e"; fi
grep -q "^TryExec=$out/menu/" "$e" 2>/dev/null || bad "menu" "TryExec isn't the AppImage"
# A file that isn't a package fails cleanly.
echo "not a package" > "$out/junk.deb"
"$bin" convert -o "$out" "$out/junk.deb" >/dev/null 2>&1 && bad "junk" "a file that isn't a package was 'converted'"
ls -l "$out"
exit $fail
