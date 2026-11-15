# KDE Plasma and GNOME, downloaded when picked (1.13)

Approved mockups: `docs/mockups/next-update/desktops-1-your-desktop-settings.png`, `desktops-2-kde-plasma.png`,
`desktops-3-gnome.png`; checklist `docs/mockups/checklists/desktops.md`. The owner's choices (Nov 14, 2026): downloaded
only when picked; their own look with Hearth inside; each desktop shows its own apps plus the ones you installed;
Mantel stays built in and is where Hearth goes back to. One of the two will probably replace Mantel later, so nothing
Hearth puts inside them depends on Mantel.

## How it's built

1. **Add-on images per release.** `base-peppermint.yml` builds the OS image, then two more stages on top of it with
   KDE Plasma (`kde-plasma-desktop` + Hearth's tray item) and GNOME (`gnome-shell`, `gnome-session` + Hearth's
   extension). Only what those stages add under `/usr` is packed as a systemd-sysext image
   (`hearth-kde-<build>.raw`, `hearth-gnome-<build>.raw`, erofs), with an `extension-release` naming the exact
   image it was made from. What they add under `/etc` goes into a matching confext. They're published next to the
   image and only ever fit that one build.
2. **Downloading.** `hearth-desktops` (root helper, through pkexec like Hearth's others) downloads the add-on for the
   running build, checks its SHA-256, puts it in `/var/lib/extensions` and runs `systemd-sysext refresh`. Remove
   takes it out the same way. After an OS update the old add-on no longer fits: it isn't merged, and the new one is
   downloaded in the background; until then Desktop Mode opens Mantel and says why.
3. **Starting.** `nimbus-session` starts KDE or GNOME only when it's picked and its add-on is merged; it waits for it
   to come up and opens Mantel instead if it doesn't (the safe start: twice in a row → Mantel and a notice; the
   choice stays saved). Game Mode, the switch and Recovery are unchanged; Mantel stays in every image.
4. **Hearth inside.** KDE: one tray item (`org.hearth.tray`) with Return to Game Mode, controller, keyboard, Hearth
   notices, home server, Hearth update and Switch desktop. GNOME: a Hearth section in quick settings
   (`hearth-quick@hearthos.fun`) with the same rows. The controller and on-screen keyboard use Hearth's own pad
   service in both.
5. **Each desktop's own apps.** KDE's apps get `OnlyShowIn=KDE;`, GNOME's `OnlyShowIn=GNOME;` (done when the add-on
   is built), Hearth's own apps `OnlyShowIn=Hearth;`; apps you install are left alone, so they show everywhere. The
   switch in Settings turns this off by giving each desktop all apps again.
