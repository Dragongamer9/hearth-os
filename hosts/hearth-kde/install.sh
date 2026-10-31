#!/bin/bash
# Hearth's KDE apps in a Hearth OS image (docs/KDE-APPS.md). Run by os-bases/anatase/build.sh and
# os-bases/peppermint/build.sh, before Hearth's default apps are worked out:
#   install.sh [HEARTH_KDE_TAR]
# HEARTH_KDE_TAR is CI's build (.github/scripts/hearth-kde-build.sh: Hearth Files and kio-hearth, built inside this
# base); without it (or with a part that doesn't check out here) the image goes without that part: the old Hearth
# Files stays the file manager, and hearth:/ isn't there.
#  - Hearth Terminal (Konsole), Hearth Text (Kate), Reader (Okular), Capture (Spectacle) and Hearth Calculator (KCalc):
#    the base's own KDE packages, unchanged (GPL; KDE's About and credits as they are), with Hearth's menu entries
#    (apps/), names, icons and colors (config/). KDE's own entries stay, hidden, for the window's app id.
#  - Where an old Hearth app does the same (terminal, text editor, calculator), it stays installed, named
#    "… (classic)"; the KDE-based one is the default (hearth-system's HEARTH_RANK).
# Never fails the image build: anything that doesn't work out is a warning, and that app is left out.
set -u
HERE=$(cd "$(dirname "$0")" && pwd)
TAR=${1:-}
APPS=/usr/share/applications
warn() { echo "warning: Hearth's KDE apps: $*"; }
before=$(du -sxm /usr 2>/dev/null | cut -f1)

# ------------------------------------------------------------------ the base's KDE apps
# dolphin: the libraries and plugins Hearth Files (built from the same Dolphin release) uses. No recommended extras.
PKGS="konsole kate okular spectacle kcalc dolphin"
if command -v dnf5 >/dev/null 2>&1; then
  dnf5 -y install --setopt=install_weak_deps=False --skip-unavailable $PKGS || warn "some of $PKGS didn't install"
elif command -v apt-get >/dev/null 2>&1; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get update -qq || true
  apt-get install -y -qq --no-install-recommends $PKGS || {
    for p in $PKGS; do apt-get install -y -qq --no-install-recommends "$p" || warn "$p didn't install"; done
  }
fi

# ------------------------------------------------------------------ CI's build: Hearth Files and kio-hearth
starts() { ! ldd "$1" 2>/dev/null | grep -q 'not found'; }
if [ -n "$TAR" ] && [ -f "$TAR" ]; then
  # Existing folders (and links to folders, like merged /usr) are left as they are; only the files are added.
  tar -C / --no-same-owner --no-overwrite-dir --keep-directory-symlink -xf "$TAR" || warn "couldn't unpack $TAR"
fi
FILES=false
if [ -x /usr/lib/hearth-files/bin/dolphin ]; then
  # It starts and says it's Hearth Files (offscreen; without Qt's offscreen screen here, linking every library has to do).
  offscreen=$(find /usr/lib /usr/lib64 -name libqoffscreen.so -path '*qt6*' 2>/dev/null | head -1)
  if starts /usr/lib/hearth-files/bin/dolphin && { [ -z "$offscreen" ] || \
       QT_QPA_PLATFORM=offscreen HOME=/tmp /usr/lib/hearth-files/bin/dolphin --help 2>/dev/null | grep -q "Hearth OS's file manager"; }; then
    FILES=true
  else
    warn "Hearth Files doesn't start in this image (libraries: $(ldd /usr/lib/hearth-files/bin/dolphin 2>/dev/null | grep 'not found' | tr '\n' ' ')); left out, the old Hearth Files stays"
    rm -rf /usr/lib/hearth-files
  fi
fi
KIO=$(find /usr/lib /usr/lib64 -path '*/plugins/kf6/kio/hearth.so' 2>/dev/null | head -1)
if [ -n "$TAR" ] && [ -f "$TAR" ] && [ -z "$KIO" ]; then
  echo "::warning title=Hearth's KDE apps::kio-hearth isn't where it was expected; the tar has: $(tar -tf "$TAR" | grep -v hearth-files/ | tr '\n' ' ' | cut -c1-600)"
fi
if [ -n "$KIO" ] && ! starts "$KIO"; then
  echo "::warning title=Hearth's KDE apps::kio-hearth ($KIO) doesn't load here: $(ldd "$KIO" 2>&1 | grep 'not found' | tr '\n' ' ' | cut -c1-600)"
  warn "kio-hearth doesn't load in this image; left out"
  rm -f "$KIO"
  KIO=""
fi

