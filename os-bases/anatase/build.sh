#!/bin/bash
# Turns Anatase (Fedora, bootc) into Hearth OS on Anatase: 1.15's test edition beside Debian (the owner, Nov 17: "Bring
# Anatase back and up to speed on 1.15 so i can test it"). Runs once, inside the image build
# (os-bases/anatase/Containerfile). The same Hearth as the Debian edition (os-bases/peppermint/build.sh) in Fedora's form:
#  - Game Mode: Steam Game Mode with Hearth Sync (Decky built in), Hearth Game Mode as the choice it is on Debian.
#  - Desktop Mode: stock KDE Plasma (plasmashell on KDE's own KWin), with Hearth's apps as ordinary apps in Plasma's
#    menu (the owner, Nov 17: "Just Have KDE plasma on it. with our apps"). No labwc and no Mantel in this image;
#    Firehold isn't offered here (its shell needs Mantel).
#  - Handheld Daemon (HHD), already in Anatase, runs the built-in controller, power limit, fan and lights.
#  - "Unlock the file system" (hearth-unlock, dnf form): off unless turned on in Settings.
#  - /usr/share/nimbus/image-info.json says this is the Anatase edition; Switch edition moves to and from it.
# See docs/BASE-ANATASE.md.
set -ouex pipefail

FEDORA=$(rpm -E %fedora)
echo "Anatase on Fedora $FEDORA"

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's Anatase-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/anatase_files/. /
# From the Debian edition, as they are (os-bases/peppermint/system_files): Hearth's boot menu writer and its units (only
# active on a PC whose boot menu is Hearth's, i.e. one that came here from Debian: their 10-hearth-anatase.conf drop-ins),
# GRUB's Hearth theme, the writable system's units and their "run from the image's /usr" drop-ins, the /etc guard.
PF=/peppermint_files
[ -d "$PF" ] || PF=/ctx/peppermint_files
install -Dm755 "$PF/usr/libexec/nimbus/hearth-bls-grub" /usr/libexec/nimbus/hearth-bls-grub
mkdir -p /usr/share/hearth && cp -a "$PF/usr/share/hearth/grub-theme" /usr/share/hearth/
for u in hearth-bls-grub.service hearth-bls-grub-refresh.service hearth-bls-grub-refresh.path \
         hearth-unlock.service hearth-unlock-good.service hearth-unlock-units.service hearth-unlock-reapply.service hearth-unlock-ldcache.service; do
  install -Dm644 "$PF/usr/lib/systemd/system/$u" "/usr/lib/systemd/system/$u"
done
for d in "$PF"/usr/lib/systemd/system/*.service.d/20-hearth-image-usr.conf; do
  install -Dm644 "$d" "/usr/lib/systemd/system/$(basename "$(dirname "$d")")/20-hearth-image-usr.conf"
done
install -Dm755 "$PF/usr/lib/systemd/system-generators/hearth-etc-guard" /usr/lib/systemd/system-generators/hearth-etc-guard
# Hearth Plasma's old look (Hearth's look-and-feel, desktop theme and plasmoids in the shared system files): not here.
# Desktop Mode is stock KDE Plasma.
rm -rf /usr/share/plasma/look-and-feel/org.hearth.desktop /usr/share/plasma/desktoptheme/hearth \
  /usr/share/hearth-plasma /usr/share/hearth-cosmic /usr/share/gnome-shell/modes/hearth.json
rm -rf /usr/share/plasma/plasmoids/org.hearth.* /usr/share/gnome-shell/extensions/*@hearthos.fun
rm -f /usr/share/applications/hearth-cosmic-*.desktop

# ------------------------------------------------------------------ Hearth icon set (branding/icons)
# Every Hearth app's icon (Hearth, Settings, Files, Terminal, Disks, Browser, Task Manager, Apps, the installer and
# the newer apps: hearth-text, hearth-photos...) in the hicolor theme, so Mantel, labwc's Alt+Tab and any other
# desktop find them by name (Icon=hearth-settings). Drawn by branding/icons/make-icons.py: one SVG each plus PNGs
# from 16 to 256 px. Apps on other branches only need their .desktop file's Icon= set to their name.
dnf5 -y install hicolor-icon-theme gtk-update-icon-cache || dnf5 -y install hicolor-icon-theme || true
for svg in /ctx/icons/apps/*.svg; do
  install -Dm644 "$svg" "/usr/share/icons/hicolor/scalable/apps/$(basename "$svg")"
done
for dir in /ctx/icons/png/*x*; do
  install -d "/usr/share/icons/hicolor/$(basename "$dir")/apps"
  install -m644 "$dir"/*.png "/usr/share/icons/hicolor/$(basename "$dir")/apps/"
done
# The old 512 px Hearth logo (os/system_files, without the tile) would win over the set at large sizes.
rm -f /usr/share/icons/hicolor/512x512/apps/hearth.png
test -f /usr/share/icons/hicolor/scalable/apps/hearth-settings.svg
test -f /usr/share/icons/hicolor/48x48/apps/hearth-files.png
gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
# ---- end of the Hearth icon set

# ------------------------------------------------------------------ Hearth itself, under /usr (the OS updates it)
mkdir -p /usr/lib/nimbus
if [ -x /ctx/nimbus-app/hearth ]; then
  # The Rust app (hosts/hearth-rs): one file, drawn by the system's WebKit.
  install -Dm755 /ctx/nimbus-app/hearth /usr/lib/nimbus/nimbus
  dnf5 -y install webkit2gtk4.1
  # Hearth Browser's Electron build (hosts/linux): the Quick Access overlay app, and the old browser.
  if [ -d /ctx/nimbus-app/browser ]; then
    mkdir -p /usr/lib/hearth-browser && cp -a /ctx/nimbus-app/browser/. /usr/lib/hearth-browser/
    if [ -f /usr/lib/hearth-browser/chrome-sandbox ]; then chown root:root /usr/lib/hearth-browser/chrome-sandbox && chmod 4755 /usr/lib/hearth-browser/chrome-sandbox; fi
  fi
  # Hearth drawn by Chromium (hosts/electron): the `nimbus` program starts it on Nvidia graphics (docs/RENDERERS.md).
  if [ -x /ctx/nimbus-app/electron/hearth-electron ]; then
    mkdir -p /usr/lib/hearth-electron && cp -a /ctx/nimbus-app/electron/. /usr/lib/hearth-electron/
    if [ -f /usr/lib/hearth-electron/chrome-sandbox ]; then chown root:root /usr/lib/hearth-electron/chrome-sandbox && chmod 4755 /usr/lib/hearth-electron/chrome-sandbox; fi
  fi
  # Hearth drawn by Qt Quick (hosts/hearth-qml, Experimental): only when chosen in Settings › Display › Renderer
  # (docs/RENDERERS.md). Built by CI inside this image's base (.github/scripts/hearth-qml-build.sh), so it links this Qt.
  # Its packages: the same list as hearth-qml-build.sh runtime_pkgs.
  if [ -x /ctx/nimbus-app/qml/hearth-qml ]; then
    dnf5 -y install --skip-unavailable --skip-broken qt6-qtbase-gui qt6-qtdeclarative qt6-qtimageformats SDL2 || echo "warning: some of the Qt renderer's packages didn't install"
    mkdir -p /usr/lib/hearth-qml && cp -a /ctx/nimbus-app/qml/. /usr/lib/hearth-qml/
    chmod 755 /usr/lib/hearth-qml/hearth-qml
  fi
else
  cp -a /ctx/nimbus-app/. /usr/lib/nimbus/
fi
test -x /usr/lib/nimbus/nimbus
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
# ---- hearthfetch (hosts/hearthfetch): Hearth OS's system summary for the terminal, built with the app.
if [ -x /ctx/nimbus-app/hearthfetch ]; then install -Dm755 /ctx/nimbus-app/hearthfetch /usr/bin/hearthfetch; fi
# ---- Hearth Sync (hosts/decky-hearth-sync, docs/HEARTH-SYNC.md): the Decky plugin that keeps Hearth's non-Steam games
# in Steam's library, as on Debian. Only files, copied into Decky's plugins folder by the built-in Decky's service
# (hearth-decky run) each time Steam Game Mode starts. Not in this build: skipped with a warning.
for pl in decky-hearth-sync decky-hearth-music decky-hearth-link; do
  if [ -f "/ctx/nimbus-app/$pl/plugin.json" ] && [ -f "/ctx/nimbus-app/$pl/dist/index.js" ]; then
    mkdir -p /usr/share/hearth && rm -rf "/usr/share/hearth/$pl" && cp -a "/ctx/nimbus-app/$pl" "/usr/share/hearth/$pl" \
      && chmod -R a+rX "/usr/share/hearth/$pl" && echo "$pl (Decky plugin) added" \
      || { echo "warning: $pl didn't copy; Steam Game Mode goes without it"; rm -rf "/usr/share/hearth/$pl"; }
  else
    echo "warning: $pl isn't in this build; Steam Game Mode goes without it"
  fi
done
# ---- Decky Loader, built in (docs/STEAM-GAME-MODE.md): the same pinned release as the Debian edition, checked against
# its SHA-256, in /usr/lib/hearth-decky. It runs only while Steam Game Mode does (hearth-decky@<you>.service, started by
# nimbus-session, not enabled; 49-hearth-decky.rules). Bump all three lines together with os-bases/peppermint/build.sh.
DECKY_VERSION=v3.3.1
DECKY_SHA256=7bb73b0502c49e7194f34f9ce4ddb6777764e9277d7a5e54665f66460d50c315
DECKY_SIZE=35277600
curl -fsSL --retry 3 -o /tmp/PluginLoader "https://github.com/SteamDeckHomebrew/decky-loader/releases/download/$DECKY_VERSION/PluginLoader"
echo "$DECKY_SHA256  /tmp/PluginLoader" | sha256sum -c -
test "$(stat -c %s /tmp/PluginLoader)" = "$DECKY_SIZE"
test "$(head -c 4 /tmp/PluginLoader | od -An -tx1 | tr -d ' \n')" = 7f454c46
install -Dm755 /tmp/PluginLoader /usr/lib/hearth-decky/PluginLoader
echo "$DECKY_VERSION" > /usr/lib/hearth-decky/version
rm -f /tmp/PluginLoader
test -f /usr/lib/systemd/system/hearth-decky@.service
echo "Decky Loader $DECKY_VERSION built in"
# ---- end Hearth Decky plugin
# ---- end hearthfetch
# CI builds the app in a debian:trixie container (base-anatase.yml, as for Debian), so every library it links must be
# here under the same name (Fedora's are newer).
dnf5 -y install --skip-unavailable --skip-broken webkit2gtk4.1 gtk3 libsoup3 libxdo openssl-libs || true
if ldd /usr/lib/nimbus/nimbus | grep "not found"; then
  echo "::error title=Hearth app::it needs libraries this image doesn't have: $(ldd /usr/lib/nimbus/nimbus | grep 'not found' | tr -s ' \t' ' ' | tr '\n' '|')"; exit 1
fi
# ---- hearth-steam (hosts/hearth-steam): the lightweight Steam engine, experimental and off by default. Hearth starts its
# user service (hearth-steam-engine.service, not enabled) only while Settings › Library › Steam engine is Lightweight.
# Not in this build, or it doesn't install: skipped with a warning (Hearth then always uses the normal Steam client).
if [ -x /ctx/nimbus-app/hearth-steam/hearth-steam ] && [ -f /ctx/nimbus-app/hearth-steam/hearth-steam-engine.service ]; then
  if mkdir -p /usr/lib/hearth-steam && cp -a /ctx/nimbus-app/hearth-steam/. /usr/lib/hearth-steam/ \
     && install -Dm644 /usr/lib/hearth-steam/hearth-steam-engine.service /usr/lib/systemd/user/hearth-steam-engine.service; then
    chmod 755 /usr/lib/hearth-steam/hearth-steam
    echo "hearth-steam installed: $(du -sh /usr/lib/hearth-steam | cut -f1)"
  else
    echo "warning: hearth-steam didn't install; this image goes without the lightweight Steam engine"
    rm -rf /usr/lib/hearth-steam /usr/lib/systemd/user/hearth-steam-engine.service
  fi
else
  echo "warning: hearth-steam isn't in this build; this image goes without the lightweight Steam engine"
fi
# ---- end hearth-steam

# ------------------------------------------------------------------ SVG icons for GTK (librsvg's gdk-pixbuf loader)
# Without it GTK can't draw an SVG icon and aborts the whole program the first time it needs one (Hearth's Game Mode
# crashed over and over on CachyOS: "image-missing.svg: Unrecognized image file format"). The loaders list is
# rebuilt so the SVG loader is in it.
dnf5 -y install librsvg2 gdk-pixbuf2
# Newer gdk-pixbuf loads SVG (and other images) through glycin's loaders instead.
dnf5 -y install --skip-unavailable --skip-broken glycin-loaders || true
for q in /usr/bin/gdk-pixbuf-query-loaders /usr/bin/gdk-pixbuf-query-loaders-64 /usr/lib/*/gdk-pixbuf-2.0/gdk-pixbuf-query-loaders; do
  [ -x "$q" ] && { "$q" --update-cache || true; break; }
done

# ---- Hearth Terminal (hosts/hearth-terminal): built by CI next to the Hearth app (base-*.yml) and carried in
# os/nimbus-app. Mantel opens it for its terminal (desktop/open.sh); its menu entry (hearth-terminal.desktop) and
# Mantel's xdg-terminal-exec list come from os/system_files. Any other terminal stays as the fallback.
if [ -x /ctx/nimbus-app/hearth-terminal ]; then
  install -Dm755 /ctx/nimbus-app/hearth-terminal /usr/bin/hearth-terminal
  # A terminal that can't start is worse than none: then Mantel falls back to the others (open.sh).
  if ldd /usr/bin/hearth-terminal | grep 'not found'; then echo "Hearth Terminal needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/hearth-terminal; fi
fi
# ---- end of Hearth Terminal

# ------------------------------------------------------------------ Hearth Disks (hosts/hearth-disks)
# The disk manager, built with the app (base-anatase.yml). udisks2 (all its changes go through it) and the tools it formats with.
dnf5 -y install udisks2 exfatprogs dosfstools || true
# Make a bootable USB (Disks' USB maker, src/usb): parted and gdisk lay out a Windows USB drive, wimlib splits and reads
# Windows' install.wim, gpg checks the projects' signatures, xz and zstd unpack compressed disk images.
dnf5 -y install parted gdisk wimlib-utils gnupg2 xz zstd curl || true
for t in parted sgdisk wimlib-imagex wiminfo gpg xz zstd mkfs.vfat fsck.vfat; do command -v "$t" >/dev/null || echo "warning: $t is missing, so Make a bootable USB can't do everything"; done
command -v udisksctl >/dev/null || echo "warning: udisks2 is missing, so Disks can't change drives"
if [ -x /ctx/nimbus-app/hearth-disks ]; then
  install -Dm755 /ctx/nimbus-app/hearth-disks /usr/bin/hearth-disks
  if ldd /usr/bin/hearth-disks | grep "not found"; then echo "warning: Disks is missing the libraries above"; fi
  install -d /usr/share/applications
  cat > /usr/share/applications/hearth-disks.desktop <<DESKTOP
[Desktop Entry]
Name=Disks
GenericName=Disk Manager
Comment=See your drives and their health, format USB drives and SD cards, and set up drives for games
Exec=/usr/bin/hearth-disks
Icon=hearth-disks
Type=Application
Categories=System;Utility;
Keywords=disk;drive;partition;format;usb;sd;card;mount;eject;storage;steam;library;
DESKTOP
  cat > /usr/share/applications/hearth-usb-maker.desktop <<DESKTOP
[Desktop Entry]
Name=Make a Bootable USB
GenericName=Bootable USB Maker
Comment=Download Linux or Windows and put it on a USB drive to install it, checked before it's written
Exec=/usr/bin/hearth-disks --usb %f
Icon=hearth-disks
Type=Application
Categories=System;Utility;
MimeType=application/x-cd-image;application/x-iso9660-image;application/x-raw-disk-image;application/x-raw-disk-image-xz-compressed;
Keywords=usb;iso;bootable;installer;flash;burn;write;linux;windows;fedora;ubuntu;debian;arch;rufus;etcher;
DESKTOP
fi

# ==================================================================== Hearth Recorder, Camera and Share
# (hosts/hearth-recorder, hosts/hearth-camera, hosts/hearth-share; built with the app by base-anatase.yml, tested by
# capture-apps.yml.) Hearth's screen recorder (instant replay and a sound recorder too), webcam app and screen
# sharing. What they run:
#  - ffmpeg: encoding (Anatase's own if it has one, else Fedora's ffmpeg-free), with Cisco's openh264 for H.264 on
#    the processor (Fedora's Mesa leaves out H.264/HEVC on AMD's and Intel's encoders; Recorder tries each one)
#  - wf-recorder and slurp: capturing Mantel's desktop, an area too
#  - gpu-screen-recorder (COPR): capture and encoding on the graphics card in Game Mode and Desktop Mode (NVENC,
#    VA-API), and its own instant replay. gsr-kms-server needs CAP_SYS_ADMIN to read the screen.
#  - GStreamer with PipeWire's plugin: Game Mode (gamescope's own stream) without gpu-screen-recorder, and Share there
#  - pactl and pw-dump (sound sources, gamescope's stream), v4l-utils (cameras), qrencode (Share's QR code)
command -v ffmpeg >/dev/null || dnf5 -y install ffmpeg-free || dnf5 -y install ffmpeg || echo "warning: no ffmpeg, so Recorder, Camera and Share can't encode"
dnf5 -y install --skip-unavailable --skip-broken --setopt=fedora-cisco-openh264.enabled=1 openh264 gstreamer1-plugin-openh264 || true
dnf5 -y install --skip-unavailable --skip-broken wf-recorder slurp pulseaudio-utils pipewire-utils pipewire-gstreamer gstreamer1-plugins-good \
  gstreamer1-plugins-bad-free gstreamer1-plugin-libav v4l-utils libnotify qrencode libcap || true
