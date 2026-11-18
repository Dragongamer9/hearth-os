#!/bin/bash
# Makes the packages AppImage Maker's CI converts (on Ubuntu, into the folder given):
#   demo-app_1.0_amd64.deb   an app laid out like Electron .debs: /opt/demo-app/demo (needs libyaml, which a bare
#                            Fedora doesn't have, so it's fetched from Fedora), a data file beside it, an absolute
#                            link /usr/bin/demo-app -> /opt/demo-app/demo, a launcher script with /opt paths, a menu
#                            entry and an icon.
#   demo-tool-2.1-linux-x64.tar.gz  a tarball app: one folder, the program and its own lib/ (RPATH $ORIGIN/lib).
#   hello_*.deb              GNU Hello from Ubuntu (no menu entry: the program is guessed).
# Fedora's hello .rpm is downloaded inside the Fedora container (e2e.sh).
set -euo pipefail
out="$1"; mkdir -p "$out"
src="$(mktemp -d)"
cat > "$src/demo.c" <<'C'
#include <stdio.h>
#include <string.h>
#include <unistd.h>
#include <libgen.h>
#include <yaml.h>
int main(int argc, char **argv) {
  char exe[4096] = {0}, path[4200];
  if (readlink("/proc/self/exe", exe, sizeof exe - 1) < 0) return 3;
  snprintf(path, sizeof path, "%s/resources/data.txt", dirname(exe));
  FILE *f = fopen(path, "r"); char data[64] = "no-data";
  if (f) { if (!fgets(data, sizeof data, f)) strcpy(data, "empty"); fclose(f); data[strcspn(data, "\n")] = 0; }
  printf("demo ok yaml=%s data=%s args=%d\n", yaml_get_version_string(), data, argc - 1);
  return 0;
}
C
gcc -O2 -o "$src/demo" "$src/demo.c" -lyaml

# The .deb
d="$src/deb"
mkdir -p "$d/DEBIAN" "$d/opt/demo-app/resources" "$d/usr/bin" "$d/usr/share/applications" "$d/usr/share/icons/hicolor/256x256/apps"
cp "$src/demo" "$d/opt/demo-app/demo"
echo "from-the-package" > "$d/opt/demo-app/resources/data.txt"
ln -s /opt/demo-app/demo "$d/usr/bin/demo-app"
printf '#!/bin/sh\n# the way Electron apps start\nexec /opt/demo-app/demo "$@"\n' > "$d/usr/bin/demo-app-launch"
chmod 755 "$d/usr/bin/demo-app-launch"
cat > "$d/usr/share/applications/demo-app.desktop" <<'D'
[Desktop Entry]
Name=Demo App
Comment=AppImage Maker's test app
Exec=demo-app-launch %U
Icon=demo-app
Type=Application
Categories=Utility;
Actions=new;

[Desktop Action new]
Name=New
Exec=demo-app-launch --new
D
convert -size 256x256 xc:'#ff8a3d' -fill '#2a1206' -draw 'circle 128,128 128,40' "$d/usr/share/icons/hicolor/256x256/apps/demo-app.png"
cat > "$d/DEBIAN/control" <<'D'
Package: demo-app
Version: 1.0
Architecture: amd64
Maintainer: Hearth CI <ci@example.invalid>
Depends: libc6, libyaml-0-2
Description: AppImage Maker's test app
D
dpkg-deb --root-owner-group --build "$d" "$out/demo-app_1.0_amd64.deb" >/dev/null

# The tarball
t="$src/demo-tool-2.1"
mkdir -p "$t/lib" "$t/resources"
gcc -O2 -o "$t/demo-tool" "$src/demo.c" -lyaml -Wl,-rpath,'$ORIGIN/lib'
cp -L "$(ldconfig -p | awk '/libyaml-0.so.2 .*x86-64/{print $NF; exit}')" "$t/lib/libyaml-0.so.2"
echo "from-the-tarball" > "$t/resources/data.txt"
tar -C "$src" -czf "$out/demo-tool-2.1-linux-x64.tar.gz" demo-tool-2.1

# GNU Hello from Ubuntu
(cd "$out" && apt-get download hello >/dev/null)
ls -l "$out"