# ------------------------------------------------------------------ Hearth's pieces
install -Dm644 "$HERE/hearth_kde.py" /usr/share/hearth-kde/hearth_kde.py
install -d /etc/xdg
for rc in "$HERE"/config/*rc; do install -Dm644 "$rc" "/etc/xdg/$(basename "$rc")"; done
install -Dm644 "$HERE/config/Hearth.profile" /usr/share/konsole/Hearth.profile
install -Dm644 "$HERE/config/Hearth.colorscheme" /usr/share/konsole/Hearth.colorscheme
# KDE apps' own color menus pick from here (the images also install it with the desktop theme).
[ -f /usr/share/color-schemes/Hearth.colors ] || install -Dm644 /usr/share/nimbus/desktop/theme/Hearth.colors /usr/share/color-schemes/Hearth.colors 2>/dev/null || true

installed=""
# id-of-Hearth's-entry  program  KDE's-entry  Hearth-name  icon  old-Hearth-entry-it-replaces
while read -r entry prog stock name icon old; do
  if [ "$entry" = hearth-files-kde.desktop ]; then
    [ "$FILES" = true ] || continue
  elif ! command -v "$prog" >/dev/null 2>&1; then
    continue
  fi
  install -Dm644 "$HERE/apps/$entry" "$APPS/$entry"
  name=${name//_/ }
  if [ -f "$APPS/$stock" ]; then
    if [ "$entry" = hearth-files-kde.desktop ]; then
      python3 /usr/share/hearth-kde/hearth_kde.py rebrand "$APPS/$stock" "$name" "$icon" /usr/bin/hearth-files-kde || warn "couldn't rename $stock"
    else
      python3 /usr/share/hearth-kde/hearth_kde.py rebrand "$APPS/$stock" "$name" "$icon" || warn "couldn't rename $stock"
    fi
  fi
  if [ "$old" != - ] && [ -f "$APPS/$old" ]; then
    python3 /usr/share/hearth-kde/hearth_kde.py classic "$APPS/$old" || true
  fi
  installed="$installed ${name}"
done <<'LIST'
hearth-files-kde.desktop      dolphin   org.kde.dolphin.desktop    Files        hearth-files       hearth-files.desktop
hearth-terminal-kde.desktop   konsole   org.kde.konsole.desktop    Terminal     hearth-terminal    hearth-terminal.desktop
hearth-text-kde.desktop       kate      org.kde.kate.desktop       Text_Editor  hearth-text        hearth-text.desktop
hearth-reader.desktop         okular    org.kde.okular.desktop     Reader       hearth-reader      -
hearth-capture.desktop        spectacle org.kde.spectacle.desktop  Capture      hearth-capture     -
hearth-calculator-kde.desktop kcalc     org.kde.kcalc.desktop      Calculator   hearth-calculator  hearth-calculator.desktop
LIST

if [ "$FILES" = true ]; then
  install -Dm755 "$HERE/hearth-files-kde" /usr/bin/hearth-files-kde
  # Hearth Storage in every KDE app's Places at sign-in (any desktop that runs XDG autostart; Hearth Files does it too).
  install -d /etc/xdg/autostart
  cat > /etc/xdg/autostart/hearth-kde-places.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth Storage in Places
Exec=/usr/bin/hearth-files-kde --sync-places
NoDisplay=true
X-KDE-autostart-phase=2
DESKTOP
fi
# hearth-files-kde also brings Places up to date; without Hearth Files the places still help KDE's file dialogs.
if [ "$FILES" != true ] && [ -n "$KIO" ]; then
  install -d /etc/xdg/autostart
  cat > /etc/xdg/autostart/hearth-kde-places.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth Storage in Places
Exec=python3 /usr/share/hearth-kde/hearth_kde.py places
NoDisplay=true
DESKTOP
fi
command -v update-desktop-database >/dev/null && update-desktop-database -q "$APPS" || true

after=$(du -sxm /usr 2>/dev/null | cut -f1)
build=$(cat /usr/share/hearth-kde/build.json 2>/dev/null | tr -d '\n' || true)
summary="installed:${installed:- none}; Hearth Files (Dolphin build): $FILES; kio-hearth: $([ -n "$KIO" ] && echo yes || echo no); /usr grew by $((after - before)) MB; CI build: ${build:-none}"
echo "Hearth's KDE apps: $summary"
echo "::notice title=Hearth's KDE apps::$summary"
mkdir -p /usr/share/hearth-kde
echo "{\"files\": $FILES, \"kio\": $([ -n "$KIO" ] && echo true || echo false), \"growMB\": $((after - before))}" > /usr/share/hearth-kde/installed.json
exit 0