# CI builds it inside Anatase itself (base-anatase.yml, gsr/ next to the app); the COPR package is the fallback.
if [ -x /ctx/nimbus-app/gsr/gpu-screen-recorder ] && ! command -v gpu-screen-recorder >/dev/null; then
  install -Dm755 /ctx/nimbus-app/gsr/gpu-screen-recorder /usr/bin/gpu-screen-recorder
  if [ -x /ctx/nimbus-app/gsr/gsr-kms-server ]; then install -Dm755 /ctx/nimbus-app/gsr/gsr-kms-server /usr/bin/gsr-kms-server; fi
  if ldd /usr/bin/gpu-screen-recorder | grep 'not found'; then
    echo "warning: gpu-screen-recorder needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/gpu-screen-recorder /usr/bin/gsr-kms-server
  fi
fi
if ! command -v gpu-screen-recorder >/dev/null; then
  if dnf5 -y copr enable brycensranch/gpu-screen-recorder-git && dnf5 -y install gpu-screen-recorder; then
    echo "gpu-screen-recorder installed (COPR)"
  else
    echo "warning: couldn't install gpu-screen-recorder; Recorder uses wf-recorder and GStreamer instead"
  fi
  dnf5 -y copr disable brycensranch/gpu-screen-recorder-git >/dev/null 2>&1 || true
fi
# Without its capability gpu-screen-recorder asks for the password every time it records.
for k in /usr/bin/gsr-kms-server /usr/local/bin/gsr-kms-server; do [ -x "$k" ] && { setcap cap_sys_admin+ep "$k" || echo "warning: couldn't give $k its capability"; }; done
for app in hearth-recorder hearth-camera hearth-share; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    # One that can't start is left out (its menu entry too).
    if ldd "/usr/bin/$app" | grep 'not found'; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
  fi
done
if [ -x /usr/bin/hearth-recorder ] || [ -x /usr/bin/hearth-camera ] || [ -x /usr/bin/hearth-share ]; then
  # Their menu entries (ids starting with "hearth": Mantel's app menu lists them as Hearth's own apps).
  install -d /usr/share/applications
  if [ -x /usr/bin/hearth-recorder ]; then
    cat > /usr/share/applications/hearth-recorder.desktop <<DESKTOP
[Desktop Entry]
Name=Recorder
GenericName=Screen Recorder
Comment=Record the screen, a game or your voice, and keep instant replays of games
Exec=/usr/bin/hearth-recorder
Icon=hearth-recorder
Type=Application
Categories=AudioVideo;Video;Recorder;
Keywords=record;screen;recording;video;capture;replay;clip;game;microphone;voice;sound;
StartupNotify=true
StartupWMClass=hearth-recorder
Actions=sound;

[Desktop Action sound]
Name=Sound recorder
Exec=/usr/bin/hearth-recorder --sound
DESKTOP
  fi
  if [ -x /usr/bin/hearth-camera ]; then
    cat > /usr/share/applications/hearth-camera.desktop <<DESKTOP
[Desktop Entry]
Name=Camera
GenericName=Webcam
Comment=Take photos and videos with your webcam
Exec=/usr/bin/hearth-camera
Icon=hearth-camera
Type=Application
Categories=AudioVideo;Video;Graphics;Photography;
Keywords=camera;webcam;photo;selfie;video;picture;
StartupNotify=true
StartupWMClass=hearth-camera
DESKTOP
  fi
  if [ -x /usr/bin/hearth-share ]; then
    cat > /usr/share/applications/hearth-share.desktop <<DESKTOP
[Desktop Entry]
Name=Share Screen
GenericName=Screen Sharing
Comment=Show this screen on a TV, phone or another computer (web browser or Moonlight)
Exec=/usr/bin/hearth-share
Icon=hearth-share
Type=Application
Categories=Network;AudioVideo;
Keywords=share;screen;cast;mirror;tv;stream;moonlight;miracast;chromecast;browser;
StartupNotify=true
StartupWMClass=hearth-share
DESKTOP
  fi
fi
# Share's browser stream (port 47800, only answered while sharing, only to home-network addresses) through firewalld.
if [ -d /usr/lib/firewalld ] || command -v firewall-offline-cmd >/dev/null; then
  install -d /usr/lib/firewalld/services
  cat > /usr/lib/firewalld/services/hearth-share.xml <<XML
<?xml version="1.0" encoding="utf-8"?>
<service>
  <short>Hearth Share</short>
  <description>Hearth Share's screen sharing for web browsers on your network. It only answers while Share is sharing.</description>
  <port protocol="tcp" port="47800"/>
</service>
XML
  firewall-offline-cmd --add-service=hearth-share >/dev/null 2>&1 || echo "note: Share's port wasn't opened in firewalld's default zone"
fi
# ==================================================================== end of Hearth Recorder, Camera and Share
# ------------------------------------------------------------------ Hearth Time (hosts/hearth-time)
# Clock, Calendar, Notes and Weather: one program, /usr/bin/hearth-time, with a link per app (hearth-clock,
# hearth-calendar, hearth-notes, hearth-weather), their menu entries, and the small background service
# that rings alarms and timers with the apps closed, shows calendar reminders and keeps the weather summary fresh
# (hearth-time.service, a systemd user service, on for every user). Built with the app (base-*.yml), carried in
# os/nimbus-app/hearth-time/. Alarms open their window with xprop/xdotool in Game Mode and wlopm in Desktop Mode
# (installed above), and play their tone with pw-play.
if [ -x /ctx/nimbus-app/hearth-time/hearth-time ]; then
  install -Dm755 /ctx/nimbus-app/hearth-time/hearth-time /usr/bin/hearth-time
  if ldd /usr/bin/hearth-time | grep "not found"; then echo "warning: Hearth Time is missing the libraries above"; fi
  install -d /usr/share/applications /usr/lib/systemd/user
  # Their icons (hearth-clock, hearth-calendar, hearth-notes, hearth-weather) come with Hearth's icon set.
  for a in clock calendar notes weather; do ln -sf hearth-time "/usr/bin/hearth-$a"; done
  hearth_time_entry() {  # id, name, comment, categories, keywords[, file types it opens]
    cat > "/usr/share/applications/hearth-$1.desktop" <<DESKTOP
[Desktop Entry]
Name=$2
Comment=$3
Exec=/usr/bin/hearth-$1${6:+ %u}
TryExec=/usr/bin/hearth-time
Icon=hearth-$1
Type=Application
Categories=$4
Keywords=$5
StartupNotify=true
StartupWMClass=io.github.dragongamer9.hearth.$1
${6:+MimeType=$6}
DESKTOP
  }
  hearth_time_entry clock Clock "Alarms, timers, a stopwatch and world clocks" "Utility;Clock;" "alarm;timer;stopwatch;world;clock;time;"
  hearth_time_entry calendar Calendar "Your events and reminders, and .ics calendars" "Office;Calendar;" "calendar;event;reminder;ics;schedule;agenda;" "text/calendar;x-scheme-handler/webcal;x-scheme-handler/webcals;"
  hearth_time_entry notes Notes "Quick notes and checklists, kept as plain files in Documents/Notes" "Office;Utility;" "note;notes;checklist;todo;markdown;memo;"
  hearth_time_entry weather Weather "The forecast for a place you choose (Open-Meteo)" "Utility;" "weather;forecast;temperature;rain;"
  cat > /usr/lib/systemd/user/hearth-time.service <<'UNIT'
[Unit]
Description=Hearth Clock alarms and timers, Calendar reminders and the weather summary

[Service]
ExecStart=/usr/bin/hearth-time --service
Restart=on-failure
RestartSec=5
Nice=5

[Install]
WantedBy=default.target
UNIT
  systemctl --global enable hearth-time.service 2>/dev/null \
    || { mkdir -p /etc/systemd/user/default.target.wants && ln -sf /usr/lib/systemd/user/hearth-time.service /etc/systemd/user/default.target.wants/hearth-time.service; }
fi
# ---- end of Hearth Time

# ------------------------------------------------------------------ Hearth's gaming apps (hosts/hearth-gaming)
# Controller Lab, Saves and Mods: built with the app (base-anatase.yml) and carried in os/nimbus-app; their icons come
# with Hearth's icon set (branding/icons, above: Icon=hearth-controller-lab ...). Desktop Mode lists them
# with Hearth's own apps (their ids start with "hearth"); Game Mode opens them from Settings › Controls and a
# game's ⋯ menu (hearth-core gaming_apps.rs). Saves' hourly timer is on for everyone and does nothing until a
# schedule is chosen in Saves (hearth-saves --auto). Mods unpacks .7z and .rar files with bsdtar.
dnf5 -y install bsdtar || true
gaming_entry() {   # id, name, generic name, comment, keywords
  [ -x "/usr/bin/$1" ] || return 0
  cat > "/usr/share/applications/$1.desktop" <<DESKTOP
[Desktop Entry]
Name=$2
GenericName=$3
Comment=$4
Exec=/usr/bin/$1
Icon=$1
Type=Application
Categories=Game;Utility;
Keywords=$5
StartupNotify=true
StartupWMClass=$1
DESKTOP
}
install -d /usr/share/applications
for app in hearth-controller-lab hearth-saves hearth-mods; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so this image goes without it"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app needs the libraries above; left out"; rm -f "/usr/bin/$app"; continue; fi
done
gaming_entry hearth-controller-lab "Controller Lab" "Controller Tester" "Test every button, stick, trigger, gyro and rumble; measure stick drift and calibrate" "controller;gamepad;joystick;test;tester;calibrate;drift;deadzone;rumble;gyro;battery;"
gaming_entry hearth-saves "Saves" "Game Save Backup" "Back up and restore your games' saves, and move them to another PC" "save;saves;backup;restore;game;steam;proton;usb;"
gaming_entry hearth-mods "Mods" "Mod Manager" "Install mods for your games, turn them on and off, and choose their order" "mod;mods;modding;manager;nexus;game;"
if [ -x /usr/bin/hearth-saves ]; then
  install -d /usr/lib/systemd/user/timers.target.wants
  cat > /usr/lib/systemd/user/hearth-saves-auto.service <<'UNIT'
[Unit]
Description=Hearth Saves: scheduled game save backup

[Service]
Type=oneshot
ExecStart=/usr/bin/hearth-saves --auto
Nice=10
IOSchedulingClass=idle
UNIT
  cat > /usr/lib/systemd/user/hearth-saves-auto.timer <<'UNIT'
[Unit]
Description=Hearth Saves: look every hour whether a scheduled backup is due

[Timer]
OnBootSec=10min
OnUnitActiveSec=1h
Persistent=true

[Install]
WantedBy=timers.target
UNIT
  ln -sf ../hearth-saves-auto.timer /usr/lib/systemd/user/timers.target.wants/hearth-saves-auto.timer
fi
# Hearth Mods and Nexus Mods: the site's "Mod Manager Download" button opens nxm:// links, which go to Hearth Mods'
# download queue (hearth-mods --nxm; hearth-system makes it the default for them with Hearth's other apps). Mods put in
# with the overlay (fuse-overlayfs) are laid over their games again at each sign-in (hearth-mods --remount).
if [ -x /usr/bin/hearth-mods ]; then
  cat > /usr/share/applications/hearth-mods-nxm.desktop <<'DESKTOP'
[Desktop Entry]
Name=Mods (Nexus Mods downloads)
Comment=Downloads a mod from Nexus Mods' "Mod Manager Download" button
Exec=/usr/bin/hearth-mods --nxm %u
Icon=hearth-mods
Type=Application
NoDisplay=true
MimeType=x-scheme-handler/nxm;
DESKTOP
  install -d /usr/lib/systemd/user/default.target.wants
  cat > /usr/lib/systemd/user/hearth-mods-remount.service <<'UNIT'
[Unit]
Description=Hearth Mods: lay overlay mods over their games again

[Service]
Type=oneshot
ExecStart=/usr/bin/hearth-mods --remount

[Install]
WantedBy=default.target
UNIT
  ln -sf ../hearth-mods-remount.service /usr/lib/systemd/user/default.target.wants/hearth-mods-remount.service
fi
# Wabbajack modlists: jackify-engine (Jackify's native Linux build of wabbajack-cli; GPL-3.0, source:
# https://github.com/Omni-guides/dev-jackify-engine), the newest Linux release, in /usr/lib/hearth/jackify-engine
# (hearth-gaming wabbajack.rs). Without it, Hearth Mods downloads it on first use.
if [ "$(uname -m)" = x86_64 ]; then
  jurl="$(curl -fsSL --retry 3 https://api.github.com/repos/Omni-guides/dev-jackify-engine/releases/latest \
    | grep -oE '"browser_download_url": *"[^"]*linux[^"]*x64[^"]*\.tar\.gz"' | head -n1 | sed -E 's/.*"(https[^"]+)"/\1/')" || true
  jtag="$(curl -fsSL --retry 3 https://api.github.com/repos/Omni-guides/dev-jackify-engine/releases/latest | grep -oE '"tag_name": *"[^"]+"' | head -n1 | sed -E 's/.*"([^"]+)"$/\1/')" || true
  if [ -n "$jurl" ] && curl -fsSL --retry 3 -o /tmp/jackify-engine.tar.gz "$jurl"; then
    rm -rf /tmp/jackify-engine && mkdir -p /tmp/jackify-engine && tar -xzf /tmp/jackify-engine.tar.gz -C /tmp/jackify-engine
    jbin="$(find /tmp/jackify-engine -type f -name jackify-engine | head -n1)"
    if [ -n "$jbin" ]; then
      rm -rf /usr/lib/hearth/jackify-engine && mkdir -p /usr/lib/hearth
      cp -a "$(dirname "$jbin")" /usr/lib/hearth/jackify-engine
      chmod 755 /usr/lib/hearth/jackify-engine/jackify-engine
      printf '%s\n' "$jtag" > /usr/lib/hearth/jackify-engine/hearth-version.txt
      printf 'jackify-engine %s, GPL-3.0 (based on Wabbajack, GPL-3.0).\nSource: https://github.com/Omni-guides/dev-jackify-engine\n' "$jtag" > /usr/lib/hearth/jackify-engine/SOURCE.txt
    else echo "warning: the jackify-engine download has no jackify-engine in it; Hearth Mods downloads it on first use"; fi
    rm -rf /tmp/jackify-engine /tmp/jackify-engine.tar.gz
  else echo "warning: couldn't download jackify-engine; Hearth Mods downloads it on first use"; fi
fi
# ---- end of Hearth's gaming apps

# ---- Nearby Share and Hearth Connect (hosts/hearth-nearby, hosts/hearth-connect) ------------------------------
# Hearth's apps for phones and nearby devices, built with the app (base-anatase.yml) and carried in os/nimbus-app
# (their icons, hearth-nearby and hearth-connect, come with Hearth's icon set). Nearby Share speaks LocalSend's protocol, Hearth Connect KDE Connect's. playerctl lets a phone
# control this PC's music, wl-clipboard/wtype give Hearth Connect the clipboard and typing in Desktop Mode, and
# notify-send's library its notifications. Their firewall helper and polkit rule come from os/system_files
# (hearth-link-firewall, 49-hearth-link.rules): ports open only while the apps listen, and only to local addresses.
dnf5 -y install --skip-unavailable --skip-broken playerctl wl-clipboard wtype libnotify || true
for app in hearth-nearby hearth-connect; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
  fi
done
chmod 755 /usr/libexec/nimbus/hearth-link-firewall 2>/dev/null || true
# Hearth Storage's root helper (Windows sharing, waking from the network; 49-hearth-storage.rules).
chmod 755 /usr/libexec/nimbus/hearth-storage-helper 2>/dev/null || true
rpm -q firewalld >/dev/null 2>&1 && echo "firewalld is in this image: Nearby Share and Hearth Connect open their ports with it" || echo "no firewalld in this image: nothing to open"
install -d /usr/share/applications /usr/lib/systemd/user
if [ -x /usr/bin/hearth-nearby ]; then
  cat > /usr/share/applications/hearth-nearby.desktop <<DESKTOP
[Desktop Entry]
Name=Nearby Share
GenericName=File Sharing
Comment=Send and receive files with phones and computers on the same network (works with LocalSend)
Exec=/usr/bin/hearth-nearby %F
Icon=hearth-nearby
Type=Application
Categories=Network;FileTransfer;Utility;
Keywords=localsend;share;send;receive;nearby;airdrop;transfer;phone;files;
DESKTOP
fi
if [ -x /usr/bin/hearth-connect ]; then
  cat > /usr/share/applications/hearth-connect.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Connect
GenericName=Phone Link
Comment=Link your phone: files, notifications, clipboard, phone as touchpad, music, find my phone (works with KDE Connect)
Exec=/usr/bin/hearth-connect
Icon=hearth-connect
Type=Application
Categories=Network;Utility;
Keywords=kde connect;kdeconnect;phone;android;iphone;notifications;clipboard;remote;touchpad;
DESKTOP
  # The background service that keeps the phone linked. Off until the person opens Hearth Connect (which turns it on
  # for their account), so nothing listens on the network before they ask for it.
  cat > /usr/lib/systemd/user/hearth-connect.service <<UNIT
