#!/bin/bash
# Turns Anatase (Fedora 45, bootc) into Hearth OS. Runs once, inside the image build (os-bases/anatase/Containerfile).
# The Hearth parts follow os/build_files/build.sh (the Bazzite build) as closely as possible: Anatase is Fedora
# too, so the same packages and the same steps. What's different is said where it happens:
#  - Handheld Daemon (HHD), already in Anatase, runs the built-in controller, power limit, fan and lights (Bazzite
#    has InputPlumber); Hearth talks to it (hosts/hearth-core/src/hhd.rs, desktop/shell/hearth-hhd).
#  - Steam is installed as Bazzite has it (the steam package, RPM Fusion), so a Bazzite home folder's Steam games
#    (~/.local/share/Steam) are found right after the move; Flathub's Steam at first boot if that can't be had.
#  - /usr/share/nimbus/image-info.json says this is the Anatase edition (Hearth's edition(); the move from the
#    Bazzite-based Hearth checks it), and hearth-clean-install finishes that move at the first start.
# See docs/BASE-ANATASE.md.
set -ouex pipefail

FEDORA=$(rpm -E %fedora)
echo "Anatase on Fedora $FEDORA"

# ------------------------------------------------------------------ files that just get copied in
# Hearth's (os/system_files, shared with the Bazzite build), then this image's Anatase-specific ones on top.
cp -avf /ctx/system_files/. /
cp -avf /ctx/anatase_files/. /

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
else
  cp -a /ctx/nimbus-app/. /usr/lib/nimbus/
fi
test -x /usr/lib/nimbus/nimbus
ln -sf /usr/lib/nimbus/nimbus /usr/bin/nimbus
# ---- hearthfetch (hosts/hearthfetch): Hearth OS's system summary for the terminal, built with the app.
if [ -x /ctx/nimbus-app/hearthfetch ]; then install -Dm755 /ctx/nimbus-app/hearthfetch /usr/bin/hearthfetch; fi
# ---- end hearthfetch

# ------------------------------------------------------------------ SVG icons for GTK (librsvg's gdk-pixbuf loader)
# Without it GTK can't draw an SVG icon and aborts the whole program the first time it needs one (Hearth's Game Mode
# crashed over and over on CachyOS: "image-missing.svg: Unrecognized image file format"). The loaders list is
# rebuilt so the SVG loader is in it.
dnf5 -y install librsvg2 gdk-pixbuf2
# Newer gdk-pixbuf loads SVG (and other images) through glycin's loaders instead.
dnf5 -y install --skip-unavailable glycin-loaders || true
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
dnf5 -y install --skip-unavailable --setopt=fedora-cisco-openh264.enabled=1 openh264 gstreamer1-plugin-openh264 || true
dnf5 -y install --skip-unavailable wf-recorder slurp pulseaudio-utils pipewire-utils pipewire-gstreamer gstreamer1-plugins-good \
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
# ---- end of Hearth's gaming apps

# ---- Nearby Share and Hearth Connect (hosts/hearth-nearby, hosts/hearth-connect) ------------------------------
# Hearth's apps for phones and nearby devices, built with the app (base-anatase.yml) and carried in os/nimbus-app
# (their icons, hearth-nearby and hearth-connect, come with Hearth's icon set). Nearby Share speaks LocalSend's protocol, Hearth Connect KDE Connect's. playerctl lets a phone
# control this PC's music, wl-clipboard/wtype give Hearth Connect the clipboard and typing in Desktop Mode, and
# notify-send's library its notifications. Their firewall helper and polkit rule come from os/system_files
# (hearth-link-firewall, 49-hearth-link.rules): ports open only while the apps listen, and only to local addresses.
dnf5 -y install --skip-unavailable playerctl wl-clipboard wtype libnotify || true
for app in hearth-nearby hearth-connect; do
  if [ -x "/ctx/nimbus-app/$app" ]; then
    install -Dm755 "/ctx/nimbus-app/$app" "/usr/bin/$app"
    if ldd "/usr/bin/$app" | grep "not found"; then echo "warning: $app is missing the libraries above"; fi
  fi
done
chmod 755 /usr/libexec/nimbus/hearth-link-firewall 2>/dev/null || true
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

