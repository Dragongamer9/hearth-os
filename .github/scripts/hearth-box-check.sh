#!/bin/bash
# Run by base-peppermint.yml and base-anatase.yml inside the built image, in a privileged container (nested rootless
# podman and FUSE need it). WARN-ONLY: everything here needs the network or nested containers, which a CI runner may
# not give. Proves what it can for a throwaway account:
#   - rootless podman works (`podman info`) with the id ranges hearth-home-repair gave the account
#   - Hearth's Debian box: made by hearth-debbox, a small .deb (hearth/linux/test/make_test_deb.py) installed into it,
#     its app exported to the account's menu, the command run in the box, then removed
#   - AppImages: a classic (FUSE 2) AppImage starts (--appimage-version, then --version through FUSE, then unpacked),
#     and Hearth Files puts it in the app menu
#   - Bazaar: whether hearth-flatpak-setup can set up the system installation and install it (network)
#   - Hearth's app store (hearth-apps): Flathub's catalog loads for the account, also when its own Flatpak folder
#     belongs to root (the system's catalog then), after the system repo was broken the way Peppermint 0.6.0 left it
# Mounted files: /box.sh (this), /make_test_deb.py.
#   bash /box.sh [box|appimage|flatpak|all]   (one part per CI step: GitHub shows only 10 notices per step)
set -u
PART=${1:-all}
want() { [ "$PART" = all ] || [ "$PART" = "$1" ]; }
w() { echo "::warning title=box check::$*"; }
ok() { echo "ok: $*"; echo "::notice title=box check ok::$*"; }
note() { echo "::notice title=box check::$*"; }
mkdir -p /var/lib /var/home /var/tmp /run/user
chmod 1777 /var/tmp      # as tmpfiles.d makes it on a real system (podman unpacks images there)
[ -d /usr/lib/sysimage/dpkg ] && ln -sfn /usr/lib/sysimage/dpkg /var/lib/dpkg
useradd -m -d /var/home/boxtest -s /bin/bash boxtest 2>/dev/null || true
/usr/libexec/nimbus/hearth-home-repair --user boxtest >/dev/null 2>&1 || true
U=$(id -u boxtest)
install -d -o boxtest -g boxtest -m 700 "/run/user/$U"
H=/var/home/boxtest
# vfs storage and cgroupfs: what works for podman nested in a container without systemd.
install -d -o boxtest -g boxtest "$H/.config/containers" "$H/Downloads"
printf '[storage]\ndriver = "vfs"\n' > "$H/.config/containers/storage.conf"
# (no systemd here, so no journald: events and container logs both go to files)
printf '[containers]\nlog_driver = "k8s-file"\n[engine]\ncgroup_manager = "cgroupfs"\nevents_logger = "file"\n' > "$H/.config/containers/containers.conf"
chown -R boxtest:boxtest "$H"
as() { runuser -u boxtest -- env HOME="$H" USER=boxtest LOGNAME=boxtest XDG_RUNTIME_DIR="/run/user/$U" PATH=/usr/local/bin:/usr/bin:/bin "$@"; }
tailof() { tail -n "${2:-4}" "$1" 2>/dev/null | tr '\n' ' ' | cut -c1-700; }

if want box; then
echo "--- rootless podman"
if as podman info >/tmp/podman-info.txt 2>&1; then ok "rootless podman works for a new account ($(grep -m1 -i 'graphDriverName' /tmp/podman-info.txt | xargs))"
else w "rootless podman info failed: $(tailof /tmp/podman-info.txt)"; fi

echo "--- Hearth's Debian box"
python3 -I /make_test_deb.py "$H/Downloads/hearth-hello.deb" >/dev/null && chown boxtest:boxtest "$H/Downloads/hearth-hello.deb"
as hearth-debbox inspect "$H/Downloads/hearth-hello.deb" > /tmp/inspect.txt 2>&1 && grep -q '"package": "hearth-hello"' /tmp/inspect.txt \
  && ok "hearth-debbox reads the .deb" || w "hearth-debbox inspect failed: $(tailof /tmp/inspect.txt)"