[Unit]
Description=Hearth Connect (your phone, linked; KDE Connect's protocol)

[Service]
ExecStart=/usr/bin/hearth-connect --daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
UNIT
fi
# ---- end of Nearby Share and Hearth Connect

# ---- Hearth Link (hosts/hearth-link): your Hearth devices paired, Hearth data kept in sync, games copied between them.
# Built with the app (base-anatase.yml) and carried in os/nimbus-app. A small user service, started with every session
# (default.target): it answers only paired devices (and pairing while a code is showing), stays quiet while a game
# runs (Hearth's hearth-playing file), and gives way to anything else under load (low CPU/IO weight). Its UDP ports
# open to the local network only, through hearth-link-firewall.
if [ -x "/ctx/nimbus-app/hearth-link" ]; then
  install -Dm755 /ctx/nimbus-app/hearth-link /usr/bin/hearth-link
  if ldd /usr/bin/hearth-link | grep "not found"; then echo "warning: hearth-link is missing the libraries above"; rm -f /usr/bin/hearth-link; fi
fi
if [ -x /usr/bin/hearth-link ]; then
  install -d /usr/lib/systemd/user/default.target.wants
  cat > /usr/lib/systemd/user/hearth-link.service <<UNIT
[Unit]
Description=Hearth Link (your Hearth devices: sync and game copies)
After=network-online.target

[Service]
ExecStart=/usr/bin/hearth-link serve
Restart=on-failure
RestartSec=3
Nice=10
CPUWeight=20
IOWeight=20
IOSchedulingClass=best-effort
IOSchedulingPriority=7

[Install]
WantedBy=default.target
UNIT
  ln -sf ../hearth-link.service /usr/lib/systemd/user/default.target.wants/hearth-link.service
fi
# ---- end of Hearth Link

# ==== Hearth Paint, the emoji picker, Hearth Fonts, Hearth Theater, and Settings › Accessibility (hosts/hearth-paint,
# hearth-emoji, hearth-fonts, hearth-gamma, hearth-theater, desktop/shell/hearth-access): built by CI next to the Hearth app and carried in
# os/nimbus-app (their icons are in the Hearth icon set, installed above). The emoji picker types with wtype and copies with wl-copy; Fonts uses fontconfig;
# Settings › Accessibility uses Orca (the screen reader, with speech-dispatcher and eSpeak NG) and xprop (gamescope's
# zoom and colour filters in Game Mode). Each app is left out if it didn't build or can't find its libraries. Colour
# emoji (Noto) for the picker and the on-screen keyboard's emoji (Desktop Mode draws them with the system's fonts).
for p in orca speech-dispatcher espeak-ng fontconfig wtype wl-clipboard xprop google-noto-color-emoji-fonts; do dnf5 -y install "$p" || echo "warning: $p isn't available"; done
for app in hearth-paint hearth-emoji hearth-fonts hearth-gamma hearth-theater; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so it's left out"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
done
install -d /usr/share/applications
if [ -x /usr/bin/hearth-paint ]; then
  cat > /usr/share/applications/hearth-paint.desktop <<DESKTOP
[Desktop Entry]
Name=Paint
GenericName=Drawing
Comment=Draw, and mark up pictures and screenshots
Exec=/usr/bin/hearth-paint %f
Icon=hearth-paint
Type=Application
Categories=Graphics;2DGraphics;RasterGraphics;
MimeType=image/png;image/jpeg;image/webp;image/bmp;image/gif;
Keywords=paint;draw;drawing;sketch;annotate;markup;screenshot;picture;image;crop;
StartupWMClass=hearth-paint
DESKTOP
fi
if [ -x /usr/bin/hearth-emoji ]; then
  cat > /usr/share/applications/hearth-emoji.desktop <<DESKTOP
[Desktop Entry]
Name=Emoji & Symbols
Comment=Type emoji, symbols and special characters into any app (Super+.)
Exec=/usr/bin/hearth-emoji
Icon=hearth-emoji
Type=Application
Categories=Utility;
Keywords=emoji;emoticon;smiley;symbol;character;special;accent;kaomoji;unicode;
StartupWMClass=hearth-emoji
DESKTOP
fi
if [ -x /usr/bin/hearth-fonts ]; then
  cat > /usr/share/applications/hearth-fonts.desktop <<DESKTOP
[Desktop Entry]
Name=Fonts
GenericName=Font Viewer
Comment=See your fonts, and install or remove fonts
Exec=/usr/bin/hearth-fonts %f
Icon=hearth-fonts
Type=Application
Categories=Utility;
MimeType=font/ttf;font/otf;font/collection;font/sfnt;application/x-font-ttf;application/x-font-otf;application/vnd.ms-opentype;
Keywords=font;fonts;typeface;ttf;otf;install;
StartupWMClass=hearth-fonts
DESKTOP
  # Font files open in it (Hearth Files too): its MimeType, made the default with the others (Hearth's default apps, below).
fi
if [ -x /usr/bin/hearth-theater ]; then
  cat > /usr/share/applications/hearth-theater.desktop <<DESKTOP
[Desktop Entry]
Name=Theater
GenericName=Video Sites
Comment=Your own wall of video sites, full screen and controller-ready
Exec=/usr/bin/hearth-theater
Icon=hearth-theater
Type=Application
Categories=AudioVideo;Video;
Keywords=video;tv;movies;anime;shows;watch;streaming;theater;
StartupWMClass=hearth-theater
DESKTOP
fi
cat > /usr/share/applications/hearth-accessibility.desktop <<DESKTOP
[Desktop Entry]
Name=Accessibility
Comment=Magnifier, colour filters, screen reader, bigger text
Exec=/usr/bin/hearth-settings accessibility
Icon=hearth-accessibility
Type=Application
Categories=Settings;Accessibility;
Keywords=accessibility;a11y;magnifier;zoom;screen reader;orca;colour;color;filter;contrast;text size;
DESKTOP
update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
# ==== end Hearth Paint, Emoji, Fonts and Accessibility

# ---- Hearth Tools (hosts/hearth-tools): Disk Usage, System Info, Logs, Network Tools, Passwords & Keys, Printers &
# Scanner and Welcome & Tips in one program (/usr/bin/hearth-tools --app NAME), built with the app (base-anatase.yml)
# and carried in os/nimbus-app/hearth-tools/ with its menu entries (hearth-*.desktop; their icons come with Hearth's icon set). What the
# tools run: SANE's scanimage and sane-airscan (network scanners), traceroute and tracepath, iw (Wi-Fi link details).
dnf5 -y install --skip-unavailable --skip-broken sane-backends sane-backends-drivers-scanners sane-airscan traceroute iputils iw || true
if [ -x /ctx/nimbus-app/hearth-tools/hearth-tools ]; then
  install -Dm755 /ctx/nimbus-app/hearth-tools/hearth-tools /usr/bin/hearth-tools
  if ldd /usr/bin/hearth-tools | grep "not found"; then echo "warning: Hearth Tools is missing the libraries above"; fi
  install -d /usr/share/applications
  install -m644 /ctx/nimbus-app/hearth-tools/hearth-*.desktop /usr/share/applications/
fi
# ---- end Hearth Tools
# ---- Hearth Benchmark (Hearth Tools' --app benchmark): the programs its tests run. 7-Zip (processor), fio (storage;
# only its libaio engine, without the weak dependencies that pull in Ceph and RDMA), glmark2 (OpenGL and the
# game-style scenes) and vkmark (Vulkan). Each is optional: a missing one is a warning here and a skipped test in the
# app, never a failed image.
dnf5 -y install --skip-unavailable --skip-broken --setopt=install_weak_deps=False 7zip fio fio-engine-libaio glmark2 vkmark \
  || echo "warning: Hearth Benchmark: some benchmark programs couldn't be installed"
command -v 7z >/dev/null || command -v 7zz >/dev/null || command -v 7za >/dev/null \
  || dnf5 -y install --skip-unavailable --skip-broken --setopt=install_weak_deps=False p7zip p7zip-plugins || true
for p in fio glmark2 vkmark; do command -v "$p" >/dev/null || echo "warning: Hearth Benchmark: $p isn't installed (its test will be skipped)"; done
command -v 7z >/dev/null || command -v 7zz >/dev/null || command -v 7za >/dev/null || echo "warning: Hearth Benchmark: 7-Zip isn't installed (the processor tests will be skipped)"
# ---- end Hearth Benchmark

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
# On the login screen: only "Hearth" (Game Mode, and Desktop Mode = KDE Plasma from inside it). Hearth Classic, Hearth
# Plasma, Hearth GNOME and Hearth COSMIC entries aren't here. KDE's own "Plasma" entry is moved aside so the login
# screen can't start Plasma without Hearth's session (Back to Game Mode and the fallback need it).
rm -f /usr/share/wayland-sessions/hearth-classic.desktop /usr/share/wayland-sessions/hearth-plasma.desktop \
  /usr/share/wayland-sessions/hearth-gnome.desktop /usr/share/wayland-sessions/hearth-cosmic.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
# Hearth's helpers in desktop/shell (Settings' helpers, Files' backends, Discord, Proton...) are used by Hearth's apps
# outside Mantel too; Mantel itself (Quickshell) isn't installed here, so nothing runs its QML.
chmod 755 /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh
find /usr/share/nimbus/desktop/shell -maxdepth 1 -type f -name 'hearth-*' -exec chmod 755 {} +
chmod 755 /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper /usr/bin/hearth-browser /usr/bin/hearth-send-logs /usr/bin/hearth-install-package
for f in /usr/bin/hearth-winapps /usr/bin/hearth-proton-run /usr/bin/hearth-screenshot /usr/share/nimbus/desktop/shell/hearth_proton_run.py; do [ -e "$f" ] && chmod 755 "$f"; done
mkdir -p /usr/share/sounds && ln -sfn /usr/share/nimbus/desktop/sounds/hearth /usr/share/sounds/hearth || true
# Desktop Mode is KDE Plasma (nimbus-session run_plasma reads this file; the Debian image has none).
test "$(cat /usr/share/nimbus/desktop-mode)" = plasma
if [ -f /usr/share/nimbus/desktop/shell/hearth_accounts.py ]; then
  chmod 755 /usr/share/nimbus/desktop/shell/hearth_accounts.py
  ln -sfn /usr/share/nimbus/desktop/shell/hearth_accounts.py /usr/bin/hearth-accounts
fi
# Hearth OS's player count (anonymous; off in Settings › About › Privacy) and Hearth Files' "Connect automatically",
# user timers as on Debian (nothing waits for them).
install -d /usr/lib/systemd/user/timers.target.wants
cat > /usr/lib/systemd/user/hearth-counter.service <<'UNIT'
[Unit]
Description=Hearth OS player count (anonymous; off in Settings › About › Privacy)

[Service]
Type=oneshot
ExecStart=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-counter ping
TimeoutStartSec=30
UNIT
cat > /usr/lib/systemd/user/hearth-counter.timer <<'UNIT'
[Unit]
Description=Hearth OS player count, every 10 minutes while you're signed in

[Timer]
OnActiveSec=20s
OnUnitActiveSec=10min
AccuracySec=30s

[Install]
WantedBy=timers.target
UNIT
ln -sf ../hearth-counter.timer /usr/lib/systemd/user/timers.target.wants/hearth-counter.timer
cat > /usr/lib/systemd/user/hearth-files-autoconnect.service <<'UNIT'
[Unit]
Description=Hearth Files: open the shared folders that connect by themselves

[Service]
Type=oneshot
ExecStart=/usr/bin/python3 /usr/share/nimbus/desktop/shell/hearth-files net-autoconnect
TimeoutStartSec=90
Nice=10
UNIT
cat > /usr/lib/systemd/user/hearth-files-autoconnect.timer <<'UNIT'
[Unit]
Description=Hearth Files: connect shared folders by themselves while you're signed in

[Timer]
OnActiveSec=30s
OnUnitActiveSec=3min
AccuracySec=30s

[Install]
WantedBy=timers.target
UNIT
ln -sf ../hearth-files-autoconnect.timer /usr/lib/systemd/user/timers.target.wants/hearth-files-autoconnect.timer
# hearth-debbox (.deb files through Hearth's Debian box) for terminal users too.
ln -sfn /usr/share/nimbus/desktop/shell/hearth-debbox /usr/bin/hearth-debbox
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; the other editions
# write this same file).
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-os", "image-ref": "ghcr.io/dragongamer9/hearth-os", "image-tag": "test-1.15", "test": true, "desktop": "plasma", "base": "anatase", "base-detail": "Anatase (Fedora ${FEDORA})", "base-image": "${BASE_IMAGE:-i.anatase.org/anatase:stable}", "version": "${NIMBUS_VERSION:-0.0.0}"}
INFO

# The Hearth app itself (the archived Game Mode in a window): hidden from menus as on Debian; Hearth Settings opens it at
# its Settings page.
cat > /usr/share/applications/nimbus.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth
Comment=Controller-first game library
Exec=/usr/bin/nimbus --windowed
Icon=hearth
Type=Application
Categories=Game;
NoDisplay=true
DESKTOP

# ------------------------------------------------------------------ Desktop Mode: stock KDE Plasma (1.15)
# The owner (Nov 17): "Just Have KDE plasma on it. with our apps." Plasma as Fedora's KDE edition has it, recommends
# included, so KWin's Overview and effects, Breeze and Aurorae window frames, the lock screen (kscreenlocker), the
# network, sound, Bluetooth and power applets, KDE's display settings and Discover's backends are all there. No labwc,
# no Quickshell, no Mantel. Plasma never takes the login screen: Anatase's own stays, with Hearth's autologin into
# "Hearth" (Game Mode), and Desktop Mode starts Plasma from inside Hearth's session (nimbus-session run_plasma).
plasma_before=$(du -sxm /usr | cut -f1)
dm_before=$(readlink /etc/systemd/system/display-manager.service 2>/dev/null || true)
dnf5 -y install plasma-workspace plasma-desktop kwin
dnf5 -y install --skip-unavailable plasma-workspace-wayland || true
dnf5 -y install --skip-unavailable --skip-broken plasma-nm plasma-pa plasma-systemmonitor powerdevil kscreen libkscreen kscreenlocker \
  bluedevil polkit-kde xdg-desktop-portal-kde plasma-breeze breeze-icon-theme breeze-cursor-theme kde-cli-tools \
  plasma-systemsettings kinfocenter kf6-kirigami kwin-effects kdecoration aurorae plasma-milou milou kmenuedit \
  khotkeys kglobalacceld plasma-integration plasma-discover-flatpak kwalletmanager5 kwallet-pam ksshaskpass \
  dolphin konsole kate okular spectacle ark gwenview kcalc kdialog kde-gtk-config plasma-thunderbolt \
  xwaylandvideobridge wl-clipboard iio-sensor-proxy || true
# Fedora's KDE comps group, if this Anatase has it, for anything the list above misses (never fails the image).
dnf5 -y group install --skip-unavailable --skip-broken kde-desktop >/tmp/kde-group.log 2>&1 || echo "note: the kde-desktop group didn't install as a whole ($(tail -n1 /tmp/kde-group.log | cut -c1-200))"
# Not here: Plasma's first-run welcome (Hearth has its own first-time setup) and KDE Connect, whose daemon would take
# the port Hearth Connect (KDE Connect's protocol, Hearth's app) listens on.
for p in plasma-welcome kde-connect kdeconnectd kde-connect-libs; do rpm -q "$p" >/dev/null 2>&1 && dnf5 -y remove "$p" || true; done
rm -f /etc/xdg/autostart/org.kde.plasma-welcome.desktop /etc/xdg/autostart/org.kde.kdeconnect.daemon.desktop
for b in startplasma-wayland kwin_wayland plasmashell kscreen-doctor; do command -v "$b" >/dev/null || { echo "::error title=KDE Plasma::$b is missing"; exit 1; }; done
# Plasma's menu needs a menu file (KService's applications.menu, or Fedora's prefixed one; nimbus-session tells
# Plasma which with XDG_MENU_PREFIX): without one the menu is empty.
ls /etc/xdg/menus/applications.menu /etc/xdg/menus/*-applications.menu 2>/dev/null | grep -q . \
  || { echo "::error title=KDE Plasma::no applications.menu in /etc/xdg/menus: Plasma's menu would be empty"; exit 1; }
echo "Plasma's menu file: $(ls /etc/xdg/menus/*applications.menu | tr '\n' ' ')"
# The lock screen's greeter must be there (a lock that can't show its screen would be a lockout).
rpm -q kscreenlocker >/dev/null 2>&1 || dnf5 -y install kscreenlocker
greet=$(find /usr/libexec /usr/lib64 /usr/lib /usr/bin -name kscreenlocker_greet -type f 2>/dev/null | head -1)
[ -n "$greet" ] || { echo "::error title=KDE Plasma::kscreenlocker_greet is missing ($(rpm -q kscreenlocker; rpm -ql kscreenlocker 2>/dev/null | grep -i greet | head -3 | tr '\n' ' ')): the lock screen couldn't show"; exit 1; }
echo "::notice title=KDE Plasma lock screen::$greet ($(rpm -q kscreenlocker))"
echo "::notice title=KDE Plasma packages::$(rpm -q plasma-nm plasma-pa powerdevil kscreen bluedevil polkit-kde xdg-desktop-portal-kde kwin plasma-workspace 2>&1 | tr '\n' ' ' | cut -c1-900)"
# KDE's own "Plasma" login entries start Plasma without Hearth's session: moved aside (Hearth's session starts it).
mkdir -p /usr/share/hearth/desktop-sessions
for f in /usr/share/wayland-sessions/plasma.desktop /usr/share/wayland-sessions/plasmawayland.desktop /usr/share/xsessions/plasma.desktop /usr/share/xsessions/plasmax11.desktop; do
  if [ -e "$f" ]; then mv "$f" "/usr/share/hearth/desktop-sessions/$(basename "$(dirname "$f")")-$(basename "$f")"; fi
done
dm_after=$(readlink /etc/systemd/system/display-manager.service 2>/dev/null || true)
if [ "$dm_before" != "$dm_after" ]; then
  echo "Plasma's packages changed the display manager ($dm_before -> $dm_after): put back"
  if [ -n "$dm_before" ]; then ln -sfn "$dm_before" /etc/systemd/system/display-manager.service; else rm -f /etc/systemd/system/display-manager.service; fi
fi
echo "::notice title=KDE Plasma::installed ($(rpm -q --qf '%{VERSION}' plasma-workspace)); it adds about $(( $(du -sxm /usr | cut -f1) - plasma_before )) MB"
# ---- Firehold on Plasma (a test; docs/FIREHOLD.md "Firehold on Plasma"): Firehold's Couch as a Plasma shell package
# (the way Plasma Bigscreen and Plasma Mobile are made) and its Desk as Plasma's own desktop in Firehold's layout, on
# KDE's own KWin through its public interfaces only (no KWin patches, nothing from archive/kwin). Opt-in only
# ("Firehold (Plasma, test)" in Plasma's menu; no line in session.conf is plain KDE Plasma); never fails the image.
dnf5 -y install --skip-unavailable layer-shell-qt kpipewire python3-evdev >/dev/null 2>&1 || true
bash /usr/share/nimbus/desktop/firehold-plasma/install.sh / || echo "::warning title=Firehold on Plasma::not installed"
for f in /usr/share/plasma/shells/org.hearthos.firehold/metadata.json /usr/share/plasma/plasmoids/org.hearthos.firehold.couch/contents/ui/firehold.js \
  /usr/share/plasma/shells/org.hearthos.firehold.desk/contents/layout.js /usr/bin/hearth-firehold-plasma; do
  [ -e "$f" ] || echo "::warning title=Firehold on Plasma::$f is missing (Desktop Mode stays plain KDE Plasma)"
done
# Tools Hearth's apps and helpers use (screenshots, brightness, the clipboard, X11 helpers for Game Mode's pickers).
dnf5 -y install --skip-unavailable --skip-broken grim slurp brightnessctl wlr-randr python3-evdev xprop xwininfo xdotool gnome-keyring cpio rsync || true
# Couch Co-op (Beta Experimental), Move games / Make Hearth bigger, Proton for games added by hand: not fatal,
# Hearth says what's missing.
dnf5 -y install 7zip || dnf5 -y install p7zip p7zip-plugins || true
# bsdtar: Hearth Files opens .rar, .iso, .cbr and the rest of the archive kinds 7-Zip doesn't.
dnf5 -y install bsdtar || true
dnf5 -y install fuse-overlayfs bubblewrap xdpyinfo xrandr || true
# Send logs decodes the screen's EDID (what the TV says it can do: HDMI 2.1's fast link, its fastest 4K rates, VRR)
# when a screen shows fewer modes than expected. Not fatal.
dnf5 -y install edid-decode || dnf5 -y install libdisplay-info-tools || true
dnf5 -y install ntfsprogs ntfs-3g e2fsprogs btrfs-progs util-linux || true
dnf5 -y install umu-launcher || true
# Claude Desktop (Hearth Settings › Claude: Anthropic's package, unpacked in the user's home folder by
# desktop/shell/hearth-claude, never shipped in this image): gpg checks Anthropic's signature; the app links
# GTK 3, NSS, libdrm/gbm, libxtst, PipeWire and AT-SPI (above), and opens libsecret (the sign-in kept in the
# password store), libnotify (notifications) and the tray library while it runs.
dnf5 -y install gnupg2
for p in libsecret libnotify libxcb at-spi2-core libuuid xdg-utils libayatana-appindicator-gtk3 pipewire-libs libdrm mesa-libgbm; do
  dnf5 -y install "$p" || echo "warning: $p isn't available"
done
# Printing (Settings › Printers: CUPS with driverless IPP Everywhere, USB printers through ipp-usb, network printers
# found with Avahi), clipboard history (Super+V: wl-clipboard, cliphist; wtype pastes) and Flatpak for Apps.
# Desktop Mode also: choosing an area for a screenshot (slurp), screenshot notifications (notify-send) and turning
# the screen off when idle (wlopm, Settings › Power).
for p in cups cups-filters ipp-usb avahi nss-mdns wl-clipboard cliphist wtype flatpak slurp libnotify wlopm; do
  dnf5 -y install "$p" || echo "warning: $p isn't available"
done
systemctl enable cups.socket || true
systemctl enable avahi-daemon.socket 2>/dev/null || true
# .deb files (Hearth's Debian box: hearth-debbox and the Install Package window put them in a Debian 13 container
# made with Distrobox; podman is in Anatase) and AppImages (FUSE 2's library, which most AppImages' runtime loads,
# and fusermount).
dnf5 -y install distrobox
dnf5 -y install fuse-libs || dnf5 -y install fuse2-libs || echo "warning: FUSE 2's library isn't available; AppImages run unpacked instead (slower)"
dnf5 -y install fuse || true
if ! command -v fusermount >/dev/null && command -v fusermount3 >/dev/null; then ln -s "$(command -v fusermount3)" /usr/bin/fusermount; fi
touch /etc/subuid /etc/subgid
command -v distrobox
# Flathub for the system installation (hearth-flatpak-setup adds it from here; Bazaar, the app store, installs from it),
# unless Anatase already brings a Flathub remote file.
if ! grep -qls 'dl.flathub.org' /etc/flatpak/remotes.d/*.flatpakrepo /usr/share/flatpak/remotes.d/*.flatpakrepo 2>/dev/null; then
  mkdir -p /etc/flatpak/remotes.d
  curl -fsSL --retry 3 -o /etc/flatpak/remotes.d/flathub.flatpakrepo https://dl.flathub.org/repo/flathub.flatpakrepo || echo "warning: couldn't fetch Flathub's remote file"
fi
# Bazaar, the Flathub store (io.github.kolunmi.Bazaar): Fedora's package when this release has it (only kept if it
# really is that app: its .desktop id); otherwise hearth-flatpak-setup installs Flathub's at the first online boot.
if dnf5 -y install bazaar >/tmp/bazaar.log 2>&1 && [ -f /usr/share/applications/io.github.kolunmi.Bazaar.desktop ]; then
  echo "Bazaar installed from Fedora"
else
  rpm -q bazaar >/dev/null 2>&1 && dnf5 -y remove bazaar || true
  echo "Bazaar isn't packaged for Fedora $FEDORA here ($(tail -n 1 /tmp/bazaar.log | cut -c1-160)): Flathub's is installed at first boot"
fi
# The move's first start (hearth-clean-install) reads Lutris' database with Python's sqlite3, and the controller
# on the login screen needs evdev (above).
python3 -c 'import sqlite3, json'

# ------------------------------------------------------------------ Steam
# As on the Bazzite-based Hearth OS: Steam installed in the system, its games in ~/.local/share/Steam. Anatase's
# own Steam isn't relied on (Hearth doesn't know its folders). From RPM Fusion when Anatase doesn't have it: its
# nonfree repository (on once rpmfusion-nonfree-release is in). There's no "rpmfusion-nonfree-steam" repository in
# those release packages (that one comes with Fedora Workstation's third-party repos), so asking for it failed the
# install every time and Hearth OS shipped without Steam.
if ! rpm -q steam >/dev/null 2>&1; then
  if { dnf5 -y install "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${FEDORA}.noarch.rpm" \
                       "https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${FEDORA}.noarch.rpm" \
       && dnf5 -y install steam; } > /tmp/steam-install.log 2>&1; then
    echo "Steam installed from RPM Fusion"
  else
    tail -n 15 /tmp/steam-install.log 2>/dev/null || true
    # Then Flathub's Steam at the first start (hearth-steam.service; Hearth knows its folders too). Anatase's own
    # /usr/bin/steam only starts its own Steam Flatpak (org.anatase.Steam), which isn't installed: Hearth's launcher
    # takes its place and starts whichever Steam there is.
    echo "warning: couldn't install Steam from RPM Fusion ($(tail -n 1 /tmp/steam-install.log 2>/dev/null | cut -c1-200)); Flathub's Steam is installed at first boot instead"
    if [ -e /usr/bin/steam ]; then mkdir -p /usr/libexec/nimbus && mv /usr/bin/steam /usr/libexec/nimbus/steam-anatase; fi
    install -Dm755 /ctx/anatase_files/usr/libexec/nimbus/hearth-steam-launch /usr/bin/steam
    mkdir -p /usr/share/nimbus && touch /usr/share/nimbus/steam-flatpak
    systemctl enable hearth-steam.service
  fi
  # The repositories aren't left on for the installed system (the image brings updates).
  for r in /etc/yum.repos.d/rpmfusion-*.repo; do
    if [ -f "$r" ]; then sed -i 's/^enabled=1/enabled=0/' "$r"; fi
  done
fi

# ------------------------------------------------------------------ Beacon (BFME, Age of the Ring, LOTR: Conquest...)
# Beacon's release package: `beacon`, the tool Hearth's library talks to (hosts/hearth-core/src/beacon.rs), and
# `beacon-app`, its window (beacon.desktop: in Mantel's launcher and pinned to its taskbar). See docs/BEACON.md.
# Optional: when there's no release to download yet, the build says so and carries on.
BEACON_URL=https://github.com/Dragongamer9/beacon/releases/latest/download
if [ "$(uname -m)" = x86_64 ] && dnf5 -y install "$BEACON_URL/beacon.x86_64.rpm"; then
  echo "Beacon installed"
else
  echo "warning: couldn't install Beacon from $BEACON_URL (no release yet?); Hearth works without it"
fi

# ------------------------------------------------------------------ Xodus Nexus (Xbox PC and PC Game Pass games)
# Shipped like Beacon: the AppImage in /usr/lib/xodus-nexus (with its version), `xodus-nexus` to start it (the same
# script hosts/hearth-core/src/xodus.rs writes for a copy it downloads) and xodus-nexus.desktop (in Mantel's launcher
# and pinned to its taskbar). Hearth's library shows it under Xbox PC; a newer release is offered in Settings and goes
# in the user's own data folder, since the image is read-only. Nexus is an independent project built on Xodus
# (github.com/domsvoboda/xodus-nexus), not made by Microsoft. It publishes only an AppImage, so its download address
# comes from the latest release's asset list. Optional: without a release to download, the build says so and carries on.
XODUS_API=https://api.github.com/repos/domsvoboda/xodus-nexus/releases/latest
xodus_json="$(curl -fsSL --retry 3 "$XODUS_API" 2>/dev/null || true)"
XODUS_URL="$(printf '%s' "$xodus_json" | python3 -c 'import json, sys; a=[x for x in json.load(sys.stdin).get("assets", []) if x["name"].lower().endswith(".appimage") and ("x86_64" in x["name"].lower() or "amd64" in x["name"].lower())]; print(a[0]["browser_download_url"] if a else "")' 2>/dev/null || true)"
XODUS_VER="$(printf '%s' "$xodus_json" | python3 -c 'import json, sys; print(json.load(sys.stdin).get("tag_name", "").lstrip("v"))' 2>/dev/null || true)"
if [ "$(uname -m)" = x86_64 ] && [ -n "$XODUS_URL" ] && install -d /usr/lib/xodus-nexus \
   && curl -fsSL --retry 3 -o /usr/lib/xodus-nexus/Xodus-Nexus.AppImage "$XODUS_URL"; then
  chmod 755 /usr/lib/xodus-nexus/Xodus-Nexus.AppImage
  printf '%s\n' "$XODUS_VER" > /usr/lib/xodus-nexus/version
  cat > /usr/bin/xodus-nexus <<'EOF'
#!/bin/sh
# Starts Xodus Nexus (shipped with Hearth OS; Hearth's library runs this).
# AppImages need FUSE; without it, unpack and run instead.
if ! command -v fusermount >/dev/null 2>&1 && ! command -v fusermount3 >/dev/null 2>&1; then export APPIMAGE_EXTRACT_AND_RUN=1; fi
# WebKit's newer GPU path draws a black window on some Nvidia drivers.
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"
# Sign-in pages open in your default browser (Hearth Browser unless you picked another), not whichever it finds first.
export BROWSER="${BROWSER:-xdg-open}"
exec /usr/lib/xodus-nexus/Xodus-Nexus.AppImage "$@"
EOF
  chmod 755 /usr/bin/xodus-nexus
  cat > /usr/share/applications/xodus-nexus.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=Xodus Nexus
GenericName=Xbox PC games
Comment=Xbox PC and PC Game Pass games (an independent launcher, not made by Microsoft)
Exec=xodus-nexus %U
Icon=applications-games
Terminal=false
Categories=Game;
EOF
  echo "Xodus Nexus ${XODUS_VER:-?} installed"
else
  rm -rf /usr/lib/xodus-nexus
  echo "warning: couldn't install Xodus Nexus from $XODUS_API (no release with an AppImage?); Hearth works without it"
fi

# ------------------------------------------------------------------ Faugus Launcher (Battle.net and other Windows launchers)
# Library › Stores › Battle.net sets Battle.net up through Faugus Launcher (hosts/hearth-core/src/battlenet.rs), which
# runs Windows launchers with umu-launcher and Proton (github.com/Faugus/faugus-launcher). Faugus's own COPR, the way
# its README installs it on Fedora. Optional: without it, Hearth sets Battle.net up with its own Proton setup.
if ! rpm -q faugus-launcher >/dev/null 2>&1; then
  if dnf5 -y copr enable faugus/faugus-launcher && dnf5 -y install faugus-launcher; then
    echo "Faugus Launcher installed (COPR)"
  else
    rpm -q faugus-launcher >/dev/null 2>&1 && dnf5 -y remove faugus-launcher || true
    echo "warning: couldn't install Faugus Launcher (COPR faugus/faugus-launcher); Battle.net uses Hearth's own setup instead"
  fi
  dnf5 -y copr disable faugus/faugus-launcher >/dev/null 2>&1 || true
fi

# ------------------------------------------------------------------ Sunshine (Multi-seat's streamed players, Remote play)
# Settings › Multi-seat can stream an extra player's own Game Mode to Moonlight on a TV, phone, tablet or laptop (one
# Sunshine per streamed player, see docs/archive/MULTI-SEAT.md); Settings › Remote play uses it too. LizardByte's COPR
# (the package is "Sunshine"), else the release's Fedora RPM. Optional: without it Hearth says streaming isn't available.
if ! rpm -q Sunshine >/dev/null 2>&1 && ! rpm -q sunshine >/dev/null 2>&1; then
  if dnf5 -y copr enable lizardbyte/stable && dnf5 -y install Sunshine; then
    echo "Sunshine installed (LizardByte COPR)"
  elif dnf5 -y copr enable lizardbyte/beta && dnf5 -y install Sunshine; then
    echo "Sunshine installed (LizardByte beta COPR)"
  else
    SUNSHINE_RPM="$(curl -fsSL --retry 3 https://api.github.com/repos/LizardByte/Sunshine/releases/latest 2>/dev/null \
      | python3 -c 'import json, re, sys; print(next((a["browser_download_url"] for a in json.load(sys.stdin).get("assets", []) if re.search(r"\.fc" + sys.argv[1] + r"\.x86_64\.rpm$", a["name"])), ""))' "$FEDORA" 2>/dev/null || true)"
    if [ "$(uname -m)" = x86_64 ] && [ -n "$SUNSHINE_RPM" ] && dnf5 -y install "$SUNSHINE_RPM"; then
      echo "Sunshine installed ($SUNSHINE_RPM)"
    else
      echo "warning: couldn't install Sunshine; streamed multi-seat players and Remote play show as unavailable"
    fi
  fi
  dnf5 -y copr disable lizardbyte/stable >/dev/null 2>&1 || true
  dnf5 -y copr disable lizardbyte/beta >/dev/null 2>&1 || true
fi
# The streamed players' address as a QR code in Settings (optional), and the udev helper that gives each streamed
# player their own virtual controllers (copied in with system_files; executable to be sure).
dnf5 -y install qrencode || echo "warning: qrencode isn't available (Settings shows the address without a QR code)"
chmod 755 /usr/libexec/nimbus/hearth-seat-input 2>/dev/null || true

# ------------------------------------------------------------------ Handheld Daemon
# Anatase has it; Hearth's drop-in (hhd.service.d/zz-hearth.conf) turns off its updater and its Game Mode autologin.
if [ -f /usr/lib/systemd/system/hhd.service ]; then
  systemctl enable hhd.service || true
else
  echo "warning: Handheld Daemon (hhd.service) isn't in this Anatase image; built-in controllers fall back to plain evdev"
fi

# ------------------------------------------------------------------ Quick Access over games: Handheld Daemon's overlay
# HHD shows an overlay app over games (its QAM button) and holds the controllers for it. Here that app is Hearth's own
# Quick Access (desktop/shell/hearth-overlay: hosts/linux/src/overlay in Hearth Browser's Electron), installed as
# hhd-ui, and HHD_OVERLAY points at it so it wins over any other hhd-ui (HHD's own, if Anatase brings it).
if rpm -q hhd-ui >/dev/null 2>&1; then rpm -e --nodeps hhd-ui || true; fi
rm -f /usr/bin/hhd-ui.AppImage /usr/bin/hhd-ui-dbg
chmod 755 /usr/share/nimbus/desktop/shell/hearth-overlay
ln -sfn /usr/share/nimbus/desktop/shell/hearth-overlay /usr/bin/hhd-ui
mkdir -p /usr/lib/systemd/system/hhd.service.d
cat > /usr/lib/systemd/system/hhd.service.d/zz-hearth-overlay.conf <<'CONF'
# Hearth: Handheld Daemon's overlay app is Hearth's Quick Access (see os-bases/*/build.sh).
[Service]
Environment=HHD_OVERLAY=/usr/share/nimbus/desktop/shell/hearth-overlay
CONF

