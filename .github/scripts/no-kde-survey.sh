#!/bin/bash
# Run inside Anatase's base image (read only): what KDE is there, and what needs it.
set +e
q() { rpm -qa --qf '%{NAME}\n' | grep -E "$1" | sort | tr '\n' ' '; }
n() { echo "$1" | wc -w; }
echo "::notice title=display manager::$(readlink /etc/systemd/system/display-manager.service) | $(rpm -qa | grep -iE 'plasmalogin|sddm|greetd|gdm|lightdm' | tr '\n' ' ')"
kde=$(q '^(plasma|kwin|kde|breeze|kscreen|powerdevil|bluedevil|polkit-kde|xdg-desktop-portal-kde|dolphin|konsole|kate|okular|spectacle|ark|gwenview|kwallet|plasmalogin|sddm|discover|libplasma|layer-shell-qt|kpipewire|kglobalacceld|kmenuedit|milou|kinfocenter|systemsettings)')
echo "::notice title=KDE and Plasma packages ($(n "$kde"))::$(echo "$kde" | cut -c1-3500)"
echo "::notice title=KDE Frameworks::$(n "$(q '^kf6-')") kf6, $(n "$(q '^kf5-')") kf5"
for p in hhd gamescope steam gamescope-session-plus gamescope-session-steam mesa-vulkan-drivers nvidia-driver akmod-nvidia kmod-nvidia; do
  rpm -q "$p" >/dev/null 2>&1 && echo "::notice title=keeps $p::KDE it needs: $(rpm -q --requires "$p" | grep -iE 'kf6|plasma|kwin|kde' | tr '\n' ' ' | cut -c1-400)"
done
echo "::notice title=what needs kwin, plasma-workspace or plasmalogin::$(rpm -q --whatrequires kwin plasma-workspace plasmalogin 2>/dev/null | grep -v 'no package' | sort -u | tr '\n' ' ' | cut -c1-3000)"
echo "::notice title=Anatase's own packages::$(rpm -qa --qf '%{NAME}|%{VENDOR}|%{PACKAGER}\n' | grep -i anatase | cut -d'|' -f1 | tr '\n' ' ' | cut -c1-2500)"
echo "::notice title=units::$(ls /usr/lib/systemd/system /usr/lib/systemd/user 2>/dev/null | grep -iE 'plasma|sddm|greet|anatase|kde' | tr '\n' ' ')"
echo "::notice title=size::/usr $(du -sxm /usr | cut -f1) MB"
