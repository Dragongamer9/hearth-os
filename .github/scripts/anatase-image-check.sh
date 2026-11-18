#!/bin/bash
# Hearth OS on Anatase (1.15 test): what the image must have. Runs INSIDE the built image (base-anatase.yml:
#   podman run --rm -v $PWD:/src:ro IMAGE bash /src/.github/scripts/anatase-image-check.sh).
# REQUIRED checks fail the job (::error); nice-to-haves are ::warning. A short summary goes out as ::notice lines.
fail=0
check() { if bash -c "$2" >/dev/null 2>&1; then echo "ok: $1"; else echo "::error title=check::$1"; fail=1; fi; }
warn() { if bash -c "$2" >/dev/null 2>&1; then echo "ok: $1"; else echo "::warning title=check::$1"; fi; }

echo "--- the base"
check "it's Fedora $(rpm -E %fedora) (45 or newer)" "[ \"\$(rpm -E %fedora)\" -ge 45 ]"
echo "::notice title=base::$(. /usr/lib/os-release; echo "$PRETTY_NAME, Fedora $(rpm -E %fedora)"); kernel $(ls /usr/lib/modules | sort -V | tail -1); $(rpm -q --qf '%{NAME} %{VERSION}' plasma-workspace kwin gamescope steam hhd 2>/dev/null | tr '\n' ';')"
check "it says it's the Anatase edition, the test" "grep -q '\"base\": \"anatase\"' /usr/share/nimbus/image-info.json && grep -q '\"image-tag\": \"test-1.15\"' /usr/share/nimbus/image-info.json"
check "bootc and podman are there (updates and Switch edition)" "command -v bootc && command -v podman"

echo "--- updates, Switch edition, the boot chain (red flag 2)"
check "hearth-admin is valid Python and knows the Anatase test tag" "python3 -m py_compile /usr/libexec/nimbus/hearth-admin && grep -q \"'test-1.15'\" /usr/libexec/nimbus/hearth-admin"
check "the polkit rule for hearth-admin is installed" "test -f /usr/share/polkit-1/rules.d/49-hearth-admin.rules"
check "the clean install's first start is installed, enabled and also runs without a plan (a switch by hand from Debian)" \
  "test -x /usr/libexec/nimbus/hearth-clean-install && test -L /etc/systemd/system/multi-user.target.wants/hearth-clean-install.service && grep -q 'ConditionPathExists=$' /usr/lib/systemd/system/hearth-clean-install.service.d/20-hearth-arrival.conf"
check "the automatic fallback and Recovery are enabled" "systemctl is-enabled hearth-boot-guard.service hearth-boot-good.service hearth-recovery.service"
check "Hearth's boot menu writer is here for a PC that came from Debian, and only runs where that menu is" \
  "test -x /usr/libexec/nimbus/hearth-bls-grub && systemctl is-enabled hearth-bls-grub.service && grep -q 'ConditionPathExists=/boot/grub2/hearth-bls.cfg' /usr/lib/systemd/system/hearth-bls-grub.service.d/10-hearth-anatase.conf"
check "bootupd never writes Fedora's loader over Debian's by itself" "grep -q 'ConditionPathExists=!/boot/grub2/hearth-bls.cfg' /usr/lib/systemd/system/bootloader-update.service.d/20-hearth-debian-loader.conf"
check "the boot splash is Hearth's and the initramfs has ostree" "plymouth-set-default-theme | grep -qx hearth && lsinitrd /usr/lib/modules/\$(ls /usr/lib/modules | sort -V | tail -1)/initramfs.img | grep -q ostree"
warn "/usr is composefs on Anatase (prepare-root.conf)" "grep -rqs 'enabled *= *\\(yes\\|true\\|verity\\)' /usr/lib/ostree/prepare-root.conf"

echo "--- Game Mode: Steam Game Mode with Hearth Sync"
check "the Hearth app links every library it needs" "test -x /usr/lib/nimbus/nimbus && ! ldd /usr/lib/nimbus/nimbus | grep -q 'not found'"
/usr/lib/nimbus/nimbus --check-update-path || { echo "::error title=check::the Hearth app's update path check failed"; fail=1; }
check "the session and its mode switch are installed" "test -x /usr/bin/nimbus-session && test -x /usr/bin/nimbus-mode && test -f /usr/share/wayland-sessions/nimbus.desktop"
check "gamescope and Steam are there" "command -v gamescope && command -v steam"
check "Decky Loader is built in" "test -x /usr/lib/hearth-decky/PluginLoader && test -f /usr/lib/systemd/system/hearth-decky@.service"
check "Hearth Sync (the Decky plugin) is in" "test -f /usr/share/hearth/decky-hearth-sync/plugin.json && test -f /usr/share/hearth/decky-hearth-sync/dist/index.js"
warn "Hearth Music and Hearth Link (Decky plugins) are in" "test -f /usr/share/hearth/decky-hearth-music/plugin.json && test -f /usr/share/hearth/decky-hearth-link/plugin.json"
check "GE-Proton is built in and hearth-proton-run picks it" "HOME=/nonexistent /usr/bin/hearth-proton-run --print-proton | grep -q GE-Proton"
warn "Handheld Daemon is there and enabled" "systemctl is-enabled hhd.service"
check "Quick Access over games is Hearth's (HHD's overlay app)" "test -L /usr/bin/hhd-ui && grep -q HHD_OVERLAY /usr/lib/systemd/system/hhd.service.d/zz-hearth-overlay.conf"