# ------------------------------------------------------------------ power button
# Hearth handles the power button in its session (Settings › Power button), not Steam's helper.
systemctl --global disable steamos-powerbuttond.service 2>/dev/null || true
# When the device last woke up (/run/hearth-resumed) and each sleep in the journal (hearth-sleep): the press that
# woke it is then not taken as a new one (os/system_files).
chmod 755 /usr/lib/systemd/system-sleep/hearth-resumed

# ------------------------------------------------------------------ Hearth Settings and the Hearth look
cat > /usr/share/applications/hearth-files.desktop <<DESKTOP
[Desktop Entry]
Name=Files
GenericName=File Manager
Comment=Browse, copy and move your files, games, USB drives and SD cards
Exec=/usr/bin/hearth-filemanager %U
Icon=hearth-files
Type=Application
Categories=System;FileManager;Utility;
MimeType=inode/directory;x-directory/normal;application/zip;application/x-tar;application/x-compressed-tar;application/x-xz-compressed-tar;application/x-bzip2-compressed-tar;application/x-zstd-compressed-tar;application/gzip;application/x-gzip;application/x-xz;application/x-bzip2;application/zstd;application/x-7z-compressed;application/vnd.rar;application/x-rar;
Keywords=files;folder;explorer;browse;usb;sd;card;copy;move;
OnlyShowIn=Nimbus;
DESKTOP
# Folders open in Hearth Files in Desktop Mode (XDG_CURRENT_DESKTOP=Nimbus), and archives (it unpacks them): Hearth's
# default apps, below. Plasma keeps Dolphin (OnlyShowIn=Nimbus).
# ---- Hearth Text (hosts/hearth-text): Hearth OS's text editor, built by CI next to the Hearth app (base-*.yml) and
# carried in os/nimbus-app. Its menu entry (hearth-text.desktop, icon hearth-text from the Hearth icon set) comes from
# os/system_files. In Desktop
# Mode it opens text files, scripts, settings files and logs (its MimeType: Hearth's default apps, below), and Hearth
# Files opens text-like files without an app in it.
if [ -x /ctx/nimbus-app/hearth-text ]; then
  install -Dm755 /ctx/nimbus-app/hearth-text /usr/bin/hearth-text
  # An editor that can't start is worse than none: then text files open in whatever else is installed.
  if ldd /usr/bin/hearth-text | grep 'not found'; then echo "Hearth Text needs libraries this image doesn't have (above); left out"; rm -f /usr/bin/hearth-text; fi
