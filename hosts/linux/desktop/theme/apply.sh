#!/bin/sh
# Gives apps in Hearth Desktop Mode the Hearth look. Run by labwc's autostart.
# Qt/KDE apps (Dolphin, Konsole, System Settings): qt6ct/qt5ct with the Hearth palette and the Breeze style.
#   QT_QPA_PLATFORMTHEME=qt6ct is set only in Desktop Mode (labwc/environment), so Plasma keeps its own look.
#   Your own qt6ct settings are kept: the files are only written while they still carry Hearth's mark.
# GTK apps (Firefox, GNOME apps): dark style and an orange accent.
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
standard_dialogs=default
style=Breeze

[Interface]
activate_item_on_single_click=1
dialog_buttons_have_icons=1
menus_have_icons=true
show_shortcuts_in_context_menus=true
CONF
  fi
done

if command -v gsettings >/dev/null 2>&1; then
  gsettings set org.gnome.desktop.interface color-scheme prefer-dark 2>/dev/null
  gsettings set org.gnome.desktop.interface accent-color orange 2>/dev/null   # GNOME 47+ apps
  gsettings set org.gnome.desktop.interface cursor-theme breeze_cursors 2>/dev/null
fi
exit 0
