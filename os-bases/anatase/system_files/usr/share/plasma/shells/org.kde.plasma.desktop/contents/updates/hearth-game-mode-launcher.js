// Hearth OS on Anatase (test, 1.15): "Back to Game Mode" pinned first on Plasma's taskbar, once per account (Plasma
// runs each update script once). Your own pins stay; nothing is pinned twice.
var defaults = "applications:hearth-gamemode.desktop,preferred://filemanager,preferred://browser";
panels().forEach(function (panel) {
    panel.widgets(["org.kde.plasma.icontasks", "org.kde.plasma.taskmanager"]).forEach(function (w) {
        w.currentConfigGroup = ["General"];
        var l = String(w.readConfig("launchers", ""));
        if (l.indexOf("hearth-gamemode.desktop") >= 0) return;
        w.writeConfig("launchers", l ? "applications:hearth-gamemode.desktop," + l : defaults);
    });
});
