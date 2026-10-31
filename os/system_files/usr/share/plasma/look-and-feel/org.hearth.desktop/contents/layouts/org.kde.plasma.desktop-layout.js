// Hearth Plasma's first layout: the same desktop as Hearth Classic (Mantel, hosts/linux/desktop/shell/Bar.qml),
// part by part. Plasma runs this for a new user (or Global Theme › Hearth with "Use desktop layout from theme").
// Everything here can be changed afterwards like any Plasma panel; your changes are kept.
//
//   Mantel taskbar                          Hearth Plasma panel
//   floating rounded bar, bottom, 60 px     floating panel, bottom, full width
//   Hearth mark (launcher, Super)           org.hearth.launcher (Hearth's launcher: Apps / Games, search)
//   pinned and open apps                    Icons-only Task Manager, Mantel's default pins
//   tray, volume, network, battery, BT      System Tray (with Hearth Link and Hearth Storage in it)
//   Quick Access                            org.hearth.quicksettings
//   bell + clock (calendar, notifications)  Digital Clock (calendar) + Plasma's notifications in the tray
//   Game Mode button                        org.hearth.gamemode
//   show-desktop strip at the very end      Peek at Desktop

var wallpaper = "file:///usr/share/wallpapers/Hearth-fireside/contents/images/3840x2160.jpg";

// ---- the desktop: Hearth's wallpaper, desktop icons on (as in Mantel)
var desktopsArray = desktopsForActivity(currentActivity());
for (var j = 0; j < desktopsArray.length; j++) {
    var d = desktopsArray[j];
    d.wallpaperPlugin = "org.kde.image";
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    d.writeConfig("Image", wallpaper);
    d.writeConfig("FillMode", "2");
}

// ---- the taskbar
var panel = new Panel;
panel.location = "bottom";
panel.height = Math.round(gridUnit * 2.67);   // about Mantel's 50 px bar inside its 60 px strip
panel.floating = true;                        // Mantel's bar floats with a margin all round
panel.lengthMode = "fill";
panel.hiding = "none";                        // windows don't cover it (Mantel's exclusive zone)

panel.addWidget("org.hearth.launcher");

var tasks = panel.addWidget("org.kde.plasma.icontasks");
tasks.currentConfigGroup = ["General"];
// Mantel's default pins (hearth-desk DEFAULT_PINS): Beacon, Xodus Nexus, Bazaar. Missing ones simply don't show.
tasks.writeConfig("launchers", ["applications:beacon.desktop", "applications:xodus-nexus.desktop", "applications:io.github.kolunmi.Bazaar.desktop"]);
tasks.writeConfig("groupingStrategy", "1");        // windows of one app share a button
tasks.writeConfig("middleClickAction", "NewInstance");
tasks.writeConfig("showOnlyCurrentDesktop", "false");
tasks.writeConfig("iconSpacing", "1");

panel.addWidget("org.kde.plasma.marginsseparator");
panel.addWidget("org.kde.plasma.systemtray");
var quick = panel.addWidget("org.hearth.quicksettings");
quick.globalShortcut = "Meta+Alt+A";   // Mantel's Super+A is Plasma's; the controller's Hearth menu › Quick Access types this

var clock = panel.addWidget("org.kde.plasma.digitalclock");
clock.currentConfigGroup = ["Appearance"];
clock.writeConfig("showDate", "true");
clock.writeConfig("dateDisplayFormat", "BelowTime");   // Mantel: the time with the date under it
clock.writeConfig("dateFormat", "shortDate");

panel.addWidget("org.hearth.gamemode");
panel.addWidget("org.kde.plasma.showdesktop");
