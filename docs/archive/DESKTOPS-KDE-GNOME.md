# KDE Plasma and GNOME, downloaded when picked (1.13): archived in 1.14

**Archived** (the owner, Nov 16, 2026: "archive the KDE and GNOME choices, since we're migrating to KWin"). Mantel
moved to KWin instead (docs/DESKTOPS.md). The code is in `archive/desktops-kde-gnome/` under its old paths; the
checklist stays in `docs/mockups/checklists/desktops.md` with its proofs pointing there. What a device that had one
picked does now: Desktop Mode opens Mantel, and Mantel says once that KDE Plasma and GNOME were retired
(`DesktopNotice.qml`, `hearth-your-desktop notice`; Settings › System › Your desktop keeps the last notice). The
`HEARTH_DESKTOP=` line in session.conf is left as it is. KDE's or GNOME's packages someone downloaded stay in their
writable system (hearth-unlock's layer, harmless: their own login entries stay diverted); nothing was ever a
systemd-sysext image, so nothing in /var/lib/extensions needs tidying.

**To bring it back:** move the files back, put the `run_kde` / `run_gnome` part
(`archive/desktops-kde-gnome/hosts/linux/session/nimbus-session-kde-gnome.sh`) back into nimbus-session and
nimbus-mode's `kde_up` / `gnome_up` (the archived copy of nimbus-mode), the login entries and the chmod / checks in
build.sh and base-peppermint.yml, the polkit rule's line, `MantelLookExport.qml`'s Loader in shell.qml and the
wallpaper still in Wallpaper.qml. Tests: `python3 -m unittest discover -s archive/desktops-kde-gnome/hosts/linux/test`
(they expect the files at their old places).


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

## Mantel's look in KDE Plasma and GNOME (1.14)

The owner, Nov 16: KDE and GNOME look exactly like Mantel, bar included, and follow it (this replaces "their own
look" from Nov 14). Settings › System › Your desktop › **KDE and GNOME look like Mantel** (on unless you turn it off:
session.conf's `HEARTH_DESKTOP_LOOK`).

- Mantel writes what it shows: `~/.local/state/hearth/mantel-look.json` (MantelLookExport.qml: the look, light or
  dark, the accent and colours, font, corners, the bar layout) and `mantel-wallpaper.png` (Wallpaper.qml: the
  wallpaper as shown, a moving one as one still moment, without the desktop clock).
- `hearth-desktop-look` puts it in, each time the desktop starts (time-limited, never in the way of starting):
  - **KDE** (before Plasma): Mantel's colour scheme (hearth-theme's, under a name that changes with its colours),
    Breeze dark or light, icons and font in kdeglobals; (once Plasma is up) the colour scheme applied, the wallpaper,
    and the panels laid out as Mantel's: the top bar (Hearth's start menu, tray, Hearth's tray item, clock) and the
    floating dock with your apps in the middle at the bottom, or One main bar on Mantel's edge. The panels are only
    rebuilt when Mantel's layout changes.
  - **GNOME** (before GNOME): dark or light, the nearest GNOME accent, font, the wallpaper, and the bar: GNOME's top bar
    with Dash to Dock floating at the bottom (Mantel's top bar and dock), or Dash to Panel on Mantel's edge (One main
    bar), in Mantel's panel colour. GTK apps' colours come from hearth-theme (gtk-3.0 / gtk-4.0 gtk.css).
- Off: nothing is written; KDE's and GNOME's own settings stay as they are.
- Not the same: GNOME's own top bar keeps GNOME's dark colours (changing it needs a Shell theme), and GNOME's accents
  are its nine (the nearest is used).
