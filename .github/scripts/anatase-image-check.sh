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
check "video can be decoded by the graphics chip (VA-API: Intel iHD and AMD)" "test -e /usr/lib64/dri/iHD_drv_video.so && test -e /usr/lib64/dri/radeonsi_drv_video.so && command -v vainfo"

echo "--- updates, Switch edition, the boot chain (red flag 2)"
check "hearth-admin is valid Python and knows the Anatase test tag" "python3 -m py_compile /usr/libexec/nimbus/hearth-admin && grep -q \"'test-1.15'\" /usr/libexec/nimbus/hearth-admin"
check "the polkit rule for hearth-admin is installed" "test -f /usr/share/polkit-1/rules.d/49-hearth-admin.rules"
check "the clean install's first start is installed, enabled and also runs without a plan (a switch by hand from Debian)" \
  "test -x /usr/libexec/nimbus/hearth-clean-install && test -L /etc/systemd/system/multi-user.target.wants/hearth-clean-install.service && grep -q 'ConditionPathExists=$' /usr/lib/systemd/system/hearth-clean-install.service.d/20-hearth-arrival.conf"
check "the automatic fallback and Recovery are enabled" "systemctl is-enabled hearth-boot-guard.service hearth-boot-good.service hearth-recovery.service"
check "Hearth's boot menu writer is here for a PC that came from Debian, and only runs where that menu is" \
  "test -x /usr/libexec/nimbus/hearth-bls-grub && systemctl is-enabled hearth-bls-grub.service && grep -q 'ConditionPathExists=/boot/grub2/hearth-bls.cfg' /usr/lib/systemd/system/hearth-bls-grub.service.d/10-hearth-anatase.conf"
check "bootupd never writes Fedora's loader over Debian's by itself" "grep -q 'ConditionPathExists=!/boot/grub2/hearth-bls.cfg' /usr/lib/systemd/system/bootloader-update.service.d/20-hearth-debian-loader.conf"
check "bootupctl never updates the boot loader on a PC whose menu is Hearth's (Debian's loader stays)" \
  "grep -q 'hearth-bls.cfg' /usr/bin/bootupctl && grep -q 'HEARTH_BOOTUPD_ALLOW' /usr/bin/bootupctl"
check "hearth-esp-guard is on and gives a loader without grub.cfg the one that finds the menu" \
  "systemctl is-enabled hearth-esp-guard.service && d=\$(mktemp -d) && mkdir -p \$d/EFI/debian \$d/EFI/anatase && echo cfg > \$d/EFI/debian/grub.cfg && echo uuid > \$d/EFI/debian/bootuuid.cfg && touch \$d/EFI/anatase/grubx64.efi && HEARTH_ESP=\$d sh /usr/libexec/nimbus/hearth-esp-guard && grep -qx cfg \$d/EFI/anatase/grub.cfg && grep -qx uuid \$d/EFI/anatase/bootuuid.cfg"
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
  "command -v startplasma-wayland && command -v plasmashell && command -v kwin_wayland && command -v kscreen-doctor && find /usr/libexec /usr/lib64 /usr/lib -name kscreenlocker_greet -type f | grep -q ."
check "Plasma's menu file is there" "ls /etc/xdg/menus/applications.menu /etc/xdg/menus/*-applications.menu 2>/dev/null | grep -q ."
warn "KWin's Overview, Breeze frames and Aurorae are there" "ls /usr/share/kwin*/effects/overview /usr/share/kwin/effects/overview 2>/dev/null | grep -q . ; ls /usr/lib64/qt6/plugins/org.kde.kdecoration*/ 2>/dev/null | grep -qi breeze"
check "no labwc, Quickshell or Mantel session here (Mantel is archived on Anatase)" \
  "! command -v labwc && ! command -v quickshell && test ! -e /usr/share/wayland-sessions/hearth-classic.desktop"
