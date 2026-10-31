// Hearth Plasma: tells Hearth's controller helper (hearth-pad --plasma, org.hearth.Pad on the session bus) which
// window is in front: its class, title, whether it's fullscreen and what kind it is (desktop, dock, popup...).
// hearth-pad then types keys in Plasma's popups and keyboard apps (A Enter, D-pad arrows) and pauses the
// controller over a fullscreen game, like Hearth Classic's Pad.qml. Turned on only for Hearth Plasma
// (/usr/share/hearth-plasma/xdg/kwinrc [Plugins] hearthpadEnabled). With hearth-pad not running the calls just fail.
var watched = [];

function kindOf(w) {
    if (w.desktopWindow) return "desktop";
    if (w.dock) return "dock";
    if (w.appletPopup || w.popupMenu || w.dropdownMenu) return "popup";
    if (w.notification || w.criticalNotification) return "notification";
    if (w.dialog) return "dialog";
    if (w.normalWindow) return "normal";
    return "other";
}

function report(w) {
    if (!w) {
        callDBus("org.hearth.Pad", "/org/hearth/Pad", "org.hearth.Pad", "Window", "", "", false, "none");
        return;
    }
    callDBus("org.hearth.Pad", "/org/hearth/Pad", "org.hearth.Pad", "Window",
             String(w.resourceClass || ""), String(w.caption || ""), w.fullScreen === true, kindOf(w));
}

function watch(w) {
    if (!w || watched.indexOf(w) >= 0) return;
    watched.push(w);
    w.fullScreenChanged.connect(function () { if (workspace.activeWindow === w) report(w); });
}

workspace.windowActivated.connect(function (w) { watch(w); report(w); });
workspace.windowRemoved.connect(function (w) {
    var i = watched.indexOf(w);
    if (i >= 0) watched.splice(i, 1);
});
report(workspace.activeWindow);