fi
if [ -x /usr/bin/hearth-text ]; then
  command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
fi
# ---- end of Hearth Text
# (Task Manager is Mantel's, so it isn't here: KDE's System Monitor is Desktop Mode's.)
cat > /usr/share/applications/hearth-settings.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Settings
Comment=Wi-Fi, Bluetooth, sound, display, controllers, accounts and updates
Exec=/usr/bin/hearth-settings
Icon=hearth-settings
Type=Application
Categories=Settings;
Keywords=wifi;bluetooth;sound;display;controller;update;
DESKTOP
# (Hearth's own app store, Apps / hearth-apps.desktop, is archived: Bazaar is the app store, installed for everyone by
# hearth-flatpak-setup. docs/ARCHIVED.md)
cat > /usr/share/applications/hearth-browser.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth Browser
GenericName=Web Browser
Comment=Browse the web: Firefox with Hearth's settings
Exec=/usr/bin/hearth-browser %U
Icon=hearth-browser
Type=Application
Categories=Network;WebBrowser;
MimeType=text/html;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=hearth-browser
StartupNotify=true
Actions=new-window;private-window;
Keywords=web;internet;browser;firefox;

[Desktop Action new-window]
Name=New window
Exec=/usr/bin/hearth-browser

[Desktop Action private-window]
Name=New private window
Exec=/usr/bin/hearth-browser --private-window
DESKTOP
# ------------------------------------------------------------------ Hearth Browser (Firefox)
# Hearth Browser is Mozilla's Firefox, unchanged, with Hearth's settings (os/system_files/usr/share/hearth-browser):
# install-firefox puts the newest release, checked against Mozilla's signed SHA256SUMS, in
# /usr/lib/hearth-browser/firefox and adds Hearth's parts (policies, look, start page, on-screen keyboard). It updates
# with the image (Firefox's own updater is off). /usr/bin/hearth-browser starts it with its own profile. The Electron
# build in /usr/lib/hearth-browser stays: it's the Quick Access overlay app (hearth-overlay).
python3 /usr/share/hearth-browser/install-firefox /usr/lib/hearth-browser/firefox

# Firefox's own libraries come with it; the system ones it links (GTK 3, X11, D-Bus...) must be in the image. Any
# still missing are looked up by file name and installed, so a new Firefox needing one more never ships broken.
ff_missing() { LD_LIBRARY_PATH=/usr/lib/hearth-browser/firefox ldd /usr/lib/hearth-browser/firefox/libxul.so /usr/lib/hearth-browser/firefox/firefox 2>/dev/null | awk '/not found/{print $1}' | sort -u; }   # Firefox's own libraries sit next to it
dnf5 -y install --setopt=install_weak_deps=False gtk3 dbus-glib dbus-libs libX11-xcb libXcomposite libXdamage libXrandr libXtst libXt alsa-lib pango cairo-gobject || true
for l in $(ff_missing); do dnf5 -y install --setopt=install_weak_deps=False "$l()(64bit)" || true; done
[ -z "$(ff_missing)" ] || echo "warning: Hearth Browser's Firefox is still missing: $(ff_missing | tr '\n' ' ')"
chmod 755 /usr/bin/hearth-browser /usr/share/hearth-browser/install-firefox
# Links open in it in Desktop Mode and Game Mode (XDG_CURRENT_DESKTOP=Nimbus), unless you pick another browser
# (Hearth's default apps, below). .deb files open in Install Package (Hearth's Debian box) the same way: its MimeType.

# ------------------------------------------------------------------ Hearth Player and Hearth Calculator
# ---- hosts/hearth-player and hosts/hearth-calculator, built by CI with the app (base-anatase.yml) and carried in
# os/nimbus-app; their menu entries are os/system_files' hearth-player.desktop and hearth-calculator.desktop.
# The player plays through WebKitGTK (GStreamer) and edits with ffmpeg, so this image gets the codecs for both:
# RPM Fusion's full ffmpeg (H.264/H.265 decoding and libx264 encoding) and its GStreamer plugins, else Fedora's own
# (ffmpeg-free, gstreamer1-plugin-libav, openh264). What ended up in is in /usr/share/nimbus/media-codecs.txt.
if [ ! -f /etc/yum.repos.d/rpmfusion-free.repo ]; then
  dnf5 -y install "https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-${FEDORA}.noarch.rpm" \
                  "https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-${FEDORA}.noarch.rpm" || true
fi
RPMF=()
for r in rpmfusion-free rpmfusion-free-updates rpmfusion-nonfree rpmfusion-nonfree-updates; do
  if grep -qs "^\[$r\]" /etc/yum.repos.d/rpmfusion-*.repo; then RPMF+=("--enablerepo=$r"); fi