check "KDE's own Plasma login entry is aside (Plasma starts from Hearth's session)" "test ! -e /usr/share/wayland-sessions/plasma.desktop"
check "Back to Game Mode is in Plasma's menu and pinned once per account" \
  "grep -qx 'Name=Back to Game Mode' /usr/share/applications/hearth-gamemode.desktop && grep -q 'Exec=nimbus-mode game' /usr/share/applications/hearth-gamemode.desktop && grep -q hearth-gamemode.desktop /usr/share/plasma/shells/org.kde.plasma.desktop/contents/updates/hearth-game-mode-launcher.js"
# Hearth's apps in Plasma's menu: every Hearth entry, as Plasma's session sees it (Hearth's one-app-of-each-kind copies
# made for Plasma first: /usr/share/hearth/app-kinds-plasma, when the image has them), by the desktop-entry rules
# (NoDisplay, Hidden, OnlyShowIn, NotShowIn, TryExec) with its program there. The four Plasma needs must show; every
# other one is listed as shown or hidden (and why), and the apps the owner named (Nov 18) are a warning when hidden.
python3 - <<'PY' || fail=1
import configparser, glob, os, shutil, sys
dirs = (['/usr/share/hearth/app-kinds-plasma/applications'] if os.path.isdir('/usr/share/hearth/app-kinds-plasma/applications') else []) + \
    ['/usr/local/share/applications', '/usr/share/applications']
must = {'hearth-settings.desktop', 'hearth-logs.desktop', 'hearth-send-logs.desktop', 'hearth-gamemode.desktop'}
named = ['hearth-calculator.desktop', 'hearth-camera.desktop', 'hearth-maps.desktop', 'hearth-write.desktop', 'hearth-sheets.desktop',
         'hearth-slides.desktop', 'hearth-paint.desktop', 'hearth-contacts.desktop', 'hearth-clock.desktop', 'hearth-calendar.desktop',
         'hearth-text.desktop', 'hearth-photos.desktop', 'hearth-player.desktop', 'hearth-terminal.desktop', 'hearth-documents.desktop',
         'hearth-recorder.desktop', 'hearth-browser.desktop', 'hearth-disks.desktop', 'hearth-system-info.desktop']
ids = {}
for d in reversed(dirs):
    for p in glob.glob(d + '/hearth-*.desktop'):
        ids[os.path.basename(p)] = p
bad, shown, hidden = [], [], []
for d in sorted(set(ids) | must):
    p = ids.get(d)
    if not p:
        if d in must: bad.append(d + ' (missing)')
        continue
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
        hidden.append('%s (%s)' % (d, why))
        if d in must: bad.append(d + ' (' + why + ')')
    else:
        shown.append(d)
for d in named:
    if d not in shown:
        print('::warning title=Plasma menu::%s isn\'t in Plasma\'s menu: %s' % (d, next((h for h in hidden if h.startswith(d)), 'not in this image')))
print('::notice title=Plasma menu (desktop-entry rules)::%d shown under KDE: %s' % (len(shown), ', '.join(shown)))
print('::notice title=Plasma menu (hidden)::%s' % (', '.join(hidden) or 'none'))
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

echo "--- every Hearth KDE app starts (the laptop's Files: symbol lookup error)"
for b in hearth-files-kde konsole kate okular spectacle kcalc dolphin; do
  p=$(command -v "$b") || { echo "::error title=check::$b isn't installed"; fail=1; continue; }
  case "$(head -c 4 "$p" | od -An -c | tr -d ' ')" in
    *ELF*) u=$(ldd -r "$p" 2>&1 | grep -E 'undefined symbol|not found' | head -3) ;;
    *) u="" ;;
  esac
  out=$(QT_QPA_PLATFORM=offscreen HOME=/tmp/kapp XDG_RUNTIME_DIR=/tmp/kapp-run timeout 30 "$p" --version 2>&1 | head -5)
  if [ -n "$u" ] || echo "$out" | grep -qE 'symbol lookup error|cannot open shared object|undefined symbol'; then
    echo "::error title=check::$b doesn't start: $(echo "$u $out" | tr '\n' ' ' | cut -c1-500)"; fail=1
  else echo "ok: $b starts ($(echo "$out" | head -1 | cut -c1-80))"; fi
