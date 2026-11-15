# KDE Plasma and GNOME, downloaded when picked (1.13)

Approved mockups: `docs/mockups/next-update/desktops-1-your-desktop-settings.png`, `desktops-2-kde-plasma.png`,
`desktops-3-gnome.png`; checklist `docs/mockups/checklists/desktops.md`. The owner's choices (Nov 14, 2026): downloaded
only when picked; their own look with Hearth inside; each desktop shows its own apps plus the ones you installed;
Mantel stays built in and is where Hearth goes back to. One of the two will probably replace Mantel later, so nothing
Hearth puts inside them depends on Mantel.

## How it's built

1. **Debian's own packages, downloaded when picked.** Peppermint is a writable system (hearth-unlock), so KDE Plasma
   (`kde-plasma-desktop` and KDE's core apps) and GNOME (`gnome-shell`, `gnome-session` and GNOME's core apps, never
   `gdm3`) are installed with apt into this version's layer, from the same frozen Debian date the image was made from
   ("made for Hearth OS <version>"). The root helper `/usr/libexec/nimbus/hearth-desktops-helper` (pkexec, no
   password for the person at the device) runs it as the systemd unit `hearth-desktop-<name>`: download first
   (Cancel works), then install (not cut off halfway). What it added is noted in `/var/lib/hearth/desktops/<name>.json`.
   (The first plan was systemd-sysext images; they'd fight hearth-unlock's own overlay on /usr, so apt it is.)
2. **After an OS update** the new version starts clean and hearth-unlock puts your packages back by itself
   (hearth-unlock-reapply.service); until then Desktop Mode opens Mantel and Settings says it's coming back. The
   previous version keeps its own layer, so it stays startable as it was.
3. **Starting.** `nimbus-session` starts KDE or GNOME only when it's picked and installed; it waits for it to come up
   and opens Mantel instead if it doesn't (the safe start: twice in a row → Mantel and a notice; the choice stays
   saved). Game Mode, the switch and Recovery are unchanged; Mantel stays in every image. The login screen lists
   "KDE Plasma (Hearth)" and "GNOME (Hearth)" once installed (TryExec), both through nimbus-session; KDE's and
   GNOME's own entries are diverted in the image (build.sh), so they can't skip the safe start.
4. **Hearth inside.** KDE: KDE's default panel with Hearth's start menu (`org.hearth.kickoff`: KDE's apps and yours,
   Switch desktop…, Return to Game Mode; `/usr/share/hearth-kde`'s panel layout, first start only) and the tray item
   `org.hearth.tray` (Return to Game Mode, Controller, Keyboard, Hearth notices, Server at home, Hearth update,
   Switch desktop). GNOME: a Hearth section in quick settings (`hearth-quick@hearthos.fun`, on by default through
   `90_hearth-desktops.gschema.override`). Both read `hearth-desktops tray` and act through `hearth-desktops act`.
   The controller is Hearth's pad service in both (hearth-companion, started once the desktop is up).
5. **Each desktop's own apps.** `hearth-app-kinds` (docs/APP-KINDS.md) after every apt run: Hearth's own apps show
   in Mantel, the apps KDE's packages added in KDE, GNOME's in GNOME; apps you install show everywhere. The switch in
   Settings (`/etc/hearth/desktops.json`, with a .bak) turns it off. The kinds table now prefers KDE's and GNOME's
   own apps there (Dolphin, Konsole, Kate, Okular, Gwenview; Files, Console, Text Editor, Calculator, Loupe).