done
if [ ${#RPMF[@]} -gt 0 ]; then
  if rpm -q ffmpeg-free >/dev/null 2>&1; then dnf5 -y swap --allowerasing "${RPMF[@]}" ffmpeg-free ffmpeg || true; fi
  rpm -q ffmpeg >/dev/null 2>&1 || dnf5 -y install --allowerasing "${RPMF[@]}" ffmpeg || true
  dnf5 -y install --allowerasing --skip-unavailable "${RPMF[@]}" gstreamer1-plugins-bad-freeworld gstreamer1-plugins-ugly gstreamer1-libav libavcodec-freeworld || true
fi
dnf5 -y install --skip-unavailable --skip-broken gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugin-libav gstreamer1-plugin-openh264 gstreamer1-plugin-dav1d gstreamer1-plugins-base-tools || true
command -v ffmpeg >/dev/null || dnf5 -y install ffmpeg-free || echo "warning: no ffmpeg in this image: Hearth Player can't edit or convert"
# The RPM Fusion repositories stay off on the installed system (the image brings updates).
for r in /etc/yum.repos.d/rpmfusion-*.repo; do
  if [ -f "$r" ]; then sed -i 's/^enabled=1/enabled=0/' "$r"; fi
done
for app in hearth-player hearth-calculator; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    # An app that can't start is worse than none (its menu entry hides itself without it: TryExec).
    if ldd "/usr/bin/$app" | grep 'not found'; then echo "warning: $app needs libraries this image doesn't have (above); left out"; rm -f "/usr/bin/$app"; fi
  else
    echo "warning: $app wasn't built for this image"
  fi
done
# Videos, music and playlists open in Hearth Player: every type in hearth-player.desktop (os/system_files) is made
# its default with the other Hearth apps' (Hearth's default apps, below).
update-desktop-database /usr/share/applications 2>/dev/null || true
gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
# What plays and what edits: in the build log and in the image.
{
  echo "Hearth Player media support, $(date -u +%F)"
  echo "ffmpeg: $(ffmpeg -hide_banner -version 2>/dev/null | head -1 || echo missing)"
  echo "--- ffmpeg decoders"
  ffmpeg -hide_banner -decoders 2>/dev/null | awk '{print $2}' | grep -xE 'h264|hevc|av1|libdav1d|vp8|vp9|libvpx-vp9|mpeg2video|mpeg4|aac|mp3|mp3float|flac|opus|libopus|vorbis|ac3|eac3|dca|truehd|alac|wmav2|subrip|ass|mov_text' | tr '\n' ' ' || true
  echo; echo "--- ffmpeg encoders"
  ffmpeg -hide_banner -encoders 2>/dev/null | awk '{print $2}' | grep -xE 'libx264|libopenh264|libx265|h264_vaapi|hevc_vaapi|libsvtav1|libaom-av1|libvpx-vp9|libvpx|mpeg4|aac|libfdk_aac|libmp3lame|libopus|libvorbis|flac|pcm_s16le|wavpack|wmav2' | tr '\n' ' ' || true
  echo; echo "--- GStreamer (what WebKitGTK plays with)"
  for e in avdec_h264 avdec_h265 avdec_av1 openh264dec dav1ddec vp9dec vah264dec vah265dec avdec_aac faad avdec_mp3 mpg123audiodec flacdec opusdec vorbisdec avdec_ac3 matroskademux qtdemux webmmux; do
    if gst-inspect-1.0 --exists "$e" 2>/dev/null; then echo "$e yes"; else echo "$e no"; fi
  done | tr '\n' ' ' || true
  echo
} | tee /usr/share/nimbus/media-codecs.txt || true
# The same as a notice on the CI run (the image build's output reaches the workflow).
echo "::notice title=Hearth Player codecs::$(tr '\n' ' ' < /usr/share/nimbus/media-codecs.txt | cut -c1-3500)" || true
# ---- end of Hearth Player and Hearth Calculator


# ------------------------------------------------------------------ Hearth Photos and Hearth Documents
# (hosts/hearth-photos, hosts/hearth-documents) Hearth's picture viewer/editor and its reader for PDFs, e-books (EPUB)
# and comic books (CBZ), built by CI with the app (base-anatase.yml) and carried in os/nimbus-app (their icons,
# hearth-photos and hearth-documents, come with the Hearth icon set above).
# Their menu entries and file types are set here, and they're the default apps for pictures and documents (Hearth's
# default apps, after the last app).
# libheif's tools let Photos open HEIC pictures from phones; wl-clipboard is how it copies a picture.
dnf5 -y install --skip-unavailable --skip-broken libheif-tools wl-clipboard || true
# HEIC needs an HEVC decoder for libheif: RPM Fusion's libheif-freeworld where RPM Fusion is on.
dnf5 -y install --skip-unavailable --skip-broken libheif-freeworld 2>/dev/null || true
for app in hearth-photos hearth-documents; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
  fi
done
HEARTH_PICTURE_TYPES="image/jpeg;image/png;image/gif;image/webp;image/bmp;image/x-bmp;image/x-ms-bmp;image/tiff;image/svg+xml;image/avif;image/heic;image/heif;image/jxl;image/x-icon;image/vnd.microsoft.icon;image/apng;"
HEARTH_DOCUMENT_TYPES="application/pdf;application/x-pdf;application/epub+zip;application/vnd.comicbook+zip;application/x-cbz;"
if [ -x /usr/bin/hearth-photos ]; then
  cat > /usr/share/applications/hearth-photos.desktop <<DESKTOP
[Desktop Entry]
Name=Photos
GenericName=Picture Viewer
Comment=Look through your pictures, and crop, straighten or brighten them
Exec=/usr/bin/hearth-photos %U
TryExec=/usr/bin/hearth-photos
Icon=hearth-photos
Type=Application
Categories=Graphics;Viewer;RasterGraphics;2DGraphics;Photography;
MimeType=${HEARTH_PICTURE_TYPES}
Keywords=photo;picture;image;viewer;gallery;slideshow;crop;rotate;edit;
StartupNotify=true
StartupWMClass=hearth-photos
Actions=slideshow;

[Desktop Action slideshow]
Name=Slideshow of your Pictures
Exec=/usr/bin/hearth-photos --slideshow
DESKTOP
fi
if [ -x /usr/bin/hearth-documents ]; then
  cat > /usr/share/applications/hearth-documents.desktop <<DESKTOP
[Desktop Entry]
Name=Documents
GenericName=Document Viewer
Comment=Read PDFs, e-books and comic books
Exec=/usr/bin/hearth-documents %U
TryExec=/usr/bin/hearth-documents
Icon=hearth-documents
Type=Application
Categories=Office;Viewer;Graphics;
MimeType=${HEARTH_DOCUMENT_TYPES}
Keywords=pdf;document;viewer;reader;ebook;epub;book;comic;cbz;
StartupNotify=true
StartupWMClass=hearth-documents
DESKTOP
fi
# They're the defaults for those types with the other Hearth apps (Hearth's default apps, below).
gtk-update-icon-cache -q -f /usr/share/icons/hicolor 2>/dev/null || true
update-desktop-database -q /usr/share/applications 2>/dev/null || true
# ---- end of Hearth Photos and Hearth Documents
install -Dm644 /usr/share/nimbus/desktop/theme/Hearth.colors /usr/share/color-schemes/Hearth.colors
declare -A WALL=([fireside]="Fireside" [embers]="Embers" [ridge]="Dusk Ridge" [mark]="The Mark" [kiln]="Kiln"
  [cat]="Hearth Cat" [cabin]="Night Cabin" [lanterns]="Lanterns" [bloom]="Ember Bloom" [peaks]="Low Poly Peaks"
  [forge]="Forge Grid" [topo]="Cinder Topo" [fox]="Fox Moon")
for id in "${!WALL[@]}"; do
  install -Dm644 "/ctx/wallpapers/hearth-$id.jpg" "/usr/share/backgrounds/hearth/hearth-$id.jpg"
  dir="/usr/share/wallpapers/Hearth-$id"
  install -Dm644 "/ctx/wallpapers/hearth-$id.jpg" "$dir/contents/images/3840x2160.jpg"
  printf '{ "KPlugin": { "Id": "Hearth-%s", "Name": "Hearth %s", "Authors": [{ "Name": "Hearth" }], "License": "CC-BY-SA-4.0" } }\n' \
    "$id" "${WALL[$id]}" > "$dir/metadata.json"
done
mkdir -p /usr/share/xdg-desktop-portal
cat > /usr/share/xdg-desktop-portal/nimbus-portals.conf <<PORTALS
[preferred]
default=gtk
PORTALS
# (Anatase: Hearth's own file chooser is Mantel's QML picker, which needs Quickshell; Game Mode's file dialogs are GTK's
# and Plasma's are KDE's.)
# ---- Hearth Files: every app's file chooser, and the file manager apps open (desktop/shell/hearth-portal, picker.qml)
# Open / Save dialogs: xdg-desktop-portal's FileChooser is Hearth's in Hearth's session (above; XDG_CURRENT_DESKTOP is
# Nimbus in both modes), the rest stays on the backends it had. hearth.portal and the D-Bus service files come from
# os/system_files. nimbus-session asks GTK and Qt apps to use the portal; Hearth Browser's prefs ask Firefox.
# Folders: Hearth Files is the default in every desktop's list as well as Desktop Mode's (hearth-filemanager hands
# them to Dolphin outside Hearth's session), and "Show in folder" (org.freedesktop.FileManager1) is
# held by Hearth's session, so no other file manager in this image takes over.
# (hearth-system defaults-system writes them with Hearth's other default apps, after the last app: EVERY_DESKTOP_TOO.)
# The backend is Python with GLib's D-Bus (python3-gobject); Qt apps' portal dialogs come with qt6-qtbase-gui.
dnf5 -y install python3-gobject xdg-desktop-portal || true
chmod 755 /usr/share/nimbus/desktop/shell/hearth-portal
# Firefox from the distribution, if there is one, also uses the portal (Hearth Browser has it in its own prefs).
for d in /usr/lib64/firefox /usr/lib/firefox /usr/lib/firefox-esr; do
  if [ -d "$d" ]; then install -Dm644 /usr/share/hearth-browser/firefox/defaults/pref/hearth-portal.js "$d/defaults/pref/hearth-portal.js"; fi
done
command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
# ---- end of Hearth Files as the file chooser

# ------------------------------------------------------------------ Hearth Contacts and Hearth Maps (hosts/hearth-contacts, hosts/hearth-maps)
# Built with the app (base-anatase.yml) and carried in os/nimbus-app; their icons (Icon=hearth-contacts, hearth-maps)
# come with Hearth's icon set above.
# Contacts opens .vcf files and keeps CardDAV passwords in GNOME Keyring (Hearth's session starts it); Maps opens
# geo: links and asks GeoClue for "Where am I" only when that button is pressed (docs/HEARTH-MAPS.md).
dnf5 -y install geoclue2 libsecret || true
install -d /usr/share/applications
for app in hearth-contacts hearth-maps; do
  [ -x "/ctx/nimbus-app/$app" ] || { echo "warning: $app wasn't built, so this image goes without it"; continue; }
  install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
  if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
done
if [ -x /usr/bin/hearth-contacts ]; then
  cat > /usr/share/applications/hearth-contacts.desktop <<DESKTOP
[Desktop Entry]
Name=Contacts
GenericName=Address Book
Comment=Names, numbers, emails and addresses, kept in step with Nextcloud, iCloud or Fastmail if you like
Exec=/usr/bin/hearth-contacts %f
Icon=hearth-contacts
Type=Application
Categories=Office;ContactManagement;
MimeType=text/vcard;text/x-vcard;text/directory;
Keywords=contacts;address;book;people;phone;email;vcard;vcf;carddav;
StartupWMClass=hearth-contacts
DESKTOP
  # .vcf files open in it (Hearth's default apps, below).
fi
if [ -x /usr/bin/hearth-maps ]; then
  cat > /usr/share/applications/hearth-maps.desktop <<DESKTOP
[Desktop Entry]
Name=Maps
GenericName=Maps
Comment=OpenStreetMap maps: find places and get directions
Exec=/usr/bin/hearth-maps %u
Icon=hearth-maps
Type=Application
Categories=Utility;Maps;
MimeType=x-scheme-handler/geo;
Keywords=map;maps;openstreetmap;osm;directions;route;navigation;places;geo;location;
StartupWMClass=hearth-maps
DESKTOP
  # GeoClue may tell Hearth Maps the location without asking an agent (Hearth's session has none). Maps only asks
  # when the person presses Where am I, and GeoClue still decides where the location comes from.
  install -d /etc/geoclue/conf.d
  printf '[hearth-maps]\nallowed=true\nsystem=false\nusers=\n' > /etc/geoclue/conf.d/90-hearth-maps.conf
fi
update-desktop-database -q /usr/share/applications 2>/dev/null || true
# ---- end of Hearth Contacts and Hearth Maps

# ------------------------------------------------------------------ Discord (desktop/shell/hearth-discord, docs/DISCORD.md)
# Since 1.6 Discord replaces Hearth Friends (Matrix, archived in archive/hosts/hearth-friends: no binary and no user
# service any more; a leftover default.target.wants link from an older image points at nothing and systemd skips it, so
# the session starts the same). Nothing of Discord is installed here: the owner picks the app (Flathub), the web window
# or Dorion himself. Invite links a friend pastes in Discord (hearthos.fun/join → hearth://join/...) open Hearth's
# helper, which finds the game or starts Play together. libsecret stays (other apps use the keyring; it was here before).
dnf5 -y install --skip-unavailable --skip-broken libsecret || true
if [ -f /usr/share/nimbus/desktop/shell/hearth-discord ]; then chmod 755 /usr/share/nimbus/desktop/shell/hearth-discord; fi
cat > /usr/share/applications/hearth-join.desktop <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Hearth (game invites)
NoDisplay=true
Exec=python3 /usr/share/nimbus/desktop/shell/hearth-discord join --link %u
MimeType=x-scheme-handler/hearth;
DESKTOP
update-desktop-database -q /usr/share/applications 2>/dev/null || true
# ---- end of Discord
# ---- Hearth Office (hosts/hearth-office): Write, Sheets and Slides, built by CI with the app (base-anatase.yml) and
# carried in os/nimbus-app/office. office/install.sh puts in the program (hearth-write, hearth-sheets and
# hearth-slides are links to it), the menu entries and the .hslides file type (the icons are in Hearth's icon set); they're
# the default apps for office files with the others (Hearth's default apps, below). Fonts with
# the same sizes as Office's (Calibri, Cambria, Arial, Times New Roman), so documents keep their layout.
dnf5 -y install --skip-unavailable --skip-broken google-carlito-fonts google-crosextra-caladea-fonts liberation-sans-fonts liberation-serif-fonts liberation-mono-fonts || true
if [ -x /ctx/nimbus-app/office/hearth-office ]; then sh /ctx/nimbus-app/office/install.sh
else echo "Hearth Office wasn't built; this image goes without it"; fi
# ---- end of Hearth Office

# ------------------------------------------------------------------ Hearth's KDE apps (hosts/hearth-kde, docs/KDE-APPS.md)
# Hearth Terminal (Konsole), Hearth Text (Kate), Reader (Okular), Capture (Spectacle) and Hearth Calculator (KCalc), the
# base's own KDE packages with Hearth's names, icons and colors; Hearth Files built from Dolphin and kio-hearth (hearth:/
# in every KDE app) from CI's build inside this base (os/nimbus-app/hearth-kde.tar), only when they built, passed their
# tests and start here (otherwise the old Hearth Files stays the file manager). The old Terminal, Text Editor and
# Calculator stay installed as "… (classic)". Never stops the image build.
# KDE Frameworks to Fedora's current ones first: the base image's are older than the KDE apps Fedora ships today
# (Fedora's own Kate 26.08 failed with "undefined symbol: KConfig::copyFrom", run 44296110901), and RPM only asks for
# the library's major version, so nothing else catches that.
kf_before=$(rpm -q --qf '%{VERSION}' kf6-kconfig 2>/dev/null || true)
dnf5 -y upgrade --refresh 'kf6-*' || echo "::warning title=KDE Frameworks::couldn't upgrade kf6-*"
echo "::notice title=KDE Frameworks::kf6-kconfig ${kf_before:-?} -> $(rpm -q --qf '%{VERSION}' kf6-kconfig 2>/dev/null || true)"
bash /ctx/hearth-kde/install.sh /ctx/nimbus-app/hearth-kde.tar || echo "warning: Hearth's KDE apps didn't install"
# Hearth Files' own Dolphin build has to match the KDE Frameworks this image really has. The owner's laptop (Nov 18):
# "symbol lookup error: libdolphinprivate.so.6: undefined symbol: KConfig::copyFrom" (built against KF 6.32, the
# image's KConfig older): Files didn't open. install.sh only checks for missing libraries, so here every library of
# the build is resolved too (ldd -r). Anything unresolved: the build and kio-hearth are left out, and Files is
# Fedora's own Dolphin (hearth-files-kde falls back to it: Hearth's name and places, KDE's program).
# (|| true: a library with nothing unresolved makes grep "fail", which stopped the whole build under set -e/pipefail.)
unresolved() { { LD_LIBRARY_PATH="$2" ldd -r "$1" 2>&1 | grep -E 'undefined symbol|not found' | head -3; } || true; }
if [ -d /usr/lib/hearth-files ]; then
  bad=""
  for f in /usr/lib/hearth-files/bin/dolphin $(find /usr/lib/hearth-files -name '*.so*' -type f 2>/dev/null); do
    u=$(unresolved "$f" /usr/lib/hearth-files/lib); if [ -n "$u" ]; then bad="$bad ${f##*/}: $u;"; fi
  done
  if [ -n "$bad" ]; then
    echo "::warning title=Hearth Files::its Dolphin build doesn't match this image's KDE Frameworks ($(echo "$bad" | cut -c1-600)): Fedora's own Dolphin is Files here"
    rm -rf /usr/lib/hearth-files
  fi
fi
for k in $(find /usr/lib /usr/lib64 -path '*/plugins/kf6/kio/hearth.so' 2>/dev/null); do
  u=$(unresolved "$k" ""); if [ -n "$u" ]; then echo "::warning title=kio-hearth::left out, it doesn't match this image's KDE Frameworks: $u"; rm -f "$k"; fi
done
rpm -q dolphin >/dev/null 2>&1 || dnf5 -y install dolphin
command -v dolphin >/dev/null || { echo "::error title=Files::no Dolphin in the image: Plasma's Files wouldn't open"; exit 1; }

# ------------------------------------------------------------------ Hearth's default apps
# Every Hearth app in the image is the default for the files and links it opens (its .desktop file's MimeType), in
# Desktop Mode's and Game Mode's list (/etc/xdg/nimbus-mimeapps.list, XDG_CURRENT_DESKTOP=Nimbus: it comes before
# every list the base's own apps bring) and in every desktop's (/usr/share/applications/mimeapps.list). One place,
# written once every app is in: hearth-system defaults-system (desktop/shell), which also picks between two Hearth
# apps for the same type (Photos before Paint, Text before Write, Sheets for CSV). Settings › Default apps saves
# your own choices in ~/.config/mimeapps.list, which comes first; nimbus-session takes Hearth apps that aren't
# installed any more out of it at each sign-in.
mkdir -p /etc/xdg
python3 /usr/share/nimbus/desktop/shell/hearth-system defaults-system / | tee /tmp/hearth-defaults.json
echo "::notice title=Hearth's default apps::$(cat /tmp/hearth-defaults.json)" || true
# Lists the base brings that Hearth's session would read after its own (shown, not changed).
echo "Other default-app lists in the image: $(ls /etc/xdg/*mimeapps.list /usr/share/applications/*mimeapps.list 2>/dev/null | tr '\n' ' ')"
update-desktop-database -q /usr/share/applications 2>/dev/null || true

# ------------------------------------------------------------------ one app of each kind per desktop (docs/APP-KINDS.md)
# >>> app kinds (feature/app-per-desktop)
# The table /usr/share/hearth/app-kinds.json says which app each desktop shows for each kind (Hearth Classic: Hearth's
# own; Hearth Plasma: the KDE-based ones; apps with no other of their kind everywhere). hearth-app-kinds writes copies
# of the menu entries with OnlyShowIn/NotShowIn into /usr/share/hearth/app-kinds/applications (nimbus-session puts it
# first in XDG_DATA_DIRS, so the packages' own files are never edited), each desktop's default apps
# (/etc/xdg/<desktop>-mimeapps.list), and stops the build if a desktop would show two apps of one kind or none
# where one is needed. Runs after Hearth's default apps (above), once every app is in.
# (Anatase: Plasma's session leaves these copies out, so both Hearth's apps and KDE's show in Plasma's menu; they're
# still made for Game Mode's lists. A conflict here is only a warning.)
python3 /usr/libexec/hearth-app-kinds apply --root / > /tmp/hearth-app-kinds.json || { cat /tmp/hearth-app-kinds.json; echo "::warning title=app kinds::a desktop would show two apps of one kind, or none (above); Plasma doesn't use these copies"; }
echo "::notice title=One app of each kind::$(python3 -c 'import json; d=json.load(open("/tmp/hearth-app-kinds.json")); print("; ".join("%s: %s" % (k, ", ".join("%s=%s" % (a, b) for a, b in sorted(v["apps"].items()) if b)) for k, v in d["desktops"].items() if v["present"]))' 2>/dev/null)" || true
chmod 755 /usr/libexec/hearth-app-kinds
# <<< app kinds

# >>> Hearth's apps in Plasma (feature/hearth-apps-rust; docs/APPS-ANATASE.md)
# Hearth's pure-Rust apps (hosts/hearth-ui, built inside this base by base-anatase.yml's "hearth-ui" job): Task
# Manager, System Restore and Reliability history, the ones that ran only inside Mantel. They take those apps' own
# menu entries and commands here (this edition has no Mantel), and are left out, with their entries as before, when
# they weren't built or can't start.
for b in hearth-task-manager hearth-system-restore hearth-reliability-history; do
  [ -x "/ctx/nimbus-app/hearth-ui/$b" ] || { echo "warning: $b wasn't built, so this image goes without it"; continue; }
  install -Dm755 "/ctx/nimbus-app/hearth-ui/$b" "/usr/bin/$b"
  # (grep -c, not -q: under pipefail an early grep -q could cut ldd off and hide the match)
  if [ "$(ldd "/usr/bin/$b" 2>&1 | grep -c 'not found' || true)" != 0 ]; then
    echo "::warning title=Hearth's pure-Rust apps::$b can't start here (missing $(ldd /usr/bin/$b | grep 'not found' | awk '{print $1}' | tr '\n' ' ')), left out"
    rm -f "/usr/bin/$b"
  fi
done
dnf5 -y install --skip-unavailable rsms-inter-fonts >/dev/null 2>&1 || true
if [ -x /usr/bin/hearth-task-manager ]; then
  cat > /usr/share/applications/hearth-tasks.desktop <<DESKTOP
[Desktop Entry]
Name=Task Manager
Comment=See what's running, close apps that stopped responding, and check performance
Exec=/usr/bin/hearth-task-manager
TryExec=/usr/bin/hearth-task-manager
Icon=hearth-tasks
Type=Application
Categories=System;Monitor;
Keywords=task;process;end;kill;cpu;memory;gpu;performance;startup;services;
StartupWMClass=hearth-tasks
Actions=performance;startup;

[Desktop Action performance]
Name=Performance
Exec=/usr/bin/hearth-task-manager --page performance

[Desktop Action startup]
Name=Startup apps
Exec=/usr/bin/hearth-task-manager --page startup
DESKTOP
fi
# System Restore and Reliability history: their entries (OnlyShowIn=Nimbus) and commands open the Rust apps.
for pair in restore:hearth-system-restore reliability:hearth-reliability-history; do
  id=${pair%%:*}; b=${pair#*:}
  [ -x "/usr/bin/$b" ] || continue
  f=/usr/share/applications/hearth-$id.desktop
  if [ -f "$f" ]; then
    sed -i -e '/^OnlyShowIn=/d' -e "s|^Exec=/usr/bin/hearth-$id\b|Exec=/usr/bin/$b|" "$f"
    grep -q '^StartupWMClass=' "$f" || sed -i "0,/^Exec=/s||StartupWMClass=hearth-$id\nExec=|" "$f"
  fi
  ln -sf "$b" "/usr/bin/hearth-$id"
done
echo "::notice title=Hearth's pure-Rust apps::$(for b in hearth-task-manager hearth-system-restore hearth-reliability-history; do [ -x /usr/bin/$b ] && printf '%s (%s) ' "$b" "$(du -h /usr/bin/$b | cut -f1)"; done)"
# The owner (Nov 18): every Hearth app that runs under Plasma is installed and shown in Plasma's menu, and Hearth's own
# apps are the defaults there (menu and file types); KDE's apps stay installed but never take Hearth's place. So, for
# Plasma (KDE) only, each kind's list becomes Hearth's own first (in Hearth Classic's order), then the Hearth KDE
# builds, then KDE's as the fallback. Apps that run only inside Mantel (Quickshell, not in this image) are never put
# first: they couldn't open. hearth-app-kinds then writes Plasma's menu copies and kde-mimeapps.list again; only when
# its check passes is Plasma told to use the copies (the marker below, read by nimbus-session's run_plasma), so a
# problem here leaves Plasma's menu as it was. Your own choices (~/.config/mimeapps.list, Default Applications) always
# come first and are never written here.
hearth_plasma_apps() {
  python3 - <<'PY'
import json
p = '/usr/share/hearth/app-kinds.json'
t = json.load(open(p))
# Mantel-only entries can't start in this image: the ones still shown only in Mantel (OnlyShowIn=Nimbus alone), and
# Files and Capture. A pure-Rust port, once in, takes the entry (above) and stops being one.
import os, re
mantel_only = set()
for d in ('/usr/share/applications',):
    for n in os.listdir(d):
        if not n.endswith('.desktop'):
            continue
        try:
            txt = open(os.path.join(d, n), encoding='utf-8', errors='replace').read()
        except OSError:
            continue
        only = re.search(r'^OnlyShowIn=(.*)$', txt, re.M)
        if only and [x for x in only.group(1).split(';') if x] == ['Nimbus']:
            mantel_only.add(n)
mantel_only |= {'hearth-files.desktop', 'hearth-capture.desktop'} & set(os.listdir('/usr/share/applications'))
changed = []
for k, kind in t['kinds'].items():
    apps = kind.get('apps') or {}
    if 'KDE' not in apps:
        continue
    old = list(apps['KDE'])
    first = [i for i in (apps.get('Nimbus') or []) if i.startswith('hearth-') and i not in mantel_only]
    builds = [i for i in old if i.startswith('hearth-') and i not in mantel_only]
    new = []
    for i in first + builds + [i for i in old if not i.startswith('hearth-')] + [i for i in old if i in mantel_only]:
        if i not in new:
            new.append(i)
    if new != old:
        apps['KDE'] = new
        changed.append('%s: %s' % (k, ', '.join(new)))
tmp = p + '.tmp'
with open(tmp, 'w') as f:
    json.dump(t, f, indent=1, ensure_ascii=False)
    f.write('\n')
import os
os.replace(tmp, p)
print('::notice title=Plasma: Hearth first::' + ('; '.join(changed) or 'no change'))
PY
}
rm -f /usr/share/hearth/app-kinds/plasma-ready
if hearth_plasma_apps && python3 /usr/libexec/hearth-app-kinds apply --root / > /tmp/hearth-app-kinds-plasma.json \
   && python3 /usr/libexec/hearth-app-kinds check --root / >/dev/null; then
  : > /usr/share/hearth/app-kinds/plasma-ready
  echo "::notice title=Plasma's apps::$(python3 -c 'import json; d=json.load(open("/tmp/hearth-app-kinds-plasma.json")); v=d["desktops"].get("KDE",{}); print(", ".join("%s=%s" % (a, b) for a, b in sorted(v.get("apps",{}).items()) if b))' 2>/dev/null)"
else
  echo "::warning title=Plasma's apps::hearth-app-kinds didn't pass for Plasma, so Plasma's menu shows both Hearth's and KDE's apps, as before: $(head -c 1500 /tmp/hearth-app-kinds-plasma.json 2>/dev/null | tr '\n' ' ')"
fi
# Every Hearth menu entry whose program isn't here or can't start (a missing library) is named, so nothing is missing
# from Plasma's menu without a reason in the build's notices.
for f in /usr/share/applications/hearth-*.desktop; do
  prog=$(sed -n 's/^TryExec=//p' "$f" | head -1); [ -n "$prog" ] || prog=$(sed -n 's/^Exec=//p' "$f" | head -1 | awk '{print $1}')
  case "$prog" in ''|env|sh|bash|python3) continue ;; esac
  path=$(command -v "$prog" 2>/dev/null) || { echo "::warning title=Plasma menu::$(basename "$f"): its program $prog isn't in this image"; continue; }
  if [ "$(ldd "$path" 2>/dev/null | grep -c 'not found' || true)" != 0 ]; then
    echo "::warning title=Plasma menu::$(basename "$f"): $prog can't start, missing $(ldd "$path" | grep 'not found' | awk '{print $1}' | tr '\n' ' ')"
  fi
done
# <<< Hearth's apps in Plasma

# ------------------------------------------------------------------ 1.15: Desktop Mode's way back to Game Mode
# Plasma's menu and taskbar have "Back to Game Mode" (os/system_files' hearth-gamemode.desktop, KDE only, Super+G;
# pinned once per account by the Plasma update script hearth-game-mode-launcher.js): nimbus-mode game logs Plasma out
# and Hearth's session starts Game Mode.
sed -i 's/^Name=Game Mode$/Name=Back to Game Mode/' /usr/share/applications/hearth-gamemode.desktop
grep -qx 'Name=Back to Game Mode' /usr/share/applications/hearth-gamemode.desktop
test -f /usr/share/plasma/shells/org.kde.plasma.desktop/contents/updates/hearth-game-mode-launcher.js

# ------------------------------------------------------------------ what the Debian edition gained since Anatase was archived
# (os-bases/peppermint/build.sh, in Fedora's form; each one optional unless it says otherwise)
# Hearth Files' shared folders (gio + gvfs SMB + FUSE, secret-tool), locked Briefcases (gocryptfs, required as on
# Debian), PDF text and previews, the light sensor for auto-brightness, Capture's text and QR reading (Spectacle on
# Plasma), Task Manager's dump files and memory slots, sensors' names, spell checking, Thunderbolt, OpenVPN files,
# notification sounds, the phone's files (sshfs).
dnf5 -y install --skip-unavailable --skip-broken glib2 gvfs gvfs-smb gvfs-fuse libsecret poppler-utils iio-sensor-proxy tesseract \
  tesseract-langpack-eng zbar gdb dmidecode lm_sensors hunspell-en-US bolt NetworkManager-openvpn sound-theme-freedesktop \
  sshfs avahi-tools || true
dnf5 -y install gocryptfs
command -v gocryptfs || { echo "::error::gocryptfs isn't in the image: Briefcases couldn't be locked or unlocked"; exit 1; }
command -v gio && command -v secret-tool
# Hearth Games: Hearth Games (the store app) next to Controller Lab, Saves and Mods.
if [ -x /ctx/nimbus-app/hearth-gamestore ]; then
  install -Dm755 /ctx/nimbus-app/hearth-gamestore /usr/bin/hearth-gamestore
  if ldd /usr/bin/hearth-gamestore | grep "not found"; then echo "warning: hearth-gamestore needs the libraries above; left out"; rm -f /usr/bin/hearth-gamestore; fi
fi
gaming_entry hearth-gamestore "Hearth Games" "Game Store" "Your GOG, Epic Games and Battle.net games: sign in, install, play and uninstall" "gog;epic;epic games;battle.net;battlenet;blizzard;store;games;library;install;heroic;"

# ------------------------------------------------------------------ Hearth home server (Settings › Home server, docs/HOME-SERVER.md)
# Samba, smartmontools, nftables, restic, rclone, Syncthing, hdparm, NUT, git and Python's venv, and Tailscale from its own
# repository. Every service they bring is off: nothing listens or runs until the owner turns it on in Settings.
chmod 755 /usr/libexec/nimbus/hearth-home-server-helper 2>/dev/null || true
dnf5 -y install --skip-unavailable --skip-broken samba smartmontools nftables restic rclone syncthing hdparm nut nut-client git python3-virtualenv || true
if curl -fsSL --retry 3 -o /etc/yum.repos.d/tailscale.repo https://pkgs.tailscale.com/stable/fedora/tailscale.repo \
   && dnf5 -y install tailscale; then echo "Tailscale installed"
else echo "::warning::couldn't install Tailscale: Settings › Home server says so"; rm -f /etc/yum.repos.d/tailscale.repo; fi
if [ -f /etc/yum.repos.d/tailscale.repo ]; then sed -i 's/^enabled=1/enabled=0/' /etc/yum.repos.d/tailscale.repo; fi
if [ -d /etc/samba ]; then cat > /etc/samba/smb.conf <<'SMBCONF'
# Hearth OS: Samba starts with nothing shared. Settings › Home server › Shared folders and Hearth Storage's Windows
# sharing add their shares from their own files, included below by their helpers. Change shares there.
[global]
   workgroup = WORKGROUP
   server string = Hearth OS
   server role = standalone server
   server min protocol = SMB3
   map to guest = Never
   restrict anonymous = 2
   usershare allow guests = no
   disable netbios = yes
   smb ports = 445
   load printers = no
   printing = bsd
   printcap name = /dev/null
   passdb backend = tdbsam
   logging = systemd
SMBCONF
fi
HS_TIMERS="hearth-home-server-snapshots.timer hearth-home-server-backup.timer hearth-home-server-alerts.timer hearth-home-server-power.timer hearth-home-server-wake.timer hearth-home-server-receive.timer hearth-home-server-services.timer hearth-ollama.timer hearth-jellyfin.timer"
for u in smb.service nmb.service samba.service tailscaled.service smartd.service nftables.service nut-server.service nut-monitor.service \
         nut-driver-enumerator.service syncthing-resume.service $HS_TIMERS; do
  systemctl disable "$u" 2>/dev/null || true
done
systemctl disable nut.target nut-driver.target nut-driver-enumerator.path hearth-ai-share-firewall.timer hearth-remote-desktop-firewall.service lactd.service 2>/dev/null || true
systemctl --global disable hearth-taildrop.service syncthing.service hearth-home-server-web.service hearth-server-mode-web.service \
  hearth-comfyui.timer hearth-kokoro.timer hearth-voice-studio.timer hearth-colibri.timer hearth-ai-share.service hearth-vnc.service sunshine.service 2>/dev/null || true
for u in smb.service nmb.service tailscaled.service smartd.service nftables.service nut-server.service nut-monitor.service \
         hearth-ai-share-firewall.timer hearth-remote-desktop-firewall.service $HS_TIMERS; do
  if [ "$(systemctl is-enabled "$u" 2>/dev/null)" = enabled ]; then echo "::error::$u is on in the image: it stays off until the owner turns it on"; exit 1; fi
done
for u in hearth-taildrop.service syncthing.service hearth-home-server-web.service hearth-ai-share.service hearth-vnc.service sunshine.service; do
  if [ "$(systemctl --global is-enabled "$u" 2>/dev/null)" = enabled ]; then echo "::error::$u (a user service) is on in the image"; exit 1; fi
done
printf '%s\n' '# Settings › Home server: the state of backups and alerts, and the restic cache.' \
  'd /var/lib/hearth 0755 root root -' 'd /var/cache/hearth-restic 0700 root root -' > /usr/lib/tmpfiles.d/hearth-home-server.conf
chmod 755 /usr/share/nimbus/desktop/shell/hearth-remote-desktop /usr/share/nimbus/desktop/shell/hearth-vnc /usr/share/nimbus/desktop/shell/hearth-record 2>/dev/null || true

# ------------------------------------------------------------------ LACT (GPU tuning) and Ludusavi (saved games), 1.11
# The same pinned versions as Debian. LACT's Fedora package from its release (lactd off until you turn tuning on).
LACT_VER=0.10.1
LACT_RPM_URL="$(curl -fsSL --retry 3 "https://api.github.com/repos/ilya-zlobintsev/LACT/releases/tags/v${LACT_VER}" 2>/dev/null \
  | python3 -c 'import json, sys; f = sys.argv[1]; a = [x["browser_download_url"] for x in json.load(sys.stdin).get("assets", []) if x["name"].endswith(".rpm") and "fedora" in x["name"] and "x86_64" in x["name"] and "headless" not in x["name"]]; a.sort(key=lambda u: (("fedora-" + f) not in u, u)); print(a[0] if a else "")' "$FEDORA" 2>/dev/null || true)"
if [ -n "$LACT_RPM_URL" ] && dnf5 -y install "$LACT_RPM_URL"; then
  systemctl disable lactd.service 2>/dev/null || true; echo "LACT $LACT_VER installed ($LACT_RPM_URL)"
else echo "warning: couldn't install LACT $LACT_VER; GPU tuning stays unavailable"; fi
LUDU_VER=0.31.0
if curl -fsSL --retry 3 -o /tmp/ludusavi.tar.gz "https://github.com/mtkennerly/ludusavi/releases/download/v${LUDU_VER}/ludusavi-v${LUDU_VER}-linux.tar.gz" \
   && mkdir -p /tmp/ludusavi && tar -xzf /tmp/ludusavi.tar.gz -C /tmp/ludusavi && install -Dm755 /tmp/ludusavi/ludusavi /usr/bin/ludusavi; then
  echo "Ludusavi $LUDU_VER installed"
else echo "warning: couldn't install Ludusavi $LUDU_VER; Saved games stays unavailable"; fi
rm -rf /tmp/ludusavi /tmp/ludusavi.tar.gz

# ------------------------------------------------------------------ Hearth Games' engines: gogdl, legendary, nile (the same pins as Debian)
hearth_tool() {   # file, url, sha256
  local tmp="/tmp/hearth-tool-$1"
  if curl -fsSL --retry 3 -o "$tmp" "$2" && echo "$3  $tmp" | sha256sum -c - >/dev/null; then
    install -Dm755 "$tmp" "/usr/lib/hearth/tools/$1" && echo "$1 put in the image ($2)"
  else
    echo "warning: couldn't get $1 ($2), or its checksum didn't match; Hearth downloads it on first use instead"
  fi
  rm -f "$tmp"
}
hearth_tool gogdl https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.3.0/gogdl_linux_x86_64 cba013d42767c808237c437335ab1d56f58405d07e8f37b3324d264ea5c49655
hearth_tool legendary https://github.com/legendary-gl/legendary/releases/download/0.21.1/legendary_linux_x64 dbed33bbe96031e65858233e4badf0a1d401bb6cb0cc995d1237cf3a0c826a45
hearth_tool nile https://github.com/imLinguin/nile/releases/download/v1.2.0/nile_linux_x86_64 d73519514df9d52c50c6feec629373cc3a0d9504e087b6c7372a8ca8a95e7bfe

# ------------------------------------------------------------------ GE-Proton, built in (docs/WINDOWS-APPS.md): the same pin as Debian, required
GE_PROTON=GE-Proton11-3
GE_PROTON_SHA256=861c2edc8d40d051fb1e7a692deb953be52bd339c46d90f2b7dde50ddad91266
if ! curl -fsSL --retry 5 --retry-delay 10 -o /tmp/ge-proton.tar.gz \
     "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/$GE_PROTON/$GE_PROTON.tar.gz"; then
  echo "::error title=GE-Proton::couldn't download $GE_PROTON"; exit 1
fi
echo "$GE_PROTON_SHA256  /tmp/ge-proton.tar.gz" | sha256sum -c - || { echo "::error title=GE-Proton::$GE_PROTON's checksum didn't match the pinned one"; exit 1; }
mkdir -p /usr/share/hearth/proton /usr/share/steam/compatibilitytools.d
tar -xzf /tmp/ge-proton.tar.gz -C /usr/share/hearth/proton
rm -f /tmp/ge-proton.tar.gz
test -f "/usr/share/hearth/proton/$GE_PROTON/proton" || { echo "::error title=GE-Proton::$GE_PROTON has no proton script"; exit 1; }
ln -sfn "/usr/share/hearth/proton/$GE_PROTON" "/usr/share/steam/compatibilitytools.d/$GE_PROTON"
if [ "$(HOME=/nonexistent /usr/bin/hearth-proton-run --print-proton | cut -f1)" != "$GE_PROTON" ]; then
  echo "::error title=GE-Proton::hearth-proton-run doesn't pick the built-in $GE_PROTON: $(HOME=/nonexistent /usr/bin/hearth-proton-run --print-proton 2>&1)"; exit 1
fi
dnf5 -y install --skip-unavailable --skip-broken icoutils minigalaxy || true

# ------------------------------------------------------------------ 1.14: VPN plugins, fingerprints, CryFS (optional)
dnf5 -y install --skip-unavailable --skip-broken NetworkManager-strongswan NetworkManager-l2tp NetworkManager-pptp NetworkManager-openconnect \
  NetworkManager-sstp NetworkManager-fortisslvpn fprintd fprintd-pam cryfs || true
systemctl disable strongswan.service strongswan-starter.service ipsec.service xl2tpd.service 2>/dev/null || true
# RED FLAG (docs/FINGERPRINT.md): a fingerprint is never the only way in, and never in front of every password. Fedora
# turns fprintd on through authselect's with-fingerprint feature: it stays off here.
if command -v authselect >/dev/null && authselect current 2>/dev/null | grep -q with-fingerprint; then authselect disable-feature with-fingerprint || true; fi
if grep -qs pam_fprintd /etc/pam.d/system-auth /etc/pam.d/password-auth; then
  echo "::error::system-auth asks for a fingerprint: that would put the reader in front of every password"; exit 1
fi

# ------------------------------------------------------------------ memory, guards and the rest of Hearth's services (as on Debian)
chmod 755 /usr/libexec/nimbus/hearth-memory /usr/libexec/nimbus/hearth-hwset /usr/libexec/nimbus/hearth-diskguard /usr/libexec/nimbus/hearth-screen-psr /usr/libexec/nimbus/hearth-oem-account 2>/dev/null || true
systemctl enable hearth-memory.service hearth-memguard.service hearth-hwset.service hearth-diskguard.service hearth-gamecores.service hearth-app-rules.service
systemctl enable hearth-screen-psr.service hearth-oem-account.service hearth-dns-client.service
systemctl --global enable hearth-oem-notice.service
# Low battery (1.11): UPower warns at 10%, is critical at 5%, and at 2% shuts down cleanly.
if [ -f /etc/UPower/UPower.conf ]; then
  sed -i -E 's/^#?\s*UsePercentageForPolicy=.*/UsePercentageForPolicy=true/; s/^#?\s*PercentageLow=.*/PercentageLow=10.0/;
    s/^#?\s*PercentageCritical=.*/PercentageCritical=5.0/; s/^#?\s*PercentageAction=.*/PercentageAction=2.0/;
    s/^#?\s*CriticalPowerAction=.*/CriticalPowerAction=PowerOff/' /etc/UPower/UPower.conf
fi
# Hearth's terminal speaks C:\ (1.11) in every interactive bash.
grep -q 'hearth/shell/windows-commands.bash' /etc/bashrc 2>/dev/null || \
  printf '\n# Hearth: the common Windows commands and C:\\ paths (HEARTH_WINDOWS_COMMANDS=0 turns them off)\n[ -r /usr/share/hearth/shell/windows-commands.bash ] && . /usr/share/hearth/shell/windows-commands.bash\n' >> /etc/bashrc
getent group netdev >/dev/null || groupadd -r netdev
# Settings › Region & language: every language's formats (Fedora's langpacks).
dnf5 -y install --skip-unavailable --skip-broken glibc-all-langpacks || true

# ------------------------------------------------------------------ the boot menu on a PC that came from Debian (red flag 2)
# Such a PC starts through Debian's signed shim and GRUB, which read Hearth's menu (/boot/grub2/hearth-bls.cfg) instead of
# ostree's entries: hearth-bls-grub keeps that menu following Anatase's updates and a rollback to Debian. Its units only
# run where that menu is (their 10-hearth-anatase.conf drop-ins), and bootupd never writes Fedora's loader over Debian's
# there (bootloader-update.service's drop-in). On a PC installed with Anatase's own loader nothing of this runs.
chmod 755 /usr/libexec/nimbus/hearth-bls-grub
systemctl enable hearth-bls-grub.service hearth-bls-grub-refresh.path
# The owner's laptop (Nov 17): after the move from Debian, Fedora's bootupd updated the EFI partition by itself:
# Debian's shim and GRUB were removed, Anatase's put in EFI/anatase WITHOUT grub.cfg and bootuuid.cfg (bootupd only
# writes those at install), the fallback loader made an "Anatase" firmware entry, and the PC stopped at "grub>".
# So on a PC whose boot menu is Hearth's: (1) bootupctl never updates or adopts the boot loader there (whoever asks:
# bootloader-update.service, HHD, Anatase's own updaters; HEARTH_BOOTUPD_ALLOW=1 for a person who means it), so
# Debian's loader stays and the previous version stays startable; (2) hearth-esp-guard gives every loader folder
# that lacks them the grub.cfg and bootuuid.cfg that find /boot's menu (repairs a PC that already got there).
if [ -e /usr/bin/bootupctl ]; then
  real=$(readlink -f /usr/bin/bootupctl)
  [ "$real" = /usr/bin/bootupctl ] && { mv /usr/bin/bootupctl /usr/libexec/nimbus/bootupctl.real; real=/usr/libexec/nimbus/bootupctl.real; }
  rm -f /usr/bin/bootupctl
  cat > /usr/bin/bootupctl <<WRAP
#!/bin/bash
# Hearth OS on Anatase: see os-bases/anatase/build.sh ("the boot menu on a PC that came from Debian"). On a PC whose
# boot menu is Hearth's own (/boot/grub2/hearth-bls.cfg), the boot loader is never updated or adopted by bootupd.
case " \$* " in
  *" update "*|*" adopt-and-update "*)
    if [ -e /boot/grub2/hearth-bls.cfg ] && [ "\${HEARTH_BOOTUPD_ALLOW:-}" != 1 ]; then
      echo "bootupctl: left as it is on this PC: it starts through Hearth OS on Debian's boot loader and menu (red flag 2). HEARTH_BOOTUPD_ALLOW=1 to do it anyway." >&2
      exit 0
    fi ;;