echo "--- Desktop Mode: stock KDE Plasma with Hearth's apps"
check "Desktop Mode is Plasma here" "[ \"\$(cat /usr/share/nimbus/desktop-mode)\" = plasma ] && grep -q 'run_plasma' /usr/bin/nimbus-session && grep -q hearth-plasma.pid /usr/bin/nimbus-mode"
check "Plasma's pieces: startplasma-wayland, plasmashell, kwin_wayland, kscreen-doctor, the lock screen's greeter" \
  "command -v startplasma-wayland && command -v plasmashell && command -v kwin_wayland && command -v kscreen-doctor && ls /usr/libexec/kscreenlocker_greet /usr/lib*/libexec/kscreenlocker_greet 2>/dev/null | grep -q ."
check "Plasma's menu file is there" "ls /etc/xdg/menus/applications.menu /etc/xdg/menus/*-applications.menu 2>/dev/null | grep -q ."
warn "KWin's Overview, Breeze frames and Aurorae are there" "ls /usr/share/kwin*/effects/overview /usr/share/kwin/effects/overview 2>/dev/null | grep -q . ; ls /usr/lib64/qt6/plugins/org.kde.kdecoration*/ 2>/dev/null | grep -qi breeze"
check "no labwc, Quickshell or Mantel session here (Mantel is archived on Anatase)" \
  "! command -v labwc && ! command -v quickshell && test ! -e /usr/share/wayland-sessions/hearth-classic.desktop"
check "KDE's own Plasma login entry is aside (Plasma starts from Hearth's session)" "test ! -e /usr/share/wayland-sessions/plasma.desktop"
check "Back to Game Mode is in Plasma's menu and pinned once per account" \
  "grep -qx 'Name=Back to Game Mode' /usr/share/applications/hearth-gamemode.desktop && grep -q 'Exec=nimbus-mode game' /usr/share/applications/hearth-gamemode.desktop && grep -q hearth-gamemode.desktop /usr/share/plasma/shells/org.kde.plasma.desktop/contents/updates/hearth-game-mode-launcher.js"
# Hearth's apps in Plasma's menu: every one shown under XDG_CURRENT_DESKTOP=KDE by the desktop-entry rules
# (NoDisplay, Hidden, OnlyShowIn, NotShowIn, TryExec), with its program there. Plasma's session leaves Hearth's
# one-app-of-each-kind copies (/usr/share/hearth/app-kinds) out of XDG_DATA_DIRS, so the packages' own files count.
python3 - <<'PY' || fail=1
import configparser, os, shutil, sys
want = ['hearth-settings.desktop', 'hearth-logs.desktop', 'hearth-send-logs.desktop', 'hearth-gamemode.desktop',
        'hearth-browser.desktop', 'hearth-text.desktop', 'hearth-disks.desktop', 'hearth-photos.desktop',
        'hearth-player.desktop', 'hearth-system-info.desktop']
must = {'hearth-settings.desktop', 'hearth-logs.desktop', 'hearth-send-logs.desktop', 'hearth-gamemode.desktop'}
bad, shown = [], []
for d in want:
    p = '/usr/share/applications/' + d
    if not os.path.exists(p):
        (bad if d in must else []).append(d + ' (missing)'); continue
    c = configparser.RawConfigParser(strict=False, interpolation=None); c.optionxform = str
    c.read(p)
    e = c['Desktop Entry'] if c.has_section('Desktop Entry') else {}
    why = None
    if e.get('NoDisplay', '').lower() == 'true' or e.get('Hidden', '').lower() == 'true': why = 'NoDisplay'
    elif e.get('OnlyShowIn') and 'KDE' not in e.get('OnlyShowIn').split(';'): why = 'OnlyShowIn=' + e.get('OnlyShowIn')
    elif 'KDE' in (e.get('NotShowIn') or '').split(';'): why = 'NotShowIn=KDE'
    elif e.get('TryExec') and not shutil.which(e.get('TryExec')): why = 'TryExec missing'
    else:
        prog = (e.get('Exec') or '').split()[0] if e.get('Exec') else ''
        if prog and not shutil.which(prog): why = 'program missing: ' + prog
    if why:
        (bad if d in must else []).append(d + ' (' + why + ')')
        if d not in must: print('::warning title=Plasma menu::%s is hidden in Plasma: %s' % (d, why))
    else:
        shown.append(d)