if as timeout 1500 hearth-debbox install "$H/Downloads/hearth-hello.deb" > /tmp/debbox.txt 2>&1 \
   && grep -q 'hearth-debian-hearth-hello.desktop' /tmp/debbox.txt && [ -f "$H/.local/share/applications/hearth-debian-hearth-hello.desktop" ]; then
  ok "a .deb installed into the Debian box and its app exported to the menu"
  note "Debian box: hearth-hello.deb installed, exported as $(grep -m1 '^Exec=' "$H/.local/share/applications/hearth-debian-hearth-hello.desktop")"
  out=$(as timeout 120 distrobox enter --name hearth-debian --no-tty -- hearth-hello 2>&1 | tail -1)
  [ "$out" = "hello from the Debian box" ] && ok "the package's command runs in the box" || w "the command didn't run in the box: $out"
  as hearth-debbox remove hearth-hello > /tmp/debrm.txt 2>&1 && [ ! -e "$H/.local/share/applications/hearth-debian-hearth-hello.desktop" ] \
    && ok "removed again (menu entry gone)" || w "removing it failed: $(tailof /tmp/debrm.txt)"
else
  w "Debian box install didn't work here (network or nested containers): $(grep -v '^progress' /tmp/debbox.txt | tail -5 | tr '\n' ' ' | cut -c1-900)"
  as podman logs --tail 25 hearth-debian > /tmp/boxlog.txt 2>&1 && w "the box's own log (distrobox's setup inside it): $(sed 's/\x1b\[[0-9;]*m//g' /tmp/boxlog.txt | tr '\n' ' ' | cut -c1-1500)"
  as podman exec --user root hearth-debian sh -c 'getent hosts deb.debian.org || echo "no DNS for deb.debian.org"; command -v sudo || echo "no sudo (distrobox setup unfinished)"; ls -l /usr/bin/distrobox-export /usr/bin/distrobox-host-exec 2>&1' > /tmp/inbox.txt 2>&1; w "inside the box: $(tr '\n' ' ' < /tmp/inbox.txt | cut -c1-600)"
  as timeout 300 distrobox enter --name hearth-debian --no-tty -- true > /tmp/enter.txt 2>&1; w "distrobox enter again (exit $?): $(sed 's/\x1b\[[0-9;]*m//g' /tmp/enter.txt | tail -8 | tr '\n' ' ' | cut -c1-900)"
fi

fi

if want appimage; then
echo "--- AppImages"
mkdir -p /tmp/ai
# AppImageKit 13's appimagetool: the classic runtime that loads libfuse.so.2 (what most AppImages still use); the
# newer appimagetool (static runtime) if that one can't be had.
if curl -fsSL --retry 3 -m 180 -o /tmp/ai/appimagetool.AppImage https://github.com/AppImage/AppImageKit/releases/download/13/appimagetool-x86_64.AppImage 2>/tmp/ai/curl.txt \
   || curl -fsSL --retry 3 -m 180 -o /tmp/ai/appimagetool.AppImage https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage 2>>/tmp/ai/curl.txt; then
  chmod +x /tmp/ai/appimagetool.AppImage
  /tmp/ai/appimagetool.AppImage --appimage-version >/tmp/ai/v.txt 2>&1 && ok "a type-2 AppImage runtime starts: $(head -1 /tmp/ai/v.txt)" || w "--appimage-version failed: $(tailof /tmp/ai/v.txt)"
  if /tmp/ai/appimagetool.AppImage --version >/tmp/ai/f.txt 2>&1; then ok "the AppImage mounts with FUSE and runs: $(tail -1 /tmp/ai/f.txt)"
  else w "the AppImage didn't run through FUSE here: $(tailof /tmp/ai/f.txt)"; fi
  APPIMAGE_EXTRACT_AND_RUN=1 /tmp/ai/appimagetool.AppImage --version >/tmp/ai/x.txt 2>&1 && ok "and unpacked (the fallback)" || w "APPIMAGE_EXTRACT_AND_RUN failed: $(tailof /tmp/ai/x.txt)"
  cp /tmp/ai/appimagetool.AppImage "$H/Downloads/appimagetool-x86_64.AppImage" && chown boxtest:boxtest "$H/Downloads/appimagetool-x86_64.AppImage"
  if as python3 /usr/share/nimbus/desktop/shell/hearth-files appimage-menu-add "$H/Downloads/appimagetool-x86_64.AppImage" > /tmp/ai/menu.txt 2>&1 \
     && f=$(ls "$H"/.local/share/applications/appimage-*.desktop 2>/dev/null | head -1) && [ -n "$f" ] && grep -q "^Exec=.*$H/Applications/appimagetool" "$f"; then
    ok "Hearth Files put the AppImage in the app menu ($(grep -m1 '^Name=' "$f"), icon: $(grep -m1 '^Icon=' "$f" | cut -c1-120))"
  else w "Add to app menu failed: $(tailof /tmp/ai/menu.txt)"; fi