done
for f in /usr/lib/hearth-files/bin/dolphin $(find /usr/lib/hearth-files /usr/lib/hearth-qml -name '*.so*' -type f 2>/dev/null) /usr/lib/hearth-qml/hearth-qml; do
  [ -f "$f" ] || continue
  u=$(LD_LIBRARY_PATH="$(dirname "$f"):$(dirname "$f")/../lib" ldd -r "$f" 2>&1 | grep -E 'undefined symbol|not found' | head -3)
  [ -z "$u" ] || { echo "::error title=check::${f}: $(echo "$u" | tr '\n' ' ' | cut -c1-400)"; fail=1; }
done
for f in /usr/bin/hearth-*; do
  [ -f "$f" ] && head -c 4 "$f" | grep -q ELF || continue
  u=$(ldd -r "$f" 2>&1 | grep -E 'undefined symbol|not found' | head -2)
  [ -z "$u" ] || echo "::warning title=check::${f##*/}: $(echo "$u" | tr '\n' ' ' | cut -c1-300)"
done

echo "--- Hearth's apps outside Mantel"
for b in hearth-settings hearth-send-logs hearth-browser hearth-filemanager; do check "$b is installed" "test -x /usr/bin/$b"; done
for b in hearth-disks hearth-text hearth-time hearth-photos hearth-documents hearth-player hearth-calculator hearth-tools hearth-recorder hearth-link hearth-connect hearth-gamestore; do
  warn "$b is installed with every library it links" "test -x /usr/bin/$b && ! ldd /usr/bin/$b | grep -q 'not found'"
done
check "Hearth Settings opens the Hearth app's Settings without Mantel" "grep -q 'nimbus --windowed --settings' /usr/bin/hearth-settings"
check "Game Mode's file dialogs aren't Mantel's picker here (it needs Quickshell)" "! grep -q 'FileChooser=hearth' /usr/share/xdg-desktop-portal/nimbus-portals.conf"

echo "--- Hearth's look, not Anatase's or Fedora's"
check "the name is Hearth OS, the logo Hearth's (os-release)" "grep -qx 'PRETTY_NAME=\"Hearth OS\"' /usr/lib/os-release && grep -qx 'NAME=\"Hearth OS\"' /usr/lib/os-release && grep -qx 'LOGO=hearth' /usr/lib/os-release && grep -q 'Fedora edition' /usr/lib/os-release && ! grep -E '^(NAME|PRETTY_NAME|VARIANT)=' /usr/lib/os-release | grep -qi anatase"
check "the boot splash is Hearth's, its presets are in the initramfs" "plymouth-set-default-theme | grep -qx hearth && grep -q hearth-aurora /usr/lib/dracut/dracut.conf.d/50-hearth-splash.conf"
check "Plasma's menu button and the distributor logo are Hearth's" \
  "cmp -s /usr/share/icons/hicolor/scalable/apps/start-here-kde.svg /usr/share/icons/hicolor/scalable/apps/hearth.svg && cmp -s /usr/share/icons/hicolor/scalable/apps/distributor-logo.svg /usr/share/icons/hicolor/scalable/apps/hearth.svg"
check "KInfoCenter's About shows Hearth (logo, website; the base only in the details)" "grep -q 'apps/hearth.png' /etc/xdg/kcm-about-distrorc && grep -q hearthos.fun /etc/xdg/kcm-about-distrorc"
check "Plasma's lock screen is Hearth's; other looks' default wallpaper is Hearth's" \
  "grep -q 'Hearth-fireside' /etc/xdg/kscreenlockerrc && for d in /usr/share/plasma/look-and-feel/*/contents/defaults; do case \"\$d\" in */org.hearthos.*) continue;; esac; grep -q 'Image=Hearth-fireside' \"\$d\" || exit 1; done"