print('::notice title=Plasma menu (desktop-entry rules)::shown under KDE: ' + ', '.join(shown))
if bad:
    print('::error title=Plasma menu::Hearth apps that must be in Plasma\'s menu but aren\'t: ' + ', '.join(bad)); sys.exit(1)
PY
# And as KService itself builds Plasma's menu (kbuildsycoca6, then the menu tree through kioclient6's applications:/).
if command -v kbuildsycoca6 >/dev/null; then
  pre=""; [ -e /etc/xdg/menus/applications.menu ] || pre=$(basename "$(ls /etc/xdg/menus/*-applications.menu | head -1)" applications.menu)
  export HOME=/tmp/kmenu-home XDG_CACHE_HOME=/tmp/kmenu-cache XDG_CURRENT_DESKTOP=KDE XDG_MENU_PREFIX="$pre" QT_QPA_PLATFORM=offscreen
  mkdir -p "$HOME" "$XDG_CACHE_HOME"
  kbuildsycoca6 --noincremental >/tmp/sycoca.log 2>&1 || true
  echo "::notice title=kbuildsycoca6::$(tail -3 /tmp/sycoca.log | tr '\n' ' ' | cut -c1-400)"
  if command -v kioclient6 >/dev/null; then
    tree=$( { for d in '' System Utilities Settings Games Internet Office Multimedia Graphics Development; do timeout 20 kioclient6 ls "applications:/$d" 2>/dev/null; done; } | tr '\n' ' ')
    echo "::notice title=Plasma menu (KService)::$(echo "$tree" | tr ' ' '\n' | grep -i hearth | sort -u | tr '\n' ' ' | cut -c1-1500)"
    for a in hearth-settings hearth-logs hearth-send-logs hearth-gamemode; do
      case "$tree" in *"$a.desktop"*) echo "ok: KService lists $a in Plasma's menu";; *) echo "::warning title=Plasma menu (KService)::$a isn't in kioclient6's listing (the desktop-entry check above decides)";; esac
    done
  fi
fi

echo "--- Hearth's apps outside Mantel"
for b in hearth-settings hearth-send-logs hearth-browser hearth-filemanager; do check "$b is installed" "test -x /usr/bin/$b"; done
for b in hearth-disks hearth-text hearth-time hearth-photos hearth-documents hearth-player hearth-calculator hearth-tools hearth-recorder hearth-link hearth-connect hearth-gamestore; do
  warn "$b is installed with every library it links" "test -x /usr/bin/$b && ! ldd /usr/bin/$b | grep -q 'not found'"
done
check "Hearth Settings opens the Hearth app's Settings without Mantel" "grep -q 'nimbus --windowed --settings' /usr/bin/hearth-settings"
check "Game Mode's file dialogs aren't Mantel's picker here (it needs Quickshell)" "! grep -q 'FileChooser=hearth' /usr/share/xdg-desktop-portal/nimbus-portals.conf"

echo "--- Unlock the file system"
check "hearth-unlock (dnf form) is installed, valid and locked by default" \
  "python3 -m py_compile /usr/bin/hearth-unlock && grep -q 'system-unlocked' /usr/bin/hearth-unlock && test ! -e /etc/hearth/system-unlocked"
check "dnf can't touch the boot chain" "grep -q 'excludepkgs=kernel' /etc/dnf/libdnf5.conf.d/20-hearth-boot-chain.conf && cmp -s /etc/dnf/libdnf5.conf.d/20-hearth-boot-chain.conf /usr/share/hearth/unlock/dnf-boot-chain.conf"
warn "dnf5 reads the boot-chain exclude" "dnf5 --dump-main-config 2>/dev/null | grep -i excludepkgs | grep -q kernel"

echo "--- services stay off until asked for"
for u in smb.service tailscaled.service hearth-ai-share-firewall.timer hearth-remote-desktop-firewall.service lactd.service; do
  check "$u is off" "[ \"\$(systemctl is-enabled $u 2>/dev/null)\" != enabled ]"
done
check "memory, guards and Hearth's own services are on (as on Debian)" "systemctl is-enabled hearth-memory.service hearth-memguard.service hearth-hwset.service hearth-diskguard.service nimbus-autologin.service hearth-flatpak-setup.service"
check "no fingerprint in front of every password" "! grep -qs pam_fprintd /etc/pam.d/system-auth /etc/pam.d/password-auth"

echo "--- lint"
bootc container lint >/tmp/lint.log 2>&1 || { echo "::error title=bootc lint::$(tail -5 /tmp/lint.log | tr '\n' ' ' | cut -c1-500)"; fail=1; }
echo "::notice title=image size::/usr $(du -sxh /usr 2>/dev/null | cut -f1)"
exit "$fail"
