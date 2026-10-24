#!/bin/sh
# Gives apps in Hearth Desktop Mode the Hearth look. Run by labwc's autostart.
# Qt/KDE apps (Dolphin, Konsole, System Settings): qt6ct/qt5ct with the Hearth palette and the Breeze style.
#   QT_QPA_PLATFORMTHEME=qt6ct is set only in Desktop Mode (labwc/environment), so Plasma keeps its own look.
#   Your own qt6ct settings are kept: the files are only written while they still carry Hearth's mark.
#   Their Open / Save dialogs go through the desktop portal (standard_dialogs=xdgdesktopportal), which is Hearth Files.
# GTK apps (Bazaar, Firefox, GNOME apps): dark style, Hearth's colours (~/.config/gtk-4.0 and gtk-3.0/gtk.css,
#   also only while they carry Hearth's mark), and Flatpak apps allowed to read them.
# Also run at the start of the Hearth session (both modes), so Bazaar looks right from Game Mode too.
SHARE="${NIMBUS_SHARE:-/usr/local/share/nimbus}"
CONF="${XDG_CONFIG_HOME:-$HOME/.config}"
MARK="# Written by Hearth Desktop Mode; delete this line to keep your own settings."

for v in qt6ct qt5ct; do
  f="$CONF/$v/$v.conf"
  if [ ! -e "$f" ] || head -n 1 "$f" | grep -qF "$MARK"; then
    mkdir -p "$CONF/$v"
    cat >"$f" <<CONF
$MARK
[Appearance]
color_scheme_path=$SHARE/desktop/theme/hearth-qt.conf
custom_palette=true
icon_theme=breeze-dark
standard_dialogs=xdgdesktopportal
style=Breeze

[Interface]
activate_item_on_single_click=1
dialog_buttons_have_icons=1
menus_have_icons=true
show_shortcuts_in_context_menus=true
CONF
  fi
done

GTK_MARK="/* Written by Hearth; delete this line to keep your own settings."
for v in 4 3; do
  f="$CONF/gtk-$v.0/gtk.css"
  if [ ! -e "$f" ] || head -n 1 "$f" | grep -qF "$GTK_MARK"; then
    mkdir -p "$CONF/gtk-$v.0"
    cp "$SHARE/desktop/theme/gtk/gtk$v.css" "$f" 2>/dev/null
  fi
done
if command -v flatpak >/dev/null 2>&1; then
  # One-time: let Flatpak apps (Bazaar is one) see those files.
  stamp="$CONF/nimbus/.flatpak-gtk-theme"
  if [ ! -e "$stamp" ]; then
    flatpak override --user --filesystem=xdg-config/gtk-4.0:ro --filesystem=xdg-config/gtk-3.0:ro 2>/dev/null && { mkdir -p "$CONF/nimbus"; touch "$stamp"; }
  fi
fi

if command -v gsettings >/dev/null 2>&1; then
  gsettings set org.gnome.desktop.interface color-scheme prefer-dark 2>/dev/null
  gsettings set org.gnome.desktop.interface accent-color orange 2>/dev/null   # GNOME 47+ apps
  gsettings set org.gnome.desktop.interface cursor-theme breeze_cursors 2>/dev/null
  # GTK 3 apps: the libadwaita-like style when it's installed (Bazzite ships adw-gtk3), which reads gtk.css colours.
  [ -d /usr/share/themes/adw-gtk3-dark ] && gsettings set org.gnome.desktop.interface gtk-theme adw-gtk3-dark 2>/dev/null
fi
exit 0