else
  w "couldn't download a test AppImage (network): $(tailof /tmp/ai/curl.txt)"
fi

fi

if want flatpak; then
echo "--- Flatpak's system installation and Bazaar"
# The way Peppermint up to 0.6.0 left it: /var/lib/flatpak/repo there but never set up (flatpak: openat(config)).
rm -rf /var/lib/flatpak && mkdir -p /var/lib/flatpak/repo/objects /var/lib/flatpak/repo/tmp
flatpak remotes --system > /tmp/fp-broken.txt 2>&1; grep -q 'openat(config)' /tmp/fp-broken.txt && note "repo broken on purpose: $(tailof /tmp/fp-broken.txt 1)"
if timeout 900 /usr/libexec/nimbus/hearth-flatpak-setup > /tmp/fp.txt 2>&1 && flatpak info --system io.github.kolunmi.Bazaar >/dev/null 2>&1 || [ -f /usr/share/applications/io.github.kolunmi.Bazaar.desktop ]; then
  ok "Bazaar is installed (hearth-flatpak-setup, or the image's package)"
else
  w "hearth-flatpak-setup couldn't install Bazaar here (network?): $(tailof /tmp/fp.txt 6)"
fi
note "hearth-flatpak-setup: $(grep -E '^(repo|flathub|bazaar)' /tmp/fp.txt | tr '\n' ' ' | cut -c1-500)"
test -f /var/lib/flatpak/repo/config && flatpak remotes --system --columns=name 2>/dev/null | grep -qx flathub \
  && ok "the broken system repo was repaired (repo/config) and has the flathub remote" || w "system repo after hearth-flatpak-setup: $(flatpak remotes --system 2>&1 | tr '\n' ' ' | cut -c1-300)"

echo "--- Hearth's app store (hearth-apps) for an account"
HA=/usr/share/nimbus/desktop/shell/hearth-apps
if as timeout 600 python3 "$HA" refresh > /tmp/ha-refresh.txt 2>&1 && as python3 "$HA" search firefox > /tmp/ha-search.txt 2>&1 \
   && grep -q 'org.mozilla.firefox' /tmp/ha-search.txt; then
  ok "Apps loads Flathub's catalog for the account ($(tail -1 /tmp/ha-refresh.txt | cut -c1-120)) and finds Firefox"
else
  w "Apps couldn't load the catalog: $(grep -v '^progress' /tmp/ha-refresh.txt | tail -3 | tr '\n' ' ' | cut -c1-500) / search: $(tailof /tmp/ha-search.txt 2)"
fi
# The account's own Flatpak folder owned by root (moving editions): Apps says so and uses the system's catalog.
timeout 600 flatpak update --appstream --system flathub > /tmp/ha-sysas.txt 2>&1 || w "system appstream: $(tailof /tmp/ha-sysas.txt 2)"
rm -rf "$H/.local/share/flatpak" "$H/.cache/nimbus/apps" && mkdir -p "$H/.local/share/flatpak" && chown root:root "$H/.local/share/flatpak"
as python3 "$HA" status > /tmp/ha-status.txt 2>&1
if grep -q '"userWritable": false' /tmp/ha-status.txt && grep -q '"installTo": "system"' /tmp/ha-status.txt; then
  ok "Apps sees a root-owned ~/.local/share/flatpak and installs to the system instead"
else w "Apps status with a root-owned Flatpak folder: $(tailof /tmp/ha-status.txt 3)"; fi
as timeout 600 python3 "$HA" refresh > /tmp/ha-refresh2.txt 2>&1
if as python3 "$HA" search firefox 2>/dev/null | grep -q 'org.mozilla.firefox'; then ok "and still lists Flathub's apps from the system's catalog"
else w "no catalog with a root-owned Flatpak folder: $(grep -v '^progress' /tmp/ha-refresh2.txt | tail -2 | tr '\n' ' ' | cut -c1-500)"; fi
/usr/libexec/nimbus/hearth-home-repair --user boxtest > /tmp/ha-repair.txt 2>&1
[ "$(stat -c %U "$H/.local/share/flatpak")" = boxtest ] && ok "hearth-home-repair gave the folder back to the account" || w "home repair: $(tailof /tmp/ha-repair.txt 3)"
fi
exit 0