esac
exec -a bootupctl $real "\$@"
WRAP
  chmod 755 /usr/bin/bootupctl
  bootupctl --version >/dev/null 2>&1 || bootupctl --help >/dev/null 2>&1 || { echo "::error title=bootupctl::the wrapper doesn't start bootupd ($real)"; exit 1; }
fi
echo "::notice title=bootupd callers::units that run bootupctl: $(grep -rls bootupctl /usr/lib/systemd/system /usr/lib/systemd/user /etc/systemd 2>/dev/null | tr '\n' ' ')"
chmod 755 /usr/libexec/nimbus/hearth-esp-guard
systemctl enable hearth-esp-guard.service
if command -v grub2-mkfont >/dev/null; then for sz in 18 22; do grub2-mkfont -n Hearth -s "$sz" -o "/usr/share/hearth/grub-theme/hearth-regular-$sz.pf2" "$(fc-match -f '%{file}' 'Inter:style=Regular' 2>/dev/null || fc-match -f '%{file}' sans)" || true; done; fi

# ------------------------------------------------------------------ Unlock the file system (1.15, hearth-unlock, dnf form)
# Off unless turned on (Settings › Updates & Recovery, or sudo hearth-unlock): then this version's own writable layer
# goes over /usr at every start, and what you install with dnf comes back after an update. The boot chain stays the
# image's (dnf excludes it; every plan is checked). See /usr/bin/hearth-unlock for what survives what.
chmod 755 /usr/bin/hearth-unlock /usr/lib/systemd/system-generators/hearth-etc-guard
python3 -m py_compile /usr/bin/hearth-unlock && rm -rf /usr/bin/__pycache__
install -Dm644 /usr/share/hearth/unlock/dnf-boot-chain.conf /etc/dnf/libdnf5.conf.d/20-hearth-boot-chain.conf
rm -f /etc/hearth/system-unlocked
systemctl enable hearth-unlock.service hearth-unlock-units.service hearth-unlock-good.service hearth-unlock-reapply.service \
  hearth-unlock-ldcache.service hearth-unlock-record.path

