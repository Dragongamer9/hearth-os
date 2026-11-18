// Firehold's Desk (mockup 2), the first time it starts: KDE Plasma's own desktop (this shell package is Plasma's
// org.kde.plasma.desktop, copied when the image is built, with this layout), with a bar on top (the app tile and the
// app's name, its menus, the date and clock in the middle, the posture pill, Wi-Fi · battery, the tray) and a dock at
// the bottom (Home, Hearth Files, the browser, Steam, Library, Settings; running apps after them). Only Plasma's
// public scripting API. It has its own layout file (plasma-org.hearthos.firehold.desk-appletsrc): plain Plasma's
// panels are never touched.
var list = desktopsForActivity(currentActivity());
for (var i = 0; i < list.length; i++) {
    list[i].wallpaperPlugin = "org.kde.image";
    list[i].currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    list[i].writeConfig("Image", "file:///usr/share/wallpapers/Hearth-fireside/");
}

var top = new Panel;
top.location = "top";
top.height = Math.round(gridUnit * 2.2);
top.addWidget("org.hearthos.firehold.app");
top.addWidget("org.kde.plasma.appmenu");
top.addWidget("org.kde.plasma.panelspacer");
var clock = top.addWidget("org.kde.plasma.digitalclock");
clock.currentConfigGroup = ["Appearance"];
clock.writeConfig("showDate", true);
clock.writeConfig("dateDisplayFormat", "BesideTime");
clock.writeConfig("dateFormat", "custom");
clock.writeConfig("customDateFormat", "ddd MMM d");
top.addWidget("org.kde.plasma.panelspacer");
top.addWidget("org.hearthos.firehold.posture");
top.addWidget("org.kde.plasma.systemtray");

var dock = new Panel;
dock.location = "bottom";
dock.height = Math.round(gridUnit * 3.6);
var tasks = dock.addWidget("org.kde.plasma.icontasks");
tasks.currentConfigGroup = ["General"];
tasks.writeConfig("launchers", ["applications:org.hearthos.firehold.home.desktop", "applications:hearth-files.desktop",
    "preferred://browser", "applications:steam.desktop", "applications:org.hearthos.firehold.library.desktop",
    "applications:systemsettings.desktop"].join(","));
tasks.writeConfig("showOnlyCurrentDesktop", false);
// Only as long as its icons, in the middle, floating (set once the dock has its widget).
dock.lengthMode = "fit";
dock.alignment = "center";
dock.floating = true;