# ==== Hearth Paint, the emoji picker, Hearth Fonts, and Settings › Accessibility (hosts/hearth-paint, hearth-emoji,
# hearth-fonts, hearth-gamma, desktop/shell/hearth-access): built by CI next to the Hearth app and carried in
# os/nimbus-app (their icons are in the Hearth icon set, installed above). The emoji picker types with wtype and copies with wl-copy; Fonts uses fontconfig;
# Settings › Accessibility uses Orca (the screen reader, with speech-dispatcher and eSpeak NG) and xprop (gamescope's
# zoom and colour filters in Game Mode). Each app is left out if it didn't build or can't find its libraries. Colour
# emoji (Noto) for the picker and the on-screen keyboard's emoji (Desktop Mode draws them with the system's fonts).
for p in orca speech-dispatcher espeak-ng fontconfig wtype wl-clipboard xprop google-noto-color-emoji-fonts; do dnf5 -y install "$p" || echo "warning: $p isn't available"; done
for app in hearth-paint hearth-emoji hearth-fonts hearth-gamma; do
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
dnf5 -y install --skip-unavailable sane-backends sane-backends-drivers-scanners sane-airscan traceroute iputils iw || true
if [ -x /ctx/nimbus-app/hearth-tools/hearth-tools ]; then
  install -Dm755 /ctx/nimbus-app/hearth-tools/hearth-tools /usr/bin/hearth-tools
  if ldd /usr/bin/hearth-tools | grep "not found"; then echo "warning: Hearth Tools is missing the libraries above"; fi
  install -d /usr/share/applications
  install -m644 /ctx/nimbus-app/hearth-tools/hearth-*.desktop /usr/share/applications/
fi
# ---- end Hearth Tools