# ------------------------------------------------------------------ boot straight into Hearth
# nimbus-autologin writes the login screen's autologin: Plasma Login Manager's /etc/plasmalogin.conf on Anatase
# (and SDDM's file, harmless when SDDM isn't there).
systemctl enable nimbus-autologin.service
systemctl enable hearth-boot-guard.service hearth-boot-guard-refresh.path hearth-boot-good.service hearth-recovery.service  # automatic fallback, Recovery on the device to the previous version
systemctl enable hearth-greeter-pad.service   # the controller on the login screen
systemctl enable hearth-boot-first.service    # Hearth stays first in the boot order
systemctl enable hearth-groups.service        # accounts in video (screen brightness)
systemctl enable hearth-home-repair.service   # home folders' Flatpak/cache folders yours, container id ranges
systemctl enable hearth-flatpak-setup.service # Flatpak's system installation, Flathub, Bazaar (when not a package)
systemctl enable hearth-hosts.service hearth-hosts.path  # /etc/hosts maps the device's name (sudo warns otherwise)
systemctl enable hearth-bluetooth-labels.service  # Bluetooth pairings readable under SELinux before bluetoothd
# The first start after moving here from the Bazzite-based Hearth OS: a clean install that keeps your games
# (os/system_files; does nothing without the plan the old system leaves).
systemctl enable hearth-clean-install.service

# ------------------------------------------------------------------ Quick Resume through turning off and restarting
# Quick Resume keeps games through Shut down and Restart by hibernating (hearth-admin hibernate-setup makes the room,
# hibernate-reboot restarts with them kept; the sleep hook hearth-restart-keep comes from os/system_files).
chmod 755 /usr/lib/systemd/system-sleep/hearth-restart-keep
# Nvidia: the driver must save the graphics card's memory before the PC turns off (PreserveVideoMemoryAllocations),
# to a folder on the drive with room for it (/var/tmp; /tmp is in memory), and its helpers must run around sleep and
# hibernation, or the picture comes back black or broken (RTX desktops). Written before the boot image (dracut,
# below) so the setting is in it too. Harmless without an Nvidia card (the module never loads).
echo "Nvidia power management Anatase brings: $(grep -rhs 'NVreg_PreserveVideoMemoryAllocations\|NVreg_TemporaryFilePath\|NVreg_UseKernelSuspendNotifiers' /usr/lib/modprobe.d /etc/modprobe.d | tr '\n' ' ')"
# What the screen side of the image is, as a notice on the CI run ("only 60 Hz on a 144 Hz TV" reports: HDMI 2.1's
# fast link (FRL) depends on the driver's version and nvidia-modeset's options, 4K above 60 Hz needs it): the
# driver's packages and version, open or Nvidia's own kernel modules, every nvidia option in modprobe.d, the
# kernel arguments the image adds, and gamescope's version.
(
  set +x   # the build's command tracing kept out of the notice
  NV_KVER=$(ls /usr/lib/modules | sort -V | tail -1)
  echo "kernel $NV_KVER;"
  echo "nvidia module: version $(modinfo -k "$NV_KVER" -F version nvidia 2>/dev/null || echo none), license $(modinfo -k "$NV_KVER" -F license nvidia 2>/dev/null || echo none) (Dual MIT/GPL = open modules);"
  echo "nvidia-modeset options: $(modinfo -k "$NV_KVER" -F parm nvidia_modeset 2>/dev/null | cut -d: -f1 | tr '\n' ' ');"
  echo "modprobe.d: $(grep -rhsE '^[[:space:]]*(options|install|blacklist)[[:space:]].*(nvidia|nouveau|nova)' /usr/lib/modprobe.d /etc/modprobe.d | tr '\n' ';');"
  echo "kargs.d: $(cat /usr/lib/bootc/kargs.d/*.toml 2>/dev/null | grep -E '^kargs' | tr '\n' ' ')"
  echo "packages: $(rpm -qa 2>/dev/null | grep -iE '^(nvidia-driver|nvidia-kmod-common|kernel-nvidia|gamescope|edid-decode|libdisplay-info-tools)-[0-9]' | sort -u | tr '\n' ' ')"
) > /tmp/hearth-nvidia-notice.txt 2>/dev/null || true
cat /tmp/hearth-nvidia-notice.txt || true
echo "::notice title=Screen and Nvidia driver in this image::$(tr '\n' ' ' < /tmp/hearth-nvidia-notice.txt | cut -c1-3500)" || true
mkdir -p /usr/lib/modprobe.d
cat > /usr/lib/modprobe.d/zz-hearth-nvidia-power.conf <<'CONF'
# Hearth: Quick Resume keeps games through sleep and turning off (hibernation). Nvidia's driver saves the graphics
# card's memory first, to the drive (/var/tmp: /tmp is in memory and lost when the PC turns off).
options nvidia NVreg_PreserveVideoMemoryAllocations=1 NVreg_TemporaryFilePath=/var/tmp
CONF
NV_HELPERS=0
for s in nvidia-suspend nvidia-resume nvidia-hibernate nvidia-suspend-then-hibernate; do
  if [ -f "/usr/lib/systemd/system/$s.service" ]; then
    echo "$s.service: $(systemctl is-enabled "$s.service" 2>&1 || true) in Anatase"
    mkdir -p "/usr/lib/systemd/system/$s.service.d"
    printf '[Unit]\nConditionPathExists=/proc/driver/nvidia\n' > "/usr/lib/systemd/system/$s.service.d/10-hearth-only-nvidia.conf"
    systemctl enable "$s.service" || true
    NV_HELPERS=1
  fi
done
if [ "$NV_HELPERS" = 1 ]; then
  # systemd 256 and later freeze the session before sleeping, which Nvidia's helpers can't work with (a black screen
  # after waking). Nvidia's own packages turn that off; done here too when Anatase's don't. Hearth's put-aside games
  # are paused with SIGSTOP, which this doesn't touch.
  if ! grep -rqs SYSTEMD_SLEEP_FREEZE_USER_SESSIONS /usr/lib/systemd/system/systemd-suspend.service.d /usr/lib/systemd/system/systemd-hibernate.service.d; then
    for u in systemd-suspend systemd-hibernate systemd-hybrid-sleep systemd-suspend-then-hibernate; do
      mkdir -p "/usr/lib/systemd/system/$u.service.d"
      printf '# Hearth: Nvidia helpers need the session running while they save the graphics memory.\n[Service]\nEnvironment="SYSTEMD_SLEEP_FREEZE_USER_SESSIONS=false"\n' > "/usr/lib/systemd/system/$u.service.d/10-hearth-nvidia.conf"
    done
    echo "Nvidia: session freezing during sleep turned off (Anatase's packages didn't)"
  fi
else
  # Anatase's driver has none (checked in CI): it saves the graphics memory itself from the kernel's suspend
  # notifications, and Anatase already sets PreserveVideoMemoryAllocations=1 with /var/tmp.
  echo "Nvidia's sleep helpers (nvidia-suspend/hibernate/resume.service) aren't in this image: the driver handles sleep and hibernation itself"
fi

# ------------------------------------------------------------------ Hearth's look, not Anatase's or Fedora's (1.15)
# The owner (Nov 18): "also just look like anatase with their logos" — it must look like Hearth. Everywhere a person sees
# a logo: the name (os-release, with "based on Anatase" only in About's details), the boot splash and its presets, the
# distribution icons (Plasma's menu button, KInfoCenter's About, anything asking for the distributor's logo), Plasma's
# wallpaper and lock screen and its start-up splash. Packages' licence and trademark files stay where they are; only
# the pictures a person sees are Hearth's. Users' own choices are never touched: these are only the system's defaults
# (/etc/xdg, the look-and-feel defaults) and, once per account, Plasma's settings that were never picked
# (hearth-look-defaults.js).
FEDORA_N=$FEDORA
HEARTH_SVG=/usr/share/icons/hicolor/scalable/apps/hearth.svg
test -f "$HEARTH_SVG"
hearth_png() {  # hearth_png SIZE: the Hearth logo PNG closest to SIZE (the icon set's own sizes)
  local best="" d
  for d in 16 22 24 32 48 64 96 128 256; do
    if [ -f "/usr/share/icons/hicolor/${d}x${d}/apps/hearth.png" ]; then best=$d; if [ "$d" -ge "${1:-256}" ]; then break; fi; fi
  done
  echo "/usr/share/icons/hicolor/${best}x${best}/apps/hearth.png"
}
swapped=0
while IFS= read -r f; do
  case "$f" in *.svg|*.svgz) cp -f "$HEARTH_SVG" "$f" ;;
    *.png) sz=$(echo "$f" | grep -oE '/[0-9]+x[0-9]+/' | head -1 | tr -d / | cut -dx -f1 || true); cp -f "$(hearth_png "${sz:-256}")" "$f" ;;
    *) continue ;; esac
  swapped=$((swapped + 1))
done < <(find /usr/share/icons /usr/share/pixmaps -type f \( -iname '*fedora*logo*' -o -iname '*anatase*' -o -iname 'start-here*' \
           -o -iname 'distributor-logo*' -o -iname 'system-logo*' -o -iname 'fedora-logo*' \) 2>/dev/null)
for n in start-here start-here-kde distributor-logo; do
  install -Dm644 "$HEARTH_SVG" "/usr/share/icons/hicolor/scalable/apps/$n.svg"
  install -Dm644 "$HEARTH_SVG" "/usr/share/icons/hicolor/scalable/places/$n.svg"
done
gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
echo "::notice title=Hearth's look::$swapped distribution logo files now show Hearth's logo"
# KInfoCenter's About this System: Hearth's logo, name and website; the base in its details.
mkdir -p /etc/xdg
cat > /etc/xdg/kcm-about-distrorc <<KCM
[General]
LogoPath=$(hearth_png 256)
Website=https://hearthos.fun
Variant=Fedora edition (test) · Fedora ${FEDORA_N}
KCM
# Plasma's wallpaper and lock screen by default: Hearth's (Fireside, from the wallpapers installed above), in every
# look-and-feel's defaults and the lock screen's system default. Your own choice in ~/.config always wins.
HW=/usr/share/wallpapers/Hearth-fireside
test -f "$HW/metadata.json"
for d in /usr/share/plasma/look-and-feel/*/contents/defaults; do
  [ -f "$d" ] || continue
  python3 - "$d" "$HW" <<'PY'
import sys, configparser
p, w = sys.argv[1], sys.argv[2]
c = configparser.RawConfigParser(strict=False, interpolation=None); c.optionxform = str
c.read(p)
if not c.has_section('Wallpaper'):
    c.add_section('Wallpaper')
c.set('Wallpaper', 'Image', w.rsplit('/', 1)[1])
with open(p, 'w') as f:
    c.write(f, space_around_delimiters=False)
PY
done
cat > /etc/xdg/kscreenlockerrc <<LOCK
[Greeter][Wallpaper][org.kde.image][General]
Image=file://$HW/contents/images/3840x2160.jpg
PreviewImage=file://$HW/contents/images/3840x2160.jpg
LOCK
# Hearth's looks as Plasma Global Themes: every look in palettes.json (Hearth, Steam-style, Gaming, Zen, Glass,
# Synthwave, Cabin, Console) becomes "Hearth OS · <name>" in System Settings › Global Theme, generated here from its
# palette so it never drifts from Mantel's (hearth-plasma-looks build): dark and light colour schemes (plus one pair per
# sub theme, System Settings › Colors), a Plasma style in its colours, Breeze window frames in its colours, Breeze icons
# and cursor, its wallpaper and a start-up splash in its colours with Hearth's logo. The accent is Plasma's own setting.
chmod 755 /usr/libexec/nimbus/hearth-plasma-looks
python3 /usr/libexec/nimbus/hearth-plasma-looks build --logo "$HEARTH_SVG" > /tmp/hearth-plasma-looks.json
n=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))))' /tmp/hearth-plasma-looks.json)
test "$n" -ge 8
for l in hearth steam gaming zen glass synthwave cabin console; do
  test -f "/usr/share/plasma/look-and-feel/org.hearthos.$l/contents/defaults"
  test -f "/usr/share/plasma/look-and-feel/org.hearthos.$l/contents/splash/Splash.qml"
  test -f "/usr/share/color-schemes/HearthOS-$l-Dark.colors" && test -f "/usr/share/color-schemes/HearthOS-$l-Light.colors"
  test -f "/usr/share/plasma/desktoptheme/hearth-$l/metadata.json"
  w=$(sed -n 's/^Image=//p' "/usr/share/plasma/look-and-feel/org.hearthos.$l/contents/defaults")
  test -f "/usr/share/wallpapers/$w/metadata.json"
done
echo "::notice title=Hearth's looks::$n Plasma Global Themes from palettes.json ($(ls /usr/share/color-schemes/HearthOS-*.colors | wc -l) colour schemes)"
# Hearth is the default for everyone who hasn't chosen a look in KDE (KDE's system defaults; a choice of yours in
# ~/.config always wins). The splash is the look's (Plasma's own splash shows after Hearth's boot splash).
# (Merged into the distribution's /etc/xdg files: their other keys stay.)
python3 - <<'PY2'
import configparser, os
def merge(p, changes):
    c = configparser.RawConfigParser(strict=False, interpolation=None); c.optionxform = str
    if os.path.exists(p):
        c.read(p)
    for g, kv in changes.items():
        if not c.has_section(g):
            c.add_section(g)
        for k, v in kv.items():
            c.set(g, k, v)
    with open(p, 'w') as f:
        c.write(f, space_around_delimiters=False)
merge('/etc/xdg/kdeglobals', {'KDE': {'LookAndFeelPackage': 'org.hearthos.hearth', 'widgetStyle': 'Breeze'},
                              'General': {'ColorScheme': 'HearthOS-hearth-Dark'}, 'Icons': {'Theme': 'breeze-dark'}})
merge('/etc/xdg/plasmarc', {'Theme': {'name': 'hearth-hearth'}})
PY2
grep -qx 'LookAndFeelPackage=org.hearthos.hearth' /etc/xdg/kdeglobals
printf '[KSplash]\nEngine=KSplashQML\nTheme=org.hearthos.hearth\n' > /etc/xdg/ksplashrc
# In the session: the first time, Hearth's look (or, when you already chose one in KDE, a notification asking once);
# afterwards the look picked in Hearth's Settings (desktop.json) and a Hearth OS Global Theme picked in System Settings
# stay in step, one source of truth (docs/BASE-ANATASE.md).
systemctl --global enable hearth-plasma-looks.path

# ------------------------------------------------------------------ boot splash
dnf5 -y install plymouth-plugin-script
plymouth-set-default-theme hearth
# Settings › Boot & startup's presets (hearth-boot-look), in the initramfs as on Debian: picking one only adds
# plymouth.splash=<preset> to the boot menu's entries; a missing one shows the default (Plymouth falls back).
chmod 755 /usr/libexec/nimbus/hearth-boot-look 2>/dev/null || true
mkdir -p /usr/lib/dracut/dracut.conf.d
{
  echo "# Written by os-bases/anatase/build.sh: Hearth's boot splash presets in the initramfs (hearth-boot-look)."
  printf 'install_items+=" %s "\n' "$(find /usr/share/plymouth/themes/hearth-steam /usr/share/plymouth/themes/hearth-minimal \
    /usr/share/plymouth/themes/hearth-aurora /usr/share/plymouth/themes/hearth-starlight /usr/share/plymouth/themes/hearth-orbit \
    -type f 2>/dev/null | sort | tr '\n' ' ')"
} > /usr/lib/dracut/dracut.conf.d/50-hearth-splash.conf
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
export DRACUT_NO_XATTR=1
dracut --no-hostonly --kver "$KVER" --reproducible --zstd --add ostree --add plymouth \
  -f "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"

# ------------------------------------------------------------------ name
# "Hearth OS" everywhere a name is shown (the login prompt, hostnamectl, KInfoCenter, fastfetch); ID stays Anatase's so
# tools still know what's underneath, and "based on Anatase" is only in the variant (About's details).
sed -i \
  -e 's/^NAME=.*/NAME="Hearth OS"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS"/' \
  -e '/^LOGO=/d' -e '/^VARIANT=/d' -e '/^VARIANT_ID=/d' -e '/^HOME_URL=/d' \
  /usr/lib/os-release
printf 'LOGO=hearth\nVARIANT="Fedora edition (test), Fedora %s"\nVARIANT_ID=hearth-anatase\nHOME_URL="https://hearthos.fun"\n' "$FEDORA" >> /usr/lib/os-release
grep -qx 'PRETTY_NAME="Hearth OS"' /usr/lib/os-release && grep -qx 'LOGO=hearth' /usr/lib/os-release

dnf5 clean all
# Unlock the file system: the image's own packages (the start of the record of yours: hearth-unlock --record).
rpm -qa --qf '%{NAME}\n' | grep -vx gpg-pubkey | sort -u > /usr/share/hearth/rpm-baseline.list
test -s /usr/share/hearth/rpm-baseline.list
# Anything that left a repository on for the installed system (the image brings updates; dnf on an unlocked system
# uses Fedora's own repositories and Anatase's).
for r in /etc/yum.repos.d/rpmfusion-*.repo /etc/yum.repos.d/tailscale.repo /etc/yum.repos.d/_copr*.repo; do
  if [ -f "$r" ]; then sed -i 's/^enabled=1/enabled=0/' "$r"; fi
done

# bootc keeps images in podman's storage (imgstorage): without podman, updates and Switch edition fail
# ("podman executable not found in PATH"). Peppermint 0.3.0 shipped without it.
command -v podman >/dev/null || { echo "podman is missing: bootc upgrade and bootc switch need it"; exit 1; }
