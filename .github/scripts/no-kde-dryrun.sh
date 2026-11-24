#!/bin/bash
# Run inside Anatase's base image: what removing all of KDE would take with it (a dry run: nothing is removed).
set +e
# What the login screen needs, and what a "remove all of KDE" would take with it (a dry run: nothing is removed).
echo "::notice title=plasma-login-manager needs::$(rpm -q --requires plasma-login-manager 2>/dev/null | grep -iE 'kf6|plasma|kwin|kde|qt6|layer' | sort -u | tr '\n' ' ' | cut -c1-1500)"
echo "::notice title=plasma-login-manager files::$(rpm -ql plasma-login-manager 2>/dev/null | grep -E 'bin/|libexec|\.service' | tr '\n' ' ' | cut -c1-800)"
kdeall=$(rpm -qa --qf '%{NAME}\n' | grep -E '^(plasma|kwin|kde|kf6-|kf5-|breeze|kscreen|powerdevil|bluedevil|polkit-kde|xdg-desktop-portal-kde|dolphin|konsole|kate|okular|spectacle|ark|gwenview|kwallet|discover|libplasma|kpipewire|kglobalacceld|kmenuedit|milou|kinfocenter|systemsettings|kio|kirigami|qqc2|layer-shell-qt|kdecoration|kdsoap|kaccounts|signon|plasma5support|ksystemstats|libkscreen|libksysguard|kuserfeedback|kcm-|aurorae|ocean-sound|oxygen|phonon|baloo|akonadi|kdeconnect|kactivitymanagerd|kded|kscreenlocker|sddm|plasmalogin|maliit|krdp|flatpak-kcm|ksshaskpass|kde-settings)' | sort -u)
echo "::notice title=remove all KDE ($(echo "$kdeall" | wc -l) packages)::dry run follows"
dnf5 remove --assumeno $kdeall >/tmp/rm.txt 2>&1
echo "::notice title=would also go (not KDE)::$(grep -E '^ [a-zA-Z0-9]' /tmp/rm.txt | awk '{print $1}' | grep -vE '^(plasma|kwin|kde|kf6-|kf5-|breeze|kscreen|powerdevil|bluedevil|polkit-kde|xdg-desktop-portal-kde|dolphin|konsole|kate|okular|spectacle|ark|gwenview|kwallet|discover|libplasma|kpipewire|kglobalacceld|kmenuedit|milou|kinfocenter|systemsettings|kio|kirigami|qqc2|layer-shell-qt|kdecoration|kdsoap|kaccounts|signon|plasma5support|ksystemstats|libkscreen|libksysguard|kuserfeedback|kcm-|aurorae|ocean-sound|oxygen|phonon|baloo|akonadi|kdeconnect|kactivitymanagerd|kded|kscreenlocker|sddm|plasmalogin|maliit|krdp|flatpak-kcm|ksshaskpass|kde-settings)' | sort -u | tr '\n' ' ' | cut -c1-3000)"
echo "::notice title=dry run summary::$(grep -iE 'Removing|Transaction Summary|Freed|error|Problem' /tmp/rm.txt | tr '\n' ' ' | cut -c1-800)"
for p in steam gamescope hhd NetworkManager pipewire wireplumber labwc flatpak firefox sunshine; do rpm -q "$p" >/dev/null 2>&1 && { grep -qE "^ $p " /tmp/rm.txt && echo "::warning title=would remove $p::"; }; done
echo "::notice title=Qt apps left::$(rpm -qa --qf '%{NAME}\n' | grep -E '^qt6-' | wc -l) qt6 packages"
