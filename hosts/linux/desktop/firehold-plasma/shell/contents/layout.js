// Firehold's Couch (org.hearthos.firehold): every screen's desktop is the Couch (org.hearthos.firehold.couch) on a
// plain colour (nothing to load or animate under it), and no panels: the Couch draws its own top bar and hints.
var list = desktops();
for (var i = 0; i < list.length; i++) {
    list[i].wallpaperPlugin = "org.kde.color";
    list[i].currentConfigGroup = ["Wallpaper", "org.kde.color", "General"];
    list[i].writeConfig("Color", "#120c09");
}