# The session (Game Mode + Desktop Mode), installed to the system paths instead of /usr/local.
install -Dm755 /ctx/session/nimbus-session /usr/bin/nimbus-session
install -Dm755 /ctx/session/nimbus-mode /usr/bin/nimbus-mode
install -Dm755 /ctx/session/nimbus-watchdog /usr/libexec/nimbus/nimbus-watchdog
mkdir -p /usr/share/wayland-sessions /usr/share/nimbus
sed 's#/usr/local/bin/nimbus-session#/usr/bin/nimbus-session#g' /ctx/session/nimbus.desktop > /usr/share/wayland-sessions/nimbus.desktop
cp -a /ctx/desktop /usr/share/nimbus/desktop
chmod 755 /usr/share/nimbus/desktop/labwc/autostart /usr/share/nimbus/desktop/open.sh /usr/share/nimbus/desktop/theme/apply.sh
find /usr/share/nimbus/desktop/shell -maxdepth 1 -type f -name 'hearth-*' -exec chmod 755 {} +
chmod 755 /usr/bin/hearth-filemanager /usr/bin/hearth-settings /usr/bin/hearth-wallpaper /usr/bin/hearth-browser /usr/bin/hearth-send-logs /usr/bin/hearth-install-package
# hearth-debbox (.deb files through Hearth's Debian box) for terminal users too.
ln -sfn /usr/share/nimbus/desktop/shell/hearth-debbox /usr/bin/hearth-debbox
chmod 755 /usr/libexec/nimbus/*
echo "${NIMBUS_VERSION:-0.0.0}" > /usr/share/nimbus/version
# Which Hearth this is (the Bazzite build has /usr/share/ublue-os/image-info.json for the same; the other editions
# write this same file).
cat > /usr/share/nimbus/image-info.json <<INFO
{"image-name": "hearth-os", "image-ref": "ghcr.io/dragongamer9/hearth-os", "image-tag": "testing", "base": "anatase", "base-detail": "Anatase (Fedora ${FEDORA})", "base-image": "${BASE_IMAGE:-i.anatase.org/anatase:stable}", "version": "${NIMBUS_VERSION:-0.0.0}"}
INFO

# Hearth's app menu entry for Desktop Mode / Plasma too.
cat > /usr/share/applications/nimbus.desktop <<DESKTOP
[Desktop Entry]
Name=Hearth
Comment=Controller-first game library
Exec=/usr/bin/nimbus --windowed
Icon=hearth
Type=Application
Categories=Game;
DESKTOP

# ------------------------------------------------------------------ Desktop Mode: labwc + Quickshell
# (gamescope comes with Anatase.)
dnf5 -y install labwc grim brightnessctl wlr-randr wlsunset python3-evdev \
  qt6ct qt5ct xdg-desktop-portal-wlr xdg-desktop-portal-gtk xprop xwininfo xdotool gnome-keyring cpio rsync
# Quickshell: Fedora's own package when it has one, else the COPR the Bazzite build uses.
if ! dnf5 -y install quickshell; then
  dnf5 -y copr enable errornointernet/quickshell
  dnf5 -y install quickshell
  dnf5 -y copr disable errornointernet/quickshell
fi
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
# Flathub for the system installation (hearth-flatpak-setup adds it from here; Bazaar and Apps' system fallback use it),
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

# ------------------------------------------------------------------ Sunshine (Multi-seat's streamed players, Remote play)
# Settings › Multi-seat can stream an extra player's own Game Mode to Moonlight on a TV, phone, tablet or laptop (one
# Sunshine per streamed player, see docs/MULTI-SEAT.md); Settings › Remote play uses it too. LizardByte's COPR
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
cat > /usr/share/applications/hearth-tasks.desktop <<DESKTOP
[Desktop Entry]
Name=Task Manager
Comment=See what's running, close apps that stopped responding, and check performance
Exec=quickshell ipc -p /usr/share/nimbus/desktop/shell call tasks open
Icon=hearth-tasks
Type=Application
Categories=System;Monitor;
Keywords=task;process;end;kill;cpu;memory;gpu;performance;startup;
OnlyShowIn=Nimbus;
DESKTOP
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
cat > /usr/share/applications/hearth-apps.desktop <<DESKTOP
[Desktop Entry]
Name=Apps
GenericName=App Store
Comment=Find, install and update apps from Flathub
Exec=quickshell ipc -p /usr/share/nimbus/desktop/shell call apps open
Icon=hearth-apps
Type=Application
Categories=System;PackageManager;
Keywords=store;software;install;update;flathub;flatpak;
OnlyShowIn=Nimbus;
DESKTOP
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
dnf5 -y install --skip-unavailable gstreamer1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugin-libav gstreamer1-plugin-openh264 gstreamer1-plugin-dav1d gstreamer1-plugins-base-tools || true
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
dnf5 -y install --skip-unavailable libheif-tools wl-clipboard || true
# HEIC needs an HEVC decoder for libheif: RPM Fusion's libheif-freeworld where RPM Fusion is on.
dnf5 -y install --skip-unavailable libheif-freeworld 2>/dev/null || true
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
org.freedesktop.impl.portal.ScreenCast=wlr
org.freedesktop.impl.portal.Screenshot=wlr
org.freedesktop.impl.portal.FileChooser=hearth
PORTALS
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
# ---- Hearth Office (hosts/hearth-office): Write, Sheets and Slides, built by CI with the app (base-anatase.yml) and
# carried in os/nimbus-app/office. office/install.sh puts in the program (hearth-write, hearth-sheets and
# hearth-slides are links to it), the menu entries and the .hslides file type (the icons are in Hearth's icon set); they're
# the default apps for office files with the others (Hearth's default apps, below). Fonts with
# the same sizes as Office's (Calibri, Cambria, Arial, Times New Roman), so documents keep their layout.
dnf5 -y install --skip-unavailable google-carlito-fonts google-crosextra-caladea-fonts liberation-sans-fonts liberation-serif-fonts liberation-mono-fonts || true
if [ -x /ctx/nimbus-app/office/hearth-office ]; then sh /ctx/nimbus-app/office/install.sh
else echo "Hearth Office wasn't built; this image goes without it"; fi
# ---- end of Hearth Office

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

# ------------------------------------------------------------------ boot straight into Hearth
# nimbus-autologin writes the login screen's autologin: Plasma Login Manager's /etc/plasmalogin.conf on Anatase
# (and SDDM's file, harmless when SDDM isn't there).
systemctl enable nimbus-autologin.service
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

# ------------------------------------------------------------------ boot splash
dnf5 -y install plymouth-plugin-script
plymouth-set-default-theme hearth
KVER=$(ls /usr/lib/modules | sort -V | tail -1)
export DRACUT_NO_XATTR=1
dracut --no-hostonly --kver "$KVER" --reproducible --zstd --add ostree --add plymouth \
  -f "/usr/lib/modules/$KVER/initramfs.img"
chmod 0600 "/usr/lib/modules/$KVER/initramfs.img"

# ------------------------------------------------------------------ name
sed -i \
  -e 's/^NAME=.*/NAME="Hearth"/' \
  -e 's/^PRETTY_NAME=.*/PRETTY_NAME="Hearth OS (based on Anatase)"/' \
  /usr/lib/os-release

dnf5 clean all

# bootc keeps images in podman's storage (imgstorage): without podman, updates and Switch edition fail
# ("podman executable not found in PATH"). Peppermint 0.3.0 shipped without it.
command -v podman >/dev/null || { echo "podman is missing: bootc upgrade and bootc switch need it"; exit 1; }
