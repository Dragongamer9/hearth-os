// Run once by hearth-pad --plasma when it starts (KWin's scripting over D-Bus): which window is in front already.
// KWin's hearthpad script (/usr/share/kwin/scripts/hearthpad) says it every time it changes after that.
(function () {
    var w = workspace.activeWindow;
    var kind = !w ? "none" : w.desktopWindow ? "desktop" : w.dock ? "dock" : (w.appletPopup || w.popupMenu || w.dropdownMenu) ? "popup"
        : w.dialog ? "dialog" : w.normalWindow ? "normal" : "other";
    callDBus("org.hearth.Pad", "/org/hearth/Pad", "org.hearth.Pad", "Window",
             w ? String(w.resourceClass || "") : "", w ? String(w.caption || "") : "", !!(w && w.fullScreen === true), kind);
})();