echo "--- Hearth's looks as Plasma Global Themes (generated from palettes.json)"
for l in $(python3 -c 'import json; d=json.load(open("/usr/share/nimbus/desktop/theme/palettes.json")); print(" ".join(k for k,v in d.items() if not k.startswith("_") and isinstance(v,dict) and isinstance(v.get("dark"),dict)))'); do
  t=/usr/share/plasma/look-and-feel/org.hearthos.$l
  check "Global Theme org.hearthos.$l: named Hearth OS, its colours, Plasma style, Breeze frames, wallpaper and splash" \
    "grep -q '\"Name\": \"Hearth OS' $t/metadata.json && grep -qx 'ColorScheme=HearthOS-$l-Dark' $t/contents/defaults && grep -qx 'name=hearth-$l' $t/contents/defaults && grep -qx 'library=org.kde.breeze' $t/contents/defaults && test -f /usr/share/wallpapers/\$(sed -n 's/^Image=//p' $t/contents/defaults)/metadata.json && test -f $t/contents/splash/Splash.qml && test -s $t/contents/splash/images/hearth.svg && test -s $t/contents/previews/preview.png && test -f /usr/share/color-schemes/HearthOS-$l-Light.colors && test -f /usr/share/plasma/desktoptheme/hearth-$l-light/colors"
done
for t in /usr/share/plasma/look-and-feel/org.hearthos.*; do
  check "${t##*/} is one of palettes.json's looks" "python3 -c 'import json,sys; d=json.load(open(\"/usr/share/nimbus/desktop/theme/palettes.json\")); sys.exit(0 if \"${t##*/org.hearthos.}\" in d else 1)'"
done
check "Hearth's look is KDE's default (a choice of yours wins); the splash is Hearth's" \
  "grep -qx 'LookAndFeelPackage=org.hearthos.hearth' /etc/xdg/kdeglobals && grep -qx 'name=hearth-hearth' /etc/xdg/plasmarc && grep -qx 'Theme=org.hearthos.hearth' /etc/xdg/ksplashrc && grep -qx 'Engine=KSplashQML' /etc/xdg/ksplashrc"
warn "xrdb is there (plasma-apply-lookandfeel runs it when a Global Theme is applied)" "command -v xrdb"
check "Hearth's Settings and System Settings stay in step (hearth-plasma-looks)" \
  "python3 -c 'import ast,sys; ast.parse(open(\"/usr/libexec/nimbus/hearth-plasma-looks\").read())' && test -x /usr/libexec/nimbus/hearth-plasma-looks && test -f /etc/xdg/autostart/hearth-plasma-looks.desktop && [ \"\$(systemctl --global is-enabled hearth-plasma-looks.path)\" = enabled ]"
check "the old Hearth Plasma look and style are gone (one copy: the generated ones)" "test ! -e /usr/share/plasma/look-and-feel/org.hearth.desktop && test ! -e /usr/share/plasma/desktoptheme/hearth"
check "Plasma's never-picked menu icon and wallpaper become Hearth's once per account" "test -f /usr/share/plasma/shells/org.kde.plasma.desktop/contents/updates/hearth-look-defaults.js"
leftover=$(find /usr/share/icons /usr/share/pixmaps -type f \( -iname '*fedora*logo*' -o -iname '*anatase*' -o -iname 'distributor-logo*' \) -name '*.svg' 2>/dev/null | while read -r f; do cmp -s "$f" /usr/share/icons/hicolor/scalable/apps/hearth.svg || echo "$f"; done | head -5)
[ -z "$leftover" ] || echo "::warning title=check::logo files that still aren't Hearth's: $leftover"

echo "--- start-up stays quick (the laptop: 20 s and 10 s before graphical.target)"
check "hearth-dns-client waits for the network inside the service, not before it" "! grep -q '^ExecStartPre=.*sleep' /usr/lib/systemd/system/hearth-dns-client.service && grep -q 'sleep 20; exec' /usr/lib/systemd/system/hearth-dns-client.service"
check "Hearth Recovery doesn't wait for Handheld Daemon" "! grep -E '^(After|Requires)=' /usr/lib/systemd/system/hearth-recovery.service | grep -q hhd.service"

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
