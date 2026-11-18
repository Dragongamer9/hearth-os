// Hearth OS on Anatase (test, 1.15): Hearth's look where Plasma still has the system's own, once per account (Plasma
// runs each update script once). Only settings that were never picked: an empty one, or Anatase's or Fedora's own
// default. A wallpaper or menu icon you chose yourself is left exactly as it is (red flag 9).
var distro = /fedora|anatase|start-here|^$/i;
var hearthWall = "file:///usr/share/wallpapers/Hearth-fireside/";
var distroWall = /fedora|anatase|\/Next\/|\/[Ff][0-9][0-9]\/|backgrounds\/(default|f[0-9][0-9])|^$/i;
panels().forEach(function (panel) {
    panel.widgets(["org.kde.plasma.kickoff", "org.kde.plasma.kicker", "org.kde.plasma.kickerdash"]).forEach(function (w) {
        w.currentConfigGroup = ["General"];
        var icon = String(w.readConfig("icon", ""));
        if (distro.test(icon)) w.writeConfig("icon", "hearth");
    });
});
desktops().forEach(function (d) {
    if (d.wallpaperPlugin !== "org.kde.image") return;
    d.currentConfigGroup = ["Wallpaper", "org.kde.image", "General"];
    var img = String(d.readConfig("Image", ""));
    if (distroWall.test(img)) d.writeConfig("Image", hearthWall);
});
